/*
 *  Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 *  Copyright (C) 2021-2026 Systemd Authors
 *
 *  SPDX-License-Identifier: LGPL-2.1-or-later
 *  Originally derived from:
 *  https://github.com/systemd/systemd/blob/7e37e01768e2f223750ead2c9e08b4490243b8d1/src/shared/creds-util.c
 *  https://github.com/systemd/systemd/blob/7e37e01768e2f223750ead2c9e08b4490243b8d1/src/shared/creds-util.h
 *
 */

//! Systemd credential format structures and parsing
//!
//! This module implements parsing and serialization for systemd's encrypted credential format.
//! Credentials are encrypted using AES-256-GCM and can be sealed to:
//! - Host key (stored in /var/lib/systemd/credential.secret)
//! - TPM2 HMAC (sealed to PCR values)
//! - Both host and TPM2 keys combined
//! - Null key (no confidentiality, integrity only)
//!
//! The binary format consists of:
//! 1. Main header (encryption type, key/block/IV/tag sizes, IV data)
//! 2. Optional TPM2 header (PCR mask, sealed blob, policy hash)
//! 3. Optional TPM2 public key header (for signed PCR policies)
//! 4. Optional scoped header (for user-scoped credentials)
//! 5. Encrypted data (metadata + payload + GCM authentication tag)
//!
//! All sections are aligned to 8-byte boundaries.

use base64::{Engine, engine::general_purpose};
use binrw::{BinRead, BinResult, BinWrite};
use hex_literal::hex;
use serde::{Deserialize, Serialize, Serializer, de::Error};
use snafu::prelude::*;
use std::io::Cursor;
use std::str;
use zeroize::{Zeroize, ZeroizeOnDrop};

type Result<T> = std::result::Result<T, snafu::Whatever>;
type SerdeResult<T, E> = std::result::Result<T, E>;

/// Parsed systemd encrypted credential
///
/// Represents the complete structure of a systemd encrypted credential file,
/// including the main header, optional TPM2 headers, and encrypted payload.
///
/// The encrypted_data field contains: metadata header + credential name + payload + GCM tag
#[derive(BinRead, BinWrite, Debug, Serialize, Deserialize, ZeroizeOnDrop)]
#[brw(little)]
pub(crate) struct ParsedCredential {
    /// Encryption type/method used for this credential
    encryption_type: EncryptionType,
    /// Size of the encryption key in bytes (typically 32 for AES-256)
    key_size: u32,
    /// Block size for the cipher in bytes
    block_size: u32,
    /// Size of the initialization vector in bytes
    iv_size: u32,
    /// Size of the GCM authentication tag in bytes
    tag_size: u32,

    /// Initialization vector for AES-GCM encryption
    #[br(count = iv_size)]
    #[serde(serialize_with = "serialize_hex", deserialize_with = "deserialize_hex")]
    iv: Vec<u8>,

    /// TPM2 header (present if encryption type requires TPM2)
    #[brw(align_before = 8)]
    #[brw(if(encryption_type.requires_tpm2()))]
    tpm2_header: Option<Tpm2Header>,

    /// TPM2 public key header (present if using signed PCR policy)
    #[brw(align_before = 8)]
    #[brw(if(encryption_type.requires_tpm2_pk()))]
    tpm2_pubkey_header: Option<Tpm2PublicKeyHeader>,

    /// Scoped header (present if credential is user-scoped)
    #[brw(align_before = 8)]
    #[brw(if(encryption_type.is_scoped()))]
    scoped_header: Option<ScopedHeader>,

    /// Encrypted data: metadata header (16 bytes) + credential name + payload + GCM tag
    /// The metadata header contains timestamp, not_after, and name_size fields
    #[br(parse_with = binrw::helpers::until_eof)]
    #[serde(serialize_with = "serialize_hex", deserialize_with = "deserialize_hex")]
    encrypted_data: Vec<u8>,
}

impl ParsedCredential {
    /// Parse a credential from bytes, attempting base64 decode first
    pub(crate) fn from_bytes(data: &[u8]) -> Result<Self> {
        // Try to decode as base64 first (systemd-creds stores credentials base64-encoded)
        let decoded = Self::try_base64_decode(data).unwrap_or_else(|_| data.to_vec());

        let mut cursor = Cursor::new(&decoded);
        Self::read_le(&mut cursor).whatever_context("failed to parse credential structure")
    }

    /// Attempt to decode base64, handling both standard and URL-safe variants
    fn try_base64_decode(data: &[u8]) -> Result<Vec<u8>> {
        // Convert to string, removing whitespace
        let s = str::from_utf8(data).whatever_context("invalid UTF-8 in credential data")?;
        let s = s.chars().filter(|c| !c.is_whitespace()).collect::<String>();

        // Try standard base64 first
        if let Ok(decoded) = general_purpose::STANDARD.decode(&s) {
            return Ok(decoded);
        }

        // Try URL-safe base64
        if let Ok(decoded) = general_purpose::URL_SAFE.decode(&s) {
            return Ok(decoded);
        }

        whatever!("failed to decode base64 data")
    }
}

