use super::*;
use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    schema::{attribute_datatypes::*, package_compilation::SchemaPackageCompiler},
};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering},
};
fn attribute(input: &Arc<RetainedCemTree>) -> &CemAstNode {
    input.ast().nodes.iter().find(|n| matches!(n,CemAstNode::Attribute {expanded_name,..} if expanded_name.local_name=="count")).unwrap()
}
fn args<'a>(
    input: &'a Arc<RetainedCemTree>,
    value: AttributeDatatypeValue<'a>,
    context: &'a AttributeDatatypeContext,
    control: &'a OperationControl,
    values: &'a BTreeMap<String, String>,
) -> AttributeDatatypeInput<'a> {
    AttributeDatatypeInput {
        value,
        source: attribute(input),
        source_tree: Some(input.clone()),
        element_name: "sample",
        attribute_values: values,
        control,
        context: Some(context),
    }
}
fn compiled_model(required: bool) -> cem_ml::schema::document_model::SchemaDocumentModel {
    compiled_kind(1, true, false, required)
        .compile(&request(&authored("integer", "")))
        .unwrap()
}
#[test]
fn native_pretyped_handoff_retains_restricted_candidate_and_repeated_budget() {
    let calls = Arc::new(AtomicUsize::new(0));
    let model = compiled_kind_with_counter(1, true, false, true, Some(calls.clone()))
        .compile(&request(&authored("integer", "")))
        .unwrap();
    let contract = &model.attribute_datatypes["count"];
    let source = tree("{sample @count=003}");
    let context = AttributeDatatypeContext::default();
    let control = OperationControl::default();
    let values = BTreeMap::new();
    let prepared = contract.prepare(args(
        &source,
        AttributeDatatypeValue::Lexical("003"),
        &context,
        &control,
        &values,
    ));
    assert_eq!(prepared.accepted, None);
    let handle = prepared.handle.expect("native evidence");
    let clone = handle.clone();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let mut completed = 0;
    loop {
        let result = contract.validate(args(
            &source,
            AttributeDatatypeValue::Prepared(&clone),
            &context,
            &control,
            &values,
        ));
        if result.accepted.is_none() {
            break;
        }
        assert_eq!(result.accepted, Some(true));
        completed += 1;
        assert!(completed < 100_001);
    }
    assert!(completed > 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        contract
            .validate(args(
                &source,
                AttributeDatatypeValue::Prepared(&handle),
                &context,
                &control,
                &values
            ))
            .accepted,
        None
    );
}
#[test]
fn foreign_owner_changed_input_or_facet_context_permanently_closes_preparation() {
    let model = compiled_model(false);
    let contract = &model.attribute_datatypes["count"];
    for change in 0..4 {
        let source = tree("{sample @count=003}");
        let copy = tree("{sample @count=003}");
        let changed = tree("{sample @count=004}");
        let context = AttributeDatatypeContext::default();
        let control = OperationControl::default();
        let values = BTreeMap::new();
        let handle = contract
            .prepare(args(
                &source,
                AttributeDatatypeValue::Lexical("003"),
                &context,
                &control,
                &values,
            ))
            .handle
            .unwrap();
        let mut changed_values = BTreeMap::new();
        changed_values.insert("other".into(), "different".into());
        let target = if change == 0 {
            &copy
        } else if change == 1 {
            &changed
        } else {
            &source
        };
        let mut input = args(
            target,
            AttributeDatatypeValue::Prepared(&handle),
            &context,
            &control,
            if change == 2 {
                &changed_values
            } else {
                &values
            },
        );
        if change == 3 {
            input.element_name = "different";
        }
        assert_eq!(contract.validate(input).accepted, None);
        assert_eq!(
            contract
                .validate(args(
                    &source,
                    AttributeDatatypeValue::Prepared(&handle),
                    &context,
                    &control,
                    &values
                ))
                .accepted,
            None
        );
        let fresh = contract
            .prepare(args(
                &source,
                AttributeDatatypeValue::Lexical("003"),
                &context,
                &control,
                &values,
            ))
            .handle
            .unwrap();
        assert_eq!(
            contract
                .validate(args(
                    &source,
                    AttributeDatatypeValue::Prepared(&fresh),
                    &context,
                    &control,
                    &values
                ))
                .accepted,
            Some(true)
        );
    }
}
#[test]
fn context_grant_epoch_operation_completion_and_cancellation_expire_native_authority() {
    let model = compiled_model(false);
    let contract = &model.attribute_datatypes["count"];
    let source = tree("{sample @count=003}");
    let values = BTreeMap::new();
    for change in 0..5 {
        let mut context = AttributeDatatypeContext::default();
        let control = OperationControl::default();
        let handle = contract
            .prepare(args(
                &source,
                AttributeDatatypeValue::Lexical("003"),
                &context,
                &control,
                &values,
            ))
            .handle
            .unwrap();
        let foreign = OperationControl::with_policy(
            control.operation_id(),
            Default::default(),
            cem_ml::scheduler::ScopePolicy::host_root(),
        )
        .unwrap();
        match change {
            0 => context.advance(),
            1 => context.close(),
            2 => {
                control.cancel_root(None, None).unwrap();
            }
            3 => {
                control.complete_scope(ROOT_EXECUTION_SCOPE_ID).unwrap();
            }
            _ => {}
        }
        assert_eq!(
            contract
                .validate(args(
                    &source,
                    AttributeDatatypeValue::Prepared(&handle),
                    &context,
                    if change == 4 { &foreign } else { &control },
                    &values
                ))
                .accepted,
            None
        );
        assert_eq!(
            contract
                .validate(args(
                    &source,
                    AttributeDatatypeValue::Prepared(&handle),
                    &context,
                    &control,
                    &values
                ))
                .accepted,
            None
        );
    }
}
#[test]
fn opaque_payload_claims_and_foreign_issuers_cannot_mint_preparation_authority() {
    let model = compiled_model(false);
    let contract = &model.attribute_datatypes["count"];
    let other = compiled_model(false);
    let source = tree("{sample @count=003}");
    let context = AttributeDatatypeContext::default();
    let control = OperationControl::default();
    let values = BTreeMap::new();
    for handle in [
        NativeAttributePreparation::new("003"),
        NativeAttributePreparation::new(vec![3i64]),
    ] {
        assert_eq!(
            contract
                .validate(args(
                    &source,
                    AttributeDatatypeValue::Prepared(&handle),
                    &context,
                    &control,
                    &values
                ))
                .accepted,
            None
        );
    }
    let handle = contract
        .prepare(args(
            &source,
            AttributeDatatypeValue::Lexical("003"),
            &context,
            &control,
            &values,
        ))
        .handle
        .unwrap();
    assert_eq!(
        other.attribute_datatypes["count"]
            .validate(args(
                &source,
                AttributeDatatypeValue::Prepared(&handle),
                &context,
                &control,
                &values
            ))
            .accepted,
        None
    );
}
#[test]
fn pending_package_replacement_preserves_receipts_and_commit_retires_them() {
    let mut engine = context(&authored("integer", "@default=003"));
    engine.schema_package_compiler = Some(Arc::new(compiled(1, true)));
    load(&mut engine, &input());
    let old = engine
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap()
        .clone();
    let source = tree("{sample @count=003}");
    let scope = AttributeDatatypeContext::default();
    let control = OperationControl::default();
    let values = BTreeMap::new();
    let handle = old.attribute_datatypes["count"]
        .prepare(args(
            &source,
            AttributeDatatypeValue::Lexical("003"),
            &scope,
            &control,
            &values,
        ))
        .handle
        .unwrap();
    set_source(&mut engine, &authored("integer", "@default=002"));
    engine.schema_package_compiler = Some(Arc::new(compiled(1, false)));
    let _ = load_schema_package_manifest_into_context(&mut engine, &input()).unwrap();
    assert_eq!(
        old.attribute_datatypes["count"]
            .validate(args(
                &source,
                AttributeDatatypeValue::Prepared(&handle),
                &scope,
                &control,
                &values
            ))
            .accepted,
        Some(true)
    );
    engine.schema_package_compiler = Some(Arc::new(compiled(1, true)));
    load(&mut engine, &input());
    assert_eq!(
        old.attribute_datatypes["count"]
            .validate(args(
                &source,
                AttributeDatatypeValue::Prepared(&handle),
                &scope,
                &control,
                &values
            ))
            .accepted,
        None
    );
    let current = engine
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap();
    assert_eq!(
        current.attribute_datatypes["count"]
            .validate(args(
                &source,
                AttributeDatatypeValue::Prepared(&handle),
                &scope,
                &control,
                &values
            ))
            .accepted,
        None
    );
    let fresh = current.attribute_datatypes["count"]
        .prepare(args(
            &source,
            AttributeDatatypeValue::Lexical("003"),
            &scope,
            &control,
            &values,
        ))
        .handle
        .unwrap();
    assert_eq!(
        current.attribute_datatypes["count"]
            .validate(args(
                &source,
                AttributeDatatypeValue::Prepared(&fresh),
                &scope,
                &control,
                &values
            ))
            .accepted,
        Some(true)
    );
    assert!(old.attribute_datatypes["count"]
        .prepare(args(
            &source,
            AttributeDatatypeValue::Lexical("003"),
            &scope,
            &control,
            &values
        ))
        .handle
        .is_none());
}

