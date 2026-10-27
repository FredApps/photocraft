//! Local experimental mask-effect list, visible only while a mask is selected in 3D.
use crate::{PhotocraftApp, theme::Tokens};
use photocraft_doc::{Layer, mask_paint::EffectKind};
use serde_json::{Value, json};

#[derive(Clone, Copy)]
struct LevelsPicker {
    doc: u64,
    layer: u64,
    index: usize,
    point: u8,
    tool: crate::state::Tool,
}

fn levels_picker_id() -> egui::Id {
    egui::Id::new("uv-mask-levels-picker")
}

pub(crate) fn levels_picker_active(app: &PhotocraftApp, ctx: &egui::Context) -> bool {
    ctx.data(|data| data.get_temp::<LevelsPicker>(levels_picker_id())).is_some_and(|picker| {
        picker.tool == app.ui.tool
            && app.session.active().is_some_and(|st| {
                st.doc.id.0 == picker.doc
                    && st.active_layer.is_some_and(|layer| layer.0 == picker.layer)
                    && selected_effect(ctx, picker.doc, picker.layer) == Some(picker.index)
            })
    })
}

fn picker_point(app: &PhotocraftApp, ctx: &egui::Context) -> Option<LevelsPicker> {
    ctx.data(|data| data.get_temp::<LevelsPicker>(levels_picker_id())).filter(|picker| {
        picker.tool == app.ui.tool
            && app.session.active().is_some_and(|st| {
                st.doc.id.0 == picker.doc
                    && st.active_layer.is_some_and(|layer| layer.0 == picker.layer)
                    && selected_effect(ctx, picker.doc, picker.layer) == Some(picker.index)
            })
    })
}

/// Apply one bounded grayscale sample from the Levels input prefix.
pub(crate) fn sample_levels_at(app: &mut PhotocraftApp, ctx: &egui::Context, point: [f64; 2]) -> bool {
    sample_levels_at_inner(app, ctx, point)
}

pub(crate) fn sample_levels_at_uv(app: &mut PhotocraftApp, ctx: &egui::Context, material: usize, uv: [f64; 2]) -> bool {
    let Some(picker) = picker_point(app, ctx) else { return false };
    let Some(st) = app.session.active() else { return false };
    if uv.iter().any(|value| !value.is_finite() || !(0.0..=1.0).contains(value)) {
        return true;
    }
    let channel = app.session.uv.current_channel();
    if app.session.uv.map_bindings(channel).get(material).copied().flatten() != Some(st.doc.id) {
        return true;
    }
    let point = [uv[0] * f64::from(st.doc.size.width), (1.0 - uv[1]) * f64::from(st.doc.size.height)];
    let _ = picker;
    sample_levels_at_inner(app, ctx, point)
}

fn sample_levels_at_inner(app: &mut PhotocraftApp, ctx: &egui::Context, point: [f64; 2]) -> bool {
    let Some(picker) = picker_point(app, ctx) else { return false };
    let Some(st) = app.session.active() else { return false };
    if !point[0].is_finite()
        || !point[1].is_finite()
        || point[0] < 0.0
        || point[1] < 0.0
        || point[0] >= f64::from(st.doc.size.width)
        || point[1] >= f64::from(st.doc.size.height)
    {
        return true;
    }
    let Some(stack) = st.doc.layer(photocraft_doc::LayerId(picker.layer)).and_then(|layer| layer.mask.as_ref()).and_then(|mask| mask.paint.as_ref()) else {
        return false;
    };
    let (x, y) = (point[0].floor() as i32, point[1].floor() as i32);
    let Some(sample) =
        photocraft_compose::mask_paint::compose_prefix(stack, picker.index, photocraft_geom::Rect::new(x, y, x.saturating_add(1), y.saturating_add(1)))
            .and_then(|values| values.first().copied())
    else {
        return true;
    };
    let Some(effect) = stack.layers.get(picker.index) else { return true };
    let EffectKind::Levels { input_black, input_white, .. } = &effect.kind else { return true };
    let sample = sample.clamp(0.0, 1.0);
    let params = match picker.point {
        0 => json!({"layer":picker.layer,"index":picker.index,"input_black":sample.min(*input_white - 0.002)}),
        1 => {
            let t = ((sample - *input_black) / (*input_white - *input_black).max(0.002)).clamp(0.01, 0.99);
            let next_gamma = (t.ln() / 0.5f32.ln()).clamp(0.01, 9.99);
            json!({"layer":picker.layer,"index":picker.index,"gamma":next_gamma})
        }
        _ => json!({"layer":picker.layer,"index":picker.index,"input_white":sample.max(*input_black + 0.002)}),
    };
    if let Err(error) = app.run("uv.maskPaint.modify", params) {
        app.ui.status = error;
        app.ui.status_error = true;
    }
    true
}

