//! corgid - Bottlerocket package inventory reporter for Amazon Inspector
//!
//! This binary collects the system's package inventory, converts it to a CycloneDX SBOM,
//! and sends it to Amazon Inspector for vulnerability scanning. It uses IMDS to gather
//! instance metadata and IAM credentials for authentication.

#![deny(unused_imports)]

mod error;
mod inspector;
mod inventory;
mod sigv4;

use error::Result;
use imdsclient::ImdsClient;
use inspector::{get_credentials, send_inspector_sbom, start_session, stop_session};
use inventory::{read_and_convert, HostMetadata};
use log::{info, warn};
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use simplelog::{Config as LogConfig, LevelFilter, SimpleLogger};
use snafu::{OptionExt, ResultExt};
use std::process;
use std::process::Command;

const DEFAULT_REGION: &str = "us-east-1";

#[tokio::main]
async fn main() {
    SimpleLogger::init(LevelFilter::Info, LogConfig::default()).expect("logger init");

    if let Err(e) = run().await {
        eprintln!("{e}");
        process::exit(1);
    }
}

/// Main execution flow for sending inventory to Inspector.
///
/// The flow is: fetch metadata -> build SBOM -> start session -> send SBOM -> stop session.
/// Session cleanup is guaranteed even if sending fails (see deferred error pattern below).
async fn run() -> Result<()> {
    // Install rustls crypto provider for TLS operations
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    info!("Fetching metadata");
    let metadata = fetch_metadata().await?;

    info!("Reading inventory and converting to SBOM");
    let sbom = read_and_convert(&metadata)?;
    // Hash used by Inspector to verify SBOM integrity
    let sbom_hash = hex::encode(Sha256::digest(sbom.as_bytes()));

    let region = &metadata.region;
    let instance_id = &metadata.instance_id;

    info!("Fetching IAM credentials");
    let client = Client::new();
    let creds = get_credentials(&client)?;

    info!("Starting session");
    let session = start_session(&client, region, instance_id, &creds)?;

    info!("Sending SBOM");
    // Deferred error pattern: capture send failure but don't return early.
    // We must always call stop_session to clean up the Inspector session,
    // regardless of whether sending succeeded. The error is returned after cleanup.
    let (status, send_err) = match send_inspector_sbom(
        &client,
        region,
        instance_id,
        &session.session_id,
        &sbom,
        &creds,
    ) {
        Ok(()) => ("COMPLETED", None),
        Err(e) => {
            log::error!("Failed to send SBOM: {e}");
            ("AGENT_INTERNAL_ERROR", Some(e))
        }
    };

    info!("Stopping session");
    stop_session(
        &client,
        region,
        instance_id,
        &session.session_id,
        &sbom_hash,
        status,
        &creds,
    )?;

    // Return deferred send error after session cleanup completes
    if let Some(e) = send_err {
        return Err(e);
    }

    Ok(())
}

/// Fetches instance metadata from IMDS for SBOM generation.
///
/// Region and instance_id are required; other fields gracefully degrade to empty strings.
async fn fetch_metadata() -> Result<HostMetadata> {
    let mut imds = ImdsClient::new();

    // Region falls back to us-east-1 if unavailable (common for local testing)
    let region = match imds.fetch_region().await {
        Ok(Some(r)) => r,
        Ok(None) => {
            warn!("IMDS returned no region, using default: {}", DEFAULT_REGION);
            DEFAULT_REGION.to_string()
        }
        Err(e) => {
            warn!("Failed to fetch region: {e}");
            DEFAULT_REGION.to_string()
        }
    };

    // Instance ID is required - fail if unavailable
    let instance_id = imds
        .fetch_instance_id()
        .await
        .context(error::ImdsSnafu)?
        .context(error::NoInstanceIdSnafu)?;

    let meta = HostMetadata {
        region,
        instance_id,
        hostname: imds
            .fetch_hostname()
            .await
            .ok()
            .flatten()
            .unwrap_or_default(),
        instance_type: imds
            .fetch_instance_type()
            .await
            .ok()
            .flatten()
            .unwrap_or_default(),
        partition: imds
            .fetch_partition()
            .await
            .ok()
            .flatten()
            .unwrap_or_default(),
        account_id: fetch_account_id_blocking().unwrap_or_default(),
        // System info from uname for SBOM metadata
        kernel_name: uname_field("-s"),
        kernel_version: uname_field("-r"),
        cpu_architecture: uname_field("-m"),
    };

    Ok(meta)
}

/// Extracts a single field from uname output.
fn uname_field(flag: &str) -> String {
    Command::new("uname")
        .arg(flag)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Fetches AWS account ID directly from IMDS instance identity document.
///
/// Uses blocking HTTP because this runs in sync context. IMDSv2 requires
/// a session token for security.
fn fetch_account_id_blocking() -> Option<String> {
    // IMDSv2 requires a PUT to get a session token first
    let client = Client::new();
    let token = client
        .put("http://169.254.169.254/latest/api/token")
        .header("X-aws-ec2-metadata-token-ttl-seconds", "60")
        .send()
        .ok()?
        .text()
        .ok()?;

    let text = client
        .get("http://169.254.169.254/latest/dynamic/instance-identity/document")
        .header("X-aws-ec2-metadata-token", &token)
        .send()
        .ok()?
        .text()
        .ok()?;

    let doc: serde_json::Value = serde_json::from_str(&text).ok()?;
    doc.get("accountId")?.as_str().map(String::from)
}
