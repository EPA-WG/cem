use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    events::cem::CemEventNormalizer,
    parser::{builder::CemAstBuilder, document::CemDocument, CemAstNode},
    schema::{
        declaration_references::{
            compile_schema_with_declaration_references, SchemaDeclarationHost,
            SchemaDeclarationNode,
        },
        document_model::compile_schema_document_model,
        reference_policy::{ReferenceOccurrence, ReferenceScopePolicy, ReferenceUnresolvedPolicy},
        reference_traversal::ReferenceTraversalLimits,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
    value::reference_resolution::{
        ReferenceLinkEvaluation, ReferenceResolutionHost, ReferenceResolutionState,
    },
};
use std::{collections::HashMap, sync::Arc};

fn parse(text: &str) -> Arc<CemDocument> {
    let tokenizer =
        CemTokenizer::from_source(BytesSource::new(SourceId(1), text.as_bytes().to_vec()));
    let document = CemAstBuilder::new(CemEventNormalizer::new(tokenizer)).build();
    assert!(
        document.diagnostics.is_empty(),
        "{:?}",
        document.diagnostics
    );
    Arc::new(document)
}
fn node(document: &Arc<CemDocument>, name: &str) -> SchemaDeclarationNode {
    let id = document
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == name => Some(*node_id),
            _ => None,
        })
        .unwrap();
    SchemaDeclarationNode::new(document.clone(), id).unwrap()
}
fn reference(text: &str) -> Arc<CemDocument> {
    parse(&format!("{{schema | {{elements | {{#{text}}} }} }}"))
}
struct Host {
    outcomes: HashMap<String, ReferenceLinkEvaluation<SchemaDeclarationNode>>,
    policy: ReferenceScopePolicy,
    deny: bool,
    calls: usize,
    schema_calls: std::sync::atomic::AtomicUsize,
    expression_calls: usize,
    denied_nodes: std::collections::HashSet<String>,
    scope_bounds: HashMap<usize, ReferenceTraversalLimits>,
}
impl Host {
    fn new() -> Self {
        Self {
            outcomes: HashMap::new(),
            policy: ReferenceScopePolicy::schema_defaults().unwrap(),
            deny: false,
            calls: 0,
            schema_calls: Default::default(),
            expression_calls: 0,
            denied_nodes: Default::default(),
            scope_bounds: Default::default(),
        }
    }
    fn disposition(&mut self, value: &str) {
        let model = compile_schema_document_model(
            "policy",
            &format!(
                r#"{{schema | {{constraints | {{constraint @kind="reference-unresolved-disposition" @value="{value}"}} }} }}"#
            ),
        );
        self.policy = self.policy.for_scope(&model).unwrap();
    }
    fn resolve(
        &mut self,
        document: Arc<CemDocument>,
    ) -> cem_ml::schema::document_model::SchemaDocumentModel {
        let limits = self.policy.limits;
        compile_schema_with_declaration_references("consumer", document, self, limits).unwrap()
    }
}
impl ReferenceResolutionHost for Host {
    type Node = SchemaDeclarationNode;
    // This fixture explicitly treats each supplied document as one scope.
    // Production hosts must provide their actual lexical/runtime scope mapping.
    type Scope = usize;
    fn scope(&self, node: &Self::Node) -> usize {
        Arc::as_ptr(node.document()) as usize
    }
    fn scope_limits(&self, scope: &usize) -> ReferenceTraversalLimits {
        self.scope_bounds
            .get(scope)
            .copied()
            .unwrap_or(self.policy.limits)
    }
    fn reference_occurrence(&self, node: &Self::Node) -> Option<ReferenceOccurrence> {
        match node.node() {
            CemAstNode::Reference {
                expression, source, ..
            } => Some(ReferenceOccurrence {
                identity: node.identity(),
                node_id: Some(node.node_id()),
                expression: Some(expression.clone()),
                source_map: source.clone(),
            }),
            _ => None,
        }
    }
    fn unresolved_policy(&self, _: &Self::Node) -> &ReferenceUnresolvedPolicy {
        &self.policy.unresolved
    }
    fn permits_edge(&self, _: &Self::Node, target: &Self::Node) -> bool {
        !self.deny && !self.denied_nodes.contains(&target.identity())
    }
    fn evaluate(&mut self, reference: &Self::Node) -> ReferenceLinkEvaluation<Self::Node> {
        self.calls += 1;
        let CemAstNode::Reference { expression, .. } = reference.node() else {
            panic!()
        };
        self.outcomes
            .get(expression)
            .cloned()
            .unwrap_or_else(|| ReferenceLinkEvaluation::Unresolved("dependency-unavailable".into()))
    }
}
impl SchemaDeclarationHost for Host {
    fn evaluate_input_expression(
        &mut self,
        source: &Self::Node,
    ) -> ReferenceLinkEvaluation<Self::Node> {
        self.expression_calls += 1;
        let expression = cem_ml::schema::input_references::native_attribute_expression(source)
            .unwrap()
            .expression
            .unwrap();
        self.outcomes.get(&expression).cloned().unwrap_or_else(|| {
            ReferenceLinkEvaluation::Pending("input-expression-consumer-not-ready".into())
        })
    }
    fn source_reference(&self, source: SchemaDeclarationNode) -> SchemaDeclarationNode {
        source
    }
    fn declaration_node(&self, target: &SchemaDeclarationNode) -> Option<SchemaDeclarationNode> {
        Some(target.clone())
    }
    fn declaration_schema(&self, target: &SchemaDeclarationNode) -> Option<SchemaDeclarationNode> {
        self.schema_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        target
            .document()
            .nodes
            .iter()
            .find_map(|value| match value {
                CemAstNode::Element {
                    node_id,
                    expanded_name,
                    ..
                } if expanded_name.local_name == "schema" => {
                    SchemaDeclarationNode::new(target.document().clone(), *node_id)
                }
                _ => None,
            })
    }
}

#[test]
fn one_declaration_is_reused_without_source_cloning_and_with_lexical_aliases() {
    let text = r#"{schema | {uses | {use @schema="https://cem.dev/ns/schema/1" @as="origin"}} {elements | {element @name="shared" @base="origin:element" @required-attributes="own"}} }"#;
    let library = parse(text);
    let target = node(&library, "element");
    let expected = compile_schema_document_model("library", text)
        .element("shared")
        .unwrap()
        .clone();
    assert!(expected.optional_attributes.contains("base"));
    let first = reference("library");
    let second = reference("library");
    let source_nodes = first.nodes.len();
    let mut host = Host::new();
    host.outcomes.insert(
        "#library".into(),
        ReferenceLinkEvaluation::Resolved(vec![target.clone(), target.clone()]),
    );
    let a = host.resolve(first.clone());
    let b = host.resolve(second);
    assert_eq!(a.element("shared"), Some(&expected));
    assert_eq!(b.element("shared"), Some(&expected));
    assert!(a.declaration_references.is_complete());
    assert!(!a.declaration_references.failed());
    let targets = &a.declaration_references.sites[0]
        .resolution
        .as_ref()
        .unwrap()
        .nodes;
    assert_eq!(targets.len(), 2);
    assert!(targets
        .iter()
        .all(|value| Arc::ptr_eq(value.document(), &library)));
    assert_eq!(first.nodes.len(), source_nodes);
    assert!(first
        .nodes
        .iter()
        .any(|value| matches!(value, CemAstNode::Reference { targets: None, .. })));
    assert_eq!(host.calls, 2);
}

#[test]
fn collection_order_zero_many_targets_and_consumer_field_contracts_are_preserved() {
    let library = parse(
        r#"{schema | {elements | {element @name="shared" @required-attributes="from-library"} {element @name="other"}} }"#,
    );
    let targets: Vec<_> = library
        .nodes
        .iter()
        .filter_map(|value| match value {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "element" => {
                SchemaDeclarationNode::new(library.clone(), *node_id)
            }
            _ => None,
        })
        .collect();
    let source = parse(
        r#"{schema | {elements | {element @name="shared" @required-attributes="before"} {#library} {element @name="shared" @required-attributes="after"} {#empty}} {field-contracts | {field-contract @name="related" @target="other" @required-attributes="own" @check-kind="required-fields"}} }"#,
    );
    let mut host = Host::new();
    host.outcomes.insert(
        "#library".into(),
        ReferenceLinkEvaluation::Resolved(targets),
    );
    host.outcomes
        .insert("#empty".into(), ReferenceLinkEvaluation::Resolved(vec![]));
    let model = host.resolve(source);
    assert_eq!(
        model
            .element("shared")
            .unwrap()
            .required_attributes
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["after"]
    );
    assert_eq!(model.element("other").unwrap().field_contracts.len(), 1);
    assert!(model.declaration_references.is_complete());
    assert_eq!(model.declaration_references.sites.len(), 2);
}