/// Validate that a credential is TPM2 HMAC encrypted with the expected PCR mask
pub(crate) fn validate_tpm2_hmac(data: &[u8], expected_pcrs: &[u32]) -> Result<()> {
    let parsed = ParsedCredential::from_bytes(data)?;

    ensure_whatever!(
        parsed.encryption_type == EncryptionType::Tpm2Hmac,
        "expected Tpm2Hmac encryption, found {:?}",
        parsed.encryption_type
    );

    let tpm2_header = parsed
        .tpm2_header
        .as_ref()
        .whatever_context("missing TPM2 header")?;

    let expected_mask = pcr_list_to_mask(expected_pcrs);
    let actual_pcrs = pcr_mask_to_list(tpm2_header.pcr_mask);
    ensure_whatever!(
        tpm2_header.pcr_mask == expected_mask,
        "PCR mask mismatch: expected {:?}, found {:?}",
        expected_pcrs,
        actual_pcrs
    );

    Ok(())
}

/// Encryption type identifier for systemd credentials
///
/// Each type uses a unique 128-bit UUID to identify the encryption method.
/// The encryption key is derived from one or more sources:
/// - Host: Key stored in /var/lib/systemd/credential.secret
/// - TPM2: HMAC key sealed to TPM2 PCR values
/// - Null: Empty key (provides integrity but no confidentiality)
///
/// Scoped variants derive a per-user key by HMAC'ing the base key with
/// the user's UID, username, and machine ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
enum EncryptionType {
    /// Encrypted with host key only
    Host,
    /// Encrypted with host key, scoped to a specific user
    HostScoped,
    /// Encrypted with TPM2 HMAC key sealed to fixed PCR values
    Tpm2Hmac,
    /// Encrypted with TPM2 HMAC key using signed PCR policy
    Tpm2HmacWithPk,
    /// Encrypted with both host and TPM2 HMAC keys
    HostAndTpm2Hmac,
    /// Encrypted with host and TPM2 keys, scoped to a specific user
    HostAndTpm2HmacScoped,
    /// Encrypted with host and TPM2 keys using signed PCR policy
    HostAndTpm2HmacWithPk,
    /// Encrypted with host and TPM2 keys using signed PCR policy, user-scoped
    HostAndTpm2HmacWithPkScoped,
    /// Encrypted with null key (integrity only, no confidentiality)
    Null,
    /// Unknown encryption type with raw UUID
    Unknown([u8; 16]),
}

/// UUID constants for systemd credential encryption types
///
/// These correspond to the CRED_AES256_GCM_BY_* constants in systemd's creds-util.h
const UUID_HOST: [u8; 16] = hex!("5a1c6a86df9d4096b1d5a65e0862f19a");
const UUID_HOST_SCOPED: [u8; 16] = hex!("55b9ed1d38594d43a8319d2ebb332ac6");
const UUID_TPM2_HMAC: [u8; 16] = hex!("0c7cc07b117645919c4b0bea08bc20fe");
const UUID_TPM2_HMAC_WITH_PK: [u8; 16] = hex!("faf7eb9341e3412ca1a436f95a29362f");
const UUID_HOST_AND_TPM2_HMAC: [u8; 16] = hex!("93a894094874449090caf2fc93cab553");
const UUID_HOST_AND_TPM2_HMAC_SCOPED: [u8; 16] = hex!("ef4ac13679a9480ea7db68897f9f165d");
const UUID_HOST_AND_TPM2_HMAC_WITH_PK: [u8; 16] = hex!("af4950a849134eb1a73846304ff30c05");
const UUID_HOST_AND_TPM2_HMAC_WITH_PK_SCOPED: [u8; 16] = hex!("adbc4ca3efb64201ba881b6f2e4095ea");
const UUID_NULL: [u8; 16] = hex!("058469daf6f54324800549da0f8ea2fb");

impl EncryptionType {
    /// Convert a 128-bit UUID to an encryption type
    fn from_id(id: [u8; 16]) -> Self {
        match id {
            UUID_HOST => Self::Host,
            UUID_HOST_SCOPED => Self::HostScoped,
            UUID_TPM2_HMAC => Self::Tpm2Hmac,
            UUID_TPM2_HMAC_WITH_PK => Self::Tpm2HmacWithPk,
            UUID_HOST_AND_TPM2_HMAC => Self::HostAndTpm2Hmac,
            UUID_HOST_AND_TPM2_HMAC_SCOPED => Self::HostAndTpm2HmacScoped,
            UUID_HOST_AND_TPM2_HMAC_WITH_PK => Self::HostAndTpm2HmacWithPk,
            UUID_HOST_AND_TPM2_HMAC_WITH_PK_SCOPED => Self::HostAndTpm2HmacWithPkScoped,
            UUID_NULL => Self::Null,
            _ => Self::Unknown(id),
        }
    }

    /// Returns true if this encryption type requires TPM2 hardware
    fn requires_tpm2(&self) -> bool {
        matches!(
            self,
            Self::Tpm2Hmac
                | Self::Tpm2HmacWithPk
                | Self::HostAndTpm2Hmac
                | Self::HostAndTpm2HmacScoped
                | Self::HostAndTpm2HmacWithPk
                | Self::HostAndTpm2HmacWithPkScoped
        )
    }

    /// Returns true if this encryption type requires a TPM2 public key for signed PCR policy
    fn requires_tpm2_pk(&self) -> bool {
        matches!(
            self,
            Self::Tpm2HmacWithPk | Self::HostAndTpm2HmacWithPk | Self::HostAndTpm2HmacWithPkScoped
        )
    }

    /// Returns true if this encryption type is user-scoped
    fn is_scoped(&self) -> bool {
        matches!(
            self,
            Self::HostScoped | Self::HostAndTpm2HmacScoped | Self::HostAndTpm2HmacWithPkScoped
        )
    }
}

