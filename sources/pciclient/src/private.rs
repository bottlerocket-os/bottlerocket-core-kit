use std::{ffi::OsStr, process::Command};

use snafu::{ensure, OptionExt, ResultExt};

use crate::{
    error::{
        CommandFailureSnafu, ExecutionFailureSnafu, MissingRequiredFieldSnafu,
        ParseListDevicesOutputFailureSnafu,
    },
    ListDevicesOutput, ListDevicesParam, Result,
};

const AMAZON_VENDOR_CODE: &str = "1d0f";
const EFA_KEYWORD: &str = "efa";

const LSPCI_PATH: &str = "/usr/bin/lspci";

pub(crate) trait CommandExecutor {
    fn execute<I, S>(self, args: I) -> Result<Vec<String>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>;
}

pub(crate) struct PciClient;

impl CommandExecutor for PciClient {
    /// Call `lspci`, it takes in the command line arguments and return the result as vector of strings.
    fn execute<I, S>(self, args: I) -> Result<Vec<String>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let result = Command::new(LSPCI_PATH)
            .args(args)
            .output()
            .context(CommandFailureSnafu)?;
        ensure!(
            result.status.success(),
            ExecutionFailureSnafu {
                reason: String::from_utf8_lossy(&result.stderr),
            }
        );

        let output = String::from_utf8_lossy(&result.stdout)
            .lines()
            .map(str::to_string)
            .collect();
        Ok(output)
    }
}

/// Parsing the lspci output and convert it to [`ListDevicesOutput`].
///
/// As mentioned in [`list_devices`], the parsing logic is based on
/// the output format that is decorated by "-n -m".
///
/// Sample output:
///
/// ```sh
/// bash-5.1# lspci -n -m -d :efa0
/// 00:1d.0 "0200" "1d0f" "efa0" -p00 "1d0f" "efa0"
/// ```
/// Quoting from the manual of `lspci` (https://man7.org/linux/man-pages/man8/lspci.8.html):
/// Some of the arguments are positional: slot, class, vendor name, device name, subsystem vendor name
/// and subsystem name (the last two are empty if the device has no subsystem). the remaining arguments
/// are option-like:
///
///    -r<rev>  Revision number.
///
///    -p<progif>  Programming interface.
/// The relative order of positional arguments and options is undefined.
fn parse_list_devices_output(lspci_output: Vec<String>) -> Result<Vec<ListDevicesOutput>> {
    lspci_output
        .iter()
        .map(|line| -> Result<ListDevicesOutput> {
            let mut tokens = line.split_whitespace().peekable();
            let mut list_devices_output = ListDevicesOutput {
                pci_slot: unquote(tokens.next().context(MissingRequiredFieldSnafu {
                    required_field: "pci_slot",
                })?),
                class: unquote(tokens.next().context(MissingRequiredFieldSnafu {
                    required_field: "class",
                })?),
                vendor: unquote(tokens.next().context(MissingRequiredFieldSnafu {
                    required_field: "vendor",
                })?),
                device: unquote(tokens.next().context(MissingRequiredFieldSnafu {
                    required_field: "device",
                })?),
                ..Default::default()
            };
            // Parse the remaining optional fields.
            // Note that the s_vendor and s_device will be together and positional. They will be empty string if the
            // device has no subsystem.
            // The "-r" (revision) and "-p" (program interface) will be inserted before the subsystems optionally, we
            // need to account for that by checking if the current token starts with the `-r` or `-p` notation.
            while let Some(token) = tokens.next() {
                if token.starts_with("-r") {
                    list_devices_output.revision = Some(token.trim_start_matches("-r").to_string())
                } else if token.starts_with("-p") {
                    list_devices_output.program_interface =
                        Some(token.trim_start_matches("-p").to_string())
                } else {
                    // This is when subsystem is matched. We will try to parse both in the same iteration so that
                    // s_vendor and s_device is captured together.
                    if list_devices_output.subsystem_vendor.is_some() {
                        // We will ignore this token since we do not recognize them.
                        // According to the doc:
                        // > New options can be added in future versions, but they will always have a single argument not
                        // > separated from the option by any spaces, so they can be easily ignored if not recognized.
                        continue;
                    }
                    // Here the "filter" ensures empty string would be parsed as None.
                    list_devices_output.subsystem_vendor =
                        Some(unquote(token)).filter(|s| !s.is_empty());
                    let s_device_token =
                        tokens.next().context(ParseListDevicesOutputFailureSnafu {
                            output: line.clone(),
                            reason: "Incomplete subsystem vendor/device fields",
                        })?;
                    // Here the "filter" ensures empty string would be parsed as None.
                    list_devices_output.subsystem_device =
                        Some(unquote(s_device_token)).filter(|s| !s.is_empty());
                }
            }
            Ok(list_devices_output)
        })
        .collect()
}

