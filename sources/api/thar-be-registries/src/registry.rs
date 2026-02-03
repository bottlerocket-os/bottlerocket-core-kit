//! Registry parsing and naming logic.

use url::Url;

use crate::error::{ParseRegistrySnafu, Result};

pub(crate) const DOCKER_HUB_HOST: &str = "docker.io";
pub(crate) const DOCKER_HUB_REGISTRY: &str = "registry-1.docker.io";

/// Parse registry string, extracting host:port and optional scheme.
/// Works with full URLs (https://docker.io) or bare hostnames (docker.io, registry:5000).
/// Defaults to https scheme when none is provided.
pub(crate) fn parse_registry(registry: &str) -> Result<(String, String)> {
    // Try parsing as-is first (handles URLs with scheme like https://docker.io)
    // Only accept if it has a host (registry:5000 parses as scheme with no host)
    if let Ok(url) = Url::parse(registry) {
        if let Some(host) = url.host_str() {
            return Ok((format_host_port(host, url.port()), url.scheme().to_string()));
        }
    }

    // Parse bare hostname with https:// prefix (handles docker.io or registry:5000)
    if let Ok(url) = Url::parse(&format!("https://{}", registry)) {
        if let Some(host) = url.host_str() {
            return Ok((format_host_port(host, url.port()), "https".to_string()));
        }
    }

    ParseRegistrySnafu { registry }.fail()
}

/// Encode registry name to directory name, replacing `:port` with `_port_`.
/// The trailing underscore matches containerd's [`hostDirectory()`] encoding,
/// which uses this format to avoid ambiguity with hostnames containing underscores.
///
/// [`hostDirectory()`]: https://github.com/containerd/containerd/blob/b668614b55183acee713ffabdbaa61843d631d0a/core/remotes/docker/config/hosts.go
pub(crate) fn encode_registry_name(name: &str) -> String {
    if name == "*" {
        return "_default".to_string();
    }
    if let Some(idx) = name.rfind(':') {
        format!("{}_{}_", &name[..idx], &name[idx + 1..])
    } else {
        name.to_string()
    }
}

/// Format host with optional port
pub(crate) fn format_host_port(host: &str, port: Option<u16>) -> String {
    match port {
        Some(p) => format!("{}:{}", host, p),
        None => host.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    // encode_registry_name tests
    #[test_case("*", "_default"; "wildcard maps to default")]
    #[test_case("docker.io", "docker.io"; "no port unchanged")]
    #[test_case("gcr.io", "gcr.io"; "gcr unchanged")]
    #[test_case("registry.example.com:5000", "registry.example.com_5000_"; "port encoded")]
    fn test_encode_registry_name(input: &str, expected: &str) {
        assert_eq!(encode_registry_name(input), expected);
    }

    // parse_registry tests
    #[test_case("*", "*", "https"; "wildcard")]
    #[test_case("docker.io", "docker.io", "https"; "bare hostname")]
    #[test_case("registry.example.com:5000", "registry.example.com:5000", "https"; "hostname with port")]
    #[test_case("https://docker.io", "docker.io", "https"; "https url")]
    #[test_case("http://registry.local:5000", "registry.local:5000", "http"; "http url with port")]
    fn test_parse_registry(input: &str, expected_host: &str, expected_scheme: &str) {
        let (host, scheme) = parse_registry(input).unwrap();
        assert_eq!(host, expected_host);
        assert_eq!(scheme, expected_scheme);
    }
}
