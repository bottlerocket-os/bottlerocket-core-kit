/*!
# Introduction

pluto is called by sundog to generate settings required by Kubernetes.
This is done dynamically because we require access to dynamic networking
and cluster setup information.

It uses IMDS to get information such as:

- Instance Type
- Node IP

It uses EKS to get information such as:

- Service IP CIDR

It uses the Bottlerocket API to get information such as:

- Kubernetes Cluster Name
- AWS Region

# Interface

Pluto takes the name of the setting that it is to generate as its first
argument.
It returns the generated setting to stdout as a JSON document.
Any other output is returned to stderr.

Pluto returns a special exit code of 2 to inform `sundog` that a setting should be skipped. For
example, if `max-pods` cannot be generated, we want `sundog` to skip it without failing since a
reasonable default is available.
*/

mod api;
mod aws;
mod ec2;
mod eks;

use api::{settings_view_get, settings_view_set, SettingsViewDelta};
use aws_smithy_experimental::hyper_1_0::CryptoMode;
use base64::Engine;
use bottlerocket_modeled_types::{KubernetesClusterDnsIp, KubernetesHostnameOverrideSource};
use imdsclient::ImdsClient;
use snafu::{ensure, OptionExt, ResultExt};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;
use std::string::String;
use std::{env, process};

// This is the default DNS unless our CIDR block begins with "10."
const DEFAULT_DNS_CLUSTER_IP: &str = "10.100.0.10";
// If our CIDR block begins with "10." this is our DNS.
const DEFAULT_10_RANGE_DNS_CLUSTER_IP: &str = "172.20.0.10";

const ENI_MAX_PODS_PATH: &str = "/usr/share/eks/eni-max-pods";
const ENI_MAX_PODS_OVERRIDE_PATH: &str = "/usr/share/eks/eni-max-pods-override";

/// The name of the AWS config file used by pluto. The file is placed in a tempdir, and
/// the contents of settings.aws.config are decoded and written here.
const AWS_CONFIG_FILE: &str = "config.pluto";
/// The environment variable that specifies the path to the AWS config file.
const AWS_CONFIG_FILE_ENV_VAR: &str = "AWS_CONFIG_FILE";

// Shared crypto provider for HyperClients
#[cfg(not(feature = "fips"))]
const PROVIDER: CryptoMode = CryptoMode::AwsLc;
#[cfg(feature = "fips")]
const PROVIDER: CryptoMode = CryptoMode::AwsLcFips;

mod error {
    use crate::{api, ec2, eks};
    use snafu::Snafu;
    use std::net::AddrParseError;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub(super) enum PlutoError {
        #[snafu(display(
            "Unable to retrieve cluster name and AWS region from Bottlerocket API: {}",
            source
        ))]
        AwsInfo { source: api::Error },

        #[snafu(display("Missing AWS region"))]
        AwsRegion,

        #[snafu(display("Unable to decode base64 in of AWS config: {}", source))]
        AwsBase64Decode { source: base64::DecodeError },

        #[snafu(display("Failed to parse setting {} as u32: {}", setting, source))]
        ParseToU32 {
            setting: String,
            source: std::num::ParseIntError,
        },

        #[snafu(display("Unable to parse CIDR '{}': {}", cidr, reason))]
        CidrParse { cidr: String, reason: String },

        #[snafu(display("Unable to parse IP '{}': {}", ip, source))]
        BadIp { ip: String, source: AddrParseError },

        #[snafu(display("No IP address found for this host"))]
        NoIp,

        #[snafu(display("IMDS request failed: {}", source))]
        ImdsRequest { source: imdsclient::Error },

        #[snafu(display("IMDS request failed: No '{}' found", what))]
        ImdsNone { what: String },

        #[snafu(display("Invalid hostname: {}", source))]
        InvalidHostname {
            source: bottlerocket_modeled_types::error::Error,
        },

        #[snafu(display("Invalid URL: {}", source))]
        InvalidUrl {
            source: bottlerocket_modeled_types::error::Error,
        },

        #[snafu(display("{}", source))]
        EksError { source: eks::Error },

        #[snafu(display("{}", source))]
        Ec2Error { source: ec2::Error },

        #[snafu(display("Failed to open eni-max-pods file at {}: {}", path, source))]
        EniMaxPodsFile {
            path: &'static str,
            source: std::io::Error,
        },

        #[snafu(display("Failed to read line: {}", source))]
        IoReadLine { source: std::io::Error },

        #[snafu(display("Failed to serialize generated settings: {}", source))]
        Serialize { source: serde_json::Error },

        #[snafu(display("Failed to set generated settings: {}", source))]
        SetFailure { source: api::Error },

        #[snafu(display(
            "Unable to find maximum number of pods supported for instance-type {}",
            instance_type
        ))]
        NoInstanceTypeMaxPods { instance_type: String },

        #[snafu(display("Unable to create AWS config file '{}': {}", filepath, source))]
        CreateAwsConfigFile {
            filepath: String,
            source: std::io::Error,
        },

        #[snafu(display("Unable to write AWS config file to '{}': {}", filepath, source))]
        WriteAwsConfigFile {
            filepath: String,
            source: std::io::Error,
        },

        #[snafu(display("Unable to create tempdir: {}", source))]
        Tempdir { source: std::io::Error },
    }
}

