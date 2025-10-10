#[macro_use]
extern crate log;

use snafu::ResultExt;
use std::{collections::HashMap};

const API_PENDING_URI_BASE: &str = "/v2/tx";
const API_COMMIT_URI_BASE: &str = "/tx/commit";

pub mod error {
    use http::StatusCode;
    use snafu::Snafu;

    /// Potential errors during user data management.
    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub enum SettingsCommitterError {
        #[snafu(display("Error sending {} to {}: {}", method, uri, source))]
        APIRequest {
            method: String,
            uri: String,
            #[snafu(source(from(apiclient::Error, Box::new)))]
            source: Box<apiclient::Error>,
        },

        #[snafu(display("Error {} when sending {} to {}: {}", code, method, uri, response_body))]
        APIResponse {
            method: String,
            uri: String,
            code: StatusCode,
            response_body: String,
        },
    }
}
pub use error::SettingsCommitterError;
pub type Result<T> = std::result::Result<T, error::SettingsCommitterError>;

/// Checks pending settings and logs them. We don't want to prevent a
/// commit if there's a blip in retrieval or parsing of the pending
/// settings.  We know the system won't be functional without a commit,
/// but we can live without logging what was committed.
async fn check_pending_settings<S: AsRef<str>>(socket_path: S, transaction: &str) {
    let uri = format!("{API_PENDING_URI_BASE}?tx={transaction}");

    debug!("GET-ing {uri} to determine if there are pending settings");
    let get_result = apiclient::raw_request(socket_path.as_ref(), &uri, "GET", None).await;
    let response_body = match get_result {
        Ok((code, response_body)) => {
            if !code.is_success() {
                warn!("Got {code} when sending GET to {uri}: {response_body}");
                return;
            }
            response_body
        }
        Err(err) => {
            warn!("Failed to GET pending settings from {uri}: {err}");
            return;
        }
    };

    let pending_result: serde_json::Result<HashMap<String, serde_json::Value>> =
        serde_json::from_str(&response_body);
    match pending_result {
        Ok(pending) => {
            debug!("Pending settings for tx {}: {:?}", transaction, &pending);
        }
        Err(err) => {
            warn!("Failed to parse response from {uri}: {err}");
        }
    }
}

/// Commits pending settings to live.
async fn commit_pending_settings<S: AsRef<str>>(socket_path: S, transaction: &str) -> Result<()> {
    let uri = format!("{API_COMMIT_URI_BASE}?tx={transaction}");
    debug!("POST-ing to {uri} to move pending settings to live");

    if let Err(e) = apiclient::raw_request(socket_path.as_ref(), &uri, "POST", None).await {
        match e {
            // Some types of response errors are OK for this use.
            apiclient::Error::ResponseStatus { code, body, .. } => {
                if code.as_u16() == 422 {
                    info!("settings-committer found no settings changes to commit");
                    return Ok(());
                } else {
                    return error::APIResponseSnafu {
                        method: "POST",
                        uri,
                        code,
                        response_body: body,
                    }
                    .fail();
                }
            }
            // Any other type of error means we couldn't even make the request.
            _ => {
                return Err(e).context(error::APIRequestSnafu {
                    method: "POST",
                    uri,
                });
            }
        }
    }
    Ok(())
}

pub async fn commit(socket_path: &str, transaction: &str) -> Result<()> {
    if log_enabled!(log::Level::Debug) {
        // We log the pending settings at Debug, so only fetch them if they won't be filtered.
        info!("Checking pending settings.");
        check_pending_settings(socket_path, transaction).await;
    }

    info!("Committing settings.");
    commit_pending_settings(socket_path, transaction).await?;

    Ok(())
}
