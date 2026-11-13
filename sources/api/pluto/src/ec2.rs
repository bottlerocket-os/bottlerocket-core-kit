use crate::aws::{sdk_config, LogFailedAttempts};
use crate::PROVIDER;
use aws_smithy_http_client::{proxy::ProxyConfig, tls, Builder as HttpClientBuilder, Connector};
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
        #[snafu(source(from(aws_sdk_ec2::error::SdkError<aws_sdk_ec2::operation::describe_instances::DescribeInstancesError>, Box::new)))]
        source: Box<
            aws_sdk_ec2::error::SdkError<
                aws_sdk_ec2::operation::describe_instances::DescribeInstancesError,
            >,
        >,
    },

    #[snafu(display("Timed out retrieving private DNS name from EC2: {}", source))]
    FetchPrivateDnsNameTimeout { source: tokio::time::error::Elapsed },

    #[snafu(display("Missing field '{}' in EC2 response", field))]
    Missing { field: &'static str },

    #[snafu(display("Invalid proxy URL: {}", source))]
    ProxyConfig {
        source: aws_smithy_http_client::proxy::ProxyError,
    },
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

    let client = build_client(https_proxy, no_proxy, config)?;

    tokio::time::timeout(
        FETCH_PRIVATE_DNS_NAME_TIMEOUT,
        Retry::start(
            FibonacciBackoff::from_millis(FIBONACCI_BACKOFF_BASE_DURATION_MILLIS).map(jitter),
            || async {
                log::info!("EC2 DescribeInstances attempt for {instance_id}");
                let response = client
                    .describe_instances()
                    .instance_ids(instance_id.to_owned())
                    .send()
                    .await
                    .context(DescribeInstancesSnafu { instance_id });
                if let Err(Error::DescribeInstances { source, .. }) = &response {
                    log::error!(
                        "EC2 DescribeInstances failed: {}",
                        DisplayErrorContext(source)
                    );
                }
                response?
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
                    .inspect_err(|e| {
                        log::error!(
                            "EC2 DescribeInstances attempt parsed to missing field, will retry: {}",
                            e
                        );
                    })
            },
        ),
    )
    .await
    .context(FetchPrivateDnsNameTimeoutSnafu)?
}

fn build_client<H, N>(
    https_proxy: Option<H>,
    no_proxy: Option<&[N]>,
    config: aws_config::SdkConfig,
) -> Result<aws_sdk_ec2::Client>
where
    H: AsRef<str>,
    N: AsRef<str>,
{
    let http_client = if let Some(https_proxy) = https_proxy {
        let https_proxy = https_proxy.as_ref().to_string();
        let mut proxy = ProxyConfig::https(&https_proxy).context(ProxyConfigSnafu)?;
        if let Some(no_proxy) = no_proxy {
            let no_proxy_str: Vec<&str> = no_proxy.iter().map(|s| s.as_ref()).collect();
            proxy = proxy.no_proxy(no_proxy_str.join(","));
        }
        HttpClientBuilder::new().build_with_connector_fn(move |settings, _runtime_components| {
            let mut builder = Connector::builder()
                .proxy_config(proxy.clone())
                .tls_provider(tls::Provider::Rustls(PROVIDER.clone()));
            builder.set_connector_settings(settings.cloned());
            builder.build()
        })
    } else {
        HttpClientBuilder::new()
            .tls_provider(tls::Provider::Rustls(PROVIDER.clone()))
            .build_https()
    };
    let ec2_config = aws_sdk_ec2::config::Builder::from(&config)
        .http_client(http_client)
        .interceptor(LogFailedAttempts::new())
        .build();

    Ok(aws_sdk_ec2::Client::from_conf(ec2_config))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::aws::test_util::{
        recording_interceptor, refused_endpoint, test_sdk_config, TEST_MAX_ATTEMPTS,
    };
    use aws_smithy_runtime_api::client::interceptors::Intercept;
    use httptest::{matchers::*, responders::*, Expectation, Server};

    const INSTANCE_ID: &str = "i-0123456789abcdef0";

    fn test_client(endpoint_url: &str) -> aws_sdk_ec2::Client {
        build_client::<&str, &str>(None, None, test_sdk_config(endpoint_url)).unwrap()
    }

    #[tokio::test]
    async fn build_client_adds_log_failed_attempts() {
        let client = test_client("http://127.0.0.1:1");
        assert!(client
            .config()
            .interceptors()
            .any(|i| i.name() == "LogFailedAttempts"));
    }

    #[tokio::test]
    async fn logs_each_refused_connection() {
        let (interceptor, messages) = recording_interceptor();
        let result = test_client(&refused_endpoint())
            .describe_instances()
            .instance_ids(INSTANCE_ID)
            .customize()
            .interceptor(interceptor)
            .send()
            .await;
        assert!(result.is_err());

        let messages = messages.lock().unwrap();
        assert_eq!(messages.len(), TEST_MAX_ATTEMPTS as usize, "{messages:?}");
        for (i, message) in messages.iter().enumerate() {
            let prefix = format!(
                "EC2 DescribeInstances attempt {}/{} failed: ",
                i + 1,
                TEST_MAX_ATTEMPTS
            );
            assert!(message.starts_with(&prefix), "{message}");
            assert!(message.contains("Connection refused"), "{message}");
            assert!(!message.contains("HTTP status"), "{message}");
        }
    }

    #[tokio::test]
    async fn logs_403_once_without_retry() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path("POST", "/"))
                .times(1)
                .respond_with(
                    status_code(403)
                        .append_header("x-amzn-requestid", "req-403")
                        .body(
                            r#"<?xml version="1.0" encoding="UTF-8"?>
<Response><Errors><Error><Code>UnauthorizedOperation</Code><Message>You are not authorized to perform this operation.</Message></Error></Errors><RequestID>req-403</RequestID></Response>"#,
                        ),
                ),
        );
        let (interceptor, messages) = recording_interceptor();
        let result = test_client(&format!("http://{}", server.addr()))
            .describe_instances()
            .instance_ids(INSTANCE_ID)
            .customize()
            .interceptor(interceptor)
            .send()
            .await;
        assert!(result.is_err());

        let messages = messages.lock().unwrap();
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert!(
            messages[0].starts_with("EC2 DescribeInstances attempt 1/3 failed: "),
            "{}",
            messages[0]
        );
        assert!(
            messages[0].contains("UnauthorizedOperation"),
            "{}",
            messages[0]
        );
        assert!(
            messages[0].ends_with("(HTTP status 403, request ID req-403)"),
            "{}",
            messages[0]
        );
    }
}
