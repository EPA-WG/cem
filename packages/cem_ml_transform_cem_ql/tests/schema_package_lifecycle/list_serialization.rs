use super::*;

#[test]
fn list_serializer_readiness_preserves_active_package_and_retries_retained_candidate() {
    let authored = SOURCE.replace("{elements |", "{types | {type @name=item @kind=scalar} {type @name=names @kind=list @base=item}} {elements |");
    let mut context = context(&authored);
    context.schema_package_compiler = Some(Arc::new(datatype_compiler_with_serializer(
        "old",
        true,
        Some(true),
    )));
    load(&mut context, &input());
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    let active = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap()
        .clone();
    let list = active
        .datatype_compilation
        .as_ref()
        .unwrap()
        .contracts
        .iter()
        .find_map(|entry| {
            entry
                .as_any()
                .downcast_ref::<cem_ql::datatype_compilation::ExecutableDatatype>()
                .filter(|d| d.list_serializer().is_some())
        })
        .unwrap();
    let control = cem_ml::operation_control::OperationControl::default();
    let runtime = cem_ql::datatype_validation::ValidationRuntime {
        control: &control,
        scope: cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let request = cem_ql::datatype_validation::ValidationInput {
        value: vec![cem_ql::eval::Item::Atomic(cem_ql::eval::AtomValue::String("a".into())); 3],
        candidate: vec![],
        fallback: Default::default(),
    };
    assert_eq!(
        list.serialize_list(&request, &runtime, Default::default())
            .text
            .as_deref(),
        Some("a a a")
    );
    set_source(
        &mut context,
        &authored.replace("@kind=list", "@kind=list @max-items=2"),
    );
    let mut replacement = input();
    replacement.bytes = MANIFEST
        .replace("runtime-converter", "new-converter")
        .replace("old.cemt", "new.cemt")
        .into_bytes();
    context.schema_package_compiler = Some(Arc::new(datatype_compiler_with_serializer(
        "new",
        true,
        Some(false),
    )));
    load(&mut context, &replacement);
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    assert!(Arc::ptr_eq(
        active.datatype_compilation.as_ref().unwrap(),
        context
            .schema_document_models
            .resolve_for_identity(Some(SCHEMA_URI), None, None)
            .unwrap()
            .datatype_compilation
            .as_ref()
            .unwrap(),
    ));
    let candidate = context
        .schema_package_sources
        .get(SOURCE_URI)
        .unwrap()
        .clone();
    let pending = context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .datatype_compilation
        .as_ref()
        .unwrap();
    assert!(!pending.is_ready());
    assert!(pending
        .issues
        .iter()
        .any(|i| i.code == "datatype-list-serializer-unavailable"));
    assert!(pending.matches_owner(candidate.ast_owner()));
    assert_eq!(
        list.serialize_list(&request, &runtime, Default::default())
            .text
            .as_deref(),
        Some("a a a")
    );
    context.schema_package_compiler = Some(Arc::new(datatype_compiler_with_serializer(
        "new",
        true,
        Some(true),
    )));
    load(&mut context, &replacement);
    assert_active(&context, "new", "new-converter", "new.cemt");
    assert!(Arc::ptr_eq(
        &candidate,
        context.schema_package_sources.get(SOURCE_URI).unwrap()
    ));
    let ready = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap();
    let new_list = ready
        .datatype_compilation
        .as_ref()
        .unwrap()
        .contracts
        .iter()
        .find_map(|entry| {
            entry
                .as_any()
                .downcast_ref::<cem_ql::datatype_compilation::ExecutableDatatype>()
                .filter(|d| d.list_serializer().is_some())
        })
        .unwrap();
    let rejected = new_list.serialize_list(&request, &runtime, Default::default());
    assert_eq!(rejected.accepted, Some(false));
    assert!(rejected.text.is_none());
    assert!(!Arc::ptr_eq(
        list.list_serializer()
            .unwrap()
            .identity()
            .source
            .declaration()
            .document(),
        new_list
            .list_serializer()
            .unwrap()
            .identity()
            .source
            .declaration()
            .document(),
    ));
}
