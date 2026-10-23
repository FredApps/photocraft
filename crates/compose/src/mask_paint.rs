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
    if stack.layers.len() != old.layers.len()
        || stack.base != old.base
        || stack
            .layers
            .iter()
            .zip(&old.layers)
            .any(|(a, b)| a.visible != b.visible || a.opacity != b.opacity || a.surface.default_pixel() != b.surface.default_pixel())
    {
        refresh(mask, None);
        return;
    }
    let mut changed = std::collections::BTreeSet::new();
    for (a, b) in stack.layers.iter().zip(&old.layers) {
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
    for coord in changed {
        refresh(mask, Some(coord.rect()));
    }
}
/// Recompose only the damaged rectangle, in bounded scanline bands.
pub fn refresh(mask: &mut LayerMask, region: Option<Rect>) {
    let Some(stack) = &mask.paint else { return };
    let Some(bounds) = region else {
        // Sparse strokes can be far apart: never scan the empty atlas between them.
        let mut tiles = std::collections::BTreeSet::new();
        tiles.extend(mask.surface.tiles().map(|(c, _)| *c));
        tiles.extend(stack.base.tiles().map(|(c, _)| *c));
        for paint in &stack.layers {
            tiles.extend(paint.surface.tiles().map(|(c, _)| *c));
        }
        for tile in tiles {
            refresh(mask, Some(tile.rect()));
        }
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
