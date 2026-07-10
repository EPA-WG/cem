//! Schema package registry contracts.
//!
//! This is the first runtime surface for the schema-package registry design.
//! It records schema URL ownership, public content types, and namespace claims.
//! Conversion planning will build on this lookup layer in a later slice.

use std::collections::{BTreeMap, BTreeSet};

use crate::events::cem::CemEventNormalizer;
use crate::parser::builder::CemAstBuilder;
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
use crate::schema::package_sources::{builtin_schema_package_sources, BuiltinSchemaPackageSource};
use crate::source::{BytesSource, SourceId};
use crate::tokenizer::cem::CemTokenizer;

pub const CEM_ML_SCHEMA_URI: &str = "https://cem.dev/ns/cem-ml/1";
pub const CEM_SCHEMA_URI: &str = "https://cem.dev/ns/schema/1";
pub const CEM_SCHEMA_PACKAGE_URI: &str = "https://cem.dev/ns/schema-package/1";
pub const CEM_NATIVE_TEMPLATE_SCHEMA_URI: &str = "https://cem.dev/ns/template/cem-native/1";
pub const CEM_TRANSFORM_SCHEMA_URI: &str = "https://cem.dev/ns/transform/cem/1";
pub const CEM_QL_SCHEMA_URI: &str = "https://cem.dev/ns/query/cem-ql/1";
pub const JSON_VALUE_SCHEMA_URI: &str = "https://cem.dev/ns/data/json/1";
pub const JSON_SCHEMA_SCHEMA_URI: &str = "https://cem.dev/ns/data/json-schema/1";
pub const YAML_SCHEMA_URI: &str = "https://cem.dev/ns/data/yaml/1";
pub const CSV_SCHEMA_URI: &str = "https://cem.dev/ns/data/csv/1";
pub const MARKDOWN_SCHEMA_URI: &str = "https://cem.dev/ns/data/markdown/1";
pub const XML_SCHEMA_URI: &str = "https://cem.dev/ns/data/xml/1";
pub const RELAX_NG_SCHEMA_URI: &str = "https://cem.dev/ns/data/relax-ng/1";
pub const RELAX_NG_NAMESPACE_URI: &str = "http://relaxng.org/ns/structure/1.0";
pub const HTML_SCHEMA_URI: &str = "https://cem.dev/ns/data/html/1";
pub const HTML_NAMESPACE_URI: &str = "http://www.w3.org/1999/xhtml";
pub const CSS_SCHEMA_URI: &str = "https://cem.dev/ns/data/css/1";
pub const XHTML_SCHEMA_URI: &str = "https://cem.dev/ns/data/xhtml/1";
pub const XHTML_NAMESPACE_URI: &str = "http://www.w3.org/1999/xhtml";
pub const SVG_SCHEMA_URI: &str = "https://cem.dev/ns/data/svg/1";
pub const SVG_NAMESPACE_URI: &str = "http://www.w3.org/2000/svg";
pub const MATHML_SCHEMA_URI: &str = "https://cem.dev/ns/data/mathml/1";
pub const MATHML_NAMESPACE_URI: &str = "http://www.w3.org/1998/Math/MathML";
pub const XSLT_SCHEMA_URI: &str = "https://cem.dev/ns/transform/xslt/1";
pub const XSLT_NAMESPACE_URI: &str = "http://www.w3.org/1999/XSL/Transform";
pub const CEM_DOM_PROJECTION_SCHEMA_URI: &str = "https://cem.dev/ns/projection/dom/1";
pub const CEM_AST_PROJECTION_SCHEMA_URI: &str = "https://cem.dev/ns/projection/ast/1";
pub const CEM_EVENTS_PROJECTION_SCHEMA_URI: &str = "https://cem.dev/ns/projection/events/1";
pub const AI_CONTEXT_SCHEMA_URI: &str = "https://cem.dev/ns/ai-context/1";

pub const CEM_ML_CONTENT_TYPE: &str = "application/cem";
pub const CEM_SCHEMA_CONTENT_TYPE: &str = "application/vnd.cem.schema+cem";
pub const CEM_SCHEMA_PACKAGE_CONTENT_TYPE: &str = "application/vnd.cem.schema-package+cem";
pub const CEM_NATIVE_TEMPLATE_CONTENT_TYPE: &str = "application/vnd.cem.template+cem";
pub const CEM_TRANSFORM_CONTENT_TYPE: &str = "application/vnd.cem.transform+cem";
pub const CEM_QL_CONTENT_TYPE: &str = "application/vnd.cem.query+cem-ql";
pub const CEM_QL_ARTIFACT_CONTENT_TYPE: &str = "application/vnd.cem.query-artifact+cem-bin";
pub const JSON_CONTENT_TYPE: &str = "application/json";
pub const JSON_SCHEMA_CONTENT_TYPE: &str = "application/schema+json";
pub const YAML_CONTENT_TYPE: &str = "application/yaml";
pub const CSV_CONTENT_TYPE: &str = "text/csv";
pub const MARKDOWN_CONTENT_TYPE: &str = "text/markdown";
pub const XML_CONTENT_TYPE: &str = "application/xml";
pub const RELAX_NG_XML_CONTENT_TYPE: &str = "application/relax-ng+xml";
pub const RELAX_NG_COMPACT_CONTENT_TYPE: &str = "application/relax-ng-compact-syntax";
pub const HTML_CONTENT_TYPE: &str = "text/html";
pub const CSS_CONTENT_TYPE: &str = "text/css";
pub const XHTML_CONTENT_TYPE: &str = "application/xhtml+xml";
pub const SVG_CONTENT_TYPE: &str = "image/svg+xml";
pub const MATHML_CONTENT_TYPE: &str = "application/mathml+xml";
pub const XSLT_CONTENT_TYPE: &str = "application/xslt+xml";
pub const CEM_DOM_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.dom+cem-bin";
pub const CEM_DOM_JSON_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.dom+json";
pub const CEM_AST_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.ast+cem-bin";
pub const CEM_AST_JSON_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.ast+json";
pub const CEM_EVENTS_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.events+cem-bin";
pub const CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.events+json";
pub const AI_CONTEXT_JSON_CONTENT_TYPE: &str = "application/vnd.cem.ai-context+json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaContentTypeRole {
    Primary,
    Alias,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaContentType {
    pub value: String,
    pub essence: String,
    pub role: SchemaContentTypeRole,
}

