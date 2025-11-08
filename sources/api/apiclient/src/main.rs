//! The apiclient binary provides some high-level, synchronous methods of interacting with the
//! API, for example an `update` subcommand that wraps the individual API calls needed to update
//! the host.  There's also a low-level `raw` subcommand for direct interaction.

// This file contains the arg parsing and high-level behavior.  (Massaging input data, making
// library calls based on the given flags, etc.)  The library modules contain the code for talking
// to the API, which is intended to be reusable by other crates.

use apiclient::{
    apply, ephemeral_storage, exec, get, lockdown, network, reboot, report, set, update,
    SettingsInput,
};
use log::{info, log_enabled, trace, warn};
use model::ephemeral_storage::{Filesystem, Preference};
use serde::{Deserialize, Serialize};
use simplelog::{
    ColorChoice, ConfigBuilder as LogConfigBuilder, LevelFilter, TermLogger, TerminalMode,
};
use snafu::ResultExt;
use std::env;
use std::ffi::OsString;
use std::iter::Peekable;
use std::process;
use std::str::FromStr;
use std::vec::IntoIter;
use unindent::unindent;

const DEFAULT_METHOD: &str = "GET";

/// Stores user-supplied global arguments.
#[derive(Debug)]
struct Args {
    log_level: LevelFilter,
    socket_path: String,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            log_level: LevelFilter::Info,
            socket_path: constants::API_SOCKET.to_string(),
        }
    }
}

/// Stores the usage mode specified by the user as a subcommand.
#[derive(Debug)]
enum Subcommand {
    Apply(ApplyArgs),
    Exec(ExecArgs),
    Get(GetArgs),
    Lockdown(LockdownArgs),
    Network(NetworkSubcommand),
    Raw(RawArgs),
    Reboot(RebootArgs),
    Set(SetArgs),
    Update(UpdateSubcommand),
    Report(ReportSubcommand),
    EphemeralStorage(EphemeralStorageSubcommand),
}

/// Stores user-supplied arguments for the 'apply' subcommand.
#[derive(Debug)]
struct ApplyArgs {
    input_sources: Vec<String>,
}

/// Stores user-supplied arguments for the 'exec' subcommand.
#[derive(Debug)]
struct ExecArgs {
    command: Vec<OsString>,
    target: String,
    tty: Option<bool>,
}

/// Stores user-supplied arguments for the 'get' subcommand.
#[derive(Debug)]
enum GetArgs {
    Prefixes {
        include: Vec<String>,
        exclude: Vec<String>,
        canonicalize: bool,
    },
    Uri(String, bool),
}

/// Stores user-supplied arguments for the 'raw' subcommand.
#[derive(Debug)]
struct RawArgs {
    method: String,
    uri: String,
    data: Option<String>,
}

/// Stores user-supplied arguments for the 'lockdown' subcommand.
#[derive(Debug)]
struct LockdownArgs {}

/// Stores user-supplied arguments for the 'reboot' subcommand.
#[derive(Debug)]
struct RebootArgs {}

/// Stores a vector of user-supplied key-value pairs for the 'set' subcommand.
#[derive(Serialize, Deserialize)]
pub struct SetKeyPairSettings {
    request_payload: Vec<String>,
}

/// Stores user-supplied arguments for the 'set' subcommand.
#[derive(Debug)]
enum SetArgs {
    Simple(Vec<String>),
    Json(serde_json::Value),
}

/// Stores the 'update' subcommand specified by the user.
#[derive(Debug)]
enum UpdateSubcommand {
    Check(UpdateCheckArgs),
    Apply(UpdateApplyArgs),
    Cancel(UpdateCancelArgs),
}

/// The available 'report' subcommands.
#[derive(Debug)]
enum ReportSubcommand {
    Cis(CisReportArgs),
    CisK8s(CisReportArgs),
    Fips(FipsReportArgs),
}

/// Stores common user-supplied arguments for the cis report subcommand.
#[derive(Debug)]
struct CisReportArgs {
    level: Option<i32>,
    format: Option<String>,
}

/// Stores common user-supplied arguments for the fips report subcommand.
#[derive(Debug)]
struct FipsReportArgs {
    format: Option<String>,
}

/// Stores user-supplied arguments for the 'update check' subcommand.
#[derive(Debug)]
struct UpdateCheckArgs {}

/// Stores user-supplied arguments for the 'update apply' subcommand.
#[derive(Debug)]
struct UpdateApplyArgs {
    check: bool,
    reboot: bool,
}

/// Stores user-supplied arguments for the 'update cancel' subcommand.
#[derive(Debug)]
struct UpdateCancelArgs {}

/// Stores the 'ephemeral-storage' subcommand specified by the user.
#[derive(Debug)]
enum EphemeralStorageSubcommand {
    Init(EphemeralStorageInitArgs),
    Bind(EphemeralStorageBindArgs),
    ListDisks(EphemeralStorageFormatArgs),
    ListEbsVolumes(EphemeralStorageFormatArgs),
    ListDirs(EphemeralStorageFormatArgs),
}

/// Stores user-supplied arguments for the 'ephemeral-storage init' subcommand.
#[derive(Debug)]
struct EphemeralStorageInitArgs {
    disks: Option<Vec<String>>,
    ebs_volumes: Option<Vec<String>>,
    prefer: Option<Vec<Preference>>,
    filesystem: Option<Filesystem>,
}

/// Stores user-supplied arguments for the 'ephemeral-storage bind' subcommand.
#[derive(Debug)]
struct EphemeralStorageBindArgs {
    targets: Vec<String>,
}
/// Stores user-supplied arguments for the 'ephemeral-storage list-disks/list-ebs-volumes/list-dirs' subcommand.
#[derive(Debug)]
struct EphemeralStorageFormatArgs {
    format: Option<String>,
}

/// Stores the 'network' subcommand specified by the user.
#[derive(Debug)]
enum NetworkSubcommand {
    Configure(NetworkConfigureArgs),
}

/// Stores user-supplied arguments for the 'network configure' subcommand.
#[derive(Debug)]
struct NetworkConfigureArgs {
    input_source: Option<String>,
}

