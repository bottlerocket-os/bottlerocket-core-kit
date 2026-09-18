//! NVIDIA driver detection for ghostdog.
//!
//! Two independent decisions live here, each exposed as one entry point:
//!
//! - **Branch (version): `lts` vs `pb`** — [`match_branch`] backs the
//!   `match-nvidia-branch` subcommand. It picks the branch and writes
//!   `/run/nvidia/branch-<branch>`, a marker that systemd `ConditionPathExists`
//!   drop-ins use to gate the per-branch overlay and module-load units. Resolution
//!   order: an explicit `settings.kernel.drivers.nvidia.branch` pin, else the PCI devices
//!   present, else `lts`.
//!
//! - **Flavor (role): `tesla`/`open-gpu`/`grid`** — [`match_flavor`] backs the
//!   `match-nvidia-driver` subcommand (an `ExecCondition` on each module-load
//!   unit). It reports whether a requested role matches the hardware.
//!
//! The two are related but resolved separately: `match-nvidia-branch` writes the
//! marker; `match-nvidia-driver` reads it (via [`branch_from_marker`]) to find the
//! active branch's supported-devices file, so the two always agree.

use crate::error::{self, Result};
use lazy_static::lazy_static;
use serde::Deserialize;
use snafu::{ensure, ResultExt};
use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tokio_retry::{strategy::FixedInterval, Retry};

/// PCI vendor ID for NVIDIA.
const NVIDIA_VENDOR_ID: &str = "10de";

/// Canonical (single-driver) location of the open-gpu supported-devices file.
const OPEN_GPU_SUPPORTED_DEVICES_PATH: &str = "/usr/share/nvidia/open-gpu-supported-devices.json";

// Branch-namespaced storage used on multi-driver images. Each branch installs its
// files under `/usr/share/.nvidia/<branch>/` The supported-devices file
// lives at `/usr/share/.nvidia/<branch>/share/nvidia/open-gpu-supported-devices.json`.
const NVIDIA_BRANCH_ROOT: &str = "/usr/share/.nvidia";
const NVIDIA_BRANCH_SHARE_SUBDIR: &str = "share/nvidia";
const OPEN_GPU_SUPPORTED_DEVICES_FILE: &str = "open-gpu-supported-devices.json";

/// Directory the branch marker file is written to.
const MARKER_DIR: &str = "/run/nvidia";
/// Prefix of the branch marker file name (`branch-lts` / `branch-pb`).
const MARKER_PREFIX: &str = "branch-";

lazy_static! {
    // PCI device IDs (raw lowercase, as `pciclient` reports `device()`) whose
    // subdevice may indicate the GRID flavor. Checked against NVIDIA_GRID_SUBDEVICES.
    static ref NVIDIA_GRID_DEVICE_IDS: HashSet<&'static str> = {
        let mut m = HashSet::new();
        m.insert("27b8");  // L4 (g6e)
        m.insert("2c3a");  // GB203GL RTX PRO 4500 Blackwell (g7/g7f)
        m
    };

    // GRID subdevice IDs, normalized to `0x` + uppercase (matching a device's
    // subsystem_device). A present GRID device with one of these uses the grid flavor.
    static ref NVIDIA_GRID_SUBDEVICES: HashSet<&'static str> = {
        let mut m = HashSet::new();
        m.insert("0x1733");
        m.insert("0x1735");
        m.insert("0x1737");
        m.insert("0x229A");  // RTX PRO 4500 VF - 4G (g7f.xlarge)
        m.insert("0x229D");  // RTX PRO 4500 VF - 8G (g7f.2xlarge)
        m.insert("0x22A1");  // RTX PRO 4500 VF - 16G (g7f.4xlarge)
        m
    };

    // PCI device IDs (normalized `0x` + uppercase) that require the product branch
    // (pb / r595) but are absent from the driver's `supported-gpus.json`
    static ref NVIDIA_PB_DEVICE_IDS: HashSet<&'static str> = {
        let mut m = HashSet::new();
        m.insert("0x2C3A");  // GB203GL RTX PRO 4500 Blackwell (g7/g7f)
        m
    };
}

#[derive(Deserialize)]
/// Open GPU struct for comparing PCI IDs to a known list of supported devices.
enum SupportedDevicesConfiguration {
    #[serde(rename = "open-gpu")]
    OpenGpu(Vec<GpuDeviceData>),
}

#[derive(Eq, Debug, Deserialize, Hash, PartialEq)]
/// The GPU Device Data contains various features of the device. Only Name, Device ID, and Features are required
/// for a particular device
struct GpuDeviceData {
    #[serde(rename = "devid")]
    /// PCI Device ID
    device_id: String,
    #[serde(rename = "subdevid")]
    /// PCI Subdevice ID
    subdevice_id: Option<String>,
    #[serde(rename = "subvendorid")]
    /// PCI Subvendor ID
    subvendor_id: Option<String>,
    /// Name of the device
    name: String,
    /// List of features the device supports. We are looking for "kernelopen" to match the driver
    features: Vec<String>,
}

