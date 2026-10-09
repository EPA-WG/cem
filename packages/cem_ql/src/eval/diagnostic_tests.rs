use super::*;
use crate::api::{compile, evaluate, CompileContext};
use cem_ml::source::{ByteRange, SourceId};
use cem_ml::source_map::{FrameSpan, SourceMapFrame, TransformKind};

#[test]
fn diagnostic_severity_is_queryable_without_losing_native_attribution() {
    for (severity, expected) in [
        (Severity::Info, "info"),
        (Severity::Warning, "warning"),
        (Severity::Error, "error"),
        (Severity::Fatal, "fatal"),
    ] {
        let diagnostic = Diagnostic {
            code: "fixture.datatype.rule".into(),
            severity,
            message: "Original validation diagnostic".into(),
            uri: Some("https://example.test/input.cem".into()),
            line: Some(3),
            column: Some(5),
            byte_offset: Some(20),
            node: Some("original-input-node".into()),
            details: Some(serde_json::json!({ "expected": "valid target" })),
            source_map: Some(SourceMapStack {
                frames: vec![SourceMapFrame {
                    source_id: SourceId(7),
                    span: FrameSpan::Single(ByteRange { start: 20, len: 11 }),
                    transform: TransformKind::CemAstBuilder,
                }],
            }),
        }
        .with_error_name("https://example.test/diagnostics", "invalid-target");
        let original = diagnostic_item(diagnostic.clone());
        let context = EvaluationContext {
            policy_bindings: BTreeMap::from([(
                "diagnostic".into(),
                ItemStream::once(original.clone()),
            )]),
            ..Default::default()
        };
        let query = compile(
            "({ accepted: true, diagnostics: diagnostic }, diagnostic.severity, record:entries(diagnostic))",
            &CompileContext {
                policy_bindings: context.policy_bindings.clone(),
                ..Default::default()
            },
        )
        .expect("native diagnostic query compiles");
        let result = evaluate(&query, &context);
        assert!(result.error.is_none(), "{result:?}");
        assert!(result.diagnostics.is_empty(), "reading is not reporting");
        assert_eq!(
            result.items.get(1),
            Some(&Item::Atomic(AtomValue::String(expected.into())))
        );

        let Item::Record(fields) = &result.items[0] else {
            panic!("constructed result record");
        };
        assert_eq!(
            fields["accepted"],
            vec![Item::Atomic(AtomValue::Boolean(true))]
        );
        assert_eq!(fields["diagnostics"], vec![original.clone()]);
        let retained = &fields["diagnostics"][0];
        assert_eq!(retained.source_map(), diagnostic.source_map);
        let retained_view = retained.view().expect("original native diagnostic");
        assert_eq!(
            retained_view.downcast_ref::<DiagnosticView>().unwrap().0,
            diagnostic,
            "all native metadata survives, including fields not exposed to queries"
        );
        assert_eq!(retained.identity(), original.identity());
        assert!(result.items[2..].iter().any(|item| {
            matches!(item, Item::Record(entry)
                if entry.get("key") == Some(&vec![Item::Atomic(AtomValue::String("severity".into()))])
                && entry.get("value") == Some(&vec![Item::Atomic(AtomValue::String(expected.into()))]))
        }), "severity must also be exposed by record enumeration");
    }
}
