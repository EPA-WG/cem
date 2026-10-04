//! Schema-owned limits for explicit consumer resolution of node references.
//! Lexical scope depth and native output-artifact limits are separate policies.

use super::{
    document_model::{compile_schema_document_model, SchemaDocumentModel},
    package_sources::builtin_schema_package_source,
    registry::CEM_ML_SCHEMA_URI,
};
use std::{fmt, sync::OnceLock};

const DEPTH: &str = "reference-traversal-depth";
const WORK: &str = "reference-traversal-work";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceTraversalLimits {
    pub max_depth: usize,
    pub max_work: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceTraversalLimitError {
    pub constraint: String,
    pub value: Option<String>,
}

impl fmt::Display for ReferenceTraversalLimitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} requires a positive traversal bound, got {:?}",
            self.constraint, self.value
        )
    }
}

impl std::error::Error for ReferenceTraversalLimitError {}

impl ReferenceTraversalLimits {
    /// Read the standard limits from the embedded CEM-ML schema, without a
    /// second Rust default table or a fallback for malformed declarations.
    pub fn schema_defaults() -> Result<Self, ReferenceTraversalLimitError> {
        static DEFAULTS: OnceLock<Result<ReferenceTraversalLimits, ReferenceTraversalLimitError>> =
            OnceLock::new();
        DEFAULTS
            .get_or_init(|| {
                let source = builtin_schema_package_source("cem-ml")
                    .expect("embedded CEM-ML schema package");
                let model = compile_schema_document_model(CEM_ML_SCHEMA_URI, source.schema_source);
                Ok(Self {
                    max_depth: bound(&model, DEPTH, None)?,
                    max_work: bound(&model, WORK, None)?,
                })
            })
            .clone()
    }

    /// Apply an effective child scope's declarations to the inherited limits.
    /// Source scope construction and timing remain the caller's responsibility.
    /// Neither the enclosing policy nor the authored references are mutated.
    pub fn for_scope(
        self,
        model: &SchemaDocumentModel,
    ) -> Result<Self, ReferenceTraversalLimitError> {
        Ok(Self {
            max_depth: bound(model, DEPTH, Some(self.max_depth))?,
            max_work: bound(model, WORK, Some(self.max_work))?,
        })
    }
}

fn bound(
    model: &SchemaDocumentModel,
    constraint: &str,
    inherited: Option<usize>,
) -> Result<usize, ReferenceTraversalLimitError> {
    let Some(declaration) = model.constraint(constraint) else {
        return inherited
            .filter(|value| *value > 0)
            .ok_or_else(|| ReferenceTraversalLimitError {
                constraint: constraint.into(),
                value: inherited.map(|value| value.to_string()),
            });
    };
    declaration
        .value
        .as_deref()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| ReferenceTraversalLimitError {
            constraint: constraint.into(),
            value: declaration.value.clone(),
        })
}
