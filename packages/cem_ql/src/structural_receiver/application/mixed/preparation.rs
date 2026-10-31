use super::*;
use crate::{
    datatype_availability::DatatypeAvailability,
    datatype_preparation::DatatypePreparation,
    preparation_evidence::{
        datatype::{DatatypePreparationInvocation, SealedDatatypePreparationEvidence},
        EvidenceStop,
    },
};
struct Availability {
    group: Weak<Group>,
}
impl std::fmt::Debug for Availability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MixedReceiverAvailability(..)")
    }
}
impl DatatypeAvailability for Availability {
    fn is_available(&self) -> bool {
        self.group
            .upgrade()
            .is_some_and(|g| g.active.load(Ordering::Acquire) && g.context.is_live())
    }
}
pub struct MixedPreparationInvocation {
    group: Arc<Group>,
    name: String,
    input: values::ReceiverInput,
    invocation: DatatypePreparationInvocation,
}
pub struct MixedPreparationEvidence {
    inner: SealedDatatypePreparationEvidence,
}
pub struct MixedPreparationResult {
    pub report: DatatypePreparation,
    pub evidence: Option<MixedPreparationEvidence>,
    pub stopped: Option<EvidenceStop>,
}
impl LinkedMixedReceivers {
    pub fn preparation_invocation(
        &self,
        host: &mut FreshStructuralHost,
        name: &str,
        input: values::ReceiverInput,
        limits: PreparationLimits,
    ) -> Result<MixedPreparationInvocation, String> {
        let _entry = self.group.enter(host)?;
        self.group.refresh(host)?;
        let value = values::lexical(host, name, &input)?;
        let receiver = self.group.receivers.get(name).ok_or(REFUSED)?;
        let Binding::Executable { root } = &receiver.binding else {
            return Err(REFUSED.into());
        };
        let owner = root.upgrade().ok_or(REFUSED)?;
        let guard = Arc::new(Availability {
            group: Arc::downgrade(&self.group),
        });
        let invocation = owner
            .with_runtime(host, |d, r, _| {
                DatatypePreparationInvocation::new(Arc::new(d.clone()), guard, value, r, limits)
            })?
            .map_err(|_| REFUSED)?;
        self.group.refresh(host)?;
        Ok(MixedPreparationInvocation {
            group: self.group.clone(),
            name: name.into(),
            input,
            invocation,
        })
    }
}
impl MixedPreparationInvocation {
    pub fn close(&self) {
        self.invocation.close();
    }
    pub fn remaining_limits(&self) -> PreparationLimits {
        self.invocation.remaining_limits()
    }
    pub fn prepare(
        &self,
        host: &mut FreshStructuralHost,
    ) -> Result<MixedPreparationResult, String> {
        let _entry = self.group.enter(host)?;
        self.group.refresh(host)?;
        let Binding::Executable { root } = &self.group.receivers[&self.name].binding else {
            return Err(REFUSED.into());
        };
        let root = root.upgrade().ok_or(REFUSED)?;
        let result = root.with_runtime(host, |_, _, contexts| {
            self.invocation.prepare_in_contexts(Some(contexts))
        })?;
        self.group.refresh(host)?;
        Ok(MixedPreparationResult {
            report: result.report,
            evidence: result
                .evidence
                .map(|inner| MixedPreparationEvidence { inner }),
            stopped: result.stopped,
        })
    }
    pub fn validate_prepared(
        &self,
        host: &mut FreshStructuralHost,
        input: &values::ReceiverInput,
        evidence: Option<&MixedPreparationEvidence>,
    ) -> Result<values::MixedDatatypeValidation, String> {
        let _entry = self.group.enter(host)?;
        self.group.refresh(host)?;
        if input.attribute != self.input.attribute
            || !Arc::ptr_eq(&input.source, &self.input.source)
        {
            return Err(REFUSED.into());
        }
        let _ = values::input(host, &self.name, input)?;
        let receiver = &self.group.receivers[&self.name];
        let Binding::Executable { root } = &receiver.binding else {
            return Err(REFUSED.into());
        };
        let root = root.upgrade().ok_or(REFUSED)?;
        let raw = evidence.map(|e| &e.inner);
        let datatype = root
            .with_runtime(host, |_, _, contexts| {
                self.invocation.consume_in_contexts(raw, Some(contexts))
            })?
            .map_err(|_| REFUSED)?;
        let diagnostics = values::counts(
            &self.group,
            receiver,
            input,
            host,
            raw.ok_or(REFUSED)?.values().len(),
        )?;
        self.invocation
            .spend_diagnostics(diagnostics.len())
            .map_err(|_| REFUSED)?;
        let accepted = datatype
            .accepted
            .map(|a| a && !diagnostics.iter().any(|d| d.severity.is_hard_violation()));
        self.group.refresh(host)?;
        Ok(values::MixedDatatypeValidation {
            accepted,
            datatype,
            diagnostics,
        })
    }
    pub fn token_spans<'a>(
        &self,
        host: &mut FreshStructuralHost,
        evidence: &'a MixedPreparationEvidence,
    ) -> Result<&'a [std::ops::Range<usize>], String> {
        let _entry = self.group.enter(host)?;
        self.group.refresh(host)?;
        if !self.invocation.matches_evidence(&evidence.inner) {
            return Err(REFUSED.into());
        }
        self.group.refresh(host)?;
        Ok(evidence.inner.token_spans())
    }
}
