use cem_ml::{
    diagnostics::Severity,
    schema::{
        document_model::{compile_schema_document_model, SchemaDocumentModel},
        reference_policy::{
            ReferenceOccurrence, ReferenceScopePolicy, ReferenceUnresolvedPolicy,
            UnresolvedDisposition, UnresolvedReferenceFact,
        },
    },
    source::{ByteRange, SourceId},
    source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind},
};

fn scope(source: &str) -> SchemaDocumentModel {
    compile_schema_document_model("https://example.test/scope", source)
}

fn declaration(value: &str) -> SchemaDocumentModel {
    scope(&format!(
        r#"{{schema | {{constraints |
        {{constraint @kind="reference-unresolved-disposition" @value="{value}"}}
    }} }}"#
    ))
}

fn fact() -> UnresolvedReferenceFact {
    UnresolvedReferenceFact {
        occurrence: ReferenceOccurrence {
            identity: "fixture:7".into(),
            node_id: Some(7),
            expression: Some("#dependency".into()),
            source_map: SourceMapStack {
                frames: vec![SourceMapFrame {
                    source_id: SourceId(1),
                    span: FrameSpan::Single(ByteRange::new(14, 11)),
                    transform: TransformKind::CemTokenizer,
                }],
            },
        },
        reason: "dependency-unavailable".into(),
    }
}

#[test]
fn neutral_fallback_and_explicit_child_overrides_preserve_parent_policy() {
    let default = ReferenceUnresolvedPolicy::schema_defaults().unwrap();
    assert_eq!(default.disposition(), UnresolvedDisposition::Neutral);
    let absent = default.for_scope(&scope("{schema}")).unwrap();
    assert_eq!(absent.disposition(), UnresolvedDisposition::Neutral);
    let parent = default.for_scope(&declaration("mandatory")).unwrap();
    let inherited = parent.for_scope(&scope("{schema}")).unwrap();
    assert_eq!(inherited.disposition(), UnresolvedDisposition::Mandatory);
    for (value, expected) in [
        ("neutral", UnresolvedDisposition::Neutral),
        ("warning", UnresolvedDisposition::Warning),
        ("ignore", UnresolvedDisposition::Ignore),
    ] {
        let child = parent.for_scope(&declaration(value)).unwrap();
        assert_eq!(child.disposition(), expected);
        assert_eq!(parent.disposition(), UnresolvedDisposition::Mandatory);
    }
}

#[test]
fn every_disposition_retains_unresolved_fact_and_its_provenance() {
    let default = ReferenceUnresolvedPolicy::schema_defaults().unwrap();
    for (value, severity, failed) in [
        ("neutral", None, false),
        ("mandatory", Some(Severity::Error), true),
        ("warning", Some(Severity::Warning), false),
        ("ignore", None, false),
    ] {
        let policy = default.for_scope(&declaration(value)).unwrap();
        let input = fact();
        let treatment = policy.apply(&input);
        assert_eq!(treatment.fact, input);
        assert_eq!(treatment.disposition, policy.disposition());
        assert_eq!(treatment.failed, failed);
        assert_eq!(treatment.diagnostic.as_ref().map(|d| d.severity), severity);
        if let Some(diagnostic) = treatment.diagnostic {
            assert_eq!(
                diagnostic.source_map.as_ref(),
                Some(&input.occurrence.source_map)
            );
            assert_eq!(diagnostic.byte_offset, Some(14));
            assert!(diagnostic.message.contains("#dependency"));
            assert!(diagnostic.message.contains("dependency-unavailable"));
        }
        // A neutral/ignored treatment still carries an unresolved fact. This
        // is not a successful empty target list or an accepted cardinality.
        assert_eq!(treatment.fact.reason, "dependency-unavailable");
    }
}

#[test]
fn custom_schema_diagnostics_are_inherited_with_the_declaring_policy() {
    let model = scope(
        r#"{schema |
        {constraints | {constraint @kind="reference-unresolved-disposition"
            @value="mandatory" @diagnostic="vendor.reference.required"}}
        {diagnostics | {diagnostic @code="vendor.reference.required" @severity="fatal"}}
    }"#,
    );
    let policy = ReferenceUnresolvedPolicy::schema_defaults()
        .unwrap()
        .for_scope(&model)
        .unwrap();
    let inherited = policy.for_scope(&scope("{schema}")).unwrap();
    let treatment = inherited.apply(&fact());
    assert!(treatment.failed);
    let diagnostic = treatment.diagnostic.unwrap();
    assert_eq!(diagnostic.code, "vendor.reference.required");
    assert_eq!(diagnostic.severity, Severity::Fatal);
}

