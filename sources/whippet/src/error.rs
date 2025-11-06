//! Error types and handling for whippet.
//!
//! This module defines all error types used throughout the application,
//! providing structured error handling with context information.

use snafu::Snafu;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    // Configuration errors
    #[snafu(display("Failed to initialize logger"))]
    Logger { source: log::SetLoggerError },

    // Policy parsing errors
    #[snafu(display("Failed to parse policy content"))]
    PolicyParse { source: toml::de::Error },

    #[snafu(display("Failed to collect paths from '{what}'"))]
    CollectPaths {
        what: String,
        source: std::io::Error,
    },

    #[snafu(display("Failed to read path '{what}'"))]
    ReadPath {
        what: String,
        source: std::io::Error,
    },

    #[snafu(display("User context for user '{username}' includes connect rule"))]
    UserContextWithConnectRule { username: String },

    #[snafu(display("User '{username}' not found"))]
    UserNotFound { username: String },

    #[snafu(display("Failed to lookup user '{username}'"))]
    UserLookupFailed {
        username: String,
        source: nix::errno::Errno,
    },

    // Broker errors
    #[snafu(display("Failed to build D-Bus broker proxy"))]
    DbusBrokerProxyBuild { source: zbus::Error },

    #[snafu(display("Failed to build connection to D-Bus broker"))]
    DbusBrokerConnectionBuild { source: zbus::Error },

    #[snafu(display("Failed to send policy and socket to the broker"))]
    DbusBrokerAddListener { source: zbus::Error },

    #[snafu(display("Failed to configure D-Bus broker"))]
    DbusBrokerConfigure { source: tokio::task::JoinError },

    #[snafu(display("Failed to parse object path for add listener call"))]
    ParseObjectPath { source: zvariant::Error },

    #[snafu(display("Failed to notify systemd"))]
    SystemdNotification { source: std::io::Error },

    #[snafu(display("Failed to clone controller socket handle"))]
    ControllerClone { source: std::io::Error },

    // Socket errors
    #[snafu(display("Failed to retrieve '{what}' environment variable"))]
    MissingEnvVar {
        what: String,
        source: std::env::VarError,
    },

    #[snafu(display("LISTEN_PID mismatch, '{expected}', found '{found}'"))]
    UnexpectedPid { expected: String, found: String },

    #[snafu(display("Expected at most 1 file descriptor, found '{found}'"))]
    UnexpectedFileDescriptorCount { found: u32 },

    #[snafu(display("Invalid file descriptor count: {count}"))]
    InvalidFileDescriptorCount {
        count: String,
        source: std::num::ParseIntError,
    },

    #[snafu(display("Failed to get flags for file descriptor '{fd}'"))]
    GetFdFlags { fd: i32, source: nix::errno::Errno },

    #[snafu(display("Failed to set '{what}' flag for file descriptor '{fd}'"))]
    SetFlag {
        what: String,
        fd: i32,
        source: nix::errno::Errno,
    },

    #[snafu(display("Socket pair creation failed"))]
    SocketPair { source: std::io::Error },

    #[snafu(display("Error while creating socket for journal"))]
    JournalSocket { source: std::io::Error },

    #[snafu(display("Error while creating socket for systemd notification"))]
    SystemdNotifySocket { source: std::io::Error },

    // Child errors
    #[snafu(display("Failed to read machine ID from '{path}'"))]
    MachineId {
        path: String,
        source: std::io::Error,
    },

    #[snafu(display("Called wait in a child that hasn't started"))]
    ChildNotRunning,

    #[snafu(display("Failed to create SIGTERM signal"))]
    CreateSigTermSignal { source: std::io::Error },

    #[snafu(display("Failed to spawn dbus-broker process"))]
    BrokerSpawn { source: std::io::Error },

    #[snafu(display("Failed to wait for dbus-broker process"))]
    BrokerWait { source: std::io::Error },

    // Dbus Policy
    #[snafu(display("Cannot convert rule '{rule_type}' to '{record_type}'"))]
    RuleToRecord {
        rule_type: String,
        record_type: String,
    },

    #[snafu(display("Cannot convert UID '{uid}' to integer"))]
    ParseUid {
        uid: String,
        source: std::num::ParseIntError,
    },

    #[snafu(display("Can't get property '{property}' from rule '{rule_type}'"))]
    InvalidPropertyForRule { property: String, rule_type: String },
}
