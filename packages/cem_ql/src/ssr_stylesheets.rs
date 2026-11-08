//! Native single-page stylesheet coordination. Transport supplies named byte
//! responses; shared import, resolution, ownership and emission stay native.
use crate::{
    render::{HostAttributeUpdate, RenderPlanNode, TemplateArtifact},
    retained_template::{RetainedStylesheet, RetainedTemplate},
    ssr_island::PreparedInlineIsland,
};
use cem_ml::{
    css_emission::CssManagedScope,
    css_imports::{CssImportClosure, CssImportLimits, CssImportRequest, CssImportResponsePolicy},
    module_resolution::CemModuleUrlResolutionCapability,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    resolver::ResolvedRead,
    source_map::SourceMapStack,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsrCssState {
    Pending,
    Ready,
    Refused,
    Cancelled,
}
#[derive(Debug, Clone)]
pub struct SsrCssRequest {
    ticket: Arc<()>,
    serial: u64,
    slot: usize,
    request: Box<CssImportRequest>,
}
impl SsrCssRequest {
    pub fn request(&self) -> &CssImportRequest {
        &self.request
    }
}
#[derive(Debug, Clone)]
pub enum SsrCssProgress {
    Pending(SsrCssRequest),
    Ready,
}
struct Load {
    closure: Option<CssImportClosure>,
    declaration: bool,
    index: usize,
    scope: CssManagedScope,
    identity: String,
    record: Option<Arc<RetainedStylesheet>>,
    references: usize,
}
pub struct SsrStylesheetCoordinator {
    input: Option<PreparedInlineIsland>,
    owners: Vec<RetainedTemplate>,
    loads: Vec<Load>,
    maps: Vec<SourceMapStack>,
    control: OperationControl,
    capability: CemModuleUrlResolutionCapability,
    fingerprint: String,
    ticket: Arc<()>,
    serial: u64,
    pending: Option<SsrCssRequest>,
    state: SsrCssState,
    stale: AtomicBool,
    resources: u32,
    bytes: u64,
    elapsed_limit: u64,
    failure: Option<cem_ml::css_imports::CssImportFailure>,
    shared_declaration: Option<Arc<CoordinatedStylesheets>>,
}
impl std::fmt::Debug for SsrStylesheetCoordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SsrStylesheetCoordinator")
            .field("state", &self.state())
            .field("resources", &self.resources)
            .finish_non_exhaustive()
    }
}
pub fn begin_inline_stylesheets(
    input: PreparedInlineIsland,
    capability: CemModuleUrlResolutionCapability,
    control: &OperationControl,
) -> Result<SsrStylesheetCoordinator, String> {
    begin_inline_stylesheets_reusing(input, capability, None, control)
}
pub(crate) fn begin_inline_stylesheets_reusing(
    input: PreparedInlineIsland,
    capability: CemModuleUrlResolutionCapability,
    shared_declaration: Option<Arc<CoordinatedStylesheets>>,
    control: &OperationControl,
) -> Result<SsrStylesheetCoordinator, String> {
    control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|e| e.code().to_owned())?;
    if !input.prepared().stylesheet_coordination || input.coordinated_css.is_some() {
        return Err("Stylesheet coordination requires its explicit deferred source profile".into());
    }
    if !input
        .prepared()
        .stylesheet_control
        .as_ref()
        .is_some_and(|c| c.same_operation(control))
    {
        return Err("Stylesheet preparation belongs to the original operation".into());
    }
    let fingerprint = capability
        .cache_identity()
        .filter(|v| !v.is_empty())
        .ok_or("Stylesheets require explicit stable resolver authority")?;
    if fingerprint.len() > crate::instance_island::LIMITS.max_bytes {
        return Err("Stylesheet authority identity exceeds bounds".into());
    }
    let prepared = input.prepared();
    let page = prepared.page();
    let policy = page.policy();
    let (instances, maps) = crate::ssr_css::collect_sources(
        &prepared.elements().tree,
        input.payload(),
        input.payload_envelope().is_some() && input.bindings().native_payload_source.is_none(),
        policy,
        control,
    )?;
    if let Some(shared) = &shared_declaration {
        shared.check(control)?;
        if !Arc::ptr_eq(
            &shared.declaration_owner,
            prepared.declaration_lease_identity(),
        ) || shared.fingerprint != fingerprint
        {
            return Err(
                "Declaration lease belongs to another native definition or resolver".into(),
            );
        }
    }
    let declarations = if shared_declaration.is_some() {
        vec![]
    } else {
        prepared.declaration_owner().artifact().stylesheets.clone()
    };
    let remaining_resources = policy
        .max_resources
        .checked_sub(page.usage().resources)
        .ok_or("CSS page resource budget exceeded")?;
    let roots = instances
        .len()
        .checked_add(declarations.len())
        .ok_or("Stylesheet count overflow")?;
    if roots > 64 || roots as u32 > remaining_resources {
        return Err("Stylesheet root resource budget exceeded".into());
    }
    let artifact = |stylesheets| TemplateArtifact {
        nodes: vec![],
        stylesheets,
        module_map: None,
        diagnostics: vec![],
    };
    let owners = vec![
        RetainedTemplate::new(artifact(declarations)),
        RetainedTemplate::new_instance_stylesheet_artifacts(
            artifact(instances),
            input.instance_id(),
        )?,
    ];
    let workspace = owners
        .iter()
        .flat_map(|owner| owner.artifact().stylesheets.iter())
        .try_fold(
            1024usize.saturating_add(fingerprint.len().saturating_mul(128)),
            |bytes, source| {
                bytes
                    .checked_add(source.css().len().saturating_mul(128))
                    .and_then(|bytes| {
                        bytes.checked_add(
                            source
                                .scope
                                .as_ref()
                                .map_or(0, String::len)
                                .saturating_mul(128),
                        )
                    })
                    .and_then(|bytes| {
                        bytes.checked_add(source.css_tree().ast().nodes.len().saturating_mul(1024))
                    })
            },
        )
        .ok_or("Stylesheet workspace size overflow")?;
    let permit = Arc::new(
        control
            .charge_memory(ROOT_EXECUTION_SCOPE_ID, workspace as u64, None)
            .map_err(|e| e.code().to_owned())?,
    );
    for source in owners
        .iter()
        .flat_map(|owner| owner.artifact().stylesheets.iter())
    {
        source.css_tree().retain_native_resource(permit.clone())?;
    }
    let identity = format!("{}#{}", page.route().source_uri, prepared.tag());
    let mut loads = vec![];
    let mut resources = roots as u32;
    for (owner_index, owner) in owners.iter().enumerate() {
        for (index, source) in owner.artifact().stylesheets.iter().enumerate() {
            control
                .check_scope(ROOT_EXECUTION_SCOPE_ID)
                .map_err(|e| e.code().to_owned())?;
            let declaration = owner_index == 0;
            let scope = if declaration {
                match &source.scope {
                    Some(name) if prepared.shared_scope.as_deref() == Some(name) => {
                        CssManagedScope::Shared {
                            name: name.clone(),
                            context: None,
                        }
                    }
                    Some(_) => {
                        return Err("Shared CSS requires matching declaration-owned scope".into())
                    }
                    None => CssManagedScope::Private {
                        tag: prepared.tag().into(),
                        context: None,
                    },
                }
            } else {
                CssManagedScope::Instance
            };
            let closure = CssImportClosure::new(
                source.css_tree().clone(),
                &page.route().document_base_url,
                capability.clone(),
                CssImportLimits {
                    max_sheets: (policy.max_resources as usize).min(64),
                    max_depth: (policy.max_declaration_depth as usize).min(16),
                },
                control.abort_signal().clone(),
            )
            .map_err(|e| e.to_string())?;
            let references = closure
                .sheets()
                .iter()
                .map(|s| s.resources.references.len())
                .sum::<usize>();
            resources = resources
                .checked_add(
                    references
                        .try_into()
                        .map_err(|_| "CSS resource count overflow")?,
                )
                .filter(|n| *n <= remaining_resources)
                .ok_or("CSS resource budget exceeded")?;
            loads.push(Load {
                closure: Some(closure),
                declaration,
                index,
                scope,
                identity: if declaration {
                    identity.clone()
                } else {
                    input.instance_id().into()
                },
                record: None,
                references,
            });
        }
    }
    let elapsed_limit = if prepared.lifecycle_cpu.is_some() {
        policy.max_wall_time_ms
    } else {
        policy.max_wall_time_ms.min(policy.max_cpu_time_ms)
    };
    if control
        .active_elapsed_ms(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|e| e.code().to_owned())?
        > elapsed_limit
    {
        return Err("Stylesheet operation elapsed budget exceeded".into());
    }
    Ok(SsrStylesheetCoordinator {
        input: Some(input),
        owners,
        loads,
        maps,
        control: control.clone(),
        capability,
        fingerprint,
        ticket: Arc::new(()),
        serial: 0,
        pending: None,
        state: SsrCssState::Pending,
        stale: AtomicBool::new(false),
        resources,
        bytes: 0,
        elapsed_limit,
        failure: None,
        shared_declaration,
    })
}
impl SsrStylesheetCoordinator {
    pub fn failure(&self) -> Option<&cem_ml::css_imports::CssImportFailure> {
        self.failure.as_ref()
    }
    pub fn state(&self) -> SsrCssState {
        if matches!(self.state, SsrCssState::Refused | SsrCssState::Cancelled) {
            return self.state;
        }
        if self.control.is_cancelled() {
            return SsrCssState::Cancelled;
        }
        if self.capability.cache_identity().as_deref() != Some(self.fingerprint.as_str()) {
            self.stale.store(true, Ordering::Relaxed);
        }
        if self.stale.load(Ordering::Relaxed)
            || self
                .control
                .active_elapsed_ms(ROOT_EXECUTION_SCOPE_ID)
                .is_ok_and(|elapsed| elapsed > self.elapsed_limit)
            || self.control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_err()
            || self.loads.iter().any(|l| {
                l.closure
                    .as_ref()
                    .is_some_and(|c| c.state() == cem_ml::css_imports::CssImportState::Failed)
                    || l.record.as_ref().is_some_and(|r| r.emission().is_err())
            })
        {
            return SsrCssState::Refused;
        }
        self.state
    }
    fn check(&self, control: &OperationControl) -> Result<(), String> {
        if !self.control.same_operation(control) {
            return Err("Stylesheet admission belongs to the original operation".into());
        }
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|e| e.code().to_owned())?;
        if let Some(cpu) = self
            .input
            .as_ref()
            .and_then(|input| input.prepared().lifecycle_cpu.as_ref())
        {
            cpu.usage_ms(control)?;
        }
        if let Some(shared) = &self.shared_declaration {
            shared.check(control)?;
        }
        if matches!(self.state(), SsrCssState::Cancelled | SsrCssState::Refused) {
            return Err("Stylesheet coordination is cancelled, refused or stale".into());
        }
        Ok(())
    }
    fn stop(&mut self) {
        if self.failure.is_none() {
            self.failure = self
                .loads
                .iter()
                .find_map(|load| load.closure.as_ref().and_then(|closure| closure.failure()));
        }
        self.state = if self.control.is_cancelled() {
            SsrCssState::Cancelled
        } else {
            SsrCssState::Refused
        };
        self.pending = None;
        self.shared_declaration = None;
        self.input = None;
        self.owners.clear();
        self.loads.clear();
    }
    pub fn advance(&mut self, control: &OperationControl) -> Result<SsrCssProgress, String> {
        if !self.control.same_operation(control) {
            return Err("Stylesheet admission belongs to the original operation".into());
        }
        let result = self.advance_inner(control);
        if result.is_err() {
            self.stop();
        }
        result
    }
    fn advance_inner(&mut self, control: &OperationControl) -> Result<SsrCssProgress, String> {
        self.check(control)?;
        if let Some(pending) = &self.pending {
            return Ok(SsrCssProgress::Pending(pending.clone()));
        }
        for slot in 0..self.loads.len() {
            if self.loads[slot].record.is_some() {
                continue;
            }
            if let Some(request) = self.loads[slot]
                .closure
                .as_mut()
                .unwrap()
                .next_import()
                .map_err(|e| e.to_string())?
            {
                self.serial = self
                    .serial
                    .checked_add(1)
                    .ok_or("CSS request identity exhausted")?;
                let request = SsrCssRequest {
                    ticket: self.ticket.clone(),
                    serial: self.serial,
                    slot,
                    request: Box::new(request),
                };
                self.pending = Some(request.clone());
                return Ok(SsrCssProgress::Pending(request));
            }
            let load = &mut self.loads[slot];
            let closure = Arc::new(load.closure.take().unwrap());
            let owner = if load.declaration { 0 } else { 1 };
            let record = self.owners[owner].retain_stylesheet(
                "ssr-page",
                load.index,
                closure,
                &load.scope,
                &load.identity,
            )?;
            if !record.emission()?.diagnostics.is_empty() {
                return Err("Unsupported managed stylesheet syntax".into());
            }
            load.record = Some(record);
        }
        self.check(control)?;
        self.state = SsrCssState::Ready;
        Ok(SsrCssProgress::Ready)
    }
    pub(crate) fn match_request(&self, request: &SsrCssRequest) -> Result<(), String> {
        if !Arc::ptr_eq(&self.ticket, &request.ticket)
            || !self.pending.as_ref().is_some_and(|p| {
                p.serial == request.serial
                    && p.slot == request.slot
                    && p.request.id == request.request.id
            })
        {
            return Err("Stylesheet delivery is stale or belongs to another coordinator".into());
        }
        Ok(())
    }
    pub fn deliver(
        &mut self,
        request: &SsrCssRequest,
        response: ResolvedRead,
        control: &OperationControl,
    ) -> Result<(), String> {
        self.match_request(request)?;
        if !self.control.same_operation(control) {
            return Err("Stylesheet admission belongs to the original operation".into());
        }
        let result = (|| {
            self.check(control)?;
            let page = self.input.as_ref().unwrap().prepared().page();
            let remaining = page
                .policy()
                .max_total_bytes
                .checked_sub(page.usage().total_bytes)
                .and_then(|n| n.checked_sub(self.bytes))
                .ok_or("CSS aggregate byte budget exceeded")?;
            if response.bytes.len() as u64 > remaining {
                return Err("CSS aggregate byte budget exceeded".into());
            }
            let delivered = response.bytes.len() as u64;
            let load = &mut self.loads[request.slot];
            let closure = load.closure.as_mut().unwrap();
            let response_policy = CssImportResponsePolicy {
                max_response_bytes: remaining.try_into().unwrap_or(usize::MAX),
                max_total_bytes: closure
                    .received_bytes()
                    .saturating_add(remaining.try_into().unwrap_or(usize::MAX)),
            };
            closure
                .complete_response_with_control(
                    request.request.id,
                    response,
                    &response_policy,
                    control,
                )
                .map_err(|e| e.to_string())?;
            let references = closure
                .sheets()
                .iter()
                .map(|s| s.resources.references.len())
                .sum::<usize>();
            let added = references
                .checked_sub(load.references)
                .ok_or("CSS resource accounting regressed")?;
            self.resources = self
                .resources
                .checked_add(
                    added
                        .try_into()
                        .map_err(|_| "CSS resource count overflow")?,
                )
                .filter(|n| {
                    page.usage()
                        .resources
                        .checked_add(*n)
                        .is_some_and(|total| total <= page.policy().max_resources)
                })
                .ok_or("CSS resource budget exceeded")?;
            load.references = references;
            self.bytes = self
                .bytes
                .checked_add(delivered)
                .ok_or("CSS byte accounting overflow")?;
            self.pending = None;
            self.check(control)
        })();
        if result.is_err() {
            self.stop();
        }
        result
    }
    pub fn fail(
        &mut self,
        request: &SsrCssRequest,
        message: &str,
        control: &OperationControl,
    ) -> Result<(), String> {
        self.match_request(request)?;
        if !self.control.same_operation(control) {
            return Err("Stylesheet admission belongs to the original operation".into());
        }
        if let Err(error) = self.check(control) {
            self.stop();
            return Err(error);
        }
        if message.len() > cem_ml::import::MAX_DOCUMENT_BYTES {
            return Err("Stylesheet transport failure exceeds bounds".into());
        }
        let closure = self.loads[request.slot].closure.as_mut().unwrap();
        let error = closure
            .fail_import(request.request.id, "cem.css.import_load_failed", message)
            .err()
            .or_else(|| closure.failure())
            .map(|e| e.to_string())
            .unwrap_or_else(|| "CSS transport failed".into());
        self.stop();
        Err(error)
    }
    pub fn finish(mut self, control: &OperationControl) -> Result<PreparedInlineIsland, String> {
        if !matches!(self.advance(control)?, SsrCssProgress::Ready) {
            return Err("Stylesheet batch is still pending".into());
        }
        let mut input = self.input.take().unwrap();
        let mut declarations = vec![];
        let mut instances = vec![];
        for load in &mut self.loads {
            let record = load.record.take().unwrap();
            if load.declaration {
                declarations.push(record)
            } else {
                instances.push(record)
            }
        }
        if let Some(shared) = &self.shared_declaration {
            declarations = shared.declarations.clone();
        }
        let ready = CoordinatedStylesheets {
            cpu: input.prepared().lifecycle_cpu.clone(),
            declaration_owner: input.prepared().declaration_lease_identity().clone(),
            shared_declaration: self.shared_declaration.take(),
            _owners: std::mem::take(&mut self.owners),
            declarations,
            instances,
            maps: std::mem::take(&mut self.maps),
            control: self.control.clone(),
            capability: self.capability.clone(),
            fingerprint: self.fingerprint.clone(),
            stale: AtomicBool::new(false),
            resources: self.resources,
            bytes: self.bytes,
            shared_scope: input.prepared().shared_scope.clone(),
            source: input.prepared().elements().tree.clone(),
            host: input.prepared().instance_node(),
            instance: input.instance_id().into(),
            elapsed_limit: self.elapsed_limit,
        };
        ready.check(control)?;
        input.coordinated_css = Some(Arc::new(ready));
        Ok(input)
    }
}
pub(crate) struct CoordinatedStylesheets {
    cpu: Option<Arc<cem_ml::operation_control::cpu::NativeCpuAccounting>>,
    declaration_owner: Arc<RetainedTemplate>,
    shared_declaration: Option<Arc<CoordinatedStylesheets>>,
    _owners: Vec<RetainedTemplate>,
    declarations: Vec<Arc<RetainedStylesheet>>,
    instances: Vec<Arc<RetainedStylesheet>>,
    maps: Vec<SourceMapStack>,
    control: OperationControl,
    capability: CemModuleUrlResolutionCapability,
    fingerprint: String,
    stale: AtomicBool,
    pub(crate) resources: u32,
    pub(crate) bytes: u64,
    shared_scope: Option<String>,
    source: Arc<cem_ml::parser::tree::RetainedCemTree>,
    host: u32,
    instance: String,
    elapsed_limit: u64,
}
impl std::fmt::Debug for CoordinatedStylesheets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoordinatedStylesheets")
            .field("resources", &self.resources)
            .finish_non_exhaustive()
    }
}
impl CoordinatedStylesheets {
    pub(crate) fn check(&self, control: &OperationControl) -> Result<(), String> {
        if !self.control.same_operation(control) {
            return Err("Stylesheet batch belongs to the original operation".into());
        }
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|e| e.code().to_owned())?;
        if control
            .active_elapsed_ms(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|e| e.code().to_owned())?
            > self.elapsed_limit
        {
            return Err("Stylesheet operation elapsed budget exceeded".into());
        }
        if self.capability.cache_identity().as_deref() != Some(self.fingerprint.as_str()) {
            self.stale.store(true, Ordering::Relaxed);
        }
        if self.stale.load(Ordering::Relaxed) {
            return Err("Stylesheet resolver authority changed".into());
        }
        if let Some(shared) = &self.shared_declaration {
            shared.check(control)?;
        }
        if let Some(cpu) = &self.cpu {
            cpu.usage_ms(control)?;
        }
        for record in self.declarations.iter().chain(&self.instances) {
            record.emission()?;
        }
        Ok(())
    }
    pub(crate) fn check_host(
        &self,
        host: &crate::eval::RetainedCemNode,
        instance: &str,
        control: &OperationControl,
    ) -> Result<(), String> {
        self.check(control)?;
        if !Arc::ptr_eq(host.owner(), &self.source)
            || host.node_id() != self.host
            || instance != self.instance
        {
            return Err("Stylesheet batch belongs to another native host".into());
        }
        Ok(())
    }
    pub(crate) fn declaration_records(&self) -> &[Arc<RetainedStylesheet>] {
        &self.declarations
    }
    pub(crate) fn instance_records(&self) -> &[Arc<RetainedStylesheet>] {
        &self.instances
    }
    pub(crate) fn strip_sources(&self, nodes: &mut Vec<RenderPlanNode>) {
        crate::ssr_css::strip_sources(nodes, &self.maps)
    }
    pub(crate) fn host_updates(
        &self,
        origin: &SourceMapStack,
    ) -> Result<Vec<HostAttributeUpdate>, String> {
        let contexts = self
            .declarations
            .iter()
            .filter_map(|r| r.identity().context_marker.as_ref())
            .collect::<std::collections::BTreeSet<_>>();
        if contexts.len() > 1 {
            return Err("Declaration CSS context identities disagree".into());
        }
        let mut updates = vec![];
        if let Some(context) = contexts.first() {
            updates.push(
                HostAttributeUpdate::new("data-cem-css-context", context.as_str())
                    .with_source_map(origin.clone()),
            );
        }
        if let Some(scope) = &self.shared_scope {
            updates.push(HostAttributeUpdate::new("scope", scope).with_source_map(origin.clone()));
        }
        Ok(updates)
    }
    pub(crate) fn instance_output(&self) -> Result<Vec<RenderPlanNode>, String> {
        self.output(&self.instances, false, &SourceMapStack::default())
    }
    pub(crate) fn declaration_output(
        &self,
        origin: &SourceMapStack,
    ) -> Result<Vec<RenderPlanNode>, String> {
        if self.shared_declaration.is_some() {
            Ok(vec![])
        } else {
            self.output(&self.declarations, true, origin)
        }
    }
    fn output(
        &self,
        records: &[Arc<RetainedStylesheet>],
        declaration: bool,
        origin: &SourceMapStack,
    ) -> Result<Vec<RenderPlanNode>, String> {
        use crate::instance_island::construct::{attribute, element};
        records
            .iter()
            .enumerate()
            .map(|(index, record)| {
                let source = if declaration {
                    origin.clone()
                } else {
                    self.maps[index].clone()
                };
                let css = record.emission()?.css();
                if css.to_ascii_lowercase().contains("</style") || css.contains('\0') {
                    return Err("Unsafe stylesheet raw-text export".into());
                }
                let attributes = if declaration {
                    let shared = match &record.identity().scope {
                        CssManagedScope::Shared { name, .. } => Some(name),
                        _ => None,
                    };
                    let mut attrs = vec![
                        attribute(
                            "data-cem-declaration-style",
                            if shared.is_some() {
                                "shared"
                            } else {
                                "private"
                            },
                            &source,
                        ),
                        attribute("data-cem-style-index", &index.to_string(), &source),
                        attribute("data-cem-style-key", &record.identity().cache_key, &source),
                    ];
                    if let Some(name) = shared {
                        attrs.push(attribute("data-cem-style-scope", name, &source));
                    }
                    attrs
                } else {
                    vec![attribute(
                        "data-cem-instance-style",
                        &index.to_string(),
                        &source,
                    )]
                };
                Ok(element(
                    "style",
                    cem_ml::schema::registry::HTML_NAMESPACE_URI,
                    None,
                    attributes,
                    vec![RenderPlanNode::Text {
                        text: css,
                        source_map: source.clone(),
                    }],
                    &source,
                ))
            })
            .collect()
    }
}
