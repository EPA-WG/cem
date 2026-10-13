//! Native XHTML output correspondence for an immutable template acquisition snapshot.
//! Scalar execution supports a shared context or recorded lexical body contexts.
//! Runtime reads and arbitrary native calls remain outside these profiles.
use crate::{
    dependency_inspection::ResourceUseKind,
    html_template_dependencies::{
        prepare_html_template_dependencies_with_sources, HtmlTemplateDependencyLimits,
        HtmlTemplateDependencyPreparation,
    },
    render::{
        self, CompileTemplateOptions, TemplateArtifact, TemplateData, TemplateModuleClosure,
        TemplateModuleSource,
    },
    template_dependency_inspection::{inspect_preflighted_bodies, PreflightBodyInventory},
};
use cem_ml::{
    engine::EngineContext,
    html_base_host::{HtmlBaseDomSlot, HtmlBaseExecutionHost},
    html_execution_inventory::HtmlExecutionInventory,
    interpreter::TransformOutput,
    module_resolution::{CemModuleUrlFrame, CemModuleUrlResolutionCapability},
    operation_control::ROOT_EXECUTION_SCOPE_ID,
    parser::tree::RetainedCemTree,
    template_snapshot::{SnapshotTemplatePreflight, TemplateSourceSnapshot},
};
use std::{collections::BTreeMap, sync::Arc};

pub struct SnapshotTemplateExecution {
    preflight: Arc<SnapshotTemplatePreflight>,
    artifact: TemplateArtifact,
    bodies: Arc<PreflightBodyInventory>,
    options: CompileTemplateOptions,
    scalar_bindings: BTreeMap<String, String>,
    capability: CemModuleUrlResolutionCapability,
    fingerprint: String,
    body_capabilities: BTreeMap<String, CemModuleUrlResolutionCapability>,
    body_fingerprints: BTreeMap<String, String>,
    scoped: bool,
    output: TransformOutput,
    tree: Arc<RetainedCemTree>,
    limits: HtmlTemplateDependencyLimits,
}
impl SnapshotTemplateExecution {
    pub fn output(&self) -> &TransformOutput {
        &self.output
    }
    pub fn tree(&self) -> &Arc<RetainedCemTree> {
        &self.tree
    }
    pub fn artifact(&self) -> &TemplateArtifact {
        &self.artifact
    }
    pub fn preflight(&self) -> &SnapshotTemplatePreflight {
        &self.preflight
    }
    pub fn bodies(&self) -> &PreflightBodyInventory {
        &self.bodies
    }
    pub fn scalar_bindings(&self) -> &BTreeMap<String, String> {
        &self.scalar_bindings
    }
    pub fn validate_current(
        &self,
        snapshot: &Arc<TemplateSourceSnapshot>,
        context: &Arc<EngineContext>,
        capability: &CemModuleUrlResolutionCapability,
    ) -> Result<(), String> {
        self.preflight.validate_current(snapshot, context)?;
        if !self.capability.same_owner(capability) || fingerprint(capability)? != self.fingerprint {
            return Err("recorded execution resolver owner or policy changed".into());
        }
        for (uri, body) in &self.body_capabilities {
            if fingerprint(body)? != self.body_fingerprints[uri] {
                return Err(format!("recorded body resolver changed: {uri}"));
            }
        }
        Ok(())
    }
    /// The host must capture this exact native output owner; equal reimported bytes
    /// do not establish correspondence. Sources come from the retained preflight,
    /// never from another call to the acquisition path.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_dependencies(
        self: &Arc<Self>,
        host: &mut HtmlBaseExecutionHost,
        document: Arc<HtmlExecutionInventory>,
        snapshot: &Arc<TemplateSourceSnapshot>,
        context: &Arc<EngineContext>,
        capability: &CemModuleUrlResolutionCapability,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base_uri: &str,
    ) -> Result<ExecutedHtmlTemplatePreparation, String> {
        self.validate_current(snapshot, context, capability)?;
        if !Arc::ptr_eq(&document.document().tree, &self.tree) {
            return Err("HTML inventory is not this execution's native output owner".into());
        }
        let preparation = prepare_html_template_dependencies_with_sources(
            host,
            document,
            self.preflight.context().clone(),
            self.preflight.root().clone(),
            capability,
            &self.body_capabilities,
            &self.options,
            self.limits,
            &mut matches,
            base_uri,
            Some((
                self.preflight.inspection_owner().clone(),
                self.bodies.clone(),
            )),
        )?;
        let blockers = preparation
            .blockers()
            .iter()
            .filter(|b| {
                !(self.scoped
                    && matches!(
                        b.as_str(),
                        "template-module-map-context-required"
                            | "module-resolution-context-required"
                    ))
                    && !matches!(
                        b.as_str(),
                        "template-output-correspondence-required"
                            | "template-acquisition-freshness-required"
                            | "template-output-document-analysis-required"
                    )
            })
            .cloned()
            .collect();
        let result = ExecutedHtmlTemplatePreparation {
            preparation,
            execution: self.clone(),
            blockers,
        };
        result.validate(host, self, snapshot, context, capability, matches, base_uri)?;
        Ok(result)
    }
}
pub struct ExecutedHtmlTemplatePreparation {
    preparation: HtmlTemplateDependencyPreparation,
    execution: Arc<SnapshotTemplateExecution>,
    blockers: Vec<String>,
}
impl ExecutedHtmlTemplatePreparation {
    pub fn preparation(&self) -> &HtmlTemplateDependencyPreparation {
        &self.preparation
    }
    pub fn blockers(&self) -> &[String] {
        &self.blockers
    }
    #[allow(clippy::too_many_arguments)]
    pub fn validate(
        &self,
        host: &mut HtmlBaseExecutionHost,
        execution: &Arc<SnapshotTemplateExecution>,
        snapshot: &Arc<TemplateSourceSnapshot>,
        context: &Arc<EngineContext>,
        capability: &CemModuleUrlResolutionCapability,
        matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base_uri: &str,
    ) -> Result<(), String> {
        if !Arc::ptr_eq(execution, &self.execution) {
            return Err("native execution owner changed".into());
        }
        execution.validate_current(snapshot, context, capability)?;
        self.preparation.validate(
            host,
            context,
            execution.preflight.root(),
            capability,
            &execution.body_capabilities,
            matches,
            base_uri,
        )?;
        execution.validate_current(snapshot, context, capability)
    }
}

