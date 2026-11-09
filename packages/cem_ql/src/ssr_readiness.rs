//! Nonportable native readiness. A diagnostic/cache identity is never a proof.
use super::*;
use crate::{
    eval::{Item, QueryContextScope},
    render::{RenderPlan, RenderPlanNode},
};
use cem_ml::{
    content_cache::ContentHash,
    ssr_artifact::{SsrArtifactRoute, SsrExecutionUsage, SsrSettlePolicy},
};
pub const SSR_READY_RENDER_PLAN: &str = "cem-native-render-plan/1";
#[derive(Debug, Clone)]
pub struct SsrReadinessContext {
    input: ContentHash,
    route: SsrArtifactRoute,
    policy: SsrSettlePolicy,
    versions: BTreeMap<String, String>,
}
impl SsrReadinessContext {
    pub fn new(
        page: &CemSsrPageArtifact,
        versions: BTreeMap<String, String>,
    ) -> Result<Self, Vec<Diagnostic>> {
        if versions.len() > 64
            || versions
                .iter()
                .any(|(tag, version)| tag.len() > 256 || version.len() > 1024)
        {
            return Err(refused("context_bounds"));
        }
        Ok(Self {
            input: page
                .render_input_identity()
                .map_err(|_| refused("identity_incomplete"))?,
            route: page.route().clone(),
            policy: page.policy().clone(),
            versions,
        })
    }
}
pub struct ReadyInlinePage {
    settled: QuiescentInlinePage,
    context: SsrReadinessContext,
    render_input: ContentHash,
}
impl std::fmt::Debug for ReadyInlinePage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadyInlinePage")
            .field("participants", &self.settled.participants.len())
            .finish_non_exhaustive()
    }
}
#[derive(Debug)]
pub struct SsrReadinessObservation {
    usage: SsrExecutionUsage,
}
impl SsrReadinessObservation {
    pub fn usage(&self) -> SsrExecutionUsage {
        self.usage
    }
}
impl QuiescentInlinePage {
    pub fn prove_ready(
        self,
        context: &SsrReadinessContext,
        control: &OperationControl,
    ) -> Result<ReadyInlinePage, Vec<Diagnostic>> {
        let cpu = self
            .cpu
            .as_ref()
            .ok_or_else(|| refused("cpu_unavailable"))?;
        let guard = cpu.begin(control).map_err(cpu_error)?;
        let result = validate(&self, &self.elements.tree, context, control)
            .and_then(|_| render_input(&self, context));
        let measured = guard.finish().map_err(cpu_error);
        let render_input = result?;
        measured?;
        cpu.usage_ms(control).map_err(cpu_error)?;
        Ok(ReadyInlinePage {
            settled: self,
            context: context.clone(),
            render_input,
        })
    }
}
impl ReadyInlinePage {
    pub fn page(&self) -> &CemSsrPageArtifact {
        &self.settled.page
    }
    pub fn tree(&self) -> &Arc<RetainedCemTree> {
        &self.settled.elements.tree
    }
    pub fn participants(&self) -> &[SettledParticipant] {
        &self.settled.participants
    }
    /// Final deterministic input identity including the closed resource reads.
    pub fn render_input_identity(&self) -> &ContentHash {
        &self.render_input
    }
    pub fn validate_current(
        &self,
        tree: &Arc<RetainedCemTree>,
        context: &SsrReadinessContext,
        control: &OperationControl,
    ) -> Result<SsrReadinessObservation, Vec<Diagnostic>> {
        let cpu = self
            .settled
            .cpu
            .as_ref()
            .ok_or_else(|| refused("cpu_unavailable"))?;
        let guard = cpu.begin(control).map_err(cpu_error)?;
        let result = (|| {
            if context.input != self.context.input
                || context.route != self.context.route
                || context.policy != self.context.policy
            {
                return Err(refused("context_stale"));
            }
            validate(&self.settled, tree, context, control)?;
            if render_input(&self.settled, context)? != self.render_input {
                return Err(refused("resource_identity"));
            }
            Ok(())
        })();
        let measured = guard.finish().map_err(cpu_error);
        result?;
        measured?;
        let mut usage = self.settled.page.usage();
        usage.cpu_time_ms = cpu.usage_ms(control).map_err(cpu_error)?;
        usage.wall_time_ms = control
            .active_elapsed_ms(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| refused("wall_time"))?;
        if usage.wall_time_ms > context.policy.max_wall_time_ms
            || usage.cpu_time_ms > context.policy.max_cpu_time_ms
        {
            return Err(refused("budget"));
        }
        Ok(SsrReadinessObservation { usage })
    }
}
fn render_input(
    settled: &QuiescentInlinePage,
    context: &SsrReadinessContext,
) -> Result<ContentHash, Vec<Diagnostic>> {
    let Some(resources) = settled
        .resources
        .as_ref()
        .filter(|resources| !resources.inputs.is_empty())
    else {
        return Ok(context.input.clone());
    };
    let metadata = serde_json::to_vec(&(
        "cem SSR resource inputs/1",
        &context.input,
        &resources.fingerprint,
        &resources.inputs,
    ))
    .map_err(|_| refused("resource_identity"))?;
    Ok(ContentHash::from_blake3(&metadata))
}
fn refused(reason: &str) -> Vec<Diagnostic> {
    vec![Diagnostic {
        code: format!("cem.ssr.readiness_{reason}"),
        message: format!("Native SSR readiness refused: {reason}"),
        severity: cem_ml::diagnostics::Severity::Error,
        ..Default::default()
    }]
}
fn compatible(a: &str, b: &str) -> bool {
    crate::ssr_browser_resume::versions_compatible(a, b)
}

