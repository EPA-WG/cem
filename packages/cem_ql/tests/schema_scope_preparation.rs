use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::{
        tree::{CemTreeSemantics, RetainedCemTree},
        CemAstNode,
    },
    schema::{
        declaration_references::SchemaDeclarationNode,
        machine::{CemSchemaMachine, LexicallyScopedDocument},
        reference_policy::ReferenceScopePolicy,
        scope_references::SchemaScopeTargetError,
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
    value::reference_resolution::ReferenceResolutionIssueKind,
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{ItemStream, RetainedCemNode},
    schema_references::{
        CemQlSchemaDeclarationHost, LexicalScopeHandoffError, SchemaScopePreparationIssue,
    },
};
use std::sync::Arc;

fn capture(text: &str) -> (LexicallyScopedDocument, Arc<RetainedCemTree>) {
    let events = CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
        SourceId(1),
        text.as_bytes().to_vec(),
    )));
    let captured =
        CemSchemaMachine::new(CompiledSchema::cem_core(), events).build_with_lexical_scopes();
    assert!(captured.document().diagnostics.is_empty());
    let tree = RetainedCemTree::from_shared(
        captured.document().clone(),
        "scope.cem",
        text,
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    (captured, tree)
}
fn policy() -> ReferenceScopePolicy {
    ReferenceScopePolicy::schema_defaults().unwrap()
}
fn source(tree: &Arc<RetainedCemTree>, id: u32) -> SchemaDeclarationNode {
    SchemaDeclarationNode::new(tree.ast_owner().clone(), id).unwrap()
}
fn elements(tree: &Arc<RetainedCemTree>, name: &str) -> Vec<u32> {
    tree.ast()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == name => Some(*node_id),
            _ => None,
        })
        .collect()
}
fn context(tree: &Arc<RetainedCemTree>, targets: &[u32]) -> StandaloneExpressionContext {
    StandaloneExpressionContext::default().with_binding(
        "library",
        StandaloneExpressionBinding::any(ItemStream::from_items(
            targets
                .iter()
                .map(|id| {
                    RetainedCemNode::new(tree.clone(), *id)
                        .unwrap()
                        .query_item()
                })
                .collect(),
        )),
    )
}

#[test]
fn preparation_admits_original_wrapper_and_compiles_its_exact_declaration() {
    let (captured, tree) = capture("@ns s = https://cem.dev/ns/schema/1\n@ns cem = https://cem.dev/ns/core/1\n{s:schema | {elements | {element @name=earlier}}} {cem:schema @cem:name=chosen | {s:schema | {elements | {element @name=selected}}}} {region | {selected}} {#library}");
    let wrapper = elements(&tree, "schema")[1];
    let reference = captured.occurrences().last().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(tree.clone(), Some(context(&tree, &[wrapper])), policy());
    host.attach_captured_lexical_scopes(&captured, |_, _, _| {
        (Some(context(&tree, &[wrapper])), policy())
    })
    .unwrap();
    let prepared = host
        .prepare_schema_scope("selected", source(&tree, reference), policy().limits)
        .unwrap();
    assert!(prepared.is_ready(), "{:?}", prepared.issue);
    let target = prepared.target.as_ref().unwrap();
    assert_eq!(target.selected.node_id(), wrapper);
    assert_ne!(target.selected.node_id(), target.declaration.node_id());
    assert!(Arc::ptr_eq(
        target.declaration.document(),
        captured.document()
    ));
    assert!(prepared
        .model
        .as_ref()
        .unwrap()
        .element("selected")
        .is_some());
    assert!(prepared
        .model
        .as_ref()
        .unwrap()
        .element("earlier")
        .is_none());
    let roots = elements(&tree, "selected");
    assert_eq!(roots.len(), 1);
    let report = host
        .validate_input_roots(
            tree.clone(),
            &roots,
            prepared.model.as_ref().unwrap(),
            policy().limits,
        )
        .unwrap();
    assert!(
        report.complete && !report.failed,
        "{:?}",
        report.diagnostics
    );
    assert_eq!(report.nodes.len(), 1);
    assert_eq!(report.nodes[0].source.node_id(), roots[0]);
    assert!(Arc::ptr_eq(&report.source, captured.document()));
    assert!(matches!(
        tree.ast().get(reference),
        Some(CemAstNode::Reference { targets: None, .. })
    ));
}

#[test]
fn preparation_waits_for_explicit_context_and_names_without_fallback() {
    let text = "@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=selected}}} {#library}";
    let (captured, tree) = capture(text);
    let target = elements(&tree, "schema")[0];
    let reference = captured.occurrences().last().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    assert_eq!(
        host.attach_captured_names(&captured),
        Err(LexicalScopeHandoffError::UnregisteredOwner)
    );
    let scope = host.register_scope(tree.clone(), None, policy());
    let pending = host
        .prepare_schema_scope("selected", source(&tree, reference), policy().limits)
        .unwrap();
    assert!(!pending.is_ready());
    assert!(pending.model.is_none());
    assert!(host.set_context(scope, Some(context(&tree, &[target]))));
    let pending = host
        .prepare_schema_scope("selected", source(&tree, reference), policy().limits)
        .unwrap();
    assert_eq!(
        pending.issue,
        Some(SchemaScopePreparationIssue::TargetAdmission(
            SchemaScopeTargetError::NameNotReady
        ))
    );
    assert!(!pending.is_ready());
    assert!(pending.model.is_none());
    let (foreign, _) = capture(text);
    assert_eq!(
        host.attach_captured_names(&foreign),
        Err(LexicalScopeHandoffError::UnregisteredOwner)
    );
    host.attach_captured_names(&captured).unwrap();
    host.attach_captured_names(&captured).unwrap();
    assert!(host
        .prepare_schema_scope("selected", source(&tree, reference), policy().limits)
        .unwrap()
        .is_ready());
}

