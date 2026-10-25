use photocraft_color::PixelFormat;
use photocraft_doc::{
    LayerId, LayerMask,
    mask_paint::{EffectKind, Paint, Stack},
};
use photocraft_raster::Surface;
use serde_json::json;

use super::{cancel, cpu_document, display_doc, set, set_fill, value};
use crate::{PhotocraftApp, Services};

fn app() -> (PhotocraftApp, LayerId) {
    let mut session = photocraft_engine::Session::new();
    session.execute("file.new", json!({"width": 32, "height": 24})).unwrap();
    session.uv.scene = Some(std::sync::Arc::new(photocraft_uv::Scene::default()));
    let id = session.active().unwrap().active_layer.unwrap();
    let state = session.active_mut().unwrap();
    let base = Surface::with_default(PixelFormat::GRAY8, &[0.2]);
    let mut mask = LayerMask::hide_all();
    mask.paint = Some(Stack {
        base,
        layers: vec![Paint {
            name: "Opacity".into(),
            visible: true,
            opacity: 1.0,
            surface: Surface::with_default(PixelFormat::RGBA8, &[1.0, 0.0, 0.0, 0.5]),
            fill_map: None,
            fill_map_size: [0, 0],
            fill_bounds: None,
            kind: EffectKind::Paint,
        }],
        active: 0,
    });
    std::sync::Arc::make_mut(&mut state.doc).layer_mut(id).unwrap().mask = Some(mask);
    (PhotocraftApp::new(session, Services::default()), id)
}

#[test]
fn opacity_preview_is_gpu_document_transient_then_cpu_materializes_when_needed() {
    let (mut app, layer) = app();
    let (revision, history) = (app.session.active().unwrap().revision, app.session.active().unwrap().history.past_len());

    set(&mut app, layer, 0, 0.25);
    let (shown, key) = display_doc(&mut app, 0).expect("transient shown document");
    assert_eq!(shown.layer(layer).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap().layers[0].opacity, 0.25);
    assert_eq!(app.session.active().unwrap().doc.layer(layer).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap().layers[0].opacity, 1.0);
    assert_eq!((app.session.active().unwrap().revision, app.session.active().unwrap().history.past_len()), (revision, history));
    let (again, same_key) = display_doc(&mut app, 0).unwrap();
    assert!(std::sync::Arc::ptr_eq(&shown, &again) && key == same_key);
    assert_eq!(value(&app, layer, 0, 0.0), 0.25);

    let cpu = cpu_document(&mut app, shown, key);
    let mask = cpu.layer(layer).unwrap().mask.as_ref().unwrap();
    let base = Surface::with_default(PixelFormat::GRAY8, &[0.2]).default_pixel()[0];
    let source = Surface::with_default(PixelFormat::RGBA8, &[1.0, 0.0, 0.0, 0.5]).default_pixel();
    let alpha = source[3] * 0.25;
    let expected = Surface::with_default(PixelFormat::GRAY8, &[source[0] * alpha + base * (1.0 - alpha)]).default_pixel();
    assert_eq!(mask.surface.default_pixel(), expected);
    assert_eq!(app.session.active().unwrap().doc.layer(layer).unwrap().mask.as_ref().unwrap().surface.default_pixel(), vec![0.0]);

    // Any command that consumes or changes the document commits the pending value first.
    app.run("layer.select", json!({"layer":layer.0})).unwrap();
    assert_eq!(app.session.active().unwrap().doc.layer(layer).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap().layers[0].opacity, 0.25);
    assert!(app.uv_mask_paint_preview.is_none());
    assert_eq!(app.session.active().unwrap().history.past_len(), history + 1);

    set(&mut app, layer, 0, 0.75);
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(app.session.active().unwrap().doc.layer(layer).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap().layers[0].opacity, 0.25);
    app.run("edit.redo", json!({})).unwrap();
    assert_eq!(app.session.active().unwrap().doc.layer(layer).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap().layers[0].opacity, 0.75);

    set(&mut app, layer, 0, 0.45);
    let _ = app.run("file.save", json!({}));
    assert_eq!(app.session.active().unwrap().doc.layer(layer).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap().layers[0].opacity, 0.45);

    // A direct tab switch is handled by committing into the originating document and restoring
    // the user's selected tab before the new command runs.
    set(&mut app, layer, 0, 0.6);
    let old_doc = app.session.active().unwrap().doc.id;
    app.session.execute("file.new", json!({"width": 16, "height": 12})).unwrap();
    let new_layer = app.session.active().unwrap().active_layer.unwrap();
    app.run("layer.select", json!({"layer":new_layer.0})).unwrap();
    assert_eq!(app.session.active().unwrap().doc.layer(new_layer).unwrap().id, new_layer);
    let old_index = app.session.documents().iter().position(|state| state.doc.id == old_doc).unwrap();
    assert_eq!(app.session.documents()[old_index].doc.layer(layer).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap().layers[0].opacity, 0.6);

    // A genuinely stale revision is reported and retained until explicitly canceled.
    app.session.set_active(old_index);
    set(&mut app, layer, 0, 0.4);
    app.session.active_mut().unwrap().revision += 1;
    assert!(app.run("layer.select", json!({"layer":layer.0})).is_err());
    assert!(app.uv_mask_paint_preview.is_some(), "a stale pending value must not disappear silently");
    cancel(&mut app);
    assert!(app.uv_mask_paint_preview.is_none());
}

