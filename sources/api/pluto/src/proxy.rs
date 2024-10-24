use headers::Authorization;
use hyper::Uri;
use hyper_http_proxy::{Proxy, ProxyConnector};
use hyper_rustls::{ConfigBuilderExt, HttpsConnector};
use hyper_util::client::legacy::connect::dns::GaiResolver;
use hyper_util::client::legacy::connect::HttpConnector;
use snafu::{ResultExt, Snafu};
use url::Url;

#[derive(Debug, Snafu)]
pub(super) enum Error {
    #[snafu(display("Unable to parse '{}' as URI: {}", input, source))]
    UriParse {
        input: String,
        source: hyper::http::uri::InvalidUri,
    },

    #[snafu(display("Unable to parse '{}' as URL: {}", input, source))]
    UrlParse {
        input: String,
        source: url::ParseError,
    },

    #[snafu(display("Failed to create proxy creator: {}", source))]
    ProxyConnector { source: std::io::Error },
}

type Result<T> = std::result::Result<T, Error>;

/// Setups a hyper-based HTTP client configured with a proxy connector.
pub(crate) fn setup_http_client<H, N>(
    https_proxy: H,
    no_proxy: Option<&[N]>,
) -> Result<ProxyConnector<HttpsConnector<HttpConnector>>>
where
    H: AsRef<str>,
    N: AsRef<str>,
{
    // Determines whether a request of a given scheme, host and port should be proxied
    // according to `https_proxy` and `no_proxy`.

    // The no-proxy intercept requires ownership of its input data.
    let no_proxy: Option<Vec<String>> =
        no_proxy.map(|n| n.iter().map(|s| s.as_ref().to_owned()).collect());
    let intercept = move |scheme: Option<&str>, host: Option<&str>, _port| {
        if let Some(host) = host {
            if let Some(no_proxy) = &no_proxy {
                if scheme != Some("https") {
                    return false;
                }
                if no_proxy.iter().any(|s| s == "*") {
                    // Don't proxy anything
                    return false;
                }
                // If the host matches one of the no proxy list entries, return false (don't proxy)
                // Note that we're not doing anything fancy here for checking `no_proxy` since
                // we only expect requests here to be going out to some AWS API endpoint.
                return !no_proxy.iter().any(|no_proxy_host| {
                    !no_proxy_host.is_empty() && host.ends_with(no_proxy_host)
                });
            }
            true
        } else {
            false
        }
    };

    let https_proxy = https_proxy.as_ref();
    let mut proxy_uri = https_proxy.parse::<Uri>().context(UriParseSnafu {
        input: https_proxy.to_owned(),
    })?;
    // If the proxy's URI doesn't have a scheme, assume HTTP for the scheme and let the proxy
    // server forward HTTPS connections and start a tunnel.
    if proxy_uri.scheme().is_none() {
        proxy_uri = format!("http://{}", https_proxy)
            .parse::<Uri>()
            .context(UriParseSnafu {
                input: https_proxy.to_owned(),
            })?;
    }
    let mut proxy = Proxy::new(intercept, proxy_uri);
    // Parse https_proxy as URL to extract out auth information if any
    let proxy_url = Url::parse(https_proxy).context(UrlParseSnafu {
        input: https_proxy.to_owned(),
    })?;

    if !proxy_url.username().is_empty() || proxy_url.password().is_some() {
        proxy.set_authorization(Authorization::basic(
            proxy_url.username(),
            proxy_url.password().unwrap_or_default(),
        ));
    }

    let mut base_connector = HttpConnector::new_with_resolver(GaiResolver::new());
    base_connector.enforce_http(false);
    let https_connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(
            rustls::ClientConfig::builder()
                .with_native_roots()
                .expect("error with TLS configuration.")
                .with_no_client_auth(),
        )
        .https_or_http()
        .enable_http1()
        .enable_http2()
        .wrap_connector(base_connector);
    let proxy_connector =
        ProxyConnector::from_proxy(https_connector, proxy).context(ProxyConnectorSnafu)?;
    Ok(proxy_connector)
}
