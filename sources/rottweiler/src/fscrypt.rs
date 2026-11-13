use std::os::fd::AsRawFd;
use std::path::Path;

use hkdf::Hkdf;
use nix::{ioctl_read, ioctl_readwrite};
use sha2::Sha512;
use snafu::prelude::*;
use zeroize::ZeroizeOnDrop;

use crate::mount_point::MountPoint;

type Result<T> = std::result::Result<T, snafu::Whatever>;

/// Public key for fscrypt operations (contains only the key identifier)
pub struct FscryptPublicKey {
    identifier: [u8; 16],
}

impl FscryptPublicKey {
    /// Read the encryption policy from a directory and extract the key identifier
    pub fn from_directory(path: &Path) -> Result<Self> {
        let policy = FscryptGetPolicyExArg::from_path(path)?;
        Ok(Self {
            identifier: policy.key_identifier(),
        })
    }

    /// Remove the encryption key from the kernel keyring, locking the directory
    pub fn lock_directory(&self, path: &Path) -> Result<()> {
        let mount_point = MountPoint::from_path(path).with_whatever_context(|_| {
            format!("Failed to find mount point for '{}'", path.display())
        })?;
        let mount_fd = mount_point.open()?;
        let mut remove_key = FscryptRemoveKey::new(self.identifier);
        // SAFETY: The ioctl requires a valid file descriptor and a pointer to fscrypt_remove_key_arg.
        // - `mount_fd` is a valid open file descriptor
        // - `remove_key` is properly initialized with repr(C) layout matching the kernel struct
        // - The kernel copies data with copy_from_user/copy_to_user, so no lifetime issues
        unsafe { remove_encryption_key_all_users(mount_fd.as_raw_fd(), &mut remove_key) }
            .with_whatever_context(|_| {
                format!("Failed to remove encryption key for '{}'", path.display())
            })?;
        Ok(())
    }

    /// Set the encryption policy on a directory
    pub fn encrypt_directory(&self, path: &Path) -> Result<()> {
        let mut policy = FscryptPolicyV2::new(self.identifier);
        let dir_fd = std::fs::File::open(path)
            .with_whatever_context(|_| format!("Failed to open directory '{}'", path.display()))?;
        let argptr = &mut policy as *mut FscryptPolicyV2 as *mut FscryptPolicyV1Ioctl;
        // SAFETY: The ioctl requires a valid file descriptor and a pointer to a policy struct.
        // - `dir_fd` is a valid open file descriptor for the directory
        // - The kernel reads the version field first (via get_user), then copies the full struct
        // - Both FscryptPolicyV2 and FscryptPolicyV1Ioctl are repr(C) and start with version field
        // - The cast is safe because the kernel interprets the data based on the version field
        unsafe { set_encryption_policy(dir_fd.as_raw_fd(), argptr) }.with_whatever_context(
            |_| format!("Failed to set encryption policy for '{}'", path.display()),
        )?;
        Ok(())
    }
}

/// Private key for fscrypt operations (contains the key identifier and raw key material)
#[derive(ZeroizeOnDrop)]
pub struct FscryptPrivateKey {
    identifier: [u8; 16],
    raw: Vec<u8>,
}

impl FscryptPrivateKey {
    /// Create a private key from raw key bytes, deriving the key identifier
    pub fn from_bytes(raw: &[u8]) -> Result<Self> {
        let identifier = calculate_key_identifier(raw)?;
        Ok(Self {
            identifier,
            raw: raw.to_vec(),
        })
    }

