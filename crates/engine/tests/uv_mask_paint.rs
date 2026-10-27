#![cfg(feature = "uv-workbench")]
use photocraft_doc::SampleType;
use photocraft_engine::Session;
use serde_json::json;

fn fixture(depth: u8) -> Session {
    let mut s = Session::new();
    s.execute("uv.workspace.activate", json!({"active":true})).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../uv/tests/fixtures/two-shells.obj");
    s.execute("uv.model.import", json!({"path":source})).unwrap();
    s.execute("uv.texture.create", json!({"width":32,"height":32,"depth":depth})).unwrap();
    s.execute("layer.layerMask.revealAll", json!({})).unwrap();
    s
}
fn value(s: &Session) -> f32 {
    let st = s.active().unwrap();
    st.doc.layer(st.active_layer.unwrap()).unwrap().mask.as_ref().unwrap().surface.sample_channel(8, 8, 0)
}
fn paint(s: &mut Session, color: &str) {
    s.execute(
        "paint.stroke",
        json!({"target":"mask","points":[[8.5,8.5,1.0]],"size":12,"color":color,"brush":{"hardness":1.0,"opacity":1.0,"flow":1.0},"seed":42}),
    )
    .unwrap();
}
#[test]
fn paint_stack_visibility_opacity_order_delete_and_undo_at_all_depths() {
    for depth in [8, 16, 32] {
        let mut s = fixture(depth);
        s.execute("uv.maskPaint.add", json!({"name":"Black"})).unwrap();
        paint(&mut s, "#000000");
        assert!(value(&s) < 0.01);
        s.execute("paint.stroke", json!({"target":"mask","points":[[8.5,8.5]],"size":12,"erase":true,"brush":{"hardness":1.0},"seed":42})).unwrap();
        assert!(value(&s) > 0.99, "Eraser removes this Paint layer's coverage");
        s.execute("edit.undo", json!({})).unwrap();
        assert!(value(&s) < 0.01);
        s.execute("uv.maskPaint.modify", json!({"index":0,"opacity":0.5})).unwrap();
        assert!((value(&s) - 0.5).abs() < 0.02);
        s.execute("uv.maskPaint.modify", json!({"index":0,"visible":false})).unwrap();
        assert!(value(&s) > 0.99);
        s.execute("edit.undo", json!({})).unwrap();
        assert!((value(&s) - 0.5).abs() < 0.02);
        s.execute("uv.maskPaint.modify", json!({"index":0,"opacity":1.0})).unwrap();
        s.execute("uv.maskPaint.add", json!({"name":"White"})).unwrap();
        paint(&mut s, "#ffffff");
        assert!(value(&s) > 0.99);
        s.execute("uv.maskPaint.modify", json!({"index":1,"to":0})).unwrap();
        assert!(value(&s) < 0.01);
        s.execute("uv.maskPaint.modify", json!({"index":1,"remove":true})).unwrap();
        assert!(value(&s) > 0.99);
        s.execute("edit.undo", json!({})).unwrap();
        assert!(value(&s) < 0.01);
        s.execute("edit.redo", json!({})).unwrap();
        assert!(value(&s) > 0.99);
    }
}
#[test]
fn live_brush_preview_and_container_keep_editable_layers() {
    let mut s = fixture(16);
    s.execute("uv.maskPaint.add", json!({"name":"Live black"})).unwrap();
    let p = json!({"target":"mask","points":[[8.5,8.5,1.0]],"size":12,"color":"#000000","brush":{"hardness":1.0},"seed":42});
    let live = photocraft_engine::brush_cmds::LiveStroke::begin(&s, &p).unwrap();
    let id = s.active().unwrap().active_layer.unwrap();
    assert!(live.doc.layer(id).unwrap().mask.as_ref().unwrap().surface.sample_channel(8, 8, 0) < 0.01);
    assert!(value(&s) > 0.99, "Preview must not change history's document");
    s.offer_live_stroke(live).unwrap();
    s.execute("paint.stroke", p).unwrap();
    assert!(value(&s) < 0.01);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paint.pcuv");
    s.execute("uv.project.save", json!({"path":path})).unwrap();
    let mut restored = Session::new();
    restored.execute("uv.project.load", json!({"path":path})).unwrap();
    assert!(value(&restored) < 0.01);
    let st = restored.active().unwrap();
    let stack = st.doc.layer(st.active_layer.unwrap()).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap();
    assert_eq!(stack.layers[0].name, "Live black");
    assert_eq!(stack.layers[0].surface.format().sample, SampleType::U16);
    restored.execute("uv.maskPaint.modify", json!({"index":0,"remove":true})).unwrap();
    assert!(value(&restored) > 0.99);
}
#[test]
fn invalid_stack_edits_preserve_document() {
    let mut s = fixture(8);
    s.execute("uv.maskPaint.add", json!({})).unwrap();
    let before = s.active().unwrap().doc.clone();
    for p in [json!({"index":999}), json!({"index":0,"opacity":-1}), json!({"index":0,"to":999}), json!({"index":0,"repeat_u":false})] {
        assert!(s.execute("uv.maskPaint.modify", p).is_err());
        assert_eq!(s.active().unwrap().doc, before);
    }
    s.execute("uv.maskPaint.modify", json!({"index":0,"visible":false})).unwrap();
    assert!(s.execute("paint.stroke", json!({"target":"mask","points":[[8,8]],"size":12})).is_err());
}

