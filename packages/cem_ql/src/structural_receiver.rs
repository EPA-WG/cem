//! Concrete native host for explicitly linked structural node receivers.
use crate::{
    api::StandaloneExpressionContext,
    schema_references::{CemQlSchemaDeclarationHost, CemQlSchemaReferenceNode, DeclarationScope},
};
use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::{tree::RetainedCemTree, AstNodeId, CemAstNode, ExpandedName},
    schema::{
        attribute_datatypes::AttributeDatatypeContext,
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        document_model::SchemaDocumentModel,
        input_references::{InputReferenceHost, StructuralInputValidation},
        reference_policy::{ReferenceOccurrence, ReferenceScopePolicy, ReferenceUnresolvedPolicy},
        reference_traversal::ReferenceTraversalLimits,
        structural_capsule::VerifiedStructuralCapsule,
        structural_receiver::StructuralNodeReceiver,
    },
    value::reference_resolution::*,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
const INVALID: &str = "fresh structural host refused";
mod native;
mod query;
pub use native::{FreshNativeOperation, FreshNativeTokenizer, NativeResultAdapter};
pub use query::{FreshQueryBinding, FreshQueryInvocation};
/// Never cloned/serialized. Contexts belong to this actual host allocation.
pub struct FreshStructuralHost {
    inner: CemQlSchemaDeclarationHost,
    context: AttributeDatatypeContext,
    registry_hash: String,
    control: OperationControl,
    grants: BTreeSet<(DeclarationScope, DeclarationScope)>,
    closed: AtomicBool,
    native_implementations: BTreeMap<String, Arc<native::Entry>>,
}
impl FreshStructuralHost {
    pub fn new(registry_hash: &str, control: OperationControl) -> Result<Self, String> {
        if !registry_hash
            .strip_prefix("cem-bin/1+blake3:")
            .is_some_and(|s| {
                s.len() == 64
                    && s.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
        {
            return Err(INVALID.into());
        }
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID)?;
        let mut inner = CemQlSchemaDeclarationHost::new();
        inner.set_operation_control(control.clone(), ROOT_EXECUTION_SCOPE_ID);
        Ok(Self {
            inner,
            context: Default::default(),
            registry_hash: registry_hash.into(),
            control,
            grants: BTreeSet::new(),
            closed: AtomicBool::new(false),
            native_implementations: BTreeMap::new(),
        })
    }
    pub fn register_scope(
        &mut self,
        tree: Arc<RetainedCemTree>,
        context: Option<StandaloneExpressionContext>,
        policy: ReferenceScopePolicy,
    ) -> Result<DeclarationScope, String> {
        self.check().map_err(|_| INVALID)?;
        self.context.advance();
        Ok(self.inner.register_scope(tree, context, policy))
    }
    pub fn set_context(
        &mut self,
        scope: DeclarationScope,
        context: Option<StandaloneExpressionContext>,
    ) -> bool {
        if self.check().is_err() || !self.inner.set_context(scope, context) {
            return false;
        }
        self.context.advance();
        true
    }
    pub fn allow_scope_crossing(&mut self, from: DeclarationScope, to: DeclarationScope) -> bool {
        if self.check().is_err() || !self.inner.allow_scope_crossing(from, to) {
            return false;
        }
        if self.grants.insert((from, to)) {
            self.context.advance();
        }
        true
    }
    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
        self.context.close();
    }
    fn check(&self) -> Result<(), ReferenceResolutionError> {
        if self.closed.load(Ordering::Acquire) || !self.context.lease().matches(&self.context) {
            return Err(ReferenceResolutionError::OperationStopped);
        }
        self.control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| ReferenceResolutionError::OperationStopped)
    }
    /// All data scopes/grants must be established first. Failure publishes no
    /// mutation; callers explicitly choose when to replace a receiving model.
    pub fn link_receiver(
        &self,
        capsule: Arc<VerifiedStructuralCapsule>,
        schema_uri: &str,
        source: Arc<RetainedCemTree>,
        attribute: AstNodeId,
    ) -> Result<SchemaDocumentModel, String> {
        self.check().map_err(|_| INVALID)?;
        cem_ml::schema::structural_capsule::validate_literal_receiver_source(
            source.ast(),
            schema_uri,
            &self.control,
        )?;
        let declaration =
            SchemaDeclarationNode::new(source.ast_owner().clone(), attribute).ok_or(INVALID)?;
        if self
            .inner
            .source_tree(&declaration)
            .is_none_or(|tree| !Arc::ptr_eq(tree, &source))
        {
            return Err(INVALID.into());
        }
        let Some(parent_id) = source.source_parent(attribute) else {
            return Err(INVALID.into());
        };
        let Some(parent) = source.ast().get(parent_id) else {
            return Err(INVALID.into());
        };
        if !matches!(parent,CemAstNode::Element{expanded_name,..} if expanded_name.local_name=="attributes"&&matches!(expanded_name.namespace_uri.as_str(),""|cem_ml::schema::registry::CEM_SCHEMA_URI))
        {
            return Err(INVALID.into());
        }
        let Some(CemAstNode::Element {
            expanded_name,
            attributes,
            ..
        }) = source
            .source_parent(parent_id)
            .and_then(|id| source.ast().get(id))
        else {
            return Err(INVALID.into());
        };
        if expanded_name.local_name!="schema" || !matches!(expanded_name.namespace_uri.as_str(),""|cem_ml::schema::registry::CEM_SCHEMA_URI) || !attributes.iter().any(|id|matches!(source.ast().get(*id),Some(CemAstNode::Attribute{expanded_name,value:Some(v),value_nodes,..}) if expanded_name.namespace_uri.is_empty()&&expanded_name.local_name=="namespace"&&v==schema_uri&&value_nodes.is_empty())) {return Err(INVALID.into());}
        let mut model = cem_ml::schema::document_model::compile_schema_document_model_from_retained(
            schema_uri, &source,
        );
        let name = cem_ml::schema::document_model::compile_attribute_model(source.ast(), attribute)
            .ok_or(INVALID)?
            .name;
        model
            .declaration_references
            .attribute_declarations
            .insert(name.clone(), declaration.clone());
        let contract = StructuralNodeReceiver::link(
            capsule,
            &self.registry_hash,
            declaration.clone(),
            &model,
            &self.context,
            &self.control,
        )?;
        let occurrences=source.ast().nodes.iter().filter(|node|matches!(node,CemAstNode::Element{expanded_name,attributes,..}
            if expanded_name.local_name=="attribute"&&attributes.iter().any(|id|matches!(source.ast().get(*id),Some(CemAstNode::Attribute{expanded_name,value:Some(v),..}) if expanded_name.local_name=="name"&&v==&name)))).count();
        if occurrences != 1 {
            return Err(INVALID.into());
        }
        self.check().map_err(|_| INVALID)?;
        let mut model = model;
        model.attribute_datatypes.insert(name, contract);
        Ok(model)
    }
    pub fn validate_input(
        &mut self,
        source: Arc<RetainedCemTree>,
        model: &SchemaDocumentModel,
    ) -> Result<StructuralInputValidation<CemQlSchemaReferenceNode>, ReferenceResolutionError> {
        self.check()?;
        let result = cem_ml::schema::input_references::validate_structural_input_references(
            source.ast_owner().clone(),
            model,
            self,
            ReferenceTraversalLimits {
                max_depth: 64,
                max_work: 100_000,
            },
        )?;
        self.check()?;
        Ok(result)
    }
}
impl Drop for FreshStructuralHost {
    fn drop(&mut self) {
        self.context.close();
    }
}
impl ReferenceResolutionHost for FreshStructuralHost {
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
        self.check().is_ok() && self.inner.permits_edge(from, to)
    }
    fn evaluate(&mut self, node: &Self::Node) -> ReferenceLinkEvaluation<Self::Node> {
        if self.check().is_err() {
            return ReferenceLinkEvaluation::Pending(INVALID.into());
        }
        self.inner.evaluate(node)
    }
}
impl InputReferenceHost for FreshStructuralHost {
    fn attribute_receiver_context(&self) -> Option<&AttributeDatatypeContext> {
        Some(&self.context)
    }
    fn input_consumed_namespace_attribute(&self, node: &SchemaDeclarationNode) -> bool {
        SchemaDeclarationHost::input_consumed_namespace_attribute(&self.inner, node)
    }
    fn input_source_tree(&self, node: &SchemaDeclarationNode) -> Option<Arc<RetainedCemTree>> {
        self.inner.source_tree(node).cloned()
    }
    fn input_expanded_name<'a>(
        &'a self,
        node: &'a SchemaDeclarationNode,
    ) -> Option<&'a ExpandedName> {
        self.check().ok()?;
        SchemaDeclarationHost::input_expanded_name(&self.inner, node)
    }
    fn source_node(&self, node: SchemaDeclarationNode) -> Self::Node {
        self.inner.source_reference(node)
    }
    fn retained_node(&self, node: &Self::Node) -> Option<SchemaDeclarationNode> {
        self.inner.declaration_node(node)
    }
    fn structural_diagnostic(
        &self,
        node: &SchemaDeclarationNode,
        d: cem_ml::diagnostics::Diagnostic,
    ) -> cem_ml::diagnostics::Diagnostic {
        SchemaDeclarationHost::structural_diagnostic(&self.inner, node, d)
    }
    fn evaluate_input_expression(
        &mut self,
        node: &Self::Node,
    ) -> ReferenceLinkEvaluation<Self::Node> {
        SchemaDeclarationHost::evaluate_input_expression(&mut self.inner, node)
    }
}
