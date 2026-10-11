use crate::{PhotocraftApp, Services};
use egui::{Key, Modifiers};
use photocraft_doc::{LayerId, LayerMask};
use photocraft_geom::Rect;
use serde_json::json;
use std::sync::{Arc, Mutex};

type Os = Arc<Mutex<Option<(u32, u32, Vec<u8>)>>>;

fn app(os: &Os) -> (PhotocraftApp, LayerId) {
    let (a, b) = (os.clone(), os.clone());
    let mut app = PhotocraftApp::new(
        photocraft_engine::Session::new(),
        Services {
            clipboard_set_image: Some(Box::new(move |w, h, bytes| {
                *a.lock().unwrap() = Some((w, h, bytes.to_vec()));
                Ok(())
            })),
            clipboard_get_image: Some(Box::new(move || Ok(b.lock().unwrap().clone()))),
            ..Default::default()
        },
    );
    app.run("file.new", json!({"width": 64, "height": 48})).unwrap();
    app.run("layer.new.layer", json!({"name": "Masked colour"})).unwrap();
    let id = app.session.active().unwrap().active_layer.unwrap();
    app.session
        .edit("fixture", |doc, _| {
            let l = doc.layer_mut(id).unwrap();
            l.surface_mut().unwrap().fill_rect(Rect::new(8, 8, 32, 24), &[1.0, 0.0, 0.0, 1.0]);
            let mut m = LayerMask::reveal_all();
            m.surface.fill_rect(Rect::new(8, 8, 20, 24), &[0.0]);
            m.surface.fill_rect(Rect::new(20, 8, 24, 24), &[0.5]);
            l.mask = Some(m);
            Ok(())
        })
        .unwrap();
    app.sync_views();
    (app, id)
}

fn mask(app: &PhotocraftApp, id: LayerId) -> Option<&LayerMask> {
    app.session.active().unwrap().doc.layer(id).unwrap().mask.as_ref()
}

#[test]
fn ctrl_c_ctrl_v_keeps_the_mask_through_the_os_bitmap_bridge() {
    let os = Os::default();
    let (app, src) = app(&os);
    let original = mask(&app, src).unwrap().clone();
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).with_max_steps(64).build_eframe(move |cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        app
    });
    h.run_steps(4);
    h.key_press_modifiers(Modifiers::COMMAND, Key::C);
    h.run_steps(2);
    assert!(h.state().session.clipboard.as_ref().unwrap().layers.is_some());
    let (width, height, bytes) = os.lock().unwrap().clone().unwrap();
    let bounds = h.state().session.clipboard.as_ref().unwrap().bounds;
    assert_eq!((width, height), (bounds.width(), bounds.height()));
    let alpha = |x, y| {
        assert!(bounds.contains(x, y), "clipboard bounds include ({x}, {y})");
        let pixel = (y - bounds.y0) as usize * width as usize + (x - bounds.x0) as usize;
        bytes[pixel * 4 + 3]
    };
    assert_eq!(alpha(8, 8), 0, "the native bitmap flattens the hidden half of the mask");
    assert!((126..=129).contains(&alpha(20, 8)), "the native bitmap keeps the partial mask");
    assert_eq!(alpha(24, 8), 255, "the native bitmap keeps the visible part");
    let before = h.state().session.active().unwrap().doc.layer_count();
    h.key_press_modifiers(Modifiers::COMMAND, Key::V);
    h.run_steps(2);
    let st = h.state().session.active().unwrap();
    assert_eq!(st.doc.layer_count(), before + 1, "one key press pastes one layer");
    let pasted = st.active_layer.unwrap();
    assert_ne!(pasted, src);
    assert!(mask(h.state(), pasted) == Some(&original));
    assert!(h.state().session.clipboard.as_ref().unwrap().layers.is_some(), "our OS image must not replace rich internal data");
}

#[test]
fn a_new_external_image_or_text_discards_the_old_layer_payload() {
    let os = Os::default();
    let (mut app, _) = app(&os);
    app.run("edit.copy", json!({})).unwrap();
    *os.lock().unwrap() = Some((2, 2, [0, 255, 0, 255].repeat(4)));
    app.run("edit.paste", json!({})).unwrap();
    let id = app.session.active().unwrap().active_layer.unwrap();
    assert!(mask(&app, id).is_none());
    assert!(app.session.clipboard.as_ref().unwrap().layers.is_none());
    *os.lock().unwrap() = None;
    let count = app.session.active().unwrap().doc.layer_count();
    assert_eq!(app.run("edit.paste", json!({})).unwrap()["pasted"], false);
    assert_eq!(app.session.active().unwrap().doc.layer_count(), count);
    assert!(app.session.clipboard.is_none());
}

#[test]
fn same_sized_external_image_with_a_changed_pixel_replaces_the_layer_payload() {
    let os = Os::default();
    let (mut app, _) = app(&os);
    app.run("edit.copy", json!({})).unwrap();
    assert!(app.session.clipboard.as_ref().unwrap().layers.is_some());
    {
        let mut image = os.lock().unwrap();
        let (_, _, bytes) = image.as_mut().unwrap();
        // The old signature sampled bytes 0, 997, ... and missed this alpha change.
        bytes[7] ^= 255;
    }
    app.run("edit.paste", json!({})).unwrap();
    let id = app.session.active().unwrap().active_layer.unwrap();
    assert!(mask(&app, id).is_none());
    assert!(app.session.clipboard.as_ref().unwrap().layers.is_none());
}
