//! Explicit schema consumer of typed targets in `{elements}` and `{attributes}`.
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

#[derive(Debug, Clone)]
pub struct DeclarationReferenceSite {
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
    pub fn failed(&self) -> bool {
        self.sites
            .iter()
            .any(|site| site.resolution.as_ref().is_some_and(|result| result.failed))
    }
    pub(crate) fn retain_pending(&mut self, schema_uri: &str, node: &CemAstNode) {
        static NEXT_OCCURRENCE: AtomicU64 = AtomicU64::new(1);
        if let CemAstNode::Reference {
            node_id,
            expression,
            source,
            ..
        } = node
        {
            self.sites.push(DeclarationReferenceSite {
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
/// named declarations of the collection's kind, preserving insertion order and
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
    for (id, expected_kind) in document_model::declaration_reference_sites(&document) {
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
                match expected_kind {
                    "element" => document_model::compile_element_model(
                        target.document(),
                        target.node_id(),
                        &aliases,
                        &mut seen,
                    )
                    .map(CompiledSchemaDeclaration::Element),
                    "attribute" => {
                        document_model::compile_attribute_model(target.document(), target.node_id())
                            .map(|attribute| {
                                CompiledSchemaDeclaration::Attribute(Box::new(attribute))
                            })
                    }
                    _ => unreachable!("supported collection discovery selects the target kind"),
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
                    &format!("Expected a named {expected_kind} declaration"),
                ));
            }
        }
        declarations.insert(id, compiled_targets);
        compilation.sites.push(DeclarationReferenceSite {
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
