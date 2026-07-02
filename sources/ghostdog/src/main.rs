/*!
ghostdog is a tool to manage ephemeral disks.
It can be called as a udev helper program to identify ephemeral disks.
It can also be called for EFA device detection which can be used for ExecCondition in systemd units.
It can also check if devices on the PCI bus match a particular NVIDIA driver.
*/

mod create_device;
mod error;
mod infiniband;

use crate::error::Result;
use crate::infiniband::find_infiniband_devices;
use argh::FromArgs;
use gptman::GPT;
use hex_literal::hex;
use lazy_static::lazy_static;
use serde::Deserialize;
use signpost::uuid_to_guid;
use snafu::{ensure, ResultExt};
use std::collections::HashSet;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs, str};
use tempfile::NamedTempFile;

const NVME_CLI_PATH: &str = "/sbin/nvme";
const NVME_IDENTIFY_DATA_SIZE: usize = 4096;
const NVIDIA_VENDOR_ID: &str = "10de";
const NVIDIA_GRID_DEVICE_ID: &str = "27b8";
const OPEN_GPU_SUPPORTED_DEVICES_PATH: &str = "/usr/share/nvidia/open-gpu-supported-devices.json";

// Generate a list of Subdevice IDs that match the format of the file at OPEN_GPU_SUPPORTED_DEVICES_PATH
// but are instead sourced here. The format in the JSON file has each ID starting with `0x` and are all upper
// case where `pciclient` will provide just the 4 character ID with no prefix. `pciclient` output has to be
// prepended and moved to uppercase to match these just as if they were sourced from the JSON file.
lazy_static! {
    static ref NVIDIA_GRID_SUBDEVICES: HashSet<&'static str> = {
        let mut m = HashSet::new();
        m.insert("0x1733");
        m.insert("0x1735");
        m.insert("0x1737");
        m
    };
}

#[derive(FromArgs, PartialEq, Debug)]
/// Manage ephemeral disks.
struct Args {
    #[argh(subcommand)]
    subcommand: SubCommand,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum SubCommand {
    Scan(ScanArgs),
    EbsDeviceName(EbsDeviceNameArgs),
    EfaPresent(EfaPresentArgs),
    NeuronPresent(NeuronPresentArgs),
    MatchDriver(MatchDriverArgs),
    MatchNvidiaDriver(MatchNvidiaDriverArgs),
    WriteInfinibandGuid(WriteInfinibandGuidArgs),
    CreateDevice(CreateDeviceArgs),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "efa-present")]
/// Detect if EFA devices are attached.
struct EfaPresentArgs {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "neuron-present")]
/// Detect if Neuron devices are attached.
struct NeuronPresentArgs {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "scan")]
/// Scan a device to see if it is an ephemeral disk.
struct ScanArgs {
    #[argh(positional)]
    device: PathBuf,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "ebs-device-name")]
/// Returns the device name used for the EBS device
struct EbsDeviceNameArgs {
    #[argh(positional)]
    device: PathBuf,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "match-nvidia-driver")]
/// Returns if devices on the PCI bus support the provided driver.
struct MatchNvidiaDriverArgs {
    #[argh(positional)]
    driver_name: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "match-driver")]
/// Returns if devices on the PCI bus support the provided driver.
struct MatchDriverArgs {
    #[argh(positional)]
    driver_name: String,
    #[argh(positional)]
    flavor_name: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "write-infiniband-primary-guid")]
/// Detect if Infiniband devices are attached and write the primary port guid to an env file if detected
struct WriteInfinibandGuidArgs {
    #[argh(positional)]
    env_file: PathBuf,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "create-device")]
/// Create a device node using mknod based on /proc/devices lookup
struct CreateDeviceArgs {
    #[argh(positional)]
    /// device name to look up in /proc/devices
    device_name: String,
    #[argh(positional)]
    /// minor device number
    minor: u32,
    #[argh(option)]
    /// override path for device node (default: /dev/<device_name><minor>)
    path: Option<String>,
    #[argh(option, default = "String::from(\"0666\")")]
    /// permissions mode (default: 0666)
    mode: String,
}

#[derive(Deserialize)]
/// Open GPU struct for comparing PCI ID's to a known list of supported devices.
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
    /// List of features the device supports. Noteably we are looking for "kernelopen" to match the driver
    features: Vec<String>,
}

