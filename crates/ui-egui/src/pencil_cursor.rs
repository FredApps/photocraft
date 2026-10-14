//! The Pencil's cursor, and the Pencil- and Block-mode Eraser's: the outline of the whole pixels
//! one dab at the pointer fills, on the pixel grid (#5262). From 4 px up a round aliased dab is a
//! stair-stepped disc, which the smooth circle that stood for it cut through.

use std::sync::Arc;

use egui::{CursorIcon, Painter, Pos2, Shape, Stroke, pos2, vec2};
use photocraft_paint::brush::{BrushSettings, TipShape};
use photocraft_paint::{grid_center, grid_square};

use crate::canvas::ViewXform;

/// Closed rings of pixel corners, as `photocraft_paint::outline::aliased_outline` traces them.
type Rings = Vec<Vec<[i32; 2]>>;

#[derive(Clone)]
struct Cached {
    key: Key,
    rings: Option<Arc<Rings>>,
}

/// The brush fields an aliased outline depends on.
#[derive(Clone)]
struct Key {
    tip: TipShape,
    size: f32,
    angle: f32,
    roundness: f32,
    flip_x: bool,
    flip_y: bool,
    square: bool,
}

impl Key {
    fn of(b: &BrushSettings) -> Self {
        Self { tip: b.tip.clone(), size: b.size, angle: b.angle, roundness: b.roundness, flip_x: b.flip_x, flip_y: b.flip_y, square: b.square }
    }
    fn matches(&self, b: &BrushSettings) -> bool {
        self.tip == b.tip
            && self.size == b.size
            && self.angle == b.angle
            && self.roundness == b.roundness
            && self.flip_x == b.flip_x
            && self.flip_y == b.flip_y
            && self.square == b.square
    }
}

/// The outline's two lines on screen, through the corners of each ring: the dark one half a
/// physical pixel outside the filled pixels, the light one half a pixel inside.
struct Lines {
    dark: Vec<Vec<Pos2>>,
    light: Vec<Vec<Pos2>>,
    /// The rings' larger side on screen, in points.
    size: f32,
}

/// The Pencil's cursor with the pointer at `at` (screen): the outline of the pixels a dab there
/// fills where [`draw`] can show them, else the tip's smooth outline. `block` is the Block-mode
/// Eraser's square in document pixels; `centre` is Preferences › Cursors › Show Crosshair in
/// Brush Tip.
pub(crate) fn cursor(painter: &Painter, xf: &ViewXform, at: Pos2, brush: &BrushSettings, block: Option<f32>, centre: bool) -> CursorIcon {
    if let Some(size) = draw(painter, xf, xf.to_doc(at), brush, block) {
        // Too small to see where it is: the hotspot as well.
        if centre || size < 6.0 {
            crate::tool_cursor::crosshair(painter, at, 4.0, 0.0);
        }
        return CursorIcon::None;
    }
    // Below 100 % a document pixel is smaller than a physical one, so the tip's smooth outline
    // is as exact: all of a round tip, or a sampled tip's 50 % contour, where the aliased dab
    // cuts them. A tip too big to trace is off by under a pixel at its edge.
    let radius = (brush.size / 2.0 * xf.zoom).max(1.0);
    let axes = crate::brush_cursor::tip_axes(xf, brush.size);
    crate::brush_cursor::cursor(painter, at, brush, radius, axes, !matches!(brush.tip, TipShape::Round), centre)
}

/// Draws the outline of the pixels a dab centred at document point `at` fills: `brush`'s, or with
/// `block` the Block-mode Eraser's square of that many pixels (16 screen pixels at any zoom,
/// #2770). Returns its larger side on screen; `None`, drawing nothing, below 100 % (a document
/// pixel smaller than a physical one), for a tip too big or ragged to trace, or off the map.
fn draw(painter: &Painter, xf: &ViewXform, at: [f64; 2], brush: &BrushSettings, block: Option<f32>) -> Option<f32> {
    let ppp = painter.ctx().pixels_per_point();
    let ppp = if ppp.is_finite() && ppp > 0.0 { ppp } else { 1.0 };
    let rings: Vec<Vec<[f32; 2]>> = match block {
        Some(size) => {
            let [x0, y0, x1, y1] = grid_square(at[0], at[1], size).map(|v| v as f32);
            vec![vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]]
        }
        None => {
            if xf.zoom.min(xf.zoom_x()) * ppp < 1.0 {
                return None;
            }
            // Traced once per brush for a dab at the origin: one at `at` fills the same pixels
            // moved by the whole pixels between their grid centres.
            let rings = outline(painter.ctx(), brush)?;
            let (cx, cy) = grid_center(at[0], at[1], brush.size);
            let (ox, oy) = grid_center(0.0, 0.0, brush.size);
            let (dx, dy) = (cx - ox, cy - oy);
            rings.iter().map(|r| r.iter().map(|&[x, y]| [x as f32 + dx, y as f32 + dy]).collect()).collect()
        }
    };
    let lines = screen_lines(xf, &rings, ppp)?;
    let [dark, light] = crate::theme::Tokens::cursor_outline();
    let px = 1.0 / ppp;
    // The dark lines first: where two filled pixels touch at a corner, the light ones stay on top.
    painter.extend(lines.dark.into_iter().map(|l| Shape::closed_line(l, Stroke::new(px, dark))));
    painter.extend(lines.light.into_iter().map(|l| Shape::closed_line(l, Stroke::new(px, light))));
    Some(lines.size)
}

