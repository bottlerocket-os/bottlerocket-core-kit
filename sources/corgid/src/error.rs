//! Error types for the corgid SBOM export service.
//!
//! This module uses the `snafu` crate for ergonomic error handling,
//! providing context-rich error messages for inventory parsing,
//! IMDS interactions, and Inspector API communication.

use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to read application inventory file: {source}"))]
    ReadInventory { source: std::io::Error },

    #[snafu(display("Failed to parse application inventory file: {source}"))]
    ParseInventory { source: serde_json::Error },

    #[snafu(display("Failed to serialize SBOM: {source}"))]
    SerializeSbom { source: serde_json::Error },

    #[snafu(display("Failed to get IMDS data: {source}"))]
    Imds { source: imdsclient::Error },

    #[snafu(display("Failed to fetch IMDS credentials: {source}"))]
    ImdsCredentials { source: reqwest::Error },

    #[snafu(display("Failed to parse IMDS credentials: {source}"))]
    ParseCredentials { source: serde_json::Error },

    #[snafu(display("HTTP request failed: {source}"))]
    HttpRequest { source: reqwest::Error },

    #[snafu(display("Inspector API error {status}: {body}"))]
    Api { status: u16, body: String },

    #[snafu(display("Failed to parse API response: {source}"))]
    ParseResponse { source: serde_json::Error },

    #[snafu(display("Failed to compress SBOM: {source}"))]
    Compress { source: std::io::Error },

    #[snafu(display("No IAM role found in IMDS"))]
    NoIamRole,

    #[snafu(display("No instance ID found in IMDS"))]
    NoInstanceId,
}

pub type Result<T> = std::result::Result<T, Error>;
