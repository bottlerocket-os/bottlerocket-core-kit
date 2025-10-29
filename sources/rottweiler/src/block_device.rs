use snafu::prelude::*;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::{key, system};

type Result<T> = std::result::Result<T, snafu::Whatever>;

/// Encrypt a block device with LUKS2 using the specified key
pub fn encrypt(path: PathBuf, key_id: String) -> Result<()> {
    let device = path
        .to_str()
        .with_whatever_context(|| format!("path is not valid UTF-8: '{}'", path.display()))?;

    let key_bytes = key::load(key_id)?;

    system::cryptsetup_luks_format(device, &key_bytes)
}

/// Attach (unlock) an encrypted block device, creating a device mapper entry
pub fn attach(path: PathBuf, key_id: String) -> Result<()> {
    let volume_name = filename(&path)?;

    let source_device = path
        .to_str()
        .with_whatever_context(|| format!("path is not valid UTF-8: '{}'", path.display()))?;

    let key_bytes = key::load(key_id)?;

    system::systemd_cryptsetup_attach(volume_name, source_device, &key_bytes)
}

/// Detach (lock) an encrypted block device, removing the device mapper entry
pub fn detach(path: PathBuf) -> Result<()> {
    let volume_name = filename(&path)?;

    system::systemd_cryptsetup_detach(volume_name)
}

/// Resize a LUKS2 encrypted block device to match the underlying device size
pub fn resize(path: PathBuf, key_id: String) -> Result<()> {
    let volume_name = filename(&path)?;

    let key_bytes = key::load(key_id)?;

    system::cryptsetup_resize(volume_name, &key_bytes)
}

const LUKS2_MAGIC: &[u8; 6] = b"LUKS\xba\xbe";
const LUKS2_VERSION: u16 = 2;

/// Check if a block device is LUKS2 encrypted by reading its header
pub fn is_encrypted(path: PathBuf) -> Result<bool> {
    let mut file = std::fs::File::open(&path)
        .with_whatever_context(|_| format!("failed to open '{}'", path.display()))?;

    let mut header = [0u8; 8];
    file.read_exact(&mut header)
        .with_whatever_context(|_| format!("failed to read header from '{}'", path.display()))?;

    if &header[..6] == LUKS2_MAGIC {
        let version = u16::from_be_bytes([header[6], header[7]]);
        if version == LUKS2_VERSION {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Extract filename from path as UTF-8 string
fn filename(path: &Path) -> Result<&str> {
    path.file_name()
        .with_whatever_context(|| format!("failed to extract filename from '{}'", path.display()))?
        .to_str()
        .with_whatever_context(|| format!("filename is not valid UTF-8: '{}'", path.display()))
}