use error::PlutoError;

type Result<T> = std::result::Result<T, PlutoError>;

async fn generate_max_pods(
    client: &mut ImdsClient,
    aws_k8s_info: &mut SettingsViewDelta,
) -> Result<()> {
    if settings_view_get!(aws_k8s_info.kubernetes.max_pods).is_some() {
        return Ok(());
    }
    if let Ok(max_pods) = get_max_pods(client).await {
        settings_view_set!(aws_k8s_info.kubernetes.max_pods = max_pods);
    }
    Ok(())
}

async fn get_max_pods(client: &mut ImdsClient) -> Result<u32> {
    let instance_type = client
        .fetch_instance_type()
        .await
        .context(error::ImdsRequestSnafu)?
        .context(error::ImdsNoneSnafu {
            what: "instance_type",
        })?;

    if let Ok(max_pods) = get_max_pods_from_file(&instance_type, ENI_MAX_PODS_OVERRIDE_PATH).await {
        return Ok(max_pods);
    }
    get_max_pods_from_file(&instance_type, ENI_MAX_PODS_PATH).await
}

// Returns the max-pods as determined by the specified instance type and max-pods file
async fn get_max_pods_from_file(instance_type: &str, pods_file: &'static str) -> Result<u32> {
    // Find the corresponding maximum number of pods supported by this instance type
    let file = BufReader::new(
        File::open(pods_file).context(error::EniMaxPodsFileSnafu { path: pods_file })?,
    );
    for line in file.lines() {
        let line = line.context(error::IoReadLineSnafu)?;
        // Skip the comments in the file
        if line.trim_start().starts_with('#') {
            continue;
        }
        let tokens: Vec<_> = line.split_whitespace().collect();
        if tokens.len() == 2 && tokens[0] == instance_type {
            let setting = tokens[1];
            return setting.parse().context(error::ParseToU32Snafu { setting });
        }
    }
    error::NoInstanceTypeMaxPodsSnafu { instance_type }.fail()
}

/// Returns the cluster's DNS address.
///
/// For IPv4 clusters, first it attempts to call EKS describe-cluster to find the `serviceIpv4Cidr`.
/// If that works, it returns the expected cluster DNS IP address which is obtained by substituting
/// `10` for the last octet. If the EKS call is not successful, it falls back to using IMDS MAC CIDR
/// blocks to return one of two default addresses.
async fn generate_cluster_dns_ip(
    client: &mut ImdsClient,
    aws_k8s_info: &mut SettingsViewDelta,
) -> Result<()> {
    if settings_view_get!(aws_k8s_info.kubernetes.cluster_dns_ip).is_some() {
        return Ok(());
    }

    // Retrieve the kubernetes network configuration for the EKS cluster
    let ip_addr = if let Some(ip) = get_eks_network_config(aws_k8s_info).await? {
        ip.clone()
    } else {
        // If we were unable to obtain or parse the cidr range from EKS, fallback to one of two default
        // values based on the IPv4 cidr range of our primary network interface
        get_ipv4_cluster_dns_ip_from_imds_mac(client).await?
    };

    settings_view_set!(
        aws_k8s_info.kubernetes.cluster_dns_ip = KubernetesClusterDnsIp::Scalar(
            IpAddr::from_str(ip_addr.as_str()).context(error::BadIpSnafu {
                ip: ip_addr.clone(),
            })?,
        )
    );
    Ok(())
}

