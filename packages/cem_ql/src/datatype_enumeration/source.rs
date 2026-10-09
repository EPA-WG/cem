//! Source descriptors are passive. Only explicit host registration enables execution.
use super::*;
use crate::{
    api::{self, StandaloneExpressionBinding, StandaloneExpressionContext},
    datatype_results::DatatypeResultError,
    datatype_validation::{value_type, RuleExecution},
    eval::{AtomValue, ItemStream},
    ir::CompiledQuery,
    types::{NodeKind, Type},
};
use cem_ml::schema::{
    datatype_enumeration::{ConstantBehaviorContract, EqualityBehaviorContract},
    datatype_validation::{ValidationImplementation, ValueRepresentation},
    value_contracts::ValueContractError,
};
use std::collections::BTreeMap;

pub trait SourceScalarEquality: Debug + Send + Sync {
    fn compare(&self, call: EqualityCall<'_>) -> RuleExecution;
}
pub trait SourceConstantInterpreter: Debug + Send + Sync {
    fn interpret(&self, call: ConstantCall<'_>) -> RuleExecution;
}
#[derive(Debug, Clone)]
enum Implementation<T: ?Sized> {
    Query(Arc<CompiledQuery>),
    Native(Arc<T>),
}
#[derive(Debug)]
struct Equality {
    contract: EqualityBehaviorContract,
    adapter: DatatypeEqualityResultAdapter,
    implementation: Implementation<dyn SourceScalarEquality>,
}
#[derive(Debug)]
struct Constant {
    contract: ConstantBehaviorContract,
    adapter: DatatypeConstantResultAdapter,
    implementation: Implementation<dyn SourceConstantInterpreter>,
}
fn error(code: &'static str, behavior: &SchemaDeclarationNode) -> ValueContractError {
    let mut error = ValueContractError::new(code);
    error.source = Some(behavior.clone());
    error
}
fn compile(
    body: &str,
    roles: &[(&str, Type)],
    behavior: &SchemaDeclarationNode,
) -> Result<Arc<CompiledQuery>, ValueContractError> {
    let mut context = StandaloneExpressionContext::default();
    for (name, ty) in roles {
        context = context.with_binding(
            *name,
            StandaloneExpressionBinding::new(ItemStream::empty(), ty.clone()),
        );
    }
    api::compile_expression(body, &context)
        .map(|compiled| Arc::new(compiled.query))
        .map_err(|_| error("query-compilation-failed", behavior))
}
fn evaluate(
    query: &CompiledQuery,
    bindings: BTreeMap<String, ItemStream>,
    runtime: &ValidationRuntime<'_>,
) -> RuleExecution {
    let mut context = runtime.query.clone();
    context.policy_bindings = bindings;
    context.current_item = None;
    RuleExecution::Complete(api::evaluate_with_control(
        query,
        &context,
        runtime.control,
        runtime.scope,
    ))
}
fn failed(
    error: DatatypeResultError,
    mut emitted: Vec<Diagnostic>,
    fallback: &DiagnosticAttribution,
) -> Vec<Diagnostic> {
    match error {
        DatatypeResultError::Execution(stream) => stream.diagnostics,
        DatatypeResultError::Contract(error) => {
            let mut diagnostic = Diagnostic {
                code: format!("cem.datatype.enumeration.{}", error.code),
                severity: cem_ml::diagnostics::Severity::Error,
                message: format!("Invalid enumeration capability result: {}", error.code),
                ..Default::default()
            };
            fallback.apply(&mut diagnostic);
            emitted.push(diagnostic);
            emitted
        }
    }
}
impl RegisteredScalarEquality {
    pub fn from_query(
        source: DatatypeSource,
        contract: EqualityBehaviorContract,
        adapter: DatatypeEqualityResultAdapter,
    ) -> Result<Self, ValueContractError> {
        let ValidationImplementation::Query { body, .. } = contract.implementation() else {
            return Err(error("query-implementation-required", contract.behavior()));
        };
        let ty = value_type(ValueRepresentation::Scalar(contract.representation()));
        let query = compile(
            body,
            &[
                ("left", ty.clone()),
                ("right", ty),
                ("datatype", Type::Node(NodeKind::Node)),
            ],
            contract.behavior(),
        )?;
        Self::from_source_impl(
            source,
            format!("query:{}", contract.behavior().identity()),
            contract,
            adapter,
            Implementation::Query(query),
        )
    }
    pub fn from_source(
        source: DatatypeSource,
        id: &str,
        contract: EqualityBehaviorContract,
        adapter: DatatypeEqualityResultAdapter,
        implementation: impl SourceScalarEquality + 'static,
    ) -> Result<Self, ValueContractError> {
        if !matches!(contract.implementation(), ValidationImplementation::Native(expected) if expected == id)
        {
            return Err(error(
                "implementation-identity-mismatch",
                contract.behavior(),
            ));
        }
        Self::from_source_impl(
            source,
            id.into(),
            contract,
            adapter,
            Implementation::Native(Arc::new(implementation)),
        )
    }
    fn from_source_impl(
        source: DatatypeSource,
        id: String,
        contract: EqualityBehaviorContract,
        adapter: DatatypeEqualityResultAdapter,
        implementation: Implementation<dyn SourceScalarEquality>,
    ) -> Result<Self, ValueContractError> {
        if contract.result_contract() != adapter.result_contract() {
            return Err(error("result-registration-mismatch", contract.behavior()));
        }
        // Keep the authored descriptor alive with the implementation, including its owner.
        Self::new(
            source,
            id,
            contract.representation(),
            Equality {
                contract,
                adapter,
                implementation,
            },
        )
        .map_err(ValueContractError::new)
    }
}
impl NativeScalarEquality for Equality {
    fn compare(&self, call: EqualityCall<'_>) -> EqualityExecution {
        if !crate::datatype_validation::scalar(call.left, self.contract.representation())
            || !crate::datatype_validation::scalar(call.right, self.contract.representation())
        {
            return EqualityExecution::Failed(failed(
                ValueContractError::new("equality-input-representation").into(),
                vec![],
                call.fallback,
            ));
        }
        let runtime = call.runtime;
        let fallback = call.fallback.clone();
        let limits = call.limits;
        if runtime.control.check_scope(runtime.scope).is_err() {
            return EqualityExecution::Failed(vec![]);
        }
        let execution = match &self.implementation {
            Implementation::Native(native) => native.compare(call),
            Implementation::Query(query) => evaluate(
                query,
                BTreeMap::from([
                    ("left".into(), ItemStream::once(call.left.clone())),
                    ("right".into(), ItemStream::once(call.right.clone())),
                    ("datatype".into(), ItemStream::once(call.datatype.clone())),
                ]),
                runtime,
            ),
        };
        if runtime.control.check_scope(runtime.scope).is_err() {
            return EqualityExecution::Failed(vec![]);
        }
        let stream = match execution {
            RuleExecution::Complete(stream) => stream,
            RuleExecution::Pending(d) => return EqualityExecution::Pending(d),
            RuleExecution::Unavailable(d) => return EqualityExecution::Unavailable(d),
        };
        let emitted = stream.diagnostics.clone();
        match self.adapter.consume(stream, &fallback, limits) {
            Ok(result) => {
                let mut diagnostics = result.execution_diagnostics;
                diagnostics.extend(result.diagnostics);
                EqualityExecution::Complete {
                    equal: result.equal,
                    diagnostics,
                }
            }
            Err(error) => EqualityExecution::Failed(failed(error, emitted, &fallback)),
        }
    }
}
impl RegisteredConstantInterpreter {
    pub fn from_query(
        source: DatatypeSource,
        contract: ConstantBehaviorContract,
        adapter: DatatypeConstantResultAdapter,
    ) -> Result<Self, ValueContractError> {
        let ValidationImplementation::Query { body, .. } = contract.implementation() else {
            return Err(error("query-implementation-required", contract.behavior()));
        };
        let query = compile(
            body,
            &[
                (
                    "value",
                    value_type(ValueRepresentation::Scalar(ScalarRepresentation::String)),
                ),
                ("datatype", Type::Node(NodeKind::Node)),
                ("candidate", Type::Node(NodeKind::Node)),
            ],
            contract.behavior(),
        )?;
        Self::from_source_impl(
            source,
            format!("query:{}", contract.behavior().identity()),
            contract,
            adapter,
            Implementation::Query(query),
        )
    }
    pub fn from_source(
        source: DatatypeSource,
        id: &str,
        contract: ConstantBehaviorContract,
        adapter: DatatypeConstantResultAdapter,
        implementation: impl SourceConstantInterpreter + 'static,
    ) -> Result<Self, ValueContractError> {
        if !matches!(contract.implementation(), ValidationImplementation::Native(expected) if expected == id)
        {
            return Err(error(
                "implementation-identity-mismatch",
                contract.behavior(),
            ));
        }
        Self::from_source_impl(
            source,
            id.into(),
            contract,
            adapter,
            Implementation::Native(Arc::new(implementation)),
        )
    }
    fn from_source_impl(
        source: DatatypeSource,
        id: String,
        contract: ConstantBehaviorContract,
        adapter: DatatypeConstantResultAdapter,
        implementation: Implementation<dyn SourceConstantInterpreter>,
    ) -> Result<Self, ValueContractError> {
        if contract.result_contract() != adapter.result_contract() {
            return Err(error("result-registration-mismatch", contract.behavior()));
        }
        Self::new(
            source,
            id,
            contract.representation(),
            Constant {
                contract,
                adapter,
                implementation,
            },
        )
        .map_err(ValueContractError::new)
    }
}
impl NativeConstantInterpreter for Constant {
    fn interpret(&self, call: ConstantCall<'_>) -> ConstantExecution {
        let runtime = call.runtime;
        let fallback = call.fallback.clone();
        let limits = call.limits;
        if runtime.control.check_scope(runtime.scope).is_err() {
            return ConstantExecution::Failed(vec![]);
        }
        let execution = match &self.implementation {
            Implementation::Native(native) => native.interpret(call),
            Implementation::Query(query) => evaluate(
                query,
                BTreeMap::from([
                    (
                        "value".into(),
                        ItemStream::once(Item::Atomic(AtomValue::String(call.token.text().into()))),
                    ),
                    ("datatype".into(), ItemStream::once(call.datatype.clone())),
                    ("candidate".into(), ItemStream::once(call.candidate.clone())),
                ]),
                runtime,
            ),
        };
        if runtime.control.check_scope(runtime.scope).is_err() {
            return ConstantExecution::Failed(vec![]);
        }
        let stream = match execution {
            RuleExecution::Complete(stream) => stream,
            RuleExecution::Pending(d) => return ConstantExecution::Pending(d),
            RuleExecution::Unavailable(d) => return ConstantExecution::Unavailable(d),
        };
        let emitted = stream.diagnostics.clone();
        match self
            .adapter
            .consume(stream, &fallback, self.contract.representation(), limits)
        {
            Ok(result) => {
                let mut diagnostics = result.execution_diagnostics;
                diagnostics.extend(result.diagnostics);
                match result.value {
                    Some(value) => ConstantExecution::Prepared {
                        value: vec![value],
                        diagnostics,
                    },
                    None => ConstantExecution::Rejected(diagnostics),
                }
            }
            Err(error) => ConstantExecution::Failed(failed(error, emitted, &fallback)),
        }
    }
}
