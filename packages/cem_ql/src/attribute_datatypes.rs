//! Explicit lifecycle binding of original attribute type slots. Binding does not
//! prepare values, invoke conversion or make a schema model ready by itself.
use crate::datatype_compilation::ExecutableDatatype;
use cem_ml::{
    parser::CemAstNode,
    schema::{
        datatype_contracts::DatatypeCompilation,
        datatype_registry::DatatypeDependencyHost,
        declaration_references::SchemaDeclarationNode,
        reference_policy::{ReferenceOccurrence, ReferenceUnresolvedPolicy},
        reference_traversal::ReferenceTraversalLimits,
    },
    value::reference_resolution::{
        resolve_reference, ReferenceLinkEvaluation, ReferenceResolutionError,
        ReferenceResolutionHost, ReferenceResolutionState,
    },
};
use std::sync::Arc;

pub trait AttributeDatatypeHost: DatatypeDependencyHost {
    /// Host-supplied binding of this exact original literal slot. Never borrow
    /// another declaration's prefix map or synthesize an AST reference.
    fn lookup_attribute_type(
        &mut self,
        attribute: &SchemaDeclarationNode,
        qname: &str,
    ) -> ReferenceLinkEvaluation<Self::Node>;
}
#[derive(Debug, Clone)]
pub struct BoundAttributeDatatype {
    pub declaration: SchemaDeclarationNode,
    pub slot: SchemaDeclarationNode,
    pub datatype: Arc<ExecutableDatatype>,
}
#[derive(Debug, Clone)]
pub struct AttributeDatatypeBinding {
    pub bound: Option<BoundAttributeDatatype>,
    pub state: ReferenceResolutionState,
    pub issue: Option<&'static str>,
    pub diagnostics: Vec<cem_ml::diagnostics::Diagnostic>,
    pub work_used: usize,
}
impl AttributeDatatypeBinding {
    fn issue(code: &'static str, state: ReferenceResolutionState) -> Self {
        Self {
            bound: None,
            state,
            issue: Some(code),
            diagnostics: vec![],
            work_used: 0,
        }
    }
}
#[derive(Clone)]
enum Selection<N> {
    Native(N),
    Literal(N, SchemaDeclarationNode, String),
}
impl<N> Selection<N> {
    fn value(&self) -> &N {
        match self {
            Self::Native(n) | Self::Literal(n, ..) => n,
        }
    }
}
struct SelectionHost<'a, H>(&'a mut H);
impl<H: AttributeDatatypeHost> ReferenceResolutionHost for SelectionHost<'_, H> {
    type Node = Selection<H::Node>;
    type Scope = H::Scope;
    fn prepare_node(&mut self, node: &mut Self::Node) -> Result<(), ReferenceResolutionError> {
        match node {
            Selection::Native(n) | Selection::Literal(n, ..) => self.0.prepare_node(n),
        }
    }
    fn scope(&self, node: &Self::Node) -> Self::Scope {
        self.0.scope(node.value())
    }
    fn scope_limits(&self, scope: &Self::Scope) -> ReferenceTraversalLimits {
        self.0.scope_limits(scope)
    }
    fn reference_occurrence(&self, node: &Self::Node) -> Option<ReferenceOccurrence> {
        match node {
            Selection::Native(n) => self.0.reference_occurrence(n),
            Selection::Literal(_, slot, _) => {
                let CemAstNode::Attribute { source, .. } = slot.node() else {
                    return None;
                };
                Some(ReferenceOccurrence {
                    identity: format!("attribute-type:{}", slot.identity()),
                    node_id: Some(slot.node_id()),
                    expression: None,
                    source_map: source.clone(),
                })
            }
        }
    }
    fn unresolved_policy(&self, node: &Self::Node) -> &ReferenceUnresolvedPolicy {
        self.0.unresolved_policy(node.value())
    }
    fn permits_edge(&self, from: &Self::Node, to: &Self::Node) -> bool {
        self.0.permits_edge(from.value(), to.value())
    }
    fn evaluate(&mut self, node: &Self::Node) -> ReferenceLinkEvaluation<Self::Node> {
        let result = match node {
            Selection::Native(n) => self.0.evaluate(n),
            Selection::Literal(_, slot, name) => self.0.lookup_attribute_type(slot, name),
        };
        match result {
            ReferenceLinkEvaluation::Resolved(nodes) => ReferenceLinkEvaluation::Resolved(
                nodes.into_iter().map(Selection::Native).collect(),
            ),
            ReferenceLinkEvaluation::Pending(r) => ReferenceLinkEvaluation::Pending(r),
            ReferenceLinkEvaluation::Unresolved(r) => ReferenceLinkEvaluation::Unresolved(r),
            ReferenceLinkEvaluation::Invalid(d) => ReferenceLinkEvaluation::Invalid(d),
        }
    }
}
/// Resolve one literal/native type slot under current host context and directed
/// grants. Only one exact original compiled named type may bind. An incomplete
/// compilation or selection is retained as incomplete, including ignored links.
pub fn bind_attribute_datatype<H: AttributeDatatypeHost>(
    declaration: SchemaDeclarationNode,
    compilation: &DatatypeCompilation,
    host: &mut H,
    limits: ReferenceTraversalLimits,
) -> Result<AttributeDatatypeBinding, ReferenceResolutionError> {
    let invalid = |code| AttributeDatatypeBinding::issue(code, ReferenceResolutionState::Invalid);
    let pending = |code| AttributeDatatypeBinding::issue(code, ReferenceResolutionState::Pending);
    let Some(name) = host.input_expanded_name(&declaration) else {
        return Ok(pending("attribute-name-pending"));
    };
    if name.local_name != "attribute"
        || (!name.namespace_uri.is_empty()
            && name.namespace_uri != cem_ml::schema::registry::CEM_SCHEMA_URI)
    {
        return Ok(invalid("attribute-declaration-required"));
    }
    let CemAstNode::Element { attributes, .. } = declaration.node() else {
        return Ok(invalid("attribute-declaration-required"));
    };
    let Some(slot) = attributes
        .iter()
        .rev()
        .filter_map(|id| SchemaDeclarationNode::new(declaration.document().clone(), *id))
        .find(|n| {
            host.input_expanded_name(n)
                .is_some_and(|n| n.local_name == "type" && n.namespace_uri.is_empty())
        })
    else {
        return Ok(pending("attribute-type-slot-unavailable"));
    };
    let CemAstNode::Attribute {
        value, value_nodes, ..
    } = slot.node()
    else {
        unreachable!()
    };
    let root = if value_nodes.is_empty() {
        let Some(value) = value.as_deref().filter(|v| {
            cem_ml::schema::document_model::shipped_datatypes::ShippedDatatype::QualifiedName
                .validate_lexical(v)
                == Some(true)
        }) else {
            return Ok(invalid("attribute-type-qname-required"));
        };
        Selection::Literal(
            host.source_reference(slot.clone()),
            slot.clone(),
            value.trim().into(),
        )
    } else {
        let [id] = value_nodes.as_slice() else {
            return Ok(invalid("attribute-type-reference-required"));
        };
        let Some(reference) = SchemaDeclarationNode::new(slot.document().clone(), *id)
            .filter(|n| matches!(n.node(), CemAstNode::Reference { .. }))
        else {
            return Ok(invalid("attribute-type-reference-required"));
        };
        Selection::Native(host.source_reference(reference))
    };
    let result = resolve_reference(root, &mut SelectionHost(host), limits)?;
    let mut report = AttributeDatatypeBinding {
        bound: None,
        state: if result.failed && result.state == ReferenceResolutionState::Resolved {
            ReferenceResolutionState::Invalid
        } else {
            result.state
        },
        issue: None,
        diagnostics: result.diagnostics,
        work_used: result.work_used,
    };
    if report.state != ReferenceResolutionState::Resolved || result.failed {
        return Ok(report);
    }
    let [selected] = result.nodes.as_slice() else {
        report.state = ReferenceResolutionState::Invalid;
        report.issue = Some("attribute-type-singleton-required");
        return Ok(report);
    };
    let target = host.declaration_node(selected.value());
    let Some(target) = target else {
        report.state = ReferenceResolutionState::Invalid;
        report.issue = Some("attribute-type-target-required");
        return Ok(report);
    };
    let Some(name) = host.input_expanded_name(&target) else {
        report.state = ReferenceResolutionState::Pending;
        report.issue = Some("attribute-type-name-pending");
        return Ok(report);
    };
    if name.local_name != "type"
        || (!name.namespace_uri.is_empty()
            && name.namespace_uri != cem_ml::schema::registry::CEM_SCHEMA_URI)
    {
        report.state = ReferenceResolutionState::Invalid;
        report.issue = Some("attribute-type-target-required");
        return Ok(report);
    }
    let source = host.datatype_source(&target);
    let Some(source) = source else {
        report.state = ReferenceResolutionState::Pending;
        report.issue = Some("attribute-type-source-unavailable");
        return Ok(report);
    };
    if !compilation.is_ready() {
        report.state = ReferenceResolutionState::Pending;
        report.issue = Some("datatype-compilation-incomplete");
        return Ok(report);
    }
    let contract = compilation
        .contracts
        .iter()
        .find(|d| {
            d.source().declaration().identity() == source.declaration().identity()
                && d.source().scope().identity() == source.scope().identity()
        })
        .and_then(|d| d.as_any().downcast_ref::<ExecutableDatatype>());
    let Some(contract) = contract else {
        report.state = ReferenceResolutionState::Pending;
        report.issue = Some("attribute-type-contract-unavailable");
        return Ok(report);
    };
    report.bound = Some(BoundAttributeDatatype {
        declaration,
        slot,
        datatype: Arc::new(contract.clone()),
    });
    Ok(report)
}