#[test]
fn returning_opacity_to_canonical_value_cancels_preview_without_consuming_undo() {
    let (mut app, layer) = app();
    app.run("uv.maskPaint.modify", json!({"layer":layer.0,"index":0,"opacity":0.5})).unwrap();
    let state = app.session.active().unwrap();
    let (revision, history) = (state.revision, state.history.past_len());

    set(&mut app, layer, 0, 0.25);
    assert!(app.uv_mask_paint_preview.is_some());
    set(&mut app, layer, 0, 0.5);

    assert!(app.uv_mask_paint_preview.is_none());
    let state = app.session.active().unwrap();
    assert_eq!((state.revision, state.history.past_len()), (revision, history));
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(app.session.active().unwrap().doc.layer(layer).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap().layers[0].opacity, 1.0);
}

#[test]
fn changing_preview_target_commits_previous_value_then_keeps_new_preview() {
    let (mut app, old_layer) = app();
    set(&mut app, old_layer, 0, 0.25);
    let old_doc = app.session.active().unwrap().doc.id;

    app.session.execute("file.new", json!({"width":16,"height":12})).unwrap();
    let new_layer = app.session.active().unwrap().active_layer.unwrap();
    let state = app.session.active_mut().unwrap();
    let mut mask = LayerMask::hide_all();
    mask.paint = Some(Stack {
        base: Surface::with_default(PixelFormat::GRAY8, &[0.2]),
        layers: vec![Paint {
            name: "Second opacity".into(),
            visible: true,
            opacity: 0.8,
            surface: Surface::with_default(PixelFormat::RGBA8, &[1.0, 0.0, 0.0, 0.5]),
            fill_map: None,
            fill_map_size: [0, 0],
            fill_bounds: None,
            kind: EffectKind::Paint,
        }],
        active: 0,
    });
    std::sync::Arc::make_mut(&mut state.doc).layer_mut(new_layer).unwrap().mask = Some(mask);

    set(&mut app, new_layer, 0, 0.3);

    assert_eq!(app.session.active().unwrap().doc.layer(new_layer).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap().layers[0].opacity, 0.8);
    let preview = app.uv_mask_paint_preview.as_ref().unwrap();
    assert_eq!((preview.doc, preview.layer, preview.index, preview.opacity), (app.session.active().unwrap().doc.id, new_layer, 0, 0.3));
    let old_index = app.session.documents().iter().position(|state| state.doc.id == old_doc).unwrap();
    assert_eq!(app.session.documents()[old_index].doc.layer(old_layer).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap().layers[0].opacity, 0.25);
}

#[test]
fn changing_paint_index_commits_old_preview_and_refreshes_same_document_revision() {
    let (mut app, layer) = app();
    let state = app.session.active_mut().unwrap();
    let paint = state.doc.layer(layer).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap().layers[0].clone();
    std::sync::Arc::make_mut(&mut state.doc).layer_mut(layer).unwrap().mask.as_mut().unwrap().paint.as_mut().unwrap().layers.push(Paint {
        name: "Second opacity".into(),
        visible: true,
        opacity: 0.8,
        surface: paint.surface,
        fill_map: None,
        fill_map_size: [0, 0],
        fill_bounds: None,
        kind: EffectKind::Paint,
    });
    set(&mut app, layer, 0, 0.25);

    set(&mut app, layer, 1, 0.3);

    let state = app.session.active().unwrap();
    let stack = state.doc.layer(layer).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap();
    assert_eq!(stack.layers[0].opacity, 0.25);
    assert_eq!(stack.layers[1].opacity, 0.8);
    let pending = app.uv_mask_paint_preview.as_ref().unwrap();
    assert_eq!((pending.doc, pending.revision, pending.layer, pending.index, pending.opacity), (state.doc.id, state.revision, layer, 1, 0.3));
    assert_eq!(value(&app, layer, 1, stack.layers[1].opacity), 0.3, "the new preview must not be stale after committing the previous value");
}

