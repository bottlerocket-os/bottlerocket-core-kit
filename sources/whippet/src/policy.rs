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
 *    - Hard-coded path to drop-in overrides
 *    - Skip default connect rule for the launcher user
 *    - Drop ignored or deprecated fields
 *    - Parse the default contexts and user contexts in fixed order, rather than parsing the
 *    first one found
 */

use crate::error::{self, Result};
use indexmap::IndexMap;
use serde::Deserialize;
use serde_repr::Serialize_repr;
use snafu::{ensure, ResultExt};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use zvariant::{Type as ZVariantType, Value as ZVariantValue};

/// Default directory path additional policies are read from
pub(crate) const DBUS_POLICY_DROPINS_PATH: &str = "/usr/share/whippet/policies.d";
/// Multiplier used to calculate the priority of a rule
const DEFAULT_POLICY_CONTEXT_MULTIPLIER: u64 = 1;
const DEFAULT_POLICY_CONTEXT_USER: u64 = 3;
const PRIORITY_BASE: u64 = u64::MAX / 7;

const DEFAULT_USER_ID: u32 = u32::MAX;

/// Represents a minimal policy configuration for the dbus-broker. Each section on the policy is
/// known as a Context.
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct Policy {
    /// The default rules that apply to all contexts
    pub(crate) default: Option<Context>,
    /// User-specific rules
    pub(crate) user: Option<IndexMap<String, Context>>,
}

/// A context contains the rules that actually make a policy
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct Context {
    pub(crate) rules: Vec<Rule>,
}

/// Represents a rule to configure permissions in the Dbus
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(untagged, deny_unknown_fields)]
pub(crate) enum Rule {
    /// Connect users dictates whether a user is allowed to connect to the bus
    ConnectUser {
        allow: bool,
        user: String,
        #[serde(skip)]
        priority: u64,
    },
    /// Own rules allow services to own Dbus service names
    Own {
        own: String,
        allow: bool,
        #[serde(skip)]
        priority: u64,
    },
    /// Send rules allow services to send messages through the bus
    Send {
        #[serde(default)]
        send_destination: String,
        #[serde(default)]
        send_interface: String,
        #[serde(default)]
        send_member: String,
        #[serde(default)]
        send_type: MessageType,
        #[serde(default)]
        send_path: String,
        #[serde(default)]
        send_broadcast: u32,
        allow: bool,
        #[serde(skip)]
        priority: u64,
    },
    /// Receive rules allow services to receive messages from the bus
    Receive {
        #[serde(default)]
        receive_sender: String,
        #[serde(default)]
        receive_path: String,
        #[serde(default)]
        receive_interface: String,
        #[serde(default)]
        receive_member: String,
        #[serde(default)]
        receive_type: MessageType,
        #[serde(default)]
        receive_broadcast: u32,
        allow: bool,
        #[serde(skip)]
        priority: u64,
    },
}

/// Represents the message type configured for the send and receive rules. The original dbus policy
/// allows for MethodReturn and Error message types, however, in the dbus-launcher, rules with
/// either message type are dropped and not serialized in the final policy
///
/// See:
/// https://github.com/bus1/dbus-broker/blob/b0db0890d1254477cf832e5f9f0a798360c80fd9/src/launch/policy.c#L439
#[derive(
    Debug, Deserialize, Serialize_repr, Clone, PartialEq, Default, Copy, ZVariantType, ZVariantValue,
)]
#[repr(u32)]
#[serde(rename_all = "kebab-case")]
pub enum MessageType {
    // Not really invalid, as it is a valid value, but this is the name given by the launcher
    // and it is the default value used when it isn't present in a send or receive rule
    //
    // See:
    // https://github.com/bus1/dbus-broker/blob/b0db0890d1254477cf832e5f9f0a798360c80fd9/src/dbus/protocol.h#L35
    #[default]
    Invalid = 0,
    MethodCall = 1,
    Signal = 4,
}

