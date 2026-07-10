//! Registry parsing and naming logic.

use url::Url;

use crate::error::{ParseRegistrySnafu, Result};

pub(crate) const DOCKER_HUB_HOST: &str = "docker.io";
pub(crate) const DOCKER_HUB_REGISTRY: &str = "registry-1.docker.io";

/// Parse a registry string into its `(host, scheme)` components.
///
/// Returns:
/// - `host`: The registry host and optional port (e.g. `docker.io`, `registry.example.com:5000`).
///   Used to construct the directory name under `certs.d/` and the `server` URL in `hosts.toml`.
/// - `scheme`: Either `http` or `https`. Written into the `server` field of `hosts.toml`
///   (e.g. `server = "https://registry.example.com:5000"`).
///
/// Scheme is determined as follows:
/// - Explicit scheme in input (`http://` or `https://`) → preserved as-is.
/// - No scheme, port 80 → `http`.
/// - No scheme, any other port or no port → `https`.
pub(crate) fn parse_registry(registry: &str) -> Result<(String, String)> {
    // Try parsing as-is first (handles URLs with scheme like https://docker.io)
    // Only accept if it has a host (registry:5000 parses as scheme with no host)
    if let Ok(url) = Url::parse(registry) {
        if let Some(host) = url.host_str() {
            let port = parsed_port(&url, registry);
            return Ok((format_host_port(host, port), url.scheme().to_string()));
        }
    }

    // Parse bare hostname with https:// prefix (handles docker.io or registry:5000)
    if let Ok(url) = Url::parse(&format!("https://{}", registry)) {
        if let Some(host) = url.host_str() {
            let port = parsed_port(&url, registry);
            let scheme = if port == Some(80) { "http" } else { "https" };
            return Ok((format_host_port(host, port), scheme.to_string()));
        }
    }

    ParseRegistrySnafu { registry }.fail()
}

/// Returns the port from a parsed URL, preserving explicit default ports (e.g. :443, :80)
/// that `Url::port()` would strip.
fn parsed_port(url: &Url, original_url: &str) -> Option<u16> {
    if let Some(port) = url.port() {
        return Some(port);
    }
    if let Some(default_port) = url.port_or_known_default() {
        if original_url.contains(&format!(
            "{}:{}",
            url.host_str().unwrap_or_default(),
            default_port
        )) {
            return Some(default_port);
        }
    }
    None
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
    // Default port preservation: explicit :443 and :80 must be retained
    #[test_case("192.168.1.1:443", "192.168.1.1_443_"; "bare host with explicit 443")]
    #[test_case("192.168.1.1:80", "192.168.1.1_80_"; "bare host with explicit 80")]
    #[test_case("registry.example.com:443", "registry.example.com_443_"; "bare hostname with explicit 443")]
    #[test_case("registry.example.com:80", "registry.example.com_80_"; "bare hostname with explicit 80")]
    // IPv6 with port
    #[test_case("[::1]:5000", "[::1]_5000_"; "ipv6 loopback with port encoded")]
    #[test_case("[2001:db8::1]:443", "[2001:db8::1]_443_"; "ipv6 with 443 encoded")]
    fn test_encode_registry_name(input: &str, expected: &str) {
        assert_eq!(encode_registry_name(input), expected);
    }

    // parse_registry tests
    #[test_case("*", "*", "https"; "wildcard")]
    #[test_case("docker.io", "docker.io", "https"; "bare hostname")]
    #[test_case("registry.example.com:5000", "registry.example.com:5000", "https"; "hostname with port")]
    #[test_case("https://docker.io", "docker.io", "https"; "https url")]
    #[test_case("http://registry.local:5000", "registry.local:5000", "http"; "http url with port")]
    // Default port preservation: explicit :443 and :80 must be retained
    #[test_case("192.168.1.1:443", "192.168.1.1:443", "https"; "bare host with explicit 443")]
    #[test_case("192.168.1.1:80", "192.168.1.1:80", "http"; "bare host with explicit 80")]
    #[test_case("registry.example.com:443", "registry.example.com:443", "https"; "bare hostname with explicit 443")]
    #[test_case("registry.example.com:80", "registry.example.com:80", "http"; "bare hostname with explicit 80")]
    #[test_case("https://192.168.1.1:443", "192.168.1.1:443", "https"; "https url with explicit 443")]
    #[test_case("http://192.168.1.1:80", "192.168.1.1:80", "http"; "http url with explicit 80")]
    #[test_case("https://registry.example.com:443", "registry.example.com:443", "https"; "https hostname with explicit 443")]
    #[test_case("http://registry.example.com:80", "registry.example.com:80", "http"; "http hostname with explicit 80")]
    // Explicit scheme always wins over port-based inference.
    #[test_case("https://registry.example.com:80", "registry.example.com:80", "https"; "explicit https on port 80")]
    #[test_case("http://registry.example.com:443", "registry.example.com:443", "http"; "explicit http on port 443")]
    // Bare IP with no port defaults to https.
    #[test_case("192.168.1.1", "192.168.1.1", "https"; "bare ip no port")]
    // IPv6 addresses
    #[test_case("[::1]:5000", "[::1]:5000", "https"; "ipv6 loopback with port")]
    #[test_case("[2001:db8::1]:443", "[2001:db8::1]:443", "https"; "ipv6 with explicit 443")]
    #[test_case("[2001:db8::1]:80", "[2001:db8::1]:80", "http"; "ipv6 with explicit 80")]
    #[test_case("https://[::1]:443", "[::1]:443", "https"; "ipv6 https with explicit 443")]
    #[test_case("http://[::1]:80", "[::1]:80", "http"; "ipv6 http with explicit 80")]
    // Ensure port-like patterns in IPv6 do not cause false positives.
    // [fe80::443] has no port — the "443" is part of the address, not a port.
    #[test_case("[fe80::443]", "[fe80::443]", "https"; "ipv6 addr containing 443 no port")]
    #[test_case("https://[fe80::443]", "[fe80::443]", "https"; "ipv6 addr containing 443 with scheme no port")]
    fn test_parse_registry(input: &str, expected_host: &str, expected_scheme: &str) {
        let (host, scheme) = parse_registry(input).unwrap();
        assert_eq!(host, expected_host);
        assert_eq!(scheme, expected_scheme);
    }
}
