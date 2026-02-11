//! TLS-dependent URI resolvers (HTTPS, S3, Secrets Manager, SSM Parameter Store).
//!
//! This module is only compiled when the `tls` feature is enabled.

use crate::uri_resolver::resolver_error::{InvalidArnFormatSnafu, ResponseSizeOverflowSnafu};
use crate::uri_resolver::SettingsInput;
use crate::uri_resolver::{
    resolve_http_url, ResolverResult, UriResolver, MAX_SIZE_BYTES, OPERATION_TIMEOUT_SECS,
};
use async_trait::async_trait;
use reqwest::Url;
use snafu::{ensure, OptionExt, ResultExt, Snafu};
use std::convert::TryFrom;
use tokio::io::AsyncReadExt;

use aws_config::{self, timeout::TimeoutConfig};
use aws_sdk_s3;
use aws_sdk_secretsmanager;
use aws_sdk_ssm;
use aws_smithy_types::error::metadata::ProvideErrorMetadata;
use std::time::Duration;
use tls_resolver_error::*;

/// Formats an AWS SDK error as "code: message" for user-friendly display.
fn format_sdk_error(e: &impl ProvideErrorMetadata) -> String {
    match (e.code(), e.message()) {
        (Some(c), Some(m)) => format!("{c}: {m}"),
        (Some(c), None) => c.to_string(),
        (None, Some(m)) => m.to_string(),
        (None, None) => "Unknown error".to_string(),
    }
}

/// Returns AWS SDK config with standard timeouts.
async fn aws_sdk_config() -> aws_config::SdkConfig {
    aws_sdk_config_with_region(None).await
}

/// Returns AWS SDK config with standard timeouts and optional region override.
async fn aws_sdk_config_with_region(region: Option<&str>) -> aws_config::SdkConfig {
    let mut builder = aws_config::defaults(aws_config::BehaviorVersion::latest()).timeout_config(
        TimeoutConfig::builder()
            .operation_timeout(Duration::from_secs(OPERATION_TIMEOUT_SECS))
            .build(),
    );
    if let Some(r) = region {
        builder = builder.region(aws_config::Region::new(r.to_owned()));
    }
    builder.load().await
}

pub struct HttpsUri {
    url: Url,
}

impl TryFrom<&SettingsInput> for HttpsUri {
    type Error = crate::uri_resolver::ResolverError;
    fn try_from(input: &SettingsInput) -> ResolverResult<Self> {
        let url = input.parsed_url.as_ref().context(InvalidHttpsUriSnafu {
            input_source: input.input.clone(),
        })?;
        ensure!(
            url.scheme() == "https",
            InvalidHttpsUriSnafu {
                input_source: url.to_string()
            }
        );
        Ok(HttpsUri { url: url.clone() })
    }
}

#[async_trait]
impl UriResolver for HttpsUri {
    async fn resolve(&self) -> ResolverResult<String> {
        resolve_http_url(&self.url).await
    }
}

struct Arn {
    service: String,
    region: String,
}

impl Arn {
    fn parse(input: &str) -> ResolverResult<Self> {
        let mut split = input.splitn(6, ':');
        let err = || InvalidArnFormatSnafu {
            input_source: input.to_string(),
            reason: "must be a valid ARN (arn:<partition>:<service>:<region>:...)".to_string(),
        };

        let arn = split.next().context(err())?;
        let _partition = split.next().context(err())?;
        let service = split.next().context(err())?;
        let region = split.next().context(err())?;
        let _account = split.next().context(err())?;
        let _resource = split.next().context(err())?;

        ensure!(arn == "arn", err());
        ensure!(!service.is_empty(), err());
        ensure!(!region.is_empty(), err());

        Ok(Arn {
            service: service.to_string(),
            region: region.to_string(),
        })
    }
}

pub struct S3Uri {
    pub bucket: String,
    pub key: String,
}

