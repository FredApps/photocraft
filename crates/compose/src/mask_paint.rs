//! Incremental compositor for experimental mask brush layers.
use photocraft_doc::{LayerMask, mask_paint::Stack};
use photocraft_geom::Rect;
/// Refresh changed tiles after an edit; property/order changes rebuild the stack.
pub fn refresh_changes(mask: &mut LayerMask, old: Option<&Stack>) {
    let Some(stack) = &mask.paint else { return };
    let Some(old) = old else {
        refresh(mask, None);
        return;
    };
    if stack.base != old.base || stack.layers.iter().chain(&old.layers).any(|p| p.surface.default_pixel().get(3).is_some_and(|a| *a != 0.0)) {
        refresh(mask, None);
        return;
    }
    if stack.layers.len() != old.layers.len() {
        let mut tiles = std::collections::BTreeSet::new();
        for paint in stack.layers.iter().chain(&old.layers) {
            tiles.extend(paint.surface.tiles().map(|(c, _)| *c));
        }
        refresh_tiles(mask, tiles.into_iter().collect());
        return;
    }
    let mut changed = std::collections::BTreeSet::new();
    for (a, b) in stack.layers.iter().zip(&old.layers) {
        if a.opacity != b.opacity || a.visible != b.visible {
            changed.extend(a.surface.tiles().map(|(coord, _)| *coord));
            changed.extend(b.surface.tiles().map(|(coord, _)| *coord));
        }
        for (coord, tile) in a.surface.tiles() {
            if b.surface.tile(*coord).is_none_or(|prior| !std::sync::Arc::ptr_eq(prior, tile)) {
                changed.insert(*coord);
            }
        }
        for (coord, _) in b.surface.tiles() {
            if a.surface.tile(*coord).is_none() {
                changed.insert(*coord);
            }
        }
    }
    if changed.len() <= 1 {
        for coord in changed {
            refresh(mask, Some(coord.rect()));
        }
    } else {
        refresh_tiles(mask, changed.into_iter().collect());
    }
}
/// Recompose only the damaged rectangle, in bounded scanline bands.
pub fn refresh(mask: &mut LayerMask, region: Option<Rect>) {
    let Some(stack) = &mask.paint else { return };
    let mut default = stack.base.default_pixel().first().copied().unwrap_or(1.0);
    for paint in &stack.layers {
        if !paint.visible || paint.opacity <= 0.0 {
            continue;
        }
        let source = paint.surface.default_pixel();
        let alpha = source.get(3).copied().unwrap_or(0.0) * paint.opacity;
        let value = source.first().copied().unwrap_or(0.0);
        default = value * alpha + default * (1.0 - alpha);
    }
    mask.surface.set_default_pixel(&[default]);
    let Some(bounds) = region else {
        // Sparse strokes can be far apart: never scan the empty atlas between them.
        let mut tiles = std::collections::BTreeSet::new();
        tiles.extend(mask.surface.tiles().map(|(c, _)| *c));
        tiles.extend(stack.base.tiles().map(|(c, _)| *c));
        for paint in &stack.layers {
            tiles.extend(paint.surface.tiles().map(|(c, _)| *c));
        }
        refresh_tiles(mask, tiles.into_iter().collect());
        return;
    };
    if bounds.is_empty() {
        return;
    }
    let mut y = bounds.y0;
    while y < bounds.y1 {
        let end = y.saturating_add(32).min(bounds.y1);
        let band = Rect::new(bounds.x0, y, bounds.x1, end);
        let mut pixels = stack.base.read_region(band);
        for p in &stack.layers {
            if !p.visible || p.opacity <= 0.0 {
                continue;
            }
            let source = p.surface.read_region(band);
            for (dst, [value, _, _, alpha]) in pixels.iter_mut().zip(source.as_chunks::<4>().0) {
                let a = *alpha * p.opacity;
                *dst = *value * a + *dst * (1.0 - a);
            }
        }
        mask.surface.write_region(band, &pixels);
        y = end;
    }
    // Keep constant cache tiles: pruning the entire mask here would scan every
    // previously touched tile on each live brush update.
}

