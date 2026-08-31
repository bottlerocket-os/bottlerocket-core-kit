//! The 'ephemeral_storage' module supports configuring and using local instance storage.

use model::ephemeral_storage::{Filesystem, Preference};

use indexmap::IndexSet;
use snafu::{ensure, ResultExt};
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

static MOUNT: &str = "/usr/bin/mount";
static MDADM: &str = "/usr/sbin/mdadm";
static BLKID: &str = "/usr/sbin/blkid";
static MKFSXFS: &str = "/usr/sbin/mkfs.xfs";
static MKFSEXT4: &str = "/usr/sbin/mkfs.ext4";
static FINDMNT: &str = "/usr/bin/findmnt";

static EPHEMERAL_MNT: &str = "/mnt/.ephemeral";
static SRV_DIR: &str = "/srv";

/// Name of the device and its path from the MD driver
static RAID_DEVICE_DIR: &str = "/dev/md/";

/// Name of the array (if created) and filesystem label. Selected to be 12 characters so it
/// fits within both the xfs and ext4 volume label limit.
static RAID_DEVICE_NAME: &str = "ephemeral";
/// Intermediate symlink for consistent rottweiler mapper naming
static EPHEMERAL_DATA_LINK: &str = "/dev/disk/EPHEMERAL-DATA";
/// Device mapper path for encrypted ephemeral storage
static EPHEMERAL_MAPPER_DEVICE: &str = "/dev/mapper/EPHEMERAL-DATA";
/// Symlink to ephemeral storage array or disk
static EPHEMERAL_STORAGE_LINK: &str = "/dev/disk/ephemeral-storage";
/// Path to ephemeral devices (instance storage disks)
static EPHEMERAL_PATH: &str = "/dev/disk/ephemeral";
/// Path to ebs volumes marked for ephemeral storage use
static EPHEMERAL_EBS_PATH: &str = "/dev/disk/ephemeral-ebs";
/// Key ID to use for ephemeral storage encryption
static EPHEMERAL_STORAGE_KEY_ID: &str = "ephemeral-storage";
static BIND_DIRS_DROPIN_DIR: &str = "/usr/lib/bottlerocket/ephemeral-storage.d";

