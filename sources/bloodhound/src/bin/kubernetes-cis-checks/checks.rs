use std::collections::HashSet;

use bloodhound::system_access::SystemAccess;
use bloodhound::{
    check_file_not_mode, ensure_file_owner_and_group_root,
    results::{CheckStatus, Checker, CheckerMetadata, CheckerResult, Mode},
};
use libc::{S_IRWXG, S_IRWXO, S_IWGRP, S_IWOTH, S_IXGRP, S_IXOTH, S_IXUSR};
use serde::Deserialize;

// Bottlerocket doesn't use the standard path for most of these files ¯\_(ツ)_/¯
const KUBELET_SERVICE_FILE: &str = "/etc/systemd/system/kubelet.service.d/exec-start.conf";
const KUBELET_KUBECONFIG_FILE: &str = "/etc/kubernetes/kubelet/kubeconfig";
const KUBELET_CLIENT_CA_FILE: &str = "/etc/kubernetes/pki/ca.crt";
const KUBELET_CONF_FILE: &str = "/etc/kubernetes/kubelet/config";
pub const KUBEPROXY_CONF_FILE: &str = "/etc/kubernetes/kube-proxy/kube-proxy.conf";

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04010100Checker {}

impl Checker for K8S04010100Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        let no_x_xwr_xwr = S_IXUSR | S_IRWXG | S_IRWXO;
        check_file_not_mode(sac, KUBELET_SERVICE_FILE, no_x_xwr_xwr)
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Ensure that the kubelet service file permissions are set to 600 or more restrictive".to_string(),
            id: "4.1.1".to_string(),
            level: 1,
            name: "k8s04010100".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04010200Checker {}

impl Checker for K8S04010200Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        ensure_file_owner_and_group_root(sac, KUBELET_SERVICE_FILE)
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Ensure that the kubelet service file ownership is set to root:root".to_string(),
            id: "4.1.2".to_string(),
            level: 1,
            name: "k8s04010200".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04010500Checker {}

impl Checker for K8S04010500Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        let no_x_xwr_xwr = S_IXUSR | S_IRWXG | S_IRWXO;
        check_file_not_mode(sac, KUBELET_KUBECONFIG_FILE, no_x_xwr_xwr)
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Ensure that the --kubeconfig kubelet.conf file permissions are set to 600 or more restrictive".to_string(),
            id: "4.1.5".to_string(),
            level: 1,
            name: "k8s04010500".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04010600Checker {}

impl Checker for K8S04010600Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        ensure_file_owner_and_group_root(sac, KUBELET_KUBECONFIG_FILE)
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Ensure that the --kubeconfig kubelet.conf file ownership is set to root:root"
                .to_string(),
            id: "4.1.6".to_string(),
            level: 1,
            name: "k8s04010600".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04010700Checker {}

impl Checker for K8S04010700Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        let no_x_xw_xw = S_IXUSR | S_IXGRP | S_IWGRP | S_IXOTH | S_IWOTH;
        check_file_not_mode(sac, KUBELET_CLIENT_CA_FILE, no_x_xw_xw)
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Ensure that the certificate authorities file permissions are set to 644 or more restrictive".to_string(),
            id: "4.1.7".to_string(),
            level: 1,
            name: "k8s04010700".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04010800Checker {}

impl Checker for K8S04010800Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        ensure_file_owner_and_group_root(sac, KUBELET_CLIENT_CA_FILE)
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title:
                "Ensure that the client certificate authorities file ownership is set to root:root"
                    .to_string(),
            id: "4.1.8".to_string(),
            level: 1,
            name: "k8s04010800".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04010900Checker {}

impl Checker for K8S04010900Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        let no_x_xwr_xwr = S_IXUSR | S_IRWXG | S_IRWXO;
        check_file_not_mode(sac, KUBELET_CONF_FILE, no_x_xwr_xwr)
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "If the kubelet config.yaml configuration file is being used validate permissions set to 600 or more restrictive".to_string(),
            id: "4.1.9".to_string(),
            level: 1,
            name: "k8s04010900".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04011000Checker {}

