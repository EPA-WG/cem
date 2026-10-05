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
}
impl Host {
    fn new() -> Self {
        Self {
            outcomes: HashMap::new(),
            policy: ReferenceScopePolicy::schema_defaults().unwrap(),
            deny: false,
            calls: 0,
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
    fn scope_limits(&self, _: &usize) -> ReferenceTraversalLimits {
        self.policy.limits
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
    fn permits_edge(&self, _: &Self::Node, _: &Self::Node) -> bool {
        !self.deny
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
    fn source_reference(&self, source: SchemaDeclarationNode) -> SchemaDeclarationNode {
        source
    }
    fn declaration_node(&self, target: &SchemaDeclarationNode) -> Option<SchemaDeclarationNode> {
        Some(target.clone())
    }
    fn declaration_schema(&self, target: &SchemaDeclarationNode) -> Option<SchemaDeclarationNode> {
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
