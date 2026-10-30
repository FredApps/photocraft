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
