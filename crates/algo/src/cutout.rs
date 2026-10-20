//! Cutout (Filter Gallery › Artistic): the image as flat sheets of cut paper.
//!
//! The image is traced into shapes, and the shapes are filled back in:
//!
//! 1. **Smooth** within tones, edges kept: a bilateral filter (C. Tomasi and R. Manduchi,
//!    "Bilateral Filtering for Gray and Color Images", 1998) in Oklab, so a photo's noise doesn't
//!    decide pixel by pixel which of two colours a pixel gets.
//! 2. **Palette**: median cut (Heckbert 1982) then k-means over the smoothed colours, Number of
//!    Levels tones of the picture's own; a tone made of two clearly different hues splits in two,
//!    and colours only noise apart are merged.
//! 3. **Regions**: 4-connected areas of one palette colour. Regions smaller than the Edge
//!    Simplicity area, or thin slivers (the blends along an edge between two other colours), are
//!    merged smallest first into the neighbour they share most border with. Each region is filled
//!    with the mean colour of its own pixels.
//! 4. **Edges**: the cracks between regions are linked into chains from junction to junction
//!    (where three regions meet, or the canvas edge). Each crack is moved along its normal to where
//!    the pixels on either side say the edge is (an anti-aliased pixel is part one colour, part the
//!    other), so outlines are right to a fraction of a pixel. A chain is fitted once, so two
//!    neighbouring shapes share one and the same edge.
//! 5. **Curves**: turns sharper than Edge Fidelity allows are kept as corners (squared back where
//!    the pixels rounded them); the runs between are simplified to the Edge Simplicity tolerance
//!    (Douglas–Peucker) and drawn as Catmull-Rom curves.
//! 6. **Fill**: each region is scan-converted with exact area coverage, so edges are anti-aliased
//!    and the coverages of shapes that share an edge sum to one: no seams.
//!
//! Tracing reads the whole image and runs once; filling runs per tile and gives the same pixels
//! whatever the tiling.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::hash::{BuildHasherDefault, Hasher};

use photocraft_geom::Rect;

use crate::par;
use crate::quantize::{kmeans, median_cut};

type Px = [f32; 3];
type Pt = [f32; 2];

/// Region id of the area outside the image.
const OUT: u32 = u32::MAX;

/// The filter's three sliders.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Settings {
    /// Number of Levels (2–8): the most colours the palette has.
    pub levels: usize,
    /// Edge Simplicity (0–10): smaller shapes merge and outlines simplify more.
    pub simplicity: f32,
    /// Edge Fidelity (1–3): more corners are kept.
    pub fidelity: f32,
}

impl Settings {
    /// Regions with fewer pixels merge into a neighbour.
    fn min_area(&self) -> u32 {
        (1.0 + self.simplicity.clamp(0.0, 10.0)).powi(2).round() as u32
    }
    /// Regions thinner than this (twice the area over the perimeter, in pixels) merge too.
    fn sliver(&self) -> f32 {
        0.6 + 0.1 * self.simplicity.clamp(0.0, 10.0)
    }
    /// Douglas–Peucker tolerance of the outlines, in pixels: looser with less Edge Fidelity
    /// (twice as loose at 1, half at 3).
    fn tolerance(&self) -> f32 {
        (0.5 + 0.12 * self.simplicity.clamp(0.0, 10.0)) * 2f32.powf(2.0 - self.fidelity.clamp(1.0, 3.0))
    }
    /// Below Edge Fidelity 2 the outline is rounded off before it is fitted: points averaged with
    /// this many neighbours either side (3 at fidelity 1).
    fn rounding(&self) -> usize {
        ((2.0 - self.fidelity.clamp(1.0, 3.0)) * 3.0).round().max(0.0) as usize
    }
    /// Turns sharper than this stay corners.
    fn corner_turn(&self) -> f32 {
        (100.0 - 22.5 * self.fidelity.clamp(1.0, 3.0)).to_radians()
    }
    /// Rounds of edge-preserving smoothing.
    fn smooth_rounds(&self) -> usize {
        2 + (self.simplicity.clamp(0.0, 10.0) / 4.0) as usize
    }
}

/// A traced image: the shapes and their colours, ready to fill any tile.
pub(crate) struct Traced {
    /// Document position of the traced window's top-left pixel; outlines are relative to it.
    x0: i32,
    y0: i32,
    /// Fitted edges (polylines), each shared by the one or two regions it separates.
    chains: Vec<Vec<Pt>>,
    /// Per chain, its bounding box `[x0, y0, x1, y1]`.
    chain_boxes: Vec<[f32; 4]>,
    regions: Vec<Region>,
    /// Width of the traced window, each pixel's region, and its paper colour (the region's
    /// lightness with the hue of its own pixels nearby).
    width: usize,
    reg: Vec<u32>,
    field: Vec<Px>,
}

struct Region {
    colour: Px,
    bbox: [f32; 4],
    /// The chains around the region, reversed where the region is on their right.
    edges: Vec<(u32, bool)>,
}

/// Traces `c` (sRGB colour of every pixel of `r`, row-major).
pub(crate) fn trace(c: &[Px], r: Rect, s: &Settings) -> Traced {
    let (w, h) = (r.width() as usize, r.height() as usize);
    let empty = Traced { x0: r.x0, y0: r.y0, chains: Vec::new(), chain_boxes: Vec::new(), regions: Vec::new(), width: 0, reg: Vec::new(), field: Vec::new() };
    if w == 0 || h == 0 || c.len() != w * h {
        return empty;
    }
    let mut lab = vec![[0.0f32; 3]; w * h];
    par::chunks_mut(&mut lab, w, true, |y, row| {
        for (o, p) in row.iter_mut().zip(&c[y * w..(y + 1) * w]) {
            *o = oklab(*p);
        }
    });
    let original = lab.clone();
    let lab = smooth(lab, w, h, s.smooth_rounds());
    let palette = palette(&lab, s.levels.clamp(2, 8));
    let mut labels = vec![0u8; w * h];
    par::chunks_mut(&mut labels, w, true, |y, row| {
        for (o, p) in row.iter_mut().zip(&lab[y * w..(y + 1) * w]) {
            *o = nearest(&palette, *p);
        }
    });
    let (comp, n) = components(&labels, w, h);
    let (reg, colours) = merge_regions(&comp, n, c, w, h, s);
    let (chains, sides) = follow_edges(&reg, &colours, c, w, h);
    let chains: Vec<Vec<Pt>> = par::map(chains.len(), true, |i| fit(&chains[i].0, chains[i].1, s));
    let chain_boxes: Vec<[f32; 4]> = chains.iter().map(|ch| bounds(ch)).collect();
    let paint = paper_colours(&reg, colours.len(), &original);
    let mut regions: Vec<Region> =
        paint.into_iter().map(|colour| Region { colour, bbox: [f32::MAX, f32::MAX, f32::MIN, f32::MIN], edges: Vec::new() }).collect();
    for (k, &(left, right)) in sides.iter().enumerate() {
        for (side, rev) in [(left, false), (right, true)] {
            if let Some(g) = regions.get_mut(side as usize) {
                g.edges.push((k as u32, rev));
                let b = chain_boxes[k];
                g.bbox = [g.bbox[0].min(b[0]), g.bbox[1].min(b[1]), g.bbox[2].max(b[2]), g.bbox[3].max(b[3])];
            }
        }
    }
    let field = local_paper(&reg, &regions, &original, w, h, 3.0 + 1.2 * s.simplicity.clamp(0.0, 10.0));
    Traced { x0: r.x0, y0: r.y0, chains, chain_boxes, regions, width: w, reg, field }
}

