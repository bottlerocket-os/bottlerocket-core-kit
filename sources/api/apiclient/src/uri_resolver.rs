//! Defines `UriResolver`, an async trait for fetching UTF-8 text from various URI schemes.
//! Concrete types parse and resolve a single scheme via `TryFrom` + `resolve()`:
//!  - `StdinUri` for "-", `Base64Uri` for `base64:`, `FileUri` for `file://`, `HttpUri` for `http://`
//!  - With TLS: `HttpsUri` for `https://`, `S3Uri`, `SecretsManagerUri`, `SecretsManagerArn`, `SsmUri`, `SsmArn`
//!
//! To add a new scheme, implement its `TryFrom` and `UriResolver::resolve()`.
use async_trait::async_trait;
use base64::{engine, Engine as _};
use reqwest::Url;
use resolver_error::*;
use snafu::{ensure, OptionExt, ResultExt, Snafu};
use std::any::Any;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::AsyncReadExt;

/// Maximum allowed object size for remote URI resolvers (2 MiB).
/// Applies to: http://, https://, s3://, ssm://, secretsmanager://, and ARN forms.
/// Matches actix-web's default JSON payload limit in apiserver.
pub const MAX_SIZE_BYTES: u64 = 2 * 1024 * 1024;

/// Timeout for establishing an HTTP/HTTPS connection.
pub const CONNECT_TIMEOUT_SECS: u64 = 10;
/// Maximum time for an entire request for all remote resolvers.
pub const OPERATION_TIMEOUT_SECS: u64 = 30;

