//! Experimental, local-only editable mask brush stacks for the UV workspace.
use crate::{EngineError, Result, Session, commands::CommandSpec};
use photocraft_doc::{
    LayerId,
    mask_paint::{Paint, Stack},
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
fn add(s: &mut Session, p: &Value) -> Result<Value> {
    let id = target(s, p)?;
    let name = p.get("name").and_then(Value::as_str).unwrap_or("Paint").chars().take(128).collect::<String>();
    s.edit("Add mask Paint layer", |doc, active| {
        let mask = doc.layer_mut(id).and_then(|l| l.mask.as_mut()).ok_or_else(|| bad("Layer has no mask"))?;
        let format = photocraft_color::PixelFormat::RGBA8.with_sample(mask.surface.format().sample);
        let base = mask.surface.clone();
        let stack = mask.paint.get_or_insert_with(|| Stack { base, layers: Vec::new(), active: 0 });
        if stack.layers.len() >= 256 {
            return Err(bad("Mask supports up to 256 Paint layers"));
        }
        stack.layers.push(Paint { name, visible: true, opacity: 1.0, surface: Surface::new(format) });
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
    s.edit("Edit mask Paint layer", |doc, _| {
        let mask = doc.layer_mut(id).and_then(|l| l.mask.as_mut()).ok_or_else(|| bad("Layer has no mask"))?;
        let stack = mask.paint.as_mut().ok_or_else(|| bad("Mask has no Paint layers"))?;
        if destination.is_some_and(|to| to >= stack.layers.len()) {
            return Err(bad("Destination is out of range"));
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
        photocraft_compose::mask_paint::refresh(mask, None);
        if mask.paint.as_ref().is_some_and(|s| s.layers.is_empty()) {
            mask.paint = None;
        }
        Ok(Value::Null)
    })
}
pub fn specs() -> Vec<CommandSpec> {
    [
        ("uv.maskPaint.add", "Add mask Paint layer", r#"{"layer":id?,"name":string?}"#, add as fn(&mut Session, &Value) -> Result<Value>),
        ("uv.maskPaint.select", "Select mask Paint layer", r#"{"layer":id?,"index":int}"#, select),
        (
            "uv.maskPaint.modify",
            "Edit mask Paint layer",
            r#"{"layer":id?,"index":int,"visible":bool?,"opacity":number?,"name":string?,"to":int?,"remove":bool?}"#,
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
