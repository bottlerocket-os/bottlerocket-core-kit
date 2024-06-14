/*!
  This crate contains a simple settings plugin for unit tests.
*/

use bottlerocket_settings_plugin::SettingsPlugin;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NtpSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    time_servers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, SettingsPlugin)]
struct SimpleSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    motd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ntp: Option<NtpSettings>,
}
