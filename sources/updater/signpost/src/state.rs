use crate::error::{self, Error};
use crate::gptprio::GptPrio;
use crate::guid::uuid_to_guid;
use crate::set::{PartitionSet, SetSelect};
use block_party::BlockDevice;
use gptman::GPT;
use hex_literal::hex;
use snafu::{ensure, OptionExt, ResultExt};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

const BOTTLEROCKET_BOOT: [u8; 16] = uuid_to_guid(hex!("6b636168 7420 6568 2070 6c616e657421"));
const BOTTLEROCKET_ROOT: [u8; 16] = uuid_to_guid(hex!("5526016a 1a97 4ea4 b39a b7c8c6ca4502"));
const BOTTLEROCKET_HASH: [u8; 16] = uuid_to_guid(hex!("598f10af c955 4456 6a99 7720068a6cea"));
const BOTTLEROCKET_PRIVATE: [u8; 16] = uuid_to_guid(hex!("440408bb eb0b 4328 a6e5 a29038fad706"));

/// The extended boot loader partition, as defined by the Discoverable Partitions Specification.
///
/// UKI images tag their FAT boot partition with this type so systemd-boot can find it; GRUB
/// images have no partition of this type, so its presence identifies a UKI layout at runtime.
const XBOOTLDR: [u8; 16] = uuid_to_guid(hex!("bc13c2ff 59e6 4262 a352 b275fd6f7172"));

#[derive(Debug, Clone)]
pub struct State {
    os_disk: PathBuf,
    // BOTTLEROCKET_PRIVATE partition number
    private_partition_num: u32,
    sets: PartitionSets,
    /// The partition numbers that correspond to the boot partitions in each partition set,
    /// respectively.
    ///
    /// This is used to load the correct partition flags from `table`.
    boot_partition_nums: Vec<u32>,
    table: GPT,
}

/// The OS disk backing the running root filesystem, and its parsed partition table.
///
/// [`DiskLayout::scan`] does the disk open and GPT parse once, shared by both
/// [`DiskLayout::into_state`] (A/B partition state) and [`DiskLayout::xbootldr`] (XBOOTLDR
/// discovery).
pub struct DiskLayout {
    os_disk: BlockDevice,
    active_partition: PathBuf,
    table: GPT,
}

impl DiskLayout {
    /// Opens the OS disk backing the running root filesystem and parses its partition table.
    pub fn scan() -> Result<Self, Error> {
        // The root filesystem is a dm-verity device. We want to determine what disk and partition
        // the backing data is part of. Look up the device major and minor via stat(2):
        let root_fs = BlockDevice::from_device_path("/")
            .context(error::BlockDeviceFromPathSnafu { device: "/" })?;
        // Get the first lower device from this one, and determine what disk it belongs to.
        let active_partition = root_fs
            .lower_devices()
            .and_then(|mut iter| iter.next().transpose())
            .context(error::RootLowerDevicesSnafu {
                root: root_fs.path(),
            })?
            .context(error::RootHasNoLowerDevicesSnafu {
                root: root_fs.path(),
            })?;
        let os_disk = active_partition
            .disk()
            .context(error::DiskFromPartitionSnafu {
                device: root_fs.path(),
            })?
            .context(error::RootNotPartitionSnafu {
                device: root_fs.path(),
            })?;
        let active_partition = active_partition.path();

        // Parse the partition table on the disk.
        let table = GPT::find_from(&mut File::open(os_disk.path()).context(error::OpenSnafu {
            path: os_disk.path(),
            what: "reading",
        })?)
        .map_err(error::GPTError)
        .context(error::GPTFindSnafu {
            device: os_disk.path(),
        })?;

        Ok(Self {
            os_disk,
            active_partition,
            table,
        })
    }

