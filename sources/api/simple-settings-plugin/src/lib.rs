/*!
  This crate contains a simple settings plugin for unit tests.
*/

use bottlerocket_settings_models::model_derive::model;
use bottlerocket_settings_plugin::SettingsPlugin;

#[derive(SettingsPlugin)]
#[model(rename = "settings", impl_default = true)]
struct SimpleSettings {
    motd: bottlerocket_settings_models::MotdV1,
    ntp: bottlerocket_settings_models::NtpSettingsV1,
}