#[test]
fn source_less_optional_input_never_acquires_candidate_access_and_context_drop_expires_it() {
    let source = tree("{sample @count=003}");
    let control = OperationControl::default();
    let values = BTreeMap::new();
    for required in [false, true] {
        let model = compiled_model(required);
        let contract = &model.attribute_datatypes["count"];
        let context = AttributeDatatypeContext::default();
        let mut input = args(
            &source,
            AttributeDatatypeValue::Lexical("003"),
            &context,
            &control,
            &values,
        );
        input.source_tree = None;
        let result = contract.prepare(input);
        if required {
            assert!(result.handle.is_none());
            continue;
        }
        let handle = result.handle.unwrap();
        let mut input = args(
            &source,
            AttributeDatatypeValue::Prepared(&handle),
            &context,
            &control,
            &values,
        );
        input.source_tree = None;
        assert_eq!(contract.validate(input).accepted, Some(true));
        assert_eq!(
            contract
                .validate(args(
                    &source,
                    AttributeDatatypeValue::Prepared(&handle),
                    &context,
                    &control,
                    &values
                ))
                .accepted,
            None
        );
    }
    let model = compiled_model(false);
    let contract = &model.attribute_datatypes["count"];
    let handle = {
        let context = AttributeDatatypeContext::default();
        contract
            .prepare(args(
                &source,
                AttributeDatatypeValue::Lexical("003"),
                &context,
                &control,
                &values,
            ))
            .handle
            .unwrap()
    };
    let context = AttributeDatatypeContext::default();
    assert_eq!(
        contract
            .validate(args(
                &source,
                AttributeDatatypeValue::Prepared(&handle),
                &context,
                &control,
                &values
            ))
            .accepted,
        None
    );
}
#[test]
fn native_defaults_keep_original_owner_and_reload_requires_fresh_preparation() {
    let model = compiled(1, true)
        .compile(&request(&authored("integer", "@default=003")))
        .unwrap();
    let contract = &model.attribute_datatypes["count"];
    let default_tree = RetainedCemTree::from_shared(
        contract.declaration().document().clone(),
        SOURCE_URI,
        "",
        Default::default(),
        None,
    )
    .unwrap();
    let default=default_tree.ast().nodes.iter().find(|node|matches!(node,CemAstNode::Attribute {expanded_name,..} if expanded_name.local_name=="default")).unwrap();
    let scope = AttributeDatatypeContext::default();
    let control = OperationControl::default();
    let values = BTreeMap::new();
    let default_input = |value| AttributeDatatypeInput {
        value,
        source: default,
        source_tree: Some(default_tree.clone()),
        element_name: "sample",
        attribute_values: &values,
        control: &control,
        context: Some(&scope),
    };
    let handle = contract
        .prepare(default_input(AttributeDatatypeValue::Lexical("003")))
        .handle
        .unwrap();
    assert_eq!(
        contract
            .validate(default_input(AttributeDatatypeValue::Prepared(&handle)))
            .accepted,
        Some(true)
    );
    let source = tree("{sample @count=003}");
    let handle = contract
        .prepare(args(
            &source,
            AttributeDatatypeValue::Lexical("003"),
            &scope,
            &control,
            &values,
        ))
        .handle
        .unwrap();
    let bytes = cem_ml::ast::encode::DebugBinaryEncoder::new().encode(source.ast());
    let decoded = cem_ml::ast::decode::DebugBinaryDecoder::new()
        .decode(&bytes.bytes)
        .unwrap();
    let reloaded = RetainedCemTree::from_shared(
        Arc::new(decoded),
        "reload.cem",
        "",
        Default::default(),
        None,
    )
    .unwrap();
    assert_eq!(
        contract
            .validate(args(
                &reloaded,
                AttributeDatatypeValue::Prepared(&handle),
                &scope,
                &control,
                &values
            ))
            .accepted,
        None
    );
    let fresh = contract
        .prepare(args(
            &reloaded,
            AttributeDatatypeValue::Lexical("003"),
            &scope,
            &control,
            &values,
        ))
        .handle
        .unwrap();
    assert_eq!(
        contract
            .validate(args(
                &reloaded,
                AttributeDatatypeValue::Prepared(&fresh),
                &scope,
                &control,
                &values
            ))
            .accepted,
        Some(true)
    );
}

