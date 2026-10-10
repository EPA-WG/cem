//! Explicit attribute consumption. The host supplies completed references and
//! declaring-scope diagnostic bindings; this adapter never activates a model.
use crate::{
    datatype_compilation::DatatypeValidation,
    datatype_facets::BoundAttributeFacets,
    datatype_preparation::{DatatypePreparation, PreparationInput, PreparationLimits},
    datatype_validation::{ValidationInput, ValidationRuntime},
    preparation_evidence::{
        AttributePreparationInvocation, EvidenceStop, SealedPreparationEvidence,
    },
};
use cem_ml::{
    operation_control::ControlError,
    schema::{
        datatype_validation::ValueRepresentation,
        document_model::attribute_facets::{
            AttributeFacetValidation, FacetContext, FacetExecutionError, FacetInput, FacetLimits,
        },
    },
};

#[derive(Debug, Clone, Copy)]
pub struct AttributeValidationLimits {
    /// Preparation and datatype validation share these budgets. Local facets
    /// receive only the remaining diagnostic allowance and the same byte limit.
    pub preparation: PreparationLimits,
    pub max_facet_model_bytes: usize,
}
impl Default for AttributeValidationLimits {
    fn default() -> Self {
        Self {
            preparation: Default::default(),
            max_facet_model_bytes: FacetLimits::default().max_model_bytes,
        }
    }
}
#[derive(Debug, Clone)]
pub enum AttributeDatatypePhase {
    ExternalTyped {
        input: ValidationInput,
        validation: DatatypeValidation,
    },
    Lexical(DatatypePreparation),
    Pretyped {
        evidence: SealedPreparationEvidence,
        /// A fresh verdict for this consumption, never cached in the receipt.
        validation: DatatypeValidation,
    },
    Nodes {
        /// Preserve the exact consumer-supplied native views, including their
        /// navigation boundaries, original owners and authored descendants.
        input: ValidationInput,
        validation: DatatypeValidation,
    },
}
#[derive(Debug)]
pub enum AttributeValidationStop {
    ExternalTyped(crate::external_typed::ExternalTypedError),
    InvalidInput,
    Evidence(EvidenceStop),
    /// The retained datatype phase provides the specific incomplete reason.
    Datatype,
    Facets(FacetExecutionError<ControlError>),
    Control(ControlError),
}
#[derive(Debug)]
pub struct AttributeValidation {
    /// Publication requires Some(true); None is incomplete, not rejection.
    pub accepted: Option<bool>,
    pub datatype: Option<AttributeDatatypePhase>,
    pub facets: Option<AttributeFacetValidation>,
    pub stopped: Option<AttributeValidationStop>,
}
impl AttributeValidation {
    pub(crate) fn empty() -> Self {
        Self {
            accepted: None,
            datatype: None,
            facets: None,
            stopped: None,
        }
    }
    pub(crate) fn stop(mut self, reason: AttributeValidationStop) -> Self {
        self.accepted = None;
        self.stopped = Some(reason);
        self
    }
}
impl BoundAttributeFacets {
    /// Consume sealed preparation in its original live invocation. Missing input
    /// remains incomplete; present empty lists are represented by valid evidence.
    /// The embedding closes the invocation before changing facet/runtime context.
    pub fn validate_pretyped(
        &self,
        invocation: &AttributePreparationInvocation,
        evidence: Option<&SealedPreparationEvidence>,
        context: FacetContext<'_>,
        max_facet_model_bytes: usize,
    ) -> AttributeValidation {
        invocation.consume(self, evidence, context, max_facet_model_bytes)
    }

