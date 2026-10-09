//! Explicit native conversion capabilities. Validation never calls these implicitly.
use crate::{
    datatype_compilation::{DatatypeValidation, ExecutableDatatype},
    datatype_results::DiagnosticAttribution,
    datatype_validation::{self, ValidationInput, ValidationLimits, ValidationRuntime},
    eval::{Item, RetainedCemNode},
};
use cem_ml::{
    diagnostics::Diagnostic,
    operation_control::ControlError,
    parser::tree::RetainedCemTree,
    schema::{
        datatype_contracts::LexicalInput,
        datatype_registry::{DatatypeKind, DatatypeSource},
        datatype_validation::{CandidateRequirement, ValueRepresentation},
    },
};
use std::{collections::BTreeSet, fmt::Debug, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionRepresentation {
    Lexical,
    Values(ValueRepresentation),
}
#[derive(Debug, Clone, Copy)]
pub struct ConversionSignature {
    pub kind: DatatypeKind,
    pub input: ConversionRepresentation,
    pub output: ValueRepresentation,
    pub candidate: CandidateRequirement,
}
#[derive(Debug, Clone)]
pub enum ConversionValue {
    Lexical(LexicalInput),
    Values(Vec<Item>),
}
#[derive(Debug, Clone)]
pub struct ConversionInput {
    pub value: ConversionValue,
    pub candidate: Vec<Item>,
    pub fallback: DiagnosticAttribution,
}
/// The tag determines completion; diagnostics and empty sequences never do.
#[derive(Debug, Clone)]
pub enum ConversionExecution {
    Converted {
        value: Vec<Item>,
        diagnostics: Vec<Diagnostic>,
    },
    Rejected {
        diagnostics: Vec<Diagnostic>,
    },
    Pending(Vec<Diagnostic>),
    Unavailable(Vec<Diagnostic>),
    Failed(Vec<Diagnostic>),
}
pub struct ConversionCall<'a> {
    pub value: &'a ConversionValue,
    /// The original declaration whose registered implementation was selected.
    pub datatype: &'a Item,
    pub candidate: &'a [Item],
    pub runtime: &'a ValidationRuntime<'a>,
    pub limits: ConversionLimits,
}
/// Native callbacks cooperate with the caller's control and bound their own work.
/// Returning native nodes requires clones of input views, retaining access boundaries.
pub trait NativeDatatypeConverter: Debug + Send + Sync {
    fn convert(&self, call: ConversionCall<'_>) -> ConversionExecution;
}
#[derive(Debug, Clone)]
pub struct ConverterIdentity {
    pub source: DatatypeSource,
    pub implementation: String,
}
#[derive(Debug, Clone)]
pub struct RegisteredDatatypeConverter {
    identity: ConverterIdentity,
    signature: ConversionSignature,
    implementation: Arc<dyn NativeDatatypeConverter>,
}
impl RegisteredDatatypeConverter {
    pub fn new(
        source: DatatypeSource,
        id: impl Into<String>,
        signature: ConversionSignature,
        implementation: impl NativeDatatypeConverter + 'static,
    ) -> Result<Self, &'static str> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err("empty-converter-id");
        }
        let valid = match signature.kind {
            DatatypeKind::Node => signature.output == ValueRepresentation::Nodes,
            DatatypeKind::List => matches!(signature.output, ValueRepresentation::List(_)),
            _ => matches!(signature.output, ValueRepresentation::Scalar(_)),
        };
        if !valid {
            return Err("converter-kind-output-mismatch");
        }
        // Native conversion preserves input handles; it is not data import or scalar extraction.
        if (signature.input == ConversionRepresentation::Values(ValueRepresentation::Nodes))
            != (signature.output == ValueRepresentation::Nodes)
        {
            return Err("converter-node-scalar-boundary");
        }
        Ok(Self {
            identity: ConverterIdentity {
                source,
                implementation: id,
            },
            signature,
            implementation: Arc::new(implementation),
        })
    }
    pub fn identity(&self) -> &ConverterIdentity {
        &self.identity
    }
    pub fn signature(&self) -> ConversionSignature {
        self.signature
    }
    pub(crate) fn bind(&self, tree: Arc<RetainedCemTree>) -> Option<BoundDatatypeConverter> {
        let source = self.identity.source.declaration();
        if !Arc::ptr_eq(tree.ast_owner(), source.document()) {
            return None;
        }
        Some(BoundDatatypeConverter {
            registration: self.clone(),
            datatype: RetainedCemNode::new(tree, source.node_id())?.query_item(),
        })
    }
}
#[derive(Debug, Clone)]
pub enum ConverterBinding {
    Unavailable,
    Ready(RegisteredDatatypeConverter),
}
#[derive(Debug, Clone)]
pub struct BoundDatatypeConverter {
    registration: RegisteredDatatypeConverter,
    datatype: Item,
}
impl BoundDatatypeConverter {
    pub fn identity(&self) -> &ConverterIdentity {
        self.registration.identity()
    }
    pub fn signature(&self) -> ConversionSignature {
        self.registration.signature
    }
}
#[derive(Debug, Clone, Copy)]
pub struct ConversionLimits {
    pub max_lexical_bytes: usize,
    pub max_input_values: usize,
    pub max_output_values: usize,
    pub validation: ValidationLimits,
}
impl Default for ConversionLimits {
    fn default() -> Self {
        Self {
            max_lexical_bytes: 1_048_576,
            max_input_values: 100_000,
            max_output_values: 100_000,
            validation: Default::default(),
        }
    }
}
#[derive(Debug, Clone)]
pub enum ConversionStop {
    NoConverter,
    MissingCandidate,
    InvalidInput(&'static str),
    InvalidOutput(&'static str),
    Limit(&'static str),
    Control(ControlError),
    Pending,
    Unavailable,
    Failed,
}
#[derive(Debug, Clone)]
pub struct DatatypeConversion {
    /// Present only after rejection or complete validation of a converted value.
    pub accepted: Option<bool>,
    /// Available converted output is inspectable even when validation rejects or stops.
    /// Publication requires accepted == Some(true).
    pub value: Option<Vec<Item>>,
    pub converter: Option<ConverterIdentity>,
    pub diagnostics: Vec<Diagnostic>,
    pub validation: Option<DatatypeValidation>,
    pub stopped: Option<ConversionStop>,
}
impl DatatypeConversion {
    fn stop(mut self, reason: ConversionStop) -> Self {
        self.accepted = None;
        self.stopped = Some(reason);
        self
    }
}
fn representation(
    values: &[Item],
    expected: ValueRepresentation,
    runtime: &ValidationRuntime<'_>,
) -> Result<bool, ControlError> {
    if matches!(expected, ValueRepresentation::Scalar(_)) && values.len() != 1 {
        return Ok(false);
    }
    for (index, value) in values.iter().enumerate() {
        if index % 64 == 0 {
            runtime.control.check_scope(runtime.scope)?;
        }
        let valid = match expected {
            ValueRepresentation::Scalar(p) | ValueRepresentation::List(p) => {
                datatype_validation::scalar(value, p)
            }
            ValueRepresentation::Nodes => datatype_validation::native_node(value),
        };
        if !valid {
            return Ok(false);
        }
    }
    Ok(true)
}
impl ExecutableDatatype {
    pub fn convert(
        &self,
        input: &ConversionInput,
        runtime: &ValidationRuntime<'_>,
        limits: ConversionLimits,
    ) -> DatatypeConversion {
        let selected = self.converter();
        let mut result = DatatypeConversion {
            accepted: None,
            value: None,
            converter: selected.map(|c| c.identity().clone()),
            diagnostics: vec![],
            validation: None,
            stopped: None,
        };
        if let Err(e) = runtime.control.check_scope(runtime.scope) {
            return result.stop(ConversionStop::Control(e));
        }
        let Some(selected) = selected else {
            return result.stop(ConversionStop::NoConverter);
        };
        let signature = selected.signature();
        if input.candidate.len() > 1
            || input
                .candidate
                .iter()
                .any(|v| !datatype_validation::native_node(v))
        {
            return result.stop(ConversionStop::InvalidInput("candidate"));
        }
        if signature.candidate == CandidateRequirement::Required && input.candidate.is_empty() {
            return result.stop(ConversionStop::MissingCandidate);
        }
        let admitted = match (&input.value, signature.input) {
            (ConversionValue::Lexical(value), ConversionRepresentation::Lexical) => {
                if value.text.len() > limits.max_lexical_bytes {
                    return result.stop(ConversionStop::Limit("lexical-bytes"));
                }
                true
            }
            (ConversionValue::Values(value), ConversionRepresentation::Values(expected)) => {
                if value.len().saturating_add(input.candidate.len()) > limits.max_input_values {
                    return result.stop(ConversionStop::Limit("input-values"));
                }
                match representation(value, expected, runtime) {
                    Ok(valid) => valid,
                    Err(e) => return result.stop(ConversionStop::Control(e)),
                }
            }
            _ => false,
        };
        if !admitted {
            return result.stop(ConversionStop::InvalidInput("value"));
        }
        if input.candidate.len() > limits.max_input_values {
            return result.stop(ConversionStop::Limit("input-values"));
        }
        let execution = selected
            .registration
            .implementation
            .convert(ConversionCall {
                value: &input.value,
                datatype: &selected.datatype,
                candidate: &input.candidate,
                runtime,
                limits,
            });
        if let Err(e) = runtime.control.check_scope(runtime.scope) {
            return result.stop(ConversionStop::Control(e));
        }
        let (value, diagnostics, stop, rejected) = match execution {
            ConversionExecution::Converted { value, diagnostics } => {
                (Some(value), diagnostics, None, false)
            }
            ConversionExecution::Rejected { diagnostics } => (None, diagnostics, None, true),
            ConversionExecution::Pending(d) => (None, d, Some(ConversionStop::Pending), false),
            ConversionExecution::Unavailable(d) => {
                (None, d, Some(ConversionStop::Unavailable), false)
            }
            ConversionExecution::Failed(d) => (None, d, Some(ConversionStop::Failed), false),
        };
        result.diagnostics = diagnostics;
        if result.diagnostics.len() > limits.validation.max_diagnostics {
            return result.stop(ConversionStop::Limit("diagnostics"));
        }
        let mut attribution = input
            .candidate
            .first()
            .map(DiagnosticAttribution::from_node)
            .unwrap_or_else(|| input.fallback.clone());
        if attribution.source_map.is_none() {
            if let ConversionValue::Lexical(lexical) = &input.value {
                attribution.source_map = Some(lexical.source.clone());
            }
        }
        for (index, diagnostic) in result.diagnostics.iter_mut().enumerate() {
            if index % 64 == 0 {
                if let Err(e) = runtime.control.check_scope(runtime.scope) {
                    return result.stop(ConversionStop::Control(e));
                }
            }
            // Preserve supplied native attribution; fill only wholly source-less diagnostics.
            if diagnostic.uri.is_none()
                && diagnostic.node.is_none()
                && diagnostic.source_map.is_none()
            {
                attribution.apply(diagnostic);
            }
        }
        if let Some(stop) = stop {
            return result.stop(stop);
        }
        if rejected {
            result.accepted = Some(false);
            return result;
        }
        let value = value.expect("converted variant retains a present value, including empty");
        if value.len() > limits.max_output_values {
            return result.stop(ConversionStop::Limit("output-values"));
        }
        match representation(&value, signature.output, runtime) {
            Ok(true) => {}
            Ok(false) => return result.stop(ConversionStop::InvalidOutput("representation")),
            Err(e) => return result.stop(ConversionStop::Control(e)),
        }
        if signature.output == ValueRepresentation::Nodes {
            let ConversionValue::Values(original) = &input.value else {
                unreachable!("checked native input signature");
            };
            let mut keys = BTreeSet::new();
            for (index, node) in original.iter().enumerate() {
                if index % 64 == 0 {
                    if let Err(e) = runtime.control.check_scope(runtime.scope) {
                        return result.stop(ConversionStop::Control(e));
                    }
                }
                keys.insert(node.view().unwrap().storage_key());
            }
            for (index, node) in value.iter().enumerate() {
                if index % 64 == 0 {
                    if let Err(e) = runtime.control.check_scope(runtime.scope) {
                        return result.stop(ConversionStop::Control(e));
                    }
                }
                if !keys.contains(&node.view().unwrap().storage_key()) {
                    return result.stop(ConversionStop::InvalidOutput("native-view-identity"));
                }
            }
        }
        let validation = self.validate(
            &ValidationInput {
                value: value.clone(),
                candidate: input.candidate.clone(),
                fallback: attribution,
            },
            runtime,
            ValidationLimits {
                max_diagnostics: limits.validation.max_diagnostics - result.diagnostics.len(),
                ..limits.validation
            },
        );
        result.accepted = validation.accepted;
        result.value = Some(value);
        result.validation = Some(validation);
        result
    }
}
