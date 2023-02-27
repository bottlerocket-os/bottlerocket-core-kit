/*!
  prairiedog is a tool for providing kernel boot related support in Bottlerocket.

It does the following:
  - _digs_ to find the active boot partition and mounts it in /boot
  - loads the crash kernel from /boot
  - creates memory dumps when the kernel panics
  - generates kernel boot config from settings
  - generates settings from the existing kernel boot config file

*/

#[macro_use]
extern crate log;

use crate::bootconfig::{generate_boot_config, generate_boot_settings, is_reboot_required};
use crate::error::Result;
use argh::FromArgs;
use simplelog::{Config as LogConfig, LevelFilter, SimpleLogger, WriteLogger};
use snafu::{ensure, ResultExt};
use std::ffi::OsStr;
use std::fs::{self, File};
use std::path::Path;
use std::process::{self, Command};
use std::thread;
use std::time::Duration;

mod bootconfig;
mod error;
mod initrd;

// Kdump related binary paths
const MAKEDUMPFILE_PATH: &str = "/sbin/makedumpfile";
const KEXEC_PATH: &str = "/sbin/kexec";

// Mount points created prairiedog
const BOOT_MOUNT_PATH: &str = "/boot";

// Files generated by prairiedog
const KDUMP_LOGS_PATH: &str = "/var/log/kdump";
const LOG_FILE: &str = "prairiedog.log";
const DMESG_DUMP_FILE: &str = "dmesg.log";
const KDUMP_FILE: &str = "vmcore.dump";

// Stores how much memory was allocated for the crash kernel
const KEXEC_CRASH_SIZE: &str = "/sys/kernel/kexec_crash_size";
// Enables/disables the kexec_load/kexec_file_load syscalls
const KEXEC_LOAD_DISABLED: &str = "/proc/sys/kernel/kexec_load_disabled";
// Crash kernel additional CMD line parameters
const KEXEC_CMD_LINE: &str = "maxcpus=1 systemd.unit='capture-kernel-dump.service' nr_cpus=1 \
                              swiotlb=noforce cma=0 reset_devices cgroup_disable=memory \
                              udev.children-max=2 panic=10 nvme_core.admin_timeout=20 swiotlb=1";

// Used to pass None to nix::mount::mount
const NONE: Option<&'static [u8]> = None;

/// Stores arguments
#[derive(FromArgs, PartialEq, Debug)]
struct Args {
    #[argh(option, default = "LevelFilter::Info", short = 'l')]
    /// log-level trace|debug|info|warn|error
    log_level: LevelFilter,
    #[argh(option, default = "constants::API_SOCKET.to_string()", short = 's')]
    /// socket-path path to apiserver socket
    socket_path: String,
    #[argh(subcommand)]
    subcommand: Subcommand,
}

/// Stores the subcommand to be executed
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum Subcommand {
    PrepareBoot(PrepareBootArgs),
    CaptureDump(CaptureDumpArgs),
    LoadCrashKernel(LoadCrashKernelArgs),
    GenerateBootConfig(GenerateBootConfigArgs),
    GenerateBootSettings(GenerateBootSettingsArgs),
    RebootIfRequired(RebootIfRequiredArgs),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "prepare-boot")]
/// Mounts the active boot partition on /boot
struct PrepareBootArgs {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "capture-dump")]
/// Captures the dmesg and kdump dumps from the memory image
struct CaptureDumpArgs {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "load-crash-kernel")]
/// Loads the crash kernel with kexec
struct LoadCrashKernelArgs {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "generate-boot-config")]
/// Generate boot configuration from settings
pub(crate) struct GenerateBootConfigArgs {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "generate-boot-settings")]
/// Generate boot settings from existing boot configuration
pub(crate) struct GenerateBootSettingsArgs {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "reboot-if-required")]
/// Reboot the host if reboot-to-reconcile is set and the boot settings changed
struct RebootIfRequiredArgs {}

