//! CEM-native transform-template adapter backed by `cem_ql::render`.
//!
//! This crate intentionally sits above both `cem_ml` and `cem_ql`: `cem_ml`
//! owns the stable transform adapter contract, while `cem_ql` owns the current
//! CEM-native fragment renderer. Keeping the bridge here avoids a dependency
//! cycle from `cem_ml` back into `cem_ql`.

use std::collections::BTreeMap;

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::engine::{
    EngineContext, FormatIdentity, TemplateInput, TransformTemplateKind,
    TRANSFORM_TEMPLATE_UNSUPPORTED_CODE,
};
use cem_ml::run_config::ScopeConfig;
use cem_ml::transform_template::{
    parse_cem_native_template_module_options, TransformTemplateAdapter,
    TransformTemplateAdapterCapability, TransformTemplateAdapterError,
    TransformTemplateAdapterExecutionPhase, TransformTemplateAdapterRegistry,
    TransformTemplateAdapterResult, TransformTemplateCompileRequest,
    TransformTemplateCompileResponse, TransformTemplateCompiledArtifact,
    TransformTemplateDataArtifact, TransformTemplateModuleOptions,
    TransformTemplateModuleParamDeclaration, TransformTemplateModuleParseRequest,
    TransformTemplateModulePreflight, TransformTemplateOutputArtifact,
    TransformTemplateRenderRequest, TransformTemplateRenderResponse,
    CEM_NATIVE_TEMPLATE_SCHEMA_URI, TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE,
};
use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{
    compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
    RenderPlan, RenderPlanAttribute, RenderPlanNode, TemplateArtifact, TemplateAttributeValue,
    TemplateData, TemplateNode,
};
use serde_json::{json, Map, Number, Value};

pub const CEM_QL_TEMPLATE_ADAPTER_ID: &str = "cem-ql-cem-native-template";

#[derive(Debug, Clone, Default)]
pub struct CemQlTransformTemplateAdapter;

#[derive(Debug, Clone)]
struct CemQlCompiledTemplatePayload {
    template_uri: String,
    artifact: TemplateArtifact,
    selected_entrypoint: Option<String>,
    params: BTreeMap<String, Value>,
    param_declarations: Vec<TransformTemplateModuleParamDeclaration>,
    entrypoints: CemQlTemplateEntrypoints,
    modules: Vec<CemQlCompiledTemplateModulePayload>,
    max_recursion_depth: u32,
}

#[derive(Debug, Clone)]
struct CemQlCompiledTemplateModulePayload {
    alias: String,
    parent_uri: Option<String>,
    uri: String,
    content_hash: String,
    artifact: TemplateArtifact,
    entrypoints: CemQlTemplateEntrypoints,
    imports: BTreeMap<String, String>,
    param_declarations: Vec<TransformTemplateModuleParamDeclaration>,
}

#[derive(Debug, Clone, Default)]
struct CemQlTemplateEntrypoints {
    implicit: Option<TemplateArtifact>,
    named: BTreeMap<String, TemplateArtifact>,
}

impl TransformTemplateAdapter for CemQlTransformTemplateAdapter {
    fn id(&self) -> &'static str {
        CEM_QL_TEMPLATE_ADAPTER_ID
    }

    fn kind(&self) -> TransformTemplateKind {
        TransformTemplateKind::CemNative
    }

    fn capability(&self) -> TransformTemplateAdapterCapability {
        TransformTemplateAdapterCapability::Executable
    }

    fn matches_template(&self, identity: &FormatIdentity) -> bool {
        matches_cem_native_identity(identity)
    }

    fn compile(
        &self,
        request: TransformTemplateCompileRequest<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateCompileResponse> {
        let source = std::str::from_utf8(&request.template.bytes).map_err(|err| {
            TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Compile,
                format!(
                    "template `{}` is not valid UTF-8: {err}",
                    request.template.uri
                ),
            )
        })?;
        let host_bindings = host_binding_names(
            request.params,
            request.data_bindings,
            &request.module_options,
        );
        let artifact = compile_template(
            source,
            &CompileTemplateOptions {
                host_bindings: host_bindings.clone(),
            },
        );
        let mut diagnostics =
            diagnostics_with_uri(&artifact.diagnostics, request.template.uri.as_str());
        let entrypoints = extract_template_entrypoints(&artifact);
        let render_artifact =
            select_entrypoint_artifact(&artifact, &entrypoints, request.entrypoint.name.as_deref());
        let mut modules = compile_preflighted_modules(self.id(), &request, &host_bindings)?;
        for module in &modules {
            diagnostics.extend(diagnostics_with_uri(
                &module.artifact.diagnostics,
                module.uri.as_str(),
            ));
        }
        let module_diagnostics = modules
            .iter()
            .map(|module| module.artifact.diagnostics.len())
            .sum::<usize>();
        let module_metadata = modules
            .iter()
            .map(|module| {
                json!({
                    "alias": module.alias,
                    "uri": module.uri,
                    "contentHash": module.content_hash,
                    "diagnostics": module.artifact.diagnostics.len(),
                })
            })
            .collect::<Vec<_>>();
        let mut render_artifact = render_artifact;
        let mut entrypoints = entrypoints;
        clear_template_artifact_diagnostics(&mut render_artifact);
        clear_template_entrypoint_diagnostics(&mut entrypoints);
        for module in &mut modules {
            clear_template_artifact_diagnostics(&mut module.artifact);
            clear_template_entrypoint_diagnostics(&mut module.entrypoints);
        }
        let opaque = json!({
            "engine": "cem-ql",
            "templateBytes": request.template.bytes.len(),
            "diagnostics": artifact.diagnostics.len(),
            "moduleImports": modules.len(),
            "moduleDiagnostics": module_diagnostics,
            "modules": module_metadata,
            "moduleCacheKey": request.module_preflight.cache_key.clone(),
        });

        Ok(TransformTemplateCompileResponse {
            artifact: TransformTemplateCompiledArtifact::new(
                self.id(),
                self.kind(),
                request.template.uri.clone(),
                request.template.identity.clone(),
                request.entrypoint.clone(),
                opaque,
            )
            .with_native_payload(CemQlCompiledTemplatePayload {
                template_uri: request.template.uri.clone(),
                artifact: render_artifact,
                selected_entrypoint: request.entrypoint.name.clone(),
                params: request.params.clone(),
                param_declarations: request.module_options.params.clone(),
                entrypoints,
                modules,
                max_recursion_depth: request.module_options.limits.max_recursion_depth,
            }),
            diagnostics,
        })
    }

    fn render(
        &self,
        request: TransformTemplateRenderRequest<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateRenderResponse> {
        let payload = request
            .compiled
            .native_payload::<CemQlCompiledTemplatePayload>()
            .ok_or_else(|| {
                TransformTemplateAdapterError::failed(
                    self.id(),
                    TransformTemplateAdapterExecutionPhase::Render,
                    "compiled template artifact was not produced by the CEM-QL adapter",
                )
            })?;
        let data = template_data_from_artifacts(request.primary_input, request.secondary_inputs);
        let plan = render_payload_template(payload, &data);
        let rendered = render_plan_to_html(&plan);
        let identity = request.target.cloned().or_else(|| {
            Some(FormatIdentity {
                content_type: Some("text/html".to_owned()),
                ..FormatIdentity::default()
            })
        });

        Ok(TransformTemplateRenderResponse {
            output: TransformTemplateOutputArtifact {
                uri: None,
                identity,
                value: Value::String(rendered),
            },
            diagnostics: plan.diagnostics,
        })
    }
}