/// Some of the token in the lspci output is quoted, this is a simple helper to unquote it.
/// Sample output: 00:1d.0 "0200" "1d0f" "efa0" -p00 "1d0f" "efa0"
fn unquote(s: &str) -> String {
    s.trim_matches('"').to_string()
}

/// Call `lspci` and check if there is any EFA device attached.
/// Internal usage, adding command_executor as parameter allows us to better unit test.
/// For external usage, check [`crate::is_efa_attached`].
pub(crate) fn check_efa_attachment<T: CommandExecutor>(command_executor: T) -> Result<bool> {
    let list_devices_param = ListDevicesParam::builder()
        .vendor(AMAZON_VENDOR_CODE.to_string())
        .build();
    let list_device_results = call_list_devices(command_executor, list_devices_param)?;
    Ok(list_device_results
        .iter()
        .any(|device_info| device_info.device.contains(EFA_KEYWORD)))
}

/// Internal usage, adding command_executor as parameter allows us to better unit test.
/// For external usage, check [`list_devices`].
pub(crate) fn call_list_devices<T: CommandExecutor>(
    command_executor: T,
    list_devices_param: ListDevicesParam,
) -> Result<Vec<ListDevicesOutput>> {
    let list_device_output = command_executor.execute(list_devices_param.into_command_args())?;
    parse_list_devices_output(list_device_output)
}

#[cfg(test)]
mod test {
    use std::ffi::{OsStr, OsString};

    use test_case::test_case;

    use crate::{ListDevicesOutput, ListDevicesParam, Result};

    use super::{
        call_list_devices, check_efa_attachment, parse_list_devices_output, CommandExecutor,
    };

    struct MockPciClient {
        output: Vec<String>,
    }

    impl CommandExecutor for MockPciClient {
        fn execute<I, S>(self, _: I) -> Result<Vec<String>>
        where
            I: IntoIterator<Item = S>,
            S: AsRef<OsStr>,
        {
            Ok(self.output)
        }
    }

