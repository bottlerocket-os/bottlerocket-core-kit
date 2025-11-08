//! This module implements network configuration API calls.
//! It supports sourcing `net.toml` configuration files from the filesystem or base64 encoded strings.

use base64::{engine, Engine};
use snafu::{OptionExt, ResultExt};
use std::path::Path;
use tokio::io::AsyncReadExt;
use url::Url;

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
/// Supports file:// and base64: URI schemes. If no input source is provided, reads from stdin.
pub async fn get_content<S>(input_source: Option<S>) -> Result<String>
where
    S: Into<String>,
{
    match input_source {
        Some(source) => get_content_from_source(source.into()).await,
        None => get_content_with_stdin(tokio::io::stdin()).await,
    }
}

/// Reads all content from an async reader into a string.
///
/// Generic reader interface allows flexible input sources and testing with mock data.
async fn get_content_with_stdin<R>(mut reader: R) -> Result<String>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut output = String::new();
    reader
        .read_to_string(&mut output)
        .await
        .context(error::StdinReadSnafu)?;
    Ok(output)
}

/// Retrieves network configuration content from a source URI.
///
/// Supports file:// and base64: URI schemes.
async fn get_content_from_source(input_source: String) -> Result<String> {
    if let Some(base64_data) = input_source.strip_prefix("base64:") {
        return get_content_from_base64(base64_data, &input_source);
    }

    let uri = Url::parse(&input_source).context(error::UriSnafu {
        input_source: &input_source,
    })?;

    match uri.scheme() {
        "file" => get_content_from_file(uri, &input_source).await,
        scheme => error::UnsupportedUriSchemeSnafu {
            input_source: &input_source,
            scheme,
        }
        .fail(),
    }
}

/// Decodes and returns content from a base64-encoded string.
fn get_content_from_base64(base64_data: &str, input_source: &str) -> Result<String> {
    let decoded_bytes = engine::general_purpose::STANDARD
        .decode(base64_data.as_bytes())
        .context(error::Base64DecodeSnafu { input_source })?;

    String::from_utf8(decoded_bytes).context(error::Base64Utf8Snafu { input_source })
}

/// Reads content from a file URI.
async fn get_content_from_file(uri: Url, input_source: &str) -> Result<String> {
    let path = uri
        .to_file_path()
        .ok()
        .context(error::FileUriSnafu { input_source })?;
    tokio::fs::read_to_string(path)
        .await
        .context(error::FileReadSnafu { input_source })
}

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub enum Error {
        #[snafu(display("Failed to decode base64 from '{}': {}", input_source, source))]
        Base64Decode {
            input_source: String,
            source: base64::DecodeError,
        },

        #[snafu(display(
            "Base64 content from '{}' is not valid UTF-8: {}",
            input_source,
            source
        ))]
        Base64Utf8 {
            input_source: String,
            source: std::string::FromUtf8Error,
        },

        #[snafu(display("Failed to {} network configuration to '{}': {}", method, uri, source))]
        ConfigureRequest {
            uri: String,
            method: String,
            #[snafu(source(from(crate::Error, Box::new)))]
            source: Box<crate::Error>,
        },

        #[snafu(display("Failed to read given file '{}': {}", input_source, source))]
        FileRead {
            input_source: String,
            source: std::io::Error,
        },

        #[snafu(display("Invalid file URI '{}'", input_source))]
        FileUri { input_source: String },

        #[snafu(display("Failed to read from stdin: {}", source))]
        StdinRead { source: std::io::Error },

        #[snafu(display(
            "Unsupported URI scheme '{}' in '{}'. Only file:// and base64: schemes are supported",
            scheme,
            input_source
        ))]
        UnsupportedUriScheme {
            input_source: String,
            scheme: String,
        },

        #[snafu(display("Invalid URI '{}': {}", input_source, source))]
        Uri {
            input_source: String,
            source: url::ParseError,
        },
    }
}

pub use error::Error;
pub type Result<T> = std::result::Result<T, error::Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine, Engine};
    use test_case::test_case;

    #[tokio::test]
    async fn test_get_content_stdin() {
        use std::io::Cursor;

        // Given network config content to simulate stdin input
        let test_content = r"version = 2

[eth0]
dhcp4 = true
primary = true
";
        let mock_stdin = tokio::io::BufReader::new(Cursor::new(test_content.as_bytes()));

        // When reading from mock stdin
        let result = get_content_with_stdin(mock_stdin).await;

        // Then content should be read successfully
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_content);
    }

    #[tokio::test]
    async fn test_get_content_stdin_empty() {
        use std::io::Cursor;

        // Given empty stdin
        let mock_stdin = tokio::io::BufReader::new(Cursor::new(b""));

        // When reading from empty mock stdin
        let result = get_content_with_stdin(mock_stdin).await;

        // Then should return empty string successfully
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[tokio::test]
    async fn test_stdin_vs_uri_behavior() {
        use std::io::Cursor;

        let content = r"version = 2

[eth0]
dhcp4 = true";
        let mock_stdin = tokio::io::BufReader::new(Cursor::new(content.as_bytes()));

        // Test that stdin reader works
        let stdin_result = get_content_with_stdin(mock_stdin).await;

        // Test that Some input uses URI processing
        let uri_result =
            get_content(Some("base64:dmVyc2lvbiA9IDIKCltldGgwXQpkaGNwNCA9IHRydWU=")).await;

        assert!(stdin_result.is_ok());
        assert!(uri_result.is_ok());
        assert_eq!(stdin_result.unwrap(), content);
        assert_eq!(uri_result.unwrap(), content);
    }

    #[tokio::test]
    async fn test_get_content_base64() {
        // Given a valid TOML network configuration encoded in base64
        let test_content = "version = 2\n\n[eth0]\ndhcp4 = true\n";
        let encoded = engine::general_purpose::STANDARD.encode(test_content.as_bytes());
        let base64_uri = format!("base64:{}", encoded);

        // Then get content from the base64 URI
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
        assert!(result.unwrap_err().to_string().contains("not valid UTF-8"));
    }

    #[tokio::test]
    async fn test_uri_parsing() {
        // Given an invalid URI string that cannot be parsed
        // When attempting to get content from the malformed URI
        let result = get_content(Some("invalid-uri")).await;

        // Then the operation should fail with a URI parsing error
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid URI"));
    }

    #[test_case("http://example.com/net.toml", "http"; "http uri rejected")]
    #[test_case("https://example.com/net.toml", "https"; "https uri rejected")]
    #[test_case("s3://bucket/net.toml", "s3"; "s3 uri rejected")]
    #[test_case("ftp://ftp.example.com/net.toml", "ftp"; "ftp uri rejected")]
    #[test_case("data:text/plain;charset=utf-8,version=2", "data"; "data uri rejected")]
    #[tokio::test]
    async fn test_unsupported_uri_schemes_rejected(uri: &str, expected_scheme: &str) {
        // When attempting to get content from an unsupported URI scheme
        let result = get_content(Some(uri)).await;

        // Then the operation should fail with an unsupported URI scheme error
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains(&format!("Unsupported URI scheme '{expected_scheme}'")));
        assert!(error_msg.contains("Only file:// and base64: schemes are supported"));
    }
}
