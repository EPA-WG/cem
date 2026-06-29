//! Schema package registry contracts.
//!
//! This is the first runtime surface for the schema-package registry design.
//! It records schema URL ownership, public content types, and namespace claims.
//! Conversion planning will build on this lookup layer in a later slice.

use std::collections::{BTreeMap, BTreeSet};

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
pub const XHTML_SCHEMA_URI: &str = "https://cem.dev/ns/data/xhtml/1";
pub const XHTML_NAMESPACE_URI: &str = "http://www.w3.org/1999/xhtml";
pub const CEM_DOM_PROJECTION_SCHEMA_URI: &str = "https://cem.dev/ns/projection/dom/1";
pub const CEM_AST_PROJECTION_SCHEMA_URI: &str = "https://cem.dev/ns/projection/ast/1";
pub const CEM_EVENTS_PROJECTION_SCHEMA_URI: &str = "https://cem.dev/ns/projection/events/1";

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
pub const XHTML_CONTENT_TYPE: &str = "application/xhtml+xml";
pub const CEM_DOM_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.dom+cem-bin";
pub const CEM_DOM_JSON_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.dom+json";
pub const CEM_AST_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.ast+cem-bin";
pub const CEM_AST_JSON_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.ast+json";
pub const CEM_EVENTS_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.events+cem-bin";
pub const CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.events+json";

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
    vec![
        SchemaDescriptor {
            package_id: "cem-ml".into(),
            schema_uri: CEM_ML_SCHEMA_URI.into(),
            version: "1.0.0".into(),
            source: "schema-packages/cem-ml/v1/schema/cem-ml-generic.cem".into(),
            content_types: vec![
                SchemaContentType::primary(CEM_ML_CONTENT_TYPE),
                SchemaContentType::alias("text/cem-ml"),
                SchemaContentType::alias("text/cem"),
                SchemaContentType::alias("application/cem+xml"),
            ],
            namespaces: vec![
                NamespaceClaim::new(Some("cemml"), CEM_ML_SCHEMA_URI),
                NamespaceClaim::new(Some("schema"), CEM_SCHEMA_URI),
            ],
            uses: Vec::new(),
        },
        SchemaDescriptor {
            package_id: "schema".into(),
            schema_uri: CEM_SCHEMA_URI.into(),
            version: "1.0.0".into(),
            source: "schema-packages/schema/v1/schema/cem-schema.cem".into(),
            content_types: vec![SchemaContentType::primary(CEM_SCHEMA_CONTENT_TYPE)],
            namespaces: vec![NamespaceClaim::new(Some("schema"), CEM_SCHEMA_URI)],
            uses: vec![CEM_ML_SCHEMA_URI.into()],
        },
        SchemaDescriptor {
            package_id: "schema-package".into(),
            schema_uri: CEM_SCHEMA_PACKAGE_URI.into(),
            version: "1.0.0".into(),
            source: "schema-packages/schema-package/v1/schema/schema-package.cem".into(),
            content_types: vec![SchemaContentType::primary(CEM_SCHEMA_PACKAGE_CONTENT_TYPE)],
            namespaces: vec![
                NamespaceClaim::new(Some("pkg"), CEM_SCHEMA_PACKAGE_URI),
                NamespaceClaim::new(Some("schema"), CEM_SCHEMA_URI),
            ],
            uses: vec![CEM_SCHEMA_URI.into()],
        },
        SchemaDescriptor {
            package_id: "cem-native-template".into(),
            schema_uri: CEM_NATIVE_TEMPLATE_SCHEMA_URI.into(),
            version: "1.0.0".into(),
            source: "schema-packages/cem-native-template/v1/schema/cem-native-template.cem".into(),
            content_types: vec![
                SchemaContentType::primary(CEM_NATIVE_TEMPLATE_CONTENT_TYPE),
                SchemaContentType::alias(CEM_ML_CONTENT_TYPE),
                SchemaContentType::alias("application/cem+xml"),
                SchemaContentType::alias("text/cem"),
                SchemaContentType::alias("text/cem-ml"),
            ],
            namespaces: vec![
                NamespaceClaim::new(Some("template"), CEM_NATIVE_TEMPLATE_SCHEMA_URI),
                NamespaceClaim::new(Some("cem"), "https://cem.dev/ns/core/1"),
            ],
            uses: vec![CEM_SCHEMA_URI.into(), CEM_ML_SCHEMA_URI.into()],
        },
        SchemaDescriptor {
            package_id: "cem-transform".into(),
            schema_uri: CEM_TRANSFORM_SCHEMA_URI.into(),
            version: "1.0.0".into(),
            source: "schema-packages/cem-transform/v1/schema/cem-transform.cem".into(),
            content_types: vec![SchemaContentType::primary(CEM_TRANSFORM_CONTENT_TYPE)],
            namespaces: vec![
                NamespaceClaim::new(Some("transform"), CEM_TRANSFORM_SCHEMA_URI),
                NamespaceClaim::new(Some("template"), CEM_NATIVE_TEMPLATE_SCHEMA_URI),
                NamespaceClaim::new(Some("cem"), "https://cem.dev/ns/core/1"),
            ],
            uses: vec![CEM_SCHEMA_URI.into(), CEM_NATIVE_TEMPLATE_SCHEMA_URI.into()],
        },
        SchemaDescriptor {
            package_id: "cem-ql".into(),
            schema_uri: CEM_QL_SCHEMA_URI.into(),
            version: "1.0.0".into(),
            source: "schema-packages/cem-ql/v1/schema/cem-ql.cem".into(),
            content_types: vec![
                SchemaContentType::primary(CEM_QL_CONTENT_TYPE),
                SchemaContentType::alias("text/cem-ql"),
                SchemaContentType::alias(CEM_QL_ARTIFACT_CONTENT_TYPE),
                SchemaContentType::alias("cem-ql/1"),
                SchemaContentType::alias("cem-ql/module"),
            ],
            namespaces: vec![NamespaceClaim::new(Some("ql"), CEM_QL_SCHEMA_URI)],
            uses: vec![CEM_SCHEMA_URI.into()],
        },
        SchemaDescriptor {
            package_id: "json".into(),
            schema_uri: JSON_VALUE_SCHEMA_URI.into(),
            version: "1.0.0".into(),
            source: "schema-packages/json/v1/schema/json.cem".into(),
            content_types: vec![
                SchemaContentType::primary(JSON_CONTENT_TYPE),
                SchemaContentType::alias("text/json"),
            ],
            namespaces: vec![NamespaceClaim::new(Some("json"), JSON_VALUE_SCHEMA_URI)],
            uses: vec![CEM_SCHEMA_URI.into()],
        },
        SchemaDescriptor {
            package_id: "yaml".into(),
            schema_uri: YAML_SCHEMA_URI.into(),
            version: "1.0.0".into(),
            source: "schema-packages/yaml/v1/schema/yaml.cem".into(),
            content_types: vec![
                SchemaContentType::primary(YAML_CONTENT_TYPE),
                SchemaContentType::alias("application/x-yaml"),
                SchemaContentType::alias("text/yaml"),
                SchemaContentType::alias("text/x-yaml"),
            ],
            namespaces: vec![NamespaceClaim::new(Some("yaml"), YAML_SCHEMA_URI)],
            uses: vec![CEM_SCHEMA_URI.into(), CEM_ML_SCHEMA_URI.into()],
        },
        SchemaDescriptor {
            package_id: "csv".into(),
            schema_uri: CSV_SCHEMA_URI.into(),
            version: "1.0.0".into(),
            source: "schema-packages/csv/v1/schema/csv.cem".into(),
            content_types: vec![SchemaContentType::primary(CSV_CONTENT_TYPE)],
            namespaces: vec![NamespaceClaim::new(Some("csv"), CSV_SCHEMA_URI)],
            uses: vec![CEM_SCHEMA_URI.into(), CEM_ML_SCHEMA_URI.into()],
        },
        SchemaDescriptor {
            package_id: "markdown".into(),
            schema_uri: MARKDOWN_SCHEMA_URI.into(),
            version: "1.0.0".into(),
            source: "schema-packages/markdown/v1/schema/markdown.cem".into(),
            content_types: vec![SchemaContentType::primary(MARKDOWN_CONTENT_TYPE)],
            namespaces: vec![NamespaceClaim::new(Some("markdown"), MARKDOWN_SCHEMA_URI)],
            uses: vec![CEM_SCHEMA_URI.into(), CEM_ML_SCHEMA_URI.into()],
        },
        SchemaDescriptor {
            package_id: "xml".into(),
            schema_uri: XML_SCHEMA_URI.into(),
            version: "1.0.0".into(),
            source: "schema-packages/xml/v1/schema/xml.cem".into(),
            content_types: vec![
                SchemaContentType::primary(XML_CONTENT_TYPE),
                SchemaContentType::alias("text/xml"),
                SchemaContentType::alias("application/xml-external-parsed-entity"),
                SchemaContentType::alias("text/xml-external-parsed-entity"),
                SchemaContentType::alias("application/xml-dtd"),
            ],
            namespaces: vec![NamespaceClaim::new(Some("xml"), XML_SCHEMA_URI)],
            uses: vec![CEM_SCHEMA_URI.into(), CEM_ML_SCHEMA_URI.into()],
        },
        SchemaDescriptor {
            package_id: "xhtml".into(),
            schema_uri: XHTML_SCHEMA_URI.into(),
            version: "1.0.0".into(),
            source: "schema-packages/xhtml/v1/schema/xhtml.cem".into(),
            content_types: vec![SchemaContentType::primary(XHTML_CONTENT_TYPE)],
            namespaces: vec![
                NamespaceClaim::new(Some("cemxhtml"), XHTML_SCHEMA_URI),
                NamespaceClaim::new(Some("xhtml"), XHTML_NAMESPACE_URI),
            ],
            uses: vec![
                CEM_SCHEMA_URI.into(),
                CEM_ML_SCHEMA_URI.into(),
                XML_SCHEMA_URI.into(),
            ],
        },
        SchemaDescriptor {
            package_id: "json-schema".into(),
            schema_uri: JSON_SCHEMA_SCHEMA_URI.into(),
            version: "1.0.0".into(),
            source: "schema-packages/json-schema/v1/schema/json-schema.cem".into(),
            content_types: vec![SchemaContentType::primary(JSON_SCHEMA_CONTENT_TYPE)],
            namespaces: vec![
                NamespaceClaim::new(Some("jsonschema"), JSON_SCHEMA_SCHEMA_URI),
                NamespaceClaim::new(Some("json"), JSON_VALUE_SCHEMA_URI),
            ],
            uses: vec![CEM_SCHEMA_URI.into(), JSON_VALUE_SCHEMA_URI.into()],
        },
        SchemaDescriptor {
            package_id: "cem-dom-projection".into(),
            schema_uri: CEM_DOM_PROJECTION_SCHEMA_URI.into(),
            version: "1.0.0".into(),
            source: "schema-packages/cem-dom-projection/v1/schema/cem-dom-projection.cem".into(),
            content_types: vec![
                SchemaContentType::primary(CEM_DOM_PROJECTION_CONTENT_TYPE),
                SchemaContentType::alias(CEM_DOM_JSON_PROJECTION_CONTENT_TYPE),
            ],
            namespaces: vec![NamespaceClaim::new(
                Some("cemdom"),
                CEM_DOM_PROJECTION_SCHEMA_URI,
            )],
            uses: vec![CEM_SCHEMA_URI.into(), CEM_ML_SCHEMA_URI.into()],
        },
        SchemaDescriptor {
            package_id: "cem-ast-projection".into(),
            schema_uri: CEM_AST_PROJECTION_SCHEMA_URI.into(),
            version: "1.0.0".into(),
            source: "schema-packages/cem-ast-projection/v1/schema/cem-ast-projection.cem".into(),
            content_types: vec![
                SchemaContentType::primary(CEM_AST_PROJECTION_CONTENT_TYPE),
                SchemaContentType::alias(CEM_AST_JSON_PROJECTION_CONTENT_TYPE),
            ],
            namespaces: vec![NamespaceClaim::new(
                Some("cemast"),
                CEM_AST_PROJECTION_SCHEMA_URI,
            )],
            uses: vec![CEM_SCHEMA_URI.into(), CEM_ML_SCHEMA_URI.into()],
        },
        SchemaDescriptor {
            package_id: "cem-events-projection".into(),
            schema_uri: CEM_EVENTS_PROJECTION_SCHEMA_URI.into(),
            version: "1.0.0".into(),
            source: "schema-packages/cem-events-projection/v1/schema/cem-events-projection.cem"
                .into(),
            content_types: vec![
                SchemaContentType::primary(CEM_EVENTS_PROJECTION_CONTENT_TYPE),
                SchemaContentType::alias(CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE),
            ],
            namespaces: vec![NamespaceClaim::new(
                Some("cemevents"),
                CEM_EVENTS_PROJECTION_SCHEMA_URI,
            )],
            uses: vec![CEM_SCHEMA_URI.into(), CEM_ML_SCHEMA_URI.into()],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn content_type_essence_strips_parameters_and_lowercases() {
        assert_eq!(
            content_type_essence("Application/Vnd.Cem.Schema+Chem; charset=utf-8"),
            "application/vnd.cem.schema+chem"
        );
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
                .resolve_content_type(XHTML_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            XHTML_SCHEMA_URI
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