    /// Add the encryption key to the kernel keyring, unlocking the directory
    pub fn unlock_directory(&self, path: &Path) -> Result<()> {
        let mount_point = MountPoint::from_path(path).with_whatever_context(|_| {
            format!("Failed to find mount point for '{}'", path.display())
        })?;
        let mount_fd = mount_point.open()?;
        let mut add_key = FscryptAddKey::new(&self.raw)?;
        let argptr = &mut add_key as *mut FscryptAddKey as *mut FscryptAddKeyIoctl;
        // SAFETY: The ioctl requires a valid file descriptor and a pointer to fscrypt_add_key_arg.
        // - `mount_fd` is a valid open file descriptor for the mount point
        // - `add_key` is properly initialized with repr(C) layout matching the kernel struct
        // - The cast is safe: FscryptAddKey contains FscryptAddKeyIoctl fields plus raw[] array
        // - The kernel uses copy_from_user to read the struct including the raw key bytes
        unsafe { add_encryption_key(mount_fd.as_raw_fd(), argptr) }.with_whatever_context(
            |_| format!("Failed to add encryption key for '{}'", path.display()),
        )?;
        Ok(())
    }
}

impl From<FscryptPrivateKey> for FscryptPublicKey {
    fn from(private: FscryptPrivateKey) -> Self {
        Self {
            identifier: private.identifier,
        }
    }
}

/// Outcome of asking the kernel whether a directory's fscrypt key is present in the
/// filesystem keyring.
pub enum DirectoryKeyStatus {
    /// The directory carries no fscrypt policy, or its filesystem can't carry one.
    NotEncrypted,
    /// The directory is encrypted and its key is present in the keyring.
    KeyPresent,
    /// The directory is encrypted and its key has been removed from the keyring.
    KeyRemoved,
}

impl DirectoryKeyStatus {
    /// Query whether `path`'s fscrypt key is present in the kernel keyring.
    ///
    /// A directory with no fscrypt policy at all, or whose filesystem can't carry one, is
    /// reported as `NotEncrypted` rather than an error; that's the common case on hosts that
    /// never enabled datastore encryption.
    pub fn of_directory(path: &Path) -> Result<Self> {
        let identifier = match key_identifier_of_directory(path)? {
            Some(identifier) => identifier,
            None => return Ok(DirectoryKeyStatus::NotEncrypted),
        };

        let mount_point = MountPoint::from_path(path).with_whatever_context(|_| {
            format!("Failed to find mount point for '{}'", path.display())
        })?;
        let mount_fd = mount_point.open()?;
        let mut status = FscryptGetKeyStatus::new(identifier);
        unsafe { get_encryption_key_status(mount_fd.as_raw_fd(), &mut status) }
            .with_whatever_context(|_| {
                format!(
                    "Failed to get encryption key status for '{}'",
                    path.display()
                )
            })?;

        directory_key_status_of_raw(status.status, path)
    }
}

fn directory_key_status_of_raw(status: u32, path: &Path) -> Result<DirectoryKeyStatus> {
    match status {
        FSCRYPT_KEY_STATUS_PRESENT => Ok(DirectoryKeyStatus::KeyPresent),
        // INCOMPLETELY_REMOVED is the expected status immediately after lockdown: the
        // removal ioctl skips files that are still open, and the apiserver itself holds
        // datastore files open, so the key is gone from the keyring but the kernel hasn't
        // finished evicting cached inodes/keys for in-use files. Both statuses mean the key
        // can no longer be used to open new files, which is what callers care about.
        FSCRYPT_KEY_STATUS_ABSENT | FSCRYPT_KEY_STATUS_INCOMPLETELY_REMOVED => {
            Ok(DirectoryKeyStatus::KeyRemoved)
        }
        other => {
            snafu::whatever!(
                "Unrecognized fscrypt key status {other} for '{}'",
                path.display()
            );
        }
    }
}

