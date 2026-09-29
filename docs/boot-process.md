# Bottlerocket Boot Process

This document describes how Bottlerocket boots, focusing on the systemd target progression and service dependencies.

**Keywords:** boot, systemd, targets, preconfigured, configured, multi-user, fipscheck, drivers, sysinit, services, dependencies, ordering, API system, bootstrap containers, settings, startup, initialization, kernel modules

## Overview

Bottlerocket's boot sequence progresses through six main stages, each represented by a systemd target:

```
sysinit.target
    ↓
fipscheck.target (FIPS mode only)
    ↓
drivers.target (kernel module loading)
    ↓
preconfigured.target (API system initialization)
    ↓
configured.target (bootstrap containers)
    ↓
multi-user.target (workload services)
```

Each stage must complete before the next begins. Services use systemd dependencies (`After=`, `Requires=`, `Wants=`) to coordinate within and between stages.

## Boot Stages

### Stage 0: sysinit.target

Standard systemd initialization. Most services implicitly depend on this through `DefaultDependencies=yes` (the default).

Early services that need to run before normal dependency chains use `DefaultDependencies=no`:

- Filesystem preparation (`prepare-var.service`, `prepare-boot.service`, etc.)
- Data store migration (`migrator.service`)

### Stage 1: fipscheck.target

**Purpose:** Verify cryptographic module integrity when FIPS mode is enabled.

**When it runs:** Only when the kernel command line includes `fips=1`.

**Key services:**

- `check-kernel-integrity.service` - Verifies kernel integrity
- `check-fips-modules.service` - Loads and tests the `tcrypt` module
  - Creates `/etc/.fips-module-check-passed` sentinel file on success
  - Blocks boot if FIPS checks fail

**Transition:** Completes before `drivers.target` begins.

### Stage 2: drivers.target

**Purpose:** Load kernel modules and hardware drivers.

**When it runs:** Always runs, after `basic.target` and before `preconfigured.target`.

**Key services:**

- `load-neuron-inf1-modules.service` - Loads AWS Neuron Inf1 kernel modules
- `load-neuron-latest-modules.service` - Loads AWS Neuron Latest kernel modules

**Dependencies:**

- Runs after `basic.target`
- Runs before `preconfigured.target`
- Required by `preconfigured.target` and `multi-user.target`

**Note:** Some driver loading services (e.g., NVIDIA GPU drivers) are required by `preconfigured.target` directly rather than using `drivers.target`.

**Transition:** Completes before `preconfigured.target` begins.

### Stage 3: preconfigured.target

**Purpose:** Initialize the API system and apply all boot-time configuration.

**What "preconfigured" means:** The system has:

- A populated data store with default and user-provided settings
- A running API server
- All configuration files generated from settings

This is the most complex boot stage. Services run in a specific order to build up the system configuration:

#### 3.1 Data Store Setup

**migrator** (`migrator.service`)

- **When:** Runs with `DefaultDependencies=no`, before everything else
- **What:** Updates data store schema if the OS version changed
- **Dependencies:** Required by `apiserver.service`, `storewolf.service`, and `preconfigured.target`

**storewolf** (`storewolf.service`)

- **When:** After `migrator.service`
- **What:** Creates data store directories and populates default settings
- **Details:**
  - Reads defaults from variant-specific `defaults.d` directories
  - Writes settings to _pending_ state in "bottlerocket-launch" transaction
  - Settings not available to other services until committed
- **Dependencies:** Required by `preconfigured.target`

#### 3.2 API Server

**apiserver** (`apiserver.service`)

- **When:** After `storewolf.service`
- **What:** Starts the API server on Unix socket `/run/api.sock`
- **Details:** Allows reading/writing settings via API
- **Dependencies:** Wanted by `preconfigured.target`

#### 3.3 Settings Population

**early-boot-config** (`early-boot-config.service`)

- **When:** After `network-online.target`, `apiserver.service`, `storewolf.service`
- **What:** Applies user data settings (cloud-init equivalent)
- **Details:**
  - Only runs on first boot (checks for `/var/lib/bottlerocket/early-boot-config.ran`)
  - Fetches user data from platform metadata service (e.g., EC2 IMDS)
  - PATCHes settings to API in _pending_ state (not committed)