#[test]
fn stale_foreign_preview_is_retained_and_does_not_apply_new_target() {
    let (mut app, old_layer) = app();
    set(&mut app, old_layer, 0, 0.25);
    let old_doc = app.session.active().unwrap().doc.id;
    app.session.execute("file.new", json!({"width":16,"height":12})).unwrap();
    let new_layer = app.session.active().unwrap().active_layer.unwrap();
    let state = app.session.active_mut().unwrap();
    let mut mask = LayerMask::hide_all();
    mask.paint = Some(Stack {
        base: Surface::with_default(PixelFormat::GRAY8, &[0.2]),
        layers: vec![Paint {
            name: "Second opacity".into(),
            visible: true,
            opacity: 0.8,
            surface: Surface::with_default(PixelFormat::RGBA8, &[1.0, 0.0, 0.0, 0.5]),
            fill_map: None,
            fill_map_size: [0, 0],
            fill_bounds: None,
            kind: EffectKind::Paint,
        }],
        active: 0,
    });
    std::sync::Arc::make_mut(&mut state.doc).layer_mut(new_layer).unwrap().mask = Some(mask);
    let old_index = app.session.documents().iter().position(|state| state.doc.id == old_doc).unwrap();
    let new_index = app.session.active_index().unwrap();
    app.session.set_active(old_index);
    app.session.active_mut().unwrap().revision += 1;
    app.session.set_active(new_index);

    set(&mut app, new_layer, 0, 0.3);

    let pending = app.uv_mask_paint_preview.as_ref().unwrap();
    assert_eq!((pending.doc, pending.layer, pending.index, pending.opacity), (old_doc, old_layer, 0, 0.25));
    assert!(app.ui.status.contains("stale"));
    assert!(app.ui.status_error);
    assert_eq!(app.session.active().unwrap().doc.layer(new_layer).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap().layers[0].opacity, 0.8);
}

#[test]
fn locked_mask_does_not_start_transient_opacity_preview() {
    let (mut app, layer) = app();
    let state = app.session.active_mut().unwrap();
    std::sync::Arc::make_mut(&mut state.doc).layer_mut(layer).unwrap().locks.all = true;
    let (revision, history) = (state.revision, state.history.past_len());

    set(&mut app, layer, 0, 0.25);

    assert!(app.uv_mask_paint_preview.is_none());
    assert_eq!((app.session.active().unwrap().revision, app.session.active().unwrap().history.past_len()), (revision, history));
}

#[test]
fn cpu_fallback_materializes_nested_transient_paint_masks() {
    let (mut app, layer) = app();
    let active = app.session.active_mut().unwrap();
    let doc = std::sync::Arc::make_mut(&mut active.doc);
    let child = doc.layers.pop().unwrap();
    doc.layers.push(photocraft_doc::Layer::group("Nested", vec![child]));
    set(&mut app, layer, 0, 0.25);
    let (shown, _) = display_doc(&mut app, 0).unwrap();
    let cpu = crate::gpu_canvas::materialize_paint_masks(&shown);
    let flattened = cpu.layer(layer).unwrap().mask.as_ref().unwrap().surface.default_pixel();
    assert_eq!(flattened, cpu_document(&mut app, shown.clone(), 0).layer(layer).unwrap().mask.as_ref().unwrap().surface.default_pixel());
    assert!((flattened[0] - 0.3).abs() < 0.01, "fallback must use transient opacity, got {flattened:?}");
    assert_eq!(shown.layer(layer).unwrap().mask.as_ref().unwrap().surface.default_pixel(), vec![0.0], "GPU preview source remains unflattened");
}

