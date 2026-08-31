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

### Aliases

For convenience, the following aliases are supported:
- `dir` can be used instead of `directory`
- `bdev` can be used instead of `block-device`

## Colophon

This text was generated using [cargo-readme](https://crates.io/crates/cargo-readme), and includes the rustdoc from `src/main.rs`.
