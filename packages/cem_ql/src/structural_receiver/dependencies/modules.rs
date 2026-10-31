//! Explicit CEMDM1 entry points. Immutable metadata stays distinct from CEMDP1;
//! both formats use the same actual-host resolver, compiler and ownership checks.
use super::*;
use crate::{
    datatype_compilation::DatatypeImplementation, datatype_module_capsule::VerifiedDatatypeModules,
};
impl FreshStructuralHost {
    /// Register one original module or leaf source on this actual host. This
    /// attaches captured metadata and never creates a crossing grant.
    pub fn register_datatype_module_scope(
        &mut self,
        graph: &VerifiedDatatypeModules,
        id: &str,
        context: Option<StandaloneExpressionContext>,
        policy: ReferenceScopePolicy,
    ) -> Result<DeclarationScope, String> {
        self.register_input_scope(InputRef::Modules(graph), Some(id), context, policy)
    }
    /// Explicitly prepare original occurrences; absent contexts remain pending.
    pub fn prepare_datatype_module_lexical_scopes<F>(
        &mut self,
        graph: &VerifiedDatatypeModules,
        id: &str,
        prepare: F,
    ) -> Result<Vec<(AstNodeId, DeclarationScope)>, String>
    where
        F: FnMut(
            &SchemaDeclarationNode,
            &LexicalScopeSnapshot,
            DeclarationScope,
        ) -> (
            Option<StandaloneExpressionContext>,
            ReferenceScopePolicyOverrides,
        ),
    {
        self.prepare_input_lexical_scopes(InputRef::Modules(graph), Some(id), prepare)
    }
    /// Inspection only. Later compilation independently renews every selection.
    pub fn select_datatype_modules(
        &mut self,
        graph: Arc<VerifiedDatatypeModules>,
        root: DeclarationScope,
        scopes: &BTreeMap<String, DeclarationScope>,
        imports: &[DatatypeDependencyImport],
        limits: ReferenceTraversalLimits,
    ) -> Result<FreshDatatypeDependencySelection, String> {
        self.select_inputs(Inputs::Modules(graph), root, scopes, imports, limits)
    }
    /// Compile the graph's root exports with actual source scopes and explicit
    /// reachable rule owners. `scopes` contains every source except the root;
    /// `root` must own the exact source named by the graph pin.
    pub fn compile_datatype_modules(
        &mut self,
        graph: Arc<VerifiedDatatypeModules>,
        root: DeclarationScope,
        scopes: &BTreeMap<String, DeclarationScope>,
        imports: &[DatatypeDependencyImport],
        implementations: &[DatatypeImplementation],
        bindings: &BTreeMap<String, Arc<FreshDatatypeRuleBinding>>,
        limits: ReferenceTraversalLimits,
    ) -> Result<FreshDependencyDatatypeCompilation, DatatypeDependencyCompileError> {
        self.compile_inputs(
            Inputs::Modules(graph),
            root,
            scopes,
            imports,
            implementations,
            bindings,
            limits,
        )
    }
    /// Bind an explicitly selected leaf rule implementation, without activation
    /// by pin, automatic imports or a manufactured source context.
    pub fn link_datatype_module_rule(
        &self,
        graph: &VerifiedDatatypeModules,
        module: &str,
        scope: DeclarationScope,
        registration: DatatypeDependencyRuleRegistration,
    ) -> Result<FreshDatatypeRuleBinding, String> {
        self.link_input_rule(InputRef::Modules(graph), module, scope, registration)
    }
}
