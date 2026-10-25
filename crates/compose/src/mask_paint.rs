//! Incremental compositor for editable mask stack effects.
use photocraft_doc::{
    LayerMask,
    mask_paint::{EffectKind, Stack},
};
use photocraft_geom::Rect;

/// Refresh changed tiles after an edit; property/order changes rebuild the stack.
pub fn refresh_changes(mask: &mut LayerMask, old: Option<&Stack>) {
    let Some(stack) = &mask.paint else { return };
    let Some(old) = old else {
        refresh(mask, None);
        return;
    };
    if stack.base != old.base || stack.layers.len() != old.layers.len() {
        refresh(mask, None);
        return;
    }
    if stack
        .layers
        .iter()
        .chain(&old.layers)
        .any(|layer| matches!(&layer.kind, EffectKind::Paint) && layer.surface.default_pixel().get(3).is_some_and(|alpha| *alpha != 0.0))
    {
        refresh(mask, None);
        return;
    }
    let mut changed = std::collections::BTreeSet::new();
    for (now, before) in stack.layers.iter().zip(&old.layers) {
        if now.fill_map != before.fill_map || now.fill_map_size != before.fill_map_size || now.fill_bounds != before.fill_bounds {
            refresh(mask, None);
            return;
        }
        if now.surface.default_pixel() != before.surface.default_pixel() {
            if !matches!(&now.kind, EffectKind::Paint) || !matches!(&before.kind, EffectKind::Paint) {
                refresh(mask, None);
                return;
            }
            changed.extend(now.surface.tiles().map(|(coord, _)| *coord));
            changed.extend(before.surface.tiles().map(|(coord, _)| *coord));
        }
        if now.visible != before.visible || now.opacity != before.opacity || now.kind != before.kind {
            // Fill and Levels can affect every pixel in the accumulated mask.
            if !matches!(&now.kind, EffectKind::Paint) || !matches!(&before.kind, EffectKind::Paint) {
                refresh(mask, None);
                return;
            }
            changed.extend(now.surface.tiles().map(|(coord, _)| *coord));
            changed.extend(before.surface.tiles().map(|(coord, _)| *coord));
        }
        for (coord, tile) in now.surface.tiles() {
            if before.surface.tile(*coord).is_none_or(|prior| !std::sync::Arc::ptr_eq(prior, tile)) {
                changed.insert(*coord);
            }
        }
        for (coord, _) in before.surface.tiles() {
            if now.surface.tile(*coord).is_none() {
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
    let default = composite_default(stack);
    mask.surface.set_default_pixel(&[default]);
    let Some(bounds) = region else {
        let mut tiles = std::collections::BTreeSet::new();
        tiles.extend(mask.surface.tiles().map(|(c, _)| *c));
        tiles.extend(stack.base.tiles().map(|(c, _)| *c));
        for paint in &stack.layers {
            if matches!(&paint.kind, EffectKind::Paint) {
                tiles.extend(paint.surface.tiles().map(|(c, _)| *c));
            }
            if let Some([x0, y0, x1, y1]) = paint.fill_bounds {
                if x1 > x0 && y1 > y0 {
                    tiles.extend(Rect::new(x0, y0, x1, y1).tiles());
                }
            }
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
        for layer in &stack.layers {
            if !layer.visible || layer.opacity <= 0.0 {
                continue;
            }
            match &layer.kind {
                EffectKind::Paint => {
                    let source = layer.surface.read_region(band);
                    for (dst, px) in pixels.iter_mut().zip(source.as_chunks::<4>().0) {
                        let alpha = px[3] * layer.opacity;
                        *dst = px[0] * alpha + *dst * (1.0 - alpha);
                    }
                }
                EffectKind::Fill { value } => {
                    for (offset, dst) in pixels.iter_mut().enumerate() {
                        let x = band.x0 + (offset as i32 % band.width() as i32);
                        let y = band.y0 + (offset as i32 / band.width() as i32);
                        if let Some(fill) = fill_sample(layer, *value, x, y) {
                            *dst = fill * layer.opacity + *dst * (1.0 - layer.opacity);
                        }
                    }
                }
                EffectKind::Levels { input_black, gamma, input_white, output_black, output_white } => {
                    for dst in &mut pixels {
                        let adjusted = levels(*dst, *input_black, *gamma, *input_white, *output_black, *output_white);
                        *dst = adjusted * layer.opacity + *dst * (1.0 - layer.opacity);
                    }
                }
            }
        }
        mask.surface.write_region(band, &pixels);
        y = end;
    }
}

fn composite_default(stack: &Stack) -> f32 {
    let mut value = stack.base.default_pixel().first().copied().unwrap_or(1.0);
    for layer in &stack.layers {
        if layer.visible && layer.opacity > 0.0 {
            value = match &layer.kind {
                EffectKind::Paint => {
                    let source = layer.surface.default_pixel();
                    let alpha = source.get(3).copied().unwrap_or(0.0) * layer.opacity;
                    source.first().copied().unwrap_or(0.0) * alpha + value * (1.0 - alpha)
                }
                EffectKind::Fill { value: fill } => {
                    let map_value = layer.fill_map.as_ref().map(|map| map.default_pixel().first().copied().unwrap_or(*fill)).unwrap_or(*fill);
                    if layer.fill_bounds.is_none() { map_value * layer.opacity + value * (1.0 - layer.opacity) } else { value }
                }
                EffectKind::Levels { input_black, gamma, input_white, output_black, output_white } => {
                    let adjusted = levels(value, *input_black, *gamma, *input_white, *output_black, *output_white);
                    adjusted * layer.opacity + value * (1.0 - layer.opacity)
                }
            };
        }
    }
    value
}

fn levels(value: f32, input_black: f32, gamma: f32, input_white: f32, output_black: f32, output_white: f32) -> f32 {
    let width = (input_white - input_black).max(f32::EPSILON);
    let normalized = ((value - input_black) / width).clamp(0.0, 1.0);
    let exponent = 1.0 / gamma.clamp(0.01, 100.0);
    output_black + normalized.powf(exponent) * (output_white - output_black)
}

/// Composite the stack prefix into an unquantized grayscale buffer for one bounded region.
/// `end_layer` is exclusive; tile-aligned regions are recommended for stroke preview caches.
pub fn compose_prefix(stack: &Stack, end_layer: usize, region: Rect) -> Option<Vec<f32>> {
    if end_layer > stack.layers.len() || region.is_empty() {
        return None;
    }
    let count = usize::try_from(region.width()).ok()?.checked_mul(usize::try_from(region.height()).ok()?)?;
    if count > 256 * 256 * 4 {
        return None;
    }
    let pixels = stack.base.read_region(region);
    Some(composite_layers(stack, 0, end_layer, region, pixels))
}

/// Apply stack layers from `start_layer` onward to a cached prefix buffer and write one region.
/// `initial` must be an unquantized, row-major grayscale buffer matching `region` exactly.
pub fn refresh_from_prefix(mask: &mut LayerMask, region: Rect, start_layer: usize, initial: &[f32]) -> bool {
    if region.is_empty() {
        return false;
    }
    let Some(stack) = mask.paint.as_ref() else { return false };
    let Some(count) = usize::try_from(region.width()).ok().and_then(|w| usize::try_from(region.height()).ok().and_then(|h| w.checked_mul(h))) else {
        return false;
    };
    if count != initial.len() || start_layer > stack.layers.len() || count > 256 * 256 * 4 {
        return false;
    }
    let pixels = composite_layers(stack, start_layer, stack.layers.len(), region, initial.to_vec());
    mask.surface.set_default_pixel(&[composite_default(stack)]);
    mask.surface.write_region(region, &pixels);
    true
}

fn composite_layers(stack: &Stack, start_layer: usize, end_layer: usize, region: Rect, mut pixels: Vec<f32>) -> Vec<f32> {
    for layer in stack.layers.get(start_layer..end_layer).unwrap_or_default() {
        if !layer.visible || layer.opacity <= 0.0 {
            continue;
        }
        match &layer.kind {
            EffectKind::Paint => {
                composite_paint_region(&mut pixels, &layer.surface, region, layer.opacity);
            }
            EffectKind::Fill { value } => {
                for (offset, dst) in pixels.iter_mut().enumerate() {
                    let x = region.x0 + (offset as i32 % region.width() as i32);
                    let y = region.y0 + (offset as i32 / region.width() as i32);
                    if let Some(fill) = fill_sample(layer, *value, x, y) {
                        *dst = fill * layer.opacity + *dst * (1.0 - layer.opacity);
                    }
                }
            }
            EffectKind::Levels { input_black, gamma, input_white, output_black, output_white } => {
                for dst in &mut pixels {
                    let adjusted = levels(*dst, *input_black, *gamma, *input_white, *output_black, *output_white);
                    *dst = adjusted * layer.opacity + *dst * (1.0 - layer.opacity);
                }
            }
        }
    }
    pixels
}

fn composite_paint_region(dst: &mut [f32], surface: &photocraft_raster::Surface, region: Rect, opacity: f32) {
    let tile = photocraft_geom::TILE_SIZE;
    if region.width() == tile as u32 && region.height() == tile as u32 && region.x0.rem_euclid(tile) == 0 && region.y0.rem_euclid(tile) == 0 {
        let coord = photocraft_geom::TileCoord::new(region.x0.div_euclid(tile), region.y0.div_euclid(tile));
        if composite_rgba_tile_into_gray(dst, surface, coord, opacity) {
            return;
        }
    }
    let source = surface.read_region(region);
    for (dst, px) in dst.iter_mut().zip(source.as_chunks::<4>().0) {
        let alpha = px[3] * opacity;
        *dst = px[0] * alpha + *dst * (1.0 - alpha);
    }
}

fn refresh_tiles(mask: &mut LayerMask, coords: Vec<photocraft_geom::TileCoord>) {
    let Some(stack) = mask.paint.as_ref() else { return };
    if coords.is_empty() {
        return;
    }
    let default = [composite_default(stack)];
    mask.surface.set_default_pixel(&default);
    let format = mask.surface.format();
    let default = mask.surface.default_pixel();
    let shared_default_tile = mask.surface.solid_tile(&default);
    let compose = |coord: &photocraft_geom::TileCoord| {
        let region = coord.rect();
        let mut pixels = stack.base.read_region(region);
        for layer in &stack.layers {
            if !layer.visible || layer.opacity <= 0.0 {
                continue;
            }
            match &layer.kind {
                EffectKind::Paint => {
                    composite_paint_region(&mut pixels, &layer.surface, region, layer.opacity);
                }
                EffectKind::Fill { value } => {
                    for (offset, dst) in pixels.iter_mut().enumerate() {
                        let x = region.x0 + (offset as i32 % region.width() as i32);
                        let y = region.y0 + (offset as i32 / region.width() as i32);
                        if let Some(fill) = fill_sample(layer, *value, x, y) {
                            *dst = fill * layer.opacity + *dst * (1.0 - layer.opacity);
                        }
                    }
                }
                EffectKind::Levels { input_black, gamma, input_white, output_black, output_white } => {
                    for dst in &mut pixels {
                        let adjusted = levels(*dst, *input_black, *gamma, *input_white, *output_black, *output_white);
                        *dst = adjusted * layer.opacity + *dst * (1.0 - layer.opacity);
                    }
                }
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

fn fill_sample(layer: &photocraft_doc::mask_paint::Paint, value: f32, x: i32, y: i32) -> Option<f32> {
    let bounds = layer.fill_bounds;
    let (u, v) = if let Some([x0, y0, x1, y1]) = bounds {
        if x < x0 || y < y0 || x >= x1 || y >= y1 || x1 <= x0 || y1 <= y0 {
            return None;
        }
        (((x - x0) as f32 + 0.5) / (x1 - x0) as f32, ((y - y0) as f32 + 0.5) / (y1 - y0) as f32)
    } else {
        (0.0, 0.0)
    };
    let Some(map) = &layer.fill_map else { return Some(value) };
    let [width, height] = layer.fill_map_size;
    if width == 0 || height == 0 {
        return Some(value);
    }
    let sx = (u * width as f32).floor().clamp(0.0, width.saturating_sub(1) as f32) as i32;
    let sy = (v * height as f32).floor().clamp(0.0, height.saturating_sub(1) as f32) as i32;
    Some(map.sample_channel(sx, sy, 0))
}

/// Blend the red channel of a straight RGBA Paint tile directly into its grayscale destination.
/// Other layouts keep the canonical region-decoding fallback.
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
                let a = f32::from(px[3]) / 255.0 * opacity;
                *out = f32::from(px[0]) / 255.0 * a + *out * (1.0 - a);
            }
        }
        SampleType::U16 => {
            for (out, px) in dst.iter_mut().zip(bytes.as_chunks::<8>().0.iter()) {
                let value = f32::from(u16::from_ne_bytes([px[0], px[1]])) / 65535.0;
                let alpha = f32::from(u16::from_ne_bytes([px[6], px[7]])) / 65535.0 * opacity;
                *out = value * alpha + *out * (1.0 - alpha);
            }
        }
        SampleType::F32 => {
            for (out, px) in dst.iter_mut().zip(bytes.as_chunks::<16>().0.iter()) {
                let value = f32::from_ne_bytes([px[0], px[1], px[2], px[3]]);
                let alpha = f32::from_ne_bytes([px[12], px[13], px[14], px[15]]) * opacity;
                *out = value * alpha + *out * (1.0 - alpha);
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
    use photocraft_geom::Rect;
    use photocraft_raster::Surface;

    fn layer(kind: EffectKind, opacity: f32) -> Paint {
        Paint { name: "effect".into(), visible: true, opacity, surface: Surface::new(PixelFormat::RGBA8), fill_map: None, fill_map_size: [0,0], fill_bounds: None, kind }
    }

    #[test]
    fn many_tile_opacity_refresh_preserves_untouched_cow_tiles() {
        let gray = PixelFormat::GRAY8;
        let rgba = PixelFormat::RGBA8;
        let mut mask = LayerMask::hide_all();
        mask.surface = Surface::with_default(gray, &[0.0]);
        let base = Surface::with_default(gray, &[0.2]);
        let mut surface = Surface::with_default(rgba, &[0.7, 0.1, 0.05, 0.0]);
        for x in 0..40 {
            let x = x * photocraft_geom::TILE_SIZE;
            surface.write_region(Rect::new(x, 0, x + 1, 1), &[0.9, 0.2, 0.1, 0.75]);
        }
        mask.paint = Some(Stack {
            base: base.clone(),
            layers: vec![Paint { name: "many tiles".into(), visible: true, opacity: 1.0, surface: surface.clone(), fill_map: None, fill_map_size: [0,0], fill_bounds: None, kind: EffectKind::Paint }],
            active: 0,
        });
        refresh(&mut mask, None);
        let distant = photocraft_geom::TileCoord::new(50, 0);
        mask.surface.write_region(Rect::new(50 * photocraft_geom::TILE_SIZE, 0, 50 * photocraft_geom::TILE_SIZE + 1, 1), &[0.4]);
        let untouched = mask.surface.tile(distant).cloned();
        let old = mask.paint.as_ref().cloned();
        if let Some(stack) = &mut mask.paint {
            stack.layers[0].opacity = 0.25;
        }
        refresh_changes(&mut mask, old.as_ref());
        assert_eq!(mask.surface.default_pixel(), Surface::with_default(gray, &[0.2]).default_pixel());
        let tile = photocraft_geom::TileCoord::new(0, 0);
        let expected = surface.read_region(Rect::new(0, 0, 1, 1));
        let base_value = base.read_region(Rect::new(0, 0, 1, 1))[0];
        assert_eq!(
            mask.surface.read_region(Rect::new(0, 0, 1, 1)),
            Surface::with_default(gray, &[expected[0] * expected[3] * 0.25 + base_value * (1.0 - expected[3] * 0.25)]).default_pixel()
        );
        assert!(std::sync::Arc::ptr_eq(untouched.as_ref().expect("test tile"), mask.surface.tile(distant).expect("preserved tile")));
        assert!(mask.surface.tile(tile).is_some());
    }

    #[test]
    fn rgba_tile_kernel_matches_region_fallback_for_each_sample_type() {
        use photocraft_geom::TileCoord;
        for sample in [photocraft_color::SampleType::U8, photocraft_color::SampleType::U16, photocraft_color::SampleType::F32] {
            let gray = photocraft_color::PixelFormat::new(photocraft_color::ColorMode::Grayscale, sample, false);
            let rgba = photocraft_color::PixelFormat::new(photocraft_color::ColorMode::Rgb, sample, true);
            let mut base = Surface::with_default(gray, &[0.23]);
            let mut surface = Surface::with_default(rgba, &[0.72, 0.13, 0.04, 0.0]);
            let rect = Rect::new(7, 9, 19, 17);
            let mut src = Vec::new();
            let mut dst = Vec::new();
            for y in rect.y0..rect.y1 {
                for x in rect.x0..rect.x1 {
                    let v = ((x * 13 + y * 7) % 31) as f32 / 31.0;
                    src.extend_from_slice(&[0.1 + v * 0.8, 0.05 + v * 0.4, 0.02 + v * 0.2, 0.15 + v * 0.75]);
                    dst.push(0.1 + v * 0.7);
                }
            }
            surface.write_region(rect, &src);
            base.write_region(rect, &dst);
            let mut optimized = LayerMask::hide_all();
            optimized.surface = Surface::with_default(gray, &[0.0]);
            optimized.paint =
                Some(Stack { base, layers: vec![Paint { name: "source".into(), visible: true, opacity: 0.73, surface, fill_map: None, fill_map_size: [0,0], fill_bounds: None, kind: EffectKind::Paint }], active: 0 });
            let mut oracle = optimized.clone();
            refresh(&mut optimized, None);
            refresh(&mut oracle, Some(TileCoord::new(0, 0).rect()));
            assert_eq!(
                optimized.surface.tile(TileCoord::new(0, 0)).map(|t| t.bytes()),
                oracle.surface.tile(TileCoord::new(0, 0)).map(|t| t.bytes()),
                "sample {sample:?}"
            );
        }
    }

    #[test]
    fn rgba_tile_kernel_matches_defaults_and_negative_coordinates() {
        use photocraft_geom::TileCoord;
        let gray = PixelFormat::GRAY8;
        let rgba = PixelFormat::RGBA8;
        let mut base = Surface::with_default(gray, &[0.3]);
        let mut surface = Surface::with_default(rgba, &[0.8, 0.2, 0.1, 0.4]);
        let painted = Rect::new(-255, 2, -254, 3);
        base.write_region(painted, &[0.75]);
        surface.write_region(painted, &[0.1, 0.7, 0.2, 0.9]);
        base.write_region(Rect::new(256, 0, 257, 1), &[0.6]);
        let mut optimized = LayerMask::hide_all();
        optimized.surface = Surface::with_default(gray, &[0.0]);
        optimized.paint = Some(Stack {
            base,
            layers: vec![Paint { name: "negative source".into(), visible: true, opacity: 0.65, surface, fill_map: None, fill_map_size: [0,0], fill_bounds: None, kind: EffectKind::Paint }],
            active: 0,
        });
        let mut oracle = optimized.clone();
        refresh(&mut optimized, None);
        refresh(&mut oracle, Some(TileCoord::new(-1, 0).rect()));
        refresh(&mut oracle, Some(TileCoord::new(1, 0).rect()));
        assert_eq!(optimized.surface.default_pixel(), oracle.surface.default_pixel());
        for coord in [TileCoord::new(-1, 0), TileCoord::new(1, 0)] {
            assert_eq!(optimized.surface.tile(coord).map(|tile| tile.bytes()), oracle.surface.tile(coord).map(|tile| tile.bytes()), "tile {coord:?}");
        }
    }

    #[test]
    fn changed_nonzero_paint_default_recomposes_sparse_tiles() {
        let gray = PixelFormat::GRAY8;
        let rgba = PixelFormat::RGBA8;
        let base = Surface::with_default(gray, &[0.2]);
        let mut surface = Surface::with_default(rgba, &[0.8, 0.2, 0.1, 0.4]);
        surface.write_region(Rect::new(1, 1, 2, 2), &[0.1, 0.2, 0.3, 0.9]);
        let mut mask = LayerMask::hide_all();
        mask.surface = Surface::with_default(gray, &[0.0]);
        mask.paint = Some(Stack {
            base,
            layers: vec![Paint { name: "default".into(), visible: true, opacity: 0.5, surface: surface.clone(), fill_map: None, fill_map_size: [0,0], fill_bounds: None, kind: EffectKind::Paint }],
            active: 0,
        });
        refresh(&mut mask, None);
        let old = mask.paint.as_ref().cloned();
        if let Some(stack) = &mut mask.paint {
            stack.layers[0].surface.set_default_pixel(&[0.3, 0.2, 0.1, 0.6]);
        }
        refresh_changes(&mut mask, old.as_ref());
        assert!((mask.surface.default_pixel()[0] - (0.3 * 0.6 * 0.5 + 0.2 * (1.0 - 0.6 * 0.5))).abs() < 0.02);
    }

    #[test]
    fn fill_and_levels_compose_in_order_and_respect_opacity() {
        let mut mask = LayerMask::hide_all();
        let base = Surface::with_default(PixelFormat::GRAY8, &[0.25]);
        mask.surface = base.clone();
        mask.paint = Some(Stack {
            base,
            layers: vec![
                layer(EffectKind::Fill { value: 0.75 }, 0.5),
                layer(EffectKind::Levels { input_black: 0.0, gamma: 1.0, input_white: 1.0, output_black: 0.0, output_white: 1.0 }, 0.5),
            ],
            active: 0,
        });
        refresh(&mut mask, None);
        // Fill: .25 -> .5; Levels identity at 50% opacity remains .5.
        assert!((mask.surface.read_region(Rect::new(0, 0, 1, 1))[0] - 0.5).abs() < 0.01);
    }

    #[test]
    fn levels_adjustment_is_non_destructive_and_applies_at_stack_position() {
        let mut mask = LayerMask::hide_all();
        let gray16 = PixelFormat::new(photocraft_color::ColorMode::Grayscale, photocraft_color::SampleType::U16, false);
        let base = Surface::with_default(gray16, &[0.25]);
        mask.surface = base.clone();
        mask.paint = Some(Stack {
            base,
            layers: vec![layer(EffectKind::Levels { input_black: 0.0, gamma: 1.0, input_white: 0.5, output_black: 0.0, output_white: 1.0 }, 1.0)],
            active: 0,
        });
        refresh(&mut mask, None);
        assert!((mask.surface.read_region(Rect::new(0, 0, 1, 1))[0] - 0.5).abs() < 0.001);
        let stored_base = mask.paint.as_ref().map(|s| s.base.default_pixel()[0]);
        assert!((stored_base.unwrap_or_default() - 0.25).abs() < 0.00002);
    }
}