/// This performs a real render. It is separate from side-effect-free inventory
/// inspection and admits only expressions without runtime reads/unknown calls.
/// The named output boundary is XHTML serialized by the native XML writer and
/// reimported by CEM-ML. Original render spans and compiled source owners are retained
/// separately; output XML coordinates are not template-source rewrite locations.
pub fn execute_snapshot_xhtml(
    preflight: Arc<SnapshotTemplatePreflight>,
    capability: &CemModuleUrlResolutionCapability,
    scalar_bindings: &BTreeMap<String, String>,
    output_uri: &str,
    limits: HtmlTemplateDependencyLimits,
) -> Result<Arc<SnapshotTemplateExecution>, String> {
    execute_snapshot_xhtml_internal(
        preflight,
        capability,
        scalar_bindings,
        output_uri,
        limits,
        false,
    )
}

/// Derive a native context for each lexical source body from its immutable import
/// ancestry. The body URI is the active base and its inline map's base. Ancestor
/// mapping and scheme rules are retained. Custom resolvers must support derivation.
pub fn execute_snapshot_xhtml_scoped(
    preflight: Arc<SnapshotTemplatePreflight>,
    capability: &CemModuleUrlResolutionCapability,
    scalar_bindings: &BTreeMap<String, String>,
    output_uri: &str,
    limits: HtmlTemplateDependencyLimits,
) -> Result<Arc<SnapshotTemplateExecution>, String> {
    execute_snapshot_xhtml_internal(
        preflight,
        capability,
        scalar_bindings,
        output_uri,
        limits,
        true,
    )
}

