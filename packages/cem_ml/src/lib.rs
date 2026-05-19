pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod diagnostics;
pub mod engine;
pub mod report;

// Tier A layered runtime contract (AC-F-10). Bodies arrive in Phase 11; types
// here fix the public boundary used by downstream layers and `cem-ml-cli`.
pub mod ast;
pub mod events;
pub mod formatter;
pub mod handoff;
pub mod interpreter;
pub mod observability;
pub mod parser;
pub mod plugin;
pub mod projection;
pub mod query;
pub mod real;
pub mod scheduler;
pub mod schema;
pub mod source;
pub mod source_map;
pub mod tokenizer;
pub mod validation;

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
            source_map: None,
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
    fn layered_runtime_contract_types_are_importable() {
        // AC-F-V-7: every public type named in AC-F-10 resolves via the crate's
        // public path. Stable identity check — this test catches an accidental
        // rename or unintended visibility change at the layer boundary.
        use crate::ast::BinaryAstEncoder;
        use crate::diagnostics::Diagnostic;
        use crate::events::NormalizedEvent;
        use crate::interpreter::Interpreter;
        use crate::parser::CemAstNode;
        use crate::schema::SchemaFrame;
        use crate::source::{ByteSource, DecodedChunk, EncodingDecoder};
        use crate::source_map::SourceMapFrame;
        use crate::tokenizer::SchemaToken;
        fn _accept<T>() {}
        _accept::<Diagnostic>();
        _accept::<NormalizedEvent>();
        _accept::<CemAstNode>();
        _accept::<SchemaFrame>();
        _accept::<DecodedChunk>();
        _accept::<SourceMapFrame>();
        _accept::<SchemaToken>();
        fn _trait_object_boundaries(
            _b: &dyn ByteSource,
            _d: &mut dyn EncodingDecoder,
            _i: &dyn Interpreter,
            _e: &dyn BinaryAstEncoder,
        ) {
        }
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
