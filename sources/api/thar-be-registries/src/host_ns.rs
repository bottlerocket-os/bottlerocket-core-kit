//! Containerd registry host namespace configuration.
//!
//! This module provides types for generating `hosts.toml` files that configure
//! containerd's registry mirrors. See the [containerd documentation] for details.
//!
//! [containerd documentation]: https://github.com/containerd/containerd/blob/c4982bffc6dd887a58a189f8a6be99b1b1542953/docs/hosts.md

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use url::Url;

/// A registry mirror endpoint URL.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Endpoint(String);

impl Endpoint {
    /// Creates a new endpoint from a URL string.
    pub fn new(url: impl Into<String>) -> Self {
        Self(url.into())
    }

    /// Checks if this endpoint has a path component beyond "/".
    pub fn has_path_component(&self) -> bool {
        // Try parsing as-is first (handles URLs with scheme like https://host/path)
        // Only accept if it has a host (registry:5000 parses as scheme with no host)
        if let Ok(url) = Url::parse(&self.0) {
            if let Some(_host) = url.host_str() {
                return !url.path().is_empty() && url.path() != "/";
            }
        }

        // Parse bare hostname with https:// prefix (handles registry:5000/v2/path)
        if let Ok(url) = Url::parse(&format!("https://{}", &self.0)) {
            if let Some(_host) = url.host_str() {
                return !url.path().is_empty() && url.path() != "/";
            }
        }

        false
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Operations a registry host can perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Capability {
    Pull,
    Resolve,
    Push,
}

/// Registry host namespace configuration (hosts.toml).
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct HostNamespace {
    /// Default server URL for this registry namespace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,

    /// Mirror host configurations, keyed by endpoint URL.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub host: BTreeMap<Endpoint, HostConfig>,
}

/// Configuration for a single host/mirror endpoint.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct HostConfig {
    /// Operations this host can perform.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<BTreeSet<Capability>>,

    /// When true, use the URL path as the API root instead of appending /v2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_path: Option<bool>,
}

impl HostConfig {
    /// Creates a new host configuration with the specified capabilities.
    pub fn new(capabilities: impl IntoIterator<Item = Capability>) -> Self {
        Self {
            capabilities: Some(capabilities.into_iter().collect()),
            override_path: None,
        }
    }

    /// Sets the override_path flag and returns self for method chaining.
    pub fn with_override_path(mut self, override_path: bool) -> Self {
        self.override_path = Some(override_path);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_server_only() {
        let ns = HostNamespace {
            server: Some("https://registry-1.docker.io".to_string()),
            host: BTreeMap::new(),
        };
        let s = toml::to_string(&ns).unwrap();
        assert!(s.contains(r#"server = "https://registry-1.docker.io""#));
        assert!(!s.contains("[host"));
    }

    #[test]
    fn test_serialize_server_and_host() {
        let mut ns = HostNamespace {
            server: Some("https://registry-1.docker.io".to_string()),
            host: BTreeMap::new(),
        };
        ns.host.insert(
            Endpoint::new("https://mirror.example.com"),
            HostConfig::new([Capability::Pull, Capability::Resolve]),
        );
        let s = toml::to_string(&ns).unwrap();
        assert!(s.contains(r#"server = "https://registry-1.docker.io""#));
        assert!(s.contains(r#"[host."https://mirror.example.com"]"#));
        assert!(s.contains("pull"));
    }

    #[test]
    fn test_serialize_override_path() {
        let mut ns = HostNamespace::default();
        ns.host.insert(
            Endpoint::new("https://ecr.example.com/v2/docker"),
            HostConfig::new([Capability::Pull]).with_override_path(true),
        );
        let s = toml::to_string(&ns).unwrap();
        assert!(s.contains("override_path = true"));
    }

    #[test]
    fn test_roundtrip() {
        let mut ns = HostNamespace {
            server: Some("https://test.io".to_string()),
            host: BTreeMap::new(),
        };
        ns.host.insert(
            Endpoint::new("https://mirror.io"),
            HostConfig::new([Capability::Pull]),
        );
        let s = toml::to_string(&ns).unwrap();
        let parsed: HostNamespace = toml::from_str(&s).unwrap();
        assert_eq!(ns, parsed);
    }

    #[test]
    fn test_empty_serializes_minimal() {
        let ns = HostNamespace::default();
        let s = toml::to_string(&ns).unwrap();
        assert!(s.is_empty() || s.trim().is_empty());
    }

    #[test]
    fn test_endpoint_has_path_component() {
        // === With scheme, no path ===
        assert!(!Endpoint::new("https://mirror.example.com").has_path_component());
        assert!(!Endpoint::new("https://mirror.example.com/").has_path_component());
        assert!(!Endpoint::new("http://mirror.example.com").has_path_component());
        assert!(!Endpoint::new("https://192.168.1.1").has_path_component());
        assert!(!Endpoint::new("https://192.168.1.1:443").has_path_component());
        assert!(!Endpoint::new("http://192.168.1.1:80").has_path_component());
        assert!(!Endpoint::new("https://192.168.1.1:5000").has_path_component());

        // === With scheme, with path ===
        assert!(Endpoint::new("https://mirror.example.com/v2/docker-hub").has_path_component());
        assert!(Endpoint::new("http://mirror.example.com/v2/docker-hub").has_path_component());
        assert!(Endpoint::new("https://192.168.1.1:443/v2/myrepo").has_path_component());
        assert!(Endpoint::new("http://192.168.1.1:80/v2/myrepo").has_path_component());
        assert!(Endpoint::new("https://192.168.1.1:5000/path/to/resource").has_path_component());
        assert!(Endpoint::new("https://registry.example.com/v2/mirror").has_path_component());

        // === Schemeless, no path ===
        assert!(!Endpoint::new("196.18.8.18:443").has_path_component());
        assert!(!Endpoint::new("192.168.1.1:5000").has_path_component());
        assert!(!Endpoint::new("registry.example.com:5000").has_path_component());
        assert!(!Endpoint::new("mirror.example.com").has_path_component());

        // === Schemeless, with path ===
        assert!(Endpoint::new("196.18.8.18:443/v2/eks-a-test").has_path_component());
        assert!(Endpoint::new("192.168.1.1:5000/path/to/resource").has_path_component());
        assert!(Endpoint::new("192.168.1.1:80/v2/myrepo").has_path_component());
        assert!(Endpoint::new("registry.example.com:5000/v2/mirror").has_path_component());
        assert!(Endpoint::new("registry.example.com/v2/mirror").has_path_component());
    }
}