/// Retrieves the ip address from the kubernetes network configuration for the
/// EKS Cluster
async fn get_eks_network_config(aws_k8s_info: &SettingsViewDelta) -> Result<Option<String>> {
    if let (Some(region), Some(cluster_name)) = (
        settings_view_get!(aws_k8s_info.aws.region),
        settings_view_get!(aws_k8s_info.kubernetes.cluster_name),
    ) {
        if let Ok(config) = eks::get_cluster_network_config(
            region,
            cluster_name,
            settings_view_get!(aws_k8s_info.network.https_proxy),
            settings_view_get!(aws_k8s_info.network.no_proxy).map(Vec::as_slice),
        )
        .await
        .context(error::EksSnafu)
        {
            // Derive cluster-dns-ip from the service IPv4 CIDR
            if let Some(ipv4_cidr) = config.service_ipv4_cidr {
                if let Ok(dns_ip) = get_dns_from_ipv4_cidr(&ipv4_cidr) {
                    return Ok(Some(dns_ip));
                }
            }
        }
    }
    Ok(None)
}

/// Replicates [this] logic from the EKS AMI:
///
/// ```sh
/// DNS_CLUSTER_IP=${SERVICE_IPV4_CIDR%.*}.10
/// ```
/// [this]: https://github.com/awslabs/amazon-eks-ami/blob/732b6b2/files/bootstrap.sh#L335
fn get_dns_from_ipv4_cidr(cidr: &str) -> Result<String> {
    let mut split: Vec<&str> = cidr.split('.').collect();
    ensure!(
        split.len() == 4,
        error::CidrParseSnafu {
            cidr,
            reason: format!("expected 4 components but found {}", split.len())
        }
    );
    split[3] = "10";
    Ok(split.join("."))
}

/// Gets gets the the first VPC IPV4 CIDR block from IMDS. If it starts with `10`, returns
/// `10.100.0.10`, otherwise returns `172.20.0.10`
async fn get_ipv4_cluster_dns_ip_from_imds_mac(client: &mut ImdsClient) -> Result<String> {
    // Take the first (primary) MAC address. Others may exist from attached ENIs.
    let mac = client
        .fetch_mac_addresses()
        .await
        .context(error::ImdsRequestSnafu)?
        .context(error::ImdsNoneSnafu {
            what: "mac addresses",
        })?
        .first()
        .context(error::ImdsNoneSnafu {
            what: "mac addresses",
        })?
        .clone();

    // Take the first CIDR block for the primary MAC.
    let cidr_block = client
        .fetch_cidr_blocks_for_mac(&mac)
        .await
        .context(error::ImdsRequestSnafu)?
        .context(error::ImdsNoneSnafu {
            what: "CIDR blocks",
        })?
        .first()
        .context(error::ImdsNoneSnafu {
            what: "CIDR blocks",
        })?
        .clone();

    // Infer the cluster DNS based on the CIDR block.
    let dns = if cidr_block.starts_with("10.") {
        DEFAULT_10_RANGE_DNS_CLUSTER_IP
    } else {
        DEFAULT_DNS_CLUSTER_IP
    }
    .to_string();
    Ok(dns)
}

/// Gets the IP address that should be associated with the node.
async fn generate_node_ip(
    client: &mut ImdsClient,
    aws_k8s_info: &mut SettingsViewDelta,
) -> Result<()> {
    if settings_view_get!(aws_k8s_info.kubernetes.node_ip).is_some() {
        return Ok(());
    }
    // Ensure that this was set in case changes to main occur
    generate_cluster_dns_ip(client, aws_k8s_info).await?;
    let cluster_dns_ip = settings_view_get!(aws_k8s_info.kubernetes.cluster_dns_ip)
        .and_then(|x| x.iter().next())
        .context(error::NoIpSnafu)?;
    // If the cluster DNS IP is an IPv4 address, retrieve the IPv4 address for the instance.
    // If the cluster DNS IP is an IPv6 address, retrieve the IPv6 address for the instance.
    let node_ip = match cluster_dns_ip {
        IpAddr::V4(_) => client
            .fetch_local_ipv4_address()
            .await
            .context(error::ImdsRequestSnafu)?
            .context(error::ImdsNoneSnafu {
                what: "node ipv4 address",
            }),
        IpAddr::V6(_) => client
            .fetch_primary_ipv6_address()
            .await
            .context(error::ImdsRequestSnafu)?
            .context(error::ImdsNoneSnafu {
                what: "ipv6s associated with primary network interface",
            }),
    }?;
    settings_view_set!(
        aws_k8s_info.kubernetes.node_ip =
            IpAddr::from_str(node_ip.as_str()).context(error::BadIpSnafu {
                ip: node_ip.clone(),
            })?
    );
    Ok(())
}