/// Informs the user about proper usage of the program and exits.
fn usage() -> ! {
    let msg = &format!(
        r#"Usage: apiclient [SUBCOMMAND] [OPTION]...

        Global options:
            -s, --socket-path PATH     Override the server socket path.  Default: {socket}
            --log-level                Desired amount of output; trace|debug|info|warn|error
            -v, --verbose              Sets log level to 'debug'.  This prints extra info,
                                       like HTTP status code to stderr in 'raw' mode.

        Subcommands:
            raw                        Makes an HTTP request and prints the response on stdout.
                                       'raw' is the default subcommand and may be omitted.
            apply                      Applies settings from TOML/JSON files at given URIs,
                                       or from stdin.
            get                        Retrieve and print settings.
            set                        Changes settings and applies them to the system.
            network configure          Configures network settings from net.toml files at
                                       given URIs.
            update check               Prints information about available updates.
            update apply               Applies available updates.
            update cancel              Deactivates an applied update.
            lockdown                   Locks down the host.
            reboot                     Reboots the host.
            exec                       Execute a command in a host container.
            report cis                 Retrieve a Bottlerocket CIS benchmark compliance report.
            report cis-k8s             Retrieve a Kubernetes CIS benchmark compliance report.
            report fips                Retrieve a FIPS Security Policy compliance report.
            ephemeral-storage init     Initialize ephemeral storage
            ephemeral-storage bind     Bind directories to previously initialized ephemeral storage.
            ephemeral-storage list-disks
                                       List the discovered ephemeral disks that can be initialized.
            ephemeral-storage list-ebs-volumes
                                       List the discovered ephemeral ebs volumes that can be initialized.
            ephemeral-storage list-dirs
                                       List the directories that can be bound to ephemeral storage.

        raw options:
            -u, --uri URI              Required; URI to request from the server, e.g. /tx
            -m, -X, --method METHOD    HTTP method to use in request.  Default: {method}
            -d, --data DATA            Data to include in the request body.  Default: empty

        apply options:
            [ URI ...]                 The list of URIs to TOML or JSON settings files that you
                                       want to apply to the system.  If no URI is specified, or
                                       if "-" is given, reads from stdin.

        lockdown options:
            None.

        reboot options:
            None.

        get options:
            [ PREFIX [PREFIX ...] ]    The settings you want to get.  Full settings names work fine,
                                       or you can specify prefixes to fetch all settings under them.
            [ /desired-uri ]           The API URI to fetch.  Cannot be specified with prefixes.
            --exclude PREFIX           Exclude settings matching this prefix from the results.
                                       Can be specified multiple times. Only valid with prefixes.
            --canonicalize             Output as canonical JSON (no whitespace, sorted keys).

                                       If neither prefixes nor URI are specified, get will show
                                       settings and OS info.

        network configure options:
            [ URI ]                    URI to a network configuration file (TOML format)
                                       to apply. Supports file:// and base64: URI schemes.
                                       If no URI is specified, reads from stdin.
                                       Configuration is written to /.bottlerocket/net.toml and
                                       validated at next boot by netdog.

        set options:
            KEY=VALUE [KEY=VALUE ...]  The settings you want to set.  For example:
                                          settings.motd="hi there" settings.ecs.cluster=example
                                       The "settings." prefix is optional.
                                       Settings with dots in the name require nested quotes:
                                          'kubernetes.node-labels."my.label"=hello'
            -j, --json JSON            Alternatively, you can specify settings in JSON format,
                                       which can simplify setting multiple values, and is necessary
                                       for some numeric settings.  For example:
                                          -j '{{"kernel": {{"sysctl": {{"vm.max_map_count": "262144"}}}}}}'

        update check options:
            None.

        update apply options:
            -c, --check                Automatically `update check` and apply whatever is found.
            -r, --reboot               Automatically reboot if an update was found and applied.

        update cancel options:
            None.

        exec options:
            -t, --tty                  Force the server to run the program in a pseudoterminal.
            -T, --no-tty               Force the server not to run the program in a pseudoterminal.

            TARGET                     Required; the name of the container in which to run the command.
            COMMAND                    Required; the command to run.
            [ ARG ...]                 Any desired arguments to the command.

        report cis options:
            -f, --format               Format of the CIS report (text or json). Default format is text.
            -l, --level                CIS compliance level to report on (1 or 2). Default is 1.

        report cis-k8s options:
            -f, --format               Format of the CIS report (text or json). Default format is text.
            -l, --level                CIS compliance level to report on (1 or 2). Default is 1.

        ephemeral-storage init options:
            -t, --filesystem           Filesystem to initialize the array as (ext4 or xfs). Default is
                                       xfs. If a single disk is provided, it is mounted directly without
                                       constructing an array. If no ephemeral disks are found, this
                                       operation does nothing.
            --disks DISK [DISK ...]    Local disks to configure for storage. Default is all ephemeral
                                       disks.
            --ebs-volumes VOLUME [VOLUME ..]
                                       EBS volumes in the `xvdda`-`xvddx` range to configure for storage.
            --prefer PREFERENCE [PREFERENCE ..]
                                       Ephemeral storage type preference, in descending order. Allowed
                                       values: `ephemeral-disk`, `ebs-volume` or a combination of these
                                       joined by `+`. Defaults to `ephemeral-disk` only. This option is
                                       ignored if `--disks` or `--ebs-volumes` is set.
        ephemeral-storage bind options:
            --dirs DIR [DIR ...]       Directories to bind to configured ephemeral storage
                                       (e.g. /var/lib/containerd). If not specified, uses platform (k8s vs. ECS)
                                       defaults. If no ephemeral disks are found this operation does nothing.


        ephemeral-storage list-disks options:
            -f, --format               Format of the disk listing (text or json). Default format is text.

        ephemeral-storage list-ebs-volumes options:
            -f, --format               Format of the volume listing (text or json). Default format is text.   

        ephemeral-storage list-dirs options:
            -f, --format               Format of the directory listing (text or json). Default format is text.

            "#,
        socket = constants::API_SOCKET,
        method = DEFAULT_METHOD,
    );
    eprintln!("{}", unindent(msg));
    process::exit(2);
}

/// Prints a more specific message before exiting through usage().
fn usage_msg<S: AsRef<str>>(msg: S) -> ! {
    eprintln!("{}\n", msg.as_ref());
    usage();
}

// =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=
// Arg parsing