impl Checker for K8S04011000Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        ensure_file_owner_and_group_root(sac, KUBELET_CONF_FILE)
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "If the kubelet config.yaml configuration file is being used validate file ownership is set to root:root"
                .to_string(),
            id: "4.1.10".to_string(),
            level: 1,
            name: "k8s04011000".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04020100Checker {}

impl Checker for K8S04020100Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        #[derive(Deserialize)]
        struct Anonymous {
            enabled: bool,
        }

        #[derive(Deserialize)]
        struct Authentication {
            anonymous: Anonymous,
        }

        #[derive(Deserialize)]
        struct KubeletConfig {
            authentication: Authentication,
        }

        let mut result = CheckerResult::default();

        if let Ok(kubelet_file) = sac.open(KUBELET_CONF_FILE) {
            if let Ok(config) = serde_yaml::from_reader::<_, KubeletConfig>(kubelet_file) {
                if config.authentication.anonymous.enabled {
                    result.error = "anonymous authentication is configured".to_string();
                    result.status = CheckStatus::FAIL;
                } else {
                    result.status = CheckStatus::PASS;
                }
            } else {
                result.error = "unable to parse kubelet config".to_string()
            }
        } else {
            result.error = format!("unable to read '{KUBELET_CONF_FILE}'");
        }

        result
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Ensure that the --anonymous-auth argument is set to false".to_string(),
            id: "4.2.1".to_string(),
            level: 1,
            name: "k8s04020100".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04020200Checker {}

impl Checker for K8S04020200Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        #[derive(Deserialize)]
        struct Authorization {
            mode: String,
        }

        #[derive(Deserialize)]
        struct KubeletConfig {
            authorization: Authorization,
        }

        let mut result = CheckerResult::default();

        if let Ok(kubelet_file) = sac.open(KUBELET_CONF_FILE) {
            if let Ok(config) = serde_yaml::from_reader::<_, KubeletConfig>(kubelet_file) {
                if config.authorization.mode == "AlwaysAllow" {
                    result.error = "AlwaysAllow authorization is configured".to_string();
                    result.status = CheckStatus::FAIL;
                } else {
                    result.status = CheckStatus::PASS;
                }
            } else {
                result.error = "unable to parse kubelet config".to_string()
            }
        } else {
            result.error = format!("unable to read '{KUBELET_CONF_FILE}'");
        }

        result
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Ensure that the --authorization-mode argument is not set to AlwaysAllow"
                .to_string(),
            id: "4.2.2".to_string(),
            level: 1,
            name: "k8s04020200".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04020300Checker {}

impl Checker for K8S04020300Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        #[derive(Deserialize)]
        struct X509 {
            #[serde(rename = "clientCAFile")]
            client_ca_file: String,
        }

        #[derive(Deserialize)]
        struct Authentication {
            x509: X509,
        }

        #[derive(Deserialize)]
        struct KubeletConfig {
            authentication: Authentication,
        }

        let mut result = CheckerResult::default();

        if let Ok(kubelet_file) = sac.open(KUBELET_CONF_FILE) {
            if let Ok(config) = serde_yaml::from_reader::<_, KubeletConfig>(kubelet_file) {
                if !config.authentication.x509.client_ca_file.is_empty()
                    && sac.exists(&config.authentication.x509.client_ca_file)
                {
                    result.status = CheckStatus::PASS;
                } else {
                    result.error = "CA file not set to expected path".to_string();
                    result.status = CheckStatus::FAIL;
                }
            } else {
                result.error = "unable to parse kubelet config".to_string()
            }
        } else {
            result.error = format!("unable to read '{KUBELET_CONF_FILE}'");
        }

        result
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Ensure that the --client-ca-file argument is set as appropriate".to_string(),
            id: "4.2.3".to_string(),
            level: 1,
            name: "k8s04020300".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04020400Checker {}

