//! Native datatype-root preparation uses the same evidence and budget rules as
//! attribute preparation. No constructor or codec exposes sealed authority.
use super::*;
use crate::{
    datatype_availability::DatatypeAvailability,
    datatype_compilation::{DatatypeValidation, ExecutableDatatype},
};
struct State {
    datatype: Arc<ExecutableDatatype>,
    input: PreparationInput,
    origin: PreparationOrigin,
    guard: Arc<dyn DatatypeAvailability>,
    control: OperationControl,
    scope: ExecutionScopeId,
    query: EvaluationContext,
    closed: AtomicBool,
    checking: AtomicBool,
    budget: Mutex<Budget>,
}
impl State {
    fn check(&self) -> Result<(), EvidenceStop> {
        self.control
            .check_scope(self.scope)
            .map_err(EvidenceStop::Control)?;
        if self.closed.load(Ordering::Acquire) {
            return Err(EvidenceStop::Closed);
        }
        let _checking = AvailabilityLease::acquire(&self.checking)?;
        if self.closed.load(Ordering::Acquire)
            || !self.guard.is_available()
            || !self.datatype.preparation_available()
            || !self.datatype.facets_available()
        {
            return Err(EvidenceStop::Closed);
        }
        check_tokenizers(&self.datatype, &self.control, self.scope)?;
        if self
            .query
            .execution_budget
            .as_ref()
            .and_then(|b| b.failure())
            .is_some()
        {
            return Err(EvidenceStop::QueryFailed);
        }
        self.control
            .check_scope(self.scope)
            .map_err(EvidenceStop::Control)?;
        if self.closed.load(Ordering::Acquire) {
            return Err(EvidenceStop::Closed);
        }
        Ok(())
    }
}
pub(crate) struct DatatypePreparationInvocation {
    state: Arc<State>,
}
impl DatatypePreparationInvocation {
    pub(crate) fn new(
        datatype: Arc<ExecutableDatatype>,
        guard: Arc<dyn DatatypeAvailability>,
        input: PreparationInput,
        runtime: &ValidationRuntime<'_>,
        limits: PreparationLimits,
    ) -> Result<Self, EvidenceStop> {
        runtime
            .control
            .check_scope(runtime.scope)
            .map_err(EvidenceStop::Control)?;
        if input.lexical.text.len() > limits.max_lexical_bytes {
            return Err(EvidenceStop::Limit("lexical-bytes"));
        }
        let mut metadata = MetadataBudget::new(runtime.control, runtime.scope);
        metadata.source(&input.lexical.source)?;
        metadata.attribution(&input.fallback)?;
        let origin = original_source(None, &input)?;
        let state = Arc::new(State {
            datatype,
            guard,
            input,
            origin,
            control: runtime.control.clone(),
            scope: runtime.scope,
            query: runtime.with_query_budget().query,
            closed: AtomicBool::new(false),
            checking: AtomicBool::new(false),
            budget: Mutex::new(Budget {
                remaining: limits,
                busy: false,
            }),
        });
        state.check()?;
        Ok(Self { state })
    }
    pub(crate) fn close(&self) {
        self.state.closed.store(true, Ordering::Release)
    }
    pub(crate) fn remaining_limits(&self) -> PreparationLimits {
        self.state
            .budget
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remaining
    }
    pub(crate) fn prepare(&self) -> DatatypePreparationEvidenceResult {
        let mut result = DatatypePreparationEvidenceResult {
            evidence: None,
            report: DatatypePreparation {
                input: self.state.input.clone(),
                preparer: self
                    .state
                    .datatype
                    .preparation()
                    .map(|p| p.identity().clone()),
                value: None,
                token_spans: vec![],
                accepted: None,
                diagnostics: vec![],
                validation: None,
                preparations: 0,
                stopped: None,
            },
            stopped: None,
        };
        let outcome = (|| {
            self.state.check()?;
            let mut lease = BudgetLease::from_budget(&self.state.budget)?;
            let runtime = ValidationRuntime {
                control: &self.state.control,
                scope: self.state.scope,
                query: self.state.query.clone(),
            };
            let (report, visits) = self.state.datatype.prepare_without_validation(
                &self.state.input,
                &runtime,
                lease.available,
            );
            lease.refund = Some(preparation_refund(lease.available, &report, visits));
            result.report = report;
            self.state.check()?;
            if let Some(stop) = &result.report.stopped {
                return Err(EvidenceStop::Preparation(stop.clone()));
            }
            let Some(values) = &result.report.value else {
                return Ok(None);
            };
            inspect_prepared_values(
                &self.state.input,
                values,
                &result.report.diagnostics,
                &self.state.control,
                self.state.scope,
                lease.available,
                &mut || self.state.check(),
            )?;
            let evidence = SealedDatatypePreparationEvidence {
                inner: Arc::new(Evidence {
                    invocation: Arc::downgrade(&self.state),
                    input: self.state.input.clone(),
                    origin: self.state.origin.clone(),
                    values: values.clone(),
                    spans: result.report.token_spans.clone(),
                    diagnostics: result.report.diagnostics.clone(),
                }),
            };
            self.state.check()?;
            Ok(Some(evidence))
        })();
        match outcome {
            Ok(evidence) => result.evidence = evidence,
            Err(stop) => result.stopped = Some(stop),
        }
        result
    }
    pub(crate) fn consume(
        &self,
        evidence: Option<&SealedDatatypePreparationEvidence>,
    ) -> Result<DatatypeValidation, EvidenceStop> {
        let evidence = evidence.ok_or(EvidenceStop::MissingEvidence)?;
        let issuer = evidence
            .inner
            .invocation
            .upgrade()
            .ok_or(EvidenceStop::Closed)?;
        if !Arc::ptr_eq(&issuer, &self.state) {
            return Err(EvidenceStop::ForeignInvocation);
        }
        self.state.check()?;
        let mut lease = BudgetLease::from_budget(&self.state.budget)?;
        let inspection = 1usize
            .saturating_add(evidence.values().len())
            .saturating_add(evidence.token_spans().len())
            .saturating_add(evidence.candidate().len());
        let Some(remaining) = lease
            .available
            .validation
            .max_input_values
            .checked_sub(inspection)
        else {
            let mut refund = lease.available;
            refund.validation.max_input_values = 0;
            lease.refund = Some(refund);
            return Err(EvidenceStop::Limit("evidence-input-values"));
        };
        let input = ValidationInput {
            value: evidence.values().to_vec(),
            candidate: evidence.candidate().to_vec(),
            fallback: self.state.input.fallback.clone(),
        };
        let datatype = &self.state.datatype;
        let mut rules = datatype.rules().len();
        let mut visits = input.value.len().saturating_add(input.candidate.len());
        if let Some(item) = datatype.item() {
            rules = rules.saturating_add(item.rules().len().saturating_mul(input.value.len()));
            visits = visits.saturating_add(
                input
                    .value
                    .len()
                    .saturating_mul(1usize.saturating_add(input.candidate.len())),
            );
        }
        let runtime = ValidationRuntime {
            control: &self.state.control,
            scope: self.state.scope,
            query: self.state.query.clone(),
        };
        let report = datatype.validate(
            &input,
            &runtime,
            crate::datatype_validation::ValidationLimits {
                max_input_values: remaining,
                ..lease.available.validation
            },
        );
        let mut refund = lease.available;
        refund.validation.max_input_values = remaining.saturating_sub(visits);
        refund.validation.max_rules = refund.validation.max_rules.saturating_sub(rules);
        refund.validation.max_comparisons = refund
            .validation
            .max_comparisons
            .saturating_sub(report.comparisons);
        refund.validation.max_diagnostics = refund.validation.max_diagnostics.saturating_sub(
            crate::attribute_validation::diagnostic_cost(&report)
                .saturating_add(stopped_diagnostic_cost(&report)),
        );
        lease.refund = Some(refund);
        self.state.check()?;
        Ok(report)
    }
}
struct Evidence {
    invocation: Weak<State>,
    input: PreparationInput,
    origin: PreparationOrigin,
    values: Vec<Item>,
    spans: Vec<Range<usize>>,
    diagnostics: Vec<Diagnostic>,
}
/// Immutable preparation evidence, never a validation verdict. The private issuer
/// and owning compilation must still be live for any consumption.
#[derive(Clone)]
pub struct SealedDatatypePreparationEvidence {
    inner: Arc<Evidence>,
}
impl SealedDatatypePreparationEvidence {
    pub fn lexical(&self) -> &LexicalInput {
        &self.inner.input.lexical
    }
    pub fn values(&self) -> &[Item] {
        &self.inner.values
    }
    pub fn candidate(&self) -> &[Item] {
        &self.inner.input.candidate
    }
    pub fn token_spans(&self) -> &[Range<usize>] {
        &self.inner.spans
    }
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.inner.diagnostics
    }
    pub fn origin(&self) -> &PreparationOrigin {
        &self.inner.origin
    }
    pub fn is_live(&self) -> bool {
        self.inner
            .invocation
            .upgrade()
            .is_some_and(|s| s.check().is_ok())
    }
}
pub struct DatatypePreparationEvidenceResult {
    pub evidence: Option<SealedDatatypePreparationEvidence>,
    pub report: DatatypePreparation,
    pub stopped: Option<EvidenceStop>,
}
