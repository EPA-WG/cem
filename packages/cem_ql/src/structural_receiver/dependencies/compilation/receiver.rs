//! Bind an original type occurrence using the actual resolver and its grants.
use super::*;
use crate::attribute_datatypes::{AttributeDatatypeHost, BoundAttributeDatatype};
struct Host<'a> {
    inner: &'a mut CemQlSchemaDeclarationHost,
    expected: DatatypeSource,
}
impl ReferenceResolutionHost for Host<'_> {
    type Node = CemQlSchemaReferenceNode;
    type Scope = Option<DeclarationScope>;
    fn prepare_node(&mut self, n: &mut Self::Node) -> Result<(), ReferenceResolutionError> {
        self.inner.prepare_node(n)
    }
    fn scope(&self, n: &Self::Node) -> Self::Scope {
        self.inner.scope(n)
    }
    fn scope_limits(&self, s: &Self::Scope) -> ReferenceTraversalLimits {
        self.inner.scope_limits(s)
    }
    fn reference_occurrence(&self, n: &Self::Node) -> Option<ReferenceOccurrence> {
        self.inner.reference_occurrence(n)
    }
    fn unresolved_policy(&self, n: &Self::Node) -> &ReferenceUnresolvedPolicy {
        self.inner.unresolved_policy(n)
    }
    fn permits_edge(&self, f: &Self::Node, t: &Self::Node) -> bool {
        self.inner.permits_edge(f, t)
    }
    fn evaluate(&mut self, n: &Self::Node) -> ReferenceLinkEvaluation<Self::Node> {
        self.inner.evaluate(n)
    }
}
impl SchemaDeclarationHost for Host<'_> {
    fn source_reference(&self, n: SchemaDeclarationNode) -> Self::Node {
        self.inner.source_reference(n)
    }
    fn declaration_node(&self, n: &Self::Node) -> Option<SchemaDeclarationNode> {
        self.inner.declaration_node(n)
    }
    fn declaration_schema(&self, n: &SchemaDeclarationNode) -> Option<SchemaDeclarationNode> {
        self.inner.declaration_schema(n)
    }
    fn input_expanded_name<'a>(&'a self, n: &'a SchemaDeclarationNode) -> Option<&'a ExpandedName> {
        SchemaDeclarationHost::input_expanded_name(self.inner, n)
    }
    fn input_consumed_namespace_attribute(&self, n: &SchemaDeclarationNode) -> bool {
        SchemaDeclarationHost::input_consumed_namespace_attribute(self.inner, n)
    }
}
impl cem_ml::schema::datatype_registry::DatatypeDependencyHost for Host<'_> {
    fn datatype_source(&self, n: &SchemaDeclarationNode) -> Option<DatatypeSource> {
        (n.identity() == self.expected.declaration().identity()).then(|| self.expected.clone())
    }
}
impl AttributeDatatypeHost for Host<'_> {
    fn lookup_attribute_type(
        &mut self,
        a: &SchemaDeclarationNode,
        q: &str,
    ) -> ReferenceLinkEvaluation<Self::Node> {
        self.inner.lookup_attribute_type(a, q)
    }
}
impl FreshDependencyDatatypeCompilation {
    pub(in crate::structural_receiver) fn bind_original_receiver(
        &self,
        host: &mut FreshStructuralHost,
        root: AstNodeId,
        declaration: SchemaDeclarationNode,
    ) -> Result<BoundAttributeDatatype, String> {
        self.check(host)?;
        if !matches!(self.inputs, Inputs::Assembly(_)) || self.executing.load(Ordering::Acquire) {
            return Err(REFUSED.into());
        }
        let expected = self.root_descriptor(root)?.source().clone();
        let mut adapter = Host {
            inner: &mut host.inner,
            expected,
        };
        let result = crate::attribute_datatypes::bind_attribute_datatype(
            declaration,
            &self.compilation,
            &mut adapter,
            ReferenceTraversalLimits {
                max_depth: 64,
                max_work: 100_000,
            },
        )
        .map_err(|_| REFUSED)?;
        self.check(host)?;
        if result.state != ReferenceResolutionState::Resolved
            || result.issue.is_some()
            || result
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation())
        {
            return Err(REFUSED.into());
        }
        result.bound.ok_or_else(|| REFUSED.into())
    }
}
