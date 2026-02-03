/*!
*thar-be-image-verifiers* writes per-plugin trust policy files from TOML config.

Reads TOML from `/etc/thar-be-image-verifiers.toml` with plugin configs, decodes
base64 trustpolicies, and writes atomically to
`/etc/containerd/image-verifiers/<plugin>/trustpolicy.json`.
*/

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use log::{info, warn};

const IMAGE_VERIFIERS_DIR: &str = "image-verifiers";
const TRUSTPOLICY_FILE: &str = "trustpolicy.json";
use snafu::{ResultExt, Snafu};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;
use typed_path::Utf8UnixPath;
use walkdir::WalkDir;

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("failed to read config file: {source}"))]
    ReadConfig { source: std::io::Error },
    #[snafu(display("failed to parse TOML: {source}"))]
    ParseToml { source: toml::de::Error },
    #[snafu(display("failed to create temp directory: {source}"))]
    CreateTempDir { source: std::io::Error },
    #[snafu(display("failed to create dir for {plugin}: {source}"))]
    CreatePluginDir {
        plugin: String,
        source: std::io::Error,
    },
    #[snafu(display("failed to decode base64 for {plugin}: {source}"))]
    DecodeBase64 {
        plugin: String,
        source: base64::DecodeError,
    },
    #[snafu(display("failed to write config for {plugin}: {source}"))]
    WriteConfig {
        plugin: String,
        source: std::io::Error,
    },
    #[snafu(display("failed to persist config for {plugin}: {source}"))]
    PersistConfig {
        plugin: String,
        source: tempfile::PersistError,
    },
    #[snafu(display("invalid plugin path '{plugin}': {source}"))]
    InvalidPluginPath {
        plugin: String,
        source: typed_path::CheckedPathError,
    },
}

type Result<T> = std::result::Result<T, Error>;

/// Configuration for a single image verifier plugin.
#[derive(serde::Deserialize)]
struct PluginConfig {
    trustpolicy: String,
}

/// Top-level configuration containing all plugin configs.
#[derive(serde::Deserialize, Default)]
struct Config {
    #[serde(default)]
    plugins: HashMap<String, PluginConfig>,
}

#[snafu::report]
fn main() -> Result<()> {
    image_verifiers::logging::init();
    info!("Reading image verifiers config from /etc/thar-be-image-verifiers.toml");
    let input = fs::read_to_string("/etc/thar-be-image-verifiers.toml").context(ReadConfigSnafu)?;
    let config: Config = toml::from_str(&input).context(ParseTomlSnafu)?;
    info!("Writing plugin configs to /etc/containerd");
    write_plugin_configs(&config.plugins, "/etc/containerd")?;
    info!("Successfully wrote plugin configs");
    Ok(())
}

