use std::io::{Error, Write};

use crate::results::{CheckStatus, ReportResults};

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub trait ReportWriter {
    fn write(&self, report: &ReportResults, output: &mut dyn Write) -> Result<(), Error>;
}

pub struct TextReportWriter {}

impl ReportWriter for TextReportWriter {
    /// Writes a text formatted report to the provided output destination.
    fn write(&self, report: &ReportResults, output: &mut dyn Write) -> Result<(), Error> {
        if let Some(name) = &report.metadata.name {
            writeln!(output, "{:17}{}", "Benchmark name:", name)?;
        }
        if let Some(version) = &report.metadata.version {
            writeln!(output, "{:17}{}", "Version:", version)?;
        }
        if let Some(url) = &report.metadata.url {
            writeln!(output, "{:17}{}", "Reference:", url)?;
        }
        writeln!(output, "{:17}{}", "Benchmark level:", report.level)?;
        writeln!(output, "{:17}{}", "Start time:", report.timestamp)?;
        writeln!(output)?;

        let mut exceptions = Vec::new();

        for test_result in report.results.values() {
            let (status_display, mode_display) = match &test_result.result.status {
                CheckStatus::OVERRIDE(embedded_status) => {
                    if let Some(reason) = &test_result.result.override_reason {
                        exceptions.push((test_result.metadata.id.clone(), reason.clone()));
                    }
                    (embedded_status.to_string(), "Exception".to_string())
                }
                _ => (
                    test_result.result.status.to_string(),
                    test_result.metadata.mode.to_string(),
                ),
            };

            writeln!(
                output,
                "[{}] {:9} {} ({})",
                status_display, test_result.metadata.id, test_result.metadata.title, mode_display
            )?;
        }

        writeln!(output)?;
        writeln!(output, "{:17}{}", "Passed:", report.passed)?;
        writeln!(output, "{:17}{}", "Failed:", report.failed)?;
        writeln!(output, "{:17}{}", "Skipped:", report.skipped)?;
        writeln!(output, "{:17}{}", "Total checks:", report.total)?;
        writeln!(output)?;
        writeln!(output, "Compliance check result: {}", report.status)?;

        if !exceptions.is_empty() {
            writeln!(output)?;
            writeln!(output, "Exceptions:")?;
            for (test_id, reason) in exceptions {
                writeln!(output, "*  {test_id} {reason}")?;
            }
        }

        Ok(())
    }
}

pub struct JsonReportWriter {}

impl ReportWriter for JsonReportWriter {
    /// Writes a json formatted report to the provided output destination.
    fn write(&self, report: &ReportResults, output: &mut dyn Write) -> Result<(), Error> {
        let json = serde_json::to_string(&report)?;
        writeln!(output, "{json}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::results::{CheckStatus, CheckerMetadata, CheckerResult, Mode, ReportMetadata};
    use std::io::Cursor;

    #[test]
    fn test_text_output_with_overrides() {
        // GIVEN: A report with regular tests and various override statuses
        // WHEN: TextReportWriter writes the report
        // THEN: Overrides should show embedded status with Exception mode and appear in Exceptions section

        let metadata = ReportMetadata {
            name: None,
            version: None,
            url: None,
        };
        let mut report = ReportResults::new(1, metadata);

        // Add regular passing test
        let pass_metadata = CheckerMetadata {
            name: "test_regular".to_string(),
            id: "1.1.1".to_string(),
            level: 1,
            title: "Regular test".to_string(),
            mode: Mode::Automatic,
        };
        report.add_result(
            pass_metadata,
            CheckerResult {
                status: CheckStatus::PASS,
                error: String::new(),
                override_reason: None,
            },
        );

        // Add override with PASS status
        let override_pass_metadata = CheckerMetadata {
            name: "test_pass".to_string(),
            id: "2.2.2".to_string(),
            level: 1,
            title: "Override pass test".to_string(),
            mode: Mode::Automatic,
        };
        report.add_result(
            override_pass_metadata,
            CheckerResult {
                status: CheckStatus::OVERRIDE(Box::new(CheckStatus::PASS)),
                error: String::new(),
                override_reason: Some("Override reason for pass".to_string()),
            },
        );

        // Add override with FAIL status
        let override_fail_metadata = CheckerMetadata {
            name: "test_fail".to_string(),
            id: "3.3.3".to_string(),
            level: 1,
            title: "Override fail test".to_string(),
            mode: Mode::Automatic,
        };
        report.add_result(
            override_fail_metadata,
            CheckerResult {
                status: CheckStatus::OVERRIDE(Box::new(CheckStatus::FAIL)),
                error: String::new(),
                override_reason: Some("Override reason for fail".to_string()),
            },
        );

        // Add override with SKIP status
        let override_skip_metadata = CheckerMetadata {
            name: "test_skip".to_string(),
            id: "4.4.4".to_string(),
            level: 1,
            title: "Override skip test".to_string(),
            mode: Mode::Automatic,
        };
        report.add_result(
            override_skip_metadata,
            CheckerResult {
                status: CheckStatus::OVERRIDE(Box::new(CheckStatus::SKIP)),
                error: String::new(),
                override_reason: Some("Override reason for skip".to_string()),
            },
        );

        let writer = TextReportWriter {};
        let mut output = Cursor::new(Vec::new());
        writer.write(&report, &mut output).unwrap();

        let output_str = String::from_utf8(output.into_inner()).unwrap();

        // Verify regular test shows as PASS
        assert!(output_str.contains("[PASS] 1.1.1     Regular test (Automatic)"));

        // Verify overrides show their embedded status with Exception mode
        assert!(output_str.contains("[PASS] 2.2.2     Override pass test (Exception)"));
        assert!(output_str.contains("[FAIL] 3.3.3     Override fail test (Exception)"));
        assert!(output_str.contains("[SKIP] 4.4.4     Override skip test (Exception)"));

        // Verify Exceptions section contains all override reasons
        assert!(output_str.contains("Exceptions:"));
        assert!(output_str.contains("*  2.2.2 Override reason for pass"));
        assert!(output_str.contains("*  3.3.3 Override reason for fail"));
        assert!(output_str.contains("*  4.4.4 Override reason for skip"));
    }
}
