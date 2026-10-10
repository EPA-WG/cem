//! Process-local, immutable evidence issued inside checked lexical preparation.
//! Inspection reports and caller-authored value/text pairs cannot construct it.
use crate::{
    api::EvaluationContext,
    datatype_facets::BoundAttributeFacets,
    datatype_preparation::{
        DatatypePreparation, PreparationInput, PreparationLimits, PreparationStop,
    },
    datatype_validation::{ValidationInput, ValidationRuntime},
    eval::{self, AtomValue, Item},
};
use cem_ml::{
    diagnostics::Diagnostic,
    operation_control::{ControlError, ExecutionScopeId, OperationControl},
    parser::CemAstNode,
    schema::{
        datatype_contracts::LexicalInput, datatype_validation::ValueRepresentation,
        declaration_references::SchemaDeclarationNode,
    },
};
use std::{
    collections::BTreeSet,
    fmt,
    ops::Range,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, Weak,
    },
};

/// Bound additional source/diagnostic metadata retained by an evidence handle.
pub const MAX_EVIDENCE_METADATA_BYTES: usize = 1_048_576;

pub(crate) fn check_attribute_source_metadata(
    source: &cem_ml::source_map::SourceMapStack,
    control: &OperationControl,
) -> Result<(), EvidenceStop> {
    let mut budget = MetadataBudget::new(control, cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID);
    budget.source(source)?;
    // The lexical input and its diagnostic fallback retain separate stacks.
    budget.source(source)
}