/// Get the fscrypt key identifier for a directory's policy, or `None` if the directory carries
/// no policy, or its filesystem can't carry one.
fn key_identifier_of_directory(path: &Path) -> Result<Option<[u8; 16]>> {
    let dir = std::fs::File::open(path)
        .with_whatever_context(|_| format!("Failed to open directory '{}'", path.display()))?;
    let mut arg = FscryptGetPolicyExArg::new()?;
    let argptr = &mut arg as *mut FscryptGetPolicyExArg as *mut FscryptGetPolicyExArgIoctl;
    match unsafe { get_encryption_policy_ex(dir.as_raw_fd(), argptr) } {
        Ok(_) => Ok(Some(FscryptRetrievedPolicy { inner: arg }.key_identifier())),
        // No policy is set, or the filesystem can't carry one at all; both are answers, not
        // failures, and the common case for hosts that never enabled datastore encryption.
        Err(nix::errno::Errno::ENODATA)
        | Err(nix::errno::Errno::ENOTTY)
        | Err(nix::errno::Errno::EOPNOTSUPP) => Ok(None),
        Err(e) => Err(e).with_whatever_context(|_| {
            format!("Failed to get encryption policy for '{}'", path.display())
        }),
    }
}

const FSCRYPT_KEY_SPEC_TYPE_IDENTIFIER: u32 = 2;
const FSCRYPT_KEY_STATUS_ABSENT: u32 = 1;
const FSCRYPT_KEY_STATUS_PRESENT: u32 = 2;
const FSCRYPT_KEY_STATUS_INCOMPLETELY_REMOVED: u32 = 3;
const FSCRYPT_MODE_AES_256_XTS: u8 = 1;
const FSCRYPT_MODE_AES_256_CTS: u8 = 4;
const FSCRYPT_POLICY_FLAGS_PAD_32: u8 = 3;
const FSCRYPT_POLICY_V2: u8 = 2;
const FSCRYPT_MAX_KEY_SIZE: usize = 64;

/// Ioctl struct for FS_IOC_SET_ENCRYPTION_POLICY (ioctl 19).
/// Corresponds to kernel's fscrypt_policy_v1. Used for ioctl definition
/// to get correct ioctl number, even when setting v2 policies.
#[repr(C)]
#[derive(Copy, Clone)]
struct FscryptPolicyV1Ioctl {
    version: u8,
    contents_encryption_mode: u8,
    filenames_encryption_mode: u8,
    flags: u8,
    master_key_descriptor: [u8; 8],
}

/// V2 encryption policy struct. Corresponds to kernel's fscrypt_policy_v2.
/// Contains the actual policy data passed to FS_IOC_SET_ENCRYPTION_POLICY.
#[repr(C)]
#[derive(Copy, Clone)]
struct FscryptPolicyV2 {
    version: u8,
    contents_encryption_mode: u8,
    filenames_encryption_mode: u8,
    flags: u8,
    __reserved: [u8; 4],
    master_key_identifier: [u8; 16],
}

impl FscryptPolicyV2 {
    fn new(key_identifier: [u8; 16]) -> Self {
        Self {
            version: FSCRYPT_POLICY_V2,
            contents_encryption_mode: FSCRYPT_MODE_AES_256_XTS,
            filenames_encryption_mode: FSCRYPT_MODE_AES_256_CTS,
            flags: FSCRYPT_POLICY_FLAGS_PAD_32,
            __reserved: [0; 4],
            master_key_identifier: key_identifier,
        }
    }
}

#[repr(C)]
union FscryptPolicy {
    version: u8,
    v2: FscryptPolicyV2,
}

#[repr(C)]
union FscryptKeySpecifierU {
    __reserved: [u8; 32],
    identifier: [u8; 16],
}

#[repr(C)]
struct FscryptKeySpecifier {
    type_: u32,
    __reserved: u32,
    u: FscryptKeySpecifierU,
}

impl FscryptKeySpecifier {
    /// Build a key specifier identifying a key by its 16-byte identifier, zeroing the rest of
    /// the union first. The kernel rejects these ioctls with EINVAL if any reserved bits are
    /// nonzero, so every caller needs the union's upper bytes zeroed, not just the first 16.
    fn by_identifier(identifier: [u8; 16]) -> Self {
        let mut u = FscryptKeySpecifierU {
            __reserved: [0; 32],
        };
        u.identifier = identifier;
        Self {
            type_: FSCRYPT_KEY_SPEC_TYPE_IDENTIFIER,
            __reserved: 0,
            u,
        }
    }
}

