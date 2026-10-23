use schnauzer::v2::import::{JsonSettingsResolver, StaticHelperResolver};
use serde_json::{json, Value};

struct Importer {
    settings_resolver: JsonSettingsResolver,
    helper_resolver: StaticHelperResolver,
}

schnauzer::impl_template_importer!(Importer, JsonSettingsResolver, StaticHelperResolver);

async fn render(version: u8, merge: Option<bool>) -> String {
    let mut settings = json!({
        "settings": {
            "kubernetes": {
                "cluster-domain": "cluster.local",
                "server-tls-bootstrap": true,
                "max-pods": 29,
                "kube-reserved": {"cpu": "100m", "memory": "442Mi"},
                "eviction-hard": {"memory.available": "600Mi"},
                "memory-swap-behavior": "LimitedSwap"
            }
        }
    });
    if let Some(enabled) = merge {
        settings["settings"]["kubernetes"]["merge-default-eviction-settings"] =
            Value::Bool(enabled);
    }
    let importer = Importer {
        settings_resolver: JsonSettingsResolver::new(settings),
        helper_resolver: StaticHelperResolver,
    };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../../packages/kubernetes-1.{version}/kubelet-config"
    ));
    schnauzer::v2::render_template_file(&importer, path)
        .await
        .unwrap()
}

#[tokio::test]
async fn merge_defaults_is_opt_in_on_supported_versions() {
    for version in 33..=37 {
        for enabled in [None, Some(false), Some(true)] {
            let rendered = render(version, enabled).await;
            let field = rendered
                .lines()
                .find(|line| line.starts_with("mergeDefaultEvictionSettings:"));
            match enabled {
                None => assert_eq!(field, None, "Kubernetes 1.{version}"),
                Some(value) => assert_eq!(
                    field,
                    Some(format!("mergeDefaultEvictionSettings: {value}").as_str()),
                    "Kubernetes 1.{version}"
                ),
            }
            assert!(rendered.contains("memory.available: \"600Mi\""));
            assert!(rendered.contains("memorySwap:\n  swapBehavior: LimitedSwap"));
        }
    }
}

#[tokio::test]
async fn older_versions_do_not_render_merge_defaults() {
    for version in [31, 32] {
        let rendered = render(version, Some(true)).await;
        assert!(!rendered.contains("mergeDefaultEvictionSettings"));
    }
}