/// Each pixel's paper colour: its region's lightness (flat across the shape) with the average
/// hue, at the average strength, of the region's own pixels within about `reach` pixels. Colour
/// can drift gently across a large shape (pink cheeks on a cream face) but never crosses a cut.
fn local_paper(reg: &[u32], regions: &[Region], lab: &[Px], w: usize, h: usize, reach: f32) -> Vec<Px> {
    let mut v: Vec<Px> = lab.iter().map(|p| [p[1], p[2], (p[1] * p[1] + p[2] * p[2]).sqrt()]).collect();
    let regt = transpose_u32(reg, w, h);
    let a = (-1.0 / reach.max(0.5)).exp();
    for _ in 0..2 {
        same_region_rows(&mut v, reg, w, a);
        let mut t = transpose(&v, w, h);
        same_region_rows(&mut t, &regt, h, a);
        v = transpose(&t, h, w);
    }
    let light: Vec<f32> = regions.iter().map(|g| oklab(g.colour)[0]).collect();
    let mut out = vec![[0.0f32; 3]; w * h];
    par::chunks_mut(&mut out, w, true, |y, row| {
        for (x, o) in row.iter_mut().enumerate() {
            let i = y * w + x;
            let (q, l) = (v[i], light.get(reg[i] as usize).copied().unwrap_or(0.5));
            let ab = (q[0] * q[0] + q[1] * q[1]).sqrt();
            let k = if ab > 1e-5 { q[2] / ab } else { 1.0 };
            *o = in_gamut(l, q[0] * k, q[1] * k);
        }
    });
    out
}

/// Exponential smoothing along each row (both ways) that restarts at every change of region.
fn same_region_rows(v: &mut [Px], reg: &[u32], w: usize, a: f32) {
    let mut chunks: Vec<(&mut [Px], &[u32])> = v.chunks_mut(w).zip(reg.chunks(w)).collect();
    let f = |(row, rg): &mut (&mut [Px], &[u32])| {
        let n = row.len();
        let src = row.to_vec();
        let mut fwd = vec![([0.0f32; 3], 0.0f32); n];
        for x in 0..n {
            let (p, d) = if x > 0 && rg[x] == rg[x - 1] { fwd[x - 1] } else { ([0.0; 3], 0.0) };
            fwd[x] = ([src[x][0] + a * p[0], src[x][1] + a * p[1], src[x][2] + a * p[2]], 1.0 + a * d);
        }
        let mut back = ([0.0f32; 3], 0.0f32);
        for x in (0..n).rev() {
            if x + 1 < n && rg[x] != rg[x + 1] {
                back = ([0.0; 3], 0.0);
            }
            back = ([src[x][0] + a * back.0[0], src[x][1] + a * back.0[1], src[x][2] + a * back.0[2]], 1.0 + a * back.1);
            let den = fwd[x].1 + back.1 - 1.0;
            row[x] = std::array::from_fn(|c| (fwd[x].0[c] + back.0[c] - src[x][c]) / den);
        }
    };
    #[cfg(not(target_arch = "wasm32"))]
    {
        use rayon::prelude::*;
        chunks.par_iter_mut().for_each(f);
    }
    #[cfg(target_arch = "wasm32")]
    chunks.iter_mut().for_each(f);
}

fn transpose_u32(buf: &[u32], w: usize, h: usize) -> Vec<u32> {
    let mut out = vec![0u32; w * h];
    for (x, col) in out.chunks_mut(h).enumerate() {
        for (y, p) in col.iter_mut().enumerate() {
            *p = buf[y * w + x];
        }
    }
    out
}

/// sRGB at Oklab lightness `l`, hue/strength `a`, `b`; out of gamut it loses strength, not hue.
fn in_gamut(l: f32, a: f32, b: f32) -> Px {
    if let Some(c) = srgb([l, a, b]) {
        return c;
    }
    let (mut ok, mut bad) = (0.0f32, 1.0f32);
    for _ in 0..10 {
        let t = (ok + bad) / 2.0;
        if srgb([l, a * t, b * t]).is_some() { ok = t } else { bad = t }
    }
    srgb([l, a * ok, b * ok]).unwrap_or([l, l, l])
}

impl Traced {
    /// The cut paper over `r` (document coordinates): sRGB colour per pixel, row-major.
    pub(crate) fn render(&self, r: Rect) -> Vec<Px> {
        let (w, h) = (r.width() as usize, r.height() as usize);
        let mut col = vec![[0.0f32; 3]; w * h];
        if w == 0 || h == 0 {
            return col;
        }
        let mut weight = vec![0.0f32; w * h];
        let stride = w + 2;
        let mut acc = vec![0.0f32; stride * h];
        let (ox, oy) = ((self.x0 - r.x0) as f32, (self.y0 - r.y0) as f32);
        let (wf, hf) = (w as f32, h as f32);
        let (dx, dy) = ((r.x0 - self.x0) as i64, (r.y0 - self.y0) as i64);
        let th = self.reg.len() / self.width.max(1);
        for (gi, g) in self.regions.iter().enumerate() {
            let b = [g.bbox[0] + ox, g.bbox[1] + oy, g.bbox[2] + ox, g.bbox[3] + oy];
            if b[2] <= 0.0 || b[0] >= wf || b[3] <= 0.0 || b[1] >= hf {
                continue;
            }
            for &(k, rev) in &g.edges {
                let (Some(pts), Some(cb)) = (self.chains.get(k as usize), self.chain_boxes.get(k as usize)) else { continue };
                // Chains left of the tile still count (they wind the rows they cross); chains to
                // its right or above or below it don't.
                if cb[0] + ox >= wf || cb[3] + oy <= 0.0 || cb[1] + oy >= hf {
                    continue;
                }
                for seg in pts.windows(2) {
                    let (a, z) = if rev { (seg[1], seg[0]) } else { (seg[0], seg[1]) };
                    line_clipped(&mut acc, stride, h, [a[0] + ox, a[1] + oy], [z[0] + ox, z[1] + oy]);
                }
            }
            let y0 = b[1].floor().max(0.0) as usize;
            let y1 = (b[3].ceil().max(0.0) as usize).min(h);
            let xe = ((b[2].ceil().max(0.0) as usize) + 1).min(w);
            for y in y0..y1 {
                let row = &mut acc[y * stride..(y + 1) * stride];
                let mut sum = 0.0f32;
                for (x, v) in row.iter().enumerate().take(xe) {
                    sum += v;
                    let cov = sum.abs().min(1.0);
                    if cov > 0.0 {
                        let i = y * w + x;
                        // Inside its own pixels a shape takes the local colour; where it reaches
                        // into a neighbour's edge pixel, its own flat colour.
                        let (tx, ty) = (x as i64 + dx, y as i64 + dy);
                        let t = (tx >= 0 && ty >= 0 && (tx as usize) < self.width && (ty as usize) < th).then(|| ty as usize * self.width + tx as usize);
                        let paint = match t {
                            Some(t) if self.reg[t] as usize == gi => self.field[t],
                            _ => g.colour,
                        };
                        col[i] = [col[i][0] + paint[0] * cov, col[i][1] + paint[1] * cov, col[i][2] + paint[2] * cov];
                        weight[i] += cov;
                    }
                }
                row.fill(0.0);
            }
        }
        for (p, &wt) in col.iter_mut().zip(&weight) {
            if wt > 1e-6 {
                *p = [p[0] / wt, p[1] / wt, p[2] / wt];
            }
        }
        col
    }
}

