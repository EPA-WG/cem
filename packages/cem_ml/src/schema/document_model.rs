//! Minimal schema-owned document model validation.
//!
//! This is the first runtime slice that consumes embedded `schema/*.cem`
//! package sources as validation data. It intentionally starts with the
//! structural declarations already present in `{elements}`:
//!
//! - declared element names;
//! - required and optional attributes per element;
//! - direct child element allow-lists.
//! - built-in base element references through schema `{uses}` aliases.
//! - schema-owned field contracts for element-bound conditional checks.
//! - schema-owned diagnostic declarations resolved through declarative engine
//!   behavior definitions, including severity and message metadata.
//! - schema-owned attribute `@values`, boolean/integer type checks, and
//!   integer `minInclusive` datatype-param checks.
//!
//! Cardinality, ordering, scalar type checks beyond boolean/integer syntax,
//! datatype params beyond `minInclusive`, and semantic constraints remain
//! follow-up work.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

use crate::diagnostics::{Diagnostic, Severity};
use crate::events::cem::CemEventNormalizer;
use crate::parser::builder::CemAstBuilder;
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
use crate::resolver::{has_uri_scheme, is_windows_drive_path};
use crate::schema::package_loader::{
    load_builtin_schema_package, load_builtin_schema_package_for_content_type,
};
use crate::schema::registry::{
    CEM_ML_SCHEMA_URI, CEM_NATIVE_TEMPLATE_SCHEMA_URI, CEM_SCHEMA_PACKAGE_URI, CEM_SCHEMA_URI,
    CEM_TRANSFORM_SCHEMA_URI,
};
use crate::source::{BytesSource, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack};
use crate::tokenizer::cem::CemTokenizer;

pub const UNKNOWN_ELEMENT_CODE: &str = "cem.schema_model.unknown_element";
pub const UNKNOWN_ATTRIBUTE_CODE: &str = "cem.schema_model.unknown_attribute";
pub const MISSING_REQUIRED_ATTRIBUTE_CODE: &str = "cem.schema_model.missing_required_attribute";
pub const INVALID_ATTRIBUTE_VALUE_CODE: &str = "cem.schema_model.invalid_attribute_value";
pub const INVALID_ATTRIBUTE_TYPE_CODE: &str = "cem.schema_model.invalid_attribute_type";
pub const INVALID_ATTRIBUTE_DATATYPE_PARAM_CODE: &str =
    "cem.schema_model.invalid_attribute_datatype_param";
pub const INVALID_CHILD_ELEMENT_CODE: &str = "cem.schema_model.invalid_child_element";
pub const UNRESOLVED_DIAGNOSTIC_REFERENCE_CODE: &str =
    "cem.schema_definition.unresolved_diagnostic_reference";
pub const UNKNOWN_DIAGNOSTIC_BEHAVIOR_CODE: &str =
    "cem.schema_definition.unknown_diagnostic_behavior";
pub const UNRESOLVED_BEHAVIOR_FUNCTION_CODE: &str =
    "cem.schema_definition.unresolved_behavior_function";
pub const INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE: &str =
    "cem.schema_definition.invalid_diagnostic_behavior_contract";
pub const SCHEMA_BEHAVIOR_QUERY_INVALID_CODE: &str = "cem.schema_behavior.query_invalid";
pub const SCHEMA_BEHAVIOR_QUERY_FAILED_CODE: &str = "cem.schema_behavior.query_failed";
pub const SCHEMA_BEHAVIOR_RESULT_INVALID_CODE: &str = "cem.schema_behavior.result_invalid";
pub const SCHEMA_BEHAVIOR_FUNCTION_FAILED_CODE: &str = "cem.schema_behavior.function_failed";
pub const FIELD_CONTRACT_DIAGNOSTIC_BEHAVIOR: &str = "schema:field-contract";
pub const VALUE_VOCABULARY_DIAGNOSTIC_BEHAVIOR: &str = "schema:value-vocabulary";
pub const SCALAR_TYPE_DIAGNOSTIC_BEHAVIOR: &str = "schema:scalar-type";
pub const DATATYPE_PARAM_DIAGNOSTIC_BEHAVIOR: &str = "schema:datatype-param";
pub const RESOURCE_READABLE_DIAGNOSTIC_BEHAVIOR: &str = "schema:resource-readable";
pub const RESOURCE_PARSE_DIAGNOSTIC_BEHAVIOR: &str = "schema:resource-parse";
pub const REFERENCE_RESOLUTION_DIAGNOSTIC_BEHAVIOR: &str = "schema:reference-resolution";

pub trait SchemaBehaviorEvaluator: std::fmt::Debug + Send + Sync {
    fn compile_model(&self, _model: &SchemaDocumentModel) -> Vec<Diagnostic> {
        Vec::new()
    }

    fn validate_document(
        &self,
        document: &CemDocument,
        model: &SchemaDocumentModel,
    ) -> Vec<Diagnostic>;
}

#[derive(Debug, Clone, Default)]
pub struct SchemaDocumentModel {
    pub schema_uri: String,
    pub elements: BTreeMap<String, ElementModel>,
    pub attributes: BTreeMap<String, AttributeModel>,
    pub behaviors: BTreeMap<String, BehaviorDefinition>,
    pub constraints: BTreeMap<String, ConstraintDefinition>,
    pub diagnostic_behaviors: BTreeMap<String, DiagnosticBehavior>,
    pub compile_diagnostics: Vec<Diagnostic>,
}

impl SchemaDocumentModel {
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn element(&self, name: &str) -> Option<&ElementModel> {
        self.elements.get(name)
    }