pub fn register_cem_ql_template_adapter(registry: &mut TransformTemplateAdapterRegistry) {
    registry.register(CemQlTransformTemplateAdapter);
}

pub fn engine_context_with_cem_ql_template_adapter() -> EngineContext {
    let mut context = EngineContext::default();
    register_cem_ql_template_adapter(&mut context.template_adapter_registry);
    context
}

fn matches_cem_native_identity(identity: &FormatIdentity) -> bool {
    if let Some(content_type) = identity.content_type.as_deref() {
        return matches!(
            content_type_essence(content_type).as_str(),
            "application/cem+xml" | "application/cem" | "text/cem" | "text/cem-ml"
        );
    }

    let schema = identity
        .schema
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    if !schema.is_empty() {
        return schema == CEM_NATIVE_TEMPLATE_SCHEMA_URI
            || schema == cem_ml::schema::ir::CEM_CORE_NAMESPACE;
    }

    identity.default_namespace.as_deref() == Some(cem_ml::schema::ir::CEM_CORE_NAMESPACE)
        || identity
            .namespaces
            .values()
            .any(|uri| uri == cem_ml::schema::ir::CEM_CORE_NAMESPACE)
}

fn content_type_essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

fn host_binding_names(
    params: &BTreeMap<String, Value>,
    data_bindings: &[String],
    module_options: &TransformTemplateModuleOptions,
) -> Vec<String> {
    let mut bindings = data_bindings.to_vec();
    for name in params.keys() {
        push_binding_name(&mut bindings, name);
    }
    extend_module_param_binding_names(&mut bindings, module_options);
    bindings
}

fn extend_module_param_binding_names(
    bindings: &mut Vec<String>,
    module_options: &TransformTemplateModuleOptions,
) {
    for param in &module_options.params {
        push_binding_name(bindings, &param.name);
        if let Some((_, local)) = param.name.split_once('.') {
            push_binding_name(bindings, local);
        }
    }
}

fn push_binding_name(bindings: &mut Vec<String>, name: &str) {
    if !bindings.iter().any(|binding| binding == name) {
        bindings.push(name.to_owned());
    }
}

fn compile_preflighted_modules(
    adapter_id: &'static str,
    request: &TransformTemplateCompileRequest<'_>,
    host_bindings: &[String],
) -> TransformTemplateAdapterResult<Vec<CemQlCompiledTemplateModulePayload>> {
    request
        .module_preflight
        .resolved_imports
        .iter()
        .map(|module| {
            let source = std::str::from_utf8(&module.bytes).map_err(|err| {
                TransformTemplateAdapterError::failed(
                    adapter_id,
                    TransformTemplateAdapterExecutionPhase::Compile,
                    format!("template module `{}` is not valid UTF-8: {err}", module.uri),
                )
            })?;
            let module_options = parse_imported_module_options(
                adapter_id,
                module.uri.clone(),
                module.bytes.clone(),
                module.identity.clone(),
            )?;
            let mut module_host_bindings = host_bindings.to_vec();
            extend_module_param_binding_names(&mut module_host_bindings, &module_options);
            let artifact = compile_template(
                source,
                &CompileTemplateOptions {
                    host_bindings: module_host_bindings,
                },
            );
            let entrypoints = extract_template_entrypoints(&artifact);
            Ok(CemQlCompiledTemplateModulePayload {
                alias: module.alias.clone(),
                parent_uri: module.parent_uri.clone(),
                uri: module.uri.clone(),
                content_hash: module.content_hash.clone(),
                artifact,
                entrypoints,
                imports: imported_module_aliases(&request.module_preflight, module.uri.as_str()),
                param_declarations: module_options.params,
            })
        })
        .collect()
}

fn imported_module_aliases(
    module_preflight: &TransformTemplateModulePreflight,
    parent_uri: &str,
) -> BTreeMap<String, String> {
    module_preflight
        .resolved_imports
        .iter()
        .filter(|module| module.parent_uri.as_deref() == Some(parent_uri))
        .map(|module| (module.alias.clone(), module.uri.clone()))
        .collect()
}

fn parse_imported_module_options(
    adapter_id: &'static str,
    uri: String,
    bytes: Vec<u8>,
    identity: Option<FormatIdentity>,
) -> TransformTemplateAdapterResult<TransformTemplateModuleOptions> {
    let response = parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
        template: TemplateInput {
            uri: uri.clone(),
            bytes,
            identity,
            root_scope: ScopeConfig::default(),
        },
    });
    if let Some(diagnostic) = response
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity == Severity::Fatal)
    {
        return Err(TransformTemplateAdapterError::failed(
            adapter_id,
            TransformTemplateAdapterExecutionPhase::Compile,
            format!(
                "template module `{uri}` declarations failed to lower: {}",
                diagnostic.message
            ),
        ));
    }
    Ok(response.module_options)
}

fn extract_template_entrypoints(artifact: &TemplateArtifact) -> CemQlTemplateEntrypoints {
    let mut entrypoints = CemQlTemplateEntrypoints::default();
    let Some(module) = artifact.nodes.iter().find_map(module_node_children) else {
        entrypoints.implicit = Some(artifact.clone());
        return entrypoints;
    };

    for node in module {
        let TemplateNode::Element {
            tag,
            attributes,
            children,
            ..
        } = node
        else {
            continue;
        };
        match local_name(tag) {
            "body" => {
                entrypoints.implicit = Some(artifact_from_nodes(artifact, children.clone()));
            }
            "template" => {
                let Some(name) = literal_attribute(attributes, "name") else {
                    continue;
                };
                if let Some(body) = children.iter().find_map(body_node_children) {
                    entrypoints
                        .named
                        .insert(name, artifact_from_nodes(artifact, body.clone()));
                }
            }
            _ => {}
        }
    }

    entrypoints
}

fn module_node_children(node: &TemplateNode) -> Option<&Vec<TemplateNode>> {
    let TemplateNode::Element { tag, children, .. } = node else {
        return None;
    };
    (local_name(tag) == "module").then_some(children)
}

fn body_node_children(node: &TemplateNode) -> Option<&Vec<TemplateNode>> {
    let TemplateNode::Element { tag, children, .. } = node else {
        return None;
    };
    (local_name(tag) == "body").then_some(children)
}

