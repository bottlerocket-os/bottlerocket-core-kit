//! This module contains schnauzer helpers for inspecting system properties.
use handlebars::{Handlebars, Helper, HelperDef, RenderContext, RenderError, ScopedJson};

/// Path to the cgroup v2 controllers file, which only exists on systems using cgroup v2.
const CGROUP_V2_CONTROLLERS_PATH: &str = "/sys/fs/cgroup/cgroup.controllers";

/// A helper that returns true if the system is using cgroup v2.
///
/// Detection is based on the existence of `/sys/fs/cgroup/cgroup.controllers`,
/// which is a cgroup v2-specific interface file present only when cgroup v2 is
/// mounted in unified mode.
pub struct IsCgroupV2;

impl HelperDef for IsCgroupV2 {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        _helper: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'reg>,
        _ctx: &'rc handlebars::Context,
        _renderctx: &mut RenderContext<'reg, 'rc>,
    ) -> Result<ScopedJson<'reg, 'rc>, RenderError> {
        let is_v2 = std::path::Path::new(CGROUP_V2_CONTROLLERS_PATH).exists();
        Ok(ScopedJson::Derived(serde_json::Value::Bool(is_v2)))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use handlebars::Handlebars;
    use serde_json::json;

    fn setup_and_render_template(tmpl: &str) -> Result<String, RenderError> {
        let mut registry = Handlebars::new();
        registry.register_helper("is_cgroup_v2", Box::new(IsCgroupV2));
        registry.render_template(tmpl, &json!({}))
    }

    #[test]
    fn test_is_cgroup_v2_returns_bool() {
        // We can't control the host cgroup version, but we can verify the helper
        // renders without error and produces a valid conditional result.
        let result =
            setup_and_render_template(r#"{{#if (is_cgroup_v2)}}v2{{else}}v1{{/if}}"#).unwrap();
        assert!(result == "v2" || result == "v1");
    }

    #[test]
    fn test_is_cgroup_v2_composes_with_not() {
        let result =
            setup_and_render_template(r#"{{#if (not (is_cgroup_v2))}}not-v2{{else}}is-v2{{/if}}"#)
                .unwrap();
        assert!(result == "not-v2" || result == "is-v2");
    }
}
