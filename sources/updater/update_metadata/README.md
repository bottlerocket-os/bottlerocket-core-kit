# Update Metadata Format

This crate defines the data structures for Bottlerocket's update manifest and metadata.

**Keywords:** update metadata, manifest.json, TUF targets, updates, migrations, waves, rollout, version, images, boot partition, root filesystem, dm-verity, schema, data structures

## Manifest Schema

The `manifest.json` file contains all available updates and is published as a TUF target in the update repository.

### Top-Level Structure

- `updates[]` - Array of Update objects describing available versions
- `migrations{}` - Map of version pairs to migration file lists

### Update Object

Each update in the manifest describes a specific Bottlerocket version:

- `variant` - Bottlerocket variant (e.g., "aws-k8s-1.31", "metal-k8s-1.31")
- `arch` - Architecture ("x86_64" or "aarch64")
- `version` - Update version (semver format)
- `max_version` - Maximum version this update can safely upgrade to
- `waves{}` - Release wave schedule mapping seed values to start timestamps
- `images` - Image file paths (see below)

### Images Object

Paths to update files, relative to the TUF repository's `targets_base_url`:

- `boot` - Boot partition image (typically `.ext4.lz4` compressed)
- `root` - Root filesystem image (typically `.ext4.lz4` compressed)
- `hash` - dm-verity hash tree (typically `.verity.lz4` compressed)

All image files must be listed in the TUF repository's `targets.json` with their cryptographic hashes.

### Example

```json
{
  "updates": [
    {
      "variant": "aws-k8s-1.31",
      "arch": "x86_64",
      "version": "1.20.0",
      "max_version": "1.20.0",
      "waves": {
        "0": "2024-01-15T00:00:00Z",
        "512": "2024-01-15T12:00:00Z",
        "1024": "2024-01-16T00:00:00Z",
        "2048": "2024-01-16T12:00:00Z"
      },
      "images": {
        "boot": "bottlerocket-aws-k8s-1.31-x86_64-v1.20.0-boot.ext4.lz4",
        "root": "bottlerocket-aws-k8s-1.31-x86_64-v1.20.0-root.ext4.lz4",
        "hash": "bottlerocket-aws-k8s-1.31-x86_64-v1.20.0-root.verity.lz4"
      }
    }
  ],
  "migrations": {
    "(1.19.0, 1.20.0)": ["migrate_v1.19.0_v1.20.0"],
    "(1.20.0, 1.19.0)": ["migrate_v1.19.0_v1.20.0"]
  }
}
```

## Migrations

The `migrations` map specifies migration binaries required when moving between versions. Each key is a tuple of `(from_version, to_version)`, and the value is a list of migration file names.

Migrations are bidirectional - the same migration binary typically handles both upgrade and downgrade paths between two versions.

## Release Waves

Update waves allow staged rollouts of new versions. The `waves` map associates seed ranges with start times:

- Hosts calculate their position in the wave based on their `settings.updates.seed` value (0-2048)
- Updates become available to a host only after its wave's start time has passed
- This prevents all hosts from updating simultaneously

See [waves/README.md](../waves/README.md) for more details on wave scheduling.