#[derive(Debug, Clone)]
pub enum EvidenceStop {
    MissingEvidence,
    BindingMismatch,
    InvalidRepresentation,
    SourceMismatch,
    UntrackedView,
    Closed,
    ForeignInvocation,
    Busy,
    QueryFailed,
    Limit(&'static str),
    Control(ControlError),
    Preparation(PreparationStop),
}
#[derive(Debug, Clone)]
pub enum PreparationOrigin {
    SourceLess,
    Attribute(SchemaDeclarationNode),
    Default(SchemaDeclarationNode),
}
struct Budget {
    remaining: PreparationLimits,
    busy: bool,
}
struct InvocationState {
    binding: BoundAttributeFacets,
    input: PreparationInput,
    origin: PreparationOrigin,
    control: OperationControl,
    scope: ExecutionScopeId,
    query: EvaluationContext,
    closed: AtomicBool,
    budget: Mutex<Budget>,
}
impl fmt::Debug for InvocationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparationInvocation")
            .field("declaration", &self.binding.binding().declaration)
            .field("operation", &self.control.operation_id())
            .field("scope", &self.scope)
            .field("closed", &self.closed.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}
impl InvocationState {
    fn check(&self) -> Result<(), EvidenceStop> {
        if self.closed.load(Ordering::Acquire) {
            return Err(EvidenceStop::Closed);
        }
        self.control
            .check_scope(self.scope)
            .map_err(EvidenceStop::Control)?;
        if self
            .query
            .execution_budget
            .as_ref()
            .and_then(|b| b.failure())
            .is_some()
        {
            return Err(EvidenceStop::QueryFailed);
        }
        Ok(())
    }
}
/// Clones retain one frozen input/binding/context and share all remaining work.
/// The embedding closes it before input/context/grant/publication changes.
#[derive(Debug, Clone)]
pub struct AttributePreparationInvocation {
    state: Arc<InvocationState>,
}
impl AttributePreparationInvocation {
    pub(crate) fn consume(
        &self,
        binding: &BoundAttributeFacets,
        evidence: Option<&SealedPreparationEvidence>,
        context: cem_ml::schema::document_model::attribute_facets::FacetContext<'_>,
        max_facet_model_bytes: usize,
    ) -> crate::attribute_validation::AttributeValidation {
        use crate::attribute_validation::{
            diagnostic_cost, AttributeDatatypePhase, AttributeValidation,
            AttributeValidationLimits, AttributeValidationStop,
        };
        let stop =
            |reason| AttributeValidation::empty().stop(AttributeValidationStop::Evidence(reason));
        let Some(evidence) = evidence else {
            return stop(EvidenceStop::MissingEvidence);
        };
        if !evidence.matches_binding(binding) {
            return stop(EvidenceStop::BindingMismatch);
        }
        if let Err(reason) = evidence.verify(self) {
            return stop(reason);
        }
        let mut lease = match BudgetLease::acquire(self.state.clone()) {
            Ok(lease) => lease,
            Err(reason) => return stop(reason),
        };
        // Even an empty receipt spends inspection work. Values and ordered spans
        // are inspected in addition to the descriptor's scheduled validation visits.
        let inspection = 1usize
            .saturating_add(evidence.values().len())
            .saturating_add(evidence.token_spans().len())
            .saturating_add(evidence.candidate().len());
        let Some(remaining_visits) = lease
            .available
            .validation
            .max_input_values
            .checked_sub(inspection)
        else {
            let mut refund = lease.available;
            refund.validation.max_input_values = 0;
            lease.refund = Some(refund);
            return stop(EvidenceStop::Limit("evidence-input-values"));
        };
        let input = ValidationInput {
            value: evidence.values().to_vec(),
            candidate: evidence.candidate().to_vec(),
            fallback: self.state.input.fallback.clone(),
        };
        let datatype = &binding.binding().datatype;
        let mut rule_cost = datatype.rules().len();
        let mut visit_cost = input.value.len().saturating_add(input.candidate.len());
        if let Some(item) = datatype.item() {
            rule_cost =
                rule_cost.saturating_add(item.rules().len().saturating_mul(input.value.len()));
            visit_cost = visit_cost.saturating_add(
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
        let mut limits = lease.available;
        limits.validation.max_input_values = remaining_visits;
        let mut report = binding.validate_prepared(
            evidence,
            &input,
            context,
            &runtime,
            AttributeValidationLimits {
                preparation: limits,
                max_facet_model_bytes,
            },
        );
        let mut refund = lease.available;
        // Reserve the complete scheduled rule/input work even if a callback stops
        // early. Comparisons and rendered/cardinality diagnostics use actual costs.
        refund.validation.max_rules = refund.validation.max_rules.saturating_sub(rule_cost);
        refund.validation.max_input_values = remaining_visits.saturating_sub(visit_cost);
        if let Some(AttributeDatatypePhase::Pretyped { validation, .. }) = &report.datatype {
            refund.validation.max_comparisons = refund
                .validation
                .max_comparisons
                .saturating_sub(validation.comparisons);
            let cost = diagnostic_cost(validation)
                .saturating_add(stopped_diagnostic_cost(validation))
                .saturating_add(report.facets.as_ref().map_or(0, |f| f.diagnostics.len()));
            refund.validation.max_diagnostics =
                refund.validation.max_diagnostics.saturating_sub(cost);
        }
        if matches!(
            report.stopped,
            Some(AttributeValidationStop::Facets(
                cem_ml::schema::document_model::attribute_facets::FacetExecutionError::Limit
            ))
        ) {
            // A stopped facet report does not expose its partial diagnostics.
            refund.validation.max_diagnostics = 0;
        }
        lease.refund = Some(refund);
        if let Err(reason) = self.state.check() {
            report = report.stop(AttributeValidationStop::Evidence(reason));
        }
        report
    }
    pub fn new(
        binding: &BoundAttributeFacets,
        input: PreparationInput,
        runtime: &ValidationRuntime<'_>,
        limits: PreparationLimits,
    ) -> Result<Self, EvidenceStop> {
        runtime
            .control
            .check_scope(runtime.scope)
            .map_err(EvidenceStop::Control)?;
        if binding.profile().family().representation() == ValueRepresentation::Nodes {
            return Err(EvidenceStop::InvalidRepresentation);
        }
        if input.lexical.text.len() > limits.max_lexical_bytes {
            return Err(EvidenceStop::Limit("lexical-bytes"));
        }
        let mut metadata = MetadataBudget::new(runtime.control, runtime.scope);
        metadata.source(&input.lexical.source)?;
        metadata.attribution(&input.fallback)?;
        let origin = original_source(binding, &input)?;
        let shared = runtime.with_query_budget();
        let state = Arc::new(InvocationState {
            binding: binding.clone(),
            input,
            origin,
            control: runtime.control.clone(),
            scope: runtime.scope,
            query: shared.query,
            closed: AtomicBool::new(false),
            budget: Mutex::new(Budget {
                remaining: limits,
                busy: false,
            }),
        });
        state.check()?;
        Ok(Self { state })
    }
    pub fn close(&self) {
        self.state.closed.store(true, Ordering::Release);
    }
    pub fn remaining_limits(&self) -> PreparationLimits {
        self.state
            .budget
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remaining
    }
    pub fn prepare(&self) -> PreparationEvidenceResult {
        let mut result = PreparationEvidenceResult {
            evidence: None,
            report: empty_report(&self.state),
            stopped: None,
        };
        let mut lease = match BudgetLease::acquire(self.state.clone()) {
            Ok(lease) => lease,
            Err(stop) => {
                result.stopped = Some(stop);
                return result;
            }
        };
        let runtime = ValidationRuntime {
            control: &self.state.control,
            scope: self.state.scope,
            query: self.state.query.clone(),
        };
        let (report, visits) = self
            .state
            .binding
            .binding()
            .datatype
            .prepare_without_validation(&self.state.input, &runtime, lease.available);
        let output_count = report.value.as_ref().map_or_else(
            || {
                if report.token_spans.is_empty() {
                    usize::from(report.preparations > 0)
                } else {
                    report.token_spans.len()
                }
            },
            Vec::len,
        );
        lease.refund = Some(PreparationLimits {
            max_preparations: lease
                .available
                .max_preparations
                .saturating_sub(report.preparations),
            max_output_values: lease
                .available
                .max_output_values
                .saturating_sub(output_count),
            validation: crate::datatype_validation::ValidationLimits {
                max_input_values: lease
                    .available
                    .validation
                    .max_input_values
                    .saturating_sub(visits),
                max_diagnostics: lease
                    .available
                    .validation
                    .max_diagnostics
                    .saturating_sub(report.diagnostics.len()),
                ..lease.available.validation
            },
            ..lease.available
        });
        result.report = report;
        let outcome = (|| {
            self.state.check()?;
            if let Some(stop) = &result.report.stopped {
                return Err(EvidenceStop::Preparation(stop.clone()));
            }
            let Some(values) = &result.report.value else {
                return Ok(None);
            };
            let mut metadata = MetadataBudget::new(&self.state.control, self.state.scope);
            metadata.source(&self.state.input.lexical.source)?;
            metadata.attribution(&self.state.input.fallback)?;
            let mut bytes = 0usize;
            let mut token_buffers = BTreeSet::new();
            for (index, value) in values.iter().enumerate() {
                if index % 64 == 0 {
                    self.state.check()?;
                }
                // A representation-id or atom() result cannot prove immutability.
                let size = match value {
                    Item::Atomic(
                        AtomValue::String(s) | AtomValue::Decimal(s) | AtomValue::AnyUri(s),
                    ) => s.len(),
                    Item::Atomic(_) => 0,
                    _ => {
                        if let Some((s, source)) = crate::typed_scalar::immutable_storage(value) {
                            if let Some(source) = source {
                                metadata.source(source)?;
                            }
                            s.len()
                        } else if let Some((lexical, _)) =
                            crate::datatype_shipped::token_source(value)
                        {
                            if token_buffers.insert(lexical as *const LexicalInput as usize) {
                                metadata.source(&lexical.source)?;
                                lexical.text.len()
                            } else {
                                0
                            }
                        } else {
                            return Err(EvidenceStop::UntrackedView);
                        }
                    }
                };
                bytes = bytes.saturating_add(size);
                if bytes > lease.available.max_lexical_bytes {
                    return Err(EvidenceStop::Limit("value-bytes"));
                }
            }
            self.state.check()?;
            for diagnostic in &result.report.diagnostics {
                metadata.diagnostic(diagnostic)?;
            }
            let evidence = SealedPreparationEvidence {
                inner: Arc::new(EvidenceInner {
                    invocation: Arc::downgrade(&self.state),
                    binding: self.state.binding.clone(),
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
}

fn stopped_diagnostic_cost(validation: &crate::datatype_compilation::DatatypeValidation) -> usize {
    use crate::{datatype_results::DatatypeResultError, datatype_validation::ValidationStopReason};
    match validation.stopped.as_ref().map(|stop| &stop.reason) {
        Some(
            ValidationStopReason::Pending(diagnostics)
            | ValidationStopReason::Unavailable(diagnostics)
            | ValidationStopReason::Failed(diagnostics),
        ) => diagnostics.len(),
        Some(ValidationStopReason::Result(DatatypeResultError::Execution(stream))) => {
            stream.diagnostics.len()
        }
        _ => 0,
    }
}

// This inspects existing diagnostic control metadata directly. It never
// serializes a value or constructs/reloads a CEM AST through JSON.
pub(crate) struct MetadataBudget<'a> {
    remaining: usize,
    control: &'a OperationControl,
    scope: ExecutionScopeId,
}
impl<'a> MetadataBudget<'a> {
    pub(crate) fn new(control: &'a OperationControl, scope: ExecutionScopeId) -> Self {
        Self {
            remaining: MAX_EVIDENCE_METADATA_BYTES,
            control,
            scope,
        }
    }
    fn charge(&mut self, bytes: usize) -> Result<(), EvidenceStop> {
        self.control
            .check_scope(self.scope)
            .map_err(EvidenceStop::Control)?;
        self.remaining = self
            .remaining
            .checked_sub(bytes)
            .ok_or(EvidenceStop::Limit("metadata-bytes"))?;
        Ok(())
    }
    pub(crate) fn source(&mut self, source: &cem_ml::source_map::SourceMapStack) -> Result<(), EvidenceStop> {
        use cem_ml::source_map::{FrameSpan, SourceMapFrame, TransformKind};
        self.charge(
            source
                .frames
                .len()
                .saturating_mul(std::mem::size_of::<SourceMapFrame>()),
        )?;
        for frame in &source.frames {
            if let FrameSpan::Multi(ranges) = &frame.span {
                self.charge(
                    ranges
                        .len()
                        .saturating_mul(std::mem::size_of::<cem_ml::source::ByteRange>()),
                )?;
            }
            let bytes = match &frame.transform {
                TransformKind::HandoffBoundary { child_content_type } => child_content_type.len(),
                TransformKind::ContentTypeTransform { content_type } => content_type.len(),
                TransformKind::TemplateTransform { function } => function.len(),
                TransformKind::ScssOrigin {
                    module_uri, name, ..
                } => module_uri
                    .len()
                    .saturating_add(name.as_ref().map_or(0, String::len)),
                _ => 0,
            };
            self.charge(bytes)?;
        }
        Ok(())
    }
    fn attribution(
        &mut self,
        value: &crate::datatype_results::DiagnosticAttribution,
    ) -> Result<(), EvidenceStop> {
        self.charge(
            value
                .uri
                .as_ref()
                .map_or(0, String::len)
                .saturating_add(value.node.as_ref().map_or(0, String::len)),
        )?;
        if let Some(source) = &value.source_map {
            self.source(source)?;
        }
        Ok(())
    }
    fn diagnostic(&mut self, value: &Diagnostic) -> Result<(), EvidenceStop> {
        self.charge(std::mem::size_of::<Diagnostic>())?;
        for text in [
            Some(&value.code),
            Some(&value.message),
            value.uri.as_ref(),
            value.node.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            self.charge(text.len())?;
        }
        if let Some(source) = &value.source_map {
            self.source(source)?;
        }
        if let Some(details) = &value.details {
            self.details(details, 0)?;
        }
        Ok(())
    }
    fn details(&mut self, value: &serde_json::Value, depth: usize) -> Result<(), EvidenceStop> {
        if depth > 64 {
            return Err(EvidenceStop::Limit("metadata-depth"));
        }
        self.charge(std::mem::size_of::<serde_json::Value>())?;
        match value {
            serde_json::Value::String(s) => self.charge(s.len())?,
            serde_json::Value::Array(values) => {
                if values.len() > self.remaining / std::mem::size_of::<serde_json::Value>() {
                    return Err(EvidenceStop::Limit("metadata-bytes"));
                }
                for value in values {
                    self.details(value, depth + 1)?;
                }
            }
            serde_json::Value::Object(fields) => {
                if fields.len() > self.remaining / std::mem::size_of::<serde_json::Value>() {
                    return Err(EvidenceStop::Limit("metadata-bytes"));
                }
                for (key, value) in fields {
                    self.charge(key.len())?;
                    self.details(value, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

// A lease reserves all remaining capacity during callbacks. Reentrant/concurrent
// calls cannot obtain another allowance. Unwinding conservatively spends it all.
struct BudgetLease {
    state: Arc<InvocationState>,
    available: PreparationLimits,
    refund: Option<PreparationLimits>,
}
impl BudgetLease {
    fn acquire(state: Arc<InvocationState>) -> Result<Self, EvidenceStop> {
        state.check()?;
        let available = {
            let mut budget = state.budget.lock().unwrap_or_else(|e| e.into_inner());
            if budget.busy {
                return Err(EvidenceStop::Busy);
            }
            budget.busy = true;
            let available = budget.remaining;
            budget.remaining.max_preparations = 0;
            budget.remaining.max_output_values = 0;
            budget.remaining.validation.max_input_values = 0;
            budget.remaining.validation.max_rules = 0;
            budget.remaining.validation.max_comparisons = 0;
            budget.remaining.validation.max_diagnostics = 0;
            available
        };
        Ok(Self {
            state,
            available,
            refund: None,
        })
    }
}
impl Drop for BudgetLease {
    fn drop(&mut self) {
        let mut budget = self.state.budget.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(refund) = self.refund {
            budget.remaining = refund;
        }
        budget.busy = false;
    }
}

fn original_source(
    binding: &BoundAttributeFacets,
    input: &PreparationInput,
) -> Result<PreparationOrigin, EvidenceStop> {
    let candidate = match input.candidate.as_slice() {
        [] => return Ok(PreparationOrigin::SourceLess),
        [value] => value,
        _ => return Err(EvidenceStop::SourceMismatch),
    };
    let original = eval::retained_cem_node(candidate)
        .or_else(|| crate::attribute_activation::original_attribute_candidate(candidate))
        .ok_or(EvidenceStop::UntrackedView)?;
    let node = SchemaDeclarationNode::new(original.owner().ast_owner().clone(), original.node_id())
        .ok_or(EvidenceStop::SourceMismatch)?;
    let CemAstNode::Attribute {
        value,
        value_nodes,
        source,
        ..
    } = node.node()
    else {
        return Err(EvidenceStop::SourceMismatch);
    };
    if !value_nodes.is_empty()
        || value.as_deref().unwrap_or("") != input.lexical.text.as_ref()
        || source != &input.lexical.source
    {
        return Err(EvidenceStop::SourceMismatch);
    }
    let is_default = binding.binding().constraint_fields().iter().any(|field| {
        field.identity() == node.identity()
            && matches!(field.node(), CemAstNode::Attribute {expanded_name,..} if expanded_name.namespace_uri.is_empty() && expanded_name.local_name == "default")
    });
    Ok(if is_default {
        PreparationOrigin::Default(node)
    } else {
        PreparationOrigin::Attribute(node)
    })
}
fn empty_report(state: &InvocationState) -> DatatypePreparation {
    DatatypePreparation {
        input: state.input.clone(),
        preparer: state
            .binding
            .binding()
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
    }
}

#[derive(Debug)]
struct EvidenceInner {
    invocation: Weak<InvocationState>,
    binding: BoundAttributeFacets,
    input: PreparationInput,
    origin: PreparationOrigin,
    values: Vec<Item>,
    spans: Vec<Range<usize>>,
    diagnostics: Vec<Diagnostic>,
}
/// Only checked preparation constructs this immutable handle. It has no codec,
/// public constructor, mutable accessor or acceptance verdict.
#[derive(Debug, Clone)]
pub struct SealedPreparationEvidence {
    inner: Arc<EvidenceInner>,
}
impl SealedPreparationEvidence {
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
    pub fn origin(&self) -> &PreparationOrigin {
        &self.inner.origin
    }
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.inner.diagnostics
    }
    pub fn binding(&self) -> &BoundAttributeFacets {
        &self.inner.binding
    }
    pub fn matches_binding(&self, binding: &BoundAttributeFacets) -> bool {
        self.inner.binding.same_binding(binding)
    }
    pub fn verify(&self, invocation: &AttributePreparationInvocation) -> Result<(), EvidenceStop> {
        let original = self
            .inner
            .invocation
            .upgrade()
            .ok_or(EvidenceStop::Closed)?;
        if !Arc::ptr_eq(&original, &invocation.state) {
            return Err(EvidenceStop::ForeignInvocation);
        }
        original.check()
    }
    pub fn is_live(&self) -> bool {
        self.inner
            .invocation
            .upgrade()
            .is_some_and(|s| s.check().is_ok())
    }
}
#[derive(Debug)]
pub struct PreparationEvidenceResult {
    pub evidence: Option<SealedPreparationEvidence>,
    /// Freely inspectable, never a constructor or authorization for evidence.
    pub report: DatatypePreparation,
    pub stopped: Option<EvidenceStop>,
}
