/*!
*digestion-image-verifier* verifies container image digests against an allowlist.

Containerd invokes: `digestion-image-verifier -name <ref> -digest <sha256:...>`

If no trust policy is configured, all images are allowed.
*/

use image_verifiers::{args, logging, policy, reference};
use log::{debug, error};
use serde::Deserialize;
use std::path::Path;
use std::process;

const DIGESTION_TRUST_POLICY: &str = "/etc/containerd/image-verifiers/digestion/trustpolicy.json";
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

/// Loads the trust policy from the configured path.
fn load_trust_policy() -> policy::Result<Option<TrustPolicy>> {
    policy::load(Path::new(DIGESTION_TRUST_POLICY))
}

fn main() {
    logging::init();
    let args: args::Args = args::parse_go_style_args();
    let image_ref = reference::construct(&args.name, &args.digest);

    debug!("verifying image: {}", image_ref);

    let policy = match load_trust_policy() {
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
}
