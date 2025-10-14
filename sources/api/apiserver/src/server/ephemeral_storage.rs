//! The 'ephemeral_storage' module supports configuring and using local instance storage.

use model::ephemeral_storage::{Filesystem, Preference};

use snafu::{ensure, ResultExt};
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::Path;
use std::process::Command;

static MOUNT: &str = "/usr/bin/mount";
static MDADM: &str = "/usr/sbin/mdadm";
static BLKID: &str = "/usr/sbin/blkid";
static MKFSXFS: &str = "/usr/sbin/mkfs.xfs";
static MKFSEXT4: &str = "/usr/sbin/mkfs.ext4";
static FINDMNT: &str = "/usr/bin/findmnt";

/// Name of the array (if created) and filesystem label. Selected to be 12 characters so it
/// fits within both the xfs and ext4 volume label limit.
static EPHEMERAL_MNT: &str = ".ephemeral";
/// Name of the device and its path from the MD driver
static RAID_DEVICE_DIR: &str = "/dev/md/";
static RAID_DEVICE_NAME: &str = "ephemeral";
/// Symlink to ephemeral storage array or disk
static EPHEMERAL_STORAGE_LINK: &str = "/dev/disk/ephemeral-storage";

pub struct BindDirs {
    pub allowed_exact: HashSet<&'static str>,
    pub allowed_prefixes: HashSet<&'static str>,
    pub disallowed_contains: HashSet<&'static str>,
}

/// initialize prepares the ephemeral storage for formatting and formats it.  For multiple disks
/// preparation is the creation of a RAID0 array, for a single disk this is a no-op. The array or disk
/// is then formatted with the specified filesystem (default=xfs) if not formatted already.
pub fn initialize(
    fs: Option<Filesystem>,
    disks: Option<Vec<String>>,
    ebs_volumes: Option<Vec<String>>,
    prefer: Option<Vec<Preference>>,
) -> Result<()> {
    let known_disks = ephemeral_devices()?;
    let known_disks_hash = HashSet::<_>::from_iter(known_disks.iter());
    let known_ebs_volumes = ephemeral_ebs_volumes()?;
    let known_ebs_volumes_hash = HashSet::<_>::from_iter(known_ebs_volumes.iter());

    let any_specified = disks.as_ref().is_some_and(|x| !x.is_empty())
        || ebs_volumes.as_ref().is_some_and(|x| !x.is_empty());

    let disks = if any_specified {
        // use all specified ephemeral disks and ebs volumes, if they're all valid
        let mut selected_disks = vec![];
        if let Some(d) = disks {
            for disk in &d {
                ensure!(
                    known_disks_hash.contains(disk),
                    error::InvalidParameterSnafu {
                        parameter: "disks",
                        reason: format!("unknown disk {disk:?}"),
                    }
                )
            }
            selected_disks.extend(d);
        }

        if let Some(e) = ebs_volumes {
            for ebs_volume in &e {
                ensure!(
                    known_ebs_volumes_hash.contains(ebs_volume),
                    error::InvalidParameterSnafu {
                        parameter: "ebs_volumes",
                        reason: format!("unknown ebs volume {ebs_volume:?}"),
                    }
                )
            }
            selected_disks.extend(e);
        }
        selected_disks
    } else {
        // if there are no specified disks, use preference list to find a non-empty set of disks
        let preferences = prefer.unwrap_or_else(|| {
            vec![Preference {
                ephemeral_disk: true,
                ebs_volume: false,
            }]
        });

        let mut disks = vec![];
        for preference in preferences {
            if preference.ephemeral_disk {
                disks.extend(&known_disks);
            }
            if preference.ebs_volume {
                disks.extend(&known_ebs_volumes);
            }
            if !disks.is_empty() {
                break;
            }
        }
        if disks.is_empty() {
            // no disks were specified and none of the preferences produced any disks
            // this is special-cased as a no-op
            info!("no ephemeral disks found, skipping ephemeral storage initialization");
            return Ok(());
        }
        disks.into_iter().cloned().collect()
    };

    ensure!(
        !disks.is_empty(),
        error::InvalidParameterSnafu {
            parameter: "disks",
            reason: "no valid local ephemeral disks or ebs volumes specified",
        }
    );

    info!("initializing ephemeral storage disks={disks:?}");
    // with a single disk, there is no need to create the array
    let device_name = match disks.len() {
        1 => disks.first().expect("non-empty").clone(),
        _ => {
            let scan_output = mdadm_scan()?;
            // no previously configured array found, so construct a new one
            if scan_output.is_empty() {
                info!("creating array named {RAID_DEVICE_NAME:?} from {disks:?}");
                mdadm_create(RAID_DEVICE_NAME, disks.iter().map(|x| x.as_str()).collect())?;
            }
            // Once it is built, it will be available in `/dev/md/`
            format!("{RAID_DEVICE_DIR}{RAID_DEVICE_NAME}")
        }
    };

    let fs = fs.unwrap_or(Filesystem::Xfs);
    if !is_formatted(&device_name, &fs)? {
        info!("formatting {device_name:?} as {fs}");
        format_device(&device_name, &fs)?;
    } else {
        info!("{device_name:?} is already formatted as {fs}, skipping format");
    }

    // Create link to formatted device for use in `bind`
    std::os::unix::fs::symlink(&device_name, EPHEMERAL_STORAGE_LINK)
        .context(error::DiskSymlinkFailureSnafu {})?;

    Ok(())
}

