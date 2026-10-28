//! Typed XSLT execution. Legacy conversion is a separate authoring tool.
use super::*;
use cem_ml::content_cache::ContentHash;
use cem_ql::xslt::{
    compiler::{
        compile_xslt_bundle_with_options_and_parser_contract,
        resolve_xslt_names_with_parser_contract, XsltCompileOptions, XsltModuleSource,
    },
    XsltBundle,
};

#[derive(Debug)]
struct Payload {
    bundle: XsltBundle,
    parameters: BTreeMap<String, String>,
}

impl TransformTemplateAdapter for XsltParityTransformTemplateAdapter {
    fn id(&self) -> &'static str {
        XSLT_PARITY_TEMPLATE_ADAPTER_ID
    }
    fn kind(&self) -> TransformTemplateKind {
        TransformTemplateKind::Xslt
    }
    fn capability(&self) -> TransformTemplateAdapterCapability {
        TransformTemplateAdapterCapability::Executable
    }
    fn matches_template(&self, identity: &FormatIdentity) -> bool {
        matches_xslt_identity(identity)
    }
    fn compile(
        &self,
        request: TransformTemplateCompileRequest<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateCompileResponse> {
        compile_response(self, request, None)
    }
    fn compile_with_context(
        &self,
        request: TransformTemplateCompileRequest<'_>,
        context: &EngineContext,
    ) -> TransformTemplateAdapterResult<TransformTemplateCompileResponse> {
        compile_response(self, request, Some(context))
    }
    fn render(
        &self,
        request: TransformTemplateRenderRequest<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateRenderResponse> {
        render(self.id(), request, None)
    }
    fn render_with_runtime(
        &self,
        request: TransformTemplateRenderRequest<'_>,
        runtime: TransformTemplateRuntimeContext<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateRenderResponse> {
        render(self.id(), request, Some(runtime))
    }
}

fn compile_response(
    adapter: &XsltParityTransformTemplateAdapter,
    request: TransformTemplateCompileRequest<'_>,
    context: Option<&EngineContext>,
) -> TransformTemplateAdapterResult<TransformTemplateCompileResponse> {
    let mut artifact = TransformTemplateCompiledArtifact::new(
        adapter.id(),
        adapter.kind(),
        request.template.uri.clone(),
        request.template.identity.clone(),
        request.entrypoint.clone(),
        json!({"engine":"cem-ql", "source":"typed-xslt-bundle"}),
    )
    .with_parameters(request.params.clone());
    let compiled = match context {
        Some(context) => cem_ml::validation::xslt::xslt_parser_contract_from_context(
            context,
            &request.template.uri,
        )
        .and_then(|parser| compile(&request, parser.as_deref())),
        None => compile(&request, None),
    };
    let diagnostics = match compiled {
        Ok(payload) => {
            artifact = artifact.with_native_payload(payload);
            Vec::new()
        }
        Err(mut diagnostics) => {
            for diagnostic in &mut diagnostics {
                if diagnostic.severity.is_hard_violation() {
                    diagnostic.severity = Severity::Fatal;
                }
            }
            diagnostics
        }
    };
    Ok(TransformTemplateCompileResponse {
        artifact,
        diagnostics,
    })
}

fn diagnostic(uri: &str, message: impl Into<String>) -> Vec<Diagnostic> {
    vec![Diagnostic {
        uri: Some(uri.into()),
        code: "cem.xslt.compile_host".into(),
        severity: Severity::Error,
        message: message.into(),
        ..Default::default()
    }]
}
fn compile(
    request: &TransformTemplateCompileRequest<'_>,
    parser: Option<&cem_ml::schema::parser_contracts::CompiledParserContract>,
) -> Result<Payload, Vec<Diagnostic>> {
    let uri = &request.template.uri;
    let source =
        std::str::from_utf8(&request.template.bytes).map_err(|e| diagnostic(uri, e.to_string()))?;
    if request.module_options.limits.max_recursion_depth
        != cem_ml::transform_template::TransformTemplateModuleLimits::default().max_recursion_depth
    {
        return Err(diagnostic(
            uri,
            "custom recursion limits are not supported by the bounded XSLT dispatch profile",
        ));
    }
    let mut options = XsltCompileOptions::default();
    let mut names = Vec::new();
    if let Some(entrypoint) = request.entrypoint.name.as_deref() {
        names.push(entrypoint);
    }
    names.extend(request.params.iter().map(|(name, _)| name));
    let mut expanded =
        resolve_xslt_names_with_parser_contract(source, uri, &names, parser)?.into_iter();
    if request.entrypoint.name.is_some() {
        options.entrypoint = expanded.next();
    }
    let mut parameters = BTreeMap::new();
    for ((name, _), expanded) in request.params.iter().zip(expanded) {
        let binding = format!("host_param_{}", parameters.len());
        if options
            .parameters
            .insert(expanded, binding.clone())
            .is_some()
        {
            return Err(diagnostic(uri, "duplicate expanded parameter name"));
        }
        parameters.insert(name.to_owned(), binding);
    }
    for module in &request.module_preflight.resolved_imports {
        let hash = ContentHash::from_blake3(&module.bytes);
        if hash.header_value() != module.content_hash {
            return Err(diagnostic(&module.uri, "preflighted module hash mismatch"));
        }
        options.modules.push(XsltModuleSource {
            parent_uri: module.parent_uri.clone().unwrap_or_else(|| uri.clone()),
            href: module.alias.clone(),
            uri: module.uri.clone(),
            source: String::from_utf8(module.bytes.clone())
                .map_err(|e| diagnostic(&module.uri, e.to_string()))?,
            content_hash: hash,
        });
    }
    let compiled =
        compile_xslt_bundle_with_options_and_parser_contract(source, uri, &options, parser)?;
    let bundle = XsltBundle::from_bytes(
        &compiled.bytes,
        &compiled.content_hash,
        &compiled.source_hash,
    )
    .map_err(|e| diagnostic(uri, e.to_string()))?;
    Ok(Payload { bundle, parameters })
}
fn render(
    adapter: &'static str,
    request: TransformTemplateRenderRequest<'_>,
    runtime: Option<TransformTemplateRuntimeContext<'_>>,
) -> TransformTemplateAdapterResult<TransformTemplateRenderResponse> {
    let error = |message| {
        TransformTemplateAdapterError::failed(
            adapter,
            TransformTemplateAdapterExecutionPhase::Render,
            message,
        )
    };
    let payload = request
        .compiled
        .native_payload::<Payload>()
        .ok_or_else(|| error("compiled artifact is not a valid typed XSLT bundle".into()))?;
    if !request.secondary_inputs.is_empty() {
        return Err(error(
            "secondary XSLT document bindings are not supported by this profile".into(),
        ));
    }
    let mut data = TemplateData::default()
        .with_binding("document", document(request.primary_input).map_err(error)?);
    for (name, value) in request.compiled.parameters().iter() {
        let binding = payload
            .parameters
            .get(name)
            .ok_or_else(|| error("compiled XSLT parameter binding missing".into()))?;
        // Parameters are explicit host control values, never a document AST handoff.
        data.bindings
            .insert(binding.clone(), parameter(value).map_err(error)?);
    }
    let plan = match runtime {
        Some(runtime) => payload.bundle.render_with_control(
            &data,
            runtime.operation_control,
            runtime.execution_scope,
        ),
        None => payload.bundle.render(&data),
    };
    render_plan_output(adapter, request, runtime, plan)
}

// The engine's native CEM arena is already an internal AST. Retain it through
// the existing shared tree constructor; do not serialize or parse it again.
fn document(artifact: &TransformTemplateDataArtifact) -> Result<ItemStream, String> {
    if let TransformArtifactBody::CemDocument(owner) = &artifact.body {
        let ast = CemDocument {
            nodes: owner.nodes.clone(),
            id_table: owner.id_table.clone(),
            unresolved_slots: owner.unresolved_slots.clone(),
            diagnostics: owner.diagnostics.clone(),
            format_identity: owner.format_identity.clone(),
            typed_preludes: owner.typed_preludes.clone(),
        };
        let tree = cem_ml::parser::tree::RetainedCemTree::new(
            ast,
            artifact.uri.as_deref().unwrap_or("memory:transform-input"),
            "",
            Default::default(),
            Some(owner.clone()),
        )?;
        return Ok(ItemStream::once(cem_ql::eval::imported_cem_tree(tree)));
    }
    artifact_query_stream(artifact)
}

fn parameter(value: &CemtEvaluatorValue<'_>) -> Result<ItemStream, String> {
    match value.kind() {
        CemtEvaluatorValueKind::Null => Ok(ItemStream::default()),
        CemtEvaluatorValueKind::Boolean
        | CemtEvaluatorValueKind::Number
        | CemtEvaluatorValueKind::String => evaluator_param_value_to_stream(value),
        kind => Err(format!(
            "XSLT host controls must be scalars; got {}",
            kind.as_str()
        )),
    }
}
