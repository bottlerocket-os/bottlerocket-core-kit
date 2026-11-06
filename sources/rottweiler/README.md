# rottweiler

Current version: 0.1.0

## Introduction

*rottweiler* is Bottlerocket's storage encryption helper. It provides a unified
interface for encrypting and managing encrypted storage resources including:

- Block devices (using LUKS or plain mode encryption)
- Directories (using fscrypt)
- TPM PCR measurements

### Commands

#### Key Management
- `generate-key <key-id>` - Generate an encryption key
- `delete-key <key-id>` - Delete an encryption key from the keystore

#### Block Device Operations
- `encrypt block-device <path> <key-id>` - Encrypt a block device using LUKS
- `encrypt-and-attach block-device <path> <key-id>` - Encrypt (plain mode) and attach a block device in one step
- `attach block-device <path> <key-id>` - Attach an encrypted block device
- `detach block-device <path>` - Detach an encrypted block device
- `resize block-device <path> <key-id>` - Resize an encrypted block device to match the underlying device size
- `check block-device <path> encrypted|unencrypted` - Check block device encryption state

#### Directory Operations
- `encrypt directory <path> <key-id>` - Encrypt a directory using fscrypt
- `lock directory <path>` - Lock an encrypted directory (remove key)
- `unlock directory <path> <key-id>` - Unlock an encrypted directory (add key)
- `check directory <path> encrypted|unencrypted` - Check directory encryption state

#### TPM Measurement Operations
- `measure settings` - Measure OS settings into PCR 8
- `measure user-data` - Measure EC2 IMDS user data into PCR 8
- `measure kernel-command-line` - Measure kernel command line into PCR 9
- `measure pcrphase <phase>` - Measure boot phase into PCR 11
  - Valid phases: `sysinit`, `preconfigured`, `configured`, `ready`, `shutdown`, `final`

### PCR 8

PCR 8 is extended twice per boot: first with the EC2 IMDS user data (`measure user-data`, before
settings are applied), then with the OS settings (`measure settings`, after `settings-applier.service`).
Both are measured as OLPC canonical JSON with `settings.aws.region` removed and with the registry
removed from every host and bootstrap container image source (the settings extension also excludes
`settings.updates.seed` and `settings.network.hostname`, as before), so the same configuration yields
the same PCR 8 value regardless of the region or registry it is deployed to. User data that is not
TOML is measured as its raw bytes. The full description of
the measured bytes, the registry rule and its caveats, and how to recompute PCR 8 is in
[docs/ENCRYPTED_STORAGE.md](../../docs/ENCRYPTED_STORAGE.md#pcr-8-user-data-and-os-settings).

### Aliases

For convenience, the following aliases are supported:
- `dir` can be used instead of `directory`
- `bdev` can be used instead of `block-device`

## Colophon

This text was generated using [cargo-readme](https://crates.io/crates/cargo-readme), and includes the rustdoc from `src/main.rs`.