/// Parses user arguments into an Args structure.
fn parse_args(args: impl Iterator<Item = String>) -> (Args, Subcommand) {
    let mut global_args = Args::default();
    let mut subcommand = None;
    let mut subcommand_args = Vec::new();

    let mut iter = args.into_iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_ref() {
            // Handle separator - only valid after exec subcommand is set
            "--" => {
                if subcommand.is_none() {
                    usage_msg("'--' separator requires a subcommand to be specified first");
                }
                // Only do special handling for exec subcommand
                if subcommand.as_deref() == Some("exec") {
                    // For exec, collect all remaining args and stop processing
                    subcommand_args.extend(iter);
                    break;
                } else {
                    // For other subcommands, treat -- as a regular argument
                    subcommand_args.push(arg);
                }
            }

            "-h" | "--help" => usage(),

            // Global args
            "--log-level" => {
                let log_level_str = iter
                    .next()
                    .unwrap_or_else(|| usage_msg("Did not give argument to --log-level"));
                global_args.log_level = LevelFilter::from_str(&log_level_str)
                    .unwrap_or_else(|_| usage_msg(format!("Invalid log level '{log_level_str}'")));
            }

            "-v" | "--verbose" => global_args.log_level = LevelFilter::Debug,

            "-s" | "--socket-path" => {
                global_args.socket_path = iter
                    .next()
                    .unwrap_or_else(|| usage_msg("Did not give argument to -s | --socket-path"))
            }

            // Subcommands
            "raw" | "apply" | "exec" | "get" | "lockdown" | "network" | "reboot" | "report"
            | "set" | "update" | "ephemeral-storage"
                if subcommand.is_none() && !arg.starts_with('-') =>
            {
                subcommand = Some(arg)
            }

            // Other arguments are passed to the subcommand parser
            _ => subcommand_args.push(arg),
        }
    }

    match subcommand.as_deref() {
        // Default subcommand is 'raw'
        None | Some("raw") => (global_args, parse_raw_args(subcommand_args)),
        Some("apply") => (global_args, parse_apply_args(subcommand_args)),
        Some("exec") => (global_args, parse_exec_args(subcommand_args)),
        Some("get") => (global_args, parse_get_args(subcommand_args)),
        Some("lockdown") => (global_args, parse_lockdown_args(subcommand_args)),
        Some("network") => (global_args, parse_network_args(subcommand_args)),
        Some("reboot") => (global_args, parse_reboot_args(subcommand_args)),
        Some("report") => (global_args, parse_report_args(subcommand_args)),
        Some("set") => (global_args, parse_set_args(subcommand_args)),
        Some("update") => (global_args, parse_update_args(subcommand_args)),
        Some("ephemeral-storage") => (global_args, parse_ephemeral_storage_args(subcommand_args)),
        _ => usage_msg("Missing or unknown subcommand"),
    }
}

/// Parses arguments for the 'raw' subcommand, which is also the default if no subcommand is
/// provided.
fn parse_raw_args(args: Vec<String>) -> Subcommand {
    let mut method = None;
    let mut uri = None;
    let mut data = None;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_ref() {
            "-X" | "-m" | "--method" => {
                method = Some(
                    iter.next()
                        .unwrap_or_else(|| usage_msg("Did not give argument to -m | --method")),
                )
            }

            "-u" | "--uri" => {
                uri = Some(
                    iter.next()
                        .unwrap_or_else(|| usage_msg("Did not give argument to -u | --uri")),
                )
            }

            "-d" | "--data" => {
                data = Some(
                    iter.next()
                        .unwrap_or_else(|| usage_msg("Did not give argument to -d | --data")),
                )
            }

            x => usage_msg(format!("Unknown argument '{x}'")),
        }
    }

    Subcommand::Raw(RawArgs {
        method: method.unwrap_or_else(|| DEFAULT_METHOD.to_string()),
        uri: uri.unwrap_or_else(|| usage_msg("Missing required argument '--uri'")),
        data,
    })
}

/// Parses arguments for the 'apply' subcommand.
fn parse_apply_args(args: Vec<String>) -> Subcommand {
    let mut input_sources = Vec::new();

    for arg in args.into_iter() {
        match arg {
            // Allow "-" for stdin, but we have no other parameters.
            x if x.starts_with('-') && x != "-" => {
                usage_msg("apiclient apply takes no parameters, just a list of URIs.")
            }

            x => input_sources.push(x),
        }
    }

    if input_sources.is_empty() {
        // Read from stdin if no URIs were given.
        input_sources.push("-".to_string());
    }

    Subcommand::Apply(ApplyArgs { input_sources })
}

/// Parses arguments for the 'exec' subcommand.
fn parse_exec_args(args: Vec<String>) -> Subcommand {
    let mut command = vec![];
    let mut target = None;
    let mut tty = None;

    for arg in args.into_iter() {
        match arg.as_ref() {
            // Check for our own arguments, but stop once we start to see the user's command; we
            // don't want to intercept its own arguments.
            "-t" | "--tty" if command.is_empty() => {
                tty = Some(true);
            }
            "-T" | "--no-tty" if command.is_empty() => {
                tty = Some(false);
            }
            x if x.starts_with('-') && command.is_empty() => {
                usage_msg(format!("Unknown argument '{x}'"))
            }

            // Target is the first arg we see.
            _ if target.is_none() => target = Some(arg),
            // Anything remaining goes to the command.
            _ => command.push(arg.into()),
        }
    }

    // (check target here because it's clearer to error about it before an error about a missing command)
    let target = target.unwrap_or_else(|| usage_msg("Missing required argument 'target'"));
    if command.is_empty() {
        usage_msg("Must specify a command for 'exec' to run.");
    }

    Subcommand::Exec(ExecArgs {
        command,
        target,
        tty,
    })
}

/// Parses arguments for the 'get' subcommand.
fn parse_get_args(args: Vec<String>) -> Subcommand {
    let mut include = vec![];
    let mut exclude = vec![];
    let mut uri = None;
    let mut canonicalize = false;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_ref() {
            "--canonicalize" => canonicalize = true,
            "--exclude" => {
                let prefix = iter
                    .next()
                    .unwrap_or_else(|| usage_msg("Did not give argument to --exclude"));
                exclude.push(prefix);
            }
            x if x.starts_with('-') => usage_msg(format!("Unknown argument '{x}'")),

            x if x.starts_with('/') => {
                if let Some(_existing_val) = uri.replace(arg) {
                    usage_msg("You can only specify one URI.");
                }
            }

            // All other arguments are settings prefixes to fetch.
            _ => include.push(arg),
        }
    }

    if let Some(uri) = uri {
        if !include.is_empty() || !exclude.is_empty() {
            usage_msg("You can specify prefixes or a URI, but not both.");
        }
        Subcommand::Get(GetArgs::Uri(uri, canonicalize))
    } else {
        Subcommand::Get(GetArgs::Prefixes {
            include: if include.is_empty() {
                vec!["os.".to_string(), "settings.".to_string()]
            } else {
                include
            },
            exclude,
            canonicalize,
        })
    }
}

/// Parses arguments for the 'lockdown' subcommand.
fn parse_lockdown_args(args: Vec<String>) -> Subcommand {
    if !args.is_empty() {
        usage_msg(format!("Unknown arguments: {}", args.join(", ")));
    }
    Subcommand::Lockdown(LockdownArgs {})
}
/// Parses the desired subcommand of 'network'.
fn parse_network_args(args: Vec<String>) -> Subcommand {
    let mut subcommand = None;
    let mut subcommand_args = Vec::new();

    for arg in args.into_iter() {
        match arg.as_ref() {
            // Subcommands
            "configure" if subcommand.is_none() => subcommand = Some(arg),

            // Other arguments are passed to the subcommand parser
            _ => subcommand_args.push(arg),
        }
    }

    match subcommand.as_deref() {
        Some("configure") => parse_network_configure_args(subcommand_args),
        _ => usage_msg("Missing or unknown subcommand for 'network'"),
    }
}

