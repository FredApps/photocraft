//! Transient opacity and spatial Fill previews for mask Paint layers. The committed document
//! changes only when the control gesture ends; display snapshots materialize the live stack.
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use photocraft_doc::{DocId, Document, LayerId};
use serde_json::json;

use crate::PhotocraftApp;

const KEY_BASE: u64 = 1 << 42;
const FILL_PREVIEW_MIN_INTERVAL: Duration = Duration::from_millis(80);

pub(crate) struct Preview {
    doc: DocId,
    revision: u64,
    layer: LayerId,
    index: usize,
    opacity: f32,
    bounds: Option<[i32; 4]>,
    rotation: Option<f32>,
    shown: Option<(u64, Arc<Document>)>,
    shown_fill: Option<([i32; 4], f32)>,
    shown_at: Option<Instant>,
    materialized: Option<(u64, Arc<Document>)>,
}

#[derive(Clone, Copy)]
pub(crate) struct Commit {
    pub doc: DocId,
    pub revision: u64,
    pub layer: LayerId,
    pub index: usize,
    pub opacity: f32,
    pub bounds: Option<[i32; 4]>,
    pub rotation: Option<f32>,
}
impl Commit {
    pub(crate) fn params(self) -> serde_json::Value {
        let mut params = json!({"layer":self.layer.0,"index":self.index});
        if let Some(opacity) = self.opacity.is_finite().then_some(self.opacity) {
            params["opacity"] = json!(opacity);
        }
        if let Some(bounds) = self.bounds {
            params["bounds"] = json!(bounds);
        }
        if let Some(rotation) = self.rotation {
            params["rotation"] = json!(rotation);
        }
        params
    }
}

pub(crate) fn set(app: &mut PhotocraftApp, layer: LayerId, index: usize, opacity: f32) {
    if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
        return;
    }

    let Some((doc, revision, _)) = active_opacity(app, layer, index) else { return };
    let matches_active = app.uv_mask_paint_preview.as_ref().is_some_and(|p| (p.doc, p.revision, p.layer, p.index) == (doc, revision, layer, index));
    if app.uv_mask_paint_preview.is_some() && !matches_active {
        let Some(previous) = pending(app) else { return };
        if let Err(error) = app.run("uv.maskPaint.modify", previous.params()) {
            app.ui.status = error;
            app.ui.status_error = true;
            return;
        }
    }

    // Committing another preview can advance this document's revision; re-read both revision
    // and canonical value before assigning the next transient value.
    let Some((doc, revision, canonical)) = active_opacity(app, layer, index) else { return };
    if let Some(p) = app.uv_mask_paint_preview.as_mut()
        && (p.doc, p.revision, p.layer, p.index) == (doc, revision, layer, index)
    {
        if opacity.to_bits() == canonical.to_bits() {
            app.uv_mask_paint_preview = None;
        } else if p.opacity.to_bits() != opacity.to_bits() {
            p.opacity = opacity;
            p.bounds = None;
            p.rotation = None;
            p.shown = None;
            p.shown_fill = None;
            p.shown_at = None;
            p.materialized = None;
        }
    } else if app.uv_mask_paint_preview.is_none() && opacity.to_bits() != canonical.to_bits() {
        app.uv_mask_paint_preview = Some(Preview {
            doc,
            revision,
            layer,
            index,
            opacity,
            bounds: None,
            rotation: None,
            shown: None,
            shown_fill: None,
            shown_at: None,
            materialized: None,
        });
    }
}

