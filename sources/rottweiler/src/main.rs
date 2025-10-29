/*!
# Introduction

*rottweiler* is Bottlerocket's storage encryption helper. It provides a unified
interface for encrypting and managing encrypted storage resources including:

- Block devices (using LUKS)
- Directories (using fscrypt)
- TPM PCR measurements

## Commands

### Key Management
- `generate-key <key-id>` - Generate an encryption key

### Block Device Operations
- `encrypt block-device <path> <key-id>` - Encrypt a block device using LUKS
- `attach block-device <path> <key-id>` - Attach an encrypted block device
- `detach block-device <path>` - Detach an encrypted block device
- `resize block-device <path> <key-id>` - Resize a LUKS block device
- `check block-device <path> encrypted|unencrypted` - Check block device encryption state

### Directory Operations
- `encrypt directory <path> <key-id>` - Encrypt a directory using fscrypt
- `lock directory <path>` - Lock an encrypted directory (remove key)
- `unlock directory <path> <key-id>` - Unlock an encrypted directory (add key)
- `check directory <path> encrypted|unencrypted` - Check directory encryption state

### TPM Measurement Operations
- `measure settings` - Measure OS settings into PCR 8
- `measure kernel-command-line` - Measure kernel command line into PCR 9
- `measure pcrphase <phase>` - Measure boot phase into PCR 11
  - Valid phases: `sysinit`, `preconfigured`, `configured`, `ready`, `shutdown`, `final`

## Aliases

For convenience, the following aliases are supported:
- `dir` can be used instead of `directory`
- `bdev` can be used instead of `block-device`
*/

use argh::FromArgs;
use snafu::Whatever;
use std::path::{Path, PathBuf};

mod block_device;
mod directory;
mod fscrypt;
mod key;
mod measure;
mod mount_point;
mod system;

type Result<T> = std::result::Result<T, Whatever>;

#[snafu::report]
fn main() -> Result<()> {
    // Support aliases: "dir" -> "directory", "bdev" -> "block-device"
    // Only replace in resource-type subcommand positions
    let mut args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 {
        let verb = args.get(1).map(|s| s.as_str());
        let resource_pos = match verb {
            Some("encrypt" | "attach" | "detach" | "resize" | "lock" | "unlock" | "check") => {
                Some(2)
            }
            _ => None,
        };
        if let Some(pos) = resource_pos
            && let Some(arg) = args.get_mut(pos)
        {
            if arg == "dir" {
                *arg = "directory".to_string();
            } else if arg == "bdev" {
                *arg = "block-device".to_string();
            }
        }
    }
    let args: Args = match Args::from_args(
        &[args[0].as_str()],
        &args[1..].iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    ) {
        Ok(args) => args,
        Err(early_exit) => {
            println!("{}", early_exit.output);
            std::process::exit(match early_exit.status {
                Ok(()) => 0,
                Err(()) => 1,
            });
        }
    };

    match args.command {
        Command::GenerateKey(cmd) => key::generate(cmd.key_id),
        Command::Encrypt(cmd) => match cmd.resource {
            EncryptResource::BlockDevice(cmd) => block_device::encrypt(cmd.path, cmd.key_id),
            EncryptResource::Directory(cmd) => directory::encrypt(cmd.path, cmd.key_id),
        },
        Command::Attach(cmd) => match cmd.resource {
            AttachResource::BlockDevice(cmd) => block_device::attach(cmd.path, cmd.key_id),
        },
        Command::Detach(cmd) => match cmd.resource {
            DetachResource::BlockDevice(cmd) => block_device::detach(cmd.path),
        },
        Command::Resize(cmd) => match cmd.resource {
            ResizeResource::BlockDevice(cmd) => block_device::resize(cmd.path, cmd.key_id),
        },
        Command::Lock(cmd) => match cmd.resource {
            LockResource::Directory(cmd) => directory::lock(cmd.path),
        },
        Command::Unlock(cmd) => match cmd.resource {
            UnlockResource::Directory(cmd) => directory::unlock(cmd.path, cmd.key_id),
        },
        Command::Check(cmd) => match cmd.resource {
            CheckResource::BlockDevice(cmd) => {
                let path = cmd.path;
                match cmd.state {
                    CheckBlockDeviceState::Encrypted(_) => handle_check(
                        block_device::is_encrypted(path.clone())?,
                        "block device",
                        &path,
                        true,
                        "encrypted",
                    ),
                    CheckBlockDeviceState::Unencrypted(_) => handle_check(
                        block_device::is_encrypted(path.clone())?,
                        "block device",
                        &path,
                        false,
                        "encrypted",
                    ),
                }
            }
            CheckResource::Directory(cmd) => {
                let path = cmd.path;
                match cmd.state {
                    CheckDirectoryState::Encrypted(_) => handle_check(
                        directory::is_encrypted(path.clone())?,
                        "directory",
                        &path,
                        true,
                        "encrypted",
                    ),
                    CheckDirectoryState::Unencrypted(_) => handle_check(
                        directory::is_encrypted(path.clone())?,
                        "directory",
                        &path,
                        false,
                        "encrypted",
                    ),
                }
            }
        },
        Command::Measure(cmd) => match cmd.resource {
            MeasureResource::Settings(_) => measure::os_settings(),
            MeasureResource::KernelCommandLine(_) => measure::kernel_command_line(),
            MeasureResource::Pcrphase(cmd) => measure::pcrphase(&cmd.phase.to_string()),
        },
    }
}