#[test]
fn partial_declarations_remain_incomplete_under_every_disposition() {
    let library = parse(r#"{schema | {elements | {element @name="shared"}} }"#);
    let dependency = reference("missing");
    let missing = dependency
        .nodes
        .iter()
        .find_map(|value| match value {
            CemAstNode::Reference { node_id, .. } => {
                SchemaDeclarationNode::new(dependency.clone(), *node_id)
            }
            _ => None,
        })
        .unwrap();
    for disposition in ["neutral", "mandatory", "warning", "ignore"] {
        let mut host = Host::new();
        host.disposition(disposition);
        host.outcomes.insert(
            "#library".into(),
            ReferenceLinkEvaluation::Resolved(vec![node(&library, "element"), missing.clone()]),
        );
        let model = host.resolve(reference("library"));
        assert!(model.element("shared").is_some());
        assert_eq!(
            model.declaration_references.state(),
            ReferenceResolutionState::Unresolved
        );
        assert!(!model.declaration_references.is_complete());
        assert_eq!(
            model.declaration_references.failed(),
            disposition == "mandatory"
        );
        assert_eq!(
            model.compile_diagnostics.len(),
            usize::from(matches!(disposition, "mandatory" | "warning"))
        );
        let issues = &model.declaration_references.sites[0]
            .resolution
            .as_ref()
            .unwrap()
            .issues;
        assert_eq!(issues[0].occurrence.identity, missing.identity());
        assert_eq!(issues[0].occurrence.expression.as_deref(), Some("#missing"));
    }
}

#[test]
fn invalid_target_kinds_and_names_report_at_the_consuming_reference() {
    for text in [
        "{schema | {other}}",
        "{schema | {elements | {element}}}",
        "{schema | {elements | {element @name=\" \"}}}",
    ] {
        let library = parse(text);
        let target = if text.contains("{other}") {
            node(&library, "other")
        } else {
            node(&library, "element")
        };
        let mut host = Host::new();
        host.disposition("ignore");
        host.outcomes.insert(
            "#library".into(),
            ReferenceLinkEvaluation::Resolved(vec![target]),
        );
        let model = host.resolve(reference("library"));
        assert_eq!(
            model.declaration_references.state(),
            ReferenceResolutionState::Invalid
        );
        assert!(model.declaration_references.failed());
        assert!(model.elements.is_empty());
        let diagnostic = &model.compile_diagnostics[0];
        assert_eq!(
            diagnostic.code,
            "cem.schema_definition.invalid_reference_target"
        );
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(diagnostic.byte_offset.is_some());
        assert!(diagnostic.source_map.is_some());
        assert_eq!(
            diagnostic.node.as_deref(),
            Some(
                model.declaration_references.sites[0]
                    .occurrence
                    .identity
                    .as_str()
            )
        );
    }
}

#[test]
fn pending_empty_invalid_and_unevaluated_source_are_distinct() {
    let source = reference("library");
    let legacy = compile_schema_document_model("consumer", "{schema | {elements | {#library}} }");
    assert_eq!(
        legacy.declaration_references.state(),
        ReferenceResolutionState::Pending
    );
    assert!(legacy.declaration_references.sites[0].resolution.is_none());
    let mut host = Host::new();
    host.outcomes.insert(
        "#library".into(),
        ReferenceLinkEvaluation::Pending("not-ready".into()),
    );
    let model = host.resolve(source.clone());
    assert_eq!(
        model.declaration_references.state(),
        ReferenceResolutionState::Pending
    );
    assert!(model.declaration_references.sites[0].resolution.is_some());
    assert!(model.compile_diagnostics.is_empty());
    host.outcomes
        .insert("#library".into(), ReferenceLinkEvaluation::Resolved(vec![]));
    let model = host.resolve(source.clone());
    assert!(model.declaration_references.is_complete());
    assert!(model.elements.is_empty());
    let original = Diagnostic {
        code: "fixture.invalid-query".into(),
        severity: Severity::Error,
        message: "scalar operand".into(),
        ..Default::default()
    };
    host.outcomes.insert(
        "#library".into(),
        ReferenceLinkEvaluation::Invalid(vec![original]),
    );
    let model = host.resolve(source);
    assert_eq!(
        model.declaration_references.state(),
        ReferenceResolutionState::Invalid
    );
    assert!(model.declaration_references.failed());
    assert_eq!(model.compile_diagnostics[0].code, "fixture.invalid-query");
}

#[test]
fn crossings_cycles_and_request_limits_use_shared_resolution_policy() {
    let source = reference("library");
    let library = parse(r#"{schema | {elements | {element @name="shared"}} }"#);
    let target = node(&library, "element");
    let root = source
        .nodes
        .iter()
        .find_map(|value| match value {
            CemAstNode::Reference { node_id, .. } => {
                SchemaDeclarationNode::new(source.clone(), *node_id)
            }
            _ => None,
        })
        .unwrap();
    let mut host = Host::new();
    host.outcomes.insert(
        "#library".into(),
        ReferenceLinkEvaluation::Resolved(vec![target.clone()]),
    );
    host.deny = true;
    let model = host.resolve(source.clone());
    assert_eq!(
        model.declaration_references.sites[0]
            .resolution
            .as_ref()
            .unwrap()
            .issues[0]
            .reason,
        "scope-denied"
    );
    assert!(model.elements.is_empty());
    host.deny = false;
    host.outcomes.insert(
        "#library".into(),
        ReferenceLinkEvaluation::Resolved(vec![target, root]),
    );
    let model = host.resolve(source.clone());
    assert!(model.element("shared").is_some());
    assert_eq!(
        model.declaration_references.sites[0]
            .resolution
            .as_ref()
            .unwrap()
            .issues[0]
            .reason,
        "cycle"
    );
    let mut limits = host.policy.limits;
    limits.max_work = 1;
    let model =
        compile_schema_with_declaration_references("consumer", source, &mut host, limits).unwrap();
    assert!(model.elements.is_empty());
    assert_eq!(
        model.declaration_references.sites[0]
            .resolution
            .as_ref()
            .unwrap()
            .issues[0]
            .reason,
        "work-limit"
    );
}

#[derive(Clone)]
enum RuntimeNode {
    Arena(SchemaDeclarationNode),
    Reference(Arc<Vec<RuntimeNode>>),
}
struct RuntimeHost {
    arena: Host,
    targets: Arc<Vec<RuntimeNode>>,
}
impl ReferenceResolutionHost for RuntimeHost {
    type Node = RuntimeNode;
    type Scope = ();
    fn scope(&self, _: &Self::Node) {}
    fn scope_limits(&self, _: &()) -> ReferenceTraversalLimits {
        self.arena.policy.limits
    }
    fn reference_occurrence(&self, node: &Self::Node) -> Option<ReferenceOccurrence> {
        match node {
            RuntimeNode::Arena(node) => self.arena.reference_occurrence(node),
            RuntimeNode::Reference(values) => Some(ReferenceOccurrence {
                identity: format!("native:{:p}", Arc::as_ptr(values)),
                node_id: None,
                expression: None,
                source_map: Default::default(),
            }),
        }
    }
    fn unresolved_policy(&self, _: &Self::Node) -> &ReferenceUnresolvedPolicy {
        &self.arena.policy.unresolved
    }
    fn permits_edge(&self, _: &Self::Node, _: &Self::Node) -> bool {
        true
    }
    fn evaluate(&mut self, node: &Self::Node) -> ReferenceLinkEvaluation<Self::Node> {
        match node {
            RuntimeNode::Arena(_) => {
                ReferenceLinkEvaluation::Resolved(vec![RuntimeNode::Reference(
                    self.targets.clone(),
                )])
            }
            RuntimeNode::Reference(values) => {
                ReferenceLinkEvaluation::Resolved(values.as_ref().clone())
            }
        }
    }
}
impl SchemaDeclarationHost for RuntimeHost {
    fn source_reference(&self, node: SchemaDeclarationNode) -> Self::Node {
        RuntimeNode::Arena(node)
    }
    fn declaration_node(&self, target: &Self::Node) -> Option<SchemaDeclarationNode> {
        match target {
            RuntimeNode::Arena(node) => Some(node.clone()),
            _ => None,
        }
    }
    fn declaration_schema(&self, target: &SchemaDeclarationNode) -> Option<SchemaDeclarationNode> {
        self.arena.declaration_schema(target)
    }
}

#[test]
fn runtime_constructed_references_need_no_saved_arena_or_context_handle() {
    let source = reference("library");
    let library = parse(r#"{schema | {elements | {element @name="shared"}} }"#);
    let weak_owner = Arc::downgrade(&library);
    let mut host = RuntimeHost {
        arena: Host::new(),
        targets: Arc::new(vec![RuntimeNode::Arena(node(&library, "element"))]),
    };
    let mut limits = host.arena.policy.limits;
    let complete =
        compile_schema_with_declaration_references("consumer", source.clone(), &mut host, limits)
            .unwrap();
    assert!(complete.declaration_references.is_complete());
    assert_eq!(
        complete.declaration_references.sites[0]
            .resolution
            .as_ref()
            .unwrap()
            .work_used,
        3
    );
    limits.max_depth = 1;
    let incomplete =
        compile_schema_with_declaration_references("consumer", source, &mut host, limits).unwrap();
    assert!(!incomplete.declaration_references.is_complete());
    let issue = &incomplete.declaration_references.sites[0]
        .resolution
        .as_ref()
        .unwrap()
        .issues[0];
    assert_eq!(issue.reason, "depth-limit");
    assert!(issue.occurrence.node_id.is_none());
    assert!(issue.occurrence.expression.is_none());
    drop(host);
    drop(library);
    let retained = &complete.declaration_references.sites[0]
        .resolution
        .as_ref()
        .unwrap()
        .nodes[0];
    assert!(weak_owner.upgrade().is_some());
    assert!(matches!(retained.node(), CemAstNode::Element { .. }));
    drop(complete);
    assert!(weak_owner.upgrade().is_none());
}

#[test]
fn incomplete_models_are_inspectable_but_never_ready_for_final_validation() {
    use cem_ml::schema::document_model::{
        load_document_model_for_identity, validate_document_model, SchemaDocumentModelRegistry,
    };
    use cem_ml::schema::registry::{SchemaRegistry, CEM_ML_SCHEMA_URI};
    let source = reference("library");
    for disposition in ["neutral", "mandatory", "warning", "ignore"] {
        let mut host = Host::new();
        host.disposition(disposition);
        let mut model = host.resolve(source.clone());
        model.schema_uri = CEM_ML_SCHEMA_URI.into();
        assert!(!model.is_ready_for_validation());
        let original_count = model.compile_diagnostics.len();
        let mut registry = SchemaDocumentModelRegistry::new();
        registry.register(model.clone());
        assert!(registry.get(CEM_ML_SCHEMA_URI).is_some());
        assert!(registry
            .resolve_for_identity(Some(CEM_ML_SCHEMA_URI), None, None)
            .is_none());
        let schemas = SchemaRegistry::with_builtin_schemas();
        assert!(registry
            .resolve_for_identity(None, Some("application/cem"), Some(&schemas))
            .is_none());
        assert!(load_document_model_for_identity(
            Some(CEM_ML_SCHEMA_URI),
            None,
            Some(&schemas),
            Some(&registry)
        )
        .is_none());
        assert!(load_document_model_for_identity(
            None,
            Some("application/cem"),
            Some(&schemas),
            Some(&registry)
        )
        .is_none());
        use cem_ml::validation::{rules::SchemaDocumentModelRule, RuleContext, SemanticRule};
        let document = parse("{unknown}");
        let rule_diagnostics = SchemaDocumentModelRule.run(&RuleContext {
            document: &document,
            schema_uri: Some(CEM_ML_SCHEMA_URI),
            content_type: Some("application/cem"),
            source_uri: Some("input.cem"),
            resource_reader: None,
            schema_registry: Some(&schemas),
            schema_document_models: Some(&registry),
            upstream_diagnostics: &[],
            schema_behavior_evaluator: None,
        });
        assert!(rule_diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()));
        assert!(!rule_diagnostics
            .iter()
            .any(|d| d.code == "cem.schema_model.unknown_element"));
        let diagnostics = validate_document_model(&parse("{unknown}"), &model);
        assert!(diagnostics.iter().any(|d| d.severity.is_hard_violation()));
        assert!(!diagnostics
            .iter()
            .any(|d| d.code == "cem.schema_model.unknown_element"));
        if disposition != "mandatory" {
            assert!(diagnostics
                .iter()
                .any(|d| d.code == "cem.schema_model.not_ready"));
        }
        assert_eq!(model.compile_diagnostics.len(), original_count);
        assert_eq!(
            model.declaration_references.state(),
            ReferenceResolutionState::Unresolved
        );
    }
}

#[test]
fn completing_the_same_source_activates_the_model_without_source_changes() {
    use cem_ml::schema::document_model::{validate_document_model, SchemaDocumentModelRegistry};
    let source = reference("library");
    let library =
        parse(r#"{schema | {elements | {element @name="shared" @required-attributes="own"}} }"#);
    let mut host = Host::new();
    let mut registry = SchemaDocumentModelRegistry::new();
    registry.register(host.resolve(source.clone()));
    assert!(registry
        .resolve_for_identity(Some("consumer"), None, None)
        .is_none());
    host.outcomes.insert(
        "#library".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&library, "element")]),
    );
    let ready = host.resolve(source.clone());
    assert!(ready.is_ready_for_validation());
    registry.register(ready);
    let model = registry
        .resolve_for_identity(Some("consumer"), None, None)
        .unwrap();
    assert!(validate_document_model(&parse("{shared @own=present}"), model).is_empty());
    assert!(validate_document_model(&parse("{shared}"), model)
        .iter()
        .any(|d| d.code == "cem.schema_model.missing_required_attribute"));
    assert!(source
        .nodes
        .iter()
        .any(|value| matches!(value, CemAstNode::Reference { targets: None, .. })));
}

#[test]
fn inactive_models_do_not_run_behavior_hooks_or_hide_original_errors() {
    use cem_ml::schema::document_model::{
        validate_document_model_with_behavior_evaluator, SchemaBehaviorEvaluator,
        SchemaDocumentModel,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[derive(Debug, Default)]
    struct Hooks(AtomicUsize);
    impl SchemaBehaviorEvaluator for Hooks {
        fn compile_model(&self, _: &SchemaDocumentModel) -> Vec<Diagnostic> {
            self.0.fetch_add(1, Ordering::Relaxed);
            vec![]
        }
        fn validate_document(&self, _: &CemDocument, _: &SchemaDocumentModel) -> Vec<Diagnostic> {
            self.0.fetch_add(1, Ordering::Relaxed);
            vec![]
        }
    }
    let mut host = Host::new();
    let pending = compile_schema_document_model(
        "consumer",
        "{schema | {elements | {element @name=shared} {#library}} }",
    );
    let hooks = Hooks::default();
    let diagnostics =
        validate_document_model_with_behavior_evaluator(&parse("{shared}"), &pending, Some(&hooks));
    assert_eq!(diagnostics[0].code, "cem.schema_model.not_ready");
    assert_eq!(hooks.0.load(Ordering::Relaxed), 0);
    let original = Diagnostic {
        code: "fixture.original-invalid".into(),
        severity: Severity::Error,
        message: "original expression failure".into(),
        ..Default::default()
    };
    host.outcomes.insert(
        "#library".into(),
        ReferenceLinkEvaluation::Invalid(vec![original]),
    );
    let invalid = host.resolve(reference("library"));
    let diagnostics =
        validate_document_model_with_behavior_evaluator(&parse("{shared}"), &invalid, Some(&hooks));
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "fixture.original-invalid");
    assert_eq!(hooks.0.load(Ordering::Relaxed), 0);
}

#[test]
fn an_incomplete_replacement_preserves_the_last_complete_active_model() {
    use cem_ml::schema::document_model::{
        load_document_model_for_identity, validate_document_model, SchemaDocumentModelRegistry,
    };
    let library =
        parse(r#"{schema | {elements | {element @name=shared @required-attributes=own}} }"#);
    let source = reference("library");
    let mut host = Host::new();
    host.outcomes.insert(
        "#library".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&library, "element")]),
    );
    let mut registry = SchemaDocumentModelRegistry::new();
    registry.register(host.resolve(source.clone()));
    host.outcomes.clear();
    registry.register(host.resolve(source.clone()));
    assert!(!registry.get("consumer").unwrap().is_ready_for_validation());
    let active = registry
        .resolve_for_identity(Some("consumer"), None, None)
        .unwrap();
    assert!(active.is_ready_for_validation());
    assert!(active
        .element("shared")
        .unwrap()
        .required_attributes
        .contains("own"));
    assert!(
        load_document_model_for_identity(Some("consumer"), None, None, Some(&registry))
            .unwrap()
            .is_ready_for_validation()
    );
    assert!(validate_document_model(&parse("{shared}"), active)
        .iter()
        .any(|d| d.code == "cem.schema_model.missing_required_attribute"));
    let replacement =
        parse(r#"{schema | {elements | {element @name=shared @required-attributes=new}} }"#);
    host.outcomes.insert(
        "#library".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&replacement, "element")]),
    );
    registry.register(host.resolve(source));
    let active = registry
        .resolve_for_identity(Some("consumer"), None, None)
        .unwrap();
    assert!(active
        .element("shared")
        .unwrap()
        .required_attributes
        .contains("new"));
    assert!(!active
        .element("shared")
        .unwrap()
        .required_attributes
        .contains("own"));
}

