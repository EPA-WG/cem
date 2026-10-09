use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::CemAstNode,
    schema::{
        declaration_references::SchemaDeclarationNode,
        machine::CemSchemaMachine,
        registry::CEM_SCHEMA_URI,
        value_contracts::{ContractName, ValueContractLimits, ValueContractSource, ValueContracts},
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    datatype_results::{DatatypeResultAdapter, DiagnosticAttribution},
    eval::{AtomValue, Item, ItemStream},
};
use std::{collections::BTreeMap, sync::Arc};

fn source(text: &str, namespace: &str) -> ValueContractSource {
    let captured = Arc::new(
        CemSchemaMachine::new(
            CompiledSchema::cem_core(),
            CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
                SourceId(1),
                text.as_bytes().to_vec(),
            ))),
        )
        .build_with_lexical_scopes(),
    );
    let doc = captured.document().clone();
    assert!(doc.diagnostics.is_empty(), "{:?}", doc.diagnostics);
    let root = doc
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "schema" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    ValueContractSource::new(
        SchemaDeclarationNode::new(doc, root).unwrap(),
        namespace,
        BTreeMap::from([("schema".into(), CEM_SCHEMA_URI.into())]),
    )
    .with_captured_names(captured)
    .unwrap()
}
fn registry(text: &str) -> ValueContracts {
    ValueContracts::compile(
        &[source(text, CEM_SCHEMA_URI)],
        ValueContractLimits::default(),
    )
    .unwrap()
}
fn shipped() -> ValueContracts {
    registry(
        cem_ml::schema::package_sources::builtin_schema_package_source("schema")
            .unwrap()
            .schema_source,
    )
}
fn query(text: &str) -> ItemStream {
    evaluate(
        &compile(text, &CompileContext::default()).unwrap_or_else(|e| panic!("{text}: {e:?}")),
        &EvaluationContext::default(),
    )
}
fn adapter(contracts: ValueContracts) -> DatatypeResultAdapter {
    DatatypeResultAdapter::new(
        Arc::new(contracts),
        ContractName::new(CEM_SCHEMA_URI, "datatype-validation-result"),
        ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"),
    )
    .unwrap()
}
#[test]
fn shipped_result_contract_checks_presence_cardinality_types_and_extensions() {
    let adapter = adapter(shipped());
    let fallback = DiagnosticAttribution::default();
    assert!(
        adapter
            .consume(query("{ accepted: true, diagnostics: () }"), &fallback)
            .unwrap()
            .accepted
    );
    for bad in [
        "()",
        "({ accepted: true, diagnostics: () }, { accepted: true, diagnostics: () })",
        "{ accepted: true }",
        "{ diagnostics: () }",
        "{ accepted: 1, diagnostics: () }",
        "{ accepted: (), diagnostics: () }",
        "{ accepted: (true, false), diagnostics: () }",
        "{ accepted: true, diagnostics: (), surprise: true }",
        "{ accepted: true, diagnostics: 42 }",
    ] {
        assert!(adapter.consume(query(bad), &fallback).is_err(), "{bad}");
    }
}
#[test]
fn diagnostics_use_schema_fields_and_do_not_determine_acceptance() {
    let adapter = adapter(shipped());
    let fallback = DiagnosticAttribution {
        uri: Some("input.cem".into()),
        ..Default::default()
    };
    let result = adapter.consume(query(r#"{ accepted: true, diagnostics: { code: "custom.notice", severity: "error", message: "notice" } }"#), &fallback).unwrap();
    assert!(result.accepted);
    assert_eq!(result.diagnostics[0].uri.as_deref(), Some("input.cem"));
    assert_eq!(result.diagnostics[0].code, "custom.notice");
    let rejected = adapter
        .consume(query("{ accepted: false, diagnostics: () }"), &fallback)
        .unwrap();
    assert!(!rejected.accepted);
    assert_eq!(
        rejected.diagnostics.len(),
        1,
        "rejection gets an explanation"
    );
    for bad in [
        r#"{ code: "x", severity: "debug", message: "x" }"#,
        r#"{ code: "x", severity: "warning" }"#,
        r#"{ code: "x", severity: "warning", message: "x", soruce: () }"#,
        r#"{ code: "x", severity: "warning", message: "x", source: "id" }"#,
    ] {
        assert!(
            adapter
                .consume(
                    query(&format!("{{ accepted: true, diagnostics: {bad} }}")),
                    &fallback
                )
                .is_err(),
            "{bad}"
        );
    }
}
#[test]
fn record_checks_follow_the_supplied_schema_without_rust_field_lists() {
    let text = r#"@ns s = "https://cem.dev/ns/schema/1"
@default s
{schema | {value-contracts |
 {value-contract @name=sample @allow-extra=true |
  {value-field @name=items @kind=string @required=true @cardinality=zero-or-more @values="red blue"}
 }
}}"#;
    let contracts = registry(text);
    let name = ContractName::new(CEM_SCHEMA_URI, "sample");
    assert!(cem_ql::datatype_results::validate_values(
        &contracts,
        &name,
        &query("{ items: (), extra: 1 }").items
    )
    .is_ok());
    for bad in ["{ extra: 1 }", "{ items: 1 }", "{ items: \"green\" }"] {
        assert!(
            cem_ql::datatype_results::validate_values(&contracts, &name, &query(bad).items)
                .is_err()
        );
    }
    let closed = registry(&text.replace("@allow-extra=true", ""));
    assert!(cem_ql::datatype_results::validate_values(
        &closed,
        &name,
        &query("{ items: (), extra: 1 }").items
    )
    .is_err());
}
#[test]
fn compiler_rejects_unknown_duplicate_and_cyclic_contracts_and_invalid_fields() {
    for body in [
        "{value-contract @name=x | {value-field @name=v @type=missing}}",
        "{value-contract @name=x}{value-contract @name=x}",
        "{value-contract @name=x | {value-field @name=v @type=x}}",
        "{value-contract @name=x | {value-field @name=v @kind=node @values=a}}",
        "{value-contract @name=x | {value-field @name=v @kind=integer}}",
        "{value-contract @name=x | {value-field @name=v @kind=string @cardinality=unknown}}",
        "{value-contract @name=x | {value-field @name=v @kind=string}{value-field @name=v @kind=boolean}}",
        "{value-contract @name=x @surprise=true}",
    ] {
        let text = format!("@ns s = \"{CEM_SCHEMA_URI}\"\n@default s\n{{schema | {{value-contracts | {body}}}}}");
        assert!(ValueContracts::compile(&[source(&text, CEM_SCHEMA_URI)], ValueContractLimits::default()).is_err(), "{body}");
    }
}

#[test]
fn name_binding_retains_original_declarations_and_has_no_local_name_fallback() {
    let a = source(
        r#"@ns s = "https://cem.dev/ns/schema/1"
@default s
{schema | {value-contracts | {value-contract @name=leaf | {value-field @name=label @kind=string @required=true}}}}"#,
        "urn:vendor:a",
    );
    let mut b = source(
        r#"@ns s = "https://cem.dev/ns/schema/1"
@default s
{schema | {value-contracts | {value-contract @name=root | {value-field @name=value @type="a:leaf" @required=true}}}}"#,
        "urn:vendor:b",
    );
    b.bindings.insert("a".into(), "urn:vendor:a".into());
    let owner = Arc::downgrade(a.schema.document());
    let contracts =
        ValueContracts::compile(&[a.clone(), b.clone()], ValueContractLimits::default()).unwrap();
    let original_id = contracts
        .get(&ContractName::new("urn:vendor:a", "leaf"))
        .unwrap()
        .source
        .identity();
    drop(a);
    assert!(owner.upgrade().is_some());
    assert!(cem_ql::datatype_results::validate_values(
        &contracts,
        &ContractName::new("urn:vendor:b", "root"),
        &query(r#"{ value: { label: "ok" } }"#).items
    )
    .is_ok());
    assert_eq!(
        contracts
            .get(&ContractName::new("urn:vendor:a", "leaf"))
            .unwrap()
            .source
            .identity(),
        original_id
    );
    b.bindings.insert("a".into(), "urn:wrong".into());
    assert!(ValueContracts::compile(&[b], ValueContractLimits::default()).is_err());
}

#[test]
fn runtime_and_compilation_limits_are_enforced() {
    let text = cem_ml::schema::package_sources::builtin_schema_package_source("schema")
        .unwrap()
        .schema_source;
    let source = source(text, CEM_SCHEMA_URI);
    for limits in [
        ValueContractLimits {
            max_contracts: 1,
            ..Default::default()
        },
        ValueContractLimits {
            max_fields: 1,
            ..Default::default()
        },
    ] {
        assert!(ValueContracts::compile(&[source.clone()], limits).is_err());
    }
    let contracts = ValueContracts::compile(
        &[source],
        ValueContractLimits {
            max_values: 3,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(adapter(contracts)
        .consume(
            query("{ accepted: true, diagnostics: () }"),
            &DiagnosticAttribution::default()
        )
        .is_err());
}

#[test]
fn original_native_targets_supply_source_attribution_and_arrays_are_not_streams() {
    let contracts = adapter(shipped());
    let inputs = query(r#"data:read("<root><bad/></root>", "xml").root"#);
    let node = inputs.items[0].clone();
    let diagnostic = Item::Record(BTreeMap::from([
        (
            "code".into(),
            vec![Item::Atomic(AtomValue::String("bad.target".into()))],
        ),
        (
            "severity".into(),
            vec![Item::Atomic(AtomValue::String("warning".into()))],
        ),
        (
            "message".into(),
            vec![Item::Atomic(AtomValue::String("bad".into()))],
        ),
        ("source".into(), vec![node.clone()]),
    ]));
    let record = |diag| {
        ItemStream::once(Item::Record(BTreeMap::from([
            (
                "accepted".into(),
                vec![Item::Atomic(AtomValue::Boolean(false))],
            ),
            ("diagnostics".into(), vec![diag]),
        ])))
    };
    let result = contracts
        .consume(
            record(diagnostic.clone()),
            &DiagnosticAttribution {
                uri: Some("wrong-fallback.cem".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(result.diagnostics[0].node, node.identity());
    assert_eq!(result.diagnostics[0].source_map, node.source_map());
    assert_ne!(
        result.diagnostics[0].uri.as_deref(),
        Some("wrong-fallback.cem")
    );
    let Item::Record(original) = &result.original else {
        panic!("record")
    };
    assert_eq!(original["diagnostics"], vec![diagnostic.clone()]);
    assert!(contracts
        .consume(
            record(Item::Array(vec![diagnostic.clone()])),
            &Default::default()
        )
        .is_err());
    let Item::Record(mut bad) = diagnostic else {
        unreachable!()
    };
    bad.insert("source".into(), vec![node.clone(), node]);
    assert!(contracts
        .consume(record(Item::Record(bad)), &Default::default())
        .is_err());
}

#[test]
fn schema_vocabulary_changes_drive_diagnostic_checks() {
    let text = cem_ml::schema::package_sources::builtin_schema_package_source("schema")
        .unwrap()
        .schema_source;
    let changed = text.replace(
        "@values=\"info warning error fatal\"",
        "@values=\"error fatal\"",
    );
    let adapter = adapter(registry(&changed));
    assert!(adapter.consume(query(r#"{ accepted: true, diagnostics: { code: "x", severity: "warning", message: "x" } }"#), &Default::default()).is_err());
}

#[test]
fn failed_execution_is_not_a_rejected_or_accepted_result() {
    assert!(adapter(shipped())
        .consume(query("1 / 0"), &Default::default())
        .is_err());
}

#[test]
fn native_diagnostics_preserve_all_metadata_and_share_record_validation() {
    use cem_ml::diagnostics::{Diagnostic, Severity};
    use cem_ql::datatype_results::native_diagnostic_value;
    let adapter = adapter(shipped());
    let diagnostic = Diagnostic {
        code: "native.rule".into(),
        severity: Severity::Fatal,
        message: "original diagnostic".into(),
        uri: Some("original.cem".into()),
        details: Some(serde_json::json!({"retained": "metadata"})),
        source_map: Some(Default::default()),
        ..Default::default()
    };
    let native = native_diagnostic_value(diagnostic.clone());
    let context = EvaluationContext {
        policy_bindings: BTreeMap::from([("diagnostic".into(), ItemStream::once(native.clone()))]),
        ..Default::default()
    };
    let compiled = compile(
        "{ accepted: true, diagnostics: diagnostic }",
        &CompileContext {
            policy_bindings: context.policy_bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let result = adapter
        .consume(
            evaluate(&compiled, &context),
            &DiagnosticAttribution {
                uri: Some("wrong-fallback.cem".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(result.accepted, "severity does not determine acceptance");
    assert_eq!(result.diagnostics, vec![diagnostic]);
    let Item::Record(original) = result.original else {
        panic!("record")
    };
    assert_eq!(original["diagnostics"], vec![native]);
    let text = cem_ml::schema::package_sources::builtin_schema_package_source("schema")
        .unwrap()
        .schema_source
        .replace(
            "@values=\"info warning error fatal\"",
            "@values=\"info warning error\"",
        );
    let changed = DatatypeResultAdapter::new(
        Arc::new(registry(&text)),
        ContractName::new(CEM_SCHEMA_URI, "datatype-validation-result"),
        ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"),
    )
    .unwrap();
    assert!(
        changed
            .consume(evaluate(&compiled, &context), &Default::default())
            .is_err(),
        "native diagnostics obey the same schema vocabulary"
    );
}

#[test]
fn side_channel_reports_and_failed_execution_keep_their_original_diagnostics() {
    use cem_ql::datatype_results::DatatypeResultError;
    let adapter = adapter(shipped());
    let stream = query(
        r#"(report:emit("notice", "notice", "warning"), { accepted: true, diagnostics: () })"#,
    );
    let emitted = stream.diagnostics.clone();
    assert!(emitted.iter().any(|d| d.code == "notice"));
    let result = adapter.consume(stream, &Default::default()).unwrap();
    assert!(result.accepted);
    assert_eq!(result.execution_diagnostics, emitted);
    let failed = query("1 / 0");
    let expected = failed.diagnostics.clone();
    let DatatypeResultError::Execution(failed) =
        adapter.consume(failed, &Default::default()).unwrap_err()
    else {
        panic!("execution outcome")
    };
    assert_eq!(failed.diagnostics, expected);
}

#[test]
fn native_result_fields_are_read_once_for_validation_and_consumption() {
    use cem_ql::eval::{QueryItemView, QueryItemViewKind};
    use std::{
        any::Any,
        sync::atomic::{AtomicUsize, Ordering},
    };
    #[derive(Debug)]
    struct ChangingRecord(Arc<AtomicUsize>);
    impl QueryItemView for ChangingRecord {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "fixture.record"
        }
        fn identity(&self) -> String {
            "fixture".into()
        }
        fn kind(&self) -> QueryItemViewKind {
            QueryItemViewKind::Record
        }
        fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
            let first = self.0.fetch_add(1, Ordering::SeqCst) == 0;
            Some(vec![
                (
                    "accepted".into(),
                    vec![Item::Atomic(AtomValue::Boolean(first))],
                ),
                ("diagnostics".into(), vec![]),
            ])
        }
    }
    let reads = Arc::new(AtomicUsize::new(0));
    let result = adapter(shipped())
        .consume(
            ItemStream::once(Item::native(ChangingRecord(reads.clone()))),
            &Default::default(),
        )
        .unwrap();
    assert!(result.accepted, "consume the checked snapshot");
    assert_eq!(reads.load(Ordering::SeqCst), 1);
}

#[test]
fn primitive_fields_require_atomic_representation_without_node_or_record_coercion() {
    use cem_ql::eval::{QueryItemView, QueryItemViewKind};
    #[derive(Debug)]
    struct ScalarView(QueryItemViewKind, AtomValue);
    impl QueryItemView for ScalarView {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "fixture.scalar"
        }
        fn identity(&self) -> String {
            "scalar".into()
        }
        fn kind(&self) -> QueryItemViewKind {
            self.0
        }
        fn atom(&self) -> Option<AtomValue> {
            Some(self.1.clone())
        }
    }
    let contracts = registry(
        r#"@ns s = "https://cem.dev/ns/schema/1"
@default s
{schema | {value-contracts | {value-contract @name=scalar |
 {value-field @name=text @kind=string @required=true}
 {value-field @name=flag @kind=boolean @required=true}
}}}"#,
    );
    let name = ContractName::new(CEM_SCHEMA_URI, "scalar");
    for kind in [
        QueryItemViewKind::Atomic,
        QueryItemViewKind::Node,
        QueryItemViewKind::Record,
        QueryItemViewKind::Array,
    ] {
        for (field, atom) in [
            ("text", AtomValue::String("text".into())),
            ("flag", AtomValue::Boolean(true)),
        ] {
            let mut fields = BTreeMap::from([
                (
                    "text".into(),
                    vec![Item::Atomic(AtomValue::String("text".into()))],
                ),
                ("flag".into(), vec![Item::Atomic(AtomValue::Boolean(true))]),
            ]);
            fields.insert(field.into(), vec![Item::native(ScalarView(kind, atom))]);
            let checked = cem_ql::datatype_results::validate_values(
                &contracts,
                &name,
                &[Item::Record(fields)],
            );
            assert_eq!(
                checked.is_ok(),
                kind == QueryItemViewKind::Atomic,
                "{kind:?} {field}"
            );
        }
    }
}

#[test]
fn new_declaration_vocabulary_validates_under_the_shipped_metamodel() {
    use cem_ml::schema::document_model::{compile_schema_document_model, validate_document_model};
    let schema = cem_ml::schema::package_sources::builtin_schema_package_source("schema")
        .unwrap()
        .schema_source;
    let model = compile_schema_document_model(CEM_SCHEMA_URI, schema);
    assert!(
        model.compile_diagnostics.is_empty(),
        "{:?}",
        model.compile_diagnostics
    );
    let authored = source(
        r#"@ns s = "https://cem.dev/ns/schema/1"
@default s
{schema @name=example @namespace="urn:example" @version="1.0.0" | {value-contracts |
 {value-contract @name=result @allow-extra=true |
  {value-field @name=accepted @kind=boolean @required=true @cardinality=one}
 }
}}"#,
        "urn:example",
    );
    let diagnostics = validate_document_model(authored.schema.document(), &model);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn dependency_depth_is_checked_even_after_a_shared_target_was_compiled() {
    let authored = source(
        r#"@ns s = "https://cem.dev/ns/schema/1"
@default s
{schema | {value-contracts |
 {value-contract @name=a_leaf}
 {value-contract @name=b_middle | {value-field @name=child @type=a_leaf}}
 {value-contract @name=c_root | {value-field @name=child @type=b_middle}}
}}"#,
        "urn:example",
    );
    let error = ValueContracts::compile(
        &[authored],
        ValueContractLimits {
            max_depth: 1,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "depth-limit");
}
