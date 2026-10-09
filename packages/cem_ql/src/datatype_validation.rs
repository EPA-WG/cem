//! Host-registered validation over checked retained behavior declarations.
//! This explicit boundary does not compile effective datatypes or activate @type.
use crate::{
    api::{self, EvaluationContext, StandaloneExpressionBinding, StandaloneExpressionContext},
    datatype_results::{
        DatatypeResultAdapter, DatatypeResultError, DatatypeValidationResult, DiagnosticAttribution,
    },
    eval::{self, AtomValue, Item, ItemStream, QueryItemViewKind},
    ir::CompiledQuery,
    types::{AtomType, NodeKind, Type},
};
use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    operation_control::{ControlError, ExecutionScopeId, OperationControl},
    parser::CemAstNode,
    schema::{
        datatype_registry::DatatypeKind,
        datatype_validation::{
            CandidateRequirement, DatatypeBehaviorContract, ResultRepresentation,
            ScalarRepresentation, ValidationImplementation, ValueRepresentation,
        },
        declaration_references::SchemaDeclarationNode,
        value_contracts::ValueContractError,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    sync::Arc,
};

/// No mapping is installed implicitly. Codes permit validity to differ from
/// severity; NoDiagnostics is available only when explicitly selected by a host.
#[derive(Debug, Clone)]
pub enum LegacyAcceptance {
    NoDiagnostics,
    RejectCodes(BTreeSet<String>),
    RejectSeverities(Vec<Severity>),
}
impl LegacyAcceptance {
    fn accepts(&self, diagnostics: &[Diagnostic]) -> bool {
        match self {
            Self::NoDiagnostics => diagnostics.is_empty(),
            Self::RejectCodes(codes) => !diagnostics.iter().any(|d| codes.contains(&d.code)),
            Self::RejectSeverities(severities) => {
                !diagnostics.iter().any(|d| severities.contains(&d.severity))
            }
        }
    }
}
#[derive(Debug, Clone)]
pub enum RuleExecution {
    Complete(ItemStream),
    Pending(Vec<Diagnostic>),
    Unavailable(Vec<Diagnostic>),
}
/// Callbacks must cooperate with the supplied control and bound their own work.
/// Query callbacks automatically use that same operation and execution scope.
pub trait NativeDatatypeValidator: Debug + Send + Sync {
    fn validate(&self, call: ValidationCall<'_>) -> RuleExecution;
}
pub struct ValidationCall<'a> {
    pub value: &'a [Item],
    pub datatype: &'a Item,
    pub candidate: &'a [Item],
    pub runtime: &'a ValidationRuntime<'a>,
}
pub struct ValidationRuntime<'a> {
    pub control: &'a OperationControl,
    pub scope: ExecutionScopeId,
    pub query: EvaluationContext,
}
#[derive(Debug, Clone)]
pub struct ValidationInput {
    pub value: Vec<Item>,
    pub candidate: Vec<Item>,
    pub fallback: DiagnosticAttribution,
}
#[derive(Debug, Clone, Copy)]
pub struct ValidationLimits {
    pub max_rules: usize,
    pub max_comparisons: usize,
    pub max_input_values: usize,
    pub max_diagnostics: usize,
}
impl Default for ValidationLimits {
    fn default() -> Self {
        Self {
            max_rules: 256,
            max_comparisons: 100_000,
            max_input_values: 100_000,
            max_diagnostics: 100_000,
        }
    }
}
#[derive(Debug, Clone)]
enum Implementation {
    Native(Arc<dyn NativeDatatypeValidator>),
    Query(Arc<CompiledQuery>),
}
#[derive(Debug, Clone)]
struct Registration {
    contract: DatatypeBehaviorContract,
    adapter: DatatypeResultAdapter,
    legacy: Option<LegacyAcceptance>,
    implementation: Implementation,
}
/// Nothing is registered by default. Registry clones retain original owners and
/// implementations; updates cannot rebind an already prepared rule by name.
#[derive(Debug, Clone, Default)]
pub struct DatatypeValidationRegistry {
    entries: BTreeMap<(String, String), Arc<Registration>>,
}
impl DatatypeValidationRegistry {
    pub fn signature(
        &self,
        owner: &SchemaDeclarationNode,
        behavior: &SchemaDeclarationNode,
    ) -> Option<&cem_ml::schema::datatype_validation::ValidationSignature> {
        self.entries
            .get(&(owner.identity(), behavior.identity()))
            .map(|entry| entry.contract.signature())
    }
    pub fn register_native(
        &mut self,
        implementation_id: &str,
        contract: DatatypeBehaviorContract,
        adapter: DatatypeResultAdapter,
        legacy: Option<LegacyAcceptance>,
        implementation: impl NativeDatatypeValidator + 'static,
    ) -> Result<(), ValueContractError> {
        if !matches!(contract.implementation(),ValidationImplementation::Native(id) if id==implementation_id)
        {
            return Err(error(
                "implementation-identity-mismatch",
                contract.behavior(),
            ));
        }
        self.insert(
            contract,
            adapter,
            legacy,
            Implementation::Native(Arc::new(implementation)),
        )
    }
    pub fn register_query(
        &mut self,
        contract: DatatypeBehaviorContract,
        adapter: DatatypeResultAdapter,
        legacy: Option<LegacyAcceptance>,
    ) -> Result<(), ValueContractError> {
        let ValidationImplementation::Query { body, .. } = contract.implementation() else {
            return Err(error("query-implementation-required", contract.behavior()));
        };
        let signature = contract.signature();
        let context = StandaloneExpressionContext::default()
            .with_binding(
                "value",
                StandaloneExpressionBinding::new(ItemStream::empty(), value_type(signature.value)),
            )
            .with_binding(
                "datatype",
                StandaloneExpressionBinding::new(ItemStream::empty(), Type::Node(NodeKind::Node)),
            )
            .with_binding(
                "candidate",
                StandaloneExpressionBinding::new(
                    ItemStream::empty(),
                    Type::stream(Type::Node(NodeKind::Node)),
                ),
            );
        let compiled = api::compile_expression(body, &context)
            .map_err(|_| error("query-compilation-failed", contract.behavior()))?;
        self.insert(
            contract,
            adapter,
            legacy,
            Implementation::Query(Arc::new(compiled.query)),
        )
    }
    fn insert(
        &mut self,
        contract: DatatypeBehaviorContract,
        adapter: DatatypeResultAdapter,
        legacy: Option<LegacyAcceptance>,
        implementation: Implementation,
    ) -> Result<(), ValueContractError> {
        let compatible = match &contract.signature().result {
            ResultRepresentation::Accepted(name) => {
                name == adapter.result_contract() && legacy.is_none()
            }
            ResultRepresentation::Diagnostics(name) => {
                name == adapter.diagnostic_contract() && legacy.is_some()
            }
        };
        if !compatible {
            return Err(error("result-registration-mismatch", contract.behavior()));
        }
        let key = (contract.owner().identity(), contract.behavior().identity());
        if self.entries.contains_key(&key) {
            return Err(error(
                "duplicate-validation-registration",
                contract.behavior(),
            ));
        }
        self.entries.insert(
            key,
            Arc::new(Registration {
                contract,
                adapter,
                legacy,
                implementation,
            }),
        );
        Ok(())
    }
    /// Called after authorized dependency selection and effective-kind compilation.
    /// The datatype view must carry its original AST owner, never an ID/record stub.
    pub fn bind(
        &self,
        owner: &SchemaDeclarationNode,
        behavior: &SchemaDeclarationNode,
        datatype: Item,
        kind: DatatypeKind,
    ) -> Result<BoundDatatypeRule, ValueContractError> {
        let registration = self
            .entries
            .get(&(owner.identity(), behavior.identity()))
            .ok_or_else(|| error("validation-capability-unavailable", behavior))?;
        if registration.contract.signature().kind != kind {
            return Err(error("datatype-kind-mismatch", behavior));
        }
        let source = eval::retained_cem_node(&datatype)
            .ok_or_else(|| error("original-datatype-required", behavior))?;
        let CemAstNode::Element {
            expanded_name,
            attributes,
            ..
        } = source.node()
        else {
            return Err(error("datatype-declaration-required", behavior));
        };
        let named = attributes.iter().any(|id| {
            matches!(source.owner().ast().get(*id),
                Some(CemAstNode::Attribute { expanded_name, value: Some(value), value_nodes, .. })
                if expanded_name.local_name == "name" && !value.trim().is_empty() && value_nodes.is_empty())
        });
        if expanded_name.local_name != "type" || !named {
            return Err(error("named-datatype-required", behavior));
        }
        let declaration =
            SchemaDeclarationNode::new(source.owner().ast_owner().clone(), source.node_id())
                .unwrap();
        Ok(BoundDatatypeRule {
            registration: registration.clone(),
            datatype,
            declaration,
        })
    }
}
#[derive(Debug, Clone)]
pub struct BoundDatatypeRule {
    registration: Arc<Registration>,
    datatype: Item,
    declaration: SchemaDeclarationNode,
}
impl BoundDatatypeRule {
    pub fn signature(&self) -> &cem_ml::schema::datatype_validation::ValidationSignature {
        self.registration.contract.signature()
    }
    pub fn datatype(&self) -> &Item {
        &self.datatype
    }
    pub fn behavior(&self) -> &SchemaDeclarationNode {
        self.registration.contract.behavior()
    }
}
#[derive(Debug, Clone)]
pub struct RuleValidation {
    pub datatype: SchemaDeclarationNode,
    pub behavior: SchemaDeclarationNode,
    pub result: DatatypeValidationResult,
}
#[derive(Debug, Clone)]
pub enum ValidationStopReason {
    InvalidInput(&'static str),
    MissingCandidate,
    Limit(&'static str),
    Control(ControlError),
    Pending(Vec<Diagnostic>),
    Unavailable(Vec<Diagnostic>),
    Result(DatatypeResultError),
    Failed(Vec<Diagnostic>),
}
#[derive(Debug, Clone)]
pub struct ValidationStop {
    pub behavior: Option<SchemaDeclarationNode>,
    pub reason: ValidationStopReason,
}
#[derive(Debug, Clone)]
pub struct ValidationBatch {
    /// Set only when all effective rules completed. Prior rejection remains in
    /// completed entries when a later rule is pending, unavailable or failed.
    pub accepted: Option<bool>,
    pub completed: Vec<RuleValidation>,
    pub stopped: Option<ValidationStop>,
}
fn stopped(
    completed: Vec<RuleValidation>,
    rule: Option<&BoundDatatypeRule>,
    reason: ValidationStopReason,
) -> ValidationBatch {
    ValidationBatch {
        accepted: None,
        completed,
        stopped: Some(ValidationStop {
            behavior: rule.map(|r| r.behavior().clone()),
            reason,
        }),
    }
}
/// Validates the supplied effective restriction list without short-circuiting a
/// rejection. The host supplies only complete inputs; unresolved selection must
/// stay in its lifecycle envelope and must not be represented as an empty value.
pub fn validate_rules(
    rules: &[BoundDatatypeRule],
    input: &ValidationInput,
    runtime: &ValidationRuntime<'_>,
    limits: ValidationLimits,
) -> ValidationBatch {
    if let Err(e) = runtime.control.check_scope(runtime.scope) {
        return stopped(vec![], None, ValidationStopReason::Control(e));
    }
    if rules.len() > limits.max_rules {
        return stopped(vec![], None, ValidationStopReason::Limit("rules"));
    }
    if input.value.len().saturating_add(input.candidate.len()) > limits.max_input_values {
        return stopped(vec![], None, ValidationStopReason::Limit("input-values"));
    }
    // Preflight every input before executing any callback.
    for rule in rules {
        if let Err(e) = runtime.control.check_scope(runtime.scope) {
            return stopped(vec![], Some(rule), ValidationStopReason::Control(e));
        }
        let signature = rule.registration.contract.signature();
        if input.candidate.len() > 1 || input.candidate.iter().any(|v| !native_node(v)) {
            return stopped(
                vec![],
                Some(rule),
                ValidationStopReason::InvalidInput("candidate"),
            );
        }
        if signature.candidate == CandidateRequirement::Required && input.candidate.is_empty() {
            return stopped(vec![], Some(rule), ValidationStopReason::MissingCandidate);
        }
        if matches!(signature.value, ValueRepresentation::Scalar(_)) && input.value.len() != 1 {
            return stopped(
                vec![],
                Some(rule),
                ValidationStopReason::InvalidInput("value"),
            );
        }
        for (index, value) in input.value.iter().enumerate() {
            if index % 64 == 0 {
                if let Err(e) = runtime.control.check_scope(runtime.scope) {
                    return stopped(vec![], Some(rule), ValidationStopReason::Control(e));
                }
            }
            let valid = match signature.value {
                ValueRepresentation::Scalar(p) | ValueRepresentation::List(p) => scalar(value, p),
                ValueRepresentation::Nodes => native_node(value),
            };
            if !valid {
                return stopped(
                    vec![],
                    Some(rule),
                    ValidationStopReason::InvalidInput("value"),
                );
            }
        }
    }
    let fallback = input
        .candidate
        .first()
        .map(DiagnosticAttribution::from_node)
        .unwrap_or_else(|| input.fallback.clone());
    let mut completed = Vec::new();
    let mut accepted = true;
    let mut diagnostics_count = 0usize;
    for rule in rules {
        if let Err(e) = runtime.control.check_scope(runtime.scope) {
            return stopped(completed, Some(rule), ValidationStopReason::Control(e));
        }
        let call = ValidationCall {
            value: &input.value,
            datatype: &rule.datatype,
            candidate: &input.candidate,
            runtime,
        };
        let execution = match &rule.registration.implementation {
            Implementation::Native(implementation) => implementation.validate(call),
            Implementation::Query(query) => {
                let mut context = runtime.query.clone();
                // Function roles are a closed binding environment; runtime capabilities
                // and navigation restrictions stay on the supplied native views.
                context.policy_bindings = BTreeMap::from([
                    ("value".into(), ItemStream::from_items(input.value.clone())),
                    ("datatype".into(), ItemStream::once(rule.datatype.clone())),
                    (
                        "candidate".into(),
                        ItemStream::from_items(input.candidate.clone()),
                    ),
                ]);
                context.current_item = None;
                RuleExecution::Complete(api::evaluate_with_control(
                    query,
                    &context,
                    runtime.control,
                    runtime.scope,
                ))
            }
        };
        if let Err(e) = runtime.control.check_scope(runtime.scope) {
            return stopped(completed, Some(rule), ValidationStopReason::Control(e));
        }
        let stream = match execution {
            RuleExecution::Complete(stream) => stream,
            RuleExecution::Pending(diagnostics) => {
                return stopped(
                    completed,
                    Some(rule),
                    ValidationStopReason::Pending(diagnostics),
                )
            }
            RuleExecution::Unavailable(diagnostics) => {
                return stopped(
                    completed,
                    Some(rule),
                    ValidationStopReason::Unavailable(diagnostics),
                )
            }
        };
        let result = consume(&rule.registration, stream, &fallback);
        let result = match result {
            Ok(result) => result,
            Err(e) => return stopped(completed, Some(rule), ValidationStopReason::Result(e)),
        };
        accepted &= result.accepted;
        diagnostics_count = diagnostics_count
            .saturating_add(result.diagnostics.len())
            .saturating_add(result.execution_diagnostics.len());
        completed.push(RuleValidation {
            datatype: rule.declaration.clone(),
            behavior: rule.behavior().clone(),
            result,
        });
        if diagnostics_count > limits.max_diagnostics {
            return stopped(
                completed,
                Some(rule),
                ValidationStopReason::Limit("diagnostics"),
            );
        }
    }
    ValidationBatch {
        accepted: Some(accepted),
        completed,
        stopped: None,
    }
}
fn consume(
    registration: &Registration,
    stream: ItemStream,
    fallback: &DiagnosticAttribution,
) -> Result<DatatypeValidationResult, DatatypeResultError> {
    let Some(mapping) = &registration.legacy else {
        return registration.adapter.consume(stream, fallback);
    };
    if stream.error.is_some() {
        return Err(DatatypeResultError::Execution(stream));
    }
    // The wrapper is new validation information, not an AST projection. Consume
    // once before applying mapping so malformed diagnostics never establish validity.
    let mut wrapped = ItemStream::once(Item::Record(BTreeMap::from([
        (
            "accepted".into(),
            vec![Item::Atomic(AtomValue::Boolean(true))],
        ),
        ("diagnostics".into(), stream.items),
    ])));
    wrapped.diagnostics = stream.diagnostics;
    let mut result = registration.adapter.consume(wrapped, fallback)?;
    result.accepted = mapping.accepts(&result.diagnostics);
    if let Item::Record(fields) = &mut result.original {
        fields.insert(
            "accepted".into(),
            vec![Item::Atomic(AtomValue::Boolean(result.accepted))],
        );
    }
    Ok(result)
}
pub(crate) fn native_node(value: &Item) -> bool {
    value
        .view()
        .is_some_and(|v| v.kind() == QueryItemViewKind::Node)
}
pub(crate) fn scalar(value: &Item, p: ScalarRepresentation) -> bool {
    if let Some(retained) = crate::typed_scalar::representation(value) {
        return retained == p;
    }
    if !matches!(value, Item::Atomic(_))
        && !value
            .view()
            .is_some_and(|v| v.kind() == QueryItemViewKind::Atomic)
    {
        return false;
    }
    matches!(
        (value.atom(), p),
        (Some(AtomValue::String(_)), ScalarRepresentation::String)
            | (Some(AtomValue::Boolean(_)), ScalarRepresentation::Boolean)
            | (Some(AtomValue::Integer(_)), ScalarRepresentation::Integer)
            | (Some(AtomValue::Decimal(_)), ScalarRepresentation::Decimal)
            | (Some(AtomValue::Double(_)), ScalarRepresentation::Double)
            | (Some(AtomValue::AnyUri(_)), ScalarRepresentation::AnyUri)
    )
}
pub(crate) fn value_type(value: ValueRepresentation) -> Type {
    let atomic = |p| {
        Type::Atom(match p {
            ScalarRepresentation::String => AtomType::String,
            ScalarRepresentation::Boolean => AtomType::Boolean,
            ScalarRepresentation::Integer => AtomType::Integer,
            ScalarRepresentation::Decimal => AtomType::Decimal,
            ScalarRepresentation::Double => AtomType::Double,
            ScalarRepresentation::AnyUri => AtomType::AnyUri,
        })
    };
    match value {
        ValueRepresentation::Scalar(p) => atomic(p),
        ValueRepresentation::List(p) => Type::stream(atomic(p)),
        ValueRepresentation::Nodes => Type::stream(Type::Node(NodeKind::Node)),
    }
}
fn error(code: &'static str, source: &SchemaDeclarationNode) -> ValueContractError {
    let mut e = ValueContractError::new(code);
    e.source = Some(source.clone());
    e
}