/// Parses arguments for the 'network configure' subcommand.
fn parse_network_configure_args(args: Vec<String>) -> Subcommand {
    let mut input_source = None;

    for arg in args.into_iter() {
        if input_source.is_some() {
            usage_msg("apiclient network configure takes only one input source URI.")
        }
        input_source = Some(arg);
    }

    Subcommand::Network(NetworkSubcommand::Configure(NetworkConfigureArgs {
        input_source,
    }))
}

/// Parses arguments for the 'reboot' subcommand.
fn parse_reboot_args(args: Vec<String>) -> Subcommand {
    if !args.is_empty() {
        usage_msg(format!("Unknown arguments: {}", args.join(", ")));
    }
    Subcommand::Reboot(RebootArgs {})
}

/// Parses arguments for the 'set' subcommand.
// Note: the API doesn't allow setting non-settings keys, e.g. services, configuration-files, and
// metadata.  If we allow it in the future, we should revisit this 'set' parsing code and decide
// what formats to accept.  This code currently makes it as convenient as possible to set settings,
// by adding/removing a "settings" prefix as necessary.
fn parse_set_args(args: Vec<String>) -> Subcommand {
    let mut simple = Vec::new();
    let mut json = None;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_ref() {
            "-j" | "--json" if json.is_some() => {
                usage_msg(
                    "Can't specify the --json argument multiple times.  You can set as many \
                     settings as needed within the JSON object.",
                );
            }
            "-j" | "--json" if json.is_none() => {
                let raw_json = iter
                    .next()
                    .unwrap_or_else(|| usage_msg("Did not give argument to -j | --json"));

                let input_val: serde_json::Value = serde_json::from_str(&raw_json)
                    .unwrap_or_else(|e| usage_msg(format!("Couldn't parse given JSON input: {e}")));

                let mut input_map = match input_val {
                    serde_json::Value::Object(map) => map,
                    _ => usage_msg("JSON input must be an object (map)"),
                };

                // To be nice, if the user specified a "settings" layer around their data, we
                // remove it.  (This should only happen if there's a single key, since we only
                // allow setting settings; fail otherwise.  If we allow setting other types in the
                // future, we'll have to do more map manipulation here to save the other values.)
                if let Some(val) = input_map.remove("settings") {
                    match val {
                        serde_json::Value::Object(map) => input_map.extend(map),
                        _ => usage_msg("JSON 'settings' value must be an object (map)"),
                    };
                }

                json = Some(input_map.into());
            }

            x if x.contains('=') => {
                // Push each key=value pair to vector.
                simple.push(x.to_string());
            }

            x => usage_msg(format!("Unknown argument '{x}'")),
        }
    }

    if json.is_some() && !simple.is_empty() {
        usage_msg("Cannot specify key=value pairs and --json settings with 'set'");
    } else if let Some(json) = json {
        Subcommand::Set(SetArgs::Json(json))
    } else if !simple.is_empty() {
        Subcommand::Set(SetArgs::Simple(simple))
    } else {
        usage_msg("Must specify key=value settings or --json settings with 'set'");
    }
}

/// Parses the desired subcommand of 'update'.
fn parse_update_args(args: Vec<String>) -> Subcommand {
    let mut subcommand = None;
    let mut subcommand_args = Vec::new();

    for arg in args.into_iter() {
        match arg.as_ref() {
            // Subcommands
            "check" | "apply" | "cancel" if subcommand.is_none() && !arg.starts_with('-') => {
                subcommand = Some(arg)
            }

            // Other arguments are passed to the subcommand parser
            _ => subcommand_args.push(arg),
        }
    }

    let update = match subcommand.as_deref() {
        Some("check") => parse_update_check_args(subcommand_args),
        Some("apply") => parse_update_apply_args(subcommand_args),
        Some("cancel") => parse_update_cancel_args(subcommand_args),
        _ => usage_msg("Missing or unknown subcommand for 'update'"),
    };

    Subcommand::Update(update)
}

/// Parses arguments for the 'update check' subcommand.
fn parse_update_check_args(args: Vec<String>) -> UpdateSubcommand {
    if !args.is_empty() {
        usage_msg(format!("Unknown arguments: {}", args.join(", ")));
    }
    UpdateSubcommand::Check(UpdateCheckArgs {})
}

/// Parses arguments for the 'update apply' subcommand.
fn parse_update_apply_args(args: Vec<String>) -> UpdateSubcommand {
    let mut check = false;
    let mut reboot = false;

    for arg in args.into_iter() {
        match arg.as_ref() {
            "-c" | "--check" => check = true,
            "-r" | "--reboot" => reboot = true,

            x => usage_msg(format!("Unknown argument '{x}'")),
        }
    }

    UpdateSubcommand::Apply(UpdateApplyArgs { check, reboot })
}

/// Parses arguments for the 'update cancel' subcommand.
fn parse_update_cancel_args(args: Vec<String>) -> UpdateSubcommand {
    if !args.is_empty() {
        usage_msg(format!("Unknown arguments: {}", args.join(", ")));
    }
    UpdateSubcommand::Cancel(UpdateCancelArgs {})
}

/// Parses the desired subcommand of 'report'.
fn parse_report_args(args: Vec<String>) -> Subcommand {
    let mut subcommand = None;
    let mut subcommand_args = Vec::new();

    for arg in args.into_iter() {
        match arg.as_ref() {
            // Subcommands
            "cis" if subcommand.is_none() && !arg.starts_with('-') => subcommand = Some(arg),
            "cis-k8s" if subcommand.is_none() && !arg.starts_with('-') => subcommand = Some(arg),
            "fips" if subcommand.is_none() && !arg.starts_with('-') => subcommand = Some(arg),

            // Other arguments are passed to the subcommand parser
            _ => subcommand_args.push(arg),
        }
    }

    let report_type = match subcommand.as_deref() {
        Some("cis") => parse_report_cis_args(subcommand_args),
        Some("cis-k8s") => parse_report_cis_k8s_args(subcommand_args),
        Some("fips") => parse_report_fips_args(subcommand_args),
        _ => usage_msg("Missing or unknown subcommand for 'report'"),
    };

    Subcommand::Report(report_type)
}

/// Parses arguments for the 'report' cis subcommand.
fn parse_report_cis_args(args: Vec<String>) -> ReportSubcommand {
    ReportSubcommand::Cis(parse_cis_arguments(args))
}

/// Parses arguments for the 'report' cis-k8s subcommand.
fn parse_report_cis_k8s_args(args: Vec<String>) -> ReportSubcommand {
    ReportSubcommand::CisK8s(parse_cis_arguments(args))
}