// ---------------------------------------------------------------- colour

/// Oklab (Björn Ottosson, 2020) of an sRGB colour.
fn oklab(p: Px) -> Px {
    let lin = |v: f32| {
        let v = v.clamp(0.0, 1.0);
        if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    let (r, g, b) = (lin(p[0]), lin(p[1]), lin(p[2]));
    let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

/// sRGB of an Oklab colour, or `None` outside the sRGB gamut.
fn srgb(lab: Px) -> Option<Px> {
    let l = (lab[0] + 0.396_337_78 * lab[1] + 0.215_803_76 * lab[2]).powi(3);
    let m = (lab[0] - 0.105_561_346 * lab[1] - 0.063_854_17 * lab[2]).powi(3);
    let s = (lab[0] - 0.089_484_18 * lab[1] - 1.291_485_5 * lab[2]).powi(3);
    let lin = [
        4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s,
        -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s,
        -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s,
    ];
    if lin.iter().any(|v| !(-1e-4..=1.0001).contains(v)) {
        return None;
    }
    let enc = |v: f32| {
        let v = v.clamp(0.0, 1.0);
        if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
    };
    Some(lin.map(enc))
}

/// The paper colour of every region. A plain average of a region's pixels greys it (a face of
/// pink and cream patches averages to beige, hues cancel), and a whole image of averages sits in
/// the middle tones. So in Oklab each region keeps its average hue at the average *strength* of
/// its pixels' colour, and lightness is stretched over the image's regions so the darkest paper is
/// near black and the lightest near white. Out-of-gamut colours lose chroma, not hue.
fn paper_colours(reg: &[u32], n: usize, lab: &[Px]) -> Vec<Px> {
    let mut acc = vec![[0.0f64; 5]; n];
    for (&r, p) in reg.iter().zip(lab) {
        if let Some(a) = acc.get_mut(r as usize) {
            let chroma = (p[1] * p[1] + p[2] * p[2]).sqrt();
            *a = [a[0] + f64::from(p[0]), a[1] + f64::from(p[1]), a[2] + f64::from(p[2]), a[3] + f64::from(chroma), a[4] + 1.0];
        }
    }
    let mean: Vec<(Px, f32, f64)> = acc
        .iter()
        .map(|a| {
            let k = a[4].max(1.0);
            ([(a[0] / k) as f32, (a[1] / k) as f32, (a[2] / k) as f32], (a[3] / k) as f32, a[4])
        })
        .collect();
    // The lightness range of the image's paper, by area (ignoring the extreme 1% at each end).
    let mut by_l: Vec<(f32, f64)> = mean.iter().map(|m| (m.0[0], m.2)).collect();
    by_l.sort_by(|a, b| a.0.total_cmp(&b.0));
    let total: f64 = by_l.iter().map(|m| m.1).sum();
    let at = |q: f64| {
        let mut run = 0.0;
        for &(l, a) in &by_l {
            run += a;
            if run >= q * total {
                return l;
            }
        }
        by_l.last().map_or(0.5, |m| m.0)
    };
    let (lo, hi) = (at(0.01), at(0.99));
    const STRETCH: f32 = 0.75;
    mean.iter()
        .map(|(m, chroma, _)| {
            let ab = (m[1] * m[1] + m[2] * m[2]).sqrt();
            let k = if ab > 1e-5 { chroma / ab } else { 1.0 };
            let stretched = if hi - lo > 0.05 { m[0] + (0.04 + (m[0] - lo) / (hi - lo) * 0.88 - m[0]) * STRETCH } else { m[0] };
            let stretched = stretched.clamp(0.0, 1.0);
            let colour = |l: f32, t: f32| srgb([l, m[1] * k * t, m[2] * k * t]);
            if let Some(c) = colour(stretched, 1.0) {
                return c;
            }
            // Colour comes before contrast: a strong colour the stretch took out of gamut (a
            // bright blue pushed towards white) moves back towards its own lightness first.
            let own = m[0].clamp(0.0, 1.0);
            if colour(own, 1.0).is_some() {
                let (mut ok, mut bad) = (own, stretched);
                for _ in 0..12 {
                    let l = (ok + bad) / 2.0;
                    if colour(l, 1.0).is_some() { ok = l } else { bad = l }
                }
                return colour(ok, 1.0).unwrap_or([ok, ok, ok]);
            }
            // Even its own lightness can't hold it: keep the stretch, lose chroma (not hue).
            let (mut ok, mut bad) = (0.0f32, 1.0f32);
            for _ in 0..12 {
                let t = (ok + bad) / 2.0;
                if colour(stretched, t).is_some() { ok = t } else { bad = t }
            }
            colour(stretched, ok).unwrap_or([stretched, stretched, stretched])
        })
        .collect()
}

fn dist2(a: Px, b: Px) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

/// Bilateral smoothing in Oklab, `rounds` times along rows then columns.
fn smooth(mut lab: Vec<Px>, w: usize, h: usize, rounds: usize) -> Vec<Px> {
    for _ in 0..rounds {
        bilateral_rows(&mut lab, w);
        let mut t = transpose(&lab, w, h);
        bilateral_rows(&mut t, h);
        lab = transpose(&t, h, w);
    }
    lab
}

fn bilateral_rows(buf: &mut [Px], w: usize) {
    const R: isize = 3;
    // Colours this far apart (Oklab) count half as much as equal ones (a Cauchy weight: as
    // edge-keeping as a Gaussian one and much cheaper).
    const HALF: f32 = 0.06;
    let k = 1.0 / (HALF * HALF);
    let spatial: [f32; 7] = std::array::from_fn(|i| (-((i as f32 - 3.0).powi(2)) / 8.0).exp());
    par::chunks_mut(buf, w, true, |_, row| {
        let src = row.to_vec();
        for (x, out) in row.iter_mut().enumerate() {
            let c = src[x];
            let (mut acc, mut wsum) = ([0.0f32; 3], 0.0f32);
            for d in -R..=R {
                let q = src[(x as isize + d).clamp(0, w as isize - 1) as usize];
                let wt = spatial[(d + R) as usize] / (1.0 + dist2(q, c) * k);
                acc = [acc[0] + q[0] * wt, acc[1] + q[1] * wt, acc[2] + q[2] * wt];
                wsum += wt;
            }
            *out = [acc[0] / wsum, acc[1] / wsum, acc[2] / wsum];
        }
    });
}

fn transpose(buf: &[Px], w: usize, h: usize) -> Vec<Px> {
    let mut out = vec![[0.0f32; 3]; w * h];
    par::chunks_mut(&mut out, h, true, |x, col| {
        for (y, p) in col.iter_mut().enumerate() {
            *p = buf[y * w + x];
        }
    });
    out
}

/// At most `levels` colours of the image's own; colours closer than noise are merged.
fn palette(lab: &[Px], levels: usize) -> Vec<Px> {
    const SAMPLES: usize = 1 << 16;
    const SAME: f32 = 0.045;
    // Hue centres (Oklab a, b) this far apart are different colours, not one colour's spread.
    const HUES_APART: f32 = 0.08;
    let step = lab.len().div_ceil(SAMPLES).max(1);
    let pts: Vec<(Px, u32)> = lab.iter().step_by(step).map(|p| (*p, 1)).collect();
    let mut centres = median_cut(&pts, levels);
    kmeans(&pts, &mut centres, 10);
    // Number of Levels counts tones: a palette colour whose pixels are two clearly different hues
    // of that tone (a blue and a pink) splits in two, or touching areas of both would be one
    // shape of their grey average.
    let mut split = Vec::with_capacity(centres.len() * 2);
    for (k, c) in centres.iter().enumerate() {
        let members: Vec<(Px, u32)> = pts.iter().filter(|(p, _)| nearest(&centres, *p) as usize == k).map(|(p, _)| ([0.0, p[1], p[2]], 1)).collect();
        let mut hues = median_cut(&members, 2);
        kmeans(&members, &mut hues, 6);
        match hues.as_slice() {
            [a, b] if dist2(*a, *b).sqrt() > HUES_APART => split.extend([[c[0], a[1], a[2]], [c[0], b[1], b[2]]]),
            _ => split.push(*c),
        }
    }
    let mut centres = split;
    kmeans(&pts, &mut centres, 4);
    let mut weight = vec![0u32; centres.len()];
    for (p, _) in &pts {
        if let Some(wt) = weight.get_mut(nearest(&centres, *p) as usize) {
            *wt += 1;
        }
    }
    let mut cs: Vec<(Px, f32)> = centres.into_iter().zip(weight).filter(|c| c.1 > 0).map(|(c, wt)| (c, wt as f32)).collect();
    loop {
        let mut best: Option<(usize, usize, f32)> = None;
        for i in 0..cs.len() {
            for j in i + 1..cs.len() {
                let d = dist2(cs[i].0, cs[j].0).sqrt();
                if d < SAME && best.is_none_or(|b| d < b.2) {
                    best = Some((i, j, d));
                }
            }
        }
        let Some((i, j, _)) = best else { break };
        let (a, b) = (cs[i], cs[j]);
        let t = a.1 + b.1;
        cs[i] = (std::array::from_fn(|k| (a.0[k] * a.1 + b.0[k] * b.1) / t), t);
        cs.remove(j);
    }
    if cs.is_empty() {
        return vec![[0.5, 0.0, 0.0]];
    }
    cs.into_iter().map(|c| c.0).collect()
}

fn nearest(palette: &[Px], p: Px) -> u8 {
    let mut best = (0usize, f32::MAX);
    for (i, c) in palette.iter().enumerate() {
        let d = dist2(*c, p);
        if d < best.1 {
            best = (i, d);
        }
    }
    best.0 as u8
}

// ---------------------------------------------------------------- regions

fn find(parent: &mut [u32], mut i: u32) -> u32 {
    while parent[i as usize] != i {
        let p = parent[i as usize];
        parent[i as usize] = parent[p as usize];
        i = p;
    }
    i
}

/// 4-connected components of equal labels, numbered in raster order of their first pixel.
fn components(labels: &[u8], w: usize, h: usize) -> (Vec<u32>, usize) {
    let mut comp = vec![0u32; w * h];
    let mut parent: Vec<u32> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let up = (y > 0 && labels[i - w] == labels[i]).then(|| comp[i - w]);
            let left = (x > 0 && labels[i - 1] == labels[i]).then(|| comp[i - 1]);
            comp[i] = match (up, left) {
                (Some(a), Some(b)) => {
                    let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                    let (lo, hi) = (ra.min(rb), ra.max(rb));
                    parent[hi as usize] = lo;
                    lo
                }
                (Some(a), None) | (None, Some(a)) => a,
                (None, None) => {
                    parent.push(parent.len() as u32);
                    parent.len() as u32 - 1
                }
            };
        }
    }
    let mut index = vec![u32::MAX; parent.len()];
    let mut n = 0u32;
    for c in comp.iter_mut() {
        let root = find(&mut parent, *c) as usize;
        if index[root] == u32::MAX {
            index[root] = n;
            n += 1;
        }
        *c = index[root];
    }
    (comp, n as usize)
}

