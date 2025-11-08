/*!
# Introduction

*bottlerocket-image-features* is a library for parsing Bottlerocket image feature flags
from the system configuration file.

## Overview

This crate provides functionality to read and parse the `/usr/share/bottlerocket/image-features.env`
file, which contains feature flags that control various aspects of the Bottlerocket system.

## Features

Currently supported feature flags:

- `IN_PLACE_UPDATES` - Controls whether in-place updates are enabled (default: true)
- `ENCRYPTED_STORAGE` - Controls whether encrypted storage is enabled (default: false)

## Usage

```rust,ignore
use bottlerocket_image_features::parse_image_features;

let features = parse_image_features()?;
if features.in_place_updates {
    println!("In-place updates are enabled");
}
```

## File Format

The image features file uses a simple key=value format with support for:
- Comments (lines starting with `#`)
- Empty lines (ignored)
- Quoted or unquoted values
- Boolean values ("true" or "false")

Example:
```text
# Image feature configuration
IN_PLACE_UPDATES="true"
```
*/

use serde::Deserialize;
use snafu::prelude::*;
use std::fs;

type Result<T> = std::result::Result<T, snafu::Whatever>;

const IMAGE_FEATURES_FILE: &str = "/usr/share/bottlerocket/image-features.env";

#[derive(Deserialize)]
pub struct ImageFeatures {
    #[serde(default = "default_true")]
    pub in_place_updates: bool,
    #[serde(default)]
    pub encrypted_storage: bool,
}

fn default_true() -> bool {
    true
}

/// Parse image features from the env file
pub fn parse_image_features() -> Result<ImageFeatures> {
    if !std::path::Path::new(IMAGE_FEATURES_FILE).exists() {
        return Ok(ImageFeatures {
            in_place_updates: true,
            encrypted_storage: false,
        });
    }

    let content = fs::read_to_string(IMAGE_FEATURES_FILE)
        .with_whatever_context(|_| format!("failed to read {}", IMAGE_FEATURES_FILE))?;

    parse_image_features_from_str(&content)
}

/// Parse image features from a string (useful for testing)
pub fn parse_image_features_from_str(content: &str) -> Result<ImageFeatures> {
    let pairs: Vec<(String, String)> = content
        .lines()
        .filter_map(|line| {
            if line.starts_with('#') || line.trim().is_empty() {
                return None;
            }
            let mut parts = line.splitn(2, '=');
            let key = parts.next()?;
            let mut value = parts.next()?;
            if value.starts_with('"') {
                value = &value[1..];
            }
            if value.ends_with('"') {
                value = &value[..value.len() - 1];
            }
            Some((key.to_owned(), value.to_owned()))
        })
        .collect();

    envy::from_iter(pairs).with_whatever_context(|_| "failed to parse image features")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_env_line_with_quotes() {
        let content = r#"IN_PLACE_UPDATES="false""#;
        let features = parse_image_features_from_str(content).unwrap();
        assert_eq!(features.in_place_updates, false);
    }

    #[test]
    fn test_parse_env_line_without_quotes() {
        let content = "IN_PLACE_UPDATES=false";
        let features = parse_image_features_from_str(content).unwrap();
        assert_eq!(features.in_place_updates, false);
    }

    #[test]
    fn test_parse_env_ignores_comments() {
        let content = r#"# This is a comment
IN_PLACE_UPDATES="true"
# Another comment"#;
        let features = parse_image_features_from_str(content).unwrap();
        assert_eq!(features.in_place_updates, true);
    }

    #[test]
    fn test_parse_env_ignores_empty_lines() {
        let content = r#"
IN_PLACE_UPDATES="false"

"#;
        let features = parse_image_features_from_str(content).unwrap();
        assert_eq!(features.in_place_updates, false);
    }

    #[test]
    fn test_default_true() {
        assert_eq!(default_true(), true);
    }

    #[test]
    fn test_default_when_missing() {
        let content = "";
        let features = parse_image_features_from_str(content).unwrap();
        assert_eq!(features.in_place_updates, true);
    }
}