impl Policy {
    /// Returns a new policy constructed from the provided file and additional policies. It loads the
    /// policies in the passed directory in lexicographic order
    #[must_use = "policy construction result must be checked"]
    pub(crate) fn new<U>(
        policy_file: impl AsRef<Path>,
        policies_dir: impl AsRef<Path>,
        mut username_resolver: U,
    ) -> Result<Self>
    where
        U: UsernameResolver,
    {
        let policy_file = policy_file.as_ref();
        let policies_dir = policies_dir.as_ref();
        let mut all_policies: Vec<PathBuf> = vec![policy_file.into()];

        let mut toml_files: Vec<_> = fs::read_dir(policies_dir)
            .context(error::ReadPathSnafu {
                what: policies_dir.display().to_string(),
            })?
            .collect::<std::io::Result<Vec<_>>>()
            .context(error::CollectPathsSnafu {
                what: policies_dir.display().to_string(),
            })?
            .into_iter()
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("toml"))
            .collect();

        toml_files.sort();
        all_policies.append(&mut toml_files);

        // The first rule in processed by the launcher actually gets priority 3 because:
        // - the dbus-launcher starts its priority counter at 1
        // - the dbus-launcher inserts a default connect rule for the current user with priority 2
        // Hence, use the same priority as the actual first rule that will be processed
        let mut current_priority: u64 = 3u64;
        let mut policy = Policy::default();

        // Load all the additional policies that are found in the policies_dir so that all
        // the users are validated while constructing the config object to stop as soon as possible
        // if there is an invalid user.
        for policy_file in all_policies {
            load_one_policy(policy_file, &mut policy, &mut current_priority)?;
        }
        policy.validate()?;
        policy.replace_uids(&mut username_resolver)?;

