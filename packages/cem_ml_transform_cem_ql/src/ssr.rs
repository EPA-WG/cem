//! Native inline-page builtin and explicit publication boundaries. The report
//! is control metadata; only the retained, nonportable proof authorizes export.
use cem_ml::{
    content_cache::ContentHash,
    conversion::{
        conversion_output_safety_contract,
        execute_conversion_output_pipeline_from_cem_tree_with_environment,
        ConversionOutputPipelineEnvironment, ConversionRegistry,
    },
    dependency_identity::{
        DependencyClosureState, DependencyLayer, DependencySnapshot, DependencyVersion,
        SourceDependency, RENDER_DEPENDENCY_INPUTS,
    },
    diagnostics::Diagnostic,
    engine::EngineContext,
    lifecycle::LoadedInputAstStream,
    operation_control::{cpu::NativeCpuAccounting, OperationControl},
    schema::registry::{SchemaRegistry, HTML_SCHEMA_URI},
    source_map::SourceMapStack,
    ssr_artifact::{CemSsrPageArtifact, SsrExecutionIdentity, SsrSettlePolicy, SsrSourceMapMode},
    ssr_route::{preflight_ssr_page_routes, SsrPageRouteRequest},
    transform_artifact::{
        CemtEvaluatorValue, TransformArtifactBody, TransformArtifactExportRequest,
        TransformArtifactExporter, TransformEncodedArtifact, TransformEncoding,
        TransformNativeArtifact,
    },
    transform_template::{
        TransformTemplateCompileRequest, TransformTemplateCompileResponse,
        TransformTemplateCompiledArtifact, TransformTemplateOutputArtifact,
        TransformTemplateRenderRequest, TransformTemplateRenderResponse,
        TransformTemplateRuntimeContext,
    },
};
use cem_ql::ssr_page::{
    prepare_inline_page_with_resources, ReadyInlinePage, SsrPageProgress, SsrReadinessContext,
    SSR_READY_RENDER_PLAN,
};
use std::{
    any::Any,
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

pub const ENTRYPOINT: &str = "ssr:render-page";
pub const BUILTIN_TEMPLATE: &str = include_str!("../templates/ssr-page.cemt");
pub const REPORT_CONTENT_TYPE: &str = "application/vnd.cem.ssr-report+json";
const REPRESENTATION: &str = "cem.ssr-ready-inline/1";

#[derive(Debug)]
pub(super) struct CompiledSsr;
pub(super) fn compile(
    adapter: &'static str,
    request: TransformTemplateCompileRequest<'_>,
) -> Result<TransformTemplateCompileResponse, String> {
    // Reserved builtin identity is bound to immutable package bytes, never an
    // arbitrary module that happens to declare the same entrypoint name.
    if request.template.bytes != BUILTIN_TEMPLATE.as_bytes()
        || !request.module_preflight.resolved_imports.is_empty()
    {
        return Err(
            "ssr:render-page requires the unmodified packaged ssr-page.cemt builtin".into(),
        );
    }
    Ok(TransformTemplateCompileResponse {
        artifact: TransformTemplateCompiledArtifact::new(
            adapter,
            cem_ml::engine::TransformTemplateKind::CemNative,
            request.template.uri.clone(),
            request.template.identity.clone(),
            request.entrypoint.clone(),
            serde_json::json!({"builtin":"ssr-inline/1"}),
        )
        .with_parameters(request.params.clone())
        .with_native_payload(CompiledSsr),
        diagnostics: vec![],
    })
}
fn diagnostics(ds: Vec<Diagnostic>) -> String {
    ds.iter()
        .map(|d| format!("{}: {}", d.code, d.message))
        .collect::<Vec<_>>()
        .join("; ")
}
fn string(value: Option<&CemtEvaluatorValue<'_>>, name: &str) -> Result<String, String> {
    value
        .and_then(CemtEvaluatorValue::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("SSR parameter `{name}` must be a string"))
}
pub(super) fn render(
    request: TransformTemplateRenderRequest<'_>,
    runtime: TransformTemplateRuntimeContext<'_>,
) -> Result<TransformTemplateRenderResponse, String> {
    let params = request.compiled.parameters();
    // CLI/config parameters are literal strings. Native callers may also pass
    // typed numbers; this is a control grammar, never a document/AST decoder.
    let number = |name: &str| {
        params
            .value(name)
            .and_then(|v| {
                v.as_number().and_then(|n| n.as_u64()).or_else(|| {
                    v.as_str()
                        .filter(|s| {
                            !s.is_empty()
                                && s.bytes().all(|b| b.is_ascii_digit())
                                && !s.starts_with('0')
                        })
                        .and_then(|s| s.parse().ok())
                })
            })
            .filter(|n| *n > 0)
            .ok_or_else(|| format!("SSR parameter `{name}` must be a positive integer"))
    };
    let small = |name: &str| {
        number(name).and_then(|n| {
            u32::try_from(n).map_err(|_| format!("SSR parameter `{name}` exceeds u32"))
        })
    };
    let policy = SsrSettlePolicy {
        max_declaration_depth: small("max-depth")?,
        max_instances: small("max-instances")?,
        max_render_passes: small("max-render-passes")?,
        max_resources: small("max-resources")?,
        max_total_bytes: number("max-bytes")?,
        max_wall_time_ms: number("wall-ms")?,
        max_cpu_time_ms: number("cpu-ms")?,
        allow_static_fallback: false,
    };
    if !request.secondary_inputs.is_empty() {
        return Err("Inline SSR accepts one native HTML page input".into());
    }
    let scope = request.target_scope;
    if scope
        .output_color_type
        .as_deref()
        .is_some_and(|v| v != "none")
        || scope
            .cemt_color_profile
            .as_deref()
            .is_some_and(|v| v != "none")
        || scope.cemt_colorizer.is_some()
        || scope.cemt_formatter.is_some()
        || scope
            .cemt_formatter_profile
            .as_deref()
            .is_some_and(|v| v != "compact")
        || !scope.cemt_formatter_options.is_empty()
    {
        return Err("SSR deploy HTML requires the uncolored DOM-safe formatter; publish colored source separately".into());
    }
    let cpu = Arc::new(NativeCpuAccounting::new(
        runtime.operation_control,
        policy.max_cpu_time_ms,
    )?);
    let guard = cpu.begin(runtime.operation_control)?;
    let prepared: Result<(CemSsrPageArtifact, SsrReadinessContext), String> = (|| {
        let source=match &request.primary_input.body {
            TransformArtifactBody::HtmlDomProjection(source)=>source.clone(),
            TransformArtifactBody::Lifecycle(owner)=>match owner.as_ref() {
                LoadedInputAstStream::HtmlDocument(source)=>Arc::new(source.clone()),
                _=>return Err("Inline SSR requires the shared native HTML ingress".into()),
            },
            _=>return Err("Inline SSR requires the shared native HTML ingress; encoded text/JSON is not a native handoff".into()),
        };
        let page_url = string(params.value("page-url"), "page-url")?;
        let versions = match params.value("consumer-versions") {
            Some(CemtEvaluatorValue::Record(record)) => record
                .field_names()
                .into_iter()
                .map(|key| {
                    let version = string(record.field(&key).as_ref(), "consumer-versions entry")?;
                    Ok((key, version))
                })
                .collect::<Result<BTreeMap<_, _>, String>>()?,
            Some(CemtEvaluatorValue::String(text)) => {
                if text.len() > 65536 {
                    return Err("SSR consumer-versions exceeds control bounds".into());
                }
                let mut versions = BTreeMap::new();
                for entry in text.split(',') {
                    let (tag, version) = entry
                        .split_once('=')
                        .ok_or("consumer-versions requires comma-separated tag=version entries")?;
                    if tag.is_empty()
                        || version.is_empty()
                        || versions.insert(tag.into(), version.into()).is_some()
                    {
                        return Err("consumer-versions contains an empty or duplicate entry".into());
                    }
                }
                versions
            }
            _ => return Err("SSR consumer-versions must be an explicit record".into()),
        };
        let routes =
            preflight_ssr_page_routes(&[SsrPageRouteRequest::new(&source, &page_url, None)])
                .map_err(diagnostics)?;
        let content = |bytes: &[u8]| DependencyVersion::Content(ContentHash::from_blake3(bytes));
        let parameter_hash = ContentHash::from_blake3(params.identity_bytes());
        let policy_bytes = serde_json::to_vec(&policy).map_err(|e| e.to_string())?;
        let input_stamp = content(
            &[
                source.source_text.as_bytes(),
                params.identity_bytes(),
                b"native-inline-no-transport/1",
            ]
            .concat(),
        );
        let identity = SsrExecutionIdentity {
            dependencies: DependencySnapshot {
                layer: DependencyLayer::RenderProjection,
                engine_version: cem_ml::VERSION.into(),
                format_version: "ssr-inline-builtin/1".into(),
                closure_state: DependencyClosureState::Ready,
                root_uri: source.source.uri.clone(),
                sources: BTreeMap::from([(
                    source.source.uri.clone(),
                    SourceDependency {
                        content_type: "text/html".into(),
                        schema: Some(HTML_SCHEMA_URI.into()),
                        version: content(source.source_text.as_bytes()),
                    },
                )]),
                imports: vec![],
                options: BTreeMap::from([(
                    "builtin".into(),
                    ContentHash::from_blake3(BUILTIN_TEMPLATE.as_bytes()).hex,
                )]),
                policy: content(&policy_bytes),
                resolution: content(b"native-inline-no-transport/1"),
                scope: content(&serde_json::to_vec(scope).map_err(|e| e.to_string())?),
                inputs: RENDER_DEPENDENCY_INPUTS
                    .iter()
                    .map(|input| (*input, input_stamp.clone()))
                    .collect(),
            },
            lifecycle_version: "0.2.0".into(),
            island_schema_version: "0.2.0".into(),
            render_plan_version: SSR_READY_RENDER_PLAN.into(),
            cem_ql_version: cem_ql::VERSION.into(),
            entrypoint: ENTRYPOINT.into(),
            parameters_hash: parameter_hash,
            source_map_mode: SsrSourceMapMode::Retained,
            formatter_profile: "dom-safe/1".into(),
        };
        let page = CemSsrPageArtifact::unrendered(
            source.clone(),
            &routes[0],
            TransformArtifactBody::HtmlDomProjection(source.clone()),
            identity,
            policy,
        )
        .map_err(|e| format!("SSR page admission: {e:?}"))?;
        let context = SsrReadinessContext::new(&page, versions).map_err(diagnostics)?;
        Ok((page, context))
    })();
    guard.finish()?;
    let (page, context) = prepared?;
    // Scoped URL resolution grants no transport. Pending external tickets
    // are refused below; inline CSS/payloads retain their native source leases.
    use cem_ml::module_resolution::{
        CemModuleUrlContext, CemModuleUrlFrame, CemModuleUrlResolutionCapability,
        CemResolutionContextHandle, CemScopedModuleUrlResolver,
    };
    let handle = CemResolutionContextHandle::new("native-inline-no-transport/1");
    let resolver = CemScopedModuleUrlResolver::new().with_context(
        handle.clone(),
        CemModuleUrlContext {
            identity: "native-inline-page/1".into(),
            resolver_identity: "native-inline-no-transport/1".into(),
            resource_policy_stamp: "native-inline-no-transport/1".into(),
            frames: vec![CemModuleUrlFrame::new(
                "page",
                &page.route().document_base_url,
            )],
        },
    );
    let capability = CemModuleUrlResolutionCapability::new(Arc::new(resolver), handle);
    let mut coordinator =
        prepare_inline_page_with_resources(page, "ssr", capability, runtime.operation_control)
            .map_err(diagnostics)?;
    loop {
        match coordinator
            .advance(runtime.operation_control)
            .map_err(diagnostics)?
        {
            SsrPageProgress::Quiescent => break,
            SsrPageProgress::Pending(_) => {
                return Err("This SSR builtin grants no external resource reads".into())
            }
            SsrPageProgress::Rendered { .. } => {}
        }
    }
    let ready = coordinator
        .finish(runtime.operation_control)
        .map_err(diagnostics)?
        .prove_ready(&context, runtime.operation_control)
        .map_err(diagnostics)?;
    let source_map = ready.page().source_map().cloned();
    Ok(TransformTemplateRenderResponse {
        output: TransformTemplateOutputArtifact {
            uri: Some(ready.page().route().page_url.clone()),
            identity: request.target.cloned(),
            body: TransformArtifactBody::Extension(Arc::new(ReadyArtifact {
                ready,
                context,
                control: runtime.operation_control.clone(),
                cpu,
                exported_bytes: AtomicU64::new(0),
            })),
            source_map,
            output_spans: vec![],
        },
        diagnostics: vec![],
    })
}
#[derive(Debug)]
struct ReadyArtifact {
    ready: ReadyInlinePage,
    context: SsrReadinessContext,
    control: OperationControl,
    cpu: Arc<NativeCpuAccounting>,
    exported_bytes: AtomicU64,
}
impl TransformNativeArtifact for ReadyArtifact {
    fn representation_id(&self) -> &'static str {
        REPRESENTATION
    }
    fn source_map(&self) -> Option<&SourceMapStack> {
        self.ready.page().source_map()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn validate_publication(
        &self,
        control: &OperationControl,
        scope: &cem_ml::run_config::ScopeConfig,
        rewrites_document: bool,
    ) -> Result<(), String> {
        if rewrites_document
            || scope
                .output_color_type
                .as_deref()
                .is_some_and(|v| v != "none")
            || scope
                .cemt_color_profile
                .as_deref()
                .is_some_and(|v| v != "none")
            || scope.cemt_colorizer.is_some()
            || scope.cemt_formatter.is_some()
            || scope
                .cemt_formatter_profile
                .as_deref()
                .is_some_and(|v| v != "compact")
            || !scope.cemt_formatter_options.is_empty()
        {
            return Err(
                "Native SSR publication requires unchanged, uncolored DOM-safe HTML".into(),
            );
        }
        self.ready
            .validate_current(self.ready.tree(), &self.context, control)
            .map_err(diagnostics)?;
        Ok(())
    }
}
pub(super) struct ReadyExporter {
    schemas: SchemaRegistry,
    conversions: ConversionRegistry,
}
impl ReadyExporter {
    pub(super) fn new(context: &EngineContext) -> Self {
        Self {
            schemas: context.schema_registry.clone(),
            conversions: context.converter_registry.clone(),
        }
    }
}
impl TransformArtifactExporter for ReadyExporter {
    fn id(&self) -> &'static str {
        "cem.ssr.ready-html-or-report"
    }
    fn representation_id(&self) -> &'static str {
        REPRESENTATION
    }
    fn export(
        &self,
        request: TransformArtifactExportRequest<'_>,
    ) -> Result<Arc<TransformEncodedArtifact>, String> {
        let TransformArtifactBody::Extension(body) = request.body else {
            return Err("SSR export requires its native readiness proof".into());
        };
        let artifact = body
            .as_any()
            .downcast_ref::<ReadyArtifact>()
            .ok_or("SSR representation has no native readiness proof")?;
        let observation = artifact
            .ready
            .validate_current(artifact.ready.tree(), &artifact.context, &artifact.control)
            .map_err(diagnostics)?;
        let guard = artifact.cpu.begin(&artifact.control)?;
        let encoded = (|| {
            let media = request.target.content_type.as_deref().unwrap_or("");
            let (encoding,bytes)=match (media,request.target.schema.as_deref()) {
                ("text/html",None|Some(HTML_SCHEMA_URI))=> {
                    let mut descriptor=self.conversions.converter("cem-dom-projection-to-html-cemt").ok_or("Missing native DOM-to-HTML converter")?.clone();
                    descriptor.output_contract.color_profile=Some("none".into());
                    let (contract,ds)=conversion_output_safety_contract(&descriptor);
                    if !ds.is_empty() {return Err(diagnostics(ds))}
                    let contract=contract.ok_or("Missing DOM-safe HTML writer contract")?;
                    let environment=ConversionOutputPipelineEnvironment {schema_registry:&self.schemas,conversion_registry:&self.conversions,package_artifact_reader:None,artifact_cache:None};
                    let stream=Arc::new(cem_ml::projection::html_dom_writer_stream(artifact.ready.tree().clone(),artifact.ready.page().source_document(),&artifact.control)?);
                    let output=execute_conversion_output_pipeline_from_cem_tree_with_environment(&environment,&contract.pipeline,stream,artifact.ready.page().source_map().cloned(),vec![],descriptor.id.as_str(),None,Some(&artifact.ready.page().route().source_uri));
                    if output.diagnostics.iter().any(|d|d.severity.is_hard_violation()) {return Err(diagnostics(output.diagnostics))}
                    let text=output.output.as_ref().and_then(serde_json::Value::as_str).ok_or("DOM-safe formatter returned no HTML")?;
                    (TransformEncoding::Text,text.as_bytes().to_vec())
                },
                (REPORT_CONTENT_TYPE,None)=> {
                    let page=artifact.ready.page();
                    let participants=artifact.ready.participants().iter().map(|p|serde_json::json!({"tag":p.tag(),"instanceId":p.instance_id(),"depth":p.depth()})).collect::<Vec<_>>();
                    let report=serde_json::json!({"schema":"https://cem.dev/ns/ssr-report/1","readiness":"native-ready","hydrationReady":false,"route":page.route(),"renderInputIdentity":page.render_input_identity().map_err(|e|format!("{e:?}"))?.hex,"usage":observation.usage(),"previousExportBytes":artifact.exported_bytes.load(Ordering::Relaxed),"participants":participants});
                    (TransformEncoding::Json,serde_json::to_vec(&report).map_err(|e|e.to_string())?)
                },
                _=>return Err("SSR requires an explicit text/html or application/vnd.cem.ssr-report+json export identity".into()),
            };
            let size = u64::try_from(bytes.len()).map_err(|_| "SSR export size overflow")?;
            let total = artifact
                .exported_bytes
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                    old.checked_add(size)
                })
                .map_err(|_| "SSR export byte accounting overflow")?;
            if observation
                .usage()
                .total_bytes
                .checked_add(total)
                .and_then(|n| n.checked_add(size))
                .is_none_or(|n| n > artifact.ready.page().policy().max_total_bytes)
            {
                return Err("SSR export exceeds original byte budget".into());
            }
            TransformEncodedArtifact::new(request.target.clone(), encoding, bytes)
                .map(Arc::new)
                .map_err(|e| e.to_string())
        })();
        guard.finish()?;
        artifact
            .ready
            .validate_current(artifact.ready.tree(), &artifact.context, &artifact.control)
            .map_err(diagnostics)?;
        encoded
    }
}

