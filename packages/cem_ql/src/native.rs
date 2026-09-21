//! Opt-in native query capabilities. Registries and callbacks are runtime-only;
//! portable query/template artifacts retain only `native:call` and its inputs.
use crate::eval::{EvalError, Item, ItemStream, QueryContextScope};
use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    module_resolution::CemModuleUrlResolutionCapability,
    operation_control::{ExecutionScopeId, OperationControl},
    source_map::SourceMapStack,
};
use std::{collections::BTreeMap, fmt::Debug, sync::Arc};

/// A borrowed capability available only during an active template expression.
/// Keeping this scoped avoids serializing renderer state or locking it across
/// nested query/template calls.
pub(crate) trait TemplateQueryHost {
    fn apply_templates(&mut self, values: ItemStream, mode: &str, source: &SourceMapStack) -> ItemStream;
}

/// Implementations own their domain semantics. Long-running implementations
/// must poll the supplied operation control and bound their work/allocations;
/// the evaluator checks control before/after invocation and caps returned items.
pub trait NativeQueryFunction: Debug + Send + Sync {
    fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream;
}

pub struct NativeQueryRequest<'a> {
    /// One successful sequence per argument, including empty and multi-item
    /// arguments. The invoking evaluator retains argument diagnostics.
    pub arguments: &'a [ItemStream],
    pub current_item: Option<&'a Item>,
    pub query_scope: QueryContextScope,
    pub source_map: &'a SourceMapStack,
    pub control: &'a OperationControl,
    pub scope: ExecutionScopeId,
    pub max_result_items: u64,
    pub module_resolution: Option<&'a CemModuleUrlResolutionCapability>,
}

impl NativeQueryRequest<'_> {
    /// Raise a recoverable domain error with the invocation's provenance.
    pub fn raise(&self, code: impl Into<String>, message: impl Into<String>) -> ItemStream {
        let code = code.into();
        let message = message.into();
        ItemStream::failed(
            EvalError::Raised {
                code: code.clone().into_boxed_str(),
                message: message.clone().into_boxed_str(),
            },
            Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: None,
                code,
                severity: Severity::Error,
                message,
                node: None,
                details: None,
                source_map: Some(self.source_map.clone()),
            },
        )
    }
}

/// Exact identifiers and arities form an allowlist, not an ambient extension
/// loader. Clones preserve callback/AST ownership; later registrations stay
/// local to the registry being changed. Nothing is registered by default.
#[derive(Debug, Clone, Default)]
pub struct NativeFunctionRegistry {
    functions: BTreeMap<(String, usize), Arc<dyn NativeQueryFunction>>,
}

impl NativeFunctionRegistry {
    pub fn register(
        &mut self,
        name: impl Into<String>,
        arity: usize,
        function: impl NativeQueryFunction + 'static,
    ) -> Result<(), String> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err("native function identifier must not be empty".into());
        }
        // The stdlib catalog uses u8 arities; the identifier is the first arg.
        if arity >= u8::MAX as usize {
            return Err("native functions support at most 254 arguments".into());
        }
        let key = (name, arity);
        if self.functions.contains_key(&key) {
            return Err(format!(
                "native function `{}#{arity}` is already registered",
                key.0
            ));
        }
        self.functions.insert(key, Arc::new(function));
        Ok(())
    }

    pub(crate) fn get(&self, name: &str, arity: usize) -> Option<Arc<dyn NativeQueryFunction>> {
        self.functions.get(&(name.to_owned(), arity)).cloned()
    }
}
