//! Provides the list of errors for `cfsignal`.

use snafu::Snafu;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(super)))]
pub enum Error {
    #[snafu(display("Command '{}' with args '{:?}' failed: {}", command, args, source))]
    Command {
        command: String,
        args: Vec<String>,
        source: std::io::Error,
    },

    #[snafu(display("Failed to parse config file {}: {}", path.display(), source))]
    ConfigParse {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[snafu(display("Failed to read config file {}: {}", path.display(), source))]
    ConfigRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[snafu(display("IMDS request failed: {}", source))]
    ImdsRequest { source: imdsclient::Error },

    #[snafu(display("IMDS request failed: No '{}' found", what))]
    ImdsNone { what: String },

    #[snafu(display("SignalResource request failed: {}", source))]
    SignalResource {
        #[snafu(source(from(aws_sdk_cloudformation::error::SdkError<aws_sdk_cloudformation::operation::signal_resource::SignalResourceError>, Box::new)))]
        source: Box<
            aws_sdk_cloudformation::error::SdkError<
                aws_sdk_cloudformation::operation::signal_resource::SignalResourceError,
            >,
        >,
    },

    #[snafu(display("Invalid proxy URL: {}", source))]
    ProxyConfig {
        source: aws_smithy_http_client::proxy::ProxyError,
    },
}
