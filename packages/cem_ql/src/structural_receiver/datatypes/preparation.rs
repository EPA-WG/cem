use super::*;
use crate::{
    datatype_availability::DatatypeAvailability,
    datatype_preparation::{PreparationInput, PreparationLimits},
    preparation_evidence::{
        datatype::{
            DatatypePreparationEvidenceResult, DatatypePreparationInvocation,
            SealedDatatypePreparationEvidence,
        },
        EvidenceStop,
    },
};
use std::sync::Weak;
#[derive(Debug)]
struct Owner {
    active: Weak<AtomicBool>,
    context: AttributeDatatypeContextLease,
    control: OperationControl,
}
impl DatatypeAvailability for Owner {
    fn is_available(&self) -> bool {
        self.active
            .upgrade()
            .is_some_and(|a| a.load(Ordering::Acquire))
            && self.context.is_live()
            && self.control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_ok()
    }
}
pub struct FreshDatatypePreparationInvocation {
    owner: Arc<Owner>,
    sources: Arc<VerifiedDatatypeSources>,
    scope: DeclarationScope,
    invocation: DatatypePreparationInvocation,
}
impl FreshDatatypePreparationInvocation {
    fn check(&self, host: &FreshStructuralHost) -> Result<(), EvidenceStop> {
        host.control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(EvidenceStop::Control)?;
        if host.check().is_err()
            || !self.owner.is_available()
            || !self.owner.context.matches(&host.context)
            || host.registry_hash != self.sources.pin().registry_hash
            || host
                .inner
                .registered_validator_context(self.scope, self.sources.source())
                .is_none()
        {
            return Err(EvidenceStop::Closed);
        }
        Ok(())
    }
    pub fn close(&self) {
        self.invocation.close()
    }
    pub fn remaining_limits(&self) -> PreparationLimits {
        self.invocation.remaining_limits()
    }
    pub fn prepare(
        &self,
        host: &FreshStructuralHost,
    ) -> Result<DatatypePreparationEvidenceResult, EvidenceStop> {
        self.check(host)?;
        let result = self.invocation.prepare();
        self.check(host)?;
        Ok(result)
    }
    pub fn validate_prepared(
        &self,
        host: &FreshStructuralHost,
        evidence: Option<&SealedDatatypePreparationEvidence>,
    ) -> Result<DatatypeValidation, EvidenceStop> {
        self.check(host)?;
        let result = self.invocation.consume(evidence);
        self.check(host)?;
        result
    }
}
impl FreshDatatypeCompilation {
    pub fn preparation_invocation(
        &self,
        host: &FreshStructuralHost,
        name: &str,
        input: PreparationInput,
        limits: PreparationLimits,
    ) -> Result<FreshDatatypePreparationInvocation, EvidenceStop> {
        host.control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(EvidenceStop::Control)?;
        self.check(host).map_err(|_| EvidenceStop::Closed)?;
        let source = self
            .sources
            .source_named(name)
            .filter(|source| {
                self.sources
                    .roots()
                    .contains(&source.declaration().node_id())
            })
            .ok_or(EvidenceStop::BindingMismatch)?;
        let datatype = self
            .compilation
            .contracts
            .iter()
            .filter_map(|c| c.as_any().downcast_ref::<ExecutableDatatype>())
            .find(|c| {
                c.source().declaration().identity() == source.declaration().identity()
                    && c.source().scope().identity() == source.scope().identity()
            })
            .ok_or(EvidenceStop::BindingMismatch)?;
        let query = host
            .inner
            .registered_validator_context(self.scope, self.sources.source())
            .ok_or(EvidenceStop::Closed)?;
        let runtime = ValidationRuntime {
            control: &host.control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query,
        };
        let owner = Arc::new(Owner {
            active: Arc::downgrade(&self.active),
            context: self.context.clone(),
            control: host.control.clone(),
        });
        let invocation = DatatypePreparationInvocation::new(
            Arc::new(datatype.clone()),
            owner.clone(),
            input,
            &runtime,
            limits,
        )?;
        let result = FreshDatatypePreparationInvocation {
            owner,
            sources: self.sources.clone(),
            scope: self.scope,
            invocation,
        };
        result.check(host)?;
        Ok(result)
    }
}