/// Wrapper around process::Command that adds error checking.
fn command<I, S>(bin_path: &str, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(bin_path);
    command.args(args);
    let output = command
        .output()
        .context(error::ExecutionFailureSnafu { command })?;

    trace!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    trace!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    ensure!(
        output.status.success(),
        error::CommandFailureSnafu { bin_path, output }
    );

    Ok(())
}

/// Dumps the memory image in `/proc/vmcore`, which is created when the kernel crashes
fn capture_dump() -> Result<()> {
    let kdump_file_path = format!("{}/{}", KDUMP_LOGS_PATH, KDUMP_FILE);
    let dmesg_file_path = format!("{}/{}", KDUMP_LOGS_PATH, DMESG_DUMP_FILE);

    // Delete previous dumps, if they exist
    if Path::new(&kdump_file_path).exists() {
        info!("Deleting existing memory dump");
        fs::remove_file(&kdump_file_path).context(error::RemoveFileSnafu {
            path: &kdump_file_path,
        })?;
    }

    if Path::new(&dmesg_file_path).exists() {
        info!("Deleting existing dmesg dump");
        fs::remove_file(&dmesg_file_path).context(error::RemoveFileSnafu {
            path: &dmesg_file_path,
        })?;
    }

    info!("Generating dmesg dump");
    // --dump-dmesg generates a dump with only dmesg logs
    command(
        MAKEDUMPFILE_PATH,
        [
            "--dump-dmesg",
            "--message-level",
            "4",
            "/proc/vmcore",
            dmesg_file_path.as_ref(),
        ],
    )?;

    info!("Generating memory dump");
    // Extract kdump-compressed dump file, without empty pages, using
    // zlib compression
    command(
        MAKEDUMPFILE_PATH,
        [
            "-c",
            "--message-level",
            "4",
            "-d",
            "31",
            "/proc/vmcore",
            kdump_file_path.as_ref(),
        ],
    )?;

    Ok(())
}

// Mounts the active boot partition
fn prepare_boot() -> Result<()> {
    // Get the current partitions state
    let state = signpost::State::load().context(error::LoadStateSnafu)?;
    let boot_partition_path = &state.active_set().boot;
    let flags = nix::mount::MsFlags::MS_NOSUID
        | nix::mount::MsFlags::MS_NODEV
        | nix::mount::MsFlags::MS_NOEXEC
        | nix::mount::MsFlags::MS_NOATIME
        | nix::mount::MsFlags::MS_RDONLY;

    info!(
        "Mounting {} in {}",
        boot_partition_path.display(),
        BOOT_MOUNT_PATH
    );
    // Mount the active boot partition in /boot
    nix::mount::mount(
        Some(boot_partition_path),
        BOOT_MOUNT_PATH,
        Some("ext4"),
        flags,
        NONE,
    )
    .context(error::MountSnafu {
        path: BOOT_MOUNT_PATH,
    })?;

    // Make the mount point private so new mount namespaces don't have
    // access to it. This has to be set as a different call otherwise
    // the mount syscall returns `EINVAL`
    nix::mount::mount(
        NONE,
        BOOT_MOUNT_PATH,
        NONE,
        nix::mount::MsFlags::MS_PRIVATE,
        NONE,
    )
    .context(error::SetupMountSnafu {
        path: BOOT_MOUNT_PATH,
    })?;

    Ok(())
}

