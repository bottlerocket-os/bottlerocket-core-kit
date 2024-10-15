# aws-smithy-experimental

See [`aws-smithy-experimental`](https://github.com/smithy-lang/smithy-rs/tree/42751e5dbf4d51c06c085e4193bf013a7333a6f5/rust-runtime/aws-smithy-experimental)


## Changes
- Remove `examples` and `tests` directories
- Remove `external-types.toml`
- Remove `examples` section in `Cargo.toml`
- Remove `package.metadata` section in `Cargo.toml`
- Remove `package.repository` in `Cargo.toml`
- Prevent crate from being published with `publish = false` in `Cargo.toml`
- Use workspace dependencies wherever possible
- Add linting rule to warn for missing `crypto_unstable` flag