pub(crate) fn prune_levels_picker(app: &PhotocraftApp, ctx: &egui::Context) {
    if ctx.input(|input| input.key_pressed(egui::Key::Escape)) || !levels_picker_active(app, ctx) {
        ctx.data_mut(|data| data.remove::<LevelsPicker>(levels_picker_id()));
    }
}

fn effect_selection_id(doc: u64, layer: u64) -> egui::Id {
    egui::Id::new(("uv-mask-effect-selection", doc, layer))
}

fn expanded_effects_id() -> egui::Id {
    egui::Id::new("uv-mask-paint-expanded-layer")
}

/// Open the selected layer's mask stack without switching the canvas to mask display.
pub(crate) fn expand_effects(ctx: &egui::Context, doc: u64, layer: u64) {
    ctx.data_mut(|data| data.insert_temp(expanded_effects_id(), (doc, layer)));
}

/// Collapse the mask stack when the user returns to the layer's image thumbnail/row.
pub(crate) fn collapse_effects(ctx: &egui::Context) {
    ctx.data_mut(|data| data.remove_temp::<(u64, u64)>(expanded_effects_id()));
}

fn effects_expanded(ctx: &egui::Context, doc: u64, layer: u64) -> bool {
    ctx.data(|data| data.get_temp::<(u64, u64)>(expanded_effects_id())) == Some((doc, layer))
}

pub(crate) fn clear_effect_selection(ctx: &egui::Context, doc: u64, layer: u64) {
    ctx.data_mut(|data| data.remove::<usize>(effect_selection_id(doc, layer)));
}

pub(crate) fn select_effect(ctx: &egui::Context, doc: u64, layer: u64, index: usize) {
    ctx.data_mut(|data| data.insert_temp(effect_selection_id(doc, layer), index));
}

pub(crate) fn selected_effect(ctx: &egui::Context, doc: u64, layer: u64) -> Option<usize> {
    ctx.data_mut(|data| data.get_temp::<usize>(effect_selection_id(doc, layer)))
}

fn effect_is_selected(ctx: &egui::Context, doc: u64, layer: u64, index: usize) -> bool {
    selected_effect(ctx, doc, layer) == Some(index)
}

