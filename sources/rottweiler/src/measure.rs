use sha2::{Digest, Sha256, Sha384, Sha512};
use snafu::prelude::*;
use std::fs;

use crate::{image_source, system, user_data_measure};

type Result<T> = std::result::Result<T, snafu::Whatever>;

/// Path to kernel command line
const PROC_CMDLINE: &str = "/proc/cmdline";

/// PCR for OS settings measurements
const PCR_SETTINGS: u32 = 8;

/// PCR for kernel command line measurements
const PCR_KERNEL_COMMAND_LINE: u32 = 9;

/// PCR for boot phase measurements
const PCR_PHASE: u32 = 11;

/// Measure OS settings into PCR 8.
///
/// The measured bytes are the OLPC canonical JSON of `settings` (minus `settings.updates.seed`
/// and `settings.network.hostname`) with the registry removed from every container image source
/// (see [`image_source`]), followed by a newline.
pub fn os_settings() -> Result<()> {
    let settings = system::apiclient_get_settings()?;
    let data = image_source::measurement_bytes(settings)?;
    extend_pcr(PCR_SETTINGS, &data)
}

/// Measure EC2 IMDS user data into PCR 8.
///
/// This extends PCR 8 exactly once, regardless of whether user data is present, empty, or
/// absent. See [`user_data_measure`] for the framing.
pub async fn user_data() -> Result<()> {
    let user_data = system::fetch_imds_userdata().await?;
    let digest_input =
        user_data_measure::measurement_input(user_data.as_deref().map(|v| v.as_slice()));
    extend_pcr(PCR_SETTINGS, &digest_input)
}

/// Measure kernel command line into PCR 9
pub fn kernel_command_line() -> Result<()> {
    let data = fs::read_to_string(PROC_CMDLINE)
        .with_whatever_context(|_| format!("failed to read {}", PROC_CMDLINE))?;
    extend_pcr(PCR_KERNEL_COMMAND_LINE, data.as_bytes())
}

/// Measure boot phase into PCR 11
pub fn pcrphase(phase: &str) -> Result<()> {
    extend_pcr(PCR_PHASE, phase.as_bytes())
}

/// Compute SHA256/384/512 hashes and extend PCR
fn extend_pcr(pcr: u32, data: &[u8]) -> Result<()> {
    let sha256 = hex::encode(Sha256::digest(data));
    let sha384 = hex::encode(Sha384::digest(data));
    let sha512 = hex::encode(Sha512::digest(data));
    system::tpm2_pcrextend(pcr, &sha256, &sha384, &sha512)
}
