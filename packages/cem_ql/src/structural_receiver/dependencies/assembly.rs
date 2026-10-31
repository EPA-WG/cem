//! CEMDM2 borrows the actual host; no conversion into the CEMDM1 wire type.
use super::*;
use crate::datatype_module_capsule::v2::VerifiedDatatypeModules;
impl FreshStructuralHost {
    pub fn register_datatype_assembly_scope(
        &mut self,
        graph: &VerifiedDatatypeModules,
        id: &str,
        context: Option<StandaloneExpressionContext>,
        policy: ReferenceScopePolicy,
    ) -> Result<DeclarationScope, String> {
        self.register_input_scope(InputRef::Assembly(graph), Some(id), context, policy)
    }
    pub fn prepare_datatype_assembly_lexical_scopes<F>(
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
        self.prepare_input_lexical_scopes(InputRef::Assembly(graph), Some(id), prepare)
    }
    /// Renew every ordinary selection. The result freezes no mutable provider.
    pub fn select_datatype_assembly(
        &mut self,
        graph: Arc<VerifiedDatatypeModules>,
        root: DeclarationScope,
        scopes: &BTreeMap<String, DeclarationScope>,
        imports: &[DatatypeDependencyImport],
        limits: ReferenceTraversalLimits,
    ) -> Result<FreshDatatypeDependencySelection, String> {
        self.select_inputs(Inputs::Assembly(graph), root, scopes, imports, limits)
    }
}
