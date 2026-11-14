# Encrypted Storage Implementation

This document describes how Bottlerocket implements encrypted storage for both the data partition (`/local`) and the datastore directory (`/.bottlerocket/datastore`).

## Overview

Bottlerocket's encrypted storage feature provides:

- **Block device encryption** using LUKS2 for the `/local` partition
- **Directory encryption** using fscrypt for `/.bottlerocket/datastore`
- **TPM2-based key management** with automatic unlocking
- **Boot phase measurements** for attestation and policy enforcement
- **Transparent operation** with no user intervention required

All encryption keys are sealed to TPM2 PCRs, ensuring data can only be decrypted when the system boots in a trusted state.

> The LUKS2 flow above applies to `encrypted-storage` variants. Variants that also enable `ephemeral-encryption-keys` use plain-mode dm-crypt with a per-boot key instead;
fscrypt datastore encryption is unchanged. See [Ephemeral Encryption Keys](#ephemeral-encryption-keys).

## Architecture

### Components

1. **rottweiler** - Unified storage encryption helper (Rust binary)
2. **systemd services** - Orchestrate encryption, unlocking, and measurements
3. **TPM2** - Hardware security module for key sealing and measurements
4. **systemd-creds** - Encrypts keys with TPM2 PCR binding
5. **cryptsetup** - LUKS2 block device encryption
6. **fscrypt** - Directory-level encryption

### Key Storage

Encryption keys are stored in `/.bottlerocket/keystore/` as TPM2-sealed credentials:
- `datastore` - Key for `/.bottlerocket/datastore` (fscrypt)
- `bottlerocket-data` - Key for `/dev/disk/by-partlabel/BOTTLEROCKET-DATA` (LUKS2)

Keys are:
- 64 bytes of random data from `/dev/random`
- Encrypted with `systemd-creds` using TPM2 PCR binding
- Automatically decrypted during boot when PCR values match

### TPM2 PCR Binding

Keys are bound to specific TPM2 Platform Configuration Registers (PCRs):

| PCR | Purpose | Why Included |
|-----|---------|--------------|
| 4 | Boot loader code (shim, grub, kernel) | Ensures kernel hasn't changed (if updates disabled) |
| 7 | Secure Boot policy | Prevents boot of unsigned code |
| 9 | Kernel command line (includes dm-verity root hash) | Ensures userspace hasn't changed (if updates disabled) |
| 11 | Boot phase | Tracks boot progression |
| 14 | Machine-owner keys (MOK) | Validates custom certificates |

Additional PCRs used for measurements (not bound to keys):

| PCR | Purpose | Usage |
|-----|---------|-------|
| 8 | EC2 user data and OS settings | Measured before and after configuration completes |
| 10 | (Reserved) | Reserved for future use |

**PCR selection logic:**
- **With in-place updates enabled**: PCRs 7+11+14 (allows kernel/userspace updates)
- **With in-place updates disabled**: PCRs 4+7+9+11+14 (locks to specific kernel/userspace)

This ensures encrypted data can only be accessed when booting a trusted configuration.

## Boot Flow

### First Boot

```
1. tpm2.target
   ↓
2. encrypt-local-fs.service
   - Checks if /dev/disk/by-partlabel/BOTTLEROCKET-DATA is unencrypted
   - Generates random key → /.bottlerocket/keystore/bottlerocket-data
   - Encrypts key with systemd-creds (TPM2 PCRs 7+11+14 or 4+7+9+11+14)
   - Formats partition with LUKS2 using the key
   ↓
3. unlock-local-fs.service
   - Decrypts key from /.bottlerocket/keystore/bottlerocket-data
   - Attaches LUKS device as /dev/mapper/BOTTLEROCKET-DATA
   ↓
4. prepare-local-fs.service
   - Creates filesystem on /dev/mapper/BOTTLEROCKET-DATA if needed
   ↓
5. local.mount
   - Mounts /dev/mapper/BOTTLEROCKET-DATA to /local
   ↓
6. encrypt-datastore.service
   - Enables encrypt feature on BOTTLEROCKET-PRIVATE filesystem
   - Generates random key → /.bottlerocket/keystore/datastore
   - Encrypts key with systemd-creds (TPM2 PCRs)
   - Sets fscrypt policy on /.bottlerocket/datastore
   ↓
7. unlock-datastore.service
   - Decrypts key from /.bottlerocket/keystore/datastore
   - Adds key to kernel keyring, unlocking directory
```

### Subsequent Boots

```
1. tpm2.target
   ↓
2. encrypt-local-fs.service
   - Checks if already encrypted → skips (ExecCondition fails)
   ↓
3. unlock-local-fs.service
   - Decrypts key and attaches LUKS device
   ↓
4. prepare-local-fs.service
   - Filesystem already exists → skips
   ↓
5. local.mount
   - Mounts encrypted partition
   ↓
6. encrypt-datastore.service
   - Directory already encrypted → skips (ExecCondition fails)
   ↓
7. unlock-datastore.service
   - Decrypts key and unlocks directory
```

## Service Details

### Block Device Encryption (/local)

#### encrypt-local-fs.service

**Purpose:** One-time setup to encrypt the BOTTLEROCKET-DATA partition.

**Key behaviors:**
- Only runs if TPM2 is available (`ConditionSecurity=tpm2`)
- Only runs if partition is unencrypted (`ExecCondition`)
- Generates 64-byte random key
- Encrypts key with TPM2 PCR binding
- Formats partition with LUKS2 (PBKDF2, 1000 iterations)

**Dependencies:**
- After: `tpm2.target`, `dev-disk-by-partlabel-BOTTLEROCKET-DATA.device`
- Before: `unlock-local-fs.service`
- Required by: `unlock-local-fs.service`

#### unlock-local-fs.service

**Purpose:** Decrypt and attach the LUKS device on every boot.

**Key behaviors:**
- Decrypts key from keystore using TPM2
- Attaches LUKS device as `/dev/mapper/BOTTLEROCKET-DATA`
- Detaches on shutdown (`ExecStop`)

**Dependencies:**
- After: `cryptsetup-pre.target`, `systemd-udevd-kernel.socket`
- Before: `cryptsetup.target`, `local-fs.target`
- Required by: `local-fs.target`

#### prepare-local-fs.service (modified)

**Drop-in:** `prepare-local-fs-encrypted.conf`

**Changes:**
- Operates on `/dev/mapper/BOTTLEROCKET-DATA` instead of raw partition
- Depends on `unlock-local-fs.service`

#### local.mount (modified)

**Drop-in:** `local-mount-encrypted.conf`

**Changes:**
- Mounts `/dev/mapper/BOTTLEROCKET-DATA` instead of raw partition

#### repart-local.service (modified)

**Drop-in:** `repart-local-encrypted.conf`

**Changes:**
- Resizes LUKS container after partition resize
- Uses `rottweiler resize block-device` command

### Directory Encryption (/.bottlerocket/datastore)

#### encrypt-datastore.service

**Purpose:** One-time setup to encrypt the datastore directory.

**Key behaviors:**
- Only runs if TPM2 is available
- Only runs if directory is unencrypted (`ExecCondition`)
- Enables encrypt feature on ext4 filesystem (`tune2fs -O encrypt`)
- Generates 64-byte random key
- Encrypts key with TPM2 PCR binding
- Sets fscrypt policy on directory

**Dependencies:**
- After: `tpm2.target`
- Before: `unlock-datastore.service`
- Required by: `unlock-datastore.service`

#### unlock-datastore.service

**Purpose:** Unlock the encrypted directory on every boot.

**Key behaviors:**
- Decrypts key from keystore using TPM2
- Adds key to kernel keyring
- Directory becomes accessible

**Dependencies:**
- Before: `migrator.service`, `storewolf.service`
- Required by: `migrator.service`, `storewolf.service`

## Ephemeral Encryption Keys

Variants with the `ephemeral-encryption-keys` image feature (in addition to `encrypted-storage`) replace the LUKS2 flow
with plain-mode dm-crypt for `BOTTLEROCKET-DATA`, `BOTTLEROCKET-PRIVATE`, and the `EPHEMERAL-DATA` device:

- Each key is 64 bytes from `/dev/random`, TPM2-sealed into the `/run/rottweiler` tmpfs keystore to ensure it can be
decrypted only if the system is in a trusted state, and deleted (`ExecStartPost=rottweiler delete-key`) within the
service that opened the device
- No key or data persists: every boot generates a new key, making prior contents unreadable, and recreates the filesystem
- The datastore still uses `fscrypt`, via a combined `encrypt-unlock-datastore.service`

### Boot Flow (Every Boot)

```
1. encrypt-unlock-local-fs.service
   - Generates a per-boot key, opens BOTTLEROCKET-DATA as /dev/mapper/BOTTLEROCKET-DATA (plain mode)
   - Creates the filesystem, grows the partition, resizes the mapper
   - Deletes the key
   ↓
2. prepare-local-fs.service
   - No-op: filesystem already created in step 1
   ↓
3. local.mount
   - Mounts /dev/mapper/BOTTLEROCKET-DATA to /local
   ↓
4. repart-local.service
   - systemd-repart grow is masked (done in step 1)
   - systemd-growfs /local grows the filesystem to fill the resized mapper
```

```
1. encrypt-unlock-private-fs.service
   - Generates a per-boot key, opens BOTTLEROCKET-PRIVATE as /dev/mapper/BOTTLEROCKET-PRIVATE (plain mode)
   - Deletes the key
   ↓
2. prepare-private-fs.service
   - Creates filesystem on /dev/mapper/BOTTLEROCKET-PRIVATE (mkfs.ext4 -O encrypt)
   ↓
3. .bottlerocket.mount
   - Mounts /dev/mapper/BOTTLEROCKET-PRIVATE to /.bottlerocket
   ↓
4. encrypt-unlock-datastore.service
   - Seals a key into /run/rottweiler, sets fscrypt policy, unlocks /.bottlerocket/datastore
   - Deletes the key
```

### Services

#### encrypt-unlock-local-fs.service

**Purpose:** Encrypt and open the BOTTLEROCKET-DATA partition on every boot.

**Key behaviors:**
- Generates a per-boot key, opens the plain-mode mapper, and creates the filesystem before the partition grow, so the
unit waits only on the raw device node, not the by-partlabel symlink recreated by resize
- Grows the partition (`systemd-repart`) and resizes the mapper (`rottweiler resize block-device`)
- Deletes the key (`ExecStartPost=rottweiler delete-key`)
- Detaches on shutdown (`ExecStop`)

**Dependencies:**
- After: `dev-disk-by-partlabel-BOTTLEROCKET-DATA.device`, `cryptsetup-pre.target`, `systemd-udevd-kernel.socket`, `tpm2.target`
- Before: `cryptsetup.target`, `blockdev@dev-mapper-BOTTLEROCKET-DATA.target`
- Required by: `local-fs.target`

#### encrypt-unlock-private-fs.service

**Purpose:** Encrypt and open the BOTTLEROCKET-PRIVATE partition on every boot.

**Key behaviors:**
- Generates a per-boot key and opens the plain-mode mapper
- Deletes the key (`ExecStartPost=rottweiler delete-key`)
- `RequiresMountsFor=/run/rottweiler` so the key lands on the tmpfs keystore, not a plain `/run`
- Detaches on shutdown (`ExecStop`)

**Dependencies:**
- After: `dev-disk-by-partlabel-BOTTLEROCKET-PRIVATE.device`, `cryptsetup-pre.target`, `systemd-udevd-kernel.socket`, `tpm2.target`
- Before: `cryptsetup.target`, `blockdev@dev-mapper-BOTTLEROCKET-PRIVATE.target`
- Required by: `prepare-private-fs.service`

#### encrypt-unlock-datastore.service

**Purpose:** Encrypt and unlock `/.bottlerocket/datastore` on every boot, replacing the `encrypt-datastore.service` / `unlock-datastore.service` pair.

**Key behaviors:**
- Seals a per-boot key into the tmpfs keystore, sets the fscrypt policy, and unlocks the directory
- Deletes the key (`ExecStartPost=rottweiler delete-key`)
- `RequiresMountsFor=/.bottlerocket /run/rottweiler`

**Dependencies:**
- Before: `migrator.service`, `storewolf.service`
- Required by: `migrator.service`, `storewolf.service`

#### prepare-local-fs.service (modified)

**Drop-in:** `prepare-local-fs-ephemeral.conf`

**Changes:**
- No-op: filesystem is created by `encrypt-unlock-local-fs.service`
- `BindsTo` `encrypt-unlock-local-fs.service`

#### local.mount (modified)

**Drop-in:** `local.mount.d/10-ephemeral.conf` (reuses the `local-mount-encrypted.conf` source; the mapper name is identical in both modes)

**Changes:**
- Mounts `/dev/mapper/BOTTLEROCKET-DATA` instead of raw partition

#### .bottlerocket.mount (modified)

**Drop-in:** `bottlerocket-mount-ephemeral.conf`

**Changes:**
- Mounts `/dev/mapper/BOTTLEROCKET-PRIVATE` instead of raw partition
- Requires `prepare-private-fs.service`

#### repart-local.service (modified)

**Drop-in:** `repart-local-ephemeral.conf`

**Changes:**
- Masks the base `systemd-repart` grow (done in `encrypt-unlock-local-fs.service`)
- Keeps `systemd-growfs /local` to grow the filesystem to the resized mapper

## TPM2 Measurements

Bottlerocket extends TPM2 PCRs at various boot stages to establish a cryptographic chain of trust.

### PCR 8: User Data and OS Settings

PCR 8 is extended exactly twice per boot, in this order.

#### EC2 user data

**Service:** `measure-user-data.service`

**What's measured:** The user data is parsed the same way early-boot-config reads it (optional gzip, then TOML), then measured as the literal prefix `ec2-imds-user-data:v2:` followed by the OLPC canonical JSON of the whole document and a newline, with `settings.aws.region` removed and the registry removed from container image sources (see below). User data that is not TOML is measured as `ec2-imds-user-data:v1:` followed by its raw bytes, as is TOML containing a float (OLPC canonical JSON has no float encoding; note that `settings.kubernetes.memory-throttling-factor` is a float, though Kubernetes variants do not ship these measurement units) or a datetime (never a valid Bottlerocket setting). Absent user data is measured as the literal `ec2-imds-user-data:v1:absent`.

**When:** After `tpm2.target`, before `early-boot-config.service` and `settings-applier.service`

**Purpose:** Bind the configuration supplied at launch before it is applied

#### OS settings

**Service:** `measure-settings.service`

**What's measured:** OLPC canonical JSON of the settings tree, minus `settings.updates.seed`, `settings.network.hostname`, and `settings.aws.region`, followed by a newline

**When:** After `settings-applier.service` and `apiserver.service`, before `bootstrap-commands.service`

**Purpose:** Detect unauthorized configuration changes

**Note:** This measurement occurs before any external configuration can be applied and before any external code can run.

#### Region

In both extensions `settings.aws.region` is removed before hashing. On AWS variants early-boot-config sets it from the instance identity document on every host, so measuring it would give an otherwise identical fleet a different PCR 8 value in every region. The region is therefore not attested by PCR 8; an attestation consumer (for example a KMS key policy on `NitroTPMPCR8`) learns the caller's region from the request context instead. (`settings.updates.seed` and `settings.network.hostname` continue to be excluded from the settings extension only, as before.)

#### Container image sources

In both extensions the registry component is removed from every `settings.host-containers.*.source` and `settings.bootstrap-containers.*.source` before hashing: `<registry>/repo[:tag][@digest]` is measured as `repo[:tag][@digest]`. The first path component is treated as a registry when it contains a `.` or a `:` or is `localhost` (the same rule Docker and containerd use). The registry an image is pulled from is a deployment detail — Bottlerocket renders region-specific ECR registries into the default admin and control sources, and operators replicate images to per-region or per-account registries — so measuring it would give the same image a different PCR 8 value in every region or account. Pin sources with a digest so the measurement identifies the image content. The stored settings are not modified; only the measured bytes are.

#### Recomputing PCR 8

No TPM event log is written for these extensions, so a verifier recomputes the expected value from the same inputs:

```
user_data = "ec2-imds-user-data:v2:" + <canonical JSON of the user-data TOML, aws.region removed, registries removed> + "\n"
            # or "ec2-imds-user-data:v1:" + <raw user data> if it is not TOML, or "ec2-imds-user-data:v1:absent"
settings  = <canonical settings JSON minus updates.seed, network.hostname and aws.region, registries removed> + "\n"
PCR8      = SHA256( SHA256( 0x00 * 32 || SHA256(user_data) ) || SHA256(settings) )
```

The same construction applies to the SHA-384 and SHA-512 banks with the corresponding hash and initial value length.

### PCR 9: Kernel Command Line

**Service:** `measure-cmdline.service`

**What's measured:** Contents of `/proc/cmdline` (includes dm-verity root hash)

**When:** Early boot, before `sysinit.target`

**Purpose:** Verify boot parameters and userspace integrity

**Note:** While the kernel normally performs this measurement, Bottlerocket measures from userspace to capture the final command line after bootconfig customization is applied.

### PCR 11: Boot Phases

**Services:**
- `systemd-pcrphase-sysinit.service`
- `systemd-pcrphase-preconfigured.service`
- `systemd-pcrphase-configured.service`
- `systemd-pcrphase-multi-user.service`

**What's measured:** Boot phase strings (`sysinit`, `preconfigured`, `configured`, `ready`, `shutdown`, `final`)

**Purpose:** Track boot progression and establish different trust levels at different stages

**Phase progression:**
```
sysinit → preconfigured → configured → ready → shutdown → final
```

Each phase extends PCR 11 with the phase name as raw bytes (no newline, no null terminator).

**Security model:** Keys sealed to PCR 11 can only be unsealed if the system has not advanced beyond the boot phase during which they were generated. For example:

- **Local storage and datastore keys** are generated before `sysinit` completes, so they can never be unsealed after the `sysinit` phase on any boot
- This provides time-limited access: keys are only accessible during early boot when needed
- After initial setup, keys become permanently inaccessible, reducing attack surface
- Ephemeral storage keys are sealed to the phase when first initialized (preconfigured, configured, or multi-user), preventing initialization from being moved to an earlier phase

## Implementation Details

### Key Generation

Keys are generated using `/dev/random` (blocking, cryptographically secure):

```rust
let mut random_bytes = vec![0u8; 64];
fs::File::open("/dev/random")?.read_exact(&mut random_bytes)?;
```

### Key Encryption

Keys are encrypted using `systemd-creds` with TPM2 PCR binding:

```bash
systemd-creds encrypt - - \
  --name <key-id> \
  --with-key=tpm2 \
  --tpm2-pcrs=7+11+14  # or 4+7+9+11+14 if updates disabled
```

### LUKS2 Formatting

Block devices are formatted with minimal PBKDF2 iterations (1000) since keys are already high-entropy:

```bash
cryptsetup luksFormat \
  --type luks2 \
  --pbkdf pbkdf2 \
  --pbkdf-force-iterations 1000 \
  --batch-mode \
  <device> -
```

This matches systemd's behavior and avoids unnecessary key stretching.

### Plain Mode Formatting

With `ephemeral-encryption-keys`, block devices are opened in plain mode. No header is written,
so encryption and attach are a single operation, and `--hash plain` uses the key bytes verbatim:

```bash
cryptsetup open \
  --type plain \
  --cipher aes-xts-plain64 \
  --key-size 512 \
  --hash plain \
  --key-file=- \
  --keyfile-size=64 \
  <device> <name>
```

### fscrypt Configuration

Directories are encrypted with:
- **Contents encryption:** AES-256-XTS
- **Filenames encryption:** AES-256-CTS
- **Padding:** 32 bytes
- **Policy version:** v2

Key identifiers are derived using HKDF-SHA512:

```rust
let hkdf = Hkdf::<Sha512>::new(None, key);
hkdf.expand(b"fscrypt\x00\x01", &mut identifier)?;
```

### PCR Extension

PCRs are extended with SHA256, SHA384, and SHA512 hashes:

```bash
tpm2_pcrextend <pcr>:sha256=<hash>,sha384=<hash>,sha512=<hash>
```

## Security Considerations

### Threat Model

**Protects against:**
- Data theft from powered-off systems (disk removal)
- Unauthorized boot configurations (via PCR binding)
- Tampering with kernel, userspace, or boot parameters
- Unauthorized configuration changes (via PCR 8)

**Does not protect against:**
- Physical attacks on running systems (keys in memory)
- Firmware-level compromises before measurements
- Physical TPM attacks (requires specialized equipment)
- Cold boot attacks (DRAM remanence)
- Substitution of a host or bootstrap container image behind an unchanged tag (see below)

#### Container image sources in PCR 8

PCR 8 measures host and bootstrap container sources **without their registry** (see [PCR 8](#pcr-8-user-data-and-os-settings)). The registry is therefore not attested: an image pulled from a different registry, account, or region produces the same PCR 8 value as long as its repository, tag, and digest are the same. This is intentional, so that one expected value covers a fleet, but it means PCR 8 cannot tell where an image came from.

A tag is a mutable pointer. Whoever controls the registry a source refers to can serve different content under the same `repo:tag` without changing PCR 8. A digest cannot be re-pointed: `repo@sha256:...` identifies exactly one image, and containerd verifies the content it stores against that digest when pulling.

**Recommendation: pin custom host container and bootstrap container sources by digest**, for example:

```toml
[settings.host-containers.agent]
source = "123456789012.dkr.ecr.us-west-2.amazonaws.com/agent@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
```

which PCR 8 measures as `agent@sha256:0123…`. With a digest-pinned source, PCR 8 attests the exact image content that will run, independent of the registry it is replicated to. Tag-only sources attest only the name under which an image was requested. The Bottlerocket admin and control containers are a trusted dependency of the OS and are referenced by tag in the default settings; operators who want them attested by content can override their `source` with a digest-pinned reference as well.

### Key Security

- Keys never touch disk in plaintext
- Keys are zeroized after use (Rust `ZeroizeOnDrop`)
- Keys are only accessible when TPM PCRs match expected values
- Keystore directory has restrictive permissions (UMask=0077)

### Update Handling

**With in-place updates enabled:**
- Keys bound to PCRs 7+11+14 (excludes kernel/userspace measurements)
- Kernel and userspace updates work without re-encryption
- Still protected by Secure Boot (PCR 7) and boot phases (PCR 11)

**With in-place updates disabled:**
- Keys bound to PCRs 4+7+9+11+14 (includes kernel/userspace measurements)
- Any kernel or userspace change breaks decryption
- Provides strongest security but requires re-encryption for updates

### Recovery

If TPM PCR values change unexpectedly (e.g., firmware update, hardware change):
- Encrypted data becomes inaccessible
- No built-in recovery mechanism
- Requires backup/restore or data loss

**Mitigation strategies:**
- Test updates in non-production environments
- Maintain backups of critical data
- Consider using PCR policies that allow updates (7+11+14)

## Verification

### Check Encryption Status

```bash
# Check if block device is encrypted
rottweiler check block-device /dev/disk/by-partlabel/BOTTLEROCKET-DATA encrypted

# Check if directory is encrypted
rottweiler check directory /.bottlerocket/datastore encrypted
```

### Verify TPM PCR Values

```bash
# Read current PCR values
tpm2_pcrread sha256:4,7,9,11,14

# View PCR event log
tpm2_eventlog /sys/kernel/security/tpm0/binary_bios_measurements
```

### Check Key Binding

```bash
# List keys in keystore
ls -la /.bottlerocket/keystore/

# Attempt to decrypt a key (requires matching PCR values)
systemd-creds decrypt /.bottlerocket/keystore/bottlerocket-data - --name bottlerocket-data
```

### Verify LUKS Configuration

```bash
# Show LUKS header information
cryptsetup luksDump /dev/disk/by-partlabel/BOTTLEROCKET-DATA

# Check LUKS status
cryptsetup status BOTTLEROCKET-DATA
```

### Verify fscrypt Configuration

```bash
# Check filesystem encryption support
tune2fs -l /dev/disk/by-partlabel/BOTTLEROCKET-PRIVATE | grep encrypt

# Show directory encryption policy
rottweiler check directory /.bottlerocket/datastore encrypted
```

## References

### External Documentation

- [systemd-creds(1)](https://www.freedesktop.org/software/systemd/man/systemd-creds.html) - Credential encryption
- [cryptsetup(8)](https://man7.org/linux/man-pages/man8/cryptsetup.8.html) - LUKS management
- [TPM2 Tools](https://github.com/tpm2-software/tpm2-tools) - TPM2 utilities
- [Linux TPM PCR Registry](https://uapi-group.org/specifications/specs/linux_tpm_pcr_registry/) - PCR definitions

### Source Code

- `sources/rottweiler/` - Storage encryption helper implementation
- `sources/bottlerocket-image-features/` - Parses `encrypted-storage` and `ephemeral-encryption-keys` image features
- `sources/api/apiserver/src/server/ephemeral_storage.rs` - Encrypts `EPHEMERAL-DATA` for `apiclient ephemeral-storage init`
- `packages/release/release.spec` - Service packaging; units split into `release-crypt` (mode-independent), `release-crypt-luks` (LUKS mode), and `release-ephemeral-crypt` (plain mode)
- `packages/release/encrypt-*.service` - Encryption services
- `packages/release/unlock-*.service` - Unlocking services
- `packages/release/encrypt-unlock-*.service` - Combined plain-mode encryption and unlock services
- `packages/release/run-rottweiler.mount` - temporary tmpfs keystore at `/run/rottweiler`
- `packages/release/measure-*.service` - Measurement services
- `packages/release/systemd-pcrphase-*.service` - Boot phase measurements