#[test]
fn attribute_declarations_retain_owners_and_drive_validation_in_multiple_schemas() {
    use cem_ml::schema::document_model::validate_document_model;
    let library_text = r#"{schema | {attributes | {attribute @name="size" @type="schema:integer" @minInclusive=1 @maxInclusive=10 @default=3}} }"#;
    let library = parse(library_text);
    let target = node(&library, "attribute");
    let expected =
        compile_schema_document_model("library", library_text).attributes["size"].clone();
    let source = parse("{schema | {elements | {element @name=box @optional-attributes=size}} {attributes | {#library}} }");
    let nodes = source.nodes.len();
    let mut host = Host::new();
    host.outcomes.insert(
        "#library".into(),
        ReferenceLinkEvaluation::Resolved(vec![target.clone(), target]),
    );
    for _ in 0..2 {
        let model = host.resolve(source.clone());
        assert!(model.is_ready_for_validation());
        assert_eq!(model.attributes["size"], expected);
        assert!(validate_document_model(&parse("{box @size=5}"), &model).is_empty());
        assert!(validate_document_model(&parse("{box @size=11}"), &model)
            .iter()
            .any(|d| d.severity.is_hard_violation()));
        let retained = &model.declaration_references.sites[0]
            .resolution
            .as_ref()
            .unwrap()
            .nodes;
        assert_eq!(retained.len(), 2);
        assert!(retained.iter().all(|n| Arc::ptr_eq(n.document(), &library)));
    }
    assert_eq!(source.nodes.len(), nodes);
    assert!(source
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
}
#[test]
fn attribute_reference_collections_preserve_order_empty_selections_and_pending_source() {
    let source = parse("{schema | {attributes | {attribute @name=size @maxInclusive=1} {#library} {attribute @name=size @maxInclusive=9} {#empty}} {elements | {#element}} }");
    let library = parse(
        "{schema | {attributes | {attribute @name=size @maxInclusive=5} {attribute @name=extra}} }",
    );
    let targets = library
        .nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "attribute" => {
                SchemaDeclarationNode::new(library.clone(), *node_id)
            }
            _ => None,
        })
        .collect();
    let element = parse("{element @name=box}");
    let mut host = Host::new();
    host.outcomes.insert(
        "#library".into(),
        ReferenceLinkEvaluation::Resolved(targets),
    );
    host.outcomes
        .insert("#empty".into(), ReferenceLinkEvaluation::Resolved(vec![]));
    host.outcomes.insert(
        "#element".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&element, "element")]),
    );
    let model = host.resolve(source);
    assert_eq!(model.attributes["size"].max_inclusive.as_deref(), Some("9"));
    assert!(model.attributes.contains_key("extra") && model.elements.contains_key("box"));
    assert_eq!(
        model
            .declaration_references
            .sites
            .iter()
            .map(|s| s.occurrence.expression.as_deref().unwrap())
            .collect::<Vec<_>>(),
        vec!["#library", "#empty", "#element"]
    );
    let pending = compile_schema_document_model("pending", "{schema | {attributes | {#library}} }");
    assert!(!pending.is_ready_for_validation());
    assert_eq!(
        pending.declaration_references.state(),
        ReferenceResolutionState::Pending
    );
    assert_eq!(pending.declaration_references.sites.len(), 1);
    assert!(pending.declaration_references.sites[0].resolution.is_none());
}
#[test]
fn attribute_reference_kind_and_name_errors_are_consumer_errors_under_every_disposition() {
    for disposition in ["mandatory", "warning", "ignore", "neutral"] {
        for target in [
            parse("{element @name=wrong}"),
            parse("{attribute}"),
            parse("{attribute @name=\" \"}"),
        ] {
            let mut host = Host::new();
            host.disposition(disposition);
            let name = if target.nodes.iter().any(|n| matches!(n, CemAstNode::Element { expanded_name, ..} if expanded_name.local_name == "attribute")) {"attribute"} else {"element"};
            host.outcomes.insert(
                "#library".into(),
                ReferenceLinkEvaluation::Resolved(vec![node(&target, name)]),
            );
            let model = host.resolve(parse("{schema | {attributes | {#library}} }"));
            assert_eq!(
                model.declaration_references.state(),
                ReferenceResolutionState::Invalid
            );
            assert!(!model.is_ready_for_validation());
            assert!(model.compile_diagnostics.iter().any(|d| d.code
                == cem_ml::schema::declaration_references::INVALID_REFERENCE_TARGET
                && d.source_map.is_some()));
        }
    }
}
#[test]
fn attribute_references_keep_partial_results_and_unresolved_readiness_under_each_policy() {
    for disposition in ["mandatory", "warning", "ignore", "neutral"] {
        let target = parse("{attribute @name=available}");
        let mut host = Host::new();
        host.disposition(disposition);
        host.outcomes.insert(
            "#library".into(),
            ReferenceLinkEvaluation::Resolved(vec![node(&target, "attribute")]),
        );
        host.outcomes.insert(
            "#later".into(),
            ReferenceLinkEvaluation::Unresolved("not ready".into()),
        );
        let model = host.resolve(parse("{schema | {attributes | {#library} {#later}} }"));
        assert!(model.attributes.contains_key("available"));
        assert!(!model.is_ready_for_validation());
        assert_eq!(
            model.declaration_references.state(),
            ReferenceResolutionState::Unresolved
        );
    }
}

#[test]
fn referenced_attribute_dependencies_are_checked_in_the_assembled_consumer() {
    use cem_ml::schema::document_model::UNRESOLVED_DIAGNOSTIC_REFERENCE_CODE;
    let library = parse(
        r#"{schema | {attributes | {attribute @name=size @type=schema:integer @type-diagnostic=fixture.value}} {diagnostics | {diagnostic @code=fixture.value @severity=error @behavior=schema:scalar-type}} }"#,
    );
    let declaration = node(&library, "attribute");
    let source_map = match declaration.node() {
        CemAstNode::Element { source, .. } => source.clone(),
        _ => unreachable!(),
    };
    let mut host = Host::new();
    host.outcomes.insert(
        "#library".into(),
        ReferenceLinkEvaluation::Resolved(vec![declaration]),
    );
    let missing = host.resolve(parse("{schema | {attributes | {#library}} }"));
    assert!(missing.diagnostics.is_empty());
    let error = missing
        .compile_diagnostics
        .iter()
        .find(|d| d.code == UNRESOLVED_DIAGNOSTIC_REFERENCE_CODE)
        .unwrap();
    assert_eq!(error.source_map.as_ref(), Some(&source_map));
    let provided = host.resolve(parse("{schema | {attributes | {#library}} {diagnostics | {diagnostic @code=fixture.value @severity=error @behavior=schema:scalar-type}} }"));
    assert!(!provided
        .compile_diagnostics
        .iter()
        .any(|d| d.code == UNRESOLVED_DIAGNOSTIC_REFERENCE_CODE));
    assert!(provided.diagnostics.contains_key("fixture.value"));
}

#[test]
fn reused_diagnostics_and_behaviors_bind_before_dependent_attribute_checks() {
    use cem_ml::schema::document_model::{validate_document_model, EngineDiagnosticBehavior};
    let library = parse(
        r#"{schema @namespace=library | {behaviors | {behavior @name=value-check @implementation=engine @execution=ast-validation @primitive=schema:scalar-type}} {diagnostics | {diagnostic @code=fixture.value @severity=warning @behavior=value-check @message="Shared type violation"}} }"#,
    );
    let source = parse("{schema | {diagnostics | {#diagnostic}} {attributes | {attribute @name=size @type=schema:integer @type-diagnostic=fixture.value}} {behaviors | {#behavior}} {elements | {element @name=input @optional-attributes=size}} }");
    let mut host = Host::new();
    host.outcomes.insert(
        "#behavior".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&library, "behavior")]),
    );
    host.outcomes.insert(
        "#diagnostic".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&library, "diagnostic")]),
    );
    for _ in 0..2 {
        let model = host.resolve(source.clone());
        assert!(model.is_ready_for_validation());
        assert!(
            model.compile_diagnostics.is_empty(),
            "{:?}",
            model.compile_diagnostics
        );
        assert_eq!(
            model.diagnostic_behaviors["fixture.value"].engine_behavior,
            Some(EngineDiagnosticBehavior::ScalarType)
        );
        assert_eq!(model.behaviors["value-check"].schema_uri, "library");
        let diagnostics = validate_document_model(&parse("{input @size=no}"), &model);
        assert!(
            diagnostics.iter().any(|d| d.code == "fixture.value"
                && d.severity == Severity::Warning
                && d.message.contains("Shared type violation")),
            "{diagnostics:?}"
        );
        let native_diagnostics = validate_document_model(&parse("{input @size={#nodes}}"), &model);
        assert_eq!(native_diagnostics.len(), 1);
        assert_eq!(native_diagnostics[0].code, "fixture.value");
        assert_eq!(native_diagnostics[0].severity, Severity::Warning);
        assert!(native_diagnostics[0]
            .message
            .contains("Shared type violation"));
        for site in &model.declaration_references.sites {
            assert!(site
                .resolution
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .all(|n| Arc::ptr_eq(n.document(), &library)));
        }
    }
    assert!(source
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
}
#[test]
fn pending_behavior_and_diagnostic_collections_do_not_invent_missing_dependencies() {
    let source = "{schema | {behaviors | {#behavior}} {diagnostics | {#diagnostic} {diagnostic @code=local @behavior=later}} {attributes | {attribute @name=size @type=schema:integer @type-diagnostic=fixture.value}} }";
    let model = compile_schema_document_model("consumer", source);
    assert_eq!(model.declaration_references.sites.len(), 2);
    assert!(!model.is_ready_for_validation());
    assert!(
        model.compile_diagnostics.is_empty(),
        "{:?}",
        model.compile_diagnostics
    );
    let mut host = Host::new();
    host.outcomes.insert(
        "#behavior".into(),
        ReferenceLinkEvaluation::Resolved(vec![]),
    );
    host.outcomes.insert(
        "#diagnostic".into(),
        ReferenceLinkEvaluation::Resolved(vec![]),
    );
    let complete = host.resolve(parse(source));
    assert!(complete.declaration_references.is_complete());
    assert!(complete
        .compile_diagnostics
        .iter()
        .any(|d| d.code == cem_ml::schema::document_model::UNKNOWN_DIAGNOSTIC_BEHAVIOR_CODE));
    assert!(complete
        .compile_diagnostics
        .iter()
        .any(|d| d.code == cem_ml::schema::document_model::UNRESOLVED_DIAGNOSTIC_REFERENCE_CODE));
}
#[test]
fn diagnostic_and_behavior_reference_order_preserves_local_overrides_and_empty_selections() {
    let library=parse("{schema | {behaviors | {behavior @name=value @implementation=engine @execution=ast-validation @primitive=schema:scalar-type}} {diagnostics | {diagnostic @code=fixture.value @behavior=value @severity=warning}} }");
    let mut host = Host::new();
    host.outcomes.insert(
        "#behavior".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&library, "behavior")]),
    );
    host.outcomes.insert(
        "#diagnostic".into(),
        ReferenceLinkEvaluation::Resolved(vec![
            node(&library, "diagnostic"),
            node(&library, "diagnostic"),
        ]),
    );
    host.outcomes
        .insert("#empty".into(), ReferenceLinkEvaluation::Resolved(vec![]));
    let model=host.resolve(parse("{schema | {behaviors | {behavior @name=value @implementation=engine @execution=ast-validation @primitive=schema:field-contract} {#behavior} {#empty}} {diagnostics | {#diagnostic} {diagnostic @code=fixture.value @behavior=value @severity=error}} }"));
    assert!(
        model.compile_diagnostics.is_empty(),
        "{:?}",
        model.compile_diagnostics
    );
    assert_eq!(
        model.behaviors["value"].primitive.as_deref(),
        Some("schema:scalar-type")
    );
    assert_eq!(model.diagnostics["fixture.value"].severity, Severity::Error);
    assert_eq!(
        model.diagnostic_behaviors["fixture.value"].severity,
        Severity::Error
    );
    assert_eq!(
        model.declaration_references.sites[2]
            .resolution
            .as_ref()
            .unwrap()
            .nodes
            .len(),
        2
    );
}
#[test]
fn diagnostic_and_behavior_targets_require_correct_kinds_keys_and_required_fields() {
    for (collection, kind, bad) in [
        ("behaviors", "behavior", "{behavior @name=value}"),
        (
            "behaviors",
            "behavior",
            "{behavior @implementation=engine @execution=ast-validation}",
        ),
        ("diagnostics", "diagnostic", "{diagnostic}"),
        ("diagnostics", "diagnostic", "{diagnostic @code=\" \"}"),
    ] {
        for disposition in ["neutral", "mandatory", "warning", "ignore"] {
            let mut host = Host::new();
            host.disposition(disposition);
            let target = parse(bad);
            host.outcomes.insert(
                "#target".into(),
                ReferenceLinkEvaluation::Resolved(vec![node(&target, kind)]),
            );
            let model = host.resolve(parse(&format!(
                "{{schema | {{{collection} | {{#target}}}} }}"
            )));
            assert_eq!(
                model.declaration_references.state(),
                ReferenceResolutionState::Invalid
            );
            assert!(model.compile_diagnostics.iter().any(
                |d| d.code == cem_ml::schema::declaration_references::INVALID_REFERENCE_TARGET
            ));
        }
    }
    let mut host = Host::new();
    let target = parse("{element @name=wrong}");
    host.outcomes.insert(
        "#target".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&target, "element")]),
    );
    assert_eq!(
        host.resolve(parse("{schema | {diagnostics | {#target}} }"))
            .declaration_references
            .state(),
        ReferenceResolutionState::Invalid
    );
}