fn execute_snapshot_xhtml_internal(
    preflight: Arc<SnapshotTemplatePreflight>,
    capability: &CemModuleUrlResolutionCapability,
    scalar_bindings: &BTreeMap<String, String>,
    output_uri: &str,
    limits: HtmlTemplateDependencyLimits,
    scoped: bool,
) -> Result<Arc<SnapshotTemplateExecution>, String> {
    preflight.validate_current(preflight.snapshot(), preflight.context())?;
    let host_fingerprint = fingerprint(capability)?;
    let output_url =
        url::Url::parse(output_uri).map_err(|_| "absolute XHTML output URI required")?;
    if !matches!(output_url.scheme(), "http" | "https") || output_url.fragment().is_some() {
        return Err("HTTP(S) XHTML output URI without a fragment required".into());
    }
    if limits.max_bodies == 0 || limits.per_body_budget == 0 {
        return Err("nonzero execution inspection budgets required".into());
    }
    if scalar_bindings.len() > limits.per_body_budget
        || scalar_bindings.iter().fold(0usize, |n, (key, value)| {
            n.saturating_add(key.len()).saturating_add(value.len())
        }) > 16 * 1024 * 1024
    {
        return Err("scalar binding count or byte budget exceeded".into());
    }
    let graph = preflight
        .inspection()
        .preflight
        .as_ref()
        .ok_or("native preflight required")?;
    let mut depths = BTreeMap::from([(preflight.root().uri.clone(), 0u32)]);
    for module in &graph.resolved_imports {
        let parent = module
            .parent_uri
            .as_deref()
            .unwrap_or(&preflight.root().uri);
        let depth = depths
            .get(parent)
            .ok_or("preflight parent context missing")?
            .saturating_add(1);
        if depth > limits.max_import_depth {
            return Err("captured preflight exceeds the execution import-depth limit".into());
        }
        depths.insert(module.uri.clone(), depth);
    }
    let options = CompileTemplateOptions {
        host_bindings: scalar_bindings.keys().cloned().collect(),
        ..Default::default()
    };
    let bodies = Arc::new(inspect_preflighted_bodies(
        preflight.root(),
        preflight.inspection(),
        &options,
        limits.max_bodies,
        limits.per_body_budget,
    ));
    if !bodies.blockers.is_empty() {
        return Err(format!(
            "incomplete recorded body inventory: {:?}",
            bodies.blockers
        ));
    }
    for body in &bodies.bodies {
        let mut css_budget = limits.per_body_budget;
        for sheet in &body.stylesheets {
            crate::html_template_dependencies::check_stylesheet_budget(
                sheet.css_tree(),
                &mut css_budget,
            )?;
        }
        if body
            .blockers
            .iter()
            .any(|b| b == "unapplied-template-module-map")
        {
            return Err(
                "unapplied authored body module maps are outside the execution profile".into(),
            );
        }
        if !scoped && body.module_map.is_some() {
            return Err(
                "authored body module maps require recorded per-body execution contexts".into(),
            );
        }
        if body
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
            || body
                .blockers
                .iter()
                .any(|b| b == "template-inspection-budget-exceeded")
        {
            return Err("complete compiled bodies required for recorded execution".into());
        }
        for expression in &body.expressions {
            if expression.query().is_none()
                || expression
                    .inspection
                    .blockers
                    .iter()
                    .any(|b| b != "module-resolution-context-required")
                || expression
                    .inspection
                    .uses
                    .iter()
                    .any(|u| u.kind != ResourceUseKind::ModuleUrl || u.pipeline_step.is_some())
            {
                return Err("runtime reads, computed resolution and unrecorded calls are outside the XHTML execution profile".into());
            }
        }
    }
    if !preflight.inspection().xpath_resource_uses.is_empty() {
        return Err("XPath execution requires its own runtime acquisition record".into());
    }
    let body_capabilities = if scoped {
        derive_body_contexts(&preflight, &bodies, capability, limits.per_body_budget)?
    } else {
        bodies
            .bodies
            .iter()
            .map(|b| (b.uri.clone(), capability.clone()))
            .collect()
    };
    let body_fingerprints = body_capabilities
        .iter()
        .map(|(uri, cap)| Ok((uri.clone(), fingerprint(cap)?)))
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let root = preflight.root();
    let graph = preflight
        .inspection()
        .preflight
        .as_ref()
        .ok_or("native preflight required")?;
    let closure = TemplateModuleClosure {
        root_uri: root.uri.clone(),
        root_content_hash: preflight.inspection().root_content_hash.clone(),
        resolver_policy_stamp: preflight.context().resolver_policy.cache_stamp(),
        entrypoint: "body".into(),
        parameter_contract: options.host_bindings.clone(),
        cem_ml_version: cem_ml::VERSION.into(),
        cem_ql_version: crate::VERSION.into(),
        modules: graph
            .resolved_imports
            .iter()
            .map(|m| {
                Ok(TemplateModuleSource {
                    alias: m.alias.clone(),
                    parent_uri: m.parent_uri.clone(),
                    uri: m.uri.clone(),
                    content_hash: m.content_hash.clone(),
                    source: std::str::from_utf8(&m.bytes)
                        .map_err(|_| "UTF-8 template sources required")?
                        .into(),
                })
            })
            .collect::<Result<_, String>>()?,
    };
    let artifact = render::compile_template_module_closure_with_resolutions(
        std::str::from_utf8(&root.bytes).map_err(|_| "UTF-8 root template required")?,
        &closure,
        &options,
        &body_capabilities,
    );
    if artifact
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation())
    {
        return Err(format!(
            "recorded template compilation failed: {:?}",
            artifact.diagnostics
        ));
    }
    let mut data = TemplateData::default();
    for (name, value) in scalar_bindings {
        data.bindings.insert(
            name.clone(),
            crate::eval::ItemStream::once(crate::eval::Item::Atomic(
                crate::eval::AtomValue::String(value.clone()),
            )),
        );
    }
    let control = &preflight.context().operation_control;
    let plan = render::render_compiled_template_with_resolution(
        &artifact,
        &data,
        capability,
        control,
        ROOT_EXECUTION_SCOPE_ID,
    );
    if !plan.host_attribute_updates.is_empty()
        || plan
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
    {
        return Err(format!(
            "recorded XHTML render failed or requires host attribute updates: {:?}",
            plan.diagnostics
        ));
    }
    let output = render::render_plan_to_xml_with_control(&plan, control, ROOT_EXECUTION_SCOPE_ID)
        .map_err(|e| e.to_string())?;
    let tree = cem_ml::import::import_data_bytes(
        output.rendered.as_bytes(),
        "application/xml",
        "cem",
        output_uri,
    )?;
    let document = tree.node(0).ok_or("XHTML document required")?;
    let elements: Vec<_> = document
        .children
        .iter()
        .filter_map(|id| tree.node(*id))
        .filter(|node| node.kind == cem_ml::parser::tree::CemTreeNodeKind::Element)
        .collect();
    if elements.len() != 1
        || !elements[0].name.as_ref().is_some_and(|name| {
            name.local_name == "html" && name.namespace_uri == "http://www.w3.org/1999/xhtml"
        })
    {
        return Err("recorded XHTML requires one HTML-namespace html document element".into());
    }
    if tree
        .ast_owner()
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation())
    {
        return Err("XHTML output import diagnostics".into());
    }
    let result = Arc::new(SnapshotTemplateExecution {
        preflight,
        artifact,
        bodies,
        options,
        scalar_bindings: scalar_bindings.clone(),
        capability: capability.clone(),
        fingerprint: host_fingerprint,
        body_capabilities,
        body_fingerprints,
        scoped,
        output,
        tree,
        limits,
    });
    result.validate_current(
        result.preflight.snapshot(),
        result.preflight.context(),
        capability,
    )?;
    Ok(result)
}
fn fingerprint(capability: &CemModuleUrlResolutionCapability) -> Result<String, String> {
    capability
        .cache_identity()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "recorded execution requires a complete resolver fingerprint".into())
}