impl TryFrom<&SettingsInput> for S3Uri {
    type Error = crate::uri_resolver::ResolverError;
    fn try_from(input: &SettingsInput) -> ResolverResult<Self> {
        const PREFIX: &str = "s3://";
        let uri_str = input.input.as_str();
        let remainder = uri_str.strip_prefix(PREFIX).context(S3UriSchemeSnafu {
            input_source: input.input.clone(),
        })?;
        let mut parts = remainder.splitn(2, '/');
        let bucket = parts.next().context(S3UriMissingBucketSnafu {
            input_source: input.input.clone(),
        })?;
        let key = parts.next().context(S3UriMissingKeySnafu {
            input_source: input.input.clone(),
        })?;
        ensure!(
            !bucket.is_empty(),
            S3UriMissingBucketSnafu {
                input_source: input.input.clone()
            }
        );
        ensure!(
            !key.is_empty(),
            S3UriMissingKeySnafu {
                input_source: input.input.clone()
            }
        );
        Ok(S3Uri {
            bucket: bucket.to_string(),
            key: key.to_string(),
        })
    }
}

#[async_trait]
impl UriResolver for S3Uri {
    async fn resolve(&self) -> ResolverResult<String> {
        let cfg = aws_sdk_config().await;
        let client = aws_sdk_s3::Client::new(&cfg);

        let resp = client
            .get_object()
            .bucket(&self.bucket)
            .key(&self.key)
            .send()
            .await
            .map_err(|e| TlsResolverError::S3Get {
                bucket: self.bucket.clone(),
                key: self.key.clone(),
                error_msg: format_sdk_error(&e),
            })?;

        // Early rejection based on Content-Length header
        if let Some(content_length) = resp.content_length() {
            let size: u64 = content_length
                .try_into()
                .context(ResponseSizeOverflowSnafu {
                    size: content_length.try_into().unwrap_or(usize::MAX),
                })?;
            if size > MAX_SIZE_BYTES {
                return Err(S3ObjectTooLargeSnafu {
                    size,
                    max_size: MAX_SIZE_BYTES,
                    bucket: self.bucket.clone(),
                    key: self.key.clone(),
                }
                .build()
                .into());
            }
        }

        // Stream with hard limit as safety net
        let mut buffer = Vec::new();
        // Read one extra byte to detect if content exceeds limit
        resp.body
            .into_async_read()
            .take(MAX_SIZE_BYTES + 1)
            .read_to_end(&mut buffer)
            .await
            .context(S3BodySnafu {
                bucket: self.bucket.clone(),
                key: self.key.clone(),
            })?;

        let size: u64 = buffer
            .len()
            .try_into()
            .context(ResponseSizeOverflowSnafu { size: buffer.len() })?;
        if size > MAX_SIZE_BYTES {
            return Err(S3ObjectTooLargeSnafu {
                size,
                max_size: MAX_SIZE_BYTES,
                bucket: self.bucket.clone(),
                key: self.key.clone(),
            }
            .build()
            .into());
        }

        String::from_utf8(buffer).context(crate::uri_resolver::resolver_error::Utf8DecodeSnafu {
            uri: format!("s3://{}/{}", self.bucket, self.key),
        })
    }
}

pub struct SecretsManagerArn {
    pub region: String,
    pub full_arn: String,
}

impl TryFrom<&SettingsInput> for SecretsManagerArn {
    type Error = crate::uri_resolver::ResolverError;
    fn try_from(input: &SettingsInput) -> ResolverResult<Self> {
        let arn = Arn::parse(input.input.as_str())?;
        ensure!(
            arn.service == "secretsmanager",
            SecretsManagerArnSnafu {
                input_source: input.input.clone()
            }
        );
        Ok(SecretsManagerArn {
            region: arn.region,
            full_arn: input.input.to_string(),
        })
    }
}

#[async_trait]
impl UriResolver for SecretsManagerArn {
    async fn resolve(&self) -> ResolverResult<String> {
        let cfg = aws_sdk_config_with_region(Some(&self.region)).await;
        let client = aws_sdk_secretsmanager::Client::new(&cfg);
        let resp = client
            .get_secret_value()
            .secret_id(self.full_arn.clone())
            .send()
            .await
            .map_err(|e| TlsResolverError::SecretsManagerGet {
                secret_id: self.full_arn.clone(),
                error_msg: format_sdk_error(&e),
            })?;
        Ok(resp
            .secret_string()
            .map(str::to_string)
            .context(SecretsManagerStringMissingSnafu {
                secret_id: self.full_arn.clone(),
            })?)
    }
}