#[test]
fn referenced_diagnostics_keep_declaring_aliases_and_original_binding_errors() {
    let library = parse(
        r#"{schema | {uses | {use @schema="https://cem.dev/ns/schema/1" @as=origin}} {diagnostics | {diagnostic @code=fixture.value @behavior=origin:scalar-type}} }"#,
    );
    let mut host = Host::new();
    host.outcomes.insert(
        "#diagnostic".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&library, "diagnostic")]),
    );
    let model = host.resolve(parse(
        "{schema | {uses | {use @schema=missing @as=origin}} {diagnostics | {#diagnostic}} }",
    ));
    assert!(
        model.compile_diagnostics.is_empty(),
        "{:?}",
        model.compile_diagnostics
    );
    assert_eq!(
        model.diagnostic_behaviors["fixture.value"].engine_behavior,
        Some(cem_ml::schema::document_model::EngineDiagnosticBehavior::ScalarType)
    );
    let bad=parse("{schema | {uses | {use @schema=external @as=origin}} {diagnostics | {diagnostic @code=bad @behavior=origin:missing}} }");
    host.outcomes.insert(
        "#diagnostic".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&bad, "diagnostic")]),
    );
    let errors=host.resolve(parse("{schema | {uses | {use @schema=consumer @as=origin}} {diagnostics | {#diagnostic}} {behaviors | {#pending}} }"));
    assert!(!errors.is_ready_for_validation());
    assert!(errors.compile_diagnostics.iter().any(|d| d.code
        == cem_ml::schema::document_model::UNKNOWN_DIAGNOSTIC_BEHAVIOR_CODE
        && d.message.contains("origin:missing")));
}
#[test]
fn reused_function_behavior_keeps_its_inline_function_and_source_metadata() {
    let text = r#"{schema @namespace=library | {uses | {use @schema="https://cem.dev/ns/schema/1" @as=origin}} {behaviors | {behavior @name=check @implementation=function @execution=ast-validation @function=result @select=input @match=true | {inputs | {input-binding @name=candidate @type=schema:node @source=candidate @required=true}} {result @type=schema:diagnostic-result @severity=diagnostic @message=function @source-range=candidate} {function @name=result @returns=object | {param @name=candidate @type=object @required=true} {body | {$ {message: "Shared failure", details: {checkKind: "shared"}}}}}}} {diagnostics | {diagnostic @code=fixture.function @behavior=check @severity=warning}} }"#;
    let library = parse(text);
    let expected = compile_schema_document_model("library", text);
    assert!(
        expected.compile_diagnostics.is_empty(),
        "{:?}",
        expected.compile_diagnostics
    );
    let mut host = Host::new();
    host.outcomes.insert(
        "#behavior".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&library, "behavior")]),
    );
    host.outcomes.insert(
        "#diagnostic".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&library, "diagnostic")]),
    );
    let model = host.resolve(parse(
        "{schema | {behaviors | {#behavior}} {diagnostics | {#diagnostic}} }",
    ));
    assert!(
        model.compile_diagnostics.is_empty(),
        "{:?}",
        model.compile_diagnostics
    );
    assert_eq!(model.behaviors["check"], expected.behaviors["check"]);
    assert_eq!(
        model.diagnostic_behaviors["fixture.function"],
        expected.diagnostic_behaviors["fixture.function"]
    );
}

#[test]
fn incomplete_behavior_links_preserve_partial_bindings_and_all_original_errors() {
    let library=parse("{schema | {behaviors | {behavior @name=known @implementation=engine @execution=ast-validation @primitive=schema:scalar-type}} }");
    let missing = reference("missing");
    let link = missing
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Reference { node_id, .. } => {
                SchemaDeclarationNode::new(missing.clone(), *node_id)
            }
            _ => None,
        })
        .unwrap();
    for disposition in ["neutral", "mandatory", "warning", "ignore"] {
        let mut host = Host::new();
        host.disposition(disposition);
        host.outcomes.insert(
            "#behavior".into(),
            ReferenceLinkEvaluation::Resolved(vec![node(&library, "behavior"), link.clone()]),
        );
        let model=host.resolve(parse("{schema | {behaviors | {#behavior}} {diagnostics | {diagnostic @code=known @behavior=known} {diagnostic @code=later @behavior=later}} }"));
        assert!(!model.is_ready_for_validation());
        assert!(model.diagnostic_behaviors.contains_key("known"));
        assert!(!model.diagnostic_behaviors.contains_key("later"));
        assert_eq!(
            model.declaration_references.failed(),
            disposition == "mandatory"
        );
        assert_eq!(
            model.compile_diagnostics.len(),
            usize::from(matches!(disposition, "mandatory" | "warning"))
        );
    }
    let mut host = Host::new();
    host.disposition("ignore");
    let original = Diagnostic {
        code: cem_ml::schema::document_model::UNKNOWN_DIAGNOSTIC_BEHAVIOR_CODE.into(),
        severity: Severity::Error,
        message: "original evaluator failure".into(),
        details: Some(serde_json::json!({"behavior":"missing"})),
        ..Default::default()
    };
    host.outcomes.insert(
        "#invalid".into(),
        ReferenceLinkEvaluation::Invalid(vec![original.clone()]),
    );
    let model = host.resolve(parse(
        "{schema | {diagnostics | {#invalid}} {behaviors | {#pending}} }",
    ));
    assert!(model.compile_diagnostics.iter().any(|d| d == &original));
    assert!(!model.is_ready_for_validation());
    let known=compile_schema_document_model("consumer", "{schema | {behaviors | {#pending} {behavior @name=known @implementation=engine @execution=ast-validation @primitive=schema:scalar-type}} {diagnostics | {diagnostic @code=invalid @behavior=known | {arguments | {argument @name=bad @value=bad}}}} {attributes | {attribute @name=size @type=schema:integer @default=no}} }");
    assert!(known.compile_diagnostics.iter().any(
        |d| d.code == cem_ml::schema::document_model::INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE
    ));
    assert!(known
        .compile_diagnostics
        .iter()
        .any(|d| d.code == cem_ml::schema::document_model::INVALID_SCHEMA_DEFAULT_VALUE_CODE));
}

#[test]
fn constraint_references_preserve_order_owners_and_nested_metadata() {
    let library = parse(
        r#"{schema @namespace="library" |
      {constraints | {constraint @kind="related" @behavior="schema:reference-resolution" |
        {candidates @select="items" @as="item"}
      }}}
    }"#,
    );
    let selected = node(&library, "constraint");
    let mut host = Host::new();
    host.outcomes.insert(
        "#rules".into(),
        ReferenceLinkEvaluation::Resolved(vec![selected.clone()]),
    );
    let model = host.resolve(parse(
        r#"{schema | {constraints |
      {constraint @kind="related" @value="old"} {#rules}
    }}"#,
    ));
    assert!(model.is_ready_for_validation());
    let rule = &model.constraints["related"];
    assert_eq!(rule.value, None);
    assert_eq!(
        rule.reference_resolution.as_ref().unwrap().candidates[0]
            .select
            .as_deref(),
        Some("items")
    );
    let retained = &model.declaration_references.sites[0]
        .resolution
        .as_ref()
        .unwrap()
        .nodes[0];
    assert!(Arc::ptr_eq(retained.document(), &library));
}

#[test]
fn constraint_references_reject_wrong_kinds_and_missing_keys() {
    for source in [
        "{schema | {constraints | {constraint}}}",
        "{schema | {elements | {element @name=box}}}",
    ] {
        let library = parse(source);
        let kind = if source.contains("elements") {
            "element"
        } else {
            "constraint"
        };
        let mut host = Host::new();
        host.outcomes.insert(
            "#rules".into(),
            ReferenceLinkEvaluation::Resolved(vec![node(&library, kind)]),
        );
        let model = host.resolve(parse("{schema | {constraints | {#rules}}}"));
        assert!(!model.is_ready_for_validation());
        assert!(model
            .compile_diagnostics
            .iter()
            .any(|d| d.code == cem_ml::schema::declaration_references::INVALID_REFERENCE_TARGET));
    }
}

#[test]
fn referenced_policy_duplicates_are_errors_and_do_not_change_current_budget() {
    let library = parse(
        r#"{schema | {constraints |
      {constraint @kind="reference-traversal-work" @value="1"}
    }}"#,
    );
    let mut host = Host::new();
    let limits = host.policy.limits;
    host.outcomes.insert(
        "#policy".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&library, "constraint")]),
    );
    let model = host.resolve(parse(
        r#"{schema | {constraints |
      {constraint @kind="reference-traversal-work" @value="10"} {#policy}
    }}"#,
    ));
    assert_eq!(
        model.constraints["reference-traversal-work"]
            .value
            .as_deref(),
        Some("10")
    );
    assert!(model
        .compile_diagnostics
        .iter()
        .any(|d| d.code == "cem.schema.reference_policy_duplicate"));
    assert_eq!(host.policy.limits, limits);
}

#[test]
fn pending_constraint_references_keep_source_only_candidates_inactive() {
    let model = compile_schema_document_model("consumer", "{schema | {constraints | {#rules}}}");
    assert!(!model.is_ready_for_validation());
    assert_eq!(model.declaration_references.sites.len(), 1);
    let mut host = Host::new();
    host.outcomes.insert(
        "#rules".into(),
        ReferenceLinkEvaluation::Pending("runtime input unavailable".into()),
    );
    let candidate = host.resolve(parse("{schema | {constraints | {#rules}}}"));
    assert!(!candidate.is_ready_for_validation());
}

#[test]
fn reused_constraints_bind_local_behaviors_and_preserve_declaring_alias_errors() {
    let mut host = Host::new();
    let library = parse(
        r#"{schema | {uses | {use @schema="https://cem.dev/ns/schema/1" @as=origin}}
      {constraints | {constraint @kind=check @behavior=origin:scalar-type @diagnostic=fixture.value}} }"#,
    );
    host.outcomes.insert(
        "#rules".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&library, "constraint")]),
    );
    let model = host.resolve(parse(
        "{schema | {uses | {use @schema=missing @as=origin}} {constraints | {#rules}}}",
    ));
    assert!(model.constraints["check"].definition.is_some());
    let bad = parse("{schema | {uses | {use @schema=external @as=origin}} {constraints | {constraint @kind=check @behavior=origin:missing @diagnostic=fixture.value}} }");
    host.outcomes.insert(
        "#rules".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&bad, "constraint")]),
    );
    host.outcomes.insert(
        "#pending".into(),
        ReferenceLinkEvaluation::Pending("waiting".into()),
    );
    let model = host.resolve(parse("{schema | {uses | {use @schema=consumer @as=origin}} {constraints | {#rules}} {behaviors | {#pending}}}"));
    assert!(model
        .compile_diagnostics
        .iter()
        .any(|d| d.code == cem_ml::schema::document_model::UNKNOWN_DIAGNOSTIC_BEHAVIOR_CODE));
    let shared = parse("{schema | {constraints | {constraint @kind=check @behavior=local @diagnostic=fixture.value}} }");
    host.outcomes.insert(
        "#rules".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&shared, "constraint")]),
    );
    for primitive in ["schema:scalar-type", "schema:value-vocabulary"] {
        let model = host.resolve(parse(&"{schema | {constraints | {#rules}} {behaviors | {behavior @name=local @implementation=engine @execution=ast-validation @primitive=PRIMITIVE}}}".replace("PRIMITIVE", primitive)));
        assert_eq!(
            model.constraints["check"]
                .definition
                .as_ref()
                .unwrap()
                .primitive
                .as_deref(),
            Some(primitive)
        );
        assert!(Arc::ptr_eq(
            model.declaration_references.sites[0]
                .resolution
                .as_ref()
                .unwrap()
                .nodes[0]
                .document(),
            &shared
        ));
    }
    let local = parse("{schema | {constraints | {constraint @kind=check @behavior=missing @diagnostic=fixture.value}} }");
    host.outcomes.insert(
        "#rules".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&local, "constraint")]),
    );
    let model = host.resolve(parse(
        "{schema | {constraints | {#rules}} {behaviors | {#pending}}}",
    ));
    assert!(!model
        .compile_diagnostics
        .iter()
        .any(|d| d.code == cem_ml::schema::document_model::UNKNOWN_DIAGNOSTIC_BEHAVIOR_CODE));
}

#[test]
fn field_contract_references_reuse_ordered_applications_without_source_cloning() {
    let library = parse(
        r#"{schema | {field-contracts |
      {field-contract @name=shared @target=box @required-attributes=command |
        {choice @name=mode @mode=exactly-one | {case @attributes=command}}}
    }}"#,
    );
    let selected = node(&library, "field-contract");
    let mut host = Host::new();
    host.outcomes.insert(
        "#contracts".into(),
        ReferenceLinkEvaluation::Resolved(vec![selected.clone(), selected]),
    );
    for attributes in ["command", "command extra"] {
        let model = host.resolve(parse(&"{schema | {elements | {element @name=box @optional-attributes=ATTRS}} {field-contracts | {field-contract @name=first @target=box} {#contracts} {field-contract @name=last @target=box}}}".replace("ATTRS", &format!("\"{attributes}\""))));
        assert!(model.is_ready_for_validation());
        assert!(
            model.compile_diagnostics.is_empty(),
            "{:?}",
            model.compile_diagnostics
        );
        let contracts = &model.elements["box"].field_contracts;
        assert_eq!(
            contracts
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            vec!["first", "shared", "shared", "last"]
        );
        assert_eq!(contracts[1].choice_groups.len(), 1);
        assert!(Arc::ptr_eq(
            model.declaration_references.sites[0]
                .resolution
                .as_ref()
                .unwrap()
                .nodes[0]
                .document(),
            &library
        ));
        let invalid =
            cem_ml::schema::document_model::validate_document_model(&parse("{box}"), &model);
        assert!(invalid.iter().any(|d| d.severity.is_hard_violation()));
    }
}

#[test]
fn field_contract_references_require_kind_name_and_target_keys() {
    for (source, kind) in [
        (
            "{schema | {field-contracts | {field-contract @name=missing}}}",
            "field-contract",
        ),
        (
            "{schema | {field-contracts | {field-contract @target=box}}}",
            "field-contract",
        ),
        (
            "{schema | {constraints | {constraint @kind=check}}}",
            "constraint",
        ),
    ] {
        let library = parse(source);
        let mut host = Host::new();
        host.outcomes.insert(
            "#contracts".into(),
            ReferenceLinkEvaluation::Resolved(vec![node(&library, kind)]),
        );
        let model = host.resolve(parse("{schema | {field-contracts | {#contracts}}}"));
        assert!(!model.is_ready_for_validation());
        assert!(model
            .compile_diagnostics
            .iter()
            .any(|d| d.code == cem_ml::schema::declaration_references::INVALID_REFERENCE_TARGET));
    }
}

