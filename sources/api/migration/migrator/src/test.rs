//! Provides an end-to-end test of `migrator` via the `run` function. This module is conditionally
//! compiled for cfg(test) only.
use crate::args::Args;
use crate::{
    cleanup_orphaned_datastores, copy_without_transient_entries, find_orphaned_entries,
    flip_to_new_version, perform_migrations, MigrationVersionMeta,
};
use datastore::memory::MemoryDataStore;
use datastore::{serialize_scalar, Committed, DataStore, Key};
use jiff::Timestamp;
use semver::Version;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::fs;

/// Provides the path to a folder where test data files reside.
fn test_data() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.join("migrator").join("tests").join("data")
}

/// Returns the filepath to a `root.json` file stored in tree for testing. This file declares
/// an expiration date of `1970-01-01` to ensure success with an expired TUF repository.
fn root() -> PathBuf {
    test_data()
        .join("expired-root.json")
        .canonicalize()
        .unwrap()
}

enum TestType {
    /// The test will raise an error in the last migration when running forward.
    ForwardFailure,
    /// The test will raise an error in the last migration when running backward.
    BackwardFailure,
    /// The test is not expected to raise an error in migrator.
    Success,
}

impl TestType {
    fn migration_names(&self) -> Vec<String> {
        match self {
            TestType::ForwardFailure => [FIRST_MIGRATION, SECOND_MIGRATION, FAILING_MIGRATION],
            TestType::BackwardFailure => [FAILING_MIGRATION, SECOND_MIGRATION, THIRD_MIGRATION],
            TestType::Success => [FIRST_MIGRATION, SECOND_MIGRATION, THIRD_MIGRATION],
        }
        .iter()
        .map(|s| s.to_string())
        .collect()
    }
}

/// Returns the filepath to a private key, stored in tree and used only for testing.
fn pem() -> PathBuf {
    test_data().join("snakeoil.pem").canonicalize().unwrap()
}

/// The name of a test migration. The prefix `b-` ensures we are not alphabetically sorting.
const FIRST_MIGRATION: &str = "b-first-migration";

/// The name of a test migration. The prefix `a-` ensures we are not alphabetically sorting.
const SECOND_MIGRATION: &str = "a-second-migration";

/// The name of another test migration.
const THIRD_MIGRATION: &str = "third-migration";

/// A migration that will fail and exit with a non-zero code.
const FAILING_MIGRATION: &str = "failing-migration";

/// Creates a script that will serve as a migration during testing. The script writes its migrations
/// name to a file named `result.txt` in the parent directory of the datastore. `pentacle` does not
/// retain the name of the executing binary or script, so we take the `migration_name` as input,
/// and 'hardcode' it into the script.
fn create_test_migration<S: AsRef<str>>(migration_name: S) -> String {
    format!(
        r#"#!/usr/bin/env bash
set -eo pipefail
migration_name="{}"
datastore_parent_dir="$(dirname "${{3}}")"
target_datastore="$5"
outfile="${{datastore_parent_dir}}/result.txt"
echo "${{migration_name}}:" "${{@}}" >> "${{outfile}}"
mkdir -p $5
if [[ "${{migration_name}}" = "failing-migration" ]]; then
  >&2 echo "this migration is supposed to fail: exit 1"
  exit 1
fi
"#,
        migration_name.as_ref()
    )
}

/// Holds the lifetime of a `TempDir` inside which a datastore directory and links are held for
/// testing.
struct TestDatastore {
    tmp: TempDir,
    datastore: PathBuf,
}

impl TestDatastore {
    /// Creates a `TempDir`, sets up the datastore links needed to represent the `from_version`
    /// and returns a `TestDatastore` populated with this information.
    fn new(from_version: Version) -> Self {
        let tmp = TempDir::new().unwrap();
        let datastore = storewolf::create_new_datastore(tmp.path(), Some(from_version)).unwrap();
        TestDatastore { tmp, datastore }
    }

