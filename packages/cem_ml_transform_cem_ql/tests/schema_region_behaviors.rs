use cem_ml::{
    diagnostics::Diagnostic,
    events::cem::CemEventNormalizer,
    parser::{
        document::CemDocument,
        tree::{CemTreeSemantics, RetainedCemTree},
        CemAstNode,
    },
    schema::{
        document_model::{
            compile_schema_document_model, SchemaBehaviorEvaluator, SchemaDocumentModel,
        },
        input_references::{RetainedBehaviorRegion, RetainedBehaviorValidation},
        machine::{CemSchemaMachine, LexicallyScopedDocument},
        reference_policy::ReferenceScopePolicy,
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use cem_ml_transform_cem_ql::CemQlSchemaBehaviorEvaluator;
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{ItemStream, RetainedCemNode},
    schema_references::CemQlSchemaDeclarationHost,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

fn capture(text: &str) -> (LexicallyScopedDocument, Arc<RetainedCemTree>) {
    let events = CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
        SourceId(1),
        text.as_bytes().to_vec(),
    )));
    let captured =
        CemSchemaMachine::new(CompiledSchema::cem_core(), events).build_with_lexical_scopes();
    assert!(
        captured.document().diagnostics.is_empty(),
        "{:?}",
        captured.document().diagnostics
    );
    let tree = RetainedCemTree::from_shared(
        captured.document().clone(),
        "fixture.cem",
        text,
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    (captured, tree)
}
fn elements(tree: &RetainedCemTree, name: &str) -> Vec<u32> {
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
fn context(
    schema: &Arc<RetainedCemTree>,
    chosen: u32,
    data: &Arc<RetainedCemTree>,
    items: &[u32],
) -> StandaloneExpressionContext {
    StandaloneExpressionContext::default()
        .with_binding(
            "library",
            StandaloneExpressionBinding::any(ItemStream::once(
                RetainedCemNode::new(schema.clone(), chosen)
                    .unwrap()
                    .query_item(),
            )),
        )
        .with_binding(
            "items",
            StandaloneExpressionBinding::any(ItemStream::from_items(
                items
                    .iter()
                    .map(|id| {
                        RetainedCemNode::new(data.clone(), *id)
                            .unwrap()
                            .query_item()
                    })
                    .collect(),
            )),
        )
}
fn behavior(code: &str, select: &str, observed: &str) -> String {
    format!(
        r#"{{behaviors | {{behavior @name=check @implementation=function @execution=ast-validation @function=result @select='{select}' @match=true |
        {{inputs | {{input-binding @name=candidate @type=schema:node @source=candidate @required=true}}}}
        {{result @type=schema:diagnostic-result}}
        {{function @name=result @returns=object | {{param @name=candidate @type=node @required=true}} {{body | {{$ {{message: "region", details: {{observed: {observed}}}}} }} }} }}
    }} }} {{diagnostics | {{diagnostic @code={code} @severity=warning @behavior=check}}}}"#
    )
}
fn policy() -> ReferenceScopePolicy {
    ReferenceScopePolicy::schema_defaults().unwrap()
}