#[test]
fn field_contract_missing_targets_error_after_element_assembly_and_defer_while_pending() {
    let library = parse("{schema | {field-contracts | {field-contract @name=shared @target=box}}}");
    let mut host = Host::new();
    host.outcomes.insert(
        "#contracts".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&library, "field-contract")]),
    );
    for collection in ["{field-contract @name=direct @target=box}", "{#contracts}"] {
        let source =
            format!("{{schema | {{elements | {{#elements}}}} {{field-contracts | {collection}}}}}");
        host.outcomes.insert(
            "#elements".into(),
            ReferenceLinkEvaluation::Pending("waiting".into()),
        );
        let pending = host.resolve(parse(&source));
        assert!(!pending.is_ready_for_validation());
        assert!(!pending.compile_diagnostics.iter().any(|d| d
            .details
            .as_ref()
            .and_then(|d| d.get("checkKind"))
            .and_then(|d| d.as_str())
            == Some("field-contract-target")));
        host.outcomes.insert(
            "#elements".into(),
            ReferenceLinkEvaluation::Resolved(vec![]),
        );
        let empty = host.resolve(parse(&source));
        assert!(empty.compile_diagnostics.iter().any(|d| d.code
            == cem_ml::schema::document_model::INVALID_SCHEMA_FIELD_CONTRACT_CODE
            && d.details
                .as_ref()
                .and_then(|d| d.get("target"))
                .and_then(|d| d.as_str())
                == Some("box")));
    }
    let direct = compile_schema_document_model(
        "consumer",
        "{schema | {field-contracts | {field-contract @name=direct @target=box}}}",
    );
    assert!(direct
        .compile_diagnostics
        .iter()
        .any(|d| d.code == cem_ml::schema::document_model::INVALID_SCHEMA_FIELD_CONTRACT_CODE));
    let retained =
        compile_schema_document_model("consumer", "{schema | {field-contracts | {#contracts}}}");
    assert!(!retained.is_ready_for_validation());
}

#[test]
fn field_contract_applications_keep_independent_lexical_alias_diagnostics() {
    let good = parse(
        r#"{schema | {uses | {use @schema="https://cem.dev/ns/schema/1" @as=origin}}
      {field-contracts | {field-contract @name=shared @target=box @behavior=origin:field-contract @diagnostic=fixture.contract}} }"#,
    );
    let bad = parse("{schema | {uses | {use @schema=external @as=origin}} {field-contracts | {field-contract @name=shared @target=box @behavior=origin:missing @diagnostic=fixture.contract | {choice @name=invalid @mode=invalid}}} }");
    let mut host = Host::new();
    host.outcomes.insert(
        "#contracts".into(),
        ReferenceLinkEvaluation::Resolved(vec![
            node(&good, "field-contract"),
            node(&bad, "field-contract"),
        ]),
    );
    host.outcomes.insert(
        "#behaviors".into(),
        ReferenceLinkEvaluation::Pending("waiting".into()),
    );
    host.outcomes.insert(
        "#diagnostics".into(),
        ReferenceLinkEvaluation::Pending("waiting".into()),
    );
    let model = host.resolve(parse("{schema | {uses | {use @schema=consumer @as=origin}} {elements | {element @name=box}} {behaviors | {#behaviors}} {diagnostics | {#diagnostics}} {field-contracts | {#contracts}}}"));
    assert!(!model.is_ready_for_validation());
    let contracts = &model.elements["box"].field_contracts;
    assert_eq!(contracts.len(), 2);
    assert!(contracts[0].definition.is_some());
    assert!(contracts[1].definition.is_none());
    assert_eq!(
        model
            .compile_diagnostics
            .iter()
            .filter(|d| d.code == cem_ml::schema::document_model::UNKNOWN_DIAGNOSTIC_BEHAVIOR_CODE)
            .count(),
        1
    );
    assert!(model
        .compile_diagnostics
        .iter()
        .any(|d| d.code == cem_ml::schema::document_model::INVALID_SCHEMA_FIELD_CONTRACT_CODE));
    assert!(!model
        .compile_diagnostics
        .iter()
        .any(|d| d.code == cem_ml::schema::document_model::UNRESOLVED_DIAGNOSTIC_REFERENCE_CODE));
}

#[test]
fn structural_input_references_validate_parent_sequences_and_consuming_subtrees() {
    use cem_ml::schema::input_references::validate_structural_input_references;
    let model = compile_schema_document_model("consumer", "{schema | {elements | {element @name=box @children=item} {element @name=item @required-attributes=command}} {field-contracts | {field-contract @name=pair @target=box @exact-child-sequence=\"item item\"}}}");
    let library = parse("{schema | {item}} ");
    let item = node(&library, "item");
    let mut host = Host::new();
    host.outcomes.insert(
        "#items".into(),
        ReferenceLinkEvaluation::Resolved(vec![item.clone(), item]),
    );
    let source = parse("{box | {#items}}");
    let limits = host.policy.limits;
    let report =
        validate_structural_input_references(source.clone(), &model, &mut host, limits).unwrap();
    assert!(report.complete);
    assert_eq!(
        report
            .diagnostics
            .iter()
            .filter(|d| d.code == cem_ml::schema::document_model::MISSING_REQUIRED_ATTRIBUTE_CODE)
            .count(),
        2
    );
    assert!(!report.diagnostics.iter().any(|d| d
        .details
        .as_ref()
        .and_then(|d| d.get("contract"))
        .and_then(|d| d.as_str())
        == Some("pair")));
    assert!(report.nodes.iter().filter(|n| matches!(n.source.node(), CemAstNode::Element {expanded_name, ..} if expanded_name.local_name == "item")).all(|n| Arc::ptr_eq(n.source.document(), &library)));
    assert!(source
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
}

#[test]
fn structural_input_pending_children_are_not_empty_and_invalid_kinds_are_errors() {
    use cem_ml::schema::input_references::validate_structural_input_references;
    let model = compile_schema_document_model("consumer", "{schema | {elements | {element @name=box @children=item @required-attributes=own} {element @name=item}} {field-contracts | {field-contract @name=required @target=box @required-children=item}}}");
    let source = parse("{box | {#items}}");
    let mut host = Host::new();
    host.outcomes.insert(
        "#items".into(),
        ReferenceLinkEvaluation::Pending("waiting".into()),
    );
    let limits = host.policy.limits;
    let pending =
        validate_structural_input_references(source.clone(), &model, &mut host, limits).unwrap();
    assert!(!pending.complete);
    assert!(pending
        .diagnostics
        .iter()
        .any(|d| d.code == cem_ml::schema::document_model::MISSING_REQUIRED_ATTRIBUTE_CODE));
    assert!(!pending.diagnostics.iter().any(|d| d
        .details
        .as_ref()
        .and_then(|d| d.get("contract"))
        .and_then(|d| d.as_str())
        == Some("required")));
    host.outcomes
        .insert("#items".into(), ReferenceLinkEvaluation::Resolved(vec![]));
    let empty =
        validate_structural_input_references(source.clone(), &model, &mut host, limits).unwrap();
    assert!(empty.complete);
    assert!(empty.diagnostics.len() > pending.diagnostics.len());
    host.outcomes.insert(
        "#items".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&parse("{box @bad=value}"), "box")]),
    );
    let invalid_child =
        validate_structural_input_references(source.clone(), &model, &mut host, limits).unwrap();
    assert!(invalid_child
        .diagnostics
        .iter()
        .any(|d| d.code == cem_ml::schema::document_model::INVALID_CHILD_ELEMENT_CODE));
    let document = parse("{box @bad=value}");
    let attribute = document
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Attribute { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .unwrap();
    host.outcomes.insert(
        "#items".into(),
        ReferenceLinkEvaluation::Resolved(vec![
            SchemaDeclarationNode::new(document, attribute).unwrap()
        ]),
    );
    let invalid = validate_structural_input_references(source, &model, &mut host, limits).unwrap();
    assert!(!invalid.complete && invalid.failed);
}

#[test]
fn structural_input_element_reference_cycles_stay_bounded_under_ignore() {
    use cem_ml::schema::input_references::validate_structural_input_references;
    let model = compile_schema_document_model("consumer", "{schema | {elements | {element @name=box @children=item} {element @name=item @children=item}}}");
    let target = parse("{item | {#again}}");
    let mut host = Host::new();
    host.disposition("ignore");
    host.outcomes.insert(
        "#items".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&target, "item")]),
    );
    host.outcomes.insert(
        "#again".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&target, "item")]),
    );
    let limits = host.policy.limits;
    let report =
        validate_structural_input_references(parse("{box | {#items}}"), &model, &mut host, limits)
            .unwrap();
    assert!(!report.complete);
    assert!(report.references[0].resolution.issues.iter().any(
        |i| i.kind == cem_ml::value::reference_resolution::ReferenceResolutionIssueKind::Cycle
    ));
    assert!(host.calls <= 3);
}

#[test]
fn pending_descendants_preserve_complete_parent_children_and_inactive_models_skip_evaluation() {
    use cem_ml::schema::input_references::validate_structural_input_references;
    let model = compile_schema_document_model("consumer", "{schema | {elements | {element @name=box @children=item} {element @name=item @children=leaf} {element @name=leaf}} {field-contracts | {field-contract @name=outer @target=box @exact-total-children=0} {field-contract @name=inner @target=item @required-children=leaf}}}");
    let library = parse("{item | {#children}}");
    let source = parse("{box | {#items}}");
    let mut host = Host::new();
    host.outcomes.insert(
        "#items".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&library, "item")]),
    );
    host.outcomes.insert(
        "#children".into(),
        ReferenceLinkEvaluation::Pending("waiting".into()),
    );
    let limits = host.policy.limits;
    let report =
        validate_structural_input_references(source.clone(), &model, &mut host, limits).unwrap();
    assert!(!report.complete);
    assert!(report.nodes.iter().find(|n|matches!(n.source.node(), CemAstNode::Element {expanded_name,..} if expanded_name.local_name == "box")).unwrap().children_complete);
    assert!(report.diagnostics.iter().any(|d| d
        .details
        .as_ref()
        .and_then(|d| d.get("contract"))
        .and_then(|d| d.as_str())
        == Some("outer")));
    assert!(!report.diagnostics.iter().any(|d| d
        .details
        .as_ref()
        .and_then(|d| d.get("contract"))
        .and_then(|d| d.as_str())
        == Some("inner")));
    let inactive = compile_schema_document_model("consumer", "{schema | {elements | {#pending}}}");
    let calls = host.calls;
    let report =
        validate_structural_input_references(source, &inactive, &mut host, limits).unwrap();
    assert!(!report.complete && report.failed);
    assert_eq!(host.calls, calls);
}

#[derive(Debug)]
struct RetainedBehavior {
    owner: Arc<CemDocument>,
    calls: std::sync::atomic::AtomicUsize,
}
impl cem_ml::schema::document_model::SchemaBehaviorEvaluator for RetainedBehavior {
    fn validate_document(
        &self,
        _: &CemDocument,
        _: &cem_ml::schema::document_model::SchemaDocumentModel,
    ) -> Vec<Diagnostic> {
        panic!("retained structure must not fall back to whole-document validation")
    }
    fn validate_retained_structure(
        &self,
        structure: cem_ml::schema::input_references::RetainedValidationStructure<'_>,
        _: &cem_ml::schema::document_model::SchemaDocumentModel,
    ) -> cem_ml::schema::input_references::RetainedBehaviorValidation {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(structure.roots, &[0]);
        let root = &structure.nodes[0];
        let placements: Vec<_> = root.children.iter().copied().filter(|id| matches!(structure.nodes[*id].source.node(), CemAstNode::Element {expanded_name, ..} if expanded_name.local_name == "item")).collect();
        assert_eq!(placements.len(), 2);
        let first = &structure.nodes[placements[0]];
        let second = &structure.nodes[placements[1]];
        assert!(Arc::ptr_eq(first.source.document(), &self.owner));
        assert!(Arc::ptr_eq(second.source.document(), &self.owner));
        assert_eq!(first.source.node_id(), second.source.node_id());
        assert_ne!(placements[0], placements[1]);
        assert!(Arc::ptr_eq(
            first.declaring_schema.as_ref().unwrap().document(),
            &self.owner
        ));
        assert!(
            matches!(first.declaring_schema.as_ref().unwrap().node(), CemAstNode::Element {expanded_name,..} if expanded_name.local_name == "schema")
        );
        assert!(first.children_complete);
        cem_ml::schema::input_references::RetainedBehaviorValidation {
            complete: true,
            diagnostics: vec![Diagnostic {
                code: "fixture.retained_behavior".into(),
                severity: Severity::Error,
                message: "retained behavior violation".into(),
                node: Some(first.source.identity()),
                source_map: Some(match first.source.node() {
                    CemAstNode::Element { source, .. } => source.clone(),
                    _ => unreachable!(),
                }),
                ..Default::default()
            }],
        }
    }
}

#[test]
fn retained_behavior_handoff_preserves_repeated_placements_and_declaring_owners() {
    use cem_ml::schema::input_references::validate_structural_input_references_with_behavior_evaluator;
    let source = parse("{box | {#items}}");
    let library = parse("{schema | {item}}");
    let target = node(&library, "item");
    let mut host = Host::new();
    host.outcomes.insert(
        "#items".into(),
        ReferenceLinkEvaluation::Resolved(vec![target.clone(), target]),
    );
    let evaluator = RetainedBehavior {
        owner: library,
        calls: Default::default(),
    };
    let model = compile_schema_document_model(
        "consumer",
        "{schema | {elements | {element @name=box @children=item} {element @name=item}}}",
    );
    let limits = host.policy.limits;
    let report = validate_structural_input_references_with_behavior_evaluator(
        source.clone(),
        &model,
        &mut host,
        limits,
        Some(&evaluator),
    )
    .unwrap();
    assert!(report.complete);
    assert!(report.failed);
    assert_eq!(evaluator.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.code == "fixture.retained_behavior"));
    assert!(Arc::ptr_eq(&report.source, &source));
    assert!(source
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
}

