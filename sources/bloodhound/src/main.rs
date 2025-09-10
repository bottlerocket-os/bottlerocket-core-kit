/*!
# Introduction

Bloodhound is a command line orchestrator for running a set of compliance
checks. This can be used to run CIS benchmark compliance, though it can be extended
to perform any kind of check that adheres to the expected checker interface.

Checks are performed and their results are provided in an overall report.
The checker report can be written to a file, or viewed from stdout.
By default the report is provided in a human readable text format, but can also
be generated as JSON to make it easy to consume programmatically for integrating
into further compliance automation.

# Usage

Bloodhound is ultimately intended to be used through the Bottlerocket `apiclient`
interface.
If executing directly, run `bloodhound --help` for usage information.
*/

use bloodhound::args::*;
use bloodhound::output::{JsonReportWriter, ReportWriter, TextReportWriter};
use bloodhound::results::{
    CheckStatus, CheckerMetadata, CheckerResult, OverrideConfig, ReportMetadata, ReportResults,
};
use std::collections::HashMap;
use std::fs::{DirEntry, File};
use std::io::{stdout, BufReader, Error, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};
use std::{fs, path::PathBuf};

// Define some exit codes for error conditions
const CHECKER_DISCOVERY_ERROR: i32 = 2;
const REPORT_OUTPUT_ERROR: i32 = 3;
const NO_CHECKS_RUN_ERROR: i32 = 4;

/// Finds the metadata information for the checkers being run or provides a default.
fn read_metadata(check_dir: &Path) -> ReportMetadata {
    let meta_path = check_dir.join("metadata.json");

    if let Ok(file) = File::open(meta_path) {
        let reader = BufReader::new(file);
        if let Ok(report_metadata) = serde_json::from_reader(reader) {
            return report_metadata;
        }
    }

    ReportMetadata {
        name: None,
        version: None,
        url: None,
    }
}

/// Find executable checkers and matching override files in the given directory.
///
/// Uses two-pass approach: first finds checker executables, then looks for
/// matching `{test_name}.json` override files to avoid parsing metadata.json.
fn find_directory_contents(
    check_dir: &Path,
    level: u8,
) -> (
    HashMap<String, CheckerMetadata>,
    HashMap<String, OverrideConfig>,
) {
    let (checkers, test_names) = find_checkers(check_dir, level);
    let overrides = find_overrides(check_dir, &test_names);
    (checkers, overrides)
}

/// Find all executable checker files in the directory.
///
/// Returns both the checker metadata map and list of test names for override lookup.
fn find_checkers(check_dir: &Path, level: u8) -> (HashMap<String, CheckerMetadata>, Vec<String>) {
    let entries: Vec<DirEntry> = fs::read_dir(check_dir)
        .unwrap()
        .filter_map(|file| file.ok())
        .collect();
    let mut checkers = HashMap::new();
    let mut test_names = Vec::new();

    for entry in entries {
        if let Ok(file_metadata) = fs::metadata(entry.path()) {
            if file_metadata.is_dir() {
                continue;
            }

            let path = entry.path();

            // Skip any files that are not executable
            if file_metadata.permissions().mode() & 0o111 == 0 {
                continue;
            }

            // It's an executable file, make sure it implements our expected checker interface
            let metadata;
            if let Ok(output) = Command::new(&path).arg("metadata").output() {
                metadata = String::from_utf8_lossy(&output.stdout).to_string();
            } else {
                eprintln!("{path:?} does not appear to be a checker executable");
                continue;
            }

            if let Ok(checker_data) = serde_json::from_str::<CheckerMetadata>(&metadata) {
                if checker_data.level <= level {
                    test_names.push(checker_data.name.clone());
                    checkers.insert(path.to_string_lossy().to_string(), checker_data);
                }
            } else {
                eprintln!("Unable to parse checker metadata from {path:?}");
            }
        }
    }

    (checkers, test_names)
}

/// Find override files matching the given test names.
///
/// Looks for `{test_name}.json` files and parses them as override configurations.
fn find_overrides(check_dir: &Path, test_names: &[String]) -> HashMap<String, OverrideConfig> {
    let mut overrides = HashMap::new();
    for test_name in test_names {
        let override_path = check_dir.join(format!("{test_name}.json"));
        if override_path.exists() {
            match load_override_config(&override_path) {
                Ok(override_config) => {
                    overrides.insert(test_name.clone(), override_config);
                }
                Err(err) => {
                    eprintln!("Warning: Failed to load override file {override_path:?}: {err}");
                }
            }
        }
    }
    overrides
}

