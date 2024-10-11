use super::{check_param_count, error, get_param, template_name};
use base64::Engine;
use handlebars::{
    handlebars_helper, Context, Handlebars, Helper, HelperDef, Output, RenderContext, RenderError,
    Renderable,
};
use serde::Deserialize;
use serde_json::value::Value;
use snafu::{OptionExt, ResultExt};

pub mod reflective;
pub use reflective::{IsArray, IsBool, IsNull, IsNumber, IsObject, IsString};

// This helper checks if any objects have '"enabled": true' in their properties.
//
// any_enabled takes one argument that is expected to be an array of objects,
// then checks if any of these objects have an "enabled" property set to true.
// If the argument is not an array of objects, does not have an "enabled" property,
// or does not include any objects with this property set to true, it evaluates
// to be falsey.
handlebars_helper!(any_enabled: |arg: Value| {
    #[derive(Deserialize)]
    struct EnablableObject {
        enabled: bool,
    }

    let mut result = false;
    match arg {
        Value::Array(items) => {
            for item in &items {
                let res = serde_json::from_value::<EnablableObject>(item.clone());
                if let Ok(eo) = res {
                    if eo.enabled {
                        result = true;
                        break;
                    }
                }
            }
        }
        Value::Object(items) => {
            for (_, item) in &items {
                let res = serde_json::from_value::<EnablableObject>(item.clone());
                if let Ok(eo) = res {
                    if eo.enabled {
                        result = true;
                        break;
                    }
                }
            }
        }
        _ => ()
    }
    result
});

#[cfg(test)]
mod test_any_enabled {
    use super::*;
    use handlebars::RenderError;
    use serde::Serialize;
    use serde_json::json;
    use std::collections::BTreeMap;

    // A thin wrapper around the handlebars render_template method that includes
    // setup and registration of helpers
    fn setup_and_render_template<T>(tmpl: &str, data: T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("any_enabled", Box::new(any_enabled));

        let mut input_data = BTreeMap::new();
        input_data.insert("input", data);

        registry.render_template(tmpl, &input_data)
    }

    const TEMPLATE: &str = r#"{{#if (any_enabled input)}}enabled{{else}}disabled{{/if}}"#;