#[test]
fn preparation_rejects_cardinality_wrong_namespaces_limits_and_ungranted_crossings() {
    let (captured, tree) = capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema} {#library}\n@ns s = urn:foreign\n{s:schema}");
    let targets = elements(&tree, "schema");
    let reference = captured.occurrences().last().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(tree.clone(), Some(context(&tree, &[])), policy());
    host.attach_captured_names(&captured).unwrap();
    for chosen in [vec![], vec![targets[0], targets[0]]] {
        host.set_context(origin, Some(context(&tree, &chosen)));
        let prepared = host
            .prepare_schema_scope("selected", source(&tree, reference), policy().limits)
            .unwrap();
        assert_eq!(
            prepared.issue,
            Some(SchemaScopePreparationIssue::TargetCount(chosen.len()))
        );
        assert!(!prepared.is_ready());
        assert!(prepared.model.is_none());
    }
    host.set_context(origin, Some(context(&tree, &[targets[1]])));
    let invalid = host
        .prepare_schema_scope("selected", source(&tree, reference), policy().limits)
        .unwrap();
    assert_eq!(
        invalid.issue,
        Some(SchemaScopePreparationIssue::TargetAdmission(
            SchemaScopeTargetError::InvalidKindOrName
        ))
    );
    host.set_context(origin, Some(context(&tree, &[targets[0]])));
    let mut limits = policy().limits;
    limits.max_work = 1;
    let limited = host
        .prepare_schema_scope("selected", source(&tree, reference), limits)
        .unwrap();
    assert!(!limited.is_ready());
    assert!(limited
        .selection
        .issues
        .iter()
        .any(|issue| issue.kind == ReferenceResolutionIssueKind::WorkLimit));
    let destination = host.register_scope(tree.clone(), Some(context(&tree, &[])), policy());
    assert!(host.assign_subtree_scope(&tree, targets[0], destination));
    let denied = host
        .prepare_schema_scope("selected", source(&tree, reference), policy().limits)
        .unwrap();
    assert!(!denied.is_ready());
    assert!(denied
        .selection
        .issues
        .iter()
        .any(|issue| issue.kind == ReferenceResolutionIssueKind::ScopeDenied));
    assert!(host.allow_scope_crossing(origin, destination));
    assert!(host.set_context(destination, None));
    let pending = host
        .prepare_schema_scope("selected", source(&tree, reference), policy().limits)
        .unwrap();
    assert_eq!(
        pending.issue,
        Some(SchemaScopePreparationIssue::TargetContextNotReady)
    );
    assert!(!pending.is_ready());
    assert!(pending.target.is_some());
    assert!(pending.model.is_none());
    assert!(host.set_context(destination, Some(context(&tree, &[]))));
    assert!(host
        .prepare_schema_scope("selected", source(&tree, reference), policy().limits)
        .unwrap()
        .is_ready());
}

#[test]
fn preparation_keeps_incomplete_dependencies_and_hard_compile_errors_inspectable() {
    for body in [
        "{elements | {#missing}}",
        "{attributes | {attribute @name=value @type={#missing}}}",
        "{attributes | {attribute @name=value @type=string @pattern='['}}",
    ] {
        let text =
            format!("@ns s = https://cem.dev/ns/schema/1\n{{s:schema | {body}}} {{#library}}");
        let (captured, tree) = capture(&text);
        let target = elements(&tree, "schema")[0];
        let reference = captured.occurrences().last().unwrap();
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(tree.clone(), Some(context(&tree, &[target])), policy());
        host.attach_captured_names(&captured).unwrap();
        let prepared = host
            .prepare_schema_scope("selected", source(&tree, reference), policy().limits)
            .unwrap();
        assert!(prepared.selection.is_complete());
        assert!(!prepared.is_ready());
        assert!(prepared.target.is_some());
        let model = prepared.model.unwrap();
        if body.contains("#missing") {
            assert!(!model.is_ready_for_validation());
        } else {
            assert!(model
                .compile_diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation() && d.source_map.is_some()));
        }
    }
}

#[test]
fn prepared_child_regions_preserve_host_contracts_and_block_unready_bodies() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    use cem_ql::schema_references::SchemaInputRegion;
    let (captured, tree) = capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=child @required-attributes=inner}}} {host @own=yes | {child @inner=yes}} {sibling @own=yes} {#library}");
    let outer = compile_schema_document_model("outer", "{schema | {elements | {element @name=host @required-attributes=own @children=child} {element @name=child @required-attributes=outer} {element @name=sibling @required-attributes=own}}}");
    let target = elements(&tree, "schema")[0];
    let root = elements(&tree, "host")[0];
    let roots = [root, elements(&tree, "sibling")[0]];
    let reference = captured.occurrences().last().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let scope = host.register_scope(tree.clone(), Some(context(&tree, &[target])), policy());
    host.attach_captured_names(&captured).unwrap();
    let ready = host
        .prepare_schema_scope("child", source(&tree, reference), policy().limits)
        .unwrap();
    assert!(ready.is_ready());
    let regions = [SchemaInputRegion {
        host: source(&tree, root),
        preparation: &ready,
    }];
    let report = host
        .validate_input_regions(tree.clone(), &roots, &outer, &regions, policy().limits)
        .unwrap();
    assert!(
        report.complete && !report.failed,
        "{:?}",
        report.diagnostics
    );
    assert!(report
        .nodes
        .iter()
        .all(|node| Arc::ptr_eq(node.source.document(), tree.ast_owner())));
    host.set_context(scope, None);
    let pending = host
        .prepare_schema_scope("child", source(&tree, reference), policy().limits)
        .unwrap();
    let regions = [SchemaInputRegion {
        host: source(&tree, root),
        preparation: &pending,
    }];
    let report = host
        .validate_input_regions(tree.clone(), &roots, &outer, &regions, policy().limits)
        .unwrap();
    assert!(!report.complete);
    assert!(report.diagnostics.is_empty());
    assert!(!report.nodes[0].children_complete);
    assert!(!report
        .nodes
        .iter()
        .any(|node| node.source.node_id() == elements(&tree, "child")[0]));
    host.set_context(scope, Some(context(&tree, &[])));
    let invalid = host
        .prepare_schema_scope("child", source(&tree, reference), policy().limits)
        .unwrap();
    let regions = [SchemaInputRegion {
        host: source(&tree, root),
        preparation: &invalid,
    }];
    let report = host
        .validate_input_regions(tree.clone(), &roots, &outer, &regions, policy().limits)
        .unwrap();
    assert!(!report.complete && report.failed);
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.schema_scope.invalid_override"
            && d.source_map.as_ref().and_then(|s| s.origin()).is_some()));
    let unrelated = host
        .validate_input_regions(tree.clone(), &[roots[1]], &outer, &regions, policy().limits)
        .unwrap();
    assert!(unrelated.complete && !unrelated.failed);
    assert!(unrelated.diagnostics.is_empty());
}