/// binds the specified directories to the pre-configured array, creating those directories if
/// they do not exist.
pub fn bind(variant: &str, dirs: Vec<String>) -> Result<()> {
    let device_name = EPHEMERAL_STORAGE_LINK;
    if !std::fs::exists(device_name).is_ok_and(|x| x) {
        info!("ephemeral storage not initialized, skipping binding");
        return Ok(());
    }

    let dirs = if dirs.is_empty() {
        let allowed_dirs = allowed_bind_dirs(variant);
        allowed_dirs
            .allowed_exact
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    } else {
        dirs
    };

    // Normalize input by trimming trailing "/"
    let dirs: Vec<String> = dirs
        .into_iter()
        .map(|dir| dir.trim_end_matches("/").to_string())
        .collect();

    let allowed_dirs = allowed_bind_dirs(variant);
    for dir in &dirs {
        let exact_match = allowed_dirs.allowed_exact.contains(dir.as_str());
        let prefix_match = allowed_dirs
            .allowed_prefixes
            .iter()
            .any(|prefix| dir.starts_with(prefix));
        let disallowed_match = allowed_dirs
            .disallowed_contains
            .iter()
            .any(|contains| dir.contains(contains));
        ensure!(
            exact_match || (prefix_match && !disallowed_match),
            error::InvalidParameterSnafu {
                parameter: dir,
                reason: "specified bind directory not in allow list",
            }
        )
    }

    let mount_point = format!("/mnt/{EPHEMERAL_MNT}");
    if !is_mounted(&mount_point)? {
        std::fs::create_dir_all(&mount_point).context(error::MkdirSnafu {})?;
        info!("mounting {device_name} as {mount_point}");
        let output = Command::new(MOUNT)
            .args([OsString::from(device_name), OsString::from(&mount_point)])
            .output()
            .context(error::ExecutionFailureSnafu { command: MOUNT })?;

        ensure!(
            output.status.success(),
            error::MountArrayFailureSnafu {
                what: device_name,
                dest: &mount_point,
                output
            }
        );
    } else {
        info!("device already mounted at {mount_point}, skipping mount");
    }

    let mount_point = Path::new(&mount_point);
    for dir in &dirs {
        // construct a directory name (E.g. /var/lib/kubelet => ._var_lib_kubelet) that will be
        // unique between the binding targets
        let mut directory_name = dir.replace('/', "_");
        directory_name.insert(0, '.');
        let mount_destination = mount_point.join(&directory_name);

        // we may run before the directories we are binding exist, so create them
        std::fs::create_dir_all(dir).context(error::MkdirSnafu {})?;
        std::fs::create_dir_all(&mount_destination).context(error::MkdirSnafu {})?;

        if is_mounted(dir)? {
            info!("skipping bind mount of {dir:?}, already mounted");
            continue;
        }
        // call the equivalent of
        // mount --rbind /mnt/.ephemeral/._var_lib_kubelet /var/lib/kubelet
        let source_dir = OsString::from(&dir);
        info!("binding {source_dir:?} to {mount_destination:?}");

        let output = Command::new(MOUNT)
            .args([
                OsStr::new("--rbind"),
                mount_destination.as_ref(),
                &source_dir,
            ])
            .output()
            .context(error::ExecutionFailureSnafu { command: MOUNT })?;

        ensure!(
            output.status.success(),
            error::BindDirectoryFailureSnafu {
                dir: String::from_utf8_lossy(source_dir.as_encoded_bytes()),
                output,
            }
        );
    }

    for dir in dirs {
        let source_dir = OsString::from(&dir);
        info!("sharing mounts for {source_dir:?}");
        // mount --make-rshared /var/lib/kubelet
        let output = Command::new(MOUNT)
            .args([OsStr::new("--make-rshared"), &source_dir])
            .output()
            .context(error::ExecutionFailureSnafu { command: MOUNT })?;

        ensure!(
            output.status.success(),
            error::ShareMountsFailureSnafu {
                dir: String::from_utf8_lossy(source_dir.as_encoded_bytes()),
                output
            }
        );
    }

    Ok(())
}

