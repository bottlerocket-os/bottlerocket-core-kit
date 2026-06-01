//! Centralized CryptoProvider for Bottlerocket Rust binaries.
//!
//! Provides runtime FIPS detection and TLS algorithm selection.
//! When the kernel FIPS flag is enabled (`/proc/sys/crypto/fips_enabled` = 1),
//! the provider restricts TLS to FIPS-approved algorithms only.
//!
//! The FIPS filtering uses rustls's `.fips()` trait method on each cipher suite
//! and key exchange group, which queries the underlying aws-lc-rs library to
//! determine FIPS approval. This means we automatically stay in sync with
//! upstream's FIPS classifications without maintaining a manual allowlist.
//!
//! # Usage
//!
//! Call [`install_provider`] at the very start of `main()`, before any TLS
//! connections are made. This installs the appropriate `CryptoProvider` as the
//! process-wide global default for all rustls usage, including downstream
//! libraries like `reqwest`, `hyper-rustls`, and `aws-smithy-http-client`.
//!
//! ```rust,ignore
//! fn main() {
//!     bottlerocket_crypto_provider::install_provider()
//!         .expect("failed to install crypto provider");
//!
//!     // All subsequent TLS connections automatically use the installed
//!     // provider — no per-client configuration needed.
//! }
//! ```
//!
//! If you need a [`CryptoProvider`] instance directly (e.g. to pass to
//! `ClientConfig::builder_with_provider()`), use [`provider`]:
//!
//! ```rust,ignore
//! use std::sync::Arc;
//!
//! let crypto = bottlerocket_crypto_provider::provider()
//!     .expect("failed to detect FIPS mode");
//! let config = rustls::ClientConfig::builder_with_provider(Arc::new(crypto))
//!     .with_safe_default_protocol_versions()
//!     .unwrap()
//!     .with_root_certificates(root_store)
//!     .with_no_client_auth();
//! ```
//!
//! # How it works
//!
//! 1. Reads `/proc/sys/crypto/fips_enabled` to detect if the kernel has FIPS
//!    mode enabled.
//! 2. If FIPS is enabled, returns a provider filtered to only FIPS-approved
//!    algorithms (AES-GCM cipher suites, P-256/P-384/X25519MLKEM768 key
//!    exchange groups).
//! 3. If FIPS is not enabled, returns the full default provider with all
//!    algorithms available (including ChaCha20 and X25519).
//!
//! # Important
//!
//! - [`install_provider`] MUST be called before any other code installs a
//!   `CryptoProvider`. The global default uses `OnceLock` and cannot be
//!   overridden once set. If another provider is already installed, it returns
//!   [`Error::ProviderAlreadyInstalled`].
//! - The `aws-lc-rs` dependency MUST be compiled with `features = ["fips"]` so
//!   that the underlying C library reports `FIPS_mode() == 1`. Without this,
//!   `.fips()` returns `false` for all algorithms, and `fips_provider()` would
//!   filter out everything — resulting in an empty provider with no cipher
//!   suites or kx groups, and all TLS connections would fail.

use log::info;
use rustls::crypto::{aws_lc_rs, CryptoProvider};
use snafu::{ResultExt, Snafu};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Failed to read /proc/sys/crypto/fips_enabled"))]
    ReadFipsEnabled { source: std::io::Error },

    #[snafu(display(
        "CryptoProvider already installed before install_provider() was called. \
         install_provider() must be the first thing called in main()."
    ))]
    ProviderAlreadyInstalled,
}

/// Detect whether the system is running in FIPS mode by reading the kernel flag.
pub fn fips_enabled() -> Result<bool, Error> {
    match std::fs::read_to_string("/proc/sys/crypto/fips_enabled") {
        Ok(content) => Ok(content.trim() == "1"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e).context(ReadFipsEnabledSnafu),
    }
}

