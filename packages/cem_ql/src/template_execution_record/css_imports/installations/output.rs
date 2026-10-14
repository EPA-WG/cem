//! Whole native output composition and joint HTML/CSS consumption. Serialized
//! exports are delivery artifacts only; private retained owners carry authority.
use super::*;
use crate::render::{self, RenderPlanNode};
use cem_ml::{
    html_execution_inventory::HtmlExecutionInventory,
    html_stylesheet_execution::HtmlStylesheetExecution,
    interpreter::TransformOutput,
    operation_control::ROOT_EXECUTION_SCOPE_ID,
    parser::{
        tree::{CemTreeNodeKind, RetainedCemTree},
        AstNodeId,
    },
    stylesheet_execution::{
        remaining_blockers, StylesheetExecutionHost, StylesheetExecutionObserver,
        StylesheetObservation, StylesheetOwner,
    },
};
const HTML: &str = "http://www.w3.org/1999/xhtml";

pub struct RecordedTemplateOutput {
    installations: Arc<RecordedBodyStylesheetInstallations>,
    output: TransformOutput,
    html_output: TransformOutput,
    tree: Arc<RetainedCemTree>,
    stylesheet_nodes: Vec<AstNodeId>,
    max_bytes: usize,
}
#[derive(Default)]
pub struct RecordedTemplateOutputHost {
    document: StylesheetExecutionHost,
    compiled: CompiledStylesheetHost,
}
pub struct RecordedTemplateOutputEvidence {
    owner: Arc<RecordedTemplateOutput>,
    css: Arc<HtmlExecutionCssImports>,
    document: HtmlStylesheetExecution,
    compiled: RecordedBodyStylesheetEvidence,
    offsets: Vec<usize>,
    blockers: Vec<String>,
}
impl RecordedTemplateOutputHost {
    /// Call before every controlled execution change, including same-value writes.
    pub fn invalidate(&mut self) -> Result<(), String> {
        self.document.invalidate()?;
        self.compiled.invalidate()
    }
}
impl RecordedBodyStylesheetInstallations {
    /// One parent ID from the original native render per compiled occurrence.
    /// Recorded declaration sheets append to the native head. Instance payloads
    /// retain their separate contract. No source tree is edited or rebuilt by JS.
    pub fn compose_output(
        self: &Arc<Self>,
        parents: &[AstNodeId],
        max_nodes: usize,
        max_bytes: usize,
    ) -> Result<Arc<RecordedTemplateOutput>, String> {
        self.compose_output_profile(parents, None, max_nodes, max_bytes)
    }
    /// Explicit element-ordinal paths into the original native output, one list
    /// per stylesheet. Context-sensitive declarations require produced hosts.
    pub fn compose_output_with_hosts(
        self: &Arc<Self>,
        parents: &[AstNodeId],
        hosts: &[Vec<Vec<usize>>],
        max_nodes: usize,
        max_bytes: usize,
    ) -> Result<Arc<RecordedTemplateOutput>, String> {
        if hosts.len() != self.len() {
            return Err("complete stylesheet host binding lists required".into());
        }
        self.compose_output_profile(parents, Some(hosts), max_nodes, max_bytes)
    }
    fn compose_output_profile(
        self: &Arc<Self>,
        parents: &[AstNodeId],
        hosts: Option<&[Vec<Vec<usize>>]>,
        max_nodes: usize,
        max_bytes: usize,
    ) -> Result<Arc<RecordedTemplateOutput>, String> {
        self.validate_current()?;
        if parents.len() != self.len() || max_nodes == 0 || max_bytes == 0 {
            return Err("complete bounded native stylesheet placements required".into());
        }
        let execution = &self.source.preparation.execution;
        let mut nodes = execution.tree.ast().nodes.len();
        let mut bytes = execution.output.rendered.len();
        let mut paths = Vec::new();
        for (index, parent) in parents.iter().enumerate() {
            let node = execution
                .tree
                .node(*parent)
                .ok_or("native installation parent missing")?;
            let name = node
                .name
                .as_ref()
                .ok_or("HTML installation parent required")?;
            if node.kind != CemTreeNodeKind::Element
                || name.namespace_uri != HTML
                || name.local_name != "head"
                || node.parent != Some(element_at(&execution.tree, &[0])?)
                || matches!(self.set.identity(index)?.scope, CssManagedScope::Instance)
            {
                return Err("recorded declaration styles require a native HTML head parent".into());
            }
            let emission = self.set.emission(index)?;
            nodes = nodes
                .checked_add(emission.fragments.len() + 1)
                .ok_or("output node overflow")?;
            bytes = bytes.checked_add(16).ok_or("output byte overflow")?;
            for fragment in &emission.fragments {
                // Conservative XML escaping allowance before cloning/allocation.
                bytes = bytes
                    .checked_add(
                        fragment
                            .fragment
                            .text
                            .len()
                            .checked_mul(6)
                            .ok_or("output byte overflow")?,
                    )
                    .ok_or("output byte overflow")?;
            }
            if nodes > max_nodes || bytes > max_bytes {
                return Err("composed output node or byte bound exceeded".into());
            }
            if self.css(index)?.to_ascii_lowercase().contains("</style") {
                return Err(
                    "HTML raw-text style terminator is outside the composed output profile".into(),
                );
            }
            paths.push(element_path(&execution.tree, *parent)?);
        }
        let mut markers = std::collections::BTreeMap::<Vec<usize>, String>::new();
        if let Some(hosts) = hosts {
            for (index, paths) in hosts.iter().enumerate() {
                let identity = self.set.identity(index)?;
                if paths.len() > 256 || (identity.context_marker.is_some() && paths.is_empty()) {
                    return Err(
                        "bounded explicit produced hosts required for contextual styles".into(),
                    );
                }
                for path in paths {
                    if path.is_empty() || path.len() > 64 {
                        return Err("bounded native host path required".into());
                    }
                    let id = element_at(&execution.tree, path)?;
                    let node = execution.tree.node(id).ok_or("native host missing")?;
                    let name = node.name.as_ref().ok_or("native host name missing")?;
                    let scope_matches = match &identity.scope {
                        CssManagedScope::Private { tag, .. } => name.local_name == *tag,
                        CssManagedScope::Shared { name, .. } => {
                            attribute(&execution.tree, id, "scope").is_some_and(|value| {
                                value.split_ascii_whitespace().any(|v| v == name)
                            })
                        }
                        _ => false,
                    };
                    let produced = node.children.iter().any(|child| {
                        execution.tree.node(*child).is_some_and(|n| {
                            n.name.as_ref().is_some_and(|n| {
                                n.namespace_uri == HTML && n.local_name == "template"
                            }) && attribute(&execution.tree, *child, "data-cem-island")
                                == Some("instance")
                        })
                    });
                    if name.namespace_uri != HTML
                        || !name.local_name.contains('-')
                        || !scope_matches
                        || !produced
                    {
                        return Err(
                            "stylesheet binding requires a matching native produced host".into(),
                        );
                    }
                    if let Some(marker) = &identity.context_marker {
                        if attribute(&execution.tree, id, "data-cem-css-context")
                            .is_some_and(|value| value != marker)
                        {
                            return Err(
                                "authored host resolver context conflicts with stylesheet".into()
                            );
                        }
                        if let Some(previous) = markers.insert(path.clone(), marker.clone()) {
                            if previous != *marker {
                                return Err(
                                    "one produced host cannot bind different resolver contexts"
                                        .into(),
                                );
                            }
                        }
                    }
                }
            }
        }
        for (path, marker) in &markers {
            if attribute(
                &execution.tree,
                element_at(&execution.tree, path)?,
                "data-cem-css-context",
            )
            .is_none()
            {
                nodes = nodes.checked_add(1).ok_or("host attribute node overflow")?;
                bytes = bytes
                    .checked_add(
                        marker
                            .len()
                            .checked_mul(6)
                            .and_then(|n| n.checked_add(32))
                            .ok_or("host attribute byte overflow")?,
                    )
                    .ok_or("host attribute byte overflow")?;
            }
        }
        if nodes > max_nodes || bytes > max_bytes {
            return Err("composed output node or byte bound exceeded".into());
        }
        let mut plan = execution.plan.clone();
        for (path, marker) in &markers {
            let attributes = plan_attributes(&mut plan.nodes, path)?;
            if attributes.iter().any(|a| {
                a.name == "data-cem-css-context" && a.namespace.as_deref().unwrap_or("").is_empty()
            }) {
                continue;
            }
            attributes.push(render::RenderPlanAttribute {
                name: "data-cem-css-context".into(),
                namespace: None,
                qualified_name: None,
                value: marker.clone(),
                value_stream: crate::eval::ItemStream::once(crate::eval::Item::Atomic(
                    crate::eval::AtomValue::String(marker.clone()),
                )),
                contract: None,
                source_map: Default::default(),
            });
        }
        let mut sheet_paths = Vec::new();
        for (index, path) in paths.iter().enumerate() {
            let children = plan_children(&mut plan.nodes, path)?;
            let mut sheet_path = path.clone();
            sheet_path.push(
                children
                    .iter()
                    .filter(|n| matches!(n, RenderPlanNode::Element { .. }))
                    .count(),
            );
            let fragments = &self.set.emission(index)?.fragments;
            children.push(RenderPlanNode::Element {
                tag: "style".into(),
                namespace: Some(HTML.into()),
                qualified_name: Some("style".into()),
                attributes: vec![],
                children: fragments
                    .iter()
                    .map(|f| RenderPlanNode::Text {
                        text: f.fragment.text.clone(),
                        source_map: f.fragment.source.clone(),
                    })
                    .collect(),
                source_map: Default::default(),
            });
            sheet_paths.push(sheet_path);
        }
        let control = &execution.preflight.context().operation_control;
        let output =
            render::render_plan_to_xml_with_control(&plan, control, ROOT_EXECUTION_SCOPE_ID)
                .map_err(|e| e.to_string())?;
        let html_output =
            render::render_plan_to_html_with_control(&plan, control, ROOT_EXECUTION_SCOPE_ID)
                .map_err(|e| e.to_string())?;
        if output.rendered.len() > max_bytes || html_output.rendered.len() > max_bytes {
            return Err("composed output byte bound exceeded".into());
        }
        // Named native XML output/import boundary, as in execute_snapshot_xhtml.
        // Original template and CSS owners/source spans remain retained separately.
        let uri = execution.tree.source_uri();
        let tree = cem_ml::import::import_data_bytes(
            output.rendered.as_bytes(),
            "application/xml",
            "cem",
            uri,
        )?;
        if tree.ast().nodes.len() > max_nodes
            || tree
                .ast_owner()
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation())
        {
            return Err("complete bounded composed output required".into());
        }
        for (path, marker) in &markers {
            if attribute(&tree, element_at(&tree, path)?, "data-cem-css-context")
                != Some(marker.as_str())
            {
                return Err("native host context/output correspondence failed".into());
            }
        }
        let stylesheet_nodes = sheet_paths
            .iter()
            .map(|path| element_at(&tree, path))
            .collect::<Result<Vec<_>, _>>()?;
        for (index, node) in stylesheet_nodes.iter().enumerate() {
            let text = tree
                .text_fragments(*node)
                .ok_or("installed CSS text missing")?
                .collect::<String>();
            if text != self.css(index)? {
                return Err("native emitted stylesheet/output correspondence failed".into());
            }
        }
        self.validate_current()?;
        Ok(Arc::new(RecordedTemplateOutput {
            installations: self.clone(),
            output,
            html_output,
            tree,
            stylesheet_nodes,
            max_bytes,
        }))
    }
}
impl RecordedTemplateOutput {
    pub fn tree(&self) -> &Arc<RetainedCemTree> {
        &self.tree
    }
    pub fn output(&self) -> &TransformOutput {
        &self.output
    }
    pub fn html_output(&self) -> &TransformOutput {
        &self.html_output
    }
    pub fn original_execution(&self) -> &SnapshotTemplateExecution {
        &self.installations.source.preparation.execution
    }
    pub fn stylesheet_nodes(&self) -> &[AstNodeId] {
        &self.stylesheet_nodes
    }
    #[allow(clippy::too_many_arguments)]
    pub fn capture(
        self: &Arc<Self>,
        html: &mut HtmlBaseExecutionHost,
        document: Arc<HtmlExecutionInventory>,
        host: &mut RecordedTemplateOutputHost,
        observer: &mut impl StylesheetExecutionObserver,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base: &str,
        limits: CssSnapshotCaptureLimits,
    ) -> Result<RecordedTemplateOutputEvidence, String> {
        host.invalidate()?;
        let result = (|| {
            self.installations.validate_current()?;
            if !Arc::ptr_eq(&document.document().tree, &self.tree) {
                return Err(
                    "whole-output inventory must retain the exact composed native owner".into(),
                );
            }
            let source = &self.installations.source;
            let old = source.document.document().base();
            let new = document.base();
            if old.effective_url() != new.effective_url()
                || old.native_resolution().fallback_url != new.native_resolution().fallback_url
                || old.observation().policy_stamp != new.observation().policy_stamp
                || old.policy_decision() != new.policy_decision()
            {
                return Err(
                    "composed output base or policy differs from recorded execution".into(),
                );
            }
            let cap = &source.preparation.execution.capability;
            let css = Arc::new(
                html.inspect_css_imports(
                    document,
                    cap,
                    source.snapshot.clone(),
                    limits,
                    source
                        .preparation
                        .execution
                        .preflight
                        .context()
                        .operation_control
                        .abort_signal()
                        .clone(),
                    &mut matches,
                    base,
                )?,
            );
            let roots = css.execution_roots();
            let mut root_offsets = Vec::new();
            let mut offset = 0;
            for root in &roots {
                root_offsets.push(offset);
                offset += root.closure.closure().sheets().len();
            }
            let offsets = self.stylesheet_nodes.iter().map(|node| {
                let index = roots.iter().position(|r| matches!(r.owner, StylesheetOwner::Document { owner, source_node } if owner == *node && source_node == *node)).ok_or("composed stylesheet root missing")?;
                if roots[index].closure.closure().sheets().len() != 1 { return Err("compiled imports must form one installation".into()); }
                Ok(root_offsets[index])
            }).collect::<Result<Vec<_>, String>>()?;
            let document = css.record_execution(
                html,
                cap,
                &source.snapshot,
                &mut host.document,
                observer,
                limits.imports.max_sheets,
                &mut matches,
                base,
            )?;
            let compiled = self.installations.capture(
                &mut host.compiled,
                &mut LinkedObserver {
                    observations: document.execution().observations(),
                    offsets: &offsets,
                },
                self.max_bytes,
            )?;
            let mut blockers = remaining_blockers(&self.installations.source.blockers);
            blockers.extend(document.blockers().iter().cloned());
            blockers.extend(compiled.execution.blockers().iter().map(|s| s.to_string()));
            blockers.sort();
            blockers.dedup();
            let evidence = RecordedTemplateOutputEvidence {
                owner: self.clone(),
                css,
                document,
                compiled,
                offsets,
                blockers,
            };
            evidence.validate(html, host, observer, matches, base)?;
            Ok(evidence)
        })();
        if result.is_err() {
            host.invalidate()?;
        }
        result
    }
}
impl RecordedTemplateOutputEvidence {
    pub fn blockers(&self) -> &[String] {
        &self.blockers
    }
    pub fn validate(
        &self,
        html: &mut HtmlBaseExecutionHost,
        host: &mut RecordedTemplateOutputHost,
        observer: &mut impl StylesheetExecutionObserver,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base: &str,
    ) -> Result<(), String> {
        let result = (|| {
            self.owner.installations.validate_current()?;
            let source = &self.owner.installations.source;
            let cap = &source.preparation.execution.capability;
            for _ in 0..2 {
                self.document.validate(
                    html,
                    cap,
                    &source.snapshot,
                    &mut host.document,
                    observer,
                    &mut matches,
                    base,
                )?;
                self.compiled.validate(
                    &mut host.compiled,
                    &mut LinkedObserver {
                        observations: self.document.execution().observations(),
                        offsets: &self.offsets,
                    },
                )?;
            }
            self.css.validate_sources(cap, &source.snapshot)?;
            self.owner.installations.validate_current()
        })();
        if result.is_err() {
            host.invalidate()?;
        }
        result
    }
}
struct LinkedObserver<'a> {
    observations: &'a [StylesheetObservation],
    offsets: &'a [usize],
}
impl CompiledStylesheetObserver for LinkedObserver<'_> {
    fn environment_revision(&mut self) -> Result<String, String> {
        Ok("native-whole-output".into())
    }
    fn observe(
        &mut self,
        slot: CompiledStylesheetSlot<'_>,
    ) -> Result<CompiledStylesheetObservation, String> {
        let observation = self
            .observations
            .get(
                *self
                    .offsets
                    .get(slot.index)
                    .ok_or("compiled output slot missing")?,
            )
            .ok_or("HTML stylesheet observation missing")?;
        Ok(CompiledStylesheetObservation {
            installation: u32::try_from(
                observation
                    .installation
                    .ok_or("compiled output installation unloaded")?,
            )
            .map_err(|_| "compiled installation identity exceeds bound")?,
            selected: observation.selected,
            source_matches: observation.loaded_source.is_some(),
        })
    }
}
fn element_path(tree: &RetainedCemTree, mut id: AstNodeId) -> Result<Vec<usize>, String> {
    let mut result = Vec::new();
    while id != 0 {
        let parent = tree
            .node(id)
            .and_then(|n| n.parent)
            .ok_or("output parent missing")?;
        let children = element_children(tree, parent)?;
        result.push(
            children
                .iter()
                .position(|child| *child == id)
                .ok_or("output element position missing")?,
        );
        id = parent;
    }
    result.reverse();
    Ok(result)
}
fn element_children(tree: &RetainedCemTree, id: AstNodeId) -> Result<Vec<AstNodeId>, String> {
    Ok(tree
        .node(id)
        .ok_or("native output node missing")?
        .children
        .iter()
        .copied()
        .filter(|id| {
            tree.node(*id)
                .is_some_and(|n| n.kind == CemTreeNodeKind::Element)
        })
        .collect())
}
fn element_at(tree: &RetainedCemTree, path: &[usize]) -> Result<AstNodeId, String> {
    let mut id = 0;
    for index in path {
        id = *element_children(tree, id)?
            .get(*index)
            .ok_or("native output path missing")?;
    }
    Ok(id)
}
fn plan_children<'a>(
    nodes: &'a mut [RenderPlanNode],
    path: &[usize],
) -> Result<&'a mut Vec<RenderPlanNode>, String> {
    let (index, tail) = path
        .split_first()
        .ok_or("element placement path required")?;
    let RenderPlanNode::Element { children, .. } = nodes
        .iter_mut()
        .filter(|n| matches!(n, RenderPlanNode::Element { .. }))
        .nth(*index)
        .ok_or("native render placement missing")?
    else {
        unreachable!()
    };
    if tail.is_empty() {
        Ok(children)
    } else {
        plan_children(children, tail)
    }
}

fn attribute<'a>(tree: &'a RetainedCemTree, id: AstNodeId, name: &str) -> Option<&'a str> {
    tree.node(id)?
        .attributes
        .iter()
        .filter_map(|id| tree.node(*id))
        .find(|a| {
            a.name
                .as_ref()
                .is_some_and(|n| n.namespace_uri.is_empty() && n.local_name == name)
        })
        .map(|a| a.value.as_str())
}
fn plan_attributes<'a>(
    nodes: &'a mut [RenderPlanNode],
    path: &[usize],
) -> Result<&'a mut Vec<render::RenderPlanAttribute>, String> {
    let (index, tail) = path.split_first().ok_or("native host path missing")?;
    let RenderPlanNode::Element {
        children,
        attributes,
        ..
    } = nodes
        .iter_mut()
        .filter(|n| matches!(n, RenderPlanNode::Element { .. }))
        .nth(*index)
        .ok_or("native host plan missing")?
    else {
        unreachable!()
    };
    if tail.is_empty() {
        Ok(attributes)
    } else {
        plan_attributes(children, tail)
    }
}