/// Recompose independent cached tiles in parallel on native builds. Each tile is evaluated in
/// f32 and encoded once into the destination mask format, matching the canonical CPU compositor.
fn refresh_tiles(mask: &mut LayerMask, coords: Vec<photocraft_geom::TileCoord>) {
    let Some(stack) = mask.paint.as_ref() else { return };
    if coords.is_empty() {
        return;
    }
    let mut default = stack.base.default_pixel().first().copied().unwrap_or(1.0);
    for paint in &stack.layers {
        if !paint.visible || paint.opacity <= 0.0 {
            continue;
        }
        let source = paint.surface.default_pixel();
        let alpha = source.get(3).copied().unwrap_or(0.0) * paint.opacity;
        let value = source.first().copied().unwrap_or(0.0);
        default = value * alpha + default * (1.0 - alpha);
    }
    mask.surface.set_default_pixel(&[default]);
    let format = mask.surface.format();
    let default = mask.surface.default_pixel();
    let shared_default_tile = mask.surface.solid_tile(&default);
    let compose = |coord: &photocraft_geom::TileCoord| {
        let region = coord.rect();
        let mut pixels = stack.base.read_region(region);
        for paint in &stack.layers {
            if !paint.visible || paint.opacity <= 0.0 {
                continue;
            }
            if composite_rgba_tile_into_gray(&mut pixels, &paint.surface, *coord, paint.opacity) {
                continue;
            }
            let source = paint.surface.read_region(region);
            for (dst, [value, _, _, alpha]) in pixels.iter_mut().zip(source.as_chunks::<4>().0) {
                let a = *alpha * paint.opacity;
                *dst = *value * a + *dst * (1.0 - a);
            }
        }
        let mut output = photocraft_raster::Surface::with_default(format, &default);
        output.write_region(region, &pixels);
        let tile = output.tile(*coord).cloned().unwrap_or_else(|| shared_default_tile.clone());
        (*coord, tile)
    };
    #[cfg(not(target_arch = "wasm32"))]
    let output = {
        use rayon::prelude::*;
        if coords.len() >= 2 * rayon::current_num_threads() {
            coords.par_iter().map(compose).collect::<Vec<_>>()
        } else {
            coords.iter().map(compose).collect::<Vec<_>>()
        }
    };
    #[cfg(target_arch = "wasm32")]
    let output = coords.iter().map(compose).collect::<Vec<_>>();
    mask.surface.put_tiles(output);
}

