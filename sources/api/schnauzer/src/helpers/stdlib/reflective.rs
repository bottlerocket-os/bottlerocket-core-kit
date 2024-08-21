//! This module contains schnauzer helpers for inspecting the type of a value within a template.
macro_rules! json_type_check_helper {
    ($name:ident, $check_fn:tt) => {
        pub struct $name;

        impl ::handlebars::HelperDef for $name {
            fn call_inner<'reg: 'rc, 'rc>(
                &self,
                helper: &::handlebars::Helper<'reg, 'rc>,
                _r: &'reg ::handlebars::Handlebars<'reg>,
                _ctx: &'rc ::handlebars::Context,
                renderctx: &mut ::handlebars::RenderContext<'reg, 'rc>,
            ) -> ::std::result::Result<::handlebars::ScopedJson<'reg, 'rc>, ::handlebars::RenderError>
            {
                ::log::trace!("Starting $name helper");
                let template_name = $crate::helpers::template_name(&renderctx);
                $crate::helpers::check_param_count(helper, template_name, 1)?;
                let value = $crate::helpers::get_param(helper, 0)?;

                let result = value.$check_fn();

                Ok(::handlebars::ScopedJson::Derived(::serde_json::Value::Bool(
                    result,
                )))
            }
        }
    };
}

json_type_check_helper!(IsBool, is_boolean);
json_type_check_helper!(IsNumber, is_number);
json_type_check_helper!(IsString, is_string);
json_type_check_helper!(IsArray, is_array);
json_type_check_helper!(IsObject, is_object);
json_type_check_helper!(IsNull, is_null);

#[cfg(test)]
mod test {
    use super::*;
    use handlebars::{Handlebars, RenderError};
    use serde::Serialize;
    use serde_json::json;
    use test_case::test_case;

    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.set_strict_mode(true);

        registry.register_helper("is_bool", Box::new(IsBool));
        registry.register_helper("is_number", Box::new(IsNumber));
        registry.register_helper("is_string", Box::new(IsString));
        registry.register_helper("is_array", Box::new(IsArray));
        registry.register_helper("is_object", Box::new(IsObject));
        registry.register_helper("is_null", Box::new(IsNull));

