//! Explicit lexical ingress for validation. This path never invokes converters.
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
        datatype_contracts::{LexicalInput, TokenizationError, TokenizationLimits},
        datatype_registry::{DatatypeKind, DatatypeSource},
        datatype_validation::{CandidateRequirement, ScalarRepresentation, ValueRepresentation},
    },
};
use std::{fmt::Debug, ops::Range, sync::Arc};
mod replacement;

#[derive(Debug, Clone, Copy)]
pub struct PreparationSignature {
    pub kind: DatatypeKind,
    pub output: ValueRepresentation,
    pub candidate: CandidateRequirement,
}
#[derive(Debug, Clone)]
pub struct PreparationIdentity {
    pub source: DatatypeSource,
    pub implementation: String,
}
#[derive(Debug, Clone)]
pub enum PreparationExecution {
    Prepared {
        value: Vec<Item>,
        diagnostics: Vec<Diagnostic>,
    },
    Rejected(Vec<Diagnostic>),
    Pending(Vec<Diagnostic>),
    Unavailable(Vec<Diagnostic>),
    Failed(Vec<Diagnostic>),
    Limit(&'static str),
}
pub struct PreparationCall<'a> {
    /// The complete original input and optional decoded token span remain separate
    /// from the scalar lexical slice being prepared.
    pub original: &'a LexicalInput,
    pub lexical: &'a LexicalInput,
    pub token_span: Option<Range<usize>>,
    pub datatype: &'a Item,
    pub candidate: &'a [Item],
    pub runtime: &'a ValidationRuntime<'a>,
    pub limits: PreparationLimits,
}
/// Implementations must cooperate with lifecycle control and bound their work.
pub trait NativeLexicalPreparer: Debug + Send + Sync {
    fn prepare(&self, call: PreparationCall<'_>) -> PreparationExecution;
}
#[derive(Debug, Clone)]
enum Implementation {
    Scalar(Arc<dyn NativeLexicalPreparer>),
    ListItems,
}
#[derive(Debug, Clone)]
pub struct RegisteredLexicalPreparation {
    identity: PreparationIdentity,
    signature: PreparationSignature,
    implementation: Implementation,
}
impl RegisteredLexicalPreparation {
    pub fn new(
        source: DatatypeSource,
        id: impl Into<String>,
        signature: PreparationSignature,
        implementation: impl NativeLexicalPreparer + 'static,
    ) -> Result<Self, &'static str> {
        if matches!(signature.kind, DatatypeKind::Node | DatatypeKind::List)
            || !matches!(signature.output, ValueRepresentation::Scalar(_))
        {
            return Err("scalar-preparation-signature-required");
        }
        Self::register(
            source,
            id.into(),
            signature,
            Implementation::Scalar(Arc::new(implementation)),
        )
    }
    /// Explicitly select shared tokenization and the effective scalar item preparer.
    pub fn list_items(
        source: DatatypeSource,
        id: impl Into<String>,
        item: ScalarRepresentation,
    ) -> Result<Self, &'static str> {
        Self::register(
            source,
            id.into(),
            PreparationSignature {
                kind: DatatypeKind::List,
                output: ValueRepresentation::List(item),
                candidate: CandidateRequirement::Optional,
            },
            Implementation::ListItems,
        )
    }
    fn register(
        source: DatatypeSource,
        id: String,
        signature: PreparationSignature,
        implementation: Implementation,
    ) -> Result<Self, &'static str> {
        if id.trim().is_empty() {
            return Err("empty-preparation-id");
        }
        Ok(Self {
            identity: PreparationIdentity {
                source,
                implementation: id,
            },
            signature,
            implementation,
        })
    }
    pub fn identity(&self) -> &PreparationIdentity {
        &self.identity
    }
    pub fn signature(&self) -> PreparationSignature {
        self.signature
    }
    pub(crate) fn bind(&self, tree: Arc<RetainedCemTree>) -> Option<BoundLexicalPreparation> {
        let source = self.identity.source.declaration();
        if !Arc::ptr_eq(tree.ast_owner(), source.document()) {
            return None;
        }
        Some(BoundLexicalPreparation {
            selected: Arc::new(PreparationStep {
                registration: self.clone(),
                datatype: RetainedCemNode::new(tree, source.node_id())?.query_item(),
            }),
            inherited: vec![],
        })
    }
}
#[derive(Debug, Clone)]
pub enum PreparationBinding {
    Unavailable,
    Ready(RegisteredLexicalPreparation),
    /// Explicitly retain and check the base's original lexical admission.
    CheckedReplacement(RegisteredLexicalPreparation),
}
#[derive(Debug, Clone)]
struct PreparationStep {
    registration: RegisteredLexicalPreparation,
    datatype: Item,
}
#[derive(Debug, Clone)]
pub struct BoundLexicalPreparation {
    selected: Arc<PreparationStep>,
    inherited: Vec<Arc<PreparationStep>>,
}
impl BoundLexicalPreparation {
    pub fn identity(&self) -> &PreparationIdentity {
        self.selected.registration.identity()
    }
    pub fn signature(&self) -> PreparationSignature {
        self.selected.registration.signature()
    }
    pub fn invocations(&self) -> usize {
        self.inherited.len().saturating_add(1)
    }
    pub(crate) fn checked_replacement(mut self, base: &Self) -> Self {
        self.inherited = base.inherited.clone();
        self.inherited.push(base.selected.clone());
        self
    }
}
#[derive(Debug, Clone)]
pub struct PreparationInput {
    pub lexical: LexicalInput,
    pub candidate: Vec<Item>,
    pub fallback: DiagnosticAttribution,
}
#[derive(Debug, Clone, Copy)]
pub struct PreparationLimits {
    pub max_lexical_bytes: usize,
    /// Shared allowance for selected preparers and inherited admission checks.
    /// List orchestration includes all selected/inherited tokenizer invocations.
    pub max_preparations: usize,
    pub max_output_values: usize,
    /// Input visits and diagnostics are shared with subsequent validation.
    pub validation: ValidationLimits,
}
impl Default for PreparationLimits {
    fn default() -> Self {
        Self {
            max_lexical_bytes: 1_048_576,
            max_preparations: 100_000,
            max_output_values: 100_000,
            validation: Default::default(),
        }
    }
}
#[derive(Debug, Clone)]
pub enum PreparationStop {
    NoPreparation,
    MissingCandidate,
    InvalidInput(&'static str),
    InvalidOutput,
    IncompatibleReplacement,
    Limit(&'static str),
    Control(ControlError),
    Pending,
    Unavailable,
    Failed,
}
#[derive(Debug, Clone)]
pub struct DatatypePreparation {
    pub input: PreparationInput,
    pub preparer: Option<PreparationIdentity>,
    /// Completed preparation is inspectable; publication requires accepted == Some(true).
    pub value: Option<Vec<Item>>,
    /// Ordered decoded UTF-8 spans corresponding to list values, never source offsets.
    pub token_spans: Vec<Range<usize>>,
    pub accepted: Option<bool>,
    pub diagnostics: Vec<Diagnostic>,
    pub validation: Option<DatatypeValidation>,
    pub preparations: usize,
    pub stopped: Option<PreparationStop>,
}
impl ExecutableDatatype {
    pub fn prepare_lexical(
        &self,
        input: &PreparationInput,
        runtime: &ValidationRuntime<'_>,
        limits: PreparationLimits,
    ) -> DatatypePreparation {
        self.prepare_phase(input, runtime, limits, true).0
    }
    pub(crate) fn prepare_without_validation(
        &self,
        input: &PreparationInput,
        runtime: &ValidationRuntime<'_>,
        limits: PreparationLimits,
    ) -> (DatatypePreparation, usize) {
        self.prepare_phase(input, runtime, limits, false)
    }
    fn prepare_phase(
        &self,
        input: &PreparationInput,
        runtime: &ValidationRuntime<'_>,
        limits: PreparationLimits,
        validate: bool,
    ) -> (DatatypePreparation, usize) {
        let shared_runtime = runtime.with_query_budget();
        let runtime = &shared_runtime;
        let mut report = DatatypePreparation {
            input: input.clone(),
            preparer: self.preparation().map(|p| p.identity().clone()),
            value: None,
            token_spans: vec![],
            accepted: None,
            diagnostics: vec![],
            validation: None,
            preparations: 0,
            stopped: None,
        };
        let mut visits = 0;
        if let Err(stop) = prepare(self, input, runtime, limits, &mut report, &mut visits, validate) {
            report.stopped = Some(stop);
            report.accepted = None;
        }
        (report, visits)
    }
}
fn check(runtime: &ValidationRuntime<'_>) -> Result<(), PreparationStop> {
    runtime
        .control
        .check_scope(runtime.scope)
        .map_err(PreparationStop::Control)
}
fn candidate(
    selected: &BoundLexicalPreparation,
    input: &PreparationInput,
) -> Result<(), PreparationStop> {
    if input.candidate.len() > 1
        || input
            .candidate
            .iter()
            .any(|n| !datatype_validation::native_node(n))
    {
        return Err(PreparationStop::InvalidInput("candidate"));
    }
    if selected
        .inherited
        .iter()
        .chain(std::iter::once(&selected.selected))
        .any(|step| step.registration.signature().candidate == CandidateRequirement::Required)
        && input.candidate.is_empty()
    {
        return Err(PreparationStop::MissingCandidate);
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
fn prepare(
    d: &ExecutableDatatype,
    input: &PreparationInput,
    runtime: &ValidationRuntime<'_>,
    limits: PreparationLimits,
    report: &mut DatatypePreparation,
    visits: &mut usize,
    validate: bool,
) -> Result<(), PreparationStop> {
    check(runtime)?;
    if input.lexical.text.len() > limits.max_lexical_bytes {
        return Err(PreparationStop::Limit("lexical-bytes"));
    }
    let selected = d.preparation().ok_or(PreparationStop::NoPreparation)?;
    candidate(selected, input)?;
    let mut attribution = input
        .candidate
        .first()
        .map(DiagnosticAttribution::from_node)
        .unwrap_or_else(|| input.fallback.clone());
    if attribution.source_map.is_none() {
        attribution.source_map = Some(input.lexical.source.clone());
    }
    let values = match &selected.selected.registration.implementation {
        Implementation::Scalar(_) => scalar(
            selected,
            input,
            &input.lexical,
            None,
            runtime,
            limits,
            &attribution,
            report,
            visits,
        )?,
        Implementation::ListItems => {
            let item = d
                .item()
                .and_then(|item| item.preparation())
                .ok_or(PreparationStop::NoPreparation)?;
            candidate(item, input)?;
            let tokenizer = d.tokenizer().ok_or(PreparationStop::Unavailable)?;
            let cost = selected
                .invocations()
                .saturating_add(tokenizer.invocations() - 1);
            if report.preparations.saturating_add(cost) > limits.max_preparations {
                return Err(PreparationStop::Limit("preparations"));
            }
            report.preparations += cost;
            let tokens = match tokenizer.tokenize(
                Some(input.lexical.clone()),
                runtime.control,
                runtime.scope,
                TokenizationLimits {
                    max_bytes: limits.max_lexical_bytes,
                    max_tokens: limits.max_output_values,
                    max_tokenizers: tokenizer.invocations(),
                },
            ) {
                Ok(tokens) => tokens,
                Err(TokenizationError::Rejected) => {
                    report.accepted = Some(false);
                    return Ok(());
                }
                Err(e) => {
                    return Err(match e {
                        TokenizationError::Control(e) => PreparationStop::Control(e),
                        TokenizationError::Limit => PreparationStop::Limit("tokenization"),
                        TokenizationError::IncompatibleReplacement => {
                            PreparationStop::IncompatibleReplacement
                        }
                        _ => PreparationStop::Failed,
                    })
                }
            };
            report.token_spans = tokens.tokens;
            // Preflight cumulative invocation count before any item callback.
            if report
                .preparations
                .saturating_add(report.token_spans.len().saturating_mul(item.invocations()))
                > limits.max_preparations
            {
                return Err(PreparationStop::Limit("preparations"));
            }
            if visits.saturating_add(
                report
                    .token_spans
                    .len()
                    .saturating_mul(item.invocations())
                    .saturating_mul(1 + input.candidate.len()),
            ) > limits.validation.max_input_values
            {
                return Err(PreparationStop::Limit("input-values"));
            }
            let mut values = Vec::with_capacity(report.token_spans.len());
            for index in 0..report.token_spans.len() {
                let span = report.token_spans[index].clone();
                let lexical = LexicalInput::new(
                    Arc::from(&input.lexical.text[span.clone()]),
                    input.lexical.source.clone(),
                );
                let Some(mut value) = scalar(
                    item,
                    input,
                    &lexical,
                    Some(span),
                    runtime,
                    limits,
                    &attribution,
                    report,
                    visits,
                )?
                else {
                    return Ok(());
                };
                values.append(&mut value);
            }
            Some(values)
        }
    };
    let Some(values) = values else {
        return Ok(());
    };
    check(runtime)?;
    if !validate {
        report.value = Some(values);
        return Ok(());
    }
    let validation = d.validate(
        &ValidationInput {
            value: values.clone(),
            candidate: input.candidate.clone(),
            fallback: attribution,
        },
        runtime,
        ValidationLimits {
            max_diagnostics: limits.validation.max_diagnostics - report.diagnostics.len(),
            max_input_values: limits.validation.max_input_values - *visits,
            ..limits.validation
        },
    );
    report.accepted = validation.accepted;
    report.value = Some(values);
    report.validation = Some(validation);
    Ok(())
}
fn spend(
    report: &mut DatatypePreparation,
    limits: PreparationLimits,
) -> Result<(), PreparationStop> {
    if report.preparations >= limits.max_preparations {
        return Err(PreparationStop::Limit("preparations"));
    }
    report.preparations += 1;
    Ok(())
}
#[allow(clippy::too_many_arguments)]
fn scalar(
    selected: &BoundLexicalPreparation,
    input: &PreparationInput,
    lexical: &LexicalInput,
    token_span: Option<Range<usize>>,
    runtime: &ValidationRuntime<'_>,
    limits: PreparationLimits,
    attribution: &DiagnosticAttribution,
    report: &mut DatatypePreparation,
    visits: &mut usize,
) -> Result<Option<Vec<Item>>, PreparationStop> {
    candidate(selected, input)?;
    if report.preparations.saturating_add(selected.invocations()) > limits.max_preparations {
        return Err(PreparationStop::Limit("preparations"));
    }
    if visits.saturating_add(
        selected
            .invocations()
            .saturating_mul(1 + input.candidate.len()),
    ) > limits.validation.max_input_values
    {
        return Err(PreparationStop::Limit("input-values"));
    }
    let mut expected = None;
    let mut result = None;
    for step in selected
        .inherited
        .iter()
        .chain(std::iter::once(&selected.selected))
    {
        let Some(value) = scalar_step(
            step,
            input,
            lexical,
            token_span.clone(),
            runtime,
            limits,
            attribution,
            report,
            visits,
        )?
        else {
            return Ok(None);
        };
        if !selected.inherited.is_empty() {
            let fingerprint = replacement::fingerprint(
                &value[0],
                input,
                token_span.as_ref(),
                limits.max_lexical_bytes,
            )?;
            if expected.as_ref().is_some_and(|base| base != &fingerprint) {
                return Err(PreparationStop::IncompatibleReplacement);
            }
            expected = Some(fingerprint);
        }
        result = Some(value);
    }
    Ok(result)
}
#[allow(clippy::too_many_arguments)]
fn scalar_step(
    selected: &PreparationStep,
    input: &PreparationInput,
    lexical: &LexicalInput,
    token_span: Option<Range<usize>>,
    runtime: &ValidationRuntime<'_>,
    limits: PreparationLimits,
    attribution: &DiagnosticAttribution,
    report: &mut DatatypePreparation,
    visits: &mut usize,
) -> Result<Option<Vec<Item>>, PreparationStop> {
    check(runtime)?;
    spend(report, limits)?;
    *visits = visits.saturating_add(1 + input.candidate.len());
    if *visits > limits.validation.max_input_values {
        return Err(PreparationStop::Limit("input-values"));
    }
    if limits.max_output_values == 0 {
        return Err(PreparationStop::Limit("output-values"));
    }
    let Implementation::Scalar(implementation) = &selected.registration.implementation else {
        return Err(PreparationStop::InvalidOutput);
    };
    let execution = if let Some(failure) = runtime.query_failure() {
        PreparationExecution::Failed(failure.diagnostics)
    } else {
        implementation.prepare(PreparationCall {
            original: &input.lexical,
            lexical,
            token_span,
            datatype: &selected.datatype,
            candidate: &input.candidate,
            runtime,
            limits,
        })
    };
    check(runtime)?;
    let execution = runtime.query_failure().map_or(execution, |failure| {
        PreparationExecution::Failed(failure.diagnostics)
    });
    let (value, diagnostics, stop) = match execution {
        PreparationExecution::Prepared { value, diagnostics } => (Some(value), diagnostics, None),
        PreparationExecution::Rejected(d) => (None, d, None),
        PreparationExecution::Pending(d) => (None, d, Some(PreparationStop::Pending)),
        PreparationExecution::Unavailable(d) => (None, d, Some(PreparationStop::Unavailable)),
        PreparationExecution::Failed(d) => (None, d, Some(PreparationStop::Failed)),
        PreparationExecution::Limit(reason) => (None, vec![], Some(PreparationStop::Limit(reason))),
    };
    let remaining = limits.validation.max_diagnostics - report.diagnostics.len();
    let overflow = diagnostics.len() > remaining;
    for mut diagnostic in diagnostics.into_iter().take(remaining) {
        check(runtime)?;
        if diagnostic.uri.is_none() && diagnostic.node.is_none() && diagnostic.source_map.is_none()
        {
            attribution.apply(&mut diagnostic);
        }
        report.diagnostics.push(diagnostic);
    }
    if overflow {
        return Err(PreparationStop::Limit("diagnostics"));
    }
    if let Some(stop) = stop {
        return Err(stop);
    }
    let Some(value) = value else {
        report.accepted = Some(false);
        return Ok(None);
    };
    let ValueRepresentation::Scalar(expected) = selected.registration.signature().output else {
        return Err(PreparationStop::InvalidOutput);
    };
    if value.len() != 1 || !datatype_validation::scalar(&value[0], expected) {
        return Err(PreparationStop::InvalidOutput);
    }
    Ok(Some(value))
}