#[test]
fn invalid_prepared_child_models_keep_compilation_diagnostics_without_fallback() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    use cem_ql::schema_references::SchemaInputRegion;
    for body in [
        "{elements | {#missing}}",
        "{attributes | {attribute @name=value @type=string @pattern='['}}",
    ] {
        let text = format!("@ns s = https://cem.dev/ns/schema/1\n{{s:schema | {body}}} {{host | {{#body}}}} {{#library}}");
        let (captured, tree) = capture(&text);
        let outer = compile_schema_document_model(
            "outer",
            "{schema | {elements | {element @name=host @children=child} {element @name=child}}}",
        );
        let target = elements(&tree, "schema")[0];
        let root = elements(&tree, "host")[0];
        let reference = captured.occurrences().last().unwrap();
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(tree.clone(), Some(context(&tree, &[target])), policy());
        host.attach_captured_names(&captured).unwrap();
        let prepared = host
            .prepare_schema_scope("child", source(&tree, reference), policy().limits)
            .unwrap();
        assert!(!prepared.is_ready());
        let regions = [SchemaInputRegion {
            host: source(&tree, root),
            preparation: &prepared,
        }];
        let report = host
            .validate_input_regions(tree.clone(), &[root], &outer, &regions, policy().limits)
            .unwrap();
        assert!(!report.complete);
        assert!(report.references.is_empty());
        assert!(!report.nodes[0].children_complete);
        for diagnostic in &prepared.model.as_ref().unwrap().compile_diagnostics {
            assert!(report.diagnostics.contains(diagnostic));
        }
        if body.contains("pattern") {
            assert!(report.failed);
        }
    }
}

#[test]
fn authored_host_selectors_prepare_literal_and_native_values_under_one_contract() {
    let (captured, tree) = capture("@ns c = https://cem.dev/ns/core/1\n@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=child @required-attributes=inner}}} {host @c:schema-select=library | {child @inner=yes}} {host @schema-select={#library} | {child @inner=yes}} {host @schema-select={library} | {child @inner=yes}} {host @schema-src=./external.cem}");
    let target = elements(&tree, "schema")[0];
    let roots = elements(&tree, "host");
    let mut host = CemQlSchemaDeclarationHost::new();
    let scope = host.register_scope(tree.clone(), Some(context(&tree, &[target])), policy());
    host.attach_captured_names(&captured).unwrap();
    for root in &roots[..3] {
        let prepared = host
            .prepare_schema_host_control("child", source(&tree, *root), policy().limits)
            .unwrap()
            .unwrap();
        assert!(prepared.is_ready(), "{:?}", prepared);
        let selection = prepared.preparation.as_ref().unwrap();
        assert_eq!(
            selection.target.as_ref().unwrap().declaration.node_id(),
            target
        );
        assert!(Arc::ptr_eq(
            prepared.control.attribute.document(),
            tree.ast_owner()
        ));
        // Prepared child validation is independent of the open host-control
        // attribute ownership decision; no host-attribute policy is assumed.
        let children = match tree.ast().get(*root).unwrap() {
            CemAstNode::Element { children, .. } => children.clone(),
            _ => unreachable!(),
        };
        let report = host
            .validate_input_roots(
                tree.clone(),
                &children,
                selection.model.as_ref().unwrap(),
                policy().limits,
            )
            .unwrap();
        assert!(
            report.complete && !report.failed,
            "{:?}",
            report.diagnostics
        );
    }
    let uri = host
        .prepare_schema_host_control("child", source(&tree, roots[3]), policy().limits)
        .unwrap()
        .unwrap();
    assert!(!uri.is_ready());
    assert!(uri.preparation.is_none());
    assert!(host
        .compiled_source_expression(&uri.control.attribute)
        .is_none());
    host.set_context(scope, None);
    let pending = host
        .prepare_schema_host_control("child", source(&tree, roots[0]), policy().limits)
        .unwrap()
        .unwrap();
    assert!(!pending.is_ready());
    host.set_context(scope, Some(context(&tree, &[])));
    let empty = host
        .prepare_schema_host_control("child", source(&tree, roots[0]), policy().limits)
        .unwrap()
        .unwrap();
    assert_eq!(
        empty.preparation.unwrap().issue,
        Some(SchemaScopePreparationIssue::TargetCount(0))
    );
}

