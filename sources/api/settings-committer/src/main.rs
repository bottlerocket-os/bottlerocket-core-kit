/*!
# Introduction

settings-committer can be called to commit a pending transaction in the API.
It logs any pending settings, then commits them to live.

By default, it commits the 'bottlerocket-launch' transaction, which is used to organize boot-time services - this program is typically run as a pre-exec command by any services that depend on settings changes from previous services.

The `--transaction` argument can be used to specify another transaction.
*/
use settings_committer::{commit};
use simplelog::{Config as LogConfig, LevelFilter, SimpleLogger};
use snafu::ResultExt;
use std::str::FromStr;
use std::{env, process};

mod error {
    use settings_committer::SettingsCommitterError;
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub(super) enum Error {
        #[snafu(display("Error when committing settings: {}", source))]
        Commit { source: SettingsCommitterError },

        #[snafu(display("Logger setup error: {}", source))]
        Logger { source: log::SetLoggerError },
    }
}
type Result<T> = std::result::Result<T, error::Error>;

/// Store the args we receive on the command line
struct Args {
    transaction: String,
    log_level: LevelFilter,
    socket_path: String,
}

/// Print a usage message in the event a bad arg is passed
fn usage() -> ! {
    let program_name = env::args().next().unwrap_or_else(|| "program".to_string());
    eprintln!(
        r"Usage: {}
            [ --transaction TX ]
            [ --socket-path PATH ]
            [ --log-level trace|debug|info|warn|error ]

    Transaction defaults to {}
    Socket path defaults to {}",
        program_name,
        constants::LAUNCH_TRANSACTION,
        constants::API_SOCKET
    );
    process::exit(2);
}

/// Prints a more specific message before exiting through usage().
fn usage_msg<S: AsRef<str>>(msg: S) -> ! {
    eprintln!("{}\n", msg.as_ref());
    usage();
}

/// Parse the args to the program and return an Args struct
fn parse_args(args: env::Args) -> Args {
    let mut transaction = None;
    let mut log_level = None;
    let mut socket_path = None;

    let mut iter = args.skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_ref() {
            "--transaction" => {
                transaction = Some(
                    iter.next()
                        .unwrap_or_else(|| usage_msg("Did not give argument to --transaction")),
                )
            }

            "--log-level" => {
                let log_level_str = iter
                    .next()
                    .unwrap_or_else(|| usage_msg("Did not give argument to --log-level"));
                log_level =
                    Some(LevelFilter::from_str(&log_level_str).unwrap_or_else(|_| {
                        usage_msg(format!("Invalid log level '{log_level_str}'"))
                    }));
            }

            "--socket-path" => {
                socket_path = Some(
                    iter.next()
                        .unwrap_or_else(|| usage_msg("Did not give argument to --socket-path")),
                )
            }

            _ => usage(),
        }
    }

    Args {
        transaction: transaction.unwrap_or_else(|| constants::LAUNCH_TRANSACTION.to_string()),
        log_level: log_level.unwrap_or(LevelFilter::Info),
        socket_path: socket_path.unwrap_or_else(|| constants::API_SOCKET.to_string()),
    }
}

async fn run() -> Result<()> {
    // Parse and store the args passed to the program
    let args = parse_args(env::args());

    // SimpleLogger will send errors to stderr and anything less to stdout.
    SimpleLogger::init(args.log_level, LogConfig::default()).context(error::LoggerSnafu)?;

    commit(&args.socket_path, &args.transaction).await.context(error::CommitSnafu)?;

    Ok(())
}

// Returning a Result from main makes it print a Debug representation of the error, but with Snafu
// we have nice Display representations of the error, so we wrap "main" (run) and print any error.
// https://github.com/shepmaster/snafu/issues/110
#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{e}");
        process::exit(1);
    }
}
