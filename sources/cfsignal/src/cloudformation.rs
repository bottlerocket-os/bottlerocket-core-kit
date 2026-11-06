use crate::error::{self, Result};
use aws_config::BehaviorVersion;
use aws_smithy_http_client::tls::rustls_provider::CryptoMode;
use aws_smithy_http_client::{proxy::ProxyConfig, tls, Builder as HttpClientBuilder, Connector};
use aws_types::region::Region;
use imdsclient::ImdsClient;
use log::info;
use snafu::{OptionExt, ResultExt};
use std::env;
use std::str::FromStr;

/// Signals Cloudformation stack resource
pub async fn signal_resource(
    stack_name: String,
    logical_resource_id: String,
    status: String,
) -> Result<()> {
    info!("Connecting to IMDS");
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let mut client = ImdsClient::new();
    let instance_id = get_instance_id(&mut client).await?;
    let region = get_region(&mut client).await?;

    info!("Region: {region:?} - InstanceID: {instance_id:?} - Signal: {status:?}");
    let config = aws_config::defaults(BehaviorVersion::v2026_01_12())
        .region(Region::new(region.to_owned()))
        .load()
        .await;

    #[cfg(feature = "fips")]
    let crypto_mode = CryptoMode::AwsLcFips;
    #[cfg(not(feature = "fips"))]
    let crypto_mode = CryptoMode::AwsLc;

    let https_proxy: Option<String> = match env::var_os("HTTPS_PROXY") {
        Some(https_proxy) => https_proxy.to_str().map(|h| h.to_string()),
        _ => None,
    };

    let no_proxy: Option<Vec<String>> = match env::var_os("NO_PROXY") {
        Some(no_proxy) => no_proxy
            .to_str()
            .map(|n| n.split(',').map(|s| s.to_string()).collect()),
        _ => None,
    };

    let http_client = if let Some(https_proxy) = https_proxy {
        let mut proxy = ProxyConfig::https(&https_proxy).context(error::ProxyConfigSnafu)?;
        if let Some(ref no_proxy) = no_proxy {
            proxy = proxy.no_proxy(no_proxy.join(","));
        }
        HttpClientBuilder::new().build_with_connector_fn(move |settings, _runtime_components| {
            let mut builder = Connector::builder()
                .proxy_config(proxy.clone())
                .tls_provider(tls::Provider::Rustls(crypto_mode.clone()));
            builder.set_connector_settings(settings.cloned());
            builder.build()
        })
    } else {
        HttpClientBuilder::new()
            .tls_provider(tls::Provider::Rustls(crypto_mode))
            .build_https()
    };

    let cloudformation_config = aws_sdk_cloudformation::config::Builder::from(&config)
        .http_client(http_client)
        .build();

    let client = aws_sdk_cloudformation::Client::from_conf(cloudformation_config);

    client
        .signal_resource()
        .stack_name(stack_name)
        .logical_resource_id(logical_resource_id)
        .status(
            aws_sdk_cloudformation::types::ResourceSignalStatus::from_str(&status)
                .expect("infallible"),
        )
        .unique_id(instance_id)
        .send()
        .await
        .context(error::SignalResourceSnafu)?;

    Ok(())
}

/// Returns the instanceId
async fn get_instance_id(client: &mut ImdsClient) -> Result<String> {
    client
        .fetch_instance_id()
        .await
        .context(error::ImdsRequestSnafu)?
        .context(error::ImdsNoneSnafu {
            what: "instance-id",
        })
}

/// Returns the region
async fn get_region(client: &mut ImdsClient) -> Result<String> {
    client
        .fetch_region()
        .await
        .context(error::ImdsRequestSnafu)?
        .context(error::ImdsNoneSnafu { what: "region" })
}