#[test]
fn literal_host_selector_errors_keep_attribute_owners_and_reference_limits() {
    let (captured, tree) = capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema} {host @schema-select='missing()'} {host @schema-select=library}");
    let target = elements(&tree, "schema")[0];
    let roots = elements(&tree, "host");
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(tree.clone(), Some(context(&tree, &[target])), policy());
    host.attach_captured_names(&captured).unwrap();
    let invalid = host
        .prepare_schema_host_control("child", source(&tree, roots[0]), policy().limits)
        .unwrap()
        .unwrap();
    assert!(!invalid.is_ready());
    let selected = invalid.preparation.as_ref().unwrap();
    assert!(selected.selection.failed);
    assert!(selected
        .selection
        .diagnostics
        .iter()
        .any(
            |d| d.node.as_deref() == Some(invalid.control.attribute.identity().as_str())
                && d.source_map.as_ref().and_then(|s| s.origin()).is_some()
        ));
    let mut limits = policy().limits;
    limits.max_work = 1;
    let limited = host
        .prepare_schema_host_control("child", source(&tree, roots[1]), limits)
        .unwrap()
        .unwrap();
    assert!(!limited.is_ready());
    assert!(limited
        .preparation
        .unwrap()
        .selection
        .issues
        .iter()
        .any(|issue| issue.kind == ReferenceResolutionIssueKind::WorkLimit));
}