// Main entry point.
#[snafu::report]
fn main() -> Result<()> {
    let args: Args = argh::from_env();
    match args.subcommand {
        SubCommand::Scan(scan_args) => {
            let path = scan_args.device;
            let mut f = fs::File::open(&path).context(error::DeviceOpenSnafu { path })?;
            let device_type = find_device_type(&mut f)?;
            emit_device_type(&device_type);
        }
        SubCommand::EbsDeviceName(ebs_device_name) => {
            let path = ebs_device_name.device;
            let device_name = find_device_name(format!("{}", path.display()))?;
            emit_device_name(&device_name);
        }
        SubCommand::EfaPresent(_) => {
            is_efa_attached()?;
        }
        SubCommand::NeuronPresent(_) => {
            is_neuron_attached()?;
        }
        SubCommand::MatchNvidiaDriver(driver) => {
            let driver_name = driver.driver_name;
            nvidia_driver_supported(&driver_name)?;
        }
        SubCommand::MatchDriver(driver) => {
            let driver_name = driver.driver_name;
            let flavor_name = driver.flavor_name;
            match driver_name.as_str() {
                "nvidia" => nvidia_driver_supported(&flavor_name)?,
                "neuron" => match_neuron_driver(&flavor_name)?,
                _ => {
                    return Err(error::Error::UnsupportedDriver {
                        driver: driver_name.to_string(),
                    })
                }
            }
        }
        SubCommand::WriteInfinibandGuid(envfile) => {
            find_and_write_infiniband_guid(envfile.env_file)?;
        }
        SubCommand::CreateDevice(args) => {
            create_device::create_device(&args)?;
        }
    }
    Ok(())
}

fn is_efa_attached() -> Result<()> {
    if pciclient::is_efa_attached().context(error::CheckEfaFailureSnafu)? {
        Ok(())
    } else {
        Err(error::Error::NoEfaPresent)
    }
}

fn is_neuron_attached() -> Result<()> {
    if pciclient::is_neuron_attached().context(error::CheckNeuronFailureSnafu)? {
        Ok(())
    } else {
        Err(error::Error::NoNeuronPresent)
    }
}

// Returns true if this is an inf1 instance
fn is_inf1_instance() -> Result<()> {
    if pciclient::is_inf1_instance().context(error::CheckInf1FailureSnafu)? {
        return Ok(());
    }
    Err(error::Error::NoInf1Present)
}

// Returns true if is a Neuron-based instance but not inf1. Inf2 is a stand in name for all hardware after
// inf1 but leaves this open for more hardware types that may need specific decisions from ghostdog in the
// future.
fn is_inf2_instance() -> Result<()> {
    if pciclient::is_inf2_instance().context(error::CheckInf2FailureSnafu)? {
        return Ok(());
    }
    Err(error::Error::NoInf2Present)
}

/// Detects if infiniband is present. If not, return early. If it does find Infiniband devices,
/// find if they match specific capabilities to be used for communication between NVIDIA Fabric
/// Manger and NVLSM, then write the guid to an env file for use by those services.
fn find_and_write_infiniband_guid(env_file: PathBuf) -> Result<()> {
    let devices = find_infiniband_devices()?;

    // Return early if no devices are found
    if devices.is_empty() {
        return Ok(());
    }

    // For each device, confirm if SW_MNG is present, then find the first port of the device. If that
    // device has the correct capability mask, then get the GUID. The first device to be found in this
    // search is the correct GUID for the configuration file.
    for device in devices {
        if device.is_device_sw_mng()? {
            let ports = device.find_ports_for_device()?;
            for port in ports {
                if port.is_sm_enabled() {
                    // NVIDIA Fabric Manager or NVLSM use -g to configure a Port GUID, use this as input
                    // to those services
                    write_config_string(
                        env_file.clone(),
                        format!("GUID_ARG=\"-g {}\"", *port.port_guid).as_str(),
                    )?;
                    return Ok(());
                }
            }
        }
    }
    // If no suitable GUIDs were found, return without writing to the file
    Ok(())
}

