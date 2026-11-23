//! The EC2 IMDS user-data input to the PCR 8 measurement.
//!
//! User data is parsed the same way early-boot-config reads it (optional gzip, then TOML, as plain
//! tables and values) and the whole document is measured as canonical JSON with the region removed
//! and registries stripped from container image sources (`v2` framing). User data that is not TOML, or that contains floats
//! (canonical JSON has no float encoding) or datetimes (never a valid Bottlerocket setting), is
//! measured as its raw bytes (`v1` framing) as before. See `docs/ENCRYPTED_STORAGE.md`.

use crate::image_source;
use crate::secret_json::SecretValue;
use early_boot_config_provider::compression::expand_slice_maybe;
use snafu::prelude::*;
use zeroize::Zeroizing;

type Result<T> = std::result::Result<T, snafu::Whatever>;

/// Domain-separation prefix for a present (possibly empty) raw EC2 IMDS user-data payload: the
/// bytes exactly as returned by IMDS with no decompression, decoding, or trimming applied.
pub const RAW_PREFIX: &[u8] = b"ec2-imds-user-data:v1:";

/// Domain-separation marker for an absent EC2 IMDS user-data payload, measured verbatim and
/// deliberately distinct from `RAW_PREFIX` with an empty payload.
pub const ABSENT_MARKER: &[u8] = b"ec2-imds-user-data:v1:absent";

/// Domain-separation prefix for user data measured as canonical settings JSON with the registry
/// removed from container image sources.
pub const CANONICAL_PREFIX: &[u8] = b"ec2-imds-user-data:v2:";

/// Build the PCR 8 digest input for the user data IMDS returned (`None` when IMDS has none).
pub fn measurement_input(user_data: Option<&[u8]>) -> Zeroizing<Vec<u8>> {
    let Some(raw) = user_data else {
        return Zeroizing::new(ABSENT_MARKER.to_vec());
    };
    match canonical_settings(raw) {
        Ok(canonical) => framed(CANONICAL_PREFIX, &canonical),
        // Not TOML settings (or not representable canonically): measure the raw bytes as before.
        // stderr is captured by systemd for the unit, so say so without a global logger.
        Err(e) => {
            eprintln!("measuring user data as raw bytes: {e}");
            framed(RAW_PREFIX, raw)
        }
    }
}

fn framed(prefix: &[u8], payload: &[u8]) -> Zeroizing<Vec<u8>> {
    let mut out = Zeroizing::new(Vec::with_capacity(prefix.len() + payload.len()));
    out.extend_from_slice(prefix);
    out.extend_from_slice(payload);
    out
}

/// Decode user data as TOML and return its canonical JSON with registries stripped from sources.
///
/// The decoded text and the parsed tree are zeroized when dropped (see [`crate::secret_json`]);
/// the TOML values are moved, not copied, into the JSON tree.
fn canonical_settings(raw: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    let text = Zeroizing::new(
        expand_slice_maybe(raw).with_whatever_context(|_| "user data is not UTF-8 text")?,
    );
    let value: toml::Value =
        toml::from_str(&text).with_whatever_context(|_| "user data is not TOML")?;
    image_source::measurement_bytes(SecretValue::new(toml_to_json(value)?))
}