/// A fast deterministic hasher for the region adjacency map.
#[derive(Default)]
struct FxHasher(u64);

impl Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(u64::from(b));
        }
    }
    fn write_u64(&mut self, v: u64) {
        self.0 = (self.0.rotate_left(5) ^ v).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

/// Merges small and sliver components into their neighbours. Returns each pixel's region and
/// each region's mean colour.
fn merge_regions(comp: &[u32], n: usize, c: &[Px], w: usize, h: usize, s: &Settings) -> (Vec<u32>, Vec<Px>) {
    let mut area = vec![0u32; n];
    let mut sum = vec![[0.0f64; 3]; n];
    for (&k, p) in comp.iter().zip(c) {
        let k = k as usize;
        area[k] += 1;
        sum[k] = [sum[k][0] + f64::from(p[0]), sum[k][1] + f64::from(p[1]), sum[k][2] + f64::from(p[2])];
    }
    // Shared border lengths (in cracks) and perimeters, the canvas edge included.
    let mut shared: HashMap<u64, u32, BuildHasherDefault<FxHasher>> = HashMap::default();
    let mut perim = vec![0u32; n];
    for y in 0..h {
        for x in 0..w {
            let a = comp[y * w + x];
            for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                if nx < w && ny < h {
                    let b = comp[ny * w + nx];
                    if a != b {
                        *shared.entry((u64::from(a.min(b)) << 32) | u64::from(a.max(b))).or_insert(0) += 1;
                    }
                }
            }
            perim[a as usize] += u32::from(x == 0) + u32::from(x + 1 == w) + u32::from(y == 0) + u32::from(y + 1 == h);
        }
    }
    let mut adj: Vec<Vec<(u32, u32)>> = vec![Vec::new(); n];
    let mut pairs: Vec<(u64, u32)> = shared.into_iter().collect();
    pairs.sort_unstable();
    for (key, len) in pairs {
        let (a, b) = ((key >> 32) as u32, key as u32);
        adj[a as usize].push((b, len));
        adj[b as usize].push((a, len));
        perim[a as usize] += len;
        perim[b as usize] += len;
    }
    let (min_area, sliver) = (s.min_area(), s.sliver());
    let mergeable = |a: u32, p: u32| a < min_area || 2.0 * a as f32 / (p.max(1) as f32) < sliver;
    let mut parent: Vec<u32> = (0..n as u32).collect();
    let mut heap: BinaryHeap<Reverse<(u32, u32)>> = (0..n).filter(|&i| mergeable(area[i], perim[i])).map(|i| Reverse((area[i], i as u32))).collect();
    while let Some(Reverse((a, id))) = heap.pop() {
        let i = id as usize;
        if find(&mut parent, id) != id || area[i] != a || !mergeable(a, perim[i]) {
            continue;
        }
        // The neighbours as they are now, with their total shared border.
        let mut nb: Vec<(u32, u32)> = std::mem::take(&mut adj[i]).into_iter().map(|(k, len)| (find(&mut parent, k), len)).filter(|&(k, _)| k != id).collect();
        nb.sort_unstable();
        nb.dedup_by(|b, a| {
            if a.0 == b.0 {
                a.1 += b.1;
                true
            } else {
                false
            }
        });
        let Some(&(t, len)) = nb.iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))) else {
            adj[i] = nb;
            continue;
        };
        let ti = t as usize;
        parent[i] = t;
        area[ti] += area[i];
        sum[ti] = [sum[ti][0] + sum[i][0], sum[ti][1] + sum[i][1], sum[ti][2] + sum[i][2]];
        perim[ti] = (perim[ti] + perim[i]).saturating_sub(2 * len);
        // Appended as they are; resolved and summed when `t` itself merges.
        adj[ti].extend(nb.into_iter().filter(|&(k, _)| k != t));
        if mergeable(area[ti], perim[ti]) {
            heap.push(Reverse((area[ti], t)));
        }
    }
    let mut index = vec![u32::MAX; n];
    let mut colours = Vec::new();
    let mut reg = vec![0u32; comp.len()];
    for (r, &k) in reg.iter_mut().zip(comp) {
        let root = find(&mut parent, k) as usize;
        if index[root] == u32::MAX {
            index[root] = colours.len() as u32;
            let a = f64::from(area[root].max(1));
            colours.push([(sum[root][0] / a) as f32, (sum[root][1] / a) as f32, (sum[root][2] / a) as f32]);
        }
        *r = index[root];
    }
    (reg, colours)
}

