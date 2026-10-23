#![cfg(feature = "uv-workbench")]
use photocraft_engine::Session;
use serde_json::json;

fn file() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "photocraft-reimport-{}-{}.obj",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::write(&path, obj("A", "B", 1.0)).unwrap();
    path
}
fn obj(first: &str, second: &str, x: f32) -> String {
    format!("v 0 0 0\nv {x} 0 0\nv 0 1 0\nv 1 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nvt 1 1\nusemtl {first}\nf 1/1 2/2 3/3\nusemtl {second}\nf 2/2 4/4 3/3\n")
}
#[test]
fn reimport_retains_documents_layers_history_and_maps_reordered_materials_by_name() {
    let path = file();
    let mut s = Session::new();
    s.execute("uv.model.import", json!({"path":path})).unwrap();
    let a = s.uv.scene.as_ref().unwrap().materials.iter().position(|m| m.name == "A").unwrap();
    s.execute("uv.texture.create", json!({"material":a,"channel":"normal","width":16,"height":16,"depth":16})).unwrap();
    let normal = s.uv.normal_bindings[a];
    s.execute("uv.texture.create", json!({"material":a,"channel":"baseColor","width":16,"height":16,"depth":16})).unwrap();
    s.execute("edit.fill", json!({"color":"#ff0000"})).unwrap();
    s.execute("uv.material.visible", json!({"material":a,"visible":false})).unwrap();
    let id = s.uv.bindings[a];
    let docs = s.documents().len();
    let before = s.active().unwrap().doc.clone();
    assert_eq!(s.execute("uv.model.checkSource", json!({})).unwrap()["changed"], false);
    std::fs::write(&path, obj("B", "A", 2.25)).unwrap();
    assert_eq!(s.execute("uv.model.checkSource", json!({})).unwrap()["changed"], true);
    s.execute("uv.model.reimport", json!({})).unwrap();
    let new_a = s.uv.scene.as_ref().unwrap().materials.iter().position(|m| m.name == "A").unwrap();
    assert_eq!(s.uv.bindings[new_a], id);
    assert_eq!(s.uv.normal_bindings[new_a], normal);
    assert_eq!(docs, s.documents().len());
    assert!(s.uv.hidden_materials.contains(&new_a));
    assert!(std::sync::Arc::ptr_eq(&before, &s.active().unwrap().doc));
    assert!(s.uv.scene.as_ref().unwrap().positions.iter().any(|p| p[0] == 2.25));
    assert_eq!(s.execute("uv.model.checkSource", json!({})).unwrap()["changed"], false);
    s.execute("edit.undo", json!({})).unwrap();
    assert_ne!(s.active().unwrap().doc.layers[0].surface().unwrap().rgba(8, 8), before.layers[0].surface().unwrap().rgba(8, 8));
    std::fs::remove_file(path).unwrap();
}
#[test]
fn project_retains_source_stamp_and_legacy_projects_can_establish_a_baseline() {
    let path = file();
    let project = path.with_extension("pcuv");
    let mut s = Session::new();
    s.execute("uv.model.import", json!({"path":path})).unwrap();
    s.execute("uv.project.save", json!({"path":project})).unwrap();
    std::fs::write(&path, obj("A", "B", 3.125)).unwrap();
    let mut loaded = Session::new();
    loaded.execute("uv.project.load", json!({"path":project})).unwrap();
    assert_eq!(loaded.execute("uv.model.checkSource", json!({})).unwrap()["changed"], true);
    loaded.uv.source_stamp = None;
    assert_eq!(loaded.execute("uv.model.checkSource", json!({})).unwrap()["changed"], false);
    std::fs::write(&path, obj("A", "B", 4.625)).unwrap();
    assert_eq!(loaded.execute("uv.model.checkSource", json!({})).unwrap()["changed"], true);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(project).unwrap();
}
#[test]
fn failed_or_partial_reimport_keeps_previous_mesh_and_missing_source_is_reported() {
    let path = file();
    let mut s = Session::new();
    s.execute("uv.model.import", json!({"path":path})).unwrap();
    let old = s.uv.scene.clone().unwrap();
    std::fs::write(&path, b"partial export").unwrap();
    assert!(s.execute("uv.model.reimport", json!({})).is_err());
    assert!(std::sync::Arc::ptr_eq(&old, s.uv.scene.as_ref().unwrap()));
    assert!(s.execute("uv.model.reimport", json!({"uvSet":-1})).is_err());
    std::fs::remove_file(&path).unwrap();
    assert_eq!(s.execute("uv.model.checkSource", json!({})).unwrap()["available"], false);
    assert!(s.execute("uv.model.reimport", json!({})).is_err());
    assert!(std::sync::Arc::ptr_eq(&old, s.uv.scene.as_ref().unwrap()));
}