    /// Returns the device path of the XBOOTLDR partition on the OS disk.
    ///
    /// Fails if the disk has no partition of that type (the case on GRUB images) or more than
    /// one.
    pub fn xbootldr(&self) -> Result<PathBuf, Error> {
        resolve_xbootldr_partition(&self.os_disk, &self.table)
    }

    /// Interprets the scanned table as a GRUB-era A/B partition layout, using the partition
    /// found under the running root filesystem to determine which set (A or B) is active.
    pub fn into_state(self) -> Result<State, Error> {
        let Self {
            os_disk,
            active_partition,
            table,
        } = self;

        // Finds the nth partition on `table` matching the partition type GUID `guid`, ignoring
        // unused entries.
        let nth_guid = |guid, n| -> Option<u32> {
            table
                .iter()
                .filter(|(_, p)| p.is_used() && p.partition_type_guid == guid)
                .nth(n)
                .map(|(n, _)| n)
        };

        // `part_type` is the human-readable name for the error message; it can't be derived
        // from `guid` at runtime, so callers must pass it explicitly.
        let required_guid = |guid, n, part_type| -> Result<u32, Error> {
            nth_guid(guid, n).context(error::PartitionMissingFromSetSnafu {
                part_type,
                set: if n == 0 { "A" } else { "B" },
            })
        };

        // Loads the path to partition number `num` on the OS disk.
        let device_from_part_num =
            |num| -> Result<PathBuf, Error> { self::device_from_part_num(&os_disk, num) };

        let mut boot_partition_nums =
            vec![required_guid(BOTTLEROCKET_BOOT, 0, "BOTTLEROCKET_BOOT")?];
        if let Some(b) = nth_guid(BOTTLEROCKET_BOOT, 1) {
            boot_partition_nums.push(b);
        }

        let mut sets = Vec::new();
        sets.push(PartitionSet {
            boot: device_from_part_num(boot_partition_nums[0])?,
            root: device_from_part_num(required_guid(BOTTLEROCKET_ROOT, 0, "BOTTLEROCKET_ROOT")?)?,
            hash: device_from_part_num(required_guid(BOTTLEROCKET_HASH, 0, "BOTTLEROCKET_HASH")?)?,
        });

        if boot_partition_nums.len() == 2 {
            sets.push(PartitionSet {
                boot: device_from_part_num(boot_partition_nums[1])?,
                root: device_from_part_num(required_guid(
                    BOTTLEROCKET_ROOT,
                    1,
                    "BOTTLEROCKET_ROOT",
                )?)?,
                hash: device_from_part_num(required_guid(
                    BOTTLEROCKET_HASH,
                    1,
                    "BOTTLEROCKET_HASH",
                )?)?,
            });
        }

        let sets = if sets.len() == 2 {
            // Determine which set is active by seeing which set contains the current running root or
            // hash partition.
            let active = if sets[0].contains(&active_partition) {
                SetSelect::A
            } else if sets[1].contains(&active_partition) {
                SetSelect::B
            } else {
                return error::ActiveNotInSetSnafu {
                    active_partition,
                    sets,
                }
                .fail();
            };
            let sets = sets.try_into().ok().context(error::ConvertVecSnafu)?;
            PartitionSets::Dual(DualPartitionSet { sets, active })
        } else {
            PartitionSets::Single(SinglePartitionSet {
                set: sets.remove(0),
            })
        };

        Ok(State {
            os_disk: os_disk.path(),
            private_partition_num: required_guid(BOTTLEROCKET_PRIVATE, 0, "BOTTLEROCKET_PRIVATE")?,
            sets,
            boot_partition_nums,
            table,
        })
    }
}

/// Finds the partition numbers of every XBOOTLDR-typed partition on `table`, in ascending
/// order. UKI images tag their boot partition with this type GUID; GRUB images have none.
///
/// Takes an already-parsed `&GPT` so this filter is unit-testable without a real block device;
/// returns bare numbers since resolving them to paths needs the caller's `BlockDevice` handle.
fn xbootldr_partition_nums(table: &GPT) -> Vec<u32> {
    table
        .iter()
        .filter(|(_, p)| p.is_used() && p.partition_type_guid == XBOOTLDR)
        .map(|(num, _)| num)
        .collect()
}

