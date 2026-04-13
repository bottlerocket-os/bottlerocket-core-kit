//! Amazon Inspector API client.
//!
//! This module handles communication with the Inspector2 service for
//! vulnerability scanning. It manages the session lifecycle: start -> send SBOM -> stop.

use crate::error::{self, Result};
use crate::sigv4::{sign, SignInput};
use chrono::Utc;
use flate2::write::GzEncoder;
use flate2::Compression;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use snafu::ResultExt;
use std::io::Write;
use std::thread;
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

/// Serializes byte slices as base64 strings for JSON encoding.
fn serialize_base64<S: serde::Serializer>(
    data: &&[u8],
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    serializer.serialize_str(&BASE64.encode(data))
}

const IMDS_BASE: &str = "http://169.254.169.254";
const IMDS_TOKEN_PATH: &str = "/latest/api/token";
const IMDS_ROLE_PATH: &str = "/latest/meta-data/iam/security-credentials/";
const SERVICE: &str = "inspector2-telemetry";
/// SBOM chunks are limited to 390KB to stay within API payload limits.
const CHUNK_SIZE: usize = 390 * 1024;
/// Retry transient failures up to 3 times with exponential backoff.
const MAX_RETRIES: u32 = 3;
const VERSION: &str = "0.1.0";

/// IAM credentials retrieved from IMDS for signing API requests.
#[derive(Deserialize)]
pub struct Credentials {
    #[serde(rename = "AccessKeyId")]
    pub access_key_id: String,
    #[serde(rename = "SecretAccessKey")]
    pub secret_access_key: String,
    #[serde(rename = "Token")]
    pub token: String,
}

/// Wrapper for all API requests.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TelemetryRequest<'a> {
    resource_id: &'a str,
    event: TelemetryEvent<'a>,
}

/// Discriminated union of event types.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum TelemetryEvent<'a> {
    StartSession(StartSessionEvent<'a>),
    SendTelemetry(SendTelemetryEvent<'a>),
    StopSession(StopSessionEvent<'a>),
}

/// Initiates a new vulnerability scan session.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StartSessionEvent<'a> {
    session_scan_type: &'a str,
    resource_type: &'a str,
    agent_version: &'a str,
}

/// Sends SBOM data chunk within an active session.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SendTelemetryEvent<'a> {
    session_id: &'a str,
    capture_time: f64,
    data: TelemetryData<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TelemetryData<'a> {
    vulnerability_data: VulnData<'a>,
}

/// SBOM chunk with integrity hash for verification.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VulnData<'a> {
    content_hash: &'a str,
    #[serde(serialize_with = "serialize_base64")]
    sbom: &'a [u8],
    sequence_number: u32,
}

/// Terminates a scan session with final status and metrics.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StopSessionEvent<'a> {
    session_id: &'a str,
    session_details: SessionDetails<'a>,
    scan_details: ScanDetails<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionDetails<'a> {
    session_status: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanDetails<'a> {
    scan_job_status: &'a str,
    data_checksum: &'a str,
    performance_details: PerformanceDetails,
}

