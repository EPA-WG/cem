//! Explicit source/query registration; source profiles do not select converters.
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
    datatype_conversion::ConversionBehaviorContract, datatype_validation::ValidationImplementation,
    value_contracts::ValueContractError,
};
use std::collections::BTreeMap;
/// Host implementations return checked source records, with pending/unavailable
/// kept separate. The outer conversion path checks input, cancellation and views.
pub trait SourceDatatypeConverter: Debug + Send + Sync {
    fn convert(&self, call: ConversionCall<'_>) -> RuleExecution;
}
#[derive(Debug, Clone)]
enum Implementation {
    Query(Arc<CompiledQuery>),
    Native(Arc<dyn SourceDatatypeConverter>),
}
#[derive(Debug, Clone)]
struct SourceConverter {
    contract: ConversionBehaviorContract,
    adapter: DatatypeConversionResultAdapter,
    implementation: Implementation,
}
fn error(code: &'static str, contract: &ConversionBehaviorContract) -> ValueContractError {
    let mut e = ValueContractError::new(code);
    e.source = Some(contract.behavior().clone());
    e
}
impl RegisteredDatatypeConverter {
    pub fn from_query(
        source: DatatypeSource,
        contract: ConversionBehaviorContract,
        adapter: DatatypeConversionResultAdapter,
    ) -> Result<Self, ValueContractError> {
        let ValidationImplementation::Query { body, .. } = contract.implementation() else {
            return Err(error("query-implementation-required", &contract));
        };
        let context = StandaloneExpressionContext::default()
            .with_binding(
                "value",
                StandaloneExpressionBinding::new(
                    ItemStream::empty(),
                    value_type(contract.signature().value_representation()),
                ),
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
        let query = api::compile_expression(body, &context)
            .map_err(|_| error("query-compilation-failed", &contract))?;
        let id = format!("query:{}", contract.behavior().identity());
        Self::from_source_impl(
            source,
            id,
            contract,
            adapter,
            Implementation::Query(Arc::new(query.query)),
        )
    }
    pub fn from_source(
        source: DatatypeSource,
        id: &str,
        contract: ConversionBehaviorContract,
        adapter: DatatypeConversionResultAdapter,
        implementation: impl SourceDatatypeConverter + 'static,
    ) -> Result<Self, ValueContractError> {
        if !matches!(contract.implementation(),ValidationImplementation::Native(expected) if expected==id)
        {
            return Err(error("implementation-identity-mismatch", &contract));
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
        contract: ConversionBehaviorContract,
        adapter: DatatypeConversionResultAdapter,
        implementation: Implementation,
    ) -> Result<Self, ValueContractError> {
        if contract.result_contract() != adapter.result_contract() {
            return Err(error("result-registration-mismatch", &contract));
        }
        let signature = contract.signature();
        Self::new(
            source,
            id,
            signature,
            SourceConverter {
                contract: contract.clone(),
                adapter,
                implementation,
            },
        )
        .map_err(|code| error(code, &contract))
    }
}
impl NativeDatatypeConverter for SourceConverter {
    fn convert(&self, call: ConversionCall<'_>) -> ConversionExecution {
        let runtime = call.runtime;
        let output = self.contract.signature().output;
        let fallback = call.fallback.clone();
        let limits = call.limits;
        let execution = match &self.implementation {
            Implementation::Native(native) => native.convert(call),
            Implementation::Query(query) => {
                let value = match call.value {
                    ConversionValue::Lexical(v) => {
                        vec![Item::Atomic(AtomValue::String(v.text.to_string()))]
                    }
                    ConversionValue::Values(v) => v.clone(),
                };
                let mut context = call.runtime.query.clone();
                context.policy_bindings = BTreeMap::from([
                    ("value".into(), ItemStream::from_items(value)),
                    ("datatype".into(), ItemStream::once(call.datatype.clone())),
                    (
                        "candidate".into(),
                        ItemStream::from_items(call.candidate.to_vec()),
                    ),
                ]);
                context.current_item = None;
                RuleExecution::Complete(api::evaluate_with_control(
                    query,
                    &context,
                    call.runtime.control,
                    call.runtime.scope,
                ))
            }
        };
        if runtime.control.check_scope(runtime.scope).is_err() {
            return ConversionExecution::Failed(vec![]);
        }
        let stream = match execution {
            RuleExecution::Complete(stream) => stream,
            RuleExecution::Pending(d) => return ConversionExecution::Pending(d),
            RuleExecution::Unavailable(d) => return ConversionExecution::Unavailable(d),
        };
        let emitted = stream.diagnostics.clone();
        match self.adapter.consume(stream, &fallback, output, limits) {
            Ok(result) => {
                let mut diagnostics = result.execution_diagnostics;
                diagnostics.extend(result.diagnostics);
                match result.value {
                    Some(value) => ConversionExecution::Converted { value, diagnostics },
                    None => ConversionExecution::Rejected { diagnostics },
                }
            }
            Err(DatatypeResultError::Execution(stream)) => {
                ConversionExecution::Failed(stream.diagnostics)
            }
            Err(DatatypeResultError::Contract(error)) => {
                let mut diagnostic = Diagnostic {
                    code: format!("cem.datatype.conversion.{}", error.code),
                    severity: cem_ml::diagnostics::Severity::Error,
                    message: format!("Invalid conversion result: {}", error.code),
                    ..Default::default()
                };
                fallback.apply(&mut diagnostic);
                let mut diagnostics = emitted;
                diagnostics.push(diagnostic);
                ConversionExecution::Failed(diagnostics)
            }
        }
    }
}
