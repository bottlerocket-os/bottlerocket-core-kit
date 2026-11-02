//! Apply settings-rendered memory.min drop-ins without restarting services.
use bottlerocket_modeled_types::KubeReservedMemoryMinValue;
use log::{info, warn};
use snafu::{ensure, OptionExt, ResultExt, Snafu};
use std::{env, fs, io, process::Command};

// Load rendered values before systemctl set-property's 50-MemoryMin.conf.
// Otherwise daemon-reload can replace the zero fallback with configured floors.
const RUNTIME: &str = "/etc/systemd/system/runtime.slice.d/10-memory-min.conf";
const KUBELET: &str = "/etc/systemd/system/kubelet.service.d/10-memory-min.conf";
const CONTAINERD: &str = "/etc/systemd/system/containerd.service.d/10-memory-min.conf";

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Unable to read {path}: {source}"))]
    Read {
        path: &'static str,
        source: io::Error,
    },
    #[snafu(display("Invalid or duplicate memory.min content in {path}"))]
    Content { path: &'static str },
    #[snafu(display("runtime.slice MemoryMin must equal the sum of both services"))]
    ParentSum,
    #[snafu(display("Unable to run systemctl for {unit}: {source}"))]
    Execute {
        unit: &'static str,
        source: io::Error,
    },
    #[snafu(display("systemctl failed for {unit}: {output}"))]
    CommandFailed { unit: &'static str, output: String },
}
type Result<T> = std::result::Result<T, Error>;

trait System {
    fn read(&self, path: &'static str) -> io::Result<String>;
    fn set_min(&self, unit: &'static str, bytes: u64) -> Result<()>;
}

struct Host;
impl System for Host {
    fn read(&self, path: &'static str) -> io::Result<String> {
        fs::read_to_string(path)
    }

    fn set_min(&self, unit: &'static str, bytes: u64) -> Result<()> {
        let output = Command::new("/usr/bin/systemctl")
            .args([
                "set-property",
                "--runtime",
                unit,
                &format!("MemoryMin={bytes}"),
            ])
            .output()
            .context(ExecuteSnafu { unit })?;
        ensure!(
            output.status.success(),
            CommandFailedSnafu {
                unit,
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

/// Our templates generate these files, so unknown lines and duplicates are errors.
fn parse(text: &str, path: &'static str, section: &str) -> Result<u64> {
    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'));
    ensure!(lines.next() == Some(section), ContentSnafu { path });
    let value = lines
        .next()
        .and_then(|line| line.strip_prefix("MemoryMin="))
        .context(ContentSnafu { path })?;
    let bytes = KubeReservedMemoryMinValue::try_from(value)
        .ok()
        .context(ContentSnafu { path })?
        .as_bytes();
    ensure!(lines.next().is_none(), ContentSnafu { path });
    Ok(bytes)
}

fn apply_with(system: &impl System, boot: bool) -> Result<()> {
    let mut values = [None; 3];
    let mut missing = Vec::new();
    for (i, (path, section)) in [
        (RUNTIME, "[Slice]"),
        (KUBELET, "[Service]"),
        (CONTAINERD, "[Service]"),
    ]
    .into_iter()
    .enumerate()
    {
        match system.read(path) {
            Ok(text) => values[i] = Some(parse(&text, path, section)?),
            Err(error) if boot && error.kind() == io::ErrorKind::NotFound => {
                missing.push(path);
            }
            Err(source) => return Err(Error::Read { path, source }),
        }
    }
    // Every file is parsed before clearing, so a missing file can't hide invalid content.
    if !missing.is_empty() {
        for path in missing {
            warn!("Missing rendered memory.min file {path}; clearing service and parent memory.min during boot");
        }
        for unit in ["kubelet.service", "containerd.service", "runtime.slice"] {
            system.set_min(unit, 0)?;
        }
        return Ok(());
    }
    let [Some(parent), Some(kubelet), Some(containerd)] = values else {
        unreachable!("all files read and parsed");
    };
    ensure!(
        kubelet
            .saturating_add(containerd)
            .min(KubeReservedMemoryMinValue::MAX_BYTES)
            == parent,
        ParentSumSnafu
    );

    // Parent first: the kernel caps children's effective protection at the parent's.
    for (unit, bytes) in [
        ("runtime.slice", parent),
        ("kubelet.service", kubelet),
        ("containerd.service", containerd),
    ] {
        system.set_min(unit, bytes)?;
    }
    info!("Applied memory.min: kubelet={kubelet}, containerd={containerd}, runtime={parent} bytes");
    Ok(())
}

pub fn apply() -> Result<()> {
    // CHANGED_SETTINGS is set only by API-triggered runs; --all omits it.
    apply_with(&Host, env::var_os("CHANGED_SETTINGS").is_none())
}

#[cfg(test)]
mod tests {
    use super::*;
    use schnauzer::v2::import::{JsonSettingsResolver, StaticHelperResolver};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use test_case::{test_case, test_matrix};

    struct Importer {
        settings_resolver: JsonSettingsResolver,
        helper_resolver: StaticHelperResolver,
    }
    schnauzer::impl_template_importer!(Importer, JsonSettingsResolver, StaticHelperResolver);

    #[tokio::test]
    async fn swap_service_without_variant_defaults_renders_and_applies_zero() {
        // Downstream registers the three release-swap templates but supplies no values.
        let importer = Importer {
            settings_resolver: JsonSettingsResolver::new(serde_json::json!({
                "settings": {"kubernetes": {}}
            })),
            helper_resolver: StaticHelperResolver,
        };
        let mut fake = Fake::new(0, 0);
        for (component, path) in [
            ("runtime", RUNTIME),
            ("kubelet", KUBELET),
            ("containerd", CONTAINERD),
        ] {
            let template = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../../../packages/release/{component}-memory-min-conf"
            ));
            let output = schnauzer::v2::render_template_file(&importer, template)
                .await
                .unwrap();
            fake.files.insert(path, output);
        }
        apply_with(&fake, true).unwrap();
        assert_eq!(
            *fake.writes.borrow(),
            [
                ("runtime.slice", 0),
                ("kubelet.service", 0),
                ("containerd.service", 0)
            ]
        );
    }

    struct Fake {
        files: HashMap<&'static str, String>,
        errors: HashMap<&'static str, io::ErrorKind>,
        writes: RefCell<Vec<(&'static str, u64)>>,
        fail_at: Option<usize>,
    }
    impl Fake {
        fn new(kubelet: u64, containerd: u64) -> Self {
            Self {
                files: HashMap::from([
                    (
                        RUNTIME,
                        format!(
                            "[Slice]\nMemoryMin={}\n",
                            kubelet
                                .saturating_add(containerd)
                                .min(KubeReservedMemoryMinValue::MAX_BYTES)
                        ),
                    ),
                    (KUBELET, format!("[Service]\nMemoryMin={kubelet}\n")),
                    (CONTAINERD, format!("[Service]\nMemoryMin={containerd}\n")),
                ]),
                errors: HashMap::new(),
                writes: RefCell::new(vec![]),
                fail_at: None,
            }
        }
    }
    impl System for Fake {
        fn read(&self, path: &'static str) -> io::Result<String> {
            if let Some(kind) = self.errors.get(path) {
                return Err(io::Error::from(*kind));
            }
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| io::ErrorKind::NotFound.into())
        }
        fn set_min(&self, unit: &'static str, bytes: u64) -> Result<()> {
            self.writes.borrow_mut().push((unit, bytes));
            ensure!(
                Some(self.writes.borrow().len()) != self.fail_at,
                CommandFailedSnafu {
                    unit,
                    output: "injected failure"
                }
            );
            Ok(())
        }
    }

    #[test_case(0, 0; "zero")]
    #[test_case(128 << 20, 128 << 20; "equal")]
    #[test_case(200 << 20, 300 << 20; "different")]
    #[test_case(KubeReservedMemoryMinValue::MAX_BYTES, 1; "capped parent")]
    fn sets_parent_then_children_without_reading_systemd(a: u64, b: u64) {
        let fake = Fake::new(a, b);
        apply_with(&fake, false).unwrap();
        assert_eq!(
            *fake.writes.borrow(),
            [
                (
                    "runtime.slice",
                    a.saturating_add(b)
                        .min(KubeReservedMemoryMinValue::MAX_BYTES)
                ),
                ("kubelet.service", a),
                ("containerd.service", b)
            ]
        );
    }

    #[test_case(RUNTIME; "runtime")]
    #[test_case(KUBELET; "kubelet")]
    #[test_case(CONTAINERD; "containerd")]
    fn missing_file_clears_children_then_parent_only_at_boot(path: &'static str) {
        let mut fake = Fake::new(128, 128);
        fake.files.remove(path);
        apply_with(&fake, true).unwrap();
        assert_eq!(
            *fake.writes.borrow(),
            [
                ("kubelet.service", 0),
                ("containerd.service", 0),
                ("runtime.slice", 0)
            ]
        );
        fake.writes.borrow_mut().clear();
        assert!(matches!(apply_with(&fake, false), Err(Error::Read { .. })));
        assert!(fake.writes.borrow().is_empty());
    }

    #[test_matrix(
        [
            "",
            "[Service]\n",
            "[Service]\nMemoryMin=-1Mi",
            "[Service]\nMemoryMin=1..2Mi",
            "[Service]\nMemoryMin=1.5Gi",
            "[Service]\nMemoryMin=01Mi",
            "[Service]\nMemoryMin=16Ei",
            "[Service]\nMemoryMin=1G",
            "[Service]\nMemoryMin=1\nMemoryMin=2",
            "[Service]\nMemoryMin=1\n[Service]",
            "[Slice]\nMemoryMin=1",
            "[Service]\nMemoryMin=1\nExecStart=/bin/false"
        ],
        [false, true]
    )]
    fn invalid_content_remains_fatal_even_if_another_file_is_missing(text: &str, boot: bool) {
        let mut fake = Fake::new(128, 128);
        fake.files.insert(CONTAINERD, text.into());
        if boot {
            fake.files.remove(RUNTIME);
        }
        assert!(apply_with(&fake, boot).is_err());
        assert!(fake.writes.borrow().is_empty());
    }

    #[test_case(io::ErrorKind::PermissionDenied)]
    #[test_case(io::ErrorKind::InvalidData)]
    #[test_case(io::ErrorKind::IsADirectory)]
    #[test_case(io::ErrorKind::Other)]
    fn unreadable_and_directory_files_never_skip(kind: io::ErrorKind) {
        let mut fake = Fake::new(128, 128);
        fake.errors.insert(KUBELET, kind);
        assert!(apply_with(&fake, true).is_err());
        assert!(fake.writes.borrow().is_empty());
    }

    #[test_case("255"; "short by one")]
    #[test_case("256Mi"; "unit")]
    fn parent_must_equal_the_children_sum(value: &str) {
        let mut fake = Fake::new(128, 128);
        fake.files
            .insert(RUNTIME, format!("[Slice]\nMemoryMin={value}"));
        assert!(matches!(apply_with(&fake, false), Err(Error::ParentSum)));
        assert!(fake.writes.borrow().is_empty());
    }

    #[test_matrix([false, true], 1..=3)]
    fn each_set_property_failure_is_fatal_with_no_later_writes(boot: bool, step: usize) {
        let mut fake = Fake::new(200, 300);
        fake.fail_at = Some(step);
        assert!(apply_with(&fake, boot).is_err());
        assert_eq!(fake.writes.borrow().len(), step);
    }

    #[test_matrix([RUNTIME, KUBELET, CONTAINERD], 1..=3)]
    fn clearing_failure_is_fatal_with_no_later_writes(path: &'static str, step: usize) {
        let mut fake = Fake::new(200, 300);
        fake.files.remove(path);
        fake.fail_at = Some(step);
        assert!(apply_with(&fake, true).is_err());
        assert_eq!(fake.writes.borrow().len(), step);
    }

    #[test_case(""; "empty")]
    #[test_case("+256"; "plus sign")]
    #[test_case("-1"; "negative")]
    #[test_case("2.56e2"; "exponent")]
    #[test_case("1..2"; "range")]
    #[test_case("18446744073709551616"; "u64 overflow")]
    fn invalid_parent_content_fails_before_writes(value: &str) {
        let mut fake = Fake::new(128, 128);
        fake.files
            .insert(RUNTIME, format!("[Slice]\nMemoryMin={value}"));
        assert!(matches!(
            apply_with(&fake, true),
            Err(Error::Content { .. })
        ));
        assert!(fake.writes.borrow().is_empty());
    }
}
