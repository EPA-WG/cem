//! Explicit value consumption for original captured native namespace properties.
use super::{
    CemQlSchemaDeclarationHost, CemQlSchemaReferenceNode, DeclarationScope,
    NamespaceScopePreparation,
};
use cem_ml::{
    parser::CemAstNode,
    schema::{
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        input_references::native_attribute_expression,
        namespace_references::{
            decode_native_namespace_property, NativeNamespaceProperty, NativeNamespacePropertyError,
        },
        reference_policy::{ReferenceOccurrence, ReferenceUnresolvedPolicy},
        reference_traversal::ReferenceTraversalLimits,
    },
    value::reference_resolution::{
        resolve_reference, ReferenceLinkEvaluation, ReferenceResolutionError,
        ReferenceResolutionHost,
    },
};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespacePropertyPreparationIssue {
    MetadataNotReady,
    Property(NativeNamespacePropertyError),
}
#[derive(Debug, Clone)]
pub struct NamespacePropertyPreparation {
    pub declaration: SchemaDeclarationNode,
    pub property: Option<NativeNamespaceProperty>,
    pub preparation: Option<NamespaceScopePreparation>,
    pub issue: Option<NamespacePropertyPreparationIssue>,
    pub(super) publication: Option<super::namespace_publication::NamespacePublicationProof>,
}
impl NamespacePropertyPreparation {
    pub fn is_ready(&self) -> bool {
        self.issue.is_none()
            && self.property.is_some()
            && self
                .preparation
                .as_ref()
                .is_some_and(NamespaceScopePreparation::is_ready)
    }
}
impl CemQlSchemaDeclarationHost {
    /// Consume a captured namespace property's original owning value slot at
    /// this explicit lifecycle stage. Original reference constructors use the
    /// shared resolver; a general expression uses the existing lifecycle hook.
    /// The entire selection/chain is one traversal with unchanged request and
    /// destination bounds, directed grants and source diagnostic policy.
    /// Require one ready original namespace target. Missing source metadata or
    /// runtime context, and unfinished selected bindings, remain inspectable.
    /// Caller-supplied occurrence scopes provide the pre-declaration context.
    /// This writes no source targets, completes no names, installs no scopes,
    /// evaluates no selected declaration's own slots and creates no grants.
    pub fn prepare_namespace_property(
        &mut self,
        declaration: SchemaDeclarationNode,
        limits: ReferenceTraversalLimits,
    ) -> Result<NamespacePropertyPreparation, ReferenceResolutionError> {
        if limits.max_depth == 0 || limits.max_work == 0 {
            return Err(ReferenceResolutionError::InvalidBounds);
        }
        let mut report = NamespacePropertyPreparation {
            declaration: declaration.clone(),
            property: None,
            preparation: None,
            issue: None,
            publication: None,
        };
        let key = Arc::as_ptr(declaration.document()) as usize;
        let Some(captured) = self.captured_namespaces.get(&key) else {
            report.issue = Some(NamespacePropertyPreparationIssue::MetadataNotReady);
            return Ok(report);
        };
        let property = match decode_native_namespace_property(declaration, captured) {
            Ok(property) => property,
            Err(issue) => {
                report.issue = Some(NamespacePropertyPreparationIssue::Property(issue));
                return Ok(report);
            }
        };
        let root = self.source_reference(property.value.clone());
        let expression = matches!(property.value.node(), CemAstNode::Element { .. })
            .then(|| property.value.clone());
        let mut consumer = NamespaceExpressionHost {
            host: self,
            expression,
            dependencies: vec![],
        };
        let selection = resolve_reference(root, &mut consumer, limits)?;
        let mut dependencies = consumer.dependencies;
        dependencies
            .sort_by_key(|(source, _)| (Arc::as_ptr(source.document()) as usize, source.node_id()));
        dependencies.dedup_by_key(|(source, _)| {
            (Arc::as_ptr(source.document()) as usize, source.node_id())
        });
        report.preparation = Some(self.admit_namespace_selection(selection));
        report.property = Some(property);
        if report.is_ready() {
            report.publication = Some(super::namespace_publication::NamespacePublicationProof {
                snapshot: self.namespace_input_snapshot,
                dependencies,
                declaration: report.declaration.clone(),
                selected: report
                    .preparation
                    .as_ref()
                    .unwrap()
                    .target
                    .as_ref()
                    .unwrap()
                    .selected
                    .clone(),
                target: cem_ml::schema::namespace_references::completed_namespace_scope_target(
                    report.property.as_ref().unwrap(),
                    report
                        .preparation
                        .as_ref()
                        .unwrap()
                        .target
                        .as_ref()
                        .unwrap(),
                ),
            });
        }
        Ok(report)
    }
}

/// Only this authored general value slot is an executable expression. Selected
/// expression-looking target nodes keep ordinary admission, not implicit execution.
struct NamespaceExpressionHost<'a> {
    host: &'a mut CemQlSchemaDeclarationHost,
    expression: Option<SchemaDeclarationNode>,
    dependencies: Vec<(SchemaDeclarationNode, Option<DeclarationScope>)>,
}
impl NamespaceExpressionHost<'_> {
    fn is_expression(&self, node: &CemQlSchemaReferenceNode) -> bool {
        self.expression
            .as_ref()
            .zip(node.source.as_ref())
            .is_some_and(|(expression, source)| {
                expression.node_id() == source.node_id()
                    && Arc::ptr_eq(expression.document(), source.document())
            })
    }
}
impl ReferenceResolutionHost for NamespaceExpressionHost<'_> {
    type Node = CemQlSchemaReferenceNode;
    type Scope = Option<DeclarationScope>;
    fn prepare_node(&mut self, node: &mut Self::Node) -> Result<(), ReferenceResolutionError> {
        self.host.prepare_node(node)?;
        if let Some(source) = node.source.as_ref() {
            self.dependencies
                .push((source.clone(), self.host.source_scope(source)));
            let key = (Arc::as_ptr(source.document()) as usize, source.node_id());
            if let Some(published) = self
                .host
                .namespace_publications
                .get(&key)
                .filter(|proof| proof.matches(self.host))
            {
                self.dependencies
                    .extend(published.dependencies.iter().cloned());
            }
        }
        Ok(())
    }
    fn scope(&self, node: &Self::Node) -> Self::Scope {
        self.host.scope(node)
    }
    fn scope_limits(&self, scope: &Self::Scope) -> ReferenceTraversalLimits {
        self.host.scope_limits(scope)
    }
    fn reference_occurrence(&self, node: &Self::Node) -> Option<ReferenceOccurrence> {
        if self.is_expression(node) {
            native_attribute_expression(node.source.as_ref().unwrap())
        } else {
            self.host.reference_occurrence(node)
        }
    }
    fn unresolved_policy(&self, node: &Self::Node) -> &ReferenceUnresolvedPolicy {
        self.host.unresolved_policy(node)
    }
    fn permits_edge(&self, from: &Self::Node, to: &Self::Node) -> bool {
        self.host.permits_edge(from, to)
    }
    fn evaluate(&mut self, node: &Self::Node) -> ReferenceLinkEvaluation<Self::Node> {
        if !self.is_expression(node) {
            return self.host.evaluate(node);
        }
        match self.host.evaluate_input_expression(node) {
            ReferenceLinkEvaluation::Invalid(diagnostics) => ReferenceLinkEvaluation::Invalid(
                diagnostics
                    .into_iter()
                    .map(|diagnostic| {
                        self.host
                            .structural_diagnostic(node.source.as_ref().unwrap(), diagnostic)
                    })
                    .collect(),
            ),
            result => result,
        }
    }
}
