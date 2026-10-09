use super::attribute_consumer::fixture_with_host;
use super::*;
use cem_ml::schema::{
    declaration_references::SchemaDeclarationHost,
    document_model::{
        attribute_facets::FacetFamily, shipped_datatypes::ShippedDatatype as T, DiagnosticBehavior,
        EngineDiagnosticBehavior,
    },
};
use cem_ql::{
    attribute_readiness::*, attribute_validation::*, datatype_facets::BoundAttributeFacets,
};

fn bindings(
    bound: &BoundAttributeFacets,
    host: &impl SchemaDeclarationHost,
    complete: bool,
) -> AttributeDiagnosticBindings {
    AttributeDiagnosticBindings {
        schema: host
            .declaration_schema(&bound.binding().declaration)
            .unwrap(),
        complete,
        behaviors: Default::default(),
    }
}
fn check(
    bound: &BoundAttributeFacets,
    host: &impl SchemaDeclarationHost,
    bindings: &AttributeDiagnosticBindings,
) -> AttributeDeclarationReadiness {
    let control = OperationControl::default();
    bound.check_declaration_readiness(
        host,
        bindings,
        &ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        },
        Default::default(),
    )
}
fn diagnostic(code: &str, family: EngineDiagnosticBehavior) -> DiagnosticBehavior {
    DiagnosticBehavior {
        code: code.into(),
        severity: cem_ml::diagnostics::Severity::Warning,
        behavior: "registered".into(),
        definition: None,
        engine_behavior: Some(family),
        function: None,
        function_definition: None,
        arguments: vec![],
        message: Some("original diagnostic".into()),
        source_map: Default::default(),
    }
}
#[test]
fn default_readiness_uses_original_field_candidate_and_both_contracts() {
    for (rule, value, expected) in [
        (true, "003", AttributeReadinessState::Ready),
        (true, "005", AttributeReadinessState::Invalid),
        (false, "003", AttributeReadinessState::Invalid),
    ] {
        let (bound, host) = fixture_with_host(
            FacetFamily::Shipped(T::Integer),
            &format!("@default={value} @maxInclusive=4"),
            true,
            rule,
            false,
        );
        let report = check(&bound, &host, &bindings(&bound, &host, true));
        assert_eq!(report.state, expected, "{report:?}");
        let field = report.default_source.unwrap();
        let CemAstNode::Attribute {
            expanded_name,
            source,
            ..
        } = field.node()
        else {
            panic!()
        };
        assert_eq!(expanded_name.local_name, "default");
        let result = report.default_validation.unwrap();
        let AttributeDatatypePhase::Lexical(prepared) = result.datatype.unwrap() else {
            panic!()
        };
        assert_eq!(&*prepared.input.lexical.text, value);
        assert_eq!(&prepared.input.lexical.source, source);
        let candidate = cem_ql::eval::retained_cem_node(&prepared.input.candidate[0]).unwrap();
        assert!(Arc::ptr_eq(candidate.owner().ast_owner(), field.document()));
        assert_eq!(candidate.node_id(), field.node_id());
        assert!(bound.binding().datatype.converter().is_none());
    }
}
#[test]
fn default_readiness_distinguishes_absence_empty_and_incomplete_preparation() {
    for (fields, prepare, expected, has_default) in [
        ("", false, AttributeReadinessState::Ready, false),
        (
            "@default=003",
            false,
            AttributeReadinessState::Pending,
            true,
        ),
        (
            r#"@default="""#,
            true,
            AttributeReadinessState::Invalid,
            true,
        ),
    ] {
        let (bound, host) = fixture_with_host(
            FacetFamily::Shipped(T::Integer),
            fields,
            prepare,
            true,
            false,
        );
        let report = check(&bound, &host, &bindings(&bound, &host, true));
        assert_eq!(report.state, expected, "{report:?}");
        assert_eq!(report.default_source.is_some(), has_default);
    }
    let (bound, host) = fixture_with_host(
        FacetFamily::Shipped(T::String),
        r#"@default="""#,
        true,
        true,
        false,
    );
    assert!(check(&bound, &host, &bindings(&bound, &host, true)).is_ready());
}
#[test]
fn default_readiness_never_turns_literal_node_defaults_into_references() {
    let (bound, host) =
        fixture_with_host(FacetFamily::Nodes, "@default=target", false, true, false);
    let report = check(&bound, &host, &bindings(&bound, &host, true));
    assert_eq!(report.state, AttributeReadinessState::Invalid);
    assert_eq!(report.issues[0].code, "attribute-default-native-required");
    assert!(report.default_validation.is_none());
}
#[test]
fn diagnostic_readiness_requires_complete_original_scope_bindings_and_expected_families() {
    for (field, family) in [
        ("type-diagnostic", EngineDiagnosticBehavior::ScalarType),
        (
            "values-diagnostic",
            EngineDiagnosticBehavior::ValueVocabulary,
        ),
        (
            "datatype-param-diagnostic",
            EngineDiagnosticBehavior::DatatypeParam,
        ),
    ] {
        let (bound, host) = fixture_with_host(
            FacetFamily::Shipped(T::Integer),
            &format!("@default=003 @{field}=custom"),
            true,
            true,
            false,
        );
        let mut snapshot = bindings(&bound, &host, false);
        assert_eq!(
            check(&bound, &host, &snapshot).state,
            AttributeReadinessState::Pending
        );
        snapshot.complete = true;
        let report = check(&bound, &host, &snapshot);
        assert_eq!(report.state, AttributeReadinessState::Invalid);
        assert_eq!(report.issues[0].code, "attribute-diagnostic-unresolved");
        assert!(report.default_validation.is_none());
        snapshot.behaviors.insert(
            "custom".into(),
            diagnostic("custom", EngineDiagnosticBehavior::ResourceParse),
        );
        assert_eq!(
            check(&bound, &host, &snapshot).issues[0].code,
            "attribute-diagnostic-family"
        );
        snapshot
            .behaviors
            .insert("custom".into(), diagnostic("custom", family));
        let report = check(&bound, &host, &snapshot);
        assert!(report.is_ready(), "{report:?}");
        snapshot.complete = false;
        assert_eq!(
            check(&bound, &host, &snapshot).state,
            AttributeReadinessState::Pending
        );
    }
}
#[test]
fn readiness_rejects_replacement_scope_and_preserves_custom_default_diagnostics() {
    let fields = "@default=005 @maxInclusive=4 @datatype-param-diagnostic=custom";
    let (bound, host) =
        fixture_with_host(FacetFamily::Shipped(T::Integer), fields, true, true, false);
    let (other, other_host) =
        fixture_with_host(FacetFamily::Shipped(T::Integer), fields, true, true, false);
    let mut foreign = bindings(&other, &other_host, true);
    foreign.behaviors.insert(
        "custom".into(),
        diagnostic("custom", EngineDiagnosticBehavior::DatatypeParam),
    );
    let report = check(&bound, &host, &foreign);
    assert_eq!(report.state, AttributeReadinessState::Invalid);
    assert_eq!(report.issues[0].code, "attribute-diagnostic-scope-mismatch");
    let mut local = bindings(&bound, &host, true);
    local.behaviors = foreign.behaviors;
    let report = check(&bound, &host, &local);
    assert_eq!(report.state, AttributeReadinessState::Invalid);
    let facets = report.default_validation.unwrap().facets.unwrap();
    assert_eq!(facets.diagnostics[0].code, "custom");
    assert_eq!(
        facets.diagnostics[0].severity,
        cem_ml::diagnostics::Severity::Warning
    );
    let CemAstNode::Attribute { source, .. } = report.default_source.unwrap().node().clone() else {
        panic!()
    };
    assert_eq!(facets.diagnostics[0].source_map, Some(source));
}
#[test]
fn readiness_cancellation_and_limits_leave_defaults_incomplete() {
    let (bound, host) = fixture_with_host(
        FacetFamily::Shipped(T::Integer),
        "@default=003",
        true,
        true,
        false,
    );
    let snapshot = bindings(&bound, &host, true);
    let signal = AbortSignal::new();
    signal.abort();
    let control = OperationControl::new(signal);
    let report = bound.check_declaration_readiness(
        &host,
        &snapshot,
        &ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        },
        Default::default(),
    );
    assert_eq!(report.state, AttributeReadinessState::Pending);
    assert!(report.control_stop.is_some());
    let control = OperationControl::default();
    let mut limits = AttributeValidationLimits::default();
    limits.preparation.max_lexical_bytes = 1;
    let report = bound.check_declaration_readiness(
        &host,
        &snapshot,
        &ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        },
        limits,
    );
    assert_eq!(report.state, AttributeReadinessState::Pending);
    assert_eq!(report.default_validation.unwrap().accepted, None);
}