#[test]
fn callback_context_or_publication_changes_prevent_handle_publication() {
    for retire in [false, true] {
        let scope = AttributeDatatypeContext::default();
        let captured_scope = scope.clone();
        let target = Arc::new(std::sync::Mutex::new(
            None::<std::sync::Weak<dyn CompiledAttributeDatatype>>,
        ));
        let captured_target = target.clone();
        let hook = Arc::new(move || {
            if retire {
                captured_target
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .upgrade()
                    .unwrap()
                    .retire_preparations();
            } else {
                captured_scope.close();
            }
        });
        let model = compiled_kind_with_hooks(1, true, false, true, None, Some(hook))
            .compile(&request(&authored("integer", "")))
            .unwrap();
        let contract = &model.attribute_datatypes["count"];
        *target.lock().unwrap() = Some(Arc::downgrade(contract));
        let source = tree("{sample @count=003}");
        let control = OperationControl::default();
        let values = BTreeMap::new();
        let result = contract.prepare(args(
            &source,
            AttributeDatatypeValue::Lexical("003"),
            &scope,
            &control,
            &values,
        ));
        assert!(result.handle.is_none());
        assert_eq!(result.accepted, None);
    }
}
#[test]
fn core_context_and_source_metadata_bounds_precede_callbacks_and_cloning() {
    let calls = Arc::new(AtomicUsize::new(0));
    let model = compiled_kind_with_counter(1, true, false, true, Some(calls.clone()))
        .compile(&request(&authored("integer", "")))
        .unwrap();
    let contract = &model.attribute_datatypes["count"];
    let scope = AttributeDatatypeContext::default();
    let control = OperationControl::default();
    let source = tree("{sample @count=003}");
    let mut values = BTreeMap::new();
    values.insert(
        "large".into(),
        "x".repeat(cem_ql::preparation_evidence::MAX_EVIDENCE_METADATA_BYTES),
    );
    assert!(contract
        .prepare(args(
            &source,
            AttributeDatatypeValue::Lexical("003"),
            &scope,
            &control,
            &values
        ))
        .handle
        .is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let mut document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(SourceId(1), b"{sample @count=003}".to_vec())))).build();
    let node=document.nodes.iter_mut().find(|node|matches!(node,CemAstNode::Attribute {expanded_name,..} if expanded_name.local_name=="count")).unwrap();
    let CemAstNode::Attribute { source: map, .. } = node else {
        panic!()
    };
    map.frames[0].transform = cem_ml::source_map::TransformKind::TemplateTransform {
        function: "x".repeat(cem_ql::preparation_evidence::MAX_EVIDENCE_METADATA_BYTES),
    };
    let huge = RetainedCemTree::from_shared(
        Arc::new(document),
        "large.cem",
        "",
        Default::default(),
        None,
    )
    .unwrap();
    let values = BTreeMap::new();
    let result = contract.prepare(args(
        &huge,
        AttributeDatatypeValue::Lexical("003"),
        &scope,
        &control,
        &values,
    ));
    assert!(result.handle.is_none());
    assert!(result.diagnostics.iter().all(|d| d.source_map.is_none()));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