pub(crate) fn rows(app: &mut PhotocraftApp, ui: &mut egui::Ui, layer: &Layer, actions: &mut Vec<(String, Value)>) -> bool {
    let Some(doc_id) = app.session.active().map(|st| st.doc.id.0) else { return false };
    if app.ui.workspace != "3D" || !effects_expanded(ui.ctx(), doc_id, layer.id.0) || app.session.active().and_then(|s| s.active_layer) != Some(layer.id) {
        return false;
    }
    let Some(stack) = layer.mask.as_ref().and_then(|m| m.paint.as_ref()) else { return false };
    let tokens = Tokens::get(ui.ctx());
    for (index, paint) in stack.layers.iter().enumerate().rev() {
        ui.push_id(("mask-paint", layer.id.0, index), |ui| {
            let row_bounds = ui.available_rect_before_wrap().intersect(ui.clip_rect());
            let response = ui
                .horizontal(|ui| {
                    ui.add_space(40.0);
                    let (eye, eye_response) = ui.allocate_exact_size(egui::vec2(22.0, 20.0), egui::Sense::click());
                    crate::icons::paint(
                        ui,
                        eye,
                        if paint.visible { "eye" } else { "eye-off" },
                        14.0,
                        if paint.visible { tokens.icon } else { tokens.text_faint },
                    );
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
                    let selected = effect_is_selected(ui.ctx(), doc_id, layer.id.0, index);
                    let name = ui.selectable_label(selected, &paint.name);
                    if name.clicked() || icon_response.clicked() {
                        let show_mask = ui.input(|i| i.modifiers.alt);
                        actions.push(("ui.maskTarget".into(), json!({"enabled":true,"show":show_mask})));
                        actions.push(("uv.maskPaint.select".into(), json!({"layer":layer.id.0,"index":index})));
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
            if matches!(&paint.kind, EffectKind::Fill { .. }) {
                let drop_rect =
                    egui::Rect::from_min_max(egui::pos2(row_bounds.left(), response.rect.top()), egui::pos2(row_bounds.right(), response.rect.bottom()));
                app.uv_cache.library.fill_targets.push((drop_rect, doc_id, layer.id.0, index));
            }
            ui.painter().hline(response.rect.x_range(), response.rect.bottom(), egui::Stroke::new(0.5, tokens.text_faint));
        });
    }
    !stack.layers.is_empty()
}

/// Levels mask effects use the familiar Properties tab instead of expanding controls in Layers.
pub(crate) fn levels_properties(app: &mut PhotocraftApp, ui: &mut egui::Ui) -> bool {
    if app.ui.workspace != "3D" {
        return false;
    }
    let Some(st) = app.session.active() else { return false };
    let Some(layer) = st.active_layer.and_then(|id| st.doc.layer(id)) else { return false };
    let Some(stack) = layer.mask.as_ref().and_then(|mask| mask.paint.as_ref()) else { return false };
    let Some(index) = selected_effect(ui.ctx(), st.doc.id.0, layer.id.0).filter(|index| *index == stack.active) else { return false };
    let Some(effect) = stack.layers.get(index) else { return false };
    let EffectKind::Levels { input_black, gamma, input_white, output_black, output_white } = &effect.kind else { return false };
    let (doc_id, revision, size) = (st.doc.id.0, st.revision, st.doc.size);
    let (layer_id, index) = (layer.id, index);
    let effect_name = effect.name.clone();
    let mut visible = effect.visible;
    let (input_black, gamma, input_white, output_black, output_white) = (*input_black, *gamma, *input_white, *output_black, *output_white);
    let canonical = serde_json::json!({
        "inBlack": (input_black * 255.0).round(),
        "gamma": gamma,
        "inWhite": (input_white * 255.0).round(),
        "outBlack": (output_black * 255.0).round(),
        "outWhite": (output_white * 255.0).round(),
    });
    let values_id = egui::Id::new(("mask-levels-values", doc_id, layer_id.0, index));
    let mut values = ui
        .ctx()
        .data(|data| data.get_temp::<(u64, Value)>(values_id))
        .filter(|(cached_revision, _)| *cached_revision == revision)
        .map_or(canonical, |(_, values)| values);
    let hist = mask_levels_histogram(ui.ctx(), doc_id, revision, layer_id.0, index, stack, size);
    let cx = crate::adjust_editors::EditorCx {
        mem: egui::Id::new(("mask-levels", doc_id, layer_id.0, index)),
        hist: Some(hist),
        gray: true,
        swatches: [[0.0; 3]; 2],
        gradients: Vec::new(),
        dialog: false,
        dynamic_color_sliders: false,
        isolated_curve: None,
    };
    let mut actions = Vec::new();
    let mut arm_picker = None;
    ui.horizontal(|ui| {
        let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 20.0), egui::Sense::hover());
        crate::icons::paint(ui, icon_rect, "contrast", 16.0, Tokens::get(ui.ctx()).icon);
        ui.strong(&effect_name);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.weak(tl!("Levels"));
        });
    });
    ui.horizontal(|ui| {
        let current =
            picker_point(app, ui.ctx()).filter(|picker| picker.doc == doc_id && picker.layer == layer_id.0 && picker.index == index).map(|picker| picker.point);
        let choices = [
            (0, tl!("Set Black Point"), egui::Color32::BLACK),
            (1, tl!("Set Neutral Gray Point"), egui::Color32::from_gray(128)),
            (2, tl!("Set White Point"), egui::Color32::WHITE),
        ];
        for (point, label, color) in choices {
            if crate::adjust_dialog::picker_button(ui, current == Some(point), true, label, color).clicked() {
                if current == Some(point) {
                    ui.ctx().data_mut(|data| data.remove::<LevelsPicker>(levels_picker_id()));
                } else {
                    arm_picker = Some(point);
                }
            }
        }
    });
    ui.add_space(8.0);
    let edit = crate::adjust_editors::editor(ui, "levels", &mut values, &cx);
    if edit.changed && !edit.commit {
        ui.ctx().data_mut(|data| data.insert_temp(values_id, (revision, values.clone())));
    }
    if edit.commit {
        let scale = 1.0 / 255.0;
        let black = values["inBlack"].as_f64().unwrap_or(0.0) as f32 * scale;
        let white = values["inWhite"].as_f64().unwrap_or(255.0) as f32 * scale;
        let out_black = values["outBlack"].as_f64().unwrap_or(0.0) as f32 * scale;
        let out_white = values["outWhite"].as_f64().unwrap_or(255.0) as f32 * scale;
        let next_gamma = values["gamma"].as_f64().unwrap_or(1.0) as f32;
        if black != input_black || white != input_white || out_black != output_black || out_white != output_white || next_gamma != gamma {
            actions.push((
                "uv.maskPaint.modify".to_string(),
                serde_json::json!({
                    "layer":layer_id.0,"index":index,
                    "input_black":black,"gamma":next_gamma,"input_white":white,
                    "output_black":out_black,"output_white":out_white
                }),
            ));
        }
        ui.ctx().data_mut(|data| data.remove::<(u64, Value)>(values_id));
    }
    ui.horizontal(|ui| {
        if ui.checkbox(&mut visible, tl!("Visible")).changed() {
            actions.push(("uv.maskPaint.modify".to_string(), serde_json::json!({"layer":layer_id.0,"index":index,"visible":visible})));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(tl!("Reset")).clicked() {
                ui.ctx().data_mut(|data| data.remove::<(u64, Value)>(values_id));
                actions.push((
                    "uv.maskPaint.modify".to_string(),
                    serde_json::json!({"layer":layer_id.0,"index":index,"input_black":0.0,"gamma":1.0,"input_white":1.0,"output_black":0.0,"output_white":1.0}),
                ));
            }
        });
    });
    if edit.changed {
        ui.ctx().request_repaint();
    }
    for (command, params) in actions {
        if let Err(error) = app.run(&command, params) {
            app.ui.status = error;
            app.ui.status_error = true;
        }
    }
    if let Some(point) = arm_picker
        && (!app.ui.uv_workbench.select_shells || crate::uv_ui::commit_current_polygon_selection(app))
    {
        app.ui.uv_workbench.select_shells = false;
        ui.ctx().data_mut(|data| data.insert_temp(levels_picker_id(), LevelsPicker { doc: doc_id, layer: layer_id.0, index, point, tool: app.ui.tool }));
    }
    true
}