/// Find the device type by examining the partition table, if present.
fn find_device_type<R>(reader: &mut R) -> Result<String>
where
    R: Read + Seek,
{
    // We expect the udev rules to only match block disk devices, so it's fair
    // to assume it could have a partition table, and that it's probably an
    // unformatted ephemeral disk if it doesn't.
    let mut device_type = "ephemeral";

    // System disks will either have a known partition type or a partition name
    // that starts with BOTTLEROCKET.
    if let Ok(gpt) = GPT::find_from(reader) {
        let system_device = gpt.iter().any(|(_, p)| {
            p.is_used()
                && (SYSTEM_PARTITION_TYPES.contains(&p.partition_type_guid)
                    || p.partition_name.as_str().starts_with("BOTTLEROCKET"))
        });
        if system_device {
            device_type = "system"
        }
    }

    Ok(device_type.to_string())
}

/// Finds the device name using the nvme-cli
fn find_device_name(path: String) -> Result<String> {
    // nvme-cli writes the binary data to STDOUT
    let output = Command::new(NVME_CLI_PATH)
        .args(["id-ctrl", &path, "-b"])
        .output()
        .context(error::NvmeCommandSnafu { path: path.clone() })?;

    parse_device_name(&output.stdout, path)
}

/// Parses the device name from the binary data returned by nvme-cli
fn parse_device_name(device_info: &[u8], path: String) -> Result<String> {
    // Bail out if the data returned isn't complete
    ensure!(
        device_info.len() == NVME_IDENTIFY_DATA_SIZE,
        error::InvalidDeviceInfoSnafu { path }
    );

    // The vendor data is stored at the last 1024 bytes
    // The device name is stored at the first 32 bytes of the vendor data
    let offset = NVME_IDENTIFY_DATA_SIZE - 1024;
    let device_name = &device_info[offset..offset + 32];

    // Remove `/dev` in case the returned device name includes it, the udev
    // rule already includes that prefix
    Ok(String::from_utf8_lossy(device_name)
        .trim_start_matches("/dev/")
        .trim_end()
        .to_string())
}

/// Read a file into a SupportedDevicesConfiguration Enum
fn read_supported_devices_file(path: PathBuf) -> Result<SupportedDevicesConfiguration> {
    let mut supported_devices_file =
        fs::File::open(&path).context(error::OpenFileSnafu { path: path.clone() })?;
    let mut supported_devices_str = String::new();
    supported_devices_file
        .read_to_string(&mut supported_devices_str)
        .context(error::ReadFileSnafu { path: path.clone() })?;
    let device_configuration: SupportedDevicesConfiguration =
        serde_json::from_str(supported_devices_str.as_str())
            .context(error::ParseGpuDevicesFileSnafu)?;
    Ok(device_configuration)
}

/// Search the Open GPU Supported Devices File to determine which driver should be used based upon PCI devices present
fn find_preferred_driver() -> Result<String> {
    let open_gpu_devices = read_supported_devices_file(OPEN_GPU_SUPPORTED_DEVICES_PATH.into())?;
    let list_input = pciclient::ListDevicesParam::builder()
        .vendor(NVIDIA_VENDOR_ID)
        .build();
    let present_devices =
        pciclient::list_devices(list_input).context(error::ListPciDevicesSnafu)?;

    // If there a multiple devices with the same ID, dedup them to minimize iterations
    let mut unique_ids = present_devices
        .iter()
        .map(|x| format!("0x{}", x.device().to_uppercase()).clone())
        .collect::<HashSet<_>>()
        .into_iter();

    let open_gpu_device_set = match &open_gpu_devices {
        SupportedDevicesConfiguration::OpenGpu(ref device_list) => device_list
            .iter()
            .map(|x| &x.device_id)
            .collect::<HashSet<_>>(),
    };

    // If the PCI device ID is one that could potentially use GRID, collect the Subdevice IDs
    let mut subdevice_ids = present_devices
        .iter()
        .filter(|x| x.device().starts_with(NVIDIA_GRID_DEVICE_ID))
        .map(|x| {
            format!(
                "0x{}",
                x.subsystem_device()
                    .as_ref()
                    .unwrap_or(&"".to_string())
                    .to_uppercase()
            )
            .clone()
        })
        .collect::<HashSet<_>>()
        .into_iter();
    // Return early with grid if a match is made for these subdevices
    if subdevice_ids.any(|subdevice| NVIDIA_GRID_SUBDEVICES.contains(subdevice.as_str())) {
        return Ok("grid".to_string());
    }

    if unique_ids.any(|input_device| open_gpu_device_set.contains(&input_device)) {
        Ok("open-gpu".to_string())
    } else {
        Ok("tesla".to_string())
    }
}