    /// Adds orphaned directories/symlinks that should be cleaned up.
    fn add_orphaned_entries(&self) -> Vec<PathBuf> {
        let base = self.tmp.path();
        let mut orphaned = Vec::new();

        // Create an orphaned directory (not linked to current)
        let orphan_dir = base.join("v0.98.0_orphaned");
        std::fs::create_dir_all(&orphan_dir).unwrap();
        orphaned.push(orphan_dir.clone());

        // Create a symlink pointing to the orphaned directory
        let orphan_link = base.join("v0.98");
        std::os::unix::fs::symlink("v0.98.0_orphaned", &orphan_link).unwrap();
        orphaned.push(orphan_link);

        orphaned
    }
}

/// Represents a TUF repository, which is held in a tempdir.
struct TestRepo {
    /// This field preserves the lifetime of the TempDir even though we never read it. When
    /// `TestRepo` goes out of scope, `TempDir` will remove the temporary directory.
    _tuf_dir: TempDir,
    metadata_path: PathBuf,
    targets_path: PathBuf,
}

/// LZ4 compresses `source` bytes to a new file at `destination`.
fn compress(source: &[u8], destination: &Path) {
    // It is easier to use blocking IO here and in test code is fine as long as it works.
    let output_file = std::fs::File::create(destination).unwrap();
    let mut encoder = lz4::EncoderBuilder::new()
        .level(4)
        .build(output_file)
        .unwrap();
    encoder.write_all(source).unwrap();
    let (_output, result) = encoder.finish();
    result.unwrap()
}

/// Creates a test repository with a couple of versions defined in the manifest and a couple of
/// migrations. See the test description for for more info.
async fn create_test_repo(test_type: TestType) -> TestRepo {
    // This is where the signed TUF repo will exist when we are done. It is the
    // root directory of the `TestRepo` we will return when we are done.
    let test_repo_dir = TempDir::new().unwrap();
    let metadata_path = test_repo_dir.path().join("metadata");
    let targets_path = test_repo_dir.path().join("targets");

    // This is where we will stage the TUF repository targets prior to signing them. We are using
    // symlinks from `tuf_indir` to `tuf_outdir/targets` so we keep both in the same `TempDir`.
    let tuf_indir = test_repo_dir.path();

    // Create a Manifest and save it to the tuftool_indir for signing.
    let mut manifest = update_metadata::Manifest::default();
    // insert the following migrations to the manifest. note that the first migration would sort
    // later than the second migration alphabetically. this is to help ensure that migrations
    // are running in their listed order (rather than sorted order as in previous
    // implementations).
    let migration_names = test_type.migration_names();
    manifest.migrations.insert(
        (Version::new(0, 99, 0), Version::new(0, 99, 1)),
        migration_names.clone(),
    );
    manifest.migrations.insert(
        (Version::new(0, 99, 1), Version::new(0, 99, 2)),
        migration_names.clone(),
    );
    update_metadata::write_file(tuf_indir.join("manifest.json").as_path(), &manifest).unwrap();

    // Create an script that we can use as the 'migration' that migrator will run. This script will
    // write its name and arguments to a file named result.txt in the directory that is the parent
    // of --source-datastore. result.txt can then be used to see what migrations ran, and in what
    // order. Note that tests are sensitive to the order and number of arguments passed. If
    // --source-datastore is given at a different position then the tests will fail and the script
    // will need to be updated.
    for migration_name in &migration_names {
        // Create a script to use as a migration.
        let data = create_test_migration(migration_name);
        // Save an lz4 compressed copy of the migration script into the tuftool_indir.
        compress(data.as_bytes(), &tuf_indir.join(migration_name))
    }

    // Create and sign the TUF repository.
    let mut editor = tough::editor::RepositoryEditor::new(root()).await.unwrap();
    let long_ago = Timestamp::UNIX_EPOCH;
    let one = std::num::NonZeroU64::new(1).unwrap();
    editor
        .targets_version(one)
        .unwrap()
        .targets_expires(long_ago)
        .unwrap()
        .snapshot_version(one)
        .snapshot_expires(long_ago)
        .timestamp_version(one)
        .timestamp_expires(long_ago);

    for path in list_dir_files(tuf_indir).await {
        editor
            .add_target(
                path.file_name().unwrap().to_str().unwrap(),
                tough::schema::Target::from_path(&path).await.unwrap(),
            )
            .unwrap();
    }
    let signed_repo = editor
        .sign(&[Box::new(tough::key_source::LocalKeySource { path: pem() })])
        .await
        .unwrap();
    signed_repo
        .link_targets(
            tuf_indir,
            &targets_path,
            tough::editor::signed::PathExists::Fail,
        )
        .await
        .unwrap();
    signed_repo.write(&metadata_path).await.unwrap();

    TestRepo {
        _tuf_dir: test_repo_dir,
        metadata_path,
        targets_path,
    }
}