impl Checker for K8S04020400Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        #[derive(Deserialize)]
        struct KubeletConfig {
            #[serde(rename = "readOnlyPort")]
            read_only_port: i32,
        }

        let mut result = CheckerResult::default();

        if let Ok(kubelet_file) = sac.open(KUBELET_CONF_FILE) {
            if let Ok(config) = serde_yaml::from_reader::<_, KubeletConfig>(kubelet_file) {
                if config.read_only_port != 0 {
                    result.error = "Kubelet readOnlyPort not set to 0".to_string();
                    result.status = CheckStatus::FAIL;
                } else {
                    result.status = CheckStatus::PASS;
                }
            } else {
                result.error = "unable to parse kubelet config".to_string()
            }
        } else {
            result.error = format!("unable to read '{KUBELET_CONF_FILE}'");
        }

        result
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Verify that if defined, readOnlyPort is set to 0".to_string(),
            id: "4.2.4".to_string(),
            level: 1,
            name: "k8s04020400".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04020500Checker {}

impl Checker for K8S04020500Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        #[derive(Deserialize)]
        struct KubeletConfig {
            #[serde(rename = "streamingConnectionIdleTimeout")]
            streaming_connection_idle_timeout: i32,
        }

        let mut result = CheckerResult::default();

        if let Ok(kubelet_file) = sac.open(KUBELET_CONF_FILE) {
            if let Ok(config) = serde_yaml::from_reader::<_, KubeletConfig>(kubelet_file) {
                if config.streaming_connection_idle_timeout == 0 {
                    result.error = "Kubelet streamingConnectionIdleTimeout is set to 0".to_string();
                    result.status = CheckStatus::FAIL;
                } else {
                    result.status = CheckStatus::PASS;
                }
            } else {
                // Normally this value should not be present in the config file, so deserialization is expected to fail.
                result.status = CheckStatus::PASS;
            }
        } else {
            result.error = format!("unable to read '{KUBELET_CONF_FILE}'");
        }

        result
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Ensure that the --streaming-connection-idle-timeout argument is not set to 0"
                .to_string(),
            id: "4.2.5".to_string(),
            level: 1,
            name: "k8s04020500".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04020600Checker {}

impl Checker for K8S04020600Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        #[derive(Deserialize)]
        struct KubeletConfig {
            #[serde(rename = "makeIPTablesUtilChains")]
            make_iptables_util_chains: bool,
        }

        let mut result = CheckerResult::default();

        if let Ok(kubelet_file) = sac.open(KUBELET_CONF_FILE) {
            if let Ok(config) = serde_yaml::from_reader::<_, KubeletConfig>(kubelet_file) {
                if !config.make_iptables_util_chains {
                    result.error = "Kubelet makeIPTablesUtilChains is disabled".to_string();
                    result.status = CheckStatus::FAIL;
                } else {
                    result.status = CheckStatus::PASS;
                }
            } else {
                // Normally this value should not be present in the config file, so deserialization is expected to fail.
                result.status = CheckStatus::PASS;
            }
        } else {
            result.error = format!("unable to read '{KUBELET_CONF_FILE}'");
        }

        result
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Ensure that the --make-iptables-util-chains argument is set to true"
                .to_string(),
            id: "4.2.6".to_string(),
            level: 1,
            name: "k8s04020600".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04020900Checker {}

impl Checker for K8S04020900Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        #[derive(Deserialize)]
        struct KubeletConfig {
            #[serde(rename = "tlsCertFile")]
            tls_cert_file: String,
            #[serde(rename = "tlsPrivateKeyFile")]
            tls_private_key_file: String,
        }

        let mut result = CheckerResult::default();

        if let Ok(kubelet_file) = sac.open(KUBELET_CONF_FILE) {
            if let Ok(config) = serde_yaml::from_reader::<_, KubeletConfig>(kubelet_file) {
                if (!config.tls_cert_file.is_empty() && sac.exists(&config.tls_cert_file))
                    && (!config.tls_private_key_file.is_empty()
                        && sac.exists(&config.tls_private_key_file))
                {
                    result.status = CheckStatus::PASS;
                } else {
                    result.error = "TLS files not set to expected path".to_string();
                    result.status = CheckStatus::FAIL;
                }
            } else {
                // If certs not provided then `serverTLSBootstrap` will be used. Deserialization expected to fail in this case.
                result.status = CheckStatus::PASS;
            }
        } else {
            result.error = format!("unable to read '{KUBELET_CONF_FILE}'");
        }

        result
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Ensure that the --tls-cert-file and --tls-private-key-file arguments are set as appropriate".to_string(),
            id: "4.2.9".to_string(),
            level: 1,
            name: "k8s04020900".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=
