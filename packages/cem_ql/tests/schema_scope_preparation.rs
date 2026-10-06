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
