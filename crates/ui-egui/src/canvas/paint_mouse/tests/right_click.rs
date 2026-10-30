//! What counts as a right-click on the canvas (#10183): a right press that went down on the
//! canvas itself, released over it, with no gesture using it in between.

use egui::pos2;

use super::*;

/// The open popup's position: the Pen's canvas tool menu, or else the Brush Preset picker.
fn popup_at(h: &Harness<'static, PhotocraftApp>, pen: bool) -> Option<[f32; 2]> {
    if pen { h.state().ui.canvas_tool_menu.as_ref().map(|m| m.pos) } else { h.state().ui.brush_picker }
}

/// A right press on a popup over the canvas (the Brush Preset picker, the Pen's menu) is the
/// popup's, even when the move off it and the release arrive in the same frame: it never reopens
/// the popup where the button came up.
#[test]
fn a_right_press_on_a_popup_over_the_canvas_is_no_canvas_click() {
    for pen in [false, true] {
        for split in [true, false] {
            let mut h = harness(None);
            if pen {
                h.state_mut().ui.tool = Tool::Pen;
                h.run_steps(2);
            }
            let c = h.state().last_canvas_rect.center();
            right_click_in_one_frame(&mut h, c, c, true);
            assert_eq!(popup_at(&h, pen), Some([c.x, c.y]), "pen {pen}: the popup opens at the click");
            let id = if pen { egui::Id::unique("canvas-selection-menu") } else { crate::brush_picker::picker_id() };
            let popup = h.ctx.memory(|m| m.area_rect(id)).expect("the popup is shown");
            let canvas = h.state().last_canvas_rect;
            let (on, off) = (popup.center(), canvas.left_top() + vec2(30.0, 30.0));
            assert!(canvas.contains(on) && !popup.contains(off));
            right_click_in_one_frame(&mut h, on, off, split);
            assert_ne!(popup_at(&h, pen), Some([off.x, off.y]), "pen {pen}, split {split}: a press on the popup is no canvas right-click");
            assert!(strokes(&h).is_empty());
        }
    }
}

/// The scrollbars are drawn over the canvas edges and take the pointer there: a right-click on
/// one, still or moved, opens neither the Brush Preset picker nor a canvas menu.
#[test]
fn a_right_click_on_a_canvas_scrollbar_opens_no_menu() {
    for tool in [Tool::Brush, Tool::Pen] {
        let mut h = harness(None);
        h.state_mut().ui.tool = tool;
        h.state_mut().ui.views[0].zoom = 6.0;
        h.run_steps(4);
        let r = h.state().last_canvas_rect;
        let bars = [(pos2(r.center().x, r.bottom() - 3.0), vec2(9.0, 0.0)), (pos2(r.right() - 3.0, r.center().y), vec2(0.0, 9.0))];
        for (p, along) in bars {
            for (to, split) in [(p, true), (p + along, true), (p + along, false)] {
                right_click_in_one_frame(&mut h, p, to, split);
                assert_eq!(h.state().ui.brush_picker, None, "{tool:?} at {p:?} to {to:?}, split {split}: no picker");
                assert!(h.state().ui.canvas_tool_menu.is_none(), "{tool:?} at {p:?} to {to:?}, split {split}: no menu");
            }
        }
        // Inside the bars a right-click is still one.
        right_click_in_one_frame(&mut h, r.center(), r.center(), true);
        assert!(h.state().ui.brush_picker.is_some() || h.state().ui.canvas_tool_menu.is_some(), "{tool:?}: a right-click on the image opens its menu");
    }
}

/// One pointer button event in its own frame.
fn button(h: &mut Harness<'static, PhotocraftApp>, pos: Pos2, button: PointerButton, pressed: bool) {
    h.event(egui::Event::PointerButton { pos, button, pressed, modifiers: Modifiers::NONE });
    h.run_steps(1);
}

/// A pointer move by `by` from `from`, in `steps` frames; returns where it ends.
fn move_by(h: &mut Harness<'static, PhotocraftApp>, from: Pos2, by: egui::Vec2, steps: usize) -> Pos2 {
    for i in 1..=steps {
        h.event(egui::Event::PointerMoved(from + by * (i as f32 / steps as f32)));
        h.run_steps(1);
    }
    from + by
}