#[test]
fn pending_retained_structure_defers_behavior_without_losing_structural_diagnostics() {
    use cem_ml::schema::input_references::validate_structural_input_references_with_behavior_evaluator;
    let source = parse("{box @bad=value | {#items}}");
    let mut host = Host::new();
    host.outcomes.insert(
        "#items".into(),
        ReferenceLinkEvaluation::Pending("runtime-unavailable".into()),
    );
    let evaluator = RetainedBehavior {
        owner: parse("{schema | {item}}"),
        calls: Default::default(),
    };
    let model = compile_schema_document_model(
        "consumer",
        "{schema | {elements | {element @name=box @children=item} {element @name=item}}}",
    );
    let limits = host.policy.limits;
    let report = validate_structural_input_references_with_behavior_evaluator(
        source,
        &model,
        &mut host,
        limits,
        Some(&evaluator),
    )
    .unwrap();
    assert!(!report.complete);
    assert!(report.failed);
    assert_eq!(evaluator.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(
        host.schema_calls.load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.code == cem_ml::schema::document_model::UNKNOWN_ATTRIBUTE_CODE));
}

#[test]
fn legacy_behavior_evaluators_leave_retained_behavior_validation_incomplete() {
    use cem_ml::schema::input_references::validate_structural_input_references_with_behavior_evaluator;
    #[derive(Debug)]
    struct Legacy;
    impl cem_ml::schema::document_model::SchemaBehaviorEvaluator for Legacy {
        fn validate_document(
            &self,
            _: &CemDocument,
            _: &cem_ml::schema::document_model::SchemaDocumentModel,
        ) -> Vec<Diagnostic> {
            panic!("no synthetic whole-document fallback")
        }
    }
    let model =
        compile_schema_document_model("consumer", "{schema | {elements | {element @name=box}}}");
    let mut host = Host::new();
    let limits = host.policy.limits;
    let source = parse("{box}");
    let report = validate_structural_input_references_with_behavior_evaluator(
        source.clone(),
        &model,
        &mut host,
        limits,
        Some(&Legacy),
    )
    .unwrap();
    assert!(!report.complete);
    assert!(!report.failed);
    assert!(report.diagnostics.is_empty());
    let structural = validate_structural_input_references_with_behavior_evaluator(
        source, &model, &mut host, limits, None,
    )
    .unwrap();
    assert!(structural.complete);
}

#[test]
fn structural_only_validation_does_not_request_declaring_schema_lookup() {
    use cem_ml::schema::input_references::validate_structural_input_references;
    let source = parse("{box | {#items}}");
    let library = parse("{schema | {item}}");
    let mut host = Host::new();
    host.outcomes.insert(
        "#items".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&library, "item")]),
    );
    let model = compile_schema_document_model(
        "consumer",
        "{schema | {elements | {element @name=box @children=item} {element @name=item}}}",
    );
    let limits = host.policy.limits;
    let report = validate_structural_input_references(source, &model, &mut host, limits).unwrap();
    assert!(report.complete);
    assert_eq!(
        host.schema_calls.load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    assert!(report
        .nodes
        .iter()
        .all(|node| node.declaring_schema.is_none()));
}

#[test]
fn native_attribute_contract_rejects_primitive_and_untyped_inputs_without_empty_coercion() {
    use cem_ml::schema::document_model::{validate_document_model, INVALID_ATTRIBUTE_TYPE_CODE};
    for ty in [
        "schema:string",
        "schema:integer",
        "schema:boolean",
        "schema:number",
        "",
    ] {
        let type_attr = if ty.is_empty() {
            String::new()
        } else {
            format!("@type={ty}")
        };
        let model = compile_schema_document_model("consumer", &format!("{{schema | {{elements | {{element @name=box @required-attributes=target}} }} {{attributes | {{attribute @name=target {type_attr} @values=accepted}} }} }}"));
        let source = parse("{box @target={#nodes}}");
        let diagnostics = validate_document_model(&source, &model);
        assert_eq!(diagnostics.len(), 1, "{ty}: {diagnostics:?}");
        assert_eq!(diagnostics[0].code, INVALID_ATTRIBUTE_TYPE_CODE);
        assert!(diagnostics[0].message.contains("native"));
        assert_eq!(
            diagnostics[0].source_map.as_ref(),
            if let CemAstNode::Attribute { source, .. } = source.get(2).unwrap() {
                Some(source)
            } else {
                None
            }
        );
    }
    let model = compile_schema_document_model("consumer", "{schema | {elements | {element @name=box @required-attributes=target}} {attributes | {attribute @name=target @type=schema:string}}}");
    assert!(validate_document_model(&parse(r#"{box @target="{#nodes}"}"#), &model).is_empty());
}
#[test]
fn native_attribute_node_contract_rejects_literal_values_conversion_and_defaults() {
    use cem_ml::schema::document_model::{
        convert_attribute_value, validate_document_model, AttributeValueContract,
        INVALID_ATTRIBUTE_TYPE_CODE,
    };
    let model = compile_schema_document_model("consumer", "{schema | {elements | {element @name=box @required-attributes=target}} {attributes | {attribute @name=target @type=schema:node}}}");
    let contract = AttributeValueContract {
        model: model.attributes["target"].clone(),
        ..Default::default()
    };
    assert!(contract.is_node_valued());
    for source in [
        "{box @target=literal}",
        "{box @target}",
        r#"{box @target="{#nodes}"}"#,
    ] {
        let diagnostics = validate_document_model(&parse(source), &model);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].code, INVALID_ATTRIBUTE_TYPE_CODE);
    }
    assert!(convert_attribute_value("{#nodes}", &contract, &Default::default()).is_err());
    assert!(validate_document_model(&parse("{box @target={#nodes}}"), &model).is_empty());
    let invalid = compile_schema_document_model("consumer", "{schema | {elements | {element @name=box @required-attributes=target}} {attributes | {attribute @name=target @type=schema:node @default=literal}}}");
    assert!(invalid
        .compile_diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code
            == cem_ml::schema::document_model::INVALID_SCHEMA_DEFAULT_VALUE_CODE
            && diagnostic.severity.is_hard_violation()));
}
#[test]
fn native_attribute_pending_values_preserve_structural_and_presence_checks() {
    use cem_ml::schema::input_references::validate_structural_input_references;
    let model = compile_schema_document_model(
        "consumer",
        r#"{schema | {elements | {element @name=box @required-attributes=target @children=item} {element @name=item}} {attributes | {attribute @name=target @type=schema:node}} {field-contracts | {field-contract @name=present @target=box @when-present-attributes=target @required-children=item} {field-contract @name=absent @target=box @when-absent-attributes=target @required-attributes=unexpected}}}"#,
    );
    for expression in ["{#nodes}", "{1 + 2}"] {
        let source = parse(&format!("{{box @target={expression}}}"));
        let mut host = Host::new();
        let limits = host.policy.limits;
        let report =
            validate_structural_input_references(source.clone(), &model, &mut host, limits)
                .unwrap();
        assert!(!report.complete);
        assert!(report.failed);
        assert_eq!(host.calls, usize::from(expression.starts_with("{#")));
        assert!(report.diagnostics.iter().any(|d| d
            .details
            .as_ref()
            .is_some_and(|details| details["contract"] == "present")));
        assert!(!report.diagnostics.iter().any(|d| d
            .details
            .as_ref()
            .is_some_and(|details| details["contract"] == "absent")));
        assert!(Arc::ptr_eq(report.nodes[0].source.document(), &source));
        assert!(source
            .nodes
            .iter()
            .filter_map(|node| if let CemAstNode::Reference { targets, .. } = node {
                Some(targets)
            } else {
                None
            })
            .all(Option::is_none));
    }
}
#[test]
fn native_attribute_source_only_behavior_waits_for_an_explicit_consumer() {
    use cem_ml::schema::document_model::{
        validate_document_model_with_behavior_evaluator, SchemaBehaviorEvaluator,
        SchemaDocumentModel,
    };
    #[derive(Debug, Default)]
    struct Behavior(std::sync::atomic::AtomicUsize);
    impl SchemaBehaviorEvaluator for Behavior {
        fn validate_document(&self, _: &CemDocument, _: &SchemaDocumentModel) -> Vec<Diagnostic> {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            vec![]
        }
    }
    let model = compile_schema_document_model("consumer", "{schema | {elements | {element @name=box @required-attributes=target}} {attributes | {attribute @name=target @type=schema:node}}}");
    let behavior = Behavior::default();
    for expression in ["{#nodes}", "{1 + 2}"] {
        validate_document_model_with_behavior_evaluator(
            &parse(&format!("{{box @target={expression}}}")),
            &model,
            Some(&behavior),
        );
    }
    assert_eq!(behavior.0.load(std::sync::atomic::Ordering::SeqCst), 0);
    let literal_model = compile_schema_document_model("consumer", "{schema | {elements | {element @name=box @required-attributes=target}} {attributes | {attribute @name=target @type=schema:string}}}");
    validate_document_model_with_behavior_evaluator(
        &parse("{box @target=literal}"),
        &literal_model,
        Some(&behavior),
    );
    assert_eq!(behavior.0.load(std::sync::atomic::Ordering::SeqCst), 1);
}
fn native_attribute(document: &Arc<CemDocument>) -> SchemaDeclarationNode {
    let id = document.nodes.iter().position(|node| matches!(node, CemAstNode::Attribute { expanded_name, .. } if expanded_name.local_name == "target")).unwrap();
    SchemaDeclarationNode::new(document.clone(), id as u32).unwrap()
}
fn native_attribute_model(facets: &str) -> cem_ml::schema::document_model::SchemaDocumentModel {
    compile_schema_document_model("consumer", &format!("{{schema | {{elements | {{element @name=box @required-attributes=target}} }} {{attributes | {{attribute @name=target @type=schema:node {facets}}} }} }}"))
}
#[test]
fn native_attribute_reference_counts_original_targets_under_explicit_envelopes() {
    use cem_ml::schema::attribute_references::validate_native_attribute_reference;
    let source = parse("{box @target={#nodes}}");
    let library = parse("{schema | {item}}");
    for (facets, count, failed) in [
        ("", 0, true),
        ("", 1, false),
        ("", 2, true),
        ("@itemCount=0", 0, false),
        ("@minItems=0 @maxItems=2", 2, false),
        ("@minItems=0", 0, false),
        ("@maxItems=1", 2, true),
    ] {
        let model = native_attribute_model(facets);
        assert!(
            model.compile_diagnostics.is_empty(),
            "{facets}: {:?}",
            model.compile_diagnostics
        );
        let mut host = Host::new();
        host.outcomes.insert(
            "#nodes".into(),
            ReferenceLinkEvaluation::Resolved(vec![node(&library, "item"); count]),
        );
        let limits = host.policy.limits;
        let report = validate_native_attribute_reference(
            native_attribute(&source),
            &model,
            "box",
            &mut host,
            limits,
        )
        .unwrap();
        assert!(report.complete);
        assert_eq!(report.failed, failed, "{facets} {count}");
        assert_eq!(report.targets.len(), count);
        assert!(report
            .targets
            .iter()
            .all(|target| Arc::ptr_eq(target.document(), &library)));
        assert!(source
            .nodes
            .iter()
            .filter_map(|n| if let CemAstNode::Reference { targets, .. } = n {
                Some(targets)
            } else {
                None
            })
            .all(Option::is_none));
    }
    for facets in [
        "@minItems=no",
        "@minItems=3 @maxItems=1",
        "@itemCount=3 @maxItems=1",
    ] {
        assert!(native_attribute_model(facets)
            .compile_diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()));
    }
}
#[test]
fn native_attribute_reference_incomplete_chains_do_not_validate_prefix_counts() {
    use cem_ml::schema::attribute_references::validate_native_attribute_reference;
    let source = parse("{box @target={#nodes}}");
    let library = parse("{schema | {item} {#later}}");
    let later = SchemaDeclarationNode::new(
        library.clone(),
        library
            .nodes
            .iter()
            .position(|n| matches!(n, CemAstNode::Reference { .. }))
            .unwrap() as u32,
    )
    .unwrap();
    for disposition in ["mandatory", "warning", "ignore"] {
        let mut host = Host::new();
        host.disposition(disposition);
        host.outcomes.insert(
            "#nodes".into(),
            ReferenceLinkEvaluation::Resolved(vec![node(&library, "item"), later.clone()]),
        );
        host.outcomes.insert(
            "#later".into(),
            ReferenceLinkEvaluation::Unresolved("not-ready".into()),
        );
        let limits = host.policy.limits;
        let report = validate_native_attribute_reference(
            native_attribute(&source),
            &native_attribute_model("@itemCount=2"),
            "box",
            &mut host,
            limits,
        )
        .unwrap();
        assert!(!report.complete);
        assert_eq!(report.targets.len(), 1);
        assert!(!report.diagnostics.iter().any(
            |d| d.code == cem_ml::schema::document_model::INVALID_ATTRIBUTE_DATATYPE_PARAM_CODE
        ));
    }
    for mode in ["pending", "denied", "cycle", "work", "scope-work"] {
        let mut host = Host::new();
        host.disposition("ignore");
        host.outcomes.insert(
            "#nodes".into(),
            if mode == "pending" {
                ReferenceLinkEvaluation::Pending("context".into())
            } else if mode == "cycle" {
                ReferenceLinkEvaluation::Resolved(vec![SchemaDeclarationNode::new(
                    source.clone(),
                    source
                        .nodes
                        .iter()
                        .position(|n| matches!(n, CemAstNode::Reference { .. }))
                        .unwrap() as u32,
                )
                .unwrap()])
            } else {
                ReferenceLinkEvaluation::Resolved(vec![node(&library, "item")])
            },
        );
        host.deny = mode == "denied";
        if mode == "scope-work" {
            host.policy.limits.max_work = 1;
        }
        let limits = if mode == "work" {
            ReferenceTraversalLimits {
                max_depth: 128,
                max_work: 1,
            }
        } else if mode == "scope-work" {
            ReferenceTraversalLimits::schema_defaults().unwrap()
        } else {
            host.policy.limits
        };
        let report = validate_native_attribute_reference(
            native_attribute(&source),
            &native_attribute_model(""),
            "box",
            &mut host,
            limits,
        )
        .unwrap();
        assert!(!report.complete, "{mode}");
    }
}
#[test]
fn native_attribute_reference_custom_count_diagnostics_and_pending_expressions() {
    use cem_ml::schema::attribute_references::validate_native_attribute_reference;
    let source = parse("{box @target={#nodes}}");
    let model = compile_schema_document_model(
        "consumer",
        r#"{schema | {elements | {element @name=box @required-attributes=target}} {attributes | {attribute @name=target @type=schema:node @datatype-param-diagnostic=fixture.count}} {behaviors | {behavior @name=count-check @implementation=engine @execution=ast-validation @primitive=schema:datatype-param}} {diagnostics | {diagnostic @code=fixture.count @severity=warning @behavior=count-check @message="Wrong node count"}}}"#,
    );
    assert!(
        model.compile_diagnostics.is_empty(),
        "{:?}",
        model.compile_diagnostics
    );
    let mut host = Host::new();
    host.outcomes
        .insert("#nodes".into(), ReferenceLinkEvaluation::Resolved(vec![]));
    let limits = host.policy.limits;
    let report = validate_native_attribute_reference(
        native_attribute(&source),
        &model,
        "box",
        &mut host,
        limits,
    )
    .unwrap();
    assert!(report.complete);
    assert!(!report.failed);
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].code, "fixture.count");
    assert_eq!(report.diagnostics[0].severity, Severity::Warning);
    assert!(report.diagnostics[0].message.contains("Wrong node count"));
    assert_eq!(
        report.diagnostics[0].source_map.as_ref(),
        if let CemAstNode::Attribute { source, .. } = native_attribute(&source).node() {
            Some(source)
        } else {
            None
        }
    );
    let mut host = Host::new();
    let limits = host.policy.limits;
    let primitive=compile_schema_document_model("consumer","{schema | {elements | {element @name=box @required-attributes=target}} {attributes | {attribute @name=target @type=schema:string}}}");
    let report = validate_native_attribute_reference(
        native_attribute(&source),
        &primitive,
        "box",
        &mut host,
        limits,
    )
    .unwrap();
    assert!(report.failed);
    assert_eq!(host.calls, 0);
    let generic = parse("{box @target={1 + 2}}");
    let report = validate_native_attribute_reference(
        native_attribute(&generic),
        &model,
        "box",
        &mut host,
        limits,
    )
    .unwrap();
    assert!(!report.complete);
    assert_eq!(host.calls, 0);
}