#[test]
fn removing_an_effect_below_the_active_effect_keeps_the_same_effect_selected() {
    let mut s = fixture(8);
    for name in ["First", "Second", "Third"] {
        s.execute("uv.maskPaint.add", json!({"name":name})).unwrap();
    }
    s.execute("uv.maskPaint.modify", json!({"index":2})).unwrap();
    s.execute("uv.maskPaint.modify", json!({"index":0,"remove":true})).unwrap();
    let stack = s.active().unwrap().doc.layer(s.active().unwrap().active_layer.unwrap()).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap();
    assert_eq!(stack.active, 1);
    assert_eq!(stack.layers[stack.active].name, "Third");
}

#[test]
fn transform_of_mask_cache_is_rejected_without_partially_mutating_the_document() {
    let mut s = fixture(8);
    s.execute("uv.maskPaint.add", json!({"name":"Paint"})).unwrap();
    let before = s.active().unwrap().doc.clone();
    let result = s.execute("edit.transform", json!({"dx":4,"dy":2,"target":"mask"}));
    assert!(result.is_err(), "legacy mask transforms must not be silently lost behind the editable stack");
    assert_eq!(s.active().unwrap().doc, before, "rejection leaves document and history untouched");

    let layer = s.active().unwrap().active_layer.unwrap();
    s.edit("Link test mask", |doc, _| {
        doc.layer_mut(layer).unwrap().mask.as_mut().unwrap().linked = true;
        Ok(serde_json::Value::Null)
    })
    .unwrap();
    let before_linked_transform = s.active().unwrap().doc.clone();
    let result = s.execute("edit.transform", json!({"dx":4,"dy":2}));
    assert!(result.is_err(), "linked layer transforms must not detach the editable stack from its cache");
    assert_eq!(s.active().unwrap().doc, before_linked_transform, "linked transform rejection is atomic too");
}

#[test]
fn fill_and_levels_layers_compose_modify_undo_and_round_trip() {
    let mut s = fixture(16);
    s.execute("uv.maskPaint.add", json!({"kind":"fill","value":0.25})).unwrap();
    s.execute("uv.maskPaint.modify", json!({"index":0,"rotation":37.0,"repeat_u":false,"repeat_v":true,"mirror_u":true,"mirror_v":false})).unwrap();
    assert!((value(&s) - 0.25).abs() < 0.01);
    s.execute("uv.maskPaint.add", json!({"kind":"levels","input_black":0.0,"gamma":1.0,"input_white":0.5,"output_black":0.0,"output_white":1.0})).unwrap();
    assert!((value(&s) - 0.5).abs() < 0.01);
    s.execute("uv.maskPaint.modify", json!({"index":1,"output_white":0.8,"opacity":0.5})).unwrap();
    assert!((value(&s) - 0.325).abs() < 0.02);
    let before_invalid = s.active().unwrap().doc.clone();
    for params in [json!({"index":1,"gamma":0.0}), json!({"index":1,"input_black":0.8}), json!({"index":0,"gamma":2.0}), json!({"index":0,"rotation":36001.0})]
    {
        assert!(s.execute("uv.maskPaint.modify", params).is_err());
        assert_eq!(s.active().unwrap().doc, before_invalid);
    }
    s.execute("edit.undo", json!({})).unwrap();
    assert!((value(&s) - 0.5).abs() < 0.01);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("effects.pcuv");
    s.execute("uv.project.save", json!({"path":path})).unwrap();
    let mut restored = Session::new();
    restored.execute("uv.project.load", json!({"path":path})).unwrap();
    assert!((value(&restored) - 0.5).abs() < 0.01);
    let stack = restored.active().unwrap().doc.layer(restored.active().unwrap().active_layer.unwrap()).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap();
    assert!(matches!(&stack.layers[0].kind, photocraft_doc::mask_paint::EffectKind::Fill { value, .. } if (*value - 0.25).abs() < f32::EPSILON));
    assert!(matches!(
        &stack.layers[0].kind,
        photocraft_doc::mask_paint::EffectKind::Fill { repeat_u: false, repeat_v: true, mirror_u: true, mirror_v: false, .. }
    ));
    assert!(matches!(&stack.layers[0].kind, photocraft_doc::mask_paint::EffectKind::Fill { rotation, .. } if (*rotation - 37.0).abs() < f32::EPSILON));
    assert!(matches!(&stack.layers[1].kind, photocraft_doc::mask_paint::EffectKind::Levels { input_white, .. } if (*input_white - 0.5).abs() < f32::EPSILON));
}

