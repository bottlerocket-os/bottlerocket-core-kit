use crate::cli::{error, Result};
use argh::FromArgs;
use snafu::ResultExt;
use std::io::Read;
use std::path::Path;
use tempfile::NamedTempFile;

/// Validate and commit network configuration from stdin
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "commit")]
pub struct CommitArgs {}

pub fn run(_args: CommitArgs) -> Result<()> {
    let mut content = String::new();
    std::io::stdin()
        .read_to_string(&mut content)
        .context(error::StdinReadSnafu)?;

    crate::net_config::deserialize_config(&content).context(error::NetConfigStdinParseSnafu)?;

    let config_dir = Path::new("/.bottlerocket");
    let config_file = config_dir.join("net.toml");

    if !config_dir.exists() {
        return error::NetConfigDirMissingSnafu { path: config_dir }.fail();
    }

    let tempfile = NamedTempFile::new_in(config_dir).context(error::CreateTempFileSnafu {
        path: config_dir.to_path_buf(),
    })?;

    std::fs::write(tempfile.path(), &content).context(error::WriteTempFileSnafu)?;

    // Ensure configuration changes are atomic to prevent partial writes
    tempfile
        .persist(&config_file)
        .context(error::PersistTempFileSnafu { path: config_file })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_commit_invalid_version() {
        let invalid_config = "version = 99\n\n[eth0]\ndhcp4 = true";
        let mut stdin = Cursor::new(invalid_config.as_bytes());

        let mut content = String::new();
        stdin.read_to_string(&mut content).unwrap();

        let result = crate::net_config::deserialize_config(&content);

        assert!(result.is_err());
    }

    #[test]
    fn test_commit_valid_config() {
        let valid_config = "version = 3\n\n[enp0s16]\ndhcp4 = true\ndhcp6 = false\nprimary = true\n\n[enp0s17]\ndhcp4 = true\ndhcp6 = false";
        let mut stdin = Cursor::new(valid_config.as_bytes());

        let mut content = String::new();
        stdin.read_to_string(&mut content).unwrap();

        let result = crate::net_config::deserialize_config(&content);

        assert!(result.is_ok());
    }
}
