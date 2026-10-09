use super::*;
use crate::{
    datatype_validation::{
        self, RuleValidation, ValidationInput, ValidationLimits, ValidationRuntime, ValidationStop,
        ValidationStopReason,
    },
    eval::Item,
};
use cem_ml::schema::datatype_validation::CandidateRequirement;
#[derive(Debug, Clone)]
pub struct CardinalityRejection {
    pub source: SchemaDeclarationNode,
    pub required: ItemBounds,
    pub actual: usize,
}
#[derive(Debug, Clone)]
pub struct DatatypeValidation {
    pub accepted: Option<bool>,
    pub enumerations: Vec<crate::datatype_enumeration::EnumerationValidation>,
    /// Includes diagnostics from a comparison that stopped before membership completed.
    pub enumeration_diagnostics: Vec<cem_ml::diagnostics::Diagnostic>,
    pub comparisons: usize,
    pub enumeration_stop: Option<SchemaDeclarationNode>,
    pub completed: Vec<RuleValidation>,
    pub cardinality: Vec<CardinalityRejection>,
    pub stopped: Option<ValidationStop>,
}
impl DatatypeValidation {
    fn stop(mut self, reason: ValidationStopReason) -> Self {
        self.accepted = None;
        self.stopped = Some(ValidationStop {
            behavior: None,
            reason,
        });
        self
    }
}
impl ExecutableDatatype {
    /// The lifecycle consumer supplies complete native values. An absent value is
    /// not an empty list, and unresolved references must stay in their envelope.
    pub fn validate(
        &self,
        input: &ValidationInput,
        runtime: &ValidationRuntime<'_>,
        limits: ValidationLimits,
    ) -> DatatypeValidation {
        let mut result = DatatypeValidation {
            accepted: Some(true),
            enumerations: vec![],
            enumeration_diagnostics: vec![],
            comparisons: 0,
            enumeration_stop: None,
            completed: vec![],
            cardinality: vec![],
            stopped: None,
        };
        let mut calls = vec![(self, input.value.as_slice())];
        // Lists currently admit scalar item contracts only, so scheduling is flat.
        if let Some(item) = &self.item {
            if input.value.len() > limits.max_input_values {
                return result.stop(ValidationStopReason::Limit("input-values"));
            }
            for (index, value) in input.value.iter().enumerate() {
                if index % 64 == 0 {
                    if let Err(e) = runtime.control.check_scope(runtime.scope) {
                        return result.stop(ValidationStopReason::Control(e));
                    }
                }
                calls.push((item.as_ref(), std::slice::from_ref(value)));
            }
        }
        let mut rules = 0usize;
        let mut values = 0usize;
        // Preflight the entire invocation before calling any implementation.
        for (descriptor, value) in &calls {
            if let Err(e) = runtime.control.check_scope(runtime.scope) {
                return result.stop(ValidationStopReason::Control(e));
            }
            rules = rules.saturating_add(descriptor.rules.len());
            values = values
                .saturating_add(value.len())
                .saturating_add(input.candidate.len());
            if rules > limits.max_rules {
                return result.stop(ValidationStopReason::Limit("rules"));
            }
            if values > limits.max_input_values {
                return result.stop(ValidationStopReason::Limit("input-values"));
            }
            if input.candidate.len() > 1
                || input
                    .candidate
                    .iter()
                    .any(|v| !datatype_validation::native_node(v))
            {
                return result.stop(ValidationStopReason::InvalidInput("candidate"));
            }
            if input.candidate.is_empty()
                && descriptor
                    .rules
                    .iter()
                    .any(|r| r.signature().candidate == CandidateRequirement::Required)
            {
                return result.stop(ValidationStopReason::MissingCandidate);
            }
            if matches!(descriptor.representation, ValueRepresentation::Scalar(_))
                && value.len() != 1
            {
                return result.stop(ValidationStopReason::InvalidInput("value"));
            }
            for (index, v) in value.iter().enumerate() {
                if index % 64 == 0 {
                    if let Err(e) = runtime.control.check_scope(runtime.scope) {
                        return result.stop(ValidationStopReason::Control(e));
                    }
                }
                let admitted = match descriptor.representation {
                    ValueRepresentation::Scalar(p) | ValueRepresentation::List(p) => {
                        datatype_validation::scalar(v, p)
                    }
                    ValueRepresentation::Nodes => datatype_validation::native_node(v),
                };
                if !admitted {
                    return result.stop(ValidationStopReason::InvalidInput("value"));
                }
            }
            for restriction in &descriptor.restrictions {
                if !restriction.bounds.admits(value.len()) {
                    result.cardinality.push(CardinalityRejection {
                        source: restriction.source.clone(),
                        required: restriction.bounds,
                        actual: value.len(),
                    });
                }
            }
        }
        if result.cardinality.len() > limits.max_diagnostics {
            return result.stop(ValidationStopReason::Limit("diagnostics"));
        }
        result.accepted = Some(result.cardinality.is_empty());
        let mut remaining_diagnostics = limits.max_diagnostics - result.cardinality.len();
        let mut remaining_comparisons = limits.max_comparisons;
        for (descriptor, value) in calls {
            let input = ValidationInput {
                value: Vec::<Item>::from(value),
                candidate: input.candidate.clone(),
                fallback: input.fallback.clone(),
            };
            let batch = datatype_validation::validate_rules(
                &descriptor.rules,
                &input,
                runtime,
                ValidationLimits {
                    max_diagnostics: remaining_diagnostics,
                    ..limits
                },
            );
            for completed in &batch.completed {
                remaining_diagnostics = remaining_diagnostics
                    .saturating_sub(completed.result.diagnostics.len())
                    .saturating_sub(completed.result.execution_diagnostics.len());
            }
            result.completed.extend(batch.completed);
            if batch.accepted.is_none() {
                result.accepted = None;
                result.stopped = batch.stopped;
                return result;
            }
            result.accepted = Some(result.accepted.unwrap() && batch.accepted.unwrap());
            let attribution = input
                .candidate
                .first()
                .map(crate::datatype_results::DiagnosticAttribution::from_node)
                .unwrap_or_else(|| input.fallback.clone());
            for restriction in &descriptor.enumerations {
                let outcome = restriction.validate(
                    &value[0],
                    runtime,
                    &attribution,
                    &mut remaining_comparisons,
                    &mut remaining_diagnostics,
                    &mut result.enumeration_diagnostics,
                );
                result.comparisons = limits.max_comparisons - remaining_comparisons;
                match outcome {
                    Ok(outcome) => {
                        result.accepted = Some(result.accepted.unwrap() && outcome.accepted);
                        result.enumerations.push(outcome);
                    }
                    Err(reason) => {
                        result.enumeration_stop =
                            Some(restriction.source().attribute("values").unwrap().clone());
                        return result.stop(reason);
                    }
                }
            }
        }
        result
    }
}
