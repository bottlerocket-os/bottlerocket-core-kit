/*!
# Bootstrap commands

`bootstrap-commands` ensures that bootstrap commands are executed as defined in the system
settings. It is called by `bootstrap-commands.service` which runs prior to the execution of
`bootstrap-containers`.

Each bootstrap command is a set of Bottlerocket API commands. The settings are first rendered
into a config file. Then, the system is configured by going through all the bootstrap commands
in lexicographical order and running all the commands inside it.

## Example:
Given a bootstrap command called `001-test-bootstrap-commands` with the following configuration:

```toml
[settings.bootstrap-commands.001-test-bootstrap-commands]
commands = [[ "apiclient", "set", "motd=helloworld"]]
essential = true
mode = "always"
```
This would set `/etc/motd` to "helloworld".

# Additional Information:
Certain valid `apiclient` commands that work in a session may fail in `bootstrap-commands`
due to relevant services not running at the time of the launch of the systemd service.

## Example:
```toml
[settings.bootstrap-commands.001-test-bootstrap-commands]
commands = [[ "apiclient", "exec", "admin", "ls"]]
essential = true
mode = "always"
```
This command fails because `bootstrap-commands.service` which calls this binary is launched
prior to `preconfigured.target` while `host-containers@.service` which is a requirement for
running "exec" commands are launched after preconfigured.target.
*/

use log::{info, warn};
use serde::Deserialize;
use simplelog::{Config as LogConfig, LevelFilter, SimpleLogger};
use snafu::{ensure, OptionExt, ResultExt};
use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::str::FromStr;

use bottlerocket_modeled_types::{ApiclientCommand, BootstrapMode, Identifier};

const DEFAULT_CONFIG_PATH: &str = "/etc/bootstrap-commands/bootstrap-commands.toml";

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct BootstrapCommandConfig {
    #[serde(default)]
    bootstrap_commands: BTreeMap<Identifier, BootstrapCommand>,
}

impl BootstrapCommandConfig {
    // Gets an iterator for bootstrap_commands, sorted in lexicographical order of their names.
    fn iter(self) -> impl Iterator<Item = (Identifier, BootstrapCommand)> {
        self.bootstrap_commands.into_iter()
    }
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct BootstrapCommand {
    #[serde(default)]
    commands: Vec<ApiclientCommand>,
    #[serde(default)]
    mode: BootstrapMode,
    #[serde(default)]
    essential: bool,
}

/// Stores user-supplied global arguments
struct Args {
    log_level: LevelFilter,
    config_path: PathBuf,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            log_level: LevelFilter::Info,
            config_path: PathBuf::from_str(DEFAULT_CONFIG_PATH).unwrap(),
        }
    }
}

/// Wrapper around process::Command that adds error checking.
fn run_command<I, S>(bin_path: &str, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(bin_path);

    let output = command
        .args(args)
        .output()
        .context(error::ExecutionFailureSnafu { command })?;

    ensure!(
        output.status.success(),
        error::CommandFailureSnafu { bin_path, output }
    );

    Ok(())
}

fn handle_bootstrap_command<S>(name: S, bootstrap_command: BootstrapCommand) -> Result<()>
where
    S: AsRef<str>,
{
    let name = name.as_ref();
    let mode = bootstrap_command.mode.as_ref();
    let commands = &bootstrap_command.commands;

    if mode == "off" {
        // If mode is 'off', just log this information.
        info!("Bootstrap command mode for '{}' is 'off'", name);
        return Ok(());
    }

    info!("Processing bootstrap command '{}' ... ", name);

    for command in commands.iter() {
        let (cmd, args) = command.get_command_and_args();
        run_command(cmd, args)?;
    }

    if mode == "once" {
        let formatted = format!("settings.bootstrap-commands.{}.mode=off", name);
        info!("Turning off bootstrap command '{}'", name);
        run_command("apiclient", ["set", formatted.as_str()])?;
    }

    info!("Successfully ran bootstrap command '{}'", name);

    Ok(())
}

/// Read our config file for the set of defined bootstrap commands
fn get_bootstrap_commands<P>(config_path: P) -> Result<BootstrapCommandConfig>
where
    P: AsRef<Path>,
{
    let config_str = fs::read_to_string(config_path.as_ref()).context(error::ReadConfigSnafu {
        config_path: config_path.as_ref(),
    })?;

    let config: BootstrapCommandConfig =
        toml::from_str(&config_str).context(error::DeserializationSnafu {
            config_path: config_path.as_ref(),
        })?;

    Ok(config)
}

/// Parse the args to the program and return an Args struct
fn parse_args(args: env::Args) -> Result<Args> {
    let mut global_args = Args::default();

    let mut iter = args.skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_ref() {
            // Global args
            "--log-level" => {
                let log_level = iter.next().context(error::UsageSnafu {
                    message: "Did not give argument to --log-level",
                })?;
                global_args.log_level = LevelFilter::from_str(&log_level)
                    .context(error::LogLevelSnafu { log_level })?;
            }

            "-c" | "--config-path" => {
                let config_str = iter.next().context(error::UsageSnafu {
                    message: "Did not give argument to --config-path",
                })?;
                global_args.config_path = PathBuf::from(config_str.as_str());
            }

            _ => (),
        }
    }

    Ok(global_args)
}

