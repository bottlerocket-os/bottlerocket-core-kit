use crate::aws::sdk_config;
use crate::proxy;
use aws_sdk_eks::types::KubernetesNetworkConfigResponse;
#[cfg(feature = "fips")]
use aws_smithy_experimental::hyper_1_0::{CryptoMode, HyperClientBuilder};
#[cfg(not(feature = "fips"))]
use aws_smithy_runtime::client::http::hyper_014::HyperClientBuilder;
use snafu::{OptionExt, ResultExt, Snafu};
use std::time::Duration;

// Limit the timeout for the EKS describe cluster API call to 5 minutes
const EKS_DESCRIBE_CLUSTER_TIMEOUT: Duration = Duration::from_secs(300);

pub(crate) type ClusterNetworkConfig = KubernetesNetworkConfigResponse;

#[derive(Debug, Snafu)]
pub(super) enum Error {
    #[snafu(display("Error describing cluster: {}", source))]
    DescribeCluster {
        source: aws_sdk_eks::error::SdkError<
            aws_sdk_eks::operation::describe_cluster::DescribeClusterError,
        >,
    },

    #[snafu(display("Timed-out waiting for EKS Describe Cluster API response: {}", source))]
    DescribeClusterTimeout { source: tokio::time::error::Elapsed },

    #[snafu(display("Missing field '{}' in EKS response", field))]
    Missing { field: &'static str },

    #[snafu(context(false), display("{}", source))]
    Proxy { source: proxy::Error },
}

type Result<T> = std::result::Result<T, Error>;

/// Returns the cluster's [kubernetesNetworkConfig] by calling the EKS API.
/// (https://docs.aws.amazon.com/eks/latest/APIReference/API_KubernetesNetworkConfigResponse.html)
pub(super) async fn get_cluster_network_config<H, N>(
    region: &str,
    cluster: &str,
    https_proxy: Option<H>,
    no_proxy: Option<&[N]>,
) -> Result<ClusterNetworkConfig>
where
    H: AsRef<str>,
    N: AsRef<str>,
{
    let config = sdk_config(region).await;

    #[cfg(not(feature = "fips"))]
    let client = build_client(https_proxy, no_proxy, config)?;

    // FIXME!: support proxies in FIPS mode
    #[cfg(feature = "fips")]
    let client = build_client(config)?;

    tokio::time::timeout(
        EKS_DESCRIBE_CLUSTER_TIMEOUT,
        client.describe_cluster().name(cluster.to_owned()).send(),
    )
    .await
    .context(DescribeClusterTimeoutSnafu)?
    .context(DescribeClusterSnafu)?
    .cluster
    .context(MissingSnafu { field: "cluster" })?
    .kubernetes_network_config
    .context(MissingSnafu {
        field: "kubernetes_network_config",
    })
}

#[cfg(not(feature = "fips"))]
fn build_client<H, N>(
    https_proxy: Option<H>,
    no_proxy: Option<&[N]>,
    config: aws_config::SdkConfig,
) -> Result<aws_sdk_eks::Client>
where
    H: AsRef<str>,
    N: AsRef<str>,
{
    let client = if let Some(https_proxy) = https_proxy {
        let http_connector = proxy::setup_http_client(https_proxy, no_proxy)?;
        let http_client = HyperClientBuilder::new().build(http_connector);
        let eks_config = aws_sdk_eks::config::Builder::from(&config)
            .http_client(http_client)
            .build();
        aws_sdk_eks::Client::from_conf(eks_config)
    } else {
        aws_sdk_eks::Client::new(&config)
    };

    Ok(client)
}

// FIXME!: support proxies in FIPS mode
#[cfg(feature = "fips")]
fn build_client(config: aws_config::SdkConfig) -> Result<aws_sdk_eks::Client> {
    let http_client = HyperClientBuilder::new()
        .crypto_mode(CryptoMode::AwsLcFips)
        .build_https();
    let eks_config = aws_sdk_eks::config::Builder::from(&config)
        .http_client(http_client)
        .build();

    Ok(aws_sdk_eks::Client::from_conf(eks_config))
}