/// Print the device type in the environment key format udev expects.
fn emit_device_type(device_type: &str) {
    println!("BOTTLEROCKET_DEVICE_TYPE={device_type}");
}

/// Print the device name in the environment key format udev expects.
fn emit_device_name(device_name: &str) {
    println!("XVD_DEVICE_NAME={device_name}")
}

/// Exit with exit code 1 if the driver name provided doesn't match the preferred driver
fn nvidia_driver_supported(driver_name: &str) -> Result<()> {
    let preferred_driver = find_preferred_driver()?;
    ensure!(
        driver_name == preferred_driver,
        error::DriverMismatchSnafu {
            requested: driver_name,
            preferred: preferred_driver
        }
    );
    Ok(())
}

fn match_neuron_driver(driver_flavor: &str) -> Result<()> {
    match driver_flavor {
        "inf1" => is_inf1_instance(),
        "latest" => is_inf2_instance(),
        _ => Err(error::Error::UnsupportedDriverFlavor {
            driver: "neuron".to_string(),
            flavor: driver_flavor.to_string(),
        }),
    }
}

// Known system partition types for Bottlerocket.
lazy_static! {
    static ref SYSTEM_PARTITION_TYPES: HashSet<[u8; 16]> = [
        uuid_to_guid(hex!("c12a7328 f81f 11d2 ba4b 00a0c93ec93b")), // EFI_SYSTEM
        uuid_to_guid(hex!("6b636168 7420 6568 2070 6c616e657421")), // BOTTLEROCKET_BOOT
        uuid_to_guid(hex!("5526016a 1a97 4ea4 b39a b7c8c6ca4502")), // BOTTLEROCKET_ROOT
        uuid_to_guid(hex!("598f10af c955 4456 6a99 7720068a6cea")), // BOTTLEROCKET_HASH
        uuid_to_guid(hex!("0c5d99a5 d331 4147 baef 08e2b855bdc9")), // BOTTLEROCKET_RESERVED
        uuid_to_guid(hex!("440408bb eb0b 4328 a6e5 a29038fad706")), // BOTTLEROCKET_PRIVATE
        uuid_to_guid(hex!("626f7474 6c65 6474 6861 726d61726b73")), // BOTTLEROCKET_DATA
    ].iter().copied().collect();
}