fn run() -> Result<()> {
    let args = parse_args(env::args())?;

    // SimpleLogger will send errors to stderr and anything less to stdout.
    SimpleLogger::init(args.log_level, LogConfig::default()).context(error::LoggerSnafu)?;

    let bootstrap_commands = get_bootstrap_commands(args.config_path)?;

    for (bootstrap_command_name, bootstrap_command) in bootstrap_commands.iter() {
        let name = bootstrap_command_name.as_ref();
        let essential = bootstrap_command.essential;
        let result = handle_bootstrap_command(name, bootstrap_command);

        if let Err(ref e) = result {
            warn!("Bootstrap command failed to execute {}", e);
        }
        ensure!(
            !essential || result.is_ok(),
            error::BootstrapCommandExecutionSnafu { name }
        )
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        process::exit(1);
    }
}

// =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=

mod error {
    use snafu::Snafu;
    use std::path::PathBuf;
    use std::process::{Command, Output};

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub(super) enum Error {
        #[snafu(display("Failed to read settings from config at {}: {}", config_path.display(), source))]
        ReadConfig {
            config_path: PathBuf,
            source: std::io::Error,
        },

        #[snafu(display("Failed to deserialize settings from config at {}: {}", config_path.display(), source))]
        Deserialization {
            config_path: PathBuf,
            source: toml::de::Error,
        },

        #[snafu(display("Bootstrap command '{}' failed.", name))]
        BootstrapCommandExecution { name: String },

        #[snafu(display("Failed to execute '{:?}': {}", command, source))]
        ExecutionFailure {
            command: Command,
            source: std::io::Error,
        },

        #[snafu(display("'{}' failed - stderr: {}",
                        bin_path, String::from_utf8_lossy(&output.stderr)))]
        CommandFailure { bin_path: String, output: Output },

        #[snafu(display("Logger setup error: {}", source))]
        Logger { source: log::SetLoggerError },

        #[snafu(display("Invalid log level '{}'", log_level))]
        LogLevel {
            log_level: String,
            source: log::ParseLevelError,
        },

        #[snafu(display("{}", message))]
        Usage { message: String },
    }
}

type Result<T> = std::result::Result<T, error::Error>;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_bootstrap_commands() {
        let config_toml = r#"[bootstrap-commands."002-test-bootstrap-commands"]
        commands = [["apiclient", "set", "motd=helloworld2"], ["apiclient", "set", "motd=helloworld3"]]
        mode = "once"
        essential = true

        [bootstrap-commands."001-test-bootstrap-commands"]
        commands = [["apiclient", "set", "motd=helloworld1"]]
        mode = "always"
        essential = true
        "#;

        let temp_dir = tempfile::TempDir::new().unwrap();
        let temp_config = Path::join(temp_dir.path(), "bootstrap-commands.toml");
        std::fs::write(&temp_config, config_toml).unwrap();

        let bootstrap_command_config = get_bootstrap_commands(&temp_config).unwrap();
        let bootstrap_commands = bootstrap_command_config.bootstrap_commands;

        let mut expected_bootstrap_commands = BTreeMap::new();
        let testcmd_1 = ApiclientCommand::try_from(vec![
            "apiclient".to_string(),
            "set".to_string(),
            "motd=helloworld1".to_string(),
        ])
        .unwrap();
        let testcmd_2 = ApiclientCommand::try_from(vec![
            "apiclient".to_string(),
            "set".to_string(),
            "motd=helloworld2".to_string(),
        ])
        .unwrap();
        let testcmd_3 = ApiclientCommand::try_from(vec![
            "apiclient".to_string(),
            "set".to_string(),
            "motd=helloworld3".to_string(),
        ])
        .unwrap();
        expected_bootstrap_commands.insert(
            Identifier::try_from("001-test-bootstrap-commands").unwrap(),
            BootstrapCommand {
                commands: vec![testcmd_1],
                mode: BootstrapMode::try_from("always").unwrap(),
                essential: true,
            },
        );
        expected_bootstrap_commands.insert(
            Identifier::try_from("002-test-bootstrap-commands").unwrap(),
            BootstrapCommand {
                commands: vec![testcmd_2, testcmd_3],
                mode: BootstrapMode::try_from("once").unwrap(),
                essential: true,
            },
        );

        assert_eq!(bootstrap_commands, expected_bootstrap_commands)
    }

    #[test]
    fn test_get_bootstrap_commands_defaults() {
        let config_toml = r#"[bootstrap-commands."001-test-bootstrap-commands"]
        commands = []
        "#;

        let temp_dir = tempfile::TempDir::new().unwrap();
        let temp_config = Path::join(temp_dir.path(), "bootstrap-commands.toml");
        std::fs::write(&temp_config, config_toml).unwrap();

        let bootstrap_command_config = get_bootstrap_commands(&temp_config).unwrap();
        let bootstrap_commands = bootstrap_command_config.bootstrap_commands;

        let mut expected_bootstrap_commands = BTreeMap::new();
        expected_bootstrap_commands.insert(
            Identifier::try_from("001-test-bootstrap-commands").unwrap(),
            BootstrapCommand {
                commands: vec![],
                mode: BootstrapMode::try_from("off").unwrap(),
                essential: false,
            },
        );

        assert_eq!(bootstrap_commands, expected_bootstrap_commands)
    }

    #[test]
    fn test_get_bootstrap_commands_invalid() {
        let config_toml = r#"[bootstrap-commands."001-test-bootstrap-commands"]
        commands = [["/usr/bin/touch", "helloworld.txt"], ["apiclient", "set", "motd=helloworld3"]]
        mode = "once"
        essential = true
        "#;

        let temp_dir = tempfile::TempDir::new().unwrap();
        let temp_config = Path::join(temp_dir.path(), "bootstrap-commands.toml");
        std::fs::write(&temp_config, config_toml).unwrap();

        // It should fail because one of the commands is not valid.
        assert!(get_bootstrap_commands(&temp_config).is_err());
    }
}