#[test]
fn implicit_host_selectors_share_chain_grants_cardinality_and_destination_limits() {
    let (captured, tree) = capture(
        "@ns s = https://cem.dev/ns/schema/1\n{s:schema} {host @schema-select=library} {#library}",
    );
    let target = elements(&tree, "schema")[0];
    let root = elements(&tree, "host")[0];
    let reference = captured.occurrences().last().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(tree.clone(), Some(context(&tree, &[reference])), policy());
    host.attach_captured_names(&captured).unwrap();
    let destination = host.register_scope(tree.clone(), Some(context(&tree, &[target])), policy());
    host.assign_subtree_scope(&tree, reference, destination);
    host.assign_subtree_scope(&tree, target, destination);
    let denied = host
        .prepare_schema_host_control("child", source(&tree, root), policy().limits)
        .unwrap()
        .unwrap();
    assert!(!denied.is_ready());
    assert!(denied
        .preparation
        .unwrap()
        .selection
        .issues
        .iter()
        .any(|issue| issue.kind == ReferenceResolutionIssueKind::ScopeDenied));
    host.allow_scope_crossing(origin, destination);
    let ready = host
        .prepare_schema_host_control("child", source(&tree, root), policy().limits)
        .unwrap()
        .unwrap();
    assert!(ready.is_ready(), "{:?}", ready);
    assert_eq!(
        ready
            .preparation
            .unwrap()
            .target
            .unwrap()
            .declaration
            .node_id(),
        target
    );
    host.set_context(origin, Some(context(&tree, &[target, target])));
    let multiple = host
        .prepare_schema_host_control("child", source(&tree, root), policy().limits)
        .unwrap()
        .unwrap();
    assert!(!multiple.is_ready());
    assert_eq!(
        multiple.preparation.unwrap().issue,
        Some(SchemaScopePreparationIssue::TargetCount(2))
    );
    host.set_context(origin, Some(context(&tree, &[reference])));
    let mut limited_policy = policy();
    limited_policy.limits.max_work = 1;
    let limited = host.register_scope(
        tree.clone(),
        Some(context(&tree, &[target])),
        limited_policy,
    );
    host.assign_subtree_scope(&tree, reference, limited);
    host.assign_subtree_scope(&tree, target, limited);
    host.allow_scope_crossing(origin, limited);
    let blocked = host
        .prepare_schema_host_control("child", source(&tree, root), policy().limits)
        .unwrap()
        .unwrap();
    assert!(!blocked.is_ready());
    assert!(blocked
        .preparation
        .unwrap()
        .selection
        .issues
        .iter()
        .any(|issue| issue.kind == ReferenceResolutionIssueKind::WorkLimit));
    assert!(tree.ast().nodes.iter().all(|node| !matches!(
        node,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}

#[test]
fn literal_selector_runtime_diagnostics_preserve_foreign_source_attribution() {
    use cem_ml::{
        source::ByteRange,
        source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind},
    };
    use cem_ql::native::{NativeQueryFunction, NativeQueryRequest};
    #[derive(Debug)]
    struct Fail(u8);
    impl NativeQueryFunction for Fail {
        fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
            let mut result = request.raise("fixture.selector_failure", "original runtime message");
            if self.0 == 1 {
                result.diagnostics[0].uri = Some("foreign.cem".into());
                result.diagnostics[0].node = Some("original-foreign-node".into());
                result.diagnostics[0].byte_offset = Some(700);
            } else if self.0 == 2 {
                result.diagnostics[0].uri = None;
                result.diagnostics[0].node = None;
                result.diagnostics[0].byte_offset = Some(700);
                result.diagnostics[0].source_map = Some(SourceMapStack {
                    frames: vec![SourceMapFrame {
                        source_id: SourceId(9),
                        span: FrameSpan::Single(ByteRange::new(700, 3)),
                        transform: TransformKind::Query,
                    }],
                });
            }
            result
        }
    }
    for mode in [0, 1, 2] {
        let (captured, tree) =
            capture("{host @schema-select='native:call(\"fixture.fail\", library)'}");
        let root = elements(&tree, "host")[0];
        let mut runtime = context(&tree, &[]);
        runtime
            .native_functions
            .register("fixture.fail", 1, Fail(mode))
            .unwrap();
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(tree.clone(), Some(runtime), policy());
        host.attach_captured_names(&captured).unwrap();
        let prepared = host
            .prepare_schema_host_control("child", source(&tree, root), policy().limits)
            .unwrap()
            .unwrap();
        assert!(!prepared.is_ready());
        let diagnostics = &prepared.preparation.as_ref().unwrap().selection.diagnostics;
        let diagnostic = diagnostics
            .iter()
            .find(|d| d.code == "fixture.selector_failure")
            .unwrap();
        assert_eq!(diagnostic.message, "original runtime message");
        match mode {
            0 => {
                assert_eq!(
                    diagnostic.node.as_deref(),
                    Some(prepared.control.attribute.identity().as_str())
                );
                assert_eq!(diagnostic.uri.as_deref(), Some("scope.cem"));
                assert_eq!(
                    diagnostic
                        .source_map
                        .as_ref()
                        .unwrap()
                        .origin()
                        .unwrap()
                        .source_id,
                    SourceId(1)
                );
            }
            1 => {
                assert_eq!(diagnostic.node.as_deref(), Some("original-foreign-node"));
                assert_eq!(diagnostic.uri.as_deref(), Some("foreign.cem"));
                assert_eq!(diagnostic.byte_offset, Some(700));
            }
            _ => {
                assert_eq!(diagnostic.node, None);
                assert_eq!(diagnostic.uri, None);
                assert_eq!(diagnostic.byte_offset, Some(700));
                assert_eq!(
                    diagnostic
                        .source_map
                        .as_ref()
                        .unwrap()
                        .origin()
                        .unwrap()
                        .source_id,
                    SourceId(9)
                );
            }
        }
    }
}

#[test]
fn shared_host_regions_apply_nested_models_and_restore_enclosing_contracts() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let (captured, tree) = capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=child @required-attributes=inner} {element @name=host @required-attributes=nested @children=leaf} {element @name=leaf @required-attributes=inner}}} {s:schema | {elements | {element @name=leaf @required-attributes=deep}}} {host @own=yes @schema-select={#library} | {child @inner=yes} {host @nested=yes @schema-select=library | {leaf @deep=yes}} {leaf @inner=yes}} {sibling @outer=yes}");
    let targets = elements(&tree, "schema");
    let hosts = elements(&tree, "host");
    let outer = compile_schema_document_model("outer", "{schema | {elements | {element @name=host @required-attributes=own @optional-attributes=schema-select @children='child host leaf'} {element @name=sibling @required-attributes=outer}} {attributes | {attribute @name=schema-select @type=integer}}}");
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(tree.clone(), Some(context(&tree, &[targets[0]])), policy());
    host.attach_captured_names(&captured).unwrap();
    let deep = host
        .register_lexical_scope(origin, Some(context(&tree, &[targets[1]])), policy())
        .unwrap();
    let attribute = match tree.ast().get(hosts[1]).unwrap() { CemAstNode::Element { attributes, .. } => attributes.iter().find(|id| matches!(tree.ast().get(**id), Some(CemAstNode::Attribute { expanded_name, .. }) if expanded_name.local_name == "schema-select")).copied().unwrap(), _ => unreachable!() };
    host.assign_subtree_scope(&tree, attribute, deep);
    let prepared: Vec<_> = hosts
        .iter()
        .map(|id| {
            host.prepare_schema_host_region("child", source(&tree, *id), policy().limits)
                .unwrap()
        })
        .collect();
    assert!(prepared.iter().all(|region| region.is_ready()));
    let roots = [hosts[0], elements(&tree, "sibling")[0]];
    let report = host
        .validate_input_host_regions(tree.clone(), &roots, &outer, &prepared, policy().limits)
        .unwrap();
    assert!(
        report.complete && !report.failed,
        "{:?}",
        report.diagnostics
    );
    assert!(report.references.is_empty());
    assert!(report
        .nodes
        .iter()
        .all(|node| Arc::ptr_eq(node.source.document(), tree.ast_owner())));
    assert!(report
        .nodes
        .iter()
        .all(|node| node.attribute_values.is_empty()));
    assert!(tree.ast().nodes.iter().all(|node| !matches!(
        node,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
    let discovered = host
        .validate_input_discovering_host_regions(
            "child",
            tree.clone(),
            &roots,
            &outer,
            policy().limits,
        )
        .unwrap();
    assert!(
        discovered.validation.complete && !discovered.validation.failed,
        "{:?}",
        discovered.validation.diagnostics
    );
    assert_eq!(discovered.preparations.len(), 2);
    assert!(discovered
        .preparations
        .iter()
        .all(|region| region.is_ready()));
    assert_eq!(
        discovered
            .validation
            .nodes
            .iter()
            .map(|node| node.source.node_id())
            .collect::<Vec<_>>(),
        report
            .nodes
            .iter()
            .map(|node| node.source.node_id())
            .collect::<Vec<_>>()
    );
}

#[test]
fn shared_host_regions_block_unavailable_or_invalid_controls_without_fallback() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let outer = compile_schema_document_model("outer", "{schema | {elements | {element @name=host @required-attributes=own @children=child}} {field-contracts | {field-contract @name=required @target=host @required-children=child}}}");
    for (control, targets, failed) in [
        ("@schema-select=library", false, true),
        ("@schema-src=./external.cem", true, false),
        ("@schema-select=library @schema-src=uri", true, true),
        ("@schema-select", true, true),
    ] {
        let text = format!("@ns s = https://cem.dev/ns/schema/1\n{{s:schema | {{elements | {{element @name=child}}}}}} {{host @own=yes {control} | {{#body}}}}");
        let (captured, tree) = capture(&text);
        let root = elements(&tree, "host")[0];
        let chosen = if targets {
            elements(&tree, "schema")
        } else {
            vec![]
        };
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(tree.clone(), Some(context(&tree, &chosen)), policy());
        host.attach_captured_names(&captured).unwrap();
        let prepared = host
            .prepare_schema_host_region("child", source(&tree, root), policy().limits)
            .unwrap();
        assert!(!prepared.is_ready());
        let report = host
            .validate_input_host_regions(
                tree.clone(),
                &[root],
                &outer,
                &[prepared],
                policy().limits,
            )
            .unwrap();
        assert!(!report.complete);
        assert_eq!(report.failed, failed, "{:?}", report.diagnostics);
        assert!(!report.nodes[0].children_complete);
        assert!(report.references.is_empty());
        assert!(!report
            .diagnostics
            .iter()
            .any(|d| d.code == cem_ml::schema::document_model::UNKNOWN_ATTRIBUTE_CODE));
        assert!(!report.diagnostics.iter().any(|d| d
            .details
            .as_ref()
            .and_then(|v| v.get("contract"))
            .and_then(|v| v.as_str())
            == Some("required")));
        if failed {
            assert!(report.diagnostics.iter().any(|d| d
                .source_map
                .as_ref()
                .and_then(|s| s.origin())
                .is_some()));
        }
    }
}

#[test]
fn selected_host_regions_keep_original_control_owners_and_retry_current_inputs() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let (_input_capture, input) = capture("{box | {#library}}");
    let (library_capture, library) =
        capture("{host @schema-select={#library} | {child @inner=yes}}");
    let (schema_capture, schema) = capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=child @required-attributes=inner}}}");
    let boundary = elements(&library, "host")[0];
    let schema_root = elements(&schema, "schema")[0];
    let outer = compile_schema_document_model("outer", "{schema | {elements | {element @name=box @children=host} {element @name=host @children=child}}}");
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(
        input.clone(),
        Some(context(&library, &[boundary])),
        policy(),
    );
    let destination = host.register_scope(
        library.clone(),
        Some(context(&schema, &[schema_root])),
        policy(),
    );
    let declaration = host.register_scope(schema.clone(), Some(context(&schema, &[])), policy());
    host.attach_captured_names(&library_capture).unwrap();
    host.attach_captured_names(&schema_capture).unwrap();
    host.allow_scope_crossing(origin, destination);
    host.allow_scope_crossing(destination, declaration);
    let root = elements(&input, "box")[0];
    let prepared = host
        .prepare_schema_host_region("child", source(&library, boundary), policy().limits)
        .unwrap();
    assert!(prepared.is_ready());
    let report = host
        .validate_input_host_regions(input.clone(), &[root], &outer, &[prepared], policy().limits)
        .unwrap();
    assert!(
        report.complete && !report.failed,
        "{:?}",
        report.diagnostics
    );
    assert_eq!(report.references.len(), 1);
    assert!(
        report
            .nodes
            .iter()
            .filter(|node| Arc::ptr_eq(node.source.document(), library.ast_owner()))
            .count()
            >= 2
    );
    assert!(report
        .nodes
        .iter()
        .all(|node| node.attribute_values.is_empty()));
    host.set_context(destination, None);
    let pending = host
        .prepare_schema_host_region("child", source(&library, boundary), policy().limits)
        .unwrap();
    assert!(!pending.is_ready());
    let report = host
        .validate_input_host_regions(input.clone(), &[root], &outer, &[pending], policy().limits)
        .unwrap();
    assert!(
        !report.complete && !report.failed,
        "{:?}",
        report.diagnostics
    );
    assert!(!report.nodes.iter().any(|node| matches!(node.source.node(), CemAstNode::Element { expanded_name, .. } if expanded_name.local_name == "child")));
    assert!(report.nodes.iter().any(|node| !node.children_complete));
    host.set_context(destination, Some(context(&schema, &[schema_root])));
    let retried = host
        .prepare_schema_host_region("child", source(&library, boundary), policy().limits)
        .unwrap();
    let report = host
        .validate_input_host_regions(input, &[root], &outer, &[retried], policy().limits)
        .unwrap();
    assert!(
        report.complete && !report.failed,
        "{:?}",
        report.diagnostics
    );
    assert!(library.ast().nodes.iter().all(|node| !matches!(
        node,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}

#[test]
fn pending_names_defer_classification_and_foreign_controls_remain_application_data() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let (captured, tree) = capture(
        "@ns c = https://cem.dev/ns/core/1\n{host @own=yes @c:schema-select=library | {#body}}",
    );
    let outer = compile_schema_document_model(
        "outer",
        "{schema | {elements | {element @name=host @required-attributes=own @children=child}}}",
    );
    let root = elements(&tree, "host")[0];
    let mut host = CemQlSchemaDeclarationHost::new();
    let scope = host.register_scope(tree.clone(), None, policy());
    let pending = host
        .prepare_schema_host_region("child", source(&tree, root), policy().limits)
        .unwrap();
    let report = host
        .validate_input_host_regions(tree.clone(), &[root], &outer, &[pending], policy().limits)
        .unwrap();
    assert!(
        !report.complete && !report.failed,
        "{:?}",
        report.diagnostics
    );
    assert!(report.references.is_empty());
    host.attach_captured_names(&captured).unwrap();
    host.set_context(scope, Some(context(&tree, &[])));
    let invalid = host
        .prepare_schema_host_region("child", source(&tree, root), policy().limits)
        .unwrap();
    let report = host
        .validate_input_host_regions(tree, &[root], &outer, &[invalid], policy().limits)
        .unwrap();
    assert!(!report.complete && report.failed);
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.schema_scope.invalid_override"));

    let (captured, tree) = capture("@ns c = urn:foreign\n{host @c:schema-select=wrong}");
    let outer = compile_schema_document_model("outer", "{schema | {elements | {element @name=host @optional-attributes=schema-select}} {attributes | {attribute @name=schema-select @type=integer}}}");
    let root = elements(&tree, "host")[0];
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(tree.clone(), None, policy());
    host.attach_captured_names(&captured).unwrap();
    let ordinary = host
        .prepare_schema_host_region("child", source(&tree, root), policy().limits)
        .unwrap();
    assert!(!ordinary.contract.has_override());
    let report = host
        .validate_input_host_regions(tree, &[root], &outer, &[ordinary], policy().limits)
        .unwrap();
    assert!(report.complete && report.failed);
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.code == cem_ml::schema::document_model::INVALID_ATTRIBUTE_TYPE_CODE));
}