#[allow(dead_code)]
pub struct K8S04021000Checker {}

// Not actually applicable for Bottlerocket, but leaving logic here in case we
// make any changes in the future.
impl Checker for K8S04021000Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        #[derive(Deserialize)]
        struct KubeletConfig {
            #[serde(rename = "rotateCertificates")]
            rotate_certificates: bool,
        }

        let mut result = CheckerResult::default();

        if let Ok(kubelet_file) = sac.open(KUBELET_CONF_FILE) {
            if let Ok(config) = serde_yaml::from_reader::<_, KubeletConfig>(kubelet_file) {
                if !config.rotate_certificates {
                    result.error = "Kubelet rotateCertificates is disabled".to_string();
                    result.status = CheckStatus::FAIL;
                } else {
                    result.status = CheckStatus::PASS;
                }
            } else {
                // Default value is `false`, so it is a failure if this is not in the config file.
                result.error = "Kubelet rotateCertificates is disabled".to_string();
                result.status = CheckStatus::FAIL;
            }
        } else {
            result.error = format!("unable to read '{KUBELET_CONF_FILE}'");
        }

        result
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Ensure that the --rotate-certificates argument is not set to false".to_string(),
            id: "4.2.10".to_string(),
            level: 1,
            name: "k8s04021000".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04021100Checker {}

impl Checker for K8S04021100Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        #[derive(Deserialize)]
        struct FeatureGates {
            #[serde(rename = "RotateKubeletServerCertificate")]
            rotate_kubelet_server_certificate: bool,
        }

        #[derive(Deserialize)]
        struct KubeletConfig {
            #[serde(rename = "featureGates")]
            feature_gates: FeatureGates,
        }

        let mut result = CheckerResult::default();

        if let Ok(kubelet_file) = sac.open(KUBELET_CONF_FILE) {
            if let Ok(config) = serde_yaml::from_reader::<_, KubeletConfig>(kubelet_file) {
                if !config.feature_gates.rotate_kubelet_server_certificate {
                    result.error = "Kubelet RotateKubeletServerCertificate is disabled".to_string();
                    result.status = CheckStatus::FAIL;
                } else {
                    result.status = CheckStatus::PASS;
                }
            } else {
                // Feature gate has been defaulted to enabled since k8s 1.12, so if it is not found that is fine
                result.status = CheckStatus::PASS;
            }
        } else {
            result.error = format!("unable to read '{KUBELET_CONF_FILE}'");
        }

        result
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Verify that the RotateKubeletServerCertificate argument is set to true"
                .to_string(),
            id: "4.2.11".to_string(),
            level: 1,
            name: "k8s04021100".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04021200Checker {}

impl Checker for K8S04021200Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        let allowed_suites: HashSet<&str> = vec![
            "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256",
            "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256",
            "TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305",
            "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384",
            "TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305",
            "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384",
        ]
        .into_iter()
        .collect();

        #[derive(Deserialize)]
        struct KubeletConfig {
            #[serde(rename = "tlsCipherSuites")]
            tls_cipher_suites: Vec<String>,
        }

        let mut result = CheckerResult::default();

        if let Ok(kubelet_file) = sac.open(KUBELET_CONF_FILE) {
            if let Ok(config) = serde_yaml::from_reader::<_, KubeletConfig>(kubelet_file) {
                let configured_suites: HashSet<&str> = config
                    .tls_cipher_suites
                    .iter()
                    .map(|s| s.as_str())
                    .collect();
                if !configured_suites.is_subset(&allowed_suites) {
                    result.error = "Found disallowed cipher suites".to_string();
                    result.status = CheckStatus::FAIL;
                } else {
                    result.status = CheckStatus::PASS;
                }
            } else {
                result.error = "unable to parse kubelet config".to_string()
            }
        } else {
            result.error = format!("unable to read '{KUBELET_CONF_FILE}'");
        }

        result
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Ensure that the Kubelet only makes use of Strong Cryptographic Ciphers"
                .to_string(),
            id: "4.2.12".to_string(),
            level: 1,
            name: "k8s04021200".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04021300Checker {}

