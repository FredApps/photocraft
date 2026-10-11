//! #3631: the rollover outline and Auto-Select for the Artboard and Type tools, and the Path
//! Selection and Artboard options-bar toggles, driven through the options bar and tool events.

use egui::vec2;
use egui_kittest::{Harness, kittest::Queryable};
use photocraft_doc::LayerId;
use photocraft_geom::Rect;
use serde_json::json;

use super::target;
use crate::PhotocraftApp;
use crate::canvas::ToolEvent;
use crate::state::{Tool, ToolOptions};

fn new_app(w: u32, h: u32) -> PhotocraftApp {
    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
    app.run("file.new", json!({"width": w, "height": h})).unwrap();
    app.sync_views();
    app.ui.extras.snap = false;
    app.ui.view.show.smart_guides = false;
    app
}

fn active(app: &PhotocraftApp) -> Option<LayerId> {
    app.session.active().unwrap().active_layer
}

fn board(app: &mut PhotocraftApp, rect: [i64; 4]) -> LayerId {
    LayerId(app.run("layer.new.artboard", json!({"rect": rect})).unwrap()["layer"].as_u64().unwrap())
}

fn press(app: &mut PhotocraftApp, from: [f64; 2], to: [f64; 2], mods: egui::Modifiers) {
    crate::canvas::tool_event(app, ToolEvent::Down { x: from[0], y: from[1], pressure: 1.0 }, mods);
    crate::canvas::tool_event(app, ToolEvent::Move { x: to[0], y: to[1], pressure: 1.0 }, mods);
    crate::canvas::tool_event(app, ToolEvent::Up { x: to[0], y: to[1] }, mods);
}

fn board_rect(app: &PhotocraftApp, id: LayerId) -> Rect {
    app.session.active().unwrap().doc.layer(id).unwrap().artboard().unwrap().rect
}

#[test]
fn new_options_default_like_photoshop() {
    let o = ToolOptions::default();
    assert!(!o.path_show_bounding_box, "Photoshop ships Show Bounding Box unchecked");
    assert!(o.artboard_auto_select);
    assert_eq!(o.artboard_target, "artboard");
    let mut v = serde_json::to_value(&o).unwrap();
    for k in ["path_show_bounding_box", "artboard_auto_select", "artboard_target"] {
        v.as_object_mut().unwrap().remove(k).unwrap_or_else(|| panic!("{k} serialised"));
    }
    assert_eq!(serde_json::from_value::<ToolOptions>(v).unwrap(), o, "older saved options get the defaults");
}

#[test]
fn new_options_are_remembered_and_reset() {
    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
    app.ui.tool_options.path_show_bounding_box = true;
    app.ui.tool_options.artboard_auto_select = false;
    app.ui.tool_options.artboard_target = "layer".into();
    crate::tool_memory::persist(&mut app, &egui::Context::default());
    let saved = app.session.prefs().clone();
    let mut next = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
    next.session.edit_prefs(|p| *p = saved);
    crate::tool_memory::restore(&mut next);
    let o = &next.ui.tool_options;
    assert!(o.path_show_bounding_box && !o.artboard_auto_select && o.artboard_target == "layer");
    crate::tool_reset::reset_options(&mut next.ui.tool_options, Tool::PathSelection);
    crate::tool_reset::reset_options(&mut next.ui.tool_options, Tool::Artboard);
    assert_eq!(next.ui.tool_options, ToolOptions::default());
}

#[test]
fn options_bar_checkboxes_drive_the_options() {
    for (tool, label) in [(Tool::PathSelection, "Show Bounding Box"), (Tool::Artboard, "Auto-Select:")] {
        let mut app = new_app(100, 100);
        app.ui.tool = tool;
        let mut h = Harness::builder().with_size(vec2(1440.0, 120.0)).build_ui_state(
            |ui, app: &mut PhotocraftApp| {
                if ui.ctx().fonts(|f| f.families().contains(&egui::FontFamily::Name("medium".into()))) {
                    crate::panels::options_bar(app, ui);
                }
            },
            app,
        );
        crate::PhotocraftApp::setup_context(&h.ctx, crate::theme::ThemeKind::ProMedium);
        h.run_steps(3);
        let get = |app: &PhotocraftApp| {
            if tool == Tool::PathSelection { app.ui.tool_options.path_show_bounding_box } else { app.ui.tool_options.artboard_auto_select }
        };
        let before = get(h.state());
        h.get_by_label(label).click();
        h.run_steps(2);
        assert_eq!(get(h.state()), !before, "{label}");
        h.get_by_label(label).click();
        h.run_steps(2);
        assert_eq!(get(h.state()), before, "{label}");
    }
}

