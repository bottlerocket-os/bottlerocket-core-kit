use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_config::provider_config::ProviderConfig;
use aws_config::{imds, BehaviorVersion};
use aws_smithy_http_client::{
    proxy::{ProxyConfig, ProxyError},
    tls, Builder as HttpClientBuilder, Connector,
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

/// If the proxy has no scheme, assume http://. The proxy then opens a
/// tunnel for the HTTPS request.
pub(crate) fn proxy_url(proxy: &str) -> String {
    let proxy = proxy.trim();
    if proxy.contains("://") {
        proxy.to_string()
    } else {
        format!("http://{proxy}")
    }
}

/// Returns the HTTP client for the AWS SDK clients, with the HTTPS proxy and
/// its no_proxy list if set.
pub(crate) fn sdk_http_client<H, N>(
    https_proxy: Option<H>,
    no_proxy: Option<&[N]>,
) -> Result<SharedHttpClient, ProxyError>
where
    H: AsRef<str>,
    N: AsRef<str>,
{
    let http_client = if let Some(https_proxy) = https_proxy {
        let mut proxy = ProxyConfig::https(proxy_url(https_proxy.as_ref()))?;
        if let Some(no_proxy) = no_proxy {
            let no_proxy_str: Vec<&str> = no_proxy.iter().map(|s| s.as_ref()).collect();
            proxy = proxy.no_proxy(no_proxy_str.join(","));
        }
        proxy_http_client(proxy)
    } else {
        HttpClientBuilder::new()
            .tls_provider(tls::Provider::Rustls(crate::crypto_mode()))
            .build_https()
    };
    Ok(http_client)
}

/// Returns an HTTP client that sends requests through the given proxy.
fn proxy_http_client(proxy: ProxyConfig) -> SharedHttpClient {
    let crypto_mode = crate::crypto_mode();
    HttpClientBuilder::new().build_with_connector_fn(move |settings, runtime_components| {
        let mut builder = Connector::builder()
            .proxy_config(proxy.clone())
            .tls_provider(tls::Provider::Rustls(crypto_mode.clone()));
        builder.set_connector_settings(settings.cloned());
        if let Some(components) = runtime_components {
            builder.set_sleep_impl(components.sleep_impl());
        }
        builder.build()
    })
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
    let settings_proxy = https_proxy.is_some();
    let service = sdk_http_client(https_proxy, no_proxy)?;
    let credentials = if settings_proxy {
        service.clone()
    } else {
        proxy_http_client(ProxyConfig::from_env())
    };
    Ok(SdkHttpClients {
        service,
        credentials,
    })
}

/// Logs the error of each failed AWS SDK attempt. The SDK retries failed
/// requests itself and only returns the last error.
#[derive(Debug)]
pub(crate) struct LogFailedAttempts;

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
        if let Some(Err(err)) = context.output_or_error() {
            let (service, operation) = cfg
                .load::<Metadata>()
                .map_or(("AWS", "request"), |m| (m.service(), m.name()));
            let attempt = cfg.load::<RequestAttempts>().map_or(0, |a| a.attempts());
            log::warn!(
                "{} {} attempt {} failed: {}",
                service,
                operation,
                attempt,
                DisplayErrorContext(err)
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn proxy_without_scheme() {
        assert_eq!(proxy_url("1.2.3.4:8080"), "http://1.2.3.4:8080");
        sdk_http_client(Some("1.2.3.4:8080"), None::<&[&str]>).unwrap();
    }
}
