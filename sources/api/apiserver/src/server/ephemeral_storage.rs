//! The 'ephemeral_storage' module supports configuring and using local instance storage.

use model::ephemeral_storage::Filesystem;

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

/// Name of the array (if created) and filesystem label. Selected to be 12 characters so it
/// fits within both the xfs and ext4 volume label limit.
static NAME: &str = "br-ephemeral";

/// initialize prepares the ephemeral storage for formatting and formats its.  For multiple disks
/// preparation is the creation of a RAID0 array, for a single disk this is a no-op. The array or disk
/// is then formatted with the specified filesystem (default=xfs) if not formatted already.
pub fn initialize(fs: Option<Filesystem>, disks: Option<Vec<String>>) -> Result<()> {
    let known_disks = ephemeral_devices()?;
    let known_disks_hash = HashSet::<_>::from_iter(known_disks.iter());

    let disks = match disks {
        Some(disks) => {
            let disks = disks.iter().map(OsString::from).collect();
            // we have disks provided, so match them against the list of valid disks
            for disk in &disks {
                ensure!(
                    known_disks_hash.contains(disk),
                    error::InvalidParameterSnafu {
                        parameter: "disks",
                        reason: format!("unknown disk {:?}", disk),
                    }
                )
            }
            disks
        }
        None => {
            // if there are no disks specified, and none are available we treat the init as a
            // no-op to allow "ephemeral storage init"/"ephemeral-storage bind" to work on instances
            // with and without ephemeral storage
            if known_disks.is_empty() {
                info!("no ephemeral disks found, skipping ephemeral storage initialization");
                return Ok(());
            }
            // no disks specified, so use the default
            known_disks
        }
    };

    ensure!(
        !disks.is_empty(),
        error::InvalidParameterSnafu {
            parameter: "disks",
            reason: "no local ephemeral disks specified",
        }
    );

    info!("initializing ephemeral storage disks={:?}", disks);
    // with a single disk, there is no need to create the array
    let device_name = match disks.len() {
        1 => disks.first().expect("non-empty").clone(),
        _ => {
            let scan_output = mdadm_scan()?;
            // no previously configured array found, so construct a new one
            if scan_output.is_empty() {
                info!("creating array named {:?} from {:?}", NAME, disks);
                mdadm_create(
                    OsString::from(NAME),
                    disks.iter().map(OsString::from).collect(),
                )?;
            }
            // can't lookup the array until it's created
            resolve_array_by_id()?
        }
    };

    let fs = fs.unwrap_or(Filesystem::XFS);
    if !is_formatted(&device_name, &fs)? {
        info!("formatting {:?} as {}", device_name, fs);
        format_device(&device_name, &fs)?;
    } else {
        info!(
            "{:?} is already formatted as {}, skipping format",
            device_name, fs
        );
    }

    Ok(())
}

