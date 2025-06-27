// Unit tests for compression-related functionality

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

use super::*;

/// Creates a temporary directory with test files for compression tests
fn setup_test_files() -> TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");

    // Create a regular object file
    let regular_file_path = temp_dir.path().join("module.o");
    File::create(&regular_file_path).expect("Failed to create regular file");

    // Create a compressed object file
    let compressed_file_path = temp_dir.path().join("compressed_module.o.gz");
    File::create(&compressed_file_path).expect("Failed to create compressed file");

    temp_dir
}

#[test]
fn test_is_compressed_with_existing_file() {
    let temp_dir = setup_test_files();

    // Test with a file that exists
    let regular_file_path = temp_dir.path().join("module.o");
    let result = is_compressed(&regular_file_path);

    // Should return None since the file exists and is not compressed
    assert!(result.is_none());
}

#[test]
fn test_is_compressed_with_compressed_alternative() {
    let temp_dir = setup_test_files();

    // Test with a non-existent file that has a compressed alternative
    let non_existent_path = temp_dir.path().join("compressed_module.o");
    let compressed_path = temp_dir.path().join("compressed_module.o.gz");

    let result = is_compressed(&non_existent_path);

    // Should return Some with the compressed file path
    assert!(result.is_some());
    assert_eq!(result.unwrap(), compressed_path);
}

#[test]
fn test_is_compressed_with_no_alternative() {
    let temp_dir = setup_test_files();

    // Test with a non-existent file that has no compressed alternative
    let non_existent_path = temp_dir.path().join("nonexistent.o");

    let result = is_compressed(&non_existent_path);

    // Should return None since neither the file nor a compressed alternative exists
    assert!(result.is_none());
}
