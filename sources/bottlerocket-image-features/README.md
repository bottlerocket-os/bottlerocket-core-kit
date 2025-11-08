# bottlerocket-image-features

Current version: 0.1.0

## Introduction

*bottlerocket-image-features* is a library for parsing Bottlerocket image feature flags
from the system configuration file.

### Overview

This crate provides functionality to read and parse the `/usr/share/bottlerocket/image-features.env`
file, which contains feature flags that control various aspects of the Bottlerocket system.

### Features

Currently supported feature flags:

- `IN_PLACE_UPDATES` - Controls whether in-place updates are enabled (default: true)
- `ENCRYPTED_STORAGE` - Controls whether encrypted storage is enabled (default: false)

### Usage

```rust
use bottlerocket_image_features::parse_image_features;

let features = parse_image_features()?;
if features.in_place_updates {
    println!("In-place updates are enabled");
}
```

### File Format

The image features file uses a simple key=value format with support for:
- Comments (lines starting with `#`)
- Empty lines (ignored)
- Quoted or unquoted values
- Boolean values ("true" or "false")

Example:
```
# Image feature configuration
IN_PLACE_UPDATES="true"
```

## Colophon

This text was generated using [cargo-readme](https://crates.io/crates/cargo-readme), and includes the rustdoc from `src/lib.rs`.