/// Handle status checks, printing the result and exiting with appropriate code.
///
/// Returns Ok(()) if the actual state matches the expected state, otherwise exits with code 1.
fn handle_check(
    actual: bool,
    resource_type: &str,
    path: &Path,
    expected: bool,
    state_name: &str,
) -> Result<()> {
    let state = if actual {
        state_name
    } else {
        &format!("not {}", state_name)
    };
    println!("{} '{}' is {}", resource_type, path.display(), state);

    match (actual, expected) {
        (true, true) | (false, false) => Ok(()),
        _ => std::process::exit(1),
    }
}

#[derive(FromArgs)]
/// Storage encryption helper
struct Args {
    #[argh(subcommand)]
    command: Command,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Command {
    GenerateKey(GenerateKeyCmd),
    Encrypt(EncryptCmd),
    Attach(AttachCmd),
    Detach(DetachCmd),
    Resize(ResizeCmd),
    Lock(LockCmd),
    Unlock(UnlockCmd),
    Check(CheckCmd),
    Measure(MeasureCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "generate-key")]
/// Generate an encryption key
struct GenerateKeyCmd {
    #[argh(positional)]
    key_id: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "encrypt")]
/// Encrypt a resource
struct EncryptCmd {
    #[argh(subcommand)]
    resource: EncryptResource,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum EncryptResource {
    BlockDevice(EncryptBlockDeviceCmd),
    Directory(EncryptDirectoryCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "block-device")]
/// Encrypt a block device using LUKS
struct EncryptBlockDeviceCmd {
    #[argh(positional)]
    path: PathBuf,

    #[argh(positional)]
    key_id: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "directory")]
/// Encrypt a directory using fscrypt
struct EncryptDirectoryCmd {
    #[argh(positional)]
    path: PathBuf,

    #[argh(positional)]
    key_id: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "attach")]
/// Attach an encrypted resource
struct AttachCmd {
    #[argh(subcommand)]
    resource: AttachResource,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum AttachResource {
    BlockDevice(AttachBlockDeviceCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "block-device")]
/// Attach an encrypted block device
struct AttachBlockDeviceCmd {
    #[argh(positional)]
    path: PathBuf,

    #[argh(positional)]
    key_id: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "detach")]
/// Detach an encrypted resource
struct DetachCmd {
    #[argh(subcommand)]
    resource: DetachResource,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum DetachResource {
    BlockDevice(DetachBlockDeviceCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "block-device")]
/// Detach an encrypted block device
struct DetachBlockDeviceCmd {
    #[argh(positional)]
    path: PathBuf,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "resize")]
/// Resize a resource
struct ResizeCmd {
    #[argh(subcommand)]
    resource: ResizeResource,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum ResizeResource {
    BlockDevice(ResizeBlockDeviceCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "block-device")]
/// Resize a LUKS block device
struct ResizeBlockDeviceCmd {
    #[argh(positional)]
    path: PathBuf,