/// Asserts that the expected directories and files are in the datastore directory after a
/// failed migration. Returns the absolute path that the `current` symlink is pointing to.
async fn assert_directory_structure_with_failed_migration(
    dir: &Path,
    from: &Version,
    to: &Version,
) -> PathBuf {
    let paths = list_dir_entries(dir).await;
    let from_ver = format!("v{from}");
    let from_ver_unique_prefix = format!("v{from}_");
    let to_ver_unique_prefix = format!("v{to}_");

    assert_eq!(paths.len(), 8);
    assert_dir_entry_exists(&paths, "current");
    assert_dir_entry_exists(&paths, "result.txt");
    assert_dir_entry_exists(&paths, "v0");
    assert_dir_entry_exists(&paths, "v0.99");
    assert_dir_entry_exists(&paths, &from_ver);
    assert_dir_starting_with_exists(&paths, &from_ver_unique_prefix);

    // There are two datastores that start with the target version followed by an underscore. This
    // is because the datastore we intended to promote (target_datastore) and one intermediate
    // datastore are expected to be left behind for debugging after a migration failure.
    let left_behind_count = paths
        .iter()
        .filter_map(|entry| {
            entry
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with(&to_ver_unique_prefix)
                .then_some(())
        })
        .collect::<Vec<()>>()
        .len();

    assert_eq!(
        left_behind_count, 2,
        "expected 2 directories to be left behind after migration failure, but found {left_behind_count}"
    );

    let symlink = paths
        .iter()
        .find(|entry| entry.file_name().unwrap().to_str().unwrap() == "current")
        .unwrap();
    symlink.canonicalize().unwrap()
}

/// Asserts that the expected directories and files are in the datastore directory after a
/// successful migration. Returns the absolute path that the `current` symlink is pointing to.
async fn assert_directory_structure(dir: &Path, from: Version, to: Version) -> PathBuf {
    let paths = list_dir_entries(dir).await;
    assert_eq!(paths.len(), 6);
    assert_dir_entry_exists(&paths, "current");
    assert_dir_entry_exists(&paths, "result.txt");
    assert_dir_entry_exists(&paths, format!("v{}", from.major).as_str());
    assert_dir_entry_exists(&paths, format!("v{}.{}", from.major, from.minor).as_str());
    assert_dir_entry_exists(&paths, format!("v{}", to).as_str());
    assert_dir_starting_with_exists(&paths, format!("v{}_", to).as_str());

    let symlink = paths
        .iter()
        .find(|entry| entry.file_name().unwrap().to_str().unwrap() == "current")
        .unwrap();
    symlink.canonicalize().unwrap()
}

fn assert_dir_entry_exists(dir_entries: &[PathBuf], exact_name: &str) {
    assert!(
        dir_entries
            .iter()
            .any(|entry| entry.file_name().unwrap().to_str().unwrap() == exact_name),
        "'{exact_name}' not found"
    );
}

fn assert_dir_starting_with_exists(dir_entries: &[PathBuf], starts_with: &str) {
    assert!(
        dir_entries.iter().any(|entry| entry
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with(starts_with)),
        "entry starting with '{starts_with}' not found"
    );
}

async fn list_dir_entries(dir: impl AsRef<Path>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut read_dir = fs::read_dir(dir).await.unwrap();
    while let Some(entry) = read_dir.next_entry().await.unwrap() {
        paths.push(entry.path())
    }
    paths
}