    pub(crate) fn validate_prepared(
        &self,
        evidence: &SealedPreparationEvidence,
        input: &ValidationInput,
        context: FacetContext<'_>,
        runtime: &ValidationRuntime<'_>,
        limits: AttributeValidationLimits,
    ) -> AttributeValidation {
        let mut report = AttributeValidation::empty();
        let validation =
            self.binding()
                .datatype
                .validate(input, runtime, limits.preparation.validation);
        let accepted = validation.accepted;
        let cost = diagnostic_cost(&validation);
        report.datatype = Some(AttributeDatatypePhase::Pretyped {
            evidence: evidence.clone(),
            validation,
        });
        let Some(accepted) = accepted else {
            return report.stop(AttributeValidationStop::Datatype);
        };
        let facet_input = match self.profile().family().representation() {
            ValueRepresentation::Scalar(_) => FacetInput::Scalar(&evidence.lexical().text),
            ValueRepresentation::List(_) => FacetInput::List {
                lexical: &evidence.lexical().text,
                count: evidence.values().len(),
            },
            ValueRepresentation::Nodes => {
                return report.stop(AttributeValidationStop::InvalidInput)
            }
        };
        self.finish(
            report,
            accepted,
            cost,
            facet_input,
            context,
            runtime,
            limits,
        )
    }
    /// Prepare original lexical input, validate its complete datatype contract,
    /// then intersect the local facets. Never invoke conversion or stringify nodes.
    pub fn validate_lexical(
        &self,
        input: &PreparationInput,
        context: FacetContext<'_>,
        runtime: &ValidationRuntime<'_>,
        limits: AttributeValidationLimits,
    ) -> AttributeValidation {
        let mut report = AttributeValidation::empty();
        if let Err(error) = runtime.control.check_scope(runtime.scope) {
            return report.stop(AttributeValidationStop::Control(error));
        }
        let representation = self.profile().family().representation();
        if representation == ValueRepresentation::Nodes {
            return report.stop(AttributeValidationStop::InvalidInput);
        }
        let prepared = self
            .binding()
            .datatype
            .prepare_lexical(input, runtime, limits.preparation);
        let accepted = prepared.accepted;
        let count = prepared.value.as_ref().map(Vec::len);
        let cost = prepared
            .diagnostics
            .len()
            .saturating_add(prepared.validation.as_ref().map_or(0, diagnostic_cost));
        report.datatype = Some(AttributeDatatypePhase::Lexical(prepared));
        let Some(accepted) = accepted else {
            return report.stop(AttributeValidationStop::Datatype);
        };
        let Some(count) = count else {
            // A completed lexical rejection has no prepared sequence to inspect.
            // Preserve rejection without manufacturing a value for local facets.
            if let Err(error) = runtime.control.check_scope(runtime.scope) {
                return report.stop(AttributeValidationStop::Control(error));
            }
            report.accepted = Some(false);
            return report;
        };
        let facet_input = match representation {
            ValueRepresentation::Scalar(_) => FacetInput::Scalar(&input.lexical.text),
            ValueRepresentation::List(_) => FacetInput::List {
                lexical: &input.lexical.text,
                count,
            },
            ValueRepresentation::Nodes => unreachable!(),
        };
        self.finish(
            report,
            accepted,
            cost,
            facet_input,
            context,
            runtime,
            limits,
        )
    }

    /// Validate a complete native sequence supplied by the reference consumer.
    /// Pending selection must remain in its lifecycle envelope, not an empty input.
    pub fn validate_nodes(
        &self,
        input: &ValidationInput,
        context: FacetContext<'_>,
        runtime: &ValidationRuntime<'_>,
        limits: AttributeValidationLimits,
    ) -> AttributeValidation {
        let mut report = AttributeValidation::empty();
        if let Err(error) = runtime.control.check_scope(runtime.scope) {
            return report.stop(AttributeValidationStop::Control(error));
        }
        if self.profile().family().representation() != ValueRepresentation::Nodes {
            return report.stop(AttributeValidationStop::InvalidInput);
        }
        let validation =
            self.binding()
                .datatype
                .validate(input, runtime, limits.preparation.validation);
        let accepted = validation.accepted;
        let cost = diagnostic_cost(&validation);
        report.datatype = Some(AttributeDatatypePhase::Nodes {
            input: input.clone(),
            validation,
        });
        let Some(accepted) = accepted else {
            return report.stop(AttributeValidationStop::Datatype);
        };
        self.finish(
            report,
            accepted,
            cost,
            FacetInput::Nodes {
                count: input.value.len(),
            },
            context,
            runtime,
            limits,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn finish(
        &self,
        mut report: AttributeValidation,
        datatype_accepted: bool,
        diagnostic_cost: usize,
        input: FacetInput<'_>,
        context: FacetContext<'_>,
        runtime: &ValidationRuntime<'_>,
        limits: AttributeValidationLimits,
    ) -> AttributeValidation {
        let Some(remaining) = limits
            .preparation
            .validation
            .max_diagnostics
            .checked_sub(diagnostic_cost)
        else {
            return report.stop(AttributeValidationStop::Facets(FacetExecutionError::Limit));
        };
        let result = self.contract().validate_with_check(
            input,
            context,
            FacetLimits {
                max_model_bytes: limits.max_facet_model_bytes,
                max_input_bytes: limits.preparation.max_lexical_bytes,
                max_diagnostics: remaining,
            },
            &mut || runtime.control.check_scope(runtime.scope),
        );
        match result {
            Ok(facets) => {
                report.accepted = Some(datatype_accepted && facets.accepted);
                report.facets = Some(facets);
                report
            }
            Err(error) => report.stop(AttributeValidationStop::Facets(error)),
        }
    }
}
/// Cardinality rejections consume diagnostics even though they retain a typed
/// report rather than a rendered diagnostic. Do not replenish that allowance.
pub(crate) fn diagnostic_cost(report: &DatatypeValidation) -> usize {
    report.completed.iter().fold(
        report
            .cardinality
            .len()
            .saturating_add(report.enumeration_diagnostics.len()),
        |cost, rule| {
            cost.saturating_add(rule.result.diagnostics.len())
                .saturating_add(rule.result.execution_diagnostics.len())
        },
    )
}