#[test]
fn artboard_auto_select_picks_the_board_or_its_layer() {
    let mut app = new_app(400, 200);
    let a = board(&mut app, [0, 0, 100, 100]);
    let b = board(&mut app, [200, 0, 100, 100]);
    app.run("layer.select", json!({"layer": a.0})).unwrap();
    app.ui.tool = Tool::Artboard;
    // On (the default): a click on B selects it; the rollover outlines B.
    assert_eq!(target(&mut app, Some([250.0, 50.0]), false, false), Some(Rect::new(200, 0, 300, 100)));
    assert_eq!(target(&mut app, Some([150.0, 50.0]), false, false), None, "between boards: nothing");
    press(&mut app, [250.0, 50.0], [250.0, 50.0], egui::Modifiers::NONE);
    assert_eq!(active(&app), Some(b));
    // Off: no rollover, and a drag on board A moves the selected board B instead.
    app.ui.tool_options.artboard_auto_select = false;
    assert_eq!(target(&mut app, Some([50.0, 50.0]), false, false), None);
    assert!(target(&mut app, Some([50.0, 50.0]), true, false).is_some(), "⌘ inverts it");
    press(&mut app, [50.0, 50.0], [50.0, 60.0], egui::Modifiers::NONE);
    assert_eq!(active(&app), Some(b));
    assert_eq!(board_rect(&app, b), Rect::new(200, 10, 300, 110));
    assert_eq!(board_rect(&app, a), Rect::new(0, 0, 100, 100));
    // The rollover preference turns the outline off for every tool.
    app.ui.tool_options.artboard_auto_select = true;
    app.session.edit_prefs(|p| p.tools.show_highlight_on_rollover = false);
    assert_eq!(target(&mut app, Some([250.0, 50.0]), false, false), None);
}

#[test]
fn artboard_layer_target_picks_the_layer_on_the_board() {
    let mut app = new_app(200, 200);
    app.run("layer.new.layer", json!({"name": "Dot"})).unwrap();
    let dot = active(&app).unwrap();
    app.session
        .edit("paint", |doc, a| {
            doc.layer_mut(a.unwrap()).unwrap().surface_mut().unwrap().fill_rect(Rect::new(20, 20, 40, 40), &[1.0, 0.0, 0.0, 1.0]);
            Ok(())
        })
        .unwrap();
    app.run("layer.new.artboardFromLayers", json!({})).unwrap();
    let bd = app.session.active().unwrap().doc.artboard_of(dot).expect("the layer is on a board");
    app.run("layer.select", json!({"layer": bd.0})).unwrap();
    app.ui.tool = Tool::Artboard;
    app.ui.tool_options.artboard_target = "layer".into();
    assert_eq!(target(&mut app, Some([30.0, 30.0]), false, false), Some(Rect::new(20, 20, 40, 40)));
    press(&mut app, [30.0, 30.0], [30.0, 30.0], egui::Modifiers::NONE);
    assert_eq!(active(&app), Some(dot));
}

#[test]
fn type_tool_outlines_the_text_a_click_would_edit() {
    let mut app = new_app(300, 200);
    app.run("type.create", json!({"x": 20, "y": 120, "text": "Hello", "size": 60})).unwrap();
    crate::type_tool::commit(&mut app);
    app.ui.tool = Tool::Type;
    let at = app.session.active().unwrap().doc.layer(active(&app).unwrap()).unwrap().surface().unwrap().content_bounds();
    assert!(!at.is_empty());
    let inside = [f64::from(at.x0 + at.x1) / 2.0, f64::from(at.y0 + at.y1) / 2.0];
    let outlined = target(&mut app, Some(inside), false, false).expect("text under the pointer is outlined");
    assert!(outlined.contains_rect(&at) || outlined == at, "{outlined:?} vs {at:?}");
    assert_eq!(target(&mut app, Some([290.0, 10.0]), false, false), None, "empty spot: a click starts new text");
    app.ui.tool = Tool::VerticalType;
    assert!(target(&mut app, Some(inside), false, false).is_some());
    for mask in [Tool::HorizontalTypeMask, Tool::VerticalTypeMask] {
        app.ui.tool = mask;
        assert_eq!(target(&mut app, Some(inside), false, false), None, "{mask:?}: existing type is only pixels");
    }
}
