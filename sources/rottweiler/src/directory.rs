use snafu::{Whatever, prelude::*};
use std::path::PathBuf;

use crate::fscrypt::*;
use crate::key;

type Result<T> = std::result::Result<T, Whatever>;

/// Encrypt a directory with fscrypt using the specified key
pub fn encrypt(path: PathBuf, key_id: String) -> Result<()> {
    // Remove the directory if it exists since any content cannot be trusted
    if path.exists() {
        std::fs::remove_dir_all(&path).with_whatever_context(|_| {
            format!("failed to remove directory '{}'", path.display())
        })?;
    }

    std::fs::create_dir_all(&path)
        .with_whatever_context(|_| format!("failed to create directory '{}'", path.display()))?;

    let key_bytes = key::load(key_id)?;
    let private_key = FscryptPrivateKey::from_bytes(&key_bytes)
        .with_whatever_context(|_| "failed to parse key")?;
    let public_key: FscryptPublicKey = private_key.into();

    public_key
        .encrypt_directory(&path)
        .with_whatever_context(|_| format!("failed to encrypt directory '{}'", path.display()))?;

    Ok(())
}

/// Lock an encrypted directory by removing its key from the kernel keyring
pub fn lock(path: PathBuf) -> Result<()> {
    let key = FscryptPublicKey::from_directory(&path)
        .with_whatever_context(|_| format!("failed to read key id from '{}'", path.display()))?;

    key.lock_directory(&path)
        .with_whatever_context(|_| format!("failed to lock directory '{}'", path.display()))?;

    Ok(())
}

/// Unlock an encrypted directory by adding its key to the kernel keyring
pub fn unlock(path: PathBuf, key_id: String) -> Result<()> {
    let key_bytes = key::load(key_id)?;
    let key = FscryptPrivateKey::from_bytes(&key_bytes)
        .with_whatever_context(|_| "failed to parse key")?;

    key.unlock_directory(&path)
        .with_whatever_context(|_| format!("failed to unlock directory '{}'", path.display()))?;

    Ok(())
}

/// Check if a directory is encrypted with fscrypt
pub fn is_encrypted(path: PathBuf) -> Result<bool> {
    match FscryptPublicKey::from_directory(&path) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