    #[test]
    fn test_any_enabled_with_enabled_elements() {
        let result =
            setup_and_render_template(TEMPLATE, json!([{"foo": [], "enabled": true}])).unwrap();
        let expected = "enabled";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_any_enabled_with_enabled_elements_from_map() {
        let result = setup_and_render_template(
            TEMPLATE,
            json!({"foo": {"enabled": false}, "bar": {"enabled": true}}),
        )
        .unwrap();
        let expected = "enabled";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_any_enabled_without_enabled_elements() {
        let result = setup_and_render_template(TEMPLATE, json!([{"enabled": false}])).unwrap();
        let expected = "disabled";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_any_enabled_without_enabled_elements_from_map() {
        let result = setup_and_render_template(
            TEMPLATE,
            json!({"foo": {"enabled": false}, "bar": {"enabled": false}}),
        )
        .unwrap();
        let expected = "disabled";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_any_enabled_with_empty_elements() {
        let result = setup_and_render_template(TEMPLATE, json!([])).unwrap();
        let expected = "disabled";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_any_enabled_with_empty_map() {
        let result = setup_and_render_template(TEMPLATE, json!({})).unwrap();
        let expected = "disabled";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_any_enabled_with_bool_flag_as_string() {
        // Helper is only expected to work with boolean values
        let result = setup_and_render_template(TEMPLATE, json!([{"enabled": "true"}])).unwrap();
        let expected = "disabled";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_any_enabled_with_different_type_array() {
        // Validates no errors if a different kind of struct is passed in
        let result = setup_and_render_template(TEMPLATE, json!([{"name": "fred"}])).unwrap();
        let expected = "disabled";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_any_enabled_with_different_type_map() {
        // Validates no errors if a different kind of struct is passed in
        let result =
            setup_and_render_template(TEMPLATE, json!({"test": {"name": "fred"}})).unwrap();
        let expected = "disabled";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_any_enabled_with_different_type() {
        // Validates no errors when a completely different JSON struct is passed
        let result = setup_and_render_template(TEMPLATE, json!({"state": "enabled"})).unwrap();
        let expected = "disabled";
        assert_eq!(result, expected);
    }
}

/// `base64_decode` decodes base64 encoded text at template render time.
/// It takes a single variable as a parameter: {{base64_decode var}}
pub fn base64_decode(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    // To give context to our errors, get the template name, if available.
    trace!("Starting base64_decode helper");
    let template_name = template_name(renderctx);
    trace!("Template name: {}", &template_name);

    // Check number of parameters, must be exactly one
    trace!("Number of params: {}", helper.params().len());
    check_param_count(helper, template_name, 1)?;

    // Get the resolved key out of the template (param(0)). value() returns
    // a serde_json::Value
    let base64_value = helper
        .param(0)
        .map(|v| v.value())
        .context(error::ParamUnwrapSnafu {})?;
    trace!("Base64 value from template: {}", base64_value);

    // Create an &str from the serde_json::Value
    let base64_str = base64_value
        .as_str()
        .context(error::InvalidTemplateValueSnafu {
            expected: "string",
            value: base64_value.to_owned(),
            template: template_name.to_owned(),
        })?;
    trace!("Base64 string from template: {}", base64_str);

    // Base64 decode the &str
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_str)
        .context(error::Base64DecodeSnafu {
            template: template_name.to_owned(),
        })?;

    // Create a valid utf8 str
    let decoded = std::str::from_utf8(&decoded_bytes).context(error::InvalidUTF8Snafu {
        base64_string: base64_str.to_string(),
        template: template_name.to_owned(),
    })?;
    trace!("Decoded base64: {}", decoded);

    // Write the string out to the template
    out.write(decoded).context(error::TemplateWriteSnafu {
        template: template_name.to_owned(),
    })?;
    Ok(())
}

#[cfg(test)]
mod test_base64_decode {
    use super::*;
    use handlebars::RenderError;
    use serde::Serialize;
    use serde_json::json;

    // A thin wrapper around the handlebars render_template method that includes
    // setup and registration of helpers
    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("base64_decode", Box::new(base64_decode));

        registry.render_template(tmpl, data)
    }

    #[test]
    fn renders_decoded_base64() {
        let result =
            setup_and_render_template("{{base64_decode var}}", &json!({"var": "SGk="})).unwrap();
        assert_eq!(result, "Hi")
    }

    #[test]
    fn does_not_render_invalid_base64() {
        assert!(setup_and_render_template("{{base64_decode var}}", &json!({"var": "hi"})).is_err())
    }

    #[test]
    fn does_not_render_invalid_utf8() {
        // "wygk" is the invalid UTF8 string "\xc3\x28" base64 encoded
        assert!(
            setup_and_render_template("{{base64_decode var}}", &json!({"var": "wygK"})).is_err()
        )
    }

    #[test]
    fn base64_helper_with_missing_param() {
        assert!(setup_and_render_template("{{base64_decode}}", &json!({"var": "foo"})).is_err());
    }

    #[test]
    fn base64_helper_with_extra_param() {
        assert!(setup_and_render_template(
            "{{base64_decode var1 var2}}",
            &json!({"var1": "Zm9v", "var2": "YmFy"})
        )
        .is_err());
    }
}

/// `join_map` lets you join together strings in a map with given characters, for example when
/// you're writing values out to a configuration file.
///
/// The map is expected to be a single level deep, with string keys and string values.
///
/// The first parameter is the character to use to join keys to values; the second parameter is the
/// character to use to join pairs; the third parameter is the name of the map.  The third
/// parameter is a literal string that describes the behavior you want if the map is missing from
/// settings; "fail-if-missing" to fail the template, or "no-fail-if-missing" to continue but write
/// out nothing for this invocation of the helper.
///
/// Example:
///    {{ join_map "=" "," "fail-if-missing" map }}
///    ...where `map` is: {"hi": "there", "whats": "up"}
///    ...will produce: "hi=there,whats=up"
pub fn join_map(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    trace!("Starting join_map helper");
    let template_name = template_name(renderctx);
    trace!("Template name: {}", &template_name);

    trace!("Number of params: {}", helper.params().len());
    check_param_count(helper, template_name, 4)?;

    // Pull out the parameters and confirm their types
    let join_key_val = get_param(helper, 0)?;
    let join_key = join_key_val
        .as_str()
        .with_context(|| error::InvalidTemplateValueSnafu {
            expected: "string",
            value: join_key_val.to_owned(),
            template: template_name.to_owned(),
        })?;
    trace!("Character used to join keys to values: {}", join_key);

    let join_pairs_val = get_param(helper, 1)?;
    let join_pairs = join_pairs_val
        .as_str()
        .with_context(|| error::InvalidTemplateValueSnafu {
            expected: "string",
            value: join_pairs_val.to_owned(),
            template: template_name.to_owned(),
        })?;
    trace!("Character used to join pairs: {}", join_pairs);

    let fail_behavior_val = get_param(helper, 2)?;
    let fail_behavior_str =
        fail_behavior_val
            .as_str()
            .with_context(|| error::InvalidTemplateValueSnafu {
                expected: "string",
                value: join_pairs_val.to_owned(),
                template: template_name.to_owned(),
            })?;
    let fail_if_missing = match fail_behavior_str {
        "fail-if-missing" => true,
        "no-fail-if-missing" => false,
        _ => {
            return Err(RenderError::from(
                error::TemplateHelperError::InvalidTemplateValue {
                    expected: "fail-if-missing or no-fail-if-missing",
                    value: fail_behavior_val.to_owned(),
                    template: template_name.to_owned(),
                },
            ))
        }
    };
    trace!(
        "Will we fail if missing the specified map: {}",
        fail_if_missing
    );

    let map_value = get_param(helper, 3)?;
    // If the requested setting is not set, we check the user's requested fail-if-missing behavior
    // to determine whether to fail hard or just write nothing quietly.
    if !map_value.is_object() {
        if fail_if_missing {
            return Err(RenderError::from(
                error::TemplateHelperError::MissingTemplateData {
                    template: template_name.to_owned(),
                },
            ));
        } else {
            return Ok(());
        }
    }
    let map = map_value.as_object().context(error::InternalSnafu {
        msg: "Already confirmed map is_object but as_object failed",
    })?;
    trace!("Map to join: {:?}", map);

    // Join the key/value pairs with requested string
    let mut pairs = Vec::new();
    for (key, val_value) in map.into_iter() {
        // We don't want the JSON form of scalars, we want the Display form of the Rust type inside.
        let val = match val_value {
            // these ones Display as their simple scalar selves
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.to_string(),
            // Null not supported; probably don't want blanks in config files, and we don't have a
            // use for this yet; consider carefully if/when we do
            Value::Null => {
                return Err(RenderError::from(
                    error::TemplateHelperError::InvalidTemplateValue {
                        expected: "non-null",
                        value: val_value.to_owned(),
                        template: template_name.to_owned(),
                    },
                ))
            }
            // composite types unsupported
            Value::Array(_) | Value::Object(_) => {
                return Err(RenderError::from(
                    error::TemplateHelperError::InvalidTemplateValue {
                        expected: "scalar",
                        value: val_value.to_owned(),
                        template: template_name.to_owned(),
                    },
                ))
            }
        };

        // Do the actual key/value join.
        pairs.push(format!("{}{}{}", key, join_key, val));
    }

    // Join all pairs with the given string.
    let joined = pairs.join(join_pairs);
    trace!("Joined output: {}", joined);

    // Write the string out to the template
    out.write(&joined).context(error::TemplateWriteSnafu {
        template: template_name.to_owned(),
    })?;
    Ok(())
}

#[cfg(test)]
mod test_join_map {
    use super::*;
    use handlebars::RenderError;
    use serde::Serialize;
    use serde_json::json;

    // A thin wrapper around the handlebars render_template method that includes
    // setup and registration of helpers
    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("join_map", Box::new(join_map));

        registry.render_template(tmpl, data)
    }

    #[test]
    fn single_pair() {
        let result = setup_and_render_template(
            "{{join_map \"=\" \",\" \"fail-if-missing\" map}}",
            &json!({"map": {"hi": "there"}}),
        )
        .unwrap();
        assert_eq!(result, "hi=there")
    }

    #[test]
    fn basic() {
        let result = setup_and_render_template(
            "{{join_map \"=\" \",\" \"fail-if-missing\" map}}",
            &json!({"map": {"hi": "there", "whats": "up"}}),
        )
        .unwrap();
        assert_eq!(result, "hi=there,whats=up")
    }

    #[test]
    fn number() {
        let result = setup_and_render_template(
            "{{join_map \"=\" \",\" \"fail-if-missing\" map}}",
            &json!({"map": {"hi": 42}}),
        )
        .unwrap();
        assert_eq!(result, "hi=42")
    }

    #[test]
    fn boolean() {
        let result = setup_and_render_template(
            "{{join_map \"=\" \",\" \"fail-if-missing\" map}}",
            &json!({"map": {"hi": true}}),
        )
        .unwrap();
        assert_eq!(result, "hi=true")
    }

    #[test]
    fn invalid_nested_map() {
        setup_and_render_template(
            "{{join_map \"=\" \",\" \"fail-if-missing\" map}}",
            &json!({"map": {"hi": {"too": "deep"}}}),
        )
        .unwrap_err();
    }

    #[test]
    fn fail_if_missing() {
        setup_and_render_template(
            "{{join_map \"=\" \",\" \"fail-if-missing\" map}}",
            &json!({}),
        )
        // Requested failure if map was missing, should fail
        .unwrap_err();
    }

    #[test]
    fn no_fail_if_missing() {
        let result = setup_and_render_template(
            "{{join_map \"=\" \",\" \"no-fail-if-missing\" map}}",
            &json!({}),
        )
        .unwrap();
        // Requested no failure even if map was missing, should get no output
        assert_eq!(result, "")
    }

    #[test]
    fn invalid_fail_if_missing() {
        setup_and_render_template("{{join_map \"=\" \",\" \"sup\" map}}", &json!({}))
            // Invalid failure mode 'sup'
            .unwrap_err();
    }
}

/// `default` lets you specify the default value for a key in a template in case that key isn't
/// set.  The first argument is the default (scalar) value; the second argument is the key (with
/// scalar value) to check and insert if it is set.
pub fn default(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    trace!("Starting default helper");
    let template_name = template_name(renderctx);
    trace!("Template name: {}", &template_name);

    trace!("Number of params: {}", helper.params().len());
    check_param_count(helper, template_name, 2)?;

    // Pull out the parameters and confirm their types
    let default_val = get_param(helper, 0)?;
    let default = match default_val {
        // these ones Display as their simple scalar selves
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.to_string(),
        // Null isn't allowed - we're here to give a default!
        // And composite types are unsupported.
        Value::Null | Value::Array(_) | Value::Object(_) => {
            return Err(RenderError::from(
                error::TemplateHelperError::InvalidTemplateValue {
                    expected: "non-null scalar",
                    value: default_val.to_owned(),
                    template: template_name.to_owned(),
                },
            ))
        }
    };
    trace!("Default value if key is not set: {}", default);

    let requested_value = get_param(helper, 1)?;
    let value = match requested_value {
        // these ones Display as their simple scalar selves
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.to_string(),
        // If no value is set, use the given default.
        Value::Null => default,
        // composite types unsupported
        Value::Array(_) | Value::Object(_) => {
            return Err(RenderError::from(
                error::TemplateHelperError::InvalidTemplateValue {
                    expected: "scalar",
                    value: requested_value.to_owned(),
                    template: template_name.to_owned(),
                },
            ))
        }
    };

    // Write the string out to the template
    out.write(&value).context(error::TemplateWriteSnafu {
        template: template_name.to_owned(),
    })?;
    Ok(())
}

#[cfg(test)]
mod test_default {
    use super::*;
    use handlebars::RenderError;
    use serde::Serialize;
    use serde_json::json;

    // A thin wrapper around the handlebars render_template method that includes
    // setup and registration of helpers
    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("default", Box::new(default));

        registry.render_template(tmpl, data)
    }

    #[test]
    fn have_setting() {
        let result =
            setup_and_render_template("{{default \"42\" setting}}", &json!({"setting": "hi"}))
                .unwrap();
        assert_eq!(result, "hi")
    }

    #[test]
    fn dont_have_setting() {
        let result = setup_and_render_template(
            "{{default \"42\" setting}}",
            &json!({"not-the-setting": "hi"}),
        )
        .unwrap();
        assert_eq!(result, "42")
    }

    #[test]
    fn have_setting_bool() {
        let result =
            setup_and_render_template("{{default \"42\" setting}}", &json!({"setting": true}))
                .unwrap();
        assert_eq!(result, "true")
    }

    #[test]
    fn dont_have_setting_bool() {
        let result = setup_and_render_template(
            "{{default \"42\" setting}}",
            &json!({"not-the-setting": true}),
        )
        .unwrap();
        assert_eq!(result, "42")
    }

    #[test]
    fn have_setting_number() {
        let result =
            setup_and_render_template("{{default \"42\" setting}}", &json!({"setting": 42.42}))
                .unwrap();
        assert_eq!(result, "42.42")
    }

    #[test]
    fn dont_have_setting_number() {
        let result = setup_and_render_template(
            "{{default \"42\" setting}}",
            &json!({"not-the-setting": 42.42}),
        )
        .unwrap();
        assert_eq!(result, "42")
    }

    #[test]
    fn number_default() {
        let result =
            setup_and_render_template("{{default 42 setting}}", &json!({"not-the-setting": 42.42}))
                .unwrap();
        assert_eq!(result, "42")
    }

    #[test]
    fn bool_default() {
        let result = setup_and_render_template(
            "{{default true setting}}",
            &json!({"not-the-setting": 42.42}),
        )
        .unwrap();
        assert_eq!(result, "true")
    }
}

/// The `if_not_null` helper is used to check when a value is not null. This is
/// useful especially for falsy values such as `false`, `0`, or `""` to
/// distinguish between "not set" and "false".
#[derive(Clone, Copy)]
pub struct IfNotNullHelper;

impl HelperDef for IfNotNullHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        helper: &Helper<'reg, 'rc>,
        registry: &'reg Handlebars<'reg>,
        ctx: &'rc Context,
        renderctx: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> Result<(), RenderError> {
        trace!("Starting if_not_null helper");
        let template_name = template_name(renderctx);
        trace!("Template name: {}", &template_name);

        trace!("Number of params: {}", helper.params().len());
        check_param_count(helper, template_name, 1)?;

        let param = get_param(helper, 0)?;

        let tmpl = if !param.is_null() {
            helper.template()
        } else {
            helper.inverse()
        };
        match tmpl {
            Some(t) => t.render(registry, ctx, renderctx, out),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod test_if_not_null {
    use super::*;
    use handlebars::RenderError;
    use serde::Serialize;
    use serde_json::json;

    // A thin wrapper around the handlebars render_template method that includes
    // setup and registration of helpers
    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("if_not_null", Box::new(IfNotNullHelper));

        registry.render_template(tmpl, data)
    }

    #[test]
    fn null_value() {
        let result =
            setup_and_render_template("{{#if_not_null setting}}foo{{/if_not_null}}", &json!({}))
                .unwrap();
        assert_eq!(result, "")
    }

    #[test]
    fn render_else() {
        let result = setup_and_render_template(
            "{{#if_not_null setting}}foo{{else}}bar{{/if_not_null}}",
            &json!({}),
        )
        .unwrap();
        assert_eq!(result, "bar")
    }

    #[test]
    fn explicit_null_value() {
        let result = setup_and_render_template(
            "{{#if_not_null setting}}foo{{/if_not_null}}",
            &json!({"setting": None::<()>}),
        )
        .unwrap();
        assert_eq!(result, "")
    }

    #[test]
    fn falsy_number_value() {
        let result = setup_and_render_template(
            "{{#if_not_null setting}}foo{{/if_not_null}}",
            &json!({"setting": 0}),
        )
        .unwrap();
        assert_eq!(result, "foo")
    }

    #[test]
    fn falsy_string_value() {
        let result = setup_and_render_template(
            "{{#if_not_null setting}}foo{{/if_not_null}}",
            &json!({"setting": ""}),
        )
        .unwrap();
        assert_eq!(result, "foo")
    }

    #[test]
    fn falsy_bool_value() {
        let result = setup_and_render_template(
            "{{#if_not_null setting}}foo{{/if_not_null}}",
            &json!({"setting": false}),
        )
        .unwrap();
        assert_eq!(result, "foo")
    }

    #[test]
    fn falsy_array_value() {
        let result = setup_and_render_template(
            "{{#if_not_null setting}}foo{{/if_not_null}}",
            &json!({"setting": Vec::<()>::new()}),
        )
        .unwrap();
        assert_eq!(result, "foo")
    }
}

/// `join_array` is used to join an array of scalar strings into an array of
/// quoted, delimited strings. The delimiter must be specified.
///
/// # Example
///
/// Consider an array of values: `[ "a", "b", "c" ]` stored in a setting such as
/// `settings.somewhere.foo-list`. In our template we can write:
/// `{{ join_array ", " settings.somewhere.foo-list }}`
///
/// This will render `"a", "b", "c"`.
pub fn join_array(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    trace!("Starting join_array helper");
    let template_name = template_name(renderctx);
    check_param_count(helper, template_name, 2)?;

    // get the delimiter
    let delimiter_param = get_param(helper, 0)?;
    let delimiter = delimiter_param
        .as_str()
        .with_context(|| error::JoinStringsWrongTypeSnafu {
            expected_type: "string",
            value: delimiter_param.to_owned(),
            template: template_name,
        })?;

    // get the array
    let array_param = get_param(helper, 1)?;
    let array = array_param
        .as_array()
        .with_context(|| error::JoinStringsWrongTypeSnafu {
            expected_type: "array",
            value: array_param.to_owned(),
            template: template_name,
        })?;

    let mut result = String::new();
    for (i, value) in array.iter().enumerate() {
        if i > 0 {
            result.push_str(delimiter);
        }
        result.push_str(
            format!(
                "\"{}\"",
                value.as_str().context(error::JoinStringsWrongTypeSnafu {
                    expected_type: "string",
                    value: array.to_owned(),
                    template: template_name,
                })?
            )
            .as_str(),
        );
    }

    // write it to the template
    out.write(&result)
        .with_context(|_| error::TemplateWriteSnafu {
            template: template_name.to_owned(),
        })?;

    Ok(())
}

#[cfg(test)]
mod test_join_array {
    use super::*;
    use handlebars::RenderError;
    use serde::Serialize;
    use serde_json::json;

    // A thin wrapper around the handlebars render_template method that includes
    // setup and registration of helpers
    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("join_array", Box::new(join_array));

        registry.render_template(tmpl, data)
    }

    const TEMPLATE: &str = r#"{{join_array ", " settings.foo-list}}"#;

    #[test]
    fn join_array_empty() {
        let result =
            setup_and_render_template(TEMPLATE, &json!({"settings": {"foo-list": []}})).unwrap();
        let expected = "";
        assert_eq!(result, expected);
    }

    #[test]
    fn join_array_one_item() {
        let result =
            setup_and_render_template(TEMPLATE, &json!({"settings": {"foo-list": ["a"]}})).unwrap();
        let expected = r#""a""#;
        assert_eq!(result, expected);
    }

    #[test]
    fn join_array_two_items() {
        let result =
            setup_and_render_template(TEMPLATE, &json!({"settings": {"foo-list": ["a", "b"]}}))
                .unwrap();
        let expected = r#""a", "b""#;
        assert_eq!(result, expected);
    }

    #[test]
    fn join_array_two_delimiter() {
        let template = r#"{{join_array "~ " settings.foo-list}}"#;
        let result = setup_and_render_template(
            template,
            &json!({"settings": {"foo-list": ["a", "b", "c"]}}),
        )
        .unwrap();
        let expected = r#""a"~ "b"~ "c""#;
        assert_eq!(result, expected);
    }

    #[test]
    fn join_array_empty_item() {
        let result =
            setup_and_render_template(TEMPLATE, &json!({"settings": {"foo-list": ["a", "", "c"]}}))
                .unwrap();
        let expected = r#""a", "", "c""#;
        assert_eq!(result, expected);
    }
}

/// `goarch` takes one parameter, the name of a machine architecture, and converts it to the "Go"
/// form, so named because its use in golang popularized it elsewhere.
///
/// The canonical architecture names in Bottlerocket are things like "x86_64" and "aarch64"; goarch
/// converts these to "amd64" and "arm64", etc.
pub fn goarch(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    trace!("Starting goarch helper");
    let template_name = template_name(renderctx);
    check_param_count(helper, template_name, 1)?;

    // Retrieve the given arch string
    let arch_val = get_param(helper, 0)?;
    let arch_str = arch_val
        .as_str()
        .with_context(|| error::InvalidTemplateValueSnafu {
            expected: "string",
            value: arch_val.to_owned(),
            template: template_name.to_owned(),
        })?;

    // Transform the arch string
    let goarch = match arch_str {
        "x86_64" | "amd64" => "amd64",
        "aarch64" | "arm64" => "arm64",
        _ => {
            return Err(RenderError::from(error::TemplateHelperError::UnknownArch {
                given: arch_str.to_string(),
            }))
        }
    };

    // write it to the template
    out.write(goarch)
        .with_context(|_| error::TemplateWriteSnafu {
            template: template_name.to_owned(),
        })?;

    Ok(())
}

#[cfg(test)]
mod test_goarch {
    use super::*;
    use handlebars::RenderError;
    use serde::Serialize;
    use serde_json::json;

    // A thin wrapper around the handlebars render_template method that includes
    // setup and registration of helpers
    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("goarch", Box::new(goarch));

        registry.render_template(tmpl, data)
    }

    #[test]
    fn good_arches() {
        for (arch, expected) in &[
            ("x86_64", "amd64"),
            ("amd64", "amd64"),
            ("aarch64", "arm64"),
            ("arm64", "arm64"),
        ] {
            let result =
                setup_and_render_template("{{ goarch os.arch }}", &json!({"os": {"arch": arch}}))
                    .unwrap();
            assert_eq!(result, *expected);
        }
    }

    #[test]
    fn bad_arches() {
        for bad_arch in &["", "amdarm", "x86", "aarch32"] {
            setup_and_render_template("{{ goarch os.arch }}", &json!({ "os": {"arch": bad_arch }}))
                .unwrap_err();
        }
    }
}

/// This helper negates a boolean value, or returns a default value when the provided key wasn't
/// set.
///
/// The first argument for the helper is the default value; the second argument is the key to
/// negate. Both values must be booleans, otherwise the helper will return an error. The default
/// value will be returned as it is if the provided key is missing.
pub fn negate_or_else(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    // To give context to our errors, get the template name, if available.
    trace!("Starting negate_or_else helper");
    let template_name = template_name(renderctx);
    trace!("Template name: {}", &template_name);

    // Check number of parameters, must be exactly two (the value to negate and the default value)
    trace!("Number of params: {}", helper.params().len());
    check_param_count(helper, template_name, 2)?;

    let fallback_value = get_param(helper, 0)?;
    let value_to_negate = get_param(helper, 1)?;

    let fallback = match fallback_value {
        Value::Bool(b) => b,
        _ => {
            return Err(RenderError::from(
                error::TemplateHelperError::InvalidTemplateValue {
                    expected: "boolean",
                    value: fallback_value.to_owned(),
                    template: template_name.to_owned(),
                },
            ))
        }
    };

    let output = match value_to_negate {
        Value::Bool(b) => !b,
        Value::Null => *fallback,
        _ => {
            return Err(RenderError::from(
                error::TemplateHelperError::InvalidTemplateValue {
                    expected: "boolean",
                    value: value_to_negate.to_owned(),
                    template: template_name.to_owned(),
                },
            ))
        }
    };

    out.write(&(output).to_string())
        .context(error::TemplateWriteSnafu {
            template: template_name.to_owned(),
        })?;

    Ok(())
}

#[cfg(test)]
mod test_negate_or_else {
    use crate::helpers::negate_or_else;
    use handlebars::{Handlebars, RenderError};
    use serde::Serialize;
    use serde_json::json;

    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("negate_or_else", Box::new(negate_or_else));

        registry.render_template(tmpl, data)
    }

    #[test]
    fn test_negated_values() {
        let template: &str = r#"{{negate_or_else false settings.value}}"#;

        let test_cases = [
            (json!({"settings": {"value": true}}), "false"),
            (json!({"settings": {"value": false}}), "true"),
            (json!({"settings": {"value": None::<bool>}}), "false"),
        ];

        test_cases.iter().for_each(|test_case| {
            let (config, expected) = test_case;
            let rendered = setup_and_render_template(template, config).unwrap();
            assert!(expected == &rendered);
        });
    }

    #[test]
    fn test_fails_when_not_booleans() {
        let test_cases = [
            json!({"settings": {"value": []}}),
            json!({"settings": {"value": {}}}),
            json!({"settings": {"value": ""}}),
        ];

        let template: &str = r#"{{negate_or_else false settings.value}}"#;

        test_cases.iter().for_each(|test_case| {
            let rendered = setup_and_render_template(template, test_case);
            assert!(rendered.is_err());
        });
    }
}

/// `toml_encode` accepts arbitrary input and encodes it as a toml string
///
/// # Example
///
/// Consider an array of values: `[ "a", "b", "c" ]` stored in a setting such as
/// `settings.somewhere.foo-list`. In our template we can write:
/// `{{ toml_encode settings.somewhere.foo-list }}`
///
/// This will render `["a", "b", "c"]`.
///
/// Similarly, for a string: `"string"`, the template {{ toml-encode "string" }}
/// will render `"string"`.
pub fn toml_encode(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    trace!("Starting toml_encode helper");
    let template_name = template_name(renderctx);
    check_param_count(helper, template_name, 1)?;

    // get the string
    let encode_param = get_param(helper, 0)?;
    let toml_value: toml::Value =
        serde_json::from_value(encode_param.to_owned()).with_context(|_| {
            error::TomlEncodeSnafu {
                value: encode_param.to_owned(),
                template: template_name,
            }
        })?;

    let result = toml_value.to_string();

    // write it to the template
    out.write(&result)
        .with_context(|_| error::TemplateWriteSnafu {
            template: template_name.to_owned(),
        })?;

    Ok(())
}

#[cfg(test)]
mod test_toml_encode {
    use crate::helpers::toml_encode;
    use handlebars::{Handlebars, RenderError};
    use serde::Serialize;
    use serde_json::json;

    // A thin wrapper around the handlebars render_template method that includes
    // setup and registration of helpers
    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("toml_encode", Box::new(toml_encode));

        registry.render_template(tmpl, data)
    }

    const TEMPLATE: &str = r#"{{ toml_encode settings.foo-string }}"#;

    #[test]
    fn toml_encode_map() {
        let result = setup_and_render_template(
            TEMPLATE,
            &json!({"settings": {"foo-string": {"hello": "world"}}}),
        )
        .unwrap();
        let expected = r#"{ hello = "world" }"#;
        assert_eq!(result, expected);
    }

    #[test]
    fn toml_encode_empty() {
        let result =
            setup_and_render_template(TEMPLATE, &json!({"settings": {"foo-string": []}})).unwrap();
        let expected = r#"[]"#;
        assert_eq!(result, expected);
    }

    #[test]
    fn toml_encode_empty_string() {
        let result =
            setup_and_render_template(TEMPLATE, &json!({"settings": {"foo-string": [""]}}))
                .unwrap();
        let expected = r#"[""]"#;
        assert_eq!(result, expected);
    }

    #[test]
    fn toml_encode_toml_injection_1() {
        let result = setup_and_render_template(
            TEMPLATE,
            &json!({"settings": {"foo-string": [ "apiclient set motd=hello', 'echo pwned\""]}}),
        )
        .unwrap();
        let expected = "['''apiclient set motd=hello', 'echo pwned\"''']";
        assert_eq!(result, expected);
    }

    #[test]
    fn toml_encode_toml_injection_2() {
        let result = setup_and_render_template(
            TEMPLATE,
            &json!({"settings": {"foo-string": [ "apiclient set motd=hello\", \"echo pwned\""]}}),
        )
        .unwrap();
        let expected = "['apiclient set motd=hello\", \"echo pwned\"']";
        assert_eq!(result, expected);
    }

    #[test]
    fn toml_encode_toml_injection_3() {
        let result = setup_and_render_template(
            TEMPLATE,
            &json!({"settings": {"foo-string": [ "apiclient set motd=hello\", \"echo pwned\", 'echo pwned2'"]}}),
        )
        .unwrap();
        let expected = "[\"apiclient set motd=hello\\\", \\\"echo pwned\\\", 'echo pwned2'\"]";
        assert_eq!(result, expected);
    }
}