async fn list_dir_files(dir: impl AsRef<Path>) -> Vec<PathBuf> {
    list_dir_entries(dir)
        .await
        .into_iter()
        .filter(|path| path.is_file())
        .collect()
}

/// Tests the migrator program end-to-end using the `run` function. Creates a TUF repo in a
/// tempdir which includes a  `manifest.json` with a couple of migrations:
/// ```
///     "(0.99.0, 0.99.1)": [
///       "b-first-migration",
///       "a-second-migration"
///     ]
/// ```
///
/// The two 'migrations' are instances of the same bash script (see `create_test_repo`) which
/// writes its name (i.e. the migration name) and its arguments to a file at `./result.txt`
/// (i.e. since migrations run in the context of the datastore directory, `result.txt` is
/// written one directory above the datastore.) We can then inspect the contents of `result.txt`
/// to see that the expected migrations ran in the correct order.
#[tokio::test]
async fn migrate_forward() {
    let from_version = Version::parse("0.99.0").unwrap();
    let to_version = Version::parse("0.99.1").unwrap();
    let version_meta = MigrationVersionMeta::new(from_version.clone(), to_version.clone()).unwrap();
    let test_datastore = TestDatastore::new(from_version.clone());
    let test_repo = create_test_repo(TestType::Success).await;
    let args = Args {
        datastore_path: test_datastore.datastore.clone(),
        log_level: log::LevelFilter::Info,
        migration_directory: test_repo.targets_path.clone(),
        migrate_to_version: to_version.clone(),
        root_path: root(),
        metadata_directory: test_repo.metadata_path.clone(),
    };
    let datastore = perform_migrations(&version_meta, &args).await.unwrap();
    // the migrations should write to a file named result.txt.
    let output_file = test_datastore.tmp.path().join("result.txt");
    let contents = std::fs::read_to_string(&output_file).unwrap();
    let lines: Vec<&str> = contents.split('\n').collect();
    assert_eq!(lines.len(), 4);
    let first_line = *lines.first().unwrap();
    let want = format!("{FIRST_MIGRATION}: --forward");
    let got: String = first_line.chars().take(want.len()).collect();
    assert_eq!(got, want);
    let second_line = *lines.get(1).unwrap();
    let want = format!("{SECOND_MIGRATION}: --forward");
    let got: String = second_line.chars().take(want.len()).collect();
    assert_eq!(got, want);
    let third_line = *lines.get(2).unwrap();
    let want = format!("{THIRD_MIGRATION}: --forward");
    let got: String = third_line.chars().take(want.len()).collect();
    assert_eq!(got, want);

    let datastore = flip_to_new_version(&args.migrate_to_version, datastore)
        .await
        .unwrap();
    cleanup_orphaned_datastores(&datastore).await;

    // Check the directory.
    let current =
        assert_directory_structure(test_datastore.tmp.path(), from_version, to_version).await;

    // We have successfully migrated so current should be pointing to a directory that starts with
    // v0.99.1.
    assert!(current
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("v0.99.1"));
}

