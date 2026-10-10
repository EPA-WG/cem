//! Explicit typed-only producers. These handles attest immutable data and host
//! admission, never lexical derivation, original spelling, or a validation verdict.
use crate::{
    attribute_validation::{
        AttributeDatatypePhase, AttributeValidation, AttributeValidationLimits,
        AttributeValidationStop,
    },
    datatype_facets::BoundAttributeFacets,
    datatype_preparation::PreparationLimits,
    datatype_results::DiagnosticAttribution,
    datatype_validation::{self, ValidationInput, ValidationRuntime},
    eval::{AtomValue, Item},
};
use cem_ml::{
    operation_control::ControlError,
    schema::{
        attribute_datatypes::{NativeAttributeTypedAdmission, NativeAttributeTypedValue},
        datatype_contracts::CompiledDatatypeContract,
        datatype_validation::ValueRepresentation,
        document_model::attribute_facets::{FacetContext, FacetInput},
    },
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Weak,
};

#[derive(Debug, Clone)]
pub enum ExternalTypedError {
    NotAdmitted,
    MissingValue,
    InvalidValue,
    BindingMismatch,
    Expired,
    Limit,
    Control(ControlError),
}
#[derive(Debug)]
struct Admission {
    binding: BoundAttributeFacets,
    publication: Option<Weak<AtomicBool>>,
}
impl Admission {
    fn current(&self) -> bool {
        self.publication
            .as_ref()
            .is_none_or(|p| p.upgrade().is_some_and(|p| p.load(Ordering::Acquire)))
    }
}
#[derive(Debug)]
struct Producer {
    admission: Arc<Admission>,
    name: String,
    active: AtomicBool,
}
#[derive(Debug, Clone)]
pub struct ExternalTypedProducer(Arc<Producer>);
#[derive(Debug, Clone)]
pub struct ExternalTypedValue {
    producer: Arc<Producer>,
    values: Arc<[Item]>,
    bytes: usize,
}
impl ExternalTypedValue {
    pub fn values(&self) -> &[Item] {
        &self.values
    }
    pub fn producer(&self) -> &str {
        &self.producer.name
    }
    pub fn native_handle(&self) -> NativeAttributeTypedValue {
        NativeAttributeTypedValue::new(self.clone())
    }
    fn check(&self, binding: &BoundAttributeFacets) -> Result<(), ExternalTypedError> {
        if !self.producer.admission.binding.same_binding(binding) {
            return Err(ExternalTypedError::BindingMismatch);
        }
        if !self.producer.active.load(Ordering::Acquire) || !self.producer.admission.current() {
            return Err(ExternalTypedError::Expired);
        }
        Ok(())
    }
}
pub(crate) fn native_admission(
    binding: &BoundAttributeFacets,
    publication: &Arc<AtomicBool>,
) -> Option<NativeAttributeTypedAdmission> {
    (binding.binding().datatype.admits_external_typed() && publication.load(Ordering::Acquire))
        .then(|| {
            NativeAttributeTypedAdmission::new(Arc::new(Admission {
                binding: binding.clone(),
                publication: Some(Arc::downgrade(publication)),
            }))
        })
}
impl ExternalTypedProducer {
    /// Host authority for this exact explicitly bound typed-only contract.
    pub fn new(binding: &BoundAttributeFacets, name: &str) -> Result<Self, ExternalTypedError> {
        Self::register(
            Arc::new(Admission {
                binding: binding.clone(),
                publication: None,
            }),
            name,
        )
    }
    /// Only the engine's private issuer can turn a core capability into admission.
    pub fn from_native(
        admission: &NativeAttributeTypedAdmission,
        name: &str,
    ) -> Result<Self, ExternalTypedError> {
        let admission = admission
            .downcast_ref::<Arc<Admission>>()
            .ok_or(ExternalTypedError::NotAdmitted)?;
        Self::register(admission.clone(), name)
    }
    fn register(admission: Arc<Admission>, name: &str) -> Result<Self, ExternalTypedError> {
        if !admission.binding.binding().datatype.admits_external_typed() || name.trim().is_empty() {
            return Err(ExternalTypedError::NotAdmitted);
        }
        if name.len() > 4096 {
            return Err(ExternalTypedError::Limit);
        }
        if !admission.current() {
            return Err(ExternalTypedError::Expired);
        }
        Ok(Self(Arc::new(Producer {
            admission,
            name: name.into(),
            active: AtomicBool::new(true),
        })))
    }
    /// Revoke every clone and value from this producer. Names cannot revive it.
    pub fn close(&self) {
        self.0.active.store(false, Ordering::Release);
    }
    pub fn produce(
        &self,
        values: Option<Vec<Item>>,
        runtime: &ValidationRuntime<'_>,
        limits: PreparationLimits,
    ) -> Result<ExternalTypedValue, ExternalTypedError> {
        runtime
            .control
            .check_scope(runtime.scope)
            .map_err(ExternalTypedError::Control)?;
        if !self.0.active.load(Ordering::Acquire) || !self.0.admission.current() {
            return Err(ExternalTypedError::Expired);
        }
        let values = values.ok_or(ExternalTypedError::MissingValue)?;
        if values.len() > limits.max_output_values
            || values.len() > limits.validation.max_input_values
        {
            return Err(ExternalTypedError::Limit);
        }
        let representation = self.0.admission.binding.binding().datatype.representation();
        let primitive = match representation {
            ValueRepresentation::Scalar(p) if values.len() == 1 => p,
            ValueRepresentation::List(p) => p,
            _ => return Err(ExternalTypedError::InvalidValue),
        };
        let mut bytes = 0usize;
        let mut metadata =
            crate::preparation_evidence::MetadataBudget::new(runtime.control, runtime.scope);
        for value in &values {
            runtime
                .control
                .check_scope(runtime.scope)
                .map_err(ExternalTypedError::Control)?;
            let size = match value {
                Item::Atomic(
                    AtomValue::String(s) | AtomValue::Decimal(s) | AtomValue::AnyUri(s),
                ) => s.len(),
                Item::Atomic(_) => 0,
                _ => {
                    let (text, source) = crate::typed_scalar::immutable_storage(value)
                        .ok_or(ExternalTypedError::InvalidValue)?;
                    if let Some(source) = source {
                        metadata.source(source).map_err(|e| match e {
                            crate::preparation_evidence::EvidenceStop::Control(e) => {
                                ExternalTypedError::Control(e)
                            }
                            _ => ExternalTypedError::Limit,
                        })?;
                    }
                    text.len()
                }
            };
            bytes = bytes
                .checked_add(size)
                .filter(|n| *n <= limits.max_lexical_bytes)
                .ok_or(ExternalTypedError::Limit)?;
            // Decimal's native storage is textual. Validate that storage without
            // parsing an attribute string, normalization, or representation coercion.
            if let Item::Atomic(AtomValue::Decimal(s)) = value {
                if !valid_decimal(s) {
                    return Err(ExternalTypedError::InvalidValue);
                }
            }
            if !datatype_validation::scalar(value, primitive) {
                return Err(ExternalTypedError::InvalidValue);
            }
        }
        runtime
            .control
            .check_scope(runtime.scope)
            .map_err(ExternalTypedError::Control)?;
        let value = ExternalTypedValue {
            producer: self.0.clone(),
            values: values.into(),
            bytes,
        };
        value.check(&self.0.admission.binding)?;
        Ok(value)
    }
}
fn valid_decimal(s: &str) -> bool {
    let s = s.strip_prefix(['+', '-']).unwrap_or(s);
    let (whole, fraction) = s.split_once('.').unwrap_or((s, ""));
    (!whole.is_empty() || !fraction.is_empty())
        && whole
            .bytes()
            .chain(fraction.bytes())
            .all(|b| b.is_ascii_digit())
}
pub struct ExternalTypedInput<'a> {
    pub value: Option<&'a ExternalTypedValue>,
    pub candidate: Vec<Item>,
    pub fallback: DiagnosticAttribution,
}
impl BoundAttributeFacets {
    /// Every use is a new bounded validation, never a cached receipt verdict.
    pub fn validate_external(
        &self,
        input: ExternalTypedInput<'_>,
        context: FacetContext<'_>,
        runtime: &ValidationRuntime<'_>,
        limits: AttributeValidationLimits,
    ) -> AttributeValidation {
        let report = AttributeValidation::empty();
        let fail = |error| {
            AttributeValidation::empty().stop(AttributeValidationStop::ExternalTyped(error))
        };
        if let Err(e) = runtime.control.check_scope(runtime.scope) {
            return fail(ExternalTypedError::Control(e));
        }
        let Some(value) = input.value else {
            return fail(ExternalTypedError::MissingValue);
        };
        if let Err(e) = value.check(self) {
            return fail(e);
        }
        if value.bytes > limits.preparation.max_lexical_bytes
            || value.values.len() > limits.preparation.max_output_values
            || value.values.len().saturating_add(input.candidate.len())
                > limits.preparation.validation.max_input_values
        {
            return fail(ExternalTypedError::Limit);
        }
        let input = ValidationInput {
            value: value.values.to_vec(),
            candidate: input.candidate,
            fallback: input.fallback,
        };
        let validation =
            self.binding()
                .datatype
                .validate(&input, runtime, limits.preparation.validation);
        let accepted = validation.accepted;
        let cost = crate::attribute_validation::diagnostic_cost(&validation);
        let facet_input = match self.profile().family().representation() {
            ValueRepresentation::Scalar(_) => FacetInput::TypedScalar,
            ValueRepresentation::List(_) => FacetInput::TypedList {
                count: input.value.len(),
            },
            _ => return fail(ExternalTypedError::NotAdmitted),
        };
        let mut report = report;
        report.datatype = Some(AttributeDatatypePhase::ExternalTyped { input, validation });
        let report = match accepted {
            Some(accepted) => self.finish(
                report,
                accepted,
                cost,
                facet_input,
                context,
                runtime,
                limits,
            ),
            None => report.stop(AttributeValidationStop::Datatype),
        };
        if let Err(e) = value.check(self) {
            return report.stop(AttributeValidationStop::ExternalTyped(e));
        }
        if let Err(e) = runtime.control.check_scope(runtime.scope) {
            return report.stop(AttributeValidationStop::Control(e));
        }
        report
    }
}
