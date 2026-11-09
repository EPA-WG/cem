//! Parent-first native first-connection worklist. Quiescence remains partial;
//! passive and explicit resource profiles share one document registry.
#[path = "ssr_declaration.rs"]
mod external_declaration;
#[path = "ssr_readiness.rs"]
mod readiness;
#[path = "ssr_resource.rs"]
pub(crate) mod ssr_resource;
use crate::{
    ssr_island::{prepare_inline_island_with_resource_reads, PreparedInlineIsland},
    ssr_preparation::{
        check, compile_inline_definition, prepare_inline_participant, problem, InlineDefinition,
        InlineParticipantSelection,
    },
    ssr_render::{render_and_apply_inline_dce, AppliedInlineDce},
    ssr_stylesheets::{
        begin_inline_stylesheets_reusing, CoordinatedStylesheets, SsrCssProgress, SsrCssRequest,
        SsrStylesheetCoordinator,
    },
};
use cem_ml::operation_control::cpu::{NativeCpuAccounting, NativeCpuGuard};
use cem_ml::{
    content_cache::ContentHash,
    diagnostics::Diagnostic,
    import::element_tree::{retain_html_elements, RetainedElementImport},
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::{
        tree::{CemDocumentMetadata, CemTreeNodeKind, RetainedCemTree},
        AstNodeId,
    },
    schema::registry::HTML_NAMESPACE_URI,
    ssr_artifact::{CemSsrPageArtifact, SsrLifecycleState, SsrPartialReason},
};
pub(crate) use external_declaration::ExternalDeclarationLease;
pub use readiness::{
    ReadyInlinePage, SsrReadinessContext, SsrReadinessObservation, SSR_READY_RENDER_PLAN,
};
pub use ssr_resource::SsrResourceReader;
pub use ssr_resource::SsrResourceReader as SsrDeclarationReader;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsrPageState {
    Prepared,
    Settling,
    Pending,
    Quiescent,
    Refused,
    Cancelled,
}
#[derive(Debug, Clone)]
pub enum SsrPageProgress {
    Pending(SsrCssRequest),
    Rendered {
        tag: String,
        instance_id: String,
        depth: u32,
    },
    Quiescent,
}
#[derive(Debug)]
pub struct SettledParticipant {
    tag: String,
    instance_id: String,
    depth: u32,
    host: AstNodeId,
    island: AstNodeId,
    start: AstNodeId,
    end: AstNodeId,
    styles: Vec<AstNodeId>,
    pass: Arc<AppliedInlineDce>,
}
impl SettledParticipant {
    pub fn tag(&self) -> &str {
        &self.tag
    }
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }
    pub fn depth(&self) -> u32 {
        self.depth
    }
    pub fn host_node(&self) -> AstNodeId {
        self.host
    }
    pub fn island_node(&self) -> AstNodeId {
        self.island
    }
    pub fn render_pass(&self) -> &AppliedInlineDce {
        &self.pass
    }
}
pub struct QuiescentInlinePage {
    control: OperationControl,
    cpu: Option<Arc<NativeCpuAccounting>>,
    resources: Option<PageResources>,
    definitions: BTreeMap<String, InlineDefinition>,
    declaration_styles: BTreeMap<String, Vec<AstNodeId>>,
    page: CemSsrPageArtifact,
    elements: RetainedElementImport,
    participants: Vec<SettledParticipant>,
}
impl std::fmt::Debug for QuiescentInlinePage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuiescentInlinePage")
            .field("participants", &self.participants.len())
            .finish_non_exhaustive()
    }
}
impl QuiescentInlinePage {
    pub fn page(&self) -> &CemSsrPageArtifact {
        &self.page
    }
    pub fn tree(&self) -> &Arc<RetainedCemTree> {
        &self.elements.tree
    }
    pub fn participants(&self) -> &[SettledParticipant] {
        &self.participants
    }
}
struct PageResources {
    payload_sources: Arc<cem_ml::import::element_tree::RetainedCemPayloadSourcePool>,
    capability: cem_ml::module_resolution::CemModuleUrlResolutionCapability,
    fingerprint: String,
    stale: AtomicBool,
    declarations: BTreeMap<String, Arc<CoordinatedStylesheets>>,
    inputs: BTreeMap<String, ResourceInput>,
}
#[derive(Clone, PartialEq, Eq, serde::Serialize)]
struct ResourceInput {
    resolved_uri: String,
    content: ContentHash,
    #[serde(skip_serializing_if = "Option::is_none")]
    declaration_bytes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload_bytes: Option<usize>,
}
impl ResourceInput {
    fn from_payload(input: &cem_ml::import::element_tree::ExternalCemPayloadInput) -> Self {
        Self {
            resolved_uri: input.uri.clone(),
            content: input.content.clone(),
            declaration_bytes: None,
            payload_bytes: Some(input.byte_length),
        }
    }
}
impl PageResources {
    fn check(&self, control: &OperationControl) -> Result<(), String> {
        if self.capability.cache_identity().as_deref() != Some(self.fingerprint.as_str()) {
            self.stale.store(true, Ordering::Relaxed);
        }
        if self.stale.load(Ordering::Relaxed) {
            return Err("Page resolver authority changed".into());
        }
        for lease in self.declarations.values() {
            lease.check(control)?;
        }
        Ok(())
    }
}
struct PendingParticipant {
    tag: String,
    depth: u32,
    instance_id: String,
    css: SsrStylesheetCoordinator,
}
pub struct InlinePageCoordinator {
    cpu: Option<Arc<NativeCpuAccounting>>,
    declaration_styles: BTreeMap<String, Vec<AstNodeId>>,
    resources: Option<PageResources>,
    pending: Option<PendingParticipant>,
    resource_failure: Option<cem_ml::css_imports::CssImportFailure>,
    page: Option<CemSsrPageArtifact>,
    elements: Option<RetainedElementImport>,
    definitions: BTreeMap<String, InlineDefinition>,
    registry: Arc<BTreeSet<String>>,
    registry_owner: Option<AstNodeId>,
    participants: Vec<SettledParticipant>,
    control: OperationControl,
    prefix: String,
    state: SsrPageState,
    elapsed_limit: u64,
}
impl std::fmt::Debug for InlinePageCoordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InlinePageCoordinator")
            .field("state", &self.state())
            .field("participants", &self.participants.len())
            .finish_non_exhaustive()
    }
}
pub fn prepare_inline_page(
    page: CemSsrPageArtifact,
    prefix: &str,
    control: &OperationControl,
) -> Result<InlinePageCoordinator, Vec<Diagnostic>> {
    prepare_measured(page, prefix, None, None, control)
}
pub fn prepare_inline_page_with_resources(
    page: CemSsrPageArtifact,
    prefix: &str,
    capability: cem_ml::module_resolution::CemModuleUrlResolutionCapability,
    control: &OperationControl,
) -> Result<InlinePageCoordinator, Vec<Diagnostic>> {
    prepare_measured(page, prefix, Some(capability), None, control)
}
/// Prepare the same page registry with a host-owned, bounded declaration reader.
pub fn prepare_inline_page_with_declaration_reads(
    page: CemSsrPageArtifact,
    prefix: &str,
    capability: cem_ml::module_resolution::CemModuleUrlResolutionCapability,
    read: &mut SsrDeclarationReader<'_>,
    control: &OperationControl,
) -> Result<InlinePageCoordinator, Vec<Diagnostic>> {
    prepare_measured(page, prefix, Some(capability), Some(read), control)
}
fn cpu_error(_: String) -> Vec<Diagnostic> {
    vec![Diagnostic {
        code: "cem.ssr.cpu_accounting".into(),
        message: "Original native CPU accounting is unavailable, incomplete or exhausted".into(),
        severity: cem_ml::diagnostics::Severity::Error,
        ..Default::default()
    }]
}
fn prepare_measured(
    page: CemSsrPageArtifact,
    prefix: &str,
    capability: Option<cem_ml::module_resolution::CemModuleUrlResolutionCapability>,
    read: Option<&mut SsrDeclarationReader<'_>>,
    control: &OperationControl,
) -> Result<InlinePageCoordinator, Vec<Diagnostic>> {
    let cpu = if NativeCpuAccounting::supported() {
        Some(Arc::new(
            NativeCpuAccounting::new(control, page.policy().max_cpu_time_ms).map_err(cpu_error)?,
        ))
    } else {
        None
    };
    let guard = cpu
        .as_ref()
        .map(|cpu| cpu.begin(control))
        .transpose()
        .map_err(cpu_error)?;
    let result = capability
        .map(|capability| prepare_resources(&page, capability, control))
        .transpose()
        .and_then(|resources| {
            prepare_inline_page_mode(page, prefix, resources, read, control, cpu)
        });
    let measured = guard
        .map(NativeCpuGuard::finish)
        .transpose()
        .map_err(cpu_error);
    let coordinator = result?;
    measured?;
    coordinator.check(control)?;
    Ok(coordinator)
}
fn prepare_resources(
    page: &CemSsrPageArtifact,
    capability: cem_ml::module_resolution::CemModuleUrlResolutionCapability,
    control: &OperationControl,
) -> Result<PageResources, Vec<Diagnostic>> {
    let fingerprint = capability
        .cache_identity()
        .filter(|id| !id.is_empty() && id.len() <= crate::instance_island::LIMITS.max_bytes)
        .ok_or_else(|| {
            problem(
                page,
                None,
                None,
                "cem.ssr.css_authority_unavailable",
                "Page resources require bounded stable explicit resolver authority",
            )
        })?;
    let payload_sources = Arc::new(
        cem_ml::import::element_tree::RetainedCemPayloadSourcePool::new(control).map_err(|_| {
            problem(
                page,
                None,
                None,
                "cem.ssr.page_bounds",
                "Payload source pool memory refused",
            )
        })?,
    );
    Ok(PageResources {
        payload_sources,
        capability,
        fingerprint,
        stale: AtomicBool::new(false),
        declarations: BTreeMap::new(),
        inputs: BTreeMap::new(),
    })
}
fn prepare_inline_page_mode(
    page: CemSsrPageArtifact,
    prefix: &str,
    mut resources: Option<PageResources>,
    mut read: Option<&mut SsrDeclarationReader<'_>>,
    control: &OperationControl,
    cpu: Option<Arc<NativeCpuAccounting>>,
) -> Result<InlinePageCoordinator, Vec<Diagnostic>> {
    check(&page, control)?;
    let fail = |code, message| problem(&page, None, None, code, message);
    if page.lifecycle() != SsrLifecycleState::Unrendered {
        return Err(fail(
            "cem.ssr.preparation_state",
            "Page coordination requires its unrendered original source",
        ));
    }
    if !matches!(
        page.document(),
        cem_ml::transform_artifact::TransformArtifactBody::HtmlDomProjection(_)
    ) {
        return Err(fail(
            "cem.ssr.preparation_source_unsupported",
            "Page coordination requires its original native HTML projection",
        ));
    }
    if prefix.is_empty()
        || prefix.len() > 200
        || prefix.chars().all(char::is_whitespace)
        || prefix.chars().any(char::is_control)
    {
        return Err(fail(
            "cem.ssr.instance_identity_invalid",
            "Page coordination requires a bounded explicit instance prefix",
        ));
    }
    let workspace = control
        .charge_memory(
            ROOT_EXECUTION_SCOPE_ID,
            page.source_document()
                .source
                .byte_length
                .saturating_mul(128)
                .saturating_add(1024) as u64,
            page.source_map().cloned(),
        )
        .map_err(|_| {
            fail(
                "cem.ssr.page_bounds",
                "Original operation refused page workspace",
            )
        })?;
    let elements = retain_html_elements(
        page.source_document().clone(),
        CemDocumentMetadata {
            base_uri: Some(page.route().document_base_url.clone()),
            document_uri: Some(page.route().page_url.clone()),
        },
    )
    .map_err(|_| {
        fail(
            "cem.ssr.page_import_refused",
            "Native HTML import refused page coordination",
        )
    })?;
    elements.validate_html_text_projection().map_err(|_| {
        fail(
            "cem.ssr.source_projection_unsupported",
            "Page text requires an unsupported decoding profile",
        )
    })?;
    let arena = control
        .charge_memory(
            ROOT_EXECUTION_SCOPE_ID,
            elements.tree.ast().nodes.len().saturating_mul(1024) as u64,
            page.source_map().cloned(),
        )
        .map_err(|_| {
            fail(
                "cem.ssr.page_bounds",
                "Original operation refused retained page arena",
            )
        })?;
    elements
        .tree
        .retain_native_resource(Arc::new(workspace))
        .map_err(|_| {
            fail(
                "cem.ssr.page_bounds",
                "Native page workspace retention refused",
            )
        })?;
    elements
        .tree
        .retain_native_resource(Arc::new(arena))
        .map_err(|_| fail("cem.ssr.page_bounds", "Native page arena retention refused"))?;
    let tree = &elements.tree;
    let mut pending = vec![0];
    let mut definitions = BTreeMap::new();
    let mut registry_owner = None;
    let mut usage = page.usage();
    while let Some(id) = pending.pop() {
        check(&page, control)?;
        let node = tree.node(id).unwrap();
        if html(tree, id, "template") {
            continue;
        }
        if html(tree, id, "cem-element") {
            let parent = tree.source_parent(id).ok_or_else(|| {
                fail(
                    "cem.ssr.registry_scope_unsupported",
                    "Inline declarations require one document body registry",
                )
            })?;
            if !html(tree, parent, "body") || registry_owner.is_some_and(|owner| owner != parent) {
                return Err(fail(
                    "cem.ssr.registry_scope_unsupported",
                    "Scoped or nested declarations require a separately admitted registry",
                ));
            }
            registry_owner = Some(parent);
            if definitions.len() >= 64 {
                return Err(fail(
                    "cem.ssr.page_bounds",
                    "Inline declaration inventory exceeds its bound",
                ));
            }
            let definition = if let Some(src) = attr(tree, id, "src") {
                let (Some(resources), Some(read)) = (resources.as_mut(), read.as_deref_mut())
                else {
                    return Err(problem(
                        &page,
                        Some(tree),
                        Some(id),
                        "cem.ssr.declaration_authority_unavailable",
                        "External declarations require a bounded explicit reader",
                    ));
                };
                if usage.resources >= page.policy().max_resources {
                    return Err(fail(
                        "cem.ssr.settle_budget",
                        "External declaration resource budget exhausted",
                    ));
                }
                let remaining = page.policy().max_total_bytes - usage.total_bytes;
                let definition = external_declaration::read_definition(
                    &page, tree, id, &src, read, remaining, control,
                )?;
                let lease = definition.external.as_ref().unwrap();
                if resources
                    .inputs
                    .insert(lease.uri().into(), lease.input.clone())
                    .is_some()
                {
                    return Err(fail(
                        "cem.ssr.declaration_identity_conflict",
                        "External declaration URLs must be unique in the page registry",
                    ));
                }
                usage.resources += 1;
                usage.total_bytes += lease.bytes() as u64;
                definition
            } else {
                compile_inline_definition(&page, &elements, id, control, resources.is_some())?
            };
            if definition.tag == "cem-element" || definitions.contains_key(&definition.tag) {
                return Err(fail(
                    "cem.ssr.declaration_identity_conflict",
                    "Inline declaration tags must be unique in their native registry",
                ));
            }
            definitions.insert(definition.tag.clone(), definition);
            continue;
        }
        pending.extend(node.children.iter().rev().copied());
    }
    let registry = Arc::new(definitions.keys().cloned().collect());
    let elapsed_limit = if cpu.is_some() {
        page.policy().max_wall_time_ms
    } else {
        page.policy()
            .max_wall_time_ms
            .min(page.policy().max_cpu_time_ms)
    };
    let refused = fail(
        "cem.ssr.settle_budget",
        "Page preparation budget was refused",
    );
    let page = page
        .into_partial(SsrPartialReason::Incomplete, usage, vec![])
        .map_err(|_| refused)?;
    let coordinator = InlinePageCoordinator {
        cpu,
        declaration_styles: BTreeMap::new(),
        resources,
        pending: None,
        resource_failure: None,
        page: Some(page),
        elements: Some(elements),
        definitions,
        registry,
        registry_owner,
        participants: vec![],
        control: control.clone(),
        prefix: prefix.into(),
        state: SsrPageState::Prepared,
        elapsed_limit,
    };
    coordinator.check(control)?;
    coordinator.discover()?;
    Ok(coordinator)
}
impl InlinePageCoordinator {
    pub fn resource_failure(&self) -> Option<&cem_ml::css_imports::CssImportFailure> {
        self.resource_failure.as_ref()
    }
    pub fn state(&self) -> SsrPageState {
        if matches!(self.state, SsrPageState::Refused | SsrPageState::Cancelled) {
            return self.state;
        }
        if self.control.is_cancelled() {
            return SsrPageState::Cancelled;
        }
        if self
            .cpu
            .as_ref()
            .is_some_and(|cpu| cpu.usage_ms(&self.control).is_err())
        {
            return SsrPageState::Refused;
        }

        if self
            .resources
            .as_ref()
            .is_some_and(|r| r.check(&self.control).is_err())
        {
            return SsrPageState::Refused;
        }
        if self.control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_err()
            || self
                .control
                .active_elapsed_ms(ROOT_EXECUTION_SCOPE_ID)
                .is_ok_and(|time| time > self.elapsed_limit)
        {
            return SsrPageState::Refused;
        }
        self.state
    }
    fn fail(&self, code: &str, message: &str, node: Option<u32>) -> Vec<Diagnostic> {
        if let Some(page) = &self.page {
            problem(
                page,
                self.elements.as_ref().map(|e| e.tree.as_ref()),
                node,
                code,
                message,
            )
        } else {
            vec![Diagnostic {
                code: code.into(),
                message: message.into(),
                severity: cem_ml::diagnostics::Severity::Error,
                ..Default::default()
            }]
        }
    }
    fn check(&self, control: &OperationControl) -> Result<(), Vec<Diagnostic>> {
        if !self.control.same_operation(control) {
            return Err(self.fail(
                "cem.ssr.participant_operation_stale",
                "Page coordination belongs to the original operation",
                None,
            ));
        }
        if matches!(self.state, SsrPageState::Refused | SsrPageState::Cancelled) {
            return Err(vec![Diagnostic {
                code: "cem.ssr.page_terminal".into(),
                message: "Page coordination is already terminal".into(),
                severity: cem_ml::diagnostics::Severity::Error,
                ..Default::default()
            }]);
        }
        check(self.page.as_ref().unwrap(), control)?;
        if let Some(cpu) = &self.cpu {
            cpu.usage_ms(control).map_err(cpu_error)?;
        }
        if let Some(resources) = &self.resources {
            resources.check(control).map_err(|_| {
                self.fail(
                    "cem.ssr.css_closure_stale",
                    "Page resource leases are no longer current",
                    None,
                )
            })?;
        }

        if control
            .active_elapsed_ms(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| {
                self.fail(
                    "cem.ssr.settle_budget",
                    "Original elapsed accounting refused page work",
                    None,
                )
            })?
            > self.elapsed_limit
        {
            return Err(self.fail(
                "cem.ssr.settle_budget",
                "Original elapsed budget exhausted page coordination",
                None,
            ));
        }
        Ok(())
    }
    fn stop(&mut self) {
        self.state = if self.control.is_cancelled() {
            SsrPageState::Cancelled
        } else {
            SsrPageState::Refused
        };
        if self.resource_failure.is_none() {
            self.resource_failure = self
                .pending
                .as_ref()
                .and_then(|pending| pending.css.failure())
                .cloned();
        }
        self.pending = None;
        self.resources = None;
        self.declaration_styles.clear();
        self.participants.clear();
        self.definitions.clear();
        self.elements = None;
        self.page = None;
    }
    fn discover(&self) -> Result<Option<(u32, String, u32)>, Vec<Diagnostic>> {
        let page = self.page.as_ref().unwrap();
        let tree = &self.elements.as_ref().unwrap().tree;
        let completed = self
            .participants
            .iter()
            .map(|p| (p.host, p))
            .collect::<BTreeMap<_, _>>();
        let mut pending = vec![(0, 0u32, false)];
        let mut first = None;
        let mut count = 0usize;
        while let Some((id, depth, visible)) = pending.pop() {
            check(page, &self.control)?;
            let n = tree.node(id).unwrap();
            let visible = visible || self.registry_owner == Some(id);
            if n.attributes.iter().any(|id| {
                tree.node(*id)
                    .unwrap()
                    .name
                    .as_ref()
                    .is_some_and(|name| name.namespace_uri.is_empty() && name.local_name == "is")
            }) {
                return Err(self.fail(
                    "cem.ssr.participant_unavailable",
                    "Customized built-in lifecycle participants are not admitted",
                    Some(id),
                ));
            }
            if html(tree, id, "template") || html(tree, id, "cem-element") {
                continue;
            }
            let mut child_depth = depth;
            if n.name.as_ref().is_some_and(|name| {
                name.namespace_uri == HTML_NAMESPACE_URI && name.local_name.contains('-')
            }) {
                let tag = n.name.as_ref().unwrap().local_name.clone();
                if !visible || !self.registry.contains(&tag) {
                    return Err(self.fail(
                        "cem.ssr.participant_unavailable",
                        "Active participant has no declaration in this native registry",
                        Some(id),
                    ));
                }
                child_depth = depth.checked_add(1).ok_or_else(|| {
                    self.fail(
                        "cem.ssr.settle_budget",
                        "Participant depth overflow",
                        Some(id),
                    )
                })?;
                count += 1;
                if count > 64
                    || count as u32 > page.policy().max_instances
                    || child_depth > page.policy().max_declaration_depth
                {
                    return Err(self.fail(
                        "cem.ssr.settle_budget",
                        "Active participant inventory exceeds original instance/depth ceilings",
                        Some(id),
                    ));
                }
                if let Some(participant) = completed.get(&id) {
                    let children = &n.children;
                    if children.first() != Some(&participant.island)
                        || children.get(1) != Some(&participant.start)
                        || children.get(children.len().saturating_sub(participant.styles.len() + 1))
                            != Some(&participant.end)
                        || !children.ends_with(&participant.styles)
                        || tree.node(participant.start).is_none_or(|n| {
                            n.kind != CemTreeNodeKind::Comment || n.value != "cem-render-start"
                        })
                        || tree.node(participant.end).is_none_or(|n| {
                            n.kind != CemTreeNodeKind::Comment || n.value != "cem-render-end"
                        })
                    {
                        return Err(self.fail(
                            "cem.ssr.participant_ownership_lost",
                            "A staged participant lost its native island or output range",
                            Some(id),
                        ));
                    }
                } else {
                    if n.children.iter().any(|id| {
                        html(tree, *id, "template")
                            && attr(tree, *id, "data-cem-island").as_deref() == Some("instance")
                    }) {
                        return Err(self.fail("cem.ssr.resume_unavailable","Authored/generated markers cannot stand in for a completed native participant",Some(id)));
                    }
                    if first.is_none() {
                        first = Some((id, tag, child_depth));
                    }
                }
            }
            pending.extend(
                n.children
                    .iter()
                    .rev()
                    .map(|id| (*id, child_depth, visible)),
            );
        }
        if completed.len() != self.participants.len()
            || self.participants.iter().any(|p| {
                tree.node(p.host)
                    .is_none_or(|n| n.name.as_ref().is_none_or(|name| name.local_name != p.tag))
            })
        {
            return Err(self.fail(
                "cem.ssr.participant_ownership_lost",
                "A completed native host disappeared or changed identity",
                None,
            ));
        }
        Ok(first)
    }
    pub fn advance(
        &mut self,
        control: &OperationControl,
    ) -> Result<SsrPageProgress, Vec<Diagnostic>> {
        self.advance_measured(control, None)
    }