/// Gets the provider ID that should be associated with the node
async fn generate_provider_id(
    client: &mut ImdsClient,
    aws_k8s_info: &mut SettingsViewDelta,
) -> Result<()> {
    if settings_view_get!(aws_k8s_info.kubernetes.provider_id).is_some() {
        return Ok(());
    }

    let instance_id = client
        .fetch_instance_id()
        .await
        .context(error::ImdsRequestSnafu)?
        .context(error::ImdsNoneSnafu {
            what: "instance ID",
        })?;

    let zone = client
        .fetch_zone()
        .await
        .context(error::ImdsRequestSnafu)?
        .context(error::ImdsNoneSnafu { what: "zone" })?;

    settings_view_set!(
        aws_k8s_info.kubernetes.provider_id = format!("aws:///{}/{}", zone, instance_id)
            .try_into()
            .context(error::InvalidUrlSnafu)?
    );
    Ok(())
}

/// generate_node_name sets the hostname_override, if it is not already specified
async fn generate_node_name(
    client: &mut ImdsClient,
    aws_k8s_info: &mut SettingsViewDelta,
) -> Result<()> {
    // hostname override provided, so we do nothing regardless of the override source
    if settings_view_get!(aws_k8s_info.kubernetes.hostname_override).is_some() {
        return Ok(());
    }

    // no hostname override or override source provided, so we don't provide this value
    let hostname_source = match settings_view_get!(aws_k8s_info.kubernetes.hostname_override_source)
    {
        None => return Ok(()),
        Some(hostname_source) => hostname_source,
    };

    let region = settings_view_get!(aws_k8s_info.aws.region).context(error::AwsRegionSnafu)?;
    let instance_id = client
        .fetch_instance_id()
        .await
        .context(error::ImdsRequestSnafu)?
        .context(error::ImdsNoneSnafu {
            what: "instance ID",
        })?;

    match hostname_source {
        KubernetesHostnameOverrideSource::PrivateDNSName => {
            let hostname_override = ec2::get_private_dns_name(
                region,
                &instance_id,
                settings_view_get!(aws_k8s_info.network.https_proxy),
                settings_view_get!(aws_k8s_info.network.no_proxy).map(Vec::as_slice),
            )
            .await
            .context(error::Ec2Snafu)?
            .try_into()
            .context(error::InvalidHostnameSnafu)?;

            settings_view_set!(aws_k8s_info.kubernetes.hostname_override = hostname_override);
        }
        KubernetesHostnameOverrideSource::InstanceID => {
            settings_view_set!(
                aws_k8s_info.kubernetes.hostname_override = instance_id
                    .try_into()
                    .context(error::InvalidHostnameSnafu)?
            );
        }
    }

    Ok(())
}

/// Temporarily copy the yet-to-be-committed settings.aws.config value to a file
/// and set the environment variable AWS_CONFIG_FILE to this file's location.
/// This ensures that subsequent calls via pluto to the AWS SDK will respect settings.aws.config.
fn set_aws_config(aws_k8s_info: &SettingsViewDelta, filepath: &Path) -> Result<()> {
    if let Some(config_contents) = settings_view_get!(aws_k8s_info.aws.config) {
        // Decode settings.aws.config.
        let decoded_bytes = base64::engine::general_purpose::STANDARD
            .decode(config_contents.as_bytes())
            .context(error::AwsBase64DecodeSnafu)?;

        // Write the decoded bytes to the provided filepath.
        let mut file = File::create(filepath).context(error::CreateAwsConfigFileSnafu {
            filepath: filepath.to_str().unwrap(),
        })?;
        file.write_all(&decoded_bytes)
            .context(error::WriteAwsConfigFileSnafu {
                filepath: filepath.to_str().unwrap(),
            })?;

        env::set_var(AWS_CONFIG_FILE_ENV_VAR, filepath);
    }

    Ok(())
}

