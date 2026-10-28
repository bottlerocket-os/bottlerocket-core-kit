//! Location-agnostic measurement of settings.
//!
//! Before settings are measured into PCR 8 — from user data and from the API alike — the region is
//! excluded and the registry is removed from every `settings.host-containers.*.source` and
//! `settings.bootstrap-containers.*.source`, leaving `repo[:tag][@digest]`. Rationale, caveats,
//! and the recomputation recipe are in `docs/ENCRYPTED_STORAGE.md` ("PCR 8: User Data and OS Settings").

use crate::secret_json::SecretValue;
use serde::Serialize;
use snafu::prelude::*;
use zeroize::{Zeroize, Zeroizing};

type Result<T> = std::result::Result<T, snafu::Whatever>;

/// The settings tables whose members carry a container image `source`.
const IMAGE_SOURCE_TABLES: &[&str] = &["host-containers", "bootstrap-containers"];

/// Settings that describe the deployment location rather than the configuration, excluded from
/// both PCR 8 extensions.
pub const LOCATION_SETTINGS: &[&str] = &["settings.aws.region"];

/// Remove every setting whose dotted path starts with one of `prefixes`, pruning objects left
/// empty, with the same prefix semantics as `apiclient get --exclude`.
pub fn remove_excluded(settings: &mut serde_json::Value, prefixes: &[&str]) {
    remove_excluded_recursive(settings, "", prefixes);
}

fn remove_excluded_recursive(
    value: &mut serde_json::Value,
    parent: &str,
    prefixes: &[&str],
) -> bool {
    let serde_json::Value::Object(map) = value else {
        return false;
    };
    let mut remove = Vec::new();
    let mut recurse = Vec::new();
    for key in map.keys() {
        let path = if parent.is_empty() {
            key.clone()
        } else {
            format!("{parent}.{key}")
        };
        if prefixes.iter().any(|prefix| path.starts_with(prefix)) {
            remove.push(key.clone());
        } else {
            recurse.push((key.clone(), path));
        }
    }
    for key in remove {
        if let Some(mut removed) = map.remove(&key) {
            zeroize_strings(&mut removed);
        }
    }
    for (key, path) in recurse {
        if let Some(nested) = map.get_mut(&key)
            && remove_excluded_recursive(nested, &path, prefixes)
        {
            map.remove(&key);
        }
    }
    map.is_empty()
}

/// Zeroize the strings of a value that is being discarded before the tree is dropped.
fn zeroize_strings(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(s) => s.zeroize(),
        serde_json::Value::Array(items) => items.iter_mut().for_each(zeroize_strings),
        serde_json::Value::Object(map) => map.values_mut().for_each(zeroize_strings),
        _ => {}
    }
}

/// Remove the registry component from a container image source, if it has one.
///
/// The first path component is a registry when it contains a `.` or a `:` or is `localhost` (the
/// Docker/containerd rule). Nothing else is normalized; sources without a registry yield `None`.
pub fn without_registry(source: &str) -> Option<&str> {
    let (first, rest) = source.split_once('/')?;
    let is_registry = first.contains('.') || first.contains(':') || first == "localhost";
    is_registry.then_some(rest)
}

/// Rewrite every container image `source` under `settings.host-containers` and
/// `settings.bootstrap-containers` without its registry, in place.
pub fn strip_registries(settings: &mut serde_json::Value) {
    for table in IMAGE_SOURCE_TABLES {
        let Some(containers) = settings
            .pointer_mut(&format!("/settings/{table}"))
            .and_then(serde_json::Value::as_object_mut)
        else {
            continue;
        };
        for container in containers.values_mut() {
            let Some(serde_json::Value::String(source)) = container.get_mut("source") else {
                continue;
            };
            let Some(stripped) = without_registry(source).map(str::to_string) else {
                continue;
            };
            // Zeroize the registry-qualified string before replacing it, so the replaced value is
            // covered like the rest of the tree.
            source.zeroize();
            *source = stripped;
        }
    }
}

/// Serialize settings as OLPC canonical JSON plus a trailing newline, byte-compatible with the
/// pre-existing settings measurement.
pub fn canonicalize(settings: &serde_json::Value) -> Result<Zeroizing<Vec<u8>>> {
    let mut buf = Zeroizing::new(Vec::new());
    let mut ser =
        serde_json::Serializer::with_formatter(&mut *buf, olpc_cjson::CanonicalFormatter::new());
    settings
        .serialize(&mut ser)
        .with_whatever_context(|_| "failed to canonicalize settings")?;
    buf.push(b'\n');
    Ok(buf)
}