pub struct BindDirs {
    pub allowed_exact: HashSet<String>,
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
    let raw_device = match disks.len() {
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

    // Encrypt the device if enabled
    let device_name = if should_encrypt()? {
        encrypt_ephemeral_device(&raw_device)?
    } else {
        raw_device
    };

    let fs = fs.unwrap_or(Filesystem::Xfs);
    if !is_formatted(&device_name, &fs)? {
        info!("formatting {device_name:?} as {fs}");
        format_device(&device_name, &fs)?;
    } else {
        info!("{device_name:?} is already formatted as {fs}, skipping format");
    }

    // Clear previous link if it exists
    if std::fs::exists(EPHEMERAL_STORAGE_LINK).is_ok_and(|x| x) {
        std::fs::remove_file(EPHEMERAL_STORAGE_LINK).context(error::DiskUnlinkFailureSnafu {})?;
    }

    // Create link to formatted device for use in `bind`
    std::os::unix::fs::symlink(&device_name, EPHEMERAL_STORAGE_LINK)
        .context(error::DiskSymlinkFailureSnafu {})?;

    Ok(())
}

/// Binds the specified directories to the pre-configured array, creating those directories if
/// they do not exist.
pub fn bind(dirs: Vec<String>) -> Result<()> {
    let device_name = EPHEMERAL_STORAGE_LINK;
    if !std::fs::exists(device_name).is_ok_and(|x| x) {
        info!("ephemeral storage not initialized, skipping binding");
        return Ok(());
    }

    let dirs = if dirs.is_empty() {
        let allowed_dirs = allowed_bind_dirs()?;
        allowed_dirs.allowed_exact.into_iter().collect()
    } else {
        dirs
    };

    // Normalize input by trimming trailing "/"
    let dirs: Vec<String> = dirs
        .into_iter()
        .map(|dir| dir.trim_end_matches("/").to_string())
        .collect();

    let allowed_dirs = allowed_bind_dirs()?;
    for dir in &dirs {
        let exact_match = allowed_dirs.allowed_exact.contains(dir);
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

    if !is_mounted(EPHEMERAL_MNT)? {
        std::fs::create_dir_all(EPHEMERAL_MNT).context(error::MkdirSnafu { dir: EPHEMERAL_MNT })?;
        info!("mounting {device_name} as {EPHEMERAL_MNT}");
        let output = Command::new(MOUNT)
            .args([
                OsString::from(device_name),
                OsString::from(EPHEMERAL_MNT),
                OsString::from("--options"),
                OsString::from("defaults,nosuid,nodev,noatime,private"),
            ])
            .output()
            .context(error::ExecutionFailureSnafu { command: MOUNT })?;

        ensure!(
            output.status.success(),
            error::MountArrayFailureSnafu {
                what: device_name,
                dest: EPHEMERAL_MNT,
                output
            }
        );
    } else {
        info!("device already mounted at {EPHEMERAL_MNT}, skipping mount");
    }

    let dirs: Vec<OsString> = dirs.iter().map(OsString::from).collect();
    let mut dirs_to_bind = IndexSet::new();
    let mut dirs_to_mask = HashSet::new();

    // Check which directories need binding and/or masking
    for target_dir in &dirs {
        // Transform the directory path to a unique name
        let source_subdir = transform_dir_name(target_dir);
        let source_dir: PathBuf = [EPHEMERAL_MNT, &source_subdir].iter().collect();

        // Create the source directory now, since there's no chance of mounting over it.
        std::fs::create_dir_all(&source_dir).context(error::MkdirSnafu {
            dir: source_dir.clone(),
        })?;

        let is_source_masked = is_masked(&source_dir)?;
        let is_target_mounted = is_mounted(target_dir)?;

        match (is_target_mounted, is_source_masked) {
            (true, true) => continue,
            (true, false) => {
                dirs_to_mask.insert(source_dir.into());
            }
            (false, false) => {
                dirs_to_bind.insert((source_dir.clone(), target_dir.clone()));
                dirs_to_mask.insert(source_dir.into_os_string());
            }
            (false, true) => error::DirectoryAlreadyMaskedSnafu { dir: source_dir }.fail()?,
        }
    }

    // Sort to ensure parent directories are mounted before children
    dirs_to_bind.sort();

    // Perform bind mounts for directories that need it
    for (source_dir, target_dir) in &dirs_to_bind {
        info!(
            "binding '{}' to '{}'",
            source_dir.display(),
            target_dir.display(),
        );

        // Create the target directory now, in case we mounted one of its parent directories in a
        // previous iteration.
        std::fs::create_dir_all(target_dir).context(error::MkdirSnafu { dir: target_dir })?;

        let output = Command::new(MOUNT)
            .args([
                OsStr::new("--rbind"),
                source_dir.as_ref(),
                target_dir.as_ref(),
            ])
            .output()
            .context(error::ExecutionFailureSnafu { command: MOUNT })?;

        ensure!(
            output.status.success(),
            error::BindDirectoryFailureSnafu {
                source_dir,
                target_dir,
                output,
            }
        );
    }

    // Mask source directories that need it
    for source_dir in &dirs_to_mask {
        info!("masking {}", source_dir.display());
        let output = Command::new(MOUNT)
            .args([
                OsStr::new("--bind"),
                OsStr::new("--options"),
                OsStr::new("nosuid,nodev,noexec,private"),
                OsStr::new(SRV_DIR),
                source_dir,
            ])
            .output()
            .context(error::ExecutionFailureSnafu { command: MOUNT })?;

        ensure!(
            output.status.success(),
            error::MaskDirectorySnafu {
                dir: source_dir,
                output,
            }
        );
    }

    // Make mounts shared
    for target_dir in &dirs {
        info!("sharing mounts for {}", target_dir.display());
        let output = Command::new(MOUNT)
            .args([OsStr::new("--make-rshared"), target_dir])
            .output()
            .context(error::ExecutionFailureSnafu { command: MOUNT })?;

        ensure!(
            output.status.success(),
            error::ShareMountsFailureSnafu {
                dir: target_dir,
                output
            }
        );
    }

    Ok(())
}

/// Transform a directory path into a unique name suitable for use as a mount source
pub fn transform_dir_name(dir: impl AsRef<OsStr>) -> String {
    let mut directory_name = dir.as_ref().to_string_lossy().replace('/', "_");
    directory_name.insert(0, '.');
    directory_name
}

/// is_mounted returns true if the specified path is already listed as a mount
fn is_mounted(path: impl AsRef<OsStr>) -> Result<bool> {
    let status = Command::new(FINDMNT)
        .arg(path)
        .status()
        .context(error::FindMntFailureSnafu {})?;
    Ok(status.success())
}

/// is_masked returns true if the specified path is already masked
fn is_masked(path: impl AsRef<OsStr>) -> Result<bool> {
    let output = Command::new(FINDMNT)
        .args([OsStr::new("-no"), OsStr::new("SOURCE")])
        .arg(&path)
        .output()
        .context(error::CheckMaskSnafu { dir: &path })?;

    // Check if the command was successful and if the output contains "[/srv]"
    if output.status.success() {
        let source = String::from_utf8_lossy(&output.stdout);
        Ok(source.trim().ends_with(&format!("[{SRV_DIR}]")))
    } else {
        // If the command failed, the path is not mounted at all
        Ok(false)
    }
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

/// ephemeral_devices returns the full path name to the block devices in EPHEMERAL_PATH
pub fn ephemeral_devices() -> Result<Vec<String>> {
    discover_disks(EPHEMERAL_PATH)
}

/// ephemeral_ebs_volumes returns the full path name to the ebs volumes in EPHEMERAL_EBS_PATH
pub fn ephemeral_ebs_volumes() -> Result<Vec<String>> {
    discover_disks(EPHEMERAL_EBS_PATH)
}

/// discover_disks returns the full path name to the entries in the specified path,
/// or an empty vector if the specified path does not exist
fn discover_disks(path: &str) -> Result<Vec<String>> {
    let mut filenames = Vec::new();
    if fs::metadata(path).is_err() {
        return Ok(filenames);
    }
    let entries = std::fs::read_dir(path).context(error::DiscoverEphemeralSnafu {
        path: String::from(path),
    })?;
    for entry in entries {
        let entry = entry.context(error::DiscoverEphemeralSnafu {
            path: String::from(path),
        })?;
        filenames.push(entry.path().into_os_string().to_string_lossy().to_string());
    }
    Ok(filenames)
}

/// read_bind_dir_dropins reads newline-delimited absolute paths from all files in the specified
/// directory, skipping comments and blank lines. Returns an error if any entry is unreadable or
/// if a fragment contains an invalid path (non-absolute or containing "..").
fn read_bind_dir_dropins(dir: &str) -> Result<HashSet<String>> {
    let mut paths = HashSet::new();

    // If the directory doesn't exist, return empty set (graceful no-op)
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(paths),
        Err(e) => {
            return Err(e).context(error::ReadDropinDirSnafu { dir });
        }
    };

    for entry in entries {
        let entry = entry.context(error::ReadDropinDirEntrySnafu { dir })?;

        let path = entry.path();
        ensure!(
            path.is_file(),
            error::InvalidDropinEntrySnafu {
                path: &path,
                reason: "not a regular file",
            }
        );

        let content =
            fs::read_to_string(&path).context(error::ReadDropinFileSnafu { path: &path })?;

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and blank lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Trim trailing slash
            let line = line.trim_end_matches('/');

            // Reject non-absolute paths
            ensure!(
                line.starts_with('/'),
                error::InvalidDropinEntrySnafu {
                    path: &path,
                    reason: format!("non-absolute path: {line}"),
                }
            );

            // Reject paths containing ".."
            ensure!(
                !line.contains(".."),
                error::InvalidDropinEntrySnafu {
                    path: &path,
                    reason: format!("path contains '..': {line}"),
                }
            );

            paths.insert(line.to_string());
        }
    }

    Ok(paths)
}

/// allowed_bind_dirs returns a set of the directories that can be bound to ephemeral storage,
/// a set of the prefixes of directories that are allowed to be bound,
/// and a set of substrings that are disallowed in the directory name.
pub fn allowed_bind_dirs() -> Result<BindDirs> {
    let allowed_exact = read_bind_dir_dropins(BIND_DIRS_DROPIN_DIR)?;
    let allowed_prefixes = HashSet::from(["/mnt/"]);
    let disallowed_contains = HashSet::from(["..", "/mnt/.ephemeral"]);

    Ok(BindDirs {
        allowed_exact,
        allowed_prefixes,
        disallowed_contains,
    })
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

/// Checks if ephemeral storage encryption is enabled via image features
fn should_encrypt() -> Result<bool> {
    let features = bottlerocket_image_features::parse_image_features().map_err(|e| {
        error::Error::LoadImageFeatures {
            message: e.to_string(),
        }
    })?;
    Ok(features.encrypted_storage)
}

/// Returns true when ephemeral-encryption-keys image feature is enabled.
fn ephemeral_encryption_keys_enabled() -> Result<bool> {
    let features = bottlerocket_image_features::parse_image_features().map_err(|e| {
        error::Error::LoadImageFeatures {
            message: e.to_string(),
        }
    })?;
    Ok(features.ephemeral_encryption_keys)
}

/// Encrypt ephemeral device using rottweiler
fn encrypt_ephemeral_device(device: &str) -> Result<String> {
    info!("encrypting ephemeral device {device:?}");

    // Clear previous link if it exists
    if std::fs::exists(EPHEMERAL_DATA_LINK).is_ok_and(|x| x) {
        std::fs::remove_file(EPHEMERAL_DATA_LINK).context(error::DiskUnlinkFailureSnafu {})?;
    }

    // Create intermediate symlink for rottweiler to use
    // This ensures consistent mapper name: /dev/mapper/EPHEMERAL-DATA
    std::os::unix::fs::symlink(device, EPHEMERAL_DATA_LINK)
        .context(error::DiskSymlinkFailureSnafu {})?;

    if ephemeral_encryption_keys_enabled()? {
        // No format or encrypted-check needed: a fresh key makes all prior contents unreadable.
        run_rottweiler_checked(
            &["generate-key", EPHEMERAL_STORAGE_KEY_ID],
            EPHEMERAL_DATA_LINK,
        )?;

        run_rottweiler_checked(
            &[
                "encrypt-and-attach",
                "block-device",
                EPHEMERAL_DATA_LINK,
                EPHEMERAL_STORAGE_KEY_ID,
            ],
            EPHEMERAL_DATA_LINK,
        )?;

        run_rottweiler_checked(
            &["delete-key", EPHEMERAL_STORAGE_KEY_ID],
            EPHEMERAL_DATA_LINK,
        )?;
    } else {
        let is_encrypted =
            run_rottweiler(&["check", "block-device", EPHEMERAL_DATA_LINK, "encrypted"])?
                .status
                .success();

        if !is_encrypted {
            run_rottweiler_checked(
                &["generate-key", EPHEMERAL_STORAGE_KEY_ID],
                EPHEMERAL_DATA_LINK,
            )?;

            run_rottweiler_checked(
                &[
                    "encrypt",
                    "block-device",
                    EPHEMERAL_DATA_LINK,
                    EPHEMERAL_STORAGE_KEY_ID,
                ],
                EPHEMERAL_DATA_LINK,
            )?;
        }

        run_rottweiler_checked(
            &[
                "attach",
                "block-device",
                EPHEMERAL_DATA_LINK,
                EPHEMERAL_STORAGE_KEY_ID,
            ],
            EPHEMERAL_DATA_LINK,
        )?;
    }

    Ok(EPHEMERAL_MAPPER_DEVICE.to_string())
}

/// Execute a rottweiler command and return the output
fn run_rottweiler(args: &[&str]) -> Result<std::process::Output> {
    Command::new("/usr/bin/rottweiler")
        .args(args)
        .output()
        .context(error::ExecutionFailureSnafu {
            command: "rottweiler",
        })
}

/// Execute a rottweiler command and ensure it succeeds
fn run_rottweiler_checked(args: &[&str], device: &str) -> Result<()> {
    let output = run_rottweiler(args)?;

    ensure!(
        output.status.success(),
        error::EncryptDeviceSnafu {
            device,
            command: "rottweiler",
            args: args.join(" "),
            output
        }
    );

    Ok(())
}

pub mod error {
    use snafu::Snafu;
    use std::{ffi::OsString, path::PathBuf};

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

        #[snafu(display("Failed to mount {} to {}: {}", what, dest, String::from_utf8_lossy(output.stderr.as_slice())))]
        MountArrayFailure {
            what: String,
            dest: String,
            output: std::process::Output,
        },

        #[snafu(display("Failed to remove disk symlink {}", source))]
        DiskUnlinkFailure { source: std::io::Error },

        #[snafu(display("Failed to create disk symlink {}", source))]
        DiskSymlinkFailure { source: std::io::Error },

        #[snafu(display("Failed to bind directory '{}' to '{}': {}", source_dir.display(), target_dir.display(), String::from_utf8_lossy(output.stderr.as_slice())))]
        BindDirectoryFailure {
            source_dir: OsString,
            target_dir: OsString,
            output: std::process::Output,
        },

        #[snafu(display("Failed to share mounts for directory {} : {}", dir.display(), String::from_utf8_lossy(output.stderr.as_slice())))]
        ShareMountsFailure {
            dir: PathBuf,
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

        #[snafu(display("Failed to create directory '{}': {}", dir.display(), source))]
        Mkdir {
            source: std::io::Error,
            dir: PathBuf,
        },

        #[snafu(display("Unable to load image features: {}", message))]
        LoadImageFeatures { message: String },

        #[snafu(display("Failed to run '{}' with args '{}' on device '{}': stdout: {}, stderr: {}", command, args, device, String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr)))]
        EncryptDevice {
            device: String,
            command: &'static str,
            args: String,
            output: std::process::Output,
        },
        #[snafu(display("Failed to check if directory '{}' is masked: {}", dir.display(), source))]
        CheckMask {
            dir: PathBuf,
            source: std::io::Error,
        },

        #[snafu(display("Failed to mask directory '{}': {}", dir.display(), String::from_utf8_lossy(output.stderr.as_slice())))]
        MaskDirectory {
            dir: PathBuf,
            output: std::process::Output,
        },

        #[snafu(display("Cannot bind directory '{}': directory is masked", dir.display()))]
        DirectoryAlreadyMasked { dir: PathBuf },

        #[snafu(display(
            "Failed to read ephemeral-storage drop-in directory '{}': {}",
            dir,
            source
        ))]
        ReadDropinDir { dir: String, source: std::io::Error },

