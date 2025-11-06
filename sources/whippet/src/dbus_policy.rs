/*
 *  Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 *  Copyright (C) 2016-2023 Red Hat, Inc.
 *  Copyright (C) 2023 David Rheinsberg <david@readahead.eu>
 *  Copyright (C) 2023 Tom Gundersen <teg@jklm.no>
 *
 *  SPDX-License-Identifier: Apache-2.0
 *  Originally derived from:
 *  https://github.com/bus1/dbus-broker/blob/b0db0890d1254477cf832e5f9f0a798360c80fd9/src/launch/policy.c
 *
 *  Changes for Bottlerocket:
 *    - Use Zvariant types to serialize in the Dbus format
 *    - Default console_entries and selinux_contexts to empty lists (they aren't used)
 *    - Default apparmor_enabled to false (not supported in Bottlerocket)
 *    - Default bus_type to system
 */

//! D-Bus policy serialization and conversion.
//!
//! This module converts whippet's TOML-based policy format into the binary format
//! expected by dbus-broker, ensuring compatibility with the broker's policy engine.

use crate::error::{self, Result};
use crate::policy::{Context, Policy, Rule};
use serde::Serialize;
use snafu::ResultExt;
use zvariant::{Type, Value as ZVariantValue};

/// Top-level policy structure that matches launcher's Dbus format. It is crucial that
/// the order of the fields remains like it is, otherwise the broker rejects the policy.
///
/// Use zvariant's Type and Value to generate both the Dbus signature (available at
/// DbusPolicy::SIGNATURE), and to simplify the serialization into the Dbus format
/// See:
/// https://github.com/bus1/dbus-broker/blob/a7960f21977059dbb2356072bcd6f583b667593c/src/launch/policy.h#L21
/// https://github.com/bus1/dbus-broker/blob/b0db0890d1254477cf832e5f9f0a798360c80fd9/src/launch/policy.c#L936
#[derive(Debug, Type, Serialize, Clone, ZVariantValue, Default)]
pub(crate) struct DbusPolicy {
    pub(crate) uid_entries: Vec<(u32, PolicyBatch)>,
    pub(crate) console_entries: Vec<(bool, u32, u32, PolicyBatch)>,
    pub(crate) selinux_contexts: Vec<(String, String)>,
    pub(crate) apparmor_enabled: bool,
    pub(crate) bus_type: String,
}

/// Represents the actual policy in the dbus-launcher, as with the DbusPolicy, the order of the
/// fields is crucial
/// See:
/// https://github.com/bus1/dbus-broker/blob/b0db0890d1254477cf832e5f9f0a798360c80fd9/src/launch/policy.c#L930
#[derive(Debug, Type, Serialize, Clone, ZVariantValue, Default)]
pub struct PolicyBatch {
    pub(crate) connect_verdict: bool,
    pub(crate) connect_priority: u64,
    pub(crate) own_rules: Vec<OwnRecord>,
    pub(crate) send_rules: Vec<SendRecord>,
    pub(crate) recv_rules: Vec<ReceiveRecord>,
}

/// Represents an Own Record in the actual dbus policy
/// See:
/// https://github.com/bus1/dbus-broker/blob/b0db0890d1254477cf832e5f9f0a798360c80fd9/src/launch/policy.c#L836
#[derive(Debug, Type, Serialize, Clone, ZVariantValue, Default)]
pub struct OwnRecord {
    pub verdict: bool,
    pub priority: u64,
    pub prefix: bool,
    pub name: String,
}

/// Represents a Send/Receive Record in the actual dbus policy
/// See:
/// https://github.com/bus1/dbus-broker/blob/b0db0890d1254477cf832e5f9f0a798360c80fd9/src/launch/policy.c#L877
macro_rules! impl_record {
    ($record_type:ident, $rule_variant:path, [$(($struct_field:ident, $rule_field:ident)),*]) => {
        #[derive(Debug, Type, Serialize, Clone, ZVariantValue, Default)]
        pub struct $record_type {
            pub verdict: bool,
            pub priority: u64,
            pub name: String,
            pub path: String,
            pub interface: String,
            pub member: String,
            pub record_type: crate::policy::MessageType,
            pub broadcast: u32,
            pub min_fds: u64,
            pub max_fds: u64,
        }

        impl TryFrom<&Rule> for $record_type {
            type Error = crate::error::Error;

            fn try_from(rule: &Rule) -> Result<Self> {
                match rule {
                    $rule_variant {
                        allow,
                        priority,
                        $($rule_field,)*
                        ..
                    } => Ok(Self {
                        verdict: *allow,
                        priority: *priority,
                        $($struct_field: $rule_field.clone(),)*
                        ..Self::default()
                    }),
                    _ => error::RuleToRecordSnafu {
                        rule_type: format!("{rule:?}"),
                        record_type: stringify!($record_type).to_string(),
                    }
                    .fail(),
                }
            }
        }
    };
}

