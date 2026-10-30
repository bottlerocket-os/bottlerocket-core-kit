use schnauzer::v2::import::{JsonSettingsResolver, StaticHelperResolver};
use serde_json::{json, Value};
use test_case::test_case;

struct Importer {
    settings_resolver: JsonSettingsResolver,
    helper_resolver: StaticHelperResolver,
}
schnauzer::impl_template_importer!(Importer, JsonSettingsResolver, StaticHelperResolver);

async fn render(component: &str, quantities: Value) -> Result<String, Box<dyn std::error::Error>> {
    let importer = Importer {
        settings_resolver: JsonSettingsResolver::new(json!({
            "settings": {"kubernetes": {"kube-reserved-memory-min": quantities}}
        })),
        helper_resolver: StaticHelperResolver,
    };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../../packages/release/{component}-memory-min-conf"
    ));
    Ok(schnauzer::v2::render_template_file(&importer, path).await?)
}

#[test_case(Value::Null, [0, 0, 0]; "unset")]
#[test_case(json!({}), [0, 0, 0]; "empty")]
#[test_case(json!({"kubelet": "128Mi", "containerd": "128Mi"}), [134217728, 134217728, 268435456]; "equal")]
#[test_case(json!({"kubelet": "200Mi", "containerd": "300Mi"}), [209715200, 314572800, 524288000]; "different")]
#[test_case(json!({"containerd": "0"}), [0, 0, 0]; "one zero")]
#[test_case(json!({"kubelet": "0", "containerd": "0"}), [0, 0, 0]; "both zero")]
#[test_case(json!({"kubelet": "1", "containerd": "1"}), [1, 1, 2]; "bare bytes")]
#[test_case(json!({"kubelet": "1Ti", "containerd": "0"}), [1 << 40, 0, 1 << 40]; "tebibytes")]
#[test_case(
    json!({"kubelet": "18446744073709551614", "containerd": "1"}),
    [u64::MAX - 1, 1, u64::MAX - 1];
    "capped parent"
)]
#[test_case(json!({"kubelet": "1Gi", "containerd": "100Mi"}), [1073741824, 104857600, 1178599424]; "mixed units")]
#[tokio::test]
async fn templates_default_override_and_preserve_zero(settings: Value, expected: [u64; 3]) {
    for ((component, section), bytes) in [
        ("kubelet", "Service"),
        ("containerd", "Service"),
        ("runtime", "Slice"),
    ]
    .into_iter()
    .zip(expected)
    {
        let output = render(component, settings.clone()).await.unwrap();
        assert_eq!(output.trim(), format!("[{section}]\nMemoryMin={bytes}"));
    }
}

#[test_case(json!({"kubelet": "-1Mi"}); "negative")]
#[test_case(json!({"kubelet": "1..2Mi"}); "range")]
#[test_case(json!({"kubelet": "1.5Gi"}); "fraction")]
#[test_case(json!({"kubelet": "01Mi"}); "leading zero")]
#[test_case(json!({"kubelet": "16Ei"}); "exbibytes")]
#[test_case(json!({"kubelet": "1G"}); "SI suffix")]
#[test_case(json!({"containerd": 123}); "number")]
#[test_case(json!({"containerd": "1Mi\nMemoryMin=0"}); "newline injection")]
#[test_case(json!({"kubelet": "18446744073709551615"}); "u64 max")]
#[test_case(json!({"kubelet": "18446744073709551616"}); "u64 overflow")]
#[test_case(json!({"kubelet": "16777216Ti"}); "unit overflow")]
#[tokio::test]
async fn templates_reject_invalid_values(quantities: Value) {
    assert!(render("runtime", quantities).await.is_err());
}
