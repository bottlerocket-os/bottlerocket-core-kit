use snafu::{OptionExt, ResultExt};
use std::path::Path;

mod merge_json;
use merge_json::merge_json;

/// Fetches the given prefixes from the API and merges them into a single Value.  (It's not
/// expected that given prefixes would overlap, but if they do, later ones take precedence.)
/// Excludes any keys matching the exclude prefixes from the final result.
pub async fn get_prefixes<P>(
    socket_path: P,
    include: Vec<String>,
    exclude: Vec<String>,
) -> Result<serde_json::Value>
where
    P: AsRef<Path>,
{
    let mut results: Vec<serde_json::Value> = Vec::with_capacity(include.len());

    // Fetch all given prefixes into separate Values.
    for prefix in include {
        let uri = format!("/?prefix={prefix}");
        let method = "GET";
        let (_status, body) = crate::raw_request(&socket_path, &uri, method, None)
            .await
            .context(error::RequestSnafu { uri, method })?;
        let value = serde_json::from_str(&body).context(error::ResponseJsonSnafu { body })?;
        results.push(value);
    }

    // Merge results together.
    let mut merged = results
        .into_iter()
        .reduce(|mut merge_into, merge_from| {
            merge_json(&mut merge_into, merge_from);
            merge_into
        })
        .context(error::NoPrefixesSnafu)?;

    // Remove excluded prefixes
    if !exclude.is_empty() {
        remove_prefixes(&mut merged, &exclude);
    }

    Ok(merged)
}

/// Removes keys matching any of the given prefixes from the JSON value.
/// Uses dotted key notation (e.g., "settings.network" matches "settings.network.hostname").
/// Prefixes without trailing dots match any key starting with that prefix.
/// Prefixes with trailing dots only match keys with that exact prefix followed by more path segments.
fn remove_prefixes(value: &mut serde_json::Value, prefixes: &[String]) {
    remove_prefixes_recursive(value, "", prefixes);
}

/// Recursively removes keys from JSON, tracking the dotted path as we traverse.
/// Returns true if the object is empty after removing any prefix matches, and should be removed.
/// Returns false otherwise.
fn remove_prefixes_recursive(
    value: &mut serde_json::Value,
    parent_path: &str,
    prefixes: &[String],
) -> bool {
    let serde_json::Value::Object(map) = value else {
        // Non-object values shouldn't be removed and can't have children
        return false;
    };

    // Build paths once and determine which keys to remove
    let mut keys_to_remove = Vec::new();
    let mut keys_to_recurse = Vec::new();

    for key in map.keys() {
        let full_path = if parent_path.is_empty() {
            key.to_string()
        } else {
            format!("{}.{}", parent_path, key)
        };

        let should_remove = prefixes.iter().any(|prefix| full_path.starts_with(prefix));

        if should_remove {
            keys_to_remove.push(key.clone());
        } else {
            keys_to_recurse.push((key.clone(), full_path));
        }
    }

    // Remove matching keys
    for key in keys_to_remove {
        map.remove(&key);
    }

    // Recursively process remaining nested objects and remove if empty
    for (key, full_path) in keys_to_recurse {
        if let Some(nested_value) = map.get_mut(&key) {
            let should_remove = remove_prefixes_recursive(nested_value, &full_path, prefixes);
            if should_remove {
                map.remove(&key);
            }
        }
    }

    map.is_empty()
}

