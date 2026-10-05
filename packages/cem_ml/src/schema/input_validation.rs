//! Opt-in engine validation at a caller-selected runtime lifecycle stage.
//! Source owners are retained after parsing; contexts and reference results are
//! supplied anew by the stage, never by the parser or query registration.
use super::{
    document_model::{SchemaBehaviorEvaluator, SchemaDocumentModel},
    reference_policy::ReferenceScopePolicy,
};
use crate::{
    diagnostics::{Diagnostic, Severity},
    parser::{
        document::CemDocument,
        tree::{CemTreeSemantics, RetainedCemTree},
    },
    run_config::ScopeConfig,
};
use std::{fmt::Debug, sync::Arc};

pub const RUNTIME_INPUT_VALIDATION_FAILED: &str = "cem.schema_validation.runtime_stage_failed";

#[derive(Debug)]
pub struct InputValidationRequest<'a> {
    /// The engine's parsed arena, moved into a retained owner without reparsing.
    pub source: Arc<RetainedCemTree>,
    pub model: &'a SchemaDocumentModel,
    pub root_scope: &'a ScopeConfig,
    /// Standard bounds/disposition with the consuming schema's overrides.
    /// The runtime still chooses child scopes and directed crossing grants.
    pub policy: ReferenceScopePolicy,
    pub behavior_evaluator: Option<&'a dyn SchemaBehaviorEvaluator>,
}
#[derive(Debug, Default)]
pub struct InputValidationOutcome {
    /// Completion is independent of diagnostic severity and violation counts.
    pub complete: bool,
    pub diagnostics: Vec<Diagnostic>,
}
/// Installed explicitly when a runtime can supply per-input context snapshots.
/// The engine invokes it for parser-backed validate/check inputs with ready
/// consuming models; specialized source validators retain their own paths.
/// Validate the retained structure and its behaviors under the supplied model.
/// A pending context returns an incomplete outcome, rather than a setup error.
/// Diagnostics for selected owners must carry their original URI/coordinates;
/// the engine never projects runtime diagnostics against the input bytes.
/// Calls may run concurrently for independent inputs. Do not cache evaluation
/// results or write targets into the source arena shared with runtime consumers.
pub trait InputValidationStage: Debug + Send + Sync {
    fn validate(
        &self,
        request: InputValidationRequest<'_>,
    ) -> Result<InputValidationOutcome, Vec<Diagnostic>>;
}

pub(crate) fn run(
    stage: &dyn InputValidationStage,
    document: CemDocument,
    uri: &str,
    bytes: &[u8],
    root_scope: &ScopeConfig,
    model: &SchemaDocumentModel,
    behavior_evaluator: Option<&dyn SchemaBehaviorEvaluator>,
) -> InputValidationOutcome {
    let prepare = || {
        let policy = ReferenceScopePolicy::schema_defaults()
            .and_then(|policy| policy.for_scope(model))
            .map_err(|error| {
                let mut diagnostic = failure(uri, error.message);
                diagnostic.source_map = Some(error.source_map);
                vec![diagnostic]
            })?;
        let text = String::from_utf8_lossy(bytes);
        let source = RetainedCemTree::new(document, uri, &text, CemTreeSemantics::default(), None)
            .map_err(|message| vec![failure(uri, message)])?;
        stage.validate(InputValidationRequest {
            source,
            model,
            root_scope,
            policy,
            behavior_evaluator,
        })
    };
    let mut result = match prepare() {
        Ok(outcome) => outcome,
        Err(mut diagnostics) => {
            if !diagnostics.iter().any(|d| d.severity.is_hard_violation()) {
                diagnostics.push(failure(
                    uri,
                    "Runtime input validation stage failed without a hard diagnostic",
                ));
            }
            InputValidationOutcome {
                complete: false,
                diagnostics,
            }
        }
    };
    for diagnostic in &mut result.diagnostics {
        // Missing URI denotes an input-stage diagnostic. Foreign-owner
        // provenance is explicit and must not inherit input line coordinates.
        diagnostic.uri.get_or_insert_with(|| uri.to_owned());
    }
    result
}
fn failure(uri: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        code: RUNTIME_INPUT_VALIDATION_FAILED.into(),
        severity: Severity::Error,
        uri: Some(uri.into()),
        message: message.into(),
        ..Default::default()
    }
}