impl SchemaContentType {
    pub fn primary(value: impl Into<String>) -> Self {
        Self::new(value, SchemaContentTypeRole::Primary)
    }

    pub fn alias(value: impl Into<String>) -> Self {
        Self::new(value, SchemaContentTypeRole::Alias)
    }

    pub fn new(value: impl Into<String>, role: SchemaContentTypeRole) -> Self {
        let value = value.into();
        let essence = content_type_essence(&value);
        Self {
            value,
            essence,
            role,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceClaim {
    pub prefix: Option<String>,
    pub uri: String,
}

impl NamespaceClaim {
    pub fn new(prefix: Option<impl Into<String>>, uri: impl Into<String>) -> Self {
        Self {
            prefix: prefix.map(Into::into),
            uri: uri.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDescriptor {
    pub package_id: String,
    pub schema_uri: String,
    pub version: String,
    pub source: String,
    pub content_types: Vec<SchemaContentType>,
    pub namespaces: Vec<NamespaceClaim>,
    pub uses: Vec<String>,
}

impl SchemaDescriptor {
    pub fn content_type_essences(&self) -> impl Iterator<Item = &str> {
        self.content_types
            .iter()
            .map(|content_type| content_type.essence.as_str())
    }

    pub fn namespace_uris(&self) -> impl Iterator<Item = &str> {
        self.namespaces
            .iter()
            .map(|namespace| namespace.uri.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaRegistryError {
    DuplicateSchemaUri {
        schema_uri: String,
    },
    ConflictingContentTypePrimary {
        content_type: String,
        existing_schema_uri: String,
        new_schema_uri: String,
    },
}

impl std::fmt::Display for SchemaRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateSchemaUri { schema_uri } => {
                write!(f, "schema URI `{schema_uri}` is already registered")
            }
            Self::ConflictingContentTypePrimary {
                content_type,
                existing_schema_uri,
                new_schema_uri,
            } => write!(
                f,
                "content type `{content_type}` already has primary schema `{existing_schema_uri}`, cannot register primary schema `{new_schema_uri}`"
            ),
        }
    }
}

impl std::error::Error for SchemaRegistryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaLookupError {
    UnknownContentType {
        content_type: String,
    },
    AmbiguousContentType {
        content_type: String,
        schema_uris: Vec<String>,
    },
}

impl std::fmt::Display for SchemaLookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownContentType { content_type } => {
                write!(f, "no schema registered for content type `{content_type}`")
            }
            Self::AmbiguousContentType {
                content_type,
                schema_uris,
            } => write!(
                f,
                "content type `{content_type}` is claimed by multiple schemas: {}",
                schema_uris.join(", ")
            ),
        }
    }
}

impl std::error::Error for SchemaLookupError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaPackageDescriptorError {
    MissingElement {
        element: &'static str,
    },
    MissingAttribute {
        element: &'static str,
        attribute: &'static str,
    },
    PackageIdMismatch {
        expected: String,
        actual: String,
    },
}

impl std::fmt::Display for SchemaPackageDescriptorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingElement { element } => {
                write!(f, "schema package manifest is missing `{element}` element")
            }
            Self::MissingAttribute { element, attribute } => {
                write!(
                    f,
                    "schema package manifest `{element}` element is missing `{attribute}`"
                )
            }
            Self::PackageIdMismatch { expected, actual } => write!(
                f,
                "embedded schema package source expected package id `{expected}`, got `{actual}`"
            ),
        }
    }
}

impl std::error::Error for SchemaPackageDescriptorError {}

#[derive(Debug, Clone, Default)]
pub struct SchemaRegistry {
    schemas_by_uri: BTreeMap<String, SchemaDescriptor>,
    content_types: BTreeMap<String, BTreeSet<String>>,
    namespaces: BTreeMap<String, BTreeSet<String>>,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtin_schemas() -> Self {
        let mut registry = Self::new();
        for descriptor in builtin_schema_descriptors() {
            registry
                .register(descriptor)
                .expect("built-in schema descriptors must not conflict");
        }
        registry
    }

