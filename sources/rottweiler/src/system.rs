use serde::Deserialize;
use settings_extension_measurement::MeasurementSettingsV1;
use snafu::prelude::*;
use std::io::Write;
use std::process::{Command, Stdio};
use zeroize::Zeroizing;

type Result<T> = std::result::Result<T, snafu::Whatever>;

const SYSTEMD_CREDS: &str = "/usr/bin/systemd-creds";
const SYSTEMD_CRYPTSETUP: &str = "/usr/lib/systemd/systemd-cryptsetup";
const CRYPTSETUP: &str = "/usr/sbin/cryptsetup";
const APICLIENT: &str = "/usr/bin/apiclient";
const TPM2_PCREXTEND: &str = "/usr/bin/tpm2_pcrextend";

/// Encrypt data using systemd-creds with TPM2 PCRs
pub fn systemd_creds_encrypt(name: &str, plaintext: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    let pcrs = format!("--tpm2-pcrs={}", get_tpm2_pcrs()?);
    execute(
        SYSTEMD_CREDS,
        &[
            "encrypt",
            "-",
            "-",
            "--name",
            name,
            "--with-key=tpm2",
            &pcrs,
        ],
        Some(plaintext),
    )
}

/// Decrypt data using systemd-creds with TPM2
pub fn systemd_creds_decrypt(name: &str, ciphertext: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    execute(
        SYSTEMD_CREDS,
        &["decrypt", "-", "-", "--name", name],
        Some(ciphertext),
    )
}

/// Attach a LUKS device using systemd-cryptsetup with the provided key
pub fn systemd_cryptsetup_attach(
    volume_name: &str,
    source_device: &str,
    key_data: &[u8],
) -> Result<()> {
    execute(
        SYSTEMD_CRYPTSETUP,
        &[
            "attach",
            volume_name,
            source_device,
            "/dev/stdin",
            "luks,headless=yes,tries=1",
        ],
        Some(key_data),
    )?;
    Ok(())
}

/// Detach a LUKS device using systemd-cryptsetup
pub fn systemd_cryptsetup_detach(volume_name: &str) -> Result<()> {
    execute(SYSTEMD_CRYPTSETUP, &["detach", volume_name], None)?;
    Ok(())
}

/// Format a device with LUKS2 using the provided key
pub fn cryptsetup_luks_format(device: &str, key_data: &[u8]) -> Result<()> {
    // Use minimal PBKDF2 iterations (1000, per NIST SP 800-132) since we're using a high-entropy
    // TPM2-sealed key. This matches systemd's behavior and avoids unnecessary key stretching.
    execute(
        CRYPTSETUP,
        &[
            "luksFormat",
            "--type",
            "luks2",
            "--pbkdf",
            "pbkdf2",
            "--pbkdf-force-iterations",
            "1000",
            "--batch-mode",
            device,
            "-",
        ],
        Some(key_data),
    )?;
    Ok(())
}

/// Resize a LUKS device using the provided key
pub fn cryptsetup_resize(volume_name: &str, key_data: &[u8]) -> Result<()> {
    execute(
        CRYPTSETUP,
        &["resize", "--key-file=-", volume_name],
        Some(key_data),
    )?;
    Ok(())
}

/// Execute a command with optional stdin input and return stdout
fn execute(cmd: &str, args: &[&str], input: Option<&[u8]>) -> Result<Zeroizing<Vec<u8>>> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_whatever_context(|_| format!("failed to spawn {}", cmd))?;

    if let Some(data) = input {
        let mut stdin = child
            .stdin
            .take()
            .with_whatever_context(|| "failed to open stdin")?;

        stdin
            .write_all(data)
            .with_whatever_context(|_| "failed to write to stdin")?;

        drop(stdin);
    }

    let output = child
        .wait_with_output()
        .with_whatever_context(|_| format!("failed to wait for {}", cmd))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        whatever!(
            "{} failed with exit code {}: {}",
            cmd,
            output.status.code().unwrap_or(-1),
            stderr.trim()
        );
    }

    Ok(Zeroizing::new(output.stdout))
}

/// Get TPM2 PCRs to bind to based on image features
///
/// PCR decoder ring:
/// - PCR 4: hashes of shim, grub, and vmlinuz
/// - PCR 7: Secure Boot policy
/// - PCR 9: kernel command line (includes dm-verity root hash of userspace)
/// - PCR 11: boot phase (sysinit, preconfigured, configured, ready, shutdown, final)
/// - PCR 14: machine-owner keys
///
/// If in-place updates are disabled, then the measurements in PCR 4 (kernel) and
/// PCR 9 (userspace) shouldn't change.
fn get_tpm2_pcrs() -> Result<String> {
    let features = bottlerocket_image_features::parse_image_features()?;
    Ok(if features.in_place_updates {
        "7+11+14".to_string()
    } else {
        "4+7+9+11+14".to_string()
    })
}