        Ok(policy)
    }

    /// Sets the priorities for the rules of a policy, starting with the users' rules
    pub(crate) fn set_rule_priorities(&mut self, current_priority: &mut u64) {
        if let Some(user_policy) = &mut self.user {
            for policy in user_policy.values_mut() {
                for rule in &mut policy.rules {
                    let priority = PRIORITY_BASE * DEFAULT_POLICY_CONTEXT_USER + *current_priority;
                    rule.set_priority(priority);
                    *current_priority += 1;
                }
            }
        }
        if let Some(default_rules) = &mut self.default {
            for rule in &mut default_rules.rules {
                let priority =
                    PRIORITY_BASE * DEFAULT_POLICY_CONTEXT_MULTIPLIER + *current_priority;
                rule.set_priority(priority);
                *current_priority += 1;
            }
        }
    }

    /// Validates that the user contexts don't include connect rules
    fn validate(&self) -> Result<()> {
        if let Some(user_policy) = &self.user {
            for (username, context) in user_policy.iter() {
                ensure!(
                    !context
                        .rules
                        .iter()
                        .any(|rule| matches!(rule, Rule::ConnectUser { .. })),
                    error::UserContextWithConnectRuleSnafu { username }
                );
            }
        }
        Ok(())
    }

    /// Replaces the UIDs of the users that were found in the policies
    fn replace_uids<U>(&mut self, username_resolver: &mut U) -> Result<()>
    where
        U: UsernameResolver,
    {
        // First, replace the users of all the connect rules in the default context
        if let Some(default_context) = &mut self.default {
            default_context
                .rules
                .iter_mut()
                .try_for_each(|rule| match rule {
                    Rule::ConnectUser { user, .. } => {
                        *user = username_resolver.resolve(user)?.to_string();
                        Ok(())
                    }
                    _ => Ok(()),
                })?;
        };

        // Then, replace all the users of the users' context. Connect rules are not allowed
        // in other contexts other than the default context, so the rules don't have to be modified
        let users_contexts = self.user.take().unwrap_or_default();
        let mut final_users_contexts = IndexMap::new();
        for (user, policy) in users_contexts.into_iter() {
            let uid = username_resolver.resolve(&user)?.to_string();
            final_users_contexts.insert(uid, policy);
        }
        self.user = Some(final_users_contexts);

        Ok(())
    }

    /// Prepares the policy before it is serialized. It determines what's the default connect rule
    /// for each user:
    /// - If the default context contains a connect rule for the user, use that
    /// - By default, leave empty and let the serialization add the default
    ///
    /// Example 1:
    ///
    /// ```toml
    /// # File 1
    /// [default]
    /// rules = [
    ///     { allow=false, user="*" },    # Rule 1
    ///     { allow=true, user="benny" }, # Rule 2
    ///     { allow=true, user="buzz" }   # Rule 3
    /// ]
    ///
    /// # File 2
    /// [user.pixie]
    /// rules = [
    ///     #...
    /// ]
    /// ```
    ///
    /// After this function is called, the result is:
    /// - Default context with only Rule 1
    /// - New user context for user "benny", with Rule 2
    /// - New user context for user "buzz", with Rule 3
    /// - No rules are moved for user "pixie"
    ///
    /// Example 2:
    ///
    /// ```toml
    /// # File 1
    /// [user.benny]
    /// rules = [
    ///     #...
    /// ]
    ///
    /// [user.buzz]
    /// rules = [
    ///     #...
    /// ]
    ///
    /// After this function is called, no connect rules were moved around since
    /// there weren't any connect rules found
    /// ```
    pub(crate) fn prepare(&mut self) -> Result<()> {
        let mut connect_rules: Vec<Rule> = if let Some(default_policy) = &mut self.default {
            default_policy
                .rules
                .extract_if(.., |rule| {
                    // Extract ONLY connect rules, that aren't for the DEFAULT user
                    if let Rule::ConnectUser { user, .. } = rule {
                        *user != DEFAULT_USER_ID.to_string()
                    } else {
                        false
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        if connect_rules.is_empty() {
            return Ok(());
        }

        // Ensure there is a user policy to inject the rules
        let mut user_policy = self.user.take().unwrap_or_default();

        // Iterate over the connect_rules found for specific users, inserting a new user policy
        // when it doesn't exist.
        while let Some(rule) = connect_rules.pop() {
            if let Rule::ConnectUser { ref user, .. } = rule {
                let policy = user_policy.entry(user.to_owned()).or_default();
                policy.rules.push(rule);
            }
        }
        self.user = Some(user_policy);

        Ok(())
    }

    /// Optimize the policy according to the optimization rules defined by the launcher. The purpose
    /// of the optimization is to keep only one default connect rule, and at most one connect rule
    /// for the user policy. This is important in case multiple files define connect rules for the
    /// same user, the last rule must win.
    ///
    /// Example:
    ///
    /// ```toml
    /// # File 1
    /// [default]
    /// rules = [
    ///     { allow=false, user="*"  },   # Deny all users, RULE 1
    ///     { allow=true, user="bob" },   # Allow bob, RULE 2
    ///     { allow=true, user="pixie" }  # Allow pixie, RULE 3
    /// ]
    ///
    /// # File 2
    /// [default]
    /// rules = [
    ///     { allow=true, user="*" },     # Allow all users, RULE 4
    /// ]
    /// ```
    ///
    /// After `prepare`, there will be:
    /// - 2 connect rules in `default` context (RULE 1 and RULE 4)
    /// - 1 connect rule in user context `bob` (RULE 2)
    /// - 1 connect rule in user context `pixie` (RULE 3)
    ///
    /// After the optimization, the result is:
    /// - 1 connect rule in `default` context (RULE 4)
    /// - 0 connect rules in user context `bob` (RULE 2 has a lower priority than RULE 4)
    /// - 0 connect rules in user context `pixie` (RULE 3 has a lower priority than RULE 4)
    ///
    /// In this example:
    ///
    /// ```toml
    /// [default]
    /// rules = [
    ///     { allow=true, user="*" },       # RULE 1
    ///     { allow=false, user="*" },      # RULE 2
    ///     { allow user="bob" },           # RULE 3
    /// ]
    ///
    /// [user.lorax]
    /// rules = []
    /// ```
    ///
    /// After `prepare, there will be:
    /// - 2 connect rules in `default` context (RULE 1 and RULE 2)
    /// - 1 connect rule in user context `bob` (RULE 3)
    ///
    /// After the optimization:
    /// - 1 connect rule in `default` context (RULE 2)
    /// - 1 connect rule in user context `bob` (RULE 3)
    /// - User context for `lorax` was removed
    pub(crate) fn optimize(&mut self) {
        // First, find the highest priority default connect rule
        let priority_threshold = self
            .default
            .as_ref()
            .map(|default_policy| find_priority_threshold(&default_policy.rules))
            .unwrap_or_default();

        // Then use the highest priority threshold to clean connect rules for both
        // the default and user contexts
        if let Some(default_policy) = &mut self.default {
            clean_connect_rules(priority_threshold, &mut default_policy.rules);
        };

        if let Some(user_policy) = &mut self.user {
            user_policy.values_mut().for_each(|policy| {
                let mut user_priority_threshold = find_priority_threshold(&policy.rules);
                if priority_threshold > user_priority_threshold {
                    user_priority_threshold = priority_threshold;
                }
                clean_connect_rules(user_priority_threshold, &mut policy.rules);
            });
            // Remove any user contexts without any rules
            user_policy.retain(|_, p| !p.rules.is_empty());
        }
    }
}

/// Loads the policy from the provided path, setting the priorities for the loaded rules
fn load_one_policy(
    policy_file: impl AsRef<Path>,
    base_policy: &mut Policy,
    current_priority: &mut u64,
) -> Result<()> {
    let policy_file = policy_file.as_ref();
    let policy_content = std::fs::read_to_string(policy_file).context(error::ReadPathSnafu {
        what: policy_file.display().to_string(),
    })?;
    let mut policy: Policy = toml::from_str(&policy_content).context(error::PolicyParseSnafu)?;
    policy.set_rule_priorities(current_priority);

    let default_policy = policy.default.take().unwrap_or_default();
    let mut base_policy_default = base_policy.default.take().unwrap_or_default();
    base_policy_default.rules.extend(default_policy.rules);
    base_policy.default = Some(base_policy_default);

    let user_policies = policy.user.take().unwrap_or_default();
    let mut base_policy_user = base_policy.user.take().unwrap_or_default();
    for (user, policy) in user_policies {
        base_policy_user
            .entry(user.to_owned())
            .or_default()
            .rules
            .extend(policy.rules);
    }
    base_policy.user = Some(base_policy_user);

    Ok(())
}

/// Retains the only connect rule that matches the priority threshold
fn clean_connect_rules(priority_threshold: u64, rules: &mut Vec<Rule>) {
    rules.retain(|r| !r.is_connect() || *r.get_priority() == priority_threshold)
}

/// Find the highest priority in the given rules, defaulting to 0 if the rules were empty
fn find_priority_threshold(rules: &[Rule]) -> u64 {
    let mut connect_rules: Vec<&Rule> = rules.iter().filter(|r| r.is_connect()).collect();
    connect_rules.sort_by(|a, b| b.get_priority().cmp(a.get_priority()));
    connect_rules
        .first()
        .map(|r| *r.get_priority())
        .unwrap_or_default()
}

/// Macro to generate rule methods that extract fields and replace wildcards with empty strings
/// See: https://github.com/bus1/dbus-broker/blob/e3324b3736fd40d95e7943fca6e485013d15d643/src/launch/policy.c#L97
macro_rules! impl_wildcard_getter {
    ($method_name:ident, ($($rule_type:ident => $field:ident),*)) => {
        pub(crate) fn $method_name(&self) -> Result<String> {
            match self {
                $(
                    Self::$rule_type { $field, .. } => {
                        if $field == "*" {
                            Ok("".to_string())
                        } else {
                            Ok($field.to_owned())
                        }
                    }
                )*
                _ => error::InvalidPropertyForRuleSnafu {
                    property: stringify!($method_name).to_string(),
                    rule_type: format!("{self:?}"),
                }
                .fail(),
            }
        }
    };
}

impl Rule {
    /// Sets the priority of the rule
    fn set_priority(&mut self, new_priority: u64) {
        match self {
            Self::ConnectUser { priority, .. } => *priority = new_priority,
            Self::Own { priority, .. } => *priority = new_priority,
            Self::Send { priority, .. } => *priority = new_priority,
            Self::Receive { priority, .. } => *priority = new_priority,
        }
    }
    /// Retrieves the priority of the rule
    fn get_priority(&self) -> &u64 {
        match self {
            Self::ConnectUser { priority, .. } => priority,
            Self::Own { priority, .. } => priority,
            Self::Send { priority, .. } => priority,
            Self::Receive { priority, .. } => priority,
        }
    }

    /// Determines whether the current rule is a connect rule
    fn is_connect(&self) -> bool {
        matches!(self, Self::ConnectUser { .. })
    }

    impl_wildcard_getter!(interface, (Send => send_interface, Receive => receive_interface));
    impl_wildcard_getter!(name, (Send => send_destination, Receive => receive_sender));
    impl_wildcard_getter!(member, (Send => send_member, Receive => receive_member));
    impl_wildcard_getter!(path, (Send => send_path, Receive => receive_path));
}

impl Context {
    /// Determines if the current context contains connect rules
    pub(crate) fn has_connect_rule(&self) -> bool {
        self.rules.iter().any(|r| r.is_connect())
    }
}

pub(crate) trait UsernameResolver {
    /// Resolves the provided username string to its corresponding UID
    fn resolve(&mut self, username: &str) -> Result<u32>;
}

#[derive(Default)]
/// Resolves usernames using the passwd file
pub(crate) struct PasswdUsernameResolver {
    cache: HashMap<String, u32>,
}

impl UsernameResolver for PasswdUsernameResolver {
    fn resolve(&mut self, username: &str) -> Result<u32> {
        debug!("Resolving username '{username}' to UID");

        // Handle wildcard case like dbus-launcher does
        if username == "*" {
            debug!("Wildcard user '*' resolved to UID {DEFAULT_USER_ID}");
            return Ok(DEFAULT_USER_ID);
        }

        if let Ok(uid) = username.parse::<u32>() {
            debug!("Numeric username '{username}' resolved to UID {uid}");
            return Ok(uid);
        }

        if let Some(uid) = self.cache.get(username) {
            debug!("Resolving username '{username}' using cached value '{uid}'");
            return Ok(*uid);
        }

        debug!("Looking up username '{username}'");
        match nix::unistd::User::from_name(username) {
            Ok(Some(user)) => {
                let uid = user.uid.as_raw();
                self.cache.insert(username.to_owned(), uid);
                debug!("Username '{username}' resolved to UID {uid}");
                Ok(uid)
            }
            Ok(None) => error::UserNotFoundSnafu {
                username: username.to_string(),
            }
            .fail(),
            Err(e) => Err(e).context(error::UserLookupFailedSnafu {
                username: username.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    const ALICE_USER: u32 = 1;
    const BOB_USER: u32 = 2;

    struct TestUsernameResolver {}
    impl UsernameResolver for TestUsernameResolver {
        fn resolve(&mut self, username: &str) -> Result<u32> {
            match username {
                "*" => Ok(DEFAULT_USER_ID),
                "alice" => Ok(ALICE_USER),
                "bob" => Ok(BOB_USER),
                _ => panic!("Unexpected user {username}"),
            }
        }
    }

    #[test]
    fn test_priorities_set_per_policy() {
        // Given A base policy configuration with default rules and additional policy files
        let resolver = TestUsernameResolver {};
        let policy = Policy::new("tests/data/main.toml", "tests/data/policies", resolver).unwrap();

        // Then Rules should have correct priorities based on their context and order
        let default_rules = policy.default.unwrap().rules;
        let user_context = policy.user.unwrap();
        let alice_rules = &user_context.get(&ALICE_USER.to_string()).unwrap().rules;

        // Since current priority is 3 and the DEFAULT multiplier is 1, the priority is equal to
        // the PRIORITY_BASE + 3
        assert!(matches!(
                default_rules.first().unwrap(), Rule::ConnectUser{ user, priority, .. }
                if *user == DEFAULT_USER_ID.to_string() && *priority == PRIORITY_BASE + 3));

        // Priority counter = 4, used in the first user rule found in the first file
        assert!(matches!(
                &alice_rules[0], Rule::Send { send_interface, priority, .. }
                if send_interface == "org.example.First" && *priority == PRIORITY_BASE * DEFAULT_POLICY_CONTEXT_USER + 4
        ));

        // Priority counter = 7, used in the last rule found in the second file
        assert!(matches!(
                &alice_rules[1], Rule::Send { send_interface, priority, .. }
                if send_interface == "org.example.SecondService" && *priority == PRIORITY_BASE * DEFAULT_POLICY_CONTEXT_USER + 7
        ));
    }

    #[test]
    fn test_additional_policies_loaded_in_order() {
        // Given A base policy configuration and additional policy files in a directory
        let resolver = TestUsernameResolver {};
        let policy = Policy::new("tests/data/main.toml", "tests/data/policies", resolver).unwrap();

        // The Rules should be merged in lexicographic order of filenames
        let default_rules = &policy.default.unwrap().rules;
        assert!(
            matches!(&default_rules[0], Rule::ConnectUser { user,.. } if *user == DEFAULT_USER_ID.to_string())
        );
        assert!(matches!(&default_rules[2], Rule::Own { own, .. } if own == "org.example.Second"));

        // Verify user rules are merged in order
        let user_rules = policy.user.unwrap();
        // This rule comes from the second file that was loaded
        assert!(
            matches!(&user_rules.get(&ALICE_USER.to_string()).unwrap().rules[1],
            Rule::Send { send_interface, .. } if send_interface == "org.example.SecondService")
        );
    }

    #[test]
    fn test_prepare() {
        // Given A policy with connect rules for specific users in the default context
        // With this policy, after calling prepare, the result is
        // - Default context contains Rule 1 and Rule 3
        // - User context "alice" contains Rule 2
        // - User context "bob" contains Rule 4
        let base_config = format!(
            r#"
            [default]
            rules = [
                {{ allow = true, user = "{DEFAULT_USER_ID}" }},             # Rule 1
                {{ allow = true, user = "alice" }},                         # Rule 2
                {{ allow = true, own = "org.example.Service" }}             # Rule 3
            ]

            [user.bob]
            rules = [
                {{ allow = true, send_interface = "org.example.Bob" }}      # Rule 4
            ]
        "#,
        );

        let mut policy: Policy = toml::from_str(&base_config).unwrap();
        policy.set_rule_priorities(&mut 1u64);

        // When Preparing the policy to move user-specific connect rules
        policy.prepare().unwrap();

        // Then User-specific connect rules should be moved to their respective contexts
        let default_rules = &policy.default.unwrap().rules;
        let user_rules = policy.user.unwrap();

        // Test default context has 2 rules (RULE 1 and RULE 3)
        assert_eq!(default_rules.len(), 2);
        // Test default context has default connect rule
        assert!(
            matches!(&default_rules[0], Rule::ConnectUser { user, .. } if user == &DEFAULT_USER_ID.to_string())
        );

        // Test RULE 2 was moved to her context
        let alice_rules = &user_rules.get("alice").unwrap().rules;
        assert_eq!(alice_rules.len(), 1);
        assert!(matches!(&alice_rules[0], Rule::ConnectUser { user, .. } if user == "alice"));

        // Test bob's existing rules are preserved (RULE 4)
        let bob_rules = &user_rules.get("bob").unwrap().rules;
        assert_eq!(bob_rules.len(), 1);
        assert!(matches!(&bob_rules[0], Rule::Send { .. }));
    }

    #[test]
    fn test_prepare_creates_user_contexts() {
        let base_config = format!(
            r#"
            [default]
            rules = [
                {{ allow = true, user = "{DEFAULT_USER_ID}" }},     # RULE 1
                {{ allow = true, user = "alice" }},                 # RULE 2
                {{ allow = false, user = "bob" }}                   # RULE 3
            ]
        "#,
        );

        let mut policy: Policy = toml::from_str(&base_config).unwrap();

        // Before prepare_policy, user should be None since we only defined default rules
        assert!(policy.user.is_none());

        policy.prepare().unwrap();

        // Default should only have RULE 1
        let default_rules = &policy.default.unwrap().rules;
        assert_eq!(default_rules.len(), 1);
        assert!(
            matches!(&default_rules[0], Rule::ConnectUser { user, .. } if user == &DEFAULT_USER_ID.to_string())
        );

        // After prepare_policy, user contexts should be created for alice and bob
        // Alice and Bob should have their connect rules moved to their contexts
        let user_rules = policy.user.as_ref().unwrap();

        // Alice gets RULE 2
        let alice_rules = &user_rules.get("alice").unwrap().rules;
        assert_eq!(alice_rules.len(), 1);
        assert!(
            matches!(&alice_rules[0], Rule::ConnectUser { user, allow: true, .. } if user == "alice")
        );

        // Bob gets RULE 3
        let bob_rules = &user_rules.get("bob").unwrap().rules;
        assert_eq!(bob_rules.len(), 1);
        assert!(
            matches!(&bob_rules[0], Rule::ConnectUser { user, allow: false, .. } if user == "bob")
        );
    }

    #[test]
    fn test_optimization() {
        // With this policy, before the optimization:
        // - Alice gets rule 1
        // - Bob gets rule 2
        // - Default context has rule 3 and 4
        // - Charlie gets rule 5 and 6
        // - Lorax doesn't have any rules
        //
        // After the optimization:
        // - Default context only has rule 4
        // - Charlie gets rule 6
        // - Alice, Bob, and Lorax don't get any rules so their contexts are removed
        let base_config = format!(
            r#"
            [default]
            rules = [
                {{ allow = true, user = "alice" }},                # RULE 1
                {{ allow = false, user = "bob" }},                 # RULE 2
                {{ allow = true, user = "{DEFAULT_USER_ID}" }},    # RULE 3
                {{ allow = false, user = "{DEFAULT_USER_ID}" }},   # RULE 4
                {{ allow = false, user = "charlie" }},             # RULE 5
                {{ allow = true, user = "charlie" }}               # RULE 6
            ]
            [user.lorax]
            rules = []
        "#,
        );

        let mut policy: Policy = toml::from_str(&base_config).unwrap();
        policy.set_rule_priorities(&mut 0u64);
        policy.prepare().unwrap();
        policy.optimize();

        let default_rules = &policy.default.unwrap().rules;
        let user_rules = policy.user.unwrap();

        // Test default context only has one rule  (RULE 4)
        assert_eq!(default_rules.len(), 1);
        assert!(matches!(
            &default_rules[0],
            Rule::ConnectUser { allow: false, .. }
        ));

        // Charlie's rule (RULE 6) has higher priority than default (RULE 4)
        let charlie_rules = &user_rules.get("charlie").unwrap().rules;
        assert_eq!(charlie_rules.len(), 1);
        assert!(matches!(
            &charlie_rules[0],
            Rule::ConnectUser { allow: true, .. }
        ));

        // Alice's (RULE 1) and Bob's (RULE 2) had lower priority than default, so their contexts are removed
        // Lorax didn't have any rule
        assert!(!user_rules.contains_key("alice"));
        assert!(!user_rules.contains_key("bob"));
        assert!(!user_rules.contains_key("lorax"));
    }

    #[test]
    fn test_optimization_preserves_non_connect_rules() {
        let base_config = format!(
            r#"
            [default]
            rules = [
                {{ allow = true, user = "alice" }},                         # RULE 1
                {{ allow = false, user = "{DEFAULT_USER_ID}" }},            # RULE 2
            ]

            [user.alice]
            rules = [
                {{ allow = true, send_interface = "org.example.Alice" }},   # RULE 3
                {{ allow = true, own = "org.example.AliceService" }}        # RULE 4
            ]
        "#,
        );

        let mut policy: Policy = toml::from_str(&base_config).unwrap();
        policy.set_rule_priorities(&mut 0u64);
        policy.prepare().unwrap();
        policy.optimize();

        let user_rules = policy.user.as_ref().unwrap();
        let alice_rules = &user_rules.get("alice").unwrap().rules;

        // Alice's connect rule (RULE 1) is removed because it has lower priority than RULE 2
        let connect_rules: Vec<_> = alice_rules
            .iter()
            .filter(|r| matches!(r, Rule::ConnectUser { .. }))
            .collect();
        assert_eq!(connect_rules.len(), 0);

        // Alice should keep her non-connect rules
        let send_rules: Vec<_> = alice_rules
            .iter()
            .filter(|r| matches!(r, Rule::Send { .. }))
            .collect();
        // RULE 3
        assert_eq!(send_rules.len(), 1);

        let own_rules: Vec<_> = alice_rules
            .iter()
            .filter(|r| matches!(r, Rule::Own { .. }))
            .collect();
        // RULE 4
        assert_eq!(own_rules.len(), 1);
    }

    #[test]
    fn test_optimization_with_missing_default_connect_rule() {
        // Since there isn't a connect rule for the default user, each user gets its own
        // user context in the final policy
        // All users keep one rule with verdict "true"
        let base_config = r#"
            [default]
            rules = [
                { allow=false, user="pixie" },
                { allow=true, user="pixie" },
                { allow=false, user="bob" },
                { allow=true, user="bob" },
                { allow=false, user="benny" },
                { allow=true, user="benny" }
            ]
        "#;
        let mut policy: Policy = toml::from_str(base_config).unwrap();
        policy.set_rule_priorities(&mut 0u64);
        policy.prepare().unwrap();
        policy.optimize();
        let user_context = policy.user.unwrap();

        assert_eq!(user_context.get("pixie").unwrap().rules.len(), 1);
        assert_eq!(user_context.get("bob").unwrap().rules.len(), 1);
        assert_eq!(user_context.get("benny").unwrap().rules.len(), 1);

        assert!(user_context
            .values()
            .all(|p| matches!(p.rules[0], Rule::ConnectUser { allow, .. } if allow)));
    }

    #[test]
    fn test_user_context_fails_with_connect_rules() {
        let base_config: &str = r#"

        [user.pixie]
        rules = [
            { allow = true, user = "pixie" }
        ]
        "#;

        let policy: Policy = toml::from_str(base_config).unwrap();
        assert!(policy.validate().is_err());
    }

    #[test]
    fn test_find_priority_threshold_returns_highest_priority() {
        let rules: Vec<Rule> = vec![
            Rule::ConnectUser {
                allow: true,
                user: "".to_owned(),
                priority: 1,
            },
            Rule::ConnectUser {
                allow: true,
                user: "".to_owned(),
                priority: 2,
            },
            Rule::ConnectUser {
                allow: true,
                user: "".to_owned(),
                priority: 3,
            },
        ];
        assert_eq!(find_priority_threshold(&rules), 3);
    }

    #[test]
    fn test_find_priority_threshold_defaults_to_zero() {
        let rules: Vec<Rule> = vec![];
        assert_eq!(find_priority_threshold(&rules), 0);
    }

    #[test]
    fn test_clean_connect_rules() {
        let mut rules: Vec<Rule> = vec![
            Rule::ConnectUser {
                allow: true,
                user: "".to_owned(),
                priority: 1,
            },
            Rule::ConnectUser {
                allow: true,
                user: "".to_owned(),
                priority: 2,
            },
            Rule::ConnectUser {
                allow: true,
                user: "".to_owned(),
                priority: 3,
            },
        ];

        clean_connect_rules(4, &mut rules);
        assert!(rules.is_empty());
    }

    #[test_case(Rule::Send {
            allow: false,
            priority: u64::default(),
            send_broadcast: u32::default(),
            send_destination: "*".to_string(),
            send_interface: "*".to_string(),
            send_member: "*".to_string(),
            send_path: "*".to_string(),
            send_type: MessageType::default(),
        }; "with a send rule wildcards are replaced with empty strings")]
    #[test_case(Rule::Receive {
            allow: false,
            priority: u64::default(),
            receive_broadcast: u32::default(),
            receive_interface: "*".to_string(),
            receive_member: "*".to_string(),
            receive_path: "*".to_string(),
            receive_sender: "*".to_string(),
            receive_type: MessageType::default(),
        }; "with a receive rule wildcards are replaced with empty strings")]
    fn rules_with_wildcards(rule: Rule) {
        assert_eq!(rule.name().unwrap(), "");
        assert_eq!(rule.interface().unwrap(), "");
        assert_eq!(rule.member().unwrap(), "");
        assert_eq!(rule.path().unwrap(), "");
    }

    #[test_case(Rule::Send {
            allow: false,
            priority: u64::default(),
            send_broadcast: u32::default(),
            send_destination: "name".to_string(),
            send_interface: "interface".to_string(),
            send_member: "member".to_string(),
            send_path: "path".to_string(),
            send_type: MessageType::default(),
        }; "with a send rule the original value is returned")]
    #[test_case(Rule::Receive {
            allow: false,
            priority: u64::default(),
            receive_broadcast: u32::default(),
            receive_interface: "interface".to_string(),
            receive_member: "member".to_string(),
            receive_path: "path".to_string(),
            receive_sender: "name".to_string(),
            receive_type: MessageType::default(),
        }; "with a receive rule the original value is returned")]
    fn rules_without_wildcards(rule: Rule) {
        assert_eq!(rule.name().unwrap(), "name");
        assert_eq!(rule.interface().unwrap(), "interface");
        assert_eq!(rule.member().unwrap(), "member");
        assert_eq!(rule.path().unwrap(), "path");
    }
}