pub(crate) fn set_fill(app: &mut PhotocraftApp, layer: LayerId, index: usize, bounds: [i32; 4], rotation: f32) {
    if !rotation.is_finite() || bounds[0] >= bounds[2] || bounds[1] >= bounds[3] {
        return;
    }
    let Some((doc, revision)) = active_fill(app, layer, index) else { return };
    if app.uv_mask_paint_preview.as_ref().is_some_and(|p| (p.doc, p.revision, p.layer, p.index) != (doc, revision, layer, index)) {
        let Some(previous) = pending(app) else { return };
        if let Err(error) = app.run("uv.maskPaint.modify", previous.params()) {
            app.ui.status = error;
            app.ui.status_error = true;
            return;
        }
    }
    let Some((doc, revision)) = active_fill(app, layer, index) else { return };
    let current =
        app.session.active().and_then(|st| st.doc.layer(layer)).and_then(|l| l.mask.as_ref()).and_then(|m| m.paint.as_ref()).and_then(|s| s.layers.get(index));
    let Some(effect) = current else { return };
    let canonical_opacity = effect.opacity;
    let canonical_bounds = effect.fill_bounds.unwrap_or_else(|| {
        app.session.active().filter(|st| st.doc.id == doc).map_or([0, 0, 0, 0], |st| [0, 0, st.doc.size.width as i32, st.doc.size.height as i32])
    });
    let canonical_rotation = match &effect.kind {
        photocraft_doc::mask_paint::EffectKind::Fill { rotation, .. } => *rotation,
        _ => return,
    };
    if bounds == canonical_bounds && rotation.to_bits() == canonical_rotation.to_bits() {
        app.uv_mask_paint_preview = None;
    } else if let Some(p) = app.uv_mask_paint_preview.as_mut()
        && (p.doc, p.revision, p.layer, p.index) == (doc, revision, layer, index)
    {
        if p.bounds == Some(bounds) && p.rotation.is_some_and(|current| current.to_bits() == rotation.to_bits()) {
            return;
        }
        if !p.opacity.is_finite() {
            p.opacity = canonical_opacity;
        }
        p.bounds = Some(bounds);
        p.rotation = Some(rotation);
        p.materialized = None;
    } else if app.uv_mask_paint_preview.is_none() {
        app.uv_mask_paint_preview = Some(Preview {
            doc,
            revision,
            layer,
            index,
            opacity: canonical_opacity,
            bounds: Some(bounds),
            rotation: Some(rotation),
            shown: None,
            shown_fill: None,
            shown_at: None,
            materialized: None,
        });
    }
}

fn active_fill(app: &PhotocraftApp, layer: LayerId, index: usize) -> Option<(DocId, u64)> {
    let st = app.session.active()?;
    if st.doc.effective_locks(layer).all {
        return None;
    }
    let effect = st.doc.layer(layer)?.mask.as_ref()?.paint.as_ref()?.layers.get(index)?;
    matches!(&effect.kind, photocraft_doc::mask_paint::EffectKind::Fill { .. }).then_some((st.doc.id, st.revision))
}

fn active_opacity(app: &PhotocraftApp, layer: LayerId, index: usize) -> Option<(DocId, u64, f32)> {
    let st = app.session.active()?;
    let target = st.doc.layer(layer)?;
    if st.doc.effective_locks(layer).all {
        return None;
    }
    let opacity = target.mask.as_ref()?.paint.as_ref()?.layers.get(index)?.opacity;
    Some((st.doc.id, st.revision, opacity))
}

pub(crate) fn value(app: &PhotocraftApp, layer: LayerId, index: usize, fallback: f32) -> f32 {
    app.uv_mask_paint_preview
        .as_ref()
        .filter(|p| {
            p.opacity.is_finite()
                && p.layer == layer
                && p.index == index
                && app.session.active().is_some_and(|st| st.doc.id == p.doc && st.revision == p.revision)
        })
        .map_or(fallback, |p| p.opacity)
}

pub(crate) fn fill_transform(app: &PhotocraftApp, layer: LayerId, index: usize, bounds: [i32; 4], rotation: f32) -> ([i32; 4], f32) {
    app.uv_mask_paint_preview
        .as_ref()
        .filter(|p| {
            p.bounds.is_some() && p.layer == layer && p.index == index && app.session.active().is_some_and(|st| st.doc.id == p.doc && st.revision == p.revision)
        })
        .map_or((bounds, rotation), |p| (p.bounds.unwrap_or(bounds), p.rotation.unwrap_or(rotation)))
}

