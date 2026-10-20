//! Boot-only zram swap setup for `prepare-swap.service`.
//!
//! An initialized device is never reset and swap is never turned off, so a changed size
//! applies after a reboot.
use bottlerocket_modeled_types::ZramSize;
use log::{info, warn};
use snafu::{ensure, OptionExt, ResultExt, Snafu};
use std::{fs, io, process::Command};

const MEMINFO: &str = "/proc/meminfo";
const DISKSIZE: &str = "/sys/block/zram0/disksize";
const DEVICE: &str = "/dev/zram0";
const DEFAULT_BYTES: u64 = 1 << 30;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Unable to read {path}: {source}"))]
    Read {
        path: &'static str,
        source: io::Error,
    },
    #[snafu(display("Invalid or missing MemTotal in /proc/meminfo"))]
    MemTotal,
    #[snafu(display(
        "Refusing to change an initialized zram device; reboot to apply the setting"
    ))]
    Initialized,
    #[snafu(display("Invalid zram disksize: {value}"))]
    DiskSize { value: String },
    #[snafu(display("Unable to write {path}: {source}"))]
    Write {
        path: &'static str,
        source: io::Error,
    },
    #[snafu(display("Unable to run {program}: {source}"))]
    Execute { program: String, source: io::Error },
    #[snafu(display("{program} failed: {output}"))]
    CommandFailed { program: String, output: String },
}
type Result<T> = std::result::Result<T, Error>;

