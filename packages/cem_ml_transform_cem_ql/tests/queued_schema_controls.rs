#![cfg(not(target_arch = "wasm32"))]
//! Engine queue verification for retained controls, public exports and peer loads.
use cem_ml::{
    diagnostics::Diagnostic,
    engine::{
        CemMlEngine, EngineContext, EngineInput, EngineResult, FailLevel, InputFormat,
        ValidateProjection, ValidateRequest, ValidateResponse,
    },
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    parser::{tree::RetainedCemTree, CemAstNode},
    real::RealCemMlEngine,
    resolver::{
        ResolveDirection, ResolvePurpose, ResolveRequest, ResolvedRead, ResolvedWrite,
        ResolverDiagnostic, ResourceResolver,
    },
    run_config::ScopeConfig,
    schema::{
        declaration_references::SchemaDeclarationNode,
        document_model::compile_schema_document_model,
        input_validation::{
            resumable::{InputValidationSession, OwnedInputValidationRequest},
            InputValidationOutcome, InputValidationRequest, InputValidationStage,
        },
        registry::CEM_ML_SCHEMA_URI,
        scope_controls::{SchemaHostControl, SchemaHostSource, SchemaScopeControlExtent},
        uri_loading::SchemaUriResource,
        vocab::CompiledSchema,
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
        SchemaHostRuntimeValidation, SchemaUriLoadedScope,
    },
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};
const BASE: &str = "https://app.test/input/main.cem";
const SCHEMA: &str = "@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=child @required-attributes=selected}}}";
const OUTER: &str = "{schema | {elements | {element @name=container @optional-attributes=c @children='host schema outside'} {element @name=host @children='schema child'} {element @name=schema @children=child @required-attributes=own} {element @name=child @required-attributes=enclosing} {element @name=outside @required-attributes=outer}}}";

