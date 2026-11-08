//! Resource-free instance CSS admission using the shared retained stylesheet owner.
use crate::{
    eval::RetainedCemNode,
    render::{RenderPlanNode, TemplateArtifact, TemplateStylesheetArtifact},
    retained_template::{RetainedStylesheet, RetainedTemplate},
};
use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::tree::RetainedCemTree,
    source_map::SourceMapStack,
};
use std::sync::Arc;
pub(crate) struct InstanceCss {
    _owner: RetainedTemplate,
    records: Vec<Arc<RetainedStylesheet>>,
    maps: Vec<SourceMapStack>,
    control: OperationControl,
}
impl std::fmt::Debug for InstanceCss {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstanceCss")
            .field("styles", &self.records.len())
            .finish_non_exhaustive()
    }
}
impl InstanceCss {
    pub(crate) fn prepare(
        source: &Arc<RetainedCemTree>,
        payload: &[RetainedCemNode],
        explicit: bool,
        instance: &str,
        base: &str,
        policy: &cem_ml::ssr_artifact::SsrSettlePolicy,
        control: &OperationControl,
    ) -> Result<Option<Self>, String> {
        use cem_ml::{
            css_emission::CssManagedScope,
            css_imports::{CssImportClosure, CssImportLimits},
            module_resolution::{
                CemModuleUrlResolutionCapability, CemResolutionContextHandle,
                CemScopedModuleUrlResolver,
            },
        };
        let (sources, maps) = collect_sources(source, payload, explicit, policy, control)?;
        if sources.is_empty() {
            return Ok(None);
        }
        let mut owner = RetainedTemplate::new_instance_stylesheet_artifacts(
            TemplateArtifact {
                nodes: vec![],
                stylesheets: sources,
                module_map: None,
                diagnostics: vec![],
            },
            instance,
        )?;
        // No resolver context is granted by this profile. Resource-bearing
        // sheets remain refused for the coordinator to admit explicitly.
        let capability = CemModuleUrlResolutionCapability::new(
            Arc::new(CemScopedModuleUrlResolver::new()),
            CemResolutionContextHandle::new("ssr-resource-free-css"),
        );
        let mut records = vec![];
        for index in 0..owner.artifact().stylesheets.len() {
            control
                .check_scope(ROOT_EXECUTION_SCOPE_ID)
                .map_err(|e| e.code().to_owned())?;
            let closure = CssImportClosure::new(
                owner.artifact().stylesheets[index].css_tree().clone(),
                base,
                capability.clone(),
                CssImportLimits {
                    max_sheets: 1,
                    max_depth: 0,
                },
                control.abort_signal().clone(),
            )
            .map_err(|e| e.to_string())?;
            if closure
                .sheets()
                .iter()
                .any(|s| !s.resources.references.is_empty())
            {
                return Err("Instance CSS resource closure requires explicit coordination".into());
            }
            let record = owner.retain_stylesheet(
                instance,
                index,
                Arc::new(closure),
                &CssManagedScope::Instance,
                instance,
            )?;
            if !record.emission()?.diagnostics.is_empty() {
                return Err("Instance CSS contains unsupported managed syntax".into());
            }
            records.push(record);
        }
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|e| e.code().to_owned())?;
        Ok(Some(Self {
            _owner: owner,
            records,
            maps,
            control: control.clone(),
        }))
    }
    pub(crate) fn check(&self, control: &OperationControl) -> Result<(), String> {
        if !self.control.same_operation(control) {
            return Err("Instance CSS admission belongs to the original operation".into());
        }
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|e| e.code().to_owned())?;
        for record in &self.records {
            record.emission()?;
        }
        Ok(())
    }
    pub(crate) fn records(&self) -> &[Arc<RetainedStylesheet>] {
        &self.records
    }
    pub(crate) fn strip_sources(&self, nodes: &mut Vec<RenderPlanNode>) {
        strip_sources(nodes, &self.maps)
    }
    pub(crate) fn output(&self) -> Result<Vec<RenderPlanNode>, String> {
        self.records
            .iter()
            .enumerate()
            .map(|(index, record)| {
                let source_map = self.maps[index].clone();
                let css = record.emission()?.css();
                // A later HTML export must not be able to terminate the raw-text
                // element. Refuse this profile rather than rewriting authored CSS.
                if css.to_ascii_lowercase().contains("</style") {
                    return Err("Unsafe style raw-text export".into());
                }
                Ok(RenderPlanNode::Element {
                    tag: "style".into(),
                    namespace: Some(cem_ml::schema::registry::HTML_NAMESPACE_URI.into()),
                    qualified_name: None,
                    attributes: vec![crate::instance_island::construct::attribute(
                        "data-cem-instance-style",
                        &index.to_string(),
                        &source_map,
                    )],
                    children: vec![RenderPlanNode::Text {
                        text: css,
                        source_map: source_map.clone(),
                    }],
                    source_map,
                })
            })
            .collect()
    }
}

pub(crate) fn collect_sources(
    source: &Arc<RetainedCemTree>,
    payload: &[RetainedCemNode],
    explicit: bool,
    policy: &cem_ml::ssr_artifact::SsrSettlePolicy,
    control: &OperationControl,
) -> Result<(Vec<TemplateStylesheetArtifact>, Vec<SourceMapStack>), String> {
    let check = || {
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|e| e.code().to_owned())
    };
    let mut pending = payload
        .iter()
        .rev()
        .map(|n| n.node_id())
        .collect::<Vec<_>>();
    let mut sources = vec![];
    let mut maps = vec![];
    let mut bytes = 0usize;
    while let Some(id) = pending.pop() {
        check()?;
        let n = source.node(id).ok_or("Missing CSS source")?;
        if let Some(name) = &n.name {
            if name.namespace_uri == cem_ml::schema::registry::HTML_NAMESPACE_URI {
                if name.local_name == "template" {
                    continue;
                }
                if name.local_name == "style" {
                    if !explicit || sources.len() >= (policy.max_resources as usize).min(64) {
                        return Err("Instance CSS requires an explicit envelope and available resource budget".into());
                    }
                    let imported = cem_ml::import::inline_css::retain_inline_css(
                        source.clone(),
                        id,
                        control,
                        &crate::instance_island::LIMITS,
                    )?;
                    bytes = bytes.saturating_add(imported.css().len());
                    if bytes as u64 > policy.max_total_bytes {
                        return Err("Instance CSS byte budget exceeded".into());
                    }
                    sources.push(TemplateStylesheetArtifact::from_inline_css(imported));
                    maps.push(source.node_source_map(id).unwrap_or_default());
                    continue;
                }
            }
        }
        pending.extend(n.children.iter().rev());
    }
    Ok((sources, maps))
}

pub(crate) fn strip_sources(nodes: &mut Vec<RenderPlanNode>, maps: &[SourceMapStack]) {
    nodes.retain(|n| !matches!(n,RenderPlanNode::Element{tag,namespace,source_map,..} if tag=="style" && namespace.as_deref()==Some(cem_ml::schema::registry::HTML_NAMESPACE_URI) && maps.contains(source_map)));
    for node in nodes {
        if let RenderPlanNode::Element {
            tag,
            namespace,
            children,
            ..
        } = node
        {
            if !(tag == "template"
                && namespace.as_deref() == Some(cem_ml::schema::registry::HTML_NAMESPACE_URI))
            {
                strip_sources(children, maps);
            }
        }
    }
}
