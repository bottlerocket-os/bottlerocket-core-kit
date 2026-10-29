use crate::aws::{sdk_config, sdk_http_clients};
use aws_sdk_eks::types::KubernetesNetworkConfigResponse;
use aws_smithy_types::error::display::DisplayErrorContext;
use aws_smithy_types::error::metadata::ProvideErrorMetadata;
use snafu::{OptionExt, ResultExt, Snafu};
use std::time::Duration;

// Limit the timeout for the EKS describe cluster API call to 5 minutes
const EKS_DESCRIBE_CLUSTER_TIMEOUT: Duration = Duration::from_secs(300);

pub(crate) type ClusterNetworkConfig = KubernetesNetworkConfigResponse;

#[derive(Debug, Snafu)]
pub(super) enum Error {
    #[snafu(display(
        "Error describing EKS cluster '{}': {}",
        cluster,
        DisplayErrorContext(source)
    ))]
    DescribeCluster {
        cluster: String,
        #[snafu(source(from(aws_sdk_eks::error::SdkError<aws_sdk_eks::operation::describe_cluster::DescribeClusterError>, Box::new)))]
        source: Box<
            aws_sdk_eks::error::SdkError<
                aws_sdk_eks::operation::describe_cluster::DescribeClusterError,
            >,
        >,
    },

    #[snafu(display("Timed-out waiting for EKS Describe Cluster API response: {}", source))]
    DescribeClusterTimeout { source: tokio::time::error::Elapsed },

    #[snafu(display("Missing field '{}' in EKS response", field))]
    Missing { field: &'static str },

    #[snafu(display("Invalid proxy URL: {}", source))]
    ProxyConfig {
        source: aws_smithy_http_client::proxy::ProxyError,
    },
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
    let clients = sdk_http_clients(https_proxy, no_proxy).context(ProxyConfigSnafu)?;
    let config = sdk_config(region, clients.credentials).await;
    let service_config = aws_sdk_eks::config::Builder::from(&config)
        .http_client(clients.service)
        .build();
    let client = aws_sdk_eks::Client::from_conf(service_config);

    tokio::time::timeout(EKS_DESCRIBE_CLUSTER_TIMEOUT, async {
        log::info!("EKS DescribeCluster for {}", cluster);
        let response = client
            .describe_cluster()
            .name(cluster.to_owned())
            .send()
            .await
            .context(DescribeClusterSnafu { cluster });
        if let Err(Error::DescribeCluster { source, .. }) = &response {
            log::error!(
                "EKS DescribeCluster attempt failed: code={} message={}",
                source.code().unwrap_or_default(),
                source.message().unwrap_or_default(),
            );
        }
        response?
            .cluster
            .context(MissingSnafu { field: "cluster" })?
            .kubernetes_network_config
            .context(MissingSnafu {
                field: "kubernetes_network_config",
            })
            .inspect_err(|e| {
                log::error!("EKS DescribeCluster response missing expected field: {}", e);
            })
    })
    .await
    .context(DescribeClusterTimeoutSnafu)?
}
