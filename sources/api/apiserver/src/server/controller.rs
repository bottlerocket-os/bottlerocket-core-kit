//! The controller module maps between the datastore and the API interface, similar to the
//! controller in the MVC model.

use bottlerocket_release::BottlerocketRelease;
use model::generator::{RawSettingsGenerator, SettingsGenerator, Strength};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use snafu::{ensure, OptionExt, ResultExt};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::process::{Command, Stdio};

use crate::server::error::{self, Result};
use actix_web::HttpResponse;
use datastore::constraints_check::{ApprovedWrite, ConstraintCheckResult};
use datastore::deserialization::{from_map, from_map_with_prefix};
use datastore::serialization::to_pairs_with_prefix;
use datastore::{
    deserialize_scalar, serialize_scalar, Committed, DataStore, Key, KeyType, ScalarError, Value,
};
use model::{ConfigurationFiles, Services, Settings};
use num::FromPrimitive;
use std::os::unix::process::ExitStatusExt;
use thar_be_updates::error::TbuErrorStatus;

/// List the open transactions from the data store.
pub(crate) fn list_transactions<D>(datastore: &D) -> Result<HashSet<String>>
where
    D: DataStore,
{
    datastore
        .list_transactions()
        .context(error::DataStoreSnafu {
            op: "list_transactions",
        })
}

