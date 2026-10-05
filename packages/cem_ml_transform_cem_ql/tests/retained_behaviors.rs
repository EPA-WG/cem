use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::{builder::CemAstBuilder, document::CemDocument, CemAstNode},
    schema::{
        declaration_references::SchemaDeclarationNode,
        document_model::{
            compile_schema_document_model, SchemaBehaviorEvaluator, SchemaDocumentModel,
        },
        input_references::{RetainedValidationStructure, StructuralValidationNode},
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use cem_ml_transform_cem_ql::CemQlSchemaBehaviorEvaluator;
use std::sync::Arc;
fn parse(text: &str) -> Arc<CemDocument> {
    let doc = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), text.as_bytes().to_vec()),
    )))
    .build();
    assert!(doc.diagnostics.is_empty(), "{:?}", doc.diagnostics);
    Arc::new(doc)
}
fn element(doc: &Arc<CemDocument>, name: &str) -> SchemaDeclarationNode {
    let id = doc
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == name => Some(*node_id),
            _ => None,
        })
        .unwrap();
    SchemaDeclarationNode::new(doc.clone(), id).unwrap()
}
fn make_model(param: &str, select: &str, query: &str, detail: &str) -> SchemaDocumentModel {
    let text = format!(
        r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema
{{schema @name=placement @namespace="https://example.test/placement" @version="1.0.0" |
 {{elements | {{element @name=left @children=item}}{{element @name=right @children=item}}{{element @name=item @optional-attributes=kind}}}}
 {{attributes | {{attribute @name=kind @type="schema:string"}}}}
 {{behaviors | {{behavior @name=check @implementation=function @execution=ast-validation @function=check-result @select='{select}' @match='{query}' |
 {{inputs | {{input-binding @name=candidate @type="schema:node" @source=candidate @required=true}}}}
 {{result @type="schema:diagnostic-result"}}
 {{function @name=check-result @returns=object @deterministic=true |
 {{param @name=candidate @type={param} @required=true}}
 {{body | {{$ {{message: "placement", details: {{observed: {detail}}}}} }} }}
 }} }} }}
 {{diagnostics | {{diagnostic @code="example.placement" @severity=warning @behavior=check}}}}
}}"#
    );
    let model = compile_schema_document_model("https://example.test/placement", &text);
    assert!(
        model.compile_diagnostics.is_empty(),
        "{:?}",
        model.compile_diagnostics
    );
    model
}
fn snapshot() -> (Arc<CemDocument>, Vec<StructuralValidationNode>) {
    let source = parse("{left | {#items}}{right | {#items}}");
    let library = parse("{item @kind=page}");
    let node = |source, children| StructuralValidationNode {
        source,
        children,
        declaring_schema: None,
        children_complete: true,
    };
    (
        source.clone(),
        vec![
            node(element(&source, "left"), vec![2]),
            node(element(&source, "right"), vec![3]),
            node(element(&library, "item"), vec![]),
            node(element(&library, "item"), vec![]),
        ],
    )
}
fn view<'a>(
    source: &'a Arc<CemDocument>,
    nodes: &'a [StructuralValidationNode],
    roots: &'a [usize],
    complete: bool,
) -> RetainedValidationStructure<'a> {
    RetainedValidationStructure {
        source,
        nodes,
        roots,
        complete,
    }
}
#[test]
fn native_behavior_checks_each_placement_and_extracts_consumed_parent() {
    let (source, nodes) = snapshot();
    let model=make_model("node","item","true","{parent: $candidate.parent.name, name: $candidate.name, kind: $candidate.attributes.value}");
    let report = CemQlSchemaBehaviorEvaluator
        .validate_retained_structure(view(&source, &nodes, &[0, 1], true), &model);
    assert!(report.complete);
    assert_eq!(report.diagnostics.len(), 2, "{:?}", report.diagnostics);
    for (diag, parent) in report.diagnostics.iter().zip(["left", "right"]) {
        assert_eq!(diag.code, "example.placement");
        let details = diag.details.as_ref().unwrap();
        assert_eq!(details["observed"]["parent"], parent);
        assert_eq!(details["observed"]["name"], "item");
        assert_eq!(details["observed"]["kind"], "page");
    }
    let model = make_model(
        "schema:node",
        "item",
        "candidate.parent.name == \"left\"",
        "$candidate.name",
    );
    let report = CemQlSchemaBehaviorEvaluator
        .validate_retained_structure(view(&source, &nodes, &[0, 1], true), &model);
    assert!(report.complete);
    assert_eq!(report.diagnostics.len(), 1, "{:?}", report.diagnostics);
    assert!(source
        .nodes
        .iter()
        .filter_map(|n| if let CemAstNode::Reference { targets, .. } = n {
            Some(targets)
        } else {
            None
        })
        .all(|t| t.is_none()));
}
#[test]
fn retained_object_signature_fails_while_legacy_object_stays_compatible() {
    let (source, nodes) = snapshot();
    let model = make_model("object", "item", "true", "$candidate.name");
    let report = CemQlSchemaBehaviorEvaluator
        .validate_retained_structure(view(&source, &nodes, &[0, 1], true), &model);
    assert!(report.complete);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.schema_behavior.function_failed"),
        "{:?}",
        report.diagnostics
    );
    let legacy =
        CemQlSchemaBehaviorEvaluator.validate_document(&parse("{item @kind=page}"), &model);
    assert_eq!(legacy.len(), 1, "{:?}", legacy);
    assert_eq!(legacy[0].code, "example.placement");
}
#[test]
fn diagnostic_output_cannot_implicitly_copy_native_candidate() {
    let (source, nodes) = snapshot();
    let model = make_model("node", "item", "true", "$candidate");
    let report = CemQlSchemaBehaviorEvaluator
        .validate_retained_structure(view(&source, &nodes, &[0, 1], true), &model);
    assert!(report.complete);
    assert_eq!(report.diagnostics.len(), 2);
    assert!(report
        .diagnostics
        .iter()
        .all(|d| d.code == "cem.schema_behavior.function_failed"));
}
#[test]
fn pending_and_empty_stages_and_invalid_selection_remain_distinct() {
    let (source, nodes) = snapshot();
    let model = make_model("node", "item", "true", "$candidate.name");
    assert!(
        !CemQlSchemaBehaviorEvaluator
            .validate_retained_structure(view(&source, &[], &[], false), &model)
            .complete
    );
    let empty = CemQlSchemaBehaviorEvaluator
        .validate_retained_structure(view(&source, &[], &[], true), &model);
    assert!(empty.complete && empty.diagnostics.is_empty());
    let model = make_model("node", "1", "true", "$candidate.name");
    let bad = CemQlSchemaBehaviorEvaluator
        .validate_retained_structure(view(&source, &nodes, &[0, 1], true), &model);
    assert!(bad.complete);
    assert!(
        bad.diagnostics
            .iter()
            .any(|d| d.code == "cem.schema_behavior.result_invalid"),
        "{:?}",
        bad.diagnostics
    );
}

