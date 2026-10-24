//! Transient opacity preview for mask Paint layers. The committed document changes only when
//! the control gesture ends; the compositor reads the live Stack directly in the meantime.
use std::sync::Arc;

use photocraft_doc::{DocId, Document, LayerId};
use serde_json::json;

use crate::PhotocraftApp;

const KEY_BASE: u64 = 1 << 42;

pub(crate) struct Preview {
    doc: DocId,
    revision: u64,
    layer: LayerId,
    index: usize,
    opacity: f32,
    shown: Option<(u64, Arc<Document>)>,
    materialized: Option<(u64, Arc<Document>)>,
}

#[derive(Clone, Copy)]
pub(crate) struct Commit {
    pub doc: DocId,
    pub revision: u64,
    pub layer: LayerId,
    pub index: usize,
    pub opacity: f32,
}
impl Commit {
    pub(crate) fn params(self) -> serde_json::Value {
        json!({"layer":self.layer.0,"index":self.index,"opacity":self.opacity})
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
            p.shown = None;
            p.materialized = None;
        }
    } else if app.uv_mask_paint_preview.is_none() && opacity.to_bits() != canonical.to_bits() {
        app.uv_mask_paint_preview = Some(Preview { doc, revision, layer, index, opacity, shown: None, materialized: None });
    }
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
        .filter(|p| p.layer == layer && p.index == index && app.session.active().is_some_and(|st| st.doc.id == p.doc && st.revision == p.revision))
        .map_or(fallback, |p| p.opacity)
}

/// A cheap immutable document snapshot for the 2D GPU compositor. Its mask.surface is deliberately
/// untouched: the Paint stack is the source of truth and the GPU plan consumes it directly.
pub(crate) fn display_doc(app: &mut PhotocraftApp, idx: usize) -> Option<(Arc<Document>, u64)> {
    let st = app.session.documents().get(idx)?;
    let p = app.uv_mask_paint_preview.as_mut().filter(|p| p.doc == st.doc.id && p.revision == st.revision)?;
    let key = preview_key(p.revision, p.layer, p.index, p.opacity);
    if let Some((seen, shown)) = &p.shown
        && *seen == key
    {
        return Some((shown.clone(), key));
    }
    let mut doc = (*st.doc).clone();
    let mask = doc.layer_mut(p.layer)?.mask.as_mut()?;
    let paint = mask.paint.as_mut()?.layers.get_mut(p.index)?;
    paint.opacity = p.opacity;
    let shown = Arc::new(doc);
    p.shown = Some((key, shown.clone()));
    Some((shown, key))
}

/// CPU-only consumers (the UV map builder or unsupported GPU fallback) need the conventional
/// flattened cache. Build it once per opacity value and keep the GPU display snapshot untouched.
pub(crate) fn cpu_document(app: &mut PhotocraftApp, doc: Arc<Document>, key: u64) -> Arc<Document> {
    let revision = app.session.documents().iter().find(|st| st.doc.id == doc.id).map(|st| st.revision);
    let Some(p) = app.uv_mask_paint_preview.as_mut().filter(|p| p.doc == doc.id && Some(p.revision) == revision) else { return doc };
    if let Some((cached, materialized)) = &p.materialized
        && *cached == key
    {
        return materialized.clone();
    }
    let mut cpu = (*doc).clone();
    if let Some(mask) = cpu.layer_mut(p.layer).and_then(|layer| layer.mask.as_mut()) {
        photocraft_compose::mask_paint::refresh(mask, None);
    }
    let cpu = Arc::new(cpu);
    p.materialized = Some((key, cpu.clone()));
    cpu
}

/// Take the latest value so app.run can commit it once before another command consumes the doc.
pub(crate) fn pending(app: &PhotocraftApp) -> Option<Commit> {
    let p = app.uv_mask_paint_preview.as_ref()?;
    Some(Commit { doc: p.doc, revision: p.revision, layer: p.layer, index: p.index, opacity: p.opacity })
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

fn preview_key(revision: u64, layer: LayerId, index: usize, opacity: f32) -> u64 {
    let seed = revision.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ layer.0.rotate_left(19) ^ (index as u64).rotate_left(37) ^ u64::from(opacity.to_bits());
    (seed & (KEY_BASE - 1)) | KEY_BASE
}

#[cfg(test)]
#[path = "uv_mask_paint_preview_tests.rs"]
mod tests;
