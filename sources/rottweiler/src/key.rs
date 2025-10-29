use snafu::prelude::*;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use zeroize::Zeroizing;

use crate::system;

type Result<T> = std::result::Result<T, snafu::Whatever>;

const DEV_RANDOM: &str = "/dev/random";
const KEYSTORE: &str = "/.bottlerocket/keystore";
const KEY_SIZE: usize = 64;

/// Generate a random encryption key and encrypt it with TPM2 PCRs 7+14
pub fn generate(key_id: String) -> Result<()> {
    if !key_id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        whatever!("key_id must contain only alphanumerics, dashes, and underscores");
    }

    let key_path = PathBuf::from(KEYSTORE).join(&key_id);

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

    fs::create_dir_all(KEYSTORE)
        .with_whatever_context(|_| format!("failed to create keystore directory '{}'", KEYSTORE))?;

    fs::write(&key_path, encrypted)
        .with_whatever_context(|_| format!("failed to write key to '{}'", key_path.display()))?;

    Ok(())
}

/// Load and decrypt a TPM2-encrypted key from the keystore
pub fn load(key_id: String) -> Result<Zeroizing<Vec<u8>>> {
    let key_path = PathBuf::from(KEYSTORE).join(&key_id);

    let encrypted = fs::read(&key_path)
        .with_whatever_context(|_| format!("failed to read key from '{}'", key_path.display()))?;

    system::systemd_creds_decrypt(&key_id, &encrypted)
}