/// Returns a `CryptoProvider` restricted to FIPS-approved algorithms.
///
/// Filters the default aws-lc-rs provider to only include cipher suites and
/// key exchange groups that report `.fips() == true`. This delegates the
/// FIPS classification to upstream rustls/aws-lc-rs rather than maintaining
/// a manual allowlist.
///
/// Each algorithm's `.fips()` returns true only when the underlying aws-lc-rs
/// library is in FIPS mode (`FIPS_mode() == 1`, compile-time constant) AND the
/// specific algorithm is FIPS-approved.
fn fips_provider() -> CryptoProvider {
    let base = aws_lc_rs::default_provider();
    CryptoProvider {
        cipher_suites: base
            .cipher_suites
            .into_iter()
            // Upstream rustls `.fips()` on cipher suites (rustls v/0.23.40):
            // https://github.com/rustls/rustls/blob/v/0.23.40/rustls/src/crypto/aws_lc_rs/tls13.rs#L116
            // https://github.com/rustls/rustls/blob/v/0.23.40/rustls/src/crypto/aws_lc_rs/tls12.rs#L190
            // After filtering, the following FIPS-approved suites remain:
            // - TLS13_AES_256_GCM_SHA384
            // - TLS13_AES_128_GCM_SHA256
            // - TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384
            // - TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256
            // - TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384
            // - TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
            .filter(|s| s.fips())
            .collect(),
        kx_groups: base
            .kx_groups
            .into_iter()
            // Upstream rustls `.fips()` on kx groups (rustls v/0.23.40):
            // https://github.com/rustls/rustls/blob/v/0.23.40/rustls/src/crypto/ring/kx.rs#L69
            // https://github.com/rustls/rustls/blob/v/0.23.40/rustls/src/crypto/aws_lc_rs/pq/hybrid.rs#L71
            // After filtering, the following FIPS-approved groups remain:
            // - secp256r1
            // - secp384r1
            // - X25519MLKEM768
            .filter(|g| g.fips())
            .collect(),
        ..base
    }
}

/// Returns the default `CryptoProvider` with all algorithms available.
fn default_provider() -> CryptoProvider {
    aws_lc_rs::default_provider()
}

/// Returns the appropriate `CryptoProvider` based on runtime FIPS detection.
pub fn provider() -> Result<CryptoProvider, Error> {
    let (crypto_mode, provider) = if fips_enabled()? {
        ("FIPS", fips_provider())
    } else {
        ("default", default_provider())
    };
    info!("Using {} CryptoProvider", crypto_mode);
    Ok(provider)
}