    /// Resolve external payloads during this participant's original CPU phase.
    pub fn advance_with_resource_reads(
        &mut self,
        control: &OperationControl,
        read: &mut SsrResourceReader<'_>,
    ) -> Result<SsrPageProgress, Vec<Diagnostic>> {
        self.advance_measured(control, Some(read))
    }

    fn advance_measured(
        &mut self,
        control: &OperationControl,
        read: Option<&mut SsrResourceReader<'_>>,
    ) -> Result<SsrPageProgress, Vec<Diagnostic>> {
        if !self.control.same_operation(control) {
            return Err(self.fail(
                "cem.ssr.participant_operation_stale",
                "Page coordination belongs to the original operation",
                None,
            ));
        }
        let guard = match self.begin_cpu(control) {
            Ok(guard) => guard,
            Err(error) => {
                self.stop();
                return Err(error);
            }
        };
        let result = self.advance_inner(control, read);
        let measured = guard
            .map(NativeCpuGuard::finish)
            .transpose()
            .map_err(cpu_error);
        let result = result.and_then(|value| measured.map(|_| value));
        if result.is_err() {
            self.stop();
        }
        result
    }
    fn advance_inner(
        &mut self,
        control: &OperationControl,
        read: Option<&mut SsrResourceReader<'_>>,
    ) -> Result<SsrPageProgress, Vec<Diagnostic>> {
        self.check(control)?;
        if self.pending.is_some() {
            return self.advance_resources(control);
        }
        let Some((instance, tag, depth)) = self.discover()? else {
            self.state = SsrPageState::Quiescent;
            return Ok(SsrPageProgress::Quiescent);
        };
        let page = self.page.as_ref().unwrap();
        let tree = &self.elements.as_ref().unwrap().tree;
        if page.usage().render_passes >= page.policy().max_render_passes
            || page.usage().instances >= page.policy().max_instances
        {
            return Err(self.fail(
                "cem.ssr.settle_budget",
                "No original render/instance budget remains",
                Some(instance),
            ));
        }
        // The ordinary profile remains passive; the explicit resource profile
        // owns its shared source leases, stylesheet batches and pending tickets.
        if self.resources.is_none()
            && tree.node(instance).unwrap().children.iter().any(|id| {
                html(tree, *id, "template")
                    && attr(tree, *id, "type").is_some_and(|value| value != "text/html")
            })
        {
            return Err(self.fail(
                "cem.ssr.page_payload_profile",
                "This page worklist admits literal HTML payload envelopes",
                Some(instance),
            ));
        }
        let definition = self.definitions.get(&tag).unwrap().clone();
        let prepared = prepare_inline_participant(
            page.clone(),
            InlineParticipantSelection {
                elements: self.elements.as_ref().unwrap().clone(),
                definition,
                instance,
                depth,
                registry: self.registry.clone(),
                payload_sources: self.resources.as_ref().map(|r| r.payload_sources.clone()),
                cpu: self.cpu.clone(),
            },
            control,
        )?;
        let instance_id = format!("{}-{}", self.prefix, self.participants.len() + 1);
        let island =
            prepare_inline_island_with_resource_reads(prepared, &instance_id, read, control)?;
        if let Some(input) = island
            .bindings()
            .native_payload_source
            .as_ref()
            .and_then(|source| source.external_input())
        {
            let resources = self
                .resources
                .as_mut()
                .expect("External payload requires page resource authority");
            let witness = ResourceInput::from_payload(input);
            if resources
                .inputs
                .get(&input.uri)
                .is_some_and(|previous| previous != &witness)
                || resources
                    .inputs
                    .values()
                    .any(|previous| previous.resolved_uri == input.uri && previous != &witness)
            {
                return Err(self.fail(
                    "cem.ssr.resource_changed",
                    "Payload response changed within the original page operation",
                    Some(instance),
                ));
            }
            resources.inputs.insert(input.uri.clone(), witness);
        }
        if self.resources.is_none()
            && (island.bindings().native_payload_source.is_some()
                || !island.instance_stylesheets().is_empty())
        {
            return Err(self.fail(
                "cem.ssr.page_payload_profile",
                "Cross-participant source/CSS leases require further admission",
                Some(instance),
            ));
        }
        if let Some(resources) = &self.resources {
            let css = begin_inline_stylesheets_reusing(
                island,
                resources.capability.clone(),
                resources.declarations.get(&tag).cloned(),
                control,
            )
            .map_err(|_| {
                self.fail(
                    "cem.ssr.css_closure_refused",
                    "Native page stylesheet coordination refused this participant",
                    Some(instance),
                )
            })?;
            self.pending = Some(PendingParticipant {
                tag,
                depth,
                instance_id,
                css,
            });
            return self.advance_resources(control);
        }
        self.apply_participant(island, tag, instance_id, depth, control)
    }
    fn advance_resources(
        &mut self,
        control: &OperationControl,
    ) -> Result<SsrPageProgress, Vec<Diagnostic>> {
        let progress = self
            .pending
            .as_mut()
            .unwrap()
            .css
            .advance(control)
            .map_err(|_| {
                self.fail(
                    "cem.ssr.css_closure_refused",
                    "Native page resource work refused",
                    None,
                )
            })?;
        match progress {
            SsrCssProgress::Pending(request) => {
                self.state = SsrPageState::Pending;
                Ok(SsrPageProgress::Pending(request))
            }
            SsrCssProgress::Ready => {
                let pending = self.pending.take().unwrap();
                let island = pending.css.finish(control).map_err(|_| {
                    self.fail(
                        "cem.ssr.css_closure_refused",
                        "Native page resource closure refused",
                        None,
                    )
                })?;
                self.apply_participant(
                    island,
                    pending.tag,
                    pending.instance_id,
                    pending.depth,
                    control,
                )
            }
        }
    }
    /// Remaining bytes on the current ticket, before a synchronous host read.
    pub fn resource_read_limit(
        &self,
        request: &SsrCssRequest,
        control: &OperationControl,
    ) -> Result<usize, Vec<Diagnostic>> {
        self.check(control)?;
        let pending = self.pending.as_ref().ok_or_else(|| {
            self.fail(
                "cem.ssr.resource_ticket_stale",
                "Page has no outstanding resource request",
                None,
            )
        })?;
        pending.css.match_request(request).map_err(|_| {
            self.fail(
                "cem.ssr.resource_ticket_stale",
                "Resource ticket is stale or foreign",
                None,
            )
        })?;
        pending.css.remaining_response_bytes().map_err(|_| {
            self.fail(
                "cem.ssr.settle_budget",
                "Page resource byte budget exhausted",
                None,
            )
        })
    }

