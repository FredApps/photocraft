//! Local experimental mask-effect list, visible only while a mask is selected in 3D.
use crate::{PhotocraftApp, theme::Tokens};
use photocraft_doc::{Layer, mask_paint::EffectKind};
use serde_json::{Value, json};

pub(crate) fn rows(app: &mut PhotocraftApp, ui: &mut egui::Ui, layer: &Layer, actions: &mut Vec<(String, Value)>) {
    if app.ui.workspace != "3D" || !app.ui.mask_target || app.session.active().and_then(|s| s.active_layer) != Some(layer.id) {
        return;
    }
    let Some(stack) = layer.mask.as_ref().and_then(|m| m.paint.as_ref()) else { return };
    let Some((doc_id, revision)) = app.session.active().map(|st| (st.doc.id.0, st.revision)) else { return };
    let selection_id = ui.id().with(("mask-effect-selection", doc_id, layer.id.0));
    let previous_selection = ui.ctx().data_mut(|data| data.get_temp::<(u64, usize)>(selection_id));
    if previous_selection != Some((revision, stack.active)) {
        if let Some((old_revision, old_index)) = previous_selection {
            for key in ["value", "input_black", "gamma", "input_white", "output_black", "output_white"] {
                let parameter_id = egui::Id::new(("mask-effect-param", doc_id, old_revision, layer.id.0, old_index, key));
                ui.ctx().data_mut(|data| {
                    let _ = data.remove::<f32>(parameter_id);
                });
            }
        }
        ui.ctx().data_mut(|data| data.insert_temp(selection_id, (revision, stack.active)));
    }
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
                    let (icon_rect, icon_response) = ui.allocate_exact_size(egui::vec2(16.0, 20.0), egui::Sense::click());
                    let icon = match &paint.kind {
                        EffectKind::Paint => "brush",
                        EffectKind::Fill { .. } => "paint-bucket",
                        EffectKind::Levels { .. } => "contrast",
                    };
                    crate::icons::paint(ui, icon_rect, icon, 14.0, tokens.icon);
                    let name = ui.selectable_label(index == stack.active, &paint.name);
                    if name.clicked() || icon_response.clicked() {
                        actions.push(("uv.maskPaint.select".into(), json!({"layer":layer.id.0,"index":index})));
                        actions.push(("ui.maskTarget".into(), json!(true)));
                    }
                    name.context_menu(|ui| {
                        add_menu(ui, layer.id.0, actions);
                        ui.separator();
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
                        let delete_label = match &paint.kind {
                            EffectKind::Paint => tl!("Delete Paint layer"),
                            EffectKind::Fill { .. } => tl!("Delete Fill layer"),
                            EffectKind::Levels { .. } => tl!("Delete Levels layer"),
                        };
                        if ui.button(delete_label).clicked() {
                            actions.push(("uv.maskPaint.modify".into(), json!({"layer":layer.id.0,"index":index,"remove":true})));
                            ui.close();
                        }
                    });
                    let preview_opacity = crate::uv_mask_paint_preview::value(app, layer.id, index, paint.opacity);
                    let mut opacity = preview_opacity * 100.0;
                    let opacity_response = ui.add(egui::DragValue::new(&mut opacity).range(0.0..=100.0).suffix("%").speed(0.5)).on_hover_text(tl!("Opacity"));
                    if opacity_response.changed() {
                        crate::uv_mask_paint_preview::set(app, layer.id, index, opacity / 100.0);
                        ui.ctx().request_repaint();
                    }
                    if opacity_response.drag_stopped() || opacity_response.lost_focus() {
                        if opacity_response.changed() {
                            actions.push(("uv.maskPaint.modify".into(), json!({"layer":layer.id.0,"index":index,"opacity":opacity/100.0})));
                        } else if preview_opacity != paint.opacity {
                            actions.push(("uv.maskPaint.modify".into(), json!({"layer":layer.id.0,"index":index,"opacity":preview_opacity})));
                        }
                    }
                })
                .response;
            if index == stack.active {
                match &paint.kind {
                    EffectKind::Paint => {}
                    EffectKind::Fill { value, .. } => {
                        ui.horizontal(|ui| {
                            ui.add_space(82.0);
                            edit_parameter(ui, doc_id, revision, layer.id.0, index, "value", tl!("Fill"), *value, 0.0..=1.0, actions);
                        });
                    }
                    EffectKind::Levels { input_black, gamma, input_white, output_black, output_white } => {
                        let controls = [
                            ("input_black", tl!("Input Black"), *input_black, 0.0..=(*input_white - 0.001).max(0.0)),
                            ("gamma", tl!("Gamma"), *gamma, 0.01..=9.99),
                            ("input_white", tl!("Input White"), *input_white, (*input_black + 0.001).min(1.0)..=1.0),
                            ("output_black", tl!("Output Black"), *output_black, 0.0..=*output_white),
                            ("output_white", tl!("Output White"), *output_white, *output_black..=1.0),
                        ];
                        ui.add_space(2.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.add_space(82.0);
                            for (key, label, value, range) in controls {
                                edit_parameter(ui, doc_id, revision, layer.id.0, index, key, label, value, range, actions);
                            }
                        });
                    }
                }
            }
            ui.painter().hline(response.rect.x_range(), response.rect.bottom(), egui::Stroke::new(0.5, tokens.text_faint));
        });
    }
}

/// Parameter widgets keep their changing value in egui memory and send one document command when
/// the gesture ends. That gives immediate feedback in the control without flattening a 4K mask on
/// every pointer update or creating one undo entry per frame.
fn edit_parameter(
    ui: &mut egui::Ui,
    doc: u64,
    revision: u64,
    layer: u64,
    index: usize,
    key: &'static str,
    label: &str,
    canonical: f32,
    range: std::ops::RangeInclusive<f32>,
    actions: &mut Vec<(String, Value)>,
) {
    let id = egui::Id::new(("mask-effect-param", doc, revision, layer, index, key));
    let mut value = ui.ctx().data_mut(|data| data.get_temp::<f32>(id).unwrap_or(canonical));
    let response = ui
        .add(egui::DragValue::new(&mut value).range(range).speed(0.005).prefix(format!("{label}: ")))
        .on_hover_text(format!("{label} — {}", tl!("Applied when you release the control")));
    if response.changed() {
        ui.ctx().data_mut(|data| data.insert_temp(id, value));
    }
    if response.drag_stopped() || response.lost_focus() {
        let staged = ui.ctx().data_mut(|data| {
            let staged = data.get_temp::<f32>(id).unwrap_or(value);
            data.remove::<f32>(id);
            staged
        });
        if (staged - canonical).abs() > f32::EPSILON {
            let mut params = json!({"layer":layer,"index":index});
            if let Some(object) = params.as_object_mut() {
                object.insert(key.to_owned(), json!(staged));
                actions.push(("uv.maskPaint.modify".into(), params));
            }
        }
    }
}

pub(crate) fn add_menu(ui: &mut egui::Ui, layer: u64, actions: &mut Vec<(String, Value)>) {
    for (label, kind) in [(tl!("Add Paint layer"), "paint"), (tl!("Add Fill layer"), "fill"), (tl!("Add Levels layer"), "levels")] {
        if ui.button(label).clicked() {
            actions.push(("layer.select".into(), json!({"layer":layer})));
            actions.push(("uv.maskPaint.add".into(), json!({"layer":layer,"kind":kind})));
            actions.push(("ui.maskTarget".into(), json!(true)));
            ui.close();
        }
    }
}