#[test]
fn removing_fill_image_clears_source_and_is_undoable() {
    let mut s = fixture(8);
    s.execute("uv.maskPaint.add", json!({"kind":"fill","value":0.4})).unwrap();
    let layer = s.active().unwrap().active_layer.unwrap();
    s.edit("Set test Fill image", |doc, _| {
        let fill = &mut doc.layer_mut(layer).unwrap().mask.as_mut().unwrap().paint.as_mut().unwrap().layers[0];
        fill.fill_map = Some(photocraft_raster::Surface::with_default(photocraft_color::PixelFormat::GRAY8, &[0.7]));
        fill.fill_map_size = [16, 16];
        Ok(serde_json::Value::Null)
    })
    .unwrap();

    s.execute("uv.maskPaint.modify", json!({"index":0,"clear_fill_map":true})).unwrap();
    let fill = &s.active().unwrap().doc.layer(layer).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap().layers[0];
    assert!(fill.fill_map.is_none());
    assert_eq!(fill.fill_map_size, [0, 0]);
    s.execute("edit.undo", json!({})).unwrap();
    let fill = &s.active().unwrap().doc.layer(layer).unwrap().mask.as_ref().unwrap().paint.as_ref().unwrap().layers[0];
    assert!(fill.fill_map.is_some(), "undo restores the image source");
    assert_eq!(fill.fill_map_size, [16, 16]);
}

#[test]
fn opacity_drag_is_one_undo_and_does_not_rewrite_untouched_mask_tiles() {
    let mut s = fixture(16);
    let id = s.active().unwrap().active_layer.unwrap();
    s.edit("Dense base", |doc, _| {
        doc.size = photocraft_geom::Size::new(1024, 1024);
        let mask = doc.layer_mut(id).unwrap().mask.as_mut().unwrap();
        mask.surface.write_region(photocraft_geom::Rect::from_xywh(0, 0, 1024, 1024), &vec![0.75; 1024 * 1024]);
        Ok(())
    })
    .unwrap();
    s.execute("uv.maskPaint.add", json!({})).unwrap();
    paint(&mut s, "#000000");
    s.execute("uv.maskPaint.add", json!({"name":"Far Paint"})).unwrap();
    s.execute("paint.stroke", json!({"target":"mask","points":[[800.5,800.5]],"size":12,"color":"#000000","brush":{"hardness":1.0}})).unwrap();
    s.execute("uv.maskPaint.select", json!({"index":0})).unwrap();
    let distant = photocraft_geom::TileCoord::new(3, 3);
    let before = s.active().unwrap().doc.layer(id).unwrap().mask.as_ref().unwrap().surface.tile(distant).unwrap().clone();
    let start = std::time::Instant::now();
    for opacity in [0.9, 0.8, 0.7] {
        s.execute("uv.maskPaint.modify", json!({"index":0,"opacity":opacity,"coalesce":"opacity-drag"})).unwrap();
    }
    println!("Three opacity updates on a dense 1K base: {:?}", start.elapsed());
    assert!(!s.active().unwrap().last_damage.unwrap().contains(768, 768), "Opacity must not invalidate the distant viewport region");
    let after = s.active().unwrap().doc.layer(id).unwrap().mask.as_ref().unwrap().surface.tile(distant).unwrap().clone();
    assert!(std::sync::Arc::ptr_eq(&before, &after), "Opacity must preserve cache tiles outside Paint coverage");
    s.execute("edit.undo", json!({})).unwrap();
    assert!(value(&s) < 0.01, "One undo restores opacity before the whole drag");
}