    pub fn register(&mut self, descriptor: SchemaDescriptor) -> Result<(), SchemaRegistryError> {
        if self.schemas_by_uri.contains_key(&descriptor.schema_uri) {
            return Err(SchemaRegistryError::DuplicateSchemaUri {
                schema_uri: descriptor.schema_uri,
            });
        }

        for content_type in &descriptor.content_types {
            if content_type.role == SchemaContentTypeRole::Primary {
                if let Some(existing_schema_uri) =
                    self.primary_schema_for_essence(&content_type.essence)
                {
                    return Err(SchemaRegistryError::ConflictingContentTypePrimary {
                        content_type: content_type.essence.clone(),
                        existing_schema_uri: existing_schema_uri.to_owned(),
                        new_schema_uri: descriptor.schema_uri.clone(),
                    });
                }
            }
        }

        let schema_uri = descriptor.schema_uri.clone();
        for content_type in &descriptor.content_types {
            self.content_types
                .entry(content_type.essence.clone())
                .or_default()
                .insert(schema_uri.clone());
        }
        for namespace in &descriptor.namespaces {
            self.namespaces
                .entry(namespace.uri.clone())
                .or_default()
                .insert(schema_uri.clone());
        }
        self.schemas_by_uri.insert(schema_uri, descriptor);
        Ok(())
    }

    pub fn schema(&self, schema_uri: &str) -> Option<&SchemaDescriptor> {
        self.schemas_by_uri.get(schema_uri)
    }

    pub fn schemas(&self) -> impl Iterator<Item = &SchemaDescriptor> {
        self.schemas_by_uri.values()
    }

    pub fn lookup_content_type(&self, content_type: &str) -> Vec<&SchemaDescriptor> {
        let essence = content_type_essence(content_type);
        self.content_types
            .get(&essence)
            .into_iter()
            .flat_map(|schema_uris| schema_uris.iter())
            .filter_map(|schema_uri| self.schemas_by_uri.get(schema_uri))
            .collect()
    }

    pub fn resolve_content_type(
        &self,
        content_type: &str,
    ) -> Result<&SchemaDescriptor, SchemaLookupError> {
        let matches = self.lookup_content_type(content_type);
        match matches.as_slice() {
            [] => Err(SchemaLookupError::UnknownContentType {
                content_type: content_type_essence(content_type),
            }),
            [descriptor] => Ok(descriptor),
            descriptors => Err(SchemaLookupError::AmbiguousContentType {
                content_type: content_type_essence(content_type),
                schema_uris: descriptors
                    .iter()
                    .map(|descriptor| descriptor.schema_uri.clone())
                    .collect(),
            }),
        }
    }

    pub fn lookup_namespace(&self, namespace_uri: &str) -> Vec<&SchemaDescriptor> {
        self.namespaces
            .get(namespace_uri)
            .into_iter()
            .flat_map(|schema_uris| schema_uris.iter())
            .filter_map(|schema_uri| self.schemas_by_uri.get(schema_uri))
            .collect()
    }

    pub fn content_type_essences(&self) -> impl Iterator<Item = &str> {
        self.content_types.keys().map(String::as_str)
    }

    fn primary_schema_for_essence(&self, essence: &str) -> Option<&str> {
        self.content_types.get(essence).and_then(|schema_uris| {
            schema_uris.iter().find_map(|schema_uri| {
                let descriptor = self.schemas_by_uri.get(schema_uri)?;
                descriptor
                    .content_types
                    .iter()
                    .any(|content_type| {
                        content_type.essence == essence
                            && content_type.role == SchemaContentTypeRole::Primary
                    })
                    .then_some(schema_uri.as_str())
            })
        })
    }
}

pub fn content_type_essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

pub fn builtin_schema_descriptors() -> Vec<SchemaDescriptor> {
    builtin_schema_package_sources()
        .iter()
        .map(|source| {
            schema_descriptor_from_package_sources(source)
                .expect("built-in schema package descriptor metadata must be valid")
        })
        .collect()
}

pub fn schema_descriptor_from_package_sources(
    source: &BuiltinSchemaPackageSource,
) -> Result<SchemaDescriptor, SchemaPackageDescriptorError> {
    let manifest = parse_cem_document(source.manifest_source);
    let schema = parse_cem_document(source.schema_source);

    let package_id = first_element_id_by_local_name(&manifest, "package")
        .ok_or(SchemaPackageDescriptorError::MissingElement { element: "package" })?;
    let package_attrs = collect_attrs(&manifest, package_id);
    let package_id_attr = required_attr(&package_attrs, "package", "id")?;
    if package_id_attr != source.package_id {
        return Err(SchemaPackageDescriptorError::PackageIdMismatch {
            expected: source.package_id.to_owned(),
            actual: package_id_attr.to_owned(),
        });
    }

    let package_version = required_attr(&package_attrs, "package", "version")?;
    let schema_id = element_child_ids_by_local_name(&manifest, package_id, "schema")
        .into_iter()
        .next()
        .ok_or(SchemaPackageDescriptorError::MissingElement { element: "schema" })?;
    let schema_attrs = collect_attrs(&manifest, schema_id);
    let schema_uri = required_attr(&schema_attrs, "schema", "uri")?;
    let schema_source = required_attr(&schema_attrs, "schema", "source")?;
    let version = optional_attr(&schema_attrs, "version").unwrap_or(package_version);

    Ok(SchemaDescriptor {
        package_id: package_id_attr.to_owned(),
        schema_uri: schema_uri.to_owned(),
        version: version.to_owned(),
        source: package_relative_path(package_id_attr, schema_source),
        content_types: collect_package_content_types(&manifest, package_id)?,
        namespaces: collect_package_namespaces(&manifest, package_id)?,
        uses: collect_schema_uses(&schema),
    })
}

