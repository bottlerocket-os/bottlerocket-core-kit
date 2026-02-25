//! Containerd registry credentials configuration.
//!
//! This module provides types for generating `credentials.toml` files that configure
//! authentication for containerd registry access.

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Registry credentials (credentials.toml).
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Zeroize, ZeroizeOnDrop)]
pub struct RegistryCredentials {
    /// Username for basic authentication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Password for basic authentication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    /// Base64-encoded "username:password" for basic authentication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<String>,

    /// Identity token for token-based authentication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identitytoken: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_username_password() {
        let creds = RegistryCredentials {
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            auth: None,
            identitytoken: None,
        };
        let s = toml::to_string(&creds).unwrap();
        assert!(s.contains(r#"username = "user""#));
        assert!(s.contains(r#"password = "pass""#));
    }

    #[test]
    fn test_serialize_auth_only() {
        let creds = RegistryCredentials {
            username: None,
            password: None,
            auth: Some("dXNlcjpwYXNz".to_string()),
            identitytoken: None,
        };
        let s = toml::to_string(&creds).unwrap();
        assert!(s.contains(r#"auth = "dXNlcjpwYXNz""#));
        assert!(!s.contains("username"));
    }

    #[test]
    fn test_serialize_identitytoken_only() {
        let creds = RegistryCredentials {
            username: None,
            password: None,
            auth: None,
            identitytoken: Some("token123".to_string()),
        };
        let s = toml::to_string(&creds).unwrap();
        assert!(s.contains(r#"identitytoken = "token123""#));
    }

    #[test]
    fn test_empty_serializes_empty() {
        let creds = RegistryCredentials::default();
        let s = toml::to_string(&creds).unwrap();
        assert!(s.is_empty() || s.trim().is_empty());
    }

    #[test]
    fn test_roundtrip() {
        let creds = RegistryCredentials {
            username: Some("u".to_string()),
            password: Some("p".to_string()),
            auth: None,
            identitytoken: None,
        };
        let s = toml::to_string(&creds).unwrap();
        let parsed: RegistryCredentials = toml::from_str(&s).unwrap();
        assert_eq!(creds, parsed);
    }
}
