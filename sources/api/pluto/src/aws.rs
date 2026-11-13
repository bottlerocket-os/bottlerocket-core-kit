use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_config::provider_config::ProviderConfig;
use aws_config::{imds, BehaviorVersion};
use aws_smithy_http_client::{
    proxy::{ProxyConfig, ProxyError},
    tls::{self, rustls_provider::CryptoMode},
    Builder as HttpClientBuilder, Connector,
};
use aws_smithy_runtime_api::box_error::BoxError;
use aws_smithy_runtime_api::client::http::SharedHttpClient;
use aws_smithy_runtime_api::client::interceptors::context::FinalizerInterceptorContextRef;
use aws_smithy_runtime_api::client::interceptors::Intercept;
use aws_smithy_runtime_api::client::orchestrator::Metadata;
use aws_smithy_runtime_api::client::retries::RequestAttempts;
use aws_smithy_runtime_api::client::runtime_components::RuntimeComponents;
use aws_smithy_types::config_bag::ConfigBag;
use aws_smithy_types::error::display::DisplayErrorContext;
use aws_smithy_types::retry::{RetryConfig, RetryConfigBuilder};
use aws_types::region::Region;
use aws_types::SdkConfig;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

// Max request retry attempts; Retry many many times and let the caller decide when to terminate
const MAX_ATTEMPTS: u32 = 100;
const IMDS_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

fn sdk_imds_client(provider_config: &ProviderConfig) -> imds::Client {
    imds::Client::builder()
        .configure(provider_config)
        .max_attempts(MAX_ATTEMPTS)
        .connect_timeout(IMDS_CONNECT_TIMEOUT)
        .build()
}

fn sdk_retry_config() -> RetryConfig {
    RetryConfigBuilder::new().max_attempts(MAX_ATTEMPTS).build()
}

pub(crate) struct SdkHttpClients {
    pub(crate) service: SharedHttpClient,
    pub(crate) credentials: SharedHttpClient,
}

/// Prefer settings for credentials while retaining the SDK environment fallback.
pub(crate) fn sdk_http_clients<H, N>(
    https_proxy: Option<H>,
    no_proxy: Option<&[N]>,
) -> Result<SdkHttpClients, ProxyError>
where
    H: AsRef<str>,
    N: AsRef<str>,
{
    let crypto_mode = crate::crypto_mode();
    if let Some(https_proxy) = https_proxy {
        let mut proxy = ProxyConfig::https(https_proxy.as_ref())?;
        if let Some(no_proxy) = no_proxy {
            let exclusions: Vec<&str> = no_proxy.iter().map(AsRef::as_ref).collect();
            proxy = proxy.no_proxy(exclusions.join(","));
        }
        let service = sdk_http_client(crypto_mode, Some(proxy));
        return Ok(SdkHttpClients {
            credentials: service.clone(),
            service,
        });
    }
    Ok(SdkHttpClients {
        service: sdk_http_client(crypto_mode.clone(), None),
        credentials: sdk_http_client(crypto_mode, Some(ProxyConfig::from_env())),
    })
}

fn sdk_http_client(crypto_mode: CryptoMode, proxy: Option<ProxyConfig>) -> SharedHttpClient {
    let Some(proxy) = proxy else {
        return HttpClientBuilder::new()
            .tls_provider(tls::Provider::Rustls(crypto_mode))
            .build_https();
    };
    HttpClientBuilder::new().build_with_connector_fn(move |settings, runtime_components| {
        let mut connector = Connector::builder()
            .proxy_config(proxy.clone())
            .tls_provider(tls::Provider::Rustls(crypto_mode.clone()));
        connector.set_connector_settings(settings.cloned());
        if let Some(components) = runtime_components {
            connector.set_sleep_impl(components.sleep_impl());
        }
        connector.build()
    })
}

pub(crate) async fn sdk_config(region: &str, http_client: SharedHttpClient) -> SdkConfig {
    let provider_config = ProviderConfig::default().with_http_client(http_client.clone());
    let provider = DefaultCredentialsChain::builder()
        .imds_client(sdk_imds_client(&provider_config))
        .configure(provider_config)
        .build()
        .await;
    aws_config::defaults(BehaviorVersion::v2026_01_12())
        .http_client(http_client)
        .region(Region::new(region.to_owned()))
        .credentials_provider(provider)
        .retry_config(sdk_retry_config())
        .load()
        .await
}

