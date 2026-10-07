#![cfg(not(target_arch = "wasm32"))]
use cem_ml::{
    diagnostics::Diagnostic,
    engine::{
        CemMlEngine, EngineContext, EngineInput, FailLevel, InputFormat, ValidateProjection,
        ValidateRequest,
    },
    import::ScopedCemImport,
    parser::CemAstNode,
    real::RealCemMlEngine,
    resolver::{
        ResolveDirection, ResolvePurpose, ResolveRequest, ResolvedRead, ResolvedWrite,
        ResolverDiagnostic, ResourceResolver,
    },
    schema::{
        declaration_references::SchemaDeclarationNode,
        document_model::compile_schema_document_model,
        input_validation::{
            resumable::{InputValidationSession, OwnedInputValidationRequest},
            InputValidationOutcome, InputValidationRequest, InputValidationStage,
        },
        registry::CEM_ML_SCHEMA_URI,
        scope_controls::SchemaHostControl,
        uri_loading::SchemaUriResource,
    },
};
use cem_ml_transform_cem_ql::schema_validation_session::{
    CemQlInputValidationSession, SchemaValidationSessionInputs,
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{ItemStream, RetainedCemNode},
    schema_references::{
        CemQlSchemaDeclarationHost, DeclarationScope, SchemaHostRuntimeContextRequest,
        SchemaUriLoadedScope,
    },
};
use std::sync::{Arc, Mutex};
#[derive(Debug, Clone, Copy)]
enum Mode {
    Ready,
    NoContext,
    NoGrant,
    ReadFailure,
    Hint,
    Namespace,
    NamespaceNoContext,
}
#[derive(Debug, Default)]
struct Seen {
    reads: Vec<String>,
    snapshots: Vec<Vec<String>>,
    owners: Vec<Arc<cem_ml::parser::document::CemDocument>>,
    policies: Vec<usize>,
    namespace_snapshots: Vec<bool>,
}
#[derive(Debug)]
struct Stage {
    mode: Mode,
    seen: Arc<Mutex<Seen>>,
}
impl InputValidationStage for Stage {
    fn validate(
        &self,
        _: InputValidationRequest<'_>,
    ) -> Result<InputValidationOutcome, Vec<Diagnostic>> {
        unreachable!()
    }
    fn start_resumable(
        &self,
        request: OwnedInputValidationRequest,
    ) -> Option<Box<dyn InputValidationSession>> {
        let mut host = CemQlSchemaDeclarationHost::new();
        let origin = host.register_scope(
            request.source.clone(),
            Some(StandaloneExpressionContext::default()),
            request.policy.clone(),
        );
        host.attach_captured_names(request.lexical_scopes.as_ref().unwrap())
            .unwrap();
        self.seen
            .lock()
            .unwrap()
            .owners
            .push(request.source.ast_owner().clone());
        let source = request.source.clone();
        Some(Box::new(CemQlInputValidationSession::new(
            request,
            host,
            Inputs {
                source,
                mode: self.mode,
                origin,
                seen: self.seen.clone(),
            },
        )))
    }
}
#[derive(Debug)]
struct Inputs {
    source: Arc<cem_ml::parser::tree::RetainedCemTree>,
    mode: Mode,
    origin: DeclarationScope,
    seen: Arc<Mutex<Seen>>,
}
impl SchemaValidationSessionInputs for Inputs {
    fn namespace_lifecycle_enabled(&self) -> bool {
        matches!(self.mode, Mode::Namespace | Mode::NamespaceNoContext)
    }
    fn namespace_context(
        &mut self,
        _: &SchemaDeclarationNode,
        _: &cem_ml::schema::namespace_references::NamespaceLexicalSnapshot,
        _: DeclarationScope,
        _: &Arc<cem_ml::schema::namespace_references::NamespaceNameCompletion>,
    ) -> (Option<StandaloneExpressionContext>, cem_ml::schema::reference_policy::ReferenceScopePolicyOverrides) {
        let context = (!matches!(self.mode, Mode::NamespaceNoContext)).then(|| {
            let id = self.source.ast().nodes.iter().find_map(|node| match node {
                CemAstNode::Element {node_id, expanded_name, ..} if expanded_name.local_name == "@ns" => Some(*node_id),
                _ => None,
            }).unwrap();
            StandaloneExpressionContext::default().with_binding("namespace", StandaloneExpressionBinding::any(ItemStream::once(
                RetainedCemNode::new(self.source.clone(), id).unwrap().query_item()
            )))
        });
        (context, Default::default())
    }
    fn namespace_inspected(&mut self, snapshot: &cem_ql::schema_references::NamespaceLifecycleSnapshot) {
        self.seen.lock().unwrap().namespace_snapshots.push(snapshot.is_complete());
        assert!(Arc::ptr_eq(snapshot.completion.captured().document(), self.source.ast_owner()));
    }
    fn runtime_context(
        &mut self,
        _: SchemaHostRuntimeContextRequest<'_>,
    ) -> Option<StandaloneExpressionContext> {
        Some(StandaloneExpressionContext::default())
    }
    fn resource_content_type_hint(&mut self, _: &SchemaHostControl) -> Option<String> {
        matches!(self.mode, Mode::Hint).then(|| "text/cem-ml".into())
    }
    fn loaded_policy(
        &mut self,
        _: &SchemaHostControl,
        inherited: &cem_ml::schema::reference_policy::ReferenceScopePolicy,
    ) -> cem_ml::schema::reference_policy::ReferenceScopePolicy {
        self.seen
            .lock()
            .unwrap()
            .policies
            .push(inherited.limits.max_work);
        inherited.clone()
    }
    fn loaded_context(
        &mut self,
        resource: &SchemaUriResource,
    ) -> Option<StandaloneExpressionContext> {
        assert!(std::thread::current()
            .name()
            .unwrap()
            .starts_with("cem-ml-cpu-"));
        self.seen
            .lock()
            .unwrap()
            .owners
            .push(resource.imported.tree.ast_owner().clone());
        (!matches!(self.mode, Mode::NoContext)).then(StandaloneExpressionContext::default)
    }
    fn public_exports(
        &mut self,
        _: &ScopedCemImport,
        _: &str,
    ) -> Result<Vec<SchemaDeclarationNode>, String> {
        Err("no exports".into())
    }
    fn prepare_loaded(
        &mut self,
        host: &mut CemQlSchemaDeclarationHost,
        _: &SchemaHostControl,
        loaded: &SchemaUriLoadedScope,
    ) -> Result<(), Vec<Diagnostic>> {
        let imported = &loaded.resource.imported;
        let id = imported
            .tree
            .ast()
            .nodes
            .iter()
            .find_map(|node| match node {
                CemAstNode::Element {
                    node_id,
                    expanded_name,
                    ..
                } if expanded_name.local_name == "element" => Some(*node_id),
                _ => None,
            })
            .unwrap();
        let context = StandaloneExpressionContext::default().with_binding(
            "library",
            StandaloneExpressionBinding::any(ItemStream::once(
                RetainedCemNode::new(imported.tree.clone(), id)
                    .unwrap()
                    .query_item(),
            )),
        );
        host.attach_captured_lexical_scopes(&imported.captured, |_, _, _| {
            (
                Some(context.clone()),
                cem_ml::schema::reference_policy::ReferenceScopePolicy::schema_defaults().unwrap(),
            )
        })
        .unwrap();
        if !matches!(self.mode, Mode::NoGrant) {
            host.allow_scope_crossing(self.origin, loaded.scope);
        }
        assert!(imported.tree.ast().nodes.iter().all(|n| !matches!(
            n,
            CemAstNode::Reference {
                targets: Some(_),
                ..
            }
        )));
        Ok(())
    }
    fn inspected(&mut self, report: &cem_ql::schema_references::SchemaHostRuntimeValidation) {
        let entered = report
            .inputs
            .iter()
            .filter_map(|inputs| inputs.region().contract.control())
            .map(|c| match &c.source {
                cem_ml::schema::scope_controls::SchemaHostSource::Uri(uri) => uri.clone(),
                _ => "selector".into(),
            })
            .collect();
        self.seen.lock().unwrap().snapshots.push(entered);
        let source = self.seen.lock().unwrap().owners[0].clone();
        assert!(report
            .validation
            .nodes
            .iter()
            .all(|node| Arc::ptr_eq(node.source.document(), &source)));
    }
}
struct Reader {
    mode: Mode,
    seen: Arc<Mutex<Seen>>,
    xml: bool,
}
impl ResourceResolver for Reader {
    fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        assert!(std::thread::current()
            .name()
            .unwrap()
            .starts_with("cem-ml-io-"));
        self.seen.lock().unwrap().reads.push(request.uri.clone());
        if matches!(self.mode, Mode::ReadFailure) {
            return Err(ResolverDiagnostic::Io {
                uri: request.uri.clone(),
                message: "offline".into(),
            });
        }
        let text = if self.xml {
            if request.uri.ends_with("outer.cem") {
                "<s:schema xmlns:s='https://cem.dev/ns/schema/1'><s:elements><s:element name='child' children='leaf'/></s:elements><s:constraints><s:constraint kind='reference-traversal-work' value='32'/></s:constraints></s:schema>"
            } else {
                "<s:schema xmlns:s='https://cem.dev/ns/schema/1'><s:elements><s:element name='leaf'/></s:elements></s:schema>"
            }
        } else if request.uri.ends_with("outer.cem") {
            "@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {#library}} {constraints | {constraint @kind=reference-traversal-work @value=32}}} {s:element @name=child @children=leaf}"
        } else {
            "@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=leaf}}}"
        };
        Ok(ResolvedRead {
            uri: request.uri.clone(),
            bytes: text.as_bytes().to_vec(),
            content_type: (!matches!(self.mode, Mode::Hint)).then(|| {
                if self.xml {
                    "application/xml"
                } else {
                    "text/cem-ml"
                }
                .into()
            }),
        })
    }
    fn write(&self, _: &ResolveRequest, _: &[u8]) -> Result<ResolvedWrite, ResolverDiagnostic> {
        unreachable!()
    }
}
fn run_format(mode: Mode, xml: bool) -> (cem_ml::engine::ValidateResponse, Arc<Mutex<Seen>>) {
    let seen = Arc::new(Mutex::new(Seen::default()));
    let mut context = EngineContext::default();
    context.schema = Some(CEM_ML_SCHEMA_URI.into());
    context.content_type = Some(
        if xml {
            "application/xml"
        } else {
            "text/cem-ml"
        }
        .into(),
    );
    context.scheduler.max_parallel_documents = Some(1);
    let mut model = compile_schema_document_model(
        "base",
        "{schema | {elements | {element @name=host @children=child}}}",
    );
    model.schema_uri = CEM_ML_SCHEMA_URI.into();
    context.schema_document_models.register(model);
    context.input_validation_stage = Some(Arc::new(Stage {
        mode,
        seen: seen.clone(),
    }));
    context.resolver_registry.register(
        "https",
        ResolvePurpose::Template,
        ResolveDirection::Read,
        Reader {
            mode,
            seen: seen.clone(),
            xml,
        },
    );
    let response = RealCemMlEngine
        .validate(ValidateRequest {
            inputs: vec![EngineInput {
                uri: "https://vendor.test/main.cem".into(),
                bytes: if xml {
                    b"<host schema-src='outer.cem'><child schema-src='inner.cem'><leaf/></child></host>".to_vec()
                } else if matches!(mode, Mode::Namespace | Mode::NamespaceNoContext) {
                    b"@ns public = https://cem.dev/ns/core/1\n{host @xmlns:c={#namespace} @c:schema-src=outer.cem | {child @c:schema-src=inner.cem | {leaf}}}".to_vec()
                } else {
                    b"{host @schema-src=outer.cem | {child @schema-src=inner.cem | {leaf}}}".to_vec()
                },
                from_format: Some(if xml { InputFormat::Xml } else { InputFormat::Cem }),
                identity: None,
                root_scope: Default::default(),
            }],
            projection: ValidateProjection::Cem,
            fail_level: FailLevel::Validate,
            context,
        })
        .unwrap();
    (response, seen)
}
#[test]
fn entered_nested_controls_load_in_separate_rounds_then_activate_the_original_source() {
    for (mode, xml) in [
        (Mode::Ready, false),
        (Mode::Ready, true),
        (Mode::Hint, false),
    ] {
        let (response, seen) = run_format(mode, xml);
        assert!(
            response.report.report_ast.validation.unwrap().complete,
            "{:?}",
            response.report.diagnostics
        );
        assert!(
            !response
                .report
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()),
            "{:?}",
            response.report.diagnostics
        );
        let seen = seen.lock().unwrap();
        assert_eq!(
            seen.reads,
            [
                "https://vendor.test/outer.cem",
                "https://vendor.test/inner.cem"
            ]
        );
        assert_eq!(seen.snapshots[0], ["outer.cem"]);
        assert_eq!(seen.snapshots[1], ["outer.cem", "inner.cem"]);
        assert_eq!(seen.snapshots.len(), 3);
        assert_eq!(seen.owners.len(), 3);
        assert_eq!(
            seen.policies,
            [
                cem_ml::schema::reference_policy::ReferenceScopePolicy::schema_defaults()
                    .unwrap()
                    .limits
                    .max_work,
                32
            ]
        );
    }
}
#[test]
fn missing_destination_context_or_grant_blocks_descendant_loading_without_fallback() {
    for mode in [Mode::NoContext, Mode::NoGrant, Mode::ReadFailure] {
        let (response, seen) = run_format(mode, false);
        assert!(!response.report.report_ast.validation.unwrap().complete);
        assert_eq!(
            seen.lock().unwrap().reads,
            ["https://vendor.test/outer.cem"]
        );
        if matches!(mode, Mode::ReadFailure) {
            assert!(response
                .report
                .diagnostics
                .iter()
                .any(|d| d.code == "cem.resolver.io"));
        }
    }
}

#[test]
fn namespace_coordinator_is_opt_in_and_preserves_resumable_uri_loading() {
    let (response, seen) = run_format(Mode::Namespace, false);
    assert!(response.report.report_ast.validation.unwrap().complete, "{:?}", response.report.diagnostics);
    let seen = seen.lock().unwrap();
    assert_eq!(seen.reads, ["https://vendor.test/outer.cem", "https://vendor.test/inner.cem"]);
    assert_eq!(seen.namespace_snapshots, [true, true, true]);
    assert!(response.report.diagnostics.iter().all(|d| d.code != "cem.lint.unbound_prefix"));
}
#[test]
fn missing_namespace_runtime_inputs_finish_incomplete_without_uri_work() {
    let (response, seen) = run_format(Mode::NamespaceNoContext, false);
    assert!(!response.report.report_ast.validation.unwrap().complete);
    let seen = seen.lock().unwrap();
    assert!(seen.reads.is_empty());
    assert_eq!(seen.namespace_snapshots, [false]);
}