/// Fetches the given URI from the API and returns the result as an untyped Value.
pub async fn get_uri<P>(socket_path: P, uri: String) -> Result<serde_json::Value>
where
    P: AsRef<Path>,
{
    let method = "GET";
    let (_status, body) = crate::raw_request(&socket_path, &uri, method, None)
        .await
        .context(error::RequestSnafu { uri, method })?;
    serde_json::from_str(&body).context(error::ResponseJsonSnafu { body })
}

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub enum Error {
        #[snafu(display("Must give prefixes to query"))]
        NoPrefixes,

        #[snafu(display("Failed {} request to '{}': {}", method, uri, source))]
        Request {
            method: String,
            uri: String,
            #[snafu(source(from(crate::Error, Box::new)))]
            source: Box<crate::Error>,
        },

        #[snafu(display("Response contained invalid JSON '{}' - {}", body, source))]
        ResponseJson {
            body: String,
            source: serde_json::Error,
        },
    }
}
pub use error::Error;
pub type Result<T> = std::result::Result<T, error::Error>;

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_remove_prefixes_simple() {
        let mut value = json!({
            "settings": {
                "motd": "hello",
                "network": {"hostname": "test"},
                "kubernetes": {"cluster-name": "dev"}
            }
        });

        remove_prefixes(&mut value, &["settings.network".to_string()]);

        assert_eq!(
            value,
            json!({
                "settings": {
                    "motd": "hello",
                    "kubernetes": {"cluster-name": "dev"}
                }
            })
        );
    }

    #[test]
    fn test_remove_prefixes_nested() {
        let mut value = json!({
            "settings": {
                "motd": "hello",
                "network": {"hostname": "test"},
                "host-containers": {
                    "admin": {
                        "enabled": true,
                        "source": "example.com/admin:v1"
                    },
                    "control": {
                        "enabled": true,
                        "source": "example.com/control:v1"
                    }
                },
                "kubernetes": {
                    "cluster-name": "dev"
                }
            }
        });

        remove_prefixes(
            &mut value,
            &[
                "settings.network".to_string(),
                "settings.host-containers.admin".to_string(),
            ],
        );

        assert_eq!(
            value,
            json!({
                "settings": {
                    "motd": "hello",
                    "host-containers": {
                        "control": {
                            "enabled": true,
                            "source": "example.com/control:v1"
                        }
                    },
                    "kubernetes": {
                        "cluster-name": "dev"
                    }
                }
            })
        );
    }

    #[test]
    fn test_remove_prefixes_no_match() {
        let mut value = json!({
            "settings": {
                "motd": "hello"
            }
        });

        let expected = value.clone();
        remove_prefixes(&mut value, &["settings.network".to_string()]);

        assert_eq!(value, expected);
    }

    #[test]
    fn test_remove_prefixes_exact_dotted_path() {
        // Test the specific case: {"settings":{"network":{"hostname":"foo"}}}
        // with exclude prefix "settings.network" should remove the entire network subtree
        let mut value = json!({
            "settings": {
                "network": {
                    "hostname": "foo"
                }
            }
        });

        remove_prefixes(&mut value, &["settings.network".to_string()]);

        // After removing "network", "settings" becomes empty and is also removed
        assert_eq!(value, json!({}));
    }

    #[test]
    fn test_remove_prefixes_trailing_dot() {
        // Test that "settings.network." with trailing dot only matches children, not the key itself
        let mut value = json!({
            "settings": {
                "network": {
                    "hostname": "foo"
                },
                "network-connections": {
                    "eth0": "up"
                },
                "motd": "hello"
            }
        });

        remove_prefixes(&mut value, &["settings.network.".to_string()]);

        // Trailing dot means only children of "network" are removed
        // Since "network" becomes empty, it should also be removed
        assert_eq!(
            value,
            json!({
                "settings": {
                    "network-connections": {
                        "eth0": "up"
                    },
                    "motd": "hello"
                }
            })
        );
    }

    #[test]
    fn test_remove_prefixes_similar_names() {
        // Test that "settings.network" (no trailing dot) matches both "network" and "network-connections"
        let mut value = json!({
            "settings": {
                "network": {
                    "hostname": "foo"
                },
                "network-connections": {
                    "eth0": "up"
                },
                "motd": "hello"
            }
        });

        remove_prefixes(&mut value, &["settings.network".to_string()]);

        // Without trailing dot, matches "network" exactly and "network-connections" by prefix
        assert_eq!(
            value,
            json!({
                "settings": {
                    "motd": "hello"
                }
            })
        );
    }

    #[test]
    fn test_remove_prefixes_trailing_dot_non_map() {
        // Test that "settings.motd." with trailing dot should NOT remove "settings.motd" when motd is a string
        let mut value = json!({
            "settings": {
                "motd": "hello"
            }
        });

        remove_prefixes(&mut value, &["settings.motd.".to_string()]);

        // motd is not a map, so "settings.motd." should not match it
        assert_eq!(
            value,
            json!({
                "settings": {
                    "motd": "hello"
                }
            })
        );
    }
}