    pub fn constraint(&self, kind: &str) -> Option<&ConstraintDefinition> {
        self.constraints.get(kind)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ElementModel {
    pub name: String,
    pub required_attributes: BTreeSet<String>,
    pub optional_attributes: BTreeSet<String>,
    pub child_elements: BTreeSet<String>,
    pub allow_any_child: bool,
    pub field_contracts: Vec<FieldContract>,
}

impl ElementModel {
    fn allows_attribute(&self, prefix: &str, local_name: &str) -> bool {
        self.required_attributes.contains(local_name)
            || self.optional_attributes.contains(local_name)
            || self.optional_attributes.contains(&format!("{prefix}:*"))
            || self.required_attributes.contains(&format!("{prefix}:*"))
    }

    fn allows_child(&self, local_name: &str) -> bool {
        self.allow_any_child || self.child_elements.contains(local_name)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttributeModel {
    pub name: String,
    pub value_type: Option<String>,
    pub allowed_values: BTreeSet<String>,
    pub min_inclusive: Option<String>,
    pub values_diagnostic: Option<String>,
    pub type_diagnostic: Option<String>,
    pub datatype_param_diagnostic: Option<String>,
    pub source_map: SourceMapStack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticBehavior {
    pub code: String,
    pub severity: Severity,
    pub behavior: String,
    pub definition: Option<BehaviorDefinition>,
    pub engine_behavior: Option<EngineDiagnosticBehavior>,
    pub function: Option<String>,
    pub function_definition: Option<BehaviorFunctionDeclaration>,
    pub arguments: Vec<BehaviorArgument>,
    pub message: Option<String>,
    pub source_map: SourceMapStack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineDiagnosticBehavior {
    FieldContract,
    ValueVocabulary,
    ScalarType,
    DatatypeParam,
    ResourceReadable,
    ResourceParse,
    ReferenceResolution,
}

impl EngineDiagnosticBehavior {
    fn reference(self) -> &'static str {
        match self {
            Self::FieldContract => FIELD_CONTRACT_DIAGNOSTIC_BEHAVIOR,
            Self::ValueVocabulary => VALUE_VOCABULARY_DIAGNOSTIC_BEHAVIOR,
            Self::ScalarType => SCALAR_TYPE_DIAGNOSTIC_BEHAVIOR,
            Self::DatatypeParam => DATATYPE_PARAM_DIAGNOSTIC_BEHAVIOR,
            Self::ResourceReadable => RESOURCE_READABLE_DIAGNOSTIC_BEHAVIOR,
            Self::ResourceParse => RESOURCE_PARSE_DIAGNOSTIC_BEHAVIOR,
            Self::ReferenceResolution => REFERENCE_RESOLUTION_DIAGNOSTIC_BEHAVIOR,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintDefinition {
    pub schema_uri: String,
    pub kind: String,
    pub target: Option<String>,
    pub value: Option<String>,
    pub policy: Option<String>,
    pub diagnostic: Option<String>,
    pub behavior: Option<String>,
    pub definition: Option<BehaviorDefinition>,
    pub engine_behavior: Option<EngineDiagnosticBehavior>,
    pub source_map: SourceMapStack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorDefinition {
    pub schema_uri: String,
    pub uses: BTreeMap<String, String>,
    pub name: String,
    pub implementation: String,
    pub execution: String,
    pub primitive: Option<String>,
    pub function: Option<String>,
    pub select: Option<String>,
    pub match_query: Option<String>,
    pub inputs: Vec<BehaviorInput>,
    pub parameters: Vec<BehaviorParameter>,
    pub result: Option<BehaviorResult>,
    pub inline_functions: BTreeMap<String, BehaviorFunctionDeclaration>,
    pub source_map: SourceMapStack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorInput {
    pub name: String,
    pub value_type: String,
    pub source: String,
    pub required: bool,
    pub source_range: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorParameter {
    pub name: String,
    pub value_type: String,
    pub required: bool,
    pub default: Option<String>,
    pub values: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorArgument {
    pub name: String,
    pub value: String,
    pub source_map: SourceMapStack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorResult {
    pub value_type: String,
    pub severity: Option<String>,
    pub message: Option<String>,
    pub source_range: Option<String>,
    pub details: Vec<BehaviorDetail>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorDetail {
    pub name: String,
    pub value_type: String,
    pub required: bool,
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorFunctionDeclaration {
    pub name: String,
    pub returns: String,
    pub visibility: String,
    pub params: Vec<BehaviorFunctionParam>,
    pub body_expression: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorFunctionParam {
    pub name: String,
    pub value_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AttributeTypeViolation {
    name: &'static str,
    check_kind: &'static str,
    expected_values: &'static [&'static str],
    expected_pattern: &'static str,
    allows_empty: bool,
    message: &'static str,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldContract {
    pub name: String,
    pub target: String,
    pub diagnostic: Option<String>,
    pub check_kind: Option<String>,
    pub required_attributes: BTreeSet<String>,
    pub optional_attributes: BTreeSet<String>,
    pub forbidden_attributes: BTreeSet<String>,
    pub forbidden_attribute_values: BTreeMap<String, BTreeSet<String>>,
    pub required_children: BTreeSet<String>,
    pub max_one_children: BTreeSet<String>,
    pub path_layout_attributes: BTreeSet<String>,
    pub path_layout_prefix: Option<String>,
    pub path_layout_extension: Option<String>,
    pub when_attribute: Option<String>,
    pub when_values: BTreeSet<String>,
    pub when_present_attributes: BTreeSet<String>,
    pub source_map: SourceMapStack,
}

impl FieldContract {
    fn applies_to(&self, attributes: &BTreeMap<String, String>) -> bool {
        if !self
            .when_present_attributes
            .iter()
            .all(|name| attributes.contains_key(name))
        {
            return false;
        }
        let Some(when_attribute) = self.when_attribute.as_deref() else {
            return true;
        };
        let Some(value) = attributes
            .get(when_attribute)
            .map(String::as_str)
            .map(str::trim)
        else {
            return false;
        };
        if self.when_values.is_empty() {
            return !value.is_empty();
        }
        self.when_values.contains(value)
    }

    fn diagnostic_code(&self) -> &str {
        self.diagnostic
            .as_deref()
            .unwrap_or(MISSING_REQUIRED_ATTRIBUTE_CODE)
    }

    fn check_kind(&self) -> &str {
        self.check_kind.as_deref().unwrap_or("field-contract")
    }
}

pub fn load_builtin_document_model_for_identity(
    schema_uri: Option<&str>,
    content_type: Option<&str>,
) -> Option<SchemaDocumentModel> {
    let package = match schema_uri {
        Some(schema_uri) => load_builtin_schema_package(schema_uri).ok(),
        None => content_type.and_then(|content_type| {
            load_builtin_schema_package_for_content_type(content_type).ok()
        }),
    }?;
    if !is_bootstrap_document_model_schema(&package.descriptor.schema_uri) {
        return None;
    }

    Some(compile_document_model(
        &package.descriptor.schema_uri,
        package.schema_source,
    ))
    .filter(|model| !model.is_empty())
}

pub fn validate_document_model(
    document: &CemDocument,
    model: &SchemaDocumentModel,
) -> Vec<Diagnostic> {
    validate_document_model_with_behavior_evaluator(document, model, None)
}

pub fn validate_document_model_with_behavior_evaluator(
    document: &CemDocument,
    model: &SchemaDocumentModel,
    behavior_evaluator: Option<&dyn SchemaBehaviorEvaluator>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if model.is_empty() {
        return diagnostics;
    }

    let Some(CemAstNode::Document { root_children, .. }) = document.root() else {
        return diagnostics;
    };
    for child_id in root_children {
        validate_node(document, model, *child_id, false, &mut diagnostics);
    }
    if let Some(behavior_evaluator) = behavior_evaluator {
        diagnostics.extend(behavior_evaluator.compile_model(model));
        diagnostics.extend(behavior_evaluator.validate_document(document, model));
    }
    if model.schema_uri == CEM_SCHEMA_URI {
        diagnostics.extend(compile_authored_schema_behaviors(
            document,
            behavior_evaluator,
        ));
    }

    diagnostics
}

fn compile_authored_schema_behaviors(
    document: &CemDocument,
    behavior_evaluator: Option<&dyn SchemaBehaviorEvaluator>,
) -> Vec<Diagnostic> {
    let Some(schema_id) = first_element_id_by_local_name(document, "schema") else {
        return Vec::new();
    };
    let attrs = collect_attrs(document, schema_id);
    let Some(schema_uri) = optional_non_empty_attr(&attrs, "namespace") else {
        return Vec::new();
    };
    let model = compile_document_model_from_document(schema_uri, document);
    let mut diagnostics = model.compile_diagnostics.clone();
    if let Some(behavior_evaluator) = behavior_evaluator {
        diagnostics.extend(behavior_evaluator.compile_model(&model));
    }
    diagnostics
}

fn validate_node(
    document: &CemDocument,
    model: &SchemaDocumentModel,
    node_id: AstNodeId,
    parent_allows_any_child: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(node) = document.get(node_id) else {
        return;
    };
    let CemAstNode::Element {
        expanded_name,
        attributes,
        children,
        ..
    } = node
    else {
        return;
    };
    let local = expanded_name.local_name.as_str();
    if should_skip_structural_name(local) {
        return;
    }

    let Some(element_model) = model.element(local) else {
        if !parent_allows_any_child {
            diagnostics.push(diag_at(
                UNKNOWN_ELEMENT_CODE,
                format!(
                    "element `{local}` is not declared by schema `{}`",
                    model.schema_uri
                ),
                node,
            ));
        }
        return;
    };

    let mut seen_attributes = BTreeSet::new();
    let mut attribute_values = BTreeMap::new();
    for attr_id in attributes {
        let Some(attr) = document.get(*attr_id) else {
            continue;
        };
        let Some((attr_prefix, attr_local, attr_value)) = attribute_parts(attr) else {
            continue;
        };
        seen_attributes.insert(attr_local.to_owned());
        attribute_values.insert(
            attr_local.to_owned(),
            attr_value.unwrap_or_default().to_owned(),
        );
        if !element_model.allows_attribute(attr_prefix, attr_local) {
            diagnostics.push(diag_at(
                UNKNOWN_ATTRIBUTE_CODE,
                format!(
                    "attribute `{attr_local}` is not declared on element `{local}` by schema `{}`",
                    model.schema_uri
                ),
                attr,
            ));
        } else if let Some(attribute_model) = model.attributes.get(attr_local) {
            validate_attribute_contracts(
                &model.schema_uri,
                &model.diagnostic_behaviors,
                local,
                attr_local,
                attr_value.unwrap_or_default(),
                attribute_model,
                &attribute_values,
                attr,
                diagnostics,
            );
        }
    }

    let child_counts = child_element_counts(document, children);

    validate_field_contracts(
        &model.schema_uri,
        &model.diagnostic_behaviors,
        local,
        element_model,
        &seen_attributes,
        &attribute_values,
        &child_counts,
        node,
        diagnostics,
    );

    for required in &element_model.required_attributes {
        if !seen_attributes.contains(required) {
            diagnostics.push(diag_at(
                MISSING_REQUIRED_ATTRIBUTE_CODE,
                format!(
                    "element `{local}` is missing required attribute `{required}` from schema `{}`",
                    model.schema_uri
                ),
                node,
            ));
        }
    }

    for child_id in children {
        let Some(child) = document.get(*child_id) else {
            continue;
        };
        let Some(child_local) = element_local_name(child) else {
            continue;
        };
        if should_skip_structural_name(child_local) {
            continue;
        }
        if !element_model.allows_child(child_local) {
            diagnostics.push(diag_at(
                INVALID_CHILD_ELEMENT_CODE,
                format!(
                    "element `{child_local}` is not an allowed child of `{local}` by schema `{}`",
                    model.schema_uri
                ),
                child,
            ));
        }
        validate_node(
            document,
            model,
            *child_id,
            element_model.allow_any_child,
            diagnostics,
        );
    }
}

fn validate_attribute_contracts(
    schema_uri: &str,
    diagnostic_behaviors: &BTreeMap<String, DiagnosticBehavior>,
    element_name: &str,
    attribute_name: &str,
    value: &str,
    attribute_model: &AttributeModel,
    attribute_values: &BTreeMap<String, String>,
    node: &CemAstNode,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let type_valid = validate_attribute_type(
        schema_uri,
        diagnostic_behaviors,
        element_name,
        attribute_name,
        value,
        attribute_model,
        attribute_values,
        node,
        diagnostics,
    );
    if type_valid {
        validate_attribute_datatype_params(
            schema_uri,
            diagnostic_behaviors,
            element_name,
            attribute_name,
            value,
            attribute_model,
            attribute_values,
            node,
            diagnostics,
        );
    }
    validate_attribute_value(
        schema_uri,
        diagnostic_behaviors,
        element_name,
        attribute_name,
        value,
        attribute_model,
        attribute_values,
        node,
        diagnostics,
    );
}

fn validate_attribute_type(
    schema_uri: &str,
    diagnostic_behaviors: &BTreeMap<String, DiagnosticBehavior>,
    element_name: &str,
    attribute_name: &str,
    value: &str,
    attribute_model: &AttributeModel,
    attribute_values: &BTreeMap<String, String>,
    node: &CemAstNode,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(value_type) = attribute_model.value_type.as_deref() else {
        return true;
    };
    let value = value.trim();
    let violation = if is_boolean_type_reference(value_type) {
        (!matches!(value, "" | "true" | "false")).then_some(AttributeTypeViolation {
            name: "boolean",
            check_kind: "type:boolean",
            expected_values: &["false", "true"],
            expected_pattern: "empty | true | false",
            allows_empty: true,
            message: "outside schema-declared boolean values",
        })
    } else if is_integer_type_reference(value_type) {
        (!is_signed_decimal_integer(value)).then_some(AttributeTypeViolation {
            name: "integer",
            check_kind: "type:integer",
            expected_values: &[],
            expected_pattern: "signed decimal integer",
            allows_empty: false,
            message: "not a schema-declared integer",
        })
    } else {
        None
    };
    let Some(violation) = violation else {
        return true;
    };

    let diagnostic_behavior = engine_diagnostic_behavior(
        diagnostic_behaviors,
        attribute_model.type_diagnostic.as_deref(),
        EngineDiagnosticBehavior::ScalarType,
    );
    let code = diagnostic_behavior
        .map(|behavior| behavior.code.as_str())
        .unwrap_or(INVALID_ATTRIBUTE_TYPE_CODE);
    let generated_message = format!(
        "attribute `{attribute_name}` on element `{element_name}` has value `{value}` {}",
        violation.message
    );
    diagnostics.push(diag_at_with_details_and_severity(
        code,
        diagnostic_behavior
            .map(|behavior| behavior.severity)
            .unwrap_or(Severity::Error),
        behavior_message(diagnostic_behavior, generated_message),
        node,
        attribute_type_details(
            schema_uri,
            element_name,
            attribute_name,
            value,
            attribute_model,
            violation,
            code,
            diagnostic_behavior
                .map(|behavior| behavior.behavior.as_str())
                .unwrap_or(SCALAR_TYPE_DIAGNOSTIC_BEHAVIOR),
            attribute_values,
            node,
        ),
    ));
    false
}

fn engine_diagnostic_behavior<'a>(
    diagnostic_behaviors: &'a BTreeMap<String, DiagnosticBehavior>,
    code: Option<&str>,
    expected_engine_behavior: EngineDiagnosticBehavior,
) -> Option<&'a DiagnosticBehavior> {
    let code = code?;
    diagnostic_behaviors
        .get(code)
        .filter(|behavior| behavior.engine_behavior == Some(expected_engine_behavior))
}

fn behavior_message(
    diagnostic_behavior: Option<&DiagnosticBehavior>,
    generated_message: String,
) -> String {
    diagnostic_behavior
        .and_then(|behavior| behavior.message.as_deref())
        .map(|message| format!("{message}: {generated_message}"))
        .unwrap_or(generated_message)
}

fn is_boolean_type_reference(value_type: &str) -> bool {
    type_reference_local_name(value_type) == "boolean"
}

fn is_integer_type_reference(value_type: &str) -> bool {
    type_reference_local_name(value_type) == "integer"
}

fn type_reference_local_name(value_type: &str) -> &str {
    let value_type = value_type.trim();
    value_type
        .rsplit_once(':')
        .map(|(_, local)| local)
        .unwrap_or(value_type)
}

fn is_signed_decimal_integer(value: &str) -> bool {
    let value = value.trim();
    let digits = value.strip_prefix(['+', '-']).unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn validate_attribute_datatype_params(
    schema_uri: &str,
    diagnostic_behaviors: &BTreeMap<String, DiagnosticBehavior>,
    element_name: &str,
    attribute_name: &str,
    value: &str,
    attribute_model: &AttributeModel,
    attribute_values: &BTreeMap<String, String>,
    node: &CemAstNode,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(min_inclusive) = attribute_model.min_inclusive.as_deref() else {
        return;
    };
    let Some(value_type) = attribute_model.value_type.as_deref() else {
        return;
    };
    if !is_integer_type_reference(value_type) {
        return;
    }
    let value = value.trim();
    if decimal_integer_cmp(value, min_inclusive) != Some(std::cmp::Ordering::Less) {
        return;
    }

    let diagnostic_behavior = engine_diagnostic_behavior(
        diagnostic_behaviors,
        attribute_model.datatype_param_diagnostic.as_deref(),
        EngineDiagnosticBehavior::DatatypeParam,
    );
    let code = diagnostic_behavior
        .map(|behavior| behavior.code.as_str())
        .unwrap_or(INVALID_ATTRIBUTE_DATATYPE_PARAM_CODE);
    let generated_message = format!(
        "attribute `{attribute_name}` on element `{element_name}` has value `{value}` below minInclusive `{min_inclusive}`"
    );
    diagnostics.push(diag_at_with_details_and_severity(
        code,
        diagnostic_behavior
            .map(|behavior| behavior.severity)
            .unwrap_or(Severity::Error),
        behavior_message(diagnostic_behavior, generated_message),
        node,
        attribute_datatype_param_details(
            schema_uri,
            element_name,
            attribute_name,
            value,
            attribute_model,
            "minInclusive",
            min_inclusive,
            code,
            diagnostic_behavior
                .map(|behavior| behavior.behavior.as_str())
                .unwrap_or(DATATYPE_PARAM_DIAGNOSTIC_BEHAVIOR),
            attribute_values,
            node,
        ),
    ));
}

fn decimal_integer_cmp(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    let left = normalize_decimal_integer(left)?;
    let right = normalize_decimal_integer(right)?;
    Some(compare_normalized_decimal_integer(&left, &right))
}

fn normalize_decimal_integer(value: &str) -> Option<(bool, String)> {
    let value = value.trim();
    let (negative, digits) = if let Some(rest) = value.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = value.strip_prefix('+') {
        (false, rest)
    } else {
        (false, value)
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let digits = digits.trim_start_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    Some((negative && digits != "0", digits.to_owned()))
}

fn compare_normalized_decimal_integer(
    left: &(bool, String),
    right: &(bool, String),
) -> std::cmp::Ordering {
    let (left_negative, left_digits) = left;
    let (right_negative, right_digits) = right;
    match (*left_negative, *right_negative) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        (false, false) => compare_positive_decimal_digits(left_digits, right_digits),
        (true, true) => compare_positive_decimal_digits(right_digits, left_digits),
    }
}

fn compare_positive_decimal_digits(left: &str, right: &str) -> std::cmp::Ordering {
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

fn validate_attribute_value(
    schema_uri: &str,
    diagnostic_behaviors: &BTreeMap<String, DiagnosticBehavior>,
    element_name: &str,
    attribute_name: &str,
    value: &str,
    attribute_model: &AttributeModel,
    attribute_values: &BTreeMap<String, String>,
    node: &CemAstNode,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let value = value.trim();
    if attribute_model.allowed_values.is_empty() || attribute_model.allowed_values.contains(value) {
        return;
    }

    let diagnostic_behavior = engine_diagnostic_behavior(
        diagnostic_behaviors,
        attribute_model.values_diagnostic.as_deref(),
        EngineDiagnosticBehavior::ValueVocabulary,
    );
    let code = diagnostic_behavior
        .map(|behavior| behavior.code.as_str())
        .unwrap_or(INVALID_ATTRIBUTE_VALUE_CODE);
    let generated_message = format!(
        "attribute `{attribute_name}` on element `{element_name}` has value `{value}` outside schema-declared values: {}",
        attribute_model
            .allowed_values
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );
    diagnostics.push(diag_at_with_details_and_severity(
        code,
        diagnostic_behavior
            .map(|behavior| behavior.severity)
            .unwrap_or(Severity::Error),
        behavior_message(diagnostic_behavior, generated_message),
        node,
        attribute_value_details(
            schema_uri,
            element_name,
            attribute_name,
            value,
            attribute_model,
            code,
            diagnostic_behavior
                .map(|behavior| behavior.behavior.as_str())
                .unwrap_or(VALUE_VOCABULARY_DIAGNOSTIC_BEHAVIOR),
            attribute_values,
            node,
        ),
    ));
}

pub fn compile_schema_document_model(schema_uri: &str, schema_source: &str) -> SchemaDocumentModel {
    compile_document_model(schema_uri, schema_source)
}

fn compile_document_model(schema_uri: &str, schema_source: &str) -> SchemaDocumentModel {
    compile_document_model_with_seen(schema_uri, schema_source, &mut BTreeSet::new())
}

fn compile_document_model_with_seen(
    schema_uri: &str,
    schema_source: &str,
    seen_schema_uris: &mut BTreeSet<String>,
) -> SchemaDocumentModel {
    let document = parse_cem_document(schema_source);
    compile_document_model_from_document_with_seen(schema_uri, &document, seen_schema_uris)
}

fn compile_document_model_from_document(
    schema_uri: &str,
    document: &CemDocument,
) -> SchemaDocumentModel {
    compile_document_model_from_document_with_seen(schema_uri, document, &mut BTreeSet::new())
}

fn compile_document_model_from_document_with_seen(
    schema_uri: &str,
    document: &CemDocument,
    seen_schema_uris: &mut BTreeSet<String>,
) -> SchemaDocumentModel {
    if !seen_schema_uris.insert(schema_uri.to_owned()) {
        return empty_document_model(schema_uri);
    }

    let mut model = empty_document_model(schema_uri);

    let Some(schema_id) = first_element_id_by_local_name(document, "schema") else {
        seen_schema_uris.remove(schema_uri);
        return model;
    };
    let uses = collect_schema_uses(document, schema_id);
    model.behaviors = collect_behavior_definitions(document, schema_id, schema_uri, &uses);

    for elements_id in element_child_ids_by_local_name(document, schema_id, "elements") {
        let Some(CemAstNode::Element { children, .. }) = document.get(elements_id) else {
            continue;
        };
        for child_id in children {
            let Some(child) = document.get(*child_id) else {
                continue;
            };
            if element_local_name(child) != Some("element") {
                continue;
            }
            let Some(element_model) =
                compile_element_model(document, *child_id, &uses, seen_schema_uris)
            else {
                continue;
            };
            model
                .elements
                .insert(element_model.name.clone(), element_model);
        }
    }

    model.attributes = collect_attribute_models(document, schema_id);
    let (diagnostic_behaviors, diagnostic_compile_diagnostics) =
        collect_diagnostic_behaviors(document, schema_id, schema_uri, &uses, &model.behaviors);
    model.diagnostic_behaviors = diagnostic_behaviors;
    model
        .compile_diagnostics
        .extend(diagnostic_compile_diagnostics);
    model.constraints =
        collect_constraint_definitions(document, schema_id, schema_uri, &uses, &model.behaviors);

    for contract in collect_field_contracts(document, schema_id) {
        if let Some(code) = contract.diagnostic.as_deref() {
            if !model.diagnostic_behaviors.contains_key(code) {
                model.compile_diagnostics.push(schema_compile_diagnostic(
                    UNRESOLVED_DIAGNOSTIC_REFERENCE_CODE,
                    format!(
                        "field contract `{}` references diagnostic behavior `{code}` that is not declared by schema `{schema_uri}`",
                        contract.name
                    ),
                    &contract.source_map,
                    serde_json::json!({
                        "schemaUri": schema_uri,
                        "contract": &contract.name,
                        "diagnostic": code,
                        "checkKind": "diagnostic-reference-resolution",
                    }),
                ));
            }
        }
        if let Some(element_model) = model.elements.get_mut(&contract.target) {
            element_model.field_contracts.push(contract);
        }
    }
    for attribute_model in model.attributes.values() {
        validate_attribute_diagnostic_reference(
            schema_uri,
            attribute_model,
            attribute_model.values_diagnostic.as_deref(),
            "values-diagnostic",
            EngineDiagnosticBehavior::ValueVocabulary,
            &model.diagnostic_behaviors,
            &mut model.compile_diagnostics,
        );
        validate_attribute_diagnostic_reference(
            schema_uri,
            attribute_model,
            attribute_model.type_diagnostic.as_deref(),
            "type-diagnostic",
            EngineDiagnosticBehavior::ScalarType,
            &model.diagnostic_behaviors,
            &mut model.compile_diagnostics,
        );
        validate_attribute_diagnostic_reference(
            schema_uri,
            attribute_model,
            attribute_model.datatype_param_diagnostic.as_deref(),
            "datatype-param-diagnostic",
            EngineDiagnosticBehavior::DatatypeParam,
            &model.diagnostic_behaviors,
            &mut model.compile_diagnostics,
        );
    }
    validate_constraint_definitions(
        schema_uri,
        &model.constraints,
        &model.diagnostic_behaviors,
        &mut model.compile_diagnostics,
    );

    seen_schema_uris.remove(schema_uri);
    model
}

fn empty_document_model(schema_uri: &str) -> SchemaDocumentModel {
    SchemaDocumentModel {
        schema_uri: schema_uri.to_owned(),
        elements: BTreeMap::new(),
        attributes: BTreeMap::new(),
        behaviors: BTreeMap::new(),
        constraints: BTreeMap::new(),
        diagnostic_behaviors: BTreeMap::new(),
        compile_diagnostics: Vec::new(),
    }
}

fn compile_element_model(
    document: &CemDocument,
    node_id: AstNodeId,
    uses: &BTreeMap<String, String>,
    seen_schema_uris: &mut BTreeSet<String>,
) -> Option<ElementModel> {
    let attrs = collect_attrs(document, node_id);
    let name = attrs.get("name")?.trim().to_owned();
    if name.is_empty() {
        return None;
    }

    let mut element_model = attrs
        .get("base")
        .and_then(|base| resolve_base_element_model(base, uses, seen_schema_uris))
        .unwrap_or_default();
    element_model.name = name;

    if attrs.contains_key("required-attributes") {
        element_model.required_attributes = parse_name_set(attrs.get("required-attributes"));
    }
    if attrs.contains_key("optional-attributes") {
        element_model.optional_attributes = parse_name_set(attrs.get("optional-attributes"));
    }
    if attrs.contains_key("children") {
        let (child_elements, allow_any_child) = parse_child_set(attrs.get("children"));
        element_model.child_elements = child_elements;
        element_model.allow_any_child = allow_any_child;
    }

    Some(element_model)
}

fn collect_attribute_models(
    document: &CemDocument,
    schema_id: AstNodeId,
) -> BTreeMap<String, AttributeModel> {
    let mut attributes = BTreeMap::new();
    for attributes_id in element_child_ids_by_local_name(document, schema_id, "attributes") {
        let Some(CemAstNode::Element { children, .. }) = document.get(attributes_id) else {
            continue;
        };
        for child_id in children {
            if document.get(*child_id).and_then(element_local_name) != Some("attribute") {
                continue;
            }
            let attrs = collect_attrs(document, *child_id);
            let Some(name) = attrs.get("name").map(String::as_str).map(str::trim) else {
                continue;
            };
            if name.is_empty() {
                continue;
            }
            attributes.insert(
                name.to_owned(),
                AttributeModel {
                    name: name.to_owned(),
                    value_type: optional_non_empty_attr(&attrs, "type").map(str::to_owned),
                    allowed_values: parse_value_set(attrs.get("values")),
                    min_inclusive: optional_non_empty_attr(&attrs, "minInclusive")
                        .map(str::to_owned),
                    values_diagnostic: optional_non_empty_attr(&attrs, "values-diagnostic")
                        .map(str::to_owned),
                    type_diagnostic: optional_non_empty_attr(&attrs, "type-diagnostic")
                        .map(str::to_owned),
                    datatype_param_diagnostic: optional_non_empty_attr(
                        &attrs,
                        "datatype-param-diagnostic",
                    )
                    .map(str::to_owned),
                    source_map: document
                        .get(*child_id)
                        .map(source_stack_for_node)
                        .cloned()
                        .unwrap_or_default(),
                },
            );
        }
    }
    attributes
}

fn validate_attribute_diagnostic_reference(
    schema_uri: &str,
    attribute_model: &AttributeModel,
    code: Option<&str>,
    diagnostic_attribute: &str,
    expected_engine_behavior: EngineDiagnosticBehavior,
    diagnostic_behaviors: &BTreeMap<String, DiagnosticBehavior>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(code) = code else {
        return;
    };
    let Some(behavior) = diagnostic_behaviors.get(code) else {
        diagnostics.push(schema_compile_diagnostic(
            UNRESOLVED_DIAGNOSTIC_REFERENCE_CODE,
            format!(
                "attribute `{}` references diagnostic behavior `{code}` through `{diagnostic_attribute}`, but it is not declared by schema `{schema_uri}`",
                attribute_model.name
            ),
            &attribute_model.source_map,
            serde_json::json!({
                "schemaUri": schema_uri,
                "attribute": &attribute_model.name,
                "diagnostic": code,
                "diagnosticAttribute": diagnostic_attribute,
                "checkKind": "diagnostic-reference-resolution",
            }),
        ));
        return;
    };
    if behavior.engine_behavior != Some(expected_engine_behavior) {
        diagnostics.push(schema_compile_diagnostic(
            INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
            format!(
                "attribute `{}` references diagnostic `{code}` through `{diagnostic_attribute}`, but behavior `{}` is not compatible with `{}`",
                attribute_model.name,
                behavior.behavior,
                expected_engine_behavior.reference()
            ),
            &attribute_model.source_map,
            serde_json::json!({
                "schemaUri": schema_uri,
                "attribute": &attribute_model.name,
                "diagnostic": code,
                "diagnosticAttribute": diagnostic_attribute,
                "behavior": &behavior.behavior,
                "expectedBehavior": expected_engine_behavior.reference(),
                "checkKind": "diagnostic-behavior-contract",
            }),
        ));
    }
}

fn collect_constraint_definitions(
    document: &CemDocument,
    schema_id: AstNodeId,
    schema_uri: &str,
    uses: &BTreeMap<String, String>,
    local_behaviors: &BTreeMap<String, BehaviorDefinition>,
) -> BTreeMap<String, ConstraintDefinition> {
    let mut constraints = BTreeMap::new();
    for constraints_id in element_child_ids_by_local_name(document, schema_id, "constraints") {
        let Some(CemAstNode::Element { children, .. }) = document.get(constraints_id) else {
            continue;
        };
        for child_id in children {
            let Some(child) = document.get(*child_id) else {
                continue;
            };
            if element_local_name(child) != Some("constraint") {
                continue;
            }
            let attrs = collect_attrs(document, *child_id);
            let Some(kind) = optional_non_empty_attr(&attrs, "kind") else {
                continue;
            };
            let behavior = optional_non_empty_attr(&attrs, "behavior").map(str::to_owned);
            let definition = behavior.as_deref().and_then(|behavior| {
                resolve_behavior_definition(behavior, schema_uri, uses, local_behaviors)
            });
            let engine_behavior = definition.as_ref().and_then(supported_engine_behavior);
            constraints.insert(
                kind.to_owned(),
                ConstraintDefinition {
                    schema_uri: schema_uri.to_owned(),
                    kind: kind.to_owned(),
                    target: optional_non_empty_attr(&attrs, "target").map(str::to_owned),
                    value: optional_non_empty_attr(&attrs, "value").map(str::to_owned),
                    policy: optional_non_empty_attr(&attrs, "policy").map(str::to_owned),
                    diagnostic: optional_non_empty_attr(&attrs, "diagnostic").map(str::to_owned),
                    behavior,
                    definition,
                    engine_behavior,
                    source_map: source_stack_for_node(child).clone(),
                },
            );
        }
    }
    constraints
}

fn validate_constraint_definitions(
    schema_uri: &str,
    constraints: &BTreeMap<String, ConstraintDefinition>,
    diagnostic_behaviors: &BTreeMap<String, DiagnosticBehavior>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for constraint in constraints.values() {
        if let Some(code) = constraint.diagnostic.as_deref() {
            if !diagnostic_behaviors.contains_key(code) {
                diagnostics.push(schema_compile_diagnostic(
                    UNRESOLVED_DIAGNOSTIC_REFERENCE_CODE,
                    format!(
                        "constraint `{}` references diagnostic behavior `{code}` that is not declared by schema `{schema_uri}`",
                        constraint.kind
                    ),
                    &constraint.source_map,
                    serde_json::json!({
                        "schemaUri": schema_uri,
                        "constraint": &constraint.kind,
                        "diagnostic": code,
                        "checkKind": "diagnostic-reference-resolution",
                    }),
                ));
            }
        } else if constraint.behavior.is_some() {
            diagnostics.push(schema_compile_diagnostic(
                INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
                format!(
                    "constraint `{}` declares a behavior but does not declare the diagnostic code that receives the result",
                    constraint.kind
                ),
                &constraint.source_map,
                serde_json::json!({
                    "schemaUri": schema_uri,
                    "constraint": &constraint.kind,
                    "behavior": &constraint.behavior,
                    "checkKind": "constraint-behavior-contract",
                }),
            ));
        }

        let Some(behavior) = constraint.behavior.as_deref() else {
            continue;
        };
        if constraint.definition.is_none() || constraint.engine_behavior.is_none() {
            diagnostics.push(schema_compile_diagnostic(
                UNKNOWN_DIAGNOSTIC_BEHAVIOR_CODE,
                format!(
                    "constraint `{}` references unsupported engine behavior `{behavior}`",
                    constraint.kind
                ),
                &constraint.source_map,
                serde_json::json!({
                    "schemaUri": schema_uri,
                    "constraint": &constraint.kind,
                    "behavior": behavior,
                    "checkKind": "constraint-behavior-resolution",
                }),
            ));
        }
    }
}

fn collect_diagnostic_behaviors(
    document: &CemDocument,
    schema_id: AstNodeId,
    schema_uri: &str,
    uses: &BTreeMap<String, String>,
    local_behaviors: &BTreeMap<String, BehaviorDefinition>,
) -> (BTreeMap<String, DiagnosticBehavior>, Vec<Diagnostic>) {
    let mut behaviors = BTreeMap::new();
    let mut diagnostics = Vec::new();
    for diagnostics_id in element_child_ids_by_local_name(document, schema_id, "diagnostics") {
        let Some(CemAstNode::Element { children, .. }) = document.get(diagnostics_id) else {
            continue;
        };
        for child_id in children {
            let Some(child) = document.get(*child_id) else {
                continue;
            };
            if element_local_name(child) != Some("diagnostic") {
                continue;
            }
            let attrs = collect_attrs(document, *child_id);
            let Some(code) = optional_non_empty_attr(&attrs, "code") else {
                continue;
            };
            let Some(behavior) = optional_non_empty_attr(&attrs, "behavior") else {
                continue;
            };
            let severity = optional_non_empty_attr(&attrs, "severity")
                .and_then(parse_diagnostic_severity)
                .unwrap_or(Severity::Error);
            let source_map = source_stack_for_node(child).clone();
            let arguments = collect_diagnostic_arguments(document, *child_id);
            let behavior_definition =
                resolve_behavior_definition(behavior, schema_uri, uses, local_behaviors);
            let (engine_behavior, function, function_definition) =
                compile_diagnostic_behavior_binding(
                    schema_uri,
                    code,
                    behavior,
                    behavior_definition.as_ref(),
                    &arguments,
                    uses,
                    local_behaviors,
                    &source_map,
                    &mut diagnostics,
                );
            behaviors.insert(
                code.to_owned(),
                DiagnosticBehavior {
                    code: code.to_owned(),
                    severity,
                    behavior: behavior.to_owned(),
                    definition: behavior_definition,
                    engine_behavior,
                    function,
                    function_definition,
                    arguments,
                    message: optional_non_empty_attr(&attrs, "message").map(str::to_owned),
                    source_map,
                },
            );
        }
    }
    (behaviors, diagnostics)
}

fn collect_diagnostic_arguments(
    document: &CemDocument,
    diagnostic_id: AstNodeId,
) -> Vec<BehaviorArgument> {
    let mut arguments = Vec::new();
    for arguments_id in element_child_ids_by_local_name(document, diagnostic_id, "arguments") {
        for argument_id in element_child_ids_by_local_name(document, arguments_id, "argument") {
            let attrs = collect_attrs(document, argument_id);
            let Some(name) = optional_non_empty_attr(&attrs, "name") else {
                continue;
            };
            let Some(value) = optional_non_empty_attr(&attrs, "value") else {
                continue;
            };
            let source_map = document
                .get(argument_id)
                .map(source_stack_for_node)
                .cloned()
                .unwrap_or_default();
            arguments.push(BehaviorArgument {
                name: name.to_owned(),
                value: value.to_owned(),
                source_map,
            });
        }
    }
    arguments
}

fn collect_behavior_definitions(
    document: &CemDocument,
    schema_id: AstNodeId,
    schema_uri: &str,
    uses: &BTreeMap<String, String>,
) -> BTreeMap<String, BehaviorDefinition> {
    let mut behaviors = BTreeMap::new();
    for behaviors_id in element_child_ids_by_local_name(document, schema_id, "behaviors") {
        let Some(CemAstNode::Element { children, .. }) = document.get(behaviors_id) else {
            continue;
        };
        for child_id in children {
            let Some(child) = document.get(*child_id) else {
                continue;
            };
            if element_local_name(child) != Some("behavior") {
                continue;
            }
            let attrs = collect_attrs(document, *child_id);
            let Some(name) = optional_non_empty_attr(&attrs, "name") else {
                continue;
            };
            let Some(implementation) = optional_non_empty_attr(&attrs, "implementation") else {
                continue;
            };
            let Some(execution) = optional_non_empty_attr(&attrs, "execution") else {
                continue;
            };
            behaviors.insert(
                name.to_owned(),
                BehaviorDefinition {
                    schema_uri: schema_uri.to_owned(),
                    uses: uses.clone(),
                    name: name.to_owned(),
                    implementation: implementation.to_owned(),
                    execution: execution.to_owned(),
                    primitive: optional_non_empty_attr(&attrs, "primitive").map(str::to_owned),
                    function: optional_non_empty_attr(&attrs, "function").map(str::to_owned),
                    select: optional_non_empty_attr(&attrs, "select").map(str::to_owned),
                    match_query: optional_non_empty_attr(&attrs, "match").map(str::to_owned),
                    inputs: collect_behavior_inputs(document, *child_id),
                    parameters: collect_behavior_parameters(document, *child_id),
                    result: collect_behavior_result(document, *child_id),
                    inline_functions: collect_behavior_inline_functions(document, *child_id),
                    source_map: source_stack_for_node(child).clone(),
                },
            );
        }
    }
    behaviors
}

fn collect_behavior_inputs(document: &CemDocument, behavior_id: AstNodeId) -> Vec<BehaviorInput> {
    let mut inputs = Vec::new();
    for inputs_id in element_child_ids_by_local_name(document, behavior_id, "inputs") {
        let input_ids = element_child_ids_by_local_name(document, inputs_id, "input-binding")
            .into_iter()
            .chain(element_child_ids_by_local_name(
                document, inputs_id, "input",
            ));
        for input_id in input_ids {
            let attrs = collect_attrs(document, input_id);
            let Some(name) = optional_non_empty_attr(&attrs, "name") else {
                continue;
            };
            let Some(value_type) = optional_non_empty_attr(&attrs, "type") else {
                continue;
            };
            inputs.push(BehaviorInput {
                name: name.to_owned(),
                value_type: value_type.to_owned(),
                source: optional_non_empty_attr(&attrs, "source")
                    .unwrap_or(name)
                    .to_owned(),
                required: attr_is_true(attrs.get("required")),
                source_range: optional_non_empty_attr(&attrs, "source-range").map(str::to_owned),
            });
        }
    }
    inputs
}

fn collect_behavior_parameters(
    document: &CemDocument,
    behavior_id: AstNodeId,
) -> Vec<BehaviorParameter> {
    let mut parameters = Vec::new();
    for parameters_id in element_child_ids_by_local_name(document, behavior_id, "parameters") {
        for parameter_id in element_child_ids_by_local_name(document, parameters_id, "parameter") {
            let attrs = collect_attrs(document, parameter_id);
            let Some(name) = optional_non_empty_attr(&attrs, "name") else {
                continue;
            };
            let Some(value_type) = optional_non_empty_attr(&attrs, "type") else {
                continue;
            };
            parameters.push(BehaviorParameter {
                name: name.to_owned(),
                value_type: value_type.to_owned(),
                required: attr_is_true(attrs.get("required")),
                default: optional_non_empty_attr(&attrs, "default").map(str::to_owned),
                values: parse_value_set(attrs.get("values")),
            });
        }
    }
    parameters
}

fn collect_behavior_result(
    document: &CemDocument,
    behavior_id: AstNodeId,
) -> Option<BehaviorResult> {
    let result_id = element_child_ids_by_local_name(document, behavior_id, "result")
        .into_iter()
        .next()?;
    let attrs = collect_attrs(document, result_id);
    let value_type = optional_non_empty_attr(&attrs, "type")?.to_owned();
    Some(BehaviorResult {
        value_type,
        severity: optional_non_empty_attr(&attrs, "severity").map(str::to_owned),
        message: optional_non_empty_attr(&attrs, "message").map(str::to_owned),
        source_range: optional_non_empty_attr(&attrs, "source-range").map(str::to_owned),
        details: collect_behavior_details(document, result_id),
    })
}

fn collect_behavior_details(document: &CemDocument, result_id: AstNodeId) -> Vec<BehaviorDetail> {
    let mut details = Vec::new();
    for detail_id in element_child_ids_by_local_name(document, result_id, "detail") {
        let attrs = collect_attrs(document, detail_id);
        let Some(name) = optional_non_empty_attr(&attrs, "name") else {
            continue;
        };
        let Some(value_type) = optional_non_empty_attr(&attrs, "type") else {
            continue;
        };
        details.push(BehaviorDetail {
            name: name.to_owned(),
            value_type: value_type.to_owned(),
            required: attr_is_true(attrs.get("required")),
            source: optional_non_empty_attr(&attrs, "source").map(str::to_owned),
        });
    }
    details
}

fn collect_behavior_inline_functions(
    document: &CemDocument,
    behavior_id: AstNodeId,
) -> BTreeMap<String, BehaviorFunctionDeclaration> {
    let mut functions = BTreeMap::new();
    for function_id in element_child_ids_by_local_name(document, behavior_id, "function") {
        let attrs = collect_attrs(document, function_id);
        let Some(name) = optional_non_empty_attr(&attrs, "name") else {
            continue;
        };
        let Some(returns) = optional_non_empty_attr(&attrs, "returns") else {
            continue;
        };
        functions.insert(
            name.to_owned(),
            BehaviorFunctionDeclaration {
                name: name.to_owned(),
                returns: returns.to_owned(),
                visibility: optional_non_empty_attr(&attrs, "visibility")
                    .unwrap_or("private")
                    .to_owned(),
                params: collect_behavior_function_params(document, function_id),
                body_expression: collect_behavior_function_body_expression(document, function_id),
            },
        );
    }
    functions
}

fn collect_behavior_function_body_expression(
    document: &CemDocument,
    function_id: AstNodeId,
) -> Option<String> {
    let mut expressions = Vec::new();
    for body_id in element_child_ids_by_local_name(document, function_id, "body") {
        collect_behavior_runtime_body_expressions(document, body_id, &mut expressions);
    }
    expressions.into_iter().next()
}

fn collect_behavior_runtime_body_expressions(
    document: &CemDocument,
    node_id: AstNodeId,
    expressions: &mut Vec<String>,
) {
    let Some(node @ CemAstNode::Element { children, .. }) = document.get(node_id) else {
        return;
    };
    if element_local_name(node) == Some("$") {
        if let Some(expression) = expression_body(document, node_id) {
            expressions.push(expression);
        }
        return;
    }
    for child in children {
        collect_behavior_runtime_body_expressions(document, *child, expressions);
    }
}

fn expression_body(document: &CemDocument, node_id: AstNodeId) -> Option<String> {
    let Some(CemAstNode::Element { children, .. }) = document.get(node_id) else {
        return None;
    };
    let mut body = String::new();
    for child in children {
        match document.get(*child) {
            Some(CemAstNode::Text { data, .. }) | Some(CemAstNode::RawText { data, .. }) => {
                body.push_str(data);
            }
            _ => {}
        }
    }
    Some(body.trim().to_owned()).filter(|body| !body.is_empty())
}

fn collect_behavior_function_params(
    document: &CemDocument,
    function_id: AstNodeId,
) -> Vec<BehaviorFunctionParam> {
    let mut params = Vec::new();
    for param_id in element_child_ids_by_local_name(document, function_id, "param") {
        let attrs = collect_attrs(document, param_id);
        let Some(name) = optional_non_empty_attr(&attrs, "name") else {
            continue;
        };
        let Some(value_type) = optional_non_empty_attr(&attrs, "type") else {
            continue;
        };
        params.push(BehaviorFunctionParam {
            name: name.to_owned(),
            value_type: value_type.to_owned(),
            required: attr_is_true(attrs.get("required")),
        });
    }
    params
}

fn compile_diagnostic_behavior_binding(
    schema_uri: &str,
    code: &str,
    behavior_reference: &str,
    behavior_definition: Option<&BehaviorDefinition>,
    arguments: &[BehaviorArgument],
    uses: &BTreeMap<String, String>,
    local_behaviors: &BTreeMap<String, BehaviorDefinition>,
    source_map: &SourceMapStack,
    diagnostics: &mut Vec<Diagnostic>,
) -> (
    Option<EngineDiagnosticBehavior>,
    Option<String>,
    Option<BehaviorFunctionDeclaration>,
) {
    let Some(behavior_definition) = behavior_definition else {
        diagnostics.push(schema_compile_diagnostic(
            UNKNOWN_DIAGNOSTIC_BEHAVIOR_CODE,
            format!("diagnostic `{code}` references unknown behavior `{behavior_reference}`"),
            source_map,
            serde_json::json!({
                "schemaUri": schema_uri,
                "diagnostic": code,
                "behavior": behavior_reference,
                "checkKind": "diagnostic-behavior-resolution",
            }),
        ));
        return (None, None, None);
    };

    match behavior_definition.implementation.as_str() {
        "engine" => {
            if !arguments.is_empty() {
                diagnostics.push(schema_compile_diagnostic(
                    INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
                    format!(
                        "diagnostic `{code}` declares behavior arguments for engine behavior `{behavior_reference}`, but diagnostic arguments are supported only for function behaviors"
                    ),
                    source_map,
                    serde_json::json!({
                        "schemaUri": schema_uri,
                        "diagnostic": code,
                        "behavior": behavior_reference,
                        "checkKind": "behavior-argument-binding",
                    }),
                ));
                return (None, None, None);
            }
            let engine_behavior = supported_engine_behavior(behavior_definition);
            if engine_behavior.is_none() {
                diagnostics.push(schema_compile_diagnostic(
                    UNKNOWN_DIAGNOSTIC_BEHAVIOR_CODE,
                    format!(
                        "diagnostic `{code}` references unsupported engine behavior `{behavior_reference}`"
                    ),
                    source_map,
                    serde_json::json!({
                        "schemaUri": schema_uri,
                        "diagnostic": code,
                        "behavior": behavior_reference,
                        "primitive": &behavior_definition.primitive,
                        "checkKind": "diagnostic-behavior-resolution",
                    }),
                ));
            }
            (engine_behavior, None, None)
        }
        "function" => compile_diagnostic_function_binding(
            schema_uri,
            code,
            behavior_reference,
            behavior_definition,
            arguments,
            uses,
            local_behaviors,
            source_map,
            diagnostics,
        ),
        implementation => {
            diagnostics.push(schema_compile_diagnostic(
                INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
                format!(
                    "diagnostic `{code}` references behavior `{behavior_reference}` with unsupported implementation `{implementation}`"
                ),
                source_map,
                serde_json::json!({
                    "schemaUri": schema_uri,
                    "diagnostic": code,
                    "behavior": behavior_reference,
                    "implementation": implementation,
                    "checkKind": "diagnostic-behavior-contract",
                }),
            ));
            (None, None, None)
        }
    }
}

fn compile_diagnostic_function_binding(
    schema_uri: &str,
    code: &str,
    behavior_reference: &str,
    behavior_definition: &BehaviorDefinition,
    arguments: &[BehaviorArgument],
    uses: &BTreeMap<String, String>,
    local_behaviors: &BTreeMap<String, BehaviorDefinition>,
    source_map: &SourceMapStack,
    diagnostics: &mut Vec<Diagnostic>,
) -> (
    Option<EngineDiagnosticBehavior>,
    Option<String>,
    Option<BehaviorFunctionDeclaration>,
) {
    if behavior_definition.execution != "ast-validation" {
        diagnostics.push(schema_compile_diagnostic(
            INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
            format!(
                "diagnostic `{code}` references function behavior `{behavior_reference}` with unsupported execution placement `{}`",
                behavior_definition.execution
            ),
            source_map,
            serde_json::json!({
                "schemaUri": schema_uri,
                "diagnostic": code,
                "behavior": behavior_reference,
                "execution": &behavior_definition.execution,
                "checkKind": "diagnostic-behavior-contract",
            }),
        ));
        return (None, None, None);
    }

    if behavior_definition.select.is_none() {
        diagnostics.push(schema_compile_diagnostic(
            INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
            format!(
                "diagnostic `{code}` references function behavior `{behavior_reference}` without a declarative CEM-QL select expression"
            ),
            source_map,
            serde_json::json!({
                "schemaUri": schema_uri,
                "diagnostic": code,
                "behavior": behavior_reference,
                "checkKind": "diagnostic-behavior-contract",
            }),
        ));
        return (None, None, None);
    }

    if behavior_definition.match_query.is_none() {
        diagnostics.push(schema_compile_diagnostic(
            INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
            format!(
                "diagnostic `{code}` references function behavior `{behavior_reference}` without a declarative CEM-QL match expression"
            ),
            source_map,
            serde_json::json!({
                "schemaUri": schema_uri,
                "diagnostic": code,
                "behavior": behavior_reference,
                "checkKind": "diagnostic-behavior-contract",
            }),
        ));
        return (None, None, None);
    }

    if !validate_diagnostic_behavior_arguments(
        schema_uri,
        code,
        behavior_reference,
        behavior_definition,
        arguments,
        diagnostics,
    ) {
        return (None, None, None);
    }

    let Some(function) = behavior_definition.function.as_deref() else {
        diagnostics.push(schema_compile_diagnostic(
            INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
            format!(
                "diagnostic `{code}` references function behavior `{behavior_reference}` without a schema function binding"
            ),
            source_map,
            serde_json::json!({
                "schemaUri": schema_uri,
                "diagnostic": code,
                "behavior": behavior_reference,
                "checkKind": "diagnostic-behavior-contract",
            }),
        ));
        return (None, None, None);
    };

    let Some(result) = behavior_definition.result.as_ref() else {
        diagnostics.push(schema_compile_diagnostic(
            INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
            format!(
                "diagnostic `{code}` references function behavior `{behavior_reference}` without a diagnostic result declaration"
            ),
            source_map,
            serde_json::json!({
                "schemaUri": schema_uri,
                "diagnostic": code,
                "behavior": behavior_reference,
                "function": function,
                "checkKind": "diagnostic-behavior-contract",
            }),
        ));
        return (None, None, None);
    };
    if result.value_type != "schema:diagnostic-result" {
        diagnostics.push(schema_compile_diagnostic(
            INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
            format!(
                "diagnostic `{code}` references function behavior `{behavior_reference}` whose result type is `{}` instead of `schema:diagnostic-result`",
                result.value_type
            ),
            source_map,
            serde_json::json!({
                "schemaUri": schema_uri,
                "diagnostic": code,
                "behavior": behavior_reference,
                "function": function,
                "resultType": &result.value_type,
                "checkKind": "diagnostic-behavior-contract",
            }),
        ));
        return (None, None, None);
    }

    let function_declaration = match resolve_behavior_function_declaration(
        function,
        schema_uri,
        uses,
        local_behaviors,
        behavior_definition,
    ) {
        Ok(Some(function_declaration)) => function_declaration,
        Ok(None) => {
            diagnostics.push(schema_compile_diagnostic(
                UNRESOLVED_BEHAVIOR_FUNCTION_CODE,
                format!(
                    "diagnostic `{code}` references behavior `{behavior_reference}` function `{function}` that is not declared by the schema behavior or a visible reusable schema function"
                ),
                source_map,
                serde_json::json!({
                    "schemaUri": schema_uri,
                    "diagnostic": code,
                    "behavior": behavior_reference,
                    "function": function,
                    "checkKind": "behavior-function-resolution",
                }),
            ));
            return (None, None, None);
        }
        Err(message) => {
            diagnostics.push(schema_compile_diagnostic(
                INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
                format!(
                    "diagnostic `{code}` references behavior `{behavior_reference}` function `{function}` that cannot be resolved: {message}"
                ),
                source_map,
                serde_json::json!({
                    "schemaUri": schema_uri,
                    "diagnostic": code,
                    "behavior": behavior_reference,
                    "function": function,
                    "checkKind": "behavior-function-resolution",
                }),
            ));
            return (None, None, None);
        }
    };
    if function_declaration.returns != "object" {
        diagnostics.push(schema_compile_diagnostic(
            INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
            format!(
                "diagnostic `{code}` references behavior `{behavior_reference}` function `{function}` that returns `{}` instead of `object`",
                function_declaration.returns
            ),
            source_map,
            serde_json::json!({
                "schemaUri": schema_uri,
                "diagnostic": code,
                "behavior": behavior_reference,
                "function": function,
                "returns": &function_declaration.returns,
                "checkKind": "diagnostic-behavior-contract",
            }),
        ));
        return (None, None, None);
    }

    if function_declaration.body_expression.is_none() {
        diagnostics.push(schema_compile_diagnostic(
            INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
            format!(
                "diagnostic `{code}` references behavior `{behavior_reference}` function `{function}` without an executable CEM-ML behavior body expression"
            ),
            source_map,
            serde_json::json!({
                "schemaUri": schema_uri,
                "diagnostic": code,
                "behavior": behavior_reference,
                "function": function,
                "checkKind": "diagnostic-behavior-contract",
            }),
        ));
        return (None, None, None);
    }

    if let Some(unbound_param) = function_declaration.params.iter().find(|param| {
        param.required
            && !behavior_function_param_has_binding(behavior_definition, arguments, &param.name)
    }) {
        diagnostics.push(schema_compile_diagnostic(
            INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
            format!(
                "diagnostic `{code}` references behavior `{behavior_reference}` function `{function}` with required parameter `{}` that has no behavior input, diagnostic argument, or defaulted behavior parameter binding",
                unbound_param.name
            ),
            source_map,
            serde_json::json!({
                "schemaUri": schema_uri,
                "diagnostic": code,
                "behavior": behavior_reference,
                "function": function,
                "parameter": &unbound_param.name,
                "checkKind": "behavior-function-parameter-binding",
            }),
        ));
        return (None, None, None);
    }

    (None, Some(function.to_owned()), Some(function_declaration))
}

fn behavior_function_param_has_binding(
    behavior_definition: &BehaviorDefinition,
    arguments: &[BehaviorArgument],
    param_name: &str,
) -> bool {
    behavior_definition
        .inputs
        .iter()
        .any(|input| input.name == param_name)
        || arguments.iter().any(|argument| argument.name == param_name)
        || behavior_definition
            .parameters
            .iter()
            .any(|parameter| parameter.name == param_name && parameter.default.is_some())
}

fn validate_diagnostic_behavior_arguments(
    schema_uri: &str,
    code: &str,
    behavior_reference: &str,
    behavior_definition: &BehaviorDefinition,
    arguments: &[BehaviorArgument],
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let mut valid = true;
    let mut seen = BTreeSet::new();
    for argument in arguments {
        if !seen.insert(argument.name.clone()) {
            diagnostics.push(schema_compile_diagnostic(
                INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
                format!(
                    "diagnostic `{code}` repeats behavior argument `{}` for behavior `{behavior_reference}`",
                    argument.name
                ),
                &argument.source_map,
                serde_json::json!({
                    "schemaUri": schema_uri,
                    "diagnostic": code,
                    "behavior": behavior_reference,
                    "argument": &argument.name,
                    "checkKind": "behavior-argument-binding",
                }),
            ));
            valid = false;
            continue;
        }
        let Some(parameter) = behavior_definition
            .parameters
            .iter()
            .find(|parameter| parameter.name == argument.name)
        else {
            diagnostics.push(schema_compile_diagnostic(
                INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
                format!(
                    "diagnostic `{code}` overrides behavior argument `{}` that is not declared as a parameter by behavior `{behavior_reference}`",
                    argument.name
                ),
                &argument.source_map,
                serde_json::json!({
                    "schemaUri": schema_uri,
                    "diagnostic": code,
                    "behavior": behavior_reference,
                    "argument": &argument.name,
                    "checkKind": "behavior-argument-binding",
                }),
            ));
            valid = false;
            continue;
        };
        if let Err(message) = validate_behavior_parameter_raw_value(parameter, &argument.value) {
            diagnostics.push(schema_compile_diagnostic(
                INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
                format!(
                    "diagnostic `{code}` behavior `{behavior_reference}` argument `{}` is invalid: {message}",
                    argument.name
                ),
                &argument.source_map,
                serde_json::json!({
                    "schemaUri": schema_uri,
                    "diagnostic": code,
                    "behavior": behavior_reference,
                    "argument": &argument.name,
                    "argumentType": &parameter.value_type,
                    "checkKind": "behavior-argument-binding",
                }),
            ));
            valid = false;
        }
    }
    valid
}

fn validate_behavior_parameter_raw_value(
    parameter: &BehaviorParameter,
    raw_value: &str,
) -> Result<(), String> {
    if !parameter.values.is_empty() && !parameter.values.contains(raw_value) {
        return Err(format!(
            "value `{raw_value}` is outside declared values `{}`",
            parameter
                .values
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    match schema_behavior_parameter_value_type(&parameter.value_type) {
        SchemaBehaviorParameterValueType::Boolean => match raw_value.trim() {
            "true" | "false" => Ok(()),
            other => Err(format!("expected boolean value, got `{other}`")),
        },
        SchemaBehaviorParameterValueType::Integer => raw_value
            .trim()
            .parse::<i64>()
            .map(|_| ())
            .map_err(|_| format!("expected integer value, got `{raw_value}`")),
        SchemaBehaviorParameterValueType::Number => {
            let value = raw_value
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("expected number value, got `{raw_value}`"))?;
            if value.is_finite() {
                Ok(())
            } else {
                Err(format!("expected finite number value, got `{raw_value}`"))
            }
        }
        SchemaBehaviorParameterValueType::Array
        | SchemaBehaviorParameterValueType::Object
        | SchemaBehaviorParameterValueType::Json => {
            let value = serde_json::from_str::<serde_json::Value>(raw_value)
                .map_err(|err| format!("value is not valid JSON: {err}"))?;
            let expected = schema_behavior_parameter_value_type(&parameter.value_type);
            if expected.accepts(&value) {
                Ok(())
            } else {
                Err(format!(
                    "expected {} value, got {}",
                    expected.as_contract_name(),
                    json_value_kind(&value)
                ))
            }
        }
        SchemaBehaviorParameterValueType::Any | SchemaBehaviorParameterValueType::String => Ok(()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchemaBehaviorParameterValueType {
    Any,
    String,
    Boolean,
    Number,
    Integer,
    Array,
    Object,
    Json,
}

impl SchemaBehaviorParameterValueType {
    fn accepts(self, value: &serde_json::Value) -> bool {
        match self {
            Self::Any | Self::Json => true,
            Self::String => value.is_string(),
            Self::Boolean => value.is_boolean(),
            Self::Number => value.is_number(),
            Self::Integer => value.as_i64().is_some() || value.as_u64().is_some(),
            Self::Array => value.is_array(),
            Self::Object => value.is_object(),
        }
    }

    fn as_contract_name(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::String => "string",
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::Integer => "integer",
            Self::Array => "array",
            Self::Object => "object",
            Self::Json => "json",
        }
    }
}

fn schema_behavior_parameter_value_type(value: &str) -> SchemaBehaviorParameterValueType {
    match value
        .trim()
        .rsplit_once(':')
        .map(|(_, local)| local)
        .unwrap_or_else(|| value.trim())
    {
        "string" | "identifier" | "diagnostic-code" | "behavior-reference" => {
            SchemaBehaviorParameterValueType::String
        }
        "boolean" => SchemaBehaviorParameterValueType::Boolean,
        "number" => SchemaBehaviorParameterValueType::Number,
        "integer" => SchemaBehaviorParameterValueType::Integer,
        "array" => SchemaBehaviorParameterValueType::Array,
        "object" | "node" | "diagnostic" | "diagnostic-result" => {
            SchemaBehaviorParameterValueType::Object
        }
        "json" => SchemaBehaviorParameterValueType::Json,
        _ => SchemaBehaviorParameterValueType::Any,
    }
}

fn json_value_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn resolve_behavior_function_declaration(
    function_reference: &str,
    current_schema_uri: &str,
    current_uses: &BTreeMap<String, String>,
    current_local_behaviors: &BTreeMap<String, BehaviorDefinition>,
    behavior_definition: &BehaviorDefinition,
) -> Result<Option<BehaviorFunctionDeclaration>, String> {
    let function_reference = function_reference.trim();
    if function_reference.is_empty() {
        return Ok(None);
    }

    let Some((alias, name)) = function_reference.split_once(':') else {
        if let Some(function) = behavior_definition.inline_functions.get(function_reference) {
            return Ok(Some(function.clone()));
        }
        let behavior_definitions = behavior_definitions_for_function_schema(
            &behavior_definition.schema_uri,
            current_schema_uri,
            current_local_behaviors,
        )?;
        return reusable_behavior_function_from_definitions(
            function_reference,
            &behavior_definitions,
            false,
        );
    };

    let alias = alias.trim();
    let name = name.trim();
    if alias.is_empty() || name.is_empty() {
        return Ok(None);
    }
    let referenced_schema_uri = behavior_definition
        .uses
        .get(alias)
        .or_else(|| current_uses.get(alias))
        .map(String::as_str)
        .map(str::trim)
        .filter(|schema_uri| !schema_uri.is_empty());
    let Some(referenced_schema_uri) = referenced_schema_uri else {
        return Ok(None);
    };
    let behavior_definitions = behavior_definitions_for_function_schema(
        referenced_schema_uri,
        current_schema_uri,
        current_local_behaviors,
    )?;
    reusable_behavior_function_from_definitions(
        name,
        &behavior_definitions,
        referenced_schema_uri != behavior_definition.schema_uri,
    )
}

fn behavior_definitions_for_function_schema(
    schema_uri: &str,
    current_schema_uri: &str,
    current_local_behaviors: &BTreeMap<String, BehaviorDefinition>,
) -> Result<BTreeMap<String, BehaviorDefinition>, String> {
    if schema_uri == current_schema_uri {
        return Ok(current_local_behaviors.clone());
    }
    let package = load_builtin_schema_package(schema_uri)
        .map_err(|_| format!("schema `{schema_uri}` is not available as a built-in package"))?;
    let document = parse_cem_document(package.schema_source);
    let schema_id = first_element_id_by_local_name(&document, "schema")
        .ok_or_else(|| format!("schema `{schema_uri}` has no root schema declaration"))?;
    let uses = collect_schema_uses(&document, schema_id);
    Ok(collect_behavior_definitions(
        &document, schema_id, schema_uri, &uses,
    ))
}

fn reusable_behavior_function_from_definitions(
    name: &str,
    behavior_definitions: &BTreeMap<String, BehaviorDefinition>,
    external_schema: bool,
) -> Result<Option<BehaviorFunctionDeclaration>, String> {
    let mut matches = Vec::new();
    for behavior in behavior_definitions.values() {
        let Some(function) = behavior.inline_functions.get(name) else {
            continue;
        };
        if !behavior_function_visible_for_reuse(function, external_schema) {
            continue;
        }
        matches.push((behavior.name.as_str(), function.clone()));
    }
    if matches.len() > 1 {
        return Err(format!(
            "function `{name}` is ambiguous across reusable behavior functions: {}",
            matches
                .iter()
                .map(|(behavior, _)| *behavior)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    Ok(matches.into_iter().next().map(|(_, function)| function))
}

fn behavior_function_visible_for_reuse(
    function: &BehaviorFunctionDeclaration,
    external_schema: bool,
) -> bool {
    if external_schema {
        function.visibility == "public"
    } else {
        function.visibility == "package" || function.visibility == "public"
    }
}

fn resolve_behavior_definition(
    reference: &str,
    schema_uri: &str,
    uses: &BTreeMap<String, String>,
    local_behaviors: &BTreeMap<String, BehaviorDefinition>,
) -> Option<BehaviorDefinition> {
    let reference = reference.trim();
    let Some((alias, name)) = reference.split_once(':') else {
        return local_behaviors.get(reference).cloned();
    };
    let referenced_schema_uri = uses.get(alias.trim())?.trim();
    let name = name.trim();
    if referenced_schema_uri.is_empty() || name.is_empty() {
        return None;
    }
    if referenced_schema_uri == schema_uri {
        return local_behaviors.get(name).cloned();
    }
    let package = load_builtin_schema_package(referenced_schema_uri).ok()?;
    let document = parse_cem_document(package.schema_source);
    let schema_id = first_element_id_by_local_name(&document, "schema")?;
    let uses = collect_schema_uses(&document, schema_id);
    collect_behavior_definitions(&document, schema_id, referenced_schema_uri, &uses)
        .get(name)
        .cloned()
}

fn supported_engine_behavior(behavior: &BehaviorDefinition) -> Option<EngineDiagnosticBehavior> {
    let primitive = behavior.primitive.as_deref().or_else(|| {
        (behavior.schema_uri == CEM_SCHEMA_URI && behavior.name == "field-contract")
            .then_some(FIELD_CONTRACT_DIAGNOSTIC_BEHAVIOR)
    })?;
    if behavior.implementation != "engine" || behavior.execution != "ast-validation" {
        return None;
    }
    match primitive {
        FIELD_CONTRACT_DIAGNOSTIC_BEHAVIOR => Some(EngineDiagnosticBehavior::FieldContract),
        VALUE_VOCABULARY_DIAGNOSTIC_BEHAVIOR => Some(EngineDiagnosticBehavior::ValueVocabulary),
        SCALAR_TYPE_DIAGNOSTIC_BEHAVIOR => Some(EngineDiagnosticBehavior::ScalarType),
        DATATYPE_PARAM_DIAGNOSTIC_BEHAVIOR => Some(EngineDiagnosticBehavior::DatatypeParam),
        RESOURCE_READABLE_DIAGNOSTIC_BEHAVIOR => Some(EngineDiagnosticBehavior::ResourceReadable),
        RESOURCE_PARSE_DIAGNOSTIC_BEHAVIOR => Some(EngineDiagnosticBehavior::ResourceParse),
        REFERENCE_RESOLUTION_DIAGNOSTIC_BEHAVIOR => {
            Some(EngineDiagnosticBehavior::ReferenceResolution)
        }
        _ => None,
    }
}

fn parse_diagnostic_severity(value: &str) -> Option<Severity> {
    match value.trim() {
        "info" => Some(Severity::Info),
        "warning" => Some(Severity::Warning),
        "error" => Some(Severity::Error),
        "fatal" => Some(Severity::Fatal),
        _ => None,
    }
}

fn collect_field_contracts(document: &CemDocument, schema_id: AstNodeId) -> Vec<FieldContract> {
    let mut contracts = Vec::new();
    for contracts_id in element_child_ids_by_local_name(document, schema_id, "field-contracts") {
        let Some(CemAstNode::Element { children, .. }) = document.get(contracts_id) else {
            continue;
        };
        for child_id in children {
            if document.get(*child_id).and_then(element_local_name) != Some("field-contract") {
                continue;
            }
            let attrs = collect_attrs(document, *child_id);
            let Some(name) = attrs.get("name").map(String::as_str).map(str::trim) else {
                continue;
            };
            let Some(target) = attrs.get("target").map(String::as_str).map(str::trim) else {
                continue;
            };
            if name.is_empty() || target.is_empty() {
                continue;
            }
            contracts.push(FieldContract {
                name: name.to_owned(),
                target: target.to_owned(),
                diagnostic: optional_non_empty_attr(&attrs, "diagnostic").map(str::to_owned),
                check_kind: optional_non_empty_attr(&attrs, "check-kind").map(str::to_owned),
                required_attributes: parse_name_set(attrs.get("required-attributes")),
                optional_attributes: parse_name_set(attrs.get("optional-attributes")),
                forbidden_attributes: parse_name_set(attrs.get("forbidden-attributes")),
                forbidden_attribute_values: parse_name_value_set(
                    attrs.get("forbidden-attribute-values"),
                ),
                required_children: parse_name_set(attrs.get("required-children")),
                max_one_children: parse_name_set(attrs.get("max-one-children")),
                path_layout_attributes: parse_name_set(attrs.get("path-layout-attributes")),
                path_layout_prefix: optional_non_empty_attr(&attrs, "path-layout-prefix")
                    .map(str::to_owned),
                path_layout_extension: optional_non_empty_attr(&attrs, "path-layout-extension")
                    .map(str::to_owned),
                when_attribute: optional_non_empty_attr(&attrs, "when-attribute")
                    .map(str::to_owned),
                when_values: parse_name_set(attrs.get("when-values")),
                when_present_attributes: parse_name_set(attrs.get("when-present-attributes")),
                source_map: document
                    .get(*child_id)
                    .map(source_stack_for_node)
                    .cloned()
                    .unwrap_or_default(),
            });
        }
    }
    contracts
}

fn validate_field_contracts(
    schema_uri: &str,
    diagnostic_behaviors: &BTreeMap<String, DiagnosticBehavior>,
    element_name: &str,
    element_model: &ElementModel,
    seen_attributes: &BTreeSet<String>,
    attribute_values: &BTreeMap<String, String>,
    child_counts: &BTreeMap<String, usize>,
    node: &CemAstNode,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for contract in &element_model.field_contracts {
        let diagnostic_behavior = match contract.diagnostic.as_deref() {
            Some(code) => {
                let Some(behavior) = diagnostic_behaviors.get(code) else {
                    continue;
                };
                if behavior.engine_behavior != Some(EngineDiagnosticBehavior::FieldContract) {
                    continue;
                }
                Some(behavior)
            }
            None => None,
        };
        if !contract.applies_to(attribute_values) {
            continue;
        }
        let missing = contract
            .required_attributes
            .iter()
            .filter(|name| !seen_attributes.contains(*name))
            .cloned()
            .collect::<Vec<_>>();
        let forbidden = contract
            .forbidden_attributes
            .iter()
            .filter(|name| seen_attributes.contains(*name))
            .cloned()
            .collect::<Vec<_>>();
        let invalid_values = contract
            .forbidden_attribute_values
            .iter()
            .filter_map(|(name, values)| {
                let value = attribute_values
                    .get(name)
                    .map(String::as_str)
                    .map(str::trim)?;
                values
                    .contains(value)
                    .then(|| (name.clone(), value.to_owned()))
            })
            .collect::<BTreeMap<_, _>>();
        let invalid_path_values = contract
            .path_layout_attributes
            .iter()
            .filter_map(|name| {
                let value = attribute_values.get(name).map(String::as_str)?.trim();
                (!path_layout_is_valid(value, contract)).then(|| (name.clone(), value.to_owned()))
            })
            .collect::<BTreeMap<_, _>>();
        let invalid_values = invalid_values
            .into_iter()
            .chain(invalid_path_values)
            .collect::<BTreeMap<_, _>>();
        let invalid_fields = forbidden
            .iter()
            .cloned()
            .chain(invalid_values.keys().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let missing_children = contract
            .required_children
            .iter()
            .filter(|name| child_counts.get(*name).copied().unwrap_or_default() == 0)
            .cloned()
            .collect::<Vec<_>>();
        let duplicate_children = contract
            .max_one_children
            .iter()
            .filter(|name| child_counts.get(*name).copied().unwrap_or_default() > 1)
            .cloned()
            .collect::<Vec<_>>();

        if missing.is_empty()
            && invalid_fields.is_empty()
            && missing_children.is_empty()
            && duplicate_children.is_empty()
        {
            continue;
        }

        let mut parts = Vec::new();
        if !missing.is_empty() {
            parts.push(format!("missing fields: {}", missing.join(", ")));
        }
        if !forbidden.is_empty() {
            parts.push(format!(
                "forbidden fields present: {}",
                forbidden.join(", ")
            ));
        }
        if !invalid_values.is_empty() {
            parts.push(format!(
                "invalid values present: {}",
                invalid_values
                    .iter()
                    .map(|(name, value)| format!("{name}={value}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !missing_children.is_empty() {
            parts.push(format!("missing children: {}", missing_children.join(", ")));
        }
        if !duplicate_children.is_empty() {
            parts.push(format!(
                "duplicate children: {}",
                duplicate_children.join(", ")
            ));
        }
        let generated_message = format!(
            "element `{element_name}` failed field contract `{}` ({}) with {}",
            contract.name,
            contract.check_kind(),
            parts.join("; ")
        );
        let message = diagnostic_behavior
            .and_then(|behavior| behavior.message.as_deref())
            .map(|message| format!("{message}: {generated_message}"))
            .unwrap_or(generated_message);
        diagnostics.push(diag_at_with_details_and_severity(
            contract.diagnostic_code(),
            diagnostic_behavior
                .map(|behavior| behavior.severity)
                .unwrap_or(Severity::Error),
            message,
            node,
            field_contract_details(
                schema_uri,
                element_name,
                contract,
                diagnostic_behavior
                    .map(|behavior| behavior.behavior.as_str())
                    .unwrap_or(FIELD_CONTRACT_DIAGNOSTIC_BEHAVIOR),
                attribute_values,
                &missing,
                &invalid_fields,
                &invalid_values,
                &missing_children,
                &duplicate_children,
                child_counts,
                node,
            ),
        ));
    }
}

fn field_contract_details(
    schema_uri: &str,
    element_name: &str,
    contract: &FieldContract,
    behavior: &str,
    attribute_values: &BTreeMap<String, String>,
    missing_fields: &[String],
    invalid_fields: &[String],
    invalid_values: &BTreeMap<String, String>,
    missing_children: &[String],
    duplicate_children: &[String],
    child_counts: &BTreeMap<String, usize>,
    node: &CemAstNode,
) -> serde_json::Value {
    serde_json::json!({
        "schemaUri": schema_uri,
        "element": element_name,
        "contract": &contract.name,
        "target": &contract.target,
        "diagnostic": contract.diagnostic_code(),
        "behavior": behavior,
        "checkKind": contract.check_kind(),
        "requiredFields": &contract.required_attributes,
        "optionalFields": &contract.optional_attributes,
        "forbiddenFields": &contract.forbidden_attributes,
        "forbiddenAttributeValues": &contract.forbidden_attribute_values,
        "requiredChildren": &contract.required_children,
        "maxOneChildren": &contract.max_one_children,
        "pathLayout": {
            "attributes": &contract.path_layout_attributes,
            "prefix": &contract.path_layout_prefix,
            "extension": &contract.path_layout_extension,
            "relative": true,
            "cleanSegments": true,
        },
        "missingFields": missing_fields,
        "invalidFields": invalid_fields,
        "invalidValues": invalid_values,
        "missingChildren": missing_children,
        "duplicateChildren": duplicate_children,
        "childCounts": child_counts,
        "actualValues": attribute_values,
        "condition": {
            "attribute": &contract.when_attribute,
            "values": &contract.when_values,
            "presentAttributes": &contract.when_present_attributes,
        },
        "sourceRange": node_source_range_details(node),
    })
}

fn attribute_value_details(
    schema_uri: &str,
    element_name: &str,
    attribute_name: &str,
    actual_value: &str,
    attribute_model: &AttributeModel,
    diagnostic_code: &str,
    behavior: &str,
    attribute_values: &BTreeMap<String, String>,
    node: &CemAstNode,
) -> serde_json::Value {
    serde_json::json!({
        "schemaUri": schema_uri,
        "element": element_name,
        "attribute": attribute_name,
        "contract": format!("attribute-values:{attribute_name}"),
        "diagnostic": diagnostic_code,
        "behavior": behavior,
        "checkKind": "value-vocabulary",
        "valueType": &attribute_model.value_type,
        "expectedValues": &attribute_model.allowed_values,
        "actualValue": actual_value,
        "requiredFields": [],
        "optionalFields": [],
        "forbiddenFields": [],
        "missingFields": [],
        "invalidFields": [attribute_name],
        "actualValues": attribute_values,
        "sourceRange": node_source_range_details(node),
    })
}

fn attribute_type_details(
    schema_uri: &str,
    element_name: &str,
    attribute_name: &str,
    actual_value: &str,
    attribute_model: &AttributeModel,
    violation: AttributeTypeViolation,
    diagnostic_code: &str,
    behavior: &str,
    attribute_values: &BTreeMap<String, String>,
    node: &CemAstNode,
) -> serde_json::Value {
    let expected_type = attribute_model.value_type.as_deref().unwrap_or_default();
    serde_json::json!({
        "schemaUri": schema_uri,
        "element": element_name,
        "attribute": attribute_name,
        "contract": format!("attribute-type:{attribute_name}"),
        "diagnostic": diagnostic_code,
        "behavior": behavior,
        "type": violation.name,
        "checkKind": violation.check_kind,
        "valueType": expected_type,
        "expectedType": expected_type,
        "expectedValues": violation.expected_values,
        "expectedPattern": violation.expected_pattern,
        "allowsEmpty": violation.allows_empty,
        "actualValue": actual_value,
        "requiredFields": [],
        "optionalFields": [],
        "forbiddenFields": [],
        "missingFields": [],
        "invalidFields": [attribute_name],
        "actualValues": attribute_values,
        "sourceRange": node_source_range_details(node),
    })
}

fn attribute_datatype_param_details(
    schema_uri: &str,
    element_name: &str,
    attribute_name: &str,
    actual_value: &str,
    attribute_model: &AttributeModel,
    param_name: &str,
    param_value: &str,
    diagnostic_code: &str,
    behavior: &str,
    attribute_values: &BTreeMap<String, String>,
    node: &CemAstNode,
) -> serde_json::Value {
    let expected_type = attribute_model.value_type.as_deref().unwrap_or_default();
    serde_json::json!({
        "schemaUri": schema_uri,
        "element": element_name,
        "attribute": attribute_name,
        "contract": format!("attribute-datatype-param:{attribute_name}:{param_name}"),
        "diagnostic": diagnostic_code,
        "behavior": behavior,
        "type": type_reference_local_name(expected_type),
        "checkKind": format!("datatype-param:{param_name}"),
        "datatypeParam": param_name,
        "paramName": param_name,
        "paramValue": param_value,
        "valueType": expected_type,
        "expectedType": expected_type,
        "expectedPattern": "signed decimal integer",
        "minInclusive": param_value,
        "actualValue": actual_value,
        "requiredFields": [],
        "optionalFields": [],
        "forbiddenFields": [],
        "missingFields": [],
        "invalidFields": [attribute_name],
        "actualValues": attribute_values,
        "sourceRange": node_source_range_details(node),
    })
}

fn collect_schema_uses(document: &CemDocument, schema_id: AstNodeId) -> BTreeMap<String, String> {
    let mut uses = BTreeMap::new();
    for uses_id in element_child_ids_by_local_name(document, schema_id, "uses") {
        let Some(CemAstNode::Element { children, .. }) = document.get(uses_id) else {
            continue;
        };
        for child_id in children {
            if document.get(*child_id).and_then(element_local_name) != Some("use") {
                continue;
            }
            let attrs = collect_attrs(document, *child_id);
            let Some(alias) = attrs.get("as").map(String::as_str).map(str::trim) else {
                continue;
            };
            let Some(schema_uri) = attrs.get("schema").map(String::as_str).map(str::trim) else {
                continue;
            };
            if !alias.is_empty() && !schema_uri.is_empty() {
                uses.insert(alias.to_owned(), schema_uri.to_owned());
            }
        }
    }
    uses
}

fn resolve_base_element_model(
    base: &str,
    uses: &BTreeMap<String, String>,
    seen_schema_uris: &mut BTreeSet<String>,
) -> Option<ElementModel> {
    let (alias, element_name) = base.trim().split_once(':')?;
    let schema_uri = uses.get(alias.trim())?.trim();
    let element_name = element_name.trim();
    if schema_uri.is_empty() || element_name.is_empty() {
        return None;
    }
    let package = load_builtin_schema_package(schema_uri).ok()?;
    if !is_bootstrap_document_model_schema(&package.descriptor.schema_uri) {
        return None;
    }
    compile_document_model_with_seen(
        &package.descriptor.schema_uri,
        package.schema_source,
        seen_schema_uris,
    )
    .element(element_name)
    .cloned()
}

fn is_bootstrap_document_model_schema(schema_uri: &str) -> bool {
    matches!(
        schema_uri,
        CEM_ML_SCHEMA_URI
            | CEM_SCHEMA_URI
            | CEM_SCHEMA_PACKAGE_URI
            | CEM_NATIVE_TEMPLATE_SCHEMA_URI
            | CEM_TRANSFORM_SCHEMA_URI
    )
}

fn parse_cem_document(input: &str) -> CemDocument {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tok);
    CemAstBuilder::new(normalizer).build()
}

fn collect_attrs(document: &CemDocument, node_id: AstNodeId) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    let Some(CemAstNode::Element { attributes, .. }) = document.get(node_id) else {
        return attrs;
    };

    for attr_id in attributes {
        let Some(CemAstNode::Attribute {
            expanded_name,
            value,
            ..
        }) = document.get(*attr_id)
        else {
            continue;
        };
        attrs.insert(
            expanded_name.local_name.clone(),
            value.clone().unwrap_or_default(),
        );
    }

    attrs
}

fn optional_non_empty_attr<'a>(attrs: &'a BTreeMap<String, String>, name: &str) -> Option<&'a str> {
    attrs
        .get(name)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn attr_is_true(value: Option<&String>) -> bool {
    matches!(
        value.map(String::as_str).map(str::trim),
        Some("true" | "required")
    )
}

fn parse_name_set(value: Option<&String>) -> BTreeSet<String> {
    value
        .map(|value| {
            value
                .split_whitespace()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_value_set(value: Option<&String>) -> BTreeSet<String> {
    parse_name_set(value)
}

fn parse_name_value_set(value: Option<&String>) -> BTreeMap<String, BTreeSet<String>> {
    let mut pairs = BTreeMap::new();
    let Some(value) = value else {
        return pairs;
    };

    for token in value.split_whitespace().map(str::trim) {
        let Some((name, value)) = token.split_once('=') else {
            continue;
        };
        let name = name.trim();
        let value = value.trim();
        if name.is_empty() || value.is_empty() {
            continue;
        }
        pairs
            .entry(name.to_owned())
            .or_insert_with(BTreeSet::new)
            .insert(value.to_owned());
    }

    pairs
}

fn child_element_counts(document: &CemDocument, children: &[AstNodeId]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for child_id in children {
        let Some(child) = document.get(*child_id) else {
            continue;
        };
        let Some(local_name) = element_local_name(child) else {
            continue;
        };
        if should_skip_structural_name(local_name) {
            continue;
        }
        *counts.entry(local_name.to_owned()).or_insert(0) += 1;
    }
    counts
}

fn path_layout_is_valid(path: &str, contract: &FieldContract) -> bool {
    let path = path.trim();
    if path.is_empty() || (has_uri_scheme(path) && !is_windows_drive_path(path)) {
        return false;
    }
    let parsed = Path::new(path);
    if parsed.is_absolute() {
        return false;
    }
    if let Some(extension) = contract.path_layout_extension.as_deref() {
        if parsed.extension().and_then(|ext| ext.to_str()) != Some(extension) {
            return false;
        }
    }

    let mut components = parsed.components();
    if let Some(prefix) = contract.path_layout_prefix.as_deref() {
        match components.next() {
            Some(Component::Normal(first)) if first == prefix => {}
            _ => return false,
        }
    }

    let mut has_path_tail = contract.path_layout_prefix.is_none();
    for component in components {
        match component {
            Component::Normal(_) => has_path_tail = true,
            _ => return false,
        }
    }
    has_path_tail
}

fn parse_child_set(value: Option<&String>) -> (BTreeSet<String>, bool) {
    let mut names = BTreeSet::new();
    let mut allow_any = false;
    let Some(value) = value else {
        return (names, false);
    };

    for token in value.split_whitespace().map(str::trim) {
        if token.is_empty() {
            continue;
        }
        if token == "*" {
            allow_any = true;
            continue;
        }
        let name = token
            .trim_end_matches('*')
            .trim_end_matches('?')
            .trim_end_matches('+');
        if !name.is_empty() {
            names.insert(name.to_owned());
        }
    }

    (names, allow_any)
}

fn first_element_id_by_local_name(document: &CemDocument, local_name: &str) -> Option<AstNodeId> {
    document.iter().find_map(|node| {
        let CemAstNode::Element {
            node_id,
            expanded_name,
            ..
        } = node
        else {
            return None;
        };
        (expanded_name.local_name == local_name).then_some(*node_id)
    })
}

fn element_child_ids_by_local_name(
    document: &CemDocument,
    node_id: AstNodeId,
    local_name: &str,
) -> Vec<AstNodeId> {
    let Some(CemAstNode::Element { children, .. }) = document.get(node_id) else {
        return Vec::new();
    };
    children
        .iter()
        .copied()
        .filter(|child_id| {
            matches!(
                document.get(*child_id),
                Some(CemAstNode::Element { expanded_name, .. })
                    if expanded_name.local_name == local_name
            )
        })
        .collect()
}

fn element_local_name(node: &CemAstNode) -> Option<&str> {
    match node {
        CemAstNode::Element { expanded_name, .. } => Some(expanded_name.local_name.as_str()),
        _ => None,
    }
}

fn attribute_parts(node: &CemAstNode) -> Option<(&str, &str, Option<&str>)> {
    match node {
        CemAstNode::Attribute {
            expanded_name,
            value,
            ..
        } => Some((
            expanded_name.namespace_uri.as_str(),
            expanded_name.local_name.as_str(),
            value.as_deref(),
        )),
        _ => None,
    }
}

fn should_skip_structural_name(local: &str) -> bool {
    local.is_empty() || local == "$" || local.starts_with('@')
}

fn source_stack_for_node(node: &CemAstNode) -> &SourceMapStack {
    match node {
        CemAstNode::Document { source, .. }
        | CemAstNode::Element { source, .. }
        | CemAstNode::Attribute { source, .. }
        | CemAstNode::Text { source, .. }
        | CemAstNode::Whitespace { source, .. }
        | CemAstNode::Comment { source, .. }
        | CemAstNode::ProcessingInstruction { source, .. }
        | CemAstNode::Cdata { source, .. }
        | CemAstNode::RawText { source, .. }
        | CemAstNode::Error { source, .. } => source,
    }
}

fn node_source_range_details(node: &CemAstNode) -> Option<serde_json::Value> {
    source_stack_for_node(node)
        .current()
        .map(source_frame_range_details)
}

fn source_frame_range_details(frame: &SourceMapFrame) -> serde_json::Value {
    serde_json::json!({
        "sourceId": frame.source_id.0,
        "span": frame_span_details(&frame.span),
    })
}

fn frame_span_details(span: &FrameSpan) -> serde_json::Value {
    match span {
        FrameSpan::Single(range) => serde_json::json!({
            "kind": "single",
            "start": range.start,
            "len": range.len,
            "end": range.end(),
        }),
        FrameSpan::Multi(ranges) => serde_json::json!({
            "kind": "multi",
            "ranges": ranges
                .iter()
                .map(|range| {
                    serde_json::json!({
                        "start": range.start,
                        "len": range.len,
                        "end": range.end(),
                    })
                })
                .collect::<Vec<_>>(),
        }),
    }
}

fn diag_at_with_details_and_severity(
    code: &str,
    severity: Severity,
    message: String,
    node: &CemAstNode,
    details: serde_json::Value,
) -> Diagnostic {
    let mut diagnostic = diag_at_with_severity(code, severity, message, node);
    diagnostic.details = Some(details);
    diagnostic
}

fn diag_at(code: &str, message: String, node: &CemAstNode) -> Diagnostic {
    diag_at_with_severity(code, Severity::Error, message, node)
}

fn diag_at_with_severity(
    code: &str,
    severity: Severity,
    message: String,
    node: &CemAstNode,
) -> Diagnostic {
    let stack = source_stack_for_node(node);
    let byte_offset = stack.frames.first().and_then(|f| match &f.span {
        FrameSpan::Single(r) => Some(r.start),
        FrameSpan::Multi(rs) => rs.first().map(|r| r.start),
    });
    Diagnostic {
        uri: None,
        line: None,
        column: None,
        byte_offset,
        code: code.to_owned(),
        severity,
        message,
        node: None,
        details: None,
        source_map: Some(stack.clone()),
    }
}

fn schema_compile_diagnostic(
    code: &str,
    message: String,
    source_map: &SourceMapStack,
    details: serde_json::Value,
) -> Diagnostic {
    let byte_offset = source_map
        .frames
        .first()
        .and_then(|frame| match &frame.span {
            FrameSpan::Single(range) => Some(range.start),
            FrameSpan::Multi(ranges) => ranges.first().map(|range| range.start),
        });
    Diagnostic {
        uri: None,
        line: None,
        column: None,
        byte_offset,
        code: code.to_owned(),
        severity: Severity::Error,
        message,
        node: None,
        details: Some(details),
        source_map: Some(source_map.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::registry::{
        CEM_ML_CONTENT_TYPE, CEM_NATIVE_TEMPLATE_CONTENT_TYPE, CEM_NATIVE_TEMPLATE_SCHEMA_URI,
        CEM_SCHEMA_CONTENT_TYPE, CEM_SCHEMA_PACKAGE_CONTENT_TYPE, CEM_SCHEMA_PACKAGE_URI,
        CEM_SCHEMA_URI, CEM_TRANSFORM_CONTENT_TYPE, CEM_TRANSFORM_SCHEMA_URI, HTML_CONTENT_TYPE,
    };

    #[test]
    fn loads_schema_definition_document_model() {
        let model = load_builtin_document_model_for_identity(Some(CEM_SCHEMA_URI), None).unwrap();

        let schema = model.element("schema").unwrap();
        assert!(schema.required_attributes.contains("name"));
        assert!(schema.required_attributes.contains("namespace"));
        assert!(schema.required_attributes.contains("version"));
        assert!(schema.child_elements.contains("elements"));
        assert!(model.element("attribute").is_some());
        let field_contract_behavior = model
            .behaviors
            .get("field-contract")
            .expect("schema-declared field-contract engine behavior");
        assert_eq!(field_contract_behavior.implementation, "engine");
        assert_eq!(field_contract_behavior.execution, "ast-validation");
        assert_eq!(
            field_contract_behavior.primitive.as_deref(),
            Some(FIELD_CONTRACT_DIAGNOSTIC_BEHAVIOR)
        );
        assert!(field_contract_behavior
            .inputs
            .iter()
            .any(|input| input.name == "candidate"
                && input.value_type == "schema:node"
                && input.source == "candidate"));
        assert!(field_contract_behavior
            .parameters
            .iter()
            .any(|parameter| parameter.name == "contract"
                && parameter.value_type == "schema:field-contract"
                && parameter.required));
        let result = field_contract_behavior
            .result
            .as_ref()
            .expect("field-contract behavior result declaration");
        assert_eq!(result.value_type, "schema:diagnostic-result");
        assert_eq!(result.severity.as_deref(), Some("error"));
        assert_eq!(result.source_range.as_deref(), Some("candidate"));
        assert!(result
            .details
            .iter()
            .any(|detail| detail.name == "checkKind" && detail.value_type == "schema:identifier"));
        for behavior_name in [
            "required-fields",
            "forbidden-fields",
            "dependent-required-fields",
            "mutual-exclusion",
            "child-occurrence",
            "path-layout",
        ] {
            let behavior = model
                .behaviors
                .get(behavior_name)
                .unwrap_or_else(|| panic!("schema-declared {behavior_name} engine behavior"));
            assert_eq!(behavior.implementation, "engine");
            assert_eq!(behavior.execution, "ast-validation");
            assert_eq!(
                behavior.primitive.as_deref(),
                Some(FIELD_CONTRACT_DIAGNOSTIC_BEHAVIOR)
            );
            assert!(behavior
                .parameters
                .iter()
                .any(|parameter| parameter.name == "contract"
                    && parameter.value_type == "schema:field-contract"
                    && parameter.required));
            assert!(behavior.result.is_some());
        }
        for (behavior_name, primitive, engine_behavior) in [
            (
                "value-vocabulary",
                VALUE_VOCABULARY_DIAGNOSTIC_BEHAVIOR,
                EngineDiagnosticBehavior::ValueVocabulary,
            ),
            (
                "scalar-type",
                SCALAR_TYPE_DIAGNOSTIC_BEHAVIOR,
                EngineDiagnosticBehavior::ScalarType,
            ),
            (
                "datatype-param",
                DATATYPE_PARAM_DIAGNOSTIC_BEHAVIOR,
                EngineDiagnosticBehavior::DatatypeParam,
            ),
            (
                "resource-readable",
                RESOURCE_READABLE_DIAGNOSTIC_BEHAVIOR,
                EngineDiagnosticBehavior::ResourceReadable,
            ),
            (
                "resource-parse",
                RESOURCE_PARSE_DIAGNOSTIC_BEHAVIOR,
                EngineDiagnosticBehavior::ResourceParse,
            ),
            (
                "reference-resolution",
                REFERENCE_RESOLUTION_DIAGNOSTIC_BEHAVIOR,
                EngineDiagnosticBehavior::ReferenceResolution,
            ),
        ] {
            let behavior = model
                .behaviors
                .get(behavior_name)
                .unwrap_or_else(|| panic!("schema-declared {behavior_name} engine behavior"));
            assert_eq!(behavior.implementation, "engine");
            assert_eq!(behavior.execution, "ast-validation");
            assert_eq!(behavior.primitive.as_deref(), Some(primitive));
            assert_eq!(supported_engine_behavior(behavior), Some(engine_behavior));
            assert!(behavior.parameters.iter().any(|parameter| {
                parameter.required
                    && ((parameter.name == "attribute"
                        && parameter.value_type == "schema:attribute")
                        || (parameter.name == "constraint"
                            && parameter.value_type == "schema:constraint"))
            }));
            assert!(behavior.result.is_some());
        }
    }

    #[test]
    fn loads_schema_package_document_model_from_content_type() {
        let model = load_builtin_document_model_for_identity(
            None,
            Some("application/vnd.cem.schema-package+cem; charset=utf-8"),
        )
        .unwrap();

        assert_eq!(model.schema_uri, CEM_SCHEMA_PACKAGE_URI);
        assert!(
            model.compile_diagnostics.is_empty(),
            "schema-package behavior declarations must compile: {:#?}",
            model.compile_diagnostics
        );
        assert_eq!(
            model
                .diagnostic_behaviors
                .get("cem.schema_package.converter_check")
                .map(|behavior| behavior.behavior.as_str()),
            Some(FIELD_CONTRACT_DIAGNOSTIC_BEHAVIOR)
        );
        let package = model.element("package").unwrap();
        assert!(package.required_attributes.contains("id"));
        assert!(package.child_elements.contains("converter"));
        let artifact = model.element("artifact").unwrap();
        assert!(artifact
            .field_contracts
            .iter()
            .any(|contract| contract.name == "artifact-stage-metadata"
                && contract.diagnostic.as_deref() == Some("cem.schema_package.artifact_check")
                && contract.required_attributes.contains("target-schema")));
        assert_eq!(
            model
                .attributes
                .get("implementation")
                .expect("implementation attribute model")
                .allowed_values,
            BTreeSet::from(["cemt".to_owned(), "rust".to_owned()])
        );
        assert_eq!(
            model
                .attributes
                .get("cost")
                .expect("cost attribute model")
                .value_type
                .as_deref(),
            Some("schema:integer")
        );
        assert_eq!(
            model
                .attributes
                .get("cost")
                .expect("cost attribute model")
                .min_inclusive
                .as_deref(),
            Some("1")
        );
        let artifact_source_readable = model
            .constraint("artifact-source-readable")
            .expect("artifact source readability constraint");
        assert_eq!(
            artifact_source_readable.diagnostic.as_deref(),
            Some("cem.schema_package.artifact_check")
        );
        assert_eq!(
            artifact_source_readable.behavior.as_deref(),
            Some("schema:resource-readable")
        );
        assert_eq!(
            artifact_source_readable.engine_behavior,
            Some(EngineDiagnosticBehavior::ResourceReadable)
        );
        let artifact_cemt_valid = model
            .constraint("artifact-cemt-valid")
            .expect("artifact CEMT validity constraint");
        assert_eq!(
            artifact_cemt_valid.engine_behavior,
            Some(EngineDiagnosticBehavior::ResourceParse)
        );
        let example_content_type_schema = model
            .constraint("example-content-type-schema")
            .expect("example content type/schema constraint");
        assert_eq!(
            example_content_type_schema.engine_behavior,
            Some(EngineDiagnosticBehavior::ReferenceResolution)
        );
    }

    #[test]
    fn schema_constraint_behavior_binding_compiles_independently_from_diagnostic_family() {
        let model = compile_document_model(
            "https://example.test/ns/constraint-behavior/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="constraint-behavior" @namespace="https://example.test/ns/constraint-behavior/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {elements |
        {element @name="artifact" @optional-attributes="path"}
    }
    {attributes |
        {attribute @name="path" @type="schema:path"}
    }
    {field-contracts |
        {field-contract
            @name="artifact-path"
            @target="artifact"
            @required-attributes="path"
            @diagnostic="example.artifact_check"
            @check-kind="required-fields"
        }
    }
    {constraints |
        {constraint
            @kind="artifact-source-readable"
            @target="artifact"
            @diagnostic="example.artifact_check"
            @behavior="schema:resource-readable"
            @policy="artifact source must be readable"
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.artifact_check"
            @severity="error"
            @behavior="schema:field-contract"
        }
    }
}"#,
        );

        assert!(
            model.compile_diagnostics.is_empty(),
            "constraint behavior binding must compile: {:#?}",
            model.compile_diagnostics
        );
        let constraint = model
            .constraint("artifact-source-readable")
            .expect("compiled constraint behavior");
        assert_eq!(
            constraint.diagnostic.as_deref(),
            Some("example.artifact_check")
        );
        assert_eq!(
            constraint.behavior.as_deref(),
            Some("schema:resource-readable")
        );
        assert_eq!(
            constraint.engine_behavior,
            Some(EngineDiagnosticBehavior::ResourceReadable)
        );
        assert_eq!(
            model
                .diagnostic_behaviors
                .get("example.artifact_check")
                .map(|behavior| behavior.engine_behavior),
            Some(Some(EngineDiagnosticBehavior::FieldContract))
        );
    }

    #[test]
    fn schema_field_contracts_drive_validation_from_cem_source() {
        let model = compile_document_model(
            "https://example.test/ns/contracts/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="contracts" @namespace="https://example.test/ns/contracts/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {elements |
        {element @name="item" @required-attributes="kind" @optional-attributes="kind name code mode"}
        {element @name="group" @children="child"}
        {element @name="child"}
        {element @name="asset" @optional-attributes="path"}
    }
    {field-contracts |
        {field-contract
            @name="kind-a-fields"
            @target="item"
            @when-attribute="kind"
            @when-values="a"
            @required-attributes="name"
            @diagnostic="example.item_check"
            @check-kind="required-fields"
        }
        {field-contract
            @name="kind-b-fields"
            @target="item"
            @when-attribute="kind"
            @when-values="b"
            @required-attributes="code"
            @diagnostic="example.item_check"
            @check-kind="required-fields"
        }
        {field-contract
            @name="name-needs-code"
            @target="item"
            @when-present-attributes="name"
            @required-attributes="code"
            @diagnostic="example.item_check"
            @check-kind="dependent-required-fields"
        }
        {field-contract
            @name="kind-a-mode-conflict"
            @target="item"
            @when-attribute="kind"
            @when-values="a"
            @forbidden-attribute-values="mode=blocked"
            @diagnostic="example.item_check"
            @check-kind="mutual-exclusion"
        }
        {field-contract
            @name="group-exact-child"
            @target="group"
            @required-children="child"
            @max-one-children="child"
            @diagnostic="example.item_check"
            @check-kind="child-occurrence"
        }
        {field-contract
            @name="asset-path-layout"
            @target="asset"
            @path-layout-attributes="path"
            @path-layout-prefix="assets"
            @path-layout-extension="cemt"
            @diagnostic="example.item_check"
            @check-kind="path-layout"
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.item_check"
            @severity="error"
            @behavior="schema:field-contract"
        }
    }
}"#,
        );
        let document = parse_cem_document(r#"{item @kind=a}"#);

        let diagnostics = validate_document_model(&document, &model);

        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "example.item_check")
            .expect("field contract diagnostic");
        assert!(diagnostic.message.contains("kind-a-fields"));
        assert!(diagnostic.message.contains("missing fields: name"));
        let details = diagnostic.details.as_ref().expect("field contract details");
        assert_eq!(
            details["schemaUri"],
            serde_json::json!("https://example.test/ns/contracts/1")
        );
        assert_eq!(details["element"], serde_json::json!("item"));
        assert_eq!(details["contract"], serde_json::json!("kind-a-fields"));
        assert_eq!(details["target"], serde_json::json!("item"));
        assert_eq!(
            details["diagnostic"],
            serde_json::json!("example.item_check")
        );
        assert_eq!(details["checkKind"], serde_json::json!("required-fields"));
        assert_eq!(details["requiredFields"], serde_json::json!(["name"]));
        assert_eq!(details["optionalFields"], serde_json::json!([]));
        assert_eq!(details["forbiddenFields"], serde_json::json!([]));
        assert_eq!(details["missingFields"], serde_json::json!(["name"]));
        assert_eq!(details["invalidFields"], serde_json::json!([]));
        assert_eq!(details["actualValues"]["kind"], serde_json::json!("a"));
        assert_eq!(details["condition"]["attribute"], serde_json::json!("kind"));
        assert_eq!(details["condition"]["values"], serde_json::json!(["a"]));
        assert_eq!(
            details["condition"]["presentAttributes"],
            serde_json::json!([])
        );
        assert!(details["sourceRange"]["span"]["start"].is_u64());
        assert!(!diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("kind-b-fields")));
        assert!(!diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("name-needs-code")));

        let document = parse_cem_document(r#"{item @kind=a @name=foo}"#);
        let diagnostics = validate_document_model(&document, &model);
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.code == "example.item_check"
                    && diagnostic.details.as_ref().and_then(|details| {
                        details.get("contract").and_then(serde_json::Value::as_str)
                    }) == Some("name-needs-code")
            })
            .expect("presence-gated field contract diagnostic");
        let details = diagnostic
            .details
            .as_ref()
            .expect("presence-gated field contract details");
        assert_eq!(details["missingFields"], serde_json::json!(["code"]));
        assert_eq!(
            details["condition"],
            serde_json::json!({
                "attribute": null,
                "values": [],
                "presentAttributes": ["name"],
            })
        );

        let document = parse_cem_document(r#"{item @kind=a @mode=open}"#);
        let diagnostics = validate_document_model(&document, &model);
        assert!(
            diagnostics.iter().all(|diagnostic| {
                diagnostic
                    .details
                    .as_ref()
                    .and_then(|details| details.get("contract").and_then(serde_json::Value::as_str))
                    != Some("kind-a-mode-conflict")
            }),
            "allowed value should not trigger mutual-exclusion field contract: {diagnostics:?}"
        );

        let document = parse_cem_document(r#"{item @kind=a @mode=blocked}"#);
        let diagnostics = validate_document_model(&document, &model);
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.code == "example.item_check"
                    && diagnostic.details.as_ref().and_then(|details| {
                        details.get("contract").and_then(serde_json::Value::as_str)
                    }) == Some("kind-a-mode-conflict")
            })
            .expect("value-specific forbidden field contract diagnostic");
        let details = diagnostic
            .details
            .as_ref()
            .expect("value-specific forbidden field contract details");
        assert_eq!(details["missingFields"], serde_json::json!([]));
        assert_eq!(details["invalidFields"], serde_json::json!(["mode"]));
        assert_eq!(
            details["forbiddenAttributeValues"],
            serde_json::json!({
                "mode": ["blocked"],
            })
        );
        assert_eq!(
            details["invalidValues"],
            serde_json::json!({
                "mode": "blocked",
            })
        );

        let document = parse_cem_document(r#"{group}"#);
        let diagnostics = validate_document_model(&document, &model);
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.code == "example.item_check"
                    && diagnostic.details.as_ref().and_then(|details| {
                        details.get("contract").and_then(serde_json::Value::as_str)
                    }) == Some("group-exact-child")
            })
            .expect("missing child field contract diagnostic");
        let details = diagnostic
            .details
            .as_ref()
            .expect("missing child field contract details");
        assert_eq!(details["missingChildren"], serde_json::json!(["child"]));
        assert_eq!(details["duplicateChildren"], serde_json::json!([]));
        assert_eq!(details["requiredChildren"], serde_json::json!(["child"]));
        assert_eq!(details["maxOneChildren"], serde_json::json!(["child"]));
        assert_eq!(details["childCounts"], serde_json::json!({}));

        let document = parse_cem_document(r#"{group | {child}}"#);
        let diagnostics = validate_document_model(&document, &model);
        assert!(
            diagnostics.iter().all(|diagnostic| {
                diagnostic
                    .details
                    .as_ref()
                    .and_then(|details| details.get("contract").and_then(serde_json::Value::as_str))
                    != Some("group-exact-child")
            }),
            "single child should satisfy child occurrence field contract: {diagnostics:?}"
        );

        let document = parse_cem_document(r#"{group | {child} {child}}"#);
        let diagnostics = validate_document_model(&document, &model);
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.code == "example.item_check"
                    && diagnostic.details.as_ref().and_then(|details| {
                        details.get("contract").and_then(serde_json::Value::as_str)
                    }) == Some("group-exact-child")
            })
            .expect("duplicate child field contract diagnostic");
        let details = diagnostic
            .details
            .as_ref()
            .expect("duplicate child field contract details");
        assert_eq!(details["missingChildren"], serde_json::json!([]));
        assert_eq!(details["duplicateChildren"], serde_json::json!(["child"]));
        assert_eq!(
            details["childCounts"],
            serde_json::json!({
                "child": 2,
            })
        );

        let document = parse_cem_document(r#"{asset @path="assets/demo.cemt"}"#);
        let diagnostics = validate_document_model(&document, &model);
        assert!(
            diagnostics.iter().all(|diagnostic| {
                diagnostic
                    .details
                    .as_ref()
                    .and_then(|details| details.get("contract").and_then(serde_json::Value::as_str))
                    != Some("asset-path-layout")
            }),
            "matching path should satisfy path-layout field contract: {diagnostics:?}"
        );

        let document = parse_cem_document(r#"{asset @path="transforms/demo.cem"}"#);
        let diagnostics = validate_document_model(&document, &model);
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.code == "example.item_check"
                    && diagnostic.details.as_ref().and_then(|details| {
                        details.get("contract").and_then(serde_json::Value::as_str)
                    }) == Some("asset-path-layout")
            })
            .expect("path-layout field contract diagnostic");
        let details = diagnostic
            .details
            .as_ref()
            .expect("path-layout field contract details");
        assert_eq!(details["invalidFields"], serde_json::json!(["path"]));
        assert_eq!(
            details["invalidValues"],
            serde_json::json!({
                "path": "transforms/demo.cem",
            })
        );
        assert_eq!(
            details["pathLayout"],
            serde_json::json!({
                "attributes": ["path"],
                "prefix": "assets",
                "extension": "cemt",
                "relative": true,
                "cleanSegments": true,
            })
        );
    }

    #[test]
    fn schema_diagnostic_behavior_drives_field_contract_execution_and_severity() {
        let model = compile_document_model(
            "https://example.test/ns/diagnostic-behavior/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="diagnostic-behavior" @namespace="https://example.test/ns/diagnostic-behavior/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {elements |
        {element @name="item" @optional-attributes="name"}
    }
    {field-contracts |
        {field-contract
            @name="item-name-required"
            @target="item"
            @required-attributes="name"
            @diagnostic="example.item_name_required"
            @check-kind="required-fields"
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.item_name_required"
            @severity="warning"
            @behavior="schema:field-contract"
            @message="Item name is required"
        }
    }
}"#,
        );

        assert!(model.compile_diagnostics.is_empty());
        let behavior = model
            .diagnostic_behaviors
            .get("example.item_name_required")
            .expect("compiled diagnostic behavior");
        assert_eq!(behavior.behavior, "schema:field-contract");
        assert_eq!(behavior.severity, Severity::Warning);

        let document = parse_cem_document(r#"{item}"#);
        let diagnostics = validate_document_model(&document, &model);
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "example.item_name_required")
            .expect("schema-owned diagnostic behavior result");

        assert_eq!(diagnostic.severity, Severity::Warning);
        assert!(diagnostic.message.starts_with("Item name is required:"));
        assert_eq!(
            diagnostic
                .details
                .as_ref()
                .and_then(|details| details.get("behavior"))
                .and_then(serde_json::Value::as_str),
            Some("schema:field-contract")
        );
    }

    #[test]
    fn schema_diagnostic_behavior_aliases_dispatch_to_field_contract_engine() {
        let model = compile_document_model(
            "https://example.test/ns/diagnostic-behavior-alias/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="diagnostic-behavior-alias" @namespace="https://example.test/ns/diagnostic-behavior-alias/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {elements |
        {element @name="item" @optional-attributes="name internal"}
        {element @name="group" @children="child"}
        {element @name="child"}
    }
    {attributes |
        {attribute @name="name" @type="schema:string"}
        {attribute @name="internal" @type="schema:string"}
    }
    {field-contracts |
        {field-contract
            @name="item-name-required"
            @target="item"
            @required-attributes="name"
            @diagnostic="example.item_name_required"
            @check-kind="required-fields"
        }
        {field-contract
            @name="item-internal-forbidden"
            @target="item"
            @forbidden-attributes="internal"
            @diagnostic="example.item_internal_forbidden"
            @check-kind="forbidden-fields"
        }
        {field-contract
            @name="group-single-child"
            @target="group"
            @required-children="child"
            @max-one-children="child"
            @diagnostic="example.group_child_occurrence"
            @check-kind="child-occurrence"
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.item_name_required"
            @severity="warning"
            @behavior="schema:required-fields"
        }
        {diagnostic
            @code="example.item_internal_forbidden"
            @severity="error"
            @behavior="schema:forbidden-fields"
        }
        {diagnostic
            @code="example.group_child_occurrence"
            @severity="error"
            @behavior="schema:child-occurrence"
        }
    }
}"#,
        );

        assert!(
            model.compile_diagnostics.is_empty(),
            "behavior aliases must compile: {:#?}",
            model.compile_diagnostics
        );
        assert_eq!(
            model
                .diagnostic_behaviors
                .get("example.item_name_required")
                .map(|behavior| behavior.engine_behavior),
            Some(Some(EngineDiagnosticBehavior::FieldContract))
        );

        let document = parse_cem_document(r#"{item @internal=yes}"#);
        let diagnostics = validate_document_model(&document, &model);
        let required = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "example.item_name_required")
            .expect("required-fields alias diagnostic");
        assert_eq!(required.severity, Severity::Warning);
        let details = required
            .details
            .as_ref()
            .expect("required-fields alias details");
        assert_eq!(
            details["behavior"],
            serde_json::json!("schema:required-fields")
        );
        assert_eq!(details["checkKind"], serde_json::json!("required-fields"));
        assert_eq!(details["missingFields"], serde_json::json!(["name"]));

        let forbidden = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "example.item_internal_forbidden")
            .expect("forbidden-fields alias diagnostic");
        let details = forbidden
            .details
            .as_ref()
            .expect("forbidden-fields alias details");
        assert_eq!(
            details["behavior"],
            serde_json::json!("schema:forbidden-fields")
        );
        assert_eq!(details["checkKind"], serde_json::json!("forbidden-fields"));
        assert_eq!(details["forbiddenFields"], serde_json::json!(["internal"]));
        assert_eq!(details["invalidFields"], serde_json::json!(["internal"]));

        let document = parse_cem_document(r#"{group | {child} {child}}"#);
        let diagnostics = validate_document_model(&document, &model);
        let child_occurrence = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "example.group_child_occurrence")
            .expect("child-occurrence alias diagnostic");
        let details = child_occurrence
            .details
            .as_ref()
            .expect("child-occurrence alias details");
        assert_eq!(
            details["behavior"],
            serde_json::json!("schema:child-occurrence")
        );
        assert_eq!(details["checkKind"], serde_json::json!("child-occurrence"));
        assert_eq!(details["duplicateChildren"], serde_json::json!(["child"]));
    }

    #[test]
    fn schema_diagnostic_behavior_rejects_undeclared_field_contract_reference() {
        let model = compile_document_model(
            "https://example.test/ns/diagnostic-reference/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="diagnostic-reference" @namespace="https://example.test/ns/diagnostic-reference/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {elements |
        {element @name="item" @optional-attributes="name"}
    }
    {field-contracts |
        {field-contract
            @name="item-name-required"
            @target="item"
            @required-attributes="name"
            @diagnostic="example.undeclared"
        }
    }
}"#,
        );

        assert!(model.compile_diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cem.schema_definition.unresolved_diagnostic_reference"
                && diagnostic.message.contains("example.undeclared")
        }));
    }

    #[test]
    fn schema_diagnostic_behavior_rejects_unknown_engine_behavior() {
        let model = compile_document_model(
            "https://example.test/ns/diagnostic-engine/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="diagnostic-engine" @namespace="https://example.test/ns/diagnostic-engine/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {elements |
        {element @name="item"}
    }
    {field-contracts |
        {field-contract
            @name="unknown-engine-contract"
            @target="item"
            @required-attributes="name"
            @diagnostic="example.unknown_engine"
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.unknown_engine"
            @severity="error"
            @behavior="schema:not-an-engine-behavior"
        }
    }
}"#,
        );

        assert!(model.compile_diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cem.schema_definition.unknown_diagnostic_behavior"
                && diagnostic.message.contains("schema:not-an-engine-behavior")
        }));
        let document = parse_cem_document(r#"{item}"#);
        assert!(validate_document_model(&document, &model)
            .iter()
            .all(|diagnostic| diagnostic.code != "example.unknown_engine"));
    }

    #[test]
    fn schema_diagnostic_behavior_accepts_schema_declared_function_binding() {
        let model = compile_document_model(
            "https://example.test/ns/function-behavior/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="function-behavior" @namespace="https://example.test/ns/function-behavior/1" @version="1.0.0" |
    {elements |
        {element @name="item" @optional-attributes="name"}
    }
    {behaviors |
        {behavior
            @name="item-label"
            @implementation="function"
            @execution="ast-validation"
            @function="item-label-result"
            @select="item"
            @match="name = null" |
            {inputs |
                {input-binding @name="candidate" @type="schema:node" @source="candidate" @required=true}
            }
            {parameters |
                {parameter @name="expected" @type="schema:string" @required=true @default="label"}
            }
            {result
                @type="schema:diagnostic-result"
                @severity="diagnostic"
                @message="function"
                @source-range="candidate" |
                {detail @name="checkKind" @type="schema:identifier" @required=true}
            }
            {function @name="item-label-result" @returns="object" @deterministic=true |
                {param @name="candidate" @type="object" @required=true}
                {param @name="expected" @type="string" @required=true}
                {body | {$ { message: "Item label is required", details: { checkKind: "item-label" } } }}
            }
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.item_label"
            @severity="warning"
            @behavior="item-label"
            @message="Item label is required"
        }
    }
}"#,
        );

        assert!(
            model.compile_diagnostics.is_empty(),
            "function behavior should compile: {:#?}",
            model.compile_diagnostics
        );
        let behavior = model
            .behaviors
            .get("item-label")
            .expect("schema-declared function behavior");
        assert_eq!(behavior.function.as_deref(), Some("item-label-result"));
        assert_eq!(behavior.select.as_deref(), Some("item"));
        assert_eq!(behavior.match_query.as_deref(), Some("name = null"));
        assert!(behavior.inline_functions.contains_key("item-label-result"));
        assert_eq!(
            behavior
                .inline_functions
                .get("item-label-result")
                .map(|function| function.visibility.as_str()),
            Some("private")
        );
        assert_eq!(
            behavior
                .inline_functions
                .get("item-label-result")
                .and_then(|function| function.body_expression.as_deref()),
            Some(r#"{ message: "Item label is required", details: { checkKind: "item-label" } }"#)
        );
        let diagnostic_behavior = model
            .diagnostic_behaviors
            .get("example.item_label")
            .expect("diagnostic behavior binding");
        assert_eq!(
            diagnostic_behavior.function.as_deref(),
            Some("item-label-result")
        );
        assert_eq!(
            diagnostic_behavior
                .function_definition
                .as_ref()
                .map(|function| function.name.as_str()),
            Some("item-label-result")
        );
        assert_eq!(diagnostic_behavior.engine_behavior, None);
        assert_eq!(diagnostic_behavior.code, "example.item_label");
    }

    #[test]
    fn schema_diagnostic_behavior_accepts_qualified_reusable_function_binding() {
        let model = compile_document_model(
            "https://example.test/ns/function-behavior/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="function-behavior" @namespace="https://example.test/ns/function-behavior/1" @version="1.0.0" |
    {uses |
        {use @schema="https://example.test/ns/function-behavior/1" @as="self"}
    }
    {elements |
        {element @name="item" @optional-attributes="name"}
    }
    {behaviors |
        {behavior @name="result-library" @implementation="function" @execution="ast-validation" |
            {function @name="shared-label-result" @returns="object" @visibility="package" @deterministic=true |
                {param @name="candidate" @type="object" @required=true}
                {param @name="expected" @type="string" @required=true}
                {body | {$ { message: "Item label is required", details: { checkKind: "item-label", expected: $expected } } }}
            }
        }
        {behavior
            @name="item-label"
            @implementation="function"
            @execution="ast-validation"
            @function="self:shared-label-result"
            @select="item"
            @match="name = null" |
            {inputs |
                {input-binding @name="candidate" @type="schema:node" @source="candidate" @required=true}
            }
            {parameters |
                {parameter @name="expected" @type="schema:string" @required=true @default="label"}
            }
            {result @type="schema:diagnostic-result" @source-range="candidate"}
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.item_label"
            @severity="warning"
            @behavior="item-label"
            @message="Item label is required"
        }
    }
}"#,
        );

        assert!(
            model.compile_diagnostics.is_empty(),
            "qualified reusable function behavior should compile: {:#?}",
            model.compile_diagnostics
        );
        let diagnostic_behavior = model
            .diagnostic_behaviors
            .get("example.item_label")
            .expect("diagnostic behavior binding");
        assert_eq!(
            diagnostic_behavior.function.as_deref(),
            Some("self:shared-label-result")
        );
        let function = diagnostic_behavior
            .function_definition
            .as_ref()
            .expect("qualified reusable function");
        assert_eq!(function.name, "shared-label-result");
        assert_eq!(function.visibility, "package");
        assert_eq!(function.params.len(), 2);
    }

    #[test]
    fn schema_diagnostic_behavior_accepts_diagnostic_argument_override() {
        let model = compile_document_model(
            "https://example.test/ns/function-argument/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="function-argument" @namespace="https://example.test/ns/function-argument/1" @version="1.0.0" |
    {elements |
        {element @name="item" @optional-attributes="name"}
    }
    {behaviors |
        {behavior
            @name="item-label"
            @implementation="function"
            @execution="ast-validation"
            @function="item-label-result"
            @select="item"
            @match="name = null" |
            {inputs |
                {input-binding @name="candidate" @type="schema:node" @source="candidate" @required=true}
            }
            {parameters |
                {parameter @name="expected" @type="schema:string" @required=true}
            }
            {result @type="schema:diagnostic-result" @source-range="candidate"}
            {function @name="item-label-result" @returns="object" @deterministic=true |
                {param @name="candidate" @type="object" @required=true}
                {param @name="expected" @type="string" @required=true}
                {body | {$ { message: "Item field is required", details: { expected: $expected } } }}
            }
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.item_label"
            @severity="warning"
            @behavior="item-label" |
            {arguments |
                {argument @name="expected" @value="title"}
            }
        }
    }
}"#,
        );

        assert!(
            model.compile_diagnostics.is_empty(),
            "diagnostic argument override should compile: {:#?}",
            model.compile_diagnostics
        );
        let diagnostic_behavior = model
            .diagnostic_behaviors
            .get("example.item_label")
            .expect("diagnostic behavior binding");
        assert_eq!(diagnostic_behavior.arguments.len(), 1);
        assert_eq!(diagnostic_behavior.arguments[0].name, "expected");
        assert_eq!(diagnostic_behavior.arguments[0].value, "title");
        assert_eq!(
            diagnostic_behavior.function.as_deref(),
            Some("item-label-result")
        );
    }

    #[test]
    fn schema_diagnostic_behavior_rejects_unresolved_schema_function_binding() {
        let model = compile_document_model(
            "https://example.test/ns/function-behavior-invalid/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="function-behavior-invalid" @namespace="https://example.test/ns/function-behavior-invalid/1" @version="1.0.0" |
    {behaviors |
        {behavior
            @name="item-label"
            @implementation="function"
            @execution="ast-validation"
            @function="missing-result"
            @select="item"
            @match="true" |
            {result @type="schema:diagnostic-result" @source-range="candidate"}
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.item_label"
            @severity="error"
            @behavior="item-label"
        }
    }
}"#,
        );

        assert!(model.compile_diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cem.schema_definition.unresolved_behavior_function"
                && diagnostic.details.as_ref().is_some_and(|details| {
                    details["diagnostic"] == "example.item_label"
                        && details["behavior"] == "item-label"
                        && details["function"] == "missing-result"
                })
        }));
        assert!(model
            .diagnostic_behaviors
            .get("example.item_label")
            .is_some_and(|behavior| behavior.function.is_none()));
    }

    #[test]
    fn schema_diagnostic_behavior_rejects_private_reusable_function_binding() {
        let model = compile_document_model(
            "https://example.test/ns/function-behavior-invalid/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="function-behavior-invalid" @namespace="https://example.test/ns/function-behavior-invalid/1" @version="1.0.0" |
    {uses |
        {use @schema="https://example.test/ns/function-behavior-invalid/1" @as="self"}
    }
    {behaviors |
        {behavior @name="result-library" @implementation="function" @execution="ast-validation" |
            {function @name="shared-label-result" @returns="object" @deterministic=true |
                {param @name="candidate" @type="object" @required=true}
                {body | {$ { message: "Item label is required" } }}
            }
        }
        {behavior
            @name="item-label"
            @implementation="function"
            @execution="ast-validation"
            @function="self:shared-label-result"
            @select="item"
            @match="true" |
            {inputs |
                {input-binding @name="candidate" @type="schema:node" @source="candidate" @required=true}
            }
            {result @type="schema:diagnostic-result" @source-range="candidate"}
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.item_label"
            @severity="error"
            @behavior="item-label"
        }
    }
}"#,
        );

        assert!(model.compile_diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cem.schema_definition.unresolved_behavior_function"
                && diagnostic.details.as_ref().is_some_and(|details| {
                    details["diagnostic"] == "example.item_label"
                        && details["behavior"] == "item-label"
                        && details["function"] == "self:shared-label-result"
                })
        }));
        assert!(model
            .diagnostic_behaviors
            .get("example.item_label")
            .is_some_and(|behavior| {
                behavior.function.is_none() && behavior.function_definition.is_none()
            }));
    }

    #[test]
    fn schema_diagnostic_behavior_rejects_unknown_diagnostic_argument() {
        let model = compile_document_model(
            "https://example.test/ns/function-argument-invalid/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="function-argument-invalid" @namespace="https://example.test/ns/function-argument-invalid/1" @version="1.0.0" |
    {behaviors |
        {behavior
            @name="item-label"
            @implementation="function"
            @execution="ast-validation"
            @function="item-label-result"
            @select="item"
            @match="true" |
            {parameters |
                {parameter @name="expected" @type="schema:string" @required=true @default="label"}
            }
            {result @type="schema:diagnostic-result" @source-range="candidate"}
            {function @name="item-label-result" @returns="object" @deterministic=true |
                {param @name="expected" @type="string" @required=true}
                {body | {$ { message: "Item field is required" } }}
            }
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.item_label"
            @severity="error"
            @behavior="item-label" |
            {arguments |
                {argument @name="field" @value="title"}
            }
        }
    }
}"#,
        );

        assert!(model.compile_diagnostics.iter().any(|diagnostic| {
            diagnostic.code == INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE
                && diagnostic.details.as_ref().is_some_and(|details| {
                    details["diagnostic"] == "example.item_label"
                        && details["behavior"] == "item-label"
                        && details["argument"] == "field"
                        && details["checkKind"] == "behavior-argument-binding"
                })
        }));
        assert!(model
            .diagnostic_behaviors
            .get("example.item_label")
            .is_some_and(|behavior| behavior.function.is_none()));
    }

    #[test]
    fn schema_diagnostic_behavior_rejects_invalid_diagnostic_argument_type() {
        let model = compile_document_model(
            "https://example.test/ns/function-argument-type-invalid/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="function-argument-type-invalid" @namespace="https://example.test/ns/function-argument-type-invalid/1" @version="1.0.0" |
    {behaviors |
        {behavior
            @name="item-count"
            @implementation="function"
            @execution="ast-validation"
            @function="item-count-result"
            @select="item"
            @match="true" |
            {parameters |
                {parameter @name="minimum" @type="schema:integer" @required=true}
            }
            {result @type="schema:diagnostic-result" @source-range="candidate"}
            {function @name="item-count-result" @returns="object" @deterministic=true |
                {param @name="minimum" @type="integer" @required=true}
                {body | {$ { message: "Item count is too small" } }}
            }
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.item_count"
            @severity="error"
            @behavior="item-count" |
            {arguments |
                {argument @name="minimum" @value="many"}
            }
        }
    }
}"#,
        );

        assert!(model.compile_diagnostics.iter().any(|diagnostic| {
            diagnostic.code == INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE
                && diagnostic.details.as_ref().is_some_and(|details| {
                    details["diagnostic"] == "example.item_count"
                        && details["behavior"] == "item-count"
                        && details["argument"] == "minimum"
                        && details["argumentType"] == "schema:integer"
                        && details["checkKind"] == "behavior-argument-binding"
                })
        }));
        assert!(model
            .diagnostic_behaviors
            .get("example.item_count")
            .is_some_and(|behavior| behavior.function.is_none()));
    }

