//! Local experimental brush-layer list, visible only while a mask is selected in 3D.
use crate::{PhotocraftApp, theme::Tokens};
use photocraft_doc::Layer;
use serde_json::{Value, json};

pub(crate) fn rows(app: &PhotocraftApp, ui: &mut egui::Ui, layer: &Layer, actions: &mut Vec<(String, Value)>) {
    if app.ui.workspace != "3D" || !app.ui.mask_target || app.session.active().and_then(|s| s.active_layer) != Some(layer.id) {
        return;
    }
    let Some(stack) = layer.mask.as_ref().and_then(|m| m.paint.as_ref()) else { return };
    let tokens = Tokens::get(ui.ctx());
    for (index, paint) in stack.layers.iter().enumerate().rev() {
        ui.push_id(("mask-paint", layer.id.0, index), |ui| {
            let response = ui
                .horizontal(|ui| {
                    ui.add_space(40.0);
                    let (eye, eye_response) = ui.allocate_exact_size(egui::vec2(22.0, 20.0), egui::Sense::click());
                    if paint.visible {
                        crate::icons::paint(ui, eye, "eye", 14.0, tokens.icon);
                    }
                    if eye_response.on_hover_text(tl!("Visibility")).clicked() {
                        actions.push(("uv.maskPaint.modify".into(), json!({"layer":layer.id.0,"index":index,"visible":!paint.visible})));
                    }
                    let (brush, brush_response) = ui.allocate_exact_size(egui::vec2(16.0, 20.0), egui::Sense::click());
                    crate::icons::paint(ui, brush, "brush", 14.0, tokens.icon);
                    let name = ui.selectable_label(index == stack.active, &paint.name);
                    if name.clicked() || brush_response.clicked() {
                        actions.push(("uv.maskPaint.select".into(), json!({"layer":layer.id.0,"index":index})));
                        actions.push(("ui.maskTarget".into(), json!(true)));
                    }
                    name.context_menu(|ui| {
                        if ui.button(tl!("Add Paint layer")).clicked() {
                            actions.push(("uv.maskPaint.add".into(), json!({"layer":layer.id.0})));
                            ui.close();
                        }
                        let mut name = paint.name.clone();
                        if ui.text_edit_singleline(&mut name).changed() {
                            actions.push(("uv.maskPaint.modify".into(), json!({"layer":layer.id.0,"index":index,"name":name})));
                        }
                        for (label, to, enabled) in
                            [(tl!("Move Up"), index.saturating_add(1), index + 1 < stack.layers.len()), (tl!("Move Down"), index.saturating_sub(1), index > 0)]
                        {
                            if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                                actions.push(("uv.maskPaint.modify".into(), json!({"layer":layer.id.0,"index":index,"to":to})));
                                ui.close();
                            }
                        }
                        if ui.button(tl!("Delete Paint layer")).clicked() {
                            actions.push(("uv.maskPaint.modify".into(), json!({"layer":layer.id.0,"index":index,"remove":true})));
                            ui.close();
                        }
                    });
                    let mut opacity = paint.opacity * 100.0;
                    let opacity_response=ui.add(egui::DragValue::new(&mut opacity).range(0.0..=100.0).suffix("%").speed(0.5)).on_hover_text(tl!("Opacity"));
                    let gesture_id=ui.id().with("opacity-gesture");
                    if opacity_response.drag_started() || opacity_response.gained_focus() {
                        let pass=ui.ctx().cumulative_pass_nr();
                        ui.ctx().data_mut(|d|d.insert_temp(gesture_id,pass));
                    }
                    if opacity_response.changed() {
                        let gesture=ui.ctx().data(|d|d.get_temp::<u64>(gesture_id)).unwrap_or_else(||ui.ctx().cumulative_pass_nr());
                        actions.push(("uv.maskPaint.modify".into(),json!({"layer":layer.id.0,"index":index,"opacity":opacity/100.0,"coalesce":format!("mask-opacity:{}:{index}:{gesture}",layer.id.0)})));
                    }
                })
                .response;
            ui.painter().hline(response.rect.x_range(), response.rect.bottom(), egui::Stroke::new(0.5, tokens.text_faint));
        });
    }
}
