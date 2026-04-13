# corgid

Current version: 0.1.0

## Overview

corgid is a Bottlerocket host-side service that sends a CycloneDX SBOM (Software Bill of Materials) to the Amazon Inspector API on every boot.

This supports the latest version of Amazon Inspector, which requires agent-based SBOM submission rather than relying solely on SSM manifests.

## How it works

1. Reads the Bottlerocket application inventory from `/usr/share/bottlerocket/application-inventory.json`
2. Converts it to a CycloneDX 1.5 SBOM
3. Fetches instance metadata (region, instance ID) and IAM credentials from IMDS
4. Sends the SBOM to the `inspector2` API using a session-based protocol:
   - `StartSession` — begins a scan session
   - `SendTelemetry` — uploads gzip-compressed SBOM chunks
   - `StopSession` — closes the session with status
5. All API requests are signed with SigV4

## Deployment

- Runs as a `Type=oneshot` systemd service (`corgid.service`)
- Starts after `multi-user.target` to ensure network and other services are available
- Non-blocking: boot continues regardless of whether corgid succeeds or fails
- Included only on AWS variants (`variant-platform(aws)`)
- Supports FIPS variants via the `fips` feature flag

## Fallback behavior

- If IMDS returns no region (e.g., Snowball), falls back to `us-east-1`
- If IMDS is unreachable, returns an error
- If SBOM upload fails, the session is still closed cleanly

## Colophon

This file was generated using [cargo-readme](https://crates.io/crates/cargo-readme), and target content is in `src/main.rs`.