/// Loads the path to partition number `num` on `os_disk`.
fn device_from_part_num(os_disk: &BlockDevice, num: u32) -> Result<PathBuf, Error> {
    Ok(os_disk
        .partition(num)
        .context(error::PartitionFromDiskSnafu {
            device: os_disk.path(),
        })?
        .context(error::PartitionNotFoundOnDeviceSnafu {
            num,
            device: os_disk.path(),
        })?
        .path())
}

/// Resolves the XBOOTLDR partition on `table` to a device path on `os_disk`. Fails if no
/// XBOOTLDR partition is found, or if more than one is found, since there would be no way to
/// choose between them.
fn resolve_xbootldr_partition(os_disk: &BlockDevice, table: &GPT) -> Result<PathBuf, Error> {
    let xbootldr_nums = xbootldr_partition_nums(table);
    ensure!(
        xbootldr_nums.len() <= 1,
        error::MultiplePartitionsOfTypeSnafu {
            partition_type: guid_to_string(XBOOTLDR),
            partitions: xbootldr_nums
                .iter()
                .copied()
                .map(|num| device_from_part_num(os_disk, num))
                .collect::<Result<Vec<_>, Error>>()?,
        }
    );
    let num = xbootldr_nums
        .first()
        .copied()
        .context(error::NoXbootldrPartitionSnafu)?;
    device_from_part_num(os_disk, num)
}

