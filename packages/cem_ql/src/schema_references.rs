//! Explicit CEM-QL consumer stage for retained schema declaration references.
//! The caller supplies contexts, effective scopes and directed crossing grants.
//! Construction/import never invokes this stage or writes targets to source.
use crate::{
    api::{evaluate_expression, StandaloneExpressionContext},
    eval::{retained_cem_node, values::ReferenceView, Item},
};
use cem_ml::{
    parser::{tree::RetainedCemTree, AstNodeId, CemAstNode},
    schema::{
        declaration_references::{
            compile_schema_with_declaration_references, SchemaDeclarationHost,
            SchemaDeclarationNode,
        },
        document_model::{SchemaBehaviorEvaluator, SchemaDocumentModel},
        input_references::{
            validate_structural_input_references,
            validate_structural_input_references_with_behavior_evaluator,
            StructuralInputValidation,
        },
        reference_policy::{ReferenceOccurrence, ReferenceScopePolicy, ReferenceUnresolvedPolicy},
        reference_traversal::ReferenceTraversalLimits,
    },
    value::reference_resolution::{
        ReferenceLinkEvaluation, ReferenceResolutionError, ReferenceResolutionHost,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

/// Runtime handle for a caller-provided scope; never an authored/context ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeclarationScope {
    host: u64,
    index: usize,
}
#[derive(Debug, Clone)]
struct Scope {
    tree: Arc<RetainedCemTree>,
    context: Option<StandaloneExpressionContext>,
    policy: ReferenceScopePolicy,
}
#[derive(Debug, Clone)]
pub struct CemQlSchemaReferenceNode {
    source: Option<SchemaDeclarationNode>,
    query: Option<Item>,
    scope: Option<DeclarationScope>,
}
#[derive(Debug, Clone)]
pub struct CemQlSchemaDeclarationHost {
    identity: u64,
    scopes: Vec<Scope>,
    // Arena addresses are private storage keys, not reference syntax or scopes.
    node_scopes: BTreeMap<(usize, AstNodeId), DeclarationScope>,
    grants: BTreeSet<(DeclarationScope, DeclarationScope)>,
    fallback_policy: ReferenceScopePolicy,
}
impl Default for CemQlSchemaDeclarationHost {
    fn default() -> Self {
        Self::new()
    }
}
impl CemQlSchemaDeclarationHost {
    pub fn new() -> Self {
        static NEXT_HOST: AtomicU64 = AtomicU64::new(1);
        Self {
            identity: NEXT_HOST.fetch_add(1, Ordering::Relaxed),
            scopes: vec![],
            node_scopes: BTreeMap::new(),
            grants: BTreeSet::new(),
            fallback_policy: ReferenceScopePolicy::schema_defaults()
                .expect("embedded reference policy"),
        }
    }
    /// Register a retained owner and an explicitly supplied lifecycle snapshot.
    /// None means its evaluation inputs are pending, including a target scope
    /// containing declarations that do not themselves need evaluation.
    pub fn register_scope(
        &mut self,
        tree: Arc<RetainedCemTree>,
        context: Option<StandaloneExpressionContext>,
        policy: ReferenceScopePolicy,
    ) -> DeclarationScope {
        let scope = DeclarationScope {
            host: self.identity,
            index: self.scopes.len(),
        };
        self.node_scopes
            .entry((Arc::as_ptr(tree.ast_owner()) as usize, 0))
            .or_insert(scope);
        self.scopes.push(Scope {
            tree,
            context,
            policy,
        });
        scope
    }
    /// Map an explicit child subtree to an effective scope. Syntax/scoping
    /// construction belongs to the caller; the nearest source ancestor wins.
    pub fn assign_subtree_scope(
        &mut self,
        tree: &Arc<RetainedCemTree>,
        node: AstNodeId,
        scope: DeclarationScope,
    ) -> bool {
        if self.scope_record(scope).is_none()
            || tree.ast().get(node).is_none()
            || !self
                .scopes
                .iter()
                .any(|s| Arc::ptr_eq(s.tree.ast_owner(), tree.ast_owner()))
        {
            return false;
        }
        self.node_scopes
            .insert((Arc::as_ptr(tree.ast_owner()) as usize, node), scope);
        true
    }
    pub fn set_context(
        &mut self,
        scope: DeclarationScope,
        context: Option<StandaloneExpressionContext>,
    ) -> bool {
        if self.scope_record(scope).is_none() {
            return false;
        }
        self.scopes[scope.index].context = context;
        true
    }
    pub fn allow_scope_crossing(&mut self, from: DeclarationScope, to: DeclarationScope) -> bool {
        if self.scope_record(from).is_none() || self.scope_record(to).is_none() {
            return false;
        }
        self.grants.insert((from, to));
        true
    }
    fn scope_record(&self, scope: DeclarationScope) -> Option<&Scope> {
        (scope.host == self.identity)
            .then(|| self.scopes.get(scope.index))
            .flatten()
    }
    pub fn compile(
        &mut self,
        schema_uri: &str,
        source: Arc<RetainedCemTree>,
        limits: ReferenceTraversalLimits,
    ) -> Result<SchemaDocumentModel, ReferenceResolutionError> {
        compile_schema_with_declaration_references(
            schema_uri,
            source.ast_owner().clone(),
            self,
            limits,
        )
    }
    /// Explicit structural-input lifecycle stage using the supplied consuming
    /// schema and this host's current contexts, policies and scope grants.
    pub fn validate_input(
        &mut self,
        source: Arc<RetainedCemTree>,
        model: &SchemaDocumentModel,
        limits: ReferenceTraversalLimits,
    ) -> Result<StructuralInputValidation<CemQlSchemaReferenceNode>, ReferenceResolutionError> {
        validate_structural_input_references(source.ast_owner().clone(), model, self, limits)
    }
    /// Explicit structural selection followed by the additive retained behavior
    /// hook; each invocation uses this host's current runtime context.
    pub fn validate_input_with_behavior_evaluator(
        &mut self,
        source: Arc<RetainedCemTree>,
        model: &SchemaDocumentModel,
        limits: ReferenceTraversalLimits,
        evaluator: Option<&dyn SchemaBehaviorEvaluator>,
    ) -> Result<StructuralInputValidation<CemQlSchemaReferenceNode>, ReferenceResolutionError> {
        validate_structural_input_references_with_behavior_evaluator(
            source.ast_owner().clone(),
            model,
            self,
            limits,
            evaluator,
        )
    }
    fn source_scope(&self, source: &SchemaDeclarationNode) -> Option<DeclarationScope> {
        let owner = Arc::as_ptr(source.document()) as usize;
        let tree = self
            .scopes
            .iter()
            .find(|s| Arc::ptr_eq(s.tree.ast_owner(), source.document()))?
            .tree
            .clone();
        let mut node = Some(source.node_id());
        while let Some(id) = node {
            if let Some(scope) = self.node_scopes.get(&(owner, id)) {
                return Some(*scope);
            }
            node = tree.source_parent(id);
        }
        None
    }
    fn invalid_constructor(
        &self,
        node: &CemQlSchemaReferenceNode,
    ) -> ReferenceLinkEvaluation<CemQlSchemaReferenceNode> {
        let occurrence = self
            .reference_occurrence(node)
            .expect("typed source reference");
        ReferenceLinkEvaluation::Invalid(vec![cem_ml::diagnostics::Diagnostic {
            code: cem_ml::schema::declaration_references::INVALID_REFERENCE_TARGET.into(),
            severity: cem_ml::diagnostics::Severity::Error,
            message: "A source reference expression must produce one outer reference constructor"
                .into(),
            node: Some(occurrence.identity),
            source_map: Some(occurrence.source_map),
            ..Default::default()
        }])
    }
    fn query_node(
        &self,
        item: Item,
        inherited_scope: Option<DeclarationScope>,
    ) -> CemQlSchemaReferenceNode {
        if let Some(node) = retained_cem_node(&item) {
            let source =
                SchemaDeclarationNode::new(node.owner().ast_owner().clone(), node.node_id())
                    .unwrap();
            return self.source_reference(source);
        }
        let source = item.view().and_then(|view| {
            if let Some(node) =
                view.downcast_ref::<crate::attribute_values::NativeAttributeQueryNode>()
            {
                Some(node.source_node().clone())
            } else {
                view.downcast_ref::<crate::validation_structure::ValidationPlacementNode>()
                    .map(|node| node.source_node())
            }
        });
        if let Some(source) = source {
            return self.source_reference(source);
        }
        CemQlSchemaReferenceNode {
            source: None,
            query: Some(item),
            scope: inherited_scope,
        }
    }
}
impl ReferenceResolutionHost for CemQlSchemaDeclarationHost {
    type Node = CemQlSchemaReferenceNode;
    type Scope = Option<DeclarationScope>;
    fn scope(&self, node: &Self::Node) -> Self::Scope {
        node.scope
    }
    fn scope_limits(&self, scope: &Self::Scope) -> ReferenceTraversalLimits {
        scope
            .and_then(|s| self.scope_record(s))
            .map_or(self.fallback_policy.limits, |s| s.policy.limits)
    }
    fn reference_occurrence(&self, node: &Self::Node) -> Option<ReferenceOccurrence> {
        if let Some(source) = &node.source {
            if let CemAstNode::Reference {
                expression,
                source: provenance,
                ..
            } = source.node()
            {
                return Some(ReferenceOccurrence {
                    identity: source.identity(),
                    node_id: Some(source.node_id()),
                    expression: Some(expression.clone()),
                    source_map: provenance.clone(),
                });
            }
            return None;
        }
        let view = node.query.as_ref()?.view()?;
        view.downcast_ref::<ReferenceView>()?;
        Some(ReferenceOccurrence {
            identity: view.identity(),
            node_id: None,
            expression: None,
            source_map: view.source_map().unwrap_or_default(),
        })
    }
    fn unresolved_policy(&self, node: &Self::Node) -> &ReferenceUnresolvedPolicy {
        &node
            .scope
            .and_then(|s| self.scope_record(s))
            .map_or(&self.fallback_policy, |s| &s.policy)
            .unresolved
    }
    fn permits_edge(&self, from: &Self::Node, to: &Self::Node) -> bool {
        match (from.scope, to.scope) {
            (Some(a), Some(b)) => a == b || self.grants.contains(&(a, b)),
            _ => false,
        }
    }
    fn evaluate(&mut self, node: &Self::Node) -> ReferenceLinkEvaluation<Self::Node> {
        if let Some(native) = node
            .query
            .as_ref()
            .and_then(|i| i.view())
            .and_then(|v| v.downcast_ref::<ReferenceView>())
        {
            return ReferenceLinkEvaluation::Resolved(
                native
                    .0
                    .values()
                    .iter()
                    .cloned()
                    .map(|i| self.query_node(i, node.scope))
                    .collect(),
            );
        }
        let Some(scope) = node.scope.and_then(|s| self.scope_record(s)) else {
            return ReferenceLinkEvaluation::Unresolved("unregistered-schema-scope".into());
        };
        let Some(context) = &scope.context else {
            return ReferenceLinkEvaluation::Pending("schema-context-not-ready".into());
        };
        let Some(CemAstNode::Reference { expression, .. }) = node.source.as_ref().map(|s| s.node())
        else {
            unreachable!("resolver only evaluates typed references");
        };
        let evaluated = match evaluate_expression(expression, context) {
            Err(error) => return ReferenceLinkEvaluation::Invalid(error.diagnostics),
            Ok(evaluated) if evaluated.result.error.is_some() => {
                return ReferenceLinkEvaluation::Invalid(evaluated.result.diagnostics)
            }
            Ok(evaluated) => evaluated,
        };
        // Consume only the outer source constructor. Nested reference operands
        // remain explicit nodes and are followed by the common bounded walker.
        let items = &evaluated.result.items;
        if items.len() != 1 {
            return self.invalid_constructor(node);
        }
        let Some(native) = items[0]
            .view()
            .and_then(|v| v.downcast_ref::<ReferenceView>())
        else {
            return self.invalid_constructor(node);
        };
        ReferenceLinkEvaluation::Resolved(
            native
                .0
                .values()
                .iter()
                .cloned()
                .map(|i| self.query_node(i, node.scope))
                .collect(),
        )
    }
}
impl SchemaDeclarationHost for CemQlSchemaDeclarationHost {
    fn source_reference(&self, source: SchemaDeclarationNode) -> Self::Node {
        let scope = self.source_scope(&source);
        CemQlSchemaReferenceNode {
            source: Some(source),
            query: None,
            scope,
        }
    }
    fn declaration_node(&self, target: &Self::Node) -> Option<SchemaDeclarationNode> {
        target.source.clone()
    }
    fn declaration_schema(&self, target: &SchemaDeclarationNode) -> Option<SchemaDeclarationNode> {
        let tree = &self
            .scopes
            .iter()
            .find(|s| Arc::ptr_eq(s.tree.ast_owner(), target.document()))?
            .tree;
        let mut node = tree.source_parent(target.node_id());
        while let Some(id) = node {
            if matches!(tree.ast().get(id), Some(CemAstNode::Element {expanded_name,..}) if expanded_name.local_name == "schema")
            {
                return SchemaDeclarationNode::new(target.document().clone(), id);
            }
            node = tree.source_parent(id);
        }
        None
    }
}
