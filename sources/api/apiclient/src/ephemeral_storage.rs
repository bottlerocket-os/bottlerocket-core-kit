use model::ephemeral_storage::{Bind, Filesystem, Init};
use snafu::ResultExt;
use std::path::Path;

/// Requests ephemeral storage initialization through the API
pub async fn initialize<P>(
    socket_path: P,
    filesystem: Option<Filesystem>,
    disks: Option<Vec<String>>,
) -> Result<()>
where
    P: AsRef<Path>,
{
    let uri = "/actions/ephemeral-storage/init";
    let opts =
        serde_json::to_string(&Init { filesystem, disks }).context(error::JsonSerializeSnafu {})?;
    let method = "POST";
    let (_status, _body) = crate::raw_request(&socket_path, &uri, method, Some(opts))
        .await
        .context(error::RequestSnafu { uri, method })?;
    Ok(())
}

/// Requests binding of directories to configured ephemeral storage through the API
pub async fn bind<P>(socket_path: P, targets: Vec<String>) -> Result<()>
where
    P: AsRef<Path>,
{
    let uri = "/actions/ephemeral-storage/bind";
    let opts = serde_json::to_string(&Bind { targets }).context(error::JsonSerializeSnafu {})?;
    let method = "POST";
    let (_status, _body) = crate::raw_request(&socket_path, &uri, method, Some(opts))
        .await
        .context(error::RequestSnafu { uri, method })?;
    Ok(())
}

/// Lists the ephemeral disks available for configuration
pub async fn list_disks<P>(socket_path: P, format: Option<String>) -> Result<String>
where
    P: AsRef<Path>,
{
    list(socket_path, "list-disks", format).await
}

/// Lists the ephemeral disks available for configuration
pub async fn list_dirs<P>(socket_path: P, format: Option<String>) -> Result<String>
where
    P: AsRef<Path>,
{
    list(socket_path, "list-dirs", format).await
}
async fn list<P>(socket_path: P, item: &str, format: Option<String>) -> Result<String>
where
    P: AsRef<Path>,
{
    let mut query = Vec::new();
    if let Some(query_format) = format {
        query.push(format!("format={}", query_format));
    }

    let uri = format!("/actions/ephemeral-storage/{}?{}", item, query.join("&"));
    let method = "GET";
    let (_status, body) = crate::raw_request(&socket_path, &uri, method, None)
        .await
        .context(error::RequestSnafu { uri, method })?;
    Ok(body)
}
mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub enum Error {
        #[snafu(display("Failed {} request to '{}': {}", method, uri, source))]
        Request {
            method: String,
            uri: String,
            #[snafu(source(from(crate::Error, Box::new)))]
            source: Box<crate::Error>,
        },
        #[snafu(display("Failed to serialize parameters"))]
        JsonSerialize { source: serde_json::Error },
    }
}
pub use error::Error;
pub type Result<T> = std::result::Result<T, error::Error>;
