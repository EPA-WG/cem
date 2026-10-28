//! Native parser contracts are compiled from the same retained schema source
//! as the model. They execute only at the document import boundary.

use super::document_model::SchemaDocumentModel;
use crate::{
    diagnostics::Diagnostic,
    engine::{EngineContext, EngineInput, FormatIdentity},
    lifecycle::LoadedInput,
    parser::{document::CemDocument, tree::RetainedCemTree},
};
use std::{collections::BTreeSet, sync::Arc};

pub(crate) trait NativeParserContract: std::fmt::Debug + Send + Sync {
    fn parse_xslt_with_modules(
        &self,
        _request: crate::validation::xslt::XsltSourceValidationRequest<'_>,
        _hrefs: &[&str],
    ) -> Option<(
        Option<crate::validation::xslt::XsltStylesheetAst>,
        Vec<Diagnostic>,
    )> {
        None
    }

    fn load(
        &self,
        input: &EngineInput,
        identity: &FormatIdentity,
        adapter_id: &'static str,
    ) -> LoadedInput;

    fn load_with_context(
        &self,
        input: &EngineInput,
        identity: &FormatIdentity,
        adapter_id: &'static str,
        _context: &EngineContext,
        _source_validation: bool,
    ) -> LoadedInput {
        self.load(input, identity, adapter_id)
    }
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
    pub(crate) fn parse_xslt_with_modules(
        &self,
        request: crate::validation::xslt::XsltSourceValidationRequest<'_>,
        hrefs: &[&str],
    ) -> Option<(
        Option<crate::validation::xslt::XsltStylesheetAst>,
        Vec<Diagnostic>,
    )> {
        self.is_ready()
            .then(|| {
                self.implementation
                    .as_ref()
                    .unwrap()
                    .parse_xslt_with_modules(request, hrefs)
            })
            .flatten()
    }

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
        context: &EngineContext,
        source_validation: bool,
    ) -> Option<LoadedInput> {
        self.is_ready().then(|| {
            self.implementation.as_ref().unwrap().load_with_context(
                input,
                identity,
                adapter_id,
                context,
                source_validation,
            )
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
    matches!(
        schema_uri,
        crate::schema::registry::CSV_SCHEMA_URI
            | crate::schema::registry::MARKDOWN_SCHEMA_URI
            | crate::schema::registry::XSLT_SCHEMA_URI
            | crate::schema::registry::RELAX_NG_SCHEMA_URI
            | crate::schema::registry::SCSS_SCHEMA_URI
    )
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
    let fact_behavior = match model.schema_uri.as_str() {
        crate::schema::registry::CSV_SCHEMA_URI => {
            Some(crate::validation::csv::CSV_PARSE_REPORT_BEHAVIOR)
        }
        crate::schema::registry::XSLT_SCHEMA_URI => {
            Some(crate::validation::xslt::XSLT_FACT_BEHAVIOR)
        }
        crate::schema::registry::RELAX_NG_SCHEMA_URI => {
            Some(crate::validation::relax_ng::RELAX_NG_FACT_BEHAVIOR)
        }
        crate::schema::registry::SCSS_SCHEMA_URI => {
            Some(crate::validation::scss::SCSS_FACT_BEHAVIOR)
        }
        _ => None,
    };
    let claimed_constraints = model
        .constraints
        .values()
        .filter(|constraint| {
            fact_behavior.is_some() && constraint.behavior.as_deref() == fact_behavior
        })
        .map(|constraint| constraint.kind.clone())
        .collect();
    let (implementation, diagnostics) = match model.schema_uri.as_str() {
        crate::schema::registry::CSV_SCHEMA_URI => {
            crate::validation::csv::compile_parser_contract(model)
        }
        crate::schema::registry::MARKDOWN_SCHEMA_URI => {
            crate::validation::markdown::compile_parser_contract(model)
        }
        crate::schema::registry::XSLT_SCHEMA_URI => {
            crate::validation::xslt::compile_parser_contract(model)
        }
        crate::schema::registry::RELAX_NG_SCHEMA_URI => {
            crate::validation::relax_ng::compile_parser_contract(model)
        }
        crate::schema::registry::SCSS_SCHEMA_URI => {
            crate::validation::scss::compile_parser_contract(model)
        }
        _ => unreachable!("closed parser contract provider catalog"),
    };
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