#[derive(Debug, Clone)]
struct Reply {
    text: String,
    final_uri: String,
    fail: bool,
}
impl Reply {
    fn schema(uri: &str) -> Self {
        Self {
            text: SCHEMA.into(),
            final_uri: uri.into(),
            fail: false,
        }
    }
}
#[derive(Debug, Default)]
struct Gate {
    beta_done: Mutex<bool>,
    changed: Condvar,
}
#[derive(Debug)]
struct Fixture {
    text: String,
    model: String,
    xml: bool,
    replies: BTreeMap<String, Reply>,
    foreign: Option<ScopedCemImport>,
    grant: bool,
    reorder: Option<Arc<Gate>>,
    hold_reads: bool,
    root_scope: ScopeConfig,
}
impl Fixture {
    fn new(text: &str) -> Self {
        Self {
            text: text.into(),
            model: OUTER.into(),
            xml: false,
            replies: BTreeMap::from([(
                "https://app.test/input/schema.cem".into(),
                Reply::schema("https://cdn.test/schema.cem"),
            )]),
            foreign: None,
            grant: true,
            reorder: None,
            hold_reads: false,
            root_scope: Default::default(),
        }
    }
}
#[derive(Debug, Default)]
struct Seen {
    source: Option<Arc<RetainedCemTree>>,
    reports: Vec<SchemaHostRuntimeValidation>,
    prepared: Vec<SchemaHostControl>,
    loaded: Vec<ScopedCemImport>,
    reads: Vec<String>,
    finished: Vec<String>,
    exports: Vec<String>,
}
#[derive(Debug)]
struct Stage {
    fixture: Arc<Fixture>,
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
        let mut context = StandaloneExpressionContext::default();
        if let Some(foreign) = &self.fixture.foreign {
            let id = element(&foreign.tree, "host");
            context = context.with_binding(
                "library",
                StandaloneExpressionBinding::any(ItemStream::once(
                    RetainedCemNode::new(foreign.tree.clone(), id)
                        .unwrap()
                        .query_item(),
                )),
            );
        }
        let mut host = CemQlSchemaDeclarationHost::new();
        let origin = host.register_scope(
            request.source.clone(),
            Some(context.clone()),
            request.policy.clone(),
        );
        host.attach_captured_lexical_scopes(request.lexical_scopes.as_ref().unwrap(), |_, _, _| {
            (Some(context.clone()), request.policy.clone())
        })
        .unwrap();
        let mut origins = vec![origin];
        if let Some(foreign) = &self.fixture.foreign {
            let scope = host.register_scope(
                foreign.tree.clone(),
                Some(StandaloneExpressionContext::default()),
                request.policy.clone(),
            );
            host.attach_captured_lexical_scopes(&foreign.captured, |_, _, _| {
                (
                    Some(StandaloneExpressionContext::default()),
                    request.policy.clone(),
                )
            })
            .unwrap();
            host.allow_scope_crossing(origin, scope);
            origins.push(scope);
        }
        self.seen.lock().unwrap().source = Some(request.source.clone());
        Some(Box::new(CemQlInputValidationSession::new(
            request,
            host,
            Inputs {
                origins,
                fixture: self.fixture.clone(),
                seen: self.seen.clone(),
            },
        )))
    }
}
#[derive(Debug)]
struct Inputs {
    origins: Vec<DeclarationScope>,
    fixture: Arc<Fixture>,
    seen: Arc<Mutex<Seen>>,
}
impl SchemaValidationSessionInputs for Inputs {
    fn runtime_context(
        &mut self,
        _: SchemaHostRuntimeContextRequest<'_>,
    ) -> Option<StandaloneExpressionContext> {
        Some(StandaloneExpressionContext::default())
    }
    fn loaded_context(&mut self, _: &SchemaUriResource) -> Option<StandaloneExpressionContext> {
        Some(StandaloneExpressionContext::default())
    }
    fn public_exports(
        &mut self,
        imported: &ScopedCemImport,
        part: &str,
    ) -> Result<Vec<SchemaDeclarationNode>, String> {
        self.seen.lock().unwrap().exports.push(part.into());
        if part != "ready" {
            return Err("Public part is not declared by this fixture vendor".into());
        }
        // This fixture vendor explicitly exports its first schema declaration as
        // 'ready'. No ID scan, query evaluation or implicit part discovery occurs.
        Ok(vec![SchemaDeclarationNode::new(
            imported.tree.ast_owner().clone(),
            element(&imported.tree, "schema"),
        )
        .unwrap()])
    }
    fn prepare_loaded(
        &mut self,
        host: &mut CemQlSchemaDeclarationHost,
        control: &SchemaHostControl,
        loaded: &SchemaUriLoadedScope,
    ) -> Result<(), Vec<Diagnostic>> {
        host.attach_captured_lexical_scopes(&loaded.resource.imported.captured, |_, _, _| {
            (
                Some(StandaloneExpressionContext::default()),
                cem_ml::schema::reference_policy::ReferenceScopePolicy::schema_defaults().unwrap(),
            )
        })
        .unwrap();
        if self.fixture.grant {
            for origin in &self.origins {
                host.allow_scope_crossing(*origin, loaded.scope);
            }
        }
        let mut seen = self.seen.lock().unwrap();
        seen.prepared.push(control.clone());
        seen.loaded.push(loaded.resource.imported.clone());
        Ok(())
    }
    fn inspected(&mut self, report: &SchemaHostRuntimeValidation) {
        self.seen.lock().unwrap().reports.push(report.clone());
    }
}
struct Reader {
    fixture: Arc<Fixture>,
    seen: Arc<Mutex<Seen>>,
}
impl ResourceResolver for Reader {
    fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        assert!(std::thread::current()
            .name()
            .unwrap()
            .starts_with("cem-ml-io-"));
        self.seen.lock().unwrap().reads.push(request.uri.clone());
        if self.fixture.hold_reads {
            // Pin the two admitted stream slots until queue admission has had a
            // chance to reject the deliberately oversized batch; always bounded.
            let gate = self.fixture.reorder.as_ref().unwrap();
            let _guard = gate
                .changed
                .wait_timeout_while(
                    gate.beta_done.lock().unwrap(),
                    Duration::from_millis(750),
                    |done| !*done,
                )
                .unwrap();
        } else if let Some(gate) = &self.fixture.reorder {
            if request.uri.ends_with("alpha.cem") {
                let (_guard, timeout) = gate
                    .changed
                    .wait_timeout_while(
                        gate.beta_done.lock().unwrap(),
                        Duration::from_secs(5),
                        |done| !*done,
                    )
                    .unwrap();
                assert!(
                    !timeout.timed_out(),
                    "beta must finish on the other I/O stream"
                );
            }
        }
        let reply = self
            .fixture
            .replies
            .get(&request.uri)
            .expect("unentered/unknown resource must not be fetched");
        self.seen.lock().unwrap().finished.push(request.uri.clone());
        if let Some(gate) = &self.fixture.reorder {
            if request.uri.ends_with("beta.cem") {
                *gate.beta_done.lock().unwrap() = true;
                gate.changed.notify_all();
            }
        }
        if reply.fail {
            return Err(ResolverDiagnostic::Io {
                uri: request.uri.clone(),
                message: "fixture transport unavailable".into(),
            });
        }
        Ok(ResolvedRead {
            uri: reply.final_uri.clone(),
            bytes: reply.text.as_bytes().to_vec(),
            content_type: Some("text/cem-ml".into()),
        })
    }
    fn write(&self, _: &ResolveRequest, _: &[u8]) -> Result<ResolvedWrite, ResolverDiagnostic> {
        unreachable!()
    }
}
fn run(fixture: Fixture) -> (EngineResult<ValidateResponse>, Arc<Mutex<Seen>>) {
    let fixture = Arc::new(fixture);
    let seen = Arc::new(Mutex::new(Seen::default()));
    let mut context = EngineContext::default();
    context.schema = Some(CEM_ML_SCHEMA_URI.into());
    context.content_type = Some(
        if fixture.xml {
            "application/xml"
        } else {
            "text/cem-ml"
        }
        .into(),
    );
    context.scheduler.max_parallel_documents = Some(1);
    let mut model = compile_schema_document_model("base", &fixture.model);
    assert!(model.is_ready_for_validation(), "{:?}", model.diagnostics);
    model.schema_uri = CEM_ML_SCHEMA_URI.into();
    context.schema_document_models.register(model);
    context.input_validation_stage = Some(Arc::new(Stage {
        fixture: fixture.clone(),
        seen: seen.clone(),
    }));
    context.resolver_registry.register(
        "https",
        ResolvePurpose::Template,
        ResolveDirection::Read,
        Reader {
            fixture: fixture.clone(),
            seen: seen.clone(),
        },
    );
    let response = RealCemMlEngine.validate(ValidateRequest {
        inputs: vec![EngineInput {
            uri: BASE.into(),
            bytes: fixture.text.as_bytes().to_vec(),
            from_format: Some(if fixture.xml {
                InputFormat::Xml
            } else {
                InputFormat::Cem
            }),
            identity: None,
            root_scope: fixture.root_scope.clone(),
        }],
        projection: ValidateProjection::Cem,
        fail_level: FailLevel::Validate,
        context,
    });
    (response, seen)
}
fn element(tree: &RetainedCemTree, name: &str) -> u32 {
    tree.ast()
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
        .unwrap()
}
fn assert_ready(response: &ValidateResponse) {
    assert!(
        response
            .report
            .report_ast
            .validation
            .as_ref()
            .unwrap()
            .complete,
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
}
fn prepared_uris(seen: &Seen) -> Vec<&str> {
    seen.prepared
        .iter()
        .map(|control| match &control.source {
            SchemaHostSource::Uri(uri) => uri.as_str(),
            _ => unreachable!(),
        })
        .collect()
}
fn assert_authored(seen: &Seen) {
    for tree in std::iter::once(seen.source.as_ref().unwrap())
        .chain(seen.loaded.iter().map(|import| &import.tree))
    {
        assert!(tree.ast().nodes.iter().all(|node| !matches!(
            node,
            CemAstNode::Reference {
                targets: Some(_),
                ..
            }
        )));
    }
}

#[test]
fn wrapping_following_and_document_prelude_controls_queue_then_preserve_enclosing_contracts() {
    for (text, xml, extent) in [
        ("{schema @src=schema.cem @own=yes | {child @selected=yes}} {outside @outer=yes}", false, SchemaScopeControlExtent::Body),
        ("{host | {schema @src=schema.cem @own=yes} {child @selected=yes}} {outside @outer=yes}", false, SchemaScopeControlExtent::Following),
        ("@schema src=schema.cem\n{child @selected=yes}", false, SchemaScopeControlExtent::Following),
        ("<container xmlns:c='https://cem.dev/ns/core/1'><c:schema src='schema.cem' own='yes'><child selected='yes'/></c:schema><outside outer='yes'/></container>", true, SchemaScopeControlExtent::Body),
        ("<container xmlns:c='https://cem.dev/ns/core/1'><host><c:schema src='schema.cem' own='yes'/><child selected='yes'/></host><outside outer='yes'/></container>", true, SchemaScopeControlExtent::Following),
    ] {
        let mut fixture = Fixture::new(text); fixture.xml = xml;
        let (response, seen) = run(fixture); let response = response.unwrap(); assert_ready(&response);
        let seen = seen.lock().unwrap();
        assert_eq!(seen.reads, ["https://app.test/input/schema.cem"]);
        assert_eq!(seen.reports.len(), 2);
        assert!(!seen.reports[0].validation.complete);
        let final_report = &seen.reports[1];
        assert_eq!(final_report.inputs[0].region().contract.extent(), extent);
        assert!(final_report.validation.nodes.iter().all(|n| Arc::ptr_eq(n.source.document(), seen.source.as_ref().unwrap().ast_owner())));
        assert_authored(&seen);
    }
}
#[test]
fn explicit_public_exports_choose_a_target_without_granting_a_crossing() {
    for grant in [true, false] {
        let mut fixture =
            Fixture::new("{host @schema-src='schema.cem#ready' | {child @selected=yes}}");
        fixture.grant = grant;
        fixture.replies.values_mut().next().unwrap().text =
            format!("{SCHEMA} {{s:schema | {{elements | {{element @name=wrong}}}}}}");
        let (response, seen) = run(fixture);
        let response = response.unwrap();
        if grant {
            assert_ready(&response);
        } else {
            assert!(
                !response
                    .report
                    .report_ast
                    .validation
                    .as_ref()
                    .unwrap()
                    .complete
            );
        }
        let seen = seen.lock().unwrap();
        assert_eq!(seen.reads, ["https://app.test/input/schema.cem"]);
        assert_eq!(seen.exports, ["ready"]);
        assert_eq!(
            seen.loaded[0].tree.source_uri(),
            "https://cdn.test/schema.cem"
        );
        assert_authored(&seen);
    }
}
#[test]
fn undeclared_public_parts_remain_incomplete_and_do_not_load_blocked_descendants() {
    let fixture = Fixture::new(
        "{host @schema-src='schema.cem#missing' | {child @schema-src=unentered.cem @selected=yes}}",
    );
    let (response, seen) = run(fixture);
    let response = response.unwrap();
    assert!(
        !response
            .report
            .report_ast
            .validation
            .as_ref()
            .unwrap()
            .complete
    );
    assert!(response
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.schema.public_part_unavailable"
            && d.uri.as_deref() == Some("https://cdn.test/schema.cem")));
    let seen = seen.lock().unwrap();
    assert_eq!(seen.exports, ["missing"]);
    assert!(seen.loaded.is_empty());
    assert_eq!(seen.reads.len(), 1);
}
#[test]
fn reference_selected_foreign_controls_use_their_original_owner_for_relative_uri_bases() {
    let foreign = import_bytes_with_lexical_scopes(
        b"{schema @src=earlier.cem} {host @schema-src=./schema.cem | {child @selected=yes}} {host @schema-src=unselected.cem | {child}}",
        "text/cem-ml",
        "https://vendor.test/library/templates.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap();
    let mut fixture = Fixture::new("{#library}");
    fixture.foreign = Some(foreign.clone());
    fixture.replies = BTreeMap::from([(
        "https://vendor.test/library/schema.cem".into(),
        Reply::schema("https://cdn.test/imported-schema.cem"),
    )]);
    let (response, seen) = run(fixture);
    let response = response.unwrap();
    assert_ready(&response);
    let seen = seen.lock().unwrap();
    assert_eq!(seen.reads, ["https://vendor.test/library/schema.cem"]);
    assert!(Arc::ptr_eq(
        seen.prepared[0].host.document(),
        foreign.tree.ast_owner()
    ));
    assert!(seen
        .reports
        .last()
        .unwrap()
        .validation
        .nodes
        .iter()
        .all(|n| Arc::ptr_eq(n.source.document(), foreign.tree.ast_owner())));
    assert_authored(&seen);
}
fn batch(overflow: &str) -> Fixture {
    let mut fixture = Fixture::new("{host @schema-src=alpha.cem | {child @selected=yes}} {host @schema-src=beta.cem | {child @selected=yes}} {host @schema-src=gamma.cem | {child @selected=yes}}");
    fixture.reorder = Some(Arc::new(Gate::default()));
    fixture.replies = ["alpha", "beta", "gamma"]
        .into_iter()
        .map(|name| {
            (
                format!("https://app.test/input/{name}.cem"),
                Reply::schema(&format!("https://cdn.test/{name}.cem")),
            )
        })
        .collect();
    fixture
        .root_scope
        .budgets
        .insert("queue".into(), "1".into());
    fixture.root_scope.budgets.insert("io".into(), "2".into());
    fixture
        .root_scope
        .budgets
        .insert("overflow".into(), overflow.into());
    fixture
}
#[test]
fn batches_preserve_completion_order_with_blocking_and_parent_spill_queue_policies() {
    for overflow in ["block", "spill-to-parent"] {
        let (response, seen) = run(batch(overflow));
        let response = response.unwrap();
        assert_ready(&response);
        let seen = seen.lock().unwrap();
        assert_eq!(prepared_uris(&seen), ["alpha.cem", "beta.cem", "gamma.cem"]);
        let beta = seen
            .finished
            .iter()
            .position(|uri| uri.ends_with("beta.cem"))
            .unwrap();
        let alpha = seen
            .finished
            .iter()
            .position(|uri| uri.ends_with("alpha.cem"))
            .unwrap();
        assert!(
            beta < alpha,
            "transport intentionally completes out of request order"
        );
        assert_eq!(seen.reports.len(), 2);
        assert_authored(&seen);
    }
}
#[test]
fn failed_peer_read_preserves_diagnostics_while_other_loaded_regions_activate() {
    let mut fixture = batch("block");
    fixture
        .replies
        .get_mut("https://app.test/input/alpha.cem")
        .unwrap()
        .fail = true;
    let (response, seen) = run(fixture);
    let response = response.unwrap();
    assert!(
        !response
            .report
            .report_ast
            .validation
            .as_ref()
            .unwrap()
            .complete
    );
    assert!(response
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.resolver.io"
            && d.uri.as_deref() == Some("https://app.test/input/alpha.cem")));
    let seen = seen.lock().unwrap();
    assert_eq!(prepared_uris(&seen), ["beta.cem", "gamma.cem"]);
    let final_report = seen.reports.last().unwrap();
    assert!(final_report.validation.failed);
    assert_eq!(final_report.validation.nodes.iter().filter(|node| matches!(node.source.node(), CemAstNode::Element { expanded_name, .. } if expanded_name.local_name == "child")).count(), 2);
    assert!(
        !response
            .report
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation() && d.message.contains("enclosing")),
        "blocked peer must not use inherited validation"
    );
    assert_eq!(
        final_report.inputs.iter().filter(|i| i.is_ready()).count(),
        2
    );
    assert_authored(&seen);
}
#[test]
fn rejecting_queue_policy_rejects_an_oversized_batch_before_any_resume() {
    let mut fixture = Fixture::new("");
    fixture.reorder = Some(Arc::new(Gate::default()));
    fixture.hold_reads = true;
    fixture.root_scope.budgets.insert("io".into(), "2".into());
    for index in 0..70 {
        fixture.text.push_str(&format!(
            "{{host @schema-src=schema{index}.cem | {{child @selected=yes}}}} "
        ));
        fixture.replies.insert(
            format!("https://app.test/input/schema{index}.cem"),
            Reply::schema(&format!("https://cdn.test/schema{index}.cem")),
        );
    }
    let (response, seen) = run(fixture);
    assert!(
        matches!(response, Err(cem_ml::engine::EngineError::Internal(ref message)) if message.contains("queue-capacity-exceeded")),
        "{response:?}"
    );
    let seen = seen.lock().unwrap();
    assert!(seen.prepared.is_empty());
    assert_eq!(seen.reports.len(), 1);
    assert!(!seen.reports[0].validation.complete);
}

