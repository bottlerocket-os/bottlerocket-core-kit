//! This module implements network configuration API calls.
//! It supports sourcing `net.toml` configuration files from any URI scheme supported by apiclient.

use crate::uri_resolver::{select_resolver, SettingsInput};
use snafu::ResultExt;
use std::path::Path;

/// Configures network settings by sending the provided content to the API server.
///
/// The configuration will be applied at the next boot - a reboot is required for changes to take effect.
pub async fn configure<P>(socket_path: P, content: String) -> Result<()>
where
    P: AsRef<Path>,
{
    let uri = "/actions/network/configure";
    let method = "POST";
    let (_status, _body) = crate::raw_request(&socket_path, uri, method, Some(content))
        .await
        .context(error::ConfigureRequestSnafu { uri, method })?;

    Ok(())
}

/// Retrieves the network configuration content from the given source URI.
///
/// Supports all URI schemes: stdin ("-"), file://, base64:, http://, https://,
/// s3://, ssm://, secretsmanager://, and ARN formats.
pub async fn get_content<S>(input_source: Option<S>) -> Result<String>
where
    S: Into<String>,
{
    let input = match input_source {
        Some(s) => s.into(),
        None => "-".to_string(),
    };

    let settings = SettingsInput::new(&input);
    let resolver = select_resolver(&settings).context(error::SelectResolverSnafu)?;
    resolver
        .resolve()
        .await
        .context(error::ResolverFailureSnafu)
}

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub enum Error {
        #[snafu(display("Failed to {} network configuration to '{}': {}", method, uri, source))]
        ConfigureRequest {
            uri: String,
            method: String,
            #[snafu(source(from(crate::Error, Box::new)))]
            source: Box<crate::Error>,
        },

        #[snafu(display("Failed to select resolver: {}", source))]
        SelectResolver {
            #[snafu(source(from(crate::uri_resolver::ResolverError, Box::new)))]
            source: Box<crate::uri_resolver::ResolverError>,
        },

        #[snafu(display("Resolver failed: {}", source))]
        ResolverFailure {
            #[snafu(source(from(crate::uri_resolver::ResolverError, Box::new)))]
            source: Box<crate::uri_resolver::ResolverError>,
        },
    }
}

pub use error::Error;
pub type Result<T> = std::result::Result<T, error::Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine, Engine};
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_get_content_none_defaults_to_stdin_path() {
        // Given None as input source
        // When select_resolver is called internally, it should receive "-"
        let settings = crate::uri_resolver::SettingsInput::new("-");
        let resolver = crate::uri_resolver::select_resolver(&settings);

        // Then a resolver should be found (StdinUri)
        assert!(resolver.is_ok(), "None input should map to stdin resolver");
    }

    #[tokio::test]
    async fn test_get_content_file_uri() {
        // Given a temporary file with network config content
        let mut tmp = NamedTempFile::new().expect("should create temp file");
        let test_content = "version = 2

[eth0]
dhcp4 = true
";
        write!(tmp, "{}", test_content).expect("should write to temp file");
        let file_uri = format!("file://{}", tmp.path().display());

        // When getting content from the file URI
        let result = get_content(Some(file_uri)).await;

        // Then the file contents should be returned
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_content);
    }

    #[tokio::test]
    async fn test_get_content_unsupported_scheme() {
        // Given an unsupported URI scheme
        // When attempting to get content
        let result = get_content(Some("ftp://example.com/net.toml")).await;

        // Then the operation should fail with a resolver selection error
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No URI resolver found"));
    }

    #[tokio::test]
    async fn test_get_content_base64() {
        // Given a valid TOML network configuration encoded in base64
        let test_content = "version = 2\n\n[eth0]\ndhcp4 = true\n";
        let encoded = engine::general_purpose::STANDARD.encode(test_content.as_bytes());
        let base64_uri = format!("base64:{}", encoded);

        // When getting content from the base64 URI
        let result = get_content(Some(base64_uri)).await;

        // Then the content should be successfully decoded
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_content);
    }

    #[tokio::test]
    async fn test_get_content_base64_invalid() {
        // Given an invalid base64 string that cannot be decoded
        // When attempting to get content from the malformed base64 URI
        let result = get_content(Some("base64:invalid!@#$")).await;

        // Then the operation should fail with a base64 decode error
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to decode base64"));
    }

    #[tokio::test]
    async fn test_get_content_base64_invalid_utf8() {
        // Given a valid base64 string that decodes to invalid UTF-8 bytes
        let invalid_bytes = vec![0xFF, 0xFE, 0xFD];
        let encoded = engine::general_purpose::STANDARD.encode(&invalid_bytes);
        let base64_uri = format!("base64:{}", encoded);

        // When attempting to get content from the base64 URI
        let result = get_content(Some(base64_uri)).await;

        // Then the operation should fail with a UTF-8 validation error
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to decode as UTF-8"));
    }
}
