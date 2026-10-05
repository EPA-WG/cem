//! Explicit schema consumer of retained declaration collection references.
//! The host supplies runtime evaluation and original lexical schema bindings.
//! No source arena is cloned or rewritten to inline referenced declarations.
use super::{
    document_model::{self, CompiledSchemaDeclaration, SchemaDocumentModel},
    reference_policy::ReferenceOccurrence,
    reference_traversal::ReferenceTraversalLimits,
};
use crate::{
    diagnostics::{Diagnostic, Severity},
    parser::{document::CemDocument, AstNodeId, CemAstNode},
    source_map::FrameSpan,
    value::reference_resolution::{
        resolve_reference, ReferenceResolutionError, ReferenceResolutionHost,
        ReferenceResolutionIssueKind, ReferenceResolutionState,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

pub const INVALID_REFERENCE_TARGET: &str = "cem.schema_definition.invalid_reference_target";

/// A validated immutable arena handle with its storage owner. Node IDs are
/// arena addresses, not authored IDs or runtime evaluation context identities.
#[derive(Debug, Clone)]
pub struct SchemaDeclarationNode {
    document: Arc<CemDocument>,
    node_id: AstNodeId,
}
impl SchemaDeclarationNode {
    pub fn new(document: Arc<CemDocument>, node_id: AstNodeId) -> Option<Self> {
        document.get(node_id)?;
        Some(Self { document, node_id })
    }
    pub fn document(&self) -> &Arc<CemDocument> {
        &self.document
    }
    pub fn node_id(&self) -> AstNodeId {
        self.node_id
    }
    pub fn node(&self) -> &CemAstNode {
        self.document.get(self.node_id).unwrap()
    }
    pub fn identity(&self) -> String {
        format!(
            "schema-node:{:p}:{}",
            Arc::as_ptr(&self.document),
            self.node_id
        )
    }
}

pub trait SchemaDeclarationHost: ReferenceResolutionHost {
    /// Adapt retained source to the host's typed runtime representation.
    fn source_reference(&self, source: SchemaDeclarationNode) -> Self::Node;
    /// Borrow an arena declaration through its original owner; never synthesize
    /// an arena to represent a runtime-constructed reference or copy a target.
    fn declaration_node(&self, target: &Self::Node) -> Option<SchemaDeclarationNode>;
    /// Original lexical schema for this declaration, in the same retained
    /// document. None denotes a declaration without schema alias bindings.
    /// Scope selection must not be inferred from the consuming schema's aliases.
    fn declaration_schema(&self, target: &SchemaDeclarationNode) -> Option<SchemaDeclarationNode>;
    /// Explicit input lifecycle hook; declaration compilation never invokes it.
    /// Hosts without an expression consumer preserve the pending source slot.
    fn evaluate_input_expression(
        &mut self,
        _expression: &Self::Node,
    ) -> crate::value::reference_resolution::ReferenceLinkEvaluation<Self::Node> {
        crate::value::reference_resolution::ReferenceLinkEvaluation::Pending(
            "input-expression-consumer-not-ready".into(),
        )
    }
}

#[derive(Debug, Clone)]
pub struct DeclarationReferenceIssue {
    pub occurrence: ReferenceOccurrence,
    pub kind: ReferenceResolutionIssueKind,
    pub reason: String,
}
/// Consumed schema outcome. Retained declaration handles preserve target owners;
/// traversal issue metadata preserves the original failing link provenance.
#[derive(Debug, Clone)]
pub struct DeclarationReferenceResolution {
    pub nodes: Vec<SchemaDeclarationNode>,
    pub state: ReferenceResolutionState,
    pub failed: bool,
    pub issues: Vec<DeclarationReferenceIssue>,
    pub diagnostics: Vec<Diagnostic>,
    pub work_used: usize,
}

/// The consuming collection owns target-kind and key requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaDeclarationKind {
    Element,
    Attribute,
    Behavior,
    Diagnostic,
    Constraint,
    FieldContract,
}
impl SchemaDeclarationKind {
    pub(crate) fn node_name(self) -> &'static str {
        match self {
            Self::Element => "element",
            Self::Attribute => "attribute",
            Self::Behavior => "behavior",
            Self::Diagnostic => "diagnostic",
            Self::Constraint => "constraint",
            Self::FieldContract => "field-contract",
        }
    }
}
#[derive(Debug, Clone)]
pub struct DeclarationReferenceSite {
    pub kind: SchemaDeclarationKind,
    pub occurrence: ReferenceOccurrence,
    /// None means compilation retained source without requesting evaluation.
    /// Some(Pending) means the host was called but its inputs were not ready.
    pub resolution: Option<DeclarationReferenceResolution>,
}
impl DeclarationReferenceSite {
    pub fn state(&self) -> ReferenceResolutionState {
        self.resolution
            .as_ref()
            .map(|result| result.state)
            .unwrap_or(ReferenceResolutionState::Pending)
    }
}