fn space(h: &mut Harness<'static, PhotocraftApp>, pressed: bool) {
    h.event(egui::Event::Key { key: egui::Key::Space, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(1);
}

/// A harness zoomed in far enough to pan, with `tool`.
fn zoomed(tool: Tool, prefs: Option<&str>) -> Harness<'static, PhotocraftApp> {
    let mut h = harness(prefs);
    h.state_mut().ui.tool = tool;
    h.state_mut().ui.views[0].zoom = 4.0;
    h.run_steps(2);
    h
}

fn nothing_opened(h: &Harness<'static, PhotocraftApp>) -> bool {
    h.state().ui.brush_picker.is_none() && h.state().ui.canvas_tool_menu.is_none()
}

/// Space is the Hand over any tool, and a right drag with it pans, as before: only the selected
/// Hand's right button is its view menu. The pan opens nothing and paints nothing.
#[test]
fn space_and_a_right_drag_pan_with_any_tool() {
    for (tool, prefs) in [(Tool::Brush, None), (Tool::Brush, Some("erase")), (Tool::Pen, None)] {
        let mut h = zoomed(tool, prefs);
        let center = h.state().ui.views[0].center;
        space(&mut h, true);
        right_drag_by(&mut h, vec2(60.0, 36.0), Modifiers::NONE);
        space(&mut h, false);
        assert_ne!(h.state().ui.views[0].center, center, "{tool:?} {prefs:?}: Space and a right drag pan");
        assert!(nothing_opened(&h), "{tool:?} {prefs:?}: and open nothing");
        assert!(strokes(&h).is_empty(), "{tool:?} {prefs:?}: nor paint");
    }
}

/// A Space+right pan whose Space comes up before the right button: the release is the end of a
/// pan, not a right-click, so it neither erases (Erase preference) nor opens the picker.
#[test]
fn letting_go_of_space_before_the_right_button_ends_the_pan_only() {
    for prefs in [Some("erase"), None] {
        let mut h = zoomed(Tool::Brush, prefs);
        h.state_mut().run("edit.fill", json!({"color": "#ff0000"})).unwrap();
        h.run_steps(2);
        let center = h.state().ui.views[0].center;
        let c = h.state().last_canvas_rect.center();
        space(&mut h, true);
        h.event(egui::Event::PointerMoved(c));
        h.run_steps(1);
        button(&mut h, c, PointerButton::Secondary, true);
        let p = move_by(&mut h, c, vec2(40.0, 0.0), 4);
        space(&mut h, false);
        let p = move_by(&mut h, p, vec2(20.0, 0.0), 2);
        button(&mut h, p, PointerButton::Secondary, false);
        h.run_steps(2);
        assert!(strokes(&h).is_empty(), "{prefs:?}: the release erases nothing");
        assert!(alpha_at(&h, p) > 0.0, "{prefs:?}: the layer is untouched where it came up");
        assert!(nothing_opened(&h), "{prefs:?}: and opens nothing");
        assert_ne!(h.state().ui.views[0].center, center, "{prefs:?}: the view panned while Space was held");
    }
}

/// With the Hand, a left pan and the right button chorded in, either way round: the right button
/// took part in the pan, so its release opens no view menu, and the view pans.
#[test]
fn a_hand_pan_with_the_right_button_chorded_in_opens_no_menu() {
    for right_first in [false, true] {
        let mut h = zoomed(Tool::Hand, None);
        let center = h.state().ui.views[0].center;
        let c = h.state().last_canvas_rect.center();
        h.event(egui::Event::PointerMoved(c));
        h.run_steps(1);
        let (first, second) = if right_first { (PointerButton::Secondary, PointerButton::Primary) } else { (PointerButton::Primary, PointerButton::Secondary) };
        button(&mut h, c, first, true);
        let p = move_by(&mut h, c, vec2(40.0, 0.0), 4);
        button(&mut h, p, second, true);
        let p = move_by(&mut h, p, vec2(40.0, 0.0), 4);
        // The right button comes up first while the left still pans, or last.
        let order = if right_first { [PointerButton::Primary, PointerButton::Secondary] } else { [PointerButton::Secondary, PointerButton::Primary] };
        for b in order {
            button(&mut h, p, b, false);
        }
        h.run_steps(2);
        assert_ne!(h.state().ui.views[0].center, center, "right first {right_first}: the view pans");
        assert!(h.state().ui.canvas_tool_menu.is_none(), "right first {right_first}: no view menu");
    }
}

/// The selected Hand pans with the back and forward buttons, as with any button but the right.
#[test]
fn the_hand_pans_with_the_extra_mouse_buttons() {
    for b in [PointerButton::Extra1, PointerButton::Extra2] {
        let mut h = zoomed(Tool::Hand, None);
        let center = h.state().ui.views[0].center;
        drag(&mut h, b);
        assert_ne!(h.state().ui.views[0].center, center, "{b:?} pans");
        assert!(h.state().ui.canvas_tool_menu.is_none());
    }
}

/// A right press while the left button is painting is no right-click of the canvas's own: only
/// egui's, a still click, opens the picker then, as it always has. The stroke commits whole.
#[test]
fn a_right_press_moved_during_a_left_stroke_opens_no_picker() {
    for moved in [true, false] {
        let mut h = harness(None);
        let c = h.state().last_canvas_rect.center();
        h.event(egui::Event::PointerMoved(c));
        h.run_steps(1);
        button(&mut h, c, PointerButton::Primary, true);
        let p = move_by(&mut h, c, vec2(40.0, 0.0), 4);
        button(&mut h, p, PointerButton::Secondary, true);
        let q = if moved { move_by(&mut h, p, vec2(0.0, 40.0), 4) } else { p };
        button(&mut h, q, PointerButton::Secondary, false);
        h.run_steps(1);
        assert_eq!(h.state().ui.brush_picker.is_some(), !moved, "moved {moved}: the picker opens only for a still click");
        h.state_mut().ui.brush_picker = None;
        let end = move_by(&mut h, q, vec2(20.0, 0.0), 2);
        button(&mut h, end, PointerButton::Primary, false);
        h.run_steps(2);
        assert_eq!(strokes(&h).len(), 1, "moved {moved}: one stroke");
    }
}