fn derive_body_contexts(
    preflight: &SnapshotTemplatePreflight,
    bodies: &PreflightBodyInventory,
    host: &CemModuleUrlResolutionCapability,
    budget: usize,
) -> Result<BTreeMap<String, CemModuleUrlResolutionCapability>, String> {
    let inventory: BTreeMap<_, _> = bodies
        .bodies
        .iter()
        .map(|body| (body.uri.as_str(), body))
        .collect();
    let derive = |uri: &str, parent: &CemModuleUrlResolutionCapability| {
        let body = inventory.get(uri).ok_or("missing body inventory")?;
        let mut frame = CemModuleUrlFrame::new(format!("template:{uri}"), uri);
        if let Some(map) = &body.module_map {
            let size = map.scopes.iter().fold(
                map.specifiers
                    .imports
                    .len()
                    .saturating_add(map.specifiers.resources.len()),
                |n, scope| {
                    n.saturating_add(1)
                        .saturating_add(scope.specifiers.imports.len())
                        .saturating_add(scope.specifiers.resources.len())
                },
            );
            if size > budget {
                return Err("body module-map budget exceeded".into());
            }
            frame.specifiers = map.specifiers.clone();
            frame.scopes = map.scopes.clone();
        }
        parent.derive_child_context(frame)
    };
    let root = &preflight.root().uri;
    let mut contexts = BTreeMap::from([(root.clone(), derive(root, host)?)]);
    let mut parents = BTreeMap::new();
    for module in &preflight
        .inspection()
        .preflight
        .as_ref()
        .ok_or("native preflight required")?
        .resolved_imports
    {
        let parent = module.parent_uri.as_deref().unwrap_or(root);
        if let Some(previous) = parents.insert(module.uri.clone(), parent.to_owned()) {
            if previous != parent {
                return Err(
                    "ambiguous body import ancestry requires occurrence-specific compilation"
                        .into(),
                );
            }
            continue;
        }
        let context = derive(
            &module.uri,
            contexts.get(parent).ok_or("missing parent body context")?,
        )?;
        contexts.insert(module.uri.clone(), context);
    }
    Ok(contexts)
}
