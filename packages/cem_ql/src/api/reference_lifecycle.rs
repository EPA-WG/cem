//! Explicit, heap-local schema/namespace consumer of retained reference sources.
//! Sessions contain host inputs and authority; reload bundles contain neither.
use super::{reference_transport::RetainedReferenceSource, StandaloneExpressionContext};
use crate::schema_references::{
    CemQlSchemaDeclarationHost, DeclarationScope, NamespaceLifecycleSnapshot,
    SchemaHostRuntimeContextRequest,
};
use cem_ml::{
    diagnostics::Diagnostic,
    parser::{AstNodeId, CemAstNode},
    schema::{
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        reference_policy::{ReferenceScopePolicy, ReferenceScopePolicyOverrides},
    },
    source_map::SourceMapStack,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, serde::Serialize)]
pub enum ReferenceConsumerDependencyKind {
    MissingLexicalMetadata,
    NamespaceNotReady,
    SchemaModelNotReady,
    SchemaRegionNotReady,
    Pending,
    Unresolved,
    Cycle,
    DepthLimit,
    WorkLimit,
    ScopeDenied,
    Invalid,
}
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceConsumerDependency {
    pub kind: ReferenceConsumerDependencyKind,
    pub reason: String,
    pub source_uri: String,
    pub node_id: Option<AstNodeId>,
    pub source_map: Option<SourceMapStack>,
}
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceConsumerReport {
    pub complete: bool,
    pub failed: bool,
    pub dependencies: Vec<ReferenceConsumerDependency>,
    pub diagnostics: Vec<Diagnostic>,
    pub placements: usize,
}
impl ReferenceConsumerReport {
    fn pending() -> Self {
        Self {
            complete: false,
            failed: false,
            dependencies: vec![],
            diagnostics: vec![],
            placements: 0,
        }
    }
}
#[derive(Debug, Clone)]
struct SourceInputs {
    source: RetainedReferenceSource,
    context: Option<StandaloneExpressionContext>,
    policy: Option<ReferenceScopePolicy>,
    limits: Option<cem_ml::schema::reference_traversal::ReferenceTraversalLimits>,
}
/// Source indices are local host handles (input=0, consuming schema=1), never
/// authored IDs. Each run builds fresh scopes over the retained owners. Context
/// readiness, destination policy and directed grants are explicit host choices.
#[derive(Debug, Clone)]
pub struct ReferenceValidationSession {
    sources: Vec<SourceInputs>,
    crossings: BTreeSet<(usize, usize)>,
}
impl ReferenceValidationSession {
    pub fn new(source: RetainedReferenceSource, schema: RetainedReferenceSource) -> Self {
        Self {
            sources: vec![
                SourceInputs {
                    source,
                    context: None,
                    policy: None,
                    limits: None,
                },
                SourceInputs {
                    source: schema,
                    context: None,
                    policy: None,
                    limits: None,
                },
            ],
            crossings: BTreeSet::new(),
        }
    }
    pub fn add_source(&mut self, source: RetainedReferenceSource) -> usize {
        // One owner has one relationship boundary in this session.
        if let Some(index) = self.sources.iter().position(|s| {
            std::sync::Arc::ptr_eq(
                s.source.ingress().source().ast_owner(),
                source.ingress().source().ast_owner(),
            )
        }) {
            return index;
        }
        let index = self.sources.len();
        self.sources.push(SourceInputs {
            source,
            context: None,
            policy: None,
            limits: None,
        });
        index
    }
    pub fn set_context(
        &mut self,
        source: usize,
        context: Option<StandaloneExpressionContext>,
    ) -> Result<(), String> {
        self.sources
            .get_mut(source)
            .ok_or("Unknown session source")?
            .context = context;
        Ok(())
    }
    pub fn set_policy(
        &mut self,
        source: usize,
        policy: ReferenceScopePolicy,
    ) -> Result<(), String> {
        if policy.limits.max_depth == 0 || policy.limits.max_work == 0 {
            return Err("Invalid reference traversal bounds".into());
        }
        self.sources
            .get_mut(source)
            .ok_or("Unknown session source")?
            .policy = Some(policy);
        Ok(())
    }
    /// Override only bounds; retain the consuming schema's unresolved policy.
    pub fn set_limits(
        &mut self,
        source: usize,
        limits: cem_ml::schema::reference_traversal::ReferenceTraversalLimits,
    ) -> Result<(), String> {
        if limits.max_depth == 0 || limits.max_work == 0 {
            return Err("Invalid reference traversal bounds".into());
        }
        self.sources
            .get_mut(source)
            .ok_or("Unknown session source")?
            .limits = Some(limits);
        Ok(())
    }
    pub fn allow_crossing(&mut self, from: usize, to: usize) -> Result<(), String> {
        if from >= self.sources.len() || to >= self.sources.len() {
            return Err("Unknown session source".into());
        }
        self.crossings.insert((from, to));
        Ok(())
    }
    fn context_for(&self, source: &SchemaDeclarationNode) -> Option<StandaloneExpressionContext> {
        self.sources
            .iter()
            .find(|s| {
                std::sync::Arc::ptr_eq(s.source.ingress().source().ast_owner(), source.document())
            })
            .and_then(|s| s.context.clone())
    }
    fn host(
        &self,
        input_policy: &ReferenceScopePolicy,
    ) -> Result<CemQlSchemaDeclarationHost, String> {
        let mut host = CemQlSchemaDeclarationHost::new();
        let mut scopes: Vec<DeclarationScope> = vec![];
        for (index, inputs) in self.sources.iter().enumerate() {
            let mut policy = inputs.policy.clone().unwrap_or_else(|| {
                if index == 0 {
                    input_policy.clone()
                } else {
                    ReferenceScopePolicy::schema_defaults().expect("embedded policy")
                }
            });
            if let Some(limits) = inputs.limits {
                policy.limits = limits;
            }
            let scope = host.register_scope(
                inputs.source.ingress().source().clone(),
                inputs.context.clone(),
                policy,
            );
            scopes.push(scope);
            if let Ok(capture) = inputs.source.require_lexical() {
                host.attach_captured_namespaces(capture.clone())
                    .map_err(|e| format!("{e:?}"))?;
                host.attach_captured_lexical_scopes_with_policy_overrides(capture, |_, _, _| {
                    (inputs.context.clone(), Default::default())
                })
                .map_err(|e| format!("{e:?}"))?;
            }
        }
        for &(from, to) in &self.crossings {
            host.allow_scope_crossing(scopes[from], scopes[to]);
        }
        Ok(host)
    }
    fn namespace_dependencies(
        report: &mut ReferenceConsumerReport,
        snapshot: &NamespaceLifecycleSnapshot,
        source: &RetainedReferenceSource,
    ) {
        for (node, issue) in &snapshot.incomplete_roots {
            report.dependencies.push(ReferenceConsumerDependency {
                kind: ReferenceConsumerDependencyKind::NamespaceNotReady,
                reason: format!("{issue:?}"),
                source_uri: source
                    .ingress()
                    .source()
                    .node_source_uri(*node)
                    .unwrap_or(source.ingress().source().source_uri())
                    .into(),
                node_id: Some(*node),
                source_map: source
                    .ingress()
                    .source()
                    .ast()
                    .get(*node)
                    .map(|n| node_source_map(n).clone()),
            });
        }
        report.failed |= report
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation());
        for property in &snapshot.properties {
            if let Some(preparation) = &property.preparation {
                report.failed |= preparation.selection.failed;
                report
                    .diagnostics
                    .extend(preparation.selection.diagnostics.clone());
            }
        }
    }
    pub fn run(&self) -> Result<ReferenceConsumerReport, String> {
        let mut report = ReferenceConsumerReport::pending();
        for inputs in &self.sources {
            let tree = inputs.source.ingress().source();
            report.diagnostics.extend(
                tree.ast()
                    .diagnostics
                    .iter()
                    .chain(
                        inputs
                            .source
                            .require_lexical()
                            .ok()
                            .into_iter()
                            .flat_map(|capture| capture.diagnostics()),
                    )
                    .cloned()
                    .map(|mut d| {
                        d.uri.get_or_insert_with(|| tree.source_uri().into());
                        d
                    }),
            );
        }
        report.failed |= report
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation());
        // Capture is required for every registered source which this execution
        // might select. An AST-only inspection never invents lexical defaults.
        for inputs in &self.sources {
            if inputs.source.require_lexical().is_err() {
                report.dependencies.push(ReferenceConsumerDependency {
                    kind: ReferenceConsumerDependencyKind::MissingLexicalMetadata,
                    reason: "Verified lexical capture is required".into(),
                    source_uri: inputs.source.ingress().source().source_uri().into(),
                    node_id: None,
                    source_map: None,
                });
            }
        }
        if !report.dependencies.is_empty() {
            return Ok(report);
        }
        let defaults = ReferenceScopePolicy::schema_defaults().map_err(|e| format!("{e:?}"))?;
        let schema = &self.sources[1].source;
        let mut host = self.host(&defaults)?;
        let schema_capture = schema.require_lexical().unwrap().clone();
        let schema_limits = self.sources[1]
            .limits
            .unwrap_or(self.sources[1].policy.as_ref().unwrap_or(&defaults).limits);
        let (namespace, model) = host
            .with_namespace_lifecycle(
                schema_capture,
                &roots(schema),
                schema_limits,
                |node, _, _, _| {
                    (
                        self.context_for(node),
                        ReferenceScopePolicyOverrides::default(),
                    )
                },
                |host, names| {
                    if names.is_complete() {
                        Some(host.compile(
                            schema.ingress().source().source_uri(),
                            schema.ingress().source().clone(),
                            schema_limits,
                        ))
                    } else {
                        None
                    }
                },
            )
            .map_err(|e| format!("{e:?}"))?;
        Self::namespace_dependencies(&mut report, &namespace, schema);
        let Some(model) = model else {
            return Ok(report);
        };
        let model = model.map_err(|e| format!("{e:?}"))?;
        report.diagnostics.extend(model.compile_diagnostics.clone());
        if !model.is_ready_for_validation()
            || model
                .compile_diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation())
        {
            report.dependencies.push(ReferenceConsumerDependency {
                kind: ReferenceConsumerDependencyKind::SchemaModelNotReady,
                reason: "Consuming schema compilation is incomplete or invalid".into(),
                source_uri: schema.ingress().source().source_uri().into(),
                node_id: None,
                source_map: None,
            });
            report.failed |= model.declaration_references.failed();
            report.failed |= report
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation());
            return Ok(report);
        }
        let mut policy = self.sources[0]
            .policy
            .clone()
            .unwrap_or(defaults.for_scope(&model).map_err(|e| format!("{e:?}"))?);
        if let Some(limits) = self.sources[0].limits {
            policy.limits = limits;
        }
        // Separate fresh compilation and consumption hosts keep invocation-local
        // completed namespace names and scope publications from escaping.
        let mut host = self.host(&policy)?;
        let input = &self.sources[0].source;
        let (namespace, validation) = host
            .with_namespace_lifecycle(
                input.require_lexical().unwrap().clone(),
                &roots(input),
                policy.limits,
                |node, _, _, _| (self.context_for(node), Default::default()),
                |host, names| {
                    host.validate_input_runtime_host_regions(
                        schema.ingress().source().source_uri(),
                        input.ingress().source().clone(),
                        &names.ready_roots,
                        &model,
                        policy.limits,
                        |request| match request {
                            SchemaHostRuntimeContextRequest::Body(region) => {
                                self.context_for(region.contract.host())
                            }
                            SchemaHostRuntimeContextRequest::Occurrence { source, .. } => {
                                self.context_for(source)
                            }
                        },
                    )
                },
            )
            .map_err(|e| format!("{e:?}"))?;
        Self::namespace_dependencies(&mut report, &namespace, input);
        let validation = validation.map_err(|e| format!("{e:?}"))?;
        for inputs in &validation.inputs {
            if !inputs.is_ready() {
                let source = inputs.region().contract.host();
                report.dependencies.push(ReferenceConsumerDependency {
                    kind: ReferenceConsumerDependencyKind::SchemaRegionNotReady,
                    reason: format!("{:?}", inputs.issue()),
                    source_uri: self
                        .sources
                        .iter()
                        .find(|s| {
                            std::sync::Arc::ptr_eq(
                                s.source.ingress().source().ast_owner(),
                                source.document(),
                            )
                        })
                        .and_then(|s| {
                            s.source
                                .ingress()
                                .source()
                                .node_source_uri(source.node_id())
                        })
                        .unwrap_or(input.ingress().source().source_uri())
                        .into(),
                    node_id: Some(source.node_id()),
                    source_map: Some(node_source_map(source.node()).clone()),
                });
            }
        }
        for structure in &validation.validation.references {
            for issue in &structure.resolution.issues {
                use cem_ml::value::reference_resolution::ReferenceResolutionIssueKind as Issue;
                let source = host.declaration_node(&issue.reference);
                let kind = match issue.kind {
                    Issue::Pending => ReferenceConsumerDependencyKind::Pending,
                    Issue::Unresolved => ReferenceConsumerDependencyKind::Unresolved,
                    Issue::Cycle => ReferenceConsumerDependencyKind::Cycle,
                    Issue::DepthLimit => ReferenceConsumerDependencyKind::DepthLimit,
                    Issue::WorkLimit => ReferenceConsumerDependencyKind::WorkLimit,
                    Issue::ScopeDenied => ReferenceConsumerDependencyKind::ScopeDenied,
                    Issue::Invalid => ReferenceConsumerDependencyKind::Invalid,
                };
                let source_uri = source
                    .as_ref()
                    .and_then(|node| {
                        host.input_source_tree(node).and_then(|tree| {
                            tree.node_source_uri(node.node_id()).map(str::to_owned)
                        })
                    })
                    .unwrap_or_else(|| input.ingress().source().source_uri().into());
                report.dependencies.push(ReferenceConsumerDependency {
                    kind,
                    reason: issue.reason.clone(),
                    source_uri,
                    node_id: source.as_ref().map(|node| node.node_id()),
                    source_map: source
                        .as_ref()
                        .map(|node| node_source_map(node.node()).clone()),
                });
            }
        }
        report.failed |= validation.validation.failed;
        report.complete = namespace.is_complete() && validation.validation.complete;
        report.placements = validation.validation.nodes.len();
        report.diagnostics.extend(validation.validation.diagnostics);
        report.failed |= report
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation());
        Ok(report)
    }
}
fn roots(source: &RetainedReferenceSource) -> Vec<AstNodeId> {
    match source.ingress().source().ast().get(0) {
        Some(CemAstNode::Document { root_children, .. }) => root_children.clone(),
        _ => vec![0],
    }
}

fn node_source_map(node: &CemAstNode) -> &SourceMapStack {
    match node {
        CemAstNode::Document { source, .. }
        | CemAstNode::Element { source, .. }
        | CemAstNode::Attribute { source, .. }
        | CemAstNode::Text { source, .. }
        | CemAstNode::Whitespace { source, .. }
        | CemAstNode::Comment { source, .. }
        | CemAstNode::ProcessingInstruction { source, .. }
        | CemAstNode::Cdata { source, .. }
        | CemAstNode::RawText { source, .. }
        | CemAstNode::Error { source, .. }
        | CemAstNode::Reference { source, .. } => source,
    }
}