/// A small, distributed histogram of the editable prefix before the selected Levels layer.
/// Sampling 32×32 spread-out 4×4 patches keeps this useful on 4k masks without recompositing the
/// full mask on every Properties frame. The cached entry is replaced after any document revision.
fn mask_levels_histogram(
    ctx: &egui::Context,
    doc: u64,
    revision: u64,
    layer: u64,
    index: usize,
    stack: &photocraft_doc::mask_paint::Stack,
    size: photocraft_doc::Size,
) -> std::sync::Arc<crate::tone::Histograms> {
    let id = egui::Id::new(("mask-levels-histogram", doc, layer, index));
    if let Some((cached_revision, cached)) = ctx.data(|data| data.get_temp::<(u64, std::sync::Arc<crate::tone::Histograms>)>(id))
        && cached_revision == revision
    {
        return cached;
    }
    let mut bins = [0u32; 256];
    let (width, height) = (size.width as i32, size.height as i32);
    for gy in 0..32 {
        for gx in 0..32 {
            let x = (gx * width / 32).min((width - 1).max(0));
            let y = (gy * height / 32).min((height - 1).max(0));
            let rect = photocraft_geom::Rect::new(x, y, (x + 4).min(width), (y + 4).min(height));
            if let Some(values) = photocraft_compose::mask_paint::compose_prefix(stack, index, rect) {
                for value in values {
                    let bin = (value.clamp(0.0, 1.0) * 255.0).round() as usize;
                    bins[bin] = bins[bin].saturating_add(1);
                }
            }
        }
    }
    let cached = std::sync::Arc::new(crate::tone::Histograms { tag: doc ^ revision ^ layer ^ index as u64, ch: vec![bins], level: 0 });
    ctx.data_mut(|data| data.insert_temp(id, (revision, cached.clone())));
    cached
}