impl_record!(SendRecord, Rule::Send, [
    (broadcast, send_broadcast),
    (name, send_destination),
    (interface, send_interface),
    (member, send_member),
    (path, send_path),
    (record_type, send_type)
]);

impl_record!(ReceiveRecord, Rule::Receive, [
    (broadcast, receive_broadcast),
    (interface, receive_interface),
    (member, receive_member),
    (path, receive_path),
    (name, receive_sender),
    (record_type, receive_type)
]);

impl DbusPolicy {
    /// Builds a new DbusPolicy object using "system" as the only supported bus_type
    fn new() -> Self {
        Self {
            uid_entries: Vec::new(),
            console_entries: Vec::new(),
            selinux_contexts: Vec::new(),
            apparmor_enabled: false,
            bus_type: String::from("system"),
        }
    }
}

impl TryFrom<Policy> for DbusPolicy {
    type Error = crate::error::Error;

    /// Convert TOML config to DbusPolicy format, inserting the default policy batch as the policy
    /// for the default user, and appending the same batch to all the policy batches
    /// See:
    /// https://github.com/bus1/dbus-broker/blob/b0db0890d1254477cf832e5f9f0a798360c80fd9/src/launch/policy.c#L1056
    fn try_from(policy: Policy) -> Result<Self> {
        debug!("Converting TOML config to D-Bus policy format");

        let mut dbus_policy = Self::new();

        let default_batch = if let Some(default_policy) = &policy.default {
            default_policy.try_into()?
        } else {
            // If there isn't a default policy batch, create one with verdict false and priority 1
            // as the dbus-launcher does
            PolicyBatch {
                connect_priority: 1,
                ..Default::default()
            }
        };
        // Insert the default batch at the beginning, this mimics what the launcher does and it is
        // easier for debugging
        dbus_policy
            .uid_entries
            .insert(0, (u32::MAX, default_batch.clone()));

        for (uid, user_policy) in policy.user.iter().flatten() {
            let mut batch: PolicyBatch = user_policy.try_into()?;
            // If this policy batch didn't include a connect rule, use the default connect
            // verdict and priority, matching the fallback behavior in the dbus-launcher
            if !user_policy.has_connect_rule() {
                batch.connect_verdict = default_batch.connect_verdict;
                batch.connect_priority = default_batch.connect_priority;
            }
            // Also, append all the default rules, as the launcher does
            batch.copy_rules_from(&default_batch);

            dbus_policy.uid_entries.push((
                uid.parse::<u32>().context(error::ParseUidSnafu { uid })?,
                batch,
            ));
        }

        Ok(dbus_policy)
    }
}

impl PolicyBatch {
    /// Copies rules from the provided policy batch
    fn copy_rules_from(&mut self, source: &PolicyBatch) {
        let mut copy = source.clone();
        self.own_rules.append(&mut copy.own_rules);
        self.send_rules.append(&mut copy.send_rules);
        self.recv_rules.append(&mut copy.recv_rules);
    }
}

impl TryFrom<&Context> for PolicyBatch {
    type Error = crate::error::Error;

    fn try_from(value: &Context) -> Result<Self> {
        let mut batch = Self::default();
        for rule in &value.rules {
            match rule {
                // The connect rule determines what's the verdict and priority for the entire batch
                Rule::ConnectUser {
                    allow, priority, ..
                } => {
                    batch.connect_verdict = *allow;
                    batch.connect_priority = *priority;
                }
                Rule::Own { .. } => {
                    batch.own_rules.push(rule.try_into()?);
                }
                Rule::Send { .. } => {
                    batch.send_rules.push(rule.try_into()?);
                }
                Rule::Receive { .. } => {
                    batch.recv_rules.push(rule.try_into()?);
                }
            }
        }

        Ok(batch)
    }
}

impl TryFrom<&Rule> for OwnRecord {
    type Error = crate::error::Error;