/// Reads HTTP response body with a size limit, streaming chunks to avoid unbounded allocation.
/// Checks Content-Length header first for early rejection, then streams with hard limit.
pub async fn read_bounded_response(
    mut resp: reqwest::Response,
    uri: &str,
) -> Result<Vec<u8>, ResolverError> {
    if let Some(content_length) = resp.content_length() {
        if content_length > MAX_SIZE_BYTES {
            return HttpObjectTooLargeSnafu {
                size: content_length,
                max_size: MAX_SIZE_BYTES,
                uri,
            }
            .fail();
        }
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = resp.chunk().await.context(HttpBodySnafu { uri })? {
        let new_len = bytes.len() + chunk.len();
        let new_size: u64 = new_len
            .try_into()
            .context(ResponseSizeOverflowSnafu { size: new_len })?;
        if new_size > MAX_SIZE_BYTES {
            return HttpObjectTooLargeSnafu {
                size: new_size,
                max_size: MAX_SIZE_BYTES,
                uri,
            }
            .fail();
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// Shared HTTP/HTTPS resolution logic.
pub async fn resolve_http_url(url: &Url) -> ResolverResult<String> {
    let uri_str = url.to_string();
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .timeout(Duration::from_secs(OPERATION_TIMEOUT_SECS))
        .build()
        .context(resolver_error::HttpClientBuildSnafu)?;
    let resp = client
        .get(url.clone())
        .send()
        .await
        .context(resolver_error::HttpRequestSnafu { uri: &uri_str })?;
    ensure!(
        resp.status().is_success(),
        resolver_error::HttpStatusSnafu {
            uri: &uri_str,
            status: resp.status(),
        }
    );
    let bytes = read_bounded_response(resp, &uri_str).await?;
    String::from_utf8(bytes).context(resolver_error::Utf8DecodeSnafu { uri: uri_str })
}

#[cfg(feature = "tls")]
pub use crate::tls_resolvers::{
    HttpsUri, S3Uri, SecretsManagerArn, SecretsManagerUri, SsmArn, SsmUri,
};

pub struct SettingsInput {
    pub input: String,
    pub parsed_url: Option<Url>,
}

impl SettingsInput {
    pub fn new(input: impl Into<String>) -> Self {
        let input = input.into();
        let parsed_url = match Url::parse(&input) {
            Ok(url) => Some(url),
            Err(err) => {
                log::debug!("URL parse failed for '{}': {}", input, err);
                None
            }
        };
        SettingsInput { input, parsed_url }
    }
}

macro_rules! try_resolvers {
    ($input:expr, $($resolver_type:ty),+ $(,)?) => {
        $(
            if let Ok(r) = <$resolver_type>::try_from($input) {
                log::debug!("select_resolver: picked {}", stringify!($resolver_type));
                return Ok(Box::new(r));
            }
        )+
    };
}

pub fn select_resolver(input: &SettingsInput) -> ResolverResult<Box<dyn UriResolver>> {
    try_resolvers!(input, StdinUri, Base64Uri, FileUri, HttpUri,);

    #[cfg(feature = "tls")]
    try_resolvers!(
        input,
        HttpsUri,
        S3Uri,
        SecretsManagerArn,
        SecretsManagerUri,
        SsmArn,
        SsmUri,
    );

    resolver_error::NoResolverSnafu {
        input_source: input.input.clone(),
    }
    .fail()
}

#[async_trait]
pub trait UriResolver: Any {
    async fn resolve(&self) -> ResolverResult<String>;
}

pub struct StdinUri;

impl TryFrom<&SettingsInput> for StdinUri {
    type Error = ();
    fn try_from(input: &SettingsInput) -> std::result::Result<Self, Self::Error> {
        if input.input == "-" {
            Ok(StdinUri)
        } else {
            Err(())
        }
    }
}

/// Reads all content from an async reader into a String.
/// Extracted for testability (stdin cannot be mocked in unit tests).
async fn read_from_async<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut R,
) -> ResolverResult<String> {
    let mut buf = String::new();
    reader
        .read_to_string(&mut buf)
        .await
        .context(resolver_error::StdinReadSnafu)?;
    Ok(buf)
}

#[async_trait]
impl UriResolver for StdinUri {
    async fn resolve(&self) -> ResolverResult<String> {
        read_from_async(&mut tokio::io::stdin()).await
    }
}

pub struct Base64Uri {
    encoded_data: String,
}

impl TryFrom<&SettingsInput> for Base64Uri {
    type Error = ResolverError;
    fn try_from(input: &SettingsInput) -> ResolverResult<Self> {
        const PREFIX: &str = "base64:";
        let encoded_data = input.input.strip_prefix(PREFIX).context(Base64UriSnafu {
            input_source: input.input.clone(),
        })?;
        ensure!(
            !encoded_data.is_empty(),
            Base64UriSnafu {
                input_source: input.input.clone()
            }
        );
        Ok(Base64Uri {
            encoded_data: encoded_data.to_string(),
        })
    }
}

#[async_trait]
impl UriResolver for Base64Uri {
    async fn resolve(&self) -> ResolverResult<String> {
        let bytes = engine::general_purpose::STANDARD
            .decode(&self.encoded_data)
            .context(Base64DecodeSnafu {
                input_source: format!("base64:{}", self.encoded_data),
            })?;
        String::from_utf8(bytes).context(Utf8DecodeSnafu {
            uri: format!("base64:{}", self.encoded_data),
        })
    }
}

pub struct FileUri {
    path: PathBuf,
}

impl TryFrom<&SettingsInput> for FileUri {
    type Error = ResolverError;
    fn try_from(input: &SettingsInput) -> ResolverResult<Self> {
        let url = input.parsed_url.clone().context(FileUriSnafu {
            input_source: input.input.clone(),
        })?;
        ensure!(
            url.scheme() == "file",
            FileUriSnafu {
                input_source: url.to_string()
            }
        );
        let path = url.to_file_path().ok().context(FileUriSnafu {
            input_source: url.to_string(),
        })?;
        Ok(FileUri { path })
    }
}

#[async_trait]
impl UriResolver for FileUri {
    async fn resolve(&self) -> ResolverResult<String> {
        tokio::fs::read_to_string(&self.path)
            .await
            .context(FileReadSnafu {
                input_source: self.path.to_string_lossy().into_owned(),
            })
    }
}

pub struct HttpUri {
    url: Url,
}

impl TryFrom<&SettingsInput> for HttpUri {
    type Error = ResolverError;
    fn try_from(input: &SettingsInput) -> ResolverResult<Self> {
        let url = input.parsed_url.clone().context(InvalidHttpUriSnafu {
            input_source: input.input.clone(),
        })?;
        ensure!(
            url.scheme() == "http",
            InvalidHttpUriSnafu {
                input_source: url.to_string()
            }
        );
        Ok(HttpUri { url })
    }
}

#[async_trait]
impl UriResolver for HttpUri {
    async fn resolve(&self) -> ResolverResult<String> {
        resolve_http_url(&self.url).await
    }
}

#[derive(Debug, Snafu)]
#[snafu(module, visibility(pub(crate)))]
pub enum ResolverError {
    #[snafu(display("No URI resolver found for '{}'", input_source))]
    NoResolver { input_source: String },

    #[snafu(display("Invalid ARN '{}': {}", input_source, reason))]
    InvalidArnFormat {
        input_source: String,
        reason: String,
    },

    #[snafu(display("Invalid base64 URI scheme for '{}', expected base64:", input_source))]
    Base64Uri { input_source: String },

    #[snafu(display("Failed to decode base64 data from '{}': {}", input_source, source))]
    Base64Decode {
        input_source: String,
        source: base64::DecodeError,
    },

    #[snafu(display("Failed to read standard input: {}", source))]
    StdinRead { source: std::io::Error },

    #[snafu(display("Given invalid file URI '{}'", input_source))]
    FileUri { input_source: String },

    #[snafu(display("Failed to read given file '{}': {}", input_source, source))]
    FileRead {
        input_source: String,
        source: std::io::Error,
    },

    #[snafu(display("Given invalid http:// URI '{}'", input_source))]
    InvalidHttpUri { input_source: String },

    #[snafu(display(
        "HTTP object at {uri} is too large ({size} bytes, maximum is {max_size} bytes)"
    ))]
    HttpObjectTooLarge {
        size: u64,
        max_size: u64,
        uri: String,
    },

    #[snafu(display("Response size {size} exceeds u64 range"))]
    ResponseSizeOverflow {
        size: usize,
        source: std::num::TryFromIntError,
    },

    #[snafu(display("Failed to perform HTTP GET to '{}': {}", uri, source))]
    HttpRequest { uri: String, source: reqwest::Error },

    #[snafu(display("Non-success HTTP status from '{}': {}", uri, status))]
    HttpStatus {
        uri: String,
        status: reqwest::StatusCode,
    },

    #[snafu(display("Failed to read HTTP response body from '{}': {}", uri, source))]
    HttpBody { uri: String, source: reqwest::Error },

    #[snafu(display("Failed to build HTTP client: {}", source))]
    HttpClientBuild { source: reqwest::Error },

    #[snafu(display("Failed to decode as UTF-8 for {uri}"))]
    Utf8Decode {
        source: std::string::FromUtf8Error,
        uri: String,
    },

    #[cfg(feature = "tls")]
    #[snafu(display("TLS resolver error: {}", source))]
    TlsResolver {
        #[snafu(source(from(crate::tls_resolvers::TlsResolverError, Box::new)))]
        source: Box<crate::tls_resolvers::TlsResolverError>,
    },
}