/// is_bound returns true if the specified path is already listed as a mount
fn is_mounted(path: &String) -> Result<bool> {
    let status = Command::new(FINDMNT)
        .arg(OsString::from(path))
        .status()
        .context(error::FindMntFailureSnafu {})?;
    Ok(status.success())
}

/// creates the array with the given name from the specified disks
fn mdadm_create<T: AsRef<str>>(name: T, disks: Vec<T>) -> Result<()> {
    let mut device_name = OsString::from(RAID_DEVICE_DIR);
    device_name.push(name.as_ref());

    let mut cmd = Command::new(MDADM);
    cmd.arg("--create");
    cmd.arg("--force");
    cmd.arg("--verbose");
    cmd.arg("--homehost=any");
    cmd.arg(device_name);
    cmd.arg("--level=0");
    // By default, mdadm uses a 512KB chunk size. mkfs.xfs attempts to match some of its settings to
    // the array size for maximum throughput, but the max log stripe size for xfs is 256KB.  We limit
    // the chunk size to 256KB here so that XFS can set the same value and avoid the fallback to
    // a 32 KB log stripe size.
    cmd.arg("--chunk=256");
    cmd.arg("--name");
    cmd.arg(OsString::from(name.as_ref()));
    cmd.arg("--raid-devices");
    cmd.arg(OsString::from(disks.len().to_string()));
    for disk in disks {
        cmd.arg(OsString::from(disk.as_ref()));
    }
    let output = cmd
        .output()
        .context(error::ExecutionFailureSnafu { command: MDADM })?;
    ensure!(
        output.status.success(),
        error::CreateArrayFailureSnafu { output }
    );
    Ok(())
}

/// ephemeral_devices returns the full path name to the block devices in /dev/disk/ephemeral
pub fn ephemeral_devices() -> Result<Vec<String>> {
    const EPHEMERAL_PATH: &str = "/dev/disk/ephemeral";
    let mut filenames = Vec::new();
    // for instances without ephemeral storage, we don't error and just return an empty vector so
    // it can be handled gracefully
    if fs::metadata(EPHEMERAL_PATH).is_err() {
        return Ok(filenames);
    }

    let entries = std::fs::read_dir(EPHEMERAL_PATH).context(error::DiscoverEphemeralSnafu {
        path: String::from(EPHEMERAL_PATH),
    })?;
    for entry in entries {
        let entry = entry.context(error::DiscoverEphemeralSnafu {
            path: String::from(EPHEMERAL_PATH),
        })?;
        filenames.push(entry.path().into_os_string().to_string_lossy().to_string());
    }
    Ok(filenames)
}

/// ephemeral_ebs_volumes returns the full path name to the ebs volumes in /dev/disk/ephemeral-ebs
pub fn ephemeral_ebs_volumes() -> Result<Vec<String>> {
    const EPHEMERAL_EBS_PATH: &str = "/dev/disk/ephemeral-ebs";
    let mut filenames = Vec::new();
    // for instances without ebs volumes attached, we don't error and just return an empty vector so
    // it can be handled gracefully
    if fs::metadata(EPHEMERAL_EBS_PATH).is_err() {
        return Ok(filenames);
    }

    let entries =
        std::fs::read_dir(EPHEMERAL_EBS_PATH).context(error::DiscoverEbsVolumesSnafu {
            path: String::from(EPHEMERAL_EBS_PATH),
        })?;
    for entry in entries {
        let entry = entry.context(error::DiscoverEbsVolumesSnafu {
            path: String::from(EPHEMERAL_EBS_PATH),
        })?;
        filenames.push(entry.path().into_os_string().to_string_lossy().to_string());
    }
    Ok(filenames)
}

/// allowed_bind_dirs returns a set of the directories that can be bound to ephemeral storage, which
/// varies based on the variant, a set of the prefixes of directories that are allowed to be bound.
/// and a set of substrings that are disallowed in the directory name.
pub fn allowed_bind_dirs(variant: &str) -> BindDirs {
    let mut allowed_exact = HashSet::from(["/var/lib/containerd", "/var/lib/host-containerd"]);
    if variant.contains("k8s") {
        allowed_exact.insert("/var/lib/kubelet");
        allowed_exact.insert("/var/log/pods");
        allowed_exact.insert("/var/lib/soci-snapshotter");
    }
    if variant.contains("ecs") {
        allowed_exact.insert("/var/lib/docker");
        allowed_exact.insert("/var/log/ecs");
    }
    let allowed_prefixes = HashSet::from(["/mnt/"]);
    let disallowed_contains = HashSet::from(["..", "/mnt/.ephemeral"]);

    BindDirs {
        allowed_exact,
        allowed_prefixes,
        disallowed_contains,
    }
}