/// Writes decoded trust policy files for each configured plugin.
/// Each trustpolicy.json is written atomically via write-to-temp + rename.
fn write_plugin_configs(config: &HashMap<String, PluginConfig>, base_path: &str) -> Result<()> {
    let verifiers_dir = Utf8UnixPath::new(base_path).join(IMAGE_VERIFIERS_DIR);

    for (plugin, cfg) in config {
        // Validate plugin name to prevent path traversal.
        let plugin_dir =
            verifiers_dir
                .join_checked(plugin)
                .map_err(|e| Error::InvalidPluginPath {
                    plugin: plugin.clone(),
                    source: e,
                })?;
        let plugin_dir: &Path = plugin_dir.as_ref();

        fs::create_dir_all(plugin_dir).context(CreatePluginDirSnafu {
            plugin: plugin.clone(),
        })?;

        let decoded = STANDARD
            .decode(&cfg.trustpolicy)
            .context(DecodeBase64Snafu {
                plugin: plugin.clone(),
            })?;

        // Write to temp file in same directory, then atomic rename.
        let policy_file = plugin_dir.join(TRUSTPOLICY_FILE);
        let tmp_file = NamedTempFile::new_in(plugin_dir).context(CreateTempDirSnafu)?;
        fs::write(tmp_file.path(), &decoded).context(WriteConfigSnafu {
            plugin: plugin.clone(),
        })?;
        tmp_file.persist(&policy_file).context(PersistConfigSnafu {
            plugin: plugin.clone(),
        })?;
    }

    // Remove plugins that are no longer configured.
    for entry in WalkDir::new(&verifiers_dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .flatten()
    {
        let name_str = entry.file_name().to_string_lossy();
        if !config.contains_key::<str>(&name_str) {
            let f = entry.path().join(TRUSTPOLICY_FILE);
            if f.exists() {
                if let Err(e) = fs::remove_file(f) {
                    warn!("Failed to remove stale policy for {}: {}", name_str, e);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test]
    fn test_parse_empty_config() {
        let input = "[plugins]";
        let config: std::result::Result<Config, _> = toml::from_str(input);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().plugins.len(), 0);
    }

    #[test]
    fn test_parse_single_plugin() {
        let input = r#"[plugins.notation]
trustpolicy = "dGVzdA==""#;
        let config: Config = toml::from_str(input).unwrap();
        assert_eq!(config.plugins.len(), 1);
        assert!(config.plugins.contains_key("notation"));
        assert_eq!(config.plugins["notation"].trustpolicy, "dGVzdA==");
    }

    #[test]
    fn test_parse_multiple_plugins() {
        let input = r#"[plugins.notation]
trustpolicy = "YWJj"

[plugins.digestion]
trustpolicy = "ZGVm"

[plugins.custom]
trustpolicy = "Z2hp""#;
        let config: Config = toml::from_str(input).unwrap();
        assert_eq!(config.plugins.len(), 3);
        assert!(config.plugins.contains_key("notation"));
        assert!(config.plugins.contains_key("digestion"));
        assert!(config.plugins.contains_key("custom"));
    }

    #[test_case("dGVzdA==", b"test" ; "simple")]
    #[test_case("YWJjZGVm", b"abcdef" ; "longer")]
    #[test_case("e30=", b"{}" ; "json_object")]
    fn test_base64_decode(encoded: &str, expected: &[u8]) {
        let decoded = STANDARD.decode(encoded).unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_write_plugin_configs() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let mut config = HashMap::new();
        config.insert(
            "notation".to_string(),
            PluginConfig {
                trustpolicy: "dGVzdA==".to_string(),
            },
        );
        write_plugin_configs(&config, base.to_str().unwrap()).unwrap();
        let policy_file = base.join("image-verifiers/notation/trustpolicy.json");
        assert!(policy_file.exists());
        let content = fs::read(&policy_file).unwrap();
        assert_eq!(content, b"test");
    }

    #[test]
    fn test_write_multiple_plugins() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let mut config = HashMap::new();
        config.insert(
            "notation".to_string(),
            PluginConfig {
                trustpolicy: "YWJj".to_string(),
            },
        );
        config.insert(
            "digestion".to_string(),
            PluginConfig {
                trustpolicy: "ZGVm".to_string(),
            },
        );
        write_plugin_configs(&config, base.to_str().unwrap()).unwrap();
        let notation_file = base.join("image-verifiers/notation/trustpolicy.json");
        let digestion_file = base.join("image-verifiers/digestion/trustpolicy.json");
        assert!(notation_file.exists());
        assert!(digestion_file.exists());
        assert_eq!(fs::read(&notation_file).unwrap(), b"abc");
        assert_eq!(fs::read(&digestion_file).unwrap(), b"def");
    }

    #[test]
    fn test_empty_config_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let config = HashMap::new();
        write_plugin_configs(&config, base.to_str().unwrap()).unwrap();
    }

    #[test]
    fn test_cleanup_removed_plugin() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let mut config = HashMap::new();
        let notation_plugin = "notation_policy";
        let digestion_plugin = "digestion_policy";
        let notation_data = "YWJj";
        let digestion_data = "ZGVm";
        config.insert(
            notation_plugin.to_string(),
            PluginConfig {
                trustpolicy: notation_data.to_string(),
            },
        );
        config.insert(
            digestion_plugin.to_string(),
            PluginConfig {
                trustpolicy: digestion_data.to_string(),
            },
        );
        write_plugin_configs(&config, base.to_str().unwrap()).unwrap();
        let notation_file = base.join(format!(
            "image-verifiers/{}/trustpolicy.json",
            notation_plugin
        ));
        let digestion_file = base.join(format!(
            "image-verifiers/{}/trustpolicy.json",
            digestion_plugin
        ));
        assert!(notation_file.exists());
        assert!(digestion_file.exists());

        config.remove(digestion_plugin);
        write_plugin_configs(&config, base.to_str().unwrap()).unwrap();
        assert!(!digestion_file.exists());
        assert!(notation_file.exists());
    }
}
