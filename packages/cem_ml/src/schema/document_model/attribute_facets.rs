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
    /// Explicitly omits lexical admission; scalar local restrictions are forbidden.
    TypedScalar(ScalarRepresentation),
    /// Typed item admission belongs to the descriptor; local count facets only.
    TypedList(ScalarRepresentation),
}
impl FacetFamily {
    pub fn representation(self) -> ValueRepresentation {
        match self {
            Self::Shipped(t) => t.representation(),
            Self::List(p) => ValueRepresentation::List(p),
            Self::Nodes => ValueRepresentation::Nodes,
            Self::TypedScalar(p) => ValueRepresentation::Scalar(p),
            Self::TypedList(p) => ValueRepresentation::List(p),
        }
    }
    fn legacy_name(self) -> &'static str {
        match self {
            Self::Shipped(t) => t.name(),
            Self::List(_) => "name-list",
            Self::Nodes => "node",
            Self::TypedScalar(_) => "string",
            Self::TypedList(_) => "name-list",
        }
    }
    pub fn is_typed_only(self) -> bool {
        matches!(self, Self::TypedScalar(_) | Self::TypedList(_))
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
struct LocalFacetContract {
    schema_uri: String,
    family: FacetFamily,
    model: AttributeModel,
    model_bytes: usize,
}
/// An ordered intersection of original facet families. Local constraints are
/// compiled under every family; unsupported fields never disappear on replacement.
#[derive(Debug, Clone)]
pub struct AttributeFacetContract {
    profiles: Vec<LocalFacetContract>,
    model_bytes: usize,
}
impl AttributeFacetContract {
    pub fn compile(
        schema_uri: &str,
        original: &AttributeModel,
        family: FacetFamily,
        limits: FacetLimits,
    ) -> Result<Self, FacetCompilationError> {
        Self::compile_profiles(schema_uri, original, &[family], limits)
    }
    /// All families must describe the same representation. Limits cover the
    /// complete chain before models are copied or local compilation begins.
    pub fn compile_profiles(
        schema_uri: &str,
        original: &AttributeModel,
        families: &[FacetFamily],
        limits: FacetLimits,
    ) -> Result<Self, FacetCompilationError> {
        let bytes = AttributeValueContract {
            model: original.clone(),
            ..Default::default()
        }
        .accounted_bytes()
        .max(1);
        let model_bytes = bytes
            .checked_mul(families.len())
            .filter(|bytes| *bytes <= limits.max_model_bytes)
            .ok_or(FacetCompilationError::Limit)?;
        if families.is_empty()
            || families.iter().any(|f| {
                f.representation() != families[0].representation()
                    || f.is_typed_only() != families[0].is_typed_only()
            })
        {
            return Err(FacetCompilationError::Invalid(vec![Diagnostic {
                code: "cem.datatype.facet-profile.incompatible".into(),
                message:
                    "Facet profiles require one common representation and a nonempty selection"
                        .into(),
                severity: Severity::Error,
                source_map: Some(original.source_map.clone()),
                ..Default::default()
            }]));
        }
        let profiles = families
            .iter()
            .map(|family| LocalFacetContract::compile(schema_uri, original, *family, limits))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            profiles,
            model_bytes,
        })
    }
    pub fn family(&self) -> FacetFamily {
        self.profiles
            .last()
            .expect("nonempty compiled profiles")
            .family
    }
    /// Check ancestors first, stopping on rejection or incomplete work. Every
    /// profile receives the original input and declaring-scope diagnostic context.
    pub fn validate_with_check<E>(
        &self,
        input: FacetInput<'_>,
        context: FacetContext<'_>,
        limits: FacetLimits,
        check: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<AttributeFacetValidation, FacetExecutionError<E>> {
        check().map_err(FacetExecutionError::Interrupted)?;
        let input_bytes = match input {
            FacetInput::Scalar(text) | FacetInput::List { lexical: text, .. } => text.len(),
            FacetInput::Nodes { .. } | FacetInput::TypedScalar | FacetInput::TypedList { .. } => 0,
        };
        if self.model_bytes > limits.max_model_bytes
            || input_bytes
                .checked_mul(self.profiles.len())
                .is_none_or(|bytes| bytes > limits.max_input_bytes)
        {
            return Err(FacetExecutionError::Limit);
        }
        for profile in &self.profiles {
            let result = profile.validate_with_check(
                input,
                FacetContext {
                    element_name: context.element_name,
                    source: context.source,
                    diagnostic_behaviors: context.diagnostic_behaviors,
                    attribute_values: context.attribute_values,
                },
                limits,
                check,
            )?;
            if !result.accepted {
                return Ok(result);
            }
        }
        Ok(AttributeFacetValidation {
            accepted: true,
            diagnostics: vec![],
        })
    }
}
#[derive(Debug, Clone, Copy)]
pub enum FacetInput<'a> {
    TypedScalar,
    TypedList {
        count: usize,
    },
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
impl LocalFacetContract {
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
        if family.is_typed_only() {
            // A whitelist by subtraction also fails closed for future fields.
            // None of these metadata fields establishes lexical independence.
            let mut residual = original.clone();
            residual.name.clear();
            residual.value_type = None;
            residual.native_type_pending = false;
            residual.source_map = Default::default();
            residual.values_diagnostic = None;
            residual.type_diagnostic = None;
            residual.datatype_param_diagnostic = None;
            if matches!(family, FacetFamily::TypedList(_)) {
                residual.item_count = None;
                residual.min_items = None;
                residual.max_items = None;
            }
            if residual != AttributeModel::default() {
                return Err(FacetCompilationError::Invalid(vec![Diagnostic {
                    code: "cem.datatype.typed-only.lexical-field".into(),
                    message: "Typed-only profiles reject lexical fields and defaults; lists admit only count facets".into(),
                    severity: Severity::Error,
                    source_map: Some(original.source_map.clone()),
                    ..Default::default()
                }]));
            }
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
        let (lexical, count) = if self.family.is_typed_only() {
            match (self.family, input) {
                (FacetFamily::TypedScalar(_), FacetInput::TypedScalar) => (None, None),
                (FacetFamily::TypedList(_), FacetInput::TypedList { count }) => (None, Some(count)),
                _ => return Err(FacetExecutionError::InvalidInput),
            }
        } else {
            match (self.family.representation(), input) {
                (ValueRepresentation::Scalar(_), FacetInput::Scalar(s)) => (Some(s), None),
                (ValueRepresentation::List(_), FacetInput::List { lexical, count }) => {
                    (Some(lexical), Some(count))
                }
                (ValueRepresentation::Nodes, FacetInput::Nodes { count }) => (None, Some(count)),
                _ => return Err(FacetExecutionError::InvalidInput),
            }
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
