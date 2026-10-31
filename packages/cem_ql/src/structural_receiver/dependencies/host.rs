//! Borrow the actual resolver and all its policies/contexts/grants. Only symbolic
//! name lookup and source membership use this attempt's checked catalog.
use super::*;
use crate::{datatype_names::DatatypeNameLookup, schema_references::CemQlSchemaReferenceNode};
use cem_ml::schema::datatype_registry::{DatatypeDependency, DatatypeDependencyHost};
pub(super) struct Host<'a> {
    pub inner: &'a mut CemQlSchemaDeclarationHost,
    pub catalog: &'a DatatypeNameCatalog,
    pub allowed: &'a BTreeMap<String, DatatypeSource>,
    pub captures: &'a [Arc<LexicallyScopedDocument>],
    pub lease: &'a AttributeDatatypeContextLease,
    pub closed: &'a AtomicBool,
    pub control: &'a OperationControl,
}
impl Host<'_> {
    pub fn check(&self) -> Result<(), ReferenceResolutionError> {
        self.control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| ReferenceResolutionError::OperationStopped)?;
        if self.closed.load(Ordering::Acquire) || !self.lease.is_live() {
            return Err(ReferenceResolutionError::OperationStopped);
        }
        Ok(())
    }
    pub fn ready(&self, node: &CemQlSchemaReferenceNode) -> bool {
        self.inner.scope(node).is_some_and(|scope| {
            self.inner
                .registered_scope_validator_context(scope)
                .is_some()
        })
    }
}
impl ReferenceResolutionHost for Host<'_> {
    type Node = CemQlSchemaReferenceNode;
    type Scope = Option<DeclarationScope>;
    fn prepare_node(&mut self, node: &mut Self::Node) -> Result<(), ReferenceResolutionError> {
        self.check()?;
        self.inner.prepare_node(node)
    }
    fn scope(&self, node: &Self::Node) -> Self::Scope {
        self.inner.scope(node)
    }
    fn scope_limits(&self, scope: &Self::Scope) -> ReferenceTraversalLimits {
        self.inner.scope_limits(scope)
    }
    fn reference_occurrence(&self, node: &Self::Node) -> Option<ReferenceOccurrence> {
        self.inner.reference_occurrence(node)
    }
    fn unresolved_policy(&self, node: &Self::Node) -> &ReferenceUnresolvedPolicy {
        self.inner.unresolved_policy(node)
    }
    fn permits_edge(&self, from: &Self::Node, to: &Self::Node) -> bool {
        self.inner.permits_edge(from, to)
    }
    fn evaluate(&mut self, node: &Self::Node) -> ReferenceLinkEvaluation<Self::Node> {
        if self.check().is_err() || !self.ready(node) {
            return ReferenceLinkEvaluation::Pending(REFUSED.into());
        }
        if let Some(source) = self.inner.declaration_node(node) {
            if matches!(source.node(), CemAstNode::Reference { .. }) {
                let saved = self
                    .captures
                    .iter()
                    .any(|c| c.snapshot(source.document(), source.node_id()).is_some());
                let prepared = self
                    .inner
                    .scope(node)
                    .is_some_and(|scope| self.inner.lexical_scope_parent(scope).is_some());
                if !saved || !prepared {
                    return ReferenceLinkEvaluation::Pending(REFUSED.into());
                }
            }
        }
        let result = self.inner.evaluate(node);
        if self.check().is_err() {
            return ReferenceLinkEvaluation::Pending(REFUSED.into());
        }
        result
    }
}
impl SchemaDeclarationHost for Host<'_> {
    fn compiled_attribute_types(&self) -> bool {
        SchemaDeclarationHost::compiled_attribute_types(self.inner)
    }
    fn declaration_name_metadata_required(&self, n: &SchemaDeclarationNode) -> bool {
        self.inner.declaration_name_metadata_required(n)
    }
    fn declaration_namespace_is_ready(&self, n: &SchemaDeclarationNode) -> bool {
        self.inner.declaration_namespace_is_ready(n)
    }
    fn input_consumed_namespace_attribute(&self, n: &SchemaDeclarationNode) -> bool {
        SchemaDeclarationHost::input_consumed_namespace_attribute(self.inner, n)
    }
    fn input_source_tree(&self, n: &SchemaDeclarationNode) -> Option<Arc<RetainedCemTree>> {
        SchemaDeclarationHost::input_source_tree(self.inner, n)
    }
    fn input_expanded_name<'a>(&'a self, n: &'a SchemaDeclarationNode) -> Option<&'a ExpandedName> {
        SchemaDeclarationHost::input_expanded_name(self.inner, n)
    }
    fn source_reference(&self, n: SchemaDeclarationNode) -> Self::Node {
        self.inner.source_reference(n)
    }
    fn declaration_node(&self, n: &Self::Node) -> Option<SchemaDeclarationNode> {
        self.inner.declaration_node(n)
    }
    fn declaration_schema(&self, n: &SchemaDeclarationNode) -> Option<SchemaDeclarationNode> {
        self.inner.declaration_schema(n)
    }
    fn structural_diagnostic(
        &self,
        n: &SchemaDeclarationNode,
        d: cem_ml::diagnostics::Diagnostic,
    ) -> cem_ml::diagnostics::Diagnostic {
        SchemaDeclarationHost::structural_diagnostic(self.inner, n, d)
    }
    fn evaluate_input_expression(&mut self, n: &Self::Node) -> ReferenceLinkEvaluation<Self::Node> {
        SchemaDeclarationHost::evaluate_input_expression(self.inner, n)
    }
}
impl DatatypeDependencyHost for Host<'_> {
    fn datatype_source(&self, n: &SchemaDeclarationNode) -> Option<DatatypeSource> {
        self.allowed.get(&n.identity()).cloned()
    }
    fn lookup_literal_type(
        &mut self,
        source: &DatatypeSource,
        _: &DatatypeDependency,
        qname: &str,
    ) -> ReferenceLinkEvaluation<Self::Node> {
        if self.check().is_err() {
            return ReferenceLinkEvaluation::Pending(REFUSED.into());
        }
        match self.catalog.lookup(source, qname) {
            DatatypeNameLookup::Target(t) => ReferenceLinkEvaluation::Resolved(vec![self
                .inner
                .source_reference(t.declaration().clone())]),
            DatatypeNameLookup::Pending(reason) => ReferenceLinkEvaluation::Pending(reason.into()),
            DatatypeNameLookup::Unresolved(reason) => {
                ReferenceLinkEvaluation::Unresolved(reason.into())
            }
        }
    }
}
