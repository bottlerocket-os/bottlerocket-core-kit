//! A JSON value whose strings are zeroized when it is dropped.
//!
//! Settings and user data can carry secrets, and rottweiler otherwise keeps such payloads in
//! `Zeroizing` buffers; `serde_json::Value` does not implement `Zeroize`, so this wrapper zeroizes
//! every string and key on drop. Like all heap zeroization this is best effort: parser-internal
//! copies and reallocation remnants are not covered.

use std::ops::{Deref, DerefMut};
use zeroize::Zeroize;

/// A `serde_json::Value` that zeroizes its strings on drop.
pub struct SecretValue(serde_json::Value);

impl SecretValue {
    pub fn new(value: serde_json::Value) -> Self {
        Self(value)
    }
}

impl Deref for SecretValue {
    type Target = serde_json::Value;
    fn deref(&self) -> &serde_json::Value {
        &self.0
    }
}

impl DerefMut for SecretValue {
    fn deref_mut(&mut self) -> &mut serde_json::Value {
        &mut self.0
    }
}

impl Drop for SecretValue {
    fn drop(&mut self) {
        zeroize_value(&mut self.0);
    }
}

/// Zeroize every string (values and object keys) in a JSON tree, leaving an empty tree behind.
fn zeroize_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(s) => s.zeroize(),
        serde_json::Value::Array(items) => {
            for item in items.iter_mut() {
                zeroize_value(item);
            }
            items.clear();
        }
        serde_json::Value::Object(map) => {
            for (mut key, mut item) in std::mem::take(map) {
                key.zeroize();
                zeroize_value(&mut item);
            }
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn zeroizes_strings_keys_and_nested_values() {
        let mut v =
            json!({"settings": {"token": "s3cret", "list": ["a", {"k": "b"}], "n": 1, "b": true}});
        zeroize_value(&mut v);
        assert_eq!(v, json!({}));
    }

    #[test]
    fn derefs_to_the_inner_value() {
        let mut v = SecretValue::new(json!({"settings": {"motd": "hi"}}));
        assert_eq!(v.pointer("/settings/motd").unwrap(), "hi");
        *v.pointer_mut("/settings/motd").unwrap() = serde_json::Value::String("bye".into());
        assert_eq!(v.pointer("/settings/motd").unwrap(), "bye");
    }
}