/// `aliased_outline` of `brush`, kept across frames: it changes only with the brush, and a sampled
/// tip builds its mipmaps to rasterise.
fn outline(ctx: &egui::Context, brush: &BrushSettings) -> Option<Arc<Rings>> {
    let id = egui::Id::new("photocraft-pencil-outline");
    if let Some(c) = ctx.data(|d| d.get_temp::<Cached>(id))
        && c.key.matches(brush)
    {
        return c.rings;
    }
    let rings = photocraft_paint::outline::aliased_outline(brush).map(Arc::new);
    ctx.data_mut(|d| d.insert_temp(id, Cached { key: Key::of(brush), rings: rings.clone() }));
    rings
}

/// Where the outline of `rings` (pixel corners in document pixels, the filled pixels on the right
/// of each edge) goes on screen, as the Pencil's square always went: each corner moves half a
/// physical pixel out of the pixels along both its edges for the dark line, and half a pixel in
/// for the light one, so 1-pixel strokes hug the pixels from either side. On a view that isn't
/// turned the corners first snap to physical pixels (`ppp` per point), for crisp lines. `None`
/// when a corner doesn't land on a finite point.
fn screen_lines(xf: &ViewXform, rings: &[Vec<[f32; 2]>], ppp: f32) -> Option<Lines> {
    let aligned = xf.rotation.rem_euclid(90.0) == 0.0;
    let snap = |p: Pos2| if aligned { pos2((p.x * ppp).round() / ppp, (p.y * ppp).round() / ppp) } else { p };
    // Half a physical pixel out of the edge from `a` to `b`: its filled pixels are on its right,
    // so out is to its left, turned, flipped and stretched with the view.
    let out = |a: [f32; 2], b: [f32; 2]| {
        let sign = |v: f32| f32::from(v > 0.0) - f32::from(v < 0.0);
        xf.map_vec(vec2(sign(b[1] - a[1]), -sign(b[0] - a[0]))).normalized() * (0.5 / ppp)
    };
    let (mut dark, mut light) = (Vec::with_capacity(rings.len()), Vec::with_capacity(rings.len()));
    let (mut min, mut max) = (pos2(f32::INFINITY, f32::INFINITY), pos2(f32::NEG_INFINITY, f32::NEG_INFINITY));
    for ring in rings {
        let n = ring.len();
        let (mut outer, mut inner) = (Vec::with_capacity(n), Vec::with_capacity(n));
        for (i, &c) in ring.iter().enumerate() {
            let (Some(&prev), Some(&next)) = (ring.get((i + n - 1) % n), ring.get((i + 1) % n)) else { continue };
            let p = snap(xf.to_screen(c[0], c[1]));
            let o = out(prev, c) + out(c, next);
            (min, max) = (min.min(p), max.max(p));
            outer.push(p + o);
            inner.push(p - o);
        }
        if !outer.iter().chain(&inner).all(|p| p.is_finite()) {
            return None;
        }
        dark.push(outer);
        light.push(inner);
    }
    let size = (max - min).max_elem();
    size.is_finite().then_some(Lines { dark, light, size })
}

#[cfg(test)]
mod tests {
    use egui::{Rect, Vec2};

    use super::*;

    fn view(zoom: f32, rotation: f32, flip: bool) -> ViewXform {
        ViewXform { rect: Rect::from_min_size(pos2(0.3, 0.7), egui::vec2(800.0, 600.0)), zoom, center: [50.2, 37.9], flip, rotation, aspect: 1.0 }
    }