/// The NVIDIA driver branch (version) to activate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NvidiaBranch {
    /// Long-Term Support branch
    Lts,
    /// Product Branch
    Pb,
}

impl NvidiaBranch {
    /// The branch's name, as used in paths and the marker file.
    fn as_str(&self) -> &'static str {
        match self {
            NvidiaBranch::Lts => "lts",
            NvidiaBranch::Pb => "pb",
        }
    }

    /// Parses a branch from a setting value.
    fn from_setting(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "lts" => Some(NvidiaBranch::Lts),
            "pb" => Some(NvidiaBranch::Pb),
            _ => None,
        }
    }

    /// This branch's marker file path (`/run/nvidia/branch-<branch>`).
    fn marker_path(&self) -> PathBuf {
        Path::new(MARKER_DIR).join(format!("{MARKER_PREFIX}{}", self.as_str()))
    }

    /// This branch's open-gpu supported-devices file under its namespaced storage
    /// (`/usr/share/.nvidia/<branch>/share/nvidia/open-gpu-supported-devices.json`).
    fn open_gpu_supported_devices_file(&self) -> PathBuf {
        PathBuf::from(NVIDIA_BRANCH_ROOT)
            .join(self.as_str())
            .join(NVIDIA_BRANCH_SHARE_SUBDIR)
            .join(OPEN_GPU_SUPPORTED_DEVICES_FILE)
    }

    /// This branch's open-gpu-supported device IDs (the file's native `0x` +
    /// uppercase form). Empty if the file is absent (e.g. single-driver image or
    /// branch not installed); unreadable/corrupt file is an error.
    fn open_gpu_device_ids(&self) -> Result<HashSet<String>> {
        match read_supported_devices_file(self.open_gpu_supported_devices_file()) {
            Ok(SupportedDevicesConfiguration::OpenGpu(devices)) => {
                Ok(devices.into_iter().map(|d| d.device_id).collect())
            }
            Err(error::Error::OpenFile { source, .. })
                if source.kind() == std::io::ErrorKind::NotFound =>
            {
                Ok(HashSet::new())
            }
            Err(e) => Err(e),
        }
    }
}

impl fmt::Display for NvidiaBranch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors with `DriverMismatch` unless `driver_name` is the flavor preferred for
/// the hardware present.
pub(crate) fn match_flavor(driver_name: &str) -> Result<()> {
    let preferred_driver = find_preferred_driver()?;
    ensure!(
        driver_name == preferred_driver,
        error::DriverMismatchSnafu {
            requested: driver_name.to_string(),
            preferred: preferred_driver.clone(),
        }
    );
    log::info!("match-nvidia-driver: selected flavor '{preferred_driver}'");
    Ok(())
}

/// Determines which driver flavor (`grid`/`open-gpu`/`tesla`) to use based on the
/// PCI devices present and the active branch's open-gpu supported-devices file.
fn find_preferred_driver() -> Result<String> {
    let open_gpu_devices = read_supported_devices_file(open_gpu_supported_devices_path())?;
    let list_input = pciclient::ListDevicesParam::builder()
        .vendor(NVIDIA_VENDOR_ID)
        .build();
    let present_devices =
        pciclient::list_devices(list_input).context(error::ListPciDevicesSnafu)?;

    let open_gpu_device_set = match &open_gpu_devices {
        SupportedDevicesConfiguration::OpenGpu(device_list) => device_list
            .iter()
            .map(|x| &x.device_id)
            .collect::<HashSet<_>>(),
    };

    // Return early with grid if any present GRID-capable device has a matching subdevice ID.
    let grid_present = present_devices
        .iter()
        .filter(|x| NVIDIA_GRID_DEVICE_IDS.contains(x.device().as_str()))
        .any(|x| {
            let sub = format!(
                "0x{}",
                x.subsystem_device().as_deref().unwrap_or("").to_uppercase()
            );
            NVIDIA_GRID_SUBDEVICES.contains(sub.as_str())
        });
    if grid_present {
        return Ok("grid".to_string());
    }

    let open_gpu_present = present_devices
        .iter()
        .any(|x| open_gpu_device_set.contains(&format!("0x{}", x.device().to_uppercase())));
    if open_gpu_present {
        Ok("open-gpu".to_string())
    } else {
        Ok("tesla".to_string())
    }
}

