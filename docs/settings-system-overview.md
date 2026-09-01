# Bottlerocket Settings System Overview

This document describes how Bottlerocket's settings system works, including storage, organization, and the flow from settings to configuration files.

**Keywords:** settings, storage, datastore, filesystem, persistent, /var/lib/bottlerocket, key-value, live, pending, transactions, /etc, tmpfs, configuration, templates, defaults, immutable

## Overview

Bottlerocket uses a filesystem-based key/value data store as the central storage location for all settings <sup>[1]</sup>.
This architecture separates persistent settings from generated configuration files, enabling reliable updates and helping to prevent configuration drift.

**Key principles:**

- Settings are stored persistently in `/var/lib/bottlerocket/datastore/` <sup><sup>[1]</sup><sup>[2]</sup></sup>
- Configuration files in `/etc` are non-persistent and regenerated on every boot <sup><sup>[3]</sup></sup>
- Templates stored on the immutable root filesystem define how settings become configuration files <sup><sup>[4]</sup></sup>
- All settings changes go through the API and are tracked in transactions <sup><sup>[1]</sup></sup>

## Data Store Location and Structure

### Filesystem Layout

```
/var/lib/bottlerocket/datastore/
├── current -> v1/              # Symlink to active data store version
└── v1/                         # Versioned data store
    ├── live/                   # Committed, active settings
    │   └── settings/
    │       ├── motd            # Setting value (JSON)
    │       ├── motd.affected-services  # Metadata
    │       └── kubernetes/
    │           └── cluster-name
    └── pending/                # Uncommitted transactions
        └── <transaction-name>/
            └── settings/
```

The data store is located at `/var/lib/bottlerocket/datastore/` <sup>[1]</sup><sup>[2]</sup>.
The `current` symlink points to the active version (e.g., `v1`), allowing the data store format to evolve over time <sup>[5]</sup>.

### Live vs. Pending