#[test]
fn native_attribute_reference_unconsumed_facets_do_not_report_full_validation() {
    use cem_ml::schema::attribute_references::validate_native_attribute_reference;
    let source = parse("{box @target={#nodes}}");
    let library = parse("{item}");
    let incompatible = native_attribute_model("@pattern=accepted");
    assert!(incompatible
        .compile_diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation()));
    let mut host = Host::new();
    let limits = host.policy.limits;
    let blocked = validate_native_attribute_reference(
        native_attribute(&source),
        &incompatible,
        "box",
        &mut host,
        limits,
    )
    .unwrap();
    assert!(!blocked.complete);
    assert!(blocked.failed);
    assert_eq!(host.calls, 0);
    for facets in ["@values=accepted"] {
        // Defensive readiness for models supplied directly by a caller without
        // schema compilation; the normal compiler now rejects this facet.
        let mut model = native_attribute_model("");
        model
            .attributes
            .get_mut("target")
            .unwrap()
            .allowed_values
            .insert(facets.to_owned());
        for count in [1, 2] {
            let mut host = Host::new();
            host.outcomes.insert(
                "#nodes".into(),
                ReferenceLinkEvaluation::Resolved(vec![node(&library, "item"); count]),
            );
            let limits = host.policy.limits;
            let report = validate_native_attribute_reference(
                native_attribute(&source),
                &model,
                "box",
                &mut host,
                limits,
            )
            .unwrap();
            assert!(!report.complete);
            assert!(report.resolution.as_ref().unwrap().is_complete());
            assert_eq!(report.targets.len(), count);
            assert_eq!(report.failed, count != 1);
        }
    }
}

#[test]
fn native_node_enum_definitions_block_evaluation_before_consumption() {
    use cem_ml::schema::attribute_references::validate_native_attribute_reference;
    let model = native_attribute_model("@values=accepted");
    assert!(model
        .compile_diagnostics
        .iter()
        .any(|d| d.code == cem_ml::schema::document_model::INVALID_SCHEMA_DATATYPE_PARAM_CODE));
    let source = parse("{box @target={#nodes}}");
    let mut host = Host::new();
    let limits = host.policy.limits;
    let report = validate_native_attribute_reference(
        native_attribute(&source),
        &model,
        "box",
        &mut host,
        limits,
    )
    .unwrap();
    assert!(!report.complete);
    assert!(report.failed);
    assert_eq!(host.calls, 0);
}
#[test]
fn retained_native_attributes_consume_in_authored_and_selected_placements() {
    use cem_ml::schema::input_references::validate_structural_input_references;
    let model = compile_schema_document_model("consumer", "{schema | {elements | {element @name=box @children=item} {element @name=item @required-attributes=target}} {attributes | {attribute @name=target @type=schema:node}}}");
    let library = parse("{item @target={#target}}");
    let targets = parse("{target @label=value | {child} {#authored}}");
    for source in [
        parse("{box | {item @target={#target}}}"),
        parse("{box | {#items}}"),
    ] {
        let mut host = Host::new();
        host.outcomes.insert(
            "#items".into(),
            ReferenceLinkEvaluation::Resolved(vec![node(&library, "item")]),
        );
        host.outcomes.insert(
            "#target".into(),
            ReferenceLinkEvaluation::Resolved(vec![node(&targets, "target")]),
        );
        let limits = host.policy.limits;
        let report =
            validate_structural_input_references(source.clone(), &model, &mut host, limits)
                .unwrap();
        assert!(report.complete, "{:?}", report.diagnostics);
        assert!(!report.failed, "{:?}", report.diagnostics);
        assert!(Arc::ptr_eq(&report.source, &source));
        assert_eq!(
            host.calls,
            if source.nodes.iter().any(
                |n| matches!(n, CemAstNode::Reference {expression,..} if expression == "#items")
            ) {
                2
            } else {
                1
            }
        );
    }
}

#[test]
fn retained_native_attribute_failures_share_budgets_without_erasing_structure() {
    use cem_ml::schema::input_references::validate_structural_input_references;
    let model=compile_schema_document_model("consumer", "{schema | {elements | {element @name=box @children=item} {element @name=item @required-attributes=target @children=leaf} {element @name=leaf}} {attributes | {attribute @name=target @type=schema:node}} {field-contracts | {field-contract @name=required @target=item @required-children=leaf}}}");
    let source = parse("{box | {#items}}");
    let library = parse("{item @target={#target}}");
    let targets = parse("{target | {child}}");
    let reference_of = |owner: &Arc<CemDocument>, expr: &str| {
        SchemaDeclarationNode::new(
            owner.clone(),
            owner
                .nodes
                .iter()
                .find_map(|node| match node {
                    CemAstNode::Reference {
                        node_id,
                        expression,
                        ..
                    } if expression == expr => Some(*node_id),
                    _ => None,
                })
                .unwrap(),
        )
        .unwrap()
    };
    for mode in [
        "unresolved",
        "initial-denied",
        "cycle",
        "work",
        "destination-work",
    ] {
        let mut host = Host::new();
        host.disposition("ignore");
        host.outcomes.insert(
            "#items".into(),
            ReferenceLinkEvaluation::Resolved(vec![node(&library, "item")]),
        );
        if mode != "unresolved" {
            host.outcomes.insert(
                "#target".into(),
                ReferenceLinkEvaluation::Resolved(vec![if mode == "cycle" {
                    reference_of(&source, "#items")
                } else {
                    node(&targets, "target")
                }]),
            );
        }
        if mode == "initial-denied" {
            host.denied_nodes
                .insert(reference_of(&library, "#target").identity());
        }
        if mode == "destination-work" {
            host.scope_bounds.insert(
                Arc::as_ptr(&targets) as usize,
                ReferenceTraversalLimits {
                    max_depth: 128,
                    max_work: 1,
                },
            );
        }
        let limits = if mode == "work" {
            ReferenceTraversalLimits {
                max_depth: 128,
                max_work: 3,
            }
        } else {
            host.policy.limits
        };
        let report =
            validate_structural_input_references(source.clone(), &model, &mut host, limits)
                .unwrap();
        assert!(!report.complete, "{mode}");
        let placement=report.nodes.iter().find(|n| matches!(n.source.node(),CemAstNode::Element{expanded_name,..} if expanded_name.local_name=="item")).unwrap();
        assert_eq!(placement.attribute_values.len(), 1, "{mode}");
        assert!(!placement.attribute_values[0].complete, "{mode}");
        assert!(
            !report
                .diagnostics
                .iter()
                .any(|d| d.code
                    == cem_ml::schema::document_model::INVALID_ATTRIBUTE_DATATYPE_PARAM_CODE),
            "{mode}"
        );
        if ["unresolved", "initial-denied", "cycle"].contains(&mode) {
            assert!(placement.children_complete, "{mode}");
            assert!(
                report.diagnostics.iter().any(|d| d
                    .details
                    .as_ref()
                    .is_some_and(|details| details["contract"] == "required")),
                "{mode}: {:?}",
                report.diagnostics
            );
        }
        if mode == "work" {
            assert_eq!(host.calls, 2);
            assert_eq!(report.references[0].resolution.work_used, 3);
        }
    }
}

#[test]
fn native_target_access_rejects_invalid_original_owning_edges() {
    use cem_ml::schema::input_references::validate_structural_input_references;
    let model = native_attribute_model("");
    let source = parse("{box @target={#nodes}}");
    for invalid in ["missing", "duplicate", "attribute-kind"] {
        let mut targets = parse("{target | {child}}");
        let target = node(&targets, "target").node_id();
        let child = node(&targets, "child").node_id();
        if let CemAstNode::Element {
            children,
            attributes,
            ..
        } = &mut Arc::get_mut(&mut targets).unwrap().nodes[target as usize]
        {
            match invalid {
                "missing" => children.push(u32::MAX),
                "duplicate" => children.push(child),
                _ => attributes.push(child),
            }
        }
        let mut host = Host::new();
        host.outcomes.insert(
            "#nodes".into(),
            ReferenceLinkEvaluation::Resolved(vec![node(&targets, "target")]),
        );
        let limits = host.policy.limits;
        let report =
            validate_structural_input_references(source.clone(), &model, &mut host, limits)
                .unwrap();
        assert!(!report.complete, "{invalid}");
        assert!(report.failed, "{invalid}");
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.code == cem_ml::schema::input_references::INVALID_STRUCTURAL_TARGET));
    }
}