/// Convert parsed TOML to JSON. Floats are rejected because canonical JSON cannot encode them;
/// datetimes because no Bottlerocket setting is a datetime (early-boot-config would not apply it).
fn toml_to_json(value: toml::Value) -> Result<serde_json::Value> {
    Ok(match value {
        toml::Value::String(s) => serde_json::Value::String(s),
        toml::Value::Integer(i) => serde_json::Value::from(i),
        toml::Value::Boolean(b) => serde_json::Value::Bool(b),
        toml::Value::Float(_) => {
            whatever!("user data contains a float; canonical JSON has no float encoding")
        }
        toml::Value::Datetime(_) => {
            whatever!("user data contains a datetime, which is not a valid Bottlerocket setting")
        }
        toml::Value::Array(items) => serde_json::Value::Array(
            items
                .into_iter()
                .map(toml_to_json)
                .collect::<Result<Vec<_>>>()?,
        ),
        toml::Value::Table(table) => serde_json::Value::Object(
            table
                .into_iter()
                .map(|(k, v)| toml_to_json(v).map(|v| (k, v)))
                .collect::<Result<serde_json::Map<_, _>>>()?,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    const DIGEST: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn user_data(region: &str, account: &str) -> String {
        format!(
            r#"
# comments and spacing are irrelevant once parsed
[settings.host-containers.admin]
enabled = true
source   = "328549459982.dkr.ecr.{region}.amazonaws.com/bottlerocket-admin:v0.21.6"

[settings.bootstrap-containers.agent]
source = "{account}.dkr.ecr.{region}.amazonaws.com/bootstrap@{DIGEST}"
mode = "always"
essential = true

[settings.motd]
"#
        )
        .replace("[settings.motd]\n", "")
    }

    fn gzip(data: &[u8]) -> Vec<u8> {
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(data).unwrap();
        enc.finish().unwrap()
    }

    fn canonical_payload(input: &[u8]) -> String {
        assert!(
            input.starts_with(CANONICAL_PREFIX),
            "not v2 framing: {:?}",
            &input[..24.min(input.len())]
        );
        String::from_utf8(input[CANONICAL_PREFIX.len()..].to_vec()).unwrap()
    }

    #[test]
    fn absent_uses_the_absent_marker() {
        assert_eq!(measurement_input(None).as_slice(), ABSENT_MARKER);
    }

    #[test]
    fn empty_user_data_is_valid_toml_and_measures_as_an_empty_table() {
        assert_eq!(
            measurement_input(Some(b"")).as_slice(),
            b"ec2-imds-user-data:v2:{}\n"
        );
        assert_ne!(
            measurement_input(Some(b"")).as_slice(),
            measurement_input(None).as_slice()
        );
    }

    #[test]
    fn non_toml_is_measured_raw() {
        for raw in [
            &b"#!/bin/bash\necho hi\n"[..],
            &[0x1f, 0x8b, 0x08, 0x00, 0xff, 0xfe][..],
            b"not = toml = at all",
        ] {
            let framed = measurement_input(Some(raw));
            assert!(framed.starts_with(RAW_PREFIX));
            assert_eq!(&framed[RAW_PREFIX.len()..], raw);
        }
    }

    #[test]
    fn gzip_of_non_toml_is_measured_raw_as_the_compressed_bytes() {
        let compressed = gzip(b"#!/bin/bash\necho hi\n");
        let framed = measurement_input(Some(&compressed));
        assert!(framed.starts_with(RAW_PREFIX));
        assert_eq!(&framed[RAW_PREFIX.len()..], compressed.as_slice());
    }

    #[test]
    fn floats_and_datetimes_fall_back_to_raw() {
        for raw in [
            "[settings.custom]\nratio = 1.5\n",
            "[settings.custom]\nwhen = 1979-05-27T07:32:00Z\n",
        ] {
            assert!(
                measurement_input(Some(raw.as_bytes())).starts_with(RAW_PREFIX),
                "{raw}"
            );
        }
    }

    #[test]
    fn registries_are_removed_from_both_container_tables() {
        let payload = canonical_payload(&measurement_input(Some(
            user_data("us-east-1", "111122223333").as_bytes(),
        )));
        assert_eq!(
            payload,
            format!(
                concat!(
                    r#"{{"settings":{{"bootstrap-containers":{{"agent":{{"essential":true,"mode":"always","source":"bootstrap@{digest}"}}}},"#,
                    r#""host-containers":{{"admin":{{"enabled":true,"source":"bottlerocket-admin:v0.21.6"}}}}}}}}"#,
                    "\n"
                ),
                digest = DIGEST
            )
        );
    }

    #[test]
    fn user_data_from_different_regions_and_accounts_measures_identically() {
        let a = measurement_input(Some(user_data("us-east-1", "111122223333").as_bytes()));
        let b = measurement_input(Some(user_data("eu-west-1", "444455556666").as_bytes()));
        assert_eq!(a.as_slice(), b.as_slice());
    }

    #[test]
    fn formatting_does_not_change_the_measurement() {
        let a = user_data("us-east-1", "111122223333");
        let b = a
            .replace("source   =", "source=")
            .replace("# comments and spacing are irrelevant once parsed\n", "");
        assert_ne!(a, b);
        assert_eq!(
            measurement_input(Some(a.as_bytes())).as_slice(),
            measurement_input(Some(b.as_bytes())).as_slice()
        );
    }

    #[test]
    fn other_settings_are_measured_as_written() {
        let raw = "[settings.aws]
profile = \"default\"
";
        let payload = canonical_payload(&measurement_input(Some(raw.as_bytes())));
        assert_eq!(
            payload,
            "{\"settings\":{\"aws\":{\"profile\":\"default\"}}}
"
        );
    }

    #[test]
    fn region_is_dropped_from_user_data_but_seed_and_hostname_are_kept() {
        let raw = "[settings.aws]
region = \"us-east-1\"
[settings.updates]
seed = 7
[settings.network]
hostname = \"h\"
";
        let payload = canonical_payload(&measurement_input(Some(raw.as_bytes())));
        assert_eq!(
            payload,
            "{\"settings\":{\"network\":{\"hostname\":\"h\"},\"updates\":{\"seed\":7}}}
"
        );
    }

    #[test]
    fn gzip_compressed_user_data_is_decoded_like_early_boot_config() {
        let raw = user_data("us-east-1", "111122223333");
        assert_eq!(
            measurement_input(Some(raw.as_bytes())).as_slice(),
            measurement_input(Some(&gzip(raw.as_bytes()))).as_slice()
        );
    }
}
