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
//! - schema-owned attribute `@values` and boolean type checks.
//!
//! Cardinality, ordering, non-boolean scalar type checks, and semantic
//! constraints remain follow-up work.

use std::collections::{BTreeMap, BTreeSet};

use crate::diagnostics::{Diagnostic, Severity};
use crate::events::cem::CemEventNormalizer;
use crate::parser::builder::CemAstBuilder;
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
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
pub const INVALID_CHILD_ELEMENT_CODE: &str = "cem.schema_model.invalid_child_element";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchemaDocumentModel {
    pub schema_uri: String,
    pub elements: BTreeMap<String, ElementModel>,
    pub attributes: BTreeMap<String, AttributeModel>,
}

impl SchemaDocumentModel {
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn element(&self, name: &str) -> Option<&ElementModel> {
        self.elements.get(name)
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
    pub when_attribute: Option<String>,
    pub when_values: BTreeSet<String>,
}

impl FieldContract {
    fn applies_to(&self, attributes: &BTreeMap<String, String>) -> bool {
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

    validate_field_contracts(
        &model.schema_uri,
        local,
        element_model,
        &seen_attributes,
        &attribute_values,
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
    element_name: &str,
    attribute_name: &str,
    value: &str,
    attribute_model: &AttributeModel,
    attribute_values: &BTreeMap<String, String>,
    node: &CemAstNode,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_attribute_type(
        schema_uri,
        element_name,
        attribute_name,
        value,
        attribute_model,
        attribute_values,
        node,
        diagnostics,
    );
    validate_attribute_value(
        schema_uri,
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
    element_name: &str,
    attribute_name: &str,
    value: &str,
    attribute_model: &AttributeModel,
    attribute_values: &BTreeMap<String, String>,
    node: &CemAstNode,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(value_type) = attribute_model.value_type.as_deref() else {
        return;
    };
    if !is_boolean_type_reference(value_type) {
        return;
    }

    let value = value.trim();
    if matches!(value, "" | "true" | "false") {
        return;
    }

    diagnostics.push(diag_at_with_details(
        INVALID_ATTRIBUTE_TYPE_CODE,
        format!(
            "attribute `{attribute_name}` on element `{element_name}` has value `{value}` outside schema-declared boolean values"
        ),
        node,
        attribute_type_details(
            schema_uri,
            element_name,
            attribute_name,
            value,
            attribute_model,
            attribute_values,
            node,
        ),
    ));
}

fn is_boolean_type_reference(value_type: &str) -> bool {
    let value_type = value_type.trim();
    value_type == "boolean"
        || value_type
            .rsplit_once(':')
            .is_some_and(|(_, local)| local == "boolean")
}

fn validate_attribute_value(
    schema_uri: &str,
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

    diagnostics.push(diag_at_with_details(
        INVALID_ATTRIBUTE_VALUE_CODE,
        format!(
            "attribute `{attribute_name}` on element `{element_name}` has value `{value}` outside schema-declared values: {}",
            attribute_model
                .allowed_values
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ),
        node,
        attribute_value_details(
            schema_uri,
            element_name,
            attribute_name,
            value,
            attribute_model,
            attribute_values,
            node,
        ),
    ));
}

fn compile_document_model(schema_uri: &str, schema_source: &str) -> SchemaDocumentModel {
    compile_document_model_with_seen(schema_uri, schema_source, &mut BTreeSet::new())
}

fn compile_document_model_with_seen(
    schema_uri: &str,
    schema_source: &str,
    seen_schema_uris: &mut BTreeSet<String>,
) -> SchemaDocumentModel {
    if !seen_schema_uris.insert(schema_uri.to_owned()) {
        return SchemaDocumentModel {
            schema_uri: schema_uri.to_owned(),
            elements: BTreeMap::new(),
            attributes: BTreeMap::new(),
        };
    }

    let document = parse_cem_document(schema_source);
    let mut model = SchemaDocumentModel {
        schema_uri: schema_uri.to_owned(),
        elements: BTreeMap::new(),
        attributes: BTreeMap::new(),
    };

    let Some(schema_id) = first_element_id_by_local_name(&document, "schema") else {
        seen_schema_uris.remove(schema_uri);
        return model;
    };
    let uses = collect_schema_uses(&document, schema_id);

    for elements_id in element_child_ids_by_local_name(&document, schema_id, "elements") {
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
                compile_element_model(&document, *child_id, &uses, seen_schema_uris)
            else {
                continue;
            };
            model
                .elements
                .insert(element_model.name.clone(), element_model);
        }
    }

    model.attributes = collect_attribute_models(&document, schema_id);

    for contract in collect_field_contracts(&document, schema_id) {
        if let Some(element_model) = model.elements.get_mut(&contract.target) {
            element_model.field_contracts.push(contract);
        }
    }

    seen_schema_uris.remove(schema_uri);
    model
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
                },
            );
        }
    }
    attributes
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
                when_attribute: optional_non_empty_attr(&attrs, "when-attribute")
                    .map(str::to_owned),
                when_values: parse_name_set(attrs.get("when-values")),
            });
        }
    }
    contracts
}

