//! Explicit lexical export of validated typed lists. Conversion and authored
//! lexical input are separate; this boundary never serializes native AST nodes.
use crate::{
    datatype_compilation::{DatatypeValidation, ExecutableDatatype},
    datatype_results::DiagnosticAttribution,
    datatype_validation::{ValidationInput, ValidationLimits, ValidationRuntime},
    eval::{Item, RetainedCemNode},
};
use cem_ml::{
    diagnostics::Diagnostic,
    operation_control::ControlError,
    parser::tree::RetainedCemTree,
    schema::{
        datatype_registry::DatatypeSource,
        datatype_validation::{CandidateRequirement, ScalarRepresentation},
    },
};
use std::{fmt::Debug, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListSerializationSignature {
    pub item: ScalarRepresentation,
    pub candidate: CandidateRequirement,
}
#[derive(Debug, Clone)]
pub struct ListSerializerIdentity {
    pub source: DatatypeSource,
    /// Identifies the exact implementation and its lexical export format.
    pub implementation: String,
}
#[derive(Debug, Clone)]
pub enum SerializationExecution {
    Serialized {
        text: String,
        diagnostics: Vec<Diagnostic>,
    },
    Rejected {
        diagnostics: Vec<Diagnostic>,
    },
    Pending(Vec<Diagnostic>),
    Unavailable(Vec<Diagnostic>),
    Failed(Vec<Diagnostic>),
    Limit(&'static str),
}
pub struct ListSerializationCall<'a> {
    /// Original ordered values, including duplicate occurrences and native atomic views.
    pub value: &'a [Item],
    pub datatype: &'a Item,
    pub candidate: &'a [Item],
    pub runtime: &'a ValidationRuntime<'a>,
    /// Diagnostic allowance is reduced by the preceding validation's consumption.
    pub limits: SerializationLimits,
    pub fallback: &'a DiagnosticAttribution,
}
/// A trusted host capability for an explicit lexical format. Implementations
/// preserve order/duplicates, cooperate with control, and bound allocations/work.
pub trait NativeListSerializer: Debug + Send + Sync {
    fn serialize(&self, call: ListSerializationCall<'_>) -> SerializationExecution;
}
#[derive(Debug, Clone)]
pub struct RegisteredListSerializer {
    identity: ListSerializerIdentity,
    signature: ListSerializationSignature,
    implementation: Arc<dyn NativeListSerializer>,
}
impl RegisteredListSerializer {
    pub fn new(
        source: DatatypeSource,
        implementation_id: impl Into<String>,
        signature: ListSerializationSignature,
        implementation: impl NativeListSerializer + 'static,
    ) -> Result<Self, &'static str> {
        let implementation_id = implementation_id.into();
        if implementation_id.trim().is_empty() {
            return Err("empty-list-serializer-id");
        }
        Ok(Self {
            identity: ListSerializerIdentity {
                source,
                implementation: implementation_id,
            },
            signature,
            implementation: Arc::new(implementation),
        })
    }
    pub fn identity(&self) -> &ListSerializerIdentity {
        &self.identity
    }
    pub fn signature(&self) -> ListSerializationSignature {
        self.signature
    }
    pub(crate) fn bind(&self, tree: Arc<RetainedCemTree>) -> Option<BoundListSerializer> {
        let source = self.identity.source.declaration();
        if !Arc::ptr_eq(tree.ast_owner(), source.document()) {
            return None;
        }
        Some(BoundListSerializer {
            registration: self.clone(),
            datatype: RetainedCemNode::new(tree, source.node_id())?.query_item(),
        })
    }
}
#[derive(Debug, Clone)]
pub enum ListSerializerBinding {
    Unavailable,
    Ready(RegisteredListSerializer),
}
#[derive(Debug, Clone)]
pub struct BoundListSerializer {
    registration: RegisteredListSerializer,
    datatype: Item,
}
impl BoundListSerializer {
    pub fn identity(&self) -> &ListSerializerIdentity {
        self.registration.identity()
    }
    pub fn signature(&self) -> ListSerializationSignature {
        self.registration.signature()
    }
}
#[derive(Debug, Clone, Copy)]
pub struct SerializationLimits {
    pub max_output_bytes: usize,
    pub validation: ValidationLimits,
}
impl Default for SerializationLimits {
    fn default() -> Self {
        Self {
            max_output_bytes: 1_048_576,
            validation: Default::default(),
        }
    }
}
#[derive(Debug, Clone)]
pub enum SerializationStop {
    NoSerializer,
    MissingCandidate,
    Validation,
    Limit(&'static str),
    Control(ControlError),
    Pending,
    Unavailable,
    Failed,
}
#[derive(Debug, Clone)]
pub struct DatatypeSerialization {
    /// False on complete rejection by validation or serialization; true only on export.
    pub accepted: Option<bool>,
    /// Present only on success. An empty string is distinct from no published output.
    pub text: Option<String>,
    pub serializer: Option<ListSerializerIdentity>,
    /// Serializer diagnostics; validation evidence stays on `validation`.
    pub diagnostics: Vec<Diagnostic>,
    pub validation: Option<DatatypeValidation>,
    pub stopped: Option<SerializationStop>,
}
impl DatatypeSerialization {
    fn stop(mut self, reason: SerializationStop) -> Self {
        self.accepted = None;
        self.stopped = Some(reason);
        self
    }
}
impl ExecutableDatatype {
    pub fn serialize_list(
        &self,
        input: &ValidationInput,
        runtime: &ValidationRuntime<'_>,
        limits: SerializationLimits,
    ) -> DatatypeSerialization {
        let shared_runtime = runtime.with_query_budget();
        let runtime = &shared_runtime;
        let selected = self.list_serializer();
        let mut result = DatatypeSerialization {
            accepted: None,
            text: None,
            serializer: selected.map(|s| s.identity().clone()),
            diagnostics: vec![],
            validation: None,
            stopped: None,
        };
        if let Err(e) = runtime.control.check_scope(runtime.scope) {
            return result.stop(SerializationStop::Control(e));
        }
        if let Some(failure) = runtime.query_failure() {
            result.diagnostics = failure.diagnostics;
            return result.stop(SerializationStop::Failed);
        }
        let Some(selected) = selected else {
            return result.stop(SerializationStop::NoSerializer);
        };
        if selected.signature().candidate == CandidateRequirement::Required
            && input.candidate.is_empty()
        {
            return result.stop(SerializationStop::MissingCandidate);
        }
        // Validation preflights representation/candidate access and checks all
        // effective typed list and item restrictions without executing conversion.
        let validation = self.validate(input, runtime, limits.validation);
        let accepted = validation.accepted;
        let mut remaining = limits.validation.max_diagnostics;
        remaining = remaining.saturating_sub(validation.cardinality.len());
        remaining = remaining.saturating_sub(validation.enumeration_diagnostics.len());
        for rule in &validation.completed {
            remaining = remaining.saturating_sub(rule.result.diagnostics.len());
            remaining = remaining.saturating_sub(rule.result.execution_diagnostics.len());
        }
        result.validation = Some(validation);
        if let Err(e) = runtime.control.check_scope(runtime.scope) {
            return result.stop(SerializationStop::Control(e));
        }
        if let Some(failure) = runtime.query_failure() {
            result.diagnostics = failure.diagnostics;
            return result.stop(SerializationStop::Failed);
        }
        match accepted {
            Some(true) => {}
            Some(false) => {
                result.accepted = Some(false);
                return result;
            }
            None => return result.stop(SerializationStop::Validation),
        }
        let attribution = input
            .candidate
            .first()
            .map(DiagnosticAttribution::from_node)
            .unwrap_or_else(|| input.fallback.clone());
        let execution = selected
            .registration
            .implementation
            .serialize(ListSerializationCall {
                value: &input.value,
                datatype: &selected.datatype,
                candidate: &input.candidate,
                runtime,
                limits: SerializationLimits {
                    validation: ValidationLimits {
                        max_diagnostics: remaining,
                        ..limits.validation
                    },
                    ..limits
                },
                fallback: &attribution,
            });
        if let Err(e) = runtime.control.check_scope(runtime.scope) {
            return result.stop(SerializationStop::Control(e));
        }
        if let Some(failure) = runtime.query_failure() {
            result.diagnostics = failure.diagnostics;
            return result.stop(SerializationStop::Failed);
        }
        let (text, diagnostics, stop) = match execution {
            SerializationExecution::Serialized { text, diagnostics } => {
                (Some(text), diagnostics, None)
            }
            SerializationExecution::Rejected { diagnostics } => (None, diagnostics, None),
            SerializationExecution::Pending(d) => (None, d, Some(SerializationStop::Pending)),
            SerializationExecution::Unavailable(d) => {
                (None, d, Some(SerializationStop::Unavailable))
            }
            SerializationExecution::Failed(d) => (None, d, Some(SerializationStop::Failed)),
            SerializationExecution::Limit(reason) => {
                (None, vec![], Some(SerializationStop::Limit(reason)))
            }
        };
        result.diagnostics = diagnostics;
        if result.diagnostics.len() > remaining {
            return result.stop(SerializationStop::Limit("diagnostics"));
        }
        for (index, diagnostic) in result.diagnostics.iter_mut().enumerate() {
            if index % 64 == 0 {
                if let Err(e) = runtime.control.check_scope(runtime.scope) {
                    return result.stop(SerializationStop::Control(e));
                }
            }
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
        if text
            .as_ref()
            .is_some_and(|text| text.len() > limits.max_output_bytes)
        {
            return result.stop(SerializationStop::Limit("output-bytes"));
        }
        if let Err(e) = runtime.control.check_scope(runtime.scope) {
            return result.stop(SerializationStop::Control(e));
        }
        result.accepted = Some(text.is_some());
        result.text = text;
        result
    }
}