/// This test ensures that migrations run when migrating from a newer to an older version.
/// See `migrate_forward` for a description of how these tests work.
#[tokio::test]
async fn migrate_backward() {
    let from_version = Version::parse("0.99.1").unwrap();
    let to_version = Version::parse("0.99.0").unwrap();
    let version_meta = MigrationVersionMeta::new(from_version.clone(), to_version.clone()).unwrap();
    let test_datastore = TestDatastore::new(from_version.clone());
    let test_repo = create_test_repo(TestType::Success).await;
    let args = Args {
        datastore_path: test_datastore.datastore.clone(),
        log_level: log::LevelFilter::Info,
        migration_directory: test_repo.targets_path.clone(),
        migrate_to_version: to_version.clone(),
        root_path: root(),
        metadata_directory: test_repo.metadata_path.clone(),
    };
    let datastore = perform_migrations(&version_meta, &args).await.unwrap();
    let output_file = test_datastore.tmp.path().join("result.txt");
    let contents = std::fs::read_to_string(&output_file).unwrap();
    let lines: Vec<&str> = contents.split('\n').collect();
    assert_eq!(lines.len(), 4);
    let first_line = *lines.first().unwrap();
    let want = format!("{THIRD_MIGRATION}: --backward");
    let got: String = first_line.chars().take(want.len()).collect();
    assert_eq!(got, want);
    let second_line = *lines.get(1).unwrap();
    let want = format!("{SECOND_MIGRATION}: --backward");
    let got: String = second_line.chars().take(want.len()).collect();
    assert_eq!(got, want);
    let second_line = *lines.get(2).unwrap();
    let want = format!("{FIRST_MIGRATION}: --backward");
    let got: String = second_line.chars().take(want.len()).collect();
    assert_eq!(got, want);

    let datastore = flip_to_new_version(&args.migrate_to_version, datastore)
        .await
        .unwrap();
    cleanup_orphaned_datastores(&datastore).await;
    // Check the directory.
    let current =
        assert_directory_structure(test_datastore.tmp.path(), from_version, to_version).await;

    // We have successfully migrated so current should be pointing to a directory that starts with
    // v0.99.0.
    assert!(current
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("v0.99.0"));
}

#[tokio::test]
async fn migrate_forward_with_failed_migration() {
    let from_version = Version::parse("0.99.0").unwrap();
    let to_version = Version::parse("0.99.1").unwrap();
    let version_meta = MigrationVersionMeta::new(from_version.clone(), to_version.clone()).unwrap();
    let test_datastore = TestDatastore::new(from_version.clone());
    let test_repo = create_test_repo(TestType::ForwardFailure).await;
    let args = Args {
        datastore_path: test_datastore.datastore.clone(),
        log_level: log::LevelFilter::Info,
        migration_directory: test_repo.targets_path.clone(),
        migrate_to_version: to_version.clone(),
        root_path: root(),
        metadata_directory: test_repo.metadata_path.clone(),
    };
    let result = perform_migrations(&version_meta, &args).await;
    assert!(result.is_err());

    // the migrations should write to a file named result.txt.
    let output_file = test_datastore.tmp.path().join("result.txt");
    let contents = std::fs::read_to_string(&output_file).unwrap();
    let lines: Vec<&str> = contents.split('\n').collect();
    assert_eq!(lines.len(), 4);
    let first_line = *lines.first().unwrap();
    let want = format!("{FIRST_MIGRATION}: --forward");
    let got: String = first_line.chars().take(want.len()).collect();
    assert_eq!(got, want);
    let second_line = *lines.get(1).unwrap();
    let want = format!("{SECOND_MIGRATION}: --forward");
    let got: String = second_line.chars().take(want.len()).collect();
    assert_eq!(got, want);
    let third_line = *lines.get(2).unwrap();
    let want = format!("{FAILING_MIGRATION}: --forward");
    let got: String = third_line.chars().take(want.len()).collect();
    assert_eq!(got, want);

    // Check the directory.
    let current = assert_directory_structure_with_failed_migration(
        test_datastore.tmp.path(),
        &from_version,
        &to_version,
    )
    .await;

    // We have not successfully migrated to v0.99.1 so we should still be pointing at the "from"
    // version.
    assert!(current
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("v0.99.0"));
}

