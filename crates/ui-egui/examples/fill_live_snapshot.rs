//! Offscreen visual check of a mask Fill while its rotation gesture is still held.
#[cfg(not(feature = "uv-workbench"))]
fn main() {}

#[cfg(feature = "uv-workbench")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use egui::{Event, Modifiers, PointerButton, vec2};
    use photocraft_engine::Session;
    use photocraft_ui_egui::{PhotocraftApp, Services, canvas::ViewXform, state::Tool};
    use serde_json::json;

    let mut session = Session::new();
    session.execute("uv.workspace.activate", json!({"active":true}))?;
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../uv/tests/fixtures/two-shells.obj");
    session.execute("uv.model.import", json!({"path":source}))?;
    session.execute("uv.texture.create", json!({"width":128,"height":128}))?;
    session.execute("edit.fill", json!({"color":"#649ecf"}))?;
    session.execute("layer.layerMask.hideAll", json!({}))?;
    session.execute("uv.maskPaint.add", json!({"kind":"fill","value":1.0}))?;
    session.execute("uv.maskPaint.modify", json!({"index":0,"bounds":[16,16,112,112]}))?;
    session.execute("view.layerMask", json!({"mode":"gray"}))?;
    let mut h = egui_kittest::Harness::builder().with_size(vec2(1200.0, 900.0)).wgpu().build_eframe(move |cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        let mut app = PhotocraftApp::new(session, Services::default());
        if let Some(render) = &cc.wgpu_render_state {
            let _ = app.set_uv_wgpu(render.clone());
        }
        app.ui.workspace = "3D".into();
        app.ui.mask_target = true;
        app.ui.tool = Tool::Move;
        app.ui.uv_workbench.view_3d = false;
        app.ui.uv_workbench.split_view = false;
        app.ui.dock_tabs.properties = 0;
        app
    });
    h.state_mut().sync_views();
    if let Some(index) = h.state().session.active_index()
        && let Some(view) = h.state_mut().ui.views.get_mut(index)
    {
        view.zoom = 3.0;
        view.center = [64.0, 64.0];
        view.fit_pending = false;
        view.fill_pending = false;
    }
    h.run_steps(6);
    let xf = ViewXform::active(h.state()).ok_or("No canvas transform")?;
    let top = xf.to_screen(64.0, 16.0);
    let start = if xf.rect.contains(top - vec2(0.0, 24.0)) { top - vec2(0.0, 24.0) } else { top + vec2(0.0, 24.0) };
    let end = start + vec2(70.0, 25.0);
    h.hover_at(start);
    h.step();
    h.event(Event::PointerButton { pos: start, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.step();
    h.event(Event::PointerMoved(end));
    h.step();
    h.step();
    if h.state().ui.transform.is_some() {
        return Err("Fill gesture opened a native parent transform".into());
    }
    let path = std::env::args().nth(1).unwrap_or_else(|| "D:/PhotoCraft-UV-build/Screenshots/fill-live-input.png".into());
    h.render()?.save(&path)?;
    println!("Saved live Fill gesture: {path}");
    Ok(())
}
