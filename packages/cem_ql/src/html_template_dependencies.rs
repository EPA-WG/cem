//! Retained native candidate analysis across template preflight and a live HTML host.
//! The two source owners are distinct. This is not render/output correspondence,
//! acquisition history, complete runtime dependency closure, or edit authority.
use crate::{
    render::CompileTemplateOptions,
    template_dependency_inspection::{
        inspect_preflighted_bodies, inspect_query_module_resolutions, PreflightBodyInventory,
        QueryModuleResolutionInventory,
    },
};
use cem_ml::{
    css_resources::CssResourcePlan,
    engine::{EngineContext, TemplateInput},
    html_base_host::{HtmlBaseDomSlot, HtmlBaseExecutionHost},
    html_execution_inventory::HtmlExecutionInventory,
    module_resolution::CemModuleUrlResolutionCapability,
    real::{inspect_native_template_dependencies, NativeTemplateDependencyInspection},
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy)]
pub struct HtmlTemplateDependencyLimits {
    pub max_import_depth: u32,
    pub max_bodies: usize,
    pub per_body_budget: usize,
}
impl Default for HtmlTemplateDependencyLimits {
    fn default() -> Self {
        Self {
            max_import_depth: 8,
            max_bodies: 64,
            per_body_budget: 10_000,
        }
    }
}

/// Indices correspond to the retained body's expressions and stylesheets.
pub struct TemplateBodyResolutions {
    pub expressions: Vec<QueryModuleResolutionInventory>,
    pub stylesheets: Vec<CssResourcePlan>,
}
struct BodyAuthority {
    capability: CemModuleUrlResolutionCapability,
    fingerprint: String,
}

/// Construction runs native preflight itself; public diagnostic DTOs cannot mint
/// this preparation. Inspection access is immutable and validation never reacquires
/// resources. Equal source bytes, URLs or resolver labels do not replace owners.
pub struct HtmlTemplateDependencyPreparation {
    document: Arc<HtmlExecutionInventory>,
    context: Arc<EngineContext>,
    root: Arc<TemplateInput>,
    authorities: BTreeMap<String, BodyAuthority>,
    preflight: Arc<NativeTemplateDependencyInspection>,
    bodies: Arc<PreflightBodyInventory>,
    resolutions: Vec<TemplateBodyResolutions>,
    blockers: Vec<String>,
}
impl HtmlTemplateDependencyPreparation {
    pub fn document(&self) -> &HtmlExecutionInventory {
        &self.document
    }
    pub fn preflight(&self) -> &NativeTemplateDependencyInspection {
        &self.preflight
    }
    pub fn bodies(&self) -> &PreflightBodyInventory {
        &self.bodies
    }
    pub fn resolutions(&self) -> &[TemplateBodyResolutions] {
        &self.resolutions
    }
    pub fn blockers(&self) -> &[String] {
        &self.blockers
    }

    /// Required at consumption. The host must invalidate before changing execution
    /// state. Resolver fingerprints must cover policy, maps and context inputs.
    /// Engine identity binds the retained configuration, not live loader freshness.
    #[allow(clippy::too_many_arguments)]
    pub fn validate(
        &self,
        host: &mut HtmlBaseExecutionHost,
        context: &Arc<EngineContext>,
        root: &Arc<TemplateInput>,
        document_capability: &CemModuleUrlResolutionCapability,
        body_capabilities: &BTreeMap<String, CemModuleUrlResolutionCapability>,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base_uri: &str,
    ) -> Result<(), String> {
        host.validate_dependencies(&self.document, document_capability, &mut matches, base_uri)?;
        if !Arc::ptr_eq(context, &self.context) || !Arc::ptr_eq(root, &self.root) {
            return Err("template preparation source or engine owner changed".into());
        }
        validate_authorities(&self.authorities, body_capabilities)?;
        host.validate_dependencies(&self.document, document_capability, matches, base_uri)?;
        validate_authorities(&self.authorities, body_capabilities)
    }
}