/// Resolves `secretsmanager://` URIs to fetch secrets from AWS Secrets Manager.
///
/// Uses the default AWS region from environment (`AWS_REGION`) or instance metadata.
/// For cross-region access, use the full ARN instead: `arn:aws:secretsmanager:REGION:...`
pub struct SecretsManagerUri {
    pub secret_id: String,
}

impl TryFrom<&SettingsInput> for SecretsManagerUri {
    type Error = crate::uri_resolver::ResolverError;
    fn try_from(input: &SettingsInput) -> ResolverResult<Self> {
        const PREFIX: &str = "secretsmanager://";
        let uri_str = input.input.as_str();
        let remainder = uri_str
            .strip_prefix(PREFIX)
            .context(SecretsManagerUriSnafu {
                input_source: input.input.clone(),
            })?;
        ensure!(
            !remainder.is_empty(),
            SecretsManagerUriSnafu {
                input_source: input.input.clone()
            }
        );
        Ok(SecretsManagerUri {
            secret_id: remainder.to_string(),
        })
    }
}

#[async_trait]
impl UriResolver for SecretsManagerUri {
    async fn resolve(&self) -> ResolverResult<String> {
        let cfg = aws_sdk_config().await;
        let client = aws_sdk_secretsmanager::Client::new(&cfg);
        let resp = client
            .get_secret_value()
            .secret_id(self.secret_id.clone())
            .send()
            .await
            .map_err(|e| TlsResolverError::SecretsManagerGet {
                secret_id: self.secret_id.clone(),
                error_msg: format_sdk_error(&e),
            })?;
        Ok(resp
            .secret_string()
            .map(str::to_string)
            .context(SecretsManagerStringMissingSnafu {
                secret_id: self.secret_id.clone(),
            })?)
    }
}

pub struct SsmArn {
    pub region: String,
    pub full_arn: String,
}

impl TryFrom<&SettingsInput> for SsmArn {
    type Error = crate::uri_resolver::ResolverError;
    fn try_from(input: &SettingsInput) -> ResolverResult<Self> {
        let arn = Arn::parse(input.input.as_str())?;
        ensure!(
            arn.service == "ssm",
            SsmArnSnafu {
                input_source: input.input.clone()
            }
        );
        Ok(SsmArn {
            region: arn.region,
            full_arn: input.input.to_string(),
        })
    }
}

#[async_trait]
impl UriResolver for SsmArn {
    async fn resolve(&self) -> ResolverResult<String> {
        let cfg = aws_sdk_config_with_region(Some(&self.region)).await;
        let client = aws_sdk_ssm::Client::new(&cfg);
        let resp = client
            .get_parameter()
            .name(self.full_arn.clone())
            .with_decryption(true)
            .send()
            .await
            .map_err(|e| TlsResolverError::SsmGetParameter {
                parameter_name: self.full_arn.clone(),
                error_msg: format_sdk_error(&e),
            })?;
        let value = resp
            .parameter
            .and_then(|p| p.value().map(|v| v.to_string()))
            .context(SsmParameterMissingSnafu {
                parameter_name: self.full_arn.clone(),
            })?;
        Ok(value)
    }
}

/// Resolves `ssm://` URIs to fetch parameters from AWS Systems Manager Parameter Store.
///
/// Uses the default AWS region from environment (`AWS_REGION`) or instance metadata.
/// For cross-region access, use the full ARN instead: `arn:aws:ssm:REGION:...`
pub struct SsmUri {
    pub parameter_name: String,
}