/// Get policy argument struct. Corresponds to kernel's fscrypt_get_policy_ex_arg.
/// Contains the full policy union for FS_IOC_GET_ENCRYPTION_POLICY_EX.
/// This is the "uninitialized" state before the ioctl is called.
#[repr(C)]
struct FscryptGetPolicyExArg {
    policy_size: u64,
    policy: FscryptPolicy,
}

/// Ioctl struct for FS_IOC_GET_ENCRYPTION_POLICY_EX (ioctl 22).
/// Packed struct with just size + version, matching kernel's ioctl definition (__u8[9]).
#[repr(C, packed)]
struct FscryptGetPolicyExArgIoctl {
    policy_size: u64,
    version: u8,
}

impl FscryptGetPolicyExArg {
    fn new() -> Result<Self> {
        Ok(Self {
            policy_size: u64::try_from(std::mem::size_of::<FscryptPolicyV2>())
                .with_whatever_context(|_| "Policy size too large")?,
            policy: FscryptPolicy {
                version: FSCRYPT_POLICY_V2,
            },
        })
    }

    fn from_path(path: &Path) -> Result<FscryptRetrievedPolicy> {
        let dir = std::fs::File::open(path)
            .with_whatever_context(|_| format!("Failed to open directory '{}'", path.display()))?;
        let mut arg = Self::new()?;
        let argptr = &mut arg as *mut FscryptGetPolicyExArg as *mut FscryptGetPolicyExArgIoctl;
        // SAFETY: The ioctl requires a valid file descriptor and a pointer to fscrypt_get_policy_ex_arg.
        // - `dir` is a valid open file descriptor for the directory
        // - `arg` is properly initialized with policy_size and version fields
        // - The kernel first copies policy_size from userspace, then copies the full policy back
        // - Both structs are repr(C) and share the same initial layout (policy_size field)
        // - Verified against kernel implementation in fs/crypto/policy.c (fscrypt_ioctl_get_policy_ex)
        unsafe { get_encryption_policy_ex(dir.as_raw_fd(), argptr) }
            .with_whatever_context(|_| "Failed to get encryption policy")?;
        Ok(FscryptRetrievedPolicy { inner: arg })
    }
}

/// Policy retrieved from the kernel via FS_IOC_GET_ENCRYPTION_POLICY_EX.
/// This type guarantees the policy union has been populated by the kernel.
struct FscryptRetrievedPolicy {
    inner: FscryptGetPolicyExArg,
}

impl FscryptRetrievedPolicy {
    fn key_identifier(&self) -> [u8; 16] {
        // SAFETY: Accessing union field is safe because:
        // - FscryptRetrievedPolicy can only be constructed after a successful ioctl
        // - The kernel's fscrypt_ioctl_get_policy_ex writes the policy based on what's stored
        // - The kernel implementation (fs/crypto/policy.c) guarantees the v2 variant is active
        //   when the stored policy version is v2
        // - The type system enforces this method is only callable on kernel-populated policies
        unsafe { self.inner.policy.v2.master_key_identifier }
    }
}

/// Remove key argument struct. Corresponds to kernel's fscrypt_remove_key_arg.
/// Used directly for both ioctl definition and data (no separate ioctl variant needed).
#[repr(C)]
struct FscryptRemoveKey {
    key_spec: FscryptKeySpecifier,
    removal_status_flags: u32,
    __reserved: [u32; 5],
}

impl FscryptRemoveKey {
    fn new(key_identifier: [u8; 16]) -> Self {
        Self {
            key_spec: FscryptKeySpecifier::by_identifier(key_identifier),
            removal_status_flags: 0,
            __reserved: [0; 5],
        }
    }
}

