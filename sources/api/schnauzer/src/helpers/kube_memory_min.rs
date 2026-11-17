//! Translate validated service memory.min settings to systemd byte counts.
use bottlerocket_modeled_types::KubeReservedMemoryMinValue;
use handlebars::{Context, Handlebars, Helper, Output, RenderContext, RenderError};
use serde_json::Value;

fn quantity_bytes(input: &str) -> Result<u64, RenderError> {
    KubeReservedMemoryMinValue::try_from(input)
        .map(|value| value.as_bytes())
        .map_err(|error| RenderError::new(error.to_string()))
}

fn service_bytes(settings: &Value, service: &str) -> Result<u64, RenderError> {
    match settings.get(service).unwrap_or(&Value::Null) {
        Value::Null => Ok(0),
        Value::String(quantity) => quantity_bytes(quantity),
        _ => Err(RenderError::new(
            "memory.min settings must be quantity strings",
        )),
    }
}

/// Render one service's floor or the sum of both children's byte counts.
pub fn kube_memory_min(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    super::check_param_count(helper, super::template_name(renderctx), 2)?;
    let service = super::get_param(helper, 0)?
        .as_str()
        .ok_or_else(|| RenderError::new("memory.min component must be a string"))?;
    let settings = super::get_param(helper, 1)?;
    if !settings.is_null() && !settings.is_object() {
        return Err(RenderError::new("memory.min settings must be an object"));
    }
    let value = match service {
        "kubelet" | "containerd" => service_bytes(settings, service)?,
        "runtime" => service_bytes(settings, "kubelet")?
            .saturating_add(service_bytes(settings, "containerd")?)
            .min(KubeReservedMemoryMinValue::MAX_BYTES),
        _ => return Err(RenderError::new("unknown memory.min component")),
    };
    out.write(&value.to_string())?;
    Ok(())
}
