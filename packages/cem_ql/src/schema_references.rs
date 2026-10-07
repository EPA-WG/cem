//! Explicit CEM-QL consumer stage for retained schema declaration references.
//! The caller supplies contexts, effective scopes and directed crossing grants.
//! Construction/import never invokes this stage or writes targets to source.
use crate::{
    api::{
        compile_expression, evaluate, CompiledExpression, EvaluationContext,
        StandaloneExpressionContext,
    },
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
            validate_structural_input_roots_references,
            validate_structural_input_references_with_behavior_evaluator,
            StructuralInputValidation,
        },
        reference_policy::{
            ReferenceOccurrence, ReferenceScopePolicy, ReferenceScopePolicyOverrides,
            ReferenceUnresolvedPolicy,
        },
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

mod source_diagnostics;
mod lexical_handoff;
mod scope_preparation;
mod namespace_preparation;
mod namespace_handoff;
mod namespace_property;
mod namespace_activation;
mod namespace_schema_names;
pub use namespace_activation::{NamespacePropertyActivation, NamespacePropertyActivationError};
pub use namespace_property::{NamespacePropertyPreparation, NamespacePropertyPreparationIssue};
pub use namespace_handoff::NamespaceLexicalScopeHandoffError;
pub use namespace_preparation::{NamespaceScopePreparation, NamespaceScopePreparationIssue};
mod uri_loads;
mod uri_loader;
pub use uri_loader::{SchemaUriLoadedScope, SchemaUriLoadTicket};
pub use uri_loads::SchemaUriLoadBindingError;
mod region_validation;
mod host_controls;
mod host_regions;
mod host_runtime_inputs;
mod host_runtime_binding;
pub use host_runtime_binding::{SchemaHostRuntimeContextRequest, SchemaHostRuntimeValidation};
pub use host_runtime_inputs::{
    SchemaHostRuntimeInputIssue, SchemaHostRuntimeInputs, SchemaHostRuntimeScope,
};
pub use host_regions::{SchemaHostRegionPreparation, SchemaHostRegionValidation};
pub use host_controls::{PreparedSchemaHostControl, SchemaHostPreparationError};
pub use region_validation::SchemaInputRegion;
pub use scope_preparation::{SchemaScopePreparation, SchemaScopePreparationIssue};
pub use lexical_handoff::LexicalScopeHandoffError;

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
    lexical_parent: Option<DeclarationScope>,
    local_policy: ReferenceScopePolicyOverrides,
    // Lexical contexts may differ while crossing permission identity stays shared.
    relationship: DeclarationScope,
}
#[derive(Debug, Clone)]
pub struct CemQlSchemaReferenceNode {
    source: Option<SchemaDeclarationNode>,
    query: Option<Item>,
    scope: Option<DeclarationScope>,
    // A consumer-owned implicit selector occurrence on an original source node.
    // This is absent on ordinary data nodes and is never persisted in the AST.
    selector_expression: Option<String>,
}
#[derive(Debug, Clone)]
pub struct CemQlSchemaDeclarationHost {
    identity: u64,
    scopes: Vec<Scope>,
    // Arena addresses are private storage keys, not reference syntax or scopes.
    node_scopes: BTreeMap<(usize, AstNodeId), DeclarationScope>,
    // Caller-completed sibling boundaries, indexed by retained structural order.
    following_scopes: BTreeMap<(usize, AstNodeId), BTreeMap<usize, DeclarationScope>>,
    grants: BTreeSet<(DeclarationScope, DeclarationScope)>,
    fallback_policy: ReferenceScopePolicy,
    // Immutable scalar name metadata, keyed by a registered original owner.
    captured_names: BTreeMap<usize, BTreeMap<AstNodeId, cem_ml::parser::ExpandedName>>,
    // Explicit invocation-only completed names, separate from immutable capture.
    namespace_name_completions:
        BTreeMap<usize, Arc<cem_ml::schema::namespace_references::NamespaceNameCompletion>>,
    captured_schema_forms:
        BTreeMap<usize, BTreeMap<AstNodeId, cem_ml::schema::machine::SchemaElementForm>>,
    // Original completed/pending namespace declarations remain immutable; their
    // consumer selection and activation are separate lifecycle stages.
    captured_namespaces: BTreeMap<usize, Arc<cem_ml::schema::machine::LexicallyScopedDocument>>,
    // Explicit loader snapshots; keys keep original controls and authored URIs distinct.
    schema_uri_loads:
        BTreeMap<(usize, AstNodeId, String), ReferenceLinkEvaluation<SchemaDeclarationNode>>,
    schema_uri_generations: BTreeMap<(usize, AstNodeId, String), u64>,
    next_schema_uri_generation: u64,
    // Compiled source only: runtime targets and contexts remain per invocation.
    source_expressions: BTreeMap<(DeclarationScope, String), Arc<CompiledExpression>>,
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
            following_scopes: BTreeMap::new(),
            grants: BTreeSet::new(),
            source_expressions: BTreeMap::new(),
            schema_uri_loads: BTreeMap::new(),
            schema_uri_generations: BTreeMap::new(),
            next_schema_uri_generation: 0,
            captured_names: BTreeMap::new(),
            namespace_name_completions: BTreeMap::new(),
            captured_schema_forms: BTreeMap::new(),
            captured_namespaces: BTreeMap::new(),
            fallback_policy: ReferenceScopePolicy::schema_defaults()
                .expect("embedded reference policy"),
        }
    }
    /// Establish an explicit relationship boundary for a retained owner and
    /// an explicitly supplied lifecycle snapshot. Lexical-only changes use
    /// `register_lexical_scope` instead.
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
            local_policy: ReferenceScopePolicyOverrides::explicit(policy.clone()),
            lexical_parent: None,
            policy,
            relationship: scope,
        });
        scope
    }

    /// Register a distinct lexical lifecycle snapshot within the parent's
    /// relationship boundary. Compilation, context readiness and effective policy
    /// remain local to this handle; directed crossing grants remain shared.
    /// No source assignment, evaluation or inheritance of runtime inputs occurs.
    pub fn register_lexical_scope(
        &mut self,
        parent: DeclarationScope,
        context: Option<StandaloneExpressionContext>,
        policy: ReferenceScopePolicy,
    ) -> Option<DeclarationScope> {
        self.register_lexical_scope_with_policy_overrides(
            parent,
            context,
            ReferenceScopePolicyOverrides::explicit(policy),
        )
    }
    /// Declare only local settings; omitted settings inherit the complete parent
    /// policy. Context readiness is explicitly supplied and is never inherited.
    pub fn register_lexical_scope_with_policy_overrides(
        &mut self,
        parent: DeclarationScope,
        context: Option<StandaloneExpressionContext>,
        local_policy: ReferenceScopePolicyOverrides,
    ) -> Option<DeclarationScope> {
        let parent_record = self.scope_record(parent)?;
        let tree = parent_record.tree.clone();
        let relationship = parent_record.relationship;
        let policy = local_policy.apply_to(&parent_record.policy);
        let scope = DeclarationScope {
            host: self.identity,
            index: self.scopes.len(),
        };
        self.scopes.push(Scope {
            tree,
            context,
            policy,
            relationship,
            lexical_parent: Some(parent),
            local_policy,
        });
        Some(scope)
    }
    /// Inspect the original tree owning this registered source handle. Returning
    /// its provenance grants no reference crossing or schema activation authority.
    pub fn source_tree(&self, source: &SchemaDeclarationNode) -> Option<&Arc<RetainedCemTree>> {
        self.scopes.iter()
            .find(|scope| Arc::ptr_eq(scope.tree.ast_owner(), source.document()))
            .map(|scope| &scope.tree)
    }
    pub fn reference_scope_policy(&self, scope: DeclarationScope) -> Option<&ReferenceScopePolicy> {
        Some(&self.scope_record(scope)?.policy)
    }
    pub fn lexical_scope_parent(&self, scope: DeclarationScope) -> Option<DeclarationScope> {
        self.scope_record(scope)?.lexical_parent
    }
    pub fn scope_policy_overrides(
        &self,
        scope: DeclarationScope,
    ) -> Option<&ReferenceScopePolicyOverrides> {
        Some(&self.scope_record(scope)?.local_policy)
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
    /// Record a completed existing schema/namespace sibling switch. The boundary
    /// retains its previous scope; following siblings and descendants inherit the
    /// supplied scope until another sibling switch or the containing scope ends.
    /// The caller determines readiness and supplies runtime inputs separately.
    /// Repeating the same handoff is idempotent; a conflicting repeat is rejected
    /// rather than silently rebinding an established lexical boundary.
    /// This neither recognizes syntax nor evaluates a selector on document load.
    pub fn assign_following_scope(
        &mut self,
        tree: &Arc<RetainedCemTree>,
        boundary: AstNodeId,
        scope: DeclarationScope,
    ) -> bool {
        if self.scope_record(scope).is_none()
            || !self
                .scopes
                .iter()
                .any(|s| Arc::ptr_eq(s.tree.ast_owner(), tree.ast_owner()))
        {
            return false;
        }
        let Some(parent) = tree.source_parent(boundary) else {
            return false;
        };
        let Some(index) = Self::source_children(tree, parent)
            .and_then(|children| children.iter().position(|id| *id == boundary))
        else {
            return false;
        };
        match self
            .following_scopes
            .entry((Arc::as_ptr(tree.ast_owner()) as usize, parent))
            .or_default()
            .entry(index + 1)
        {
            std::collections::btree_map::Entry::Occupied(existing) => *existing.get() == scope,
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(scope);
                true
            }
        }
    }

    fn source_children(tree: &RetainedCemTree, parent: AstNodeId) -> Option<&[AstNodeId]> {
        match tree.ast().get(parent)? {
            CemAstNode::Document { root_children, .. } => Some(root_children),
            CemAstNode::Element { children, .. } => Some(children),
            _ => None,
        }
    }

    pub fn set_context(
        &mut self,
        scope: DeclarationScope,
        context: Option<StandaloneExpressionContext>,
    ) -> bool {
        if self.scope_record(scope).is_none() {
            return false;
        }
        // Binding types, expected type and registered capabilities can change.
        // Invalidate this scope even when only values appear to have changed;
        // do not infer compilation compatibility from runtime data.
        self.source_expressions
            .retain(|(owner, _), _| *owner != scope);
        self.scopes[scope.index].context = context;
        true
    }
    /// Inspect compilation for an original expression occurrence in its current
    /// effective scope. This does not compile, evaluate or expose stored targets.
    pub fn compiled_source_expression(
        &self,
        source: &SchemaDeclarationNode,
    ) -> Option<Arc<CompiledExpression>> {
        let scope = self.source_scope(source)?;
        self.source_expressions
            .get(&(scope, source.identity()))
            .cloned()
    }

    fn evaluate_source_expression(
        &mut self,
        node: &CemQlSchemaReferenceNode,
        expression: &str,
    ) -> Result<crate::eval::ItemStream, Vec<cem_ml::diagnostics::Diagnostic>> {
        // Callers distinguish missing scope/context from compilation failure.
        let scope = node.scope.expect("registered expression scope");
        let source = node.source.as_ref().expect("original expression source");
        let key = (scope, source.identity());
        let context = self
            .scope_record(scope)
            .and_then(|scope| scope.context.as_ref())
            .expect("ready expression context");
        let compiled = match self.source_expressions.get(&key) {
            Some(compiled) => compiled.clone(),
            None => Arc::new(
                compile_expression(expression, context)
                    .map_err(|error| self.source_diagnostics(source, error.diagnostics))?,
            ),
        };
        let result = evaluate(
            &compiled.query,
            &EvaluationContext {
                scope: context.scope,
                scope_policy: context.scope_policy,
                diagnostics: context.diagnostics.clone(),
                policy_bindings: context.policy_bindings(),
                current_item: context.context_item.clone(),
                module_resolution: context.module_resolution.clone(),
                native_functions: context.native_functions.clone(),
                data_readers: Default::default(),
            },
        );
        self.source_expressions.entry(key).or_insert(compiled);
        if result.error.is_some() {
            Err(self.source_diagnostics(source, result.diagnostics))
        } else {
            Ok(result)
        }
    }

    /// Grant a directed crossing between explicit relationship boundaries.
    /// Lexical snapshots normalize to their inherited boundary identity.
    pub fn allow_scope_crossing(&mut self, from: DeclarationScope, to: DeclarationScope) -> bool {
        let Some(from) = self.scope_record(from).map(|scope| scope.relationship) else {
            return false;
        };
        let Some(to) = self.scope_record(to).map(|scope| scope.relationship) else {
            return false;
        };
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
    /// Validate explicit original roots with a supplied ready consuming model.
    /// Region/boundary ownership is chosen by the caller; the original arena and
    /// existing reference contexts, grants and placement reporting remain shared.
    pub fn validate_input_roots(
        &mut self,
        source: Arc<RetainedCemTree>,
        roots: &[AstNodeId],
        model: &SchemaDocumentModel,
        limits: ReferenceTraversalLimits,
    ) -> Result<StructuralInputValidation<CemQlSchemaReferenceNode>, ReferenceResolutionError> {
        validate_structural_input_roots_references(
            source.ast_owner().clone(), roots, model, self, limits,
        )
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
        self.source_scope_with_assignments(source, &self.node_scopes)
    }
    fn source_scope_with_assignments(
        &self,
        source: &SchemaDeclarationNode,
        assignments: &BTreeMap<(usize, AstNodeId), DeclarationScope>,
    ) -> Option<DeclarationScope> {
        let owner = Arc::as_ptr(source.document()) as usize;
        let tree = self
            .scopes
            .iter()
            .find(|s| Arc::ptr_eq(s.tree.ast_owner(), source.document()))?
            .tree
            .clone();
        let mut node = Some(source.node_id());
        while let Some(id) = node {
            if let Some(scope) = assignments.get(&(owner, id)) {
                return Some(*scope);
            }
            let parent = tree.source_parent(id);
            if let Some(parent) = parent {
                if let Some(transitions) = self.following_scopes.get(&(owner, parent)) {
                    if let Some(index) = Self::source_children(&tree, parent)
                        .and_then(|children| children.iter().position(|child| *child == id))
                    {
                        if let Some((_, scope)) = transitions.range(..=index).next_back() {
                            return Some(*scope);
                        }
                    }
                }
            }
            node = parent;
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
            selector_expression: None,
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
            if let Some(expression) = &node.selector_expression {
                let provenance = match source.node() {
                    CemAstNode::Attribute { source, .. }
                    | CemAstNode::Element { source, .. }
                    | CemAstNode::Text { source, .. } => source,
                    _ => unreachable!(
                        "implicit selectors keep their original attribute, expression or directive payload"
                    ),
                };
                return Some(ReferenceOccurrence {
                    identity: source.identity(),
                    node_id: Some(source.node_id()),
                    expression: Some(expression.clone()),
                    source_map: provenance.clone(),
                });
            }
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
        let relationship = |scope: Option<DeclarationScope>| {
            scope
                .and_then(|scope| self.scope_record(scope))
                .map(|scope| scope.relationship)
        };
        match (relationship(from.scope), relationship(to.scope)) {
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
        if scope.context.is_none() {
            return ReferenceLinkEvaluation::Pending("schema-context-not-ready".into());
        }
        if let Some(expression) = &node.selector_expression {
            return match self.evaluate_source_expression(node, expression) {
                Err(diagnostics) => ReferenceLinkEvaluation::Invalid(diagnostics),
                Ok(evaluated) => ReferenceLinkEvaluation::Resolved(
                    evaluated
                        .items
                        .into_iter()
                        .map(|item| self.query_node(item, node.scope))
                        .collect(),
                ),
            };
        }
        let Some(CemAstNode::Reference { expression, .. }) = node.source.as_ref().map(|s| s.node())
        else {
            unreachable!("resolver only evaluates typed references");
        };
        let evaluated = match self.evaluate_source_expression(node, expression) {
            Err(diagnostics) => return ReferenceLinkEvaluation::Invalid(diagnostics),
            Ok(evaluated) => evaluated,
        };
        // Consume only the outer source constructor. Nested reference operands
        // remain explicit nodes and are followed by the common bounded walker.
        let items = &evaluated.items;
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
    fn input_source_tree(
        &self,
        source: &SchemaDeclarationNode,
    ) -> Option<Arc<RetainedCemTree>> {
        self.source_tree(source).cloned()
    }
    fn input_expanded_name<'a>(
        &'a self,
        source: &'a SchemaDeclarationNode,
    ) -> Option<&'a cem_ml::parser::ExpandedName> {
        if self
            .captured_names
            .contains_key(&(Arc::as_ptr(source.document()) as usize))
        {
            self.consuming_expanded_name(source)
        } else {
            // Fixed native trees without a lexical capture retain their API.
            match source.node() {
                CemAstNode::Element { expanded_name, .. }
                | CemAstNode::Attribute { expanded_name, .. } => Some(expanded_name),
                _ => None,
            }
        }
    }
    fn structural_diagnostic(
        &self,
        source: &SchemaDeclarationNode,
        diagnostic: cem_ml::diagnostics::Diagnostic,
    ) -> cem_ml::diagnostics::Diagnostic {
        self.authored_structural_diagnostic(source, diagnostic)
    }
    fn evaluate_input_expression(
        &mut self,
        node: &Self::Node,
    ) -> ReferenceLinkEvaluation<Self::Node> {
        let Some(source) = node.source.as_ref() else {
            return ReferenceLinkEvaluation::Pending("input-expression-source-not-ready".into());
        };
        let Some(occurrence) =
            cem_ml::schema::input_references::native_attribute_expression(source)
        else {
            return ReferenceLinkEvaluation::Pending("unsupported-input-expression".into());
        };
        let Some(scope) = node.scope.and_then(|scope| self.scope_record(scope)) else {
            return ReferenceLinkEvaluation::Unresolved("unregistered-schema-scope".into());
        };
        if scope.context.is_none() {
            return ReferenceLinkEvaluation::Pending("schema-context-not-ready".into());
        }
        let evaluated = match self
            .evaluate_source_expression(node, occurrence.expression.as_deref().unwrap())
        {
            Err(diagnostics) => return ReferenceLinkEvaluation::Invalid(diagnostics),
            Ok(evaluated) => evaluated,
        };
        let nodes: Vec<_> = evaluated
            .items
            .into_iter()
            .map(|item| self.query_node(item, node.scope))
            .collect();
        if nodes.iter().any(|target| {
            self.declaration_node(target).is_none() && self.reference_occurrence(target).is_none()
        }) {
            return ReferenceLinkEvaluation::Invalid(vec![cem_ml::diagnostics::Diagnostic {
                code: cem_ml::schema::attribute_references::INVALID_NATIVE_TARGET.into(),
                severity: cem_ml::diagnostics::Severity::Error,
                message: "A node-valued attribute expression must produce retained nodes or native references".into(),
                node: Some(occurrence.identity),
                source_map: Some(occurrence.source_map),
                ..Default::default()
            }]);
        }
        ReferenceLinkEvaluation::Resolved(nodes)
    }
    fn source_reference(&self, source: SchemaDeclarationNode) -> Self::Node {
        let scope = self.source_scope(&source);
        CemQlSchemaReferenceNode {
            source: Some(source),
            query: None,
            scope,
            selector_expression: None,
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