struct Work<'a> {
    remaining: usize,
    cpu: &'a NativeCpuAccounting,
    control: &'a OperationControl,
}
impl Work<'_> {
    fn step(&mut self, depth: usize) -> Result<(), Vec<Diagnostic>> {
        if depth > 64 || self.remaining == 0 {
            return Err(refused("bounds"));
        }
        self.remaining -= 1;
        self.cpu.usage_ms(self.control).map_err(cpu_error)?;
        Ok(())
    }
}
fn attrs(
    tree: &RetainedCemTree,
    id: u32,
) -> Result<BTreeMap<(String, String), String>, Vec<Diagnostic>> {
    let mut values = BTreeMap::new();
    for id in &tree
        .node(id)
        .ok_or_else(|| refused("node_missing"))?
        .attributes
    {
        let a = tree.node(*id).ok_or_else(|| refused("attribute_missing"))?;
        let n = a.name.as_ref().ok_or_else(|| refused("attribute_name"))?;
        if values
            .insert(
                (n.namespace_uri.clone(), n.local_name.clone()),
                a.value.clone(),
            )
            .is_some()
        {
            return Err(refused("duplicate_attribute"));
        }
    }
    Ok(values)
}
fn literal(
    node: &RenderPlanNode,
    tree: &RetainedCemTree,
    id: u32,
    work: &mut Work<'_>,
    depth: usize,
) -> Result<(), Vec<Diagnostic>> {
    work.step(depth)?;
    let actual = tree.node(id).ok_or_else(|| refused("node_missing"))?;
    use cem_ml::value::CemValueNode as V;
    match node {
        V::Element {
            tag,
            namespace,
            attributes,
            children,
            ..
        } => {
            if actual.kind != CemTreeNodeKind::Element
                || actual.name.as_ref().is_none_or(|n| {
                    n.local_name != *tag
                        || n.namespace_uri != namespace.as_deref().unwrap_or_default()
                })
            {
                return Err(refused("element"));
            }
            let expected = attributes
                .iter()
                .map(|a| {
                    (
                        (a.namespace.clone().unwrap_or_default(), a.name.clone()),
                        a.value.clone(),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            if expected.len() != attributes.len()
                || attrs(tree, id)? != expected
                || actual.children.len() != children.len()
            {
                return Err(refused("literal_content"));
            }
            for (expected, actual) in children.iter().zip(&actual.children) {
                literal(expected, tree, *actual, work, depth + 1)?;
            }
        }
        V::Text { text, .. } | V::Cdata { text, .. }
            if actual.kind == CemTreeNodeKind::Text && actual.value == *text => {}
        V::Comment { text, .. }
            if actual.kind == CemTreeNodeKind::Comment && actual.value == *text => {}
        _ => return Err(refused("literal_content")),
    }
    Ok(())
}
fn same_tree(
    a: &RetainedCemTree,
    ai: u32,
    b: &RetainedCemTree,
    bi: u32,
    work: &mut Work<'_>,
    depth: usize,
) -> Result<(), Vec<Diagnostic>> {
    work.step(depth)?;
    let an = a.node(ai).ok_or_else(|| refused("node_missing"))?;
    let bn = b.node(bi).ok_or_else(|| refused("node_missing"))?;
    if an.kind != bn.kind
        || an.value != bn.value
        || an.name.as_ref().map(|n| (&n.namespace_uri, &n.local_name))
            != bn.name.as_ref().map(|n| (&n.namespace_uri, &n.local_name))
        || attrs(a, ai)? != attrs(b, bi)?
        || an.children.len() != bn.children.len()
    {
        return Err(refused("source_changed"));
    }
    for (x, y) in an.children.iter().zip(&bn.children) {
        same_tree(a, *x, b, *y, work, depth + 1)?;
    }
    Ok(())
}
fn rendered(
    node: &RenderPlanNode,
    tree: &RetainedCemTree,
    id: u32,
    participants: &BTreeMap<u32, &SettledParticipant>,
    work: &mut Work<'_>,
    depth: usize,
) -> Result<(), Vec<Diagnostic>> {
    work.step(depth)?;
    if let Some(child) = participants.get(&id) {
        // Prove the exact pre-child node produced by its parent's plan; the
        // child's own final output, canonical island and writes are checked separately.
        return literal(
            node,
            &child.pass.input().prepared().elements().tree,
            child.pass.input().prepared().instance_node(),
            work,
            depth,
        );
    }
    let actual = tree.node(id).ok_or_else(|| refused("node_missing"))?;
    if let cem_ml::value::CemValueNode::Element {
        tag,
        namespace,
        attributes,
        children,
        ..
    } = node
    {
        let expected = attributes
            .iter()
            .map(|a| {
                (
                    (a.namespace.clone().unwrap_or_default(), a.name.clone()),
                    a.value.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        if actual.name.as_ref().is_none_or(|n| {
            n.local_name != *tag || n.namespace_uri != namespace.as_deref().unwrap_or_default()
        }) || attrs(tree, id)? != expected
            || actual.children.len() != children.len()
        {
            return Err(refused("range_content"));
        }
        for (expected, actual) in children.iter().zip(&actual.children) {
            rendered(expected, tree, *actual, participants, work, depth + 1)?;
        }
        Ok(())
    } else {
        literal(node, tree, id, work, depth)
    }
}
// Source keys are runtime provenance within this exact retained application
// lineage, never portable authority. Descendant commits may replace only their
// own island/range; their source host identity remains the parent's produced node.
fn native_output(
    before: &RetainedCemTree,
    original: u32,
    after: &RetainedCemTree,
    current: u32,
    hosts: &BTreeMap<u32, &SettledParticipant>,
    work: &mut Work<'_>,
    depth: usize,
) -> Result<(), Vec<Diagnostic>> {
    work.step(depth)?;
    if before.source_key(original).is_none()
        || before.source_key(original) != after.source_key(current)
    {
        return Err(refused("output_identity"));
    }
    if let Some(child) = hosts.get(&current) {
        let source = &child.pass.input().prepared().elements().tree;
        let host = child.pass.input().prepared().instance_node();
        if before.source_key(original) != source.source_key(host) {
            return Err(refused("output_identity"));
        }
        return same_tree(before, original, source, host, work, depth);
    }
    let a = before
        .node(original)
        .ok_or_else(|| refused("node_missing"))?;
    let b = after.node(current).ok_or_else(|| refused("node_missing"))?;
    if a.kind != b.kind
        || a.value != b.value
        || a.name.as_ref().map(|n| (&n.namespace_uri, &n.local_name))
            != b.name.as_ref().map(|n| (&n.namespace_uri, &n.local_name))
        || attrs(before, original)? != attrs(after, current)?
        || a.children.len() != b.children.len()
    {
        return Err(refused("range_content"));
    }
    for (a, b) in a.children.iter().zip(&b.children) {
        native_output(before, *a, after, *b, hosts, work, depth + 1)?;
    }
    Ok(())
}
fn island_projection(
    item: &Item,
    control: &OperationControl,
) -> Result<RenderPlan, Vec<Diagnostic>> {
    let plan = RenderPlan {
        nodes: vec![RenderPlanNode::Reference {
            reference: cem_ml::value::CemReference::new(vec![item.clone()]),
            source_map: Default::default(),
        }],
        host_attribute_updates: vec![],
        diagnostics: vec![],
    };
    crate::render::project_render_plan_with_control(
        &plan,
        QueryContextScope(0),
        &crate::instance_island::LIMITS,
        control,
        ROOT_EXECUTION_SCOPE_ID,
    )
    .map_err(|_| refused("island_projection"))
}
fn owned_range_end(
    tree: &RetainedCemTree,
    p: &SettledParticipant,
) -> Result<usize, Vec<Diagnostic>> {
    let host = tree.node(p.host).ok_or_else(|| refused("host_missing"))?;
    let end = host
        .children
        .len()
        .checked_sub(p.styles.len() + 1)
        .ok_or_else(|| refused("range"))?;
    if end < 2
        || host.children.first() != Some(&p.island)
        || host.children.get(1) != Some(&p.start)
        || host.children.get(end) != Some(&p.end)
        || !host.children.ends_with(&p.styles)
        || tree
            .node(p.start)
            .is_none_or(|n| n.kind != CemTreeNodeKind::Comment || n.value != "cem-render-start")
        || tree
            .node(p.end)
            .is_none_or(|n| n.kind != CemTreeNodeKind::Comment || n.value != "cem-render-end")
    {
        return Err(refused("range"));
    }
    Ok(end)
}
fn validate(
    settled: &QuiescentInlinePage,
    tree: &Arc<RetainedCemTree>,
    context: &SsrReadinessContext,
    control: &OperationControl,
) -> Result<(), Vec<Diagnostic>> {
    if !settled.control.same_operation(control)
        || !Arc::ptr_eq(tree, &settled.elements.tree)
        || !settled.page.accepts_source_tree(tree)
    {
        return Err(refused("owner_stale"));
    }
    control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|_| refused("operation_stale"))?;
    let cpu = settled
        .cpu
        .as_ref()
        .ok_or_else(|| refused("cpu_unavailable"))?;
    cpu.usage_ms(control).map_err(cpu_error)?;
    if context.route != *settled.page.route()
        || context.policy != *settled.page.policy()
        || context.input
            != settled
                .page
                .render_input_identity()
                .map_err(|_| refused("identity_incomplete"))?
    {
        return Err(refused("context_stale"));
    }
    let identity = settled.page.execution_identity();
    if identity.lifecycle_version != crate::instance_island::VERSION
        || identity.island_schema_version != crate::instance_island::VERSION
        || identity.cem_ql_version != crate::VERSION
        || identity.render_plan_version != SSR_READY_RENDER_PLAN
        || identity.dependencies.engine_version != cem_ml::VERSION
        || identity.entrypoint != "ssr:render-page"
        || identity.formatter_profile != "dom-safe/1"
    {
        return Err(refused("protocol_version"));
    }
    if settled.page.usage().instances as usize != settled.participants.len()
        || settled.page.usage().render_passes as usize != settled.participants.len()
    {
        return Err(refused("inventory"));
    }
    if control
        .active_elapsed_ms(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|_| refused("wall_time"))?
        > context.policy.max_wall_time_ms
    {
        return Err(refused("wall_time"));
    }
    if let Some(resources) = &settled.resources {
        resources
            .check(control)
            .map_err(|_| refused("lease_stale"))?;
    }
    let hosts = settled
        .participants
        .iter()
        .map(|p| (p.host, p))
        .collect::<BTreeMap<_, _>>();
    let tags = settled
        .participants
        .iter()
        .map(|p| p.tag.clone())
        .collect::<BTreeSet<_>>();
    if hosts.len() != settled.participants.len()
        || tags != context.versions.keys().cloned().collect()
        || settled
            .participants
            .iter()
            .map(|p| &p.instance_id)
            .collect::<BTreeSet<_>>()
            .len()
            != settled.participants.len()
    {
        return Err(refused("inventory"));
    }
    let mut resources = 0u32;
    let mut bytes = settled.page.authored_page_byte_length();
    for (tag, definition) in &settled.definitions {
        let Some(lease) = &definition.external else {
            continue;
        };
        let witness = settled
            .resources
            .as_ref()
            .and_then(|resources| resources.inputs.get(lease.uri()));
        if witness != Some(&lease.input)
            || !lease.control.same_operation(control)
            || lease.tree.source_uri() != lease.uri()
            || !html(&lease.tree, lease.declaration, "cem-element")
            || !html(&lease.tree, definition.template, "template")
            || lease.tree.source_parent(definition.template) != Some(lease.declaration)
            || super::attr(&lease.tree, lease.declaration, "tag").as_ref() != Some(tag)
            || super::attr(&lease.tree, lease.declaration, "version") != definition.version
        {
            return Err(refused("source_lease"));
        }
        resources = resources
            .checked_add(1)
            .ok_or_else(|| refused("accounting"))?;
        bytes = bytes
            .checked_add(lease.bytes() as u64)
            .ok_or_else(|| refused("accounting"))?;
    }
    for participant in &settled.participants {
        let input = participant.pass.input();
        if let Some((source, external)) = input
            .bindings()
            .native_payload_source
            .as_ref()
            .and_then(|source| source.external_input().map(|external| (source, external)))
        {
            let witness = settled
                .resources
                .as_ref()
                .and_then(|resources| resources.inputs.get(&external.uri));
            if witness != Some(&super::ResourceInput::from_payload(external))
                || source.tree().source_uri() != external.uri
                || source.tree().document_uri() != Some(external.uri.as_str())
            {
                return Err(refused("source_lease"));
            }
            resources = resources
                .checked_add(1)
                .ok_or_else(|| refused("accounting"))?;
            bytes = bytes
                .checked_add(external.byte_length as u64)
                .ok_or_else(|| refused("accounting"))?;
        }
        resources = resources
            .checked_add(
                input
                    .coordinated_css
                    .as_ref()
                    .map_or(input.instance_stylesheets().len() as u32, |css| {
                        css.resources
                    }),
            )
            .ok_or_else(|| refused("accounting"))?;
        bytes = bytes
            .checked_add(input.coordinated_css.as_ref().map_or(0, |css| css.bytes))
            .ok_or_else(|| refused("accounting"))?;
    }
    if settled.page.usage().resources != resources
        || settled.page.usage().total_bytes != bytes
        || settled.page.usage().declaration_depth
            != settled
                .participants
                .iter()
                .map(|p| p.depth)
                .max()
                .unwrap_or(0)
    {
        return Err(refused("accounting"));
    }
    let mut work = Work {
        remaining: tree.ast().nodes.len().saturating_mul(128).min(1_048_576),
        cpu,
        control,
    };
    for (tag, definition) in &settled.definitions {
        let original = definition
            .source
            .node(definition.source_declaration)
            .ok_or_else(|| refused("declaration_missing"))?;
        let actual = tree
            .node(definition.declaration)
            .ok_or_else(|| refused("declaration_missing"))?;
        if attrs(&definition.source, definition.source_declaration)?
            != attrs(tree, definition.declaration)?
        {
            return Err(refused("declaration_changed"));
        }
        if !html(tree, definition.declaration, "cem-element")
            || definition.source.source_key(definition.source_declaration)
                != tree.source_key(definition.declaration)
            || super::attr(tree, definition.declaration, "tag").as_deref() != Some(tag.as_str())
            || super::attr(tree, definition.declaration, "version") != definition.version
        {
            return Err(refused("declaration_changed"));
        }
        let styles = settled
            .declaration_styles
            .get(tag)
            .map_or(&[][..], Vec::as_slice);
        if actual.children.len() != original.children.len() + styles.len()
            || !actual.children.ends_with(styles)
        {
            return Err(refused("declaration_range"));
        }
        for (a, b) in original.children.iter().zip(&actual.children) {
            same_tree(&definition.source, *a, tree, *b, &mut work, 0)?;
        }
        if !styles.is_empty() {
            let lease = settled
                .resources
                .as_ref()
                .and_then(|r| r.declarations.get(tag))
                .ok_or_else(|| refused("lease_missing"))?;
            let expected = lease
                .declaration_output(&Default::default())
                .map_err(|_| refused("lease_stale"))?;
            if expected.len() != styles.len() {
                return Err(refused("declaration_range"));
            }
            for (expected, id) in expected.iter().zip(styles) {
                literal(expected, tree, *id, &mut work, 0)?;
            }
        }
    }
    for p in &settled.participants {
        let definition = settled
            .definitions
            .get(&p.tag)
            .ok_or_else(|| refused("declaration_missing"))?;
        let version = definition
            .version
            .as_deref()
            .ok_or_else(|| refused("declaration_version"))?;
        if !compatible(version, &context.versions[&p.tag]) {
            return Err(refused("declaration_version"));
        }
        let pass = &p.pass;
        let input = pass.input();
        if input.prepared().declaration_version.as_deref() != Some(version) {
            return Err(refused("declaration_version"));
        }
        if let Some(css) = &input.coordinated_css {
            css.check(control).map_err(|_| refused("lease_stale"))?;
        }
        if !Arc::ptr_eq(
            input.prepared().declaration_lease_identity(),
            &definition.retained,
        ) || definition.external.as_ref().is_some_and(|lease| {
            !std::ptr::eq(input.prepared().template_source_tree(), lease.tree.as_ref())
                || input.prepared().template_node() != definition.template
        }) {
            return Err(refused("declaration_changed"));
        }
        let source = input.bindings().native_payload_source.as_ref();
        if let Some(source) = source {
            let envelope = input
                .payload_envelope()
                .ok_or_else(|| refused("source_lease"))?;
            if !source.matches_operation(control)
                || !source.matches_source(envelope.owner(), envelope.node_id())
                || !pass
                    .committed_island()
                    .bindings()
                    .native_payload_source
                    .as_ref()
                    .is_some_and(|other| Arc::ptr_eq(source, other))
            {
                return Err(refused("source_lease"));
            }
        }
        crate::instance_island::bind_retained_root(
            pass.committed_island().context_root(),
            &p.instance_id,
            control,
            source,
        )
        .map_err(|_| refused("island_incomplete"))?;
        let host = tree.node(p.host).ok_or_else(|| refused("host_missing"))?;
        let end = owned_range_end(tree, p)?;
        if tree.source_key(p.host).is_none()
            || tree.source_key(p.host)
                != pass
                    .application()
                    .tree()
                    .source_key(pass.application().output_target())
        {
            return Err(refused("host_identity"));
        }
        if attrs(tree, p.host)?
            != attrs(
                pass.application().tree(),
                pass.application().output_target(),
            )?
        {
            return Err(refused("host_attributes"));
        }
        let projection = island_projection(pass.committed_island().template(), control)?;
        if projection.nodes.len() != 1 {
            return Err(refused("island_incomplete"));
        }
        literal(&projection.nodes[0], tree, p.island, &mut work, 0)?;
        let island_root = tree
            .node(p.island)
            .and_then(|n| n.children.first())
            .copied()
            .ok_or_else(|| refused("island_incomplete"))?;
        let data = tree
            .node(island_root)
            .and_then(|n| n.children.first())
            .copied()
            .ok_or_else(|| refused("island_incomplete"))?;
        let version_fields = tree
            .node(data)
            .unwrap()
            .children
            .iter()
            .filter(|id| super::attr(tree, **id, "name").as_deref() == Some("declarationVersion"))
            .collect::<Vec<_>>();
        if version_fields.len() != 1
            || tree
                .text_fragments(*version_fields[0])
                .ok_or_else(|| refused("declaration_version"))?
                .collect::<String>()
                != version
        {
            return Err(refused("declaration_version"));
        }
        if pass.projected_plan().nodes.len() != end - 2 {
            return Err(refused("range_content"));
        }
        for (expected, id) in pass
            .projected_plan()
            .nodes
            .iter()
            .zip(&host.children[2..end])
        {
            rendered(expected, tree, *id, &hosts, &mut work, 0)?;
        }
        let before = pass.application().tree();
        let original_host = before
            .node(pass.application().output_target())
            .ok_or_else(|| refused("host_missing"))?;
        let original_end = original_host
            .children
            .len()
            .checked_sub(p.styles.len() + 1)
            .ok_or_else(|| refused("range"))?;
        if original_end < 2 || original_end != end {
            return Err(refused("range_content"));
        }
        for (a, b) in original_host.children[2..original_end]
            .iter()
            .zip(&host.children[2..end])
        {
            native_output(before, *a, tree, *b, &hosts, &mut work, 0)?;
        }
        let expected = input
            .coordinated_css
            .as_ref()
            .map(|css| css.instance_output())
            .transpose()
            .map_err(|_| refused("lease_stale"))?
            .unwrap_or_default();
        if expected.len() != p.styles.len() {
            return Err(refused("instance_style_range"));
        }
        for (expected, id) in expected.iter().zip(&p.styles) {
            literal(expected, tree, *id, &mut work, 0)?;
        }
    }
    // Complete active census and live ID uniqueness; inert data and declarations
    // are excluded exactly as they are during lifecycle discovery.
    let mut pending = vec![0];
    let mut seen = BTreeSet::new();
    let mut ids = BTreeSet::new();
    while let Some(id) = pending.pop() {
        work.step(0)?;
        let n = tree.node(id).ok_or_else(|| refused("node_missing"))?;
        if html(tree, id, "template") || html(tree, id, "cem-element") {
            continue;
        }
        if n.name.as_ref().is_some_and(|name| {
            name.namespace_uri == HTML_NAMESPACE_URI && name.local_name.contains('-')
        }) && (!hosts.contains_key(&id) || !seen.insert(id))
        {
            return Err(refused("inventory"));
        }
        if let Some(value) = super::attr(tree, id, "id") {
            if !value.is_empty() && !ids.insert(value) {
                return Err(refused("duplicate_id"));
            }
        }
        pending.extend(n.children.iter().copied());
    }
    if seen.len() != hosts.len() {
        return Err(refused("inventory"));
    }
    if let Some(resources) = &settled.resources {
        resources
            .check(control)
            .map_err(|_| refused("lease_stale"))?;
    }
    control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|_| refused("operation_stale"))?;
    cpu.usage_ms(control).map_err(cpu_error)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn readiness_version_compatibility_matrix_is_exact_and_matches_browser_bands() {
        for (a, b, yes) in [
            ("1.2.3", "1.9.0", true),
            ("0.2.3", "0.2.9", true),
            ("0.2.3", "0.3.0", false),
            ("0.0.3", "0.0.4", false),
            ("1.2.3-rc.1+a", "1.2.3-rc.1+b", true),
            ("1.2.3-rc.1", "1.2.4-rc.1", false),
            ("1.2.3", "1.2.3-beta", false),
            ("1.2.3", " 1.2.3", false),
            ("01.2.3", "1.2.3", false),
        ] {
            assert_eq!(compatible(a, b), yes, "{a} {b}");
        }
    }
    #[test]
    fn readiness_literal_witness_refuses_changed_removed_or_reordered_island_content() {
        let control = OperationControl::new(cem_ml::scheduler::AbortSignal::new());
        let cpu = Arc::new(NativeCpuAccounting::new(&control, 1000).unwrap());
        let guard = cpu.begin(&control).unwrap();
        let owner=Arc::new(cem_ml::validation::html::html_document_ast_from_source_bytes(cem_ml::validation::html::HtmlSourceValidationRequest{bytes:b"<html><body><template data-cem-island='instance'><p>Payload</p><b>State</b></template></body></html>",source_uri:"file:///ready.html",content_type:Some("text/html")}).0.unwrap());
        let imported = retain_html_elements(owner, Default::default()).unwrap();
        let template = imported
            .elements
            .iter()
            .map(|e| e.0)
            .find(|id| html(&imported.tree, *id, "template"))
            .unwrap();
        let node = |tag: &str, text: &str| RenderPlanNode::Element {
            tag: tag.into(),
            namespace: Some(HTML_NAMESPACE_URI.into()),
            qualified_name: None,
            attributes: vec![],
            children: vec![RenderPlanNode::Text {
                text: text.into(),
                source_map: Default::default(),
            }],
            source_map: Default::default(),
        };
        let children = vec![node("p", "Payload"), node("b", "State")];
        let expected = RenderPlanNode::Element {
            tag: "template".into(),
            namespace: Some(HTML_NAMESPACE_URI.into()),
            qualified_name: None,
            attributes: vec![crate::instance_island::construct::attribute(
                "data-cem-island",
                "instance",
                &Default::default(),
            )],
            children: children.clone(),
            source_map: Default::default(),
        };
        let mut work = Work {
            remaining: 1000,
            cpu: &cpu,
            control: &control,
        };
        literal(&expected, &imported.tree, template, &mut work, 0).unwrap();
        for bad in [
            vec![node("p", "Changed"), node("b", "State")],
            vec![node("p", "Payload")],
            vec![node("b", "State"), node("p", "Payload")],
        ] {
            let applied = cem_ml::parser::tree_apply::apply_children(
                imported.tree.clone(),
                template,
                Arc::new(bad),
                Arc::new(()),
                Default::default(),
                &control,
            )
            .unwrap();
            let mut work = Work {
                remaining: 1000,
                cpu: &cpu,
                control: &control,
            };
            assert!(literal(
                &expected,
                applied.tree(),
                applied.output_target(),
                &mut work,
                0
            )
            .is_err());
        }
        guard.finish().unwrap();
    }
}
#[cfg(test)]
mod range_tests {
    use super::*;
    #[test]
    fn owned_range_refuses_suffix_grafting_after_native_application() {
        let control = OperationControl::new(cem_ml::scheduler::AbortSignal::new());
        let source=Arc::new(cem_ml::validation::html::html_document_ast_from_source_bytes(cem_ml::validation::html::HtmlSourceValidationRequest{bytes:b"<html><body><cem-element tag='x-a'><template id='x-a' type='text/cem-ml'>{b | Ready}</template></cem-element><x-a></x-a></body></html>",source_uri:"file:///ready.html",content_type:Some("text/html")}).0.unwrap());
        let routes = cem_ml::ssr_route::preflight_ssr_page_routes(&[
            cem_ml::ssr_route::SsrPageRouteRequest::new(&source, "https://ready.test/", None),
        ])
        .unwrap();
        use cem_ml::{dependency_identity::*, ssr_artifact::*};
        let page = CemSsrPageArtifact::unrendered(
            source.clone(),
            &routes[0],
            cem_ml::transform_artifact::TransformArtifactBody::HtmlDomProjection(source.clone()),
            SsrExecutionIdentity {
                dependencies: DependencySnapshot {
                    layer: DependencyLayer::RenderProjection,
                    engine_version: cem_ml::VERSION.into(),
                    format_version: "ssr/1".into(),
                    closure_state: DependencyClosureState::Pending,
                    root_uri: source.source.uri.clone(),
                    sources: Default::default(),
                    imports: vec![],
                    options: Default::default(),
                    policy: DependencyVersion::Opaque,
                    resolution: DependencyVersion::Opaque,
                    scope: DependencyVersion::Opaque,
                    inputs: Default::default(),
                },
                lifecycle_version: "0.2.0".into(),
                island_schema_version: "0.2.0".into(),
                render_plan_version: SSR_READY_RENDER_PLAN.into(),
                cem_ql_version: crate::VERSION.into(),
                entrypoint: "ssr:render-page".into(),
                parameters_hash: ContentHash::from_blake3(b"params"),
                source_map_mode: SsrSourceMapMode::Retained,
                formatter_profile: "dom-safe/1".into(),
            },
            SsrSettlePolicy {
                max_declaration_depth: 8,
                max_instances: 4,
                max_render_passes: 4,
                max_resources: 4,
                max_total_bytes: 1_048_576,
                max_wall_time_ms: 10000,
                max_cpu_time_ms: 1000,
                allow_static_fallback: false,
            },
        )
        .unwrap();
        let mut work = prepare_inline_page(page, "ready", &control).unwrap();
        while !matches!(work.advance(&control).unwrap(), SsrPageProgress::Quiescent) {}
        let settled = work.finish(&control).unwrap();
        let p = &settled.participants[0];
        owned_range_end(&settled.elements.tree, p).unwrap();
        use cem_ml::parser::tree_apply::{
            apply_elements, CemChildEditMode, CemElementEdit, CemElementPatch,
        };
        let app = apply_elements(
            settled.elements.tree.clone(),
            p.host,
            vec![CemElementPatch {
                target: p.host,
                edit: CemElementEdit {
                    children: Arc::new(vec![RenderPlanNode::Text {
                        text: "Grafted".into(),
                        source_map: Default::default(),
                    }]),
                    attributes: Arc::new(vec![]),
                },
                children: CemChildEditMode::Append,
            }],
            Arc::new(()),
            Default::default(),
            &control,
        )
        .unwrap();
        let map = |id| app.output_for_source(id).unwrap();
        let moved = SettledParticipant {
            tag: p.tag.clone(),
            instance_id: p.instance_id.clone(),
            depth: p.depth,
            host: map(p.host),
            island: map(p.island),
            start: map(p.start),
            end: map(p.end),
            styles: vec![],
            pass: p.pass.clone(),
        };
        assert!(owned_range_end(app.tree(), &moved).is_err());
    }
}