fn artifact_from_nodes(source: &TemplateArtifact, nodes: Vec<TemplateNode>) -> TemplateArtifact {
    TemplateArtifact {
        nodes,
        diagnostics: source.diagnostics.clone(),
    }
}

fn diagnostics_with_uri(diagnostics: &[Diagnostic], uri: &str) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .cloned()
        .map(|mut diagnostic| {
            if diagnostic.uri.is_none() {
                diagnostic.uri = Some(uri.to_owned());
            }
            diagnostic
        })
        .collect()
}

fn fill_diagnostic_uri(diagnostics: &mut [Diagnostic], uri: &str) {
    for diagnostic in diagnostics {
        if diagnostic.uri.is_none() {
            diagnostic.uri = Some(uri.to_owned());
        }
    }
}

fn clear_template_artifact_diagnostics(artifact: &mut TemplateArtifact) {
    artifact.diagnostics.clear();
}

fn clear_template_entrypoint_diagnostics(entrypoints: &mut CemQlTemplateEntrypoints) {
    if let Some(artifact) = &mut entrypoints.implicit {
        clear_template_artifact_diagnostics(artifact);
    }
    for artifact in entrypoints.named.values_mut() {
        clear_template_artifact_diagnostics(artifact);
    }
}

fn select_entrypoint_artifact(
    fallback: &TemplateArtifact,
    entrypoints: &CemQlTemplateEntrypoints,
    selected: Option<&str>,
) -> TemplateArtifact {
    match selected {
        Some(name) => entrypoints
            .named
            .get(name)
            .cloned()
            .unwrap_or_else(|| fallback.clone()),
        None => entrypoints
            .implicit
            .clone()
            .unwrap_or_else(|| fallback.clone()),
    }
}

fn render_payload_template(
    payload: &CemQlCompiledTemplatePayload,
    data: &TemplateData,
) -> RenderPlan {
    let data = root_template_data_with_params(payload, data);
    let mut plan = render_compiled_template(&payload.artifact, &data);
    fill_diagnostic_uri(&mut plan.diagnostics, payload.template_uri.as_str());
    let nodes = expand_call_nodes(&plan.nodes, payload, None, &data, 0, &mut plan.diagnostics);
    RenderPlan {
        nodes,
        diagnostics: plan.diagnostics,
    }
}

fn root_template_data_with_params(
    payload: &CemQlCompiledTemplatePayload,
    data: &TemplateData,
) -> TemplateData {
    let mut data = data.clone();
    for (name, value) in &payload.params {
        bind_param_value(
            &mut data,
            payload.selected_entrypoint.as_deref(),
            name,
            value,
        );
    }
    apply_param_declarations(
        &mut data,
        &payload.param_declarations,
        payload.selected_entrypoint.as_deref(),
    );
    data
}

fn expand_call_nodes(
    nodes: &[RenderPlanNode],
    payload: &CemQlCompiledTemplatePayload,
    current_module: Option<&CemQlCompiledTemplateModulePayload>,
    data: &TemplateData,
    depth: u32,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<RenderPlanNode> {
    nodes
        .iter()
        .flat_map(|node| expand_call_node(node, payload, current_module, data, depth, diagnostics))
        .collect()
}

fn expand_call_node(
    node: &RenderPlanNode,
    payload: &CemQlCompiledTemplatePayload,
    current_module: Option<&CemQlCompiledTemplateModulePayload>,
    data: &TemplateData,
    depth: u32,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<RenderPlanNode> {
    let RenderPlanNode::Element {
        tag,
        attributes,
        children,
        source_map,
    } = node
    else {
        return vec![node.clone()];
    };

    if local_name(tag) == "call" {
        return render_call_node(
            attributes,
            payload,
            current_module,
            data,
            depth,
            diagnostics,
            source_map,
        );
    }

    vec![RenderPlanNode::Element {
        tag: tag.clone(),
        attributes: attributes.clone(),
        children: expand_call_nodes(children, payload, current_module, data, depth, diagnostics),
        source_map: source_map.clone(),
    }]
}

fn render_call_node(
    attributes: &[RenderPlanAttribute],
    payload: &CemQlCompiledTemplatePayload,
    current_module: Option<&CemQlCompiledTemplateModulePayload>,
    data: &TemplateData,
    depth: u32,
    diagnostics: &mut Vec<Diagnostic>,
    source_map: &cem_ml::source_map::SourceMapStack,
) -> Vec<RenderPlanNode> {
    let call_site_uri = current_template_uri(payload, current_module);
    if depth >= payload.max_recursion_depth {
        diagnostics.push(module_render_diagnostic(
            TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE,
            format!(
                "native template call recursion limit exceeded at depth {depth}; max depth is {}",
                payload.max_recursion_depth
            ),
            call_site_uri,
            source_map.clone(),
        ));
        return Vec::new();
    }

    let Some(template) = render_attr(attributes, "template") else {
        diagnostics.push(module_render_diagnostic(
            "cem.transform_template.call_unknown",
            "native template call is missing a `template` target",
            call_site_uri,
            source_map.clone(),
        ));
        return Vec::new();
    };
    let from = render_attr(attributes, "from");

    let (target, target_module) = match from.as_deref() {
        Some(alias) => match current_module {
            Some(current) => {
                let module = current
                    .imports
                    .get(alias)
                    .and_then(|uri| payload.modules.iter().find(|module| module.uri == *uri));
                (
                    module.and_then(|module| module.entrypoints.named.get(&template)),
                    module,
                )
            }
            None => {
                let module = payload
                    .modules
                    .iter()
                    .find(|module| module.parent_uri.is_none() && module.alias == alias);
                (
                    module.and_then(|module| module.entrypoints.named.get(&template)),
                    module,
                )
            }
        },
        None => match current_module {
            Some(module) => (module.entrypoints.named.get(&template), Some(module)),
            None => (payload.entrypoints.named.get(&template), None),
        },
    };

    let Some(target) = target else {
        diagnostics.push(module_render_diagnostic(
            "cem.transform_template.call_unknown",
            format!("native template call target `{template}` was not compiled"),
            call_site_uri,
            source_map.clone(),
        ));
        return Vec::new();
    };

    let mut call_data = call_data_with_bindings(data, attributes);
    apply_param_declarations(
        &mut call_data,
        param_declarations_for_module(payload, target_module),
        Some(template.as_str()),
    );
    let mut plan = render_compiled_template(target, &call_data);
    fill_diagnostic_uri(
        &mut plan.diagnostics,
        current_template_uri(payload, target_module),
    );
    diagnostics.append(&mut plan.diagnostics);
    expand_call_nodes(
        &plan.nodes,
        payload,
        target_module,
        &call_data,
        depth + 1,
        diagnostics,
    )
}

fn current_template_uri<'a>(
    payload: &'a CemQlCompiledTemplatePayload,
    module: Option<&'a CemQlCompiledTemplateModulePayload>,
) -> &'a str {
    module
        .map(|module| module.uri.as_str())
        .unwrap_or(payload.template_uri.as_str())
}