#[test]
fn real_host_retained_behavior_obeys_readiness_grants_and_original_owners() {
    use cem_ml::{
        parser::tree::{CemTreeSemantics, RetainedCemTree},
        schema::reference_policy::ReferenceScopePolicy,
    };
    use cem_ql::{
        api::{StandaloneExpressionBinding, StandaloneExpressionContext},
        eval::{ItemStream, RetainedCemNode},
        schema_references::CemQlSchemaDeclarationHost,
    };
    let retained = |text: &str| {
        RetainedCemTree::new(
            Arc::try_unwrap(parse(text)).unwrap(),
            "fixture.cem",
            text,
            CemTreeSemantics::default(),
            None,
        )
        .unwrap()
    };
    let source = retained("{left | {#items}}{right | {#items}}");
    let first = retained("{item @kind=first}");
    let second = retained("{item @kind=second}");
    let first_node = element(first.ast_owner(), "item");
    let second_node = element(second.ast_owner(), "item");
    assert_eq!(first_node.node_id(), second_node.node_id());
    let original_maps = [first_node.node(), second_node.node()].map(|node| {
        let CemAstNode::Element { source, .. } = node else {
            panic!()
        };
        source.clone()
    });
    let targets = vec![
        RetainedCemNode::new(first.clone(), first_node.node_id())
            .unwrap()
            .query_item(),
        RetainedCemNode::new(second.clone(), second_node.node_id())
            .unwrap()
            .query_item(),
    ];
    let context = |items| {
        StandaloneExpressionContext::default().with_binding(
            "items",
            StandaloneExpressionBinding::any(ItemStream::from_items(items)),
        )
    };
    let policy = ReferenceScopePolicy::schema_defaults().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let from = host.register_scope(source.clone(), Some(context(targets)), policy.clone());
    let to_first = host.register_scope(first, None, policy.clone());
    let to_second = host.register_scope(second, None, policy.clone());
    let model = make_model(
        "node",
        "item",
        "true",
        "{parent: $candidate.parent.name, kind: $candidate.attributes.value}",
    );
    let evaluate = |host: &mut CemQlSchemaDeclarationHost| {
        host.validate_input_with_behavior_evaluator(
            source.clone(),
            &model,
            policy.limits,
            Some(&CemQlSchemaBehaviorEvaluator),
        )
        .unwrap()
    };
    let denied = evaluate(&mut host);
    assert!(!denied.complete);
    assert!(!denied
        .diagnostics
        .iter()
        .any(|d| d.code == "example.placement"));
    assert!(host.allow_scope_crossing(from, to_first));
    assert!(host.allow_scope_crossing(from, to_second));
    let selected = evaluate(&mut host);
    assert!(selected.complete, "{:?}", selected.diagnostics);
    let emitted: Vec<_> = selected
        .diagnostics
        .iter()
        .filter(|d| d.code == "example.placement")
        .collect();
    assert_eq!(emitted.len(), 4, "{:?}", selected.diagnostics);
    for ((diag, parent), kind) in emitted
        .iter()
        .zip(["left", "left", "right", "right"])
        .zip(["first", "second", "first", "second"])
    {
        let details = diag.details.as_ref().unwrap();
        assert_eq!(details["observed"]["parent"], parent);
        assert_eq!(details["observed"]["kind"], kind);
        assert_eq!(
            diag.source_map.as_ref().unwrap(),
            &original_maps[usize::from(kind == "second")]
        );
    }
    assert!(host.set_context(from, Some(context(vec![]))));
    let empty = evaluate(&mut host);
    assert!(empty.complete && !empty.failed, "{:?}", empty.diagnostics);
    assert!(!empty
        .diagnostics
        .iter()
        .any(|d| d.code == "example.placement"));
    assert!(host.set_context(from, None));
    assert!(!evaluate(&mut host).complete);
    assert!(source
        .ast_owner()
        .nodes
        .iter()
        .filter_map(|n| if let CemAstNode::Reference { targets, .. } = n {
            Some(targets)
        } else {
            None
        })
        .all(|targets| targets.is_none()));
}

#[test]
fn signature_validation_and_attribute_selection_do_not_depend_on_candidates() {
    let (source, nodes) = snapshot();
    let model = make_model("object", "item", "true", "$candidate.name");
    let empty = CemQlSchemaBehaviorEvaluator
        .validate_retained_structure(view(&source, &[], &[], true), &model);
    assert!(empty.complete);
    assert!(empty
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.schema_behavior.function_failed"));
    let model = make_model("node", "item.attributes", "true", "$candidate.name");
    let attributes = CemQlSchemaBehaviorEvaluator
        .validate_retained_structure(view(&source, &nodes, &[0, 1], true), &model);
    assert!(attributes.complete);
    assert!(attributes
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.schema_behavior.result_invalid"));
}