#[test]
fn discovered_host_regions_prepare_only_entered_hosts_and_restore_models() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let text = "@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=child @required-attributes=inner}}} {host @own=yes @schema-select=library | {child @inner=yes}} {sibling @outer=yes} {host @schema-select=missing | {#missing}}";
    let (captured, tree) = capture(text);
    let boundary = elements(&tree, "host")[0];
    let outer = compile_schema_document_model("outer", "{schema | {elements | {element @name=host @required-attributes=own @children=child} {element @name=sibling @required-attributes=outer}}}");
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(
        tree.clone(),
        Some(context(&tree, &elements(&tree, "schema"))),
        policy(),
    );
    host.attach_captured_names(&captured).unwrap();
    let result = host
        .validate_input_discovering_host_regions(
            "child",
            tree.clone(),
            &[boundary, elements(&tree, "sibling")[0]],
            &outer,
            policy().limits,
        )
        .unwrap();
    assert!(
        result.validation.complete && !result.validation.failed,
        "{:?}",
        result.validation.diagnostics
    );
    assert_eq!(result.preparations.len(), 1);
    assert_eq!(result.preparations[0].contract.host().node_id(), boundary);
    assert!(result.preparations[0].is_ready());
    assert!(result.validation.references.is_empty());
    assert_eq!(
        result
            .validation
            .nodes
            .iter()
            .filter(|node| matches!(node.source.node(), CemAstNode::Element { .. }))
            .count(),
        3
    );
    assert!(result
        .validation
        .nodes
        .iter()
        .all(|node| Arc::ptr_eq(node.source.document(), tree.ast_owner())));
}