/// Fill settings live in Properties; the rectangle itself is edited directly on the 2D canvas.
pub(crate) fn fill_properties(app: &mut PhotocraftApp, ui: &mut egui::Ui) -> bool {
    if app.ui.workspace != "3D" {
        return false;
    }
    let Some(st) = app.session.active() else { return false };
    let Some(layer) = st.active_layer.and_then(|id| st.doc.layer(id)) else { return false };
    let Some(stack) = layer.mask.as_ref().and_then(|mask| mask.paint.as_ref()) else { return false };
    let Some(index) = selected_effect(ui.ctx(), st.doc.id.0, layer.id.0).filter(|index| *index == stack.active) else { return false };
    let Some(effect) = stack.layers.get(index) else { return false };
    let EffectKind::Fill { value, rotation, repeat_u, repeat_v, mirror_u, mirror_v } = &effect.kind else { return false };
    let layer_id = layer.id.0;
    let (_, rotation) = crate::uv_mask_paint_preview::fill_transform(
        app,
        layer.id,
        index,
        effect.fill_bounds.unwrap_or([0, 0, st.doc.size.width as i32, st.doc.size.height as i32]),
        *rotation,
    );
    let mut actions = Vec::new();
    ui.heading(&effect.name);
    ui.label(tl!("Fill"));
    gradient_parameter(ui, st.doc.id.0, st.revision, layer_id, index, *value, &mut actions);
    edit_parameter_speed(ui, st.doc.id.0, st.revision, layer_id, index, "rotation", tl!("Rotation"), rotation, -360.0..=360.0, 1.0, &mut actions);
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        let mut next = *repeat_u;
        if ui.checkbox(&mut next, tl!("Repeat U")).changed() {
            actions.push(("uv.maskPaint.modify".into(), json!({"layer":layer_id,"index":index,"repeat_u":next})));
        }
        let mut next = *repeat_v;
        if ui.checkbox(&mut next, tl!("Repeat V")).changed() {
            actions.push(("uv.maskPaint.modify".into(), json!({"layer":layer_id,"index":index,"repeat_v":next})));
        }
    });
    ui.horizontal_wrapped(|ui| {
        let mut next = *mirror_u;
        if ui.checkbox(&mut next, tl!("Mirror U")).changed() {
            actions.push(("uv.maskPaint.modify".into(), json!({"layer":layer_id,"index":index,"mirror_u":next})));
        }
        let mut next = *mirror_v;
        if ui.checkbox(&mut next, tl!("Mirror V")).changed() {
            actions.push(("uv.maskPaint.modify".into(), json!({"layer":layer_id,"index":index,"mirror_v":next})));
        }
    });
    ui.add_space(6.0);
    if let Some(map) = effect.fill_map.as_ref() {
        let [source_w, source_h] = effect.fill_map_size;
        if source_w > 0 && source_h > 0 {
            ui.add_space(6.0);
            let preview_w = ui.available_width().clamp(1.0, 120.0);
            let preview_h = (preview_w * (source_h as f32 / source_w as f32)).clamp(1.0, 72.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(preview_w, preview_h), egui::Sense::hover());
            let tex_w = 120usize;
            let tex_h = ((source_h as f64 / source_w as f64 * tex_w as f64).round() as usize).clamp(1, 72);
            let texture_id = egui::Id::new(("uv-fill-map-preview", st.doc.id.0, layer_id, index));
            let cached = ui.ctx().data_mut(|data| data.get_temp::<(u64, egui::TextureHandle)>(texture_id));
            let handle = if let Some((_, handle)) = cached.filter(|(revision, _)| *revision == st.revision) {
                handle
            } else {
                let pixels = (0..tex_h)
                    .flat_map(|y| (0..tex_w).map(move |x| (x, y)))
                    .map(|(x, y)| {
                        let sx = ((x as u64 * u64::from(source_w)) / tex_w as u64).min(u64::from(source_w - 1)) as i32;
                        let sy = ((y as u64 * u64::from(source_h)) / tex_h as u64).min(u64::from(source_h - 1)) as i32;
                        egui::Color32::from_gray((map.sample_channel(sx, sy, 0).clamp(0.0, 1.0) * 255.0).round() as u8)
                    })
                    .collect();
                let image = egui::ColorImage::new([tex_w, tex_h], pixels);
                let handle = ui.ctx().load_texture(format!("uv-fill-map-{}-{layer_id}-{index}", st.doc.id.0), image, egui::TextureOptions::LINEAR);
                ui.ctx().data_mut(|data| data.insert_temp(texture_id, (st.revision, handle.clone())));
                handle
            };
            ui.painter().image(handle.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        }
        if ui.button(tl!("Remove image")).clicked() {
            actions.push(("uv.maskPaint.modify".into(), json!({"layer":layer_id,"index":index,"clear_fill_map":true})));
        }
    } else {
        ui.weak(tl!("Uniform grayscale Fill"));
    }
    for (command, params) in actions {
        if let Err(error) = app.run(&command, params) {
            app.ui.status = error;
            app.ui.status_error = true;
        }
    }
    true
}

