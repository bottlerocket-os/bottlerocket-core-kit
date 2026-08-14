use snafu::prelude::*;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use zeroize::Zeroizing;

use crate::system;

type Result<T> = std::result::Result<T, snafu::Whatever>;

const DEV_RANDOM: &str = "/dev/random";
const KEYSTORE_PERMANENT: &str = "/.bottlerocket/keystore";
const KEYSTORE_EPHEMERAL: &str = "/run/rottweiler";
const KEY_SIZE: usize = 64;

/// Checks if ephemeral encryption keys feature is enabled via image features
fn ephemeral_encryption_keys_enabled() -> Result<bool> {
    let features = bottlerocket_image_features::parse_image_features()
        .with_whatever_context(|_| "failed to load image features")?;
    Ok(features.ephemeral_encryption_keys)
}

fn keystore_dir() -> PathBuf {
    let is_ephemeral_encryption_keys_enabled = ephemeral_encryption_keys_enabled().unwrap_or(false);
    // If ephemeral encryption keys are enabled, we store the keys in tmpfs rather than the disk
    if is_ephemeral_encryption_keys_enabled {
        PathBuf::from(KEYSTORE_EPHEMERAL)
    } else {
        PathBuf::from(KEYSTORE_PERMANENT)
    }
}

fn validate_key_id(key_id: &str) -> Result<()> {
    snafu::ensure_whatever!(
        !key_id.is_empty()
            && key_id
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_'),
        "key_id must be non-empty and contain only alphanumerics, dashes, and underscores"
    );

    Ok(())
}

/// Generate a random encryption key and encrypt it with TPM2 PCRs 7+14
pub fn generate(key_id: String) -> Result<()> {
    validate_key_id(&key_id)?;
    let keystore = keystore_dir();
    let key_path = keystore.join(&key_id);

    // Skip generation if key already exists
    if key_path.exists() {
        return Ok(());
    }

    let mut random_bytes = Zeroizing::new(vec![0u8; KEY_SIZE]);

    let mut random = fs::File::open(DEV_RANDOM)
        .with_whatever_context(|_| format!("failed to open {}", DEV_RANDOM))?;

    random
        .read_exact(&mut random_bytes)
        .with_whatever_context(|_| "failed to read random bytes")?;

    let encrypted = system::systemd_creds_encrypt(&key_id, &random_bytes)?;

    fs::create_dir_all(&keystore).with_whatever_context(|_| {
        format!(
            "failed to create keystore directory '{}'",
            keystore.display()
        )
    })?;

    fs::write(&key_path, encrypted)
        .with_whatever_context(|_| format!("failed to write key to '{}'", key_path.display()))?;

    Ok(())
}

/// Delete a sealed key from the keystore.
pub fn delete(key_id: String) -> Result<()> {
    validate_key_id(&key_id)?;
    let key_path = keystore_dir().join(&key_id);

    // Skip deletion if key doesn't exists
    if !key_path.exists() {
        return Ok(());
    }

    match fs::remove_file(&key_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e)
            .with_whatever_context(|_| format!("failed to delete key '{}'", key_path.display())),
    }
}

/// Load and decrypt a TPM2-encrypted key from the keystore
pub fn load(key_id: String) -> Result<Zeroizing<Vec<u8>>> {
    validate_key_id(&key_id)?;
    let key_path = keystore_dir().join(&key_id);

    let encrypted = fs::read(&key_path)
        .with_whatever_context(|_| format!("failed to read key from '{}'", key_path.display()))?;

    system::systemd_creds_decrypt(&key_id, &encrypted)
}