#[test]
fn discovered_selected_hosts_share_structural_walk_and_prepare_once_per_call() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let (input_capture, input) = capture("{box | {#library} {#library}}");
    let (library_capture, library) = capture("{host @schema-select=library | {child @inner=yes}}");
    let (schema_capture, schema) = capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=child @required-attributes=inner}}}");
    let boundary = elements(&library, "host")[0];
    let chosen = elements(&schema, "schema")[0];
    let outer = compile_schema_document_model("outer", "{schema | {elements | {element @name=box @children=host} {element @name=host @children=child}}}");
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(
        input.clone(),
        Some(context(&library, &[boundary])),
        policy(),
    );
    let destination =
        host.register_scope(library.clone(), Some(context(&schema, &[chosen])), policy());
    let declaration = host.register_scope(schema.clone(), Some(context(&schema, &[])), policy());
    for captured in [&input_capture, &library_capture, &schema_capture] {
        host.attach_captured_names(captured).unwrap();
    }
    host.allow_scope_crossing(origin, destination);
    host.allow_scope_crossing(destination, declaration);
    let roots = elements(&input, "box");
    let result = host
        .validate_input_discovering_host_regions(
            "child",
            input.clone(),
            &roots,
            &outer,
            policy().limits,
        )
        .unwrap();
    assert!(
        result.validation.complete && !result.validation.failed,
        "{:?}",
        result.validation.diagnostics
    );
    assert_eq!(result.preparations.len(), 1);
    assert_eq!(result.validation.references.len(), 2);
    assert!(result
        .validation
        .references
        .iter()
        .all(|reference| reference.resolution.work_used == 1 + reference.resolution.nodes.len()));
    assert_eq!(
        result
            .validation
            .nodes
            .iter()
            .filter(|node| matches!(node.source.node(), CemAstNode::Element { .. }))
            .count(),
        5
    );
    assert!(Arc::ptr_eq(
        result.preparations[0].contract.host().document(),
        library.ast_owner()
    ));
    host.set_context(destination, None);
    let pending = host
        .validate_input_discovering_host_regions(
            "child",
            input.clone(),
            &roots,
            &outer,
            policy().limits,
        )
        .unwrap();
    assert!(
        !pending.validation.complete && !pending.validation.failed,
        "{:?}",
        pending.validation.diagnostics
    );
    assert_eq!(pending.preparations.len(), 1);
    assert_eq!(
        pending
            .validation
            .nodes
            .iter()
            .filter(|node| matches!(node.source.node(), CemAstNode::Element { .. }))
            .count(),
        3
    );
    assert!(pending
        .validation
        .references
        .iter()
        .all(|reference| reference.resolution.work_used == 2));
    host.set_context(destination, Some(context(&schema, &[chosen])));
    let retried = host
        .validate_input_discovering_host_regions(
            "child",
            input.clone(),
            &roots,
            &outer,
            policy().limits,
        )
        .unwrap();
    assert!(
        retried.validation.complete && !retried.validation.failed,
        "{:?}",
        retried.validation.diagnostics
    );
    assert!(input
        .ast()
        .nodes
        .iter()
        .chain(&library.ast().nodes)
        .all(|node| !matches!(
            node,
            CemAstNode::Reference {
                targets: Some(_),
                ..
            }
        )));
}