// ---------------------------------------------------------------- edges

/// A chain's points, and whether it is closed.
type Chain = (Vec<Pt>, bool);
/// A pixel, or a lattice point (the corner at a pixel's top left).
type Cell = (i64, i64);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dir {
    E,
    W,
    S,
    N,
}

/// Links the cracks between regions into chains. Each chain is its points (the junctions it runs
/// between, if any, and one sub-pixel point per crack) and whether it is closed; its sides are
/// the regions on its left and right (y down).
fn follow_edges(reg: &[u32], colours: &[Px], c: &[Px], w: usize, h: usize) -> (Vec<Chain>, Vec<(u32, u32)>) {
    let (wi, hi) = (w as i64, h as i64);
    let at = |x: i64, y: i64| -> u32 { if x < 0 || y < 0 || x >= wi || y >= hi { OUT } else { reg[y as usize * w + x as usize] } };
    let hcount = w * (h + 1);
    // Horizontal crack (x, y)–(x+1, y) is `y * w + x`; vertical (x, y)–(x, y+1) follows them.
    let hc = |x: i64, y: i64| (y as usize) * w + x as usize;
    let vc = |x: i64, y: i64| hcount + (y as usize) * (w + 1) + x as usize;
    let mut visited = vec![false; hcount + (w + 1) * h];
    // The crack leaving lattice point (x, y) in `d`: its id, the pixels on its left and right,
    // and the point it reaches.
    let step = |x: i64, y: i64, d: Dir| -> Option<(usize, Cell, Cell, Cell)> {
        match d {
            Dir::E if x < wi => Some((hc(x, y), (x, y - 1), (x, y), (x + 1, y))),
            Dir::W if x > 0 => Some((hc(x - 1, y), (x - 1, y), (x - 1, y - 1), (x - 1, y))),
            Dir::S if y < hi => Some((vc(x, y), (x, y), (x - 1, y), (x, y + 1))),
            Dir::N if y > 0 => Some((vc(x, y - 1), (x - 1, y - 1), (x, y - 1), (x, y - 1))),
            _ => None,
        }
    };
    // Which cracks separate two regions, and which lattice points are junctions, worked out
    // once (in parallel) before the walk.
    let mut is_edge = vec![false; visited.len()];
    let (hb, vb) = is_edge.split_at_mut(hcount);
    par::chunks_mut(hb, w, true, |y, row| {
        for (x, b) in row.iter_mut().enumerate() {
            *b = at(x as i64, y as i64 - 1) != at(x as i64, y as i64);
        }
    });
    par::chunks_mut(vb, w + 1, true, |y, row| {
        for (x, b) in row.iter_mut().enumerate() {
            *b = at(x as i64 - 1, y as i64) != at(x as i64, y as i64);
        }
    });
    let boundary = |x: i64, y: i64, d: Dir| step(x, y, d).filter(|s| is_edge[s.0]);
    let mut is_junction = vec![false; (w + 1) * (h + 1)];
    par::chunks_mut(&mut is_junction, w + 1, true, |y, row| {
        let y = y as i64;
        for (x, j) in row.iter_mut().enumerate() {
            let x = x as i64;
            let deg = [Dir::E, Dir::W, Dir::S, Dir::N].iter().filter(|&&d| boundary(x, y, d).is_some()).count();
            *j = deg >= 3 || (deg > 0 && (x == 0 || x == wi) && (y == 0 || y == hi));
        }
    });
    let junction = |x: i64, y: i64| is_junction[y as usize * (w + 1) + x as usize];
    // One point per crack: its middle, moved along the normal from left to right pixel by what
    // their colours say about where the edge really is.
    let crack_point = |l: (i64, i64), r: (i64, i64)| -> Pt {
        let mid = [(l.0 + r.0 + 1) as f32 / 2.0, (l.1 + r.1 + 1) as f32 / 2.0];
        let (n0, n1) = ((r.0 - l.0) as f32, (r.1 - l.1) as f32);
        let (ra, rb) = (at(l.0, l.1), at(r.0, r.1));
        let (Some(ca), Some(cb)) = (colours.get(ra as usize), colours.get(rb as usize)) else { return mid };
        let d = [cb[0] - ca[0], cb[1] - ca[1], cb[2] - ca[2]];
        let dd = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
        if dd < 1e-4 {
            return mid;
        }
        // How far each pixel's colour is from its own region's towards the other's (0–1).
        let frac = |p: (i64, i64)| {
            let q = c[p.1 as usize * w + p.0 as usize];
            ((q[0] - ca[0]) * d[0] + (q[1] - ca[1]) * d[1] + (q[2] - ca[2]) * d[2]) / dd
        };
        let (fl, fr) = (frac(l).clamp(0.0, 1.0), frac(r).clamp(0.0, 1.0));
        let off = ((1.0 - fr) - fl).clamp(-0.5, 0.5);
        [mid[0] + n0 * off, mid[1] + n1 * off]
    };
    let mut chains = Vec::new();
    let mut sides = Vec::new();
    let mut walk = |x: i64, y: i64, first: Dir, visited: &mut Vec<bool>| {
        let open = junction(x, y);
        let mut pts = if open { vec![[x as f32, y as f32]] } else { Vec::new() };
        let (mut cx, mut cy, mut d) = (x, y, first);
        let mut side = (OUT, OUT);
        let mut closed = !open;
        while let Some((id, l, r, to)) = boundary(cx, cy, d) {
            if visited[id] {
                break;
            }
            visited[id] = true;
            if pts.len() <= usize::from(open) {
                side = (at(l.0, l.1), at(r.0, r.1));
            }
            pts.push(crack_point(l, r));
            (cx, cy) = to;
            if junction(cx, cy) {
                pts.push([cx as f32, cy as f32]);
                closed = false;
                break;
            }
            let back = match d {
                Dir::E => Dir::W,
                Dir::W => Dir::E,
                Dir::S => Dir::N,
                Dir::N => Dir::S,
            };
            let Some(next) = [Dir::E, Dir::S, Dir::W, Dir::N].into_iter().find(|&nd| nd != back && boundary(cx, cy, nd).is_some_and(|b| !visited[b.0])) else {
                break;
            };
            d = next;
        }
        if pts.len() >= 2 {
            chains.push((pts, closed));
            sides.push(side);
        }
    };
    for y in 0..=hi {
        for x in 0..=wi {
            if junction(x, y) {
                for d in [Dir::E, Dir::S, Dir::W, Dir::N] {
                    if boundary(x, y, d).is_some_and(|b| !visited[b.0]) {
                        walk(x, y, d, &mut visited);
                    }
                }
            }
        }
    }
    // What is left are closed loops with no junction (an island inside one other region).
    for y in 0..=hi {
        for x in 0..=wi {
            for d in [Dir::E, Dir::S] {
                if boundary(x, y, d).is_some_and(|b| !visited[b.0]) {
                    walk(x, y, d, &mut visited);
                }
            }
        }
    }
    (chains, sides)
}