#[test]
fn fill_bounds_preview_updates_pixels_and_commits_once_on_release() {
    let (mut app, layer) = app();
    {
        let state = app.session.active_mut().unwrap();
        let paint = std::sync::Arc::make_mut(&mut state.doc).layer_mut(layer).unwrap().mask.as_mut().unwrap().paint.as_mut().unwrap();
        paint.layers[0].kind = EffectKind::Fill { value: 1.0, rotation: 0.0, repeat_u: false, repeat_v: false, mirror_u: false, mirror_v: false };
    }
    let history = app.session.active().unwrap().history.past_len();
    let next = [4, 4, 20, 16];
    set_fill(&mut app, layer, 0, next, 20.0);

    let (shown, key) = display_doc(&mut app, 0).expect("Fill preview snapshot");
    let preview_mask = shown.layer(layer).unwrap().mask.as_ref().unwrap();
    assert!(preview_mask.surface.sample_channel(8, 8, 0) > 0.95, "Fill pixels update before release");
    assert!(preview_mask.surface.sample_channel(2, 2, 0) < 0.25, "pixels outside Fill remain unchanged");
    let cpu = cpu_document(&mut app, shown, key);
    assert!(cpu.layer(layer).unwrap().mask.as_ref().unwrap().surface.sample_channel(8, 8, 0) > 0.95);
    assert_eq!(app.session.active().unwrap().history.past_len(), history, "preview is transient");

    let commit = super::pending(&app).expect("latest Fill preview is pending");
    app.run("uv.maskPaint.modify", commit.params()).unwrap();
    assert!(app.uv_mask_paint_preview.is_none());
    assert_eq!(app.session.active().unwrap().history.past_len(), history + 1, "gesture creates one history entry");
    let committed = app.session.active().unwrap().doc.layer(layer).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap().layers[0].clone();
    assert_eq!(committed.fill_bounds, Some(next));
    assert!(matches!(committed.kind, EffectKind::Fill { rotation, .. } if rotation == 20.0));
    app.run("edit.undo", json!({})).unwrap();
    let undone = app.session.active().unwrap().doc.layer(layer).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap().layers[0].clone();
    assert_eq!(undone.fill_bounds, None);
    assert!(matches!(undone.kind, EffectKind::Fill { rotation, .. } if rotation == 0.0));
}

#[test]
fn fill_transform_preview_recomposes_only_damaged_tiles() {
    let mut session = photocraft_engine::Session::new();
    session.execute("file.new", json!({"width":512,"height":128})).unwrap();
    let layer = session.active().unwrap().active_layer.unwrap();
    let state = session.active_mut().unwrap();
    let mut base = Surface::with_default(PixelFormat::GRAY8, &[0.2]);
    base.write_region(photocraft_geom::Rect::new(300, 20, 301, 21), &[0.8]);
    let mut mask = LayerMask::hide_all();
    mask.paint = Some(Stack {
        base,
        layers: vec![Paint {
            name: "Fill".into(),
            visible: true,
            opacity: 1.0,
            surface: Surface::with_default(PixelFormat::RGBA8, &[0.0, 0.0, 0.0, 0.0]),
            fill_map: None,
            fill_map_size: [0, 0],
            fill_bounds: Some([0, 0, 32, 32]),
            kind: EffectKind::Fill { value: 1.0, rotation: 0.0, repeat_u: false, repeat_v: false, mirror_u: false, mirror_v: false },
        }],
        active: 0,
    });
    photocraft_compose::mask_paint::refresh(&mut mask, Some(state.doc.bounds()));
    std::sync::Arc::make_mut(&mut state.doc).layer_mut(layer).unwrap().mask = Some(mask);
    let mut app = PhotocraftApp::new(session, Services::default());
    set_fill(&mut app, layer, 0, [8, 0, 40, 32], 0.0);

    let (shown, _) = display_doc(&mut app, 0).expect("Fill transform preview");
    let shown_mask = shown.layer(layer).unwrap().mask.as_ref().unwrap();
    let untouched = photocraft_geom::TileCoord::new(1, 0);
    let canonical = app.session.active().unwrap().doc.layer(layer).unwrap().mask.as_ref().unwrap();
    assert!(std::sync::Arc::ptr_eq(canonical.surface.tile(untouched).unwrap(), shown_mask.surface.tile(untouched).unwrap()));
    assert!(shown_mask.surface.sample_channel(34, 8, 0) > 0.95, "new Fill extent is recomposed in the damaged tile");
    assert!(shown_mask.surface.sample_channel(2, 8, 0) < 0.25, "old Fill extent is cleared in the same damage update");
}