- **Dependencies:** Required by `preconfigured.target`

**sundog** (`sundog.service`)

- **When:** After `network-online.target`, `apiserver.service`, `early-boot-config.service`
- **What:** Generates dynamic settings that can't be determined until runtime
- **Details:**
  - Examples: primary IP address, cluster DNS settings
  - Runs `settings-committer` first to access user data settings
  - PATCHes generated settings to API in _pending_ state
- **Dependencies:** Required by `preconfigured.target`
- **Subcomponent:** `pluto.service` generates Kubernetes-specific settings

#### 3.4 Configuration Application

**settings-applier** (`settings-applier.service`)

- **When:** After `storewolf.service`, `sundog.service`, `early-boot-config.service`, `apiserver.service`
- **What:** Writes all configuration files based on settings
- **Details:**
  - Runs `settings-committer` to commit the "bottlerocket-launch" transaction
  - Runs `thar-be-settings --all` to generate all config files
  - This is when pending settings become live
- **Dependencies:** Required by `preconfigured.target`

#### 3.5 Stage Transition

**activate-configured** (`activate-configured.service`)

- **When:** After `preconfigured.target` completes
- **What:** Transitions to `configured.target`
- **Details:**
  - Sets systemd default target to `configured.target`
  - Starts `configured.target` asynchronously
- **Dependencies:** Wanted by `preconfigured.target`

### Stage 4: configured.target

**Purpose:** Run bootstrap containers that perform additional system configuration.

**What "configured" means:** The system has:

- Completed all API-based configuration
- Run any user-defined bootstrap containers
- Applied any additional configuration from bootstrap containers

**Key services:**

**bootstrap-containers@** (`bootstrap-containers@.service`)

- **When:** After `host-containerd.service`, before `configured.target`
- **What:** Runs bootstrap containers defined in settings
- **Details:**
  - Template unit instantiated for each configured bootstrap container
  - Only runs once per container (checks for `/run/bootstrap-containers/%i.ran`)
  - Containers have access to host filesystem at `/.bottlerocket/rootfs`
  - Boot blocks until all bootstrap containers complete
  - Useful for: installing software, modifying files, running setup scripts
- **Dependencies:** Runs before `configured.target`

**activate-multi-user** (`activate-multi-user.service`)

- **When:** After `configured.target` and `reboot-if-required.service`
- **What:** Transitions to `multi-user.target`
- **Details:**
  - Sets systemd default target to `multi-user.target`
  - Starts `multi-user.target` asynchronously
- **Dependencies:** Wanted by `configured.target`

### Stage 5: multi-user.target

**Purpose:** Start workload services (kubelet, ECS agent, etc.).

**What "multi-user" means:** The system is fully configured and ready to run workloads.

**Key services:**

- `kubelet.service` (Kubernetes variants)
- `ecs.service` (ECS variants)
- `host-containers@admin.service` (admin container)
- `host-containers@control.service` (control container)

**Dependencies:**

- Requires `basic.target` and `configured.target`
- This ensures all configuration is complete before workloads start

## Service Dependency Patterns

### Ordering Dependencies

- `After=` - This service starts after the specified units
- `Before=` - This service starts before the specified units

### Requirement Dependencies

- `Requires=` - This service requires the specified units (hard dependency)
- `Wants=` - This service wants the specified units (soft dependency)
- `RequiredBy=` - Reverse of `Requires=` (specified in `[Install]` section)
- `WantedBy=` - Reverse of `Wants=` (specified in `[Install]` section)

### Early Boot Services

Services that need to run very early use `DefaultDependencies=no` to avoid the standard dependency chain:

- `migrator.service`
- `prepare-*.service` (filesystem preparation)
- `activate-preconfigured.service`

## Synchronization Mechanisms

### Systemd Targets

Targets serve as synchronization points. A target is "reached" when all services required by or wanted by that target have completed.

Target relationships:

```
drivers.target:
  Requires: basic.target
  RequiredBy: preconfigured.target, multi-user.target

preconfigured.target:
  Requires: basic.target
  RequiredBy: configured.target, multi-user.target

configured.target:
  Requires: preconfigured.target
  RequiredBy: multi-user.target

multi-user.target:
  Requires: basic.target, configured.target
```