#[test]
fn malformed_or_conflicting_policies_do_not_replace_valid_inherited_policy() {
    let parent = ReferenceUnresolvedPolicy::schema_defaults()
        .unwrap()
        .for_scope(&declaration("mandatory"))
        .unwrap();
    for value in ["", "optional", "MANDATORY", "mandatory warning"] {
        assert!(parent.for_scope(&declaration(value)).is_err(), "{value}");
    }
    for source in [
        r#"{schema | {constraints | {constraint @kind="reference-unresolved-disposition"}}}"#,
        r#"{schema | {constraints |
            {constraint @kind="reference-unresolved-disposition" @value="mandatory"}
            {constraint @kind="reference-unresolved-disposition" @value="ignore"}
        }}"#,
        r#"{schema | {constraints | {constraint @kind="reference-unresolved-disposition"
            @value="warning" @diagnostic="missing"}}}"#,
        r#"{schema |
            {constraints | {constraint @kind="reference-unresolved-disposition"
                @value="mandatory" @diagnostic="vendor.low"}}
            {diagnostics | {diagnostic @code="vendor.low" @severity="warning"}}
        }"#,
        r#"{schema |
            {constraints | {constraint @kind="reference-unresolved-disposition"
                @value="warning" @diagnostic="vendor.high"}}
            {diagnostics | {diagnostic @code="vendor.high" @severity="error"}}
        }"#,
        r#"{schema |
            {constraints | {constraint @kind="reference-unresolved-disposition"
                @value="ignore" @diagnostic="vendor.warning"}}
            {diagnostics | {diagnostic @code="vendor.warning" @severity="warning"}}
        }"#,
    ] {
        assert!(parent.for_scope(&scope(source)).is_err(), "{source}");
        assert_eq!(parent.disposition(), UnresolvedDisposition::Mandatory);
    }
}

#[test]
fn effective_policy_combines_limits_and_disposition_without_partial_overrides() {
    let default = ReferenceScopePolicy::schema_defaults().unwrap();
    assert_eq!(default.limits.max_depth, 128);
    assert_eq!(default.limits.max_work, 100_000);
    let parent = default
        .for_scope(&scope(
            r#"{schema | {constraints |
        {constraint @kind="reference-traversal-depth" @value="8"}
        {constraint @kind="reference-unresolved-disposition" @value="mandatory"}
    }}"#,
        ))
        .unwrap();
    let child = parent
        .for_scope(&scope(
            r#"{schema | {constraints |
        {constraint @kind="reference-traversal-work" @value="50"}
        {constraint @kind="reference-unresolved-disposition" @value="neutral"}
    }}"#,
        ))
        .unwrap();
    assert_eq!(child.limits.max_depth, 8);
    assert_eq!(child.limits.max_work, 50);
    assert_eq!(
        child.unresolved.disposition(),
        UnresolvedDisposition::Neutral
    );
    for kind in ["reference-traversal-depth", "reference-traversal-work"] {
        let duplicate = scope(&format!(
            r#"{{schema | {{constraints |
            {{constraint @kind="{kind}" @value="2"}}
            {{constraint @kind="{kind}" @value="3"}}
        }} }}"#
        ));
        let error = parent.for_scope(&duplicate).unwrap_err();
        assert!(error.message.contains("more than once"));
        assert!(!error.source_map.frames.is_empty());
        let malformed = scope(&format!(
            r#"{{schema | {{constraints |
            {{constraint @kind="{kind}" @value="0"}}
            {{constraint @kind="reference-unresolved-disposition" @value="ignore"}}
        }} }}"#
        ));
        let error = parent.for_scope(&malformed).unwrap_err();
        assert!(!error.source_map.frames.is_empty());
        assert_eq!(parent.limits.max_depth, 8);
        assert_eq!(parent.limits.max_work, 100_000);
        assert_eq!(
            parent.unresolved.disposition(),
            UnresolvedDisposition::Mandatory
        );
    }
}

#[test]
fn native_occurrences_do_not_require_saved_arena_or_context_handles() {
    let policy = ReferenceUnresolvedPolicy::schema_defaults()
        .unwrap()
        .for_scope(&declaration("mandatory"))
        .unwrap();
    let fact = UnresolvedReferenceFact {
        occurrence: ReferenceOccurrence {
            identity: "native:occurrence".into(),
            node_id: None,
            expression: None,
            source_map: SourceMapStack::default(),
        },
        reason: "cycle".into(),
    };
    let treatment = policy.apply(&fact);
    assert_eq!(treatment.fact, fact);
    let diagnostic = treatment.diagnostic.unwrap();
    assert_eq!(diagnostic.node.as_deref(), Some("native:occurrence"));
    assert!(diagnostic.message.contains("native:occurrence"));
    assert!(diagnostic.byte_offset.is_none());
}
