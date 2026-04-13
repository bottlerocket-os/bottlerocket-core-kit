//! AWS Signature Version 4 request signing.
//!
//! Implements the AWS SigV4 algorithm for authenticating requests to AWS services.
//! The signing process creates a cryptographic signature using the request details
//! and AWS credentials, allowing AWS to verify request authenticity and integrity.

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// Input parameters required to sign an AWS request.
pub struct SignInput<'a> {
    pub method: &'a str,
    pub host: &'a str,
    pub path: &'a str,
    pub body: &'a [u8],
    pub region: &'a str,
    pub service: &'a str,
    pub access_key: &'a str,
    pub secret_key: &'a str,
    pub token: &'a str,
    pub time: DateTime<Utc>,
}

/// Headers produced by the signing process, ready to attach to an HTTP request.
pub struct SignedRequest {
    pub authorization: String,
    pub x_amz_date: String,
    pub x_amz_security_token: String,
    pub x_amz_content_sha256: String,
}

/// Signs an AWS request using the SigV4 algorithm.
///
/// Returns headers that must be included in the HTTP request for AWS to
/// authenticate it. The signature covers the method, path, headers, and body.
pub fn sign(input: &SignInput) -> SignedRequest {
    let date_stamp = input.time.format("%Y%m%d").to_string();
    let amz_date = input.time.format("%Y%m%dT%H%M%SZ").to_string();

    // Hash the payload to ensure body integrity
    let payload_hash = hex::encode(Sha256::digest(input.body));

    // Canonical headers must be sorted and newline-terminated
    let canonical_headers = format!(
        "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\nx-amz-security-token:{}\n",
        input.host, payload_hash, amz_date, input.token
    );
    let signed_headers = "host;x-amz-content-sha256;x-amz-date;x-amz-security-token";

    // Canonical request: normalized representation of the request to sign
    let canonical_request = format!(
        "{}\n{}\n\n{}\n{}\n{}",
        input.method, input.path, canonical_headers, signed_headers, payload_hash
    );

    // Scope binds the signature to a specific date, region, and service
    let scope = format!(
        "{}/{}/{}/aws4_request",
        date_stamp, input.region, input.service
    );
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        amz_date,
        scope,
        hex::encode(Sha256::digest(canonical_request.as_bytes()))
    );

    // Key derivation chain: each step scopes the key more narrowly
    // AWS4 + secret -> date -> region -> service -> signing key
    let k_date = hmac_sha256(
        format!("AWS4{}", input.secret_key).as_bytes(),
        date_stamp.as_bytes(),
    );
    let k_region = hmac_sha256(&k_date, input.region.as_bytes());
    let k_service = hmac_sha256(&k_region, input.service.as_bytes());
    let k_signing = hmac_sha256(&k_service, b"aws4_request");
    let signature = hex::encode(hmac_sha256(&k_signing, string_to_sign.as_bytes()));

    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        input.access_key, scope, signed_headers, signature
    );

    SignedRequest {
        authorization,
        x_amz_date: amz_date,
        x_amz_security_token: input.token.to_string(),
        x_amz_content_sha256: payload_hash,
    }
}

/// Computes HMAC-SHA256 for the key derivation chain.
fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}