/// Wrapper structs for deserializing the apiclient verbose JSON response.
/// apiclient returns: `{"settings":{"measurement":{"excluded-settings":[...]}}}`
#[derive(Deserialize)]
struct ApiSettingsResponse {
    settings: ApiSettingsMeasurement,
}

#[derive(Deserialize)]
struct ApiSettingsMeasurement {
    measurement: MeasurementSettingsV1,
}

/// Get the user-configured excluded settings from the API.
/// Returns an empty list if the setting is not configured.
fn get_extra_excluded_settings() -> Result<Vec<String>> {
    let output = execute(
        APICLIENT,
        &["get", "settings.measurement.excluded-settings"],
        None,
    )?;

    let text = String::from_utf8_lossy(&output);
    let response: ApiSettingsResponse =
        serde_json::from_str(text.trim()).with_whatever_context(|e| {
            format!("failed to parse measurement exclusion settings: {e}")
        })?;

    let excluded = match response.settings.measurement.excluded_settings {
        Some(values) => values,
        None => return Ok(Vec::new()),
    };

    // Normalize values by stripping a leading "settings." prefix if present.
    // Users may provide either "host-containers" or "settings.host-containers";
    // we need bare names for constructing the apiclient `--exclude` arguments.
    Ok(excluded
        .iter()
        .map(|v| {
            v.trim()
                .strip_prefix("settings.")
                .unwrap_or(v.trim())
                .to_string()
        })
        .collect())
}

/// Get canonicalized settings from apiclient, excluding seed, hostname, and
/// any additional settings the customer has configured via
/// `settings.measurement.excluded-settings`.
pub fn apiclient_get_settings() -> Result<Zeroizing<Vec<u8>>> {
    let extra_exclusions = get_extra_excluded_settings()?;

    let mut args: Vec<String> = vec![
        "get".into(),
        "settings".into(),
        "--exclude".into(),
        "settings.updates.seed".into(),
        "--exclude".into(),
        "settings.network.hostname".into(),
    ];

    args.extend(
        extra_exclusions
            .iter()
            .flat_map(|s| ["--exclude".to_string(), format!("settings.{}", s)]),
    );

    args.push("--canonicalize".into());

    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    execute(APICLIENT, &arg_refs, None)
}

/// Extend a TPM PCR with SHA256, SHA384, and SHA512 hashes
pub fn tpm2_pcrextend(pcr: u32, sha256: &str, sha384: &str, sha512: &str) -> Result<()> {
    execute(
        TPM2_PCREXTEND,
        &[&format!(
            "{}:sha256={},sha384={},sha512={}",
            pcr, sha256, sha384, sha512
        )],
        None,
    )?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    // === API response deserialization ===

    #[test]
    fn parse_nested_json_response() {
        // Simulate what apiclient actually returns
        let json = r#"{"settings":{"measurement":{"excluded-settings":["host-containers","bootstrap-containers"]}}}"#;
        let response: ApiSettingsResponse = serde_json::from_str(json).unwrap();
        let excluded = response.settings.measurement.excluded_settings.unwrap();
        assert_eq!(excluded, vec!["host-containers", "bootstrap-containers"]);
    }

    #[test]
    fn parse_empty_excluded_settings() {
        let json = r#"{"settings":{"measurement":{"excluded-settings":[]}}}"#;
        let response: ApiSettingsResponse = serde_json::from_str(json).unwrap();
        let excluded = response.settings.measurement.excluded_settings.unwrap();
        assert!(excluded.is_empty());
    }

    #[test]
    fn parse_missing_excluded_settings_field() {
        // When the excluded-settings key is absent, it deserializes as None
        let json = r#"{"settings":{"measurement":{}}}"#;
        let response: ApiSettingsResponse = serde_json::from_str(json).unwrap();
        assert!(response.settings.measurement.excluded_settings.is_none());
    }

    #[test]
    fn parse_malformed_json_fails() {
        let json = r#"not json at all"#;
        let result = serde_json::from_str::<ApiSettingsResponse>(json);
        assert!(result.is_err());
    }

    #[test]
    fn parse_wrong_structure_fails() {
        // Missing "settings" wrapper
        let json = r#"{"measurement":{"excluded-settings":["host-containers"]}}"#;
        let result = serde_json::from_str::<ApiSettingsResponse>(json);
        assert!(result.is_err());
    }

    // === Normalization for apiclient args ===

    #[test]
    fn normalize_strips_settings_prefix() {
        let value = "settings.host-containers";
        let normalized = value
            .trim()
            .strip_prefix("settings.")
            .unwrap_or(value.trim());
        assert_eq!(normalized, "host-containers");
    }

    #[test]
    fn normalize_leaves_bare_value_unchanged() {
        let value = "host-containers";
        let normalized = value
            .trim()
            .strip_prefix("settings.")
            .unwrap_or(value.trim());
        assert_eq!(normalized, "host-containers");
    }
}
