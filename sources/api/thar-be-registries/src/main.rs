/*!
# Background

thar-be-registries generates containerd registry configuration from Bottlerocket settings.

It reads `/etc/containerd/thar-be-registries.toml` and writes per-registry configuration files to
`/etc/containerd/certs.d/`.

For each configured registry, it creates:
* `hosts.toml` - mirror endpoints with pull/resolve capabilities
* `credentials.toml` - authentication credentials (mode 0600)

## Behavior

* Exits successfully (0) if the input file doesn't exist (graceful no-op)
* Uses atomic directory replacement to avoid race conditions with containerd
* containerd reads these files on-demand during image pulls

*/

use log::{error, info, warn};
use nix::fcntl::{renameat2, RenameFlags};
use simplelog::{Config as LogConfig, LevelFilter, SimpleLogger};
use snafu::ResultExt;
use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

mod creds;
mod error;
mod host_ns;
mod registry;
mod settings;

use creds::RegistryCredentials;
use error::{CreateDirSnafu, RenameDirSnafu, Result, SerializeTomlSnafu, WriteFileSnafu};
use host_ns::{Capability, Endpoint, HostConfig, HostNamespace};
use registry::{encode_registry_name, parse_registry, DOCKER_HUB_HOST, DOCKER_HUB_REGISTRY};
use settings::{read_settings, Credential, Mirror};

const CERTS_DIR: &str = "/etc/containerd/certs.d";
const INPUT_FILE: &str = "/etc/containerd/thar-be-registries.toml";

/// Entry point - generates containerd registry configuration files.
fn main() {
    if let Err(e) = run() {
        error!("{}", e);
        std::process::exit(1);
    }
}

/// Main execution logic
fn run() -> Result<()> {
    SimpleLogger::init(LevelFilter::Info, LogConfig::default()).unwrap_or(());
    info!("Reading registry config from {}", INPUT_FILE);

    if !Path::new(INPUT_FILE).exists() {
        warn!(
            "No registry settings file found at '{}', skipping",
            INPUT_FILE
        );
        return Ok(());
    }

    let settings = read_settings(INPUT_FILE)?;

    let temp_dir = tempfile::tempdir_in("/etc/containerd").context(CreateDirSnafu {
        path: "/etc/containerd",
    })?;

    let mirrors = settings.mirrors.unwrap_or_default();
    let credentials = settings.credentials.unwrap_or_default();
    let total_count = mirrors.len() + credentials.len();
    let mut failure_count = 0;

    for mirror in &mirrors {
        if let Err(e) = write_hosts_toml(temp_dir.path(), mirror) {
            error!(
                "Failed to write hosts.toml for '{}': {}",
                mirror.registry, e
            );
            failure_count += 1;
        }
    }

    for cred in &credentials {
        if let Err(e) = write_credentials_toml(temp_dir.path(), cred) {
            error!(
                "Failed to write credentials.toml for '{}': {}",
                cred.registry, e
            );
            failure_count += 1;
        }
    }

    if failure_count > 0 {
        return Err(error::Error::WriteRegistries {
            failure_count,
            total_count,
        });
    }

    // Ensure target exists for atomic swap.
    fs::create_dir_all(CERTS_DIR).context(CreateDirSnafu { path: CERTS_DIR })?;
    // After the exchange, the old content is removed when tmp_dir is dropped.
    rename_exchange_dir(temp_dir.path(), Path::new(CERTS_DIR))?;
    info!("Successfully wrote registry configs to {}", CERTS_DIR);

    Ok(())
}

/// Atomically exchange two directories using Linux renameat2.
/// Both paths must exist. After the call, each path points to what the other contained.
fn rename_exchange_dir(a: &Path, b: &Path) -> Result<()> {
    renameat2(None, a, None, b, RenameFlags::RENAME_EXCHANGE)
        .context(RenameDirSnafu { from: a, to: b })
}

