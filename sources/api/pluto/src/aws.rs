use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_config::provider_config::ProviderConfig;
use aws_config::{imds, BehaviorVersion};
use aws_smithy_http_client::{
    proxy::{ProxyConfig, ProxyError},
    tls::{self, rustls_provider::CryptoMode},
    Builder as HttpClientBuilder, Connector,
};
use aws_smithy_runtime_api::client::http::SharedHttpClient;
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