impl BinRead for EncryptionType {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> BinResult<Self> {
        let id = <[u8; 16]>::read_options(reader, endian, args)?;
        Ok(Self::from_id(id))
    }
}

impl BinWrite for EncryptionType {
    type Args<'a> = ();

    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> BinResult<()> {
        let id = match self {
            Self::Host => UUID_HOST,
            Self::HostScoped => UUID_HOST_SCOPED,
            Self::Tpm2Hmac => UUID_TPM2_HMAC,
            Self::Tpm2HmacWithPk => UUID_TPM2_HMAC_WITH_PK,
            Self::HostAndTpm2Hmac => UUID_HOST_AND_TPM2_HMAC,
            Self::HostAndTpm2HmacScoped => UUID_HOST_AND_TPM2_HMAC_SCOPED,
            Self::HostAndTpm2HmacWithPk => UUID_HOST_AND_TPM2_HMAC_WITH_PK,
            Self::HostAndTpm2HmacWithPkScoped => UUID_HOST_AND_TPM2_HMAC_WITH_PK_SCOPED,
            Self::Null => UUID_NULL,
            Self::Unknown(id) => *id,
        };
        id.write_options(writer, endian, args)
    }
}

serde_plain::derive_fromstr_from_deserialize!(EncryptionType);
serde_plain::derive_display_from_serialize!(EncryptionType);

/// TPM2-specific header for credentials sealed to TPM2 PCR values
///
/// Contains the TPM2 sealed blob and policy hash. The blob is created by sealing
/// a random key to specific PCR values. At decryption, TPM2 only unseals if current
/// PCR values match the policy.
///
/// Corresponds to the TPM2 metadata in systemd's encrypted credential format.
#[derive(BinRead, BinWrite, Debug, Serialize, Deserialize, ZeroizeOnDrop)]
#[brw(little)]
struct Tpm2Header {
    /// Bitmask of PCRs used for sealing (e.g., 0b0000_0111 = PCRs 0,1,2)
    #[serde(
        serialize_with = "serialize_pcr_mask",
        deserialize_with = "deserialize_pcr_mask"
    )]
    pcr_mask: u64,
    /// Hash algorithm used for PCR bank
    pcr_bank: PcrBank,
    /// TPM2 primary key algorithm
    primary_alg: PrimaryAlg,
    /// Size of the TPM2 sealed blob in bytes
    blob_size: u32,
    /// Size of the TPM2 policy hash in bytes
    policy_hash_size: u32,
    /// TPM2 sealed blob containing the encryption key
    #[br(count = blob_size)]
    #[serde(serialize_with = "serialize_hex", deserialize_with = "deserialize_hex")]
    blob: Vec<u8>,
    /// TPM2 policy hash for authorization
    #[br(count = policy_hash_size)]
    #[serde(serialize_with = "serialize_hex", deserialize_with = "deserialize_hex")]
    policy_hash: Vec<u8>,
}

/// TPM2 public key header for signed PCR policies
///
/// Used when credentials are sealed with a signed PCR policy, allowing
/// PCR values to be updated without re-encrypting the credential.
#[derive(BinRead, BinWrite, Debug, Serialize, Deserialize, ZeroizeOnDrop)]
#[brw(little)]
struct Tpm2PublicKeyHeader {
    /// Bitmask of PCRs covered by the signed policy
    #[serde(
        serialize_with = "serialize_pcr_mask",
        deserialize_with = "deserialize_pcr_mask"
    )]
    pcr_mask: u64,
    /// Size of the public key data in bytes
    size: u32,
    /// Public key data for verifying PCR policy signatures
    #[br(count = size)]
    #[serde(serialize_with = "serialize_hex", deserialize_with = "deserialize_hex")]
    data: Vec<u8>,
}

/// Scoped credential header for user-specific credentials
///
/// Contains flags indicating the scope (e.g., per-user) of the credential.
#[derive(BinRead, BinWrite, Debug, Serialize, Deserialize, ZeroizeOnDrop)]
#[brw(little)]
struct ScopedHeader {
    /// Flags indicating credential scope
    flags: u64,
}

/// TPM2 primary key algorithm used for sealing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
enum PrimaryAlg {
    /// RSA algorithm (TPM_ALG_RSA = 0x0001)
    #[serde(rename = "RSA")]
    Rsa,
    /// ECC algorithm (TPM_ALG_ECC = 0x0023)
    #[serde(rename = "ECC")]
    Ecc,
    /// Unknown or unsupported algorithm
    #[serde(rename = "Unknown")]
    Unknown(u16),
}

impl PrimaryAlg {
    fn from_u16(value: u16) -> Self {
        match value {
            0x01 => Self::Rsa,
            0x23 => Self::Ecc,
            _ => Self::Unknown(value),
        }
    }

    fn to_u16(self) -> u16 {
        match self {
            Self::Rsa => 0x01,
            Self::Ecc => 0x23,
            Self::Unknown(v) => v,
        }
    }
}

impl BinRead for PrimaryAlg {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> BinResult<Self> {
        let value = u16::read_options(reader, endian, args)?;
        Ok(Self::from_u16(value))
    }
}

impl BinWrite for PrimaryAlg {
    type Args<'a> = ();

    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> BinResult<()> {
        self.to_u16().write_options(writer, endian, args)
    }
}

serde_plain::derive_fromstr_from_deserialize!(PrimaryAlg);
serde_plain::derive_display_from_serialize!(PrimaryAlg);