fn collect_package_content_types(
    document: &CemDocument,
    package_id: AstNodeId,
) -> Result<Vec<SchemaContentType>, SchemaPackageDescriptorError> {
    element_child_ids_by_local_name(document, package_id, "content-type")
        .into_iter()
        .map(|node_id| {
            let attrs = collect_attrs(document, node_id);
            let value = required_attr(&attrs, "content-type", "value")?;
            let role = if manifest_bool_attr(&attrs, "primary") {
                SchemaContentTypeRole::Primary
            } else {
                SchemaContentTypeRole::Alias
            };
            Ok(SchemaContentType::new(value, role))
        })
        .collect()
}

fn collect_package_namespaces(
    document: &CemDocument,
    package_id: AstNodeId,
) -> Result<Vec<NamespaceClaim>, SchemaPackageDescriptorError> {
    element_child_ids_by_local_name(document, package_id, "namespace")
        .into_iter()
        .map(|node_id| {
            let attrs = collect_attrs(document, node_id);
            let uri = required_attr(&attrs, "namespace", "uri")?;
            Ok(NamespaceClaim {
                prefix: optional_attr(&attrs, "prefix").map(str::to_owned),
                uri: uri.to_owned(),
            })
        })
        .collect()
}

fn collect_schema_uses(document: &CemDocument) -> Vec<String> {
    let Some(schema_id) = first_element_id_by_local_name(document, "schema") else {
        return Vec::new();
    };
    let mut uses = Vec::new();
    for uses_id in element_child_ids_by_local_name(document, schema_id, "uses") {
        for use_id in element_child_ids_by_local_name(document, uses_id, "use") {
            let attrs = collect_attrs(document, use_id);
            let Some(schema_uri) = optional_attr(&attrs, "schema") else {
                continue;
            };
            if !schema_uri.is_empty() && !uses.iter().any(|existing| existing == schema_uri) {
                uses.push(schema_uri.to_owned());
            }
        }
    }
    uses
}

fn package_relative_path(package_id: &str, path: &str) -> String {
    let path = path.trim();
    if path.is_empty() || path.starts_with('/') || path.starts_with("schema-packages/") {
        return path.to_owned();
    }
    format!(
        "schema-packages/{}/v1/{}",
        package_id,
        path.trim_start_matches("./")
    )
}

fn manifest_bool_attr(attrs: &BTreeMap<String, String>, name: &str) -> bool {
    matches!(
        attrs.get(name).map(String::as_str).map(str::trim),
        Some("") | Some("true") | Some("1")
    )
}

fn required_attr<'a>(
    attrs: &'a BTreeMap<String, String>,
    element: &'static str,
    attribute: &'static str,
) -> Result<&'a str, SchemaPackageDescriptorError> {
    optional_attr(attrs, attribute)
        .ok_or(SchemaPackageDescriptorError::MissingAttribute { element, attribute })
}