/// binds the specified directories to the pre-configured array, creating those directories if
/// they do not exist.
pub fn bind(variant: &str, dirs: Vec<String>) -> Result<()> {
    // handle the no local instance storage case
    if ephemeral_devices()?.is_empty() {
        info!("no ephemeral disks found, skipping ephemeral storage binding");
        return Ok(());
    }

    let device_name = resolve_device_by_label()?;
    let mount_point = format!("/mnt/{}", NAME);
    let mount_point = Path::new(&mount_point);
    let allowed_dirs = allowed_bind_dirs(variant);
    for dir in &dirs {
        ensure!(
            allowed_dirs.contains(dir.as_str()),
            error::InvalidParameterSnafu {
                parameter: dir,
                reason: "specified bind directory not in allow list",
            }
        )
    }
    std::fs::create_dir_all(mount_point).context(error::MkdirSnafu {})?;

    info!("mounting {:?} as {:?}", device_name, mount_point);
    ensure!(
        Command::new(MOUNT)
            .args([&device_name, mount_point.as_os_str()])
            .status()
            .context(error::MountArrayFailureSnafu {})?
            .success(),
        error::MountArrayFailureStatusSnafu {}
    );

    for dir in dirs {
        // construct a directory name (E.g. /var/lib/kubelet => _var_lib_kubelet) that will be
        // unique between the binding targets
        let directory_name = OsString::from(dir.replace('/', "_"));
        let mount_destination = mount_point.join(&directory_name);
        let source_dir = OsString::from(&dir);

        // we may run before the directories we are binding exist, so create them
        std::fs::create_dir_all(&dir).context(error::MkdirSnafu {})?;
        std::fs::create_dir_all(&mount_destination).context(error::MkdirSnafu {})?;

        // call the equivalent of
        // mount --rbind /mnt/ephemeral/_var_lib_kubelet /var/lib/kubelet
        info!("binding {:?} to {:?}", source_dir, mount_destination);
        ensure!(
            Command::new(MOUNT)
                .args([
                    OsStr::new("--rbind"),
                    mount_destination.as_ref(),
                    &source_dir
                ])
                .status()
                .context(error::BindDirectoryFailureSnafu {
                    dir: String::from_utf8_lossy(source_dir.as_encoded_bytes())
                })?
                .success(),
            error::BindDirectoryFailureStatusSnafu {
                dir: String::from_utf8_lossy(source_dir.as_encoded_bytes())
            }
        );

        info!("sharing mounts for {:?}", source_dir);
        // mount --make-rshared /var/lib/kubelet
        ensure!(
            Command::new(MOUNT)
                .args([OsStr::new("--make-rshared"), &source_dir])
                .status()
                .context(error::ShareMountsFailureSnafu {
                    dir: String::from_utf8_lossy(source_dir.as_encoded_bytes())
                })?
                .success(),
            error::ShareMountsFailureStatusSnafu {
                dir: String::from_utf8_lossy(source_dir.as_encoded_bytes())
            }
        );
    }

    Ok(())
}

/// resolve_device_by_label resolves the by-label link for the raid array or single disk to the device name
fn resolve_device_by_label() -> Result<OsString> {
    let device_name = OsString::from(format!("/dev/disk/by-label/{}", NAME));

    // read the symlink from known named location to get the current device that we need
    // to use
    let device_name = std::fs::canonicalize(device_name)
        .context(error::CanonicalizeFailureSnafu {})?
        .into_os_string();

    Ok(device_name)
}

/// resolve_array_by_name resolves the by-id link for the raid array
fn resolve_array_by_id() -> Result<OsString> {
    let device_name = OsString::from(format!("/dev/disk/by-id/md-name-{}", NAME));

    // read the symlink from known named location to get the current device that we need
    // to use
    let device_name = std::fs::canonicalize(device_name)
        .context(error::CanonicalizeFailureSnafu {})?
        .into_os_string();

    Ok(device_name)
}

/// creates the array with the given name from the specified disks
fn mdadm_create<S: AsRef<OsStr>, T: AsRef<OsStr>>(name: S, disks: Vec<T>) -> Result<()> {
    let mut device_name = OsString::from("/dev/md/");
    device_name.push(name.as_ref());

    let mut cmd = Command::new(MDADM);
    cmd.arg("--create");
    cmd.arg("--force");
    cmd.arg("--verbose");
    cmd.arg(device_name);
    cmd.arg("--level=0");
    // By default, mdadm uses a 512KB chunk size. mkfs.xfs attempts to match some of its settings to
    // the array size for maximum throughput, but the max log stripe size for xfs is 256KB.  We limit
    // the chunk size to 256KB here so that XFS can set the same value and avoid the fallback to
    // a 32 KB log stripe size.
    cmd.arg("--chunk=256");
    cmd.arg("--name");
    cmd.arg(name);
    cmd.arg("--raid-devices");
    cmd.arg(OsString::from(disks.len().to_string()));
    for disk in disks {
        cmd.arg(disk);
    }
    ensure!(
        cmd.status()
            .context(error::CreateArrayFailureSnafu {})?
            .success(),
        error::CreateArrayFailureStatusSnafu {}
    );
    Ok(())
}

