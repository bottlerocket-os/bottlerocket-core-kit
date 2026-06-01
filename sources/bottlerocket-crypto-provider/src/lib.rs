//! Centralized CryptoProvider for Bottlerocket Rust binaries.
//!
//! Provides runtime FIPS detection and TLS algorithm selection.
//! When the kernel FIPS flag is enabled (`/proc/sys/crypto/fips_enabled` = 1),
//! the provider restricts TLS to FIPS-approved algorithms only.

use log::info;
use rustls::crypto::{aws_lc_rs, CryptoProvider};
use rustls::{CipherSuite, NamedGroup};

/// FIPS-approved TLS cipher suites.
const FIPS_CIPHER_SUITES: &[CipherSuite] = &[
    CipherSuite::TLS13_AES_256_GCM_SHA384,
    CipherSuite::TLS13_AES_128_GCM_SHA256,
    CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
    CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
    CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
    CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
];

/// FIPS-approved key exchange groups.
const FIPS_KX_GROUPS: &[NamedGroup] = &[
    NamedGroup::secp256r1,
    NamedGroup::secp384r1,
    NamedGroup::X25519MLKEM768,
];

/// Detect whether the system is running in FIPS mode by reading the kernel flag.
pub fn fips_enabled() -> bool {
    std::fs::read_to_string("/proc/sys/crypto/fips_enabled")
        .unwrap_or_default()
        .trim()
        == "1"
}

/// Returns a `CryptoProvider` restricted to FIPS-approved algorithms.
fn fips_provider() -> CryptoProvider {
    let base = aws_lc_rs::default_provider();
    CryptoProvider {
        cipher_suites: base
            .cipher_suites
            .into_iter()
            .filter(|s| FIPS_CIPHER_SUITES.contains(&s.suite()))
            .collect(),
        kx_groups: base
            .kx_groups
            .into_iter()
            .filter(|g| FIPS_KX_GROUPS.contains(&g.name()))
            .collect(),
        ..base
    }
}

/// Returns the default `CryptoProvider` with all algorithms available.
fn default_provider() -> CryptoProvider {
    aws_lc_rs::default_provider()
}

/// Returns the appropriate `CryptoProvider` based on runtime FIPS detection.
pub fn provider() -> CryptoProvider {
    if fips_enabled() {
        fips_provider()
    } else {
        default_provider()
    }
}

/// Detect FIPS mode and install the appropriate `CryptoProvider` as the global default.
///
/// This should be called once at the start of `main()` before any TLS connections are made.
/// If a provider is already installed (by another component), this is a no-op.
pub fn install_provider() {
    if CryptoProvider::get_default().is_some() {
        return;
    }
    let mode = if fips_enabled() { "FIPS" } else { "default" };
    info!("Installing {} CryptoProvider", mode);
    let _ = provider().install_default();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fips_provider_excludes_chacha20() {
        let p = fips_provider();
        assert!(p.cipher_suites.iter().all(|s| {
            s.suite() != CipherSuite::TLS13_CHACHA20_POLY1305_SHA256
                && s.suite() != CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256
                && s.suite() != CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256
        }));
    }

    #[test]
    fn fips_provider_excludes_x25519() {
        let p = fips_provider();
        assert!(p.kx_groups.iter().all(|g| g.name() != NamedGroup::X25519));
    }

    #[test]
    fn fips_provider_has_aes_gcm() {
        let p = fips_provider();
        assert!(!p.cipher_suites.is_empty());
        assert!(p
            .cipher_suites
            .iter()
            .any(|s| s.suite() == CipherSuite::TLS13_AES_256_GCM_SHA384));
    }

    #[test]
    fn fips_provider_has_p256_p384() {
        let p = fips_provider();
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
        let p = default_provider();
        assert!(p
            .cipher_suites
            .iter()
            .any(|s| s.suite() == CipherSuite::TLS13_CHACHA20_POLY1305_SHA256));
    }

    #[test]
    fn default_provider_includes_x25519() {
        let p = default_provider();
        assert!(p.kx_groups.iter().any(|g| g.name() == NamedGroup::X25519));
    }

    #[test]
    fn fips_enabled_defaults_to_false() {
        // On non-FIPS systems (or when /proc/sys/crypto/fips_enabled is missing/0),
        // fips_enabled() should return false
        let enabled = fips_enabled();
        assert!(
            !enabled,
            "Expected fips_enabled() to be false on non-FIPS system"
        );
    }
}