impl Checker for K8S04021300Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        #[derive(Deserialize)]
        struct KubeletConfig {
            #[serde(rename = "podPidsLimit")]
            pod_pids_limit: i64,
        }

        let mut result = CheckerResult::default();

        if let Ok(kubelet_file) = sac.open(KUBELET_CONF_FILE) {
            if let Ok(config) = serde_yaml::from_reader::<_, KubeletConfig>(kubelet_file) {
                if config.pod_pids_limit <= 0 {
                    result.error = "podPidsLimit is unrestricted".to_string();
                    result.status = CheckStatus::FAIL;
                } else {
                    result.status = CheckStatus::PASS;
                }
            } else {
                // If the setting is not present then there is no pod pid limit (whatever the host allows)
                result.error = "podPidsLimit is not configured".to_string();
                result.status = CheckStatus::FAIL;
            }
        } else {
            result.error = format!("unable to read '{KUBELET_CONF_FILE}'");
        }

        result
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Ensure that a limit is set on pod PIDs".to_string(),
            id: "4.2.13".to_string(),
            level: 1,
            name: "k8s04021300".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04021400Checker {}

impl Checker for K8S04021400Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        #[derive(Deserialize)]
        struct KubeletConfig {
            #[serde(rename = "seccompDefault")]
            seccomp_default: bool,
        }

        let mut result = CheckerResult::default();

        if let Ok(kubelet_file) = sac.open(KUBELET_CONF_FILE) {
            if let Ok(config) = serde_yaml::from_reader::<_, KubeletConfig>(kubelet_file) {
                if !config.seccomp_default {
                    result.error = "Kubelet seccompDefault is not set to true".to_string();
                    result.status = CheckStatus::FAIL;
                } else {
                    result.status = CheckStatus::PASS;
                }
            } else {
                // If the setting is not present then seccompDefault is false by default
                result.error =
                    "Kubelet seccompDefault is not configured or set to true".to_string();
                result.status = CheckStatus::FAIL;
            }
        } else {
            result.error = format!("unable to read '{KUBELET_CONF_FILE}'");
        }

        result
    }

    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Ensure that the --seccomp-default parameter is set to true".to_string(),
            id: "4.2.14".to_string(),
            level: 1,
            name: "k8s04021400".to_string(),
            mode: Mode::Automatic,
        }
    }
}
// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04030100Checker {}
impl Checker for K8S04030100Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        #[derive(Deserialize)]
        struct KubeProxyConfig {
            #[serde(rename = "metricsBindAddress")]
            metrics_bind_address: String,
        }
        let mut result = CheckerResult::default();

        if let Ok(kubelet_file) = sac.open(KUBEPROXY_CONF_FILE) {
            if let Ok(config) = serde_yaml::from_reader::<_, KubeProxyConfig>(kubelet_file) {
                if config.metrics_bind_address.contains("0.0.0.0")
                    || config.metrics_bind_address.contains("[::]")
                {
                    result.error =
                        "Kubelet metricsBindAddress binds to more than localhost".to_string();
                    result.status = CheckStatus::FAIL;
                } else {
                    result.status = CheckStatus::PASS;
                }
            } else {
                // If the setting is not present it defaults to 127.0.0.1:10249 which is localhost only
                result.status = CheckStatus::PASS;
            }
        } else {
            result.error = format!("unable to read '{KUBEPROXY_CONF_FILE}'");
        }
        result
    }
    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "Ensure that the kube-proxy metrics service is bound to localhost".to_string(),
            id: "4.3.1".to_string(),
            level: 1,
            name: "k8s04030100".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04010300Checker {}
impl Checker for K8S04010300Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        let no_x_xwr_xwr = S_IXUSR | S_IRWXG | S_IRWXO;
        check_file_not_mode(sac, KUBEPROXY_CONF_FILE, no_x_xwr_xwr)
    }
    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "If proxy kubeconfig file exists ensure permissions are set to 600 or more restrictive".to_string(),
            id: "4.1.3".to_string(),
            level: 1,
            name: "k8s04010300".to_string(),
            mode: Mode::Automatic,
        }
    }
}

