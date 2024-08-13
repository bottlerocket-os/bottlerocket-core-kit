use bottlerocket_settings_models::{AwsSettingsV1, KubernetesSettingsV1, NetworkSettingsV1};
use serde::Deserialize;
use snafu::{ensure, ResultExt, Snafu};
use std::ffi::OsStr;
use tokio::process::Command;

/// The result type for the [`api`] module.
pub(super) type Result<T> = std::result::Result<T, Error>;

/// A mutable view of API settings
///
/// `SettingsViewDelta` keeps track of all changes in a separate structure so that only the changed
/// set can be sent back as writes to the API server.
///
/// For convenience, `settings_view_get!` and `settings_view_set!` macros can be used to handle
/// the nested optional values present in the structure succinctly.
///
/// `settings_view_get!` also automatically attempts to read from the settings delta before falling
/// back to the readonly settings.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingsViewDelta {
    readonly: SettingsView,
    delta: SettingsView,
}

impl SettingsViewDelta {
    /// Constructs a `SettingsViewDelta` based on an initial read-only view of settings.
    pub fn from_api_response(readonly: SettingsView) -> Self {
        Self {
            readonly,
            delta: SettingsView::default(),
        }
    }

    /// Returns the initial read-only settings model view
    ///
    /// Users should prefer to interact with this struct via the [`settings_view_get!`] and
    /// [`settings_view_set!`] macros.
    pub fn initial(&self) -> &SettingsView {
        &self.readonly
    }

    /// Returns a mutable reference to the "delta" settings model view
    ///
    /// Users should prefer to interact with this struct via the [`settings_view_get!`] and
    /// [`settings_view_set!`] macros.
    pub fn write(&mut self) -> &mut SettingsView {
        &mut self.delta
    }

    /// Returns an immutable reference to the "delta" settings model view
    ///
    /// Users should prefer to interact with this struct via the [`settings_view_get!`] and
    /// [`settings_view_set!`] macros.
    pub fn delta(&self) -> &SettingsView {
        &self.delta
    }
}

/// Returns the optional value of a settings nested within `SettingsViewDelta`.
///
/// Will refer to the delta before falling back to the readonly settings.
///
/// ```
/// let settings = SettingsViewDelta::from_api_response(SettingsView {
///     aws: Some(AwsSettingsV1 {
///         region: Some("us-west-2"),
///         ..Default::default()
///     })
///     ..Default::default()
/// });
/// assert_eq!(settings_view_get!(settings.aws.region), Some("us-west-2"));
/// ```
macro_rules! settings_view_get {
    (impl $parent:ident.$field:ident) => {
        $parent.$field.as_ref()
    };
    (impl $parent:ident.$field:ident$(.$fields:ident)+) => {{
        settings_view_get!(impl $parent.$field).and_then(|p| settings_view_get!(impl p.$($fields)+))
    }};
    ($settings:ident.$field:ident$(.$fields:ident)*) => {{
        let reader = $settings.initial();
        let delta = $settings.delta();
        settings_view_get!(impl delta.$field$(.$fields)*)
            .or_else(|| settings_view_get!(impl reader.$field$(.$fields)*))
    }};
}

/// Writes an optional value to the delta in a `SettingsViewDelta`.
///
/// ```
/// let settings = SettingsViewDelta::from_api_response(SettingsView {
///     aws: Some(AwsSettingsV1 {
///         region: Some("us-west-2"),
///         ..Default::default()
///     })
///     ..Default::default()
/// });
/// settings_view_set!(settings.aws.region = "us-east-1");
/// assert_eq!(settings_view_get!(settings.aws.region), Some("us-east-1"));
/// ```
macro_rules! settings_view_set {
    (impl $parent:ident.$field:ident = $value:expr) => {
        $parent.$field = Some($value)
    };
    (impl $parent:ident.$field:ident$(.$fields:ident)+ = $value:expr) => {{
        let curr_val = $parent.$field.get_or_insert_with(Default::default);
        settings_view_set!(impl curr_val.$($fields)+ = $value);
    }};
    ($settings:ident.$field:ident$(.$fields:ident)* = $value:expr) => {{
        let writer = $settings.write();
        settings_view_set!(impl writer.$field$(.$fields)* = $value);
    }}
}
pub(crate) use {settings_view_get, settings_view_set};