/// Used directly for both ioctl definition and data
#[repr(C)]
struct FscryptGetKeyStatus {
    key_spec: FscryptKeySpecifier,
    __reserved: [u32; 6],
    status: u32,
    status_flags: u32,
    user_count: u32,
    __out_reserved: [u32; 13],
}

impl FscryptGetKeyStatus {
    fn new(key_identifier: [u8; 16]) -> Self {
        Self {
            key_spec: FscryptKeySpecifier::by_identifier(key_identifier),
            __reserved: [0; 6],
            status: 0,
            status_flags: 0,
            user_count: 0,
            __out_reserved: [0; 13],
        }
    }
}

/// Ioctl struct for FS_IOC_ADD_ENCRYPTION_KEY (ioctl 23).
/// Base struct without the raw key bytes. Corresponds to kernel's fscrypt_add_key_arg
/// without the flexible array member.
#[repr(C)]
struct FscryptAddKeyIoctl {
    key_spec: FscryptKeySpecifier,
    raw_size: u32,
    key_id: u32,
    __reserved: [u32; 8],
}

/// Add key argument struct with raw key bytes. Corresponds to kernel's fscrypt_add_key_arg
/// with the flexible array member (raw[]) included as a fixed-size array.
#[repr(C)]
#[derive(ZeroizeOnDrop)]
struct FscryptAddKey {
    #[zeroize(skip)]
    key_spec: FscryptKeySpecifier,
    raw_size: u32,
    key_id: u32,
    __reserved: [u32; 8],
    raw: [u8; FSCRYPT_MAX_KEY_SIZE],
}

impl FscryptAddKey {
    fn new(key: &[u8]) -> Result<Self> {
        if key.is_empty() || key.len() > FSCRYPT_MAX_KEY_SIZE {
            snafu::whatever!("key must be between 1 and {} bytes", FSCRYPT_MAX_KEY_SIZE);
        }

        let mut raw = [0u8; FSCRYPT_MAX_KEY_SIZE];
        raw[..key.len()].copy_from_slice(key);

        let key_identifier = calculate_key_identifier(key)?;

        Ok(Self {
            key_spec: FscryptKeySpecifier::by_identifier(key_identifier),
            raw_size: u32::try_from(key.len()).with_whatever_context(|_| "Key size too large")?,
            key_id: 0,
            __reserved: [0; 8],
            raw,
        })
    }
}

ioctl_readwrite!(
    get_encryption_policy_ex,
    b'f',
    22,
    FscryptGetPolicyExArgIoctl
);
ioctl_readwrite!(remove_encryption_key_all_users, b'f', 25, FscryptRemoveKey);
ioctl_readwrite!(add_encryption_key, b'f', 23, FscryptAddKeyIoctl);
ioctl_readwrite!(get_encryption_key_status, b'f', 26, FscryptGetKeyStatus);
ioctl_read!(set_encryption_policy, b'f', 19, FscryptPolicyV1Ioctl);

/// Calculate the fscrypt key identifier from raw key bytes using HKDF-SHA512
fn calculate_key_identifier(key: &[u8]) -> Result<[u8; 16]> {
    let hkdf = Hkdf::<Sha512>::new(None, key);
    let mut output = [0u8; 16];
    hkdf.expand(b"fscrypt\x00\x01", &mut output)
        .with_whatever_context(|_| "Failed to derive key identifier")?;
    Ok(output)
}