// =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<= =>o.o<=

pub struct K8S04010400Checker {}
impl Checker for K8S04010400Checker {
    fn execute(&self, sac: &dyn SystemAccess) -> CheckerResult {
        ensure_file_owner_and_group_root(sac, KUBEPROXY_CONF_FILE)
    }
    fn metadata(&self) -> CheckerMetadata {
        CheckerMetadata {
            title: "If proxy kubeconfig file exists ensure ownership is set to root:root"
                .to_string(),
            id: "4.1.4".to_string(),
            level: 1,
            name: "k8s04010400".to_string(),
            mode: Mode::Automatic,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use bloodhound::results::{CheckStatus, Checker};
    use bloodhound::system_access::UnitTestSystemAccess;

    // K8S04010100Checker tests - kubelet service file permissions
    #[test]
    pub fn test_k8s04010100checker_pass() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBELET_SERVICE_FILE, "", 0o600, 0, 0);
        let checker = K8S04010100Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04010100checker_fail_too_permissive() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBELET_SERVICE_FILE, "", 0o755, 0, 0);
        let checker = K8S04010100Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04010100checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04010100Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04010200Checker tests - kubelet service file ownership
    #[test]
    pub fn test_k8s04010200checker_pass() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBELET_SERVICE_FILE, "", 0o600, 0, 0);
        let checker = K8S04010200Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04010200checker_fail_wrong_owner() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBELET_SERVICE_FILE, "", 0o600, 1000, 1000);
        let checker = K8S04010200Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04010200checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04010200Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04010300Checker tests - proxy kubeconfig file permissions
    #[test]
    pub fn test_k8s04010300checker_pass() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBEPROXY_CONF_FILE, "", 0o600, 0, 0);
        let checker = K8S04010300Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04010300checker_fail_too_permissive() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBEPROXY_CONF_FILE, "", 0o644, 0, 0);
        let checker = K8S04010300Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04010300checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04010300Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04010400Checker tests - proxy kubeconfig file ownership
    #[test]
    pub fn test_k8s04010400checker_pass() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBEPROXY_CONF_FILE, "", 0o600, 0, 0);
        let checker = K8S04010400Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04010400checker_fail_wrong_owner() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBEPROXY_CONF_FILE, "", 0o600, 500, 500);
        let checker = K8S04010400Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04010400checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04010400Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04010500Checker tests - kubelet kubeconfig file permissions
    #[test]
    pub fn test_k8s04010500checker_pass() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBELET_KUBECONFIG_FILE, "", 0o600, 0, 0);
        let checker = K8S04010500Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04010500checker_fail_too_permissive() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBELET_KUBECONFIG_FILE, "", 0o777, 0, 0);
        let checker = K8S04010500Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04010500checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04010500Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04010600Checker tests - kubelet kubeconfig file ownership
    #[test]
    pub fn test_k8s04010600checker_pass() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBELET_KUBECONFIG_FILE, "", 0o600, 0, 0);
        let checker = K8S04010600Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04010600checker_fail_wrong_owner() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBELET_KUBECONFIG_FILE, "", 0o600, 1001, 0);
        let checker = K8S04010600Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04010600checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04010600Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04010700Checker tests - certificate authorities file permissions
    #[test]
    pub fn test_k8s04010700checker_pass() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBELET_CLIENT_CA_FILE, "", 0o644, 0, 0);
        let checker = K8S04010700Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04010700checker_fail_too_permissive() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBELET_CLIENT_CA_FILE, "", 0o666, 0, 0);
        let checker = K8S04010700Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04010700checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04010700Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04010800Checker tests - certificate authorities file ownership
    #[test]
    pub fn test_k8s04010800checker_pass() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBELET_CLIENT_CA_FILE, "", 0o644, 0, 0);
        let checker = K8S04010800Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04010800checker_fail_wrong_owner() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBELET_CLIENT_CA_FILE, "", 0o644, 0, 100);
        let checker = K8S04010800Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04010800checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04010800Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04010900Checker tests - kubelet config file permissions
    #[test]
    pub fn test_k8s04010900checker_pass() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBELET_CONF_FILE, "", 0o600, 0, 0);
        let checker = K8S04010900Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04010900checker_fail_too_permissive() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBELET_CONF_FILE, "", 0o666, 0, 0);
        let checker = K8S04010900Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04010900checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04010900Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04011000Checker tests - kubelet config file ownership
    #[test]
    pub fn test_k8s04011000checker_pass() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBELET_CONF_FILE, "", 0o600, 0, 0);
        let checker = K8S04011000Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04011000checker_fail_wrong_owner() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file_with_metadata(KUBELET_CONF_FILE, "", 0o600, 1000, 1000);
        let checker = K8S04011000Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04011000checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04011000Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }
    // K8S04020100Checker tests - anonymous authentication
    #[test]
    pub fn test_k8s04020100checker_pass() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