trait System {
    fn read(&self, path: &'static str) -> io::Result<String>;
    fn write(&self, path: &'static str, value: &str) -> io::Result<()>;
    fn command(&self, program: &str, args: &[&str]) -> Result<()>;
}
struct Host;
impl System for Host {
    fn read(&self, path: &'static str) -> io::Result<String> {
        fs::read_to_string(path)
    }
    fn write(&self, path: &'static str, value: &str) -> io::Result<()> {
        fs::write(path, value)
    }
    fn command(&self, program: &str, args: &[&str]) -> Result<()> {
        let output = Command::new(program)
            .args(args)
            .output()
            .context(ExecuteSnafu { program })?;
        ensure!(
            output.status.success(),
            CommandFailedSnafu {
                program,
                output: format!(
                    "{}: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                ),
            }
        );
        Ok(())
    }
}

fn mem_total(meminfo: &str) -> Result<u64> {
    let mut values = meminfo.lines().filter_map(|line| {
        let mut fields = line.split_whitespace();
        (fields.next() == Some("MemTotal:")).then(|| {
            let kib = fields.next()?.parse::<u64>().ok()?;
            if fields.next() != Some("kB") || fields.next().is_some() || kib == 0 {
                return None;
            }
            kib.checked_mul(1024)
        })
    });
    let value = values.next().flatten().context(MemTotalSnafu)?;
    ensure!(values.next().is_none(), MemTotalSnafu);
    Ok(value)
}

/// The requested size in bytes, clamped to MemTotal.
fn capacity(size: Option<&ZramSize>, mem_total: u64) -> u64 {
    let bytes = size
        .map(|size| size.as_bytes(mem_total))
        .unwrap_or(DEFAULT_BYTES);
    if bytes > mem_total {
        warn!("Requested zram size {bytes} bytes exceeds MemTotal; clamping to {mem_total} bytes");
        return mem_total;
    }
    bytes
}

fn require_uninitialized(system: &impl System) -> Result<()> {
    match system.read(DISKSIZE) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::Read {
            path: DISKSIZE,
            source,
        }),
        Ok(value) => {
            let value = value.trim();
            let bytes = value.parse::<u64>().ok().context(DiskSizeSnafu { value })?;
            ensure!(bytes == 0, InitializedSnafu);
            Ok(())
        }
    }
}

fn prepare_with(system: &impl System, size: Option<&ZramSize>) -> Result<()> {
    let meminfo = system.read(MEMINFO).context(ReadSnafu { path: MEMINFO })?;
    let total = mem_total(&meminfo)?;
    let bytes = capacity(size, total);
    require_uninitialized(system)?;
    if bytes == 0 {
        info!("Zram swap disabled; no swap device initialized");
        return Ok(());
    }
    info!("Preparing zram swap with {bytes} bytes (MemTotal {total} bytes)");
    system.command("/usr/sbin/modprobe", &["zram"])?;
    // Set the size directly rather than with zramctl, which resets the device first. The
    // kernel refuses to resize an initialized device, so this never discards existing swap.
    system
        .write(DISKSIZE, &bytes.to_string())
        .context(WriteSnafu { path: DISKSIZE })?;
    system.command("/usr/sbin/mkswap", &["--uuid", "clear", DEVICE])?;
    system.command(
        "/usr/sbin/swapon",
        &["--discard", "--priority", "1024", DEVICE],
    )?;
    Ok(())
}

pub fn prepare(size: Option<&ZramSize>) -> Result<()> {
    prepare_with(&Host, size)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::cell::RefCell;
    use test_case::test_case;
    const MEM: &str = "MemFree: 42 kB\nMemTotal: 3936368 kB\n";
    const TOTAL: u64 = 3936368 * 1024;

    fn size(value: &str) -> ZramSize {
        ZramSize::try_from(value).unwrap()
    }

    #[test_case(None, 1 << 30; "default")]
    #[test_case(Some("512Mi"), 512 << 20; "mebibytes")]
    #[test_case(Some("2Gi"), 2 << 30; "gibibytes")]
    #[test_case(Some("25%"), TOTAL / 4; "percent")]
    #[test_case(Some("0"), 0; "zero")]
    #[test_case(Some("0%"), 0; "zero percent")]
    fn computes_capacity(value: Option<&str>, bytes: u64) {
        assert_eq!(capacity(value.map(size).as_ref(), TOTAL), bytes);
    }

    #[test_case(Some("1024Gi"), TOTAL; "above memtotal")]
    #[test_case(Some("18446744073709551615"), TOTAL; "u64 max")]
    #[test_case(Some("100%"), TOTAL; "all memory")]
    #[test_case(None, 512 << 20; "default on a small node")]
    fn clamps_to_memtotal(value: Option<&str>, mem_total: u64) {
        assert_eq!(capacity(value.map(size).as_ref(), mem_total), mem_total);
    }

    #[test]
    fn reads_memtotal() {
        assert_eq!(mem_total(MEM).unwrap(), TOTAL);
    }

    #[test_case(""; "empty")]
    #[test_case("MemTotal: 0 kB"; "zero")]
    #[test_case("MemTotal: -1 kB"; "negative")]
    #[test_case("MemTotal: 1 MB"; "MB unit")]
    #[test_case("MemTotal: 18446744073709551615 kB"; "overflow")]
    #[test_case("MemTotal: 1 kB\nMemTotal: 2 kB"; "duplicate")]
    fn rejects_bad_memtotal(text: &str) {
        assert!(mem_total(text).is_err());
    }

    struct Fake {
        disk: Option<String>,
        fail: Option<Fail>,
        calls: RefCell<Vec<(String, Vec<String>)>>,
    }
    #[derive(Clone, Copy, PartialEq)]
    enum Fail {
        /// Writes fail with EBUSY.
        Write,
        Command(&'static str),
    }
    impl System for Fake {
        fn read(&self, path: &'static str) -> io::Result<String> {
            if path == MEMINFO {
                return Ok(MEM.into());
            }
            self.disk
                .clone()
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
        }
        fn write(&self, path: &'static str, value: &str) -> io::Result<()> {
            self.calls
                .borrow_mut()
                .push((format!("write {path}"), vec![value.into()]));
            if self.fail == Some(Fail::Write) {
                return Err(io::Error::from_raw_os_error(16));
            }
            Ok(())
        }
        fn command(&self, program: &str, args: &[&str]) -> Result<()> {
            self.calls
                .borrow_mut()
                .push((program.into(), args.iter().map(|s| s.to_string()).collect()));
            if matches!(self.fail, Some(Fail::Command(failing)) if failing == program) {
                return Err(Error::CommandFailed {
                    program: program.into(),
                    output: "test failure".into(),
                });
            }
            Ok(())
        }
    }
    fn fake(disk: Option<&str>, fail: Option<Fail>) -> Fake {
        Fake {
            disk: disk.map(str::to_owned),
            fail,
            calls: RefCell::new(Vec::new()),
        }
    }

    #[test]
    fn disabled_does_not_load_format_or_enable_a_device() {
        let host = fake(None, None);
        prepare_with(&host, Some(&size("0%"))).unwrap();
        assert!(host.calls.borrow().is_empty());
    }

    #[test_case("0"; "disabling")]
    #[test_case("2Gi"; "resizing")]
    fn initialized_device_is_never_reset(value: &str) {
        let host = fake(Some("1073741824"), None);
        assert!(prepare_with(&host, Some(&size(value))).is_err());
        assert!(host.calls.borrow().is_empty());
    }

    #[test]
    fn formats_before_enabling_and_stops_at_failure() {
        let host = fake(None, None);
        prepare_with(&host, None).unwrap();
        assert_eq!(
            *host.calls.borrow(),
            vec![
                ("/usr/sbin/modprobe".into(), vec!["zram".into()]),
                (format!("write {DISKSIZE}"), vec!["1073741824".into()]),
                (
                    "/usr/sbin/mkswap".into(),
                    vec!["--uuid".into(), "clear".into(), DEVICE.into()]
                ),
                (
                    "/usr/sbin/swapon".into(),
                    vec![
                        "--discard".into(),
                        "--priority".into(),
                        "1024".into(),
                        DEVICE.into()
                    ]
                ),
            ]
        );
        let host = fake(None, Some(Fail::Command("/usr/sbin/mkswap")));
        assert!(prepare_with(&host, None).is_err());
        assert_eq!(host.calls.borrow().len(), 3);
    }

    #[test]
    fn device_initialized_after_the_check_is_left_alone() {
        // Another initializer wins the race between the check and the write; the kernel
        // refuses the new size, and nothing formats or enables the device.
        let host = fake(None, Some(Fail::Write));
        assert!(prepare_with(&host, None).is_err());
        let calls = host.calls.borrow();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[1].0, format!("write {DISKSIZE}"));
    }
}