// Compile-time validation of struct layouts against kernel definitions
// These assertions ensure our repr(C) structs match the kernel's C structs exactly.
const _: () = {
    use std::mem::{align_of, offset_of, size_of};

    // struct fscrypt_policy_v1: 1 + 1 + 1 + 1 + 8 = 12 bytes
    const _: () = assert!(size_of::<FscryptPolicyV1Ioctl>() == 12);
    const _: () = assert!(align_of::<FscryptPolicyV1Ioctl>() == 1);

    // struct fscrypt_policy_v2: 1 + 1 + 1 + 1 + 1 + 3 + 16 = 24 bytes
    const _: () = assert!(size_of::<FscryptPolicyV2>() == 24);
    const _: () = assert!(align_of::<FscryptPolicyV2>() == 1);
    const _: () = assert!(offset_of!(FscryptPolicyV2, version) == 0);
    const _: () = assert!(offset_of!(FscryptPolicyV2, contents_encryption_mode) == 1);
    const _: () = assert!(offset_of!(FscryptPolicyV2, filenames_encryption_mode) == 2);
    const _: () = assert!(offset_of!(FscryptPolicyV2, flags) == 3);
    const _: () = assert!(offset_of!(FscryptPolicyV2, __reserved) == 4);
    const _: () = assert!(offset_of!(FscryptPolicyV2, master_key_identifier) == 8);

    // struct fscrypt_key_specifier: 4 + 4 + 32 = 40 bytes
    const _: () = assert!(size_of::<FscryptKeySpecifier>() == 40);
    const _: () = assert!(align_of::<FscryptKeySpecifier>() == 4);
    const _: () = assert!(offset_of!(FscryptKeySpecifier, type_) == 0);
    const _: () = assert!(offset_of!(FscryptKeySpecifier, __reserved) == 4);
    const _: () = assert!(offset_of!(FscryptKeySpecifier, u) == 8);

    // struct fscrypt_add_key_arg (without raw[]): 40 + 4 + 4 + 32 = 80 bytes
    const _: () = assert!(size_of::<FscryptAddKeyIoctl>() == 80);
    const _: () = assert!(align_of::<FscryptAddKeyIoctl>() == 4);
    const _: () = assert!(offset_of!(FscryptAddKey, key_spec) == 0);
    const _: () = assert!(offset_of!(FscryptAddKey, raw_size) == 40);
    const _: () = assert!(offset_of!(FscryptAddKey, key_id) == 44);
    const _: () = assert!(offset_of!(FscryptAddKey, __reserved) == 48);
    const _: () = assert!(offset_of!(FscryptAddKey, raw) == 80);

    // struct fscrypt_add_key_arg with raw[64]: 80 + 64 = 144 bytes
    const _: () = assert!(size_of::<FscryptAddKey>() == 144);
    const _: () = assert!(align_of::<FscryptAddKey>() == 4);

    // struct fscrypt_remove_key_arg: 40 + 4 + 20 = 64 bytes
    const _: () = assert!(size_of::<FscryptRemoveKey>() == 64);
    const _: () = assert!(align_of::<FscryptRemoveKey>() == 4);
    const _: () = assert!(offset_of!(FscryptRemoveKey, key_spec) == 0);
    const _: () = assert!(offset_of!(FscryptRemoveKey, removal_status_flags) == 40);
    const _: () = assert!(offset_of!(FscryptRemoveKey, __reserved) == 44);

    // struct fscrypt_get_policy_ex_arg: 8 + max(1, 12, 24) = 32 bytes
    const _: () = assert!(size_of::<FscryptGetPolicyExArg>() == 32);
    const _: () = assert!(align_of::<FscryptGetPolicyExArg>() == 8);

    // __u8[9] for ioctl definition: 8 + 1 = 9 bytes (packed)
    const _: () = assert!(size_of::<FscryptGetPolicyExArgIoctl>() == 9);
    const _: () = assert!(align_of::<FscryptGetPolicyExArgIoctl>() == 1);

    // struct fscrypt_get_key_status_arg: 40 + 24 + 4 + 4 + 4 + 52 = 128 bytes
    const _: () = assert!(size_of::<FscryptGetKeyStatus>() == 128);
    const _: () = assert!(offset_of!(FscryptGetKeyStatus, status) == 64);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tempdir_reports_not_encrypted() {
        // A tempdir's filesystem either lacks fscrypt support or has it disabled, so this must
        // report `NotEncrypted` rather than erroring; getting this wrong breaks every host that
        // never enabled datastore encryption.
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        match DirectoryKeyStatus::of_directory(dir.path()) {
            Ok(DirectoryKeyStatus::NotEncrypted) => {}
            Ok(_) => panic!("expected NotEncrypted for an unencrypted tempdir"),
            Err(e) => panic!("expected NotEncrypted for an unencrypted tempdir, got error: {e}"),
        }
    }

    #[test]
    fn raw_status_present_maps_to_key_present() {
        let path = Path::new("/test/path");
        match directory_key_status_of_raw(FSCRYPT_KEY_STATUS_PRESENT, path) {
            Ok(DirectoryKeyStatus::KeyPresent) => {}
            Ok(_) => panic!("expected KeyPresent"),
            Err(e) => panic!("expected KeyPresent, got error: {e}"),
        }
    }

    #[test]
    fn raw_status_absent_maps_to_key_removed() {
        let path = Path::new("/test/path");
        match directory_key_status_of_raw(FSCRYPT_KEY_STATUS_ABSENT, path) {
            Ok(DirectoryKeyStatus::KeyRemoved) => {}
            Ok(_) => panic!("expected KeyRemoved"),
            Err(e) => panic!("expected KeyRemoved, got error: {e}"),
        }
    }

    #[test]
    fn raw_status_incompletely_removed_maps_to_key_removed() {
        // This is the status the kernel actually reports once lockdown has run: the removal
        // ioctl skips files the apiserver still has open, so the key is gone from the keyring
        // but not yet fully evicted. Callers must still see this as locked.
        let path = Path::new("/test/path");
        match directory_key_status_of_raw(FSCRYPT_KEY_STATUS_INCOMPLETELY_REMOVED, path) {
            Ok(DirectoryKeyStatus::KeyRemoved) => {}
            Ok(_) => panic!("expected KeyRemoved"),
            Err(e) => panic!("expected KeyRemoved, got error: {e}"),
        }
    }

    #[test]
    fn raw_status_unrecognized_value_errors() {
        let path = Path::new("/test/path");
        assert!(
            directory_key_status_of_raw(99, path).is_err(),
            "expected an error for status 99"
        );
    }

    #[test]
    fn raw_status_zero_errors() {
        let path = Path::new("/test/path");
        assert!(
            directory_key_status_of_raw(0, path).is_err(),
            "expected an error for status 0"
        );
    }

    #[test]
    fn key_specifier_by_identifier_zeroes_trailing_bytes() {
        let identifier = [0xffu8; 16];
        let spec = FscryptKeySpecifier::by_identifier(identifier);
        let raw = unsafe { spec.u.__reserved };
        assert_eq!(&raw[..16], &identifier);
        assert_eq!(&raw[16..], &[0u8; 16]);
    }

    #[test]
    fn ioctl_request_numbers_match_the_kernel() {
        // The kernel encodes a request's payload size into its ioctl number, so a struct whose
        // size doesn't match the kernel's own definition produces a request number the kernel
        // doesn't recognize; every call then returns ENOTTY, which this module treats as "not
        // encrypted" rather than an error. Pin both request numbers to the values the kernel
        // header computes for FS_IOC_GET_ENCRYPTION_POLICY_EX and
        // FS_IOC_GET_ENCRYPTION_KEY_STATUS, so a future change to either struct's size is caught
        // here instead of surfacing as a silent fail-open.
        let policy_ex_request = nix::request_code_readwrite!(
            b'f',
            22,
            std::mem::size_of::<FscryptGetPolicyExArgIoctl>()
        );
        assert_eq!(policy_ex_request as u32, 0xC009_6616);

        let key_status_request =
            nix::request_code_readwrite!(b'f', 26, std::mem::size_of::<FscryptGetKeyStatus>());
        assert_eq!(key_status_request as u32, 0xC080_661A);
    }
}