fn validate_field_contracts(
    schema_uri: &str,
    element_name: &str,
    element_model: &ElementModel,
    seen_attributes: &BTreeSet<String>,
    attribute_values: &BTreeMap<String, String>,
    node: &CemAstNode,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for contract in &element_model.field_contracts {
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

        if missing.is_empty() && forbidden.is_empty() {
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
        diagnostics.push(diag_at_with_details(
            contract.diagnostic_code(),
            format!(
                "element `{element_name}` failed field contract `{}` ({}) with {}",
                contract.name,
                contract.check_kind(),
                parts.join("; ")
            ),
            node,
            field_contract_details(
                schema_uri,
                element_name,
                contract,
                attribute_values,
                &missing,
                &forbidden,
                node,
            ),
        ));
    }
}

fn field_contract_details(
    schema_uri: &str,
    element_name: &str,
    contract: &FieldContract,
    attribute_values: &BTreeMap<String, String>,
    missing_fields: &[String],
    invalid_fields: &[String],
    node: &CemAstNode,
) -> serde_json::Value {
    serde_json::json!({
        "schemaUri": schema_uri,
        "element": element_name,
        "contract": &contract.name,
        "target": &contract.target,
        "diagnostic": contract.diagnostic_code(),
        "checkKind": contract.check_kind(),
        "requiredFields": &contract.required_attributes,
        "optionalFields": &contract.optional_attributes,
        "forbiddenFields": &contract.forbidden_attributes,
        "missingFields": missing_fields,
        "invalidFields": invalid_fields,
        "actualValues": attribute_values,
        "condition": {
            "attribute": &contract.when_attribute,
            "values": &contract.when_values,
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
    attribute_values: &BTreeMap<String, String>,
    node: &CemAstNode,
) -> serde_json::Value {
    serde_json::json!({
        "schemaUri": schema_uri,
        "element": element_name,
        "attribute": attribute_name,
        "contract": format!("attribute-values:{attribute_name}"),
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
    attribute_values: &BTreeMap<String, String>,
    node: &CemAstNode,
) -> serde_json::Value {
    let expected_type = attribute_model.value_type.as_deref().unwrap_or_default();
    serde_json::json!({
        "schemaUri": schema_uri,
        "element": element_name,
        "attribute": attribute_name,
        "contract": format!("attribute-type:{attribute_name}"),
        "checkKind": "type:boolean",
        "valueType": expected_type,
        "expectedType": expected_type,
        "expectedValues": ["false", "true"],
        "allowsEmpty": true,
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

fn diag_at_with_details(
    code: &str,
    message: String,
    node: &CemAstNode,
    details: serde_json::Value,
) -> Diagnostic {
    let mut diagnostic = diag_at(code, message, node);
    diagnostic.details = Some(details);
    diagnostic
}

fn diag_at(code: &str, message: String, node: &CemAstNode) -> Diagnostic {
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
        severity: Severity::Error,
        message,
        node: None,
        details: None,
        source_map: Some(stack.clone()),
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
    }

    #[test]
    fn loads_schema_package_document_model_from_content_type() {
        let model = load_builtin_document_model_for_identity(
            None,
            Some("application/vnd.cem.schema-package+cem; charset=utf-8"),
        )
        .unwrap();

        assert_eq!(model.schema_uri, CEM_SCHEMA_PACKAGE_URI);
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
    }

    #[test]
    fn schema_field_contracts_drive_validation_from_cem_source() {
        let model = compile_document_model(
            "https://example.test/ns/contracts/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="contracts" @namespace="https://example.test/ns/contracts/1" @version="1.0.0" |
    {elements |
        {element @name="item" @required-attributes="kind" @optional-attributes="kind name code"}
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
        assert!(details["sourceRange"]["span"]["start"].is_u64());
        assert!(!diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("kind-b-fields")));
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
