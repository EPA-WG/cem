use super::*;
use cem_ml::{
    operation_control::OperationControl, value::reference_resolution::ReferenceResolutionError,
};

#[test]
fn cancelled_namespace_preparation_publication_and_activation_require_fresh_operation() {
    let input = import("@ns public = urn:provider\n@ns ui = {$ library}\n{ui:item}");
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(
        input.tree.clone(),
        Some(context(&input, "library", &[elements(&input, "@ns")[0]])),
        policy(),
    );
    host.attach_captured_namespaces(input.captured.clone())
        .unwrap();
    let declaration = node(&input, elements(&input, "@ns")[1]);
    let operation = OperationControl::default();
    host.set_operation_control(operation.clone(), operation.root_scope());
    let ready = host
        .prepare_namespace_property(declaration.clone(), policy().limits)
        .unwrap();
    assert!(ready.is_ready());
    operation.abort_signal().abort();
    assert!(host.publish_namespace_property(&ready).is_err());
    assert!(host
        .activate_namespace_properties(
            input.captured.clone(),
            &elements(&input, "item"),
            &[ready],
            |_, _, _, _| panic!("cancelled activation")
        )
        .is_err());
    assert!(matches!(
        host.prepare_namespace_property(declaration.clone(), policy().limits),
        Err(ReferenceResolutionError::OperationStopped)
    ));
    let fresh = OperationControl::default();
    host.set_operation_control(fresh.clone(), fresh.root_scope());
    assert!(host
        .prepare_namespace_property(declaration, policy().limits)
        .unwrap()
        .is_ready());
    assert_original(&input);
}

#[test]
fn cancellation_during_schema_context_handoff_restores_original_assignments() {
    use cem_ml::{
        schema::{
            declaration_references::SchemaDeclarationHost,
            document_model::compile_schema_document_model,
        },
        value::reference_resolution::ReferenceResolutionHost,
    };
    let input = import("@ns s = https://cem.dev/ns/schema/1\n@schema select={#library}\n{after} {s:schema | {elements | {element @name=after}}}");
    let mut host = CemQlSchemaDeclarationHost::new();
    let original = host.register_scope(
        input.tree.clone(),
        Some(context(&input, "library", &elements(&input, "schema"))),
        policy(),
    );
    host.attach_captured_names(&input.captured).unwrap();
    let outer =
        compile_schema_document_model("outer", "{schema | {elements | {element @name=after}}}");
    let operation = OperationControl::default();
    host.set_operation_control(operation.clone(), operation.root_scope());
    let result = host.validate_input_runtime_host_regions(
        "selected",
        input.tree.clone(),
        &[elements(&input, "@schema")[0], elements(&input, "after")[0]],
        &outer,
        policy().limits,
        |_| {
            operation.abort_signal().abort();
            Some(Default::default())
        },
    );
    assert!(matches!(
        result,
        Err(ReferenceResolutionError::OperationStopped)
    ));
    for id in input.captured.occurrences() {
        assert_eq!(
            host.scope(&host.source_reference(node(&input, id))),
            Some(original)
        );
    }
    assert_original(&input);
}

#[test]
fn namespace_cancellation_during_lexical_handoff_restores_names_and_can_retry() {
    let input = import("@ns public = urn:provider\n@ns ui = {#library}\n{ui:item}");
    let mut host = CemQlSchemaDeclarationHost::new();
    let ctx = context(&input, "library", &[elements(&input, "@ns")[0]]);
    host.register_scope(input.tree.clone(), Some(ctx.clone()), policy());
    for cancelled in [true, false] {
        let operation = OperationControl::default();
        host.set_operation_control(operation.clone(), operation.root_scope());
        let result = host.with_namespace_lifecycle(
            input.captured.clone(),
            &elements(&input, "item"),
            policy().limits,
            |_, _, _, _| {
                if cancelled {
                    operation.abort_signal().abort();
                }
                (Some(ctx.clone()), Default::default())
            },
            |_, snapshot| {
                assert!(!cancelled);
                assert!(snapshot.is_complete());
            },
        );
        assert_eq!(result.is_ok(), !cancelled);
        assert!(host
            .consuming_expanded_name(&node(&input, elements(&input, "item")[0]))
            .is_none());
    }
    assert_original(&input);
}

#[test]
fn direct_namespace_activation_checks_cancellation_before_installing_prepared_contexts() {
    use cem_ml::{
        schema::declaration_references::SchemaDeclarationHost,
        value::reference_resolution::ReferenceResolutionHost,
    };
    let input = import("@ns public = urn:provider\n@ns ui = {#library}\n{ui:item | {#library}}");
    let mut host = CemQlSchemaDeclarationHost::new();
    let ctx = context(&input, "library", &[elements(&input, "@ns")[0]]);
    let original = host.register_scope(input.tree.clone(), Some(ctx.clone()), policy());
    host.attach_captured_namespaces(input.captured.clone())
        .unwrap();
    for cancelled in [true, false] {
        let operation = OperationControl::default();
        host.set_operation_control(operation.clone(), operation.root_scope());
        let ready = host
            .prepare_namespace_property(node(&input, elements(&input, "@ns")[1]), policy().limits)
            .unwrap();
        assert!(ready.is_ready());
        let result = host.activate_namespace_properties(
            input.captured.clone(),
            &elements(&input, "item"),
            &[ready],
            |_, _, _, _| {
                if cancelled {
                    operation.abort_signal().abort();
                }
                (Some(ctx.clone()), Default::default())
            },
        );
        assert_eq!(result.is_ok(), !cancelled);
        if cancelled {
            for id in input.captured.occurrences() {
                assert_eq!(
                    host.scope(&host.source_reference(node(&input, id))),
                    Some(original)
                );
            }
        }
    }
    assert_original(&input);
}
