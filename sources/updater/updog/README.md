# what is updog

not much what's up with you

**Keywords:** updog, updates, TUF client, check-update, apply updates, download, waves, version, upgrade, rollout, update command

## no really

The Updog client provides an interface to a TUF repository and prepares for, downloads, and applies updates to the Bottlerocket instance. Updog can be called manually, but will more commonly be called automatically by some cluster orchestrator. For usage run `updog --help`.

## Quick reference

### Check for the most recent update
```
# updog check-update
aws-k8s-1.15 0.1.4 (v0.0)
```

### List all available updates, including older versions
```
# updog check-update --all
aws-k8s-1.15 0.1.4 (v0.0)
aws-k8s-1.15 0.1.2 (v0.0)
aws-k8s-1.15 0.1.1 (v0.0)
```

### Specify JSON output
```
# updog check-update --json
[{"variant":"aws-k8s-1.15","arch":"x86_64","version":"0.1.4","max_version":"0.1.4","waves":{"512":"2019-10-03T20:45:52Z","1024":"2019-10-03T21:00:52Z","1536":"2019-10-03T22:00:52Z","2048":"2019-10-03T23:00:52Z"},"images":{"boot":"bottlerocket-x86_64-aws-k8s-1.15-v0.1.4-boot.ext4.lz4","root":"bottlerocket-x86_64-aws-k8s-1.15-v0.1.4-root.ext4.lz4","hash":"bottlerocket-x86_64-aws-k8s-1.15-v0.1.4-root.verity.lz4"}}]
```

### Try to update with wave information
```
# updog update
Update available at 2019-10-03 21:24:00 UTC
```
Once timestamp has passed:
```
# updog update --timestamp 2019-10-03T21:24:00+00:00
Starting update to 0.1.4
Update applied: aws-k8s-1.15 0.1.4
```

### Force an immediate update, ignoring wave limits
```
# updog update --now
Starting update to 0.1.4
** Updating immediately **
Update applied: aws-k8s-1.15 0.1.4
```

## Configuration

Updog reads its configuration from `/etc/updog.toml`. This file is typically rendered by the Bottlerocket API server from system settings.

### Configuration Format

```toml
# TUF repository metadata location (required)
metadata_base_url = "https://updates.bottlerocket.aws/2020-07-07/aws-k8s-1.31/x86_64/"

# TUF repository targets location (required)
targets_base_url = "https://updates.bottlerocket.aws/targets/"

# Update wave seed value (required, 0-2048)
# Determines when this host receives updates in a staged rollout
seed = 1234

# Version selection policy (required)
# Options: "latest", "1.2.3" (specific version), "^1.2" (semver range)
version_lock = "latest"

# Skip wave scheduling and update immediately (required)
ignore_waves = false

# HTTPS proxy for update downloads (optional)
https_proxy = "http://proxy.example.com:3128"

# Hosts to exclude from proxy (optional)
no_proxy = ["localhost", "169.254.169.254"]
```

### Configuration Fields

- **metadata_base_url** - Base URL for TUF metadata files (root.json, timestamp.json, snapshot.json, targets.json)
- **targets_base_url** - Base URL for update images and migration files. Image paths in manifest.json are relative to this URL
- **seed** - Integer from 0-2048 used to calculate this host's position in update waves
- **version_lock** - Controls which updates are considered: "latest" for newest available, specific version string, or semver range
- **ignore_waves** - If true, updates are applied immediately without waiting for wave schedule
- **https_proxy** - Optional HTTP proxy URL for downloading updates (overrides HTTPS_PROXY environment variable)
- **no_proxy** - Optional list of hosts to exclude from proxy (overrides NO_PROXY environment variable)
