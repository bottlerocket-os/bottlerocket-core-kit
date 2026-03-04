/*!
*notation-image-verifier* verifies container image signatures using the notation CLI.

Containerd invokes: `notation-image-verifier -name <ref> -digest <sha256:...>`

If no trust policy is configured, all images are allowed.
*/

use image_verifiers::{args, logging, policy, reference};
use log::{debug, error};
use serde::Deserialize;
use std::env;
use std::path::Path;
use std::process::{self, Command};

const NOTATION_TRUST_POLICY: &str = "/etc/containerd/image-verifiers/notation/trustpolicy.json";
const SUPPORTED_VERSION: &str = "1.0";

/// Notation trust policy deserialized from trustpolicy.json.
#[derive(Deserialize)]
struct TrustPolicy {
    version: String,
    #[serde(rename = "trustPolicies")]
    trust_policies: Vec<serde_json::Value>,
}

impl policy::Policy for TrustPolicy {
    fn is_empty(&self) -> bool {
        self.trust_policies.is_empty()
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
    policy::load(Path::new(NOTATION_TRUST_POLICY))
}

fn main() {
    logging::init();
    let args: args::Args = args::parse_go_style_args();
    let image_ref = reference::construct(&args.name, &args.digest);

    debug!("verifying image: {}", image_ref);

    match load_trust_policy() {
        Ok(Some(_)) => {}
        Ok(None) => {
            debug!("image verification skipped: no trust policy configured");
            return;
        }
        Err(e) => {
            error!("image verification failed: {}", e);
            process::exit(1);
        }
    }

    let mut cmd = Command::new("/usr/bin/notation");
    cmd.args(["verify", &image_ref])
        .env(
            "NOTATION_CONFIG",
            "/etc/containerd/image-verifiers/notation",
        )
        .env("NOTATION_CACHE", "/var/cache/notation")
        .env("NOTATION_LIBEXEC", "/usr/libexec/notation-plugins")
        .env("HOME", "/root");

    // We upgrade to fips140=only for notation to enforce strict FIPS-only
    // cryptography: all hash functions, TLS ciphers, and signature algorithms
    // used during image verification must be FIPS-approved with no fallback.
    if let Ok(godebug) = env::var("GODEBUG") {
        if godebug == "fips140=on" {
            cmd.env("GODEBUG", "fips140=only");
        }
    }

    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            error!("image verification failed: {}", e);
            process::exit(1);
        }
    };

    if !output.status.success() {
        let msg = String::from_utf8_lossy(&output.stdout);
        let err = String::from_utf8_lossy(&output.stderr);
        error!("image verification failed: {}{}", msg, err);
        process::exit(1);
    }

    debug!("image verification successful");
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test]
    fn parse_trust_policy_with_policies() {
        let json = r#"{"version": "1.0", "trustPolicies": [{"name": "test"}]}"#;
        let policy: TrustPolicy = serde_json::from_str(json).unwrap();
        assert_eq!(policy.trust_policies.len(), 1);
    }

    #[test_case(r#"{"version": "1.0", "trustPolicies": []}"#, true; "empty array parses")]
    #[test_case(r#"{"version": "1.0", "trustPolicies": [{}]}"#, false; "non-empty array")]
    fn test_trust_policies_empty(json: &str, expected_empty: bool) {
        let policy: TrustPolicy = serde_json::from_str(json).unwrap();
        assert_eq!(policy.trust_policies.is_empty(), expected_empty);
    }
}