pub type ResolverResult<T> = std::result::Result<T, ResolverError>;

#[cfg(feature = "tls")]
impl From<crate::tls_resolvers::TlsResolverError> for ResolverError {
    fn from(e: crate::tls_resolvers::TlsResolverError) -> Self {
        ResolverError::TlsResolver {
            source: Box::new(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Base64Uri, FileUri, UriResolver};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    #[tokio::test(flavor = "multi_thread")]
    async fn file_uri_reads_file_content() -> Result<(), Box<dyn std::error::Error>> {
        // Given a temporary file with known content
        let mut tmp = NamedTempFile::new()?;
        write!(tmp, "test, tempfile!")?;
        let path: PathBuf = tmp.path().into();
        let file_uri = FileUri { path: path.clone() };

        // When resolving the file URI
        let result = file_uri.resolve().await?;

        // Then the file contents should be returned
        assert_eq!(result, "test, tempfile!");
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn base64_uri_decodes_valid_data() -> Result<(), Box<dyn std::error::Error>> {
        // Given a base64-encoded string ("Hello World")
        let base64_uri = Base64Uri {
            encoded_data: "SGVsbG8gV29ybGQ=".to_string(),
        };

        // When resolving the base64 URI
        let result = base64_uri.resolve().await?;

        // Then the decoded string should be returned
        assert_eq!(result, "Hello World");
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn base64_uri_fails_on_invalid_base64() {
        // Given an invalid base64 string
        let base64_uri = Base64Uri {
            encoded_data: "not!valid!base64!".to_string(),
        };

        // When attempting to resolve
        let result = base64_uri.resolve().await;

        // Then it should fail with a decode error
        assert!(result.is_err(), "invalid base64 should fail");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn base64_uri_fails_on_non_utf8() {
        // Given valid base64 that decodes to invalid UTF-8 (0xFF)
        let base64_uri = Base64Uri {
            encoded_data: "/w==".to_string(),
        };

        // When attempting to resolve
        let result = base64_uri.resolve().await;

        // Then it should fail with a UTF-8 error
        assert!(result.is_err(), "non-UTF8 data should fail");
    }
}

#[cfg(test)]
mod parse_uri_tests {
    //! Tests for URI parsing (TryFrom implementations).
    //! Each test follows: Given an input string, When parsing, Then expect success/failure.

    use super::{Base64Uri, FileUri, HttpUri, SettingsInput, StdinUri};
    use std::convert::TryFrom;
    use test_case::test_case;

    #[test_case("-"; "stdin_ok")]
    fn parse_stdin(input: &str) {
        // Given a stdin indicator, When parsing, Then StdinUri should be created
        let settings = SettingsInput::new(input);
        let uri = StdinUri::try_from(&settings).expect("should parse stdin");
        let _ = uri;
    }

    #[test_case(""; "empty_input")]
    #[test_case(" -"; "leading_space")]
    #[test_case("--"; "double_dash")]
    fn parse_stdin_fail(input: &str) {
        let settings = SettingsInput::new(input);
        assert!(
            StdinUri::try_from(&settings).is_err(),
            "only `-` should parse as stdin"
        );
    }

    #[test_case("file:///tmp/foo", "/tmp/foo"; "file_ok")]
    fn parse_file(input: &str, expected_path: &str) {
        let settings = SettingsInput::new(input);
        let uri = FileUri::try_from(&settings).expect("should parse file URI");
        assert_eq!(
            uri.path.to_str().unwrap(),
            expected_path,
            "file:// path must match"
        );
    }

    #[test_case("file_:/"; "weird_path")]
    #[test_case("file://no/leading/slash"; "no_leading_slash")]
    fn parse_file_fail(input: &str) {
        let settings = SettingsInput::new(input);
        assert!(
            FileUri::try_from(&settings).is_err(),
            "invalid file URI should fail"
        );
    }

    #[test_case("http://example.com/foo",  "http://example.com/foo";  "http_ok")]
    fn parse_http(input: &str, expected: &str) {
        let settings = SettingsInput::new(input);
        let uri = HttpUri::try_from(&settings).expect("should parse HTTP URI");
        assert_eq!(uri.url.as_str(), expected, "HTTP URI must round‑trip");
    }

    #[test_case("ftp://example.com";           "unsupported_scheme")]
    #[test_case("http://";                     "empty_authority")]
    #[test_case("https:// ";                   "space_after_scheme")]
    #[test_case("https://example.com";         "https_rejected")]
    fn parse_http_fail(input: &str) {
        let settings = SettingsInput::new(input);
        assert!(
            HttpUri::try_from(&settings).is_err(),
            "invalid HTTP URI should fail"
        );
    }

    #[test_case("base64:SGVsbG8gV29ybGQ=", "SGVsbG8gV29ybGQ="; "base64_ok")]
    fn parse_base64(input: &str, exp_data: &str) {
        let settings = SettingsInput::new(input);
        let uri = Base64Uri::try_from(&settings).expect("should parse base64 URI");
        assert_eq!(uri.encoded_data, exp_data, "base64 encoded data");
    }

    #[test_case("base64";                      "missing_colon")]
    #[test_case("base64:";                     "empty_data")]
    #[test_case("file://data";                 "wrong_scheme")]
    fn parse_base64_fail(input: &str) {
        let settings = SettingsInput::new(input);
        assert!(
            Base64Uri::try_from(&settings).is_err(),
            "invalid base64 URI should fail"
        );
    }
}