    fn try_from(rule: &Rule) -> Result<Self> {
        match rule {
            Rule::Own {
                own,
                allow,
                priority,
                ..
            } => {
                // Follows what the launcher does for own rules
                // See:
                // https://github.com/bus1/dbus-broker/blob/b0db0890d1254477cf832e5f9f0a798360c80fd9/src/launch/policy.c#L373
                let (name, prefix) = if own == "*" {
                    ("", true)
                } else {
                    (own.as_ref(), false)
                };
                Ok(OwnRecord {
                    verdict: *allow,
                    name: name.to_owned(),
                    priority: *priority,
                    prefix,
                })
            }
            _ => error::RuleToRecordSnafu {
                rule_type: format!("{rule:?}"),
                record_type: "OwnRecord".to_string(),
            }
            .fail(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dbus_policy_signature_compatibility() {
        // Expected signature that matches dbus-broker launcher. Any changes to the DbusPolicy
        // ordering results in changes to this signature
        let expected_signature = "(a(u(bta(btbs)a(btssssuutt)a(btssssuutt)))a(buu(bta(btbs)a(btssssuutt)a(btssssuutt)))a(ss)bs)";
        let actual_signature = DbusPolicy::SIGNATURE.to_string();

        assert_eq!(
            actual_signature, expected_signature,
            "DbusPolicy signature must match dbus-broker launcher expectations exactly"
        );
    }

    #[test]
    fn test_dbus_policy_fallback_connect() {
        // With this policy, user "1" should get the default connect rule from the default context
        let config_str = format!(
            r#"
            [default]
            rules = [
                {{ allow = true, user = "{}" }}
            ]

            [user."1"]
            rules = [
                {{ allow = true, send_interface = "org.example.Dummy" }}
            ]
        "#,
            u32::MAX
        );

        let mut policy: Policy = toml::from_str(&config_str).unwrap();
        let mut current_priority: u64 = 0;
        policy.set_rule_priorities(&mut current_priority);
        policy.prepare().unwrap();
        policy.optimize();

        let dbus_policy: DbusPolicy = policy.try_into().unwrap();
        // Test user "1" gets connect_verdict == true due to the default policy
        assert!(dbus_policy
            .uid_entries
            .iter()
            .any(|(id, batch)| *id == 1 && batch.connect_verdict));
    }

    #[test]
    fn test_dbus_policy_fallback_connect_missing_default() {
        // With this policy, the default connect rule is missing so the user "1" gets
        // verdict = false for its policy batch
        let config_str = r#"
            [user."1"]
            rules = [
                { allow = true, send_interface = "org.example.Dummy" }
            ]
        "#;

        let mut policy: Policy = toml::from_str(config_str).unwrap();
        let mut current_priority: u64 = 0;
        policy.set_rule_priorities(&mut current_priority);
        policy.prepare().unwrap();
        policy.optimize();

        let dbus_policy: DbusPolicy = policy.try_into().unwrap();
        // There is always a default batch, even if it is missing in the policy
        assert_eq!(dbus_policy.uid_entries.len(), 2);
        // Since there wasn't an allow connect rule, the user's verdict is false
        assert!(dbus_policy
            .uid_entries
            .iter()
            .any(|(id, batch)| *id == 1 && !batch.connect_verdict));
    }

    #[test]
    fn test_dbus_policy_user_with_connect_rule_default_deny() {
        // With this policy, user 2 gets a default deny due to Rule 1
        let config_str = format!(
            r#"
            [default]
            rules = [
                {{ allow = false, user = "{}" }},                           # Rule 1
                {{ allow = true, user = "1" }}                              # Rule 2
            ]

            [user."1"]
            rules = [
                {{ allow = true, send_interface = "org.example.Dummy" }}    # Rule 3
            ]
            [user."2"]
            rules = [
                {{ allow = true, send_interface = "org.example.Dummy" }}    # Rule 4
            ]
        "#,
            u32::MAX
        );

        let mut policy: Policy = toml::from_str(&config_str).unwrap();
        policy.set_rule_priorities(&mut 0u64);
        policy.prepare().unwrap();
        policy.optimize();

        let dbus_policy: DbusPolicy = policy.try_into().unwrap();
        // User 1 gets verdict = true, as RULE 2 allows connections
        assert!(dbus_policy
            .uid_entries
            .iter()
            .any(|(id, batch)| *id == 1 && batch.connect_verdict));
        // User 2 gets verdict = false, as RULE 1 forbids connections
        assert!(dbus_policy
            .uid_entries
            .iter()
            .any(|(id, batch)| *id == 2 && !batch.connect_verdict));
    }

    #[test]
    fn test_default_user_is_first() {
        let config_str = r#"
            [default]
            rules = [
                { allow = true, send_interface = "*" }
            ]

            [user."1"]
            rules = [
                { allow = true, send_interface = "org.example.Dummy" }
            ]
        "#;
        let mut policy: Policy = toml::from_str(config_str).unwrap();
        policy.set_rule_priorities(&mut 0u64);
        policy.prepare().unwrap();
        policy.optimize();

        let dbus_policy: DbusPolicy = policy.try_into().unwrap();
        assert_eq!(dbus_policy.uid_entries[0].0, u32::MAX);
    }

    #[test]
    fn test_users_get_default_policy() {
        let config_str = r#"
            [default]
            rules = [
                { allow = true, send_interface = "*" }
            ]

            [user."1"]
            rules = [
                { allow = true, send_interface = "org.example.Dummy" }
            ]
        "#;
        let mut policy: Policy = toml::from_str(config_str).unwrap();
        policy.set_rule_priorities(&mut 0u64);
        policy.prepare().unwrap();
        policy.optimize();

        let dbus_policy: DbusPolicy = policy.try_into().unwrap();
        assert_eq!(dbus_policy.uid_entries[1].1.send_rules.len(), 2);
    }
}