/// Black-to-white source-value control. It stages pointer movement in UI memory and commits one
/// mask recomposition after release, just like the numeric controls above.
fn gradient_parameter(ui: &mut egui::Ui, doc: u64, revision: u64, layer: u64, index: usize, canonical: f32, actions: &mut Vec<(String, Value)>) {
    let id = egui::Id::new(("mask-effect-param", doc, revision, layer, index, "value"));
    let mut value = ui.ctx().data_mut(|data| data.get_temp::<f32>(id).unwrap_or(canonical));
    ui.horizontal(|ui| {
        ui.label(tl!("Gray value"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.monospace(format!("{value:.3}"));
        });
    });
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::click_and_drag());
    let track = rect.shrink2(egui::vec2(1.0, 3.0));
    let painter = ui.painter();
    let steps = 64;
    for step in 0..steps {
        let x0 = egui::lerp(track.x_range(), step as f32 / steps as f32);
        let x1 = egui::lerp(track.x_range(), (step + 1) as f32 / steps as f32);
        let gray = (step * 255 / (steps - 1)) as u8;
        painter.rect_filled(egui::Rect::from_min_max(egui::pos2(x0, track.top()), egui::pos2(x1 + 0.5, track.bottom())), 0.0, egui::Color32::from_gray(gray));
    }
    painter.rect_stroke(track, 2.0, egui::Stroke::new(1.0, Tokens::get(ui.ctx()).text_faint), egui::StrokeKind::Inside);
    let pointer_active = response.drag_started_by(egui::PointerButton::Primary)
        || response.dragged_by(egui::PointerButton::Primary)
        || response.drag_stopped_by(egui::PointerButton::Primary)
        || response.clicked_by(egui::PointerButton::Primary);
    if pointer_active && let Some(pointer) = response.interact_pointer_pos() {
        value = ((pointer.x - track.left()) / track.width().max(1.0)).clamp(0.0, 1.0);
        ui.ctx().data_mut(|data| data.insert_temp(id, value));
    }
    let thumb_x = egui::lerp(track.x_range(), value.clamp(0.0, 1.0));
    painter.line_segment(
        [egui::pos2(thumb_x, track.top() - 2.0), egui::pos2(thumb_x, track.bottom() + 2.0)],
        egui::Stroke::new(2.0, Tokens::get(ui.ctx()).accent),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, tl!("Gray value")));
    if response.drag_stopped_by(egui::PointerButton::Primary) || response.clicked_by(egui::PointerButton::Primary) {
        let staged = ui.ctx().data_mut(|data| {
            let staged = data.get_temp::<f32>(id).unwrap_or(value);
            data.remove::<f32>(id);
            staged
        });
        if (staged - canonical).abs() > f32::EPSILON {
            actions.push(("uv.maskPaint.modify".into(), json!({"layer":layer,"index":index,"value":staged})));
        }
    }
}