fn param_declarations_for_module<'a>(
    payload: &'a CemQlCompiledTemplatePayload,
    module: Option<&'a CemQlCompiledTemplateModulePayload>,
) -> &'a [TransformTemplateModuleParamDeclaration] {
    module
        .map(|module| module.param_declarations.as_slice())
        .unwrap_or(payload.param_declarations.as_slice())
}

fn bind_param_value(
    data: &mut TemplateData,
    selected_entrypoint: Option<&str>,
    name: &str,
    value: &Value,
) {
    let stream = value_to_stream(value);
    data.bindings.insert(name.to_owned(), stream.clone());
    if let Some((qualified, local)) = entrypoint_param_aliases(name, selected_entrypoint) {
        data.bindings
            .entry(qualified)
            .or_insert_with(|| stream.clone());
        data.bindings.entry(local).or_insert(stream);
    }
}

fn apply_param_declarations(
    data: &mut TemplateData,
    declarations: &[TransformTemplateModuleParamDeclaration],
    selected_entrypoint: Option<&str>,
) {
    for declaration in declarations {
        if let Some((qualified, local)) =
            entrypoint_param_aliases(&declaration.name, selected_entrypoint)
        {
            normalize_param_aliases(data, qualified.as_str(), local.as_str());
            if !data.bindings.contains_key(&qualified) && !data.bindings.contains_key(&local) {
                if let Some(default_value) = &declaration.default_value {
                    let stream = value_to_stream(default_value);
                    data.bindings.insert(qualified, stream.clone());
                    data.bindings.insert(local, stream);
                }
            }
            continue;
        }

        if declaration.name.contains('.') {
            continue;
        }
        if !data.bindings.contains_key(&declaration.name) {
            if let Some(default_value) = &declaration.default_value {
                data.bindings
                    .insert(declaration.name.clone(), value_to_stream(default_value));
            }
        }
    }
}

fn normalize_param_aliases(data: &mut TemplateData, qualified: &str, local: &str) {
    match (
        data.bindings.get(qualified).cloned(),
        data.bindings.get(local).cloned(),
    ) {
        (Some(stream), None) => {
            data.bindings.insert(local.to_owned(), stream);
        }
        (None, Some(stream)) => {
            data.bindings.insert(qualified.to_owned(), stream);
        }
        _ => {}
    }
}

fn entrypoint_param_aliases(
    name: &str,
    selected_entrypoint: Option<&str>,
) -> Option<(String, String)> {
    let entrypoint = selected_entrypoint?;
    if let Some(local) = name
        .strip_prefix(entrypoint)
        .and_then(|remaining| remaining.strip_prefix('.'))
        .filter(|local| !local.trim().is_empty())
    {
        return Some((name.to_owned(), local.to_owned()));
    }

    if !name.contains('.') && !name.trim().is_empty() {
        return Some((format!("{entrypoint}.{name}"), name.to_owned()));
    }
    None
}