/// Detect FIPS mode and install the appropriate `CryptoProvider` as the global default.
///
/// This MUST be called once at the very start of `main()`, before any TLS
/// connections are made or any library has a chance to install its own provider.
/// The global provider uses `OnceLock` internally and cannot be overridden once set.
pub fn install_provider() -> Result<(), Error> {
    provider()?
        .install_default()
        .map_err(|_| Error::ProviderAlreadyInstalled)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustls::crypto::aws_lc_rs;
    use rustls::{CipherSuite, NamedGroup};
    use std::sync::Arc;

    #[test]
    fn fips_provider_excludes_chacha20() {
        // Given a FIPS-restricted provider
        let p = fips_provider();

        // Then no ChaCha20 cipher suites are present
        assert!(p.cipher_suites.iter().all(|s| {
            s.suite() != CipherSuite::TLS13_CHACHA20_POLY1305_SHA256
                && s.suite() != CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256
                && s.suite() != CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256
        }));
    }

    #[test]
    fn fips_provider_excludes_x25519() {
        // Given a FIPS-restricted provider
        let p = fips_provider();

        // Then X25519 is not in the kx groups
        assert!(p.kx_groups.iter().all(|g| g.name() != NamedGroup::X25519));
    }

    #[test]
    fn fips_provider_has_aes_gcm() {
        // Given a FIPS-restricted provider
        let p = fips_provider();

        // Then AES-GCM cipher suites are present
        assert!(!p.cipher_suites.is_empty());
        assert!(p
            .cipher_suites
            .iter()
            .any(|s| s.suite() == CipherSuite::TLS13_AES_256_GCM_SHA384));
    }

    #[test]
    fn fips_provider_has_p256_p384() {
        // Given a FIPS-restricted provider
        let p = fips_provider();

        // Then P-256 and P-384 kx groups are present
        assert!(!p.kx_groups.is_empty());
        assert!(p
            .kx_groups
            .iter()
            .any(|g| g.name() == NamedGroup::secp256r1));
        assert!(p
            .kx_groups
            .iter()
            .any(|g| g.name() == NamedGroup::secp384r1));
    }

    #[test]
    fn default_provider_includes_chacha20() {
        // Given the default (non-FIPS) provider
        let p = default_provider();

        // Then ChaCha20 cipher suites are available
        assert!(p
            .cipher_suites
            .iter()
            .any(|s| s.suite() == CipherSuite::TLS13_CHACHA20_POLY1305_SHA256));
    }

    #[test]
    fn default_provider_includes_x25519() {
        // Given the default (non-FIPS) provider
        let p = default_provider();

        // Then X25519 kx group is available
        assert!(p.kx_groups.iter().any(|g| g.name() == NamedGroup::X25519));
    }

    #[test]
    fn fips_enabled_reads_system_state() {
        // Given the system's FIPS state
        // When fips_enabled() is called
        match fips_enabled() {
            Ok(enabled) => {
                // Then the result matches the kernel flag
                let raw =
                    std::fs::read_to_string("/proc/sys/crypto/fips_enabled").unwrap_or_default();
                assert_eq!(enabled, raw.trim() == "1");
            }
            Err(_) => {
                // Then on systems without the file, an error is acceptable
            }
        }
    }

    /// Validates that a FIPS-restricted client cannot complete a TLS handshake
    /// with a server offering only non-FIPS cipher suites (ChaCha20-Poly1305).
    ///
    /// This test uses an in-memory byte buffer instead of real TCP sockets.
    /// TLS cipher negotiation is a protocol-level concern. It only depends on
    /// the ClientHello and ServerHello message contents, not on how bytes are
    /// transported. The rustls `ClientConnection` and `ServerConnection` state
    /// machines operate on raw bytes via `write_tls`/`read_tls`, so we can feed
    /// them through a `Vec<u8>` and get the exact same negotiation result as a
    /// real TCP connection, without needing threads, sockets, or network access.
    #[test]
    fn fips_provider_rejects_non_fips_server() {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
        let server_cert = rustls::pki_types::CertificateDer::from(cert.cert);
        let server_key =
            rustls::pki_types::PrivateKeyDer::try_from(cert.key_pair.serialize_der()).unwrap();

        // Given a server configured with only ChaCha20-Poly1305 (non-FIPS) ciphers
        let server_provider = {
            let base = aws_lc_rs::default_provider();
            CryptoProvider {
                cipher_suites: base
                    .cipher_suites
                    .into_iter()
                    .filter(|s| {
                        s.suite() == CipherSuite::TLS13_CHACHA20_POLY1305_SHA256
                            || s.suite() == CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256
                    })
                    .collect(),
                ..base
            }
        };
        let server_config = Arc::new(
            rustls::ServerConfig::builder_with_provider(Arc::new(server_provider))
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(vec![server_cert.clone()], server_key)
                .unwrap(),
        );

        // Given a client using the FIPS provider
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add(server_cert).unwrap();
        let client_config = Arc::new(
            rustls::ClientConfig::builder_with_provider(Arc::new(fips_provider()))
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_root_certificates(root_store)
                .with_no_client_auth(),
        );

        let server_name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
        let mut client = rustls::ClientConnection::new(client_config, server_name).unwrap();
        let mut server = rustls::ServerConnection::new(server_config).unwrap();

        // When the client initiates the TLS handshake and the server attempts
        // to negotiate a cipher suite. The handshake is performed entirely in
        // memory bytes flow through a Vec<u8> buffer rather than a TCP socket,
        // since cipher negotiation is a protocol-level concern independent of
        // the transport layer.
        let mut buf = Vec::new();
        // The client serializes its ClientHello message into the buffer. This
        // message contains the list of cipher suites the client supports (only
        // AES-GCM suites, since we use the FIPS provider).
        client.write_tls(&mut buf).unwrap();
        // The server reads the ClientHello from the buffer and attempts to find
        // a cipher suite it shares with the client. Since the server only has
        // ChaCha20 and the client only offers AES-GCM, no match is possible.
        server.read_tls(&mut &buf[..]).unwrap();
        let result = server.process_new_packets();

        // Then the handshake fails because there are no shared ciphers
        assert!(
            matches!(
                result,
                Err(rustls::Error::PeerIncompatible(
                    rustls::PeerIncompatible::NoCipherSuitesInCommon
                ))
            ),
            "Expected PeerIncompatible(NoCipherSuitesInCommon), got: {:?}",
            result
        );
    }
}