#[tokio::test]
async fn migrate_backward_with_failed_migration() {
    let from_version = Version::parse("0.99.1").unwrap();
    let to_version = Version::parse("0.99.0").unwrap();
    let version_meta = MigrationVersionMeta::new(from_version.clone(), to_version.clone()).unwrap();
    let test_datastore = TestDatastore::new(from_version.clone());
    let test_repo = create_test_repo(TestType::BackwardFailure).await;
    let args = Args {
        datastore_path: test_datastore.datastore.clone(),
        log_level: log::LevelFilter::Info,
        migration_directory: test_repo.targets_path.clone(),
        migrate_to_version: to_version.clone(),
        root_path: root(),
        metadata_directory: test_repo.metadata_path.clone(),
    };
    let result = perform_migrations(&version_meta, &args).await;
    assert!(result.is_err());

    let output_file = test_datastore.tmp.path().join("result.txt");
    let contents = std::fs::read_to_string(&output_file).unwrap();
    let lines: Vec<&str> = contents.split('\n').collect();
    assert_eq!(lines.len(), 4);
    let first_line = *lines.first().unwrap();
    let want = format!("{THIRD_MIGRATION}: --backward");
    let got: String = first_line.chars().take(want.len()).collect();
    assert_eq!(got, want);
    let second_line = *lines.get(1).unwrap();
    let want = format!("{SECOND_MIGRATION}: --backward");
    let got: String = second_line.chars().take(want.len()).collect();
    assert_eq!(got, want);
    let second_line = *lines.get(2).unwrap();
    let want = format!("{FAILING_MIGRATION}: --backward");
    let got: String = second_line.chars().take(want.len()).collect();
    assert_eq!(got, want);

    // Check the directory.
    let current = assert_directory_structure_with_failed_migration(
        test_datastore.tmp.path(),
        &from_version,
        &to_version,
    )
    .await;

    // We have not successfully migrated to v0.99.0 so we should still be pointing at the "from"
    // version.
    assert!(current
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("v0.99.1"));
}

#[tokio::test]
async fn test_remove_all_metadata() {
    let mut source = MemoryDataStore::new();
    let data_key = Key::new(datastore::KeyType::Data, "a.b.c").unwrap();
    let metadata_key = Key::new(datastore::KeyType::Meta, "c").unwrap();
    let value = serialize_scalar::<std::string::String, serde_json::Error>(
        &"Test metadata value".to_string(),
    )
    .unwrap();

    source
        .set_metadata(&metadata_key, &data_key, value, &Committed::Live)
        .unwrap();

    // Ensure that metadata exists in the source datastore
    let metadata = source
        .get_metadata(&metadata_key, &data_key, &Committed::Live)
        .unwrap();
    assert!(metadata.is_some());
    assert_eq!(metadata.unwrap(), "\"Test metadata value\"");

    let mut target = MemoryDataStore::new();

    let result = copy_without_transient_entries(source, &mut target);
    assert!(result.is_ok());

    // Ensure that metadata does not exists in the target datastore
    let metadata = target
        .get_metadata(&metadata_key, &data_key, &Committed::Live)
        .unwrap();
    assert!(metadata.is_none());
}

#[tokio::test]
async fn test_only_weak_settings_are_removed() {
    let mut source = MemoryDataStore::new();
    let weak_data_key = Key::new(datastore::KeyType::Data, "a.b.c").unwrap();
    let weak_metadata_key = Key::new(datastore::KeyType::Meta, "strength").unwrap();
    let weak_metadata_value =
        serialize_scalar::<std::string::String, serde_json::Error>(&"weak".to_string()).unwrap();
    let weak_data_value = serialize_scalar::<std::string::String, serde_json::Error>(
        &"strong data value".to_string(),
    )
    .unwrap();

    source
        .set_key(&weak_data_key, weak_data_value, &Committed::Live)
        .unwrap();
    source
        .set_metadata(
            &weak_metadata_key,
            &weak_data_key,
            weak_metadata_value,
            &Committed::Live,
        )
        .unwrap();

    let strong_data_key = Key::new(datastore::KeyType::Data, "e.f").unwrap();
    let strong_metadata_key = Key::new(datastore::KeyType::Meta, "strength").unwrap();
    let strong_metadata_value =
        serialize_scalar::<std::string::String, serde_json::Error>(&"strong".to_string()).unwrap();
    let strong_data_value = serialize_scalar::<std::string::String, serde_json::Error>(
        &"strong data value".to_string(),
    )
    .unwrap();

    source
        .set_key(
            &strong_data_key,
            strong_data_value.clone(),
            &Committed::Live,
        )
        .unwrap();
    source
        .set_metadata(
            &strong_metadata_key,
            &strong_data_key,
            strong_metadata_value,
            &Committed::Live,
        )
        .unwrap();

    let mut target = MemoryDataStore::new();

    let result = copy_without_transient_entries(source, &mut target);
    assert!(result.is_ok());

    // Ensure that metadata does not exists in the target datastore
    let weak_data = target.get_key(&weak_data_key, &Committed::Live).unwrap();
    assert!(weak_data.is_none());

    // Ensure that metadata does not exists in the target datastore
    let strong_data = target.get_key(&strong_data_key, &Committed::Live).unwrap();
    assert!(strong_data.is_some());
    assert_eq!(strong_data.unwrap(), strong_data_value);
}