async fn run() -> Result<()> {
    let mut client = ImdsClient::new();
    let current_settings = api::get_aws_k8s_info().await.context(error::AwsInfoSnafu)?;
    let mut aws_k8s_info = SettingsViewDelta::from_api_response(current_settings);

    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let temp_dir = tempfile::tempdir().context(error::TempdirSnafu)?;
    let aws_config_file_path = temp_dir.path().join(AWS_CONFIG_FILE);
    set_aws_config(&aws_k8s_info, Path::new(&aws_config_file_path))?;

    generate_cluster_dns_ip(&mut client, &mut aws_k8s_info).await?;
    generate_node_ip(&mut client, &mut aws_k8s_info).await?;
    generate_max_pods(&mut client, &mut aws_k8s_info).await?;
    generate_provider_id(&mut client, &mut aws_k8s_info).await?;
    generate_node_name(&mut client, &mut aws_k8s_info).await?;

    if let Some(k8s_settings) = &aws_k8s_info.delta().kubernetes {
        let generated_settings = serde_json::json!({
            "kubernetes": serde_json::to_value(k8s_settings).context(error::SerializeSnafu)?
        });
        let json_str = generated_settings.to_string();
        let uri = &format!(
            "{}?tx={}",
            constants::API_SETTINGS_URI,
            constants::LAUNCH_TRANSACTION
        );
        api::client_command(&["raw", "-m", "PATCH", "-u", uri, "-d", json_str.as_str()])
            .await
            .context(error::SetFailureSnafu)?;
    }

    Ok(())
}