/// Write hosts.toml for a registry mirror.
fn write_hosts_toml(base_dir: &Path, mirror: &Mirror) -> Result<()> {
    let (host, scheme) = parse_registry(&mirror.registry)?;
    let encoded = encode_registry_name(&host);
    let dir = base_dir.join(&encoded);
    fs::create_dir_all(&dir).context(CreateDirSnafu {
        path: dir.display().to_string(),
    })?;

    // For the default mirror, indicated by '*' as the registry, there are two options. If only the
    // mirror should be used, and upstream should never be contacted, then the mirror can be set in
    // the "server" field. Otherwise, the "server" field should be left unset, so it resolves to
    // the upstream registry, and the mirror should be listed as a "hosts" entry.
    //
    // For consistency with hosts.toml for non-default mirrors, we go with the second option, which
    // means that there's always a potential fallback to upstream.
    let server = match host.as_str() {
        "*" => None,
        DOCKER_HUB_HOST => Some(format!("https://{}", DOCKER_HUB_REGISTRY)),
        _ => Some(format!("{}://{}", scheme, host)),
    };

    let mut ns = HostNamespace {
        server,
        ..Default::default()
    };

    let caps = resolve_capabilities(mirror.capabilities.as_deref());
    for endpoint in &mirror.endpoint {
        let ep = Endpoint::new(endpoint);
        let mut cfg = HostConfig::new(caps.iter().copied());
        if ep.has_path_component() {
            cfg = cfg.with_override_path(true);
        }
        ns.host.insert(ep, cfg);
    }

    let content = toml::to_string(&ns).context(SerializeTomlSnafu)?;
    let path = dir.join("hosts.toml");
    fs::write(&path, content).context(WriteFileSnafu {
        path: path.display().to_string(),
    })?;

    info!("Wrote hosts.toml for '{}'", mirror.registry);
    Ok(())
}

/// Map an optional list of capability strings to Capability values.
/// Returns the default ["pull", "resolve"] when capabilities is None.
/// Unknown strings are skipped with a warning.
fn resolve_capabilities(caps: Option<&[String]>) -> Vec<Capability> {
    match caps {
        None => vec![Capability::Pull, Capability::Resolve],
        Some(strings) => strings
            .iter()
            .filter_map(|s| match s.as_str() {
                "pull" => Some(Capability::Pull),
                "resolve" => Some(Capability::Resolve),
                "push" => Some(Capability::Push),
                other => {
                    warn!("Unknown capability '{}', skipping", other);
                    None
                }
            })
            .collect(),
    }
}