    #[test]
    fn schema_diagnostic_behavior_rejects_unbound_required_function_parameter() {
        let model = compile_document_model(
            "https://example.test/ns/function-param-invalid/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="function-param-invalid" @namespace="https://example.test/ns/function-param-invalid/1" @version="1.0.0" |
    {elements |
        {element @name="item"}
    }
    {behaviors |
        {behavior
            @name="item-label"
            @implementation="function"
            @execution="ast-validation"
            @function="item-label-result"
            @select="item"
            @match="true" |
            {inputs |
                {input-binding @name="candidate" @type="schema:node" @source="candidate" @required=true}
            }
            {result @type="schema:diagnostic-result" @source-range="candidate"}
            {function @name="item-label-result" @returns="object" @deterministic=true |
                {param @name="candidate" @type="object" @required=true}
                {param @name="expected" @type="string" @required=true}
                {body | {$ { message: "Item label is required" } }}
            }
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.item_label"
            @severity="error"
            @behavior="item-label"
        }
    }
}"#,
        );

        assert!(model.compile_diagnostics.iter().any(|diagnostic| {
            diagnostic.code == INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE
                && diagnostic.details.as_ref().is_some_and(|details| {
                    details["diagnostic"] == "example.item_label"
                        && details["behavior"] == "item-label"
                        && details["function"] == "item-label-result"
                        && details["parameter"] == "expected"
                        && details["checkKind"] == "behavior-function-parameter-binding"
                })
        }));
        assert!(model
            .diagnostic_behaviors
            .get("example.item_label")
            .is_some_and(|behavior| behavior.function.is_none()));
    }

    #[test]
    fn schema_document_validation_surfaces_diagnostic_behavior_compile_errors() {
        let schema_language_model =
            load_builtin_document_model_for_identity(Some(CEM_SCHEMA_URI), None).unwrap();
        let document = parse_cem_document(
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-behavior" @namespace="https://example.test/ns/invalid-behavior/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {elements |
        {element @name="item"}
    }
    {diagnostics |
        {diagnostic
            @code="example.invalid_behavior"
            @severity="error"
            @behavior="schema:not-an-engine-behavior"
        }
    }
}"#,
        );

        let diagnostics = validate_document_model(&document, &schema_language_model);

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == UNKNOWN_DIAGNOSTIC_BEHAVIOR_CODE
                && diagnostic
                    .source_map
                    .as_ref()
                    .is_some_and(|source_map| !source_map.frames.is_empty())
                && diagnostic.details.as_ref().is_some_and(|details| {
                    details["behavior"] == "schema:not-an-engine-behavior"
                        && details["checkKind"] == "diagnostic-behavior-resolution"
                })
        }));
    }

    #[test]
    fn schema_attribute_values_drive_validation_from_cem_source() {
        let model = compile_document_model(
            "https://example.test/ns/value-contracts/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="value-contracts" @namespace="https://example.test/ns/value-contracts/1" @version="1.0.0" |
    {elements |
        {element @name="item" @optional-attributes="mode"}
    }
    {attributes |
        {attribute @name="mode" @type="schema:identifier" @values="compact pretty"}
    }
}"#,
        );
        let document = parse_cem_document(r#"{item @mode=tabular}"#);

        let diagnostics = validate_document_model(&document, &model);

        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == INVALID_ATTRIBUTE_VALUE_CODE)
            .expect("attribute value diagnostic");
        assert!(diagnostic.message.contains("mode"));
        assert!(diagnostic.message.contains("tabular"));
        let details = diagnostic
            .details
            .as_ref()
            .expect("attribute value details");
        assert_eq!(
            details["schemaUri"],
            serde_json::json!("https://example.test/ns/value-contracts/1")
        );
        assert_eq!(details["element"], serde_json::json!("item"));
        assert_eq!(details["attribute"], serde_json::json!("mode"));
        assert_eq!(
            details["contract"],
            serde_json::json!("attribute-values:mode")
        );
        assert_eq!(details["checkKind"], serde_json::json!("value-vocabulary"));
        assert_eq!(
            details["expectedValues"],
            serde_json::json!(["compact", "pretty"])
        );
        assert_eq!(details["actualValue"], serde_json::json!("tabular"));
        assert_eq!(details["invalidFields"], serde_json::json!(["mode"]));
        assert_eq!(
            details["actualValues"]["mode"],
            serde_json::json!("tabular")
        );
        assert!(details["sourceRange"]["span"]["start"].is_u64());
    }

    #[test]
    fn schema_attribute_diagnostic_behavior_aliases_drive_engine_checks() {
        let model = compile_document_model(
            "https://example.test/ns/attribute-behavior-alias/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="attribute-behavior-alias" @namespace="https://example.test/ns/attribute-behavior-alias/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {elements |
        {element @name="item" @optional-attributes="mode enabled count"}
    }
    {attributes |
        {attribute
            @name="mode"
            @type="schema:identifier"
            @values="compact pretty"
            @values-diagnostic="example.mode_value"
        }
        {attribute
            @name="enabled"
            @type="schema:boolean"
            @type-diagnostic="example.enabled_type"
        }
        {attribute
            @name="count"
            @type="schema:integer"
            @minInclusive=1
            @datatype-param-diagnostic="example.count_min"
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.mode_value"
            @severity="warning"
            @behavior="schema:value-vocabulary"
            @message="Mode must use the declared vocabulary"
        }
        {diagnostic
            @code="example.enabled_type"
            @severity="info"
            @behavior="schema:scalar-type"
            @message="Enabled must be boolean"
        }
        {diagnostic
            @code="example.count_min"
            @severity="error"
            @behavior="schema:datatype-param"
            @message="Count must satisfy its datatype parameters"
        }
    }
}"#,
        );

        assert!(
            model.compile_diagnostics.is_empty(),
            "attribute behavior aliases must compile: {:#?}",
            model.compile_diagnostics
        );
        assert_eq!(
            model
                .diagnostic_behaviors
                .get("example.mode_value")
                .map(|behavior| behavior.engine_behavior),
            Some(Some(EngineDiagnosticBehavior::ValueVocabulary))
        );

        let document = parse_cem_document(r#"{item @mode=tabular @enabled=maybe @count=0}"#);
        let diagnostics = validate_document_model(&document, &model);

        let value = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "example.mode_value")
            .expect("value-vocabulary alias diagnostic");
        assert_eq!(value.severity, Severity::Warning);
        assert!(value
            .message
            .starts_with("Mode must use the declared vocabulary:"));
        let details = value
            .details
            .as_ref()
            .expect("value-vocabulary alias details");
        assert_eq!(
            details["behavior"],
            serde_json::json!("schema:value-vocabulary")
        );
        assert_eq!(
            details["diagnostic"],
            serde_json::json!("example.mode_value")
        );
        assert_eq!(details["checkKind"], serde_json::json!("value-vocabulary"));
        assert_eq!(
            details["expectedValues"],
            serde_json::json!(["compact", "pretty"])
        );
        assert_eq!(details["actualValue"], serde_json::json!("tabular"));

        let scalar_type = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "example.enabled_type")
            .expect("scalar-type alias diagnostic");
        assert_eq!(scalar_type.severity, Severity::Info);
        assert!(scalar_type.message.starts_with("Enabled must be boolean:"));
        let details = scalar_type
            .details
            .as_ref()
            .expect("scalar-type alias details");
        assert_eq!(details["behavior"], serde_json::json!("schema:scalar-type"));
        assert_eq!(
            details["diagnostic"],
            serde_json::json!("example.enabled_type")
        );
        assert_eq!(details["checkKind"], serde_json::json!("type:boolean"));
        assert_eq!(details["expectedType"], serde_json::json!("schema:boolean"));
        assert_eq!(details["actualValue"], serde_json::json!("maybe"));

        let datatype_param = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "example.count_min")
            .expect("datatype-param alias diagnostic");
        assert_eq!(datatype_param.severity, Severity::Error);
        assert!(datatype_param
            .message
            .starts_with("Count must satisfy its datatype parameters:"));
        let details = datatype_param
            .details
            .as_ref()
            .expect("datatype-param alias details");
        assert_eq!(
            details["behavior"],
            serde_json::json!("schema:datatype-param")
        );
        assert_eq!(
            details["diagnostic"],
            serde_json::json!("example.count_min")
        );
        assert_eq!(
            details["checkKind"],
            serde_json::json!("datatype-param:minInclusive")
        );
        assert_eq!(details["datatypeParam"], serde_json::json!("minInclusive"));
        assert_eq!(details["minInclusive"], serde_json::json!("1"));
        assert_eq!(details["actualValue"], serde_json::json!("0"));
    }

    #[test]
    fn schema_attribute_diagnostic_behavior_rejects_unresolved_reference() {
        let model = compile_document_model(
            "https://example.test/ns/attribute-behavior-missing/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="attribute-behavior-missing" @namespace="https://example.test/ns/attribute-behavior-missing/1" @version="1.0.0" |
    {elements |
        {element @name="item" @optional-attributes="mode"}
    }
    {attributes |
        {attribute
            @name="mode"
            @type="schema:identifier"
            @values="compact pretty"
            @values-diagnostic="example.mode_value"
        }
    }
}"#,
        );

        assert!(model.compile_diagnostics.iter().any(|diagnostic| {
            diagnostic.code == UNRESOLVED_DIAGNOSTIC_REFERENCE_CODE
                && diagnostic.details.as_ref().is_some_and(|details| {
                    details["attribute"] == "mode"
                        && details["diagnostic"] == "example.mode_value"
                        && details["diagnosticAttribute"] == "values-diagnostic"
                        && details["checkKind"] == "diagnostic-reference-resolution"
                })
        }));
    }

    #[test]
    fn schema_attribute_diagnostic_behavior_rejects_incompatible_engine_reference() {
        let model = compile_document_model(
            "https://example.test/ns/attribute-behavior-incompatible/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="attribute-behavior-incompatible" @namespace="https://example.test/ns/attribute-behavior-incompatible/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {elements |
        {element @name="item" @optional-attributes="mode"}
    }
    {attributes |
        {attribute
            @name="mode"
            @type="schema:identifier"
            @values="compact pretty"
            @values-diagnostic="example.mode_value"
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.mode_value"
            @severity="error"
            @behavior="schema:scalar-type"
        }
    }
}"#,
        );

        assert!(model.compile_diagnostics.iter().any(|diagnostic| {
            diagnostic.code == INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE
                && diagnostic.details.as_ref().is_some_and(|details| {
                    details["attribute"] == "mode"
                        && details["diagnostic"] == "example.mode_value"
                        && details["diagnosticAttribute"] == "values-diagnostic"
                        && details["behavior"] == "schema:scalar-type"
                        && details["expectedBehavior"] == "schema:value-vocabulary"
                        && details["checkKind"] == "diagnostic-behavior-contract"
                })
        }));
    }

    #[test]
    fn schema_boolean_attribute_type_drives_validation_from_cem_source() {
        let model = compile_document_model(
            "https://example.test/ns/type-contracts/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="type-contracts" @namespace="https://example.test/ns/type-contracts/1" @version="1.0.0" |
    {elements |
        {element @name="item" @optional-attributes="enabled"}
    }
    {attributes |
        {attribute @name="enabled" @type="schema:boolean"}
    }
}"#,
        );
        for source in [
            r#"{item @enabled}"#,
            r#"{item @enabled=true}"#,
            r#"{item @enabled=false}"#,
        ] {
            let document = parse_cem_document(source);
            let diagnostics = validate_document_model(&document, &model);
            assert!(
                !diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == INVALID_ATTRIBUTE_TYPE_CODE),
                "valid boolean source produced type diagnostics: {source}: {diagnostics:?}"
            );
        }

        let document = parse_cem_document(r#"{item @enabled=maybe}"#);
        let diagnostics = validate_document_model(&document, &model);

        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == INVALID_ATTRIBUTE_TYPE_CODE)
            .expect("attribute type diagnostic");
        assert!(diagnostic.message.contains("enabled"));
        assert!(diagnostic.message.contains("maybe"));
        let details = diagnostic.details.as_ref().expect("attribute type details");
        assert_eq!(
            details["schemaUri"],
            serde_json::json!("https://example.test/ns/type-contracts/1")
        );
        assert_eq!(details["element"], serde_json::json!("item"));
        assert_eq!(details["attribute"], serde_json::json!("enabled"));
        assert_eq!(
            details["contract"],
            serde_json::json!("attribute-type:enabled")
        );
        assert_eq!(details["checkKind"], serde_json::json!("type:boolean"));
        assert_eq!(details["expectedType"], serde_json::json!("schema:boolean"));
        assert_eq!(
            details["expectedValues"],
            serde_json::json!(["false", "true"])
        );
        assert_eq!(details["allowsEmpty"], serde_json::json!(true));
        assert_eq!(details["actualValue"], serde_json::json!("maybe"));
        assert_eq!(details["invalidFields"], serde_json::json!(["enabled"]));
        assert_eq!(
            details["actualValues"]["enabled"],
            serde_json::json!("maybe")
        );
        assert!(details["sourceRange"]["span"]["start"].is_u64());
    }

    #[test]
    fn schema_integer_attribute_type_drives_validation_from_cem_source() {
        let model = compile_document_model(
            "https://example.test/ns/integer-contracts/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="integer-contracts" @namespace="https://example.test/ns/integer-contracts/1" @version="1.0.0" |
    {elements |
        {element @name="item" @optional-attributes="count"}
    }
    {attributes |
        {attribute @name="count" @type="schema:integer"}
    }
}"#,
        );
        for source in [
            r#"{item @count=0}"#,
            r#"{item @count=-12}"#,
            r#"{item @count=+12}"#,
            r#"{item @count=120000000000000000000000000000}"#,
        ] {
            let document = parse_cem_document(source);
            let diagnostics = validate_document_model(&document, &model);
            assert!(
                !diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == INVALID_ATTRIBUTE_TYPE_CODE),
                "valid integer source produced type diagnostics: {source}: {diagnostics:?}"
            );
        }

        for source in [r#"{item @count}"#, r#"{item @count=1.5}"#] {
            let document = parse_cem_document(source);
            let diagnostics = validate_document_model(&document, &model);
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == INVALID_ATTRIBUTE_TYPE_CODE),
                "invalid integer source did not produce type diagnostic: {source}: {diagnostics:?}"
            );
        }

        let document = parse_cem_document(r#"{item @count=1.5}"#);
        let diagnostics = validate_document_model(&document, &model);
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == INVALID_ATTRIBUTE_TYPE_CODE)
            .expect("attribute type diagnostic");
        assert!(diagnostic.message.contains("count"));
        assert!(diagnostic.message.contains("1.5"));
        let details = diagnostic.details.as_ref().expect("attribute type details");
        assert_eq!(
            details["schemaUri"],
            serde_json::json!("https://example.test/ns/integer-contracts/1")
        );
        assert_eq!(details["element"], serde_json::json!("item"));
        assert_eq!(details["attribute"], serde_json::json!("count"));
        assert_eq!(
            details["contract"],
            serde_json::json!("attribute-type:count")
        );
        assert_eq!(details["checkKind"], serde_json::json!("type:integer"));
        assert_eq!(details["expectedType"], serde_json::json!("schema:integer"));
        assert_eq!(
            details["expectedPattern"],
            serde_json::json!("signed decimal integer")
        );
        assert_eq!(details["allowsEmpty"], serde_json::json!(false));
        assert_eq!(details["actualValue"], serde_json::json!("1.5"));
        assert_eq!(details["invalidFields"], serde_json::json!(["count"]));
        assert_eq!(details["actualValues"]["count"], serde_json::json!("1.5"));
        assert!(details["sourceRange"]["span"]["start"].is_u64());
    }

    #[test]
    fn schema_min_inclusive_datatype_param_drives_validation_from_cem_source() {
        let model = compile_document_model(
            "https://example.test/ns/datatype-param-contracts/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="datatype-param-contracts" @namespace="https://example.test/ns/datatype-param-contracts/1" @version="1.0.0" |
    {elements |
        {element @name="item" @optional-attributes="count"}
    }
    {attributes |
        {attribute @name="count" @type="schema:integer" @minInclusive=1}
    }
}"#,
        );
        for source in [
            r#"{item @count=1}"#,
            r#"{item @count=+1}"#,
            r#"{item @count=120000000000000000000000000000}"#,
        ] {
            let document = parse_cem_document(source);
            let diagnostics = validate_document_model(&document, &model);
            assert!(
                !diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == INVALID_ATTRIBUTE_DATATYPE_PARAM_CODE),
                "valid minInclusive source produced diagnostics: {source}: {diagnostics:?}"
            );
        }

        for source in [r#"{item @count=0}"#, r#"{item @count=-12}"#] {
            let document = parse_cem_document(source);
            let diagnostics = validate_document_model(&document, &model);
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == INVALID_ATTRIBUTE_DATATYPE_PARAM_CODE),
                "invalid minInclusive source did not produce diagnostic: {source}: {diagnostics:?}"
            );
        }

        let document = parse_cem_document(r#"{item @count=0}"#);
        let diagnostics = validate_document_model(&document, &model);
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == INVALID_ATTRIBUTE_DATATYPE_PARAM_CODE)
            .expect("attribute datatype param diagnostic");
        assert!(diagnostic.message.contains("count"));
        assert!(diagnostic.message.contains("minInclusive"));
        let details = diagnostic
            .details
            .as_ref()
            .expect("attribute datatype param details");
        assert_eq!(
            details["schemaUri"],
            serde_json::json!("https://example.test/ns/datatype-param-contracts/1")
        );
        assert_eq!(details["element"], serde_json::json!("item"));
        assert_eq!(details["attribute"], serde_json::json!("count"));
        assert_eq!(
            details["contract"],
            serde_json::json!("attribute-datatype-param:count:minInclusive")
        );
        assert_eq!(
            details["checkKind"],
            serde_json::json!("datatype-param:minInclusive")
        );
        assert_eq!(details["datatypeParam"], serde_json::json!("minInclusive"));
        assert_eq!(details["minInclusive"], serde_json::json!("1"));
        assert_eq!(details["actualValue"], serde_json::json!("0"));
        assert_eq!(details["invalidFields"], serde_json::json!(["count"]));
        assert_eq!(details["actualValues"]["count"], serde_json::json!("0"));
        assert!(details["sourceRange"]["span"]["start"].is_u64());
    }

    #[test]
    fn loads_native_template_document_model_from_schema_uri() {
        let model =
            load_builtin_document_model_for_identity(Some(CEM_NATIVE_TEMPLATE_SCHEMA_URI), None)
                .unwrap();

        let template = model.element("template").unwrap();
        assert!(template.required_attributes.contains("name"));
        assert!(template.child_elements.contains("body"));
        let call = model.element("call").unwrap();
        assert!(call.optional_attributes.contains("with:*"));
    }

    #[test]
    fn loads_transform_document_model_with_template_base_elements() {
        let model =
            load_builtin_document_model_for_identity(Some(CEM_TRANSFORM_SCHEMA_URI), None).unwrap();

        let template = model.element("template").unwrap();
        assert!(template.required_attributes.contains("name"));
        assert!(template.child_elements.contains("body"));
        let call = model.element("call").unwrap();
        assert!(call.required_attributes.contains("template"));
        assert!(call.optional_attributes.contains("with:*"));
        let body = model.element("body").unwrap();
        assert!(body.allow_any_child);
        let function = model.element("function").unwrap();
        assert!(function.required_attributes.contains("name"));
        assert!(function.required_attributes.contains("returns"));
        assert!(function.optional_attributes.contains("deterministic"));
        assert!(function.child_elements.contains("param"));
        assert!(function.child_elements.contains("body"));
        let encoding_function = model.element("encoding-function").unwrap();
        assert!(encoding_function.required_attributes.contains("name"));
        assert!(encoding_function.required_attributes.contains("category"));
        assert!(encoding_function
            .required_attributes
            .contains("content-type"));
        assert!(encoding_function
            .optional_attributes
            .contains("implementation"));
        assert!(encoding_function.child_elements.contains("param"));
        assert!(encoding_function.child_elements.contains("body"));
        let format_function = model.element("format-function").unwrap();
        assert!(format_function.required_attributes.contains("produces"));
        let color_function = model.element("color-function").unwrap();
        assert!(color_function.optional_attributes.contains("capability"));
    }

    #[test]
    fn ambiguous_generic_cem_content_type_does_not_select_a_model() {
        assert!(
            load_builtin_document_model_for_identity(None, Some(CEM_ML_CONTENT_TYPE)).is_none()
        );
    }

    #[test]
    fn non_bootstrap_schema_content_type_does_not_select_a_model_yet() {
        assert!(load_builtin_document_model_for_identity(None, Some(HTML_CONTENT_TYPE)).is_none());
    }

    #[test]
    fn validates_native_template_wildcard_attributes() {
        let model =
            load_builtin_document_model_for_identity(None, Some(CEM_NATIVE_TEMPLATE_CONTENT_TYPE))
                .unwrap();
        let document = parse_cem_document(
            r#"@doc cem-ml 1
@ns template = "https://cem.dev/ns/template/cem-native/1"
@default template

{module |
    {template @name="page" |
        {body | {call @template="heading" @with:title="Welcome"}}
    }
    {template @name="heading" |
        {body | {h1 | Heading}}
    }
}"#,
        );

        let diagnostics = validate_document_model(&document, &model);

        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }

    #[test]
    fn validates_transform_template_inherited_model() {
        let model =
            load_builtin_document_model_for_identity(None, Some(CEM_TRANSFORM_CONTENT_TYPE))
                .unwrap();
        let document = parse_cem_document(
            r#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module |
    {template @name="main" |
        {body |
            {call @template="card" @with:title="Welcome"}
            {article | Output content}
        }
    }
    {template @name="card" |
        {body | {section | Card}}
    }
}"#,
        );

        let diagnostics = validate_document_model(&document, &model);

        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }

    #[test]
    fn validates_transform_template_inherited_required_attribute() {
        let model =
            load_builtin_document_model_for_identity(None, Some(CEM_TRANSFORM_CONTENT_TYPE))
                .unwrap();
        let document = parse_cem_document(
            r#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module |
    {template |
        {body | Missing required template name.}
    }
}"#,
        );

        let diagnostics = validate_document_model(&document, &model);

        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code
            == MISSING_REQUIRED_ATTRIBUTE_CODE
            && diagnostic.message.contains("name")));
    }

    #[test]
    fn validates_missing_required_attribute() {
        let model = load_builtin_document_model_for_identity(Some(CEM_SCHEMA_URI), None).unwrap();
        let document = parse_cem_document(
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="broken" @version="1.0.0"}"#,
        );

        let diagnostics = validate_document_model(&document, &model);

        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code
            == MISSING_REQUIRED_ATTRIBUTE_CODE
            && diagnostic.message.contains("namespace")));
    }

    #[test]
    fn validates_unknown_attribute() {
        let model = load_builtin_document_model_for_identity(Some(CEM_SCHEMA_URI), None).unwrap();
        let document = parse_cem_document(
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="broken" @namespace="https://example.test/ns/broken/1" @version="1.0.0" @extra=true}"#,
        );

        let diagnostics = validate_document_model(&document, &model);

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == UNKNOWN_ATTRIBUTE_CODE
                && diagnostic.message.contains("extra")));
    }

    #[test]
    fn validates_invalid_child_element() {
        let model =
            load_builtin_document_model_for_identity(None, Some(CEM_SCHEMA_CONTENT_TYPE)).unwrap();
        let document = parse_cem_document(
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="broken" @namespace="https://example.test/ns/broken/1" @version="1.0.0" |
    {unknown}
}"#,
        );

        let diagnostics = validate_document_model(&document, &model);

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == UNKNOWN_ELEMENT_CODE));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == INVALID_CHILD_ELEMENT_CODE));
    }

    #[test]
    fn validates_schema_package_missing_source() {
        let model =
            load_builtin_document_model_for_identity(None, Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE))
                .unwrap();
        let document = parse_cem_document(
            r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="broken" @version="1.0.0" |
    {schema @uri="https://example.test/ns/broken/1"}
}"#,
        );

        let diagnostics = validate_document_model(&document, &model);

        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code
            == MISSING_REQUIRED_ATTRIBUTE_CODE
            && diagnostic.message.contains("source")));
    }
}
