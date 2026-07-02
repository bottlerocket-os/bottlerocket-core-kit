//! Create device nodes using mknod based on /proc/devices lookup.

use crate::error::{self, Result};
use crate::CreateDeviceArgs;
use snafu::{ensure, ResultExt};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::{fs, str};

const MKNOD_PATH: &str = "/usr/bin/mknod";
const PROC_DEVICES_PATH: &str = "/proc/devices";

/// Type of device (character or block)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeviceType {
    Char,
    Block,
}

impl DeviceType {
    /// Returns the mknod type argument ("c" for char, "b" for block)
    fn as_mknod_arg(&self) -> &'static str {
        match self {
            DeviceType::Char => "c",
            DeviceType::Block => "b",
        }
    }
}

/// Parsed contents of /proc/devices
struct ProcDevices {
    char_devices: HashMap<String, u32>,
    block_devices: HashMap<String, u32>,
}

/// Section being parsed in /proc/devices
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseSection {
    None,
    Char,
    Block,
}

/// Parse /proc/devices content into character and block device maps
fn parse_proc_devices(content: &str) -> Result<ProcDevices> {
    let mut char_devices = HashMap::new();
    let mut block_devices = HashMap::new();
    let mut section = ParseSection::None;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed == "Character devices:" {
            section = ParseSection::Char;
            continue;
        }
        if trimmed == "Block devices:" {
            section = ParseSection::Block;
            continue;
        }

        if section == ParseSection::None {
            continue;
        }

        // Parse device line: "<major> <name>"
        let mut parts = trimmed.split_whitespace();
        let major_str = parts.next();
        let name = parts.next();

        match (major_str, name) {
            (Some(major_str), Some(name)) => {
                let major: u32 = major_str
                    .parse()
                    .map_err(|_| error::Error::ParseProcDevices {
                        line: line.to_string(),
                    })?;

                match section {
                    ParseSection::Char => {
                        char_devices.insert(name.to_string(), major);
                    }
                    ParseSection::Block => {
                        block_devices.insert(name.to_string(), major);
                    }
                    ParseSection::None => {}
                }
            }
            _ => {
                return error::ParseProcDevicesSnafu {
                    line: line.to_string(),
                }
                .fail();
            }
        }
    }

    Ok(ProcDevices {
        char_devices,
        block_devices,
    })
}

/// Look up a device name in the parsed /proc/devices and return (major, type)
fn lookup_device(devices: &ProcDevices, name: &str) -> Result<(u32, DeviceType)> {
    if let Some(&major) = devices.char_devices.get(name) {
        return Ok((major, DeviceType::Char));
    }
    if let Some(&major) = devices.block_devices.get(name) {
        return Ok((major, DeviceType::Block));
    }

    error::DeviceNotFoundSnafu {
        name: name.to_string(),
    }
    .fail()
}

/// Create a device node using mknod
pub(crate) fn create_device(args: &CreateDeviceArgs) -> Result<()> {
    let content = fs::read_to_string(PROC_DEVICES_PATH).context(error::ReadProcDevicesSnafu)?;

    let devices = parse_proc_devices(&content)?;

    let (major, device_type) = lookup_device(&devices, &args.device_name)?;

    let path_str = args
        .path
        .clone()
        .unwrap_or_else(|| format!("/dev/{}{}", args.device_name, args.minor));
    let path = Path::new(&path_str);

    ensure!(
        !path.exists(),
        error::DevicePathExistsSnafu {
            path: path.to_path_buf()
        }
    );

    let output = Command::new(MKNOD_PATH)
        .arg(format!("--mode={}", args.mode))
        .arg(&path_str)
        .arg(device_type.as_mknod_arg())
        .arg(major.to_string())
        .arg(args.minor.to_string())
        .output()
        .context(error::MknodExecutionSnafu {
            path: path.to_path_buf(),
        })?;

    ensure!(
        output.status.success(),
        error::MknodFailedSnafu {
            path: path.to_path_buf(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    );

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    const SAMPLE_PROC_DEVICES: &str = r#"Character devices:
  1 mem
  4 tty
 10 misc
195 nvidia

Block devices:
  8 sd
259 blkext
"#;

    #[test]
    fn test_parse_proc_devices() {
        let devices = parse_proc_devices(SAMPLE_PROC_DEVICES).unwrap();

        assert_eq!(devices.char_devices.get("mem"), Some(&1));
        assert_eq!(devices.char_devices.get("tty"), Some(&4));
        assert_eq!(devices.char_devices.get("misc"), Some(&10));
        assert_eq!(devices.char_devices.get("nvidia"), Some(&195));

        assert_eq!(devices.block_devices.get("sd"), Some(&8));
        assert_eq!(devices.block_devices.get("blkext"), Some(&259));
    }

    #[test]
    fn test_lookup_device_char() {
        let devices = parse_proc_devices(SAMPLE_PROC_DEVICES).unwrap();
        let (major, dtype) = lookup_device(&devices, "nvidia").unwrap();
        assert_eq!(major, 195);
        assert_eq!(dtype, DeviceType::Char);
    }

    #[test]
    fn test_lookup_device_block() {
        let devices = parse_proc_devices(SAMPLE_PROC_DEVICES).unwrap();
        let (major, dtype) = lookup_device(&devices, "sd").unwrap();
        assert_eq!(major, 8);
        assert_eq!(dtype, DeviceType::Block);
    }

    #[test]
    fn test_lookup_device_not_found() {
        let devices = parse_proc_devices(SAMPLE_PROC_DEVICES).unwrap();
        let result = lookup_device(&devices, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_device_type_mknod_arg() {
        assert_eq!(DeviceType::Char.as_mknod_arg(), "c");
        assert_eq!(DeviceType::Block.as_mknod_arg(), "b");
    }
}
