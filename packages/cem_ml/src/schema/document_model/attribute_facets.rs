//! Explicit adapters to shipped facet semantics. Profile choice is host authority;
//! source datatype names are never used to choose a profile.
use super::*;
use crate::schema::datatype_validation::{ScalarRepresentation, ValueRepresentation};
use shipped_datatypes::ShippedDatatype;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetFamily {
    Shipped(ShippedDatatype),
    List(ScalarRepresentation),
    Nodes,
}
impl FacetFamily {
    pub fn representation(self) -> ValueRepresentation {
        match self {
            Self::Shipped(t) => t.representation(),
            Self::List(p) => ValueRepresentation::List(p),
            Self::Nodes => ValueRepresentation::Nodes,
        }
    }
    fn legacy_name(self) -> &'static str {
        match self {
            Self::Shipped(t) => t.name(),
            Self::List(_) => "name-list",
            Self::Nodes => "node",
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct FacetLimits {
    pub max_model_bytes: usize,
    pub max_input_bytes: usize,
    pub max_diagnostics: usize,
}
impl Default for FacetLimits {
    fn default() -> Self {
        Self {
            max_model_bytes: 1_048_576,
            max_input_bytes: 1_048_576,
            max_diagnostics: 100_000,
        }
    }
}
#[derive(Debug, Clone)]
pub enum FacetCompilationError {
    Limit,
    Invalid(Vec<Diagnostic>),
}
#[derive(Debug)]
pub enum FacetExecutionError<E> {
    Interrupted(E),
    Limit,
    InvalidInput,
}
#[derive(Debug, Clone)]
pub struct AttributeFacetContract {
    schema_uri: String,
    family: FacetFamily,
    model: AttributeModel,
    model_bytes: usize,
}
#[derive(Debug, Clone, Copy)]
pub enum FacetInput<'a> {
    Scalar(&'a str),
    /// Count comes from the already-prepared sequence, never a second tokenizer.
    List {
        lexical: &'a str,
        count: usize,
    },
    Nodes {
        count: usize,
    },
}
pub struct FacetContext<'a> {
    pub element_name: &'a str,
    pub source: &'a CemAstNode,
    pub diagnostic_behaviors: &'a BTreeMap<String, DiagnosticBehavior>,
    pub attribute_values: &'a BTreeMap<String, String>,
}
#[derive(Debug, Clone)]
pub struct AttributeFacetValidation {
    /// Local facets only. Overall acceptance also requires the datatype contract.
    pub accepted: bool,
    pub diagnostics: Vec<Diagnostic>,
}
impl AttributeFacetContract {
    /// Compile a previously captured local model under an explicitly selected
    /// family. The source metadata and native readiness flag are not mutated.
    pub fn compile(
        schema_uri: &str,
        original: &AttributeModel,
        family: FacetFamily,
        limits: FacetLimits,
    ) -> Result<Self, FacetCompilationError> {
        let bytes = AttributeValueContract {
            model: original.clone(),
            ..Default::default()
        }
        .accounted_bytes();
        if bytes > limits.max_model_bytes {
            return Err(FacetCompilationError::Limit);
        }
        let mut model = original.clone();
        model.value_type = Some(family.legacy_name().into());
        model.native_type_pending = false;
        let mut diagnostics = vec![];
        validate_attribute_datatype_param_definition(schema_uri, &model, &mut diagnostics);
        if diagnostics.len() > limits.max_diagnostics {
            return Err(FacetCompilationError::Limit);
        }
        if !diagnostics.is_empty() {
            return Err(FacetCompilationError::Invalid(diagnostics));
        }
        Ok(Self {
            schema_uri: schema_uri.into(),
            family,
            model,
            model_bytes: bytes,
        })
    }
    pub fn family(&self) -> FacetFamily {
        self.family
    }
    /// Validate local restrictions without conversion or node atomization.
    /// Callers supply original source and diagnostic bindings from the declaring scope.
    pub fn validate_with_check<E>(
        &self,
        input: FacetInput<'_>,
        context: FacetContext<'_>,
        limits: FacetLimits,
        check: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<AttributeFacetValidation, FacetExecutionError<E>> {
        check().map_err(FacetExecutionError::Interrupted)?;
        if self.model_bytes > limits.max_model_bytes {
            return Err(FacetExecutionError::Limit);
        }
        let (lexical, count) = match (self.family.representation(), input) {
            (ValueRepresentation::Scalar(_), FacetInput::Scalar(s)) => (Some(s), None),
            (ValueRepresentation::List(_), FacetInput::List { lexical, count }) => {
                (Some(lexical), Some(count))
            }
            (ValueRepresentation::Nodes, FacetInput::Nodes { count }) => (None, Some(count)),
            _ => return Err(FacetExecutionError::InvalidInput),
        };
        if lexical.is_some_and(|s| s.len() > limits.max_input_bytes) {
            return Err(FacetExecutionError::Limit);
        }
        let mut diagnostics = vec![];
        if let Some(lexical) = lexical {
            let value = validation_attribute_value(lexical, &self.model);
            // Descriptor validation already checks list items. Do not retokenize
            // them or impose name-list syntax on a generic registered item type.
            let type_valid = count.is_some()
                || validate_attribute_type(
                    &self.schema_uri,
                    context.diagnostic_behaviors,
                    context.element_name,
                    &self.model.name,
                    &value,
                    &self.model,
                    context.attribute_values,
                    context.source,
                    &mut diagnostics,
                );
            if type_valid {
                let mut scalar_facets = self.model.clone();
                scalar_facets.item_count = None;
                scalar_facets.min_items = None;
                scalar_facets.max_items = None;
                validate_attribute_datatype_params(
                    &self.schema_uri,
                    context.diagnostic_behaviors,
                    context.element_name,
                    &self.model.name,
                    &value,
                    &scalar_facets,
                    context.attribute_values,
                    context.source,
                    &mut diagnostics,
                );
            }
            validate_attribute_value(
                &self.schema_uri,
                context.diagnostic_behaviors,
                context.element_name,
                &self.model.name,
                &value,
                &self.model,
                context.attribute_values,
                context.source,
                &mut diagnostics,
            );
        }
        check().map_err(FacetExecutionError::Interrupted)?;
        if let Some(count) = count {
            validate_attribute_count(
                &self.schema_uri,
                context.diagnostic_behaviors,
                context.element_name,
                &self.model.name,
                &self.model,
                count,
                context.source,
                (self.family == FacetFamily::Nodes).then_some(1),
                &mut diagnostics,
            );
        }
        check().map_err(FacetExecutionError::Interrupted)?;
        if diagnostics.len() > limits.max_diagnostics {
            return Err(FacetExecutionError::Limit);
        }
        Ok(AttributeFacetValidation {
            accepted: diagnostics.is_empty(),
            diagnostics,
        })
    }
}