/// An immutable document snapshot for the 2D compositor. Opacity uses the live GPU stack;
/// spatial Fill transforms also update the bounded flattened mask used by the GPU fallback.
pub(crate) fn display_doc(app: &mut PhotocraftApp, idx: usize) -> Option<(Arc<Document>, u64)> {
    let st = app.session.documents().get(idx)?;
    let p = app.uv_mask_paint_preview.as_mut().filter(|p| p.doc == st.doc.id && p.revision == st.revision)?;
    let key = preview_key(p.revision, p.layer, p.index, p.opacity, p.bounds, p.rotation);
    if let Some((seen, shown)) = &p.shown
        && *seen == key
    {
        return Some((shown.clone(), key));
    }
    if let Some((seen, shown)) = &p.shown
        && !fill_preview_refresh_due(p.shown_at, Instant::now())
    {
        return Some((shown.clone(), *seen));
    }
    let source = p.shown.as_ref().map_or_else(|| st.doc.clone(), |(_, shown)| shown.clone());
    let mut doc = (*source).clone();
    let canvas = doc.bounds();
    let mask = doc.layer_mut(p.layer)?.mask.as_mut()?;
    let paint = mask.paint.as_mut()?.layers.get_mut(p.index)?;
    let canonical_bounds = paint.fill_bounds.unwrap_or([canvas.x0, canvas.y0, canvas.x1, canvas.y1]);
    let canonical_rotation = match &paint.kind {
        photocraft_doc::mask_paint::EffectKind::Fill { rotation, .. } => *rotation,
        _ => return None,
    };
    let old_fill = p.shown_fill.unwrap_or((canonical_bounds, canonical_rotation));
    if p.opacity.is_finite() {
        paint.opacity = p.opacity;
    }
    if let Some(bounds) = p.bounds {
        paint.fill_bounds = Some(bounds);
    }
    if let (Some(rotation), photocraft_doc::mask_paint::EffectKind::Fill { rotation: current, .. }) = (p.rotation, &mut paint.kind) {
        *current = rotation;
    }
    if p.bounds.is_some() || p.rotation.is_some() {
        let bounds = paint.fill_bounds.unwrap_or(canonical_bounds);
        let rotation = match &paint.kind {
            photocraft_doc::mask_paint::EffectKind::Fill { rotation, .. } => *rotation,
            _ => canonical_rotation,
        };
        let repeats = paint.fill_map.is_some()
            && matches!(
                &paint.kind,
                photocraft_doc::mask_paint::EffectKind::Fill { repeat_u: true, .. } | photocraft_doc::mask_paint::EffectKind::Fill { repeat_v: true, .. }
            );
        let damage = if repeats { canvas } else { fill_aabb(old_fill.0, old_fill.1).union(&fill_aabb(bounds, rotation)).intersect(&canvas) };
        photocraft_compose::mask_paint::refresh(mask, Some(damage));
        p.shown_fill = Some((bounds, rotation));
    }
    let shown = Arc::new(doc);
    p.shown = Some((key, shown.clone()));
    p.shown_at = Some(Instant::now());
    Some((shown, key))
}

fn fill_preview_refresh_due(last: Option<Instant>, now: Instant) -> bool {
    last.is_none_or(|last| now.duration_since(last) >= FILL_PREVIEW_MIN_INTERVAL)
}

fn fill_aabb([x0, y0, x1, y1]: [i32; 4], rotation: f32) -> photocraft_geom::Rect {
    if !rotation.is_finite() || rotation.rem_euclid(360.0) == 0.0 {
        return photocraft_geom::Rect::new(x0, y0, x1, y1);
    }
    let angle = f64::from(rotation.rem_euclid(360.0)).to_radians();
    let (sin, cos) = angle.sin_cos();
    let width = f64::from(x1 - x0);
    let height = f64::from(y1 - y0);
    let center_x = (f64::from(x0) + f64::from(x1)) * 0.5;
    let center_y = (f64::from(y0) + f64::from(y1)) * 0.5;
    let half_width = (cos.abs() * width + sin.abs() * height) * 0.5;
    let half_height = (sin.abs() * width + cos.abs() * height) * 0.5;
    photocraft_geom::Rect::new(
        (center_x - half_width).floor() as i32,
        (center_y - half_height).floor() as i32,
        (center_x + half_width).ceil() as i32,
        (center_y + half_height).ceil() as i32,
    )
}

