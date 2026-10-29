use schnauzer::v2::import::{JsonSettingsResolver, StaticHelperResolver};
use serde_json::json;
use test_case::test_case;

struct Importer {
    settings_resolver: JsonSettingsResolver,
    helper_resolver: StaticHelperResolver,
}
schnauzer::impl_template_importer!(Importer, JsonSettingsResolver, StaticHelperResolver);

#[test_case(None; "unset")]
#[test_case(Some("0"); "zero")]
#[test_case(Some("0%"); "zero percent")]
#[test_case(Some("512Mi"); "mebibytes")]
#[test_case(Some("2Gi"); "gibibytes")]
#[test_case(Some("25%"); "percent")]
#[tokio::test]
async fn renders_zram_size(size: Option<&str>) {
    let mut settings = json!({"settings": {"kernel": {"lockdown": "integrity", "sysctl": {"vm.swappiness": "200"}}}});
    if let Some(size) = size {
        settings["settings"]["kernel"]["zram"] = json!({"size": size});
    }
    let importer = Importer {
        settings_resolver: JsonSettingsResolver::new(settings),
        helper_resolver: StaticHelperResolver,
    };
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../packages/os/corndog-toml");
    let rendered = schnauzer::v2::render_template_file(&importer, path)
        .await
        .unwrap();
    let value: toml::Value = toml::from_str(&rendered).unwrap();
    assert_eq!(value["lockdown"].as_str(), Some("integrity"));
    assert_eq!(value["sysctl"]["vm.swappiness"].as_str(), Some("200"));
    assert_eq!(
        value.get("zram").and_then(|zram| zram["size"].as_str()),
        size
    );
}