#[test]
fn consuming_models_route_repeated_original_targets_to_distinct_native_behaviors() {
    let inner = behavior(
        "fixture.inner",
        "nodes",
        "{name: $candidate.name, parent: $candidate.parent.name}",
    );
    let (captured,tree)=capture(&format!("@ns s = https://cem.dev/ns/schema/1\n{{s:schema | {{elements | {{element @name=child @children=item}} {{element @name=item}}}} {inner}}} {{host @schema-select=library | {{child | {{#items}}}}}} {{sibling | {{#items}}}}"));
    let (data_capture, data) = capture("{schema | {item}}");
    let item = elements(&data, "item")[0];
    let selected = elements(&tree, "schema")[0];
    let outer=compile_schema_document_model("outer",&format!("{{schema | {{elements | {{element @name=host @children=child}} {{element @name=sibling @children=item}} {{element @name=item}}}} {} }}",behavior("fixture.outer","nodes","{name: $candidate.name, parent: $candidate.parent.name}")));
    assert!(
        outer.compile_diagnostics.is_empty(),
        "{:?}",
        outer.compile_diagnostics
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(
        tree.clone(),
        Some(context(&tree, selected, &data, &[item])),
        policy(),
    );
    let target = host.register_scope(
        data.clone(),
        Some(StandaloneExpressionContext::default()),
        policy(),
    );
    host.attach_captured_names(&captured).unwrap();
    host.attach_captured_names(&data_capture).unwrap();
    host.allow_scope_crossing(origin, target);
    let mut inputs = 0;
    let roots = [elements(&tree, "host")[0], elements(&tree, "sibling")[0]];
    let report = host
        .validate_input_runtime_host_regions_with_behavior_evaluator(
            "child",
            tree.clone(),
            &roots,
            &outer,
            policy().limits,
            |_| {
                inputs += 1;
                Some(context(&tree, selected, &data, &[item]))
            },
            Some(&CemQlSchemaBehaviorEvaluator),
        )
        .unwrap();
    assert!(
        report.validation.complete && !report.validation.failed,
        "{:?}",
        report.validation.diagnostics
    );
    assert_eq!(inputs, 1);
    let diagnostics = &report.validation.diagnostics;
    assert_eq!(diagnostics.len(), 5, "{diagnostics:?}");
    let observed: BTreeMap<_, _> = diagnostics
        .iter()
        .map(|d| {
            let details = d.details.as_ref().unwrap();
            (
                (
                    details["observed"]["name"].as_str().unwrap(),
                    details["observed"]["parent"].as_str().unwrap_or("root"),
                ),
                d.code.as_str(),
            )
        })
        .collect();
    assert_eq!(observed[&("host", "root")], "fixture.outer");
    assert_eq!(observed[&("sibling", "root")], "fixture.outer");
    assert_eq!(observed[&("child", "host")], "fixture.inner");
    assert_eq!(observed[&("item", "child")], "fixture.inner");
    assert_eq!(observed[&("item", "sibling")], "fixture.outer");
    let repeated: Vec<_> = report
        .validation
        .nodes
        .iter()
        .filter(|node| {
            Arc::ptr_eq(node.source.document(), data.ast_owner()) && node.source.node_id() == item
        })
        .collect();
    assert_eq!(repeated.len(), 2);
    for node in repeated {
        assert_eq!(
            node.declaring_schema.as_ref().unwrap().node_id(),
            elements(&data, "schema")[0]
        );
    }
    assert!(report
        .validation
        .references
        .iter()
        .all(|r| r.resolution.work_used == 2));
    assert!(tree.ast().nodes.iter().all(|node| !matches!(
        node,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}

#[derive(Debug, Default)]
struct RecordingEvaluator {
    compiled: Mutex<Vec<String>>,
    domains: Mutex<Vec<(String, Vec<usize>)>>,
}
impl SchemaBehaviorEvaluator for RecordingEvaluator {
    fn compile_model(&self, model: &SchemaDocumentModel) -> Vec<Diagnostic> {
        self.compiled.lock().unwrap().push(model.schema_uri.clone());
        CemQlSchemaBehaviorEvaluator.compile_model(model)
    }
    fn validate_retained_region(
        &self,
        region: RetainedBehaviorRegion<'_>,
        model: &SchemaDocumentModel,
    ) -> RetainedBehaviorValidation {
        self.domains
            .lock()
            .unwrap()
            .push((model.schema_uri.clone(), region.placements.to_vec()));
        CemQlSchemaBehaviorEvaluator.validate_retained_region(region, model)
    }
    fn validate_document(&self, _: &CemDocument, _: &SchemaDocumentModel) -> Vec<Diagnostic> {
        panic!("native regions must not use the legacy document hook")
    }
}

#[test]
fn incomplete_runtime_regions_defer_all_behavior_and_retry_current_inputs() {
    let (captured,tree)=capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=child}}} {host @schema-select=library | {child}}");
    let outer = compile_schema_document_model(
        "outer",
        "{schema | {elements | {element @name=host @children=child}}}",
    );
    let selected = elements(&tree, "schema")[0];
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(
        tree.clone(),
        Some(context(&tree, selected, &tree, &[])),
        policy(),
    );
    host.attach_captured_names(&captured).unwrap();
    let evaluator = RecordingEvaluator::default();
    let pending = host
        .validate_input_runtime_host_regions_with_behavior_evaluator(
            "child",
            tree.clone(),
            &elements(&tree, "host"),
            &outer,
            policy().limits,
            |_| None,
            Some(&evaluator),
        )
        .unwrap();
    assert!(!pending.validation.complete);
    assert!(evaluator.compiled.lock().unwrap().is_empty());
    assert!(evaluator.domains.lock().unwrap().is_empty());
    let ready = host
        .validate_input_runtime_host_regions_with_behavior_evaluator(
            "child",
            tree.clone(),
            &elements(&tree, "host"),
            &outer,
            policy().limits,
            |_| Some(context(&tree, selected, &tree, &[])),
            Some(&evaluator),
        )
        .unwrap();
    assert!(
        ready.validation.complete && !ready.validation.failed,
        "{:?}",
        ready.validation.diagnostics
    );
    assert_eq!(*evaluator.compiled.lock().unwrap(), vec!["outer", "child"]);
    let domains = evaluator.domains.lock().unwrap();
    assert_eq!(domains.len(), 2);
    assert!(domains[0].1.contains(&0));
    let child=ready.validation.nodes.iter().position(|node| matches!(node.source.node(),CemAstNode::Element{expanded_name,..}if expanded_name.local_name=="child")).unwrap();
    assert!(domains[1].1.contains(&child));
    assert!(!domains[0].1.contains(&child));
}

#[test]
fn body_behavior_reads_consumed_node_attributes_without_expanding_descendant_links() {
    let inner = behavior(
        "fixture.attribute",
        "child",
        "$candidate.attributes.value.name",
    );
    let (captured,tree)=capture(&format!("@ns s = https://cem.dev/ns/schema/1\n{{s:schema | {{elements | {{element @name=child @required-attributes=target}}}} {{attributes | {{attribute @name=target @type=schema:node}}}} {inner}}} {{host @schema-select=library | {{child @target={{items}}}}}}"));
    let (data_capture, data) = capture("{schema | {item | {#missing}}}");
    let selected = elements(&tree, "schema")[0];
    let item = elements(&data, "item")[0];
    let outer = compile_schema_document_model(
        "outer",
        "{schema | {elements | {element @name=host @children=child}}}",
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(
        tree.clone(),
        Some(context(&tree, selected, &data, &[item])),
        policy(),
    );
    let destination = host.register_scope(
        data.clone(),
        Some(StandaloneExpressionContext::default()),
        policy(),
    );
    host.allow_scope_crossing(origin, destination);
    host.attach_captured_names(&captured).unwrap();
    host.attach_captured_names(&data_capture).unwrap();
    let result = host
        .validate_input_runtime_host_regions_with_behavior_evaluator(
            "child",
            tree.clone(),
            &elements(&tree, "host"),
            &outer,
            policy().limits,
            |_| Some(context(&tree, selected, &data, &[item])),
            Some(&CemQlSchemaBehaviorEvaluator),
        )
        .unwrap();
    assert!(
        result.validation.complete && !result.validation.failed,
        "{:?}",
        result.validation.diagnostics
    );
    assert_eq!(result.validation.diagnostics.len(), 1);
    let diagnostic = &result.validation.diagnostics[0];
    assert_eq!(diagnostic.code, "fixture.attribute");
    assert_eq!(diagnostic.details.as_ref().unwrap()["observed"], "item");
    let node = &result.validation.nodes[diagnostic.details.as_ref().unwrap()["placement"]
        .as_u64()
        .unwrap() as usize];
    assert_eq!(node.attribute_values.len(), 1);
    assert!(node.attribute_values[0].complete);
    let access = &node.attribute_values[0].access;
    assert_eq!(access.node(access.roots()[0]).unwrap().node_id(), item);
    let reference = access
        .children(access.roots()[0])
        .unwrap()
        .iter()
        .filter_map(|index| access.node(*index))
        .find(|node| matches!(node.node(), CemAstNode::Reference { .. }))
        .unwrap();
    assert!(matches!(
        reference.node(),
        CemAstNode::Reference { targets: None, .. }
    ));
    assert!(result.validation.references.is_empty());
}

#[test]
fn nested_region_behaviors_restore_the_enclosing_execution_domain() {
    let first = behavior(
        "fixture.first",
        "nodes",
        "{name: $candidate.name, parent: $candidate.parent.name}",
    );
    let second = behavior(
        "fixture.second",
        "nodes",
        "{name: $candidate.name, parent: $candidate.parent.name}",
    );
    let (captured,tree)=capture(&format!("@ns s = https://cem.dev/ns/schema/1\n{{s:schema | {{elements | {{element @name=nested @children=inner}} {{element @name=after}}}} {first}}} {{s:schema | {{elements | {{element @name=inner}}}} {second}}} {{host @schema-select=library | {{nested @schema-select=library | {{inner}}}} {{after}}}}"));
    let selected = elements(&tree, "schema");
    let outer = compile_schema_document_model(
        "outer",
        &format!(
            "{{schema | {{elements | {{element @name=host @children='nested after'}}}} {} }}",
            behavior("fixture.base", "nodes", "$candidate.name")
        ),
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(
        tree.clone(),
        Some(context(&tree, selected[0], &tree, &[])),
        policy(),
    );
    host.attach_captured_names(&captured).unwrap();
    let report = host
        .validate_input_runtime_host_regions_with_behavior_evaluator(
            "child",
            tree.clone(),
            &elements(&tree, "host"),
            &outer,
            policy().limits,
            |_| Some(context(&tree, selected[1], &tree, &[])),
            Some(&CemQlSchemaBehaviorEvaluator),
        )
        .unwrap();
    assert!(
        report.validation.complete && !report.validation.failed,
        "{:?}",
        report.validation.diagnostics
    );
    let codes: BTreeMap<_, _> = report
        .validation
        .diagnostics
        .iter()
        .map(|diag| {
            let observed = &diag.details.as_ref().unwrap()["observed"];
            (
                observed
                    .as_str()
                    .or_else(|| observed["name"].as_str())
                    .unwrap(),
                diag.code.as_str(),
            )
        })
        .collect();
    assert_eq!(codes["host"], "fixture.base");
    assert_eq!(codes["nested"], "fixture.first");
    assert_eq!(codes["after"], "fixture.first");
    assert_eq!(codes["inner"], "fixture.second");
}

#[derive(Debug)]
struct UnsupportedRegionEvaluator;
impl SchemaBehaviorEvaluator for UnsupportedRegionEvaluator {
    fn validate_document(&self, _: &CemDocument, _: &SchemaDocumentModel) -> Vec<Diagnostic> {
        panic!("region evaluation must not use legacy whole-document fallback")
    }
    fn validate_retained_structure(
        &self,
        _: cem_ml::schema::input_references::RetainedValidationStructure<'_>,
        _: &SchemaDocumentModel,
    ) -> RetainedBehaviorValidation {
        panic!("region evaluation must not borrow all-candidate compatibility behavior")
    }
}

#[test]
fn an_unsupported_region_evaluator_stays_incomplete_without_compatibility_fallback() {
    let (captured, tree) = capture("{host}");
    let outer =
        compile_schema_document_model("outer", "{schema | {elements | {element @name=host}}}");
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(
        tree.clone(),
        Some(StandaloneExpressionContext::default()),
        policy(),
    );
    host.attach_captured_names(&captured).unwrap();
    let report = host
        .validate_input_runtime_host_regions_with_behavior_evaluator(
            "child",
            tree.clone(),
            &elements(&tree, "host"),
            &outer,
            policy().limits,
            |_| panic!("ordinary hosts need no child input"),
            Some(&UnsupportedRegionEvaluator),
        )
        .unwrap();
    assert!(!report.validation.complete && !report.validation.failed);
    assert!(report.validation.diagnostics.is_empty());
}

#[test]
fn an_empty_ready_child_region_still_compiles_its_behavior_contract() {
    let (captured, tree) =
        capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema} {host @schema-select=library}");
    let outer =
        compile_schema_document_model("outer", "{schema | {elements | {element @name=host}}}");
    let selected = elements(&tree, "schema")[0];
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(
        tree.clone(),
        Some(context(&tree, selected, &tree, &[])),
        policy(),
    );
    host.attach_captured_names(&captured).unwrap();
    let evaluator = RecordingEvaluator::default();
    let report = host
        .validate_input_runtime_host_regions_with_behavior_evaluator(
            "child",
            tree.clone(),
            &elements(&tree, "host"),
            &outer,
            policy().limits,
            |_| Some(StandaloneExpressionContext::default()),
            Some(&evaluator),
        )
        .unwrap();
    assert!(
        report.validation.complete && !report.validation.failed,
        "{:?}",
        report.validation.diagnostics
    );
    assert_eq!(*evaluator.compiled.lock().unwrap(), vec!["outer", "child"]);
    assert!(evaluator.domains.lock().unwrap()[1].1.is_empty());
}

#[derive(Debug)]
struct PanickingRegionEvaluator;
impl SchemaBehaviorEvaluator for PanickingRegionEvaluator {
    fn validate_document(&self, _: &CemDocument, _: &SchemaDocumentModel) -> Vec<Diagnostic> {
        unreachable!()
    }
    fn validate_retained_region(
        &self,
        _: RetainedBehaviorRegion<'_>,
        _: &SchemaDocumentModel,
    ) -> RetainedBehaviorValidation {
        panic!("behavior consumer failed during its region stage")
    }
}
#[test]
fn behavior_consumer_unwind_restores_the_original_runtime_bindings() {
    use cem_ml::schema::declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode};
    use cem_ml::value::reference_resolution::ReferenceResolutionHost;
    let (captured,tree)=capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=child}}} {host @schema-select=library | {child}}");
    let outer = compile_schema_document_model(
        "outer",
        "{schema | {elements | {element @name=host @children=child}}}",
    );
    let selected = elements(&tree, "schema")[0];
    let child =
        SchemaDeclarationNode::new(tree.ast_owner().clone(), elements(&tree, "child")[0]).unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let original = host.register_scope(
        tree.clone(),
        Some(context(&tree, selected, &tree, &[])),
        policy(),
    );
    host.attach_captured_names(&captured).unwrap();
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = host.validate_input_runtime_host_regions_with_behavior_evaluator(
            "child",
            tree.clone(),
            &elements(&tree, "host"),
            &outer,
            policy().limits,
            |_| Some(StandaloneExpressionContext::default()),
            Some(&PanickingRegionEvaluator),
        );
    }));
    assert!(failed.is_err());
    assert_eq!(host.scope(&host.source_reference(child)), Some(original));
    let retry = host
        .validate_input_runtime_host_regions_with_behavior_evaluator(
            "child",
            tree.clone(),
            &elements(&tree, "host"),
            &outer,
            policy().limits,
            |_| Some(StandaloneExpressionContext::default()),
            Some(&CemQlSchemaBehaviorEvaluator),
        )
        .unwrap();
    assert!(
        retry.validation.complete && !retry.validation.failed,
        "{:?}",
        retry.validation.diagnostics
    );
}