// ---------------------------------------------------------------- curves

fn sub(a: Pt, b: Pt) -> Pt {
    [a[0] - b[0], a[1] - b[1]]
}
fn len(a: Pt) -> f32 {
    (a[0] * a[0] + a[1] * a[1]).sqrt()
}

/// Fits one chain: corners kept, runs simplified and smoothed. Its ends stay where they are, so
/// chains meeting at a junction still meet.
fn fit(pts: &[Pt], closed: bool, s: &Settings) -> Vec<Pt> {
    let n = pts.len();
    if n < 3 {
        return pts.to_vec();
    }
    let rounded = round_off(pts, closed, s.rounding());
    let pts = &rounded[..];
    let at = |i: isize| pts[i.rem_euclid(n as isize) as usize];
    // Turn at each point, measured over a few cracks either side.
    const K: isize = 3;
    let turn = |i: isize| {
        let (a, b) = (sub(at(i), at(i - K)), sub(at(i + K), at(i)));
        let (la, lb) = (len(a), len(b));
        if la < 1e-6 || lb < 1e-6 { 0.0 } else { ((a[0] * b[0] + a[1] * b[1]) / (la * lb)).clamp(-1.0, 1.0).acos() }
    };
    let span = if closed { 0..n as isize } else { K..(n as isize - K) };
    let turns: Vec<(isize, f32)> = span.clone().map(|i| (i, turn(i))).collect();
    let limit = s.corner_turn();
    let mut corners: Vec<usize> = Vec::new();
    for &(i, t) in &turns {
        if t <= limit {
            continue;
        }
        // Only the sharpest point of a bend.
        let sharpest = (-K..=K).all(|o| {
            let j = i + o;
            let in_span = closed || span.contains(&j);
            !in_span || o == 0 || {
                let tj = turn(j);
                tj < t || (tj == t && o > 0)
            }
        });
        if sharpest {
            corners.push(i.rem_euclid(n as isize) as usize);
        }
    }
    // Square a corner back where the pixels cut it: where the straight runs either side meet.
    let mut p = pts.to_vec();
    for &i in &corners {
        let ii = i as isize;
        if let Some(q) = intersect(at(ii - K), at(ii - 1), at(ii + 1), at(ii + K))
            && len(sub(q, at(ii))) <= 1.0
        {
            p[i] = q;
        }
    }
    let mut keep: Vec<usize> = if closed { corners.clone() } else { std::iter::once(0).chain(corners.iter().copied()).chain(std::iter::once(n - 1)).collect() };
    keep.sort_unstable();
    keep.dedup();
    let mut is_corner = vec![false; n];
    for &k in &keep {
        is_corner[k] = true;
    }
    if closed && keep.len() < 2 {
        // A smooth loop: split it at two far-apart points (not corners) to simplify each half.
        let a = keep.first().copied().unwrap_or(0);
        let far = (0..n).max_by(|&i, &j| len(sub(p[i], p[a])).total_cmp(&len(sub(p[j], p[a])))).unwrap_or(n / 2);
        keep = if a == far { vec![a, (a + n / 2) % n] } else { vec![a.min(far), a.max(far)] };
    }
    // Douglas–Peucker between consecutive kept points.
    let tol = s.tolerance();
    let mut verts: Vec<usize> = Vec::new();
    let pieces = if closed { keep.len() } else { keep.len() - 1 };
    for k in 0..pieces {
        let a = keep[k];
        let b = if k + 1 < keep.len() { keep[k + 1] } else { keep[0] + n };
        verts.push(a);
        let idx: Vec<usize> = (a..=b).collect();
        let mut marks = vec![false; idx.len()];
        simplify(&idx.iter().map(|&i| p[i % n]).collect::<Vec<_>>(), 0, idx.len() - 1, tol, &mut marks);
        verts.extend(idx.iter().zip(&marks).skip(1).take(idx.len().saturating_sub(2)).filter(|m| *m.1).map(|(&i, _)| i % n));
    }
    if !closed {
        verts.push(n - 1);
    }
    let v: Vec<Pt> = verts.iter().map(|&i| p[i]).collect();
    let corner: Vec<bool> = verts.iter().map(|&i| is_corner[i]).collect();
    curves(&v, &corner, closed)
}

/// Two passes of a moving average over `r` points either side; an open chain's ends (junctions)
/// stay put, and the window shrinks towards them so they don't pull the outline.
fn round_off(pts: &[Pt], closed: bool, r: usize) -> Vec<Pt> {
    let n = pts.len();
    let mut p = pts.to_vec();
    if r == 0 || n < 3 {
        return p;
    }
    for _ in 0..2 {
        let src = p.clone();
        for (i, q) in p.iter_mut().enumerate() {
            let rr = if closed { r.min((n - 1) / 2) } else { r.min(i).min(n - 1 - i) };
            if rr == 0 {
                continue;
            }
            let (mut sx, mut sy) = (0.0f32, 0.0f32);
            for o in -(rr as isize)..=rr as isize {
                let s = src[(i as isize + o).rem_euclid(n as isize) as usize];
                sx += s[0];
                sy += s[1];
            }
            let k = (2 * rr + 1) as f32;
            *q = [sx / k, sy / k];
        }
    }
    p
}

