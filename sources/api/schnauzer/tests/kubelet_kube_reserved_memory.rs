//! Renders the kubelet config template of every supported Kubernetes version and checks the
//! kubeReserved memory: the default formula `max-pods * 11 + 255` Mi, the formula with its two
//! terms set through `kube-reserved-memory-base-mib` and `kube-reserved-memory-per-pod-mib`, and
//! a set `kube-reserved.memory` that wins over both.
use schnauzer::v2::import::{JsonSettingsResolver, StaticHelperResolver};
use serde_json::{json, Value};
use test_case::test_matrix;

struct Importer {
    settings_resolver: JsonSettingsResolver,
    helper_resolver: StaticHelperResolver,
}

schnauzer::impl_template_importer!(Importer, JsonSettingsResolver, StaticHelperResolver);

async fn render(version: u8, extra: Value) -> String {
    let mut kubernetes = json!({
        "cluster-domain": "cluster.local",
        "server-tls-bootstrap": true,
        "max-pods": 56
    });
    for (key, value) in extra.as_object().unwrap() {
        kubernetes[key] = value.clone();
    }
    let importer = Importer {
        settings_resolver: JsonSettingsResolver::new(
            json!({"settings": {"kubernetes": kubernetes}}),
        ),
        helper_resolver: StaticHelperResolver,
    };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../../packages/kubernetes-1.{version}/kubelet-config"
    ));
    schnauzer::v2::render_template_file(&importer, path)
        .await
        .unwrap()
}

fn kube_reserved_memory(rendered: &str) -> Option<&str> {
    let block = rendered.split("kubeReserved:\n").nth(1)?;
    block
        .lines()
        .take_while(|line| line.starts_with("  "))
        .find_map(|line| line.strip_prefix("  memory: "))
}

#[test_matrix(31..=37)]
#[tokio::test]
async fn default_formula_is_unchanged(version: u8) {
    let rendered = render(version, json!({})).await;
    assert_eq!(kube_reserved_memory(&rendered), Some("\"871Mi\""));
}

#[test_matrix(31..=37)]
#[tokio::test]
async fn terms_change_the_formula(version: u8) {
    let rendered = render(
        version,
        json!({"kube-reserved-memory-base-mib": 512, "kube-reserved-memory-per-pod-mib": 11}),
    )
    .await;
    assert_eq!(kube_reserved_memory(&rendered), Some("\"1128Mi\""));
}

#[test_matrix(31..=37)]
#[tokio::test]
async fn set_memory_wins_over_terms(version: u8) {
    let rendered = render(
        version,
        json!({
            "kube-reserved": {"memory": "2Gi"},
            "kube-reserved-memory-base-mib": 512,
            "kube-reserved-memory-per-pod-mib": 11
        }),
    )
    .await;
    assert_eq!(kube_reserved_memory(&rendered), Some("\"2Gi\""));
}