/// Blend the red channel of a straight RGBA Paint tile directly into its grayscale destination.
/// Paint is single-channel data replicated for display, so decoding G/B and allocating a full
/// RGBA f32 tile for each layer only adds memory traffic. Unsupported pixel layouts keep the
/// canonical region-decoding fallback above.
fn composite_rgba_tile_into_gray(dst: &mut [f32], surface: &photocraft_raster::Surface, coord: photocraft_geom::TileCoord, opacity: f32) -> bool {
    use photocraft_color::{ColorMode, SampleType};
    let format = surface.format();
    if format.mode != ColorMode::Rgb || !format.alpha || format.channels() != 4 || dst.len() != (photocraft_geom::TILE_SIZE as usize).pow(2) {
        return false;
    }
    let Some(tile) = surface.tile(coord) else {
        let default = surface.default_pixel();
        let Some((&red, &alpha)) = default.first().zip(default.get(3)) else { return false };
        let a = alpha * opacity;
        for value in dst {
            *value = red * a + *value * (1.0 - a);
        }
        return true;
    };
    let bytes = tile.bytes();
    match format.sample {
        SampleType::U8 => {
            for (out, px) in dst.iter_mut().zip(bytes.as_chunks::<4>().0.iter()) {
                let value = f32::from(px[0]) / 255.0;
                let a = f32::from(px[3]) / 255.0 * opacity;
                *out = value * a + *out * (1.0 - a);
            }
        }
        SampleType::U16 => {
            for (out, px) in dst.iter_mut().zip(bytes.as_chunks::<8>().0.iter()) {
                let value = f32::from(u16::from_ne_bytes([px[0], px[1]])) / 65535.0;
                let alpha = f32::from(u16::from_ne_bytes([px[6], px[7]])) / 65535.0;
                let a = alpha * opacity;
                *out = value * a + *out * (1.0 - a);
            }
        }
        SampleType::F32 => {
            for (out, px) in dst.iter_mut().zip(bytes.as_chunks::<16>().0.iter()) {
                let value = f32::from_ne_bytes([px[0], px[1], px[2], px[3]]);
                let alpha = f32::from_ne_bytes([px[12], px[13], px[14], px[15]]);
                let a = alpha * opacity;
                *out = value * a + *out * (1.0 - a);
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use photocraft_color::PixelFormat;
    use photocraft_doc::mask_paint::Paint;
    use photocraft_geom::{Rect, TileCoord};
    use photocraft_raster::Surface;
    use std::sync::Arc;

    #[test]
    fn many_tile_opacity_refresh_matches_default_and_preserves_untouched_cow_tiles() {
        let gray = PixelFormat::GRAY8;
        let rgba = PixelFormat::RGBA8;
        let mut mask = LayerMask::hide_all();
        mask.surface = Surface::with_default(gray, &[0.0]);
        let base = Surface::with_default(gray, &[0.2]);
        let mut paint_surface = Surface::with_default(rgba, &[0.7, 0.1, 0.05, 0.0]);
        for x in 0..40 {
            let x = x * photocraft_geom::TILE_SIZE;
            paint_surface.write_region(Rect::new(x, 0, x + 1, 1), &[0.9, 0.2, 0.1, 0.75]);
        }
        mask.paint = Some(Stack {
            base: base.clone(),
            layers: vec![Paint { name: "parallel".into(), visible: true, opacity: 1.0, surface: paint_surface.clone() }],
            active: 0,
        });
        refresh(&mut mask, None);
        mask.surface.write_region(Rect::new(50 * photocraft_geom::TILE_SIZE, 0, 50 * photocraft_geom::TILE_SIZE + 1, 1), &[0.4]);
        let untouched = mask.surface.tile(TileCoord::new(50, 0)).unwrap().clone();
        let old = mask.paint.as_ref().unwrap().clone();
        mask.paint.as_mut().unwrap().layers[0].opacity = 0.25;
        refresh_changes(&mut mask, Some(&old));

        let expected_default = Surface::with_default(gray, &[0.2]).default_pixel();
        assert_eq!(mask.surface.default_pixel(), expected_default);
        let base_value = base.read_region(Rect::new(0, 0, 1, 1))[0];
        let source = paint_surface.read_region(Rect::new(0, 0, 1, 1));
        let expected_pixel = Surface::with_default(gray, &[source[0] * source[3] * 0.25 + base_value * (1.0 - source[3] * 0.25)]).default_pixel();
        assert_eq!(mask.surface.read_region(Rect::new(0, 0, 1, 1)), expected_pixel);
        assert!(Arc::ptr_eq(&untouched, mask.surface.tile(TileCoord::new(50, 0)).unwrap()), "unaffected cache tiles retain COW storage");
    }

    #[test]
    fn direct_rgba_tile_blend_matches_region_compositor_for_all_sample_types() {
        for sample in [photocraft_color::SampleType::U8, photocraft_color::SampleType::U16, photocraft_color::SampleType::F32] {
            let gray = photocraft_color::PixelFormat::new(photocraft_color::ColorMode::Grayscale, sample, false);
            let rgba = photocraft_color::PixelFormat::new(photocraft_color::ColorMode::Rgb, sample, true);
            let mut base = Surface::with_default(gray, &[0.23]);
            let mut source = Surface::with_default(rgba, &[0.72, 0.13, 0.04, 0.0]);
            let rect = Rect::new(7, 9, 19, 17);
            let mut source_pixels = Vec::new();
            let mut base_pixels = Vec::new();
            for y in rect.y0..rect.y1 {
                for x in rect.x0..rect.x1 {
                    let v = ((x * 13 + y * 7) % 31) as f32 / 31.0;
                    source_pixels.extend_from_slice(&[0.1 + v * 0.8, 0.05 + v * 0.4, 0.02 + v * 0.2, 0.15 + v * 0.75]);
                    base_pixels.push(0.1 + v * 0.7);
                }
            }
            source.write_region(rect, &source_pixels);
            base.write_region(rect, &base_pixels);
            let mut fast = LayerMask::hide_all();
            fast.surface = Surface::with_default(gray, &[0.0]);
            fast.paint = Some(Stack {
                base: base.clone(),
                layers: vec![Paint { name: "source".into(), visible: true, opacity: 0.73, surface: source.clone() }],
                active: 0,
            });
            let mut oracle = fast.clone();
            refresh(&mut fast, None);
            let tile = TileCoord::new(0, 0);
            // The bounded scanline path remains an independent oracle for the tile-local kernel.
            refresh(&mut oracle, Some(tile.rect()));
            assert_eq!(fast.surface.default_pixel(), oracle.surface.default_pixel(), "default mismatch at {sample:?}");
            assert_eq!(fast.surface.tile(tile).unwrap().bytes(), oracle.surface.tile(tile).unwrap().bytes(), "tile mismatch at {sample:?}");
        }
    }

    #[test]
    fn direct_rgba_tile_blend_matches_defaults_and_negative_coordinates() {
        let gray = PixelFormat::GRAY8;
        let rgba = PixelFormat::RGBA8;
        let mut base = Surface::with_default(gray, &[0.3]);
        let mut source = Surface::with_default(rgba, &[0.8, 0.2, 0.1, 0.4]);
        let painted = Rect::new(-255, 2, -254, 3);
        base.write_region(painted, &[0.75]);
        source.write_region(painted, &[0.1, 0.7, 0.2, 0.9]);
        // A base-only tile exercises the non-allocated Paint tile with a nonzero RGBA default.
        base.write_region(Rect::new(256, 0, 257, 1), &[0.6]);
        let mut optimized = LayerMask::hide_all();
        optimized.surface = Surface::with_default(gray, &[0.0]);
        optimized.paint = Some(Stack { base, layers: vec![Paint { name: "default source".into(), visible: true, opacity: 0.65, surface: source }], active: 0 });
        let mut oracle = optimized.clone();
        refresh(&mut optimized, None);
        refresh(&mut oracle, Some(TileCoord::new(-1, 0).rect()));
        refresh(&mut oracle, Some(TileCoord::new(1, 0).rect()));
        assert_eq!(optimized.surface.default_pixel(), oracle.surface.default_pixel());
        for coord in [TileCoord::new(-1, 0), TileCoord::new(1, 0)] {
            assert_eq!(optimized.surface.tile(coord).unwrap().bytes(), oracle.surface.tile(coord).unwrap().bytes(), "tile {coord:?}");
        }
    }

    #[test]
    #[ignore = "dense 4K opacity refresh diagnostic; run with --ignored --nocapture"]
    fn benchmark_dense_4k_ten_paint_mask_refresh_changes() {
        const SIZE: i32 = 4096;
        const TILE: i32 = photocraft_geom::TILE_SIZE;
        let gray = PixelFormat::new(photocraft_color::ColorMode::Grayscale, photocraft_color::SampleType::U16, false);
        let rgba = PixelFormat::RGBA16;
        let mut base = Surface::with_default(gray, &[0.18]);
        let mut paints = Vec::new();
        for index in 0..10 {
            let mut surface = Surface::with_default(rgba, &[0.75, 0.2, 0.05, 0.28 + index as f32 * 0.01]);
            for y in (0..SIZE).step_by(TILE as usize) {
                for x in (0..SIZE).step_by(TILE as usize) {
                    let mut pixels = Vec::with_capacity(TILE as usize * TILE as usize * 4);
                    for py in 0..TILE {
                        for px in 0..TILE {
                            let pattern = ((x + px) * 7 + (y + py) * 11 + index * 29).rem_euclid(257) as f32 / 257.0;
                            pixels.extend_from_slice(&[0.18 + pattern * 0.55, 0.08 + pattern * 0.22, 0.04 + pattern * 0.12, 0.24 + pattern * 0.52]);
                        }
                    }
                    surface.write_region(Rect::new(x, y, x + TILE, y + TILE), &pixels);
                }
            }
            paints.push(Paint { name: format!("Paint {index}"), visible: true, opacity: 0.75, surface });
        }
        for y in (0..SIZE).step_by(TILE as usize) {
            for x in (0..SIZE).step_by(TILE as usize) {
                let mut pixels = Vec::with_capacity(TILE as usize * TILE as usize);
                for py in 0..TILE {
                    for px in 0..TILE {
                        pixels.push(0.12 + (((x + px) * 3 + (y + py) * 5).rem_euclid(701) as f32) / 1400.0);
                    }
                }
                base.write_region(Rect::new(x, y, x + TILE, y + TILE), &pixels);
            }
        }
        let mut mask = LayerMask::hide_all();
        mask.surface = Surface::with_default(gray, &[0.0]);
        mask.paint = Some(Stack { base, layers: paints, active: 9 });
        refresh(&mut mask, None);

        let old = mask.paint.as_ref().unwrap().clone();
        let base_eq = std::time::Instant::now();
        for _ in 0..20 {
            std::hint::black_box(mask.paint.as_ref().unwrap().base == old.base);
        }
        let base_eq_ms = base_eq.elapsed().as_secs_f64() * 1000.0 / 20.0;
        let mut samples = Vec::new();
        for iteration in 0..5 {
            let previous = mask.paint.as_ref().unwrap().clone();
            mask.paint.as_mut().unwrap().layers[9].opacity = if iteration % 2 == 0 { 0.70 } else { 0.75 };
            let start = std::time::Instant::now();
            refresh_changes(&mut mask, Some(&previous));
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        samples.sort_by(f64::total_cmp);
        println!("dense 4K/10 Paint mask refresh_changes ms={samples:?}; base Surface equality ms={base_eq_ms:.2} per compare");
    }
}