/// Resource usage metrics (currently placeholder values).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PerformanceDetails {
    cpu_metrics: CpuMetrics,
    memory_metrics: MemoryMetrics,
    artifact_metrics: ArtifactMetrics,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CpuMetrics {
    average_cpu: f64,
    max_cpu: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryMetrics {
    average_memory_consumed: f64,
    max_memory_consumed: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactMetrics {
    data_collection_in_milliseconds: i64,
}

/// API response containing optional session start result.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TelemetryResponse {
    start_session_result: Option<StartSessionResult>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartSessionResult {
    session_id: String,
}

/// Returns the parent domain based on region partition.
fn parent_domain(region: &str) -> &'static str {
    // China regions use a different domain suffix
    if region.starts_with("cn-") {
        "api.amazonwebservices.com.cn"
    } else {
        "api.aws"
    }
}

/// Constructs the endpoint URL and host header for a region.
fn endpoint_and_host(region: &str) -> (String, String) {
    let host = format!("inspector2-telemetry.{}.{}", region, parent_domain(region));
    let endpoint = format!("https://{}/telemetry", host);
    (endpoint, host)
}

/// Result of starting a session.
pub struct StartResponse {
    pub session_id: String,
}

/// Retrieves inspector IAM credentials from the EC2 Instance Metadata Service.
///
/// Uses IMDSv2 with a session token for security.
pub fn get_credentials(client: &Client) -> Result<Credentials> {
    // IMDSv2 requires a session token obtained via PUT
    let token = client
        .put(format!("{}{}", IMDS_BASE, IMDS_TOKEN_PATH))
        .header("X-aws-ec2-metadata-token-ttl-seconds", "21600")
        .send()
        .context(error::ImdsCredentialsSnafu)?
        .text()
        .context(error::ImdsCredentialsSnafu)?;

    // List available IAM roles attached to the instance
    let roles = client
        .get(format!("{}{}", IMDS_BASE, IMDS_ROLE_PATH))
        .header("X-aws-ec2-metadata-token", &token)
        .send()
        .context(error::ImdsCredentialsSnafu)?
        .text()
        .context(error::ImdsCredentialsSnafu)?;

    let role = roles.lines().next().ok_or(error::Error::NoIamRole)?;

    // Fetch credentials for the first available role
    let creds_json = client
        .get(format!("{}{}{}", IMDS_BASE, IMDS_ROLE_PATH, role))
        .header("X-aws-ec2-metadata-token", &token)
        .send()
        .context(error::ImdsCredentialsSnafu)?
        .text()
        .context(error::ImdsCredentialsSnafu)?;

    serde_json::from_str(&creds_json).context(error::ParseCredentialsSnafu)
}

/// Sends a signed POST request with exponential backoff retry.
///
/// Retries on 429 (throttling) and 5xx (server errors) up to MAX_RETRIES times.
/// Each retry doubles the delay starting from 1 second.
fn post_with_retry(
    client: &Client,
    body: &[u8],
    region: &str,
    creds: &Credentials,
) -> Result<String> {
    let (endpoint, host) = endpoint_and_host(region);
    let mut delay = Duration::from_secs(1);
    log::info!("POST {}", endpoint);
    for attempt in 0..=MAX_RETRIES {
        // Sign each attempt fresh since timestamp must be current
        let signed = sign(&SignInput {
            method: "POST",
            host: &host,
            path: "/telemetry",
            body,
            region,
            service: SERVICE,
            access_key: &creds.access_key_id,
            secret_key: &creds.secret_access_key,
            token: &creds.token,
            time: Utc::now(),
        });

        let resp = client
            .post(&endpoint)
            .header("Content-Type", "application/json")
            .header("Authorization", &signed.authorization)
            .header("X-Amz-Date", &signed.x_amz_date)
            .header("X-Amz-Security-Token", &signed.x_amz_security_token)
            .header("X-Amz-Content-Sha256", &signed.x_amz_content_sha256)
            .body(body.to_vec())
            .send()
            .context(error::HttpRequestSnafu)?;

        let status = resp.status().as_u16();
        if (200..300).contains(&status) {
            return resp.text().context(error::HttpRequestSnafu);
        }
        // Retry transient errors: throttling (429) or server errors (5xx)
        if (status == 429 || status >= 500) && attempt < MAX_RETRIES {
            thread::sleep(delay);
            delay *= 2; // Exponential backoff
            continue;
        }
        let body_text = resp.text().unwrap_or_default();
        return Err(error::Error::Api {
            status,
            body: body_text,
        });
    }
    unreachable!()
}

/// Starts a new vulnerability scan session with Inspector.
///
/// Must be called before sending SBOM data. Returns a session ID
/// that must be used for subsequent send and stop calls.
pub fn start_session(
    client: &Client,
    region: &str,
    instance_id: &str,
    creds: &Credentials,
) -> Result<StartResponse> {
    let req = TelemetryRequest {
        resource_id: instance_id,
        event: TelemetryEvent::StartSession(StartSessionEvent {
            session_scan_type: "VULNERABILITY_SCAN",
            resource_type: "AWS_EC2_INSTANCE",
            agent_version: VERSION,
        }),
    };
    let body = serde_json::to_vec(&req).context(error::SerializeSbomSnafu)?;
    let resp = post_with_retry(client, &body, region, creds)?;
    log::info!("StartSession response: {}", &resp);
    let parsed: TelemetryResponse =
        serde_json::from_str(&resp).context(error::ParseResponseSnafu)?;
    let result = parsed
        .start_session_result
        .ok_or_else(|| error::Error::Api {
            status: 500,
            body: "Missing startSessionResult".to_string(),
        })?;
    Ok(StartResponse {
        session_id: result.session_id,
    })
}

/// Sends SBOM data to Inspector within an active session.
///
/// The SBOM is gzip-compressed and split into chunks if needed.
/// Each chunk is hashed and sent with a sequence number for reassembly.
pub fn send_inspector_sbom(
    client: &Client,
    region: &str,
    instance_id: &str,
    session_id: &str,
    sbom: &str,
    creds: &Credentials,
) -> Result<()> {
    // Compress SBOM to reduce payload size
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(sbom.as_bytes())
        .context(error::CompressSnafu)?;
    let compressed = encoder.finish().context(error::CompressSnafu)?;

    let capture_time = Utc::now().timestamp() as f64;

    // Split into chunks to stay within API payload limits
    for (seq, chunk) in compressed.chunks(CHUNK_SIZE).enumerate() {
        let chunk_hash = hex::encode(Sha256::digest(chunk));
        let req = TelemetryRequest {
            resource_id: instance_id,
            event: TelemetryEvent::SendTelemetry(SendTelemetryEvent {
                session_id,
                capture_time,
                data: TelemetryData {
                    vulnerability_data: VulnData {
                        content_hash: &chunk_hash,
                        sbom: chunk,
                        sequence_number: seq as u32,
                    },
                },
            }),
        };
        let body = serde_json::to_vec(&req).context(error::SerializeSbomSnafu)?;
        post_with_retry(client, &body, region, creds)?;
    }
    Ok(())
}

/// Terminates a scan session and reports final status.
///
/// Should be called after all SBOM data is sent, or on error to clean up.
/// The session_status is derived from scan_job_status: AGENT_INTERNAL_ERROR
/// results in FAILURE, all other statuses result in SUCCESSFUL.
pub fn stop_session(
    client: &Client,
    region: &str,
    instance_id: &str,
    session_id: &str,
    sbom_hash: &str,
    scan_job_status: &str,
    creds: &Credentials,
) -> Result<()> {
    // Map scan job status to session outcome
    let session_status = if scan_job_status == "AGENT_INTERNAL_ERROR" {
        "FAILURE"
    } else {
        "SUCCESSFUL"
    };
    let req = TelemetryRequest {
        resource_id: instance_id,
        event: TelemetryEvent::StopSession(StopSessionEvent {
            session_id,
            session_details: SessionDetails { session_status },
            scan_details: ScanDetails {
                scan_job_status,
                data_checksum: sbom_hash,
                performance_details: PerformanceDetails {
                    cpu_metrics: CpuMetrics {
                        average_cpu: 0.0,
                        max_cpu: 0.0,
                    },
                    memory_metrics: MemoryMetrics {
                        average_memory_consumed: 0.0,
                        max_memory_consumed: 0.0,
                    },
                    artifact_metrics: ArtifactMetrics {
                        data_collection_in_milliseconds: 0,
                    },
                },
            },
        }),
    };
    let body = serde_json::to_vec(&req).context(error::SerializeSbomSnafu)?;
    post_with_retry(client, &body, region, creds)?;
    Ok(())
}