/// Remove the location settings and the image registries from a settings tree and return its
/// canonical measurement bytes. Used for both PCR 8 extensions.
///
/// The tree is consumed and its strings are zeroized when this returns; the result is zeroized
/// on drop as well.
pub fn measurement_bytes(mut settings: SecretValue) -> Result<Zeroizing<Vec<u8>>> {
    remove_excluded(&mut settings, LOCATION_SETTINGS);
    strip_registries(&mut settings);
    canonicalize(&settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const DIGEST: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn strips_any_registry() {
        let cases = [
            (
                "328549459982.dkr.ecr.us-west-2.amazonaws.com/bottlerocket-admin:v0.21.6",
                "bottlerocket-admin:v0.21.6",
            ),
            (
                "328549459982.dkr.ecr-fips.us-gov-west-1.amazonaws.com/bottlerocket-admin:v0.21.6",
                "bottlerocket-admin:v0.21.6",
            ),
            (
                "111122223333.dkr.ecr.cn-north-1.amazonaws.com.cn/team/agent:v1",
                "team/agent:v1",
            ),
            (
                "public.ecr.aws/bottlerocket/bottlerocket-control:v0.21.6",
                "bottlerocket/bottlerocket-control:v0.21.6",
            ),
            ("docker.io/library/nginx:latest", "library/nginx:latest"),
            (
                "quay.io/prometheus/node-exporter:v1.5.0",
                "prometheus/node-exporter:v1.5.0",
            ),
            ("localhost/repo:tag", "repo:tag"),
            ("localhost:5000/repo:tag", "repo:tag"),
            ("registry.example.com:443/a/b/c@", "a/b/c@"),
        ];
        for (input, expected) in cases {
            assert_eq!(without_registry(input), Some(expected), "{input}");
        }
    }

    #[test]
    fn keeps_repository_tag_and_digest() {
        assert_eq!(
            without_registry(&format!(
                "111122223333.dkr.ecr.us-east-1.amazonaws.com/agent@{DIGEST}"
            )),
            Some(format!("agent@{DIGEST}").as_str())
        );
        assert_eq!(
            without_registry(&format!(
                "111122223333.dkr.ecr.us-east-1.amazonaws.com/team/agent:v1@{DIGEST}"
            )),
            Some(format!("team/agent:v1@{DIGEST}").as_str())
        );
        assert_eq!(
            without_registry("111122223333.dkr.ecr.us-east-1.amazonaws.com/agent"),
            Some("agent")
        );
    }

    #[test]
    fn leaves_sources_without_a_registry_untouched() {
        for input in [
            "nginx:latest",
            "library/nginx:latest",
            "bottlerocket-admin:v0.21.6",
            &format!("agent@{DIGEST}"),
            "team/agent:v1",
            "",
        ] {
            assert_eq!(without_registry(input), None, "{input}");
        }
    }

    #[test]
    fn different_regions_and_accounts_collapse_to_the_same_value() {
        let a = format!("111111111111.dkr.ecr.us-east-1.amazonaws.com/agent@{DIGEST}");
        let b = format!("222222222222.dkr.ecr.eu-west-1.amazonaws.com/agent@{DIGEST}");
        let c = format!("public.ecr.aws/agent@{DIGEST}");
        assert_eq!(without_registry(&a), without_registry(&b));
        assert_eq!(without_registry(&a), without_registry(&c));
        assert_eq!(
            without_registry(&a),
            Some(format!("agent@{DIGEST}").as_str())
        );
    }

    fn sample() -> serde_json::Value {
        json!({"settings": {
            "aws": {"region": "us-west-2"},
            "host-containers": {
                "admin": {"enabled": true, "source": "328549459982.dkr.ecr.us-west-2.amazonaws.com/bottlerocket-admin:v0.21.6"},
                "control": {"enabled": false, "source": "public.ecr.aws/bottlerocket/bottlerocket-control:v0.21.6"},
                "plain": {"enabled": true, "source": "nginx:latest"},
                "nosource": {"enabled": true}
            },
            "bootstrap-containers": {
                "agent": {"mode": "always", "source": format!("111122223333.dkr.ecr.us-west-2.amazonaws.com/agent@{DIGEST}")}
            },
            "motd": "328549459982.dkr.ecr.us-west-2.amazonaws.com/not-a-source"
        }})
    }

    #[test]
    fn strips_registries_in_both_container_tables_only() {
        let mut v = sample();
        strip_registries(&mut v);
        assert_eq!(
            v.pointer("/settings/host-containers/admin/source").unwrap(),
            "bottlerocket-admin:v0.21.6"
        );
        assert_eq!(
            v.pointer("/settings/host-containers/control/source")
                .unwrap(),
            "bottlerocket/bottlerocket-control:v0.21.6"
        );
        assert_eq!(
            v.pointer("/settings/host-containers/plain/source").unwrap(),
            "nginx:latest"
        );
        assert_eq!(
            v.pointer("/settings/bootstrap-containers/agent/source")
                .unwrap(),
            &format!("agent@{DIGEST}")
        );
        assert_eq!(
            v.pointer("/settings/motd").unwrap(),
            "328549459982.dkr.ecr.us-west-2.amazonaws.com/not-a-source"
        );
        assert_eq!(v.pointer("/settings/aws/region").unwrap(), "us-west-2");
    }

    #[test]
    fn strip_is_a_no_op_without_container_tables_or_string_sources() {
        for original in [
            json!({"settings": {"aws": {"region": "us-west-2"}}}),
            json!({}),
            json!({"settings": {"host-containers": {"weird": {"source": 42}, "flat": "x.io/y"}, "bootstrap-containers": "not-a-table"}}),
        ] {
            let mut v = original.clone();
            strip_registries(&mut v);
            assert_eq!(v, original);
        }
    }

    #[test]
    fn remove_excluded_drops_the_region_and_prunes_the_empty_aws_table() {
        let mut v = json!({"settings": {
            "updates": {"seed": 12, "ignore-waves": false},
            "network": {"hostname": "h"},
            "aws": {"region": "us-west-2"},
            "motd": "hi"
        }});
        remove_excluded(&mut v, LOCATION_SETTINGS);
        assert_eq!(
            v,
            json!({"settings": {
                "updates": {"seed": 12, "ignore-waves": false},
                "network": {"hostname": "h"},
                "motd": "hi"
            }})
        );
    }

    #[test]
    fn remove_excluded_keeps_other_aws_settings() {
        let mut v = json!({"settings": {"aws": {"region": "us-west-2", "profile": "default"}}});
        remove_excluded(&mut v, LOCATION_SETTINGS);
        assert_eq!(v, json!({"settings": {"aws": {"profile": "default"}}}));
    }

    #[test]
    fn remove_excluded_uses_apiclient_prefix_semantics() {
        let mut v =
            json!({"settings": {"network": {"hostname": "h"}, "network-connections": {"x": 1}}});
        remove_excluded(&mut v, &["settings.network"]);
        assert_eq!(v, json!({}));
        let mut v =
            json!({"settings": {"network": {"hostname": "h"}, "network-connections": {"x": 1}}});
        remove_excluded(&mut v, &["settings.network."]);
        assert_eq!(v, json!({"settings": {"network-connections": {"x": 1}}}));
    }

    #[test]
    fn measurement_bytes_excludes_region_and_strips_registries() {
        let v = json!({"settings": {
            "aws": {"region": "us-east-1"},
            "host-containers": {"admin": {"source": "328549459982.dkr.ecr.us-east-1.amazonaws.com/bottlerocket-admin:v0.21.6"}}
        }});
        let measured = measurement_bytes(SecretValue::new(v)).unwrap();
        assert_eq!(
            std::str::from_utf8(&measured).unwrap(),
            "{\"settings\":{\"host-containers\":{\"admin\":{\"source\":\"bottlerocket-admin:v0.21.6\"}}}}\n"
        );
    }

    #[test]
    fn canonical_output_is_sorted_compact_and_newline_terminated() {
        let v = json!({"settings": {"motd": "hi", "aws": {"region": "us-west-2"}, "ntp": {"time-servers": ["a", "b"]}, "updates": {"ignore-waves": false, "seed": 1234}}});
        let expected = concat!(
            r#"{"settings":{"aws":{"region":"us-west-2"},"motd":"hi","ntp":{"time-servers":["a","b"]},"updates":{"ignore-waves":false,"seed":1234}}}"#,
            "\n"
        );
        assert_eq!(canonicalize(&v).unwrap().as_slice(), expected.as_bytes());
    }

    #[test]
    fn measurement_bytes_from_pretty_json_equal_canonical_of_stripped_value() {
        let pretty = serde_json::to_string_pretty(&sample()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&pretty).unwrap();
        let measured = measurement_bytes(SecretValue::new(parsed)).unwrap();
        let mut expected = sample();
        remove_excluded(&mut expected, LOCATION_SETTINGS);
        strip_registries(&mut expected);
        assert_eq!(
            measured.as_slice(),
            canonicalize(&expected).unwrap().as_slice()
        );
        let text = std::str::from_utf8(&measured).unwrap();
        assert!(text.contains(r#""source":"bottlerocket-admin:v0.21.6""#));
        assert!(text.contains(&format!(r#""source":"agent@{DIGEST}""#)));
        assert!(!text.contains("dkr.ecr.us-west-2.amazonaws.com/bottlerocket-admin"));
    }

    #[test]
    fn canonicalize_rejects_floats() {
        assert!(canonicalize(&json!({"settings": {"x": 1.5}})).is_err());
    }
}
