use super::*;
use cem_ml::schema::document_model::compile_schema_document_model;
use cem_ql::schema_references::SchemaHostRuntimeContextRequest;

#[test]
fn following_schema_uses_native_slot_and_never_validates_its_payload_as_application_data() {
    for expression in ["{#library}", "{$ library}", "\"library\""] {
        let input = import(&format!("@ns s = https://cem.dev/ns/schema/1\n@schema select={expression}\n{{after @selected=yes}} {{s:schema | {{elements | {{element @name=after @required-attributes=selected}}}}}}"));
        let mut host = CemQlSchemaDeclarationHost::new();
        let ctx = context(&input, "library", &elements(&input, "schema"));
        host.register_scope(input.tree.clone(), Some(ctx.clone()), policy());
        host.attach_captured_names(&input.captured).unwrap();
        // An enclosing declaration for the directive must not expose its native
        // selector child to the ordinary structural/reference walker.
        let outer = compile_schema_document_model("outer", "{schema | {elements | {element @name=@schema} {element @name=after @required-attributes=wrong}}}");
        let roots = [elements(&input, "@schema")[0], elements(&input, "after")[0]];
        let report = host
            .validate_input_runtime_host_regions(
                "selected",
                input.tree.clone(),
                &roots,
                &outer,
                policy().limits,
                |request| {
                    assert!(matches!(request, SchemaHostRuntimeContextRequest::Body(_)));
                    Some(ctx.clone())
                },
            )
            .unwrap();
        assert!(
            report.validation.complete && !report.validation.failed,
            "{expression}: {:?}; {:?}",
            report.validation.diagnostics,
            report
                .inputs
                .iter()
                .map(|i| i.region().contract.issue())
                .collect::<Vec<_>>()
        );
        assert_eq!(report.scopes.len(), 1);
        let source = &report.inputs[0].region().contract.control().unwrap().source;
        assert_eq!(
            matches!(
                source,
                cem_ml::schema::scope_controls::SchemaHostSource::LiteralSelector(_)
            ),
            expression.starts_with('"')
        );
        assert!(report.validation.references.is_empty());
        assert_original(&input);
    }
}

#[test]
fn invalid_or_unready_selectors_block_following_governance_without_inherited_fallback() {
    for expression in [
        "{$ ()}",
        "{$ (library, library)}",
        "{$ 'urn:not-schema'}",
        "{$ 7}",
        "{#missing}",
    ] {
        let input = import(&format!("@ns s = https://cem.dev/ns/schema/1\n@schema select={expression}\n{{after | {{#missing}}}} {{s:schema | {{elements | {{element @name=after}}}}}}"));
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(
            input.tree.clone(),
            Some(context(&input, "library", &elements(&input, "schema"))),
            policy(),
        );
        host.attach_captured_names(&input.captured).unwrap();
        let outer = compile_schema_document_model(
            "outer",
            "{schema | {elements | {element @name=after @required-attributes=wrong}}}",
        );
        let report = host
            .validate_input_runtime_host_regions(
                "selected",
                input.tree.clone(),
                &[elements(&input, "@schema")[0], elements(&input, "after")[0]],
                &outer,
                policy().limits,
                |_| panic!("unready selector must not request activation"),
            )
            .unwrap();
        assert!(!report.validation.complete, "{expression}");
        assert!(report.scopes.is_empty());
        assert!(report.validation.references.is_empty());
        assert!(!report
            .validation
            .diagnostics
            .iter()
            .any(|d| d.message.contains("wrong")));
        assert_original(&input);
    }
}

#[test]
fn missing_capture_and_wrong_selected_kind_do_not_infer_typed_authority() {
    for attach in [false, true] {
        let input = import("@schema select={$ library}\n{after} {ordinary | {$ missing()}}");
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(
            input.tree.clone(),
            Some(context(&input, "library", &elements(&input, "ordinary"))),
            policy(),
        );
        if attach {
            host.attach_captured_names(&input.captured).unwrap();
        }
        let outer = compile_schema_document_model(
            "outer",
            "{schema | {elements | {element @name=after @required-attributes=wrong}}}",
        );
        let report = host
            .validate_input_runtime_host_regions(
                "selected",
                input.tree.clone(),
                &[elements(&input, "@schema")[0], elements(&input, "after")[0]],
                &outer,
                policy().limits,
                |_| panic!("unready"),
            )
            .unwrap();
        assert!(!report.validation.complete);
        assert!(report.scopes.is_empty());
        assert!(host
            .compiled_source_expression(&node(&input, elements(&input, "$")[1]))
            .is_none());
    }
}

