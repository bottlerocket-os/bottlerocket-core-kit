/*!
*digestion-image-verifier* verifies container image digests against an allowlist.

Containerd invokes: `digestion-image-verifier -name <ref> -digest <sha256:...>`

If no trust policy is configured, all images are allowed.
*/

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use image_verifiers::{args, logging, policy, reference};
use log::{debug, error};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process;

const DIGESTION_TRUST_POLICY: &str = "/etc/containerd/image-verifiers/digestion/trustpolicy.json";
// Per-image drop-in dir for host containers, keyed by base64url of the reference.
const HOST_CTR_DIGESTION_TRUST_POLICY_DIR: &str =
    "/etc/host-containerd/image-verifiers/digestion/trustpolicy.d";
// argv[0] when invoked via the host bin_dir symlink; selects the host policy.
const HOST_CTR_VERIFIER_NAME: &str = "host-ctr-digestion-image-verifier";
const SUPPORTED_VERSION: &str = "1.0";

/// Digestion trust policy deserialized from trustpolicy.json.
#[derive(Deserialize)]
struct TrustPolicy {
    #[serde(default = "default_version")]
    version: String,
    #[serde(rename = "trustedDigests")]
    trusted_digests: Vec<String>,
}

fn default_version() -> String {
    SUPPORTED_VERSION.to_string()
}

impl policy::Policy for TrustPolicy {
    fn is_empty(&self) -> bool {
        self.trusted_digests.is_empty()
    }

    fn validate(&self) -> Result<(), String> {
        if self.version != SUPPORTED_VERSION {
            return Err(format!("unsupported policy version {}", self.version));
        }
        Ok(())
    }
}

/// Trust policy path: a per-image drop-in when invoked as the host verifier,
/// else the orchestrator's single fixed policy file.
fn trust_policy_path(called_as: &str, image_ref: &str) -> PathBuf {
    if called_as == HOST_CTR_VERIFIER_NAME {
        let key = URL_SAFE_NO_PAD.encode(image_ref.as_bytes());
        Path::new(HOST_CTR_DIGESTION_TRUST_POLICY_DIR).join(format!("{key}.json"))
    } else {
        PathBuf::from(DIGESTION_TRUST_POLICY)
    }
}

/// The base name the binary was invoked as (argv[0]); drives policy selection.
fn invoked_name() -> String {
    std::env::args_os()
        .next()
        .map(PathBuf::from)
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_default()
}

/// Loads the trust policy for the given image reference.
fn load_trust_policy(image_ref: &str) -> policy::Result<Option<TrustPolicy>> {
    policy::load(&trust_policy_path(&invoked_name(), image_ref))
}

fn main() {
    logging::init();
    let args: args::Args = args::parse_go_style_args();
    let image_ref = reference::construct(&args.name, &args.digest);

    debug!("verifying image: {}", image_ref);

    let policy = match load_trust_policy(&args.name) {
        Ok(Some(p)) => p,
        Ok(None) => {
            debug!("image verification skipped: no trust policy configured");
            return;
        }
        Err(e) => {
            error!("image verification failed: {}", e);
            process::exit(1);
        }
    };

    if policy.trusted_digests.contains(&args.digest) {
        debug!("image verification successful");
    } else {
        error!("image verification failed: digest not in allowlist");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image_verifiers::policy::Policy;
    use test_case::test_case;

    #[test]
    fn parse_valid_policy() {
        let json = r#"{"version": "1.0", "trustedDigests": ["sha256:abc", "sha256:def"]}"#;
        let policy: TrustPolicy = serde_json::from_str(json).unwrap();
        assert_eq!(policy.version, "1.0");
        assert_eq!(policy.trusted_digests, vec!["sha256:abc", "sha256:def"]);
    }

    #[test]
    fn parse_policy_without_version_defaults() {
        let json = r#"{"trustedDigests": ["sha256:abc"]}"#;
        let policy: TrustPolicy = serde_json::from_str(json).unwrap();
        assert_eq!(policy.version, "1.0");
    }

    #[test]
    fn parse_policy_empty_digests() {
        let json = r#"{"version": "1.0", "trustedDigests": []}"#;
        let policy: TrustPolicy = serde_json::from_str(json).unwrap();
        assert!(policy.trusted_digests.is_empty());
    }

    #[test]
    fn parse_invalid_json_fails() {
        let json = r#"{"version": "1.0", trustedDigests: []}"#;
        assert!(serde_json::from_str::<TrustPolicy>(json).is_err());
    }

    #[test]
    fn version_validation_rejects_unsupported() {
        let json = r#"{"version": "2.0", "trustedDigests": ["sha256:abc"]}"#;
        let policy: TrustPolicy = serde_json::from_str(json).unwrap();
        assert!(policy.validate().is_err());
    }

    #[test_case("sha256:abc", &["sha256:abc", "sha256:def"], true; "digest in list")]
    #[test_case("sha256:xyz", &["sha256:abc", "sha256:def"], false; "digest not in list")]
    fn test_digest_matching(digest: &str, trusted: &[&str], expected: bool) {
        let trusted: Vec<String> = trusted.iter().map(|s| s.to_string()).collect();
        assert_eq!(trusted.contains(&digest.to_string()), expected);
    }

    #[test]
    fn orchestrator_uses_fixed_single_file() {
        assert_eq!(
            trust_policy_path("digestion-image-verifier", "example.com/image:tag"),
            PathBuf::from(DIGESTION_TRUST_POLICY),
        );
    }

    #[test]
    fn host_ctr_uses_per_reference_dropin() {
        let image_ref = "example.com/image:tag";
        let key = URL_SAFE_NO_PAD.encode(image_ref.as_bytes());
        assert_eq!(
            trust_policy_path(HOST_CTR_VERIFIER_NAME, image_ref),
            Path::new(HOST_CTR_DIGESTION_TRUST_POLICY_DIR).join(format!("{key}.json")),
        );
    }

    #[test]
    fn host_ctr_distinct_references_distinct_files() {
        let a = trust_policy_path(HOST_CTR_VERIFIER_NAME, "example.com/a:tag");
        let b = trust_policy_path(HOST_CTR_VERIFIER_NAME, "example.com/b:tag");
        assert_ne!(a, b);
    }
}