        registry.render_template(tmpl, data)
    }

    #[test_case(
        json!({}),
        r#"{{#if (is_bool true)}}pass{{else}}fail{{/if}}"#;
        "is_bool_true: value is true"
    )]
    #[test_case(
        json!({"input": false}),
        r#"{{#if (is_bool input)}}pass{{else}}fail{{/if}}"#;
        "is_bool_true: value is false"
    )]
    #[test_case(
        json!({"input": 12}),
        r#"{{#if (is_bool input)}}fail{{else}}pass{{/if}}"#;
        "is_bool_false: value is num"
    )]
    #[test_case(
        json!({"input": "hello!"}),
        r#"{{#if (is_bool input)}}fail{{else}}pass{{/if}}"#;
        "is_bool_false: value is string"
    )]
    #[test_case(
        json!({"input": [1, 2, 3]}),
        r#"{{#if (is_bool input)}}fail{{else}}pass{{/if}}"#;
        "is_bool_false: value is array"
    )]
    #[test_case(
        json!({"input": {"inner": "object"}}),
        r#"{{#if (is_bool input)}}fail{{else}}pass{{/if}}"#;
        "is_bool_false: value is object"
    )]
    #[test_case(
        json!({"input": null}),
        r#"{{#if (is_bool input)}}fail{{else}}pass{{/if}}"#;
        "is_bool_false: value is null"
    )]
    #[test_case(
        json!({}),
        r#"{{#if (is_bool settings.a.b.c)}}fail{{else}}pass{{/if}}"#;
        "is_bool: disregards strict mode"
    )]
    #[test_case(
        json!({"input": 12}),
        r#"{{#if (not (is_bool input))}}pass{{else}}fail{{/if}}"#;
        "is_bool: composes"
    )]
    #[test_case(
        json!({"input": true}),
        r#"{{#if (is_number input)}}fail{{else}}pass{{/if}}"#;
        "is_number_false: value is bool"
    )]
    #[test_case(
        json!({"input": 57}),
        r#"{{#if (is_number input)}}pass{{else}}fail{{/if}}"#;
        "is_number_true: value is number"
    )]
    #[test_case(
        json!({"input": "hello"}),
        r#"{{#if (is_number input)}}fail{{else}}pass{{/if}}"#;
        "is_number_false: value is string"
    )]
    #[test_case(
        json!({"input": [1, 2, 3]}),
        r#"{{#if (is_number input)}}fail{{else}}pass{{/if}}"#;
        "is_number_false: value is array"
    )]
    #[test_case(
        json!({"input": {"inner": "object"}}),
        r#"{{#if (is_number input)}}fail{{else}}pass{{/if}}"#;
        "is_number_false: value is object"
    )]
    #[test_case(
        json!({"input": null}),
        r#"{{#if (is_number input)}}fail{{else}}pass{{/if}}"#;
        "is_number_false: value is null"
    )]
    #[test_case(
        json!({}),
        r#"{{#if (is_number settings.a.b.c)}}fail{{else}}pass{{/if}}"#;
        "is_number: disregards strict mode"
    )]
    #[test_case(
        json!({"input": true}),
        r#"{{#if (is_string input)}}fail{{else}}pass{{/if}}"#;
        "is_string_false: value is bool"
    )]
    #[test_case(
        json!({"input": 57}),
        r#"{{#if (is_string input)}}fail{{else}}pass{{/if}}"#;
        "is_string_false: value is number"
    )]
    #[test_case(
        json!({"input": "hello"}),
        r#"{{#if (is_string input)}}pass{{else}}fail{{/if}}"#;
        "is_string_true: value is string"
    )]
    #[test_case(
        json!({"input": [1, 2, 3]}),
        r#"{{#if (is_string input)}}fail{{else}}pass{{/if}}"#;
        "is_string_false: value is array"
    )]
    #[test_case(
        json!({"input": {"inner": "object"}}),
        r#"{{#if (is_string input)}}fail{{else}}pass{{/if}}"#;
        "is_string_false: value is object"
    )]
    #[test_case(
        json!({"input": null}),
        r#"{{#if (is_string input)}}fail{{else}}pass{{/if}}"#;
        "is_string_false: value is null"
    )]
    #[test_case(
        json!({}),
        r#"{{#if (is_string settings.a.b.c)}}fail{{else}}pass{{/if}}"#;
        "is_string: disregards strict mode"
    )]
    #[test_case(
        json!({"input": true}),
        r#"{{#if (is_array input)}}fail{{else}}pass{{/if}}"#;
        "is_array_false: value is bool"
    )]
    #[test_case(
        json!({"input": 57}),
        r#"{{#if (is_array input)}}fail{{else}}pass{{/if}}"#;
        "is_array_false: value is number"
    )]
    #[test_case(
        json!({"input": "hello"}),
        r#"{{#if (is_array input)}}fail{{else}}pass{{/if}}"#;
        "is_array_false: value is string"
    )]
    #[test_case(
        json!({"input": [1, 2, 3]}),
        r#"{{#if (is_array input)}}pass{{else}}fail{{/if}}"#;
        "is_array_true: value is array"
    )]
    #[test_case(
        json!({"input": {"inner": "object"}}),
        r#"{{#if (is_array input)}}fail{{else}}pass{{/if}}"#;
        "is_array_false: value is object"
    )]
    #[test_case(
        json!({"input": null}),
        r#"{{#if (is_array input)}}fail{{else}}pass{{/if}}"#;
        "is_array_false: value is null"
    )]
    #[test_case(
        json!({}),
        r#"{{#if (is_array settings.a.b.c)}}fail{{else}}pass{{/if}}"#;
        "is_array: disregards strict mode"
    )]
    #[test_case(
        json!({"input": true}),
        r#"{{#if (is_object input)}}fail{{else}}pass{{/if}}"#;
        "is_object_false: value is bool"
    )]
    #[test_case(
        json!({"input": 57}),
        r#"{{#if (is_object input)}}fail{{else}}pass{{/if}}"#;
        "is_object_false: value is number"
    )]
    #[test_case(
        json!({"input": "hello"}),
        r#"{{#if (is_object input)}}fail{{else}}pass{{/if}}"#;
        "is_object_false: value is string"
    )]
    #[test_case(
        json!({"input": [1, 2, 3]}),
        r#"{{#if (is_object input)}}fail{{else}}pass{{/if}}"#;
        "is_object_false: value is array"
    )]
    #[test_case(
        json!({"input": {"inner": "object"}}),
        r#"{{#if (is_object input)}}pass{{else}}fail{{/if}}"#;
        "is_object_true: value is object"
    )]
    #[test_case(
        json!({"input": null}),
        r#"{{#if (is_object input)}}fail{{else}}pass{{/if}}"#;
        "is_object_false: value is null"
    )]
    #[test_case(
        json!({}),
        r#"{{#if (is_object settings.a.b.c)}}fail{{else}}pass{{/if}}"#;
        "is_object: disregards strict mode"
    )]
    #[test_case(
        json!({"input": true}),
        r#"{{#if (is_null input)}}fail{{else}}pass{{/if}}"#;
        "is_null_false: value is bool"
    )]
    #[test_case(
        json!({"input": 57}),
        r#"{{#if (is_null input)}}fail{{else}}pass{{/if}}"#;
        "is_null_false: value is number"
    )]
    #[test_case(
        json!({"input": "hello"}),
        r#"{{#if (is_null input)}}fail{{else}}pass{{/if}}"#;
        "is_null_false: value is string"
    )]
    #[test_case(
        json!({"input": [1, 2, 3]}),
        r#"{{#if (is_null input)}}fail{{else}}pass{{/if}}"#;
        "is_null_false: value is array"
    )]
    #[test_case(
        json!({"input": {"inner": "object"}}),
        r#"{{#if (is_null input)}}fail{{else}}pass{{/if}}"#;
        "is_null_false: value is object"
    )]
    #[test_case(
        json!({"input": null}),
        r#"{{#if (is_null input)}}pass{{else}}fail{{/if}}"#;
        "is_null_true: value is null"
    )]
    #[test_case(
        json!({}),
        r#"{{#if (is_null settings.a.b.c)}}pass{{else}}fail{{/if}}"#;
        "is_null_true: value is unset"
    )]
    fn test_reflective_helpers(json_inputs: serde_json::Value, template_str: &str) {
        let result = setup_and_render_template(template_str, &json_inputs);
        assert_eq!(result.unwrap(), "pass");
    }
}