/// Formats a GPT partition type GUID as its canonical mixed-endian string form (first three
/// fields little-endian, last two big-endian on disk).
fn guid_to_string(g: [u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-\
         {:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        g[3],
        g[2],
        g[1],
        g[0],
        g[5],
        g[4],
        g[7],
        g[6],
        g[8],
        g[9],
        g[10],
        g[11],
        g[12],
        g[13],
        g[14],
        g[15],
    )
}

#[derive(Debug, Clone)]
enum PartitionSets {
    Single(SinglePartitionSet),
    Dual(DualPartitionSet),
}

#[derive(Debug, Clone)]
struct SinglePartitionSet {
    set: PartitionSet,
}

#[derive(Debug, Clone)]
struct DualPartitionSet {
    sets: [PartitionSet; 2],
    active: SetSelect,
}

impl State {
    /// Finds the partition sets available on disk, and determines which is active under the root
    /// filesystem.
    pub fn load() -> Result<Self, Error> {
        DiskLayout::scan()?.into_state()
    }

    pub(crate) fn os_disk(&self) -> &Path {
        &self.os_disk
    }

    fn gpt_attributes(&self, num_part: u32) -> u64 {
        self.table[num_part].attribute_bits
    }

    fn gptprio(&self, select: SetSelect) -> GptPrio {
        GptPrio::from(self.gpt_attributes(self.boot_partition_nums[select.idx()]))
    }

    fn set_gpt_attributes(&mut self, num_part: u32, flags: GptPrio) {
        self.table[num_part].attribute_bits = flags.into();
    }

    fn set_gptprio(&mut self, select: SetSelect, flags: GptPrio) {
        self.set_gpt_attributes(self.boot_partition_nums[select.idx()], flags);
    }

    pub fn active(&self) -> SetSelect {
        match &self.sets {
            PartitionSets::Dual(s) => s.active,
            PartitionSets::Single(_) => SetSelect::A,
        }
    }

    pub fn inactive(&self) -> Option<SetSelect> {
        match &self.sets {
            PartitionSets::Dual(s) => Some(!s.active),
            PartitionSets::Single(_) => None,
        }
    }

    pub fn active_set(&self) -> &PartitionSet {
        match &self.sets {
            PartitionSets::Dual(s) => &s.sets[s.active.idx()],
            PartitionSets::Single(s) => &s.set,
        }
    }

    pub fn inactive_set(&self) -> Option<&PartitionSet> {
        match &self.sets {
            PartitionSets::Dual(s) => Some(&s.sets[(!s.active).idx()]),
            PartitionSets::Single(_) => None,
        }
    }

    pub fn next(&self) -> Option<SetSelect> {
        if let PartitionSets::Single(_) = self.sets {
            if self.gptprio(SetSelect::A).will_boot() {
                return Some(SetSelect::A);
            } else {
                return None;
            }
        }

        let gptprio_a = self.gptprio(SetSelect::A);
        let gptprio_b = self.gptprio(SetSelect::B);
        match (gptprio_a.will_boot(), gptprio_b.will_boot()) {
            (true, true) => {
                if gptprio_a.priority() >= gptprio_b.priority() {
                    Some(SetSelect::A)
                } else {
                    Some(SetSelect::B)
                }
            }
            (true, false) => Some(SetSelect::A),
            (false, true) => Some(SetSelect::B),
            (false, false) => None,
        }
    }

    /// Sets the active partition as successfully booted, but **does not write to the disk**.
    /// Marks the BOTTLEROCKET_PRIVATE partition table to indicate that boot has succeeded at least once
    pub fn mark_successful_boot(&mut self) {
        let mut flags = self.gptprio(self.active());
        flags.set_successful(true);
        self.set_gptprio(self.active(), flags);

        let mut private_flags = GptPrio::from(self.gpt_attributes(self.private_partition_num));
        private_flags.boot_has_succeeded();
        self.set_gpt_attributes(self.private_partition_num, private_flags);
    }

    /// Clears priority bits of the inactive partition in preparation to write new images, but
    /// **does not write to the disk**.
    pub fn clear_inactive(&mut self) -> Result<(), Error> {
        let inactive = self.inactive().context(error::InactiveNotAvailableSnafu {
            inactive: &self.os_disk,
        })?;

        let mut inactive_flags = self.gptprio(inactive);
        inactive_flags.set_priority(0);
        inactive_flags.set_tries_left(0);
        inactive_flags.set_successful(false);
        self.set_gptprio(inactive, inactive_flags);

        Ok(())
    }

    /// Sets 'tries left' to 1 on the inactive partition to represent a
    /// potentially valid image, but does not change the priority.
    /// **does not write to the disk**.
    pub fn mark_inactive_valid(&mut self) -> Result<(), Error> {
        let inactive = self.inactive().context(error::InactiveNotAvailableSnafu {
            inactive: &self.os_disk,
        })?;

        let mut inactive_flags = self.gptprio(inactive);
        inactive_flags.set_tries_left(1);
        self.set_gptprio(inactive, inactive_flags);

        Ok(())
    }

    /// Sets the inactive partition as a new upgrade partition, but **does not write to the disk**.
    /// Ensures that the inactive partition is marked as valid beforehand
    ///
    /// * Sets the inactive partition's priority to 2 and the active partition's priority to 1.
    /// * Sets the inactive partition as not successfully booted.
    /// * Returns an error if the partition has not been marked as potentially
    ///   valid or if it has already been marked for upgrade.
    pub fn upgrade_to_inactive(&mut self) -> Result<(), Error> {
        let inactive = self.inactive().context(error::InactiveNotAvailableSnafu {
            inactive: &self.os_disk,
        })?;

        let mut inactive_flags = self.gptprio(inactive);
        ensure!(
            inactive_flags.priority() == 0 && !inactive_flags.successful(),
            error::InactiveAlreadyMarkedSnafu {
                inactive: &self.os_disk
            }
        );
        ensure!(
            inactive_flags.tries_left() > 0,
            error::InactiveNotValidSnafu {
                inactive: &self.os_disk
            }
        );

        inactive_flags.set_priority(2);
        inactive_flags.set_successful(false);
        self.set_gptprio(inactive, inactive_flags);

        let mut active_flags = self.gptprio(self.active());
        active_flags.set_priority(1);
        self.set_gptprio(self.active(), active_flags);
        Ok(())
    }

    /// Reverts upgrade_to_inactive(), but **does not write to the disk**.
    ///
    /// * Sets the inactive partition's priority to 0
    /// * Restores the active partition's priority to 2
    pub fn cancel_upgrade(&mut self) -> Result<(), Error> {
        let inactive = self.inactive().context(error::InactiveNotAvailableSnafu {
            inactive: &self.os_disk,
        })?;

        let mut inactive_flags = self.gptprio(inactive);
        inactive_flags.set_priority(0);
        self.set_gptprio(inactive, inactive_flags);

        let mut active_flags = self.gptprio(self.active());
        active_flags.set_priority(2);
        self.set_gptprio(self.active(), active_flags);

        Ok(())
    }

    /// Prioritizes the inactive partition, but **does not write to the disk**.
    ///
    /// Returns an error if the inactive partition is not bootable (it doesn't have a prior
    /// successful boot and doesn't have the priority/tries_left that would make it safe to try).
    pub fn rollback_to_inactive(&mut self) -> Result<(), Error> {
        let inactive = self.inactive().context(error::InactiveNotAvailableSnafu {
            inactive: &self.os_disk,
        })?;

        let mut inactive_flags = self.gptprio(inactive);
        if !inactive_flags.will_boot() {
            return error::InactiveInvalidRollbackSnafu {
                priority: inactive_flags.priority(),
                tries_left: inactive_flags.tries_left(),
                successful: inactive_flags.successful(),
            }
            .fail();
        }
        inactive_flags.set_priority(2);
        self.set_gptprio(inactive, inactive_flags);

        let mut active_flags = self.gptprio(self.active());
        active_flags.set_priority(1);
        self.set_gptprio(self.active(), active_flags);

        Ok(())
    }

    /// Returns whether boot has ever succeeded or not
    pub fn has_boot_succeeded(&mut self) -> bool {
        let private_flags = GptPrio::from(self.gpt_attributes(self.private_partition_num));
        private_flags.has_boot_succeeded()
    }

    /// Writes the partition table to the OS disk.
    pub fn write(&mut self) -> Result<(), Error> {
        self.table
            .write_into(
                &mut OpenOptions::new()
                    .write(true)
                    .open(self.os_disk())
                    .context(error::OpenSnafu {
                        path: &self.os_disk,
                        what: "writing",
                    })?,
            )
            .map_err(error::GPTError)
            .context(error::GPTWriteSnafu {
                device: &self.os_disk,
            })?;
        Ok(())
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "OS disk: {}", self.os_disk.display())?;
        writeln!(
            f,
            "Set A:   {} {}",
            self.active_set(),
            self.gptprio(SetSelect::A)
        )?;
        if let Some(inactive_set) = self.inactive_set() {
            writeln!(
                f,
                "Set B:   {} {}",
                inactive_set,
                self.gptprio(SetSelect::B)
            )?;
        }
        writeln!(f, "Active:  Set {}", self.active())?;
        match self.next() {
            Some(next) => write!(f, "Next:    Set {next}"),
            None => write!(f, "Next:    None"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        guid_to_string, xbootldr_partition_nums, BOTTLEROCKET_BOOT, BOTTLEROCKET_HASH,
        BOTTLEROCKET_PRIVATE, BOTTLEROCKET_ROOT, XBOOTLDR,
    };
    use crate::error::{self, Error};
    use gptman::{GPTPartitionEntry, GPT};
    use snafu::OptionExt;
    use std::io::Cursor;

    const SECTOR_SIZE: u64 = 512;

    /// Builds an in-memory partition table whose partitions have the given type GUIDs, in order
    /// starting at partition number 1.
    fn table_with_types(types: &[[u8; 16]]) -> GPT {
        let mut disk = Cursor::new(vec![0u8; 1024 * SECTOR_SIZE as usize]);
        let mut table =
            GPT::new_from(&mut disk, SECTOR_SIZE, [0xff; 16]).expect("could not build a GPT");

        for (i, partition_type) in types.iter().enumerate() {
            let num = u32::try_from(i).unwrap() + 1;
            let starting_lba = 2048 + u64::from(num) * 64;
            table[num] = GPTPartitionEntry {
                partition_type_guid: *partition_type,
                unique_partition_guid: [num as u8; 16],
                starting_lba,
                ending_lba: starting_lba + 63,
                attribute_bits: 0,
                partition_name: "test".into(),
            };
        }

        table
    }

    /// Finds the numbers of the partitions in `table` matching `partition_type`, in ascending
    /// order. Mirrors the private `nth_guid` filter predicate in `DiskLayout::into_state`, since
    /// that closure can't be called directly from a unit test.
    fn partition_nums_with_type(table: &GPT, partition_type: [u8; 16]) -> Vec<u32> {
        table
            .iter()
            .filter(|(_, p)| p.is_used() && p.partition_type_guid == partition_type)
            .map(|(num, _)| num)
            .collect()
    }

    #[test]
    fn finds_partitions_of_a_type_in_order() {
        // A GRUB layout: two boot partitions and the private partition.
        let table = table_with_types(&[BOTTLEROCKET_BOOT, BOTTLEROCKET_PRIVATE, BOTTLEROCKET_BOOT]);

        assert_eq!(
            partition_nums_with_type(&table, BOTTLEROCKET_BOOT),
            vec![1, 3]
        );
        assert_eq!(
            partition_nums_with_type(&table, BOTTLEROCKET_PRIVATE),
            vec![2]
        );
    }

    #[test]
    fn ignores_unused_and_other_types() {
        // A UKI layout: one XBOOTLDR boot partition and the private partition. The rest of the
        // partition array is unused, i.e. has an all-zero type GUID.
        let table = table_with_types(&[XBOOTLDR, BOTTLEROCKET_PRIVATE]);

        assert_eq!(partition_nums_with_type(&table, XBOOTLDR), vec![1]);
        // The GRUB boot type must not match the XBOOTLDR partition, which is what tells the two
        // layouts apart.
        assert!(partition_nums_with_type(&table, BOTTLEROCKET_BOOT).is_empty());
        assert!(partition_nums_with_type(&table, [0u8; 16]).is_empty());
    }

    #[test]
    fn is_used_matches_nonzero_type_guid() {
        // gptman's `is_used()` is defined purely in terms of the type GUID being non-zero
        // (`is_used() == (partition_type_guid != [0; 16])`). Confirm that equivalence holds, so
        // a future gptman upgrade that changes `is_used()`'s definition is caught here instead
        // of silently changing which partitions `DiskLayout::into_state` and the XBOOTLDR scan
        // consider.
        let mut disk = Cursor::new(vec![0u8; 1024 * SECTOR_SIZE as usize]);
        let mut table =
            GPT::new_from(&mut disk, SECTOR_SIZE, [0xff; 16]).expect("could not build a GPT");
        table[1] = GPTPartitionEntry {
            partition_type_guid: XBOOTLDR,
            unique_partition_guid: [1u8; 16],
            starting_lba: 2048,
            ending_lba: 2048 + 63,
            attribute_bits: 0,
            partition_name: "test".into(),
        };

        assert!(table[1].is_used());
        // partition 2 was never assigned, so its type GUID is still all-zero.
        assert!(!table[2].is_used());
    }

    #[test]
    fn xbootldr_guid_byte_order() {
        // The canonical form, from `partyplanner`, is "bc13c2ff-59e6-4262-a352-b275fd6f7172". On
        // disk the first three fields are little-endian and the last two are big-endian, so the
        // raw bytes are the first three fields reversed followed by the rest unchanged.
        assert_eq!(
            XBOOTLDR,
            [
                0xff, 0xc2, 0x13, 0xbc, // bc13c2ff, little-endian
                0xe6, 0x59, // 59e6, little-endian
                0x62, 0x42, // 4262, little-endian
                0xa3, 0x52, // a352, big-endian
                0xb2, 0x75, 0xfd, 0x6f, 0x71, 0x72, // b275fd6f7172, big-endian
            ]
        );
    }

    #[test]
    fn guid_to_string_matches_canonical_xbootldr_form() {
        // Checked against the same canonical form documented in `xbootldr_guid_byte_order` above.
        assert_eq!(
            guid_to_string(XBOOTLDR),
            "bc13c2ff-59e6-4262-a352-b275fd6f7172"
        );
    }

    #[test]
    fn bottlerocket_boot_guid_byte_order() {
        // Bottlerocket's boot partition typecode spells out a message on disk.
        assert_eq!(BOTTLEROCKET_BOOT, *b"hack the planet!");
    }

    #[test]
    fn xbootldr_partition_nums_finds_uki_partition_without_bottlerocket_boot() {
        // A UKI layout has no BOTTLEROCKET_BOOT partition at all; `xbootldr_partition_nums` must
        // find the XBOOTLDR partition without requiring BOTTLEROCKET_BOOT to be present.
        let table = table_with_types(&[
            XBOOTLDR,
            BOTTLEROCKET_ROOT,
            BOTTLEROCKET_HASH,
            BOTTLEROCKET_PRIVATE,
        ]);

        assert_eq!(xbootldr_partition_nums(&table), vec![1]);
        assert!(partition_nums_with_type(&table, BOTTLEROCKET_BOOT).is_empty());
    }

    #[test]
    fn xbootldr_partition_nums_is_empty_on_a_grub_layout() {
        // A GRUB layout has no XBOOTLDR partition; `xbootldr_partition_nums` must return no
        // matches, while the GRUB-era boot/root/hash/private partitions are still found normally.
        let table = table_with_types(&[
            BOTTLEROCKET_BOOT,
            BOTTLEROCKET_ROOT,
            BOTTLEROCKET_HASH,
            BOTTLEROCKET_BOOT,
            BOTTLEROCKET_ROOT,
            BOTTLEROCKET_HASH,
            BOTTLEROCKET_PRIVATE,
        ]);

        assert!(xbootldr_partition_nums(&table).is_empty());
        assert_eq!(
            partition_nums_with_type(&table, BOTTLEROCKET_BOOT),
            vec![1, 4]
        );
        assert_eq!(
            partition_nums_with_type(&table, BOTTLEROCKET_ROOT),
            vec![2, 5]
        );
        assert_eq!(
            partition_nums_with_type(&table, BOTTLEROCKET_HASH),
            vec![3, 6]
        );
        assert_eq!(
            partition_nums_with_type(&table, BOTTLEROCKET_PRIVATE),
            vec![7]
        );
    }

    #[test]
    fn missing_xbootldr_partition_is_an_error() {
        // Exercises the same `.first().copied().context(...)` lookup that
        // `resolve_xbootldr_partition` uses, since that function takes a `&BlockDevice` and
        // can't be constructed from a synthetic table in a unit test.
        let table = table_with_types(&[
            BOTTLEROCKET_BOOT,
            BOTTLEROCKET_ROOT,
            BOTTLEROCKET_HASH,
            BOTTLEROCKET_PRIVATE,
        ]);

        let result: Result<u32, Error> = xbootldr_partition_nums(&table)
            .first()
            .copied()
            .context(error::NoXbootldrPartitionSnafu);

        assert!(matches!(result, Err(Error::NoXbootldrPartition)));
    }
}
