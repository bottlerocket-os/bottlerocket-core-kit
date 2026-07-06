use bottlerocket_modeled_types::{
    HugepageConfig, HugepageSize, HugepagesStatic, HugepagesTransparent,
    TransparentHugepageDefragPolicy, TransparentHugepagePolicy,
};
use log::{info, warn};
use snafu::{ensure, OptionExt, ResultExt};
use std::path::Path;

/// Sysfs directory controlling Transparent Huge Pages (THP) policy (e.g. `enabled`, `defrag`).
const TRANSPARENT_HUGEPAGE_SYSFS_ROOT: &str = "/sys/kernel/mm/transparent_hugepage";
/// Sysfs directory holding the node-agnostic (global) huge page pools, with one
/// `hugepages-<size>kB` subdirectory per supported page size.
const HUGEPAGES_SYSFS_ROOT: &str = "/sys/kernel/mm/hugepages";
/// Sysfs directory exposing per-NUMA-node state; each `node<N>` subdirectory has its own
/// `hugepages` pool used for per-node allocations.
const NODE_SYSFS_ROOT: &str = "/sys/devices/system/node";

/// A NUMA node index.
type Node = usize;
/// A count of huge pages to allocate.
type HugepageCount = u64;

/// A parsed hugepage allocation count: either a node-agnostic global total or a single
/// per-NUMA-node count.
#[derive(Debug, PartialEq, Eq)]
enum AllocationType {
    Global(HugepageCount),
    PerNode(Node, HugepageCount),
}

struct HugepageAllocation {
    size: HugepageSize,
    allocation_type: AllocationType,
}

impl HugepageAllocation {
    fn parse_hugepage_setting(size: &HugepageSize, pool: &HugepageConfig) -> Result<Vec<Self>> {
        let count = pool.count.trim();
        ensure!(!count.is_empty(), error::EmptyAllocationSnafu);

        // case when a node-agnostic hugepages are requested
        if !count.contains(':') {
            let total = count
                .parse::<HugepageCount>()
                .ok()
                .context(error::MalformedAllocationSnafu { value: count })?;
            return Ok(vec![Self {
                size: size.clone(),
                allocation_type: AllocationType::Global(total),
            }]);
        }

        let mut allocations = Vec::new();
        for pair in count.split(',') {
            let (node, pages) = pair
                .split_once(':')
                .context(error::MalformedAllocationSnafu { value: count })?;
            let node = node
                .parse::<Node>()
                .ok()
                .context(error::MalformedAllocationSnafu { value: count })?;
            let pages = pages
                .parse::<HugepageCount>()
                .ok()
                .context(error::MalformedAllocationSnafu { value: count })?;
            allocations.push(Self {
                size: size.clone(),
                allocation_type: AllocationType::PerNode(node, pages),
            });
        }

        ensure!(!allocations.is_empty(), error::EmptyAllocationSnafu);
        Ok(allocations)
    }

    fn request_hugepages(&self, essential: bool) -> Result<()> {
        let page_size = self
            .size
            .as_kib()
            .context(error::InvalidHugepageSettingsSnafu {})?;
        let size_dir_name = format!("hugepages-{page_size}kB");

        let (size_dir, requested) = match &self.allocation_type {
            AllocationType::Global(requested) => (
                Path::new(HUGEPAGES_SYSFS_ROOT).join(&size_dir_name),
                *requested,
            ),
            AllocationType::PerNode(node, requested) => {
                let node_dir = Path::new(NODE_SYSFS_ROOT)
                    .join(format!("node{node}"))
                    .join("hugepages");
                ensure!(
                    node_dir.exists(),
                    error::NumaNodeUnavailableSnafu { node: *node }
                );
                (node_dir.join(&size_dir_name), *requested)
            }
        };

        ensure!(
            size_dir.exists(),
            error::UnsupportedHugepageSizeSnafu {
                size_kib: page_size,
            }
        );

        let hugepage_path = size_dir.join("nr_hugepages");
        match reserve_hugepages(&hugepage_path, requested, page_size) {
            Ok(()) => Ok(()),
            // Failing to reserve every requested page is only fatal for essential requests;
            // for non-essential requests we log the shortfall and carry on.
            Err(e @ error::Error::HugepageShortfall { .. }) => {
                if essential {
                    Err(e)
                } else {
                    warn!("{e}");
                    Ok(())
                }
            }
            // Any other failure (I/O, parse, unsupported size or node) is always fatal.
            Err(e) => Err(e),
        }
    }
}

/// This function actually reserves hugepages. It does this by writing the requested value to the
/// sysfs file. The value written to the file is the allocated number of hugepages. We Error out
/// if we get less than the requested allocation.
fn reserve_hugepages(hugepage_path: &Path, requested: HugepageCount, page_size: u64) -> Result<()> {
    info!("reserving {requested} pages of {page_size} kiB huge pages via {hugepage_path:?}");

    std::fs::write(hugepage_path, requested.to_string()).context(error::WriteSysfsSnafu {
        path: hugepage_path.to_path_buf(),
    })?;

    let read_back = std::fs::read_to_string(hugepage_path).context(error::ReadSysfsSnafu {
        path: hugepage_path.to_path_buf(),
    })?;

    let allocated: u64 = read_back.trim().parse().context(error::ParseSysfsSnafu {
        path: hugepage_path.to_path_buf(),
        value: read_back,
    })?;

    ensure!(
        allocated == requested,
        error::HugepageShortfallSnafu {
            size_kib: page_size,
            requested,
            allocated,
        }
    );

    Ok(())
}