impl TryFrom<&SettingsInput> for SsmUri {
    type Error = crate::uri_resolver::ResolverError;
    fn try_from(input: &SettingsInput) -> ResolverResult<Self> {
        const PREFIX: &str = "ssm://";
        let uri_str = input.input.as_str();
        let remainder = uri_str.strip_prefix(PREFIX).context(SsmUriSnafu {
            input_source: input.input.clone(),
        })?;
        ensure!(
            !remainder.is_empty(),
            SsmUriSnafu {
                input_source: input.input.clone()
            }
        );
        Ok(SsmUri {
            parameter_name: remainder.to_string(),
        })
    }
}

#[async_trait]
impl UriResolver for SsmUri {
    async fn resolve(&self) -> ResolverResult<String> {
        let config = aws_sdk_config().await;
        let client = aws_sdk_ssm::Client::new(&config);
        let resp = client
            .get_parameter()
            .name(self.parameter_name.clone())
            .with_decryption(true)
            .send()
            .await
            .map_err(|e| TlsResolverError::SsmGetParameter {
                parameter_name: self.parameter_name.clone(),
                error_msg: format_sdk_error(&e),
            })?;
        let value = resp
            .parameter
            .and_then(|p| p.value().map(|v| v.to_string()))
            .context(SsmParameterMissingSnafu {
                parameter_name: self.parameter_name.clone(),
            })?;
        Ok(value)
    }
}

#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum TlsResolverError {
    #[snafu(display("Given invalid https:// URI '{}'", input_source))]
    InvalidHttpsUri { input_source: String },

    #[snafu(display("Failed to fetch S3 object '{bucket}/{key}': {error_msg}"))]
    S3Get {
        bucket: String,
        key: String,
        error_msg: String,
    },

    #[snafu(display("Failed to read S3 object body '{bucket}/{key}': {}", source))]
    S3Body {
        bucket: String,
        key: String,
        source: std::io::Error,
    },

    #[snafu(display("Failed to fetch secret '{secret_id}' from Secrets Manager: {error_msg}"))]
    SecretsManagerGet {
        secret_id: String,
        error_msg: String,
    },

    #[snafu(display("Failed to fetch parameter '{parameter_name}' from SSM: {error_msg}"))]
    SsmGetParameter {
        parameter_name: String,
        error_msg: String,
    },

    #[snafu(display(
        "S3 object s3://{bucket}/{key} is too large ({size} bytes, maximum is {max_size} bytes)"
    ))]
    S3ObjectTooLarge {
        size: u64,
        max_size: u64,
        bucket: String,
        key: String,
    },

    #[snafu(display("Invalid S3 URI scheme for '{}', expected s3://", input_source))]
    S3UriScheme { input_source: String },

    #[snafu(display("Invalid S3 URI '{}': missing bucket name", input_source))]
    S3UriMissingBucket { input_source: String },

    #[snafu(display("Invalid S3 URI '{}': missing key name", input_source))]
    S3UriMissingKey { input_source: String },

    #[snafu(display(
        "Invalid Secrets Manager URI scheme for '{}', expected secretsmanager://",
        input_source
    ))]
    SecretsManagerUri { input_source: String },

    #[snafu(display("Secrets Manager secret '{}' did not return a string value", secret_id))]
    SecretsManagerStringMissing { secret_id: String },

    #[snafu(display(
        "Invalid Secrets Manager ARN for '{}', expected arn:<partition>:secretsmanager:...",
        input_source
    ))]
    SecretsManagerArn { input_source: String },

    #[snafu(display(
        "Invalid SSM ARN for '{}', expected arn:<partition>:ssm:...",
        input_source
    ))]
    SsmArn { input_source: String },

    #[snafu(display("SSM ARN parameter '{}' did not return a string value", parameter_name))]
    SsmArnValueMissing { parameter_name: String },

    #[snafu(display("Invalid SSM URI scheme for '{}', expected ssm://", input_source))]
    SsmUri { input_source: String },

    #[snafu(display("SSM parameter '{}' did not return a string value", parameter_name))]
    SsmParameterMissing { parameter_name: String },
}

#[cfg(test)]
mod tests {
    //! Tests for TLS-dependent URI parsing (HTTPS, S3, Secrets Manager, SSM).
    //! Each test follows: Given an input string, When parsing, Then expect success/failure.