authentication:
  anonymous:
    enabled: false
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04020100Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04020100checker_fail_anonymous_enabled() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
authentication:
  anonymous:
    enabled: true
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04020100Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04020100checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04020100Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    #[test]
    pub fn test_k8s04020100checker_invalid_config() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file(KUBELET_CONF_FILE, "invalid yaml content");
        let checker = K8S04020100Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04020200Checker tests - authorization mode
    #[test]
    pub fn test_k8s04020200checker_pass() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
authorization:
  mode: Webhook
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04020200Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04020200checker_fail_always_allow() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
authorization:
  mode: AlwaysAllow
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04020200Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04020200checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04020200Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    #[test]
    pub fn test_k8s04020200checker_invalid_config() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file(KUBELET_CONF_FILE, "invalid: yaml: content:");
        let checker = K8S04020200Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04020300Checker tests - client CA file
    #[test]
    pub fn test_k8s04020300checker_pass() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
authentication:
  x509:
    clientCAFile: /etc/kubernetes/pki/ca.crt
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        // Register the CA file to simulate it exists
        sac.register_file("/etc/kubernetes/pki/ca.crt", "dummy ca content");
        let checker = K8S04020300Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04020300checker_fail_ca_file_missing() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
authentication:
  x509:
    clientCAFile: /nonexistent/ca.crt
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04020300Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04020300checker_fail_empty_ca_file() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
authentication:
  x509:
    clientCAFile: ""
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04020300Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04020300checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04020300Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04020400Checker tests - read only port
    #[test]
    pub fn test_k8s04020400checker_pass() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
readOnlyPort: 0
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04020400Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04020400checker_fail_port_not_zero() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
readOnlyPort: 10255
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04020400Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04020400checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04020400Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    #[test]
    pub fn test_k8s04020400checker_invalid_config() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file(KUBELET_CONF_FILE, "invalid yaml");
        let checker = K8S04020400Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04020500Checker tests - streaming connection idle timeout
    #[test]
    pub fn test_k8s04020500checker_pass_timeout_set() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
streamingConnectionIdleTimeout: 300
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04020500Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04020500checker_pass_timeout_not_present() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
someOtherSetting: value
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04020500Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04020500checker_fail_timeout_zero() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
streamingConnectionIdleTimeout: 0
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04020500Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04020500checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04020500Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04020600Checker tests - make iptables util chains
    #[test]
    pub fn test_k8s04020600checker_pass_enabled() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
makeIPTablesUtilChains: true
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04020600Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04020600checker_pass_not_present() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
someOtherSetting: value
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04020600Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04020600checker_fail_disabled() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
makeIPTablesUtilChains: false
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04020600Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04020600checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04020600Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }
    // K8S04020900Checker tests - TLS cert and key files
    #[test]
    pub fn test_k8s04020900checker_pass_with_files() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
tlsCertFile: /etc/kubernetes/pki/kubelet.crt
tlsPrivateKeyFile: /etc/kubernetes/pki/kubelet.key
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        sac.register_file("/etc/kubernetes/pki/kubelet.crt", "dummy cert");
        sac.register_file("/etc/kubernetes/pki/kubelet.key", "dummy key");
        let checker = K8S04020900Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04020900checker_pass_without_files() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
someOtherSetting: value
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04020900Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04020900checker_fail_cert_missing() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
tlsCertFile: /nonexistent/kubelet.crt
tlsPrivateKeyFile: /etc/kubernetes/pki/kubelet.key
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        sac.register_file("/etc/kubernetes/pki/kubelet.key", "dummy key");
        let checker = K8S04020900Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04020900checker_fail_key_missing() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
