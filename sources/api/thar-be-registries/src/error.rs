use snafu::Snafu;
use std::path::PathBuf;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(super)))]
pub(super) enum Error {
    #[snafu(display("Failed to read settings from '{}': {}", path, source))]
    ReadSettings {
        path: String,
        source: std::io::Error,
    },

    #[snafu(display("Failed to parse settings from '{}': {}", path, source))]
    ParseSettings {
        path: String,
        source: toml::de::Error,
    },

    #[snafu(display("Failed to create directory '{}': {}", path, source))]
    CreateDir {
        path: String,
        source: std::io::Error,
    },

    #[snafu(display("Failed to write file '{}': {}", path, source))]
    WriteFile {
        path: String,
        source: std::io::Error,
    },

    #[snafu(display("Failed to serialize TOML: {}", source))]
    SerializeToml { source: toml::ser::Error },

    #[snafu(display(
        "Failed to write {} of {} registry configs; see above",
        failure_count,
        total_count
    ))]
    WriteRegistries {
        failure_count: usize,
        total_count: usize,
    },

    #[snafu(display("Failed to parse registry '{}' as a valid URL", registry))]
    ParseRegistry { registry: String },

    #[snafu(display("Failed to rename '{}' to '{}': {}", from.display(), to.display(), source))]
    RenameDir {
        from: PathBuf,
        to: PathBuf,
        source: nix::errno::Errno,
    },
}

pub(super) type Result<T> = std::result::Result<T, Error>;