/// write_config_string will write the provided string to the provided path
fn write_config_string(config_path: PathBuf, content_string: &str) -> Result<()> {
    // Create a temporary file in the desired config directory
    // Check that a path was specified and isn't /
    let env_directory = config_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty() && *p != Path::new("/"))
        .ok_or_else(|| error::Error::NoParentDirectory {
            path: config_path.clone(),
        })?;

    let mut tempfile =
        NamedTempFile::new_in(env_directory).context(error::CreateTempFileSnafu {
            path: PathBuf::from(env_directory),
        })?;

    // Write the config to the temporary file
    if !content_string.is_empty() {
        writeln!(tempfile, "{content_string}").context(error::WriteTempFileSnafu)?;
    }

    // Construct the final path and atomically move the temporary file to it
    tempfile
        .persist(&config_path)
        .context(error::PersistTempFileSnafu {
            path: config_path.clone(),
        })?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    use gptman::{GPTPartitionEntry, GPT};
    use signpost::uuid_to_guid;
    use std::{env, io::Cursor};
    use tempfile::TempDir;

    fn test_data() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/tests")
    }

    fn gpt_data(partition_type: [u8; 16], partition_name: &str) -> Vec<u8> {
        let mut data = vec![0; 21 * 512 * 2048];
        let mut cursor = Cursor::new(&mut data);
        let mut gpt = GPT::new_from(&mut cursor, 512, [0xff; 16]).unwrap();
        gpt[1] = GPTPartitionEntry {
            partition_name: partition_name.into(),
            partition_type_guid: partition_type,
            unique_partition_guid: [0xff; 16],
            starting_lba: gpt.header.first_usable_lba,
            ending_lba: gpt.header.last_usable_lba,
            attribute_bits: 0,
        };
        gpt.write_into(&mut cursor).unwrap();
        cursor.into_inner().to_vec()
    }

    #[test]
    fn empty_disk() {
        let data = vec![0; 21 * 512 * 2048];
        assert_eq!(
            find_device_type(&mut Cursor::new(&data)).unwrap(),
            "ephemeral"
        );
    }

    #[test]
    fn partitioned_disk_with_unknown_type() {
        let partition_type = uuid_to_guid(hex!("00000000 0000 0000 0000 000000000000"));
        let partition_name = "";
        let data = gpt_data(partition_type, partition_name);
        assert_eq!(
            find_device_type(&mut Cursor::new(&data)).unwrap(),
            "ephemeral"
        );
    }

    #[test]
    fn partitioned_disk_with_system_type() {
        let partition_type = uuid_to_guid(hex!("440408bb eb0b 4328 a6e5 a29038fad706"));
        let partition_name = "";
        let data = gpt_data(partition_type, partition_name);
        assert_eq!(find_device_type(&mut Cursor::new(&data)).unwrap(), "system");
    }

    #[test]
    fn partitioned_disk_with_system_name() {
        let partition_type = uuid_to_guid(hex!("11111111 1111 1111 1111 111111111111"));
        let partition_name = "BOTTLEROCKET-STUFF";
        let data = gpt_data(partition_type, partition_name);
        assert_eq!(find_device_type(&mut Cursor::new(&data)).unwrap(), "system");
    }

    #[test]
    fn test_valid_device_info() {
        for device_name in ["xvdcz", "/dev/xvdcz"] {
            let device_info = build_device_info(device_name);
            assert_eq!(
                parse_device_name(&device_info, "".to_string()).unwrap(),
                "xvdcz".to_string()
            );
        }
    }

    fn build_device_info(device_name: &str) -> Vec<u8> {
        let mut device_name = device_name.as_bytes().to_vec();
        let mut device_info: Vec<u8> = vec![0; NVME_IDENTIFY_DATA_SIZE - 1024];
        let mut padding = vec![32; NVME_IDENTIFY_DATA_SIZE - device_info.len() - device_name.len()];
        device_info.append(&mut device_name);
        device_info.append(&mut padding);

        device_info
    }

    #[test]
    fn parse_open_gpu_supported_devices_file() {
        let test_json = test_data().join("open-gpu-supported-devices-test.json");

        let test_data = read_supported_devices_file(test_json).unwrap();

        match test_data {
            SupportedDevicesConfiguration::OpenGpu(data) => {
                assert!(data.len() == 6);
            }
        }
    }

    #[test]
    fn test_write_config_string_success() -> Result<()> {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.txt");
        let test_content = "test content";

        // Write content to file
        write_config_string(config_path.clone(), test_content)?;

        // Verify the content was written correctly
        let read_content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(read_content, format!("{test_content}\n"));

        Ok(())
    }

    #[test]
    fn test_write_config_string_no_parent_directory() {
        // Try to write to a path without a parent directory
        let result = write_config_string(PathBuf::from("file.txt"), "content");
        println!("{result:?}");
        assert!(matches!(
            result.unwrap_err(),
            error::Error::NoParentDirectory { path: _ }
        ));
        // Try to write to /
        let result = write_config_string(PathBuf::from("/file.txt"), "content");

        assert!(matches!(
            result.unwrap_err(),
            error::Error::NoParentDirectory { path: _ }
        ));
    }

    #[test]
    fn test_write_config_string_empty_content() -> Result<()> {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.txt");
        let empty_content = "";

        // Write empty content to file
        write_config_string(config_path.clone(), empty_content)?;

        // Verify the content was written correctly
        let read_content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(read_content, ""); // Note: writeln! adds a newline

        Ok(())
    }

    #[test]
    fn test_write_config_string_overwrite_existing() -> Result<()> {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.txt");

        // Write initial content
        let initial_content = "initial content";
        write_config_string(config_path.clone(), initial_content)?;

        // Write new content
        let new_content = "new content";
        write_config_string(config_path.clone(), new_content)?;

        // Verify the content was overwritten correctly
        let read_content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(read_content, format!("{new_content}\n"));

        Ok(())
    }
}