/// Logs each failed attempt of an AWS SDK request.
///
/// The SDK retries failed requests itself, so pluto only sees the error of the last attempt.
/// This interceptor runs after every attempt, before the SDK decides whether to retry, and logs
/// the error of each failed attempt.
#[derive(Clone)]
pub(crate) struct LogFailedAttempts {
    sink: Arc<dyn Fn(&str) + Send + Sync>,
}

impl LogFailedAttempts {
    pub(crate) fn new() -> Self {
        Self::with_sink(|message| log::warn!("{}", message))
    }

    /// Sends each message to `sink` instead of the logger.
    fn with_sink(sink: impl Fn(&str) + Send + Sync + 'static) -> Self {
        Self {
            sink: Arc::new(sink),
        }
    }
}

impl fmt::Debug for LogFailedAttempts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogFailedAttempts").finish_non_exhaustive()
    }
}

impl Intercept for LogFailedAttempts {
    fn name(&self) -> &'static str {
        "LogFailedAttempts"
    }

    fn read_after_attempt(
        &self,
        context: &FinalizerInterceptorContextRef<'_>,
        _runtime_components: &RuntimeComponents,
        cfg: &mut ConfigBag,
    ) -> Result<(), BoxError> {
        let Some(Err(err)) = context.output_or_error() else {
            return Ok(());
        };
        let operation = cfg.load::<Metadata>().map_or_else(
            || "AWS request".to_string(),
            |metadata| format!("{} {}", metadata.service(), metadata.name()),
        );
        let attempt = cfg
            .load::<RequestAttempts>()
            .map_or_else(|| "?".to_string(), |a| a.attempts().to_string());
        let max_attempts = cfg
            .load::<RetryConfig>()
            .map_or_else(|| "?".to_string(), |c| c.max_attempts().to_string());
        let mut message = format!(
            "{} attempt {}/{} failed: {}",
            operation,
            attempt,
            max_attempts,
            DisplayErrorContext(err)
        );
        if let Some(response) = context.response() {
            message.push_str(&format!(" (HTTP status {}", response.status().as_u16()));
            if let Some(request_id) = response.headers().get("x-amzn-requestid") {
                message.push_str(&format!(", request ID {}", request_id));
            }
            message.push(')');
        }
        (self.sink)(&message);
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod test_util {
    use super::LogFailedAttempts;
    use aws_config::BehaviorVersion;
    use aws_sdk_ec2::config::{Credentials, SharedCredentialsProvider};
    use aws_smithy_types::retry::RetryConfig;
    use aws_types::region::Region;
    use aws_types::SdkConfig;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// Max attempts used by the test clients.
    pub(crate) const TEST_MAX_ATTEMPTS: u32 = 3;

    /// Returns an SDK config that sends requests to `endpoint_url` with static test credentials
    /// and short retry backoffs.
    pub(crate) fn test_sdk_config(endpoint_url: &str) -> SdkConfig {
        SdkConfig::builder()
            .behavior_version(BehaviorVersion::v2026_01_12())
            .region(Region::new("us-west-2"))
            .endpoint_url(endpoint_url)
            .credentials_provider(SharedCredentialsProvider::new(Credentials::new(
                "AKIDTEST",
                "SECRETTEST",
                None,
                None,
                "pluto-test",
            )))
            .retry_config(
                RetryConfig::standard()
                    .with_max_attempts(TEST_MAX_ATTEMPTS)
                    .with_initial_backoff(Duration::from_millis(1))
                    .with_max_backoff(Duration::from_millis(10)),
            )
            .build()
    }

    /// Returns an interceptor that records its messages, and the recorded messages.
    pub(crate) fn recording_interceptor() -> (LogFailedAttempts, Arc<Mutex<Vec<String>>>) {
        let messages = Arc::new(Mutex::new(Vec::new()));
        let sink = messages.clone();
        let interceptor = LogFailedAttempts::with_sink(move |message| {
            sink.lock().unwrap().push(message.to_string());
        });
        (interceptor, messages)
    }

    /// Returns a local address that refuses connections.
    pub(crate) fn refused_endpoint() -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        format!("http://{}", addr)
    }
}