/// Supply the execution capability for every final preflight body URI, including
/// the root. Capability selection is an explicit trusted-host input, never inferred
/// from source coordinates or the HTML base. Inline template CSS uses its body's
/// final URI; expressions use that capability's context and authored referrer.
/// Effective template module-map installation remains an explicit blocker.
///
/// Native preflight may acquire declared imports through the supplied engine's
/// resolver policy. It does not evaluate templates or runtime reads. Its depth and
/// engine operation controls apply to acquisition; max_bodies bounds analysis after
/// preflight, not the number of reads performed by preflight. Hosts needing immutable
/// snapshot-only acquisition use `SnapshotTemplateExecution::prepare_dependencies`,
/// which reuses a sealed snapshot preflight without this acquisition path.
#[allow(clippy::too_many_arguments)]
pub fn prepare_html_template_dependencies(
    host: &mut HtmlBaseExecutionHost,
    document: Arc<HtmlExecutionInventory>,
    context: Arc<EngineContext>,
    root: Arc<TemplateInput>,
    document_capability: &CemModuleUrlResolutionCapability,
    body_capabilities: &BTreeMap<String, CemModuleUrlResolutionCapability>,
    options: &CompileTemplateOptions,
    limits: HtmlTemplateDependencyLimits,
    matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
    base_uri: &str,
) -> Result<HtmlTemplateDependencyPreparation, String> {
    prepare_html_template_dependencies_with_sources(
        host,
        document,
        context,
        root,
        document_capability,
        body_capabilities,
        options,
        limits,
        matches,
        base_uri,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_html_template_dependencies_with_sources(
    host: &mut HtmlBaseExecutionHost,
    document: Arc<HtmlExecutionInventory>,
    context: Arc<EngineContext>,
    root: Arc<TemplateInput>,
    document_capability: &CemModuleUrlResolutionCapability,
    body_capabilities: &BTreeMap<String, CemModuleUrlResolutionCapability>,
    options: &CompileTemplateOptions,
    limits: HtmlTemplateDependencyLimits,
    mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
    base_uri: &str,
    sources: Option<(
        Arc<NativeTemplateDependencyInspection>,
        Arc<PreflightBodyInventory>,
    )>,
) -> Result<HtmlTemplateDependencyPreparation, String> {
    host.validate_dependencies(&document, document_capability, &mut matches, base_uri)?;
    if options.skip_cemt_function_bodies || limits.max_bodies == 0 || limits.per_body_budget == 0 {
        return Err("complete template bodies and a nonzero analysis budget are required".into());
    }
    if body_capabilities.len() > limits.max_bodies || !body_capabilities.contains_key(&root.uri) {
        return Err("bounded execution contexts including the root are required".into());
    }
    // Capture all body policies before preflight callbacks can acquire imports.
    let authorities = body_capabilities
        .iter()
        .map(|(uri, capability)| {
            Ok((
                uri.clone(),
                BodyAuthority {
                    capability: capability.clone(),
                    fingerprint: fingerprint(capability)?,
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let (preflight, bodies) = match sources {
        Some(sources) => sources,
        None => {
            let preflight = Arc::new(inspect_native_template_dependencies(
                &context,
                &root,
                limits.max_import_depth,
            ));
            let bodies = Arc::new(inspect_preflighted_bodies(
                &root,
                &preflight,
                options,
                limits.max_bodies,
                limits.per_body_budget,
            ));
            (preflight, bodies)
        }
    };
    if preflight.xpath_resource_uses.iter().any(|slot| {
        slot.inspection
            .blockers
            .iter()
            .any(|b| b == "inspection-budget-exceeded")
    }) {
        return Err("complete native XPath inspection required within budget".into());
    }
    if !bodies.blockers.is_empty() {
        return Err(format!(
            "native template preflight/body analysis failed: {}",
            bodies.blockers.join(", ")
        ));
    }
    let mut seen = BTreeMap::new();
    for body in &bodies.bodies {
        if !authorities.contains_key(&body.uri) {
            return Err(format!(
                "missing execution context for template body {}",
                body.uri
            ));
        }
        if let Some(previous_hash) = seen.insert(&body.uri, &body.content_hash) {
            if previous_hash != &body.content_hash {
                return Err("preflight returned conflicting content for one template URI".into());
            }
        }
        let mut css_budget = limits.per_body_budget;
        for sheet in &body.stylesheets {
            check_stylesheet_budget(sheet.css_tree(), &mut css_budget)?;
        }
        if body
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
            || body
                .blockers
                .iter()
                .any(|b| b == "template-inspection-budget-exceeded")
            || body.expressions.iter().any(|e| {
                e.query().is_none()
                    || e.inspection
                        .blockers
                        .iter()
                        .any(|b| b == "inspection-budget-exceeded")
            })
        {
            return Err(format!(
                "complete compiled template analysis required for {}",
                body.uri
            ));
        }
    }
    if seen.len() != authorities.len() {
        return Err("execution contexts must match the complete preflight body set".into());
    }
    validate_authorities(&authorities, body_capabilities)?;
    host.validate_dependencies(&document, document_capability, &mut matches, base_uri)?;

    let mut blockers = document.blockers().to_vec();
    let mut resolutions = Vec::new();
    for body in &bodies.bodies {
        let capability = &authorities[&body.uri].capability;
        let mut resolved = TemplateBodyResolutions {
            expressions: Vec::new(),
            stylesheets: Vec::new(),
        };
        blockers.extend(body.blockers.iter().cloned());
        for expression in &body.expressions {
            // Completeness was checked for every body before any resolution.
            let inventory = inspect_query_module_resolutions(
                expression.query().unwrap(),
                capability,
                capability.context(),
                limits.per_body_budget,
            );
            blockers.extend(inventory.inspection.blockers.iter().cloned());
            if inventory.resolutions.iter().any(|(_, r)| r.is_err()) {
                blockers.push("template-module-resolution-incomplete".into());
            }
            resolved.expressions.push(inventory);
        }
        for index in 0..body.stylesheets.len() {
            let plan = body.resolve_stylesheet_resources(index, capability)?;
            if plan.references.iter().any(|r| r.resolution.is_err()) {
                blockers.push("template-stylesheet-resolution-incomplete".into());
            }
            resolved.stylesheets.push(plan);
        }
        resolutions.push(resolved);
    }
    for slot in &preflight.xpath_resource_uses {
        blockers.extend(slot.inspection.blockers.iter().cloned());
    }
    blockers.extend(
        [
            "template-output-correspondence-required",
            "template-acquisition-freshness-required",
            "runtime-dependency-closure-required",
            "atomic-host-transaction-required",
        ]
        .map(str::to_owned),
    );
    blockers.sort();
    blockers.dedup();
    let result = HtmlTemplateDependencyPreparation {
        document,
        context,
        root,
        authorities,
        preflight,
        bodies,
        resolutions,
        blockers,
    };
    result.validate(
        host,
        &result.context,
        &result.root,
        document_capability,
        body_capabilities,
        matches,
        base_uri,
    )?;
    Ok(result)
}
fn fingerprint(capability: &CemModuleUrlResolutionCapability) -> Result<String, String> {
    capability
        .cache_identity()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "template preparation requires a complete resolver fingerprint".into())
}
pub(crate) fn check_stylesheet_budget(
    tree: &cem_ml::parser::tree::RetainedCemTree,
    remaining: &mut usize,
) -> Result<(), String> {
    let mut pending = vec![0];
    while let Some(id) = pending.pop() {
        *remaining = remaining
            .checked_sub(1)
            .ok_or("complete retained stylesheet inspection required within budget")?;
        let node = tree.node(id).ok_or("invalid retained stylesheet node")?;
        // Include attributes and bound the work queue before extending it.
        if node.children.len().saturating_add(node.attributes.len())
            > remaining.saturating_sub(pending.len())
        {
            return Err("complete retained stylesheet inspection required within budget".into());
        }
        pending.extend(node.children.iter().chain(&node.attributes).copied());
    }
    Ok(())
}
fn validate_authorities(
    authorities: &BTreeMap<String, BodyAuthority>,
    capabilities: &BTreeMap<String, CemModuleUrlResolutionCapability>,
) -> Result<(), String> {
    if authorities.len() != capabilities.len() {
        return Err("template execution context set changed".into());
    }
    for (uri, authority) in authorities {
        let capability = capabilities
            .get(uri)
            .ok_or("template execution context missing")?;
        if !authority.capability.same_owner(capability)
            || authority.fingerprint != fingerprint(capability)?
        {
            return Err(format!(
                "template resolver owner or policy changed for {uri}"
            ));
        }
    }
    Ok(())
}
