#[cfg(not(feature = "uv-workbench"))]
fn main() {}
#[cfg(feature = "uv-workbench")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use egui_kittest::kittest::Queryable;
    use photocraft_engine::Session;
    use photocraft_ui_egui::{PhotocraftApp, Services};
    use serde_json::json;
    let mut session = Session::new();
    session.execute("uv.workspace.activate", json!({"active":true}))?;
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../uv/tests/fixtures/two-shells.obj");
    session.execute("uv.model.import", json!({"path":source}))?;
    session.execute("uv.texture.create", json!({"width":128,"height":128}))?;
    session.execute("edit.fill", json!({"color":"#649ecf"}))?;
    session.execute("layer.layerMask.revealAll", json!({}))?;
    session.execute("uv.maskPaint.add", json!({"name":"Paint 1"}))?;
    session.execute("paint.stroke", json!({"target":"mask","points":[[30,30],[60,60]],"size":20,"color":"#000000"}))?;
    session.execute("uv.maskPaint.add", json!({"name":"Paint 2"}))?;
    let mut h =
        egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(64).wgpu().build_eframe(move |cc| {
            PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
            let mut app = PhotocraftApp::new(session, Services::default());
            if let Some(rs) = &cc.wgpu_render_state {
                let _ = app.set_uv_wgpu(rs.clone());
            }
            app.ui.workspace = "3D".into();
            app.ui.mask_target = true;
            app.ui.uv_workbench.select_shells = false;
            app.ui.uv_workbench.split_view = true;
            app
        });
    h.state_mut().sync_views();
    for _ in 0..12 {
        h.step();
    }
    if h.query_by_label("Paint 1").is_none() {
        return Err("Mask Paint layer list did not expand".into());
    }
    h.render()?.save("D:/PhotoCraft-UV-build/Screenshots/mask-paint-expanded.png")?;
    h.state_mut().ui.mask_target = false;
    for _ in 0..4 {
        h.step();
    }
    if h.query_by_label("Paint 1").is_some() {
        return Err("Mask Paint layer list did not collapse".into());
    }
    h.render()?.save("D:/PhotoCraft-UV-build/Screenshots/mask-paint-collapsed.png")?;
    println!("Mask Paint snapshots saved.");
    Ok(())
}
