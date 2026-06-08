# Partition-Omitting Image Features

Bottlerocket exposes two opt-in image features that omit on-disk
partitions, intended primarily for diskless / minimal deployments
where the operator attaches persistent storage at runtime:

- `no-data-partitions` — omits the `BOTTLEROCKET-DATA-{A,B}` filesystems.
- `no-private-partition` — omits the `BOTTLEROCKET-PRIVATE` filesystem.

Both flags default to **off**. With neither flag set, behavior is
identical to a stock Bottlerocket image — there is no behavioral diff.

## What gets omitted

| Image feature           | GPT entries dropped              | Filesystem normally mounted at |
|-------------------------|----------------------------------|--------------------------------|
| `no-data-partitions`    | `BOTTLEROCKET-DATA-A`, `-B`      | `/local`                       |
| `no-private-partition`  | `BOTTLEROCKET-PRIVATE`           | `/var/lib/bottlerocket`        |

The rootfs is built with `dm-verity` and is mounted read-only, so the
in-rootfs paths under `/var`, `/opt`, `/var/lib/bottlerocket`, and `/mnt`
are unwritable. **Writable storage at those paths is mandatory** for the
API server, datastore, container runtimes, and orchestrator to function.
When a partition is omitted, the operator is responsible for attaching a
labeled persistent device before boot. There is no in-image fallback; if
no device with the matching `PARTLABEL=` is attached, the units that
mount and prepare these filesystems will fail. This is the expected
failure mode for a misconfigured deployment.

## Operator-attached persistent storage

Operators attach persistent storage at runtime by exposing a virtio-blk
device with the correct partition label.

For `/local`:

1. Create a partition table on the backing device with at least one
   partition labeled `BOTTLEROCKET-DATA`.
2. Attach the device to the Firecracker VM as a virtio-blk drive before
   boot.

On boot, `/dev/disk/by-partlabel/BOTTLEROCKET-DATA` exists, and:

- `prepare-local-fs.service` runs `systemd-makefs` on the device,
  formatting it if empty.
- `local.mount` mounts the partition at `/local`.
- The standard `/local/{var,opt,mnt}` bind pattern (`var.mount`,
  `opt.mount`, `mnt.mount`) takes over from there.

For `/var/lib/bottlerocket`: same pattern with label
`BOTTLEROCKET-PRIVATE`. The partition-backed `bottlerocket.mount`
(mounting `/.bottlerocket`) and `var-lib-bottlerocket.mount` (rbinding
`/.bottlerocket` → `/var/lib/bottlerocket`) handle the rest.

Operators must attach storage **before** boot. Hot-plugging a labeled
partition device into a running VM is not supported; partition discovery
and mount activation happen during early boot.

### Failure mode when no device is attached

If a build with `no-data-partitions` (or `no-private-partition`) boots
without a matching labeled device:

- `local.mount` / `bottlerocket.mount` will fail to find their
  `What=/dev/disk/by-partlabel/...` source and report a mount failure
  to the journal.
- Dependent units (`var.mount`, `opt.mount`, the datastore, the API
  server) will not start.
- `local-fs.target` and `preconfigured.target` will not reach
  isolation; the system will not finish booting.

This is intentional. These builds are designed for environments where
operator-attached storage is part of the deployment contract; failing
to provide it is a configuration error, and the resulting boot failure
surfaces it loudly rather than masking it with an ephemeral fallback.

## Incompatibilities

### `encrypted-storage`

The `encrypted-storage` image feature relies on **both**
`BOTTLEROCKET-DATA` (LUKS device) and `BOTTLEROCKET-PRIVATE` (datastore
directory and keystore). It is incompatible with `no-data-partitions`
and `no-private-partition`.

This is enforced two ways:

1. **Build time.** `release-crypt` declares
   `Conflicts: image-feature(no-data-partitions)` and
   `Conflicts: image-feature(no-private-partition)`. RPM dependency
   resolution fails with a clear conflict if a variant tries to enable
   both.
2. **Runtime.** Even if `ENCRYPTED_STORAGE=true` is set on a build that
   omits the partitions, `apiserver`'s ephemeral-storage logic returns
   `should_encrypt = false` when either `NO_DATA_PARTITIONS` or
   `NO_PRIVATE_PARTITION` is true, so the encryption flow is skipped.

Operator-attached persistent storage on a `no-data-partitions` /
`no-private-partition` build is **plaintext only**. There is no
TPM-backed unlocking path on these builds.

See [`ENCRYPTED_STORAGE.md`](ENCRYPTED_STORAGE.md) for the full encrypted
storage design.

## Implementation summary

- The two image features add image-feature flags only; no in-image
  fallback units are shipped.
- `apiserver` is taught to read the flags from
  `/usr/share/bottlerocket/image-features.env` and force
  `should_encrypt = false` when either is set.
- `release-crypt` declares `Conflicts:` against both image features.
- Standard unit files (`local.mount`, `bottlerocket.mount`,
  `prepare-local-fs.service`, `repart-*`, `encrypt-*`, `unlock-*`,
  `opt-{civ,cni,csi}.mount`, `lib-modules.mount`, kernel-devel mounts)
  are unchanged from a stock build. They mount or fail based on the
  presence of the `PARTLABEL=` device at boot.
