use libbpf_rs::MapCore;
use nix::mount::{MsFlags, mount};
use nix::sys::statvfs::{FsFlags, statvfs};
use snafu::prelude::*;
use std::collections::HashSet;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

type Result<T> = std::result::Result<T, snafu::Whatever>;

const NONE: Option<&'static [u8]> = None;
const PROTECTED: i32 = 1;

/// Convert device number from stat encoding to kernel encoding
///
/// stat() returns dev_t as u64 using old encoding: (major & 0xff) << 8 | (minor & 0xff) | ((minor & 0xfff00) << 12)
/// kernel uses u32: (major & 0xfff) << 20 | (minor & 0xfffff)
fn dev_to_kernel_encoding(stat_dev: u64) -> u32 {
    let major = ((stat_dev >> 8) & 0xff) | ((stat_dev >> 32) & 0xfffff00);
    let minor = (stat_dev & 0xff) | ((stat_dev >> 12) & 0xffffff00);
    ((major << 20) | minor) as u32
}

/// Represents a filesystem mount point with its path and device ID
pub struct MountPoint {
    pub path: PathBuf,
    pub device_id: u32,
}

impl MountPoint {
    /// Find the mount point for a given path by walking up the directory tree
    pub fn from_path(path: &Path) -> Result<Self> {
        let mut current = path.canonicalize().with_whatever_context(|_| {
            format!("failed to canonicalize path '{}'", path.display())
        })?;

        let current_stat = std::fs::metadata(&current).with_whatever_context(|_| {
            format!("failed to get metadata for '{}'", current.display())
        })?;
        let current_dev = current_stat.dev();

        loop {
            let parent = match current.parent() {
                Some(p) if p != current.as_path() => p,
                _ => break,
            };

            let parent_stat = std::fs::metadata(parent).with_whatever_context(|_| {
                format!("failed to get metadata for '{}'", parent.display())
            })?;

            if parent_stat.dev() != current_dev {
                break;
            }

            current = parent.to_path_buf();
        }

        Ok(Self {
            path: current.clone(),
            device_id: dev_to_kernel_encoding(current_dev),
        })
    }

    /// Open the mount point path as a file descriptor
    pub fn open(&self) -> Result<std::fs::File> {
        std::fs::File::open(&self.path).with_whatever_context(|_| {
            format!("failed to open mount point '{}'", self.path.display())
        })
    }

    /// Find all child mount points within this mount point's directory tree
    pub fn find_children(&self) -> Result<Vec<Self>> {
        let mut child_mounts = Vec::new();
        let mut seen_devices = HashSet::new();
        seen_devices.insert(self.device_id);

        for entry in WalkDir::new(&self.path).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_dir() {
                continue;
            }

            let entry_dev = dev_to_kernel_encoding(
                entry
                    .metadata()
                    .with_whatever_context(|_| {
                        format!("failed to get metadata for '{}'", entry.path().display())
                    })?
                    .dev(),
            );

            if !seen_devices.contains(&entry_dev) {
                seen_devices.insert(entry_dev);
                child_mounts.push(Self {
                    path: entry.path().to_path_buf(),
                    device_id: entry_dev,
                });
            }
        }

        Ok(child_mounts)
    }

    /// Remount the filesystem with the specified mount flags
    pub fn remount(&self, flags: MsFlags) -> Result<()> {
        mount(NONE, &self.path, NONE, MsFlags::MS_REMOUNT | flags, NONE)
            .with_whatever_context(|_| format!("failed to remount '{}'", self.path.display()))
    }

    /// Add this mount point's device ID to the BPF protected_mounts map
    pub fn add_to_map(&self, map: &libbpf_rs::MapHandle) -> Result<()> {
        map.update(
            &self.device_id.to_ne_bytes(),
            &PROTECTED.to_ne_bytes(),
            libbpf_rs::MapFlags::ANY,
        )
        .with_whatever_context(|_| {
            format!(
                "failed to add device {} to protected_mounts map",
                self.device_id
            )
        })
    }

    /// Remove this mount point's device ID from the BPF protected_mounts map
    pub fn remove_from_map(&self, map: &libbpf_rs::MapHandle) -> Result<()> {
        map.delete(&self.device_id.to_ne_bytes())
            .with_whatever_context(|_| {
                format!(
                    "failed to remove device {} from protected_mounts map",
                    self.device_id
                )
            })
    }

    /// Check if this mount point's device ID is in the BPF protected_mounts map
    pub fn is_in_map(&self, map: &libbpf_rs::MapHandle) -> Result<bool> {
        match map.lookup(&self.device_id.to_ne_bytes(), libbpf_rs::MapFlags::ANY) {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => whatever!(
                "failed to check if device {} is in map: {}",
                self.device_id,
                e
            ),
        }
    }

    /// Check if this mount point is mounted read-only
    pub fn is_readonly(&self) -> Result<bool> {
        let stat = statvfs(&self.path)
            .with_whatever_context(|_| format!("failed to statvfs '{}'", self.path.display()))?;

        Ok(stat.flags().contains(FsFlags::ST_RDONLY))
    }
}
