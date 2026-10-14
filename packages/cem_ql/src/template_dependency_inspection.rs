//! Analysis of immutable native-preflight bodies. Never renders or acquires data.

#[cfg(test)]
mod tests {
    use super::*;
    use cem_ml::{
        engine::{EngineContext, TemplateInput},
        real::inspect_native_template_dependencies,
        run_config::ScopeConfig,
    };
    fn root() -> TemplateInput {
        TemplateInput { uri: "https://example.test/root.cemt".into(),
            bytes: br#"{div @title='{module:module_url("asset")}' | {cem:if @test='true' | {cem:variable @name=loaded @select='ct:read("file:///never-read.xml")'}}}"#.to_vec(),
            identity: None, root_scope: ScopeConfig::default() }
    }
    #[test]
    fn preflight_inventory_retains_slots_without_evaluation() {
        let root = root();
        let preflight = inspect_native_template_dependencies(&EngineContext::default(), &root, 8);
        let result = inspect_preflighted_bodies(&root, &preflight, &Default::default(), 8, 100);
        assert!(result.blockers.is_empty(), "{:?}", result.blockers);
        assert_eq!(result.bodies.len(), 1);
        let body = &result.bodies[0];
        assert_eq!(body.uri, root.uri);
        assert_eq!(body.expressions.len(), 3, "{:?}", body.expressions);
        assert_eq!(
            body.expressions
                .iter()
                .map(|e| e.inspection.uses.len())
                .sum::<usize>(),
            2
        );
        assert!(result.body_analysis_required);
    }
    #[test]
    fn preflight_inventory_rejects_stale_and_failed_preflight() {
        let mut root = root();
        let mut preflight =
            inspect_native_template_dependencies(&EngineContext::default(), &root, 8);
        root.bytes.push(b' ');
        let result = inspect_preflighted_bodies(&root, &preflight, &Default::default(), 8, 100);
        assert!(result.bodies.is_empty());
        assert!(result.blockers.iter().any(|b| b.contains("hash")));
        root.bytes.pop();
        preflight.preflight = None;
        let result = inspect_preflighted_bodies(&root, &preflight, &Default::default(), 8, 100);
        assert!(result.bodies.is_empty());
        assert!(result.body_analysis_required);
    }
    #[test]
    fn preflight_inventory_imports_keep_owners_and_reject_stale_bytes() {
        use cem_ml::resolver::*;
        struct Loader;
        impl ResourceResolver for Loader {
            fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
                Ok(ResolvedRead { uri: request.uri.clone(), bytes: br#"{module | {template @name=card | {div @title='{module:module_url("icon")}'}}}"#.to_vec(), content_type: Some("text/cem-ml".into()) })
            }
            fn write(
                &self,
                request: &ResolveRequest,
                _: &[u8],
            ) -> Result<ResolvedWrite, ResolverDiagnostic> {
                Err(ResolverDiagnostic::UnsupportedResolver {
                    uri: request.uri.clone(),
                    purpose: request.purpose,
                    direction: ResolveDirection::Write,
                })
            }
        }
        let mut root = root();
        root.bytes = br#"{module | {import @as=card @src="https://example.test/card.cemt"} {template @name=main | {span | Ready}}}"#.to_vec();
        let mut context = EngineContext::default();
        context.resolver_registry.register(
            "https",
            ResolvePurpose::Template,
            ResolveDirection::Read,
            Loader,
        );
        let mut preflight = inspect_native_template_dependencies(&context, &root, 8);
        assert!(preflight.preflight.is_some(), "{:?}", preflight.diagnostics);
        let result = inspect_preflighted_bodies(&root, &preflight, &Default::default(), 8, 100);
        assert!(result.blockers.is_empty(), "{:?}", result.blockers);
        assert_eq!(result.bodies.len(), 2);
        assert_eq!(result.bodies[1].uri, "https://example.test/card.cemt");
        assert_eq!(
            result.bodies[1].expressions[0].template_uri,
            result.bodies[1].uri
        );
        assert_eq!(
            result.bodies[1].expressions[0].inspection.uses[0]
                .literal_argument
                .as_deref(),
            Some("icon")
        );
        preflight.preflight.as_mut().unwrap().resolved_imports[0]
            .bytes
            .push(b' ');
        let rejected = inspect_preflighted_bodies(&root, &preflight, &Default::default(), 8, 100);
        assert!(rejected.bodies.is_empty());
        assert!(rejected.blockers.iter().any(|b| b.contains("hash")));
    }
    #[test]
    fn preflight_stylesheet_inventory_keeps_native_source_and_owner_base() {
        use cem_ml::module_resolution::*;
        let mut root = root();
        root.bytes = br#"{style |```p { background: url(./image.svg) }```}{p | Ready}"#.to_vec();
        let preflight = inspect_native_template_dependencies(&EngineContext::default(), &root, 8);
        let inventory = inspect_preflighted_bodies(&root, &preflight, &Default::default(), 8, 100);
        assert!(inventory.blockers.is_empty(), "{:?}", inventory.blockers);
        let body = &inventory.bodies[0];
        let context = CemResolutionContextHandle::new("execution");
        let resolver = CemScopedModuleUrlResolver::new().with_context(
            context.clone(),
            CemModuleUrlContext {
                identity: "execution".into(),
                resolver_identity: "resolver".into(),
                resource_policy_stamp: "policy".into(),
                frames: vec![CemModuleUrlFrame::new(
                    "execution",
                    "https://example.test/wrong/base.xhtml",
                )],
            },
        );
        let capability =
            CemModuleUrlResolutionCapability::new(std::sync::Arc::new(resolver), context);
        let plan = body.resolve_stylesheet_resources(0, &capability).unwrap();
        assert!(std::sync::Arc::ptr_eq(
            &plan.tree,
            body.stylesheets[0].css_tree()
        ));
        assert_eq!(
            plan.references[0].resolution.as_ref().unwrap().resolved_url,
            "https://example.test/image.svg"
        );
        assert_eq!(
            plan.references[0]
                .resolution
                .as_ref()
                .unwrap()
                .resource_policy_stamp,
            "policy"
        );
        assert!(body.resolve_stylesheet_resources(1, &capability).is_err());
        assert!(inventory.body_analysis_required);
    }
    #[test]
    fn preflight_inventory_bounds_body_and_slot_work() {
        let root = root();
        let preflight = inspect_native_template_dependencies(&EngineContext::default(), &root, 8);
        let result = inspect_preflighted_bodies(&root, &preflight, &Default::default(), 0, 100);
        assert!(result.bodies.is_empty());
        assert!(!result.blockers.is_empty());
        let result = inspect_preflighted_bodies(&root, &preflight, &Default::default(), 8, 0);
        assert!(result.bodies[0].expressions.is_empty());
        assert!(!result.bodies[0].blockers.is_empty());
    }
}

use crate::{
    dependency_inspection::{
        inspect_template_expression_resources, TemplateExpressionResourceInspection,
    },
    render::{CompileTemplateOptions, TemplateAttributePart, TemplateAttributeValue, TemplateNode},
};
use cem_ml::{engine::TemplateInput, real::NativeTemplateDependencyInspection};

pub struct TemplateBodyInventory {
    pub uri: String,
    pub content_hash: String,
    pub expressions: Vec<TemplateExpressionResourceInspection>,
    pub stylesheets: Vec<crate::render::TemplateStylesheetArtifact>,
    pub module_map: Option<crate::render::TemplateModuleMapArtifact>,
    pub blockers: Vec<String>,
    pub diagnostics: Vec<cem_ml::diagnostics::Diagnostic>,
}
impl TemplateBodyInventory {
    /// Resolve one retained inline sheet under the supplied execution capability.
    /// The final preflight body URI is its URL base; synthetic CSS source URIs
    /// only identify local coordinates. The host must include this body's effective
    /// module maps in the capability. This neither loads imports nor grants edits.
    pub fn resolve_stylesheet_resources(
        &self,
        index: usize,
        capability: &cem_ml::module_resolution::CemModuleUrlResolutionCapability,
    ) -> Result<cem_ml::css_resources::CssResourcePlan, String> {
        if self
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
        {
            return Err("template compilation diagnostics prevent stylesheet analysis".into());
        }
        let sheet = self
            .stylesheets
            .get(index)
            .ok_or("unknown template stylesheet occurrence")?;
        cem_ml::css_resources::resolve_css_resources(
            sheet.css_tree().clone(),
            capability,
            &self.uri,
        )
    }
}
pub struct PreflightBodyInventory {
    pub bodies: Vec<TemplateBodyInventory>,
    pub blockers: Vec<String>,
    pub body_analysis_required: bool,
}

/// Consume an execution preflight without resolving or acquiring its imports again.
/// Hashes bind inventories to supplied bytes, not to a live project revision or grant.
/// Limits bound body count and each body's traversal and expression IR inspection;
/// compilation retains its own native limits. No result authorizes an edit.
pub fn inspect_preflighted_bodies(
    root: &TemplateInput,
    preflight: &NativeTemplateDependencyInspection,
    options: &CompileTemplateOptions,
    max_bodies: usize,
    per_body_budget: usize,
) -> PreflightBodyInventory {
    let mut result = PreflightBodyInventory {
        bodies: Vec::new(),
        blockers: Vec::new(),
        body_analysis_required: true,
    };
    let Some(graph) = &preflight.preflight else {
        result.blockers.push("template-preflight-required".into());
        return result;
    };
    if preflight
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation())
    {
        result
            .blockers
            .push("template-preflight-diagnostics".into());
        return result;
    }
    if root.uri != preflight.root_uri || hash(&root.bytes) != preflight.root_content_hash {
        result
            .blockers
            .push("root-preflight-identity-or-hash-mismatch".into());
        return result;
    }
    if graph.resolved_imports.len().saturating_add(1) > max_bodies {
        result.blockers.push("body-budget-exceeded".into());
        return result;
    }
    let bodies = std::iter::once((
        root.uri.as_str(),
        root.bytes.as_slice(),
        preflight.root_content_hash.as_str(),
    ))
    .chain(
        graph
            .resolved_imports
            .iter()
            .map(|m| (m.uri.as_str(), m.bytes.as_slice(), m.content_hash.as_str())),
    );
    let mut sources = Vec::new();
    for (uri, bytes, expected_hash) in bodies {
        if hash(bytes) != expected_hash {
            result
                .blockers
                .push("import-preflight-hash-mismatch".into());
            return result;
        }
        let Ok(source) = std::str::from_utf8(bytes) else {
            result
                .blockers
                .push("template-source-encoding-unsupported".into());
            return result;
        };
        sources.push((uri, expected_hash, source));
    }
    for (uri, content_hash, source) in sources {
        let mut artifact = crate::render::compile_template(source, options);
        let mut body = TemplateBodyInventory {
            uri: uri.into(),
            content_hash: content_hash.into(),
            expressions: Vec::new(),
            stylesheets: Vec::new(),
            module_map: None,
            blockers: vec!["template-runtime-analysis-required".into()],
            diagnostics: artifact.diagnostics.clone(),
        };
        if options.skip_cemt_function_bodies {
            body.blockers.push("function-bodies-skipped".into());
        }
        if !artifact.stylesheets.is_empty() {
            body.blockers
                .push("stylesheet-owner-context-required".into());
            body.blockers
                .push("template-stylesheet-import-closure-required".into());
        }
        if artifact.module_map.is_some() {
            body.blockers
                .push("template-module-map-context-required".into());
        }
        if artifact
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
        {
            body.blockers
                .push("template-compilation-diagnostics".into());
        }
        body.blockers
            .push("template-output-document-analysis-required".into());
        inspect_nodes(&artifact.nodes, per_body_budget, &mut body);
        body.stylesheets = std::mem::take(&mut artifact.stylesheets);
        body.module_map = artifact.module_map.take();
        result.bodies.push(body);
    }
    result
}
fn hash(bytes: &[u8]) -> String {
    cem_ml::content_cache::ContentHash::from_blake3(bytes).header_value()
}
fn inspect_nodes(nodes: &[TemplateNode], budget: usize, body: &mut TemplateBodyInventory) {
    use crate::render::{ChooseBranch, CompiledTemplateExpression, TemplateAttribute};
    enum Work<'a> {
        Nodes(&'a [TemplateNode]),
        Attributes(&'a [TemplateAttribute]),
        Parts(&'a [TemplateAttributePart]),
        Branches(&'a [ChooseBranch]),
        Expression(&'a CompiledTemplateExpression),
    }
    let mut pending = vec![Work::Nodes(nodes)];
    let mut remaining = budget;
    while let Some(work) = pending.pop() {
        if remaining == 0 {
            body.blockers
                .push("template-inspection-budget-exceeded".into());
            break;
        }
        remaining -= 1;
        match work {
            Work::Expression(expression) => body.expressions.push(
                inspect_template_expression_resources(&body.uri, expression, budget),
            ),
            Work::Nodes(nodes) => {
                if let Some((node, tail)) = nodes.split_first() {
                    if !tail.is_empty() {
                        pending.push(Work::Nodes(tail));
                    }
                    match node {
                        TemplateNode::Element {
                            attributes,
                            children,
                            ..
                        }
                        | TemplateNode::Result {
                            attributes,
                            children,
                            ..
                        } => {
                            if matches!(node, TemplateNode::Element { tag, .. }
                                if tag.rsplit(':').next() == Some("module-map"))
                            {
                                body.blockers.push("unapplied-template-module-map".into());
                            }
                            pending.push(Work::Nodes(children));
                            pending.push(Work::Attributes(attributes));
                        }
                        TemplateNode::Expression(e) => pending.push(Work::Expression(e)),
                        TemplateNode::If { test, children, .. } => {
                            pending.push(Work::Nodes(children));
                            if let Some(e) = test {
                                pending.push(Work::Expression(e));
                            }
                        }
                        TemplateNode::ForEach {
                            select, children, ..
                        } => {
                            pending.push(Work::Nodes(children));
                            if let Some(e) = select {
                                pending.push(Work::Expression(e));
                            }
                        }
                        TemplateNode::Variable { select, .. }
                        | TemplateNode::ProjectPayload { select, .. } => {
                            if let Some(e) = select {
                                pending.push(Work::Expression(e));
                            }
                        }
                        TemplateNode::Choose { branches, .. } => {
                            pending.push(Work::Branches(branches))
                        }
                        TemplateNode::Text { .. } | TemplateNode::Comment { .. } => {}
                    }
                }
            }
            Work::Attributes(attrs) => {
                if let Some((attr, tail)) = attrs.split_first() {
                    if !tail.is_empty() {
                        pending.push(Work::Attributes(tail));
                    }
                    match &attr.value {
                        Some(TemplateAttributeValue::Expression(e)) => {
                            pending.push(Work::Expression(e))
                        }
                        Some(TemplateAttributeValue::Template(parts)) => {
                            pending.push(Work::Parts(parts))
                        }
                        _ => {}
                    }
                }
            }
            Work::Parts(parts) => {
                if let Some((part, tail)) = parts.split_first() {
                    if !tail.is_empty() {
                        pending.push(Work::Parts(tail));
                    }
                    if let TemplateAttributePart::Expression(e) = part {
                        pending.push(Work::Expression(e));
                    }
                }
            }
            Work::Branches(branches) => {
                if let Some((branch, tail)) = branches.split_first() {
                    if !tail.is_empty() {
                        pending.push(Work::Branches(tail));
                    }
                    pending.push(Work::Nodes(&branch.children));
                    if let Some(e) = &branch.test {
                        pending.push(Work::Expression(e));
                    }
                }
            }
        }
    }
}

pub struct QueryModuleResolutionInventory {
    pub inspection: crate::dependency_inspection::ResourceInspection,
    /// Indices refer to `inspection.uses`; errors preserve native policy provenance.
    pub resolutions: Vec<(
        usize,
        Result<
            cem_ml::module_resolution::CemModuleUrlResolution,
            cem_ml::module_resolution::CemModuleUrlResolutionError,
        >,
    )>,
}
/// Resolve direct, literal module_url calls using the same purpose/current-context/
/// explicit-referrer contract as evaluation. The caller supplies the live execution
/// context; template URI and synthetic source coordinates are never fallback bases.
/// Computed arguments, unsupported pipeline steps and content reads stay unresolved.
pub fn inspect_query_module_resolutions(
    query: &crate::ir::CompiledQuery,
    capability: &cem_ml::module_resolution::CemModuleUrlResolutionCapability,
    current_context: &cem_ml::module_resolution::CemResolutionContextHandle,
    budget: usize,
) -> QueryModuleResolutionInventory {
    use cem_ml::module_resolution::{CemModuleUrlReferrer, CemModuleUrlResolutionPurpose};
    let mut inspection = crate::dependency_inspection::inspect_query_resources(query, budget);
    let mut resolutions = Vec::new();
    for (index, candidate) in inspection.uses.iter().enumerate() {
        if candidate.kind != crate::dependency_inspection::ResourceUseKind::ModuleUrl {
            continue;
        }
        if candidate.pipeline_step.is_some() {
            inspection
                .blockers
                .push("unsupported-module-url-pipeline".into());
            continue;
        }
        let Some(crate::ir::IrNode::StdlibCall { args, .. }) = query.tree.node(candidate.node_id)
        else {
            continue;
        };
        if !(1..=2).contains(&args.len()) {
            inspection.blockers.push("invalid-module-url-arity".into());
            continue;
        }
        let Some(specifier) = &candidate.literal_argument else {
            continue;
        };
        let referrer = if args.len() == 2 {
            let Some(referrer) = &candidate.literal_referrer else {
                continue;
            };
            Some(CemModuleUrlReferrer::Url(referrer.clone()))
        } else {
            None
        };
        // Evaluation attributes resolution diagnostics to the first argument.
        let Some(source) = query.tree.source_maps.get(args[0].0 as usize) else {
            inspection
                .blockers
                .push("missing-resolution-source-provenance".into());
            continue;
        };
        resolutions.push((
            index,
            capability.resolve_from_context(
                CemModuleUrlResolutionPurpose::CemQl,
                specifier,
                current_context,
                referrer,
                source.clone(),
            ),
        ));
    }
    QueryModuleResolutionInventory {
        inspection,
        resolutions,
    }
}

#[cfg(test)]
mod resolution_tests {
    use super::*;
    use cem_ml::module_resolution::*;
    use std::sync::{Arc, Mutex};
    struct Recorder(Mutex<Vec<CemModuleUrlResolutionRequest>>);
    impl CemModuleUrlResolver for Recorder {
        fn resolve_module_url(
            &self,
            request: &CemModuleUrlResolutionRequest,
        ) -> Result<CemModuleUrlResolution, CemModuleUrlResolutionError> {
            self.0.lock().unwrap().push(request.clone());
            Err(CemModuleUrlResolutionError {
                authored_specifier: request.authored_specifier.clone(),
                normalized_specifier: None,
                context_identity: "context".into(),
                resolver_identity: "fixture-resolver".into(),
                resource_policy_stamp: "denied-policy".into(),
                referrer_kind: None,
                authored_referrer: None,
                resolved_referrer_url: None,
                current_context_identity: request.current_context.0.clone(),
                selected_context_identity: None,
                reason: CemModuleUrlResolutionErrorReason::PolicyDenied,
                message: "fixture denial".into(),
                matched_frame_id: None,
                matched_key: None,
            })
        }
    }
    #[test]
    fn preflight_inventory_resolution_uses_explicit_execution_context_and_keeps_denials() {
        let recorder = Arc::new(Recorder(Mutex::new(Vec::new())));
        let capability = CemModuleUrlResolutionCapability::new(
            recorder.clone(),
            CemResolutionContextHandle::new("wrong-default"),
        );
        let query = crate::api::compile_expression(r#"(module:module_url("asset"), module:module_url("other", "https://example.test/lib/"))"#, &Default::default()).unwrap().query;
        let result = inspect_query_module_resolutions(
            &query,
            &capability,
            &CemResolutionContextHandle::new("execution-context"),
            100,
        );
        assert_eq!(result.resolutions.len(), 2);
        assert!(result.resolutions.iter().all(|(_, r)| r
            .as_ref()
            .unwrap_err()
            .resource_policy_stamp
            == "denied-policy"));
        let requests = recorder.0.lock().unwrap();
        assert!(requests
            .iter()
            .all(|r| r.purpose == CemModuleUrlResolutionPurpose::CemQl
                && r.current_context.0 == "execution-context"));
        assert_eq!(requests[0].referrer, None);
        assert_eq!(
            requests[1].referrer,
            Some(CemModuleUrlReferrer::Url(
                "https://example.test/lib/".into()
            ))
        );
        assert!(result.inspection.body_analysis_required);
    }
    #[test]
    fn preflight_inventory_resolution_does_not_guess_computed_referrers_or_run_reads() {
        let recorder = Arc::new(Recorder(Mutex::new(Vec::new())));
        let context = CemResolutionContextHandle::new("execution");
        let capability = CemModuleUrlResolutionCapability::new(recorder.clone(), context.clone());
        let query = crate::api::compile_expression(r#"(module:module_url("asset", str:trim("base")), ct:read("file:///never-read.xml"), "asset".module:module_url())"#, &Default::default()).unwrap().query;
        let result = inspect_query_module_resolutions(&query, &capability, &context, 100);
        assert!(result.resolutions.is_empty());
        assert!(recorder.0.lock().unwrap().is_empty());
        assert!(result
            .inspection
            .blockers
            .iter()
            .any(|b| b.contains("pipeline")));
    }
}
