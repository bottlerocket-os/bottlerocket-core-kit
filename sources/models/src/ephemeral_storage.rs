//! The 'ephemeral_storage' module holds types used to communicate between client and server for
//! 'apiclient ephemeral-storage'.
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Supported filesystems for ephemeral storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Filesystem {
    Xfs,
    Ext4,
}
impl Display for Filesystem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Filesystem::Xfs => f.write_str("xfs"),
            Filesystem::Ext4 => f.write_str("ext4"),
        }
    }
}

/// Initialize ephemeral storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Init {
    pub filesystem: Option<Filesystem>,
    pub disks: Option<Vec<String>>,
}

/// Bind directories to configured ephemeral storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bind {
    pub targets: Vec<String>,
}
