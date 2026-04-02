use nix::mount::MsFlags;
use snafu::{Whatever, prelude::*};
use std::path::PathBuf;

use crate::fscrypt::*;
use crate::key;
use crate::{bpf, mount_point::MountPoint};

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

/// Collect all unique mount points (parents and children) for the given paths.
fn collect_mounts(paths: &[PathBuf]) -> Result<std::collections::HashMap<u32, MountPoint>> {
    let mut mounts = std::collections::HashMap::new();

    for path in paths {
        let parent = MountPoint::from_path(path)?;
        let children = parent.find_children()?;

        mounts.entry(parent.device_id).or_insert(parent);
        for child in children {
            mounts.entry(child.device_id).or_insert(child);
        }
    }

    Ok(mounts)
}

/// Write-protect a directory using BPF LSM hooks
pub fn protect(paths: Vec<PathBuf>) -> Result<()> {
    let map = bpf::load_bpf()?;
    let mounts = collect_mounts(&paths)?;

    for mount in mounts.values() {
        // Check for inconsistent state: in map but not read-only
        if mount.is_in_map(&map)? && !mount.is_readonly()? {
            whatever!(
                "device {} is in protected map but not read-only - inconsistent state",
                mount.device_id
            );
        }

        if !mount.is_in_map(&map)? {
            mount.remount(MsFlags::MS_RDONLY)?;
            mount.add_to_map(&map)?;
        }
    }

    Ok(())
}

/// Remove write protection from a directory
pub fn unprotect(paths: Vec<PathBuf>) -> Result<()> {
    let map = bpf::load_bpf()?;
    let mounts = collect_mounts(&paths)?;

    for mount in mounts.values() {
        mount.remove_from_map(&map)?;
    }

    for mount in mounts.values() {
        mount.remount(MsFlags::empty())?;
    }

    Ok(())
}

/// Check if a directory is write-protected (read-only and in BPF map)
pub fn is_protected(path: PathBuf) -> Result<bool> {
    let map = bpf::load_bpf()?;
    let mount = MountPoint::from_path(&path)?;

    Ok(mount.is_readonly()? && mount.is_in_map(&map)?)
}
