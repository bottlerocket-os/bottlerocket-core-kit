use sha2::{Digest, Sha256, Sha384, Sha512};
use snafu::prelude::*;
use std::fs;
use zeroize::Zeroizing;

use crate::system;

type Result<T> = std::result::Result<T, snafu::Whatever>;

/// Path to kernel command line
const PROC_CMDLINE: &str = "/proc/cmdline";

/// PCR for OS settings measurements
const PCR_SETTINGS: u32 = 8;

/// PCR for kernel command line measurements
const PCR_KERNEL_COMMAND_LINE: u32 = 9;

/// PCR for boot phase measurements
const PCR_PHASE: u32 = 11;

/// Domain-separation prefix for a present (possibly empty) EC2 IMDS user-data payload.
///
/// The digest input is this prefix concatenated with the raw user-data bytes exactly as
/// returned by IMDS with no decompression, decoding, or trimming applied.
const USER_DATA_PRESENT_PREFIX: &[u8] = b"ec2-imds-user-data:v1:";

/// Domain-separation marker for an absent EC2 IMDS user-data payload.
///
/// This value is measured verbatim (it is not combined with any additional bytes), and is
/// deliberately distinct from `USER_DATA_PRESENT_PREFIX` with an empty payload so a verifier
/// can always distinguish "user data present but zero-length" from "user data absent".
const USER_DATA_ABSENT_MARKER: &[u8] = b"ec2-imds-user-data:v1:absent";

/// Measure OS settings into PCR 8
pub fn os_settings() -> Result<()> {
    let data = system::apiclient_get_settings()?;
    extend_pcr(PCR_SETTINGS, &data)
}

/// Measure EC2 IMDS user data into PCR 8.
///
/// This extends PCR 8 exactly once, regardless of whether user data is present, empty, or
/// absent. The three cases are domain-separated so a verifier can tell them apart:
///
/// - Present: `USER_DATA_PRESENT_PREFIX` followed by the raw bytes.
/// - Present but empty: `USER_DATA_PRESENT_PREFIX` followed by zero
///   bytes. This naturally yields a digest distinct from the absent marker below, since the
///   prefix bytes alone differ from `USER_DATA_ABSENT_MARKER`'s bytes.
/// - Absent: `USER_DATA_ABSENT_MARKER`.
pub async fn user_data() -> Result<()> {
    let user_data = system::fetch_imds_userdata().await?;
    let digest_input = frame_user_data(user_data.as_deref().map(|v| v.as_slice()));
    extend_pcr(PCR_SETTINGS, &digest_input)
}

/// Build the domain-separated digest input for a `user_data()` measurement.
fn frame_user_data(user_data: Option<&[u8]>) -> Zeroizing<Vec<u8>> {
    match user_data {
        Some(raw) => {
            let mut framed = Zeroizing::new(Vec::with_capacity(
                USER_DATA_PRESENT_PREFIX.len() + raw.len(),
            ));
            framed.extend_from_slice(USER_DATA_PRESENT_PREFIX);
            framed.extend_from_slice(raw);
            framed
        }
        None => Zeroizing::new(USER_DATA_ABSENT_MARKER.to_vec()),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn present_nonempty_uses_prefix_and_bytes() {
        let framed = frame_user_data(Some(b"#!/bin/bash\necho hi\n"));
        let mut expected = USER_DATA_PRESENT_PREFIX.to_vec();
        expected.extend_from_slice(b"#!/bin/bash\necho hi\n");
        assert_eq!(framed.as_slice(), expected.as_slice());
    }

    #[test]
    fn present_empty_is_just_the_prefix() {
        let framed = frame_user_data(Some(b""));
        assert_eq!(framed.as_slice(), USER_DATA_PRESENT_PREFIX);
    }

    #[test]
    fn absent_uses_the_absent_marker() {
        let framed = frame_user_data(None);
        assert_eq!(framed.as_slice(), USER_DATA_ABSENT_MARKER);
    }

    #[test]
    fn present_empty_and_absent_are_distinct() {
        let present_empty = frame_user_data(Some(b""));
        let absent = frame_user_data(None);
        assert_ne!(present_empty.as_slice(), absent.as_slice());
    }

    #[test]
    fn present_empty_and_absent_digests_are_distinct() {
        let present_empty = frame_user_data(Some(b""));
        let absent = frame_user_data(None);
        assert_ne!(
            Sha256::digest(present_empty.as_slice()),
            Sha256::digest(absent.as_slice())
        );
    }

    #[test]
    fn raw_bytes_are_not_altered_by_framing() {
        // Framing must not decompress, decode, or trim the payload - verify a gzip-looking
        // binary blob and bytes with leading/trailing whitespace survive untouched.
        let raw: &[u8] = &[0x1f, 0x8b, 0x08, 0x00, b' ', b'\n', b' '];
        let framed = frame_user_data(Some(raw));
        assert!(framed.ends_with(raw));
        assert_eq!(&framed[USER_DATA_PRESENT_PREFIX.len()..], raw);
    }

    #[test]
    fn different_present_payloads_yield_different_digests() {
        let a = frame_user_data(Some(b"payload-a"));
        let b = frame_user_data(Some(b"payload-b"));
        assert_ne!(Sha256::digest(a.as_slice()), Sha256::digest(b.as_slice()));
    }
}