#[derive(Debug, Deserialize, Default, PartialEq, Clone)]
pub struct SettingsView {
    pub aws: Option<AwsSettingsV1>,
    pub network: Option<NetworkSettingsV1>,
    pub kubernetes: Option<KubernetesSettingsV1>,
}

#[derive(Deserialize)]
struct APISettingsResponse {
    pub settings: SettingsView,
}

#[derive(Debug, Snafu)]
pub(crate) enum Error {
    #[snafu(display("Failed to call apiclient: {}", source))]
    CommandFailure { source: std::io::Error },
    #[snafu(display("apiclient execution failed: {}", reason))]
    ExecutionFailure { reason: String },
    #[snafu(display("Deserialization of configuration file failed: {}", source))]
    Deserialize {
        #[snafu(source(from(serde_json::Error, Box::new)))]
        source: Box<serde_json::Error>,
    },
}

pub(crate) async fn client_command<I, S>(args: I) -> Result<Vec<u8>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let result = Command::new("/usr/bin/apiclient")
        .args(args)
        .output()
        .await
        .context(CommandFailureSnafu)?;

    ensure!(
        result.status.success(),
        ExecutionFailureSnafu {
            reason: String::from_utf8_lossy(&result.stderr)
        }
    );

    Ok(result.stdout)
}

/// Gets the info that we need to know about the EKS cluster from the Bottlerocket API.
pub(crate) async fn get_aws_k8s_info() -> Result<SettingsView> {
    let view_str = client_command(&[
        "get",
        "settings.aws",
        "settings.network",
        "settings.kubernetes",
    ])
    .await?;

    let api_response: APISettingsResponse =
        serde_json::from_slice(view_str.as_slice()).context(DeserializeSnafu)?;
    Ok(api_response.settings)
}

#[cfg(test)]
mod test {
    use super::*;
    use bottlerocket_settings_models::{AwsSettingsV1, KubernetesSettingsV1};

    #[test]
    fn test_default_kubernetes_settings_empty() {
        // `SettingsViewDelta` relies on its components default implementations being empty.
        // If this test fails, `pluto` could submit incorrect settings changes.
        let kubernetes_defaults = serde_json::to_value(KubernetesSettingsV1::default()).unwrap();
        assert_eq!(kubernetes_defaults, serde_json::json!({}));
    }

    #[test]
    fn test_default_network_settings_empty() {
        // `SettingsViewDelta` relies on its components default implementations being empty.
        // If this test fails, `pluto` could submit incorrect settings changes.
        let network_defaults = serde_json::to_value(NetworkSettingsV1::default()).unwrap();
        assert_eq!(network_defaults, serde_json::json!({}));
    }

    #[test]
    fn test_default_aws_settings_empty() {
        // `SettingsViewDelta` relies on its components default implementations being empty.
        // If this test fails, `pluto` could submit incorrect settings changes.
        let aws_defaults = serde_json::to_value(AwsSettingsV1::default()).unwrap();
        assert_eq!(aws_defaults, serde_json::json!({}));
    }

    #[test]
    fn test_settings_view_set() {
        // When settings are written, the originals are preserved
        let readonly_settings = SettingsView {
            aws: Some(AwsSettingsV1 {
                region: Some("us-west-2".try_into().unwrap()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut settings = SettingsViewDelta::from_api_response(readonly_settings.clone());

        settings_view_set!(settings.aws.region = "us-east-1".try_into().unwrap());

        let expected = SettingsViewDelta {
            readonly: settings.readonly.clone(),
            delta: SettingsView {
                aws: Some(AwsSettingsV1 {
                    region: Some("us-east-1".try_into().unwrap()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        };

        assert_eq!(settings, expected);
    }

    #[test]
    fn test_settings_view_read_overwritten() {
        // When settings are written, the delta is fetched first
        let readonly_settings = SettingsView {
            aws: Some(AwsSettingsV1 {
                region: Some("us-west-2".try_into().unwrap()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut settings = SettingsViewDelta::from_api_response(readonly_settings.clone());

        settings_view_set!(settings.aws.region = "us-east-1".try_into().unwrap());
        assert_eq!(
            settings_view_get!(settings.aws.region).map(ToString::to_string),
            Some("us-east-1".to_string())
        );
    }
}