#[allow(clippy::too_many_arguments)] // The fields form one explicit UI parameter binding.
fn edit_parameter_speed(
    ui: &mut egui::Ui,
    doc: u64,
    revision: u64,
    layer: u64,
    index: usize,
    key: &'static str,
    label: &str,
    canonical: f32,
    range: std::ops::RangeInclusive<f32>,
    speed: f64,
    actions: &mut Vec<(String, Value)>,
) {
    let id = egui::Id::new(("mask-effect-param", doc, revision, layer, index, key));
    let mut value = ui.ctx().data_mut(|data| data.get_temp::<f32>(id).unwrap_or(canonical));
    let response = ui
        .add(egui::DragValue::new(&mut value).range(range).speed(speed).suffix(if key == "rotation" { "°" } else { "" }).prefix(format!("{label}: ")))
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
    if ui.button(tl!("Fill from selection")).clicked() {
        actions.push(("layer.select".into(), json!({"layer":layer})));
        actions.push(("uv.maskPaint.add".into(), json!({"layer":layer,"kind":"fill","from_selection":true,"name":"Selection Fill"})));
        actions.push(("ui.maskTarget".into(), json!(true)));
        ui.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use photocraft_color::PixelFormat;
    use photocraft_doc::{
        LayerMask,
        mask_paint::{Paint, Stack},
    };
    use photocraft_raster::Surface;

    #[test]
    fn mask_effect_rows_require_expansion_and_mask_target_has_no_selected_child() {
        let mut session = photocraft_engine::Session::new();
        session.execute("file.new", json!({"width":16,"height":16})).unwrap();
        let layer_id = session.active().unwrap().active_layer.unwrap();
        let state = session.active_mut().unwrap();
        let base = Surface::with_default(PixelFormat::GRAY8, &[0.0]);
        let mut mask = LayerMask::hide_all();
        mask.paint = Some(Stack {
            base,
            layers: vec![Paint {
                name: "Fill".into(),
                visible: true,
                opacity: 1.0,
                surface: Surface::with_default(PixelFormat::RGBA8, &[0.0, 0.0, 0.0, 0.0]),
                fill_map: None,
                fill_map_size: [0, 0],
                fill_bounds: Some([0, 0, 16, 16]),
                kind: EffectKind::Fill { value: 1.0, rotation: 0.0, repeat_u: false, repeat_v: false, mirror_u: false, mirror_v: false },
            }],
            active: 0,
        });
        std::sync::Arc::make_mut(&mut state.doc).layer_mut(layer_id).unwrap().mask = Some(mask);
        let mut app = PhotocraftApp::new(session, crate::Services::default());
        app.ui.workspace = "3D".into();
        app.ui.mask_target = true;
        let layer = app.session.active().unwrap().doc.layer(layer_id).unwrap().clone();
        let ctx = egui::Context::default();
        let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(300.0, 120.0))), ..Default::default() };
        let mut visible = false;
        let mut actions = Vec::new();
        let mut output = ctx.run_ui(input, |ui| visible = rows(&mut app, ui, &layer, &mut actions));
        output.textures_delta.clear();
        assert!(!visible, "targeting the mask alone keeps its effect stack collapsed");
        expand_effects(&ctx, doc_id(&app), layer.id.0);
        let mut output = ctx.run_ui(Default::default(), |ui| visible = rows(&mut app, ui, &layer, &mut actions));
        output.textures_delta.clear();
        assert!(visible, "expanding the mask exposes its effect rows");
        assert!(actions.is_empty(), "targeting a mask does not select a child effect");
        assert_eq!(selected_effect(&ctx, doc_id(&app), layer.id.0), None);
        collapse_effects(&ctx);
        let mut output = ctx.run_ui(Default::default(), |ui| visible = rows(&mut app, ui, &layer, &mut actions));
        output.textures_delta.clear();
        assert!(!visible, "returning to layer pixels collapses its effect rows");
    }

    #[test]
    fn targeting_mask_thumbnail_clears_selected_child_effect() {
        let ctx = egui::Context::default();
        let doc = 41;
        let layer = 9;
        ctx.data_mut(|data| data.insert_temp(effect_selection_id(doc, layer), 2usize));
        assert_eq!(selected_effect(&ctx, doc, layer), Some(2));

        // This is the state transition used by the Layers-panel mask-thumbnail handler.
        clear_effect_selection(&ctx, doc, layer);
        assert_eq!(selected_effect(&ctx, doc, layer), None, "the mask is targeted without selecting a child effect");
    }

    fn doc_id(app: &PhotocraftApp) -> u64 {
        app.session.active().unwrap().doc.id.0
    }
}
