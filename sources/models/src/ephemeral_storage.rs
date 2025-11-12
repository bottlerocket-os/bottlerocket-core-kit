//! The 'ephemeral_storage' module holds types used to communicate between client and server for
//! 'apiclient ephemeral-storage'.
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Supported filesystems for ephemeral storage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// Storage type preferences for ephemeral storage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Preference {
    pub ephemeral_disk: bool,
    pub ebs_volume: bool,
}
impl Display for Preference {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut first = false;
        let mut write = |s| {
            if first {
                first = false;
            } else {
                f.write_str("+")?;
            }
            f.write_str(s)
        };
        if self.ephemeral_disk {
            write("ephemeral-disk")?;
        }
        if self.ebs_volume {
            write("ebs-volume")?;
        }
        Ok(())
    }
}
impl<'a> TryFrom<&'a str> for Preference {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let mut preference = Self {
            ephemeral_disk: false,
            ebs_volume: false,
        };
        for i in value.split("+") {
            match i {
                "ephemeral-disk" => preference.ephemeral_disk = true,
                "ebs-volume" => preference.ebs_volume = true,
                "" => {}
                x => return Err(x),
            }
        }
        Ok(preference)
    }
}

/// Initialize ephemeral storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Init {
    pub filesystem: Option<Filesystem>,
    pub disks: Option<Vec<String>>,
    pub ebs_volumes: Option<Vec<String>>,
    pub prefer: Option<Vec<Preference>>,
}

/// Bind directories to configured ephemeral storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bind {
    pub targets: Vec<String>,
}
