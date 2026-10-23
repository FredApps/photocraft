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
    for p in [json!({"index":999}), json!({"index":0,"opacity":-1}), json!({"index":0,"to":999})] {
        assert!(s.execute("uv.maskPaint.modify", p).is_err());
        assert_eq!(s.active().unwrap().doc, before);
    }
    s.execute("uv.maskPaint.modify", json!({"index":0,"visible":false})).unwrap();
    assert!(s.execute("paint.stroke", json!({"target":"mask","points":[[8,8]],"size":12})).is_err());
}