#[test]
fn nested_following_scope_restores_enclosing_schema_and_retries_original_values() {
    use cem_ml::{
        schema::declaration_references::SchemaDeclarationHost,
        value::reference_resolution::ReferenceResolutionHost,
    };
    let input = import("@ns s = https://cem.dev/ns/schema/1\n{host |\n @schema select={#library}\n {child @selected=yes}\n} {outside @outer=yes} {s:schema | {elements | {element @name=child @required-attributes=selected}}} {s:schema | {elements | {element @name=child @required-attributes=other}}}");
    let schemas = elements(&input, "schema");
    let roots = [elements(&input, "host")[0], elements(&input, "outside")[0]];
    let outer = compile_schema_document_model("outer", "{schema | {elements | {element @name=host @children=child} {element @name=child @required-attributes=wrong} {element @name=outside @required-attributes=outer}}}");
    let mut host = CemQlSchemaDeclarationHost::new();
    let scope = host.register_scope(input.tree.clone(), None, policy());
    host.attach_captured_names(&input.captured).unwrap();
    for selected in [Some(0), None, Some(1), Some(0)] {
        host.set_context(
            scope,
            selected.map(|index| context(&input, "library", &[schemas[index]])),
        );
        let report = host
            .validate_input_runtime_host_regions(
                "selected",
                input.tree.clone(),
                &roots,
                &outer,
                policy().limits,
                |_| Some(Default::default()),
            )
            .unwrap();
        assert_eq!(report.validation.complete, selected.is_some());
        assert_eq!(
            report.validation.failed,
            selected == Some(1),
            "{:?}",
            report.validation.diagnostics
        );
        assert!(!report
            .validation
            .diagnostics
            .iter()
            .any(|d| d.message.contains("wrong")));
        assert!(report
            .validation
            .nodes
            .iter()
            .any(|n| n.source.node_id() == roots[1]));
        for id in input.captured.occurrences() {
            assert_eq!(
                host.scope(&host.source_reference(node(&input, id))),
                Some(scope)
            );
        }
    }
    assert_original(&input);
}

#[test]
fn schema_preludes_respect_directed_grants_and_request_bounds() {
    let input = import("@schema select={$ library}\n{after}");
    let vendor = import(
        "@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=after}}}",
    );
    let ctx = context(&vendor, "library", &elements(&vendor, "schema"));
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(input.tree.clone(), Some(ctx), policy());
    let destination = host.register_scope(vendor.tree.clone(), Some(Default::default()), policy());
    host.attach_captured_names(&input.captured).unwrap();
    host.attach_captured_names(&vendor.captured).unwrap();
    let outer = compile_schema_document_model(
        "outer",
        "{schema | {elements | {element @name=after @required-attributes=wrong}}}",
    );
    for (grant, bounded) in [(false, false), (true, true), (true, false)] {
        if grant {
            host.allow_scope_crossing(origin, destination);
        }
        let mut limits = policy().limits;
        if bounded {
            limits.max_work = 1;
        }
        let report = host
            .validate_input_runtime_host_regions(
                "selected",
                input.tree.clone(),
                &[elements(&input, "@schema")[0], elements(&input, "after")[0]],
                &outer,
                limits,
                |_| Some(Default::default()),
            )
            .unwrap();
        assert_eq!(report.validation.complete, grant && !bounded);
        assert!(!report
            .validation
            .diagnostics
            .iter()
            .any(|d| d.message.contains("wrong")));
    }
    assert_original(&input);
}

#[test]
fn namespace_directive_payloads_are_inert_in_an_empty_application_model() {
    let input = import("@ns ui = {#missing}\n@default {$ missing()}");
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(input.tree.clone(), Some(Default::default()), policy());
    host.attach_captured_names(&input.captured).unwrap();
    let model = compile_schema_document_model("empty", "{schema}");
    assert!(model.is_empty());
    let report = host
        .validate_input_runtime_host_regions(
            "empty",
            input.tree.clone(),
            &[elements(&input, "@ns")[0], elements(&input, "@default")[0]],
            &model,
            policy().limits,
            |_| panic!("namespace selection belongs to its explicit lifecycle"),
        )
        .unwrap();
    assert!(report.validation.references.is_empty());
    assert!(input
        .captured
        .occurrences()
        .all(|id| host.compiled_source_expression(&node(&input, id)).is_none()));
}