#[cfg(all(test, any(target_os = "linux", windows)))]
mod tests {
    use super::*;
    use cem_ml::{
        engine::{
            FormatIdentity, TransformExecutionPolicy, TransformTemplateEntrypoint,
            TransformTemplateKind,
        },
        transform_template::{TransformTemplateDataArtifact, TransformTemplateParameterArena},
        validation::html::{html_document_ast_from_source_bytes, HtmlSourceValidationRequest},
    };
    fn native() -> (TransformArtifactBody, EngineContext) {
        let context = crate::engine_context_with_cem_ql_template_adapter();
        let params = TransformTemplateParameterArena::from_normalized_values(BTreeMap::from([
            ("page-url".into(), serde_json::json!("https://ready.test/")),
            ("consumer-versions".into(), serde_json::json!("x-a=1.9.0")),
            ("max-depth".into(), serde_json::json!("8")),
            ("max-instances".into(), serde_json::json!("8")),
            ("max-render-passes".into(), serde_json::json!("8")),
            ("max-resources".into(), serde_json::json!("16")),
            ("max-bytes".into(), serde_json::json!("32768")),
            ("wall-ms".into(), serde_json::json!("60000")),
            ("cpu-ms".into(), serde_json::json!("10000")),
        ]))
        .unwrap();
        let compiled = TransformTemplateCompiledArtifact::new(
            "test",
            TransformTemplateKind::CemNative,
            "file:///builtin.cemt",
            None,
            TransformTemplateEntrypoint::named(ENTRYPOINT),
            serde_json::Value::Null,
        )
        .with_parameters(params);
        let source=Arc::new(html_document_ast_from_source_bytes(HtmlSourceValidationRequest {bytes:br#"<!doctype html><html><head><title>SSR</title></head><body><cem-element tag="x-a" version="1.2.3"><template id="x-a" type="text/cem-ml">{b | Ready}</template></cem-element><x-a></x-a></body></html>"#,source_uri:"file:///source.html",content_type:Some("text/html")}).0.unwrap());
        let primary = TransformTemplateDataArtifact::new(
            "input",
            None,
            None,
            TransformArtifactBody::HtmlDomProjection(source),
        );
        let output = render(
            TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary,
                secondary_inputs: &BTreeMap::new(),
                target: Some(&FormatIdentity {
                    content_type: Some("text/html".into()),
                    ..Default::default()
                }),
                target_scope: &Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            },
            TransformTemplateRuntimeContext {
                resolver_registry: &context.resolver_registry,
                resolver_policy: &context.resolver_policy,
                operation_control: &context.operation_control,
                execution_scope: cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
            },
        )
        .unwrap();
        (output.output.body, context)
    }
    #[test]
    fn fresh_exports_are_deterministic_and_late_cancellation_refuses_the_retained_proof() {
        let (body, c) = native();
        let target = cem_ml::engine::FormatIdentity {
            content_type: Some("text/html".into()),
            ..Default::default()
        };
        let first = c
            .transform_artifact_exporter_registry
            .export(&body, &target)
            .unwrap();
        let second = c
            .transform_artifact_exporter_registry
            .export(&body, &target)
            .unwrap();
        assert_eq!(first.bytes, second.bytes);
        let TransformArtifactBody::Extension(native) = &body else {
            panic!()
        };
        let native = native.as_any().downcast_ref::<ReadyArtifact>().unwrap();
        assert_eq!(
            native.exported_bytes.load(Ordering::Relaxed),
            2 * first.bytes.len() as u64
        );
        assert!(native.cpu.is_idle());
        c.abort_signal().abort();
        assert!(c
            .transform_artifact_exporter_registry
            .export(&body, &target)
            .is_err());
    }
    #[test]
    fn generic_envelope_and_report_metadata_cannot_authorize_html_export() {
        let (body, c) = native();
        let TransformArtifactBody::Extension(native) = &body else {
            panic!()
        };
        let native = native.as_any().downcast_ref::<ReadyArtifact>().unwrap();
        let partial = native
            .ready
            .page()
            .clone()
            .into_transform_artifact("partial");
        let target = cem_ml::engine::FormatIdentity {
            content_type: Some("text/html".into()),
            ..Default::default()
        };
        assert!(c
            .transform_artifact_exporter_registry
            .export(&partial.body, &target)
            .is_err());
        assert!(!native.ready.page().adoption_identity().hydration_ready());
        assert!(native
            .ready
            .validate_current(
                native.ready.tree(),
                &native.context,
                &OperationControl::default()
            )
            .is_err());
        assert!(native
            .validate_publication(&OperationControl::default(), &Default::default(), false)
            .is_err());
        assert!(native
            .validate_publication(&c.operation_control, &Default::default(), true)
            .is_err());
        assert!(native
            .validate_publication(
                &c.operation_control,
                &cem_ml::run_config::ScopeConfig {
                    output_color_type: Some("html".into()),
                    ..Default::default()
                },
                false
            )
            .is_err());
    }
    #[test]
    fn sequential_exports_share_the_original_byte_budget() {
        let (body, c) = native();
        let target = cem_ml::engine::FormatIdentity {
            content_type: Some("text/html".into()),
            ..Default::default()
        };
        let mut successes = 0;
        while c
            .transform_artifact_exporter_registry
            .export(&body, &target)
            .is_ok()
        {
            successes += 1;
            assert!(successes < 100);
        }
        assert!(successes > 1);
        assert!(c
            .transform_artifact_exporter_registry
            .export(&body, &target)
            .is_err());
    }
}