#[test]
fn discovered_controls_block_unavailable_bodies_before_discovering_descendants() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let (captured, tree) =
        capture("{host @schema-src=external | {host @schema-select=missing | {#missing}}}");
    let outer = compile_schema_document_model(
        "outer",
        "{schema | {elements | {element @name=host @children=host}}}",
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(tree.clone(), None, policy());
    let root = elements(&tree, "host")[0];
    let pending = host
        .validate_input_discovering_host_regions(
            "child",
            tree.clone(),
            &[root],
            &outer,
            policy().limits,
        )
        .unwrap();
    assert!(
        !pending.validation.complete && !pending.validation.failed,
        "{:?}",
        pending.validation.diagnostics
    );
    assert_eq!(pending.preparations.len(), 1);
    assert!(pending.preparations[0].contract.issue().is_some());
    host.attach_captured_names(&captured).unwrap();
    let waiting = host
        .validate_input_discovering_host_regions(
            "child",
            tree.clone(),
            &[root],
            &outer,
            policy().limits,
        )
        .unwrap();
    assert!(
        !waiting.validation.complete && !waiting.validation.failed,
        "{:?}",
        waiting.validation.diagnostics
    );
    assert_eq!(waiting.preparations.len(), 1);
    assert_eq!(waiting.validation.nodes.len(), 1);
    assert!(waiting.validation.references.is_empty());
    assert!(waiting.preparations[0].contract.issue().is_none());
}

#[test]
fn discovered_overrides_work_under_empty_models_and_reject_invalid_roots_before_preparation() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let (captured, tree) = capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=child @required-attributes=inner}}} {container | {host @schema-select=library | {child}}}");
    let outer = compile_schema_document_model("outer", "{schema}");
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(
        tree.clone(),
        Some(context(&tree, &elements(&tree, "schema"))),
        policy(),
    );
    host.attach_captured_names(&captured).unwrap();
    let roots = elements(&tree, "container");
    let invalid = host
        .validate_input_discovering_host_regions(
            "child",
            tree.clone(),
            &[roots[0], roots[0]],
            &outer,
            policy().limits,
        )
        .unwrap();
    assert!(!invalid.validation.complete && invalid.validation.failed);
    assert!(invalid.preparations.is_empty());
    let result = host
        .validate_input_discovering_host_regions(
            "child",
            tree.clone(),
            &roots,
            &outer,
            policy().limits,
        )
        .unwrap();
    assert!(
        result.validation.complete && result.validation.failed,
        "{:?}",
        result.validation.diagnostics
    );
    assert_eq!(result.preparations.len(), 1);
    assert!(result.preparations[0].is_ready());
    assert_eq!(
        result
            .validation
            .nodes
            .iter()
            .filter(|node| matches!(node.source.node(), CemAstNode::Element { .. }))
            .count(),
        3
    );
    assert!(result
        .validation
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code
            == cem_ml::schema::document_model::MISSING_REQUIRED_ATTRIBUTE_CODE));
}

#[test]
fn discovered_selected_hosts_obey_structural_grants_and_remaining_work() {
    use cem_ml::schema::{
        document_model::compile_schema_document_model,
        reference_traversal::ReferenceTraversalLimits,
    };
    let (input_capture, input) = capture("{box | {#library}}");
    let (library_capture, library) =
        capture("{host @schema-src=external | {host @schema-select=missing}}");
    let boundary = elements(&library, "host")[0];
    let roots = elements(&input, "box");
    let outer = compile_schema_document_model("outer", "{schema | {elements | {element @name=box @children=host} {element @name=host @children=host}}}");
    for (granted, work, expected_preparations) in [(false, 100, 0), (true, 1, 0), (true, 2, 1)] {
        let mut host = CemQlSchemaDeclarationHost::new();
        let origin = host.register_scope(
            input.clone(),
            Some(context(&library, &[boundary])),
            policy(),
        );
        let destination = host.register_scope(library.clone(), None, policy());
        host.attach_captured_names(&input_capture).unwrap();
        host.attach_captured_names(&library_capture).unwrap();
        if granted {
            host.allow_scope_crossing(origin, destination);
        }
        let result = host
            .validate_input_discovering_host_regions(
                "child",
                input.clone(),
                &roots,
                &outer,
                ReferenceTraversalLimits {
                    max_depth: 128,
                    max_work: work,
                },
            )
            .unwrap();
        assert!(!result.validation.complete);
        assert_eq!(result.preparations.len(), expected_preparations);
        assert!(result.validation.references[0].resolution.work_used <= work);
        if !granted {
            assert!(result.validation.references[0]
                .resolution
                .issues
                .iter()
                .any(|issue| issue.kind == ReferenceResolutionIssueKind::ScopeDenied));
        }
        if work == 1 {
            assert!(result.validation.references[0]
                .resolution
                .issues
                .iter()
                .any(|issue| issue.kind == ReferenceResolutionIssueKind::WorkLimit));
        }
        if work == 2 {
            assert_eq!(result.validation.references[0].resolution.work_used, 2);
            assert!(
                !result.validation.failed,
                "{:?}",
                result.validation.diagnostics
            );
            assert!(result.preparations[0].preparation.is_none());
        }
    }
}

#[test]
fn discovered_invalid_selector_retains_diagnostics_without_body_fallback() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let (captured, tree) =
        capture("{host @schema-select=library | {host @schema-select=missing | {#missing}}}");
    let outer = compile_schema_document_model(
        "outer",
        "{schema | {elements | {element @name=host @children=host}}}",
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(tree.clone(), Some(context(&tree, &[])), policy());
    host.attach_captured_names(&captured).unwrap();
    let root = elements(&tree, "host")[0];
    let result = host
        .validate_input_discovering_host_regions(
            "child",
            tree.clone(),
            &[root],
            &outer,
            policy().limits,
        )
        .unwrap();
    assert!(!result.validation.complete && result.validation.failed);
    assert_eq!(result.preparations.len(), 1);
    assert!(result.validation.references.is_empty());
    let error = result
        .validation
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "cem.schema_scope.invalid_override")
        .unwrap();
    assert_eq!(
        error.node.as_deref(),
        Some(source(&tree, root).identity().as_str())
    );
    assert!(error.source_map.as_ref().unwrap().origin().is_some());
    assert_eq!(
        result.preparations[0].preparation.as_ref().unwrap().issue,
        Some(SchemaScopePreparationIssue::TargetCount(0))
    );
}
