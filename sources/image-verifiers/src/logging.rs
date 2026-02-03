/*!
Logger that routes output by level for containerd's image verifier interface.

Containerd captures stdout as the rejection "reason" shown in kubelet events,
and pipes stderr to debug logs. This logger routes accordingly:
- Error → stdout (single-line rejection reason for kubelet events)
- Everything else → stderr (operational logs for debugging)

Output is the bare message with no timestamp or level prefix.
*/

use log::{Level, Log, Metadata, Record};
use std::io::Write;

struct VerifierLogger;

impl Log for VerifierLogger {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        match record.level() {
            Level::Error => {
                let _ = writeln!(std::io::stdout(), "{}", record.args());
            }
            _ => {
                let _ = writeln!(std::io::stderr(), "{}", record.args());
            }
        }
    }

    fn flush(&self) {
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
    }
}

/// Initialize the verifier logger at debug level.
pub fn init() {
    log::set_logger(&VerifierLogger).ok();
    log::set_max_level(log::LevelFilter::Debug);
}
