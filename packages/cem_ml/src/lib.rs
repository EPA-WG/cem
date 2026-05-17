pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod diagnostics;
pub mod engine;
pub mod report;

#[cfg(feature = "fake-engine")]
pub mod fake;

#[cfg(test)]
mod tests {
    use super::*;
    use diagnostics::{Diagnostic, Severity};
    use engine::FailLevel;
    use report::{Report, ReportOptionsSnapshot, DETERMINISTIC_TIMESTAMP};

    fn diag(sev: Severity) -> Diagnostic {
        Diagnostic {
            uri: None,
            line: None,
            column: None,
            byte_offset: None,
            code: "x".into(),
            severity: sev,
            message: "x".into(),
            node: None,
        }
    }

    #[test]
    fn version_matches_cargo() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn report_summary_counts_severities() {
        let diags = vec![
            diag(Severity::Info),
            diag(Severity::Warning),
            diag(Severity::Error),
            diag(Severity::Fatal),
        ];
        let report = Report::deterministic(
            vec!["a.cem".into(), "b.cem".into()],
            diags,
            ReportOptionsSnapshot {
                fail_level: FailLevel::Validate,
                schema: None,
                content_type: None,
                base_uri: None,
            },
        );
        assert_eq!(report.summary.input_count, 2);
        assert_eq!(report.summary.info_count, 1);
        assert_eq!(report.summary.warning_count, 1);
        assert_eq!(report.summary.error_count, 1);
        assert_eq!(report.summary.fatal_count, 1);
        assert_eq!(report.summary.hard_violation_count, 2);
        assert_eq!(report.generated_at, DETERMINISTIC_TIMESTAMP);
    }

    #[test]
    fn report_serializes_with_contract_field_names() {
        let report = Report::deterministic(
            vec!["a.cem".into()],
            vec![],
            ReportOptionsSnapshot {
                fail_level: FailLevel::Strict,
                schema: Some("s".into()),
                content_type: Some("application/cem".into()),
                base_uri: Some("file:///x/".into()),
            },
        );
        let v = serde_json::to_value(&report).unwrap();
        assert!(v.get("generatedAt").is_some());
        assert!(v.get("inputs").is_some());
        let summary = v.get("summary").unwrap();
        for k in [
            "inputCount",
            "infoCount",
            "warningCount",
            "errorCount",
            "fatalCount",
            "hardViolationCount",
        ] {
            assert!(summary.get(k).is_some(), "missing summary.{k}");
        }
        let opts = v.get("options").unwrap();
        for k in ["failLevel", "schema", "contentType", "baseUri"] {
            assert!(opts.get(k).is_some(), "missing options.{k}");
        }
    }
}
