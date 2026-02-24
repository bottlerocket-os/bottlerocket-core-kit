# image-verifiers

Current version: 0.1.0

## Image Verifiers

Container image verification plugins for containerd's image verification interface.

### Overview

This crate provides verifier binaries that implement the containerd image
verification plugin interface. Containerd invokes these binaries before allowing
an image pull, passing the image reference and digest via command-line flags.

Both verifiers support Go-style single-hyphen flags (`-name` instead of `--name`)
for compatibility with containerd's invocation.

### Binaries

#### notation-image-verifier

Verifies container image signatures using the [notation](https://notaryproject.dev/) CLI.

```
notation-image-verifier -name <image-ref> -digest <sha256:hash>
```

**Trust Policy:** `/etc/containerd/image-verifiers/notation/trustpolicy.json`

```json
{
  "version": "1.0",
  "trustPolicies": [
    {
      "name": "example-tp",
      "registryScopes": ["*"],
      "signatureVerification": { "level": "strict" },
      "trustStores": ["signingAuthority:example-ts"],
      "trustedIdentities": ["*"]
    }
  ]
}
```

See the [notation trust policy spec](https://github.com/notaryproject/specifications/blob/main/specs/trust-store-trust-policy.md) for details.

If no trust policy is configured, all images are allowed. This permits enabling
only digest-based verification without signature verification.

#### digestion-image-verifier

Verifies container image digests against an allowlist.

```
digestion-image-verifier -name <image-ref> -digest <sha256:hash>
```

**Trust Policy:** `/etc/containerd/image-verifiers/digestion/trustpolicy.json`

```json
{
  "version": "1.0",
  "trustedDigests": ["sha256:abc123...", "sha256:def456..."]
}
```

If no trust policy is configured, all images are allowed.

#### thar-be-image-verifiers

Writes per-plugin trust policy files from TOML config. Reads TOML from
`/etc/thar-be-image-verifiers.toml`, decodes base64 trustpolicies, and writes
atomically to `/etc/containerd/image-verifiers/<plugin>/trustpolicy.json`.

This binary is invoked as an ExecStartPre command for containerd.

### Exit Codes

- `0`: Verification passed or skipped (no policy configured)
- `1`: Verification failed

### Configuration

Verifiers are configured via `settings.image-verifier-plugins`:

```toml
[settings.image-verifier-plugins]
enabled = true

[settings.image-verifier-plugins.notation]
trustpolicy = "<base64-encoded-json>"

[settings.image-verifier-plugins.digestion]
trustpolicy = "<base64-encoded-json>"

[settings.image-verifier-plugins.my-custom-verifier]
trustpolicy = "<base64-encoded-json>"
```

Any plugin name can be used - the config-helper writes trust policies for all
configured plugins.

## Colophon

This text was generated from `README.tpl` using [cargo-readme](https://crates.io/crates/cargo-readme), and includes the rustdoc from `src/lib.rs`.
