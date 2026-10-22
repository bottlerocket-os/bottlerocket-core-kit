use schnauzer::v2::import::{JsonSettingsResolver, StaticHelperResolver};
use serde_json::json;

struct Importer {
    settings_resolver: JsonSettingsResolver,
    helper_resolver: StaticHelperResolver,
}
schnauzer::impl_template_importer!(Importer, JsonSettingsResolver, StaticHelperResolver);

#[tokio::test]
async fn zram_is_optional_and_preserves_explicit_zero() {
    for size in [
        None,
        Some("0"),
        Some("0%"),
        Some("512Mi"),
        Some("2Gi"),
        Some("25%"),
    ] {
        let mut settings = json!({"settings": {"kernel": {"lockdown": "integrity", "sysctl": {"vm.swappiness": "200"}}}});
        if let Some(size) = size {
            settings["settings"]["kernel"]["zram"] = json!({"size": size});
        }
        let importer = Importer {
            settings_resolver: JsonSettingsResolver::new(settings),
            helper_resolver: StaticHelperResolver,
        };
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../packages/os/corndog-toml");
        let rendered = schnauzer::v2::render_template_file(&importer, path)
            .await
            .unwrap();
        let value: toml::Value = toml::from_str(&rendered).unwrap();
        assert_eq!(value["lockdown"].as_str(), Some("integrity"));
        assert_eq!(value["sysctl"]["vm.swappiness"].as_str(), Some("200"));
        match size {
            None => assert!(value.get("zram").is_none()),
            Some(size) => assert_eq!(value["zram"]["size"].as_str(), Some(size)),
        }
    }
}