    #[argh(positional)]
    key_id: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "lock")]
/// Lock a resource
struct LockCmd {
    #[argh(subcommand)]
    resource: LockResource,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum LockResource {
    Directory(LockDirectoryCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "directory")]
/// Lock an encrypted directory (remove key)
struct LockDirectoryCmd {
    #[argh(positional)]
    path: PathBuf,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "unlock")]
/// Unlock a resource
struct UnlockCmd {
    #[argh(subcommand)]
    resource: UnlockResource,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum UnlockResource {
    Directory(UnlockDirectoryCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "directory")]
/// Unlock an encrypted directory (add key)
struct UnlockDirectoryCmd {
    #[argh(positional)]
    path: PathBuf,

    #[argh(positional)]
    key_id: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "check")]
/// Check resource state
struct CheckCmd {
    #[argh(subcommand)]
    resource: CheckResource,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum CheckResource {
    BlockDevice(CheckBlockDeviceCmd),
    Directory(CheckDirectoryCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "block-device")]
/// Check block device state
struct CheckBlockDeviceCmd {
    #[argh(positional)]
    path: PathBuf,

    #[argh(subcommand)]
    state: CheckBlockDeviceState,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum CheckBlockDeviceState {
    Encrypted(CheckBlockDeviceEncryptedCmd),
    Unencrypted(CheckBlockDeviceUnencryptedCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "encrypted")]
/// Check if encrypted
struct CheckBlockDeviceEncryptedCmd {}

#[derive(FromArgs)]
#[argh(subcommand, name = "unencrypted")]
/// Check if unencrypted
struct CheckBlockDeviceUnencryptedCmd {}

#[derive(FromArgs)]
#[argh(subcommand, name = "directory")]
/// Check directory state
struct CheckDirectoryCmd {
    #[argh(positional)]
    path: PathBuf,

    #[argh(subcommand)]
    state: CheckDirectoryState,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum CheckDirectoryState {
    Encrypted(CheckDirectoryEncryptedCmd),
    Unencrypted(CheckDirectoryUnencryptedCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "encrypted")]
/// Check if encrypted
struct CheckDirectoryEncryptedCmd {}

#[derive(FromArgs)]
#[argh(subcommand, name = "unencrypted")]
/// Check if unencrypted
struct CheckDirectoryUnencryptedCmd {}

#[derive(FromArgs)]
#[argh(subcommand, name = "measure")]
/// Measure data into TPM PCR
struct MeasureCmd {
    #[argh(subcommand)]
    resource: MeasureResource,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum MeasureResource {
    Settings(MeasureSettingsCmd),
    KernelCommandLine(MeasureKernelCommandLineCmd),
    Pcrphase(MeasurePcrphaseCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "settings")]
/// Measure OS settings into PCR
struct MeasureSettingsCmd {}

#[derive(FromArgs)]
#[argh(subcommand, name = "kernel-command-line")]
/// Measure kernel command line into PCR
struct MeasureKernelCommandLineCmd {}

#[derive(FromArgs)]
#[argh(subcommand, name = "pcrphase")]
/// Measure boot phase into PCR
struct MeasurePcrphaseCmd {
    #[argh(positional)]
    phase: Phase,
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    Sysinit,
    Preconfigured,
    Configured,
    Ready,
    Shutdown,
    Final,
}

impl std::str::FromStr for Phase {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "sysinit" => Ok(Phase::Sysinit),
            "preconfigured" => Ok(Phase::Preconfigured),
            "configured" => Ok(Phase::Configured),
            "ready" => Ok(Phase::Ready),
            "shutdown" => Ok(Phase::Shutdown),
            "final" => Ok(Phase::Final),
            _ => Err(format!(
                "invalid phase '{}', must be one of: sysinit, preconfigured, configured, ready, shutdown, final",
                s
            )),
        }
    }
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Phase::Sysinit => write!(f, "sysinit"),
            Phase::Preconfigured => write!(f, "preconfigured"),
            Phase::Configured => write!(f, "configured"),
            Phase::Ready => write!(f, "ready"),
            Phase::Shutdown => write!(f, "shutdown"),
            Phase::Final => write!(f, "final"),
        }
    }
}