fn parse_cis_arguments(args: Vec<String>) -> CisReportArgs {
    let mut level: Option<i32> = None;
    let mut format = None;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_ref() {
            "-l" | "--level" => {
                let level_str = iter
                    .next()
                    .unwrap_or_else(|| usage_msg("Did not give argument to -l | --level"));
                let level_int = level_str
                    .parse::<i32>()
                    .unwrap_or_else(|_| usage_msg("Invalid argument to -l | --level"));
                level = Some(level_int);
            }

            "-f" | "--format" => {
                format = Some(
                    iter.next()
                        .unwrap_or_else(|| usage_msg("Did not give argument to -f | --format")),
                )
            }

            x => usage_msg(format!("Unknown argument '{x}'")),
        }
    }

    CisReportArgs { level, format }
}

/// Parses arguments for the 'report' fips subcommand.
fn parse_report_fips_args(args: Vec<String>) -> ReportSubcommand {
    ReportSubcommand::Fips(parse_fips_arguments(args))
}

fn parse_fips_arguments(args: Vec<String>) -> FipsReportArgs {
    let mut format = None;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_ref() {
            "-f" | "--format" => {
                format = Some(
                    iter.next()
                        .unwrap_or_else(|| usage_msg("Did not give argument to -f | --format")),
                )
            }

            x => usage_msg(format!("Unknown argument '{x}'")),
        }
    }

    FipsReportArgs { format }
}

/// Parse the desired subcommand of 'ephemeral-storage'
fn parse_ephemeral_storage_args(args: Vec<String>) -> Subcommand {
    let mut subcommand = None;
    let mut subcommand_args = Vec::new();

    for arg in args.into_iter() {
        match arg.as_ref() {
            // Subcommands
            "init" | "bind" | "list-disks" | "list-ebs-volumes" | "list-dirs"
                if subcommand.is_none() && !arg.starts_with('-') =>
            {
                subcommand = Some(arg)
            }

            // Other arguments are passed to the subcommand parser
            _ => subcommand_args.push(arg),
        }
    }

    let cmd = match subcommand.as_deref() {
        Some("init") => parse_ephemeral_storage_init_args(subcommand_args),
        Some("bind") => parse_ephemeral_storage_bind_args(subcommand_args),
        Some("list-disks") => EphemeralStorageSubcommand::ListDisks(
            parse_ephemeral_storage_list_format_args(subcommand_args),
        ),
        Some("list-ebs-volumes") => EphemeralStorageSubcommand::ListEbsVolumes(
            parse_ephemeral_storage_list_format_args(subcommand_args),
        ),
        Some("list-dirs") => EphemeralStorageSubcommand::ListDirs(
            parse_ephemeral_storage_list_format_args(subcommand_args),
        ),
        _ => usage_msg("Missing or unknown subcommand for 'ephemeral-storage'"),
    };
    Subcommand::EphemeralStorage(cmd)
}

/// Parses arguments for the 'init' ephemeral-storage subcommand.
fn parse_ephemeral_storage_init_args(args: Vec<String>) -> EphemeralStorageSubcommand {
    let mut disks: Option<Vec<String>> = None;
    let mut ebs_volumes: Option<Vec<String>> = None;
    let mut prefer: Option<Vec<Preference>> = None;
    let mut filesystem = None;
    let mut iter = args.into_iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_ref() {
            "-t" | "--filesystem" => {
                match iter
                    .next()
                    .unwrap_or_else(|| usage_msg("Did not give argument to -t | --filesystem"))
                    .as_str()
                {
                    "ext4" => filesystem = Some(Filesystem::Ext4),
                    "xfs" => filesystem = Some(Filesystem::Xfs),
                    _ => usage_msg("Unsupported filesystem type"),
                }
            }
            "--disks" => {
                let mut names = collect_non_args(&mut iter);
                if names.is_empty() {
                    usage_msg("Did not give argument to --disks")
                }
                if let Some(existing) = &mut disks {
                    existing.append(&mut names);
                } else {
                    disks = Some(names);
                }
            }
            "--ebs-volumes" => {
                let mut names = collect_non_args(&mut iter);
                if names.is_empty() {
                    usage_msg("Did not give argument to --ebs-volumes")
                }
                if let Some(existing) = &mut ebs_volumes {
                    existing.append(&mut names);
                } else {
                    ebs_volumes = Some(names);
                }
            }
            "--prefer" => {
                let prefs = collect_non_args(&mut iter);
                if prefs.is_empty() {
                    usage_msg("Did not give argument to --prefer")
                }
                if let Some(existing) = &mut prefer {
                    for p in &prefs {
                        if let Ok(p) = p.as_str().try_into() {
                            existing.push(p);
                        } else {
                            usage_msg("Invalid ephemeral storage type preference");
                        }
                    }
                } else if let Ok(p) = prefs.iter().map(|x| x.as_str().try_into()).collect() {
                    prefer = Some(p);
                } else {
                    usage_msg("Invalid ephemeral storage type preference");
                }
            }
            x => usage_msg(format!("Unknown argument '{x}'")),
        }
    }
    EphemeralStorageSubcommand::Init(EphemeralStorageInitArgs {
        disks,
        ebs_volumes,
        filesystem,
        prefer,
    })
}

/// Parses arguments for the 'bind' ephemeral-storage subcommand.
fn parse_ephemeral_storage_bind_args(args: Vec<String>) -> EphemeralStorageSubcommand {
    // If no arguments, use default directories
    if args.is_empty() {
        return EphemeralStorageSubcommand::Bind(EphemeralStorageBindArgs {
            targets: Vec::new(),
        });
    }

    let mut targets = Vec::new();
    let mut iter = args.into_iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_ref() {
            "--dirs" => {
                targets.append(&mut collect_non_args(&mut iter));
                if targets.is_empty() {
                    usage_msg("Did not give argument to --dirs")
                }
            }
            x => usage_msg(format!("Unknown argument '{x}'")),
        }
    }

    EphemeralStorageSubcommand::Bind(EphemeralStorageBindArgs { targets })
}

/// Parses arguments for the 'list-disks', 'list-ebs-volumes', and 'list-dirs' ephemeral-storage subcommands.
fn parse_ephemeral_storage_list_format_args(args: Vec<String>) -> EphemeralStorageFormatArgs {
    let mut format = None;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_ref() {
            "-f" | "--format" => {
                format = Some(
                    iter.next()
                        .unwrap_or_else(|| usage_msg("Did not give argument to -f | --format")),
                )
            }
            x => usage_msg(format!("Unknown argument '{x}'")),
        }
    }
    EphemeralStorageFormatArgs { format }
}