#[test]
fn composite_native_attributes_keep_one_budget_and_authored_order() {
    use cem_ml::schema::input_references::validate_structural_input_references;
    let model = compile_schema_document_model("consumer", "{schema | {elements | {element @name=box @optional-attributes=target}} {attributes | {attribute @name=target @type=schema:node @minItems=0 @maxItems=2}}}");
    let library = parse("{first}{second}");
    for second_expression in ["#second", "second"] {
        let mut authored = parse(&format!(
            "{{box @target={{#first}} @second={{{second_expression}}}}}"
        ));
        let attribute = native_attribute(&authored).node_id();
        let owner = Arc::get_mut(&mut authored).unwrap();
        let CemAstNode::Element { attributes, .. } = &mut owner.nodes[1] else {
            panic!()
        };
        let other = attributes.pop().unwrap();
        let CemAstNode::Attribute { value_nodes, .. } = &mut owner.nodes[other as usize] else {
            panic!()
        };
        let second = std::mem::take(value_nodes);
        let CemAstNode::Attribute { value_nodes, .. } = &mut owner.nodes[attribute as usize] else {
            panic!()
        };
        value_nodes.extend(second);
        for selected in [false, true] {
            let source = if selected {
                parse("{#placements}")
            } else {
                authored.clone()
            };
            for mode in [
                "complete",
                "depth-one",
                "pending",
                "work",
                "destination-work",
                "denied",
            ] {
                let mut host = Host::new();
                host.disposition("ignore");
                host.outcomes.insert(
                    "#placements".into(),
                    ReferenceLinkEvaluation::Resolved(vec![node(&authored, "box")]),
                );
                host.outcomes.insert(
                    "#first".into(),
                    ReferenceLinkEvaluation::Resolved(vec![node(&library, "first")]),
                );
                host.outcomes.insert(
                    second_expression.into(),
                    if mode == "pending" {
                        ReferenceLinkEvaluation::Pending("later".into())
                    } else {
                        ReferenceLinkEvaluation::Resolved(vec![node(&library, "second")])
                    },
                );
                if mode == "destination-work" {
                    host.scope_bounds.insert(
                        Arc::as_ptr(&library) as usize,
                        ReferenceTraversalLimits {
                            max_depth: 128,
                            max_work: 1,
                        },
                    );
                }
                if mode == "denied" {
                    host.denied_nodes
                        .insert(node(&library, "second").identity());
                }
                let work = if mode == "work" { 4 } else { 100 };
                let report = validate_structural_input_references(
                    source.clone(),
                    &model,
                    &mut host,
                    ReferenceTraversalLimits {
                        max_depth: if mode == "depth-one" {
                            1 + usize::from(selected)
                        } else {
                            128
                        },
                        max_work: work,
                    },
                )
                .unwrap();
                assert_eq!(
                    report.complete,
                    matches!(mode, "complete" | "depth-one"),
                    "{second_expression}/{selected}/{mode}: {:?}",
                    report.diagnostics
                );
                let value = &report.nodes[0].attribute_values[0];
                if matches!(mode, "complete" | "depth-one") {
                    assert_eq!(
                        host.calls + host.expression_calls,
                        2 + usize::from(selected)
                    );
                    let names: Vec<_> = value
                        .access
                        .roots()
                        .iter()
                        .map(|index| match value.access.node(*index).unwrap().node() {
                            CemAstNode::Element { expanded_name, .. } => {
                                expanded_name.local_name.as_str()
                            }
                            _ => panic!(),
                        })
                        .collect();
                    assert_eq!(names, vec!["first", "second"]);
                    assert!(value.access.roots().iter().all(|index| Arc::ptr_eq(
                        value.access.node(*index).unwrap().document(),
                        &library
                    )));
                } else {
                    assert!(!value.complete);
                    assert!(value.access.roots().len() < 2);
                    assert!(!report.diagnostics.iter().any(|diagnostic| diagnostic
                        .details
                        .as_ref()
                        .is_some_and(|details| details.get("datatypeParam").is_some())));
                }
            }
        }
    }
}

#[test]
fn malformed_native_source_slots_never_become_complete_values() {
    use cem_ml::schema::input_references::validate_structural_input_references;
    let model = native_attribute_model("");
    let targets = parse("{item}");
    for invalid in ["missing", "duplicate"] {
        let mut source = parse("{box @target={#nodes}}");
        let attribute = native_attribute(&source).node_id();
        let owner = Arc::get_mut(&mut source).unwrap();
        let CemAstNode::Attribute { value_nodes, .. } = &mut owner.nodes[attribute as usize] else {
            panic!()
        };
        if invalid == "missing" {
            *value_nodes = vec![u32::MAX];
        } else {
            value_nodes.push(value_nodes[0]);
        }
        let mut host = Host::new();
        host.outcomes.insert(
            "#nodes".into(),
            ReferenceLinkEvaluation::Resolved(vec![node(&targets, "item")]),
        );
        let limits = host.policy.limits;
        let report =
            validate_structural_input_references(source, &model, &mut host, limits).unwrap();
        assert!(
            !report.complete && report.failed,
            "{invalid}: {:?}",
            report.diagnostics
        );
        assert!(!report.nodes[0].attribute_values[0].complete);
        assert!(report.diagnostics.iter().any(|diagnostic| diagnostic.code
            == cem_ml::schema::input_references::INVALID_STRUCTURAL_TARGET));
    }
}

#[test]
fn source_only_native_element_bases_keep_schema_candidates_pending() {
    let source =
        "{schema | {elements | {element @name=child @base={#base} @required-attributes=own}}}";
    let model = compile_schema_document_model("consumer", source);
    assert!(!model.is_ready_for_validation());
    assert!(!model.declaration_references.failed());
    assert_eq!(
        model.declaration_references.state(),
        ReferenceResolutionState::Pending
    );
    assert!(model
        .element("child")
        .unwrap()
        .required_attributes
        .contains("own"));
    assert_eq!(model.declaration_references.sites.len(), 1);
    assert!(model.declaration_references.sites[0].resolution.is_none());
    assert_eq!(
        model.declaration_references.sites[0]
            .occurrence
            .expression
            .as_deref(),
        Some("#base")
    );
    let literal = compile_schema_document_model("consumer", "{schema | {uses | {use @schema='https://cem.dev/ns/schema/1' @as=origin}} {elements | {element @name=child @base=origin:element}}}");
    assert!(literal.is_ready_for_validation());
    assert!(literal
        .element("child")
        .unwrap()
        .optional_attributes
        .contains("base"));
}

#[test]
fn non_reference_native_element_bases_are_source_attributed_schema_errors() {
    for text in [
        "{schema | {elements | {element @name=child @base={items}}}}",
        "{schema | {elements | {element @name=child @base={#base} @other={#other}}}}",
    ] {
        let mut document = parse(text);
        if text.contains("@other") {
            let owner = Arc::get_mut(&mut document).unwrap();
            let mut base = None;
            let mut extra = vec![];
            for node in &mut owner.nodes {
                if let CemAstNode::Attribute {
                    node_id,
                    expanded_name,
                    value_nodes,
                    ..
                } = node
                {
                    if expanded_name.local_name == "base" {
                        base = Some(*node_id);
                    }
                    if expanded_name.local_name == "other" {
                        extra = std::mem::take(value_nodes);
                    }
                }
            }
            let CemAstNode::Attribute { value_nodes, .. } =
                &mut owner.nodes[base.unwrap() as usize]
            else {
                unreachable!()
            };
            value_nodes.extend(extra);
        }
        let mut host = Host::new();
        let limits = host.policy.limits;
        let model = if text.contains("@other") {
            compile_schema_with_declaration_references("consumer", document, &mut host, limits)
                .unwrap()
        } else {
            compile_schema_document_model("consumer", text)
        };
        assert!(!model.is_ready_for_validation());
        assert!(model.declaration_references.failed());
        let diagnostic = model
            .compile_diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.code == cem_ml::schema::declaration_references::INVALID_REFERENCE_TARGET
            })
            .unwrap();
        assert!(diagnostic.source_map.is_some());
        assert!(diagnostic.byte_offset.is_some());
        assert_eq!(diagnostic.uri.as_deref(), Some("consumer"));
    }
}

#[test]
fn native_element_bases_inherit_original_lexical_declarations_and_local_overrides() {
    let source = parse("{schema | {uses | {use @schema=unrelated @as=origin}} {elements | {element @name=child @base={#base} @required-attributes=own} {element @name=other @base={#base}}}}");
    let library = parse("{schema | {uses | {use @schema='https://cem.dev/ns/schema/1' @as=origin}} {elements | {element @name=shared @base=origin:element @required-attributes=inherited}}}");
    let mut host = Host::new();
    host.outcomes.insert(
        "#base".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&library, "element")]),
    );
    let limits = host.policy.limits;
    let model =
        compile_schema_with_declaration_references("consumer", source.clone(), &mut host, limits)
            .unwrap();
    assert!(
        model.is_ready_for_validation(),
        "{:?}",
        model.compile_diagnostics
    );
    assert!(
        model.compile_diagnostics.is_empty(),
        "{:?}",
        model.compile_diagnostics
    );
    assert!(model
        .element("child")
        .unwrap()
        .required_attributes
        .contains("own"));
    assert!(!model
        .element("child")
        .unwrap()
        .required_attributes
        .contains("inherited"));
    assert!(model
        .element("child")
        .unwrap()
        .optional_attributes
        .contains("base"));
    assert!(model
        .element("other")
        .unwrap()
        .required_attributes
        .contains("inherited"));
    drop(host);
    assert!(model
        .declaration_references
        .sites
        .iter()
        .flat_map(|site| site.resolution.as_ref().unwrap().nodes.iter())
        .any(|target| Arc::ptr_eq(target.document(), &library)));
}

#[test]
fn native_element_base_cardinality_and_incomplete_chains_do_not_activate_models() {
    let source = parse("{schema | {elements | {element @name=child @base={#base}}}}");
    let library = parse("{element @name=shared}");
    let invalid = parse("{attribute @name=wrong}{element}");
    for mode in [
        "empty",
        "many",
        "kind",
        "unnamed",
        "pending",
        "unresolved",
        "denied",
        "cycle",
    ] {
        let mut host = Host::new();
        host.disposition("ignore");
        let outcome = match mode {
            "empty" => ReferenceLinkEvaluation::Resolved(vec![]),
            "many" => ReferenceLinkEvaluation::Resolved(vec![node(&library, "element"); 2]),
            "kind" => ReferenceLinkEvaluation::Resolved(vec![node(&invalid, "attribute")]),
            "unnamed" => ReferenceLinkEvaluation::Resolved(vec![node(&invalid, "element")]),
            "pending" => ReferenceLinkEvaluation::Pending("later".into()),
            "unresolved" => ReferenceLinkEvaluation::Unresolved("missing".into()),
            "cycle" => ReferenceLinkEvaluation::Resolved(vec![node(&source, "element")]),
            _ => ReferenceLinkEvaluation::Resolved(vec![node(&library, "element")]),
        };
        host.outcomes.insert("#base".into(), outcome);
        host.deny = mode == "denied";
        let limits = host.policy.limits;
        let model = compile_schema_with_declaration_references(
            "consumer",
            source.clone(),
            &mut host,
            limits,
        )
        .unwrap();
        assert!(!model.is_ready_for_validation(), "{mode}");
        assert_eq!(
            model.declaration_references.failed(),
            matches!(mode, "empty" | "many" | "kind" | "unnamed"),
            "{mode}: {:?}",
            model.compile_diagnostics
        );
        assert!(host.calls <= 2, "{mode}");
    }
}

#[test]
fn native_element_base_chains_use_one_request_and_destination_budget() {
    let source = parse("{schema | {elements | {element @name=child @base={#base}}}}");
    let middle = parse("{element @name=middle @base={#next}}");
    let leaf = parse("{element @name=leaf @required-attributes=deep}");
    for mode in ["complete", "work", "destination-work", "depth"] {
        let mut host = Host::new();
        host.disposition("ignore");
        host.outcomes.insert(
            "#base".into(),
            ReferenceLinkEvaluation::Resolved(vec![node(&middle, "element")]),
        );
        host.outcomes.insert(
            "#next".into(),
            ReferenceLinkEvaluation::Resolved(vec![node(&leaf, "element")]),
        );
        if mode == "destination-work" {
            host.scope_bounds.insert(
                Arc::as_ptr(&middle) as usize,
                ReferenceTraversalLimits {
                    max_depth: 128,
                    max_work: 2,
                },
            );
        }
        let limits = ReferenceTraversalLimits {
            max_depth: if mode == "depth" { 1 } else { 128 },
            max_work: if mode == "work" { 4 } else { 100 },
        };
        let model = compile_schema_with_declaration_references(
            "consumer",
            source.clone(),
            &mut host,
            limits,
        )
        .unwrap();
        assert_eq!(
            model.is_ready_for_validation(),
            mode == "complete",
            "{mode}: {:?}",
            model.compile_diagnostics
        );
        if mode == "complete" {
            assert!(model
                .element("child")
                .unwrap()
                .required_attributes
                .contains("deep"));
        } else {
            assert!(
                !model.declaration_references.failed(),
                "{mode}: {:?}",
                model.compile_diagnostics
            );
        }
    }
}

#[test]
fn element_base_metamodel_contract_preserves_literal_and_native_boundaries() {
    use cem_ml::schema::document_model::validate_document_model;
    let model = compile_schema_document_model(
        "metamodel",
        include_str!("../schema-packages/schema/v1/schema/cem-schema.cem"),
    );
    assert_eq!(
        model.attributes["base"].value_type.as_deref(),
        Some("schema:element-base")
    );
    for value in ["origin:element", "origin:*", "{#library}"] {
        let source = parse(
            &[
                "{schema @name=test @namespace=https://example.test/test @version=1.0.0 | {elements | {element @name=child @base=",
                value,
                "}}}",
            ]
            .concat(),
        );
        assert!(
            validate_document_model(&source, &model).is_empty(),
            "{value}: {:?}",
            validate_document_model(&source, &model)
        );
    }
    let invalid = parse("{schema | {types | {type @name=custom @base={#library}}}}");
    assert!(validate_document_model(&invalid, &model)
        .iter()
        .any(|diagnostic| diagnostic.code
            == cem_ml::schema::document_model::INVALID_ATTRIBUTE_TYPE_CODE));
}

#[test]
fn nested_base_cardinality_errors_preserve_the_failing_constructor() {
    let source = parse("{schema | {elements | {#library}}}");
    let library = parse("{element @name=base @base={#empty}}");
    let mut host = Host::new();
    host.outcomes.insert(
        "#library".into(),
        ReferenceLinkEvaluation::Resolved(vec![node(&library, "element")]),
    );
    host.outcomes
        .insert("#empty".into(), ReferenceLinkEvaluation::Resolved(vec![]));
    let limits = host.policy.limits;
    let model =
        compile_schema_with_declaration_references("consumer", source, &mut host, limits).unwrap();
    let diagnostic = model
        .compile_diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == cem_ml::schema::declaration_references::INVALID_REFERENCE_TARGET
        })
        .unwrap();
    assert!(diagnostic.message.contains("#empty"), "{diagnostic:?}");
    let reference = library
        .nodes
        .iter()
        .find(|node| matches!(node, CemAstNode::Reference { .. }))
        .unwrap();
    let CemAstNode::Reference { source, .. } = reference else {
        unreachable!()
    };
    assert_eq!(diagnostic.source_map.as_ref(), Some(source));
}
