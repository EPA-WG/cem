//! Parent-first native first-connection worklist. Quiescence remains partial;
//! this profile admits passive inline participants in one document registry.
use crate::{
    ssr_island::prepare_inline_island,
    ssr_preparation::{
        check, compile_inline_definition, prepare_inline_participant, problem, InlineDefinition,
        InlineParticipantSelection,
    },
    ssr_render::{render_and_apply_inline_dce, AppliedInlineDce},
};
use cem_ml::{
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
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsrPageState {
    Prepared,
    Settling,
    Quiescent,
    Refused,
    Cancelled,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SsrPageProgress {
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
#[derive(Debug)]
pub struct QuiescentInlinePage {
    page: CemSsrPageArtifact,
    elements: RetainedElementImport,
    participants: Vec<SettledParticipant>,
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
pub struct InlinePageCoordinator {
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
            let definition = compile_inline_definition(&page, &elements, id, control)?;
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
    let elapsed_limit = page
        .policy()
        .max_wall_time_ms
        .min(page.policy().max_cpu_time_ms);
    let usage = page.usage();
    let refused = fail(
        "cem.ssr.settle_budget",
        "Page preparation budget was refused",
    );
    let page = page
        .into_partial(SsrPartialReason::Incomplete, usage, vec![])
        .map_err(|_| refused)?;
    let coordinator = InlinePageCoordinator {
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
    pub fn state(&self) -> SsrPageState {
        if matches!(self.state, SsrPageState::Refused | SsrPageState::Cancelled) {
            return self.state;
        }
        if self.control.is_cancelled() {
            return SsrPageState::Cancelled;
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
                        || children.last() != Some(&participant.end)
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
        if !self.control.same_operation(control) {
            return Err(self.fail(
                "cem.ssr.participant_operation_stale",
                "Page coordination belongs to the original operation",
                None,
            ));
        }
        let result = self.advance_inner(control);
        if result.is_err() {
            self.stop();
        }
        result
    }
    fn advance_inner(
        &mut self,
        control: &OperationControl,
    ) -> Result<SsrPageProgress, Vec<Diagnostic>> {
        self.check(control)?;
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
        // Source-code and stylesheet owners need native shared leases before
        // their admission can widen from the one-participant coordinator.
        if tree.node(instance).unwrap().children.iter().any(|id| {
            html(tree, *id, "template")
                && attr(tree, *id, "type").is_some_and(|value| value != "text/html")
        }) {
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
            },
            control,
        )?;
        let instance_id = format!("{}-{}", self.prefix, self.participants.len() + 1);
        let island = prepare_inline_island(prepared, &instance_id, control)?;
        if island.bindings().native_payload_source.is_some()
            || !island.instance_stylesheets().is_empty()
        {
            return Err(self.fail(
                "cem.ssr.page_payload_profile",
                "Cross-participant source/CSS leases require further admission",
                Some(instance),
            ));
        }
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
            ] {
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
        self.participants.push(SettledParticipant {
            tag: tag.clone(),
            instance_id: instance_id.clone(),
            depth,
            host: application.output_target(),
            island: children[0],
            start: children[1],
            end: *children.last().unwrap(),
            pass: pass.clone(),
        });
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
    pub fn finish(
        mut self,
        control: &OperationControl,
    ) -> Result<QuiescentInlinePage, Vec<Diagnostic>> {
        self.check(control)?;
        if self.discover()?.is_some() {
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