#[test]
fn default_readiness_preserves_list_tokens_and_last_authored_default() {
    let (bound, host) = fixture_with_host(
        FacetFamily::List(ScalarRepresentation::Integer),
        r#"@default="003 003" @itemCount=2"#,
        true,
        true,
        false,
    );
    let report = check(&bound, &host, &bindings(&bound, &host, true));
    assert!(report.is_ready(), "{report:?}");
    let AttributeDatatypePhase::Lexical(prepared) =
        report.default_validation.unwrap().datatype.unwrap()
    else {
        panic!()
    };
    assert_eq!(prepared.token_spans, vec![0..3, 4..7]);
    assert_eq!(
        prepared
            .value
            .unwrap()
            .iter()
            .map(Item::atom)
            .collect::<Vec<_>>(),
        vec![Some(AtomValue::Integer(3)); 2]
    );
    let (bound, host) = fixture_with_host(
        FacetFamily::Shipped(T::Integer),
        "@default=bad @default=003",
        true,
        true,
        false,
    );
    let report = check(&bound, &host, &bindings(&bound, &host, false));
    assert!(
        report.is_ready(),
        "no diagnostic dependencies need the incomplete catalog"
    );
    assert!(
        matches!(report.default_source.unwrap().node(),CemAstNode::Attribute {value:Some(value),..} if value=="003")
    );
}