#[derive(Debug, Clone, Default)]
pub struct DeclarationReferenceCompilation {
    pub sites: Vec<DeclarationReferenceSite>,
}
impl DeclarationReferenceCompilation {
    pub fn state(&self) -> ReferenceResolutionState {
        let states: Vec<_> = self
            .sites
            .iter()
            .map(DeclarationReferenceSite::state)
            .collect();
        [
            ReferenceResolutionState::Invalid,
            ReferenceResolutionState::Unresolved,
            ReferenceResolutionState::Pending,
        ]
        .into_iter()
        .find(|state| states.contains(state))
        .unwrap_or(ReferenceResolutionState::Resolved)
    }
    pub fn is_complete(&self) -> bool {
        self.state() == ReferenceResolutionState::Resolved
    }
    pub(crate) fn collection_is_complete(&self, kind: SchemaDeclarationKind) -> bool {
        self.sites
            .iter()
            .filter(|site| site.kind == kind)
            .all(|site| {
                site.state() == ReferenceResolutionState::Resolved
                    && !site.resolution.as_ref().is_some_and(|r| r.failed)
            })
    }
    pub fn failed(&self) -> bool {
        self.sites
            .iter()
            .any(|site| site.resolution.as_ref().is_some_and(|result| result.failed))
    }
    pub(crate) fn retain_pending(
        &mut self,
        schema_uri: &str,
        node: &CemAstNode,
        kind: SchemaDeclarationKind,
    ) {
        static NEXT_OCCURRENCE: AtomicU64 = AtomicU64::new(1);
        if let CemAstNode::Reference {
            node_id,
            expression,
            source,
            ..
        } = node
        {
            self.sites.push(DeclarationReferenceSite {
                kind,
                occurrence: ReferenceOccurrence {
                    identity: format!(
                        "pending-schema:{}:{schema_uri}:{node_id}",
                        NEXT_OCCURRENCE.fetch_add(1, Ordering::Relaxed)
                    ),
                    node_id: Some(*node_id),
                    expression: Some(expression.clone()),
                    source_map: source.clone(),
                },
                resolution: None,
            });
        }
    }
}