#[tokio::test]
async fn test_configuration_files_data_removed() {
    let mut source = MemoryDataStore::new();
    source.set_keys(&maplit::hashmap! {
        Key::new(datastore::KeyType::Data, "settings.a.b").unwrap() => "\"hello\"",
        Key::new(datastore::KeyType::Data, "settings.a.c").unwrap() => "\"world\"",
        Key::new(datastore::KeyType::Data, "something-else.d.e").unwrap() => "\"bottlerocket\"",
        Key::new(datastore::KeyType::Data, "yet-another-thing.f.g").unwrap() => "\"rules\"",
        Key::new(datastore::KeyType::Data, "configuration-files.aws-config.path").unwrap() => "\"/root/.aws/config\"",
        Key::new(datastore::KeyType::Data, "configuration-files.aws-config.template-path").unwrap() => "\"/usr/share/templates/aws-config\"",
    }, &Committed::Live).unwrap();

    let mut target = MemoryDataStore::new();
    copy_without_transient_entries(source, &mut target).unwrap();

    assert_eq!(
        target.get_prefix("", &Committed::Live).unwrap(),
        maplit::hashmap! {
            Key::new(datastore::KeyType::Data, "settings.a.b").unwrap() => "\"hello\"".to_string(),
            Key::new(datastore::KeyType::Data, "settings.a.c").unwrap() => "\"world\"".to_string(),
            Key::new(datastore::KeyType::Data, "something-else.d.e").unwrap() => "\"bottlerocket\"".to_string(),
            Key::new(datastore::KeyType::Data, "yet-another-thing.f.g").unwrap() => "\"rules\"".to_string(),
        }
    );
}

#[tokio::test]
async fn test_services_removed() {
    let mut source = MemoryDataStore::new();
    source.set_keys(&maplit::hashmap! {
        Key::new(datastore::KeyType::Data, "settings.a.b").unwrap() => "\"hello\"",
        Key::new(datastore::KeyType::Data, "settings.a.c").unwrap() => "\"world\"",
        Key::new(datastore::KeyType::Data, "something-else.d.e").unwrap() => "\"bottlerocket\"",
        Key::new(datastore::KeyType::Data, "yet-another-thing.f.g").unwrap() => "\"rules\"",
        Key::new(datastore::KeyType::Data, "services.dns.configuration-files").unwrap() => "[\"netdog-toml\"]",
        Key::new(datastore::KeyType::Data, "services.aws.restart-commands").unwrap() => "[]",
    }, &Committed::Live).unwrap();

    let mut target = MemoryDataStore::new();
    copy_without_transient_entries(source, &mut target).unwrap();

    assert_eq!(
        target.get_prefix("", &Committed::Live).unwrap(),
        maplit::hashmap! {
            Key::new(datastore::KeyType::Data, "settings.a.b").unwrap() => "\"hello\"".to_string(),
            Key::new(datastore::KeyType::Data, "settings.a.c").unwrap() => "\"world\"".to_string(),
            Key::new(datastore::KeyType::Data, "something-else.d.e").unwrap() => "\"bottlerocket\"".to_string(),
            Key::new(datastore::KeyType::Data, "yet-another-thing.f.g").unwrap() => "\"rules\"".to_string(),
        }
    );
}