/// ephemeral_devices returns the full path name to the block devices in /dev/disk/ephemeral
pub fn ephemeral_devices() -> Result<Vec<OsString>> {
    const EPHEMERAL_PATH: &str = "/dev/disk/ephemeral";
    let mut filenames = Vec::new();
    // for instances without ephemeral storage, we don't error and just return an empty vector so
    // it can be handled gracefully
    if fs::metadata(EPHEMERAL_PATH).is_err() {
        return Ok(filenames);
    }

    let entries = std::fs::read_dir(EPHEMERAL_PATH).context(error::DiscoverEphemeralSnafu {})?;
    for entry in entries {
        let entry = entry.context(error::DiscoverEphemeralSnafu {})?;
        filenames.push(entry.path().into_os_string());
    }
    Ok(filenames)
}

/// allowed_bind_dirs returns a set of the directories that can be bound to ephemeral storage, which
/// varies based on the variant
pub fn allowed_bind_dirs(variant: &str) -> HashSet<&'static str> {
    let mut allowed = HashSet::from(["/var/lib/containerd"]);
    if variant.contains("k8s") {
        allowed.insert("/var/lib/kubelet");
        allowed.insert("/var/log/pods");
    }
    if variant.contains("ecs") {
        allowed.insert("/var/log/ecs");
    }
    allowed
}

/// scans the raid array to identify if it has been created already
fn mdadm_scan() -> Result<Vec<u8>> {
    let output = Command::new(MDADM)
        .args([OsStr::new("--detail"), OsStr::new("--scan")])
        .output()
        .context(error::ScanArrayFailureSnafu {})?;
    ensure!(
        output.status.success(),
        error::ScanArrayFailureStatusSnafu {}
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
        Filesystem::XFS => MKFSXFS,
        Filesystem::EXT4 => MKFSEXT4,
    };

    let mut mkfs = Command::new(binary);
    mkfs.arg(device.as_ref());
    // labeled, XFS has a max of 12 characters, EXT4 allows 16
    mkfs.arg("-L");
    mkfs.arg(NAME);

    let mkfs = mkfs
        .status()
        .context(error::FormatFilesystemFailureSnafu {})?;

    ensure!(mkfs.success(), error::FormatFilesystemFailureStatusSnafu {});
    Ok(())
}

pub mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub enum Error {
        #[snafu(display("Failed to discover ephemeral disk"))]
        DiscoverEphemeral { source: std::io::Error },

        #[snafu(display("Failed to mount array {}", source))]
        MountArrayFailure { source: std::io::Error },
        #[snafu(display("Failed to mount array (non-zero exit)"))]
        MountArrayFailureStatus {},

        #[snafu(display("Failed to create disk symlink {}", source))]
        DiskSymlinkFailure { source: std::io::Error },

        #[snafu(display("Failed to bind directory {}, {}", dir, source))]
        BindDirectoryFailure { dir: String, source: std::io::Error },
        #[snafu(display("Failed to bind directory {} (non-zero exit)", dir))]
        BindDirectoryFailureStatus { dir: String },

        #[snafu(display("Failed to share mounts for directory {}, {}", dir, source))]
        ShareMountsFailure { dir: String, source: std::io::Error },
        #[snafu(display("Failed to share mounts for directory {} (non-zero exit)", dir))]
        ShareMountsFailureStatus { dir: String },

        #[snafu(display("Failed to create array {}", source))]
        CreateArrayFailure { source: std::io::Error },
        #[snafu(display("Failed to create array (non-zero exit)"))]
        CreateArrayFailureStatus {},

        #[snafu(display("Failed to scan array {}", source))]
        ScanArrayFailure { source: std::io::Error },
        #[snafu(display("Failed to scan array (non-zero exit)"))]
        ScanArrayFailureStatus {},

        #[snafu(display("Failed to determine filesystem format {}", source))]
        DetermineFormatFailure { source: std::io::Error },

        #[snafu(display("Failed to format filesystem {}", source))]
        FormatFilesystemFailure { source: std::io::Error },
        #[snafu(display("Failed to format filesystem (non-zero exit)"))]
        FormatFilesystemFailureStatus {},

        #[snafu(display("Invalid Parameter '{}', {}", parameter, reason))]
        InvalidParameter { parameter: String, reason: String },

        #[snafu(display("Failed to create directory, {}", source))]
        Mkdir { source: std::io::Error },

        #[snafu(display("Failed to canonicalize path, {}", source))]
        CanonicalizeFailure { source: std::io::Error },
    }
}

pub type Result<T> = std::result::Result<T, error::Error>;
