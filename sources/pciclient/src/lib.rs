/*!
`pciclient` provides high-level methods to invoke `lspci` and query for attached devices information.

pciclient provides util functions that can:
- List the devices attached with some filtering option.
- Detect specifically if EFA device is attached.
*/
mod private;

use private::{call_list_devices, check_efa_attachment, check_neuron_attachment, PciClient};

use bon::Builder;
use derive_getters::Getters;
use std::ffi::OsString;

/// This [`ListDevicesParam`] is based on the "-d" flag for `lspci`, which
/// allows selection/filtering of the output. Spec for "-d":
/// ```sh
/// -d [<vendor>]:[<device>][:<class>[:<prog-if>]]
///
/// Show only devices with specified vendor, device, class ID,
/// and programming interface.  The ID's are given in
/// hexadecimal and may be omitted or given as "*", both
/// meaning "any value". The class ID can contain "x"
/// characters which stand for "any digit".
/// ```
#[derive(Debug, Default, PartialEq, Builder)]
pub struct ListDevicesParam {
    #[builder(into)]
    vendor: Option<String>,
    #[builder(into)]
    device: Option<String>,
    #[builder(into)]
    class: Option<String>,
    #[builder(into)]
    program_interface: Option<String>,
}

impl ListDevicesParam {
    /// Convert the ListDevicesParam into proper command line arguments so that
    /// we can feed it to the lspci() function where it makes the actual call to the
    /// lspci binary.
    ///
    /// As mentioned in [`list_devices`], we will use "-n" and "-m" to decorate the output
    /// so that it returns machine-readable format which ensures compatibility and numeric output
    /// for the vendor code and device codes.
    fn into_command_args(self) -> Vec<OsString> {
        let mut args: Vec<String> = vec!["-n".into(), "-m".into()];
        if self != ListDevicesParam::default() {
            let parts: Vec<String> =
                vec![self.vendor, self.device, self.class, self.program_interface]
                    .into_iter()
                    .map(|part| part.unwrap_or_default())
                    .collect();
            let additional_args = parts.join(":");
            args.push("-d".to_string());
            args.push(additional_args);
        }
        args.into_iter().map(OsString::from).collect()
    }
}

/// The [`ListDevicesOutput`] is based on the output format that is decorated by "-n -m".
#[derive(Debug, Default, PartialEq, Eq, Getters)]
pub struct ListDevicesOutput {
    pci_slot: String,
    class: String,
    vendor: String,
    device: String,
    subsystem_vendor: Option<String>,
    subsystem_device: Option<String>,
    revision: Option<String>,
    program_interface: Option<String>,
}

/// Query the list of the devices with options to filter the output.
/// # Input
/// list_devices_param: [`ListDevicesParam`]. We will under the hood convert the [`ListDevicesParam`]
/// to the low level arguments to `lspci`.
/// According to https://man7.org/linux/man-pages/man8/lspci.8.html#MACHINE_READABLE_OUTPUT,
/// > If you intend to process the output of lspci automatically,
/// > please use one of the machine-readable output formats (-m, -vm, -vmm)
/// > described in this section. All other formats are likely to change between versions of lspci.
///
/// * We will use `-m` to get the machine-readable format which ensures compatibility.
/// * We will use `-n` to get the numeric output for the vendor code and device codes.
/// * We will optionally insert `-d <selection_expression>` to filter the output.
///
/// # Output
/// Output is [`Result<Vec<ListDevicesOutput>>`], the [`ListDevicesOutput`] is based on
/// the output format that is decorated by "-n -m".
///
/// # Example
/// ```rust,ignore
/// use pciclient::{ListDevicesParam, list_devices};
///
/// let list_devices_param = ListDevicesParam::builder().vendor("1d0f").build();
/// match list_devices(list_devices_param) {
///     Ok(list_devices_output) => {
///         for device_info in list_devices_output {
///             println!("vendor: {}, class: {}, device: {}", device_info.vendor(), device_info.class(), device_info.device());
///         }
///     },
///     Err(_) => {
///         println!("Failed to list devices");
///     }
/// }
/// ```
pub fn list_devices(list_devices_param: ListDevicesParam) -> Result<Vec<ListDevicesOutput>> {
    call_list_devices(PciClient {}, list_devices_param)
}

/// Call `lspci` and check if there is any EFA device attached.
pub fn is_efa_attached() -> Result<bool> {
    check_efa_attachment(PciClient {})
}

/// Call `lspci` and check if there is any Neuron device attached.
pub fn is_neuron_attached() -> Result<bool> {
    check_neuron_attachment(PciClient {})
}

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]

    pub enum PciClientError {
        #[snafu(display("Deserialization error: {}", source))]
        Serde { source: serde_json::Error },

        #[snafu(display("Command lspci failed: {}", source))]
        CommandFailure { source: std::io::Error },

        #[snafu(display("Exection of lspci failed: {}", reason))]
        ExecutionFailure { reason: String },

        #[snafu(display("Failed to parse the lspci output: {}, reason: {}", output, reason))]
        ParseListDevicesOutputFailure { output: String, reason: String },

        #[snafu(display(
            "Failed to parse the lspci output, missing required field: {}",
            required_field
        ))]
        MissingRequiredField { required_field: String },
    }
}

pub use error::PciClientError;
pub type Result<T> = std::result::Result<T, PciClientError>;