fn intersect(a0: Pt, a1: Pt, b0: Pt, b1: Pt) -> Option<Pt> {
    let (da, db) = (sub(a1, a0), sub(b1, b0));
    let den = da[0] * db[1] - da[1] * db[0];
    if den.abs() < 1e-6 {
        return None;
    }
    let t = ((b0[0] - a0[0]) * db[1] - (b0[1] - a0[1]) * db[0]) / den;
    Some([a0[0] + da[0] * t, a0[1] + da[1] * t])
}

/// Marks the points of `p[a..=b]` that Douglas–Peucker keeps at tolerance `tol`.
fn simplify(p: &[Pt], a: usize, b: usize, tol: f32, marks: &mut [bool]) {
    let mut stack = vec![(a, b)];
    while let Some((a, b)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        let (pa, d) = (p[a], sub(p[b], p[a]));
        let l = len(d);
        let mut best = (a, 0.0f32);
        for (i, q) in p.iter().enumerate().take(b).skip(a + 1) {
            let e = sub(*q, pa);
            let dist = if l < 1e-6 { len(e) } else { (e[0] * d[1] - e[1] * d[0]).abs() / l };
            if dist > best.1 {
                best = (i, dist);
            }
        }
        if best.1 > tol {
            marks[best.0] = true;
            stack.push((a, best.0));
            stack.push((best.0, b));
        }
    }
}

/// Catmull-Rom curves through `v` (one-sided at corners), flattened to a polyline.
fn curves(v: &[Pt], corner: &[bool], closed: bool) -> Vec<Pt> {
    let m = v.len();
    if m < 2 {
        return v.to_vec();
    }
    let at = |i: isize| v[i.rem_euclid(m as isize) as usize];
    let sharp = |i: isize| (!closed && (i <= 0 || i >= m as isize - 1)) || corner[i.rem_euclid(m as isize) as usize];
    let mut out = vec![v[0]];
    let segs = if closed { m } else { m - 1 };
    for i in 0..segs as isize {
        let (p0, p1) = (at(i), at(i + 1));
        let d = len(sub(p1, p0));
        let (s0, s1) = (sharp(i), sharp(i + 1));
        if s0 && s1 || d < 1e-6 {
            out.push(p1);
            continue;
        }
        // Tangents scaled to this segment, so short and long neighbours don't overshoot.
        let t0 = if s0 {
            sub(p1, p0)
        } else {
            let dp = len(sub(p0, at(i - 1)));
            let k = d / (dp + d).max(1e-6);
            let t = sub(p1, at(i - 1));
            [t[0] * k, t[1] * k]
        };
        let t1 = if s1 {
            sub(p1, p0)
        } else {
            let dn = len(sub(at(i + 2), p1));
            let k = d / (d + dn).max(1e-6);
            let t = sub(at(i + 2), p0);
            [t[0] * k, t[1] * k]
        };
        let c1 = [p0[0] + t0[0] / 3.0, p0[1] + t0[1] / 3.0];
        let c2 = [p1[0] - t1[0] / 3.0, p1[1] - t1[1] / 3.0];
        let steps = ((d / 1.5).ceil() as usize).clamp(2, 24);
        for k in 1..=steps {
            let t = k as f32 / steps as f32;
            let u = 1.0 - t;
            let (b0, b1, b2, b3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            out.push([b0 * p0[0] + b1 * c1[0] + b2 * c2[0] + b3 * p1[0], b0 * p0[1] + b1 * c1[1] + b2 * c2[1] + b3 * p1[1]]);
        }
    }
    out
}

fn bounds(pts: &[Pt]) -> [f32; 4] {
    pts.iter().fold([f32::MAX, f32::MAX, f32::MIN, f32::MIN], |b, p| [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])])
}

// ---------------------------------------------------------------- fill

/// Accumulates the signed area of segment `a`–`b` into `acc` (rows of `stride` cells, `h` rows
/// visible, columns `0..stride - 2` visible): the part left of the tile is drawn down its left
/// edge (it still winds the pixels to its right), the part right of it is dropped.
fn line_clipped(acc: &mut [f32], stride: usize, h: usize, a: Pt, b: Pt) {
    if !(a[0].is_finite() && a[1].is_finite() && b[0].is_finite() && b[1].is_finite()) || a[1] == b[1] {
        return;
    }
    let wf = (stride - 2) as f32;
    // At most the two ends and two crossings; on the stack, as this runs for every segment drawn.
    let mut buf = [0.0f32, 1.0, 0.0, 0.0];
    let mut n = 2;
    for x in [0.0, wf] {
        let t = (x - a[0]) / (b[0] - a[0]);
        if t > 0.0 && t < 1.0 {
            buf[n] = t;
            n += 1;
        }
    }
    let cuts = &mut buf[..n];
    cuts.sort_by(f32::total_cmp);
    for t in cuts.windows(2) {
        let p = [a[0] + (b[0] - a[0]) * t[0], a[1] + (b[1] - a[1]) * t[0]];
        let q = [a[0] + (b[0] - a[0]) * t[1], a[1] + (b[1] - a[1]) * t[1]];
        let mx = (p[0] + q[0]) / 2.0;
        if mx >= wf {
            continue;
        }
        if mx < 0.0 {
            line(acc, stride, h, [0.0, p[1]], [0.0, q[1]]);
        } else {
            line(acc, stride, h, [p[0].clamp(0.0, wf), p[1]], [q[0].clamp(0.0, wf), q[1]]);
        }
    }
}