#[test]
fn queued_schema_validation_keeps_foreign_source_diagnostic_uri_and_coordinates() {
    for (text, content_type) in [
        (
            "\n{host @schema-src=./schema.cem @unexpected=yes | {child}}",
            "text/cem-ml",
        ),
        (
            "\n<host schema-src='./schema.cem' unexpected='yes'><child/></host>",
            "application/xml",
        ),
    ] {
        let foreign = import_bytes_with_lexical_scopes(
            text.as_bytes(),
            content_type,
            "https://vendor.test/library/templates.cem",
            CompiledSchema::cem_core(),
        )
        .unwrap();
        let foreign_owner = foreign.tree.clone();
        // The enclosing host disallows a host child, so the relationship crosses
        // original owners and must report at the selected foreign child.
        let mut fixture = Fixture::new("{host | {#library}}");
        fixture.foreign = Some(foreign);
        fixture.replies = BTreeMap::from([(
            "https://vendor.test/library/schema.cem".into(),
            Reply::schema("https://cdn.test/imported-schema.cem"),
        )]);
        let (response, seen) = run(fixture);
        let response = response.unwrap();
        assert!(
            response
                .report
                .report_ast
                .validation
                .as_ref()
                .unwrap()
                .complete,
            "completed validation can report violations"
        );
        for (code, needle) in [
            ("cem.schema_model.missing_required_attribute", "selected"),
            ("cem.schema_model.unknown_attribute", "unexpected"),
            ("cem.schema_model.invalid_child_element", "host"),
        ] {
            let diagnostic = response
                .report
                .diagnostics
                .iter()
                .find(|d| d.code == code && d.message.contains(needle))
                .unwrap_or_else(|| panic!("missing {code}: {:?}", response.report.diagnostics));
            assert_eq!(diagnostic.uri.as_deref(), Some(foreign_owner.source_uri()));
            assert_eq!(diagnostic.line, Some(2), "{diagnostic:?}");
            let offset = diagnostic.byte_offset.unwrap();
            assert!(
                offset > 0,
                "must use node/attribute position, not whole-document origin"
            );
            let original_stack = foreign_owner
                .ast()
                .nodes
                .iter()
                .find_map(|node| match node {
                    CemAstNode::Element {
                        expanded_name,
                        source,
                        ..
                    } if (needle == "selected" && expanded_name.local_name == "child")
                        || (needle == "host" && expanded_name.local_name == "host") =>
                    {
                        Some(source)
                    }
                    CemAstNode::Attribute {
                        expanded_name,
                        source,
                        ..
                    } if needle == "unexpected" && expanded_name.local_name == "unexpected" => {
                        Some(source)
                    }
                    _ => None,
                })
                .unwrap();
            assert_eq!(diagnostic.source_map.as_ref(), Some(original_stack));
            let coordinate = cem_ml::source::line_index::LineIndex::from_utf8(text).project(offset);
            assert_eq!(diagnostic.column, Some(coordinate.column));
            assert!(!diagnostic.source_map.as_ref().unwrap().frames.is_empty());
        }
        let seen = seen.lock().unwrap();
        assert_eq!(seen.reports.len(), 2);
        assert!(seen.reports.last().unwrap().validation.failed);
        assert_authored(&seen);
    }
}
