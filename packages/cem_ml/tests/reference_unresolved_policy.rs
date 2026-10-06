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


#[test]
fn local_policy_provenance_preserves_explicit_defaults_and_inherited_settings() {
    use cem_ml::schema::reference_policy::{
        ReferencePolicyOverrideOrigin, ReferenceScopePolicyOverrides,
    };
    let local_model =
        scope("{schema | {constraints | {constraint @kind=reference-traversal-depth @value=128}}}");
    let local = ReferenceScopePolicyOverrides::from_schema(&local_model).unwrap();
    assert_eq!(*local.depth().unwrap().value(), 128);
    let ReferencePolicyOverrideOrigin::Schema(source) = local.depth().unwrap().origin() else {
        panic!("expected original schema declaration provenance");
    };
    assert!(source.origin().is_some());
    assert!(local.work().is_none());
    assert!(local.unresolved().is_none());
    let child = ReferenceScopePolicy::schema_defaults().unwrap().for_scope(&scope("{schema | {constraints | {constraint @kind=reference-traversal-depth @value=3} {constraint @kind=reference-traversal-work @value=17} {constraint @kind=reference-unresolved-disposition @value=warning}}}")).unwrap();
    let effective = local.apply_to(&child);
    assert_eq!(effective.limits.max_depth, 128);
    assert_eq!(effective.limits.max_work, 17);
    assert_eq!(
        effective.unresolved.disposition(),
        UnresolvedDisposition::Warning
    );
    assert_eq!(child.limits.max_depth, 3);
    let neutral = ReferenceScopePolicyOverrides::from_schema(&scope("{schema | {constraints | {constraint @kind=reference-unresolved-disposition @value=neutral}}}")).unwrap();
    assert!(neutral.unresolved().is_some());
    assert_eq!(
        neutral.apply_to(&child).unresolved.disposition(),
        UnresolvedDisposition::Neutral
    );
}

#[test]
fn complete_caller_policies_are_explicit_and_malformed_schema_provenance_is_rejected() {
    use cem_ml::schema::reference_policy::{
        ReferencePolicyOverrideOrigin, ReferenceScopePolicyOverrides,
    };
    let defaults = ReferenceScopePolicy::schema_defaults().unwrap();
    let explicit = ReferenceScopePolicyOverrides::explicit(defaults.clone());
    assert_eq!(
        explicit.depth().unwrap().origin(),
        &ReferencePolicyOverrideOrigin::Caller
    );
    assert_eq!(
        explicit.work().unwrap().origin(),
        &ReferencePolicyOverrideOrigin::Caller
    );
    assert_eq!(
        explicit.unresolved().unwrap().origin(),
        &ReferencePolicyOverrideOrigin::Caller
    );
    let child = defaults.for_scope(&scope("{schema | {constraints | {constraint @kind=reference-traversal-depth @value=3} {constraint @kind=reference-traversal-work @value=17} {constraint @kind=reference-unresolved-disposition @value=warning}}}")).unwrap();
    let effective = explicit.apply_to(&child);
    assert_eq!(effective.limits, defaults.limits);
    assert_eq!(
        effective.unresolved.disposition(),
        defaults.unresolved.disposition()
    );
    for text in [
        "{schema | {constraints | {constraint @kind=reference-traversal-depth @value=0}}}",
        "{schema | {constraints | {constraint @kind=reference-unresolved-disposition @value=bad}}}",
    ] {
        let error = ReferenceScopePolicyOverrides::from_schema(&scope(text)).unwrap_err();
        assert!(error.source_map.origin().is_some());
    }
}