/// Explicit compilation stage. Collection references accept zero or more
/// keyed declarations of the collection's kind, preserving insertion order and
/// last-name-wins collection behavior. Available valid targets are compiled
/// even when another branch is incomplete; outcomes remain visibly incomplete.
/// Invalid target kinds/names are compile errors irrespective of disposition.
/// Callers must inspect completeness and compile diagnostics before publishing
/// or treating a model as ready. This does not schedule retries or evaluate URLs.
pub fn compile_schema_with_declaration_references<H: SchemaDeclarationHost>(
    schema_uri: &str,
    document: Arc<CemDocument>,
    host: &mut H,
    limits: ReferenceTraversalLimits,
) -> Result<SchemaDocumentModel, ReferenceResolutionError> {
    let mut declarations: BTreeMap<AstNodeId, Vec<CompiledSchemaDeclaration>> = BTreeMap::new();
    let mut compilation = DeclarationReferenceCompilation::default();
    let mut seen = BTreeSet::from([schema_uri.to_owned()]);
    for (id, kind) in document_model::declaration_reference_sites(&document) {
        let expected_kind = kind.node_name();
        let reference =
            host.source_reference(SchemaDeclarationNode::new(document.clone(), id).unwrap());
        let occurrence = host
            .reference_occurrence(&reference)
            .ok_or(ReferenceResolutionError::NotReference)?;
        let evaluated = resolve_reference(reference, host, limits)?;
        let mut resolution = DeclarationReferenceResolution {
            nodes: vec![],
            state: evaluated.state,
            failed: evaluated.failed,
            issues: evaluated
                .issues
                .into_iter()
                .map(|issue| DeclarationReferenceIssue {
                    occurrence: issue.occurrence,
                    kind: issue.kind,
                    reason: issue.reason,
                })
                .collect(),
            diagnostics: evaluated.diagnostics,
            work_used: evaluated.work_used,
        };
        let mut compiled_targets = vec![];
        for value in &evaluated.nodes {
            let Some(target) = host.declaration_node(value) else {
                resolution.failed = true;
                resolution.state = ReferenceResolutionState::Invalid;
                resolution.diagnostics.push(invalid_target(
                    schema_uri,
                    &occurrence,
                    None,
                    &format!("Expected a retained {expected_kind} declaration target"),
                ));
                continue;
            };
            resolution.nodes.push(target.clone());
            let lexical_schema = host.declaration_schema(&target);
            let declaring_uri = lexical_schema
                .as_ref()
                .and_then(|schema| {
                    document_model::declaration_schema_uri(schema.document(), schema.node_id())
                })
                .unwrap_or_else(|| schema_uri.into());
            let aliases = match lexical_schema {
                Some(schema)
                    if Arc::ptr_eq(schema.document(), target.document())
                        && is_element(schema.node(), "schema") =>
                {
                    document_model::collect_schema_uses(schema.document(), schema.node_id())
                }
                Some(_) => {
                    resolution.failed = true;
                    resolution.state = ReferenceResolutionState::Invalid;
                    resolution.diagnostics.push(invalid_target(
                        schema_uri,
                        &occurrence,
                        Some(&target),
                        "Invalid declaring lexical schema owner or kind",
                    ));
                    continue;
                }
                None => BTreeMap::new(),
            };
            let compiled = if is_element(target.node(), expected_kind) {
                match kind {
                    SchemaDeclarationKind::Element => document_model::compile_element_model(
                        target.document(),
                        target.node_id(),
                        &aliases,
                        &mut seen,
                    )
                    .map(CompiledSchemaDeclaration::Element),
                    SchemaDeclarationKind::Attribute => {
                        document_model::compile_attribute_model(target.document(), target.node_id())
                            .map(|attribute| {
                                CompiledSchemaDeclaration::Attribute(Box::new(attribute))
                            })
                    }
                    SchemaDeclarationKind::Behavior => document_model::compile_behavior_definition(
                        target.document(),
                        target.node_id(),
                        &declaring_uri,
                        &aliases,
                    )
                    .map(|behavior| CompiledSchemaDeclaration::Behavior(Box::new(behavior))),
                    SchemaDeclarationKind::FieldContract => {
                        document_model::compile_field_contract_declaration(
                            target.clone(),
                            aliases.clone(),
                        )
                        .map(|contract| {
                            CompiledSchemaDeclaration::FieldContract(Box::new(contract))
                        })
                    }
                    SchemaDeclarationKind::Constraint => {
                        document_model::compile_constraint_declaration(
                            target.clone(),
                            aliases.clone(),
                        )
                        .map(|constraint| {
                            CompiledSchemaDeclaration::Constraint(Box::new(constraint))
                        })
                    }
                    SchemaDeclarationKind::Diagnostic => {
                        document_model::compile_diagnostic_declaration(
                            target.document(),
                            target.node_id(),
                            &aliases,
                        )
                        .map(|diagnostic| {
                            CompiledSchemaDeclaration::Diagnostic(Box::new(diagnostic))
                        })
                    }
                }
            } else {
                None
            };
            if let Some(declaration) = compiled {
                compiled_targets.push(declaration);
            } else {
                resolution.failed = true;
                resolution.state = ReferenceResolutionState::Invalid;
                resolution.diagnostics.push(invalid_target(
                    schema_uri,
                    &occurrence,
                    Some(&target),
                    &format!("Expected a complete {expected_kind} declaration with its required key and fields"),
                ));
            }
        }
        declarations.insert(id, compiled_targets);
        compilation.sites.push(DeclarationReferenceSite {
            kind,
            occurrence,
            resolution: Some(resolution),
        });
    }
    Ok(document_model::compile_document_model_with_declarations(
        schema_uri,
        &document,
        &declarations,
        compilation,
    ))
}
fn is_element(node: &CemAstNode, local_name: &str) -> bool {
    matches!(node, CemAstNode::Element {expanded_name, ..} if expanded_name.local_name == local_name)
}
fn invalid_target(
    schema_uri: &str,
    occurrence: &ReferenceOccurrence,
    target: Option<&SchemaDeclarationNode>,
    reason: &str,
) -> Diagnostic {
    Diagnostic {
        code: INVALID_REFERENCE_TARGET.into(),
        severity: Severity::Error,
        message: format!(
            "{reason} in {}",
            occurrence
                .expression
                .as_deref()
                .unwrap_or(&occurrence.identity)
        ),
        uri: Some(schema_uri.into()),
        node: Some(occurrence.identity.clone()),
        source_map: Some(occurrence.source_map.clone()),
        byte_offset: occurrence
            .source_map
            .origin()
            .and_then(|frame| match &frame.span {
                FrameSpan::Single(range) => Some(range.start),
                FrameSpan::Multi(ranges) => ranges.first().map(|range| range.start),
            }),
        details: Some(
            serde_json::json!({"target": target.map(SchemaDeclarationNode::identity), "target_node_id": target.map(SchemaDeclarationNode::node_id)}),
        ),
        ..Diagnostic::default()
    }
}