    use super::*;
    use crate::uri_resolver::SettingsInput;
    use std::convert::TryFrom;
    use test_case::test_case;

    #[test_case("https://example.com/foo", "https://example.com/foo"; "https_ok")]
    fn parse_https(input: &str, expected: &str) {
        // Given an HTTPS URL, When parsing, Then HttpsUri should capture the URL
        let settings = SettingsInput::new(input);
        let uri = HttpsUri::try_from(&settings).expect("should parse HTTPS URI");
        assert_eq!(uri.url.as_str(), expected);
    }

    #[test_case("http://example.com"; "http_rejected")]
    #[test_case("ftp://example.com"; "unsupported_scheme")]
    fn parse_https_fail(input: &str) {
        let settings = SettingsInput::new(input);
        assert!(
            HttpsUri::try_from(&settings).is_err(),
            "should reject non-HTTPS URI"
        );
    }

    #[test_case("s3://bucket/key", "bucket", "key"; "s3_simple")]
    #[test_case("s3://testbucket/file with spaces.json", "testbucket", "file with spaces.json"; "key_with_spaces")]
    #[test_case("s3://my-bucket/path/to/config.toml", "my-bucket", "path/to/config.toml"; "key_with_path")]
    #[test_case("s3://bucket/key:with:colons", "bucket", "key:with:colons"; "key_with_colons")]
    fn parse_s3(input: &str, exp_bucket: &str, exp_key: &str) {
        // Given an S3 URI, When parsing, Then bucket and key should be extracted
        let settings = SettingsInput::new(input);
        let uri = S3Uri::try_from(&settings).expect("should parse S3 URI");
        assert_eq!(uri.bucket, exp_bucket, "S3 bucket");
        assert_eq!(uri.key, exp_key, "S3 key");
    }

    #[test_case("s3://bucket"; "missing_key")]
    #[test_case("s3:/bucket/key"; "malformed_scheme")]
    #[test_case("s3://"; "empty_bucket_and_key")]
    #[test_case("s3:///key"; "empty_bucket")]
    #[test_case("s3://bucket/"; "empty_key")]
    fn parse_s3_fail(input: &str) {
        let settings = SettingsInput::new(input);
        assert!(
            S3Uri::try_from(&settings).is_err(),
            "invalid S3 URI should fail"
        );
    }

