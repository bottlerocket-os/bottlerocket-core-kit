use schnauzer::v2::import::{JsonSettingsResolver, StaticHelperResolver};
use serde_json::{json, Value};
use test_case::test_matrix;

struct Importer {
    settings_resolver: JsonSettingsResolver,
    helper_resolver: StaticHelperResolver,
}

schnauzer::impl_template_importer!(Importer, JsonSettingsResolver, StaticHelperResolver);

const DEFAULT: &str = "nvidia-k8s-device-plugin-exec-start-conf";
const COMPAT: &str = "nvidia-k8s-device-plugin-exec-start-conf-compat";
const ENV_PREFIX: &str = "Environment=DP_DISABLE_HEALTHCHECKS=";

async fn render(template: &str, nvidia: Option<Value>) -> String {
    let mut settings = json!({"settings": {"kubelet-device-plugins": {}}});
    if let Some(nvidia) = nvidia {
        settings["settings"]["kubelet-device-plugins"]["nvidia"] = nvidia;
    }
    let importer = Importer {
        settings_resolver: JsonSettingsResolver::new(settings),
        helper_resolver: StaticHelperResolver,
    };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../packages/nvidia-k8s-device-plugin")
        .join(template);
    schnauzer::v2::render_template_file(&importer, path)
        .await
        .unwrap()
}

fn health_check_lines(rendered: &str) -> Vec<&str> {
    rendered
        .lines()
        .filter(|line| line.starts_with(ENV_PREFIX))
        .collect()
}

#[test_matrix([DEFAULT, COMPAT], ["94", "94,119", "all"], [None, Some(true)])]
#[tokio::test]
async fn renders_disable_health_checks_when_set(
    template: &str,
    value: &str,
    enabled: Option<bool>,
) {
    let mut nvidia = json!({"disable-health-checks": value});
    if let Some(enabled) = enabled {
        nvidia["enabled"] = json!(enabled);
    }
    let rendered = render(template, Some(nvidia)).await;

    let expected = format!("{ENV_PREFIX}{value}");
    assert_eq!(health_check_lines(&rendered), vec![expected.as_str()]);

    // systemd only applies Environment= inside the [Service] section.
    let service = rendered.find("[Service]").unwrap();
    let env = rendered.find(&expected).unwrap();
    assert!(service < env);
}

#[test_matrix([DEFAULT, COMPAT], [None, Some(true)])]
#[tokio::test]
async fn omits_disable_health_checks_when_unset(template: &str, enabled: Option<bool>) {
    let nvidia = match enabled {
        Some(enabled) => json!({"enabled": enabled, "pass-device-specs": true}),
        None => json!({"pass-device-specs": true}),
    };
    let rendered = render(template, Some(nvidia)).await;

    assert!(rendered.contains("ExecStart=/usr/bin/nvidia-device-plugin"));
    assert!(!rendered.contains("DP_DISABLE_HEALTHCHECKS"));
}

#[test_matrix([DEFAULT, COMPAT], ["94", "all"])]
#[tokio::test]
async fn omits_disable_health_checks_when_plugin_disabled(template: &str, value: &str) {
    let rendered = render(
        template,
        Some(json!({"enabled": false, "disable-health-checks": value})),
    )
    .await;

    assert!(rendered.contains("ExecStart=/usr/bin/true"));
    assert!(!rendered.contains("DP_DISABLE_HEALTHCHECKS"));
}

#[test_matrix([DEFAULT, COMPAT])]
#[tokio::test]
async fn renders_nothing_without_nvidia_settings(template: &str) {
    let rendered = render(template, None).await;
    assert!(rendered.trim().is_empty());
}
