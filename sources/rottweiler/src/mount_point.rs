use snafu::prelude::*;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, snafu::Whatever>;

/// Represents a filesystem mount point with its path
pub struct MountPoint {
    pub path: PathBuf,
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

        Ok(Self { path: current })
    }

    /// Open the mount point path as a file descriptor
    pub fn open(&self) -> Result<std::fs::File> {
        std::fs::File::open(&self.path).with_whatever_context(|_| {
            format!("failed to open mount point '{}'", self.path.display())
        })
    }
}