    #[test_case(
        "arn:aws:secretsmanager:us-east-1:111122223333:secret:mysecret",
        "us-east-1", "arn:aws:secretsmanager:us-east-1:111122223333:secret:mysecret";
        "secretsmanager_arn_aws"
    )]
    #[test_case(
        "arn:aws-cn:secretsmanager:cn-north-1:111122223333:secret:mysecret",
        "cn-north-1", "arn:aws-cn:secretsmanager:cn-north-1:111122223333:secret:mysecret";
        "secretsmanager_arn_china"
    )]
    #[test_case(
        "arn:aws-us-gov:secretsmanager:us-gov-west-1:111122223333:secret:mysecret",
        "us-gov-west-1", "arn:aws-us-gov:secretsmanager:us-gov-west-1:111122223333:secret:mysecret";
        "secretsmanager_arn_govcloud"
    )]
    fn parse_secretsmanager_arn(input: &str, exp_region: &str, exp_id: &str) {
        // Given a Secrets Manager ARN, When parsing, Then region and full ARN should be extracted
        let settings = SettingsInput::new(input);
        let uri = SecretsManagerArn::try_from(&settings).expect("should parse SecretsManager ARN");
        assert_eq!(uri.region, exp_region, "SecretsManager ARN region");
        assert_eq!(uri.full_arn, exp_id, "SecretsManager ARN secret id");
    }

    #[test_case("arn:aws:ssm:us-west-2:123456789012:parameter/myparam"; "ssm_arn_not_secretsmanager")]
    #[test_case("arn:aws:secretsmanager::111122223333:secret:mysecret"; "empty_region")]
    #[test_case("arn:aws::us-east-1:111122223333:secret:mysecret"; "empty_service")]
    fn parse_secretsmanager_arn_fail(input: &str) {
        let settings = SettingsInput::new(input);
        assert!(
            SecretsManagerArn::try_from(&settings).is_err(),
            "SSM ARN should not parse as SecretsManagerArn"
        );
    }

    #[test_case("secretsmanager://mysecret", "mysecret"; "secrets_simple")]
    #[test_case("secretsmanager://my-app/prod/db-creds", "my-app/prod/db-creds"; "secrets_with_slashes")]
    #[test_case("secretsmanager://MySecret-abc123", "MySecret-abc123"; "secrets_with_suffix")]
    fn parse_secrets(input: &str, exp_id: &str) {
        // Given a secretsmanager:// URI, When parsing, Then secret_id should be extracted
        let settings = SettingsInput::new(input);
        let uri = SecretsManagerUri::try_from(&settings).expect("should parse SecretsManager URI");
        assert_eq!(uri.secret_id, exp_id, "secret_id");
    }

    #[test_case("secretsmanager:/mysecret"; "missing_double_slash")]
    #[test_case("secretsmanager://"; "empty_secret_id")]
    #[test_case("ssm://mysecret"; "wrong_scheme")]
    fn parse_secrets_uri_fail(input: &str) {
        let settings = SettingsInput::new(input);
        assert!(
            SecretsManagerUri::try_from(&settings).is_err(),
            "invalid SecretsManager URI should fail"
        );
    }

    #[test_case(
        "arn:aws:ssm:us-west-2:123456789012:parameter/myparam",
        "us-west-2", "arn:aws:ssm:us-west-2:123456789012:parameter/myparam";
        "ssm_arn_aws"
    )]
    #[test_case(
        "arn:aws-cn:ssm:cn-north-1:123456789012:parameter/myparam",
        "cn-north-1", "arn:aws-cn:ssm:cn-north-1:123456789012:parameter/myparam";
        "ssm_arn_china"
    )]
    fn parse_ssm_arn(input: &str, exp_region: &str, exp_param: &str) {
        // Given an SSM parameter ARN, When parsing, Then region and full ARN should be extracted
        let settings = SettingsInput::new(input);
        let uri = SsmArn::try_from(&settings).expect("should parse SSM ARN");
        assert_eq!(uri.region, exp_region, "SSM ARN region");
        assert_eq!(uri.full_arn, exp_param, "SSM ARN parameter");
    }

    #[test_case("arn:aws:secretsmanager:us-east-1:111122223333:secret:mysecret"; "secretsmanager_arn_not_ssm")]
    #[test_case("arn:aws:ssm::123456789012:parameter/foo"; "empty_region")]
    #[test_case("arn:aws::us-west-2:123456789012:parameter/foo"; "empty_service")]
    #[test_case("arn:aws:ssm:us-west-2:123456789012"; "too_few_colons")]
    fn parse_ssm_arn_fail(input: &str) {
        let settings = SettingsInput::new(input);
        assert!(
            SsmArn::try_from(&settings).is_err(),
            "invalid SSM ARN should fail"
        );
    }

    #[test_case("ssm://parameter", "parameter"; "ssm_flat")]
    #[test_case("ssm:///path/to/param", "/path/to/param"; "ssm_hierarchical")]
    #[test_case("ssm:///my-app/prod/db-password", "/my-app/prod/db-password"; "ssm_hierarchical_multi")]
    fn parse_ssm(input: &str, exp_param: &str) {
        // Given an ssm:// URI, When parsing, Then parameter_name should be extracted
        let settings = SettingsInput::new(input);
        let uri = SsmUri::try_from(&settings).expect("should parse SSM URI");
        assert_eq!(uri.parameter_name, exp_param, "parameter name");
    }

    #[test_case("ssm:/parameter"; "missing_double_slash")]
    #[test_case("ssm://"; "empty_parameter")]
    #[test_case("secretsmanager://parameter"; "wrong_scheme")]
    fn parse_ssm_uri_fail(input: &str) {
        let settings = SettingsInput::new(input);
        assert!(
            SsmUri::try_from(&settings).is_err(),
            "invalid SSM URI should fail"
        );
    }
}