/// Resolves the path to the open-gpu supported-devices file for the active branch.
/// If there is no marker or branch file (single-driver image), fall back to the
/// canonical path.
fn open_gpu_supported_devices_path() -> PathBuf {
    let path = branch_from_marker()
        .map(|branch| branch.open_gpu_supported_devices_file())
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from(OPEN_GPU_SUPPORTED_DEVICES_PATH));
    log::debug!("using supported-devices file: {}", path.display());
    path
}

/// Read a file into a SupportedDevicesConfiguration Enum
fn read_supported_devices_file(path: PathBuf) -> Result<SupportedDevicesConfiguration> {
    let mut supported_devices_file =
        fs::File::open(&path).context(error::OpenFileSnafu { path: path.clone() })?;
    let mut supported_devices_str = String::new();
    supported_devices_file
        .read_to_string(&mut supported_devices_str)
        .context(error::ReadFileSnafu { path: path.clone() })?;
    serde_json::from_str(supported_devices_str.as_str()).context(error::ParseGpuDevicesFileSnafu)
}

/// Resolves the active branch and writes its marker file into `/run/nvidia`.
pub(crate) fn match_branch() -> Result<()> {
    let branch = resolve_active_branch()?;
    write_marker(branch)
}

/// Resolves the branch: an explicit `settings.kernel.drivers.nvidia.branch` wins;
/// otherwise detect from the present PCI devices; otherwise prefer `lts`.
fn resolve_active_branch() -> Result<NvidiaBranch> {
    if let Some(branch) = branch_setting()? {
        log::info!("branch '{branch}' chosen from settings.kernel.drivers.nvidia.branch pin");
        return Ok(branch);
    }

    let present = present_nvidia_device_ids();
    let lts_open = NvidiaBranch::Lts.open_gpu_device_ids()?;
    let pb_open = NvidiaBranch::Pb.open_gpu_device_ids()?;

    let branch = if requires_pb(&present, &lts_open, &pb_open) {
        NvidiaBranch::Pb
    } else {
        NvidiaBranch::Lts
    };
    log::info!("branch '{branch}' chosen from PCI device detection");
    Ok(branch)
}

/// Returns true if any present device requires the product branch: it is
/// open-gpu-supported by pb but not lts (too new for lts), or it is a curated pb
/// device (a VF devid `supported-gpus.json` cannot describe). All IDs are compared
/// in the normalized `0x` + uppercase form.
fn requires_pb(
    present: &HashSet<String>,
    lts_open: &HashSet<String>,
    pb_open: &HashSet<String>,
) -> bool {
    present.iter().any(|devid| {
        (pb_open.contains(devid) && !lts_open.contains(devid))
            || NVIDIA_PB_DEVICE_IDS.contains(devid.as_str())
    })
}

/// Lists the present NVIDIA PCI device IDs, normalized to `0x` + uppercase.
/// Returns an empty set if the PCI bus cannot be listed.
fn present_nvidia_device_ids() -> HashSet<String> {
    let list_input = pciclient::ListDevicesParam::builder()
        .vendor(NVIDIA_VENDOR_ID)
        .build();
    match pciclient::list_devices(list_input) {
        Ok(devices) => devices
            .iter()
            .map(|x| format!("0x{}", x.device().to_uppercase()))
            .collect(),
        Err(e) => {
            log::warn!("Failed to list PCI devices for NVIDIA branch detection: {e}");
            HashSet::new()
        }
    }
}

/// Reads `settings.kernel.drivers.nvidia.branch` from the Bottlerocket API.
///
/// - `Ok(Some(branch))` -- a valid pin is set.
/// - `Ok(None)` -- the API responded but no valid pin is set (unset, empty, or
///   invalid value); the caller falls through to PCI detection.
/// - `Err(_)` -- the setting was unreadable after retries. We deliberately do
///   *not* fall back here: an unreadable setting is indistinguishable from an
///   unset one, so PCI detection could silently override an explicit pin. The
///   unit is ordered after apiserver, so a persistent failure is a real error.
fn branch_setting() -> Result<Option<NvidiaBranch>> {
    // The setting is served over the async API socket, so we need a runtime.
    // A single blocking GET needs only the current-thread scheduler, not the
    // multi-threaded worker pool.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context(error::TokioRuntimeSnafu)?;
    let uri = "/?prefix=settings.kernel.drivers.nvidia.branch";
    let value = rt
        .block_on(Retry::start(
            FixedInterval::from_millis(500).take(2),
            || apiclient::get::get_uri(constants::API_SOCKET, uri.to_string()),
        ))
        .context(error::ReadBranchSettingSnafu)?;

    // The API returns JSON like: {"settings":{"kernel":{"drivers":{"nvidia":{"branch":"lts"}}}}}
    // or an empty object if the setting is unset.
    Ok(value
        .pointer("/settings/kernel/drivers/nvidia/branch")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .and_then(NvidiaBranch::from_setting))
}