/// Load and parse an override configuration file.
///
/// Reads the JSON file and parses it into an OverrideConfig struct.
fn load_override_config(path: &Path) -> Result<OverrideConfig, Box<dyn std::error::Error>> {
    let file_content = fs::read_to_string(path)?;
    let override_config = serde_json::from_str(&file_content)?;
    Ok(override_config)
}

/// Find an override entry for the given test name.
///
/// Looks up override configuration by test name (filename-based).
fn find_override_for_test<'a>(
    overrides: &'a HashMap<String, OverrideConfig>,
    test_name: &str,
) -> Option<&'a OverrideConfig> {
    overrides.get(test_name)
}

/// Apply an override to a test result.
///
/// Creates an override result with the specified status and reason.
fn apply_override(
    report: &mut ReportResults,
    data: CheckerMetadata,
    override_config: &OverrideConfig,
) {
    let override_status = match override_config.status.as_str() {
        "PASS" => CheckStatus::PASS,
        "FAIL" => CheckStatus::FAIL,
        "SKIP" => CheckStatus::SKIP,
        _ => CheckStatus::SKIP, // Default to skip for unknown statuses
    };

    report.add_result(
        data,
        CheckerResult {
            status: CheckStatus::OVERRIDE(Box::new(override_status)),
            error: String::new(),
            override_reason: Some(override_config.reason.clone()),
        },
    );
}

/// Process output from a checker executable and add results to the report.
///
/// Parses checker output as JSON and converts to CheckerResult format.
fn process_checker_results(output: Output, data: CheckerMetadata, report: &mut ReportResults) {
    if output.status.success() {
        let check_output = String::from_utf8_lossy(&output.stdout).to_string();
        if let Ok(checker_result) = serde_json::from_str::<CheckerResult>(&check_output) {
            report.add_result(data, checker_result);
            return;
        }
    }

    let check_output = String::from_utf8_lossy(&output.stderr).to_string();
    report.add_result(
        data,
        CheckerResult {
            status: CheckStatus::SKIP,
            error: check_output,
            override_reason: None,
        },
    );
}

fn get_output(output: &Option<String>) -> Result<Box<dyn Write>, Error> {
    match output {
        Some(path) => File::create(path).map(|f| Box::new(f) as Box<dyn Write>),
        None => Ok(Box::new(stdout())),
    }
}