        #[snafu(display(
            "Failed to read entry in ephemeral-storage drop-in directory '{}': {}",
            dir,
            source
        ))]
        ReadDropinDirEntry { dir: String, source: std::io::Error },

        #[snafu(display("Failed to read ephemeral-storage drop-in file '{}': {}", path.display(), source))]
        ReadDropinFile {
            path: PathBuf,
            source: std::io::Error,
        },

        #[snafu(display("Invalid entry in ephemeral-storage drop-in '{}': {}", path.display(), reason))]
        InvalidDropinEntry { path: PathBuf, reason: String },
    }
}

pub type Result<T> = std::result::Result<T, error::Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_read_bind_dir_dropins_union_and_dedup() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        // Create two fragment files with some overlapping paths
        fs::write(
            dir_path.join("kubelet.conf"),
            "/var/lib/kubelet\n/var/lib/containerd\n/var/log/pods\n",
        )
        .unwrap();
        fs::write(
            dir_path.join("soci-snapshotter.conf"),
            "/var/lib/containerd\n/var/lib/soci-snapshotter\n",
        )
        .unwrap();

        let result = read_bind_dir_dropins(dir_path.to_str().unwrap()).unwrap();

        // Should have union of all paths (4 unique paths)
        assert_eq!(result.len(), 4);
        assert!(result.contains("/var/lib/kubelet"));
        assert!(result.contains("/var/lib/containerd"));
        assert!(result.contains("/var/log/pods"));
        assert!(result.contains("/var/lib/soci-snapshotter"));
    }

    #[test]
    fn test_read_bind_dir_dropins_skip_comments_and_blanks() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        // Create fragment with comments and blank lines
        fs::write(
            dir_path.join("test.conf"),
            "# This is a comment\n\n/var/lib/kubelet\n  \n# Another comment\n/var/log/pods\n\n",
        )
        .unwrap();

        let result = read_bind_dir_dropins(dir_path.to_str().unwrap()).unwrap();

        // Should only have the two valid paths
        assert_eq!(result.len(), 2);
        assert!(result.contains("/var/lib/kubelet"));
        assert!(result.contains("/var/log/pods"));
    }

    #[test]
    fn test_read_bind_dir_dropins_trim_trailing_slash() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        // Create fragment with paths that have trailing slashes
        fs::write(
            dir_path.join("test.conf"),
            "/var/lib/kubelet/\n/var/log/pods/\n",
        )
        .unwrap();

        let result = read_bind_dir_dropins(dir_path.to_str().unwrap()).unwrap();

        // Should have paths without trailing slashes
        assert_eq!(result.len(), 2);
        assert!(result.contains("/var/lib/kubelet"));
        assert!(result.contains("/var/log/pods"));
        assert!(!result.contains("/var/lib/kubelet/"));
        assert!(!result.contains("/var/log/pods/"));
    }

    #[test]
    fn test_read_bind_dir_dropins_reject_non_absolute_path() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        fs::write(
            dir_path.join("test.conf"),
            "/var/lib/kubelet\nrelative/path\n",
        )
        .unwrap();

        let result = read_bind_dir_dropins(dir_path.to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_read_bind_dir_dropins_reject_dotdot_path() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        fs::write(dir_path.join("test.conf"), "/var/../etc/passwd\n").unwrap();

        let result = read_bind_dir_dropins(dir_path.to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_read_bind_dir_dropins_nonexistent_dir() {
        // Call with a directory that doesn't exist
        let result = read_bind_dir_dropins("/nonexistent/path/to/nowhere").unwrap();

        // Should return empty set without error
        assert!(result.is_empty());
    }

    #[test]
    fn test_transform_dir_name() {
        // Test basic path transformation
        assert_eq!(transform_dir_name("/var/lib/kubelet"), "._var_lib_kubelet");

        // Test path with trailing slash
        assert_eq!(transform_dir_name("/var/lib/docker/"), "._var_lib_docker_");

        // Test root path
        assert_eq!(transform_dir_name("/"), "._");

        // Test empty string
        assert_eq!(transform_dir_name(""), ".");
    }
}