// Returning a Result from main makes it print a Debug representation of the error, but with Snafu
// we have nice Display representations of the error, so we wrap "main" (run) and print any error.
// https://github.com/shepmaster/snafu/issues/110
#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{}", e);
        process::exit(1);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::api::SettingsViewDelta;
    use api::SettingsView;
    use bottlerocket_modeled_types::ValidBase64;
    use bottlerocket_settings_models::AwsSettingsV1;
    use httptest::{matchers::*, responders::*, Expectation, Server};

    #[test]
    fn test_get_dns_from_cidr_ok() {
        let input = "123.456.789.0/123";
        let expected = "123.456.789.10";
        let actual = get_dns_from_ipv4_cidr(input).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_get_dns_from_cidr_err() {
        let input = "123_456_789_0/123";
        let result = get_dns_from_ipv4_cidr(input);
        assert!(result.is_err());
    }

    // Because of test parallelization, serialize the AWS config tests such that
    // the AWS_CONFIG_FILE env variable is deterministically set or unset.
    #[test]
    fn test_aws_config_sequential() {
        test_set_aws_config();
        test_set_aws_config_is_not_set();
    }

    fn test_set_aws_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_file_path = temp_dir.path().join("config.fake");

        // base64 encoded string:
        // [default]
        // use_fips_endpoint=false
        let config_base64 =
            ValidBase64::try_from("W2RlZmF1bHRdCnVzZV9maXBzX2VuZHBvaW50PWZhbHNl").unwrap();
        let input = SettingsViewDelta::from_api_response(SettingsView {
            aws: Some(AwsSettingsV1 {
                config: Some(config_base64),
                ..Default::default()
            }),
            ..Default::default()
        });
        let result = set_aws_config(&input, &temp_file_path);

        assert!(result.is_ok());
        assert!(env::var(AWS_CONFIG_FILE_ENV_VAR).is_ok());
        assert_eq!(
            env::var(AWS_CONFIG_FILE_ENV_VAR).unwrap(),
            temp_file_path.to_str().unwrap()
        );

        // Remove the env variable such that it's no longer set.
        env::remove_var(AWS_CONFIG_FILE_ENV_VAR);
    }

    fn test_set_aws_config_is_not_set() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_file_path = temp_dir.path().join("config.fake");

        let input = SettingsViewDelta::from_api_response(SettingsView {
            aws: Some(AwsSettingsV1 {
                ..Default::default()
            }),
            ..Default::default()
        });
        let result = set_aws_config(&input, &temp_file_path);
        assert!(result.is_ok());
        assert!(env::var(AWS_CONFIG_FILE_ENV_VAR).is_err()); // NotPresent
    }

    #[tokio::test]
    async fn test_hostname_override_source() {
        let server = Server::run();
        let base_uri = format!("http://{}", server.addr());
        println!("listen on {}", base_uri);
        let token = "some+token";
        let schema_version = "2021-07-15";
        let target = "meta-data/instance-id";
        let response_code = 200;
        let response_body = "i-123456789";
        server.expect(
            Expectation::matching(request::method_path("PUT", "/latest/api/token"))
                .times(1)
                .respond_with(
                    status_code(200)
                        .append_header("X-aws-ec2-metadata-token-ttl-seconds", "60")
                        .body(token),
                ),
        );
        server.expect(
            Expectation::matching(request::method_path(
                "GET",
                format!("/{}/{}", schema_version, target),
            ))
            .times(1)
            .respond_with(
                status_code(response_code)
                    .append_header("X-aws-ec2-metadata-token", token)
                    .body(response_body),
            ),
        );

        let mut imds_client = ImdsClient::new_impl(base_uri);

        let mut info = SettingsViewDelta::from_api_response(SettingsView {
            aws: Some(AwsSettingsV1 {
                region: Some("us-west-2".try_into().unwrap()),
                ..Default::default()
            }),
            ..Default::default()
        });

        // specifying a hostname will cause it to be used
        settings_view_set!(
            info.kubernetes.hostname_override =
                String::from("hostname-specified").try_into().unwrap()
        );
        generate_node_name(&mut imds_client, &mut info)
            .await
            .unwrap();
        assert_eq!(
            settings_view_get!(info.kubernetes.hostname_override).map(ToString::to_string),
            Some(String::from("hostname-specified"))
        );

        // regardless of the hostname override source
        settings_view_set!(
            info.kubernetes.hostname_override =
                String::from("hostname-specified").try_into().unwrap()
        );
        settings_view_set!(
            info.kubernetes.hostname_override_source = KubernetesHostnameOverrideSource::InstanceID
        );
        generate_node_name(&mut imds_client, &mut info)
            .await
            .unwrap();
        assert_eq!(
            settings_view_get!(info.kubernetes.hostname_override).map(ToString::to_string),
            Some(String::from("hostname-specified"))
        );

        settings_view_set!(
            info.kubernetes.hostname_override =
                String::from("hostname-specified").try_into().unwrap()
        );
        settings_view_set!(
            info.kubernetes.hostname_override_source =
                KubernetesHostnameOverrideSource::PrivateDNSName
        );
        generate_node_name(&mut imds_client, &mut info)
            .await
            .unwrap();
        assert_eq!(
            settings_view_get!(info.kubernetes.hostname_override).map(ToString::to_string),
            Some(String::from("hostname-specified"))
        );

        // no override provided if neither value is set
        let mut info = SettingsViewDelta::from_api_response(SettingsView {
            aws: Some(AwsSettingsV1 {
                region: Some("us-west-2".try_into().unwrap()),
                ..Default::default()
            }),
            ..Default::default()
        });

        assert!(settings_view_get!(info.kubernetes.hostname_override).is_none());
        assert!(settings_view_get!(info.kubernetes.hostname_override_source).is_none());
        generate_node_name(&mut imds_client, &mut info)
            .await
            .unwrap();
        assert_eq!(settings_view_get!(info.kubernetes.hostname_override), None);

        // skipping tests that call use the private dns name since we would need to make the EC2
        // API mockable to implement them

        // specifying no hostname, with override of instance-id causes the instance-id to be used
        // and pulled from IMDS
        let mut info = SettingsViewDelta::from_api_response(SettingsView {
            aws: Some(AwsSettingsV1 {
                region: Some("us-west-2".try_into().unwrap()),
                ..Default::default()
            }),
            ..Default::default()
        });

        assert!(settings_view_get!(info.kubernetes.hostname_override).is_none());
        settings_view_set!(
            info.kubernetes.hostname_override_source = KubernetesHostnameOverrideSource::InstanceID
        );
        generate_node_name(&mut imds_client, &mut info)
            .await
            .unwrap();
        assert_eq!(
            settings_view_get!(info.kubernetes.hostname_override).map(ToString::to_string),
            Some(String::from("i-123456789"))
        );
    }
}
