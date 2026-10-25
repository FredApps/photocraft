//! Experimental, local-only editable mask brush stacks for the UV workspace.
use crate::{EngineError, Result, Session, commands::CommandSpec};
use photocraft_doc::{
    LayerId,
    mask_paint::{EffectKind, Paint, Stack},
};
use photocraft_raster::Surface;
use serde_json::{Value, json};

fn bad(message: &str) -> EngineError {
    EngineError::Other(message.into())
}
fn target(s: &Session, p: &Value) -> Result<LayerId> {
    let st = s.active().ok_or(EngineError::NoDocument)?;
    let id = p.get("layer").and_then(Value::as_u64).map(LayerId).or(st.active_layer).ok_or_else(|| bad("Select a masked layer"))?;
    let l = st.doc.layer(id).ok_or(EngineError::NoLayer(id))?;
    if l.mask.is_none() {
        return Err(bad("Layer has no mask"));
    }
    if st.doc.effective_locks(id).all {
        return Err(bad("Layer is locked"));
    }
    Ok(id)
}
fn index(p: &Value) -> Result<usize> {
    p.get("index").and_then(Value::as_u64).and_then(|n| usize::try_from(n).ok()).ok_or_else(|| bad("Invalid paint layer index"))
}
fn unit_param(p: &Value, key: &str, default: f32) -> Result<f32> {
    p.get(key)
        .map(|v| v.as_f64().filter(|n| n.is_finite() && (0.0..=1.0).contains(n)).map(|n| n as f32).ok_or_else(|| bad("Effect values must be between 0 and 1")))
        .transpose()
        .map(|v| v.unwrap_or(default))
}
fn gamma_param(p: &Value) -> Result<f32> {
    p.get("gamma")
        .map(|v| v.as_f64().filter(|n| n.is_finite() && (0.01..=100.0).contains(n)).map(|n| n as f32).ok_or_else(|| bad("Gamma must be between 0.01 and 100")))
        .transpose()
        .map(|v| v.unwrap_or(1.0))
}
fn validate_effect(effect: &EffectKind) -> Result<()> {
    match effect {
        EffectKind::Paint => Ok(()),
        EffectKind::Fill { value } if value.is_finite() && (0.0..=1.0).contains(value) => Ok(()),
        EffectKind::Fill { .. } => Err(bad("Fill value must be between 0 and 1")),
        EffectKind::Levels { input_black, gamma, input_white, output_black, output_white }
            if input_black.is_finite()
                && gamma.is_finite()
                && input_white.is_finite()
                && output_black.is_finite()
                && output_white.is_finite()
                && (0.0..=1.0).contains(input_black)
                && (0.01..=100.0).contains(gamma)
                && (0.0..=1.0).contains(input_white)
                && (0.0..=1.0).contains(output_black)
                && (0.0..=1.0).contains(output_white)
                && input_black < input_white
                && output_black <= output_white =>
        {
            Ok(())
        }
        EffectKind::Levels { .. } => Err(bad("Invalid Levels range")),
    }
}
fn has_any(p: &Value, keys: &[&str]) -> bool {
    keys.iter().any(|key| p.get(key).is_some())
}
fn add(s: &mut Session, p: &Value) -> Result<Value> {
    let id = target(s, p)?;
    let kind = p.get("kind").and_then(Value::as_str).unwrap_or("paint").to_owned();
    let default_name = match kind.as_str() {
        "fill" => "Fill",
        "levels" => "Levels",
        _ => "Paint",
    };
    let name = p.get("name").and_then(Value::as_str).unwrap_or(default_name).chars().take(128).collect::<String>();
    s.edit("Add mask Paint layer", |doc, active| {
        let mask = doc.layer_mut(id).and_then(|l| l.mask.as_mut()).ok_or_else(|| bad("Layer has no mask"))?;
        let format = photocraft_color::PixelFormat::RGBA8.with_sample(mask.surface.format().sample);
        let base = mask.surface.clone();
        let stack = mask.paint.get_or_insert_with(|| Stack { base, layers: Vec::new(), active: 0 });
        if stack.layers.len() >= 256 {
            return Err(bad("Mask supports up to 256 Paint layers"));
        }
        let surface = Surface::new(format);
        let effect = match kind.as_str() {
            "paint" => EffectKind::Paint,
            "fill" => EffectKind::Fill { value: unit_param(p, "value", 1.0)? },
            "levels" => EffectKind::Levels {
                input_black: unit_param(p, "input_black", 0.0)?,
                gamma: gamma_param(p)?,
                input_white: unit_param(p, "input_white", 1.0)?,
                output_black: unit_param(p, "output_black", 0.0)?,
                output_white: unit_param(p, "output_white", 1.0)?,
            },
            _ => return Err(bad("Kind must be paint, fill, or levels")),
        };
        validate_effect(&effect)?;
        match &effect {
            EffectKind::Paint if has_any(p, &["value", "input_black", "gamma", "input_white", "output_black", "output_white"]) => {
                return Err(bad("Effect settings do not apply to Paint layers"));
            }
            EffectKind::Fill { .. } if has_any(p, &["input_black", "gamma", "input_white", "output_black", "output_white"]) => {
                return Err(bad("Levels settings do not apply to Fill layers"));
            }
            EffectKind::Levels { .. } if p.get("value").is_some() => return Err(bad("Fill value does not apply to Levels layers")),
            _ => {}
        }
        stack.layers.push(Paint { name, visible: true, opacity: 1.0, surface, kind: effect });
        stack.active = stack.layers.len().saturating_sub(1);
        *active = Some(id);
        Ok(json!({"index":stack.active}))
    })
}
fn select(s: &mut Session, p: &Value) -> Result<Value> {
    let id = target(s, p)?;
    let index = index(p)?;
    let st = s.active_mut().ok_or(EngineError::NoDocument)?;
    let mask = std::sync::Arc::make_mut(&mut st.doc).layer_mut(id).and_then(|l| l.mask.as_mut()).ok_or_else(|| bad("Layer has no mask"))?;
    let stack = mask.paint.as_mut().ok_or_else(|| bad("Mask has no Paint layers"))?;
    if index >= stack.layers.len() {
        return Err(bad("Paint layer is out of range"));
    }
    stack.active = index;
    st.active_layer = Some(id);
    Ok(Value::Null)
}
fn modify(s: &mut Session, p: &Value) -> Result<Value> {
    let id = target(s, p)?;
    let index = index(p)?;
    let opacity = p
        .get("opacity")
        .map(|v| v.as_f64().filter(|n| n.is_finite() && (0.0..=1.0).contains(n)).ok_or_else(|| bad("Opacity must be between 0 and 1")))
        .transpose()?;
    let visible = p.get("visible").map(|v| v.as_bool().ok_or_else(|| bad("Visibility must be boolean"))).transpose()?;
    let destination = p.get("to").map(|v| v.as_u64().and_then(|n| usize::try_from(n).ok()).ok_or_else(|| bad("Invalid destination"))).transpose()?;
    let remove = p.get("remove").and_then(Value::as_bool).unwrap_or(false);
    let fill_value = p.get("value").map(|_| unit_param(p, "value", 0.0)).transpose()?;
    let input_black = p.get("input_black").map(|_| unit_param(p, "input_black", 0.0)).transpose()?;
    let input_white = p.get("input_white").map(|_| unit_param(p, "input_white", 1.0)).transpose()?;
    let output_black = p.get("output_black").map(|_| unit_param(p, "output_black", 0.0)).transpose()?;
    let output_white = p.get("output_white").map(|_| unit_param(p, "output_white", 1.0)).transpose()?;
    let gamma = p.get("gamma").map(|_| gamma_param(p)).transpose()?;
    let effect_changed =
        fill_value.is_some() || input_black.is_some() || input_white.is_some() || output_black.is_some() || output_white.is_some() || gamma.is_some();
    let mut damage = photocraft_geom::Rect::EMPTY;
    let result = s.edit("Edit mask Paint layer", |doc, _| {
        let canvas = doc.bounds();
        let mask = doc.layer_mut(id).and_then(|l| l.mask.as_mut()).ok_or_else(|| bad("Layer has no mask"))?;
        let stack = mask.paint.as_mut().ok_or_else(|| bad("Mask has no Paint layers"))?;
        if destination.is_some_and(|to| to >= stack.layers.len()) {
            return Err(bad("Destination is out of range"));
        }
        if opacity.is_some() || visible.is_some() || remove || destination.is_some() || effect_changed {
            let affected = if destination.is_some() { stack.layers.iter().collect::<Vec<_>>() } else { stack.layers.get(index).into_iter().collect() };
            for paint in affected {
                let bounds = if !matches!(&paint.kind, EffectKind::Paint) || paint.surface.default_pixel().get(3).is_some_and(|a| *a != 0.0) {
                    canvas
                } else {
                    paint.surface.content_bounds()
                };
                damage = damage.union(&bounds);
            }
            if !damage.is_empty() {
                damage = damage.inflate((mask.feather.max(0.0) * 3.0).ceil().min(100_000.0) as i32).intersect(&canvas);
            }
        }
        let paint = stack.layers.get_mut(index).ok_or_else(|| bad("Paint layer is out of range"))?;
        if let Some(value) = opacity {
            paint.opacity = value as f32;
        }
        if let Some(value) = visible {
            paint.visible = value;
        }
        if let Some(name) = p.get("name").and_then(Value::as_str) {
            paint.name = name.chars().take(128).collect();
        }
        match &mut paint.kind {
            EffectKind::Fill { value } => {
                if input_black.is_some() || gamma.is_some() || input_white.is_some() || output_black.is_some() || output_white.is_some() {
                    return Err(bad("Levels settings do not apply to Fill layers"));
                }
                if let Some(next) = fill_value {
                    *value = next;
                }
            }
            EffectKind::Levels { input_black: ib, gamma: g, input_white: iw, output_black: ob, output_white: ow } => {
                if fill_value.is_some() {
                    return Err(bad("Fill value does not apply to Levels layers"));
                }
                if let Some(next) = input_black {
                    *ib = next;
                }
                if let Some(next) = gamma {
                    *g = next;
                }
                if let Some(next) = input_white {
                    *iw = next;
                }
                if let Some(next) = output_black {
                    *ob = next;
                }
                if let Some(next) = output_white {
                    *ow = next;
                }
            }
            EffectKind::Paint => {
                if fill_value.is_some() || input_black.is_some() || gamma.is_some() || input_white.is_some() || output_black.is_some() || output_white.is_some()
                {
                    return Err(bad("Effect settings do not apply to Paint layers"));
                }
            }
        }
        validate_effect(&paint.kind)?;
        if remove {
            stack.layers.remove(index);
            stack.active = stack.active.min(stack.layers.len().saturating_sub(1));
        } else if let Some(to) = destination {
            let selected = stack.active;
            let paint = stack.layers.remove(index);
            stack.layers.insert(to, paint);
            stack.active = if selected == index {
                to
            } else if index < selected && to >= selected {
                selected.saturating_sub(1)
            } else if index > selected && to <= selected {
                selected.saturating_add(1)
            } else {
                selected
            };
        }
        if mask.paint.as_ref().is_some_and(|s| s.layers.is_empty()) {
            // No other paint remains: restore the original mask without a full blend pass.
            if let Some(stack) = &mask.paint {
                mask.surface = stack.base.clone();
            }
            mask.paint = None;
        }
        Ok(Value::Null)
    })?;
    if let Some(st) = s.active_mut() {
        st.last_damage = Some(damage);
    }
    Ok(result)
}
pub fn specs() -> Vec<CommandSpec> {
    [
        ("uv.maskPaint.add", "Add mask stack layer", r#"{"layer":id?,"name":string?,"kind":"paint"|"fill"|"levels"?,"value":number?,"input_black":number?,"gamma":number?,"input_white":number?,"output_black":number?,"output_white":number?}"#, add as fn(&mut Session, &Value) -> Result<Value>),
        ("uv.maskPaint.select", "Select mask Paint layer", r#"{"layer":id?,"index":int}"#, select),
        (
            "uv.maskPaint.modify",
            "Edit mask Paint layer",
            r#"{"layer":id?,"index":int,"visible":bool?,"opacity":number?,"name":string?,"to":int?,"remove":bool?,"value":number?,"input_black":number?,"gamma":number?,"input_white":number?,"output_black":number?,"output_white":number?}"#,
            modify,
        ),
    ]
    .into_iter()
    .map(|(id, label, params, run)| CommandSpec {
        id,
        label,
        params,
        run,
        menu: &[],
        shortcut: None,
        journal: id != "uv.maskPaint.select",
        enabled: |s| if s.uv.scene.is_some() { Ok(()) } else { Err("Import a model first".into()) },
    })
    .collect()
}
