use crate::aws::sdk_config;
use crate::proxy;
#[cfg(feature = "fips")]
use aws_smithy_experimental::hyper_1_0::{CryptoMode, HyperClientBuilder};
#[cfg(not(feature = "fips"))]
use aws_smithy_runtime::client::http::hyper_014::HyperClientBuilder;
use aws_smithy_types::error::display::DisplayErrorContext;
use snafu::{OptionExt, ResultExt, Snafu};
use std::time::Duration;
use tokio_retry::{
    strategy::{jitter, FibonacciBackoff},
    Retry,
};

// Limit the timeout for fetching the private DNS name of the EC2 instance to 5 minutes.
const FETCH_PRIVATE_DNS_NAME_TIMEOUT: Duration = Duration::from_secs(300);
// Fibonacci backoff base duration when retrying requests
const FIBONACCI_BACKOFF_BASE_DURATION_MILLIS: u64 = 200;

#[derive(Debug, Snafu)]
pub(super) enum Error {
    #[snafu(display(
        "Error describing instance '{}': {}",
        instance_id,
        DisplayErrorContext(source)
    ))]
    DescribeInstances {
        instance_id: String,
        source: aws_sdk_eks::error::SdkError<
            aws_sdk_ec2::operation::describe_instances::DescribeInstancesError,
        >,
    },

    #[snafu(display("Timed out retrieving private DNS name from EC2: {}", source))]
    FetchPrivateDnsNameTimeout { source: tokio::time::error::Elapsed },

    #[snafu(display("Missing field '{}' in EC2 response", field))]
    Missing { field: &'static str },

    #[snafu(context(false), display("{}", source))]
    Proxy { source: proxy::Error },
}

type Result<T> = std::result::Result<T, Error>;

pub(super) async fn get_private_dns_name<H, N>(
    region: &str,
    instance_id: &str,
    https_proxy: Option<H>,
    no_proxy: Option<&[N]>,
) -> Result<String>
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
        FETCH_PRIVATE_DNS_NAME_TIMEOUT,
        Retry::spawn(
            FibonacciBackoff::from_millis(FIBONACCI_BACKOFF_BASE_DURATION_MILLIS).map(jitter),
            || async {
                client
                    .describe_instances()
                    .instance_ids(instance_id.to_owned())
                    .send()
                    .await
                    .context(DescribeInstancesSnafu { instance_id })?
                    .reservations
                    .and_then(|reservations| {
                        reservations.first().and_then(|r| {
                            r.instances.clone().and_then(|instances| {
                                instances
                                    .first()
                                    .and_then(|i| i.private_dns_name().map(|s| s.to_string()))
                            })
                        })
                    })
                    .filter(|private_dns_name| !private_dns_name.is_empty())
                    .context(MissingSnafu {
                        field: "Reservation.Instance.PrivateDNSName",
                    })
            },
        ),
    )
    .await
    .context(FetchPrivateDnsNameTimeoutSnafu)?
}

#[cfg(not(feature = "fips"))]
fn build_client<H, N>(
    https_proxy: Option<H>,
    no_proxy: Option<&[N]>,
    config: aws_config::SdkConfig,
) -> Result<aws_sdk_ec2::Client>
where
    H: AsRef<str>,
    N: AsRef<str>,
{
    let client = if let Some(https_proxy) = https_proxy {
        let http_connector = proxy::setup_http_client(https_proxy, no_proxy)?;
        let http_client = HyperClientBuilder::new().build(http_connector);
        let ec2_config = aws_sdk_ec2::config::Builder::from(&config)
            .http_client(http_client)
            .build();
        aws_sdk_ec2::Client::from_conf(ec2_config)
    } else {
        aws_sdk_ec2::Client::new(&config)
    };

    Ok(client)
}

// FIXME!: support proxies in FIPS mode
#[cfg(feature = "fips")]
fn build_client(config: aws_config::SdkConfig) -> Result<aws_sdk_ec2::Client> {
    let http_client = HyperClientBuilder::new()
        .crypto_mode(CryptoMode::AwsLcFips)
        .build_https();
    let ec2_config = aws_sdk_ec2::config::Builder::from(&config)
        .http_client(http_client)
        .build();

    Ok(aws_sdk_ec2::Client::from_conf(ec2_config))
}