/// collects non-argument parameters (those not starting with a '-') up until the next
/// argument is seen
fn collect_non_args(iter: &mut Peekable<IntoIter<String>>) -> Vec<String> {
    let mut result = Vec::new();
    loop {
        // look at the following argument and stop accepting disk names
        // once we reach the end of arguments, or find the beginning of
        // the next argument
        match iter.peek() {
            None => {
                break;
            }
            Some(peeked) => {
                if peeked.is_empty() || peeked.starts_with('-') {
                    break;
                }
            }
        }
        let next = iter
            .next()
            .unwrap_or_else(|| usage_msg("Expected non-empty argument"));
        result.push(next);
    }
    result
}

// =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=
// Helpers

/// Requests an update status check through the API, printing the updated status, in a pretty
/// format if possible.
async fn check(args: &Args) -> Result<String> {
    let output = update::check(&args.socket_path)
        .await
        .context(error::UpdateCheckSnafu)?;

    match serde_json::from_str::<serde_json::Value>(&output) {
        Ok(value) => println!("{value:#}"),
        Err(e) => {
            warn!("Unable to deserialize response (invalid JSON?): {e}");
            println!("{output}");
        }
    }

    Ok(output)
}

// =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=
// Main dispatch

/// Main entry point, dispatches subcommands.
async fn run() -> Result<()> {
    let (args, subcommand) = parse_args(env::args());
    trace!("Parsed args for subcommand {subcommand:?}: {args:?}");

    // We use TerminalMode::Stderr because apiclient users expect server response data on stdout.
    TermLogger::init(
        args.log_level,
        LogConfigBuilder::new()
            .add_filter_allow_str("apiclient")
            .build(),
        TerminalMode::Stderr,
        ColorChoice::Auto,
    )
    .context(error::LoggerSnafu)?;

    #[cfg(feature = "tls")]
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    match subcommand {
        Subcommand::Raw(raw) => {
            let (status, body) =
                apiclient::raw_request(&args.socket_path, &raw.uri, &raw.method, raw.data)
                    .await
                    .context(error::RequestSnafu {
                        uri: &raw.uri,
                        method: &raw.method,
                    })?;

            // In raw mode, the user is expecting only the server response on stdout, so we more
            // carefully control other output and only write it to stderr.
            if log_enabled!(log::Level::Debug) {
                eprintln!("{status}");
            }
            if !body.is_empty() {
                println!("{body}");
            }
        }

        Subcommand::Apply(apply) => {
            apply::apply(&args.socket_path, apply.input_sources)
                .await
                .context(error::ApplySnafu)?;
        }

        Subcommand::Exec(exec) => {
            exec::exec(&args.socket_path, exec.command, exec.target, exec.tty)
                .await
                .context(error::ExecSnafu)?;
        }

        Subcommand::Get(get) => {
            let (value, canonicalize) = match get {
                GetArgs::Uri(uri, canonicalize) => {
                    (get::get_uri(&args.socket_path, uri).await, canonicalize)
                }
                GetArgs::Prefixes {
                    include,
                    exclude,
                    canonicalize,
                } => (
                    get::get_prefixes(&args.socket_path, include, exclude).await,
                    canonicalize,
                ),
            };
            let value = value.context(error::GetSnafu)?;

            if canonicalize {
                let mut buf = Vec::new();
                let mut ser = serde_json::Serializer::with_formatter(
                    &mut buf,
                    olpc_cjson::CanonicalFormatter::new(),
                );
                value
                    .serialize(&mut ser)
                    .expect("JSON Value already validated as JSON");
                println!("{}", String::from_utf8(buf).expect("Valid UTF-8"));
            } else {
                let pretty = serde_json::to_string_pretty(&value)
                    .expect("JSON Value already validated as JSON");
                println!("{pretty}");
            }
        }

        Subcommand::Lockdown(_lockdown) => {
            lockdown::lockdown(&args.socket_path)
                .await
                .context(error::LockdownSnafu)?;
        }

        Subcommand::Network(subcommand) => match subcommand {
            NetworkSubcommand::Configure(configure_args) => {
                let content = network::get_content(configure_args.input_source)
                    .await
                    .context(error::NetworkGetContentSnafu)?;
                network::configure(&args.socket_path, content)
                    .await
                    .context(error::NetworkConfigureSnafu)?;
            }
        },

        Subcommand::Reboot(_reboot) => {
            reboot::reboot(&args.socket_path)
                .await
                .context(error::RebootSnafu)?;
        }

        Subcommand::Set(set) => {
            let settings = match set {
                SetArgs::Simple(simple) => {
                    trace!("User supplied Key Value settings {simple:#?}");
                    // Construct the Key Pair struct.
                    let set_key_pair = SetKeyPairSettings {
                        request_payload: simple,
                    };
                    let settings_string =
                        serde_json::to_string(&set_key_pair).context(error::SerializeSnafu)?;
                    SettingsInput::KeyPair(settings_string)
                }
                SetArgs::Json(json) => {
                    trace!("User supplied Json settings {json:#?}");
                    // Convert JSON Value to a string.
                    SettingsInput::Json(json.to_string())
                }
            };

            set::set(&args.socket_path, settings)
                .await
                .context(error::SetSnafu)?;
        }

        Subcommand::Update(subcommand) => match subcommand {
            UpdateSubcommand::Check(_check) => {
                check(&args).await?;
            }

            UpdateSubcommand::Apply(apply) => {
                if apply.check {
                    let output = check(&args).await?;
                    // Exit early if no update is required, either because none is available or one
                    // is already applied and ready.
                    if !update::required(&output) {
                        return Ok(());
                    }
                }

                update::apply(&args.socket_path)
                    .await
                    .context(error::UpdateApplySnafu)?;

                // If the user requested it, and if we applied an update, reboot.  (update::apply
                // will fail if no update was available or it couldn't apply the update.)
                if apply.reboot {
                    reboot::reboot(&args.socket_path)
                        .await
                        .context(error::RebootSnafu)?;
                } else {
                    info!("Update has been applied and will take effect on next reboot.");
                }
            }

            UpdateSubcommand::Cancel(_cancel) => {
                update::cancel(&args.socket_path)
                    .await
                    .context(error::UpdateCancelSnafu)?;
            }
        },

        Subcommand::Report(subcommand) => match subcommand {
            ReportSubcommand::Cis(cis_args) => {
                let body = report::get_cis_report(
                    &args.socket_path,
                    "bottlerocket",
                    cis_args.format,
                    cis_args.level,
                )
                .await
                .context(error::ReportSnafu)?;

                if !body.is_empty() {
                    print!("{body}");
                }
            }

            ReportSubcommand::CisK8s(cis_args) => {
                let body = report::get_cis_report(
                    &args.socket_path,
                    "kubernetes",
                    cis_args.format,
                    cis_args.level,
                )
                .await
                .context(error::ReportSnafu)?;

                if !body.is_empty() {
                    print!("{body}");
                }
            }

            ReportSubcommand::Fips(fips_args) => {
                let body = report::get_fips_report(&args.socket_path, fips_args.format)
                    .await
                    .context(error::ReportSnafu)?;

                if !body.is_empty() {
                    print!("{body}");
                }
            }
        },

        Subcommand::EphemeralStorage(subcommand) => match subcommand {
            EphemeralStorageSubcommand::Init(cfg_args) => {
                ephemeral_storage::initialize(
                    &args.socket_path,
                    cfg_args.filesystem,
                    cfg_args.disks,
                    cfg_args.ebs_volumes,
                    cfg_args.prefer,
                )
                .await
                .context(error::EphemeralStorageSnafu)?;
            }
            EphemeralStorageSubcommand::Bind(bind_args) => {
                ephemeral_storage::bind(&args.socket_path, bind_args.targets)
                    .await
                    .context(error::EphemeralStorageSnafu)?;
            }
            EphemeralStorageSubcommand::ListDisks(bind_args) => {
                let body = ephemeral_storage::list_disks(&args.socket_path, bind_args.format)
                    .await
                    .context(error::EphemeralStorageSnafu)?;
                if !body.is_empty() {
                    print!("{body}");
                }
            }
            EphemeralStorageSubcommand::ListEbsVolumes(bind_args) => {
                let body = ephemeral_storage::list_ebs_volumes(&args.socket_path, bind_args.format)
                    .await
                    .context(error::EphemeralStorageSnafu)?;
                if !body.is_empty() {
                    print!("{body}");
                }
            }
            EphemeralStorageSubcommand::ListDirs(bind_args) => {
                let body = ephemeral_storage::list_dirs(&args.socket_path, bind_args.format)
                    .await
                    .context(error::EphemeralStorageSnafu)?;
                if !body.is_empty() {
                    print!("{body}");
                }
            }
        },
    }

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

mod error {
    use apiclient::{
        apply, ephemeral_storage, exec, get, lockdown, network, reboot, report, set, update,
    };
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub(crate) enum Error {
        #[snafu(display("Failed to apply settings: {}", source))]
        Apply { source: apply::Error },

        #[snafu(display("Failed to exec: {}", source))]
        Exec { source: exec::Error },

        #[snafu(display("Failed to get settings: {}", source))]
        Get { source: get::Error },

        #[snafu(display("Logger setup error: {}", source))]
        Logger { source: log::SetLoggerError },

        #[snafu(display("Failed to lockdown: {}", source))]
        Lockdown { source: lockdown::Error },
        #[snafu(display("Failed to get network configuration content: {}", source))]
        NetworkGetContent { source: network::Error },

        #[snafu(display("Failed to configure network: {}", source))]
        NetworkConfigure { source: network::Error },

        #[snafu(display("Failed to reboot: {}", source))]
        Reboot { source: reboot::Error },

        #[snafu(display("Failed {} request to '{}': {}", method, uri, source))]
        Request {
            method: String,
            uri: String,
            #[snafu(source(from(apiclient::Error, Box::new)))]
            source: Box<apiclient::Error>,
        },

        #[snafu(display("Failed to get report: {}", source))]
        Report { source: report::Error },

        #[snafu(display("Unable to serialize data: {}", source))]
        Serialize { source: serde_json::Error },

        #[snafu(display("Failed to change settings: {}", source))]
        Set { source: set::Error },

        #[snafu(display("Failed to apply update: {}", source))]
        UpdateApply { source: update::Error },

        #[snafu(display("Failed to cancel update: {}", source))]
        UpdateCancel { source: update::Error },

        #[snafu(display("Failed to check for updates: {}", source))]
        UpdateCheck { source: update::Error },

        #[snafu(display("Failed to initialize ephemeral storage: {}", source))]
        EphemeralStorage { source: ephemeral_storage::Error },
    }
}
type Result<T> = std::result::Result<T, error::Error>;
#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    // Helper macro for creating global Args with consistent formatting
    macro_rules! global_args {
        // Create Args with specified log_level and socket_path
        ($log_level:expr, $socket_path:expr) => {
            Args {
                log_level: $log_level,
                socket_path: $socket_path.to_string(),
            }
        };
        // Create Args with just log_level (using default socket_path)
        ($log_level:expr) => {
            Args {
                log_level: $log_level,
                socket_path: constants::API_SOCKET.to_string(),
            }
        };
    }

    // Helper macro for creating exec subcommand
    macro_rules! exec_cmd {
        // For Exec subcommand with target, command args, and tty option
        ($target:expr, [$($arg:expr),*], $tty:expr) => {
            Subcommand::Exec(ExecArgs {
                target: $target.to_string(),
                command: vec![$($arg.into()),*],
                tty: $tty,
            })
        };
    }

    fn parse_command_line(cmd_str: &str) -> (Args, Subcommand) {
        let args: Vec<String> = cmd_str.split_whitespace().map(|s| s.to_string()).collect();
        parse_args(args.into_iter())
    }

    #[test_case("apiclient exec admin -- sheltie df -h",
        global_args!(LevelFilter::Info),
        exec_cmd!("admin", ["sheltie", "df", "-h"], None);
        "exec with separator")]
    #[test_case("apiclient exec admin sheltie df",
        global_args!(LevelFilter::Info),
        exec_cmd!("admin", ["sheltie", "df"], None);
        "exec without separator")]
    #[test_case("apiclient exec -t admin -- sheltie df -h",
        global_args!(LevelFilter::Info),
        exec_cmd!("admin", ["sheltie", "df", "-h"], Some(true));
        "exec with tty and separator")]
    #[test_case("apiclient exec -T admin sheltie df",
        global_args!(LevelFilter::Info),
        exec_cmd!("admin", ["sheltie", "df"], Some(false));
        "exec with no-tty")]
    fn test_exec_parsing(cmd_str: &str, expected_args: Args, expected_subcommand: Subcommand) {
        let (global_args, subcommand) = parse_command_line(cmd_str);

        // Check the global arguments match what we expect
        assert_eq!(global_args.log_level, expected_args.log_level);
        assert_eq!(global_args.socket_path, expected_args.socket_path);

        // Check the subcommand matches what we expect
        match (&subcommand, &expected_subcommand) {
            (Subcommand::Exec(actual), Subcommand::Exec(expected)) => {
                assert_eq!(actual.target, expected.target);
                assert_eq!(actual.tty, expected.tty);
                assert_eq!(actual.command, expected.command);
            }
            _ => panic!("Expected Exec subcommand: {expected_subcommand:?}, got: {subcommand:?}"),
        }
    }

    #[test_case("apiclient -v exec admin -- sheltie df -h",
        global_args!(LevelFilter::Debug),
        exec_cmd!("admin", ["sheltie", "df", "-h"], None);
        "verbose flag with exec as global arg")]
    #[test_case("apiclient --log-level error exec admin sheltie df",
        global_args!(LevelFilter::Error),
        exec_cmd!("admin", ["sheltie", "df"], None);
        "log level with exec")]
    #[test_case("apiclient exec admin -- sheltie -v df -h",
        global_args!(LevelFilter::Info),
        exec_cmd!("admin", ["sheltie", "-v", "df", "-h"], None);
        "verbose flag after separator as command arg")]
    #[test_case("apiclient -v exec admin -- sheltie -v df -h",
        global_args!(LevelFilter::Debug),
        exec_cmd!("admin", ["sheltie", "-v", "df", "-h"], None);
        "verbose flag in both global and command positions")]
    fn test_args_parsing_with_separator(
        cmd_str: &str,
        expected_args: Args,
        expected_subcommand: Subcommand,
    ) {
        let (global_args, subcommand) = parse_command_line(cmd_str);

        // Check the global arguments match what we expect
        assert_eq!(
            global_args.log_level, expected_args.log_level,
            "Global log level should match"
        );
        assert_eq!(
            global_args.socket_path, expected_args.socket_path,
            "Socket path should match"
        );

        // Check the subcommand matches what we expect
        match (&subcommand, &expected_subcommand) {
            (Subcommand::Exec(actual), Subcommand::Exec(expected)) => {
                assert_eq!(actual.target, expected.target, "Target should match");
                assert_eq!(actual.tty, expected.tty, "TTY setting should match");
                assert_eq!(actual.command, expected.command, "Command should match");
            }
            _ => panic!("Expected Exec subcommand: {expected_subcommand:?}, got: {subcommand:?}"),
        }
    }

    #[test]
    fn test_get_with_exclude() {
        let (_, subcommand) =
            parse_command_line("apiclient get settings. --exclude settings.network");
        match subcommand {
            Subcommand::Get(GetArgs::Prefixes {
                include,
                exclude,
                canonicalize,
            }) => {
                assert_eq!(include, vec!["settings."]);
                assert_eq!(exclude, vec!["settings.network"]);
                assert_eq!(canonicalize, false);
            }
            _ => panic!("Expected Get with Prefixes"),
        }
    }

    #[test]
    fn test_get_with_multiple_excludes() {
        let (_, subcommand) = parse_command_line(
            "apiclient get settings. --exclude settings.network --exclude settings.host-containers",
        );
        match subcommand {
            Subcommand::Get(GetArgs::Prefixes {
                include,
                exclude,
                canonicalize,
            }) => {
                assert_eq!(include, vec!["settings."]);
                assert_eq!(
                    exclude,
                    vec!["settings.network", "settings.host-containers"]
                );
                assert_eq!(canonicalize, false);
            }
            _ => panic!("Expected Get with Prefixes"),
        }
    }

    #[test]
    fn test_get_empty() {
        let (_, subcommand) = parse_command_line("apiclient get");
        match subcommand {
            Subcommand::Get(GetArgs::Prefixes {
                include,
                exclude,
                canonicalize,
            }) => {
                assert_eq!(include, vec!["os.", "settings."]);
                assert_eq!(exclude, Vec::<String>::new());
                assert_eq!(canonicalize, false);
            }
            _ => panic!("Expected Get with Prefixes"),
        }
    }

    #[test]
    fn test_get_canonicalize() {
        let (_, subcommand) = parse_command_line("apiclient get --canonicalize");
        match subcommand {
            Subcommand::Get(GetArgs::Prefixes {
                include,
                exclude,
                canonicalize,
            }) => {
                assert_eq!(include, vec!["os.", "settings."]);
                assert_eq!(exclude, Vec::<String>::new());
                assert_eq!(canonicalize, true);
            }
            _ => panic!("Expected Get with Prefixes"),
        }
    }

    #[test]
    fn test_get_exclude_only() {
        let (_, subcommand) = parse_command_line("apiclient get --exclude settings.network");
        match subcommand {
            Subcommand::Get(GetArgs::Prefixes {
                include,
                exclude,
                canonicalize,
            }) => {
                assert_eq!(include, vec!["os.", "settings."]);
                assert_eq!(exclude, vec!["settings.network"]);
                assert_eq!(canonicalize, false);
            }
            _ => panic!("Expected Get with Prefixes"),
        }
    }

    #[test]
    fn test_network_configure_parsing_stdin() {
        // Test that network configure with no arguments defaults to stdin
        let (global_args, subcommand) = parse_command_line("apiclient network configure");

        // Test global arguments match expected defaults
        assert_eq!(global_args.log_level, LevelFilter::Info);
        assert_eq!(global_args.socket_path, "/run/api.sock");

        // Test network configure subcommand with no input source (stdin)
        match subcommand {
            Subcommand::Network(NetworkSubcommand::Configure(configure_args)) => {
                assert_eq!(configure_args.input_source, None);
            }
            _ => panic!("Expected Network::Configure subcommand, got: {subcommand:?}"),
        }
    }

    #[test_case("apiclient network configure file:///tmp/net.toml",
        global_args!(LevelFilter::Info),
        "file:///tmp/net.toml";
        "network configure with file URI")]
    #[test_case("apiclient network configure base64:dmVyc2lvbiA9IDIKCltldGgwXQpkaGNwNCA9IHRydWU=",
        global_args!(LevelFilter::Info),
        "base64:dmVyc2lvbiA9IDIKCltldGgwXQpkaGNwNCA9IHRydWU=";
        "network configure with base64 URI")]
    #[test_case("apiclient -v network configure file:///tmp/net.toml",
        global_args!(LevelFilter::Debug),
        "file:///tmp/net.toml";
        "verbose flag with network configure")]
    #[test_case("apiclient --log-level error network configure base64:test",
        global_args!(LevelFilter::Error),
        "base64:test";
        "log level with network configure")]
    #[test_case("apiclient --socket-path /tmp/custom.sock network configure file:///etc/net.toml",
        global_args!(LevelFilter::Info, "/tmp/custom.sock"),
        "file:///etc/net.toml";
        "custom socket path with network configure")]
    fn test_network_configure_parsing(
        cmd_str: &str,
        expected_args: Args,
        expected_input_source: &str,
    ) {
        // Given a command line string for network configure
        let (global_args, subcommand) = parse_command_line(cmd_str);

        // Test global arguments match expected values
        assert_eq!(global_args.log_level, expected_args.log_level);
        assert_eq!(global_args.socket_path, expected_args.socket_path);

        // Test network configure subcommand and extract input source
        match subcommand {
            Subcommand::Network(NetworkSubcommand::Configure(configure_args)) => {
                assert_eq!(
                    configure_args.input_source,
                    Some(expected_input_source.to_string())
                );
            }
            _ => panic!("Expected Network::Configure subcommand, got: {subcommand:?}"),
        }
    }
}