#[test]
fn wrapping_controls_route_native_behaviors_and_preserve_enclosing_host_validation() {
    let inner = behavior("fixture.inner", "nodes", "$candidate.parent.name");
    let (captured,tree)=capture(&format!("@ns s = https://cem.dev/ns/schema/1\n@ns c = https://cem.dev/ns/core/1\n{{s:schema | {{elements | {{element @name=inner}}}} {inner}}} {{c:schema @own=yes @select={{#library}} | {{inner}}}} {{sibling}}"));
    let schemas = elements(&tree, "schema");
    let outer=compile_schema_document_model("base",&format!("{{schema | {{elements | {{element @name=schema @required-attributes=own @children=inner}} {{element @name=sibling}}}} {} }}",behavior("fixture.outer","nodes","$candidate.name")));
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(
        tree.clone(),
        Some(context(&tree, schemas[0], &tree, &[])),
        policy(),
    );
    host.attach_captured_names(&captured).unwrap();
    let report = host
        .validate_input_runtime_host_regions_with_behavior_evaluator(
            "child",
            tree.clone(),
            &[schemas[1], elements(&tree, "sibling")[0]],
            &outer,
            policy().limits,
            |_| Some(StandaloneExpressionContext::default()),
            Some(&CemQlSchemaBehaviorEvaluator),
        )
        .unwrap();
    assert!(
        report.validation.complete && !report.validation.failed,
        "{:?}",
        report.validation.diagnostics
    );
    let outer_count = report
        .validation
        .diagnostics
        .iter()
        .filter(|d| d.code == "fixture.outer")
        .count();
    assert_eq!(outer_count, 2);
    let inner = report
        .validation
        .diagnostics
        .iter()
        .find(|d| d.code == "fixture.inner")
        .unwrap();
    assert_eq!(inner.details.as_ref().unwrap()["observed"], "schema");
    assert!(report
        .validation
        .nodes
        .iter()
        .all(|node| Arc::ptr_eq(node.source.document(), tree.ast_owner())));
    assert_eq!(report.validation.diagnostics.len(), 3);
}