/// Sets the requested hugepage settings in the kernel.
///
/// The settings are rendered at boot only. Because the hugepage allocation is best-effort
/// from kernel side, we re-verify the number of allocated hugepages from sysfs. Depending
/// on `settings.kernel.hugepages.static.essential`, we error out if we fail to get the
/// requested number of hugepages.
pub fn set_static_hugepages(static_hugepages: &HugepagesStatic) -> Result<()> {
    let essential = static_hugepages.essential;
    let mut allocations: Vec<HugepageAllocation> = Vec::new();

    for (size, pool) in &static_hugepages.hugepages_config {
        allocations.extend(HugepageAllocation::parse_hugepage_setting(size, pool)?);
    }

    allocations.sort_by_key(|allocation| std::cmp::Reverse(allocation.size.as_kib().unwrap_or(0)));
    allocations
        .into_iter()
        .try_for_each(|allocation| allocation.request_hugepages(essential))
}

/// Sets the requested transparent hugepages settings in the kernel.
///
/// The settings are rendered at boot only. We set the settings to kernel's defaults. i.e.
/// enabled and defrag set to madvise. If not explicitly set, defrag is dependent on the
/// enabled setting.
pub fn set_transparent_hugepages(transparent_hugepages: &HugepagesTransparent) -> Result<()> {
    let thp_sysfs_root = Path::new(TRANSPARENT_HUGEPAGE_SYSFS_ROOT);

    // The settings model itself has a default `madvise` for the enabled setting
    let thp_enabled = transparent_hugepages.enabled.clone().unwrap_or_default();
    // When `defrag` isn't set explicitly, derive it from the `enabled` policy.
    let thp_defrag = transparent_hugepages
        .defrag
        .clone()
        .unwrap_or(match thp_enabled {
            TransparentHugepagePolicy::Always => TransparentHugepageDefragPolicy::Madvise,
            TransparentHugepagePolicy::Madvise => TransparentHugepageDefragPolicy::Madvise,
            TransparentHugepagePolicy::Never => TransparentHugepageDefragPolicy::Never,
        });

    info!("Setting transparent-hugepage enabled to {thp_enabled}");
    std::fs::write(thp_sysfs_root.join("enabled"), thp_enabled.to_string()).context(
        error::TransparentHugepageSnafu {
            setting: "enabled",
            transparent_hugepage: thp_enabled,
        },
    )?;

    info!("Setting transparent-hugepage defrag to {thp_defrag}");
    std::fs::write(thp_sysfs_root.join("defrag"), thp_defrag.to_string()).context(
        error::TransparentHugepageSnafu {
            setting: "defrag",
            transparent_hugepage: thp_defrag,
        },
    )?;

    Ok(())
}

pub mod error {
    use snafu::Snafu;
    use std::path::PathBuf;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub enum Error {
        #[snafu(display("Invalid Hugepages settings provided"))]
        InvalidHugepageSettings {},

        #[snafu(display("Unsupported huge page size {} kiB", size_kib))]
        UnsupportedHugepageSize { size_kib: u64 },

        #[snafu(display("NUMA node {} is not available", node))]
        NumaNodeUnavailable { node: usize },

        #[snafu(display("Empty huge page allocation count"))]
        EmptyAllocation {},

        #[snafu(display("Malformed huge page allocation count '{}'", value))]
        MalformedAllocation { value: String },

        #[snafu(display(
            "{} kiB huge page pool short: requested {} pages but only {} were allocated",
            size_kib,
            requested,
            allocated
        ))]
        HugepageShortfall {
            size_kib: u64,
            requested: u64,
            allocated: u64,
        },

        #[snafu(display("Failed to write huge page count to '{}': {}", path.display(), source))]
        WriteSysfs {
            path: PathBuf,
            source: std::io::Error,
        },

        #[snafu(display("Failed to read huge page count from '{}': {}", path.display(), source))]
        ReadSysfs {
            path: PathBuf,
            source: std::io::Error,
        },

        #[snafu(display("Failed to parse huge page count '{}' from '{}': {}", value, path.display(), source))]
        ParseSysfs {
            path: PathBuf,
            value: String,
            source: std::num::ParseIntError,
        },

        #[snafu(display(
            "Failed to change transparent-hugepage '{}' setting to '{}': {}",
            setting,
            transparent_hugepage,
            source
        ))]
        TransparentHugepage {
            setting: String,
            transparent_hugepage: String,
            source: std::io::Error,
        },
    }
}

pub type Result<T> = std::result::Result<T, error::Error>;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_hugepage_setting_total() {
        let size = HugepageSize::try_from("2Mi").unwrap();
        let pool = HugepageConfig {
            count: "512".try_into().unwrap(),
        };
        let allocations = HugepageAllocation::parse_hugepage_setting(&size, &pool).unwrap();
        assert_eq!(allocations.len(), 1);
        assert_eq!(allocations[0].size, size);
        assert_eq!(allocations[0].allocation_type, AllocationType::Global(512));
    }

    #[test]
    fn test_parse_hugepage_setting_per_node() {
        let size = HugepageSize::try_from("2Mi").unwrap();

        let pool = HugepageConfig {
            count: "0:128,1:256".try_into().unwrap(),
        };
        let allocations = HugepageAllocation::parse_hugepage_setting(&size, &pool).unwrap();
        assert_eq!(allocations.len(), 2);
        assert_eq!(
            allocations[0].allocation_type,
            AllocationType::PerNode(0, 128)
        );
        assert_eq!(
            allocations[1].allocation_type,
            AllocationType::PerNode(1, 256)
        );

        let pool = HugepageConfig {
            count: "2:512".try_into().unwrap(),
        };
        let allocations = HugepageAllocation::parse_hugepage_setting(&size, &pool).unwrap();
        assert_eq!(allocations.len(), 1);
        assert_eq!(
            allocations[0].allocation_type,
            AllocationType::PerNode(2, 512)
        );
    }
}