async fn do_migration(
    from_version: &str,
    to_version: &str,
    test_repo: &TestRepo,
    datastore: &TestDatastore,
) {
    let from = Version::parse(from_version).unwrap();
    let to = Version::parse(to_version).unwrap();

    let version_meta = MigrationVersionMeta::new(from.clone(), to.clone()).unwrap();
    let args = Args {
        datastore_path: datastore.datastore.clone(),
        log_level: log::LevelFilter::Info,
        migration_directory: test_repo.targets_path.clone(),
        migrate_to_version: to.clone(),
        root_path: root(),
        metadata_directory: test_repo.metadata_path.clone(),
    };
    let outstore = perform_migrations(&version_meta, &args).await.unwrap();
    let outstore = flip_to_new_version(&to, &outstore).await.unwrap();
    cleanup_orphaned_datastores(&outstore).await;
}

#[tokio::test]
async fn test_datastore_cleanup_across_migrations_forward() {
    let test_repo = create_test_repo(TestType::Success).await;
    let mut test_datastore = TestDatastore::new(Version::parse("0.99.0").unwrap());
    // First migration: from source -> intermediate
    do_migration("0.99.0", "0.99.1", &test_repo, &test_datastore).await;

    let current = assert_directory_structure(
        test_datastore.tmp.path(),
        Version::parse("0.99.0").unwrap(),
        Version::parse("0.99.1").unwrap(),
    )
    .await;
    assert!(current
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("v0.99.1"));

    // We need to "flip" the datastore to point to the new current datastore
    test_datastore.datastore = current;

    // Second migration: intermediate -> target
    do_migration("0.99.1", "0.99.2", &test_repo, &test_datastore).await;

    let current = assert_directory_structure(
        test_datastore.tmp.path(),
        Version::parse("0.99.1").unwrap(),
        Version::parse("0.99.2").unwrap(),
    )
    .await;
    assert!(current
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("v0.99.2"));
}

#[tokio::test]
async fn test_datastore_cleanup_across_migrations_backward() {
    let test_repo = create_test_repo(TestType::Success).await;
    let mut test_datastore = TestDatastore::new(Version::parse("0.99.2").unwrap());
    // First migration: from source -> intermediate
    do_migration("0.99.2", "0.99.1", &test_repo, &test_datastore).await;

    let current = assert_directory_structure(
        test_datastore.tmp.path(),
        Version::parse("0.99.2").unwrap(),
        Version::parse("0.99.1").unwrap(),
    )
    .await;
    assert!(current
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("v0.99.1"));

    // We need to "flip" the datastore to point to the new current datastore
    test_datastore.datastore = current;

    // Second migration: intermediate -> target
    do_migration("0.99.1", "0.99.0", &test_repo, &test_datastore).await;

    let current = assert_directory_structure(
        test_datastore.tmp.path(),
        Version::parse("0.99.1").unwrap(),
        Version::parse("0.99.0").unwrap(),
    )
    .await;
    assert!(current
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("v0.99.0"));
}

#[tokio::test]
async fn test_symlink_graph() {
    let test = TestDatastore::new(Version::parse("0.99.2").unwrap());
    let expected_orphans = test.add_orphaned_entries();
    let orphaned = find_orphaned_entries(test.tmp.path(), test.datastore.clone()).await;

    // All entries we added should be detected as orphaned
    for expected in &expected_orphans {
        assert!(
            orphaned.contains(expected),
            "Expected {:?} to be in orphaned set",
            expected
        );
    }
}

#[tokio::test]
async fn test_symlink_loop_handled() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path();

    // Create a symlink loop: a -> b -> c -> a
    std::os::unix::fs::symlink("b", base.join("a")).unwrap();
    std::os::unix::fs::symlink("c", base.join("b")).unwrap();
    std::os::unix::fs::symlink("a", base.join("c")).unwrap();

    // Create the "real" datastore that current points to
    let real_datastore = base.join("real_datastore");
    std::fs::create_dir_all(&real_datastore).unwrap();
    std::os::unix::fs::symlink(&real_datastore, base.join("current")).unwrap();

    let orphaned = find_orphaned_entries(base, real_datastore.clone()).await;

    // The loop entries should be orphaned (cycle detection marks them unreachable)
    assert!(orphaned.contains(&base.join("a")));
    assert!(orphaned.contains(&base.join("b")));
    assert!(orphaned.contains(&base.join("c")));
}