    #[test_case(
        ListDevicesParam::default(),
        vec!["-n", "-m"]
    )]
    #[test_case(
        ListDevicesParam::builder().vendor("1d0f").build(),
        vec!["-n", "-m", "-d", "1d0f:::"]
    )]
    #[test_case(
        ListDevicesParam::builder().device("efa0").build(),
        vec!["-n", "-m", "-d", ":efa0::"]
    )]
    #[test_case(
        ListDevicesParam::builder().class("0200").build(),
        vec!["-n", "-m", "-d", "::0200:"]
    )]
    #[test_case(
        ListDevicesParam::builder().program_interface("00").build(),
        vec!["-n", "-m", "-d", ":::00"]
    )]
    #[test_case(
        ListDevicesParam::builder().vendor("1d0f").device("efa0").build(),
        vec!["-n", "-m", "-d", "1d0f:efa0::"]
    )]
    #[test_case(
        ListDevicesParam::builder().vendor("1d0f").program_interface("00").build(),
        vec!["-n", "-m", "-d", "1d0f:::00"]
    )]
    #[test_case(
        ListDevicesParam::builder().vendor("1d0f").class("0200").build(),
        vec!["-n", "-m", "-d", "1d0f::0200:"]
    )]
    #[test_case(
        ListDevicesParam::builder().vendor("1d0f")
            .device("efa0")
            .class("0200")
            .program_interface("01")
            .build(),
        vec!["-n", "-m", "-d", "1d0f:efa0:0200:01"]
    )]
    fn list_device_param_to_command_args_test(param: ListDevicesParam, args: Vec<&str>) {
        let actual_param = param.into_command_args();
        let expected_param: Vec<OsString> = args.into_iter().map(OsString::from).collect();
        assert_eq!(actual_param, expected_param);
    }

    #[test]
    fn test_parse_list_devices_output() {
        let raw_lspci_output = r#"00:00.0 "0600" "8086" "1237" -p00 "1d0f" "1237"
        00:01.0 "0601" "8086" "7000" -p00 "" ""
        00:1e.0 "0302" "10de" "1eb8" -ra1 -p00 "10de" "12a2""#;
        let lspci_output = raw_lspci_output.lines().map(str::to_string).collect();
        let list_devices_output_result = parse_list_devices_output(lspci_output);
        assert!(list_devices_output_result.is_ok());
        let list_devices_output = list_devices_output_result.unwrap();
        assert_eq!(list_devices_output.len(), 3); // Three items in the list_devices_output
        assert_eq!(
            list_devices_output[0],
            ListDevicesOutput {
                pci_slot: "00:00.0".to_string(),
                class: "0600".to_string(),
                vendor: "8086".to_string(),
                device: "1237".to_string(),
                program_interface: Some("00".to_string()),
                subsystem_vendor: Some("1d0f".to_string()),
                subsystem_device: Some("1237".to_string()),
                ..Default::default()
            }
        );
        assert_eq!(
            list_devices_output[1],
            ListDevicesOutput {
                pci_slot: "00:01.0".to_string(),
                class: "0601".to_string(),
                vendor: "8086".to_string(),
                device: "7000".to_string(),
                program_interface: Some("00".to_string()),
                ..Default::default()
            }
        );
        assert_eq!(
            list_devices_output[2],
            ListDevicesOutput {
                pci_slot: "00:1e.0".to_string(),
                class: "0302".to_string(),
                vendor: "10de".to_string(),
                device: "1eb8".to_string(),
                program_interface: Some("00".to_string()),
                revision: Some("a1".to_string()),
                subsystem_vendor: Some("10de".to_string()),
                subsystem_device: Some("12a2".to_string()),
            }
        );
    }

    #[test]
    fn test_list_devices() {
        let mock_pci_client = MockPciClient {
            output: vec![r#"00:00.0 "0600" "8086" "1237" -p00 "1d0f" "1237""#.to_string()],
        };
        let list_devices_param = ListDevicesParam::builder()
            .vendor("8086")
            .device("1237")
            .class("0600")
            .program_interface("00")
            .build();
        let list_devices_result = call_list_devices(mock_pci_client, list_devices_param);
        assert!(list_devices_result.is_ok());
        assert_eq!(
            list_devices_result.unwrap(),
            vec![ListDevicesOutput {
                pci_slot: "00:00.0".to_string(),
                class: "0600".to_string(),
                vendor: "8086".to_string(),
                device: "1237".to_string(),
                program_interface: Some("00".to_string()),
                subsystem_vendor: Some("1d0f".to_string()),
                subsystem_device: Some("1237".to_string()),
                ..Default::default()
            }]
        )
    }

    #[test]
    fn test_is_efa_attached() {
        let mock_pci_client = MockPciClient {
            // EFA device has class code: 0200 for ethernet controller, vendor 1d0f for amazon, device code efa0.
            // Below is an actual output from lspci for efa device.
            output: vec![
                r#"00:1d.0 "0200" "1d0f" "efa0" -p00 "1d0f" "efa0""#.to_string(),
                r#"00:1e.0 "0302" "10de" "1eb8" -ra1 -p00 "10de" "12a2""#.to_string(),
            ],
        };
        let check_efa_attachment_result = check_efa_attachment(mock_pci_client);
        assert!(check_efa_attachment_result.is_ok());
        assert_eq!(check_efa_attachment_result.unwrap(), true);
    }

    #[test]
    fn test_is_efa_attached_negative_case() {
        let mock_pci_client = MockPciClient {
            // Below is an actual output from lspci for ena device (not efa).
            output: vec![r#"00:06.0 "0200" "1d0f" "ec20" -p00 "1d0f" "ec20""#.to_string()],
        };
        let check_efa_attachment_result = check_efa_attachment(mock_pci_client);
        assert!(check_efa_attachment_result.is_ok());
        assert_eq!(check_efa_attachment_result.unwrap(), false);
    }
}