### Sentinel Files

Services use sentinel files to track state across reboots:

- `/var/lib/bottlerocket/early-boot-config.ran` - Prevents `early-boot-config.service` from running after first boot
- `/run/bootstrap-containers/<name>.ran` - Prevents bootstrap containers from re-running
- `/etc/.fips-module-check-passed` - Marks FIPS check completion

Services use `ConditionPathExists=` or `ConditionPathExists=!` to check for these files.

### Transaction Commits

The API system uses transactions to ensure atomic updates:

1. Services write settings to _pending_ state during boot
2. Settings are grouped in the "bottlerocket-launch" transaction
3. `settings-committer` commits the transaction, making settings live
4. `settings-applier` then generates configuration files from live settings

This ensures all boot-time settings are applied together, preventing partial configuration.

## Boot Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│ sysinit.target                                                  │
│ - Standard systemd initialization                               │
│ - prepare-var.service, prepare-boot.service (early filesystem)  │
└────────────────────────────┬────────────────────────────────────┘
                             ↓
┌─────────────────────────────────────────────────────────────────┐
│ fipscheck.target (FIPS mode only)                               │
│ - check-kernel-integrity.service                                │
│ - check-fips-modules.service                                    │
└────────────────────────────┬────────────────────────────────────┘
                             ↓
┌─────────────────────────────────────────────────────────────────┐
│ drivers.target                                                  │
│ - load-neuron-inf1-modules.service (AWS Neuron Inf1)            │
│ - load-neuron-latest-modules.service (AWS Neuron Latest)        │
└────────────────────────────┬────────────────────────────────────┘
                             ↓
┌─────────────────────────────────────────────────────────────────┐
│ preconfigured.target                                            │
│                                                                 │
│ 1. migrator.service          (data store migration)             │
│ 2. storewolf.service         (data store creation)              │
│ 3. apiserver.service         (API server)                       │
│ 4. early-boot-config.service (user data, first boot only)       │
│ 5. sundog.service            (dynamic settings)                 │
│ 6. settings-applier.service  (commit & apply settings)          │
│                                                                 │
│ Result: API system running, all settings applied                │
└────────────────────────────┬────────────────────────────────────┘
                             ↓
                    activate-configured.service
                             ↓
┌─────────────────────────────────────────────────────────────────┐
│ configured.target                                               │
│                                                                 │
│ - bootstrap-containers@*.service (user-defined setup)           │
│                                                                 │
│ Result: Additional configuration complete                       │
└────────────────────────────┬────────────────────────────────────┘
                             ↓
                    activate-multi-user.service
                             ↓
┌─────────────────────────────────────────────────────────────────┐
│ multi-user.target                                               │
│                                                                 │
│ - kubelet.service / ecs.service (workload orchestrator)         │
│ - host-containers@admin.service (admin container)               │
│ - host-containers@control.service (control container)           │
│                                                                 │
│ Result: System ready for workloads                              │
└─────────────────────────────────────────────────────────────────┘
```

## Debugging Boot Issues

### Check Target Status

```bash
# Check if a target has been reached
systemctl is-active drivers.target
systemctl is-active preconfigured.target
systemctl is-active configured.target
systemctl is-active multi-user.target

# See what's blocking a target
systemctl list-dependencies drivers.target
systemctl list-dependencies preconfigured.target
systemctl list-dependencies --reverse preconfigured.target
```

### Check Service Status

```bash
# See all failed services
systemctl --failed

# Check specific service
systemctl status migrator.service
systemctl status apiserver.service

# View service logs
journalctl -u migrator.service
journalctl -u apiserver.service
```

### Boot Timeline

To see the boot timeline:

```bash
systemd-analyze
systemd-analyze blame
systemd-analyze critical-chain
```

## Related Documentation

- [API System](../sources/api/README.md) - Detailed API component documentation
- [Bootstrap Containers](../sources/api/bootstrap-containers/README.md) - Bootstrap container usage
- [Early Boot Config](../sources/early-boot-config/README.md) - User data configuration
- [Settings System](../sources/api/thar-be-settings/README.md) - Configuration file generation