/// Signed-area accumulation of one segment inside the tile (the scanline method of font
/// rasterisers): after a running sum along each row, a cell holds the coverage winding.
fn line(acc: &mut [f32], stride: usize, h: usize, p0: Pt, p1: Pt) {
    if p0[1] == p1[1] {
        return;
    }
    let (dir, p0, p1) = if p0[1] < p1[1] { (1.0f32, p0, p1) } else { (-1.0, p1, p0) };
    let dxdy = (p1[0] - p0[0]) / (p1[1] - p0[1]);
    let mut x = p0[0];
    let ystart = p0[1].max(0.0);
    if p0[1] < 0.0 {
        x -= p0[1] * dxdy;
    }
    let yend = p1[1].min(h as f32);
    if ystart >= yend {
        return;
    }
    let max_x = (stride - 1) as f32;
    for y in (ystart as usize)..(yend.ceil() as usize).min(h) {
        let ls = y * stride;
        let dy = ((y + 1) as f32).min(p1[1]) - (y as f32).max(p0[1]);
        let xnext = x + dxdy * dy;
        let d = dy * dir;
        let (x0, x1) = if x < xnext { (x, xnext) } else { (xnext, x) };
        let (x0, x1) = (x0.clamp(0.0, max_x - 1.0), x1.clamp(0.0, max_x - 1.0));
        let x0f = x0.floor();
        let x0i = x0f as usize;
        let x1c = x1.ceil();
        let x1i = x1c as usize;
        if x1i <= x0i + 1 {
            let xmf = 0.5 * (x0 + x1) - x0f;
            acc[ls + x0i] += d - d * xmf;
            acc[ls + x0i + 1] += d * xmf;
        } else {
            let sx = (x1 - x0).recip();
            let x0r = x0 - x0f;
            let a0 = 0.5 * sx * (1.0 - x0r) * (1.0 - x0r);
            let x1r = x1 - x1c + 1.0;
            let am = 0.5 * sx * x1r * x1r;
            acc[ls + x0i] += d * a0;
            if x1i == x0i + 2 {
                acc[ls + x0i + 1] += d * (1.0 - a0 - am);
            } else {
                let a1 = sx * (1.5 - x0r);
                acc[ls + x0i + 1] += d * (a1 - a0);
                for xi in x0i + 2..x1i - 1 {
                    acc[ls + xi] += d * sx;
                }
                let a2 = a1 + (x1i - x0i - 3) as f32 * sx;
                acc[ls + x1i - 1] += d * (1.0 - a2 - am);
            }
            acc[ls + x1i] += d * am;
        }
        x = xnext;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(levels: usize, simplicity: f32, fidelity: f32) -> Settings {
        Settings { levels, simplicity, fidelity }
    }

    /// Total coverage of every pixel: each pixel is covered exactly once by the shapes.
    fn coverage(t: &Traced, r: Rect) -> Vec<f32> {
        let (w, h) = (r.width() as usize, r.height() as usize);
        let mut weight = vec![0.0f32; w * h];
        let stride = w + 2;
        let mut acc = vec![0.0f32; stride * h];
        for g in &t.regions {
            for &(k, rev) in &g.edges {
                for seg in t.chains[k as usize].windows(2) {
                    let (a, z) = if rev { (seg[1], seg[0]) } else { (seg[0], seg[1]) };
                    line_clipped(&mut acc, stride, h, a, z);
                }
            }
            for y in 0..h {
                let mut sum = 0.0;
                for x in 0..w {
                    sum += acc[y * stride + x];
                    weight[y * w + x] += sum.abs().min(1.0);
                }
                acc[y * stride..(y + 1) * stride].fill(0.0);
            }
        }
        weight
    }

    #[test]
    fn a_square_covers_its_area_exactly() {
        let mut acc = vec![0.0f32; 12 * 10];
        let sq = [[2.25, 2.5], [7.75, 2.5], [7.75, 6.5], [2.25, 6.5], [2.25, 2.5]];
        for s in sq.windows(2) {
            line_clipped(&mut acc, 12, 10, s[0], s[1]);
        }
        let mut total = 0.0;
        for y in 0..10 {
            let mut sum = 0.0;
            for x in 0..10 {
                sum += acc[y * 12 + x];
                total += sum.abs().min(1.0);
            }
        }
        assert!((total - 5.5 * 4.0).abs() < 1e-4, "{total}");
    }

    #[test]
    fn shapes_cover_every_pixel_once_with_no_seams() {
        // Blobs of four colours touching each other and the canvas edge, with T-junctions,
        // saddles and holes.
        let (w, h) = (48usize, 36usize);
        let pal = [[0.1, 0.1, 0.1], [0.9, 0.2, 0.2], [0.2, 0.7, 0.3], [0.95, 0.95, 0.9]];
        let c: Vec<Px> = (0..w * h)
            .map(|i| {
                let (x, y) = ((i % w) as f32, (i / w) as f32);
                let k = ((x / 7.0).sin() + (y / 5.0).cos() + (x * y / 90.0).sin()).abs() * 1.6;
                pal[(k as usize).min(3)]
            })
            .collect();
        let r = Rect::new(0, 0, w as i32, h as i32);
        for fidelity in [1.0, 3.0] {
            let t = trace(&c, r, &settings(4, 2.0, fidelity));
            assert!(t.regions.len() > 4);
            let cov = coverage(&t, r);
            let worst = cov.iter().map(|v| (v - 1.0).abs()).fold(0.0, f32::max);
            assert!(worst < 1e-3, "coverage off by {worst}");
        }
    }

    #[test]
    fn edge_fidelity_changes_the_outlines() {
        // It used to set only which bends count as corners: on smooth shapes 0.5% of pixels
        // changed from fidelity 1 to 2 and 2.8% from 2 to 3 (now about 9% and 5%).
        // Large smooth shapes, as in a photo: few corners, long curved outlines.
        let (w, h) = (160usize, 120usize);
        let pal = [[0.1, 0.1, 0.1], [0.9, 0.3, 0.2], [0.3, 0.6, 0.9]];
        let c: Vec<Px> = (0..w * h)
            .map(|i| {
                let (x, y) = ((i % w) as f32, (i / w) as f32);
                let k = ((x / 17.0).sin() * 1.3 + (y / 13.0).cos() + (x * y / 900.0).sin()).abs();
                pal[(k as usize).min(2)]
            })
            .collect();
        let r = Rect::new(0, 0, w as i32, h as i32);
        let out = |f: f32| trace(&c, r, &settings(3, 2.0, f)).render(r);
        let changed = |a: &[Px], b: &[Px]| a.iter().zip(b).filter(|(p, q)| dist2(**p, **q) > 0.01).count() as f32 / a.len() as f32;
        let (loose, normal, tight) = (out(1.0), out(2.0), out(3.0));
        assert!(changed(&loose, &normal) > 0.04, "fidelity 1 vs 2: {}", changed(&loose, &normal));
        assert!(changed(&normal, &tight) > 0.04, "fidelity 2 vs 3: {}", changed(&normal, &tight));
    }

    #[test]
    fn filling_does_not_depend_on_the_tiling() {
        let (w, h) = (40usize, 30usize);
        let c: Vec<Px> = (0..w * h).map(|i| if ((i % w) / 6 + (i / w) / 5) % 3 == 0 { [0.8, 0.3, 0.1] } else { [0.1, 0.3, 0.7] }).collect();
        let r = Rect::new(10, 20, 10 + w as i32, 20 + h as i32);
        let t = trace(&c, r, &settings(3, 1.0, 2.0));
        let whole = t.render(r);
        for tile in [Rect::new(10, 20, 23, 31), Rect::new(23, 31, 50, 50), Rect::new(37, 20, 50, 33)] {
            let part = t.render(tile);
            for y in tile.y0..tile.y1 {
                for x in tile.x0..tile.x1 {
                    let a = whole[((y - r.y0) as usize) * w + (x - r.x0) as usize];
                    let b = part[((y - tile.y0) as usize) * tile.width() as usize + (x - tile.x0) as usize];
                    assert!(dist2(a, b) < 1e-8, "({x}, {y}): {a:?} vs {b:?}");
                }
            }
        }
    }

    #[test]
    fn noise_merges_into_neighbours_and_noise_apart_colours_are_one() {
        let (w, h) = (32usize, 32usize);
        let c: Vec<Px> = (0..w * h)
            .map(|i| {
                let v = 0.3 + crate::photo_util::hash01((i % w) as i64, (i / w) as i64, 5) * 0.06;
                if (11..14).contains(&(i % w)) && (11..14).contains(&(i / w)) { [0.9, 0.9, 0.9] } else { [v, v, v] }
            })
            .collect();
        let r = Rect::new(0, 0, w as i32, h as i32);
        assert_eq!(trace(&c, r, &settings(6, 0.0, 2.0)).regions.len(), 2, "the speck stays at simplicity 0");
        assert_eq!(trace(&c, r, &settings(6, 10.0, 2.0)).regions.len(), 1, "the speck merges at simplicity 10");
    }
}
