//! Original-binding capture and explicit fresh-host relinking. Source packets
//! are passive dependencies; the caller still owns checkpoint/privacy policy.
use super::*;
use crate::query_validator_capsule::handoff::{self, BuiltQueryHandoff, VerifiedQueryHandoff};

impl FreshStructuralHost {
    pub fn capture_query_handoff(
        &self,
        binding: &FreshQueryBinding,
    ) -> Result<BuiltQueryHandoff, String> {
        let _entry = self.runtime_entry()?;
        self.check().map_err(|_| INVALID)?;
        if !binding.live(self) {
            return Err(INVALID.into());
        }
        // Native callback requirements have a different code-ownership contract.
        let declaration = match &binding.declaration {
            BindingOwner::Query(declaration) => declaration,
            #[cfg(not(target_arch = "wasm32"))]
            BindingOwner::Native(_) => return Err(INVALID.into()),
        };
        let datatype = binding.rule.source_declaration();
        if !Arc::ptr_eq(datatype.document(), declaration.source().ast_owner()) {
            return Err(INVALID.into());
        }
        let packet = handoff::export(
            declaration,
            datatype.node_id(),
            binding.rule.result_adapter(),
            &self.control,
        )?;
        self.check().map_err(|_| INVALID)?;
        if !binding.live(self) {
            return Err(INVALID.into());
        }
        Ok(packet)
    }
    /// Register this exact restored declaration's source in a real host scope
    /// first. Supply fresh checked functions/adapters; no scope or grant is made
    /// from bytes and no old request/context/budget is copied or reset.
    pub fn link_query_handoff(
        &self,
        handoff: &VerifiedQueryHandoff,
        scope: DeclarationScope,
        functions: &SelectedFunctionRegistry,
        adapter: DatatypeResultAdapter,
    ) -> Result<FreshQueryBinding, String> {
        let _entry = self.runtime_entry()?;
        self.check().map_err(|_| INVALID)?;
        if adapter.result_contract() != &handoff.selection.result
            || adapter.diagnostic_contract() != &handoff.selection.diagnostic
        {
            return Err(INVALID.into());
        }
        self.link_query(
            handoff.declaration().clone(),
            scope,
            handoff.selection.datatype,
            functions,
            adapter,
        )
    }
}
