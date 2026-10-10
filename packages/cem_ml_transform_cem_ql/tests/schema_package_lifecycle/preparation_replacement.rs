use super::*;
use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    schema::attribute_datatypes::*,
};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering},
};

#[test]
fn checked_preparation_replacement_preserves_defaults_sealed_values_and_publication() {
    let authored = authored("derived", "@default=003").replace(
        "{type @name=integer @kind=scalar}",
        "{type @name=integer @kind=scalar} {type @name=derived @base=integer}",
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let mut context = context(&authored);
    context.schema_package_compiler = Some(Arc::new(compiled_kind_with_counter(
        1,
        true,
        false,
        true,
        Some(calls.clone()),
    )));
    load(&mut context, &input());
    let active = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap()
        .clone();
    let original = tree("{sample @count=003}");
    let source = original.ast().nodes.iter().find(|n| matches!(n, CemAstNode::Attribute { expanded_name, .. } if expanded_name.local_name == "count")).unwrap();
    let values = BTreeMap::new();
    let control = OperationControl::default();
    let epoch = AttributeDatatypeContext::default();
    let args = |value| AttributeDatatypeInput {
        value,
        source,
        source_tree: Some(original.clone()),
        element_name: "sample",
        attribute_values: &values,
        control: &control,
        context: Some(&epoch),
    };
    let contract = &active.attribute_datatypes["count"];
    calls.store(0, Ordering::SeqCst);
    let issued = contract.prepare(args(AttributeDatatypeValue::Lexical("003")));
    let handle = issued.handle.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    for _ in 0..2 {
        assert_eq!(
            contract
                .validate(args(AttributeDatatypeValue::Prepared(&handle)))
                .accepted,
            Some(true)
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    set_source(
        &mut context,
        &authored.replace("@default=003", "@default=9"),
    );
    let errors = load_schema_package_manifest_into_context(&mut context, &input()).unwrap();
    assert!(errors.iter().any(|d| d.severity.is_hard_violation()));
    assert!(Arc::ptr_eq(
        active.datatype_compilation.as_ref().unwrap(),
        context
            .schema_document_models
            .resolve_for_identity(Some(SCHEMA_URI), None, None)
            .unwrap()
            .datatype_compilation
            .as_ref()
            .unwrap()
    ));
    assert_eq!(
        contract
            .validate(args(AttributeDatatypeValue::Prepared(&handle)))
            .accepted,
        Some(true)
    );
    set_source(
        &mut context,
        &authored.replace("@default=003", "@default=002"),
    );
    load(&mut context, &input());
    assert_eq!(
        contract
            .validate(args(AttributeDatatypeValue::Prepared(&handle)))
            .accepted,
        None
    );
    control.complete_scope(ROOT_EXECUTION_SCOPE_ID).unwrap();
}