/// Loads the crash kernel using kexec-tools
fn load_crash_kernel() -> Result<()> {
    let kexec_crash_size_path = Path::new(KEXEC_CRASH_SIZE);
    let kexec_crash_size = fs::read(kexec_crash_size_path).context(error::ReadFileSnafu {
        path: kexec_crash_size_path,
    })?;
    let memory_allocated = String::from_utf8_lossy(&kexec_crash_size);

    // We provide a more useful message when no memory was reserved for the crash kernel. Exit
    // gracefully since the user could have decided to use a tiny host, and the system shouldn't be
    // in "degraded" state
    if memory_allocated.trim() == "0" {
        info!("No memory assigned for crash kernel. If you want to use kdump, please make sure the host has at least 2GB of memory");
        return Ok(());
    }

    let kexec_load_disabled_path = Path::new(KEXEC_LOAD_DISABLED);
    let kexec_load_disabled_value =
        fs::read(kexec_load_disabled_path).context(error::ReadFileSnafu {
            path: kexec_load_disabled_path,
        })?;
    let kexec_load_disabled = String::from_utf8_lossy(&kexec_load_disabled_value).trim() == "1";

    // We provide a more useful message when `kexec_load_disabled` is set to 1
    if kexec_load_disabled {
        return error::KexecLoadDisabledSnafu.fail();
    }

    // Conditionally add `irqpoll` depending on the architecture
    let kexec_cmd_line = if cfg!(target_arch = "x86_64") {
        String::from(KEXEC_CMD_LINE) + " irqpoll"
    } else {
        String::from(KEXEC_CMD_LINE)
    };

    info!("Loading crash kernel");
    // Load the panic kernel from `BOOT_MOUNT_PATH`, using the kexec_file_load syscall.
    // We reuse the cmd line in /proc/cmdline to boot the crash kernel, and we start
    // a specific systemd service instead of the default target.
    command(
        KEXEC_PATH,
        [
            "-ps",
            "--reuse-cmdline",
            "--append",
            kexec_cmd_line.as_ref(),
            format!("{}/vmlinuz", BOOT_MOUNT_PATH).as_ref(),
        ],
    )?;

    info!("Crash kernel loaded");
    Ok(())
}

async fn reboot_if_required<P>(socket_path: P) -> Result<()>
where
    P: AsRef<Path>,
{
    if is_reboot_required(socket_path).await? {
        info!("Boot settings changed and require a reboot to take effect. Initiating reboot...");
        command("/usr/bin/systemctl", ["reboot"])?;
        // The "systemctl reboot" process will not block until the host does
        // reboot, but return as soon as the request either failed or the job
        // to start the systemd reboot.target and its dependencies have been
        // enqueued. As the shutdown.target that is being pulled in conflicts
        // with most anything else, the other jobs needed to boot the host
        // will be cancelled and the boot will not proceed.
        //
        // The above is subtle, so slowly spin here until systemd kills this
        // prairiedog process as part of the host shutting down by sending it
        // SIGTERM. This serves as a more obvious line of defense against the
        // boot proceeding past a required reboot.
        loop {
            thread::sleep(Duration::from_secs(5));
            info!("Still waiting for the host to be rebooted...");
        }
    } else {
        info!("No reboot required");
    }

    Ok(())
}

fn setup_logger(args: &Args) -> Result<()> {
    match args.subcommand {
        // Write the logs to a file while capturing dumps, since the journal isn't available
        Subcommand::CaptureDump(_) => {
            let log_file_path = Path::new(KDUMP_LOGS_PATH).join(LOG_FILE);
            let log_file = File::create(&log_file_path).context(error::WriteFileSnafu {
                path: log_file_path,
            })?;

            WriteLogger::init(args.log_level, LogConfig::default(), log_file)
                .context(error::LoggerSnafu)?;
        }
        // SimpleLogger will send errors to stderr and anything less to stdout.
        _ => {
            SimpleLogger::init(args.log_level, LogConfig::default()).context(error::LoggerSnafu)?
        }
    }

    Ok(())
}

async fn run() -> Result<()> {
    let args: Args = argh::from_env();
    setup_logger(&args)?;

    match args.subcommand {
        Subcommand::CaptureDump(_) => capture_dump(),
        Subcommand::PrepareBoot(_) => prepare_boot(),
        Subcommand::LoadCrashKernel(_) => load_crash_kernel(),
        Subcommand::GenerateBootConfig(_) => generate_boot_config(args.socket_path).await,
        Subcommand::GenerateBootSettings(_) => generate_boot_settings().await,
        Subcommand::RebootIfRequired(_) => reboot_if_required(args.socket_path).await,
    }
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        error!("{}", e);
        process::exit(1);
    }
}