/// Build a Settings based on pending data in the datastore; the Settings will be empty if there
/// are no pending settings.
pub(crate) fn get_transaction<D, S>(datastore: &D, transaction: S) -> Result<Settings>
where
    D: DataStore,
    S: Into<String>,
{
    let pending = Committed::Pending {
        tx: transaction.into(),
    };
    get_prefix(datastore, &pending, "settings.", None)
        .map(|maybe_settings| maybe_settings.unwrap_or_default())
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct SettingsMetadata {
    pub(crate) inner: HashMap<String, HashMap<String, String>>,
}

impl From<HashMap<Key, HashMap<Key, String>>> for SettingsMetadata {
    fn from(transaction_metadata: HashMap<Key, HashMap<Key, String>>) -> Self {
        let mut metadata = HashMap::new();
        for (key, value) in transaction_metadata {
            let mut inner_map = HashMap::new();
            for (inner_key, inner_value) in value {
                inner_map.insert(inner_key.name().clone(), inner_value);
            }
            metadata.insert(key.name().clone(), inner_map);
        }

        SettingsMetadata { inner: metadata }
    }
}

/// Gets the metadata for metadata_key_name in the given transaction
/// Returns all metadata if metadata_key_name is None
pub(crate) fn get_transaction_metadata<D, S>(
    datastore: &D,
    transaction: S,
    metadata_key_name: Option<String>,
) -> Result<SettingsMetadata>
where
    D: DataStore,
    S: Into<String>,
{
    let pending = Committed::Pending {
        tx: transaction.into(),
    };

    let metadata = datastore
        .get_metadata_prefix("settings.", &pending, &metadata_key_name)
        .with_context(|_| error::DataStoreSnafu {
            op: format!("get_metadata_prefix '{}' for {:?}", "settings.", pending),
        })?;

    Ok(SettingsMetadata::from(metadata))
}

/// Deletes the transaction from the data store, removing any uncommitted settings under that
/// transaction name.
pub(crate) fn delete_transaction<D: DataStore>(
    datastore: &mut D,
    transaction: &str,
) -> Result<HashSet<Key>> {
    datastore
        .delete_transaction(transaction)
        .context(error::DataStoreSnafu {
            op: "delete_pending",
        })
}

/// check_prefix is a helper for get_*_prefix functions that determines what prefix to use when
/// checking whether settings match the prefix.  Pass in the prefix that was given in the API
/// request, and the expected prefix of settings in the subject area (like "settings." or
/// "services.") and it will return the prefix you should use to filter, or None if the prefix
/// can't match.
fn check_prefix<'a>(given: &'a str, expected: &'static str) -> Option<&'a str> {
    if expected.starts_with(given) {
        // Example: expected "settings." and given "se" - return "settings." since querying for
        // "se" can be ambiguous with other values ("services") that can't be deserialized into a
        // Settings.
        return Some(expected);
    }

    if given.starts_with(expected) {
        // Example: expected "settings." and given "settings.motd" - return the more specific
        // "settings.motd" so the user only gets what they clearly want to see.
        return Some(given);
    }

    // No overlap, we won't find any data and should return early.
    None
}

/// Build a Settings based on the data in the datastore.  Errors if no settings are found.
pub(crate) fn get_settings<D: DataStore>(datastore: &D, committed: &Committed) -> Result<Settings> {
    get_prefix(datastore, committed, "settings.", None)
        .transpose()
        // None is not OK here - we always have *some* settings
        .context(error::MissingDataSnafu { prefix: "settings" })?
}

/// Build a Settings based on the data in the datastore that begins with the given prefix.
pub(crate) fn get_settings_prefix<D: DataStore, S: AsRef<str>>(
    datastore: &D,
    prefix: S,
    committed: &Committed,
) -> Result<Option<Settings>> {
    // Return early if the prefix can't match settings.  (This is important because get_model
    // checks all of our model types using the same given prefix.)
    let prefix = match check_prefix(prefix.as_ref(), "settings.") {
        Some(prefix) => prefix,
        None => return Ok(None),
    };

    get_prefix(datastore, committed, prefix, None)
        .transpose()
        // None is OK here - they could ask for a prefix we don't have
        .unwrap_or_else(|| Ok(None))
}

// The "os" APIs don't deal with the data store at all, they just read a release field.
/// Build a BottlerocketRelease using the bottlerocket-release library.
pub(crate) fn get_os_info() -> Result<BottlerocketRelease> {
    BottlerocketRelease::new().context(error::ReleaseDataSnafu)
}

/// Build a BottlerocketRelease using the bottlerocket-release library, returning only the fields
/// that start with the given prefix.  If the prefix was meant for another structure, we return
/// None, making it easier to decide whether to include an empty structure in API results.
pub(crate) fn get_os_prefix<S>(prefix: S) -> Result<Option<serde_json::Value>>
where
    S: AsRef<str>,
{
    let prefix = prefix.as_ref();

    // Return early if the prefix can't match os data.  (This is important because get_model checks
    // all of our model types using the same given prefix.)
    let prefix = match check_prefix(prefix, "os.") {
        Some(prefix) => prefix,
        None => return Ok(None),
    };

    // We're not using the data store here, there are no dotted keys, we're just matching against
    // field names.  Strip off the structure-level prefix.
    let field_prefix = prefix.trim_start_matches("os.");

    let os = BottlerocketRelease::new().context(error::ReleaseDataSnafu)?;

    // Turn into a serde Value we can manipulate.
    let val = serde_json::to_value(os).expect("struct to value can't fail");

    // Structs are Objects in serde_json, which have a map of field -> value inside.  We
    // destructure to get it by value, instead of as_object() which gives references.
    let map = match val {
        Value::Object(map) => map,
        _ => panic!("structs are always objects"),
    };

    // Keep the fields whose names match the requested prefix.
    let filtered = map
        .into_iter()
        .filter(|(field_name, _val)| field_name.starts_with(field_prefix))
        .collect();

    Ok(Some(filtered))
}

/// Build a Services based on the data in the datastore.
pub(crate) fn get_services<D: DataStore>(datastore: &D) -> Result<Services> {
    get_prefix(
        datastore,
        &Committed::Live,
        "services.",
        Some("services".to_string()),
    )
    .transpose()
    // None is not OK here - we always have services
    .context(error::MissingDataSnafu { prefix: "services" })?
}

/// Build a Services based on the data in the datastore, returning only the fields that start with
/// the given prefix.  If the prefix was meant for another structure, we return None, making it
/// easier to decide whether to include an empty structure in API results.
pub(crate) fn get_services_prefix<D, S>(datastore: &D, prefix: S) -> Result<Option<Services>>
where
    D: DataStore,
    S: AsRef<str>,
{
    // Return early if the prefix can't match services.  (This is important because get_model
    // checks all of our model types using the same given prefix.)
    let prefix = match check_prefix(prefix.as_ref(), "services.") {
        Some(prefix) => prefix,
        None => return Ok(None),
    };

    get_prefix(
        datastore,
        &Committed::Live,
        prefix,
        Some("services".to_string()),
    )
    .transpose()
    // None is OK here - they could ask for a prefix we don't have
    .unwrap_or(Ok(None))
}

/// Build a ConfigurationFiles based on the data in the datastore.
pub(crate) fn get_configuration_files<D: DataStore>(datastore: &D) -> Result<ConfigurationFiles> {
    get_prefix(
        datastore,
        &Committed::Live,
        "configuration-files",
        Some("configuration-files".to_string()),
    )
    .transpose()
    // None is not OK here - we always have configuration files
    .context(error::MissingDataSnafu {
        prefix: "configuration-files",
    })?
}

/// Build a ConfigurationFiles based on the data in the datastore, returning only the fields that
/// start with the given prefix.  If the prefix was meant for another structure, we return None,
/// making it easier to decide whether to include an empty structure in API results.
pub(crate) fn get_configuration_files_prefix<D, S>(
    datastore: &D,
    prefix: S,
) -> Result<Option<ConfigurationFiles>>
where
    D: DataStore,
    S: AsRef<str>,
{
    // Return early if the prefix can't match configuration-files.  (This is important because
    // get_model checks all of our model types using the same given prefix.)
    let prefix = match check_prefix(prefix.as_ref(), "configuration-files.") {
        Some(prefix) => prefix,
        None => return Ok(None),
    };

    get_prefix(
        datastore,
        &Committed::Live,
        prefix,
        Some("configuration-files".to_string()),
    )
    .transpose()
    // None is OK here - they could ask for a prefix we don't have
    .unwrap_or(Ok(None))
}

/// Helper to get data from the datastore, starting with the given find_prefix, and deserialize it
/// into the desired type.  map_prefix should be the prefix to remove if you're deserializing into
/// a map; see docs on from_map_with_prefix.  Returns Err if we couldn't pull expected data;
/// returns Ok(None) if we found there were no populated keys.
fn get_prefix<D, T, S>(
    datastore: &D,
    committed: &Committed,
    find_prefix: S,
    map_prefix: Option<String>,
) -> Result<Option<T>>
where
    D: DataStore,
    T: DeserializeOwned,
    S: AsRef<str>,
{
    let find_prefix = find_prefix.as_ref();

    let data = datastore
        .get_prefix(find_prefix, committed)
        .with_context(|_| error::DataStoreSnafu {
            op: format!("get_prefix '{find_prefix}' for {committed:?}"),
        })?;
    if data.is_empty() {
        return Ok(None);
    }

    from_map_with_prefix(map_prefix, &data)
        .context(error::DeserializationSnafu { given: find_prefix })
}

/// Build a Settings based on the data in the datastore for the given keys.
pub(crate) fn get_settings_keys<D: DataStore>(
    datastore: &D,
    keys: &HashSet<&str>,
    committed: &Committed,
) -> Result<Settings> {
    let mut data = HashMap::new();
    for key_str in keys {
        trace!("Pulling value from datastore for key: {key_str}");
        let key = Key::new(KeyType::Data, key_str).context(error::NewKeySnafu {
            key_type: "data",
            name: *key_str,
        })?;
        let value = match datastore
            .get_key(&key, committed)
            .context(error::DataStoreSnafu { op: "get_key" })?
        {
            Some(v) => v,
            // TODO: confirm we want to skip requested keys if not populated, or error
            None => continue,
        };
        data.insert(key, value);
    }

    let settings = from_map(&data).context(error::DeserializationSnafu {
        given: "given keys",
    })?;
    Ok(settings)
}

/// Build a collection of Service items with the given names using data from the datastore.
pub(crate) fn get_services_names<D: DataStore>(
    datastore: &D,
    names: &HashSet<&str>,
    committed: &Committed,
) -> Result<Services> {
    get_map_from_prefix(datastore, "services.".to_string(), names, committed)
}

/// Build a collection of ConfigurationFile items with the given names using data from the
/// datastore.
pub(crate) fn get_configuration_files_names<D: DataStore>(
    datastore: &D,
    names: &HashSet<&str>,
    committed: &Committed,
) -> Result<ConfigurationFiles> {
    get_map_from_prefix(
        datastore,
        "configuration-files.".to_string(),
        names,
        committed,
    )
}

/// Helper to get data from the datastore for a collection of requested items under a given prefix.  For
/// example, a collection of Service items under "services" that have the requested names.
/// Returns Err if we couldn't pull expected data, including the case where a name was specified
/// for which we have no data.
fn get_map_from_prefix<D: DataStore, T>(
    datastore: &D,
    prefix: String,
    names: &HashSet<&str>,
    committed: &Committed,
) -> Result<HashMap<String, T>>
where
    T: DeserializeOwned,
{
    let mut result = HashMap::new();
    for &name in names {
        let item_prefix = prefix.clone() + name;

        let item_data = datastore
            .get_prefix(&item_prefix, committed)
            .with_context(|_| error::DataStoreSnafu {
                op: format!("get_prefix '{}' for {:?}", item_prefix, committed),
            })?;

        ensure!(
            !item_data.is_empty(),
            error::ListKeysSnafu {
                requested: item_prefix
            }
        );

        let item = from_map_with_prefix(Some(item_prefix.clone()), &item_data)
            .context(error::DeserializationSnafu { given: item_prefix })?;
        result.insert(name.to_string(), item);
    }

    Ok(result)
}

/// Given a Settings, takes any Some values and updates them in the datastore.
pub(crate) fn set_settings<D: DataStore>(
    datastore: &mut D,
    settings: &Settings,
    transaction: &str,
    strength: Strength,
) -> Result<()> {
    trace!("Serializing Settings to write to data store");
    let settings_json = serde_json::to_value(settings).context(error::SettingsToJsonSnafu)?;
    let pairs = to_pairs_with_prefix("settings", &settings_json)
        .context(error::DataStoreSerializationSnafu { given: "Settings" })?;
    let pending = Committed::Pending {
        tx: transaction.into(),
    };

    info!("Writing Metadata to pending transaction in datastore.");

    // Write the metadata to pending transaction as provided.
    // We will validate this transaction while committing.
    for key in pairs.keys() {
        let metadata_key_strength =
            Key::new(KeyType::Meta, "strength").context(error::NewKeySnafu {
                key_type: "meta",
                name: "strength",
            })?;

        let metadata_value = datastore::serialize_scalar::<_, ScalarError>(&strength.to_string())
            .with_context(|_| error::SerializeSnafu {})?;

        datastore
            .set_metadata(&metadata_key_strength, key, metadata_value, &pending)
            .context(error::DataStoreSnafu {
                op: "Failure in setting metadata.",
            })?;
    }

    info!("Writing Settings to pending transaction in datastore: {pairs:?}");
    datastore
        .set_keys(&pairs, &pending)
        .context(error::DataStoreSnafu { op: "set_keys" })
}

// This is not as nice as get_settings, which uses Serializer/Deserializer to properly use the
// data model and check types.
/// Gets the value of a metadata key for the requested list of data keys.
pub(crate) fn get_metadata_for_data_keys<D: DataStore, S: AsRef<str>>(
    datastore: &D,
    md_key_str: S,
    data_key_strs: &HashSet<&str>,
) -> Result<HashMap<String, Value>> {
    trace!("Getting metadata '{}'", md_key_str.as_ref());
    let md_key = Key::new(KeyType::Meta, md_key_str.as_ref()).context(error::NewKeySnafu {
        key_type: "meta",
        name: md_key_str.as_ref(),
    })?;

    let mut result = HashMap::new();
    for data_key_str in data_key_strs {
        trace!("Pulling metadata from datastore for key: {data_key_str}");
        let data_key = Key::new(KeyType::Data, data_key_str).context(error::NewKeySnafu {
            key_type: "data",
            name: *data_key_str,
        })?;
        let value_str = match datastore.get_metadata(&md_key, &data_key, &Committed::Live) {
            Ok(Some(v)) => v,
            // TODO: confirm we want to skip requested keys if not populated, or error
            Ok(None) => continue,
            // May want to make it possible to receive an error if a metadata key doesn't
            // exist, but to start, we expect to request metadata for multiple keys and not all
            // of them will necessarily have the metadata.
            Err(_) => continue,
        };
        trace!("Deserializing scalar from metadata");
        let value: Value = deserialize_scalar::<_, ScalarError>(&value_str).context(
            error::InvalidMetadataSnafu {
                key: md_key.name(),
                data_key: data_key.name(),
            },
        )?;
        result.insert(data_key.to_string(), value);
    }

    Ok(result)
}

// Sort the given settings hashmap where a parent
// key is always before its successors.
fn sort_metadata(metadata: HashMap<Key, HashMap<Key, String>>) -> Vec<(Key, HashMap<Key, String>)> {
    let mut metadata_sorted: Vec<_> = metadata.into_iter().collect();
    metadata_sorted.sort_by_key(|(k1, _)| k1.segments().len());
    metadata_sorted.to_vec()
}

/// Settings generators can be dynamically applied to a set of settings.
///
/// This function resolves a dynamically expanded generator into a finite set of
/// distinct settings generators.
fn expand_setting_generator<D: DataStore>(
    datastore: &D,
    data_key: &Key,
    raw_setting_generator: RawSettingsGenerator,
    result: &mut HashMap<String, Value>,
) -> Result<()> {
    let depth = raw_setting_generator.depth;
    let metadata_value = serde_json::to_value(SettingsGenerator::from(raw_setting_generator))
        .context(error::SerializeSnafu)?;

    if depth == 0 {
        // If the depth is 0, we don't need to process the successors
        result.insert(data_key.to_string(), metadata_value);
        return Ok(());
    }

    // Get the leaf of the data key
    let segments = data_key.segments();
    let data_key_for_metadata = segments.last().context(error::InvalidKeyPairSnafu {
        input: data_key.name(),
    })?;

    // Extract the prefix leaving the leaf
    let parent_key =
        Key::from_segments(KeyType::Data, &segments[..segments.len().saturating_sub(1)]).context(
            error::DataStoreSnafu {
                op: format!("Unable to create key from segments: {segments:?}"),
            },
        )?;

    let all_child_keys = datastore
        .list_populated_keys(parent_key.name(), &Committed::Live)
        .context(error::DataStoreSnafu {
            op: format!(
                "Unable to get the populated keys for given prefix: {}",
                parent_key.name()
            ),
        })?;

    let final_segment_length = segments.len() + depth as usize;

    let child_keys_at_target_depth = all_child_keys
        .iter()
        .filter_map(|child_key| child_key.segments().get(..(final_segment_length)))
        .map(|child_segments| {
            let mut child_segments = child_segments.to_vec();
            child_segments.pop();
            child_segments.push(data_key_for_metadata.to_string());
            Key::from_segments(KeyType::Data, &child_segments).context(error::DataStoreSnafu {
                op: "Key from segments",
            })
        })
        .collect::<Result<HashSet<_>>>()?;

    for child_key in child_keys_at_target_depth {
        result.insert(child_key.to_string(), metadata_value.to_owned());
    }

    Ok(())
}

/// Gets the value of a settings-generator metadata key everywhere it's found in the data store
/// and returns a mapping of data key to the metadata value associated with the requested key.
pub(crate) fn get_settings_generator_metadata<D: DataStore, S: AsRef<str>>(
    datastore: &D,
    md_key_str: S,
) -> Result<HashMap<String, Value>> {
    trace!("Getting metadata '{}'", md_key_str.as_ref());
    let meta_map = get_metadata_for_all_data_keys(datastore, md_key_str.as_ref())?;

    // Sort the returned metadata so that we process parent first.
    let sorted_metadata = sort_metadata(meta_map);

    let mut result = HashMap::new();
    for (data_key, metadata) in sorted_metadata {
        for (meta_key, value_str) in metadata {
            trace!("Deserializing scalar from metadata");
            let value: Value = deserialize_scalar::<_, ScalarError>(&value_str).context(
                error::InvalidMetadataSnafu {
                    key: meta_key.name(),
                    data_key: data_key.name(),
                },
            )?;

            let setting_generator: RawSettingsGenerator = serde_json::from_value(value.clone())
                .context(error::DeserializeSettingsGeneratorSnafu)?;

            trace!(
                "Getting the metadata for key {data_key:?} with setting generator {setting_generator:?}"
            );
            expand_setting_generator(datastore, &data_key, setting_generator, &mut result)?;
        }
    }

    Ok(result)
}

/// Gets the value of a metadata key everywhere it's found in the data store.  Returns a mapping
/// of data key to the metadata Key and value associated with the requested key.
pub(crate) fn get_metadata_for_all_data_keys<D: DataStore, S: AsRef<str>>(
    datastore: &D,
    md_key_str: S,
) -> Result<HashMap<Key, HashMap<Key, String>>> {
    trace!("Getting metadata '{}'", md_key_str.as_ref());
    let result = datastore
        .get_metadata_prefix("", &Committed::Live, &Some(md_key_str))
        .context(error::DataStoreSnafu {
            op: "get_metadata_prefix",
        })?;

    Ok(result)
}

// Parses and validates the settings and metadata in pending transaction and
// returns the constraint check result containing approved settings and metadata to
// commit to live transaction.
// We will pass this function as argument to commit transaction function.
fn datastore_transaction_check<D>(
    datastore: &mut D,
    committed: &Committed,
) -> Result<ConstraintCheckResult>
where
    D: DataStore,
{
    // Get settings to commit from pending transaction
    let settings_to_commit = datastore
        .get_prefix("settings.", committed)
        .context(error::DataStoreSnafu { op: "get_prefix" })?;

    // Get metadata from pending transaction
    let mut transaction_metadata = datastore
        .get_metadata_prefix("settings.", committed, &None as &Option<&str>)
        .context(error::DataStoreSnafu {
            op: "get_metadata_prefix",
        })?;

    let mut metadata_to_commit: Vec<(Key, Key, String)> = Vec::new();

    // Parse and validate all the metadata enteries from pending transaction
    for (key, value) in transaction_metadata.iter_mut() {
        for (metadata_key, metadata_value) in value {
            // For now we are only processing the strength metadata from pending
            // transaction to live
            if metadata_key.name() != "strength" {
                warn!(
                    "Metadata key is {}, Skipping the commit as we only allow Strength metadata.",
                    metadata_key.name()
                );
                continue;
            }

            // strength in pending transaction
            let pending_strength =
                deserialize_scalar::<Strength, ScalarError>(metadata_value.as_ref())
                    .with_context(|_| error::DeserializeStrengthSnafu {})?;

            // Get the setting strength in live
            // get_metadata function returns Ok(None) in case strength does not exist
            // We will consider this case as strength equals strong.
            let committed_strength: Strength = datastore
                .get_metadata(metadata_key, key, &Committed::Live)
                .context(error::DataStoreSnafu { op: "get_metadata" })?
                .map(|x| deserialize_scalar::<Strength, ScalarError>(x.as_ref()))
                .transpose()
                .context(error::DeserializeStrengthSnafu {})?
                .unwrap_or_default();

            let may_be_value = datastore
                .get_key(key, &Committed::Live)
                .context(error::DataStoreSnafu { op: "get_key" })?;

            trace!(
                "datastore_transaction_check: key: {:?}, metadata_key: {:?}, metadata_value: {:?}",
                key.name(),
                metadata_key.name(),
                metadata_value
            );

            match (pending_strength, committed_strength, may_be_value) {
                (Strength::Weak, Strength::Strong, None)
                | (Strength::Strong, Strength::Weak, Some(..))
                | (Strength::Strong, Strength::Weak, None) => {
                    let met_value = serialize_scalar::<_, ScalarError>(&pending_strength)
                        .with_context(|_| error::SerializeSnafu {})?;

                    metadata_to_commit.push((metadata_key.clone(), key.clone(), met_value));
                }
                (Strength::Weak, Strength::Strong, Some(..)) => {
                    // Do not change from strong to weak if setting exists
                    // as the default strength is strong for a setting.
                    return Ok(ConstraintCheckResult::Reject(format!(
                        "Cannot change setting {} strength from strong to weak",
                        key.name()
                    )));
                }
                (Strength::Weak, Strength::Weak, ..) => {
                    trace!("The strength for setting {} is already weak", key.name());
                    continue;
                }
                (Strength::Strong, Strength::Strong, ..) => {
                    trace!("The strength for setting {} is already strong", key.name());
                    continue;
                }
            };
        }
    }

    let approved_write = ApprovedWrite {
        settings: settings_to_commit,
        metadata: metadata_to_commit,
    };

    Ok(ConstraintCheckResult::from(Some(approved_write)))
}

/// Wrapper for our transaction constraint check that converts errors into the expected type
fn datastore_transaction_check_wrapper<D>(
    datastore: &mut D,
    committed: &Committed,
) -> Result<ConstraintCheckResult, Box<dyn std::error::Error + Send + Sync + 'static>>
where
    D: DataStore,
{
    datastore_transaction_check::<D>(datastore, committed)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

/// Makes live any pending settings in the datastore, returning the changed keys.
pub(crate) fn commit_transaction<D>(datastore: &mut D, transaction: &str) -> Result<HashSet<Key>>
where
    D: DataStore,
{
    datastore
        .commit_transaction(transaction, &datastore_transaction_check_wrapper::<D>)
        .context(error::DataStoreSnafu { op: "commit" })
}

/// Launches the config applier to make appropriate changes to the system based on any settings
/// that have been committed.  Can be called after a commit, with the keys that changed in that
/// commit, or called on its own to reset configuration state with all known keys.
///
/// If `keys_limit` is Some, gives those keys to the applier so only changes relevant to those
/// keys are made.  Otherwise, tells the applier to apply changes for all known keys.
pub(crate) fn apply_changes<S>(keys_limit: Option<&HashSet<S>>) -> Result<()>
where
    S: AsRef<str>,
{
    if let Some(keys_limit) = keys_limit {
        let keys_limit: Vec<&str> = keys_limit.iter().map(|s| s.as_ref()).collect();
        // Prepare input to config applier; it uses the changed keys to update the right config
        trace!("Serializing the commit's changed keys: {keys_limit:?}");
        let cmd_input =
            serde_json::to_string(&keys_limit).context(error::CommandSerializationSnafu {
                given: "commit's changed keys",
            })?;

        // Start config applier
        debug!("Launching thar-be-settings to apply changes");
        let mut cmd = Command::new("/usr/bin/thar-be-settings")
            // Ask it to fork itself so we don't block the API
            .arg("--daemon")
            .stdin(Stdio::piped())
            // FIXME where to send output?
            //.stdout()
            //.stderr()
            .spawn()
            .context(error::ConfigApplierStartSnafu)?;

        // Send changed keys to config applier
        trace!("Sending changed keys");
        cmd.stdin
            .as_mut()
            .context(error::ConfigApplierStdinSnafu)?
            .write_all(cmd_input.as_bytes())
            .context(error::ConfigApplierWriteSnafu)?;

        // The config applier forks quickly; this wait ensures we don't get a zombie from its
        // initial process.  Its child is reparented to init and init waits for that one.
        let status = cmd.wait().context(error::ConfigApplierWaitSnafu)?;
        // Similarly, this is just checking that it was able to fork, not checking its work.
        ensure!(
            status.success(),
            error::ConfigApplierForkSnafu {
                code: status
                    .code()
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
            }
        );
    } else {
        // Start config applier
        // (See comments above about daemonizing and checking the fork result; we don't need a
        // separate wait() here because we don't pass any stdin, status() does it for us.)
        debug!("Launching thar-be-settings to apply any and all changes");
        let status = Command::new("/usr/bin/thar-be-settings")
            .arg("--daemon")
            .arg("--all")
            // FIXME where to send output?
            //.stdout()
            //.stderr()
            .status()
            .context(error::ConfigApplierStartSnafu)?;
        ensure!(
            status.success(),
            error::ConfigApplierForkSnafu {
                code: status
                    .code()
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
            }
        );
    }

    Ok(())
}

/// Dispatches an update command via `thar-be-updates`
pub(crate) fn dispatch_update_command(args: &[&str]) -> Result<HttpResponse> {
    let status = Command::new("/usr/bin/thar-be-updates")
        .args(args)
        .status()
        .context(error::UpdateDispatcherSnafu)?;
    if status.success() {
        return Ok(HttpResponse::NoContent().finish());
    }
    let exit_status = match status.code() {
        Some(code) => code,
        None => status.signal().unwrap_or(1),
    };
    let error_type = FromPrimitive::from_i32(exit_status);
    let error = match error_type {
        Some(TbuErrorStatus::UpdateLockHeld) => error::Error::UpdateLockHeld,
        Some(TbuErrorStatus::DisallowCommand) => error::Error::DisallowCommand,
        Some(TbuErrorStatus::UpdateDoesNotExist) => error::Error::UpdateDoesNotExist,
        Some(TbuErrorStatus::NoStagedImage) => error::Error::NoStagedImage,
        // other errors
        _ => error::Error::UpdateError,
    };
    Err(error)
}

#[cfg(test)]
mod test {
    use super::*;
    use datastore::memory::MemoryDataStore;
    use datastore::{Committed, DataStore, Key, KeyType};
    use maplit::{hashmap, hashset};
    use model::{ConfigurationFile, Service};
    use serde::{Deserialize, Serialize};
    use std::convert::TryInto;

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    struct TestSettings {
        motd: Option<String>,
        ntp: Option<NtpSettings>,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    struct NtpSettings {
        time_servers: Option<String>,
    }

    macro_rules! extract {
        ($settings:ident.$field:ident) => {{
            let json = serde_json::to_string(&$settings).unwrap();
            let settings = serde_json::from_str::<TestSettings>(&json).unwrap();
            settings.$field
        }};
    }

    #[test]
    fn get_settings_works() {
        let mut ds = MemoryDataStore::new();
        // Set directly with data store
        ds.set_key(
            &Key::new(KeyType::Data, "settings.motd").unwrap(),
            "\"json string\"",
            &Committed::Live,
        )
        .unwrap();

        // Retrieve with helper
        let settings = get_settings(&ds, &Committed::Live).unwrap();
        assert_eq!(extract!(settings.motd), Some("json string".into()));
    }

    #[test]
    fn get_settings_prefix_works() {
        let mut ds = MemoryDataStore::new();
        // Set directly with data store
        ds.set_key(
            &Key::new(KeyType::Data, "settings.motd").unwrap(),
            "\"json string\"",
            &Committed::Live,
        )
        .unwrap();

        // Retrieve with short prefix OK
        let settings = get_settings_prefix(&ds, "settings.", &Committed::Live)
            .unwrap() // Result Ok
            .unwrap(); // got Some result
        assert_eq!(extract!(settings.motd), Some("json string".into()));

        // Retrieve with more specific prefix OK
        let settings = get_settings_prefix(&ds, "settings.mot", &Committed::Live)
            .unwrap() // Result Ok
            .unwrap(); // got Some result
        assert_eq!(extract!(settings.motd), Some("json string".into()));

        // No match should return None; the "view" layer of the API, in mod.rs, turns this into an
        // empty object if desired.
        let settings = get_settings_prefix(&ds, "settings.motdxxx", &Committed::Live).unwrap();
        assert_eq!(settings, None);

        // Unrelated prefix should return None
        let settings = get_settings_prefix(&ds, "xyz", &Committed::Live).unwrap();
        assert_eq!(settings, None);
    }

    #[test]
    fn get_settings_keys_works() {
        let mut ds = MemoryDataStore::new();
        // Set directly with data store
        ds.set_key(
            &Key::new(KeyType::Data, "settings.motd").unwrap(),
            "\"json string 1\"",
            &Committed::Live,
        )
        .unwrap();

        ds.set_key(
            &Key::new(KeyType::Data, "settings.ntp.time-servers").unwrap(),
            "\"json string 2\"",
            &Committed::Live,
        )
        .unwrap();

        // Retrieve with helper
        let settings =
            get_settings_keys(&ds, &hashset!("settings.motd"), &Committed::Live).unwrap();
        assert_eq!(extract!(settings.motd), Some("json string 1".into()));
        assert_eq!(extract!(settings.ntp), None);
    }

    #[test]
    fn get_services_names_works() {
        let mut ds = MemoryDataStore::new();
        // Set directly with data store
        ds.set_key(
            &Key::new(KeyType::Data, "services.foo.configuration-files").unwrap(),
            "[\"file1\"]",
            &Committed::Live,
        )
        .unwrap();
        ds.set_key(
            &Key::new(KeyType::Data, "services.foo.restart-commands").unwrap(),
            "[\"echo hi\"]",
            &Committed::Live,
        )
        .unwrap();

        // Retrieve built service
        let names = hashset!("foo");
        let services = get_services_names(&ds, &names, &Committed::Live).unwrap();
        assert_eq!(
            services,
            hashmap!("foo".to_string() => Service {
                configuration_files: vec!["file1".try_into().unwrap()],
                restart_commands: vec!["echo hi".to_string()]
            })
        );
    }

    #[test]
    fn get_services_prefix_works() {
        let mut ds = MemoryDataStore::new();
        // Set directly with data store
        ds.set_key(
            &Key::new(KeyType::Data, "services.foo.configuration-files").unwrap(),
            "[\"file1\"]",
            &Committed::Live,
        )
        .unwrap();
        ds.set_key(
            &Key::new(KeyType::Data, "services.foo.restart-commands").unwrap(),
            "[\"echo hi\"]",
            &Committed::Live,
        )
        .unwrap();

        // Retrieve built service OK
        let prefix = "services.foo";
        let services = get_services_prefix(&ds, prefix)
            .unwrap() // Result Ok
            .unwrap(); // got Some result
        assert_eq!(
            services,
            hashmap!("foo".to_string() => Service {
                configuration_files: vec!["file1".try_into().unwrap()],
                restart_commands: vec!["echo hi".to_string()]
            })
        );

        // No match returns None
        let prefix = "services.bar";
        let services = get_services_prefix(&ds, prefix).unwrap();
        assert_eq!(services, None);

        // Unrelated prefix returns None
        let prefix = "settings";
        let services = get_services_prefix(&ds, prefix).unwrap();
        assert_eq!(services, None);
    }

    #[test]
    fn get_configuration_files_prefix_works() {
        let mut ds = MemoryDataStore::new();
        // Set directly with data store
        ds.set_key(
            &Key::new(KeyType::Data, "configuration-files.foo.path").unwrap(),
            "\"file\"",
            &Committed::Live,
        )
        .unwrap();
        ds.set_key(
            &Key::new(KeyType::Data, "configuration-files.foo.template-path").unwrap(),
            "\"template\"",
            &Committed::Live,
        )
        .unwrap();

        // Retrieve built configuration file OK
        let prefix = "configuration-files.foo";
        let configuration_files = get_configuration_files_prefix(&ds, prefix)
            .unwrap() // Result Ok
            .unwrap(); // got Some result
        assert_eq!(
            configuration_files,
            hashmap!("foo".to_string() => ConfigurationFile {
                path: "file".try_into().unwrap(),
                template_path: "template".try_into().unwrap(),
                mode: None,
                overwrite_path_if_present: None
            })
        );

        // No match returns None
        let prefix = "configuration-files.bar";
        let configuration_files = get_configuration_files_prefix(&ds, prefix).unwrap();
        assert_eq!(configuration_files, None);

        // Unrelated prefix returns None
        let prefix = "settings";
        let configuration_files = get_configuration_files_prefix(&ds, prefix).unwrap();
        assert_eq!(configuration_files, None);
    }

    #[test]
    fn set_settings_works() {
        let settings = serde_json::from_str::<model::Settings>("{\"motd\": \"tz\"}").unwrap();

        // Set with helper
        let mut ds = MemoryDataStore::new();
        let tx = "test transaction";
        let pending = Committed::Pending { tx: tx.into() };
        set_settings(&mut ds, &settings, tx, Strength::Strong).unwrap();

        // Retrieve directly
        let key = Key::new(KeyType::Data, "settings.motd").unwrap();
        assert_eq!(
            Some("\"tz\"".to_string()),
            ds.get_key(&key, &pending).unwrap()
        );
    }

    #[test]
    fn get_metadata_keys_works() {
        let mut ds = MemoryDataStore::new();
        // Set directly with data store
        for data_key in &["abc", "def", "ghi"] {
            ds.set_metadata(
                &Key::new(KeyType::Meta, "my-meta").unwrap(),
                &Key::new(KeyType::Data, data_key).unwrap(),
                "\"json string\"",
                &Committed::Live,
            )
            .unwrap();
        }

        // We'll check a subset by specifying 2 of the 3 keys
        let expected = hashmap!(
            "abc".to_string() => "json string".into(),
            "def".to_string() => "json string".into(),
        );
        // Retrieve with helper
        let actual = get_metadata_for_data_keys(&ds, "my-meta", &hashset!("abc", "def")).unwrap();

        assert_eq!(expected, actual);
    }

    #[test]
    fn get_metadata_all_works() {
        let mut ds = MemoryDataStore::new();
        // Set directly with data store
        for data_key in &["abc", "def"] {
            ds.set_metadata(
                &Key::new(KeyType::Meta, "my-meta").unwrap(),
                &Key::new(KeyType::Data, data_key).unwrap(),
                "\"json string\"",
                &Committed::Live,
            )
            .unwrap();
        }

        let expected = hashmap!(
            Key::new(KeyType::Data, "abc").unwrap() =>  hashmap!(Key::new(KeyType::Meta, "my-meta").unwrap() => "\"json string\"".into()),
            Key::new(KeyType::Data, "def").unwrap() =>  hashmap!(Key::new(KeyType::Meta, "my-meta").unwrap() => "\"json string\"".into()),
        );
        // Retrieve with helper
        let actual = get_metadata_for_all_data_keys(&ds, "my-meta").unwrap();

        assert_eq!(expected, actual);
    }

    #[test]
    fn commit_works() {
        // Set directly with data store
        let mut ds = MemoryDataStore::new();
        let tx = "test transaction";
        let pending = Committed::Pending { tx: tx.into() };
        ds.set_key(
            &Key::new(KeyType::Data, "settings.motd").unwrap(),
            "\"json string\"",
            &pending,
        )
        .unwrap();

        // Confirm pending
        let settings = get_settings(&ds, &pending).unwrap();
        assert_eq!(extract!(settings.motd), Some("json string".into()));
        // No live settings yet
        get_settings(&ds, &Committed::Live).unwrap_err();

        // Commit, pending -> live
        commit_transaction::<datastore::memory::MemoryDataStore>(&mut ds, tx).unwrap();

        // No more pending settings
        get_settings(&ds, &pending).unwrap_err();
        // Confirm live
        let settings = get_settings(&ds, &Committed::Live).unwrap();
        assert_eq!(extract!(settings.motd), Some("json string".into()));
    }

    #[test]
    fn sorting_metadata_works() {
        let metadata: HashMap<Key, HashMap<Key, String>> = hashmap!(
            Key::new(KeyType::Data, "settings.a.b").unwrap() =>  hashmap!(Key::new(KeyType::Meta, "my-meta").unwrap() => "\"json string\"".into()),
            Key::new(KeyType::Data, "settings.a.b.c").unwrap() =>  hashmap!(Key::new(KeyType::Meta, "my-meta").unwrap() => "\"json string\"".into()),
            Key::new(KeyType::Data, "settings.a").unwrap() =>  hashmap!(Key::new(KeyType::Meta, "my-meta").unwrap() => "\"json string\"".into()),
        );

        let expected = vec![
            (
                Key::new(KeyType::Data, "settings.a").unwrap(),
                hashmap!(Key::new(KeyType::Meta, "my-meta").unwrap() => "\"json string\"".into()),
            ),
            (
                Key::new(KeyType::Data, "settings.a.b").unwrap(),
                hashmap!(Key::new(KeyType::Meta, "my-meta").unwrap() => "\"json string\"".into()),
            ),
            (
                Key::new(KeyType::Data, "settings.a.b.c").unwrap(),
                hashmap!(Key::new(KeyType::Meta, "my-meta").unwrap() => "\"json string\"".into()),
            ),
        ];

        let actual = sort_metadata(metadata);
        assert_eq!(expected, actual);
    }

    #[test]
    fn can_get_metadata_for_successor() {
        let mut ds = MemoryDataStore::new();
        let data_key = Key::new(KeyType::Data, "settings.abc.def").unwrap();
        ds.set_metadata(
            &Key::new(KeyType::Meta, "my-meta").unwrap(),
            &data_key,
            "\"json string\"",
            &Committed::Live,
        )
        .unwrap();
        ds.set_key(
            &Key::new(KeyType::Data, "settings.abc.xyz.mode").unwrap(),
            "\"once\"",
            &Committed::Live,
        )
        .unwrap();
        let mut result = HashMap::new();
        let raw_setting_generator = RawSettingsGenerator {
            command: "\"json string\"".to_owned(),
            depth: 1,
            strength: Strength::Weak,
        };

        let setting_generator = SettingsGenerator::from(raw_setting_generator.clone());

        expand_setting_generator(&ds, &data_key, raw_setting_generator, &mut result).unwrap();

        let expected = hashmap!("settings.abc.xyz.def".to_owned() => serde_json::to_value(setting_generator).unwrap());
        assert_eq!(result, expected);
    }

    #[test]
    fn expand_setting_generator_with_depth_zero() {
        let mut ds = MemoryDataStore::new();
        let data_key = Key::new(KeyType::Data, "settings.abc.def").unwrap();
        ds.set_metadata(
            &Key::new(KeyType::Meta, "my-meta").unwrap(),
            &data_key,
            "\"json string\"",
            &Committed::Live,
        )
        .unwrap();
        ds.set_key(
            &Key::new(KeyType::Data, "settings.abc.xyz.mode").unwrap(),
            "\"once\"",
            &Committed::Live,
        )
        .unwrap();
        let mut result = HashMap::new();
        let raw_setting_generator = RawSettingsGenerator {
            command: "\"json string\"".to_owned(),
            depth: 0,
            strength: Strength::Weak,
        };

        let setting_generator = SettingsGenerator::from(raw_setting_generator.clone());

        expand_setting_generator(&ds, &data_key, raw_setting_generator, &mut result).unwrap();

        let expected = hashmap!("settings.abc.def".to_owned() => serde_json::to_value(setting_generator).unwrap());
        assert_eq!(result, expected);
    }

    #[test]
    fn test_child_key_len() {
        let mut ds = MemoryDataStore::new();
        let data_key = Key::new(KeyType::Data, "settings.abc.def").unwrap();
        ds.set_metadata(
            &Key::new(KeyType::Meta, "my-meta").unwrap(),
            &data_key,
            "\"json string\"",
            &Committed::Live,
        )
        .unwrap();

        ds.set_key(
            &Key::new(KeyType::Data, "settings.abc.def").unwrap(),
            "\"once\"",
            &Committed::Live,
        )
        .unwrap();

        let mut result = HashMap::new();
        let raw_setting_generator = RawSettingsGenerator {
            command: "\"json string\"".to_owned(),
            depth: 1,
            strength: Strength::Weak,
        };

        expand_setting_generator(&ds, &data_key, raw_setting_generator, &mut result).unwrap();

        for key_name in result.keys() {
            let key = Key::new(KeyType::Data, key_name).unwrap();
            assert_eq!(key.segments().len(), 4);
        }
    }

    #[test]
    fn test_dynamic_setting_is_only_applied_on_descendants() {
        let mut ds = MemoryDataStore::new();
        let data_key = Key::new(KeyType::Data, "settings.abc.def").unwrap();

        ds.set_key(
            &Key::new(KeyType::Data, "settings.abc.some-value").unwrap(),
            "\"parent\"",
            &Committed::Live,
        )
        .unwrap();

        ds.set_key(
            &Key::new(KeyType::Data, "settings.abc.some-map.xyz").unwrap(),
            "\"child\"",
            &Committed::Live,
        )
        .unwrap();

        let mut result = HashMap::new();
        let raw_setting_generator = RawSettingsGenerator {
            command: "\"json string\"".to_owned(),
            depth: 1,
            strength: Strength::Weak,
        };

        let setting_generator = SettingsGenerator::from(raw_setting_generator.clone());
        let expected = hashmap!("settings.abc.some-map.def".to_owned() => serde_json::to_value(setting_generator).unwrap());

        expand_setting_generator(&ds, &data_key, raw_setting_generator.clone(), &mut result)
            .unwrap();

        assert_eq!(expected, result);
        assert!(!result.contains_key("settings.abc.def"))
    }
}