/// Writes the branch marker file, creating `/run/nvidia` if needed.
fn write_marker(branch: NvidiaBranch) -> Result<()> {
    fs::create_dir_all(MARKER_DIR).context(error::CreateMarkerDirSnafu {
        path: PathBuf::from(MARKER_DIR),
    })?;

    let path = branch.marker_path();
    fs::write(&path, "").context(error::WriteMarkerFileSnafu { path: path.clone() })
}

/// Returns the branch whose marker file exists, or `None` if neither is present
/// (e.g. a single-driver image, where `match_branch` never runs).
fn branch_from_marker() -> Option<NvidiaBranch> {
    [NvidiaBranch::Lts, NvidiaBranch::Pb]
        .into_iter()
        .find(|branch| branch.marker_path().exists())
}

#[cfg(test)]
mod test {
    use super::*;

    fn devset(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    // ------------------------------------------------------------------------
    // requires_pb -- the pure branch-detection logic
    // ------------------------------------------------------------------------

    #[test]
    fn requires_pb_when_open_gpu_supported_by_pb_only() {
        // Device is open-gpu-supported by pb but not lts → too new for lts.
        assert!(requires_pb(
            &devset(&["0xABCD"]),
            &devset(&[]),
            &devset(&["0xABCD"]),
        ));
    }

    #[test]
    fn no_pb_when_open_gpu_supported_by_both() {
        // Supported by both branches → prefer lts.
        assert!(!requires_pb(
            &devset(&["0xABCD"]),
            &devset(&["0xABCD"]),
            &devset(&["0xABCD"]),
        ));
    }

    #[test]
    fn requires_pb_from_curated_supplement() {
        // Curated VF devid (absent from both open-gpu files) forces pb.
        assert!(requires_pb(
            &devset(&["0x2C3A"]),
            &devset(&[]),
            &devset(&[])
        ));
    }

    #[test]
    fn no_pb_when_device_only_in_lts_open() {
        assert!(!requires_pb(
            &devset(&["0x1234"]),
            &devset(&["0x1234"]),
            &devset(&[]),
        ));
    }

    #[test]
    fn no_pb_when_nothing_matches() {
        // No present devices.
        assert!(!requires_pb(&devset(&[]), &devset(&[]), &devset(&[])));
        // Present device matches nothing.
        assert!(!requires_pb(
            &devset(&["0x9999"]),
            &devset(&[]),
            &devset(&["0xABCD"]),
        ));
    }

    #[test]
    fn requires_pb_if_any_present_device_needs_it() {
        // One benign device plus one pb-only device → pb.
        assert!(requires_pb(
            &devset(&["0x1234", "0x2C3A"]),
            &devset(&["0x1234"]),
            &devset(&["0x1234"]),
        ));
    }

    // ------------------------------------------------------------------------
    // NvidiaBranch
    // ------------------------------------------------------------------------

    #[test]
    fn branch_display() {
        assert_eq!(NvidiaBranch::Lts.to_string(), "lts");
        assert_eq!(NvidiaBranch::Pb.to_string(), "pb");
    }

    #[test]
    fn branch_from_setting_valid_case_insensitive_trimmed() {
        assert_eq!(NvidiaBranch::from_setting("lts"), Some(NvidiaBranch::Lts));
        assert_eq!(NvidiaBranch::from_setting("pb"), Some(NvidiaBranch::Pb));
        assert_eq!(NvidiaBranch::from_setting("LTS"), Some(NvidiaBranch::Lts));
        assert_eq!(NvidiaBranch::from_setting(" Pb\n"), Some(NvidiaBranch::Pb));
    }

    #[test]
    fn branch_from_setting_invalid() {
        assert_eq!(NvidiaBranch::from_setting(""), None);
        assert_eq!(NvidiaBranch::from_setting("invalid"), None);
        assert_eq!(NvidiaBranch::from_setting("latest"), None);
    }

    #[test]
    fn marker_path_matches_expected() {
        assert_eq!(
            NvidiaBranch::Lts.marker_path(),
            PathBuf::from("/run/nvidia/branch-lts")
        );
        assert_eq!(
            NvidiaBranch::Pb.marker_path(),
            PathBuf::from("/run/nvidia/branch-pb")
        );
    }

    // ------------------------------------------------------------------------
    // Supported-devices file parsing
    // ------------------------------------------------------------------------

    #[test]
    fn parse_open_gpu_supported_devices_file() {
        let test_json = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/tests")
            .join("open-gpu-supported-devices-test.json");

        match read_supported_devices_file(test_json).unwrap() {
            SupportedDevicesConfiguration::OpenGpu(data) => assert_eq!(data.len(), 6),
        }
    }
}