/// scans the raid array to identify if it has been created already
fn mdadm_scan() -> Result<Vec<u8>> {
    let output = Command::new(MDADM)
        .args([OsStr::new("--detail"), OsStr::new("--scan")])
        .output()
        .context(error::ExecutionFailureSnafu { command: MDADM })?;
    ensure!(
        output.status.success(),
        error::ScanArrayFailureSnafu { output }
    );
    Ok(output.stdout)
}

/// is_formatted returns true if the array is already formatted with the specified filesystem
pub fn is_formatted<S: AsRef<OsStr>>(device: S, format: &Filesystem) -> Result<bool> {
    let mut fmt_arg = OsString::from("TYPE=");
    fmt_arg.push(OsString::from(format.to_string()));

    let blkid = Command::new(BLKID)
        .args([
            OsStr::new("--match-token"),
            fmt_arg.as_ref(),
            device.as_ref(),
        ])
        .status()
        .context(error::DetermineFormatFailureSnafu {})?;

    Ok(blkid.success())
}

/// formats the specified device with the given filesystem format
pub fn format_device<S: AsRef<OsStr>>(device: S, format: &Filesystem) -> Result<()> {
    let binary = match format {
        Filesystem::Xfs => MKFSXFS,
        Filesystem::Ext4 => MKFSEXT4,
    };

    let mut mkfs = Command::new(binary);
    mkfs.arg(device.as_ref());
    // labeled, XFS has a max of 12 characters, EXT4 allows 16
    mkfs.arg("-L");
    mkfs.arg(RAID_DEVICE_NAME);

    let output = mkfs
        .output()
        .context(error::ExecutionFailureSnafu { command: binary })?;

    ensure!(
        output.status.success(),
        error::FormatFilesystemFailureSnafu { output }
    );
    Ok(())
}

pub mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub enum Error {
        #[snafu(display("Failed to execute '{:?}': {}", command, source))]
        ExecutionFailure {
            command: &'static str,
            source: std::io::Error,
        },

        #[snafu(display("Failed to discover ephemeral disks from {}: {}", path, source))]
        DiscoverEphemeral {
            source: std::io::Error,
            path: String,
        },

        #[snafu(display("Failed to discover ebs volumes from {}: {}", path, source))]
        DiscoverEbsVolumes {
            source: std::io::Error,
            path: String,
        },

        #[snafu(display("Failed to mount {} to {}: {}", what, dest, String::from_utf8_lossy(output.stderr.as_slice())))]
        MountArrayFailure {
            what: String,
            dest: String,
            output: std::process::Output,
        },

        #[snafu(display("Failed to create disk symlink {}", source))]
        DiskSymlinkFailure { source: std::io::Error },

        #[snafu(display("Failed to bind directory {}: {}", dir, String::from_utf8_lossy(output.stderr.as_slice())))]
        BindDirectoryFailure {
            dir: String,
            output: std::process::Output,
        },

        #[snafu(display("Failed to share mounts for directory {} : {}", dir, String::from_utf8_lossy(output.stderr.as_slice())))]
        ShareMountsFailure {
            dir: String,
            output: std::process::Output,
        },

        #[snafu(display("Failed to create array : {}", String::from_utf8_lossy(output.stderr.as_slice())))]
        CreateArrayFailure { output: std::process::Output },

        #[snafu(display("Failed to scan array : {}", String::from_utf8_lossy(output.stderr.as_slice())))]
        ScanArrayFailure { output: std::process::Output },

        #[snafu(display("Failed to determine filesystem format {}", source))]
        DetermineFormatFailure { source: std::io::Error },

        #[snafu(display("Failed to determine mount status {}", source))]
        FindMntFailure { source: std::io::Error },

        #[snafu(display("Failed to format filesystem : {}", String::from_utf8_lossy(output.stderr.as_slice())))]
        FormatFilesystemFailure { output: std::process::Output },

        #[snafu(display("Invalid Parameter '{}', {}", parameter, reason))]
        InvalidParameter { parameter: String, reason: String },

        #[snafu(display("Failed to create directory, {}", source))]
        Mkdir { source: std::io::Error },
    }
}

pub type Result<T> = std::result::Result<T, error::Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bind_with_default_dirs_k8s() {
        let variant = "aws-k8s-1.33";
        let allowed_dirs = allowed_bind_dirs(variant);

        for dir in [
            "/var/lib/kubelet",
            "/var/lib/containerd",
            "/var/lib/soci-snapshotter",
            "/var/log/pods",
        ] {
            assert!(allowed_dirs.allowed_exact.contains(dir));
        }
    }

    #[test]
    fn test_bind_with_default_dirs_ecs() {
        let variant = "aws-ecs-2";
        let allowed_dirs = allowed_bind_dirs(variant);

        for dir in ["/var/lib/docker", "/var/lib/containerd", "/var/log/ecs"] {
            assert!(allowed_dirs.allowed_exact.contains(dir));
        }
    }
}
