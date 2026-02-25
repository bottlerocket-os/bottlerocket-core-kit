//! Input configuration types and deserialization.

use serde::Deserialize;
use snafu::ResultExt;
use std::fs;

use crate::error::{ParseSettingsSnafu, ReadSettingsSnafu, Result};

/// Container registry settings
#[derive(Deserialize)]
pub(crate) struct Settings {
    pub(crate) mirrors: Option<Vec<Mirror>>,
    pub(crate) credentials: Option<Vec<Credential>>,
}

/// Registry mirror configuration
#[derive(Deserialize)]
pub(crate) struct Mirror {
    pub(crate) registry: String,
    pub(crate) endpoint: Vec<String>,
}

/// Registry authentication credentials
#[derive(Deserialize)]
pub(crate) struct Credential {
    pub(crate) registry: String,
    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,
    pub(crate) auth: Option<String>,
    pub(crate) identitytoken: Option<String>,
}

/// Read and parse settings from TOML file
pub(crate) fn read_settings(path: &str) -> Result<Settings> {
    let contents = fs::read_to_string(path).context(ReadSettingsSnafu { path })?;
    toml::from_str(&contents).context(ParseSettingsSnafu { path })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use test_case::test_case;

    // Settings schema tests
    #[test_case(r#"[[mirrors]]
 registry = "docker.io"
 endpoint = ["https://mirror.example.com"]"#, true, false; "mirrors only")]
    #[test_case(r#"[[credentials]]
 registry = "r.io"
 username = "u"
 password = "p""#, false, true; "credentials only")]
    fn test_settings_schema(toml_str: &str, has_mirrors: bool, has_creds: bool) {
        let settings: Settings = toml::from_str(toml_str).unwrap();
        assert_eq!(settings.mirrors.is_some(), has_mirrors);
        assert_eq!(settings.credentials.is_some(), has_creds);
    }

    #[test]
    fn test_read_settings_file_not_found() {
        let result = read_settings("/nonexistent/path/to/file.toml");
        assert!(matches!(result, Err(Error::ReadSettings { .. })));
    }

    #[test]
    fn test_read_settings_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.toml");
        std::fs::write(&path, "this is not valid toml [[[").unwrap();
        let result = read_settings(path.to_str().unwrap());
        assert!(matches!(result, Err(Error::ParseSettings { .. })));
    }
}