tlsCertFile: /etc/kubernetes/pki/kubelet.crt
tlsPrivateKeyFile: /nonexistent/kubelet.key
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        sac.register_file("/etc/kubernetes/pki/kubelet.crt", "dummy cert");
        let checker = K8S04020900Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04020900checker_fail_empty_paths() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
tlsCertFile: ""
tlsPrivateKeyFile: ""
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04020900Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04020900checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04020900Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04021000Checker tests - rotate certificates
    #[test]
    pub fn test_k8s04021000checker_pass() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
rotateCertificates: true
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04021000Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04021000checker_fail_disabled() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
rotateCertificates: false
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04021000Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04021000checker_fail_not_present() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
someOtherSetting: value
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04021000Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04021000checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04021000Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04021100Checker tests - rotate kubelet server certificate
    #[test]
    pub fn test_k8s04021100checker_pass_enabled() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
featureGates:
  RotateKubeletServerCertificate: true
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04021100Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04021100checker_pass_not_present() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
someOtherSetting: value
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04021100Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04021100checker_fail_disabled() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
featureGates:
  RotateKubeletServerCertificate: false
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04021100Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04021100checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04021100Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04021200Checker tests - TLS cipher suites
    #[test]
    pub fn test_k8s04021200checker_pass_allowed_suites() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
tlsCipherSuites:
  - TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256
  - TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04021200Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04021200checker_fail_disallowed_suites() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
tlsCipherSuites:
  - TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256
  - TLS_RSA_WITH_RC4_128_SHA
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04021200Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04021200checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04021200Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    #[test]
    pub fn test_k8s04021200checker_invalid_config() {
        let mut sac = UnitTestSystemAccess::default();
        sac.register_file(KUBELET_CONF_FILE, "invalid yaml");
        let checker = K8S04021200Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04021300Checker tests - pod PIDs limit
    #[test]
    pub fn test_k8s04021300checker_pass() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
podPidsLimit: 1024
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04021300Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04021300checker_fail_zero_limit() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
podPidsLimit: 0
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04021300Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04021300checker_fail_negative_limit() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
podPidsLimit: -1
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04021300Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04021300checker_fail_not_configured() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
someOtherSetting: value
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04021300Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04021300checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04021300Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }

    // K8S04021400Checker tests - seccomp default
    #[test]
    pub fn test_k8s04021400checker_pass() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
seccompDefault: true
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04021400Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04021400checker_fail_disabled() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
seccompDefault: false
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04021400Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04021400checker_fail_not_configured() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
someOtherSetting: value
"#;
        sac.register_file(KUBELET_CONF_FILE, config);
        let checker = K8S04021400Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04021400checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04021400Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }
    // K8S04030100Checker tests - kube-proxy metrics bind address
    #[test]
    pub fn test_k8s04030100checker_pass_localhost() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
metricsBindAddress: 127.0.0.1:10249
"#;
        sac.register_file(KUBEPROXY_CONF_FILE, config);
        let checker = K8S04030100Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04030100checker_pass_not_configured() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
someOtherSetting: value
"#;
        sac.register_file(KUBEPROXY_CONF_FILE, config);
        let checker = K8S04030100Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::PASS);
    }

    #[test]
    pub fn test_k8s04030100checker_fail_all_interfaces_ipv4() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
metricsBindAddress: 0.0.0.0:10249
"#;
        sac.register_file(KUBEPROXY_CONF_FILE, config);
        let checker = K8S04030100Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04030100checker_fail_all_interfaces_ipv6() {
        let mut sac = UnitTestSystemAccess::default();
        let config = r#"
metricsBindAddress: "[::]:10249"
"#;
        sac.register_file(KUBEPROXY_CONF_FILE, config);
        let checker = K8S04030100Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::FAIL);
    }

    #[test]
    pub fn test_k8s04030100checker_file_missing() {
        let sac = UnitTestSystemAccess::default();
        let checker = K8S04030100Checker {};
        let result = checker.execute(&sac);
        assert_eq!(result.status, CheckStatus::SKIP);
    }
}
