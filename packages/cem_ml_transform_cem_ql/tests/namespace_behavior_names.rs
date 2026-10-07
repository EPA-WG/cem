use cem_ml::{
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    parser::CemAstNode,
    schema::{
        declaration_references::SchemaDeclarationNode,
        document_model::{
            compile_schema_document_model, SchemaBehaviorEvaluator, SchemaDocumentModel,
        },
        input_references::validate_structural_input_roots_references,
        namespace_references::{admit_namespace_scope_target, NamespaceNameCompletion},
        reference_policy::ReferenceScopePolicy,
        vocab::CompiledSchema,
    },
};
use cem_ml_transform_cem_ql::CemQlSchemaBehaviorEvaluator;
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{AtomValue, Item, ItemStream, RetainedCemNode},
    schema_references::CemQlSchemaDeclarationHost,
    validation_structure::{RetainedValidationQueryTree, ValidationPlacementNode},
};
use std::{collections::BTreeMap, sync::Arc};
fn import(text: &str) -> ScopedCemImport {
    import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "behavior.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
}
fn ids(input: &ScopedCemImport, name: &str) -> Vec<u32> {
    input
        .tree
        .ast()
        .nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == name => Some(*node_id),
            _ => None,
        })
        .collect()
}
fn source(input: &ScopedCemImport, id: u32) -> SchemaDeclarationNode {
    SchemaDeclarationNode::new(input.tree.ast_owner().clone(), id).unwrap()
}
fn names(input: &ScopedCemImport, roots: &[u32], uri: &str) -> Arc<NamespaceNameCompletion> {
    let vendor = import(&format!("@ns public = {uri}\n"));
    let target =
        admit_namespace_scope_target(source(&vendor, ids(&vendor, "@ns")[0]), &vendor.captured)
            .unwrap();
    let property = input
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Attribute {
                node_id,
                expanded_name,
                ..
            } if expanded_name.namespace_uri == "xmlns" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    Arc::new(
        NamespaceNameCompletion::new(
            input.captured.clone(),
            roots,
            BTreeMap::from([(property, target)]),
        )
        .unwrap(),
    )
}
fn context(input: &ScopedCemImport, name: &str, targets: &[u32]) -> StandaloneExpressionContext {
    StandaloneExpressionContext::default().with_binding(
        name,
        StandaloneExpressionBinding::any(ItemStream::from_items(
            targets
                .iter()
                .map(|id| {
                    RetainedCemNode::new(input.tree.clone(), *id)
                        .unwrap()
                        .query_item()
                })
                .collect(),
        )),
    )
}
fn model_text() -> &'static str {
    r#"{schema |
        {elements | {element @name=left @children=item} {element @name=right @children=item} {element @name=item @optional-attributes='kind target'}}
        {attributes | {attribute @name=kind @type=string} {attribute @name=target @type=node}}
        {behaviors | {behavior @name=check @implementation=function @execution=ast-validation @function=result @select=item @match=true |
            {inputs | {input-binding @name=candidate @type=node @source=candidate @required=true}}
            {result @type=schema:diagnostic-result}
            {function @name=result @returns=object | {param @name=candidate @type=node @required=true}
                {body | {$ {message: "names", details: {observed: {namespace: $candidate.namespace, attribute: $candidate.attributes.namespace, parent: $candidate.parent.name, source: $candidate.source.namespace}}}}}
            }
        }}
        {diagnostics | {diagnostic @code=fixture.names @severity=warning @behavior=check}}
    }"#
}
fn model() -> SchemaDocumentModel {
    let model = compile_schema_document_model("consumer", model_text());
    assert!(
        model.compile_diagnostics.is_empty(),
        "{:?}",
        model.compile_diagnostics
    );
    assert!(model.diagnostic_behaviors.contains_key("fixture.names"));
    model
}
fn field(item: &Item, name: &str) -> Vec<Item> {
    item.view().unwrap().field(name).unwrap()
}
fn string(item: &Item, name: &str) -> String {
    let values = field(item, name);
    match &values[0] {
        Item::Atomic(AtomValue::String(value)) => value.clone(),
        other => panic!("{other:?}"),
    }
}
#[test]
fn completed_behavior_snapshots_survive_restoration_and_keep_authored_sources() {
    let input = import("{left | {#items}}{right | {#items}}");
    let data = import("{container @xmlns:p={#namespace} | {p:item @p:kind=page @target={#values}} {p:payload @p:label=ready | {#missing}} {p:outside}}");
    let item = ids(&data, "item")[0];
    let payload = ids(&data, "payload")[0];
    let model = model();
    let policy = ReferenceScopePolicy::schema_defaults().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let request = host.register_scope(
        input.tree.clone(),
        Some(context(&data, "items", &[item])),
        policy.clone(),
    );
    let destination = host.register_scope(
        data.tree.clone(),
        Some(context(&data, "values", &[payload])),
        policy.clone(),
    );
    host.attach_captured_names(&input.captured).unwrap();
    host.attach_captured_names(&data.captured).unwrap();
    host.allow_scope_crossing(request, destination);
    let roots = [ids(&input, "left")[0], ids(&input, "right")[0]];
    let mut reports = vec![];
    for uri in ["urn:first", "urn:second"] {
        let report = host
            .with_completed_namespace_names(names(&data, &[item, payload], uri), |host| {
                validate_structural_input_roots_references(
                    input.tree.ast_owner().clone(),
                    &roots,
                    &model,
                    host,
                    policy.limits,
                )
                .unwrap()
            })
            .unwrap();
        assert!(
            report.complete && !report.failed,
            "{:?}",
            report.diagnostics
        );
        assert!(host.consuming_expanded_name(&source(&data, item)).is_none());
        reports.push(report);
    }
    drop(host);
    for (report, uri) in reports.iter().zip(["urn:first", "urn:second"]) {
        let tree = RetainedValidationQueryTree::new(report.structure()).unwrap();
        let placements: Vec<_> = report
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| {
                n.source.node_id() == item
                    && Arc::ptr_eq(n.source.document(), data.tree.ast_owner())
            })
            .map(|(index, _)| tree.node(index).unwrap())
            .collect();
        assert_eq!(placements.len(), 2);
        assert_ne!(
            placements[0].view().unwrap().identity(),
            placements[1].view().unwrap().identity()
        );
        for placement in &placements {
            assert_eq!(string(placement, "namespace"), uri);
            let original = field(placement, "source").remove(0);
            assert_eq!(string(&original, "namespace"), "p");
            assert!(Arc::ptr_eq(
                cem_ql::eval::retained_cem_node(&original)
                    .unwrap()
                    .owner()
                    .ast_owner(),
                data.tree.ast_owner()
            ));
            assert_eq!(placement.source_map(), original.source_map());
            let provenance = placement.view().unwrap().provenance().unwrap();
            assert_eq!(provenance.source_uri.as_deref(), Some("behavior.cem"));
            assert_eq!(provenance.line_number, data.tree.source_line_number(item));
            let attributes = field(placement, "attributes");
            let kind = attributes
                .iter()
                .find(|a| string(a, "name") == "kind")
                .unwrap();
            assert_eq!(string(kind, "namespace"), uri);
            assert_eq!(string(kind, "value"), "page");
            let target = attributes
                .iter()
                .find(|a| string(a, "name") == "target")
                .unwrap();
            let value = field(target, "value").remove(0);
            assert_eq!(string(&value, "namespace"), uri);
            assert_eq!(string(&field(&value, "attributes")[0], "namespace"), uri);
            assert_eq!(string(&field(&value, "source")[0], "namespace"), "p");
            let authored = field(target, "source").remove(0);
            assert_eq!(
                string(&field(&authored, "valueNodes")[0], "kind"),
                "reference"
            );
            assert!(field(&value, "children")
                .iter()
                .any(|n| string(n, "kind") == "reference"));
            assert_eq!(field(&value, "parent").len(), 0);
            assert_eq!(
                placement
                    .view()
                    .unwrap()
                    .downcast_ref::<ValidationPlacementNode>()
                    .unwrap()
                    .source_node()
                    .node_id(),
                item
            );
        }
        let behavior =
            CemQlSchemaBehaviorEvaluator.validate_retained_structure(report.structure(), &model);
        assert!(behavior.complete);
        assert_eq!(behavior.diagnostics.len(), 2, "{:?}", behavior.diagnostics);
        for (diagnostic, parent) in behavior.diagnostics.iter().zip(["left", "right"]) {
            assert_eq!(diagnostic.code, "fixture.names", "{diagnostic:?}");
            let observed = &diagnostic.details.as_ref().unwrap()["observed"];
            assert_eq!(observed["namespace"], uri);
            assert_eq!(observed["attribute"], serde_json::json!([uri, ""]));
            assert_eq!(observed["source"], "p");
            assert_eq!(observed["parent"], parent);
        }
    }
    assert!(data.tree.ast().nodes.iter().all(|n| !matches!(
        n,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}

#[test]
fn target_subtree_pending_names_defer_behavior_without_resolving_descendant_links() {
    let input = import("{left | {#items}}");
    let data=import("{container @xmlns:p={#namespace} | {p:item @p:kind=page @target={#values}} {p:payload | {#missing}}}");
    let item = ids(&data, "item")[0];
    let payload = ids(&data, "payload")[0];
    let model = model();
    let policy = ReferenceScopePolicy::schema_defaults().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let request = host.register_scope(
        input.tree.clone(),
        Some(context(&data, "items", &[item])),
        policy.clone(),
    );
    let destination = host.register_scope(
        data.tree.clone(),
        Some(context(&data, "values", &[payload])),
        policy.clone(),
    );
    host.attach_captured_names(&input.captured).unwrap();
    host.attach_captured_names(&data.captured).unwrap();
    host.allow_scope_crossing(request, destination);
    for (selected, ready) in [(vec![item], false), (vec![item, payload], true)] {
        let report = host
            .with_completed_namespace_names(names(&data, &selected, "urn:ready"), |host| {
                validate_structural_input_roots_references(
                    input.tree.ast_owner().clone(),
                    &ids(&input, "left"),
                    &model,
                    host,
                    policy.limits,
                )
                .unwrap()
            })
            .unwrap();
        assert_eq!(report.complete, ready, "{:?}", report.diagnostics);
        assert!(!report.failed);
        assert_eq!(
            RetainedValidationQueryTree::new(report.structure()).is_ok(),
            ready
        );
        let behavior =
            CemQlSchemaBehaviorEvaluator.validate_retained_structure(report.structure(), &model);
        assert_eq!(behavior.complete, ready);
        assert_eq!(behavior.diagnostics.len(), usize::from(ready));
    }
}

#[test]
fn completed_names_follow_repeated_placements_across_consuming_models() {
    let declaration = model_text()
        .replace("fixture.names", "fixture.child")
        .replacen("{schema |", "{s:schema |", 1);
    let input = import(&format!("@ns s = https://cem.dev/ns/schema/1\n{declaration} {{left @schema-select=library | {{#items}}}} {{right | {{#items}}}}"));
    let data = import("{container @xmlns:p={#namespace} | {p:item @p:kind=page}}");
    let item = ids(&data, "item")[0];
    let schema = ids(&input, "schema")[0];
    let policy = ReferenceScopePolicy::schema_defaults().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let inputs = context(&input, "library", &[schema]).with_binding(
        "items",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(data.tree.clone(), item)
                .unwrap()
                .query_item(),
        )),
    );
    let origin = host.register_scope(input.tree.clone(), Some(inputs.clone()), policy.clone());
    let target = host.register_scope(
        data.tree.clone(),
        Some(StandaloneExpressionContext::default()),
        policy.clone(),
    );
    host.allow_scope_crossing(origin, target);
    host.attach_captured_names(&input.captured).unwrap();
    host.attach_captured_names(&data.captured).unwrap();
    let report = host
        .with_completed_namespace_names(names(&data, &[item], "urn:execution"), |host| {
            host.validate_input_runtime_host_regions_with_behavior_evaluator(
                "child",
                input.tree.clone(),
                &[ids(&input, "left")[0], ids(&input, "right")[0]],
                &model(),
                policy.limits,
                |_| Some(inputs.clone()),
                Some(&CemQlSchemaBehaviorEvaluator),
            )
            .unwrap()
        })
        .unwrap();
    assert!(
        report.validation.complete && !report.validation.failed,
        "{:?}",
        report.validation.diagnostics
    );
    let diagnostics = &report.validation.diagnostics;
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    for (code, parent) in [("fixture.names", "right"), ("fixture.child", "left")] {
        let diagnostic = diagnostics.iter().find(|d| d.code == code).unwrap();
        let observed = &diagnostic.details.as_ref().unwrap()["observed"];
        assert_eq!(observed["namespace"], "urn:execution");
        assert_eq!(observed["attribute"], "urn:execution");
        assert_eq!(observed["source"], "p");
        assert_eq!(observed["parent"], parent);
        let item = RetainedCemNode::new(data.tree.clone(), item)
            .unwrap()
            .query_item();
        assert_eq!(diagnostic.source_map, item.source_map());
        assert!(diagnostic.byte_offset.is_some());
    }
    assert!(host.consuming_expanded_name(&source(&data, item)).is_none());
}