/// TPM2 PCR bank (hash algorithm) used for sealing credentials
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
enum PcrBank {
    /// SHA-1 hash algorithm (TPM_ALG_SHA1 = 0x0004)
    #[serde(rename = "SHA1")]
    Sha1,
    /// SHA-256 hash algorithm (TPM_ALG_SHA256 = 0x000B)
    #[serde(rename = "SHA256")]
    Sha256,
    /// SHA-384 hash algorithm (TPM_ALG_SHA384 = 0x000C)
    #[serde(rename = "SHA384")]
    Sha384,
    /// SHA-512 hash algorithm (TPM_ALG_SHA512 = 0x000D)
    #[serde(rename = "SHA512")]
    Sha512,
    /// Unknown or unsupported hash algorithm
    #[serde(rename = "Unknown")]
    Unknown(u16),
}

impl PcrBank {
    fn from_u16(value: u16) -> Self {
        match value {
            0x04 => Self::Sha1,
            0x0b => Self::Sha256,
            0x0c => Self::Sha384,
            0x0d => Self::Sha512,
            _ => Self::Unknown(value),
        }
    }

    fn to_u16(self) -> u16 {
        match self {
            Self::Sha1 => 0x04,
            Self::Sha256 => 0x0b,
            Self::Sha384 => 0x0c,
            Self::Sha512 => 0x0d,
            Self::Unknown(v) => v,
        }
    }
}

impl BinRead for PcrBank {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> BinResult<Self> {
        let value = u16::read_options(reader, endian, args)?;
        Ok(Self::from_u16(value))
    }
}

impl BinWrite for PcrBank {
    type Args<'a> = ();

    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> BinResult<()> {
        self.to_u16().write_options(writer, endian, args)
    }
}

serde_plain::derive_fromstr_from_deserialize!(PcrBank);
serde_plain::derive_display_from_serialize!(PcrBank);

/// Serialize a byte array as a hex string with 0x prefix
fn serialize_hex<S>(bytes: &[u8], serializer: S) -> SerdeResult<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format!("0x{}", hex::encode(bytes)))
}

/// Deserialize a hex string (with or without 0x prefix) to bytes
fn deserialize_hex<'de, D>(deserializer: D) -> SerdeResult<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let s = s.strip_prefix("0x").unwrap_or(&s);
    hex::decode(s).map_err(D::Error::custom)
}

/// Serialize PCR mask as a list of PCR numbers for JSON output
fn serialize_pcr_mask<S>(mask: &u64, serializer: S) -> SerdeResult<S::Ok, S::Error>
where
    S: Serializer,
{
    pcr_mask_to_list(*mask).serialize(serializer)
}

/// Deserialize a list of PCR numbers back to a bitmask
fn deserialize_pcr_mask<'de, D>(deserializer: D) -> SerdeResult<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let pcrs: Vec<u32> = Deserialize::deserialize(deserializer)?;
    Ok(pcr_list_to_mask(&pcrs))
}

/// Convert a PCR bitmask to a list of PCR numbers
fn pcr_mask_to_list(mask: u64) -> Vec<u32> {
    (0..64).filter(|i| mask & (1 << i) != 0).collect()
}