    /// A square of pixels is drawn where the old square cursor was (`rect_stroke` outside and
    /// inside): a dark physical pixel just outside its edges and a light one just inside, with its
    /// corners on physical pixels at any zoom and density. A turned view turns the outline with it.
    #[test]
    fn the_outline_hugs_the_pixels_on_physical_pixels_and_turns_with_the_view() {
        let square = vec![vec![[8.0, 4.0], [12.0, 4.0], [12.0, 8.0], [8.0, 8.0]]];
        for (zoom, ppp) in [(8.0, 1.0), (3.3, 2.0), (1.0, 1.0), (13.7, 1.5)] {
            let xf = view(zoom, 0.0, false);
            let lines = screen_lines(&xf, &square, ppp).unwrap();
            let snap = |v: f32| (v * ppp).round() / ppp;
            let (a, b) = (xf.to_screen(8.0, 4.0), xf.to_screen(12.0, 8.0));
            let (x0, y0, x1, y1, h) = (snap(a.x), snap(a.y), snap(b.x), snap(b.y), 0.5 / ppp);
            assert_eq!(lines.dark, [[pos2(x0 - h, y0 - h), pos2(x1 + h, y0 - h), pos2(x1 + h, y1 + h), pos2(x0 - h, y1 + h)]], "zoom {zoom}, ppp {ppp}");
            assert_eq!(lines.light, [[pos2(x0 + h, y0 + h), pos2(x1 - h, y0 + h), pos2(x1 - h, y1 - h), pos2(x0 + h, y1 - h)]], "zoom {zoom}, ppp {ppp}");
            assert!(((x1 - x0) - 4.0 * zoom).abs() <= 1.0 / ppp + 1e-3, "zoom {zoom}, ppp {ppp}");
            assert_eq!(lines.size, (x1 - x0).max(y1 - y0));
        }
        // Turned 30° and flipped: each corner of the dark line is half a pixel out along both
        // edges (√½ pixel diagonally), outside the square; the light line's inside it.
        let xf = view(8.0, 30.0, true);
        let lines = screen_lines(&xf, &square, 1.0).unwrap();
        let centre = xf.to_screen(10.0, 6.0);
        for (i, [x, y]) in square[0].iter().copied().enumerate() {
            let p = xf.to_screen(x, y);
            let (d, l): (Vec2, Vec2) = (lines.dark[0][i] - p, lines.light[0][i] - p);
            assert!((d.length() - 0.5f32.hypot(0.5)).abs() < 1e-4 && (d + l).length() < 1e-4, "corner {i}: {d:?} {l:?}");
            assert!((lines.dark[0][i] - centre).length() > (p - centre).length(), "corner {i}");
        }
        // Hostile views and corners draw nothing.
        for xf in [view(f32::NAN, 0.0, false), view(f32::INFINITY, 0.0, false), view(8.0, f32::NAN, false)] {
            assert!(screen_lines(&xf, &square, 1.0).is_none());
        }
        assert!(screen_lines(&view(8.0, 0.0, false), &[vec![[f32::NAN, 4.0], [12.0, 4.0], [12.0, 8.0], [8.0, 8.0]]], 1.0).is_none());
        assert!(screen_lines(&view(8.0, 0.0, false), &[], 1.0).is_none());
    }

    /// From 100 % up the outline is drawn (the Block square at any zoom); below it, for a tip
    /// too big to trace or a pointer off the map the caller keeps the smooth outline. The traced
    /// outline is kept until the brush changes.
    #[test]
    fn below_100_percent_or_for_huge_tips_the_smooth_outline_stays() {
        let ctx = egui::Context::default();
        let brush = BrushSettings { size: 9.0, ..Default::default() };
        let huge = BrushSettings { size: 5000.0, ..Default::default() };
        let mut drawn = Vec::new();
        let mut output = ctx.run_ui(Default::default(), |ui| {
            let p = ui.painter();
            for zoom in [0.5, 0.99, 1.0, 8.0] {
                drawn.push(draw(p, &view(zoom, 0.0, false), [20.3, 15.6], &brush, None).is_some());
            }
            drawn.push(draw(p, &view(0.25, 0.0, false), [20.3, 15.6], &brush, Some(64.0)).is_some());
            drawn.push(draw(p, &view(8.0, 0.0, false), [20.3, 15.6], &huge, None).is_some());
            drawn.push(draw(p, &view(8.0, 0.0, false), [f64::NAN, 15.6], &brush, None).is_some());
        });
        // No renderer takes the font atlas here.
        output.textures_delta.clear();
        assert_eq!(drawn, [false, false, true, true, true, false, false]);
        let first = outline(&ctx, &brush).unwrap();
        assert!(Arc::ptr_eq(&first, &outline(&ctx, &brush).unwrap()));
        assert!(!Arc::ptr_eq(&first, &outline(&ctx, &BrushSettings { size: 10.0, ..brush }).unwrap()));
    }
}