fn render_attr(attributes: &[RenderPlanAttribute], name: &str) -> Option<String> {
    attributes
        .iter()
        .find(|attribute| attribute.name == name)
        .map(|attribute| attribute.value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn call_data_with_bindings(
    data: &TemplateData,
    attributes: &[RenderPlanAttribute],
) -> TemplateData {
    let mut data = data.clone();
    for attribute in attributes {
        let Some(name) = attribute.name.strip_prefix("with:") else {
            continue;
        };
        if name.trim().is_empty() {
            continue;
        }
        data.bindings
            .insert(name.to_owned(), attribute.value_stream.clone());
    }
    data
}

fn literal_attribute(
    attributes: &[cem_ql::render::TemplateAttribute],
    name: &str,
) -> Option<String> {
    attributes
        .iter()
        .find(|attribute| attribute.name == name)
        .and_then(|attribute| match &attribute.value {
            Some(TemplateAttributeValue::Literal(value)) => Some(value.clone()),
            _ => None,
        })
        .filter(|value| !value.trim().is_empty())
}

fn local_name(name: &str) -> &str {
    name.rsplit_once(':')
        .map(|(_, local)| local)
        .unwrap_or(name)
}

fn module_render_diagnostic(
    code: &str,
    message: impl Into<String>,
    uri: &str,
    source_map: cem_ml::source_map::SourceMapStack,
) -> Diagnostic {
    Diagnostic {
        uri: Some(uri.to_owned()),
        code: code.to_owned(),
        severity: Severity::Fatal,
        message: message.into(),
        source_map: Some(source_map),
        ..Diagnostic::default()
    }
}

fn template_data_from_artifacts(
    primary: &TransformTemplateDataArtifact,
    secondary: &BTreeMap<String, TransformTemplateDataArtifact>,
) -> TemplateData {
    let mut data = TemplateData::default().with_binding("input", value_to_stream(&primary.value));
    match &primary.value {
        Value::Object(fields) => {
            for (name, value) in fields {
                data = data.with_binding(name.clone(), value_to_stream(value));
            }
        }
        _ => {
            data = data.with_binding("value", value_to_stream(&primary.value));
        }
    }
    for (name, artifact) in secondary {
        data = data.with_binding(name.clone(), value_to_stream(&artifact.value));
    }
    data
}

fn value_to_stream(value: &Value) -> ItemStream {
    match value {
        Value::Array(items) => ItemStream::from_items(items.iter().map(value_to_item).collect()),
        _ => ItemStream::once(value_to_item(value)),
    }
}

fn value_to_item(value: &Value) -> Item {
    match value {
        Value::Null => Item::Atomic(AtomValue::Null),
        Value::Bool(value) => Item::Atomic(AtomValue::Boolean(*value)),
        Value::Number(value) => number_to_item(value),
        Value::String(value) => Item::Atomic(AtomValue::String(value.clone())),
        Value::Array(items) => Item::Array(items.iter().map(value_to_item).collect()),
        Value::Object(fields) => Item::Record(
            fields
                .iter()
                .map(|(name, value)| (name.clone(), vec![value_to_item(value)]))
                .collect(),
        ),
    }
}

fn number_to_item(value: &Number) -> Item {
    if let Some(value) = value.as_i64() {
        Item::Atomic(AtomValue::Integer(value))
    } else if let Some(value) = value.as_u64().and_then(|value| i64::try_from(value).ok()) {
        Item::Atomic(AtomValue::Integer(value))
    } else if let Some(value) = value.as_f64() {
        Item::Atomic(AtomValue::Double(value))
    } else {
        Item::Atomic(AtomValue::Decimal(value.to_string()))
    }
}

pub fn json_object(fields: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Value {
    Value::Object(
        fields
            .into_iter()
            .map(|(name, value)| (name.into(), value))
            .collect::<Map<_, _>>(),
    )
}

pub fn unsupported_identity_error_message(identity: &FormatIdentity) -> String {
    format!(
        "{TRANSFORM_TEMPLATE_UNSUPPORTED_CODE}: CEM-QL template adapter does not support template identity {identity:?}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cem_ml::engine::CemMlEngine;
    use cem_ml::engine::{
        EngineInput, TemplateInput, TransformExecutionPolicy, TransformGraphExport,
        TransformGraphImport, TransformGraphRequest, TransformGraphStage, TransformRequest,
        TransformSchedulerScopeIds, TransformStageSchedulerScopeIds, TransformTemplateEntrypoint,
    };
    use cem_ml::real::RealCemMlEngine;
    use cem_ml::run_config::ScopeConfig;
    use cem_ml::transform_template::{
        TransformTemplateAdapterLookup, TransformTemplateModuleParamType,
        TransformTemplateModulePreflight, TransformTemplateResolvedModule,
    };

    #[test]
    fn adapter_compiles_and_renders_cem_native_template() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{span @class="{$datadom.attributes.tone}" | {$datadom.attributes.label}}"#
                .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: Some("data.json".to_owned()),
            identity: None,
            value: json_object([
                ("label", Value::String("Save".to_owned())),
                ("tone", Value::String("primary".to_owned())),
            ]),
        };
        let secondary_inputs = BTreeMap::new();
        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("template should render");

        assert_eq!(
            rendered.output.value,
            Value::String(r#"<span class="primary">Save</span>"#.to_owned())
        );
        assert_eq!(
            rendered
                .output
                .identity
                .and_then(|identity| identity.content_type),
            Some("text/html".to_owned())
        );
    }

    #[test]
    fn adapter_surfaces_compile_diagnostics_once() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{span | {$missing}}"#.to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();

        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("template should compile with diagnostics");
        assert_eq!(compiled.artifact.opaque["diagnostics"], 1);
        assert!(compiled
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.ql.render.compile_failed"));
        assert!(compiled.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cem.ql.render.compile_failed"
                && diagnostic.uri.as_deref() == Some("template.cem")
        }));

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled.artifact,
                primary_input: &TransformTemplateDataArtifact {
                    artifact_id: "data".to_owned(),
                    uri: None,
                    identity: None,
                    value: Value::Null,
                },
                secondary_inputs: &BTreeMap::new(),
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("template should render without repeating compile diagnostics");

        assert!(!rendered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.ql.render.compile_failed"));
    }

    #[test]
    fn adapter_attributes_imported_module_compile_diagnostics_to_module_uri() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {body | {div | {call @from="ui" @template="card"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();

        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:ui".to_owned(),
                        bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="card" @visibility="public" | {body | {span | {$missing}}}}
}"#
                        .to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("template should compile with imported module diagnostics");

        assert_eq!(compiled.artifact.opaque["moduleDiagnostics"], 1);
        assert!(compiled.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cem.ql.render.compile_failed"
                && diagnostic.uri.as_deref() == Some("templates/ui.cem")
        }));
        assert!(!compiled.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cem.ql.render.compile_failed"
                && diagnostic.uri.as_deref() == Some("template.cem")
        }));
    }

    #[test]
    fn adapter_binds_primary_input_and_secondary_labels() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{span | {$input.label}:{$meta.count}}"#.to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned(), "meta".to_owned()];
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: Some("data.json".to_owned()),
            identity: None,
            value: json_object([("label", Value::String("orders".to_owned()))]),
        };
        let secondary_inputs = BTreeMap::from([(
            "meta".to_owned(),
            TransformTemplateDataArtifact {
                artifact_id: "meta".to_owned(),
                uri: Some("meta.json".to_owned()),
                identity: None,
                value: json_object([("count", Value::Number(3.into()))]),
            },
        )]);

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<span>orders:3</span>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_compiles_preflighted_modules_into_payload() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{article | Main}"#.to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();

        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:test".to_owned(),
                        bytes: br#"{span | Imported}"#.to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("template and import should compile")
            .artifact;

        assert_eq!(compiled.opaque["moduleImports"], 1);
        assert_eq!(compiled.opaque["modules"][0]["alias"], "ui");
        assert_eq!(
            compiled.opaque["modules"][0]["contentHash"],
            "cem-bin/1+blake3:test"
        );
        let payload = compiled
            .native_payload::<CemQlCompiledTemplatePayload>()
            .expect("CEM-QL payload");
        assert_eq!(payload.modules.len(), 1);
        assert_eq!(payload.modules[0].alias, "ui");
        assert!(!payload.modules[0].artifact.nodes.is_empty());
    }

    #[test]
    fn adapter_dispatches_same_module_calls_during_render() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="helper" | {body | {span | Help}}}
  {body | {div | {call @template="helper"}}}
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Help</span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_passes_with_bindings_to_same_module_calls() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="helper" |
    {param @name="title"}
    {body | {span | {$title}}}
  }
  {body | {div | {call @template="helper" @with:title="Hello"}}}
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    params: vec![cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                        name: "helper.title".to_owned(),
                        value_type: TransformTemplateModuleParamType::Any,
                        nullable: false,
                        default_value: None,
                        required: false,
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                    }],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Hello</span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_preserves_typed_with_bindings_for_same_module_calls() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="helper" |
    {param @name="enabled"}
    {body | {cem:if @test="enabled" | {span | Enabled}}{span | Done}}
  }
  {body | {div | {call @template="helper" @with:enabled="{enabled}"}}}
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = vec!["enabled".to_owned()];
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    params: vec![cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                        name: "helper.enabled".to_owned(),
                        value_type: TransformTemplateModuleParamType::Any,
                        nullable: false,
                        default_value: None,
                        required: false,
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                    }],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: json_object([("enabled", Value::Bool(false))]),
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Done</span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_preserves_structured_with_bindings_for_same_module_calls() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="helper" |
    {param @name="settings"}
    {body | {cem:if @test="settings.enabled" | {span | Enabled}}}
  }
  {body | {div | {call @template="helper" @with:settings="{sourceSettings}"}}}
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = vec!["sourceSettings".to_owned()];
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    params: vec![cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                        name: "helper.settings".to_owned(),
                        value_type: TransformTemplateModuleParamType::Any,
                        nullable: false,
                        default_value: None,
                        required: false,
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                    }],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: json_object([(
                "sourceSettings",
                json_object([("enabled", Value::Bool(true))]),
            )]),
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Enabled</span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_applies_named_entrypoint_param_defaults() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {param @name="locale" @default="en-US"}
  {template @name="card" @visibility="public" |
    {param @name="title" @default="Untitled"}
    {body | {p | {$locale}:{$title}}}
  }
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::named("card"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    entrypoints: vec![cem_ml::transform_template::TransformTemplateModuleEntrypointDeclaration {
                        name: "card".to_owned(),
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Public,
                    }],
                    params: vec![
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "locale".to_owned(),
                            value_type: TransformTemplateModuleParamType::Any,
                            nullable: false,
                            default_value: Some(Value::String("en-US".to_owned())),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "card.title".to_owned(),
                            value_type: TransformTemplateModuleParamType::Any,
                            nullable: false,
                            default_value: Some(Value::String("Untitled".to_owned())),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                    ],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<p>en-US:Untitled</p>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_applies_typed_param_defaults() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="card" @visibility="public" |
    {param @name="enabled" @type="boolean" @default="true"}
    {param @name="count" @type="integer" @default="3"}
    {body | {p | {cem:if @test="enabled" | Enabled:}{$count}}}
  }
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::named("card"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    entrypoints: vec![cem_ml::transform_template::TransformTemplateModuleEntrypointDeclaration {
                        name: "card".to_owned(),
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Public,
                    }],
                    params: vec![
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "card.enabled".to_owned(),
                            value_type: TransformTemplateModuleParamType::Boolean,
                            nullable: false,
                            default_value: Some(Value::Bool(true)),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "card.count".to_owned(),
                            value_type: TransformTemplateModuleParamType::Integer,
                            nullable: false,
                            default_value: Some(Value::Number(Number::from(3))),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                    ],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<p>Enabled:3</p>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_applies_nullable_typed_param_defaults() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="card" @visibility="public" |
    {param @name="subtitle" @type="string" @nullable="true" @default="null"}
    {body | {p | A{$subtitle}B}}
  }
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::named("card"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    entrypoints: vec![cem_ml::transform_template::TransformTemplateModuleEntrypointDeclaration {
                        name: "card".to_owned(),
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Public,
                    }],
                    params: vec![cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                        name: "card.subtitle".to_owned(),
                        value_type: TransformTemplateModuleParamType::String,
                        nullable: true,
                        default_value: Some(Value::Null),
                        required: false,
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                    }],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(rendered.output.value, Value::String("<p>AB</p>".to_owned()));
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_binds_caller_params_for_named_entrypoints() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {param @name="locale" @default="en-US"}
  {template @name="card" @visibility="public" |
    {param @name="title" @default="Untitled"}
    {body | {p | {$locale}:{$title}}}
  }
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::from([
            ("locale".to_owned(), Value::String("fr-FR".to_owned())),
            ("title".to_owned(), Value::String("Intro".to_owned())),
        ]);
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::named("card"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    entrypoints: vec![cem_ml::transform_template::TransformTemplateModuleEntrypointDeclaration {
                        name: "card".to_owned(),
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Public,
                    }],
                    params: vec![
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "locale".to_owned(),
                            value_type: TransformTemplateModuleParamType::Any,
                            nullable: false,
                            default_value: Some(Value::String("en-US".to_owned())),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "card.title".to_owned(),
                            value_type: TransformTemplateModuleParamType::Any,
                            nullable: false,
                            default_value: Some(Value::String("Untitled".to_owned())),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                    ],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<p>fr-FR:Intro</p>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_binds_qualified_caller_params_for_named_entrypoints() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {param @name="locale" @default="en-US"}
  {template @name="card" @visibility="public" |
    {param @name="title" @default="Untitled"}
    {body | {p | {$locale}:{$title}}}
  }
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::from([
            ("locale".to_owned(), Value::String("fr-FR".to_owned())),
            ("card.title".to_owned(), Value::String("Intro".to_owned())),
        ]);
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::named("card"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    entrypoints: vec![cem_ml::transform_template::TransformTemplateModuleEntrypointDeclaration {
                        name: "card".to_owned(),
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Public,
                    }],
                    params: vec![
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "locale".to_owned(),
                            value_type: TransformTemplateModuleParamType::Any,
                            nullable: false,
                            default_value: Some(Value::String("en-US".to_owned())),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "card.title".to_owned(),
                            value_type: TransformTemplateModuleParamType::Any,
                            nullable: false,
                            default_value: Some(Value::String("Untitled".to_owned())),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                    ],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<p>fr-FR:Intro</p>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_treats_explicit_null_params_as_default_overrides() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {param @name="locale" @default="en-US"}
  {template @name="card" @visibility="public" |
    {param @name="title" @default="Untitled"}
    {body | {p | {$locale}:{$title}}}
  }
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::from([("title".to_owned(), Value::Null)]);
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::named("card"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    entrypoints: vec![cem_ml::transform_template::TransformTemplateModuleEntrypointDeclaration {
                        name: "card".to_owned(),
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Public,
                    }],
                    params: vec![
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "locale".to_owned(),
                            value_type: TransformTemplateModuleParamType::Any,
                            nullable: false,
                            default_value: Some(Value::String("en-US".to_owned())),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "card.title".to_owned(),
                            value_type: TransformTemplateModuleParamType::Any,
                            nullable: true,
                            default_value: Some(Value::String("Untitled".to_owned())),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                    ],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<p>en-US:</p>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_stops_recursive_same_module_calls_at_limit() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="loop" | {body | {span | Loop {call @template="loop"}}}}
  {body | {div | {call @template="loop"}}}
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    limits: cem_ml::transform_template::TransformTemplateModuleLimits {
                        max_import_depth: 32,
                        max_recursion_depth: 2,
                    },
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render with recursion diagnostic");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Loop <span>Loop </span></span></div>".to_owned())
        );
        assert!(rendered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE));
        assert!(rendered.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE
                && diagnostic.uri.as_deref() == Some("template.cem")
        }));
    }

    #[test]
    fn adapter_dispatches_imported_module_calls_during_render() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {body | {div | {call @from="ui" @template="icon"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:ui".to_owned(),
                        bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="icon" @visibility="public" | {body | {span | Icon}}}
}"#
                        .to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Icon</span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_dispatches_same_module_calls_inside_imported_modules() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="helper" | {body | {i | Root}}}
  {body | {div | {call @from="ui" @template="icon"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:ui".to_owned(),
                        bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="icon" @visibility="public" |
    {body | {span | Icon {call @template="helper"}}}
  }
  {template @name="helper" | {body | {i | Imported}}}
}"#
                        .to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Icon <i>Imported</i></span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_dispatches_nested_import_calls_inside_imported_modules() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {body | {div | {call @from="ui" @template="card"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![
                        TransformTemplateResolvedModule {
                            alias: "ui".to_owned(),
                            parent_uri: None,
                            uri: "templates/ui.cem".to_owned(),
                            identity: Some(identity.clone()),
                            content_hash: "cem-bin/1+blake3:ui".to_owned(),
                            bytes: br#"{@doc cem-ml 1}
{module |
  {import @as="icons" @src="icons.cem"}
  {template @name="card" @visibility="public" |
    {body | {section | Card {call @from="icons" @template="check"}}}
  }
}"#
                            .to_vec(),
                        },
                        TransformTemplateResolvedModule {
                            alias: "icons".to_owned(),
                            parent_uri: Some("templates/ui.cem".to_owned()),
                            uri: "templates/icons.cem".to_owned(),
                            identity: Some(identity),
                            content_hash: "cem-bin/1+blake3:icons".to_owned(),
                            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="check" @visibility="public" | {body | {span | Check}}}
}"#
                            .to_vec(),
                        },
                    ],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><section>Card <span>Check</span></section></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_bounds_recursive_calls_inside_imported_modules() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {body | {div | {call @from="ui" @template="loop"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    limits: cem_ml::transform_template::TransformTemplateModuleLimits {
                        max_import_depth: 32,
                        max_recursion_depth: 2,
                    },
                    ..Default::default()
                },
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:ui".to_owned(),
                        bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="loop" @visibility="public" |
    {body | {span | Loop {call @template="loop"}}}
  }
}"#
                        .to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render with recursion diagnostic");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Loop <span>Loop </span></span></div>".to_owned())
        );
        assert!(rendered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE));
        assert!(rendered.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE
                && diagnostic.uri.as_deref() == Some("templates/ui.cem")
        }));
        assert!(!rendered.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE
                && diagnostic.uri.as_deref() == Some("template.cem")
        }));
    }

    #[test]
    fn adapter_passes_with_bindings_to_imported_module_calls() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {body | {div | {call @from="ui" @template="icon" @with:title="Imported"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:ui".to_owned(),
                        bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="icon" @visibility="public" |
    {param @name="title"}
    {body | {span | {$title}}}
  }
}"#
                        .to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Imported</span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_rejects_non_utf8_preflighted_modules() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{article | Main}"#.to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();

        let error = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:test".to_owned(),
                        bytes: vec![0xff],
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect_err("invalid module bytes should fail compile");

        assert_eq!(
            error.diagnostic_origin(),
            cem_ml::engine::TransformDiagnosticOrigin::TemplateCompile
        );
        assert!(error.to_string().contains("templates/ui.cem"));
    }

    #[test]
    fn registration_makes_adapter_available_through_engine_context() {
        let context = engine_context_with_cem_ql_template_adapter();
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };

        match context.template_adapter_registry.select_adapter(&identity) {
            TransformTemplateAdapterLookup::Matched(adapter) => {
                assert_eq!(adapter.id(), CEM_QL_TEMPLATE_ADAPTER_ID)
            }
            other => panic!("expected built-in plus runtime adapter ambiguity, got {other:?}"),
        }
    }

    #[test]
    fn adapter_accepts_cem_native_template_schema_identity() {
        let context = engine_context_with_cem_ql_template_adapter();
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            default_namespace: Some(
                cem_ml::transform_template::CEM_NATIVE_TEMPLATE_NAMESPACE_URI.to_owned(),
            ),
            ..FormatIdentity::default()
        };

        match context.template_adapter_registry.select_adapter(&identity) {
            TransformTemplateAdapterLookup::Matched(adapter) => {
                assert_eq!(adapter.id(), CEM_QL_TEMPLATE_ADAPTER_ID)
            }
            other => panic!("expected schema identity adapter match, got {other:?}"),
        }
    }

    #[test]
    fn real_engine_transform_uses_registered_cem_ql_adapter() {
        let context = engine_context_with_cem_ql_template_adapter();
        let template_identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let request = TransformRequest {
            data: EngineInput {
                uri: "data.cem".to_owned(),
                bytes: b"{p @id=\"guide\"}".to_vec(),
                from_format: None,
                identity: Some(FormatIdentity {
                    content_type: Some("text/cem-ml".to_owned()),
                    ..FormatIdentity::default()
                }),
                root_scope: ScopeConfig::default(),
            },
            template: TemplateInput {
                uri: "template.cem".to_owned(),
                bytes: br#"{p | {$datadom.attributes.kind}}"#.to_vec(),
                identity: Some(template_identity),
                root_scope: ScopeConfig::default(),
            },
            template_kind: TransformTemplateKind::CemNative,
            template_entrypoint: TransformTemplateEntrypoint::implicit(),
            params: BTreeMap::new(),
            preserve_source_offsets: false,
            context,
            target: Some(FormatIdentity {
                content_type: Some("text/html".to_owned()),
                ..FormatIdentity::default()
            }),
            target_scope: ScopeConfig::default(),
            scheduler_scope_ids: TransformSchedulerScopeIds {
                data_load: 10,
                template_load: 11,
                execution: 12,
                output: 13,
            },
            execution_policy: TransformExecutionPolicy::default(),
        };

        let response = RealCemMlEngine::new()
            .transform(request)
            .expect("transform should run through registered adapter");

        assert_eq!(
            response.primary,
            Value::String("<p>document</p>".to_owned())
        );
        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert_eq!(response.scheduler_trace.event_count, 12);
        let scopes = response
            .scheduler_trace
            .events
            .iter()
            .map(|event| event.scope_id)
            .collect::<Vec<_>>();
        assert!(scopes.contains(&10));
        assert!(scopes.contains(&11));
        assert!(scopes.contains(&12));
        assert!(scopes.contains(&13));
    }

    #[test]
    fn real_engine_transform_binds_cem_native_template_params() {
        let context = engine_context_with_cem_ql_template_adapter();
        let request = TransformRequest {
            data: EngineInput {
                uri: "data.cem".to_owned(),
                bytes: b"{p}".to_vec(),
                from_format: None,
                identity: Some(FormatIdentity {
                    content_type: Some("text/cem-ml".to_owned()),
                    ..FormatIdentity::default()
                }),
                root_scope: ScopeConfig::default(),
            },
            template: TemplateInput {
                uri: "template.cem".to_owned(),
                bytes: br#"{@doc cem-ml 1}
{module |
  {param @name="locale" @default="en-US" @visibility="public"}
  {template @name="card" @visibility="public" |
    {param @name="title" @default="Untitled"}
    {body | {p | {$locale}:{$title}}}
  }
}"#
                .to_vec(),
                identity: Some(FormatIdentity {
                    schema: Some(
                        cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned(),
                    ),
                    ..FormatIdentity::default()
                }),
                root_scope: ScopeConfig::default(),
            },
            template_kind: TransformTemplateKind::CemNative,
            template_entrypoint: TransformTemplateEntrypoint::named("card"),
            params: BTreeMap::from([
                ("locale".to_owned(), Value::String("fr-FR".to_owned())),
                ("title".to_owned(), Value::String("Intro".to_owned())),
            ]),
            preserve_source_offsets: false,
            context,
            target: Some(FormatIdentity {
                content_type: Some("text/html".to_owned()),
                ..FormatIdentity::default()
            }),
            target_scope: ScopeConfig::default(),
            scheduler_scope_ids: TransformSchedulerScopeIds {
                data_load: 10,
                template_load: 11,
                execution: 12,
                output: 13,
            },
            execution_policy: TransformExecutionPolicy {
                runtime_phase: cem_ml::engine::TransformRuntimePhase::CemNativeModules,
                ..TransformExecutionPolicy::default()
            },
        };

        let response = RealCemMlEngine::new()
            .transform(request)
            .expect("transform should bind CEM-native params");

        assert_eq!(
            response.primary,
            Value::String("<p>fr-FR:Intro</p>".to_owned()),
            "{:?}",
            response.diagnostics
        );
        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
    }

    #[test]
    fn real_engine_transform_graph_executes_branched_cem_native_outputs() {
        let context = engine_context_with_cem_ql_template_adapter();
        let data_identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let template_identity = data_identity.clone();
        let request = TransformGraphRequest {
            imports: vec![TransformGraphImport {
                id: "book".to_owned(),
                input: EngineInput {
                    uri: "book.cem".to_owned(),
                    bytes: b"{p @id=\"guide\"}".to_vec(),
                    from_format: None,
                    identity: Some(data_identity),
                    root_scope: ScopeConfig::default(),
                },
                scheduler_scope_id: 20,
            }],
            joins: Vec::new(),
            stages: vec![
                TransformGraphStage {
                    id: "html".to_owned(),
                    template: TemplateInput {
                        uri: "html.cem".to_owned(),
                        bytes: br#"{article | {$datadom.attributes.kind}}"#.to_vec(),
                        identity: Some(template_identity.clone()),
                        root_scope: ScopeConfig::default(),
                    },
                    template_kind: TransformTemplateKind::CemNative,
                    template_entrypoint: TransformTemplateEntrypoint::implicit(),
                    params: BTreeMap::new(),
                    primary_input: "book".to_owned(),
                    secondary_inputs: BTreeMap::new(),
                    scheduler_scope_ids: TransformStageSchedulerScopeIds {
                        template_load: 21,
                        execution: 22,
                    },
                },
                TransformGraphStage {
                    id: "chart".to_owned(),
                    template: TemplateInput {
                        uri: "chart.cem".to_owned(),
                        bytes: br#"{svg | {$datadom.attributes.kind}}"#.to_vec(),
                        identity: Some(template_identity),
                        root_scope: ScopeConfig::default(),
                    },
                    template_kind: TransformTemplateKind::CemNative,
                    template_entrypoint: TransformTemplateEntrypoint::implicit(),
                    params: BTreeMap::new(),
                    primary_input: "book".to_owned(),
                    secondary_inputs: BTreeMap::new(),
                    scheduler_scope_ids: TransformStageSchedulerScopeIds {
                        template_load: 23,
                        execution: 24,
                    },
                },
            ],
            exports: vec![
                TransformGraphExport {
                    id: "main".to_owned(),
                    input: "html".to_owned(),
                    destination: Some("dist/book.html".to_owned()),
                    target: Some(FormatIdentity {
                        content_type: Some("text/html".to_owned()),
                        ..FormatIdentity::default()
                    }),
                    target_scope: ScopeConfig::default(),
                    scheduler_scope_id: 25,
                },
                TransformGraphExport {
                    id: "chart-svg".to_owned(),
                    input: "chart".to_owned(),
                    destination: Some("dist/book/chart.svg".to_owned()),
                    target: Some(FormatIdentity {
                        content_type: Some("image/svg+xml".to_owned()),
                        ..FormatIdentity::default()
                    }),
                    target_scope: ScopeConfig::default(),
                    scheduler_scope_id: 26,
                },
            ],
            edges: Vec::new(),
            preserve_source_offsets: false,
            context,
            execution_policy: TransformExecutionPolicy::default(),
        };

        let response = RealCemMlEngine::new()
            .transform_graph(request)
            .expect("transform graph should execute through registered adapter");

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert_eq!(response.artifacts.len(), 2);
        assert_eq!(
            response.artifacts[0].primary,
            Value::String("<article>document</article>".to_owned())
        );
        assert_eq!(
            response.artifacts[1].primary,
            Value::String("<svg>document</svg>".to_owned())
        );
        assert_eq!(
            response.artifacts[1]
                .identity
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            Some("image/svg+xml")
        );
        assert!(response
            .scheduler_trace
            .events
            .iter()
            .any(|event| event.scope_id == 26 && event.task == "chart-svg:export"));
    }

    #[test]
    fn real_engine_transform_graph_passes_secondary_inputs_to_stage() {
        let context = engine_context_with_cem_ql_template_adapter();
        let data_identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let template_identity = data_identity.clone();
        let request = TransformGraphRequest {
            imports: vec![TransformGraphImport {
                id: "book".to_owned(),
                input: EngineInput {
                    uri: "book.cem".to_owned(),
                    bytes: b"{p @id=\"guide\"}".to_vec(),
                    from_format: None,
                    identity: Some(data_identity),
                    root_scope: ScopeConfig::default(),
                },
                scheduler_scope_id: 30,
            }],
            joins: Vec::new(),
            stages: vec![
                TransformGraphStage {
                    id: "stats".to_owned(),
                    template: TemplateInput {
                        uri: "stats.cem".to_owned(),
                        bytes: br#"{span | {$datadom.attributes.kind}}"#.to_vec(),
                        identity: Some(template_identity.clone()),
                        root_scope: ScopeConfig::default(),
                    },
                    template_kind: TransformTemplateKind::CemNative,
                    template_entrypoint: TransformTemplateEntrypoint::implicit(),
                    params: BTreeMap::new(),
                    primary_input: "book".to_owned(),
                    secondary_inputs: BTreeMap::new(),
                    scheduler_scope_ids: TransformStageSchedulerScopeIds {
                        template_load: 31,
                        execution: 32,
                    },
                },
                TransformGraphStage {
                    id: "report".to_owned(),
                    template: TemplateInput {
                        uri: "report.cem".to_owned(),
                        bytes: br#"{section | {$input.kind}:{$stats}}"#.to_vec(),
                        identity: Some(template_identity),
                        root_scope: ScopeConfig::default(),
                    },
                    template_kind: TransformTemplateKind::CemNative,
                    template_entrypoint: TransformTemplateEntrypoint::implicit(),
                    params: BTreeMap::new(),
                    primary_input: "book".to_owned(),
                    secondary_inputs: BTreeMap::from([("stats".to_owned(), "stats".to_owned())]),
                    scheduler_scope_ids: TransformStageSchedulerScopeIds {
                        template_load: 33,
                        execution: 34,
                    },
                },
            ],
            exports: vec![TransformGraphExport {
                id: "main".to_owned(),
                input: "report".to_owned(),
                destination: None,
                target: Some(FormatIdentity {
                    content_type: Some("text/html".to_owned()),
                    ..FormatIdentity::default()
                }),
                target_scope: ScopeConfig::default(),
                scheduler_scope_id: 35,
            }],
            edges: Vec::new(),
            preserve_source_offsets: false,
            context,
            execution_policy: TransformExecutionPolicy::default(),
        };

        let response = RealCemMlEngine::new()
            .transform_graph(request)
            .expect("transform graph should execute secondary input join");

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert_eq!(response.artifacts.len(), 1);
        assert_eq!(
            response.artifacts[0].primary,
            Value::String(
                "<section>document:&lt;span&gt;document&lt;/span&gt;</section>".to_owned()
            )
        );
    }
}
