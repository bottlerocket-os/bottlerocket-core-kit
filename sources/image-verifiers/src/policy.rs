//! Shared policy loading logic for image verifiers.

use serde::de::DeserializeOwned;
use snafu::{ResultExt, Snafu};
use std::{fs, io, path::Path};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("failed to read trust policy: {}", source))]
    ReadPolicy { source: io::Error },

    #[snafu(display("invalid trust policy JSON: {}", source))]
    ParsePolicy { source: serde_json::Error },

    #[snafu(display("{}", message))]
    InvalidPolicy { message: String },
}

pub type Result<T> = std::result::Result<T, Error>;

/// Trait for trust policies that can be loaded and validated.
pub trait Policy: DeserializeOwned {
    /// Returns true if the policy is empty (verification should be skipped).
    fn is_empty(&self) -> bool;

    /// Validate the policy, returning an error message if invalid.
    fn validate(&self) -> std::result::Result<(), String> {
        Ok(())
    }
}

/// Load and validate a trust policy from a JSON file.
///
/// Returns:
/// - `Ok(Some(policy))` if a valid, non-empty policy exists
/// - `Ok(None)` if no policy is configured (file missing, empty, or empty array)
/// - `Err` if the policy is invalid (reject images, fail closed)
pub fn load<P: Policy>(path: &Path) -> Result<Option<P>> {
    // No policy file means verification is disabled.
    if !path.exists() {
        return Ok(None);
    }

    // Read errors (permissions, I/O) should reject images.
    let data = fs::read_to_string(path).context(ReadPolicySnafu)?;

    // Empty file means verification is disabled.
    if data.trim().is_empty() {
        return Ok(None);
    }

    // Invalid JSON should reject images (fail closed).
    let policy: P = serde_json::from_str(&data).context(ParsePolicySnafu)?;

    // Run policy-specific validation.
    policy
        .validate()
        .map_err(|message| Error::InvalidPolicy { message })?;

    // Empty policy means verification is disabled.
    if policy.is_empty() {
        return Ok(None);
    }

    Ok(Some(policy))
}
