//! Native parser contracts are compiled from the same retained schema source
//! as the model. They execute only at the document import boundary.

use super::document_model::SchemaDocumentModel;
use crate::{
    diagnostics::Diagnostic,
    engine::{EngineInput, FormatIdentity},
    lifecycle::LoadedInput,
    parser::{document::CemDocument, tree::RetainedCemTree},
};
use std::{collections::BTreeSet, sync::Arc};

pub(crate) trait NativeParserContract: std::fmt::Debug + Send + Sync {
    fn load(
        &self,
        input: &EngineInput,
        identity: &FormatIdentity,
        adapter_id: &'static str,
    ) -> LoadedInput;
}

/// In-process executable contract paired with its original native schema owner.
/// Inspection can compile declarations, but cannot manufacture a retained owner.
#[derive(Clone)]
pub struct CompiledParserContract {
    source_owner: Option<Arc<RetainedCemTree>>,
    implementation: Option<Arc<dyn NativeParserContract>>,
}

impl std::fmt::Debug for CompiledParserContract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledParserContract")
            .field("source_owner", &self.source_owner)
            .field("ready", &self.is_ready())
            .finish()
    }
}

impl CompiledParserContract {
    pub fn source_owner(&self) -> Option<&Arc<RetainedCemTree>> {
        self.source_owner.as_ref()
    }
    pub fn is_ready(&self) -> bool {
        self.source_owner.is_some() && self.implementation.is_some()
    }
    pub fn matches_owner(&self, owner: &Arc<CemDocument>) -> bool {
        self.source_owner
            .as_ref()
            .is_some_and(|source| Arc::ptr_eq(source.ast_owner(), owner))
    }
    pub(crate) fn load(
        &self,
        input: &EngineInput,
        identity: &FormatIdentity,
        adapter_id: &'static str,
    ) -> Option<LoadedInput> {
        self.is_ready().then(|| {
            self.implementation
                .as_ref()
                .unwrap()
                .load(input, identity, adapter_id)
        })
    }
}

#[derive(Default)]
pub(crate) struct ParserContractCompilation {
    pub claimed_constraints: BTreeSet<String>,
    pub contract: Option<Arc<CompiledParserContract>>,
    pub diagnostics: Vec<Diagnostic>,
}

pub(crate) fn requires_parser_contract(schema_uri: &str) -> bool {
    schema_uri == crate::schema::registry::CSV_SCHEMA_URI
}

/// Closed native provider catalog. Recognizing a name alone never grants an
/// arbitrary format an executor; each provider checks its own complete contract.
pub(crate) fn compile_registered_parser_contract(
    model: &SchemaDocumentModel,
    owner: Option<Arc<RetainedCemTree>>,
) -> ParserContractCompilation {
    if !requires_parser_contract(&model.schema_uri) {
        return ParserContractCompilation::default();
    }
    let claimed_constraints = model
        .constraints
        .values()
        .filter(|constraint| {
            constraint.behavior.as_deref()
                == Some(crate::validation::csv::CSV_PARSE_REPORT_BEHAVIOR)
        })
        .map(|constraint| constraint.kind.clone())
        .collect();
    let (implementation, diagnostics) = crate::validation::csv::compile_parser_contract(model);
    ParserContractCompilation {
        claimed_constraints,
        contract: Some(Arc::new(CompiledParserContract {
            source_owner: owner,
            implementation,
        })),
        diagnostics,
    }
}

#[cfg(test)]
mod tests;
