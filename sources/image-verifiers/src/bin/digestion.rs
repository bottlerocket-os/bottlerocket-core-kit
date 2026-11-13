/*!
*digestion-image-verifier* verifies container image digests against an allowlist.

Containerd invokes: `digestion-image-verifier -name <ref> -digest <sha256:...>`

If no trust policy is configured, all images are allowed.
*/

use image_verifiers::{args, logging, policy, reference};
use log::{debug, error};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process;

const DIGESTION_TRUST_POLICY: &str = "/etc/containerd/image-verifiers/digestion/trustpolicy.json";
// Per-image drop-in dir for host containers, keyed by the hex sha256 of the reference.
const HOST_CTR_DIGESTION_TRUST_POLICY_DIR: &str =
    "/etc/host-containerd/image-verifiers/digestion/trustpolicy.d";
// Binary name used when called through the orchestrator containerd.
const DIGESTION_VERIFIER_NAME: &str = "digestion-image-verifier";
// Binary name used when called through host-ctr.
const HOST_CTR_VERIFIER_NAME: &str = "host-ctr-digestion-image-verifier";
const SUPPORTED_VERSION: &str = "1.0";

/// Which containerd invoked the verifier, derived from the installed name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VerifierRole {
    /// The orchestrator containerd, which uses one fixed trust policy file.
    Orchestrator,
    /// host-containerd, which uses a per-image drop-in written by host-ctr.
    Host,
}

impl VerifierRole {
    /// Maps the invoked binary name to a role; None for an unrecognized name.
    fn from_invoked_name(name: &str) -> Option<Self> {
        match name {
            DIGESTION_VERIFIER_NAME => Some(VerifierRole::Orchestrator),
            HOST_CTR_VERIFIER_NAME => Some(VerifierRole::Host),
            _ => None,
        }
    }
}

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

/// Trust policy path for the role: a per-image drop-in for the host verifier,
/// the single fixed policy file for the orchestrator.
fn trust_policy_path(role: VerifierRole, image_ref: &str) -> PathBuf {
    match role {
        VerifierRole::Host => {
            let key = hex::encode(Sha256::digest(image_ref.as_bytes()));
            Path::new(HOST_CTR_DIGESTION_TRUST_POLICY_DIR).join(format!("{key}.json"))
        }
        VerifierRole::Orchestrator => PathBuf::from(DIGESTION_TRUST_POLICY),
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

/// Loads the role's trust policy for the given image reference.
fn load_trust_policy(role: VerifierRole, image_ref: &str) -> policy::Result<Option<TrustPolicy>> {
    policy::load(&trust_policy_path(role, image_ref))
}

fn main() {
    logging::init();
    let args: args::Args = args::parse_go_style_args();
    let image_ref = reference::construct(&args.name, &args.digest);

    debug!("verifying image: {}", image_ref);

    let name = invoked_name();
    let Some(role) = VerifierRole::from_invoked_name(&name) else {
        error!(
            "image verification failed: unrecognized verifier name {name:?}, expected {DIGESTION_VERIFIER_NAME} or {HOST_CTR_VERIFIER_NAME}"
        );
        process::exit(1);
    };

    let policy = match load_trust_policy(role, &args.name) {
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

    #[test_case(DIGESTION_VERIFIER_NAME, Some(VerifierRole::Orchestrator); "orchestrator name")]
    #[test_case(HOST_CTR_VERIFIER_NAME, Some(VerifierRole::Host); "host name")]
    #[test_case("digestion-image-verifier-typo", None; "unrecognized name")]
    #[test_case("", None; "empty name")]
    fn role_from_invoked_name(name: &str, expected: Option<VerifierRole>) {
        assert_eq!(VerifierRole::from_invoked_name(name), expected);
    }

    #[test]
    fn orchestrator_uses_fixed_single_file() {
        assert_eq!(
            trust_policy_path(VerifierRole::Orchestrator, "example.com/image:tag"),
            PathBuf::from(DIGESTION_TRUST_POLICY),
        );
    }

    #[test]
    fn host_ctr_uses_per_reference_dropin() {
        // The key must match host-ctr's dropinKey for the same reference.
        let key = "d3985c25ba6d24b6662bc9de2c6e5844cf376c4d3464a82d5df94de0b270d86a";
        assert_eq!(
            trust_policy_path(VerifierRole::Host, "example.com/image:tag"),
            Path::new(HOST_CTR_DIGESTION_TRUST_POLICY_DIR).join(format!("{key}.json")),
        );
    }

    #[test]
    fn host_ctr_dropin_name_fits_name_max_for_long_references() {
        let image_ref = "r".repeat(1024);
        let path = trust_policy_path(VerifierRole::Host, &image_ref);
        let name = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(name.len(), 64 + ".json".len());
    }

    #[test]
    fn host_ctr_distinct_references_distinct_files() {
        let a = trust_policy_path(VerifierRole::Host, "example.com/a:tag");
        let b = trust_policy_path(VerifierRole::Host, "example.com/b:tag");
        assert_ne!(a, b);
    }
}