**live/** - Contains committed settings that are active and available to the system <sup>[1]</sup><sup>[6]</sup>.
Services and configuration file rendering use settings from this directory.

**pending/** - Contains uncommitted settings organized by transaction name <sup>[1]</sup><sup>[6]</sup>.
Settings remain here until explicitly committed, allowing atomic updates of multiple related settings.

The most common transaction is `bottlerocket-launch`, which coordinates settings from multiple boot-time services <sup>[6]</sup>.

## Key-to-Path Mapping

The data store maps dotted key names to filesystem paths <sup>[2]</sup><sup>[7]</sup>:

| Key                                      | Filesystem Path                          |
| ---------------------------------------- | ---------------------------------------- |
| `settings.motd`                          | `settings/motd`                          |
| `settings.kubernetes.cluster-name`       | `settings/kubernetes/cluster-name`       |
| `settings.host-containers.admin.enabled` | `settings/host-containers/admin/enabled` |

**Implementation details:**

- Each key segment (separated by `.`) becomes a directory or file component <sup>[7]</sup>
- Segments are percent-encoded for filesystem safety <sup>[7]</sup>
- Only alphanumeric characters, underscores, and hyphens are allowed unencoded <sup>[7]</sup>
- Values are stored as JSON for human readability <sup>[2]</sup>

**Example:**

```bash
# Setting the key
apiclient set settings.motd="Welcome to Bottlerocket!"

# Results in file
/var/lib/bottlerocket/datastore/current/live/settings/motd
# Containing
"Welcome to Bottlerocket!"
```

## Metadata Storage

Metadata about settings is stored alongside the data using a dot-prefix pattern <sup>[7]</sup>:

```
settings/motd                      # The setting value
settings/motd.affected-services    # Metadata: which services to restart
settings/motd.template-path        # Metadata: template location
```

Metadata files use the pattern `<data-file>.<metadata-key>` <sup>[7]</sup>.
This allows the system to track additional information about each setting, such as:

- Which services are affected by changes (`affected-services`)
- Which configuration files use this setting (`configuration-files`)
- How to generate the setting value (`setting-generator`)

## Settings Lifecycle

### 1. Default Values

Default settings are defined in variant-specific `defaults.d` directories <sup>[1]</sup><sup>[8]</sup>.
These TOML files specify initial values and configuration file mappings.

**Example from defaults:**

```toml
[settings]
motd = "Welcome to Bottlerocket!"

[configuration-files.motd]
path = "/etc/motd"
template-path = "/usr/share/templates/motd"
```

### 2. Data Store Population

On first boot, `storewolf` creates the data store and populates it with default values <sup>[1]</sup><sup>[8]</sup>:

1. Creates `/var/lib/bottlerocket/datastore/` directory structure
1. Writes default settings to the `pending/bottlerocket-launch/` transaction
1. Settings remain uncommitted until `settings-committer` runs

### 3. Runtime Settings

Additional settings are added during boot by:

- **early-boot-config** - Applies user data from cloud providers (first boot only) <sup>[1]</sup>
- **sundog** - Generates settings that can only be determined at runtime (e.g., primary IP address) <sup>[1]</sup>
- **pluto** - Generates Kubernetes-specific settings (e.g., cluster DNS) <sup>[1]</sup>

All of these write to the `bottlerocket-launch` transaction in pending state.

### 4. Commit

`settings-committer` moves settings from `pending/bottlerocket-launch/` to `live/` <sup>[6]</sup>.
This makes them available to the rest of the system.
The commit is atomic - all settings in the transaction become live together.

### 5. Configuration File Generation

`thar-be-settings` renders configuration files from templates using committed settings <sup>[1]</sup><sup>[9]</sup>:

1. Reads settings from `live/` in the data store (via Bottlerocket API)
1. Loads templates from `/usr/share/templates/` (immutable root filesystem)
1. Renders templates using the `schnauzer` library <sup>[9]</sup>
1. Writes configuration files to `/etc` (tmpfs)
1. Restarts affected services as needed

### 6. Service Restart

Services are restarted based on metadata in the data store <sup>[1]</sup>.
The `affected-services` metadata for each setting determines which services need to be restarted when that setting changes.

## Persistence Model

Bottlerocket has three storage layers with different persistence characteristics:

### Persistent: Data Store

**Location:** `/var/lib/bottlerocket/datastore/`

**Characteristics:**

- Survives reboots and OS updates <sup>[1]</sup>
- Stored on the data partition (not the OS partition)
- Migrated automatically during OS upgrades <sup>[5]</sup>
- Only modified through the API

**What's stored:**

- All user-configured settings
- Generated settings (IP addresses, cluster information)
- Setting metadata

### Non-Persistent: Configuration Files

**Location:** `/etc` (tmpfs - memory-backed filesystem) <sup>[3]</sup>

**Characteristics:**

- Regenerated on every boot from templates <sup>[1]</sup><sup>[3]</sup>
- Changes do not survive reboot
- Cannot be directly modified by users <sup>[3]</sup>
- Provides protection against persistent configuration tampering

**What's stored:**

- Service configuration files (e.g., `/etc/containerd/config.toml`)
- Network configuration (e.g., `/etc/resolv.conf`)
- System configuration files

### Immutable: Templates and Defaults

**Location:** `/usr/share/templates/`, `/etc/storewolf/defaults.toml` (root filesystem) <sup>[1]</sup><sup>[8]</sup>

**Characteristics:**

- Read-only, dm-verity protected <sup>[3]</sup>
- Part of the OS image
- Updated only through OS upgrades
- Cannot be modified at runtime

**What's stored:**

- Configuration file templates
- Default setting values
- Template rendering logic

## Configuration Flow

The complete flow from settings to running services:

```
┌─────────────────┐
│ Variant         │
│ defaults.d/     │  Default values defined
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ storewolf       │  Populate data store
│ /var/lib/       │  (pending state)
│ bottlerocket/   │
│ datastore/      │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ settings-       │  Commit transaction
│ committer       │  (pending → live)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ thar-be-        │  Render templates
│ settings        │  using live settings
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ /etc/           │  Configuration files
│ (tmpfs)         │  written to memory
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Services        │  Services (re)started
│ (systemd)       │  with new configuration
└─────────────────┘
```

## User Interaction

```bash
# Change a setting
apiclient set settings.motd="Custom message"

# Read a single setting
apiclient get settings.motd

# Read all settings
apiclient get settings
```

## Related Documentation

- [API System Overview](../sources/api/README.md) - How API components work together
- [Boot Process](boot-process.md) - When settings are populated and applied during boot
- [apiserver](../sources/api/apiserver/README.md) - API server implementation
- [datastore](../sources/api/datastore/README.md) - Data store library implementation
- [storewolf](../sources/api/storewolf/README.md) - Data store initialization
- [thar-be-settings](../sources/api/thar-be-settings/README.md) - Configuration file rendering
- [schnauzer](../sources/api/schnauzer/README.md) - Template rendering library

## Sources

<sup>[1]</sup> [`sources/api/README.md`](../sources/api/README.md)

- API system overview and component workflow
- Data store description and boot process
- Settings lifecycle from defaults to services

<sup>[2]</sup> [`sources/api/apiserver/README.md`](../sources/api/apiserver/README.md)

- Data store key/value structure
- Default location: `/var/lib/bottlerocket/datastore/current`
- JSON serialization of values

<sup>[3]</sup> [`SECURITY_FEATURES.md`](https://github.com/bottlerocket-os/bottlerocket/blob/develop/SECURITY_FEATURES.md) (bottlerocket repo)

- Stateless tmpfs for `/etc`
- Immutable rootfs backed by dm-verity
- Configuration regeneration on boot
- Security implications of the storage model

<sup>[4]</sup> [`sources/api/schnauzer/README.md`](../sources/api/schnauzer/README.md)

- Template rendering with handlebars
- Settings extensions and helpers
- Template frontmatter format

<sup>[5]</sup> [`sources/api/migration/migrator/README.md`](../sources/api/migration/migrator/README.md)

- Data store versioning and migration
- Version detection from symlink naming

<sup>[6]</sup> [`sources/api/settings-committer/README.md`](../sources/api/settings-committer/README.md)

- Transaction commit process
- `bottlerocket-launch` transaction
- Pending to live transition

<sup>[7]</sup> [`sources/api/datastore/src/filesystem.rs`](../sources/api/datastore/src/filesystem.rs)

- Key-to-path encoding implementation
- Metadata storage with dot-prefix pattern
- `live/` and `pending/` directory structure
- Percent-encoding for filesystem safety

<sup>[8]</sup> [`sources/api/storewolf/README.md`](../sources/api/storewolf/README.md)

- Data store creation and initialization
- Default settings population from `/etc/storewolf/defaults.toml`

<sup>[9]</sup> [`sources/api/thar-be-settings/README.md`](../sources/api/thar-be-settings/README.md)

- Configuration file rendering from templates
- Service restart mechanism