    pub fn deliver(
        &mut self,
        request: &SsrCssRequest,
        response: cem_ml::resolver::ResolvedRead,
        control: &OperationControl,
    ) -> Result<(), Vec<Diagnostic>> {
        if !self.control.same_operation(control) {
            return Err(self.fail(
                "cem.ssr.participant_operation_stale",
                "Page delivery belongs to the original operation",
                None,
            ));
        }
        let Some(pending) = self.pending.as_mut() else {
            return Err(self.fail(
                "cem.ssr.resource_ticket_stale",
                "Page has no outstanding resource request",
                None,
            ));
        };
        // Foreign/stale tickets cannot retire another participant's current work.
        pending.css.match_request(request).map_err(|_| {
            self.fail(
                "cem.ssr.resource_ticket_stale",
                "Resource ticket is stale or foreign",
                None,
            )
        })?;
        let guard = match self.begin_cpu(control) {
            Ok(guard) => guard,
            Err(error) => {
                self.stop();
                return Err(error);
            }
        };
        let result = (|| {
            let limit = self.resource_read_limit(request, control)?;
            let uri = &request.request().resolution.resolved_url;
            if response.bytes.len() > limit || uri.len() > 4096 || response.uri.len() > 4096 {
                return Err(self.fail(
                    "cem.ssr.settle_budget",
                    "Page resource response exceeds native bounds",
                    None,
                ));
            }
            let witness = ResourceInput {
                resolved_uri: response.uri.clone(),
                content: ContentHash::from_blake3(&response.bytes),
                declaration_bytes: None,
                payload_bytes: None,
            };
            let resources = self.resources.as_ref().unwrap();
            if resources
                .inputs
                .get(uri)
                .is_some_and(|previous| previous != &witness)
                || resources.inputs.values().any(|previous| {
                    previous.resolved_uri == witness.resolved_uri && previous != &witness
                })
            {
                return Err(self.fail(
                    "cem.ssr.resource_changed",
                    "Stylesheet response changed within the original page operation",
                    None,
                ));
            }
            self.pending
                .as_mut()
                .unwrap()
                .css
                .deliver(request, response, control)
                .map_err(|_| {
                    self.fail(
                        "cem.ssr.css_closure_refused",
                        "Native page resource response refused",
                        None,
                    )
                })?;
            self.resources
                .as_mut()
                .unwrap()
                .inputs
                .insert(uri.clone(), witness);
            Ok(())
        })();
        let measured = guard
            .map(NativeCpuGuard::finish)
            .transpose()
            .map_err(cpu_error);
        let result = result.and_then(|_| measured.map(|_| ()));
        if result.is_err() {
            self.stop();
        }
        result
    }
    pub fn fail_resource(
        &mut self,
        request: &SsrCssRequest,
        message: &str,
        control: &OperationControl,
    ) -> Result<(), Vec<Diagnostic>> {
        if !self.control.same_operation(control) {
            return Err(self.fail(
                "cem.ssr.participant_operation_stale",
                "Page failure belongs to the original operation",
                None,
            ));
        }
        let Some(pending) = self.pending.as_mut() else {
            return Err(self.fail(
                "cem.ssr.resource_ticket_stale",
                "Page has no outstanding resource request",
                None,
            ));
        };
        pending.css.match_request(request).map_err(|_| {
            self.fail(
                "cem.ssr.resource_ticket_stale",
                "Resource ticket is stale or foreign",
                None,
            )
        })?;
        let guard = match self.begin_cpu(control) {
            Ok(guard) => guard,
            Err(error) => {
                self.stop();
                return Err(error);
            }
        };
        let result = self
            .pending
            .as_mut()
            .unwrap()
            .css
            .fail(request, message, control)
            .map_err(|_| {
                self.fail(
                    "cem.ssr.css_transport_failed",
                    "Page resource transport failed",
                    None,
                )
            });
        let measured = guard
            .map(NativeCpuGuard::finish)
            .transpose()
            .map_err(cpu_error);
        let result = result.and_then(|_| measured.map(|_| ()));
        self.stop();
        result
    }
    fn apply_participant(
        &mut self,
        island: PreparedInlineIsland,
        tag: String,
        instance_id: String,
        depth: u32,
        control: &OperationControl,
    ) -> Result<SsrPageProgress, Vec<Diagnostic>> {
        let instance = island.prepared().instance_node();
        let lease = island.coordinated_css.clone();
        let pass = Arc::new(render_and_apply_inline_dce(island, control)?);
        let application = pass.application();
        let elements = self
            .elements
            .as_ref()
            .unwrap()
            .after_application(application)
            .map_err(|_| {
                self.fail(
                    "cem.ssr.participant_source_stale",
                    "Native source handles refused application continuation",
                    Some(instance),
                )
            })?;
        for participant in &mut self.participants {
            for id in [
                &mut participant.host,
                &mut participant.island,
                &mut participant.start,
                &mut participant.end,
            ]
            .into_iter()
            .chain(participant.styles.iter_mut())
            {
                *id = application.output_for_source(*id).ok_or_else(|| {
                    vec![Diagnostic {
                        code: "cem.ssr.participant_ownership_lost".into(),
                        message: "An existing participant was replaced by later work".into(),
                        severity: cem_ml::diagnostics::Severity::Error,
                        ..Default::default()
                    }]
                })?;
            }
        }
        let missing = || {
            vec![Diagnostic {
                code: "cem.ssr.participant_ownership_lost".into(),
                message: "A native declaration or registry owner disappeared during application"
                    .into(),
                severity: cem_ml::diagnostics::Severity::Error,
                ..Default::default()
            }]
        };
        for ids in self.declaration_styles.values_mut() {
            for id in ids {
                *id = application.output_for_source(*id).ok_or_else(missing)?;
            }
        }
        let declaration = self.definitions.get(&tag).unwrap().declaration;
        let count = pass.input().declaration_stylesheets().len();
        if count > 0 && !self.declaration_styles.contains_key(&tag) {
            let target = application
                .output_for_source(declaration)
                .ok_or_else(missing)?;
            let children = &application.tree().node(target).unwrap().children;
            self.declaration_styles
                .insert(tag.clone(), children[children.len() - count..].to_vec());
        }
        for definition in self.definitions.values_mut() {
            definition.declaration = application
                .output_for_source(definition.declaration)
                .ok_or_else(missing)?;
            definition.template = application
                .output_for_source(definition.template)
                .ok_or_else(missing)?;
        }
        if let Some(owner) = self.registry_owner {
            self.registry_owner = Some(application.output_for_source(owner).ok_or_else(missing)?);
        }
        let children = application.inserted_children();
        let end_index = children.len() - pass.input().instance_stylesheets().len() - 1;
        self.participants.push(SettledParticipant {
            tag: tag.clone(),
            instance_id: instance_id.clone(),
            depth,
            host: application.output_target(),
            island: children[0],
            start: children[1],
            end: children[end_index],
            styles: children[end_index + 1..].to_vec(),
            pass: pass.clone(),
        });
        if let (Some(resources), Some(lease)) = (&mut self.resources, lease) {
            resources.declarations.entry(tag.clone()).or_insert(lease);
        }
        self.page = Some(pass.page().clone());
        self.elements = Some(elements);
        self.state = SsrPageState::Settling;
        self.check(control)?;
        self.discover()?;
        Ok(SsrPageProgress::Rendered {
            tag,
            instance_id,
            depth,
        })
    }
    fn begin_cpu(
        &self,
        control: &OperationControl,
    ) -> Result<Option<NativeCpuGuard>, Vec<Diagnostic>> {
        self.cpu
            .as_ref()
            .map(|cpu| cpu.begin(control))
            .transpose()
            .map_err(cpu_error)
    }
    pub fn finish(
        self,
        control: &OperationControl,
    ) -> Result<QuiescentInlinePage, Vec<Diagnostic>> {
        let guard = self.begin_cpu(control)?;
        let result = self.finish_inner(control);
        let measured = guard
            .map(NativeCpuGuard::finish)
            .transpose()
            .map_err(cpu_error);
        let page = result?;
        measured?;
        Ok(page)
    }
    fn finish_inner(
        mut self,
        control: &OperationControl,
    ) -> Result<QuiescentInlinePage, Vec<Diagnostic>> {
        self.check(control)?;
        if self.pending.is_some() || self.discover()?.is_some() {
            return Err(self.fail(
                "cem.ssr.page_pending",
                "Page coordination still has undispatched native participants",
                None,
            ));
        }
        let page = self.page.take().unwrap();
        let mut usage = page.usage();
        usage.wall_time_ms =
            usage
                .wall_time_ms
                .max(
                    control
                        .active_elapsed_ms(ROOT_EXECUTION_SCOPE_ID)
                        .map_err(|_| {
                            self.fail(
                                "cem.ssr.settle_budget",
                                "Final elapsed observation refused page coordination",
                                None,
                            )
                        })?,
                );
        if let Some(cpu) = &self.cpu {
            usage.cpu_time_ms = cpu.usage_ms(control).map_err(cpu_error)?;
        }
        let diagnostics = self
            .participants
            .last()
            .map_or_else(Vec::new, |participant| {
                participant.pass.plan().diagnostics.clone()
            });
        let page = page
            .into_partial(SsrPartialReason::Incomplete, usage, diagnostics)
            .map_err(|_| {
                self.fail(
                    "cem.ssr.settle_budget",
                    "Final page usage exceeds the original policy",
                    None,
                )
            })?;
        Ok(QuiescentInlinePage {
            control: self.control.clone(),
            cpu: self.cpu.clone(),
            resources: self.resources.take(),
            definitions: std::mem::take(&mut self.definitions),
            declaration_styles: std::mem::take(&mut self.declaration_styles),
            page,
            elements: self.elements.take().unwrap(),
            participants: std::mem::take(&mut self.participants),
        })
    }
}
fn html(tree: &RetainedCemTree, id: u32, tag: &str) -> bool {
    tree.node(id)
        .and_then(|n| n.name.as_ref())
        .is_some_and(|name| name.namespace_uri == HTML_NAMESPACE_URI && name.local_name == tag)
}
fn attr(tree: &RetainedCemTree, id: u32, name: &str) -> Option<String> {
    tree.node(id)?.attributes.iter().find_map(|id| {
        let n = tree.node(*id)?;
        n.name
            .as_ref()
            .filter(|n| n.namespace_uri.is_empty() && n.local_name == name)
            .map(|_| n.value.clone())
    })
}