fn optional_attr<'a>(attrs: &'a BTreeMap<String, String>, attribute: &str) -> Option<&'a str> {
    attrs
        .get(attribute)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::package_consistency::SCHEMA_PACKAGE_SOURCE_CONSISTENCY_CONSTRAINT_DIAGNOSTICS;
    use crate::validation::rules::SCHEMA_PACKAGE_CONVERTER_CONSTRAINT_DIAGNOSTICS;

    fn descriptor(schema_uri: &str, content_type: &str) -> SchemaDescriptor {
        SchemaDescriptor {
            package_id: schema_uri.rsplit('/').next().unwrap_or(schema_uri).into(),
            schema_uri: schema_uri.into(),
            version: "1.0.0".into(),
            source: "schema/test.cem".into(),
            content_types: vec![SchemaContentType::primary(content_type)],
            namespaces: Vec::new(),
            uses: Vec::new(),
        }
    }

    fn schema_package_schema_diagnostic_codes() -> BTreeSet<String> {
        let source = builtin_schema_package_sources()
            .iter()
            .find(|source| source.package_id == "schema-package")
            .expect("schema-package source");
        let document = parse_cem_document(source.schema_source);
        let schema_id = first_element_id_by_local_name(&document, "schema")
            .expect("schema-package schema root");
        let diagnostics_id = element_child_ids_by_local_name(&document, schema_id, "diagnostics")
            .into_iter()
            .next()
            .expect("schema-package diagnostics");

        element_child_ids_by_local_name(&document, diagnostics_id, "diagnostic")
            .into_iter()
            .filter_map(|diagnostic_id| {
                collect_attrs(&document, diagnostic_id)
                    .get("code")
                    .map(String::to_owned)
            })
            .collect()
    }

    fn schema_package_schema_constraint_kinds() -> BTreeSet<String> {
        let source = builtin_schema_package_sources()
            .iter()
            .find(|source| source.package_id == "schema-package")
            .expect("schema-package source");
        let document = parse_cem_document(source.schema_source);
        let schema_id = first_element_id_by_local_name(&document, "schema")
            .expect("schema-package schema root");
        let constraints_id = element_child_ids_by_local_name(&document, schema_id, "constraints")
            .into_iter()
            .next()
            .expect("schema-package constraints");

        element_child_ids_by_local_name(&document, constraints_id, "constraint")
            .into_iter()
            .filter_map(|constraint_id| {
                collect_attrs(&document, constraint_id)
                    .get("kind")
                    .map(String::to_owned)
            })
            .collect()
    }

    fn schema_package_diagnostic_code_literals(source: &str) -> BTreeSet<String> {
        let source = production_source_for_diagnostic_audit(source);
        let mut rest = source;
        let mut codes = BTreeSet::new();

        while let Some(index) = rest.find("\"cem.schema_package.") {
            rest = &rest[index + 1..];
            let Some(end_quote) = rest.find('"') else {
                break;
            };
            codes.insert(rest[..end_quote].to_owned());
            rest = &rest[end_quote + 1..];
        }

        codes
    }

    fn schema_package_diagnostic_codes_after_call(source: &str, call: &str) -> BTreeSet<String> {
        let source = production_source_for_diagnostic_audit(source);
        let mut rest = source;
        let mut codes = BTreeSet::new();

        while let Some(index) = rest.find(call) {
            rest = &rest[index + call.len()..];
            rest = rest.trim_start();
            let Some(after_quote) = rest.strip_prefix('"') else {
                continue;
            };
            let Some(end_quote) = after_quote.find('"') else {
                break;
            };
            let code = &after_quote[..end_quote];
            if code.starts_with("cem.schema_package.") {
                codes.insert(code.to_owned());
            }
            rest = &after_quote[end_quote + 1..];
        }

        codes
    }

    fn production_source_for_diagnostic_audit(source: &str) -> &str {
        source
            .split("\n#[cfg(test)]\nmod tests")
            .next()
            .expect("production source before tests")
    }

    fn emitted_schema_package_diagnostic_codes() -> BTreeSet<String> {
        let mut codes = schema_package_diagnostic_codes_after_call(
            include_str!("../validation/rules.rs"),
            "diag_at(",
        );
        codes.extend(schema_package_diagnostic_code_literals(include_str!(
            "../real.rs"
        )));
        codes.extend(schema_package_diagnostic_code_literals(include_str!(
            "package_consistency.rs"
        )));
        codes
    }

    fn emitted_schema_package_validation_diagnostic_codes() -> BTreeSet<String> {
        let mut codes = schema_package_diagnostic_codes_after_call(
            include_str!("../validation/rules.rs"),
            "diag_at(",
        );
        codes.extend(schema_package_diagnostic_codes_after_call(
            include_str!("package_consistency.rs"),
            "schema_package_consistency_diagnostic(",
        ));
        codes
    }

    #[test]
    fn content_type_essence_strips_parameters_and_lowercases() {
        assert_eq!(
            content_type_essence("Application/Vnd.Cem.Schema+Chem; charset=utf-8"),
            "application/vnd.cem.schema+chem"
        );
    }

    #[test]
    fn builtin_schema_descriptors_are_loaded_from_package_sources() {
        let descriptors = builtin_schema_descriptors();

        assert_eq!(descriptors.len(), builtin_schema_package_sources().len());

        let html = descriptors
            .iter()
            .find(|descriptor| descriptor.package_id == "html")
            .expect("HTML descriptor");
        assert_eq!(html.schema_uri, HTML_SCHEMA_URI);
        assert_eq!(html.source, "schema-packages/html/v1/schema/html.cem");
        assert_eq!(
            html.content_type_essences().collect::<Vec<_>>(),
            vec![HTML_CONTENT_TYPE]
        );
        assert_eq!(
            html.namespace_uris().collect::<Vec<_>>(),
            vec![
                HTML_SCHEMA_URI,
                SVG_SCHEMA_URI,
                MATHML_SCHEMA_URI,
                HTML_NAMESPACE_URI,
                SVG_NAMESPACE_URI,
                MATHML_NAMESPACE_URI,
            ]
        );
        assert_eq!(
            html.uses,
            vec![
                CEM_SCHEMA_URI.to_owned(),
                CEM_ML_SCHEMA_URI.to_owned(),
                SVG_SCHEMA_URI.to_owned(),
                MATHML_SCHEMA_URI.to_owned(),
            ]
        );

        let mathml = descriptors
            .iter()
            .find(|descriptor| descriptor.package_id == "mathml")
            .expect("MathML descriptor");
        assert_eq!(
            mathml.uses,
            vec![
                CEM_SCHEMA_URI.to_owned(),
                CEM_ML_SCHEMA_URI.to_owned(),
                XML_SCHEMA_URI.to_owned(),
            ]
        );
    }

    #[test]
    fn schema_package_schema_declares_runtime_diagnostic_codes() {
        let emitted = emitted_schema_package_diagnostic_codes();
        let declared = schema_package_schema_diagnostic_codes();
        let missing = emitted.difference(&declared).collect::<Vec<_>>();

        assert!(
            !emitted.is_empty(),
            "expected schema-package diagnostics emitted by runtime sources"
        );
        assert!(
            missing.is_empty(),
            "schema-package schema is missing runtime diagnostic declarations: {missing:?}"
        );
    }

    #[test]
    fn schema_package_schema_declares_runtime_constraint_kinds() {
        let emitted = emitted_schema_package_validation_diagnostic_codes();
        let implemented = SCHEMA_PACKAGE_CONVERTER_CONSTRAINT_DIAGNOSTICS
            .iter()
            .chain(SCHEMA_PACKAGE_SOURCE_CONSISTENCY_CONSTRAINT_DIAGNOSTICS.iter())
            .copied()
            .collect::<Vec<_>>();
        let implemented_diagnostics = implemented
            .iter()
            .map(|(diagnostic_code, _)| (*diagnostic_code).to_owned())
            .collect::<BTreeSet<_>>();
        let implemented_constraints = implemented
            .iter()
            .map(|(_, constraint_kind)| (*constraint_kind).to_owned())
            .collect::<BTreeSet<_>>();
        let declared = schema_package_schema_constraint_kinds();
        let missing_diagnostic_mappings = emitted
            .difference(&implemented_diagnostics)
            .collect::<Vec<_>>();
        let missing_constraint_declarations = implemented_constraints
            .difference(&declared)
            .collect::<Vec<_>>();

        assert!(
            !emitted.is_empty(),
            "expected schema-package validation diagnostics emitted by runtime validators"
        );
        assert!(
            missing_diagnostic_mappings.is_empty(),
            "schema-package validation diagnostics need constraint mappings: {missing_diagnostic_mappings:?}"
        );
        assert!(
            missing_constraint_declarations.is_empty(),
            "schema-package schema is missing runtime constraint declarations: {missing_constraint_declarations:?}"
        );
    }

    #[test]
    fn schema_package_schema_exposes_converter_template_validation_metadata() {
        let constraint_kinds = schema_package_schema_constraint_kinds();

        assert!(
            constraint_kinds.contains("converter-template-output-stage-contract"),
            "schema-package schema must publish the CEMT converter template output-stage constraint"
        );

        let diagnostic_codes = schema_package_schema_diagnostic_codes();

        for expected_code in [
            "cem.schema_package.converter_template_schema_missing",
            "cem.schema_package.converter_template_source_unreadable",
            "cem.schema_package.converter_template_contract_invalid",
        ] {
            assert!(
                diagnostic_codes.contains(expected_code),
                "schema-package schema must publish diagnostic code {expected_code}"
            );
        }
    }

    #[test]
    fn builtin_registry_resolves_unambiguous_primary_content_types() {
        let registry = SchemaRegistry::with_builtin_schemas();

        assert_eq!(
            registry
                .resolve_content_type(CEM_SCHEMA_PACKAGE_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            CEM_SCHEMA_PACKAGE_URI
        );
        assert_eq!(
            registry
                .resolve_content_type(CEM_NATIVE_TEMPLATE_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            CEM_NATIVE_TEMPLATE_SCHEMA_URI
        );
        assert_eq!(
            registry
                .resolve_content_type(CEM_TRANSFORM_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            CEM_TRANSFORM_SCHEMA_URI
        );
        assert_eq!(
            registry
                .resolve_content_type(CEM_QL_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            CEM_QL_SCHEMA_URI
        );
        assert_eq!(
            registry
                .resolve_content_type(JSON_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            JSON_VALUE_SCHEMA_URI
        );
        assert_eq!(
            registry
                .resolve_content_type(JSON_SCHEMA_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            JSON_SCHEMA_SCHEMA_URI
        );
        assert_eq!(
            registry
                .resolve_content_type(YAML_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            YAML_SCHEMA_URI
        );
        assert_eq!(
            registry
                .resolve_content_type(CSV_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            CSV_SCHEMA_URI
        );
        assert_eq!(
            registry
                .resolve_content_type(MARKDOWN_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            MARKDOWN_SCHEMA_URI
        );
        assert_eq!(
            registry
                .resolve_content_type(XML_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            XML_SCHEMA_URI
        );
        assert_eq!(
            registry
                .resolve_content_type(RELAX_NG_XML_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            RELAX_NG_SCHEMA_URI
        );
        assert_eq!(
            registry
                .resolve_content_type(HTML_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            HTML_SCHEMA_URI
        );
        assert_eq!(
            registry
                .resolve_content_type(CSS_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            CSS_SCHEMA_URI
        );
        assert_eq!(
            registry
                .resolve_content_type(XHTML_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            XHTML_SCHEMA_URI
        );
        assert_eq!(
            registry
                .resolve_content_type(SVG_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            SVG_SCHEMA_URI
        );
        assert_eq!(
            registry
                .resolve_content_type(MATHML_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            MATHML_SCHEMA_URI
        );
        assert_eq!(
            registry
                .resolve_content_type(XSLT_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            XSLT_SCHEMA_URI
        );
    }

    #[test]
    fn builtin_registry_resolves_cem_ql_alias_content_types() {
        let registry = SchemaRegistry::with_builtin_schemas();

        for content_type in [
            "text/cem-ql; charset=utf-8",
            CEM_QL_ARTIFACT_CONTENT_TYPE,
            "cem-ql/1",
            "cem-ql/module",
        ] {
            assert_eq!(
                registry
                    .resolve_content_type(content_type)
                    .unwrap()
                    .schema_uri,
                CEM_QL_SCHEMA_URI
            );
        }
    }

    #[test]
    fn builtin_registry_resolves_json_alias_content_types() {
        let registry = SchemaRegistry::with_builtin_schemas();

        assert_eq!(
            registry
                .resolve_content_type("text/json; charset=utf-8")
                .unwrap()
                .schema_uri,
            JSON_VALUE_SCHEMA_URI
        );
    }

    #[test]
    fn builtin_registry_resolves_json_schema_content_type_with_parameters() {
        let registry = SchemaRegistry::with_builtin_schemas();

        assert_eq!(
            registry
                .resolve_content_type("application/schema+json; charset=utf-8")
                .unwrap()
                .schema_uri,
            JSON_SCHEMA_SCHEMA_URI
        );
    }

    #[test]
    fn builtin_registry_resolves_yaml_alias_content_types() {
        let registry = SchemaRegistry::with_builtin_schemas();

        for content_type in [
            "application/yaml; charset=utf-8",
            "application/x-yaml",
            "text/yaml",
            "text/x-yaml; charset=utf-8",
        ] {
            assert_eq!(
                registry
                    .resolve_content_type(content_type)
                    .unwrap()
                    .schema_uri,
                YAML_SCHEMA_URI
            );
        }
    }

    #[test]
    fn builtin_registry_resolves_csv_content_type_with_parameters() {
        let registry = SchemaRegistry::with_builtin_schemas();

        for content_type in [
            "text/csv",
            "text/csv; charset=utf-8",
            "text/csv; header=present",
        ] {
            assert_eq!(
                registry
                    .resolve_content_type(content_type)
                    .unwrap()
                    .schema_uri,
                CSV_SCHEMA_URI
            );
        }
    }

    #[test]
    fn builtin_registry_resolves_markdown_content_type_with_parameters() {
        let registry = SchemaRegistry::with_builtin_schemas();

        for content_type in [
            "text/markdown",
            "text/markdown; charset=utf-8",
            "text/markdown; charset=utf-8; variant=CommonMark",
        ] {
            assert_eq!(
                registry
                    .resolve_content_type(content_type)
                    .unwrap()
                    .schema_uri,
                MARKDOWN_SCHEMA_URI
            );
        }
    }

    #[test]
    fn builtin_registry_resolves_xml_content_types() {
        let registry = SchemaRegistry::with_builtin_schemas();

        for content_type in [
            "application/xml",
            "application/xml; charset=utf-8",
            "text/xml",
            "text/xml; charset=utf-8",
            "application/xml-external-parsed-entity",
            "text/xml-external-parsed-entity",
            "application/xml-dtd",
        ] {
            assert_eq!(
                registry
                    .resolve_content_type(content_type)
                    .unwrap()
                    .schema_uri,
                XML_SCHEMA_URI
            );
        }
    }

    #[test]
    fn builtin_registry_resolves_relax_ng_content_types() {
        let registry = SchemaRegistry::with_builtin_schemas();

        for content_type in [
            "application/relax-ng+xml",
            "application/relax-ng+xml; charset=utf-8",
            "application/relax-ng-compact-syntax",
            "application/relax-ng-compact-syntax; charset=utf-8",
        ] {
            assert_eq!(
                registry
                    .resolve_content_type(content_type)
                    .unwrap()
                    .schema_uri,
                RELAX_NG_SCHEMA_URI
            );
        }
    }

    #[test]
    fn builtin_registry_resolves_xhtml_content_type_with_parameters() {
        let registry = SchemaRegistry::with_builtin_schemas();

        for content_type in [
            "application/xhtml+xml",
            "application/xhtml+xml; charset=utf-8",
            "application/xhtml+xml; charset=utf-8; profile=https://example.test/profile",
        ] {
            assert_eq!(
                registry
                    .resolve_content_type(content_type)
                    .unwrap()
                    .schema_uri,
                XHTML_SCHEMA_URI
            );
        }
    }

    #[test]
    fn builtin_registry_resolves_html_content_type_with_parameters() {
        let registry = SchemaRegistry::with_builtin_schemas();

        for content_type in [
            "text/html",
            "text/html; charset=utf-8",
            "TEXT/HTML; CHARSET=windows-1252",
        ] {
            assert_eq!(
                registry
                    .resolve_content_type(content_type)
                    .unwrap()
                    .schema_uri,
                HTML_SCHEMA_URI
            );
        }
    }

    #[test]
    fn builtin_registry_resolves_css_content_type_with_parameters() {
        let registry = SchemaRegistry::with_builtin_schemas();

        for content_type in [
            "text/css",
            "text/css; charset=utf-8",
            "TEXT/CSS; CHARSET=iso-8859-1",
        ] {
            assert_eq!(
                registry
                    .resolve_content_type(content_type)
                    .unwrap()
                    .schema_uri,
                CSS_SCHEMA_URI
            );
        }
    }

    #[test]
    fn builtin_registry_resolves_svg_content_type_with_parameters() {
        let registry = SchemaRegistry::with_builtin_schemas();

        for content_type in ["image/svg+xml", "image/svg+xml; charset=utf-8"] {
            assert_eq!(
                registry
                    .resolve_content_type(content_type)
                    .unwrap()
                    .schema_uri,
                SVG_SCHEMA_URI
            );
        }
    }

    #[test]
    fn builtin_registry_resolves_mathml_content_types() {
        let registry = SchemaRegistry::with_builtin_schemas();

        for content_type in [
            "application/mathml+xml",
            "application/mathml+xml; charset=utf-8",
            "application/mathml-presentation+xml",
            "application/mathml-content+xml; charset=utf-8",
        ] {
            assert_eq!(
                registry
                    .resolve_content_type(content_type)
                    .unwrap()
                    .schema_uri,
                MATHML_SCHEMA_URI
            );
        }
    }

    #[test]
    fn builtin_registry_resolves_xslt_content_types() {
        let registry = SchemaRegistry::with_builtin_schemas();

        for content_type in [
            "application/xslt+xml",
            "application/xslt+xml; charset=utf-8",
            "text/xsl",
            "custom-element-xslt",
            "text/custom-element-xslt",
            "application/custom-element-xslt",
            "text/x-custom-element-xslt",
        ] {
            assert_eq!(
                registry
                    .resolve_content_type(content_type)
                    .unwrap()
                    .schema_uri,
                XSLT_SCHEMA_URI
            );
        }
    }

    #[test]
    fn builtin_registry_resolves_projection_content_types() {
        let registry = SchemaRegistry::with_builtin_schemas();

        for (content_type, schema_uri) in [
            (
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            (
                CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            (
                CEM_AST_PROJECTION_CONTENT_TYPE,
                CEM_AST_PROJECTION_SCHEMA_URI,
            ),
            (
                CEM_AST_JSON_PROJECTION_CONTENT_TYPE,
                CEM_AST_PROJECTION_SCHEMA_URI,
            ),
            (
                CEM_EVENTS_PROJECTION_CONTENT_TYPE,
                CEM_EVENTS_PROJECTION_SCHEMA_URI,
            ),
            (
                CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
                CEM_EVENTS_PROJECTION_SCHEMA_URI,
            ),
        ] {
            assert_eq!(
                registry
                    .resolve_content_type(content_type)
                    .unwrap()
                    .schema_uri,
                schema_uri
            );
        }
    }

    #[test]
    fn builtin_registry_requires_schema_for_shared_cem_content_types() {
        let registry = SchemaRegistry::with_builtin_schemas();

        let error = registry
            .resolve_content_type("text/cem-ml; charset=utf-8")
            .unwrap_err();

        assert_eq!(
            error,
            SchemaLookupError::AmbiguousContentType {
                content_type: "text/cem-ml".into(),
                schema_uris: vec![
                    CEM_ML_SCHEMA_URI.into(),
                    CEM_NATIVE_TEMPLATE_SCHEMA_URI.into()
                ]
            }
        );
    }

    #[test]
    fn lookup_namespace_returns_claiming_schemas() {
        let registry = SchemaRegistry::with_builtin_schemas();
        let schema_uris = registry
            .lookup_namespace(CEM_SCHEMA_URI)
            .into_iter()
            .map(|descriptor| descriptor.schema_uri.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            schema_uris,
            vec![CEM_ML_SCHEMA_URI, CEM_SCHEMA_PACKAGE_URI, CEM_SCHEMA_URI]
        );
    }

    #[test]
    fn duplicate_schema_uri_is_rejected() {
        let mut registry = SchemaRegistry::new();
        registry
            .register(descriptor(
                "https://cem.dev/ns/test/1",
                "application/vnd.test+cem",
            ))
            .unwrap();

        let error = registry
            .register(descriptor(
                "https://cem.dev/ns/test/1",
                "application/vnd.other+cem",
            ))
            .unwrap_err();

        assert_eq!(
            error,
            SchemaRegistryError::DuplicateSchemaUri {
                schema_uri: "https://cem.dev/ns/test/1".into()
            }
        );
    }

    #[test]
    fn conflicting_primary_content_type_is_rejected() {
        let mut registry = SchemaRegistry::new();
        registry
            .register(descriptor(
                "https://cem.dev/ns/a/1",
                "application/vnd.same+cem",
            ))
            .unwrap();

        let error = registry
            .register(descriptor(
                "https://cem.dev/ns/b/1",
                "application/vnd.same+cem",
            ))
            .unwrap_err();

        assert_eq!(
            error,
            SchemaRegistryError::ConflictingContentTypePrimary {
                content_type: "application/vnd.same+cem".into(),
                existing_schema_uri: "https://cem.dev/ns/a/1".into(),
                new_schema_uri: "https://cem.dev/ns/b/1".into()
            }
        );
    }

    #[test]
    fn shared_content_type_claim_resolves_as_ambiguous() {
        let mut registry = SchemaRegistry::new();
        registry
            .register(descriptor(
                "https://cem.dev/ns/a/1",
                "application/vnd.shared+cem",
            ))
            .unwrap();
        registry
            .register(SchemaDescriptor {
                package_id: "b".into(),
                schema_uri: "https://cem.dev/ns/b/1".into(),
                version: "1.0.0".into(),
                source: "schema/b.cem".into(),
                content_types: vec![SchemaContentType::alias("application/vnd.shared+cem")],
                namespaces: Vec::new(),
                uses: Vec::new(),
            })
            .unwrap();

        let error = registry
            .resolve_content_type("application/vnd.shared+cem")
            .unwrap_err();

        assert_eq!(
            error,
            SchemaLookupError::AmbiguousContentType {
                content_type: "application/vnd.shared+cem".into(),
                schema_uris: vec![
                    "https://cem.dev/ns/a/1".into(),
                    "https://cem.dev/ns/b/1".into()
                ]
            }
        );
    }
}
