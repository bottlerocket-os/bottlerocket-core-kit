# thar-be-registries

Current version: 0.1.0

## Background

thar-be-registries generates containerd registry configuration from Bottlerocket settings.

It reads `/etc/containerd/thar-be-registries.toml` and writes per-registry configuration files to
`/etc/containerd/certs.d/`.

For each configured registry, it creates:
* `hosts.toml` - mirror endpoints with pull/resolve capabilities
* `credentials.toml` - authentication credentials (mode 0600)

### Behavior

* Exits successfully (0) if the input file doesn't exist (graceful no-op)
* Uses atomic directory replacement to avoid race conditions with containerd
* containerd reads these files on-demand during image pulls


## Colophon

This text was generated using [cargo-readme](https://crates.io/crates/cargo-readme), and includes the rustdoc from `src/main.rs`.