fn main() {
    let args: Arguments = argh::from_env();

    // Find all checkers in checks directory
    let check_dir = PathBuf::from(args.check_dir);
    if !check_dir.is_dir() {
        eprintln!("Checker path {check_dir:?} is not a directory!");
        std::process::exit(CHECKER_DISCOVERY_ERROR);
    }

    // Look through the directory and find any checkers and overrides, then filter out checks based on input
    let (checkers, overrides) = find_directory_contents(&check_dir, args.level);
    if checkers.is_empty() {
        eprintln!("No checkers found in {check_dir:?}!");
        std::process::exit(CHECKER_DISCOVERY_ERROR);
    }

    let report_metadata = read_metadata(&check_dir);
    let mut report = ReportResults::new(args.level, report_metadata);

    // Execute each checker and capture results
    for (checker, data) in checkers {
        if let Some(override_config) = find_override_for_test(&overrides, &data.name) {
            apply_override(&mut report, data, override_config);
        } else if let Ok(output) = Command::new(checker).output() {
            process_checker_results(output, data, &mut report);
        } else {
            // Something failed in execution, mark as skipped with message
            let msg = format!("Error executing {} checker.", data.name);
            report.add_result(
                data,
                CheckerResult {
                    status: CheckStatus::SKIP,
                    error: msg,
                    override_reason: None,
                },
            );
        }
    }

    // Write appropriate output results report
    let mut output_dest = get_output(&args.output).unwrap_or_else(|err| {
        eprintln!("Error writing to output destination {err}!");
        std::process::exit(REPORT_OUTPUT_ERROR);
    });

    let reporter: &dyn ReportWriter = match args.format {
        Format::Json => &JsonReportWriter {},
        Format::Text => &TextReportWriter {},
    };

    if let Err(err) = reporter.write(&report, &mut *output_dest) {
        eprintln!("Error writing report output: {err}");
        std::process::exit(REPORT_OUTPUT_ERROR);
    }

    if report.status == CheckStatus::SKIP {
        // Something is wrong, no automated checks were able to run. Better
        // alert on something like this as it may be a sign of a larger problem.
        eprintln!("Warning: No checks were able to run");
        std::process::exit(NO_CHECKS_RUN_ERROR);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_directory_contents_with_valid_override() {
        // GIVEN: A directory with a mock checker and matching override JSON file
        // WHEN: find_directory_contents is called
        // THEN: Override should be parsed and returned correctly

        let temp_dir = TempDir::new().unwrap();

        // Create mock checker executable
        let checker_script = r#"#!/bin/bash
if [ "$1" = "metadata" ]; then
    echo '{"name": "test123", "id": "test123", "level": 1, "title": "Test Check", "mode": "Automatic"}'
else
    echo '{"status": "PASS", "error": ""}'
fi
"#;
        let checker_path = temp_dir.path().join("test123");
        fs::write(&checker_path, checker_script).unwrap();
        fs::set_permissions(&checker_path, fs::Permissions::from_mode(0o755)).unwrap();

        let override_json = r#"{
            "reason": "Test reason",
            "status": "SKIP"
        }"#;

        fs::write(temp_dir.path().join("test123.json"), override_json).unwrap();

        let (checkers, overrides) = find_directory_contents(&temp_dir.path().to_path_buf(), 1);

        assert_eq!(checkers.len(), 1);
        assert_eq!(overrides.len(), 1);
        assert!(overrides.contains_key("test123"));

        let override_config = &overrides["test123"];
        assert_eq!(override_config.reason, "Test reason");
    }

    #[test]
    fn test_find_directory_contents_ignores_metadata_json() {
        // GIVEN: A directory with metadata.json and other non-override JSON files
        // WHEN: find_directory_contents is called
        // THEN: Non-override JSON files should be ignored

        let temp_dir = TempDir::new().unwrap();

        // Create metadata.json (should be ignored)
        let metadata_json = r#"{"version": "1.0", "description": "metadata file"}"#;
        fs::write(temp_dir.path().join("metadata.json"), metadata_json).unwrap();

        // Create other JSON file (should be ignored)
        let other_json = r#"{"some": "data"}"#;
        fs::write(temp_dir.path().join("other.json"), other_json).unwrap();

        let (checkers, overrides) = find_directory_contents(&temp_dir.path().to_path_buf(), 1);

        assert!(checkers.is_empty());
        assert!(overrides.is_empty());
    }

    #[test]
    fn test_find_directory_contents_no_overrides() {
        // GIVEN: A directory with no JSON files
        // WHEN: find_directory_contents is called
        // THEN: No overrides should be returned (backward compatibility)

        let temp_dir = TempDir::new().unwrap();

        let (checkers, overrides) = find_directory_contents(&temp_dir.path().to_path_buf(), 1);

        assert!(checkers.is_empty());
        assert!(overrides.is_empty());
    }

    #[test]
    fn test_find_directory_contents_multiple_overrides() {
        // GIVEN: A directory with multiple checkers and matching override files
        // WHEN: find_directory_contents is called
        // THEN: All valid overrides should be parsed

        let temp_dir = TempDir::new().unwrap();

        // Create first checker and override
        let checker1_script = r#"#!/bin/bash
if [ "$1" = "metadata" ]; then
    echo '{"name": "test1", "id": "test1", "level": 1, "title": "Test 1", "mode": "Automatic"}'
else
    echo '{"status": "PASS", "error": ""}'
fi
"#;
        let checker1_path = temp_dir.path().join("test1");
        fs::write(&checker1_path, checker1_script).unwrap();
        fs::set_permissions(&checker1_path, fs::Permissions::from_mode(0o755)).unwrap();

        let override1_json = r#"{
            "reason": "Reason 1",
            "status": "SKIP"
        }"#;

        // Create second checker and override
        let checker2_script = r#"#!/bin/bash
if [ "$1" = "metadata" ]; then
    echo '{"name": "test2", "id": "test2", "level": 1, "title": "Test 2", "mode": "Automatic"}'
else
    echo '{"status": "PASS", "error": ""}'
fi
"#;
        let checker2_path = temp_dir.path().join("test2");
        fs::write(&checker2_path, checker2_script).unwrap();
        fs::set_permissions(&checker2_path, fs::Permissions::from_mode(0o755)).unwrap();

        let override2_json = r#"{
            "reason": "Reason 2",
            "status": "SKIP"
        }"#;

        fs::write(temp_dir.path().join("test1.json"), override1_json).unwrap();
        fs::write(temp_dir.path().join("test2.json"), override2_json).unwrap();

        let (checkers, overrides) = find_directory_contents(&temp_dir.path().to_path_buf(), 1);

        assert_eq!(checkers.len(), 2);
        assert_eq!(overrides.len(), 2);
        assert!(overrides.contains_key("test1"));
        assert!(overrides.contains_key("test2"));
    }
}