/// Write credentials.toml for a registry.
/// For docker.io, writes to registry-1.docker.io since containerd's credential
/// callback receives the actual registry host, not the image reference host.
fn write_credentials_toml(base_dir: &Path, cred: &Credential) -> Result<()> {
    let (host, _) = parse_registry(&cred.registry)?;
    let cred_host = if host == DOCKER_HUB_HOST {
        DOCKER_HUB_REGISTRY
    } else {
        &host
    };
    let encoded = encode_registry_name(cred_host);
    let dir = base_dir.join(&encoded);
    fs::create_dir_all(&dir).context(CreateDirSnafu {
        path: dir.display().to_string(),
    })?;

    let rc = RegistryCredentials {
        username: cred.username.clone(),
        password: cred.password.clone(),
        auth: cred.auth.clone(),
        identitytoken: cred.identitytoken.clone(),
    };

    let content = toml::to_string(&rc).context(SerializeTomlSnafu)?;
    let path = dir.join("credentials.toml");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)
        .context(WriteFileSnafu {
            path: path.display().to_string(),
        })?;
    file.write_all(content.as_bytes()).context(WriteFileSnafu {
        path: path.display().to_string(),
    })?;

    info!("Wrote credentials.toml for '{}'", cred.registry);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use settings::Mirror;
    use test_case::test_case;

    // Workflow tests - helper to create temp dir and verify file contents
    fn verify_file(dir: &std::path::Path, rel_path: &str, expected_contents: &[&str]) {
        let content = fs::read_to_string(dir.join(rel_path)).unwrap();
        for expected in expected_contents {
            assert!(
                content.contains(expected),
                "Missing '{}' in:\n{}",
                expected,
                content
            );
        }
    }

    #[test_case(
    "docker.io",
    &["https://mirror.example.com"],
    None,
    "docker.io/hosts.toml",
    &[r#"server = "https://registry-1.docker.io""#, r#"[host."https://mirror.example.com"]"#]
    ; "docker.io mirror"
  )]
    #[test_case(
    "registry.example.com:5000",
    &["https://mirror.local"],
    None,
    "registry.example.com_5000_/hosts.toml",
    &[r#"server = "https://registry.example.com:5000""#]
    ; "registry with port"
  )]
    #[test_case(
    "docker.io",
    &["https://ecr-cache.example.com/v2/docker-hub"],
    None,
    "docker.io/hosts.toml",
    &["override_path = true"]
    ; "endpoint with path sets override_path"
  )]
    #[test_case(
    "*",
    &["https://mirror.global"],
    None,
    "_default/hosts.toml",
    &[r#"[host."https://mirror.global"]"#]
    ; "global mirror"
  )]
    #[test_case(
    "registry.example.com:443",
    &["https://mirror.local"],
    None,
    "registry.example.com_443_/hosts.toml",
    &[r#"server = "https://registry.example.com:443""#]
    ; "port 443 preserves port in directory and server"
  )]
    #[test_case(
    "registry.example.com:80",
    &["http://mirror.local"],
    None,
    "registry.example.com_80_/hosts.toml",
    &[r#"server = "http://registry.example.com:80""#]
    ; "port 80 infers http scheme"
  )]
    #[test_case(
    "http://registry.local:5000",
    &["http://mirror.local:5000"],
    None,
    "registry.local_5000_/hosts.toml",
    &[r#"server = "http://registry.local:5000""#]
    ; "explicit http scheme preserved"
  )]
    #[test_case(
    "docker.io",
    &["https://10.0.0.1:443/path/to/resource"],
    None,
    "docker.io/hosts.toml",
    &[r#"[host."https://10.0.0.1:443/path/to/resource"]"#, "override_path = true"]
    ; "https ip port 443 with path sets override_path"
  )]
    #[test_case(
    "registry.example.com",
    &["https://172.16.0.5:8443/path/to/resource"],
    None,
    "registry.example.com/hosts.toml",
    &[r#"[host."https://172.16.0.5:8443/path/to/resource"]"#, "override_path = true"]
    ; "https ip custom port with path sets override_path"
  )]
    // Schemeless endpoints with path — override_path must be set
    #[test_case(
    "public.ecr.aws",
    &["196.18.8.18:443/v2/eks-a-test"],
    None,
    "public.ecr.aws/hosts.toml",
    &[r#"[host."196.18.8.18:443/v2/eks-a-test"]"#, "override_path = true"]
    ; "schemeless ip port 443 with path against public.ecr.aws sets override_path"
  )]
    #[test_case(
    "registry.example.com",
    &["192.168.1.1:5000/path/to/resource"],
    None,
    "registry.example.com/hosts.toml",
    &[r#"[host."192.168.1.1:5000/path/to/resource"]"#, "override_path = true"]
    ; "schemeless ip custom port with path sets override_path"
  )]
    #[test_case(
    "registry.example.com",
    &["mirror.local:5000/path/to/resource"],
    None,
    "registry.example.com/hosts.toml",
    &[r#"[host."mirror.local:5000/path/to/resource"]"#, "override_path = true"]
    ; "schemeless hostname custom port with path sets override_path"
  )]
    // http scheme with path — override_path
    #[test_case(
    "registry.example.com",
    &["http://mirror.local:5000/path/to/resource"],
    None,
    "registry.example.com/hosts.toml",
    &[r#"[host."http://mirror.local:5000/path/to/resource"]"#, "override_path = true"]
    ; "http hostname custom port with path sets override_path"
  )]
    // Hostname no port with path
    #[test_case(
    "docker.io",
    &["https://mirror.example.com/path/to/resource"],
    None,
    "docker.io/hosts.toml",
    &[r#"[host."https://mirror.example.com/path/to/resource"]"#, "override_path = true"]
    ; "https hostname no port with path sets override_path"
  )]
    #[test_case(
    "docker.io",
    &["https://mirror.example.com"],
    Some(&["pull"]),
    "docker.io/hosts.toml",
    &[r#"capabilities = ["pull"]"#]
    ; "explicit pull-only capability"
  )]
    #[test_case(
    "docker.io",
    &["https://mirror.example.com"],
    None,
    "docker.io/hosts.toml",
    &["pull", "resolve"]
    ; "default capabilities when none specified"
  )]
    #[test_case(
    "docker.io",
    &["https://mirror.example.com"],
    Some(&["pull", "resolve", "push"]),
    "docker.io/hosts.toml",
    &["pull", "resolve", "push"]
    ; "all three capabilities"
  )]
    fn test_write_hosts_toml(
        registry: &str,
        endpoints: &[&str],
        capabilities: Option<&[&str]>,
        expected_path: &str,
        expected_contents: &[&str],
    ) {
        let dir = tempfile::tempdir().unwrap();
        let mirror = Mirror {
            registry: registry.to_string(),
            endpoint: endpoints.iter().map(|s| s.to_string()).collect(),
            capabilities: capabilities.map(|cs| cs.iter().map(|s| s.to_string()).collect()),
        };
        write_hosts_toml(dir.path(), &mirror).unwrap();
        verify_file(dir.path(), expected_path, expected_contents);
    }

    #[test_case(
    "registry.example.com",
    Some("user"), Some("pass"), None, None,
    "registry.example.com/credentials.toml",
    &[r#"username = "user""#, r#"password = "pass""#]
    ; "username and password"
  )]
    #[test_case(
    "docker.io",
    Some("user"), Some("pass"), None, None,
    "registry-1.docker.io/credentials.toml",
    &[r#"username = "user""#, r#"password = "pass""#]
    ; "docker.io writes to registry-1.docker.io"
  )]
    #[test_case(
    "registry.example.com",
    None, None, None, Some("token123"),
    "registry.example.com/credentials.toml",
    &[r#"identitytoken = "token123""#]
    ; "identitytoken only"
  )]
    #[test_case(
    "registry.example.com",
    None, None, Some("dXNlcjpwYXNz"), None,
    "registry.example.com/credentials.toml",
    &[r#"auth = "dXNlcjpwYXNz""#]
    ; "auth only"
  )]
    #[test_case(
    "registry.example.com:443",
    Some("user"), Some("pass"), None, None,
    "registry.example.com_443_/credentials.toml",
    &[r#"username = "user""#, r#"password = "pass""#]
    ; "port 443 encodes directory name"
  )]
    #[test_case(
    "registry.example.com:80",
    Some("user"), Some("pass"), None, None,
    "registry.example.com_80_/credentials.toml",
    &[r#"username = "user""#, r#"password = "pass""#]
    ; "port 80 encodes directory name"
  )]
    fn test_write_credentials_toml(
        registry: &str,
        username: Option<&str>,
        password: Option<&str>,
        auth: Option<&str>,
        identitytoken: Option<&str>,
        expected_path: &str,
        expected_contents: &[&str],
    ) {
        let dir = tempfile::tempdir().unwrap();
        let cred = Credential {
            registry: registry.to_string(),
            username: username.map(String::from),
            password: password.map(String::from),
            auth: auth.map(String::from),
            identitytoken: identitytoken.map(String::from),
        };
        write_credentials_toml(dir.path(), &cred).unwrap();
        verify_file(dir.path(), expected_path, expected_contents);
    }

    #[test]
    fn test_workflow_mirror_and_credential_same_registry() {
        let dir = tempfile::tempdir().unwrap();
        let mirror = Mirror {
            registry: "registry.example.com".to_string(),
            endpoint: vec!["https://mirror.example.com".to_string()],
            capabilities: None,
        };
        let cred = Credential {
            registry: "registry.example.com".to_string(),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            auth: None,
            identitytoken: None,
        };
        write_hosts_toml(dir.path(), &mirror).unwrap();
        write_credentials_toml(dir.path(), &cred).unwrap();
        assert!(dir.path().join("registry.example.com/hosts.toml").exists());
        assert!(dir
            .path()
            .join("registry.example.com/credentials.toml")
            .exists());
    }

    #[test]
    fn test_credentials_file_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let cred = Credential {
            registry: "test.io".to_string(),
            username: Some("u".to_string()),
            password: Some("p".to_string()),
            auth: None,
            identitytoken: None,
        };
        write_credentials_toml(dir.path(), &cred).unwrap();
        let path = dir.path().join("test.io/credentials.toml");
        let perms = fs::metadata(&path).unwrap().permissions();
        assert_eq!(
            perms.mode() & 0o777,
            0o600,
            "credentials.toml should be mode 0600"
        );
    }

    #[test]
    fn test_wildcard_no_server() {
        let dir = tempfile::tempdir().unwrap();
        let mirror = Mirror {
            registry: "*".to_string(),
            endpoint: vec!["https://mirror.example.com".to_string()],
            capabilities: None,
        };
        write_hosts_toml(dir.path(), &mirror).unwrap();
        let content = fs::read_to_string(dir.path().join("_default/hosts.toml")).unwrap();
        assert!(!content.contains(r#"server = "https://*""#));
    }

    #[test]
    fn test_docker_io_special_server() {
        let dir = tempfile::tempdir().unwrap();
        let mirror = Mirror {
            registry: "docker.io".to_string(),
            endpoint: vec!["https://mirror.example.com".to_string()],
            capabilities: None,
        };
        write_hosts_toml(dir.path(), &mirror).unwrap();
        let content = fs::read_to_string(dir.path().join("docker.io/hosts.toml")).unwrap();
        assert!(content.contains(r#"server = "https://registry-1.docker.io""#));
    }

    #[test]
    fn test_multiple_endpoints() {
        let dir = tempfile::tempdir().unwrap();
        let mirror = Mirror {
            registry: "test.io".to_string(),
            endpoint: vec![
                "https://mirror1.example.com".to_string(),
                "https://mirror2.example.com".to_string(),
                "https://mirror3.example.com".to_string(),
            ],
            capabilities: None,
        };
        write_hosts_toml(dir.path(), &mirror).unwrap();
        let content = fs::read_to_string(dir.path().join("test.io/hosts.toml")).unwrap();
        assert!(content.contains(r#"[host."https://mirror1.example.com"]"#));
        assert!(content.contains(r#"[host."https://mirror2.example.com"]"#));
        assert!(content.contains(r#"[host."https://mirror3.example.com"]"#));
        assert_eq!(content.matches("capabilities").count(), 3);
    }

    #[test]
    fn test_empty_endpoints() {
        let dir = tempfile::tempdir().unwrap();
        let mirror = Mirror {
            registry: "test.io".to_string(),
            endpoint: vec![],
            capabilities: None,
        };
        write_hosts_toml(dir.path(), &mirror).unwrap();
        let content = fs::read_to_string(dir.path().join("test.io/hosts.toml")).unwrap();
        assert!(content.contains(r#"server = "https://test.io""#));
        assert!(!content.contains("[host."));
    }

    #[test]
    fn test_resolve_capabilities_none_returns_default() {
        let caps = resolve_capabilities(None);
        assert_eq!(caps, vec![Capability::Pull, Capability::Resolve]);
    }

    #[test]
    fn test_resolve_capabilities_pull_only() {
        let caps = resolve_capabilities(Some(&["pull".to_string()]));
        assert_eq!(caps, vec![Capability::Pull]);
    }

    #[test]
    fn test_resolve_capabilities_all_three() {
        let caps = resolve_capabilities(Some(&[
            "pull".to_string(),
            "resolve".to_string(),
            "push".to_string(),
        ]));
        assert_eq!(
            caps,
            vec![Capability::Pull, Capability::Resolve, Capability::Push]
        );
    }

    #[test]
    fn test_resolve_capabilities_unknown_skipped() {
        let caps = resolve_capabilities(Some(&["pull".to_string(), "fly".to_string()]));
        assert_eq!(caps, vec![Capability::Pull]);
    }

    #[test]
    fn test_resolve_capabilities_empty_returns_empty() {
        let caps = resolve_capabilities(Some(&[]));
        assert!(caps.is_empty());
    }

    #[test]
    fn test_rename_exchange_dir() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::create_dir(&a).unwrap();
        fs::create_dir(&b).unwrap();
        fs::write(a.join("file"), "from_a").unwrap();
        fs::write(b.join("file"), "from_b").unwrap();

        rename_exchange_dir(&a, &b).unwrap();

        assert_eq!(fs::read_to_string(a.join("file")).unwrap(), "from_b");
        assert_eq!(fs::read_to_string(b.join("file")).unwrap(), "from_a");
    }
}