/// CPU-only consumers (the UV map builder or unsupported GPU fallback) need the conventional
/// flattened cache. Reuse an already materialized Fill or build once per opacity value.
pub(crate) fn cpu_document(app: &mut PhotocraftApp, doc: Arc<Document>, key: u64) -> Arc<Document> {
    let revision = app.session.documents().iter().find(|st| st.doc.id == doc.id).map(|st| st.revision);
    let Some(p) = app.uv_mask_paint_preview.as_mut().filter(|p| p.doc == doc.id && Some(p.revision) == revision) else { return doc };
    if let Some((cached, materialized)) = &p.materialized
        && *cached == key
    {
        return materialized.clone();
    }
    let mut cpu = (*doc).clone();
    if p.bounds.is_none()
        && p.rotation.is_none()
        && let Some(mask) = cpu.layer_mut(p.layer).and_then(|layer| layer.mask.as_mut())
    {
        photocraft_compose::mask_paint::refresh(mask, None);
    }
    let cpu = Arc::new(cpu);
    p.materialized = Some((key, cpu.clone()));
    cpu
}

/// Take the latest value so app.run can commit it once before another command consumes the doc.
pub(crate) fn pending(app: &PhotocraftApp) -> Option<Commit> {
    let p = app.uv_mask_paint_preview.as_ref()?;
    Some(Commit { doc: p.doc, revision: p.revision, layer: p.layer, index: p.index, opacity: p.opacity, bounds: p.bounds, rotation: p.rotation })
}
pub(crate) fn active_document(app: &PhotocraftApp) -> Option<DocId> {
    let p = app.uv_mask_paint_preview.as_ref()?;
    app.session.documents().iter().any(|st| st.doc.id == p.doc && st.revision == p.revision).then_some(p.doc)
}

pub(crate) fn clear(app: &mut PhotocraftApp, commit: Commit) {
    if app.uv_mask_paint_preview.as_ref().is_some_and(|p| (p.doc, p.revision, p.layer, p.index) == (commit.doc, commit.revision, commit.layer, commit.index)) {
        app.uv_mask_paint_preview = None;
    }
}

pub(crate) fn cancel_stale_on_escape(app: &mut PhotocraftApp, ctx: &egui::Context) {
    let Some(p) = app.uv_mask_paint_preview.as_ref() else { return };
    let stale = !app.session.documents().iter().any(|st| st.doc.id == p.doc && st.revision == p.revision);
    if stale && app.ui.dialogs.is_empty() && !ctx.egui_wants_keyboard_input() && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        cancel(app);
        app.ui.status = "Discarded an outdated mask Paint opacity preview".into();
    }
}

pub(crate) fn cancel(app: &mut PhotocraftApp) {
    app.uv_mask_paint_preview = None;
}

fn preview_key(revision: u64, layer: LayerId, index: usize, opacity: f32, bounds: Option<[i32; 4]>, rotation: Option<f32>) -> u64 {
    let bounds_key = bounds.map_or(0, |b| b.into_iter().fold(0_u64, |key, value| key.rotate_left(13) ^ value as u32 as u64));
    let rotation_key = rotation.map_or(0, f32::to_bits) as u64;
    let opacity_key = if opacity.is_finite() { u64::from(opacity.to_bits()) } else { 0 };
    let seed = revision.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ layer.0.rotate_left(19)
        ^ (index as u64).rotate_left(37)
        ^ opacity_key
        ^ bounds_key.rotate_left(9)
        ^ rotation_key.rotate_left(29);
    (seed & (KEY_BASE - 1)) | KEY_BASE
}

#[cfg(test)]
#[path = "uv_mask_paint_preview_tests.rs"]
mod tests;