/// Convert a list of PCR numbers to a bitmask
fn pcr_list_to_mask(pcrs: &[u32]) -> u64 {
    pcrs.iter().fold(0u64, |acc, &pcr| acc | (1 << pcr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine, engine::general_purpose};

    use binrw::BinRead;
    use binrw::BinWrite;

    use std::io::Cursor;
    use test_case::test_case;

    const TEST_CRED_NULL: &str = "BYRp2vb1QySABUnaD46i+yAAAAABAAAADAAAABAAAAATPU8srWAq3mtWrGkAAAAAb3P+yO/nQ2tRS+zpGHvQ8Jffr3a9SizoK5fzgIdgPxngfrszhbnh06X70Z+O1MObn+Jug1bwyvf1PBLNPdJevk8=";
    const TEST_CRED_HOST: &str = "Whxqht+dQJax1aZeCGLxmiAAAAABAAAADAAAABAAAABuACQdV2GLhe0fc8IAAAAAhqpZ5GFTeZCZh4YSKvZ4TG8+SnHrIMduVkhbim5KDU7weMALI2GYks0GtAc1+HZraySbYV5klqXQwqlGvFFor6sl";
    const TEST_CRED_HOST_SCOPED: &str = "VbntHThZTUOoMZ0uuzMqxiAAAAABAAAADAAAABAAAACsr1L/GQT7Ec6jyREAAAAABwAAAAAAAAALOLwic485DC2MU64Nxw+u2Vc3f1smLnndgkpXsc2SpTxQa0vpGBKR/VKrgR5/So44bzfZ42R+uyKUDwtUygezSvQmNEqiBCkYQb40oPgmKSY=";
    const TEST_CRED_TPM2: &str = "DHzAexF2RZGcSwvqCLwg/iAAAAABAAAADAAAABAAAACwL4g6mLs1KRACizoAAAAAgEgAAAAAAAALACMA8AAAACAAAAAAngAgCy4nQemrR8CRWFSvIa27VXsNfribuCyxTulHJEIJAEQAEHhKu845D580QKiryffnpiFy2okaY7/3/1teZpcYz7uSzhZygCf/7jVsVGohFseJQD2bZDXHoLlbamkVer5uHYqOhA6k45wp8jNSDnpyNegg1wPDW2l/7nG9SlcwP6ydYeGJqDgr5XvkfL6aKPb7PzaKyDHS6pwKRaRZAE4ACAQLAAAEEgAgPCLw7Y+QTu3anSLQAz/AdihEDbxRyFRBjJlurO3yJGYAEAAgnITcQgel3etyLepe53Hvzt4yFTyLtHi+IeBBWMWpr9M8IvDtj5BO7dqdItADP8B2KEQNvFHIVEGMmW6s7fIkZgAAAAAQxnOVemcSI6hw8np7pKCysTwF9+x91U1R3WbZhSHiRJjBl7dlNJe6XqyHRVcmpuMwP94mcd2POgpIzc+k1IPkeXsC46yc5HUH03DH0lN5xLpXe/0q33SRPTa8uCVaAyX2V+c4JlL+oR9AxEHeYk66";
    const TEST_CRED_TPM2_PK: &str = "+vfrk0HjQSyhpDb5Wik2LyAAAAABAAAADAAAABAAAADr/l6SPu4hgcyb8hsAAAAAgAAAAAAAAAALACMA8AAAACAAAAAAngAgXEtd5ZKymJc4LAUa24ihiDpMwenVj+2cx54ndzbLt2EAEPZ2qz3k1NKXdVubpcUoPiUHcQmmyADucLIPQdSMbtOBiZDJnewqiV4huRajKq/qyAP8TaCF5Cy/sSlrmgk3gbNyiUp/pWHjkysPpxLDGgOfgSQmTnVJQ/YvUGrfcz10ANZ5bDM0fiRLUuT7N3J7jHhidSeg5gxboWzfAE4ACAALAAAEEgAgzMd0m0Ud4W/pko5DzNAYtpqKVP/aELu40y6ytxXVtFoAEAAg1XfuDJrHZGKNvFZ9CmOxpmbXQOTbFIptMLG9kXHGqwXMx3SbRR3hb+mSjkPM0Bi2mopU/9oQu7jTLrK3FdW0WgAAAAAACAAAAAAAAMMBAAAtLS0tLUJFR0lOIFBVQkxJQyBLRVktLS0tLQpNSUlCSWpBTkJna3Foa2lHOXcwQkFRRUZBQU9DQVE4QU1JSUJDZ0tDQVFFQXpOK0NDNGJ4aCtVQ3lQQVNzT2MrCkx2Zm5EdEU4bXBRalZMcFJVS1o5aTRLZlE4UGhqUlY5Kyt5SWY4S2t5NWJQMnFlMkt0Mk9OMlpPZWkyNUg4cEwKekRQUzV4SUtPTksvNTBTVDg3NjRDVjBnanVTSExkMXFJSVZsUUE1bkUzcEluU1h4R2pDeHRCQW5OOWNaRkNpeApTdWdER29ROThwVk9PUWJJSHBYZHR0VWZRSmVMRGhkNlB4ZmRtanMvdG9QbmdWTmVRVG5ZaXo2T3NCZ0VmNm5oClVMYkZTOHlhNGd6WmRKR0xZcHdQUmt6YUtRQ1JXQkhXOEp4VXVPUEpPdVJ6TUhjWW11ci9RWEdXTGEyeEZlcjEKQ05Od0tEczhvMDNucFFOczFWcVlwQndhM2J5Qk9NU1FkSk1oUVV5U0VpY1F1a1dBN3QwY0JwVnpRTlM5VFFDUwp0d0lEQVFBQgotLS0tLUVORCBQVUJMSUMgS0VZLS0tLS0KACeqyiAxYPRchoIUhLacmLm8i2s0EozluIWleMivmvCbL2lbjg0uWagbzFnUPktEWNMz1EdPPOP+sDfdHSlPDaTw7V0grg==";
    const TEST_CRED_HOST_TPM2: &str = "k6iUCUh0RJCQyvL8k8q1UyAAAAABAAAADAAAABAAAAAc6j5b3UjlfGGuwgoAAAAAgAAAAAAAAAALACMA8AAAACAAAAAAngAgRULYCfz4rcbWuYQ9n9Mpj8sH44iEzs8JxnUBJI5Y45QAEGN4WkhMiMPu0sEmjdRgzyQ+206EKqgCaQwqYWUkao0d4z4aNfXqOYQP4OTli/mSS+960q7TnuEzY3rR8WhBDP6+0Cg7+i+WqD5+/x5LfiAMUHM4JAJF4pyHvauPEaBK0FIvlhuYvO7uafRO+maaw7o3fKXgrFYIy86/AE4ACAALAAAEEgAg8iJDoUtcVQ9l/zoZVoeBqDQw2CJeyuE6Z98IqPlTJb0AEAAgV/J/fU+Zfu2wtVC1f5MZiuf193nROgCS6o1P6Xg98w/yIkOhS1xVD2X/OhlWh4GoNDDYIl7K4Tpn3wio+VMlvQAAAACV56EMGxp9JjGTW+JekslX30ylk9re29gQgt+GmpdKAVkz0m9L0Q+X5OVusSFfP9Wl8+hFYzgs08BPwupraCHiAYU=";
    const TEST_CRED_HOST_TPM2_SCOPED: &str = "70rBNnmpSA6n22iJf58WXSAAAAABAAAADAAAABAAAACDq8reWt0mjtVQvAsAAAAAgAAAAAAAAAALACMA8AAAACAAAAAAngAg9sCnvuaDetNfPQFADWFbCNHJ20by95yBrVKorCXjy+YAEMxvT/qs/i6e2618Cp6ArqWKfVrHhhs0wZ8qnRnEtPimM2ujRrX++b523Hum/YEVlUot3G6eCn40Mge5u28FvQbMIKIHd0Ta6dHmTdI18kwSQEuv5wVvPZiL8mbbA3zjEHw741t3Qc0Nl0I9UBXGkHU0LPCGStQsxc+HAE4ACAALAAAEEgAg8iJDoUtcVQ9l/zoZVoeBqDQw2CJeyuE6Z98IqPlTJb0AEAAgsMFQiG2Oao/Z9wPrsa6fmg3F5LUOcT/Tx9egzh9YBnfyIkOhS1xVD2X/OhlWh4GoNDDYIl7K4Tpn3wio+VMlvQAAAAAHAAAAAAAAANx7GNuQDUPpYw/3PoFGgXx9N8yXDACcmBW3S1caSurSmr+0VZRW0qf5owKxyExBxQFIhzBFa8PY15mGxyABJCaR/JrJDeXC/3Oe5eX4hTuMGg==";
    const TEST_CRED_HOST_TPM2_PK: &str = "r0lQqEkTTrGnOEYwT/MMBSAAAAABAAAADAAAABAAAAAOPM2bh8TYfQZabS4AAAAAgAAAAAAAAAALACMA8AAAACAAAAAAngAg+4OBD0KI8a/F2QVD5T4bkYJ1+91xmDc9lU+d4GLbATIAEPmc/sqtwShsliB/tvNj3SVIqMEWkgt1eYr4KKYMX0kRvhQ91aENQrDFMtHaoNe4+Fcj6KV4CDAgeAIFe+Fhrdr/6OErMvOCzuaFtF22vmIhD/iUeMGugX1i3L7VCwFl9c97RpczIE8OSr5JRgHv1dz02OX84mzW65MjAE4ACAALAAAEEgAgzMd0m0Ud4W/pko5DzNAYtpqKVP/aELu40y6ytxXVtFoAEAAgZflj1qxOgOX6yNtBp6VZyHW7e1e/1P0t64Y7ioj6WvXMx3SbRR3hb+mSjkPM0Bi2mopU/9oQu7jTLrK3FdW0WgAAAAAACAAAAAAAAMMBAAAtLS0tLUJFR0lOIFBVQkxJQyBLRVktLS0tLQpNSUlCSWpBTkJna3Foa2lHOXcwQkFRRUZBQU9DQVE4QU1JSUJDZ0tDQVFFQXpOK0NDNGJ4aCtVQ3lQQVNzT2MrCkx2Zm5EdEU4bXBRalZMcFJVS1o5aTRLZlE4UGhqUlY5Kyt5SWY4S2t5NWJQMnFlMkt0Mk9OMlpPZWkyNUg4cEwKekRQUzV4SUtPTksvNTBTVDg3NjRDVjBnanVTSExkMXFJSVZsUUE1bkUzcEluU1h4R2pDeHRCQW5OOWNaRkNpeApTdWdER29ROThwVk9PUWJJSHBYZHR0VWZRSmVMRGhkNlB4ZmRtanMvdG9QbmdWTmVRVG5ZaXo2T3NCZ0VmNm5oClVMYkZTOHlhNGd6WmRKR0xZcHdQUmt6YUtRQ1JXQkhXOEp4VXVPUEpPdVJ6TUhjWW11ci9RWEdXTGEyeEZlcjEKQ05Od0tEczhvMDNucFFOczFWcVlwQndhM2J5Qk9NU1FkSk1oUVV5U0VpY1F1a1dBN3QwY0JwVnpRTlM5VFFDUwp0d0lEQVFBQgotLS0tLUVORCBQVUJMSUMgS0VZLS0tLS0KAHU4LyqVyGZfdeNitbe7p9Ycol2sGR2jHFhNojU20RFySitY4tkVArDXzp08UaIRqulBg6AlFxROWTDSYueyaadWQYXgkg==";
    const TEST_CRED_HOST_TPM2_PK_SCOPED: &str = "rbxMo++2QgG6iBtvLkCV6iAAAAABAAAADAAAABAAAABnM7Vsx0eBsO4oC+AAAAAAgAAAAAAAAAALACMA8AAAACAAAAAAngAgVJglupflu/1r6BPo/Kx3FBaZxvkzNTnvYNbuuOQ+K+4AELDrnjG7ZNHTZqcgFePAdlrLLXpHrV91WTR4EsPXSa+w16l6RCabdfzbX/xkq0Z+bTiCNaxYtq3YzPYcor3ldpMB85er8MISzJWka7q+1j1NaSXtuI20zchmvQFUYW8ZScGQdm0/26UZX1A+7EO5Y5M+TCF+rFyMeKjPAE4ACAALAAAEEgAgzMd0m0Ud4W/pko5DzNAYtpqKVP/aELu40y6ytxXVtFoAEAAgh4mzAtHOBi8U0FiIyth5c6f+G0I56yHFKBvgIf1BwlDMx3SbRR3hb+mSjkPM0Bi2mopU/9oQu7jTLrK3FdW0WgAAAAAACAAAAAAAAMMBAAAtLS0tLUJFR0lOIFBVQkxJQyBLRVktLS0tLQpNSUlCSWpBTkJna3Foa2lHOXcwQkFRRUZBQU9DQVE4QU1JSUJDZ0tDQVFFQXpOK0NDNGJ4aCtVQ3lQQVNzT2MrCkx2Zm5EdEU4bXBRalZMcFJVS1o5aTRLZlE4UGhqUlY5Kyt5SWY4S2t5NWJQMnFlMkt0Mk9OMlpPZWkyNUg4cEwKekRQUzV4SUtPTksvNTBTVDg3NjRDVjBnanVTSExkMXFJSVZsUUE1bkUzcEluU1h4R2pDeHRCQW5OOWNaRkNpeApTdWdER29ROThwVk9PUWJJSHBYZHR0VWZRSmVMRGhkNlB4ZmRtanMvdG9QbmdWTmVRVG5ZaXo2T3NCZ0VmNm5oClVMYkZTOHlhNGd6WmRKR0xZcHdQUmt6YUtRQ1JXQkhXOEp4VXVPUEpPdVJ6TUhjWW11ci9RWEdXTGEyeEZlcjEKQ05Od0tEczhvMDNucFFOczFWcVlwQndhM2J5Qk9NU1FkSk1oUVV5U0VpY1F1a1dBN3QwY0JwVnpRTlM5VFFDUwp0d0lEQVFBQgotLS0tLUVORCBQVUJMSUMgS0VZLS0tLS0KAAcAAAAAAAAAu88CqoLt4hAGrueYixcusMt0mKH0OZngkM/3rm5hriZRr+pBgp9wfsBZ14KmctN5/zFmW+s37Sw3w6DWCYmwM9Zma+LgA8hOKOQuUqLMOOKptSmo";

    #[test_case(TEST_CRED_NULL, EncryptionType::Null, false ; "null encryption")]
    #[test_case(TEST_CRED_HOST, EncryptionType::Host, false ; "host encryption")]
    #[test_case(TEST_CRED_HOST_SCOPED, EncryptionType::HostScoped, false ; "host scoped")]
    #[test_case(TEST_CRED_TPM2, EncryptionType::Tpm2Hmac, true ; "tpm2 hmac")]
    #[test_case(TEST_CRED_TPM2_PK, EncryptionType::Tpm2HmacWithPk, true ; "tpm2 with public key")]
    #[test_case(TEST_CRED_HOST_TPM2, EncryptionType::HostAndTpm2Hmac, true ; "host and tpm2")]
    #[test_case(TEST_CRED_HOST_TPM2_SCOPED, EncryptionType::HostAndTpm2HmacScoped, true ; "host and tpm2 scoped")]
    #[test_case(TEST_CRED_HOST_TPM2_PK, EncryptionType::HostAndTpm2HmacWithPk, true ; "host and tpm2 with public key")]
    #[test_case(TEST_CRED_HOST_TPM2_PK_SCOPED, EncryptionType::HostAndTpm2HmacWithPkScoped, true ; "host and tpm2 with public key scoped")]
    fn test_from_bytes(base64: &str, expected_type: EncryptionType, has_tpm2: bool) {
        let parsed = ParsedCredential::from_bytes(base64.as_bytes()).unwrap();
        assert_eq!(parsed.encryption_type, expected_type);
        assert_eq!(parsed.tpm2_header.is_some(), has_tpm2);
    }

    #[test_case(TEST_CRED_NULL ; "null")]
    #[test_case(TEST_CRED_HOST ; "host")]
    #[test_case(TEST_CRED_HOST_SCOPED ; "host scoped")]
    #[test_case(TEST_CRED_TPM2 ; "tpm2")]
    #[test_case(TEST_CRED_TPM2_PK ; "tpm2 pk")]
    #[test_case(TEST_CRED_HOST_TPM2 ; "host tpm2")]
    #[test_case(TEST_CRED_HOST_TPM2_SCOPED ; "host tpm2 scoped")]
    #[test_case(TEST_CRED_HOST_TPM2_PK ; "host tpm2 pk")]
    #[test_case(TEST_CRED_HOST_TPM2_PK_SCOPED ; "host tpm2 pk scoped")]
    fn test_serialize_deserialize_roundtrip(base64: &str) {
        let decoded = general_purpose::STANDARD.decode(base64).unwrap();
        let mut cursor = Cursor::new(&decoded);
        let parsed = ParsedCredential::read_le(&mut cursor).unwrap();

        let json = serde_json::to_string(&parsed).unwrap();
        let deserialized: ParsedCredential = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.encryption_type, deserialized.encryption_type);
        assert_eq!(parsed.key_size, deserialized.key_size);
        assert_eq!(parsed.iv, deserialized.iv);
        assert_eq!(parsed.encrypted_data, deserialized.encrypted_data);
    }

    #[test_case(TEST_CRED_NULL ; "null")]
    #[test_case(TEST_CRED_HOST ; "host")]
    #[test_case(TEST_CRED_HOST_SCOPED ; "host scoped")]
    #[test_case(TEST_CRED_TPM2 ; "tpm2")]
    #[test_case(TEST_CRED_TPM2_PK ; "tpm2 pk")]
    #[test_case(TEST_CRED_HOST_TPM2 ; "host tpm2")]
    #[test_case(TEST_CRED_HOST_TPM2_SCOPED ; "host tpm2 scoped")]
    #[test_case(TEST_CRED_HOST_TPM2_PK ; "host tpm2 pk")]
    #[test_case(TEST_CRED_HOST_TPM2_PK_SCOPED ; "host tpm2 pk scoped")]
    fn test_binary_encoding_roundtrip(base64: &str) {
        let original_decoded = general_purpose::STANDARD.decode(base64).unwrap();
        let mut cursor = Cursor::new(&original_decoded);
        let parsed = ParsedCredential::read_le(&mut cursor).unwrap();

        let mut output = Cursor::new(Vec::new());
        parsed.write_le(&mut output).unwrap();
        let reencoded = output.into_inner();

        assert_eq!(original_decoded, reencoded);
    }

    #[test_case(0b0000_0001, vec![0] ; "single pcr 0")]
    #[test_case(0b0000_0010, vec![1] ; "single pcr 1")]
    #[test_case(0b0000_0101, vec![0, 2] ; "pcr 0 and 2")]
    #[test_case(0b1111_1111, vec![0, 1, 2, 3, 4, 5, 6, 7] ; "first 8 pcrs")]
    fn test_pcr_mask_to_list(mask: u64, expected: Vec<u32>) {
        assert_eq!(pcr_mask_to_list(mask), expected);
    }

    #[test_case(&[0], 0b0000_0001 ; "single pcr 0")]
    #[test_case(&[1], 0b0000_0010 ; "single pcr 1")]
    #[test_case(&[0, 2], 0b0000_0101 ; "pcr 0 and 2")]
    #[test_case(&[0, 1, 2, 3, 4, 5, 6, 7], 0b1111_1111 ; "first 8 pcrs")]
    fn test_pcr_list_to_mask(pcrs: &[u32], expected: u64) {
        assert_eq!(pcr_list_to_mask(pcrs), expected);
    }

    #[test_case(PcrBank::Sha1, "SHA1" ; "sha1")]
    #[test_case(PcrBank::Sha256, "SHA256" ; "sha256")]
    #[test_case(PcrBank::Sha384, "SHA384" ; "sha384")]
    #[test_case(PcrBank::Sha512, "SHA512" ; "sha512")]
    fn test_pcr_bank_display(bank: PcrBank, expected: &str) {
        assert_eq!(bank.to_string(), expected);
    }

    #[test_case(PrimaryAlg::Rsa, "RSA" ; "rsa")]
    #[test_case(PrimaryAlg::Ecc, "ECC" ; "ecc")]
    fn test_primary_alg_display(alg: PrimaryAlg, expected: &str) {
        assert_eq!(alg.to_string(), expected);
    }

    #[test]
    fn test_encryption_type_requires_tpm2() {
        assert!(!EncryptionType::Null.requires_tpm2());
        assert!(!EncryptionType::Host.requires_tpm2());
        assert!(EncryptionType::Tpm2Hmac.requires_tpm2());
        assert!(EncryptionType::HostAndTpm2Hmac.requires_tpm2());
    }

    #[test]
    fn test_encryption_type_requires_tpm2_pk() {
        assert!(!EncryptionType::Tpm2Hmac.requires_tpm2_pk());
        assert!(EncryptionType::Tpm2HmacWithPk.requires_tpm2_pk());
        assert!(EncryptionType::HostAndTpm2HmacWithPk.requires_tpm2_pk());
    }

    #[test]
    fn test_encryption_type_is_scoped() {
        assert!(!EncryptionType::Host.is_scoped());
        assert!(EncryptionType::HostScoped.is_scoped());
        assert!(EncryptionType::HostAndTpm2HmacScoped.is_scoped());
    }

    #[test]
    fn test_pcr_mask_empty() {
        assert_eq!(pcr_mask_to_list(0), Vec::<u32>::new());
        assert_eq!(pcr_list_to_mask(&[]), 0);
    }

    #[test]
    fn test_pcr_mask_high_bits() {
        assert_eq!(pcr_mask_to_list(1u64 << 63), vec![63]);
        assert_eq!(pcr_list_to_mask(&[63]), 1u64 << 63);
    }

    #[test]
    fn test_pcr_bank_roundtrip() {
        assert_eq!(PcrBank::from_u16(0x04).to_u16(), 0x04);
        assert_eq!(PcrBank::from_u16(0x0b).to_u16(), 0x0b);
        assert_eq!(PcrBank::from_u16(0x99).to_u16(), 0x99);
    }

    #[test]
    fn test_primary_alg_roundtrip() {
        assert_eq!(PrimaryAlg::from_u16(0x01).to_u16(), 0x01);
        assert_eq!(PrimaryAlg::from_u16(0x23).to_u16(), 0x23);
        assert_eq!(PrimaryAlg::from_u16(0xff).to_u16(), 0xff);
    }

    #[test]
    fn test_base64_decode_variants() {
        let data = b"test";
        let standard = general_purpose::STANDARD.encode(data);
        let url_safe = general_purpose::URL_SAFE.encode(data);

        assert!(ParsedCredential::try_base64_decode(standard.as_bytes()).is_ok());
        assert!(ParsedCredential::try_base64_decode(url_safe.as_bytes()).is_ok());
    }

    #[test]
    fn test_invalid_base64() {
        assert!(ParsedCredential::try_base64_decode(b"not valid base64!!!").is_err());
    }

    #[test]
    fn test_hex_serde_roundtrip() {
        let data = vec![0xde, 0xad, 0xbe, 0xef];
        let json = serde_json::json!({"data": format!("0x{}", hex::encode(&data))});
        let hex_str: String = serde_json::from_value(json["data"].clone()).unwrap();
        let decoded = hex::decode(hex_str.strip_prefix("0x").unwrap()).unwrap();
        assert_eq!(data, decoded);
    }
}
