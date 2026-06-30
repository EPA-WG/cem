//! Embedded schema-package source loader.
//!
//! The registry owns identity lookup. This module owns loading the matching
//! built-in package sources so validators and tooling can resolve a schema URL
//! or content type to the actual `package.cem` manifest plus schema document.

use crate::schema::registry::{SchemaDescriptor, SchemaLookupError, SchemaRegistry};

#[derive(Debug, Clone)]
pub struct BuiltinSchemaPackage {
    pub descriptor: SchemaDescriptor,
    pub manifest_source: &'static str,
    pub schema_source: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuiltinSchemaPackageLoadError {
    UnknownSchemaUri {
        schema_uri: String,
    },
    MissingEmbeddedSource {
        package_id: String,
        schema_uri: String,
    },
    Lookup(SchemaLookupError),
}

impl std::fmt::Display for BuiltinSchemaPackageLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSchemaUri { schema_uri } => {
                write!(f, "unknown built-in schema URI `{schema_uri}`")
            }
            Self::MissingEmbeddedSource {
                package_id,
                schema_uri,
            } => write!(
                f,
                "built-in schema package `{package_id}` for `{schema_uri}` has no embedded source"
            ),
            Self::Lookup(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for BuiltinSchemaPackageLoadError {}

impl From<SchemaLookupError> for BuiltinSchemaPackageLoadError {
    fn from(error: SchemaLookupError) -> Self {
        Self::Lookup(error)
    }
}

pub fn load_builtin_schema_package(
    schema_uri: &str,
) -> Result<BuiltinSchemaPackage, BuiltinSchemaPackageLoadError> {
    let registry = SchemaRegistry::with_builtin_schemas();
    let descriptor = registry.schema(schema_uri).ok_or_else(|| {
        BuiltinSchemaPackageLoadError::UnknownSchemaUri {
            schema_uri: schema_uri.to_owned(),
        }
    })?;
    load_descriptor(descriptor)
}

pub fn load_builtin_schema_package_for_content_type(
    content_type: &str,
) -> Result<BuiltinSchemaPackage, BuiltinSchemaPackageLoadError> {
    let registry = SchemaRegistry::with_builtin_schemas();
    let descriptor = registry.resolve_content_type(content_type)?;
    load_descriptor(descriptor)
}

fn load_descriptor(
    descriptor: &SchemaDescriptor,
) -> Result<BuiltinSchemaPackage, BuiltinSchemaPackageLoadError> {
    let Some((manifest_source, schema_source)) = embedded_sources(&descriptor.package_id) else {
        return Err(BuiltinSchemaPackageLoadError::MissingEmbeddedSource {
            package_id: descriptor.package_id.clone(),
            schema_uri: descriptor.schema_uri.clone(),
        });
    };
    Ok(BuiltinSchemaPackage {
        descriptor: descriptor.clone(),
        manifest_source,
        schema_source,
    })
}

fn embedded_sources(package_id: &str) -> Option<(&'static str, &'static str)> {
    Some(match package_id {
        "cem-ml" => (
            include_str!("../../schema-packages/cem-ml/v1/package.cem"),
            include_str!("../../schema-packages/cem-ml/v1/schema/cem-ml-generic.cem"),
        ),
        "schema" => (
            include_str!("../../schema-packages/schema/v1/package.cem"),
            include_str!("../../schema-packages/schema/v1/schema/cem-schema.cem"),
        ),
        "schema-package" => (
            include_str!("../../schema-packages/schema-package/v1/package.cem"),
            include_str!("../../schema-packages/schema-package/v1/schema/schema-package.cem"),
        ),
        "cem-native-template" => (
            include_str!("../../schema-packages/cem-native-template/v1/package.cem"),
            include_str!(
                "../../schema-packages/cem-native-template/v1/schema/cem-native-template.cem"
            ),
        ),
        "cem-transform" => (
            include_str!("../../schema-packages/cem-transform/v1/package.cem"),
            include_str!("../../schema-packages/cem-transform/v1/schema/cem-transform.cem"),
        ),
        "cem-ql" => (
            include_str!("../../schema-packages/cem-ql/v1/package.cem"),
            include_str!("../../schema-packages/cem-ql/v1/schema/cem-ql.cem"),
        ),
        "json" => (
            include_str!("../../schema-packages/json/v1/package.cem"),
            include_str!("../../schema-packages/json/v1/schema/json.cem"),
        ),
        "yaml" => (
            include_str!("../../schema-packages/yaml/v1/package.cem"),
            include_str!("../../schema-packages/yaml/v1/schema/yaml.cem"),
        ),
        "csv" => (
            include_str!("../../schema-packages/csv/v1/package.cem"),
            include_str!("../../schema-packages/csv/v1/schema/csv.cem"),
        ),
        "markdown" => (
            include_str!("../../schema-packages/markdown/v1/package.cem"),
            include_str!("../../schema-packages/markdown/v1/schema/markdown.cem"),
        ),
        "xml" => (
            include_str!("../../schema-packages/xml/v1/package.cem"),
            include_str!("../../schema-packages/xml/v1/schema/xml.cem"),
        ),
        "relax-ng" => (
            include_str!("../../schema-packages/relax-ng/v1/package.cem"),
            include_str!("../../schema-packages/relax-ng/v1/schema/relax-ng.cem"),
        ),
        "xhtml" => (
            include_str!("../../schema-packages/xhtml/v1/package.cem"),
            include_str!("../../schema-packages/xhtml/v1/schema/xhtml.cem"),
        ),
        "svg" => (
            include_str!("../../schema-packages/svg/v1/package.cem"),
            include_str!("../../schema-packages/svg/v1/schema/svg.cem"),
        ),
        "mathml" => (
            include_str!("../../schema-packages/mathml/v1/package.cem"),
            include_str!("../../schema-packages/mathml/v1/schema/mathml.cem"),
        ),
        "xslt" => (
            include_str!("../../schema-packages/xslt/v1/package.cem"),
            include_str!("../../schema-packages/xslt/v1/schema/xslt.cem"),
        ),
        "html" => (
            include_str!("../../schema-packages/html/v1/package.cem"),
            include_str!("../../schema-packages/html/v1/schema/html.cem"),
        ),
        "css" => (
            include_str!("../../schema-packages/css/v1/package.cem"),
            include_str!("../../schema-packages/css/v1/schema/css.cem"),
        ),
        "json-schema" => (
            include_str!("../../schema-packages/json-schema/v1/package.cem"),
            include_str!("../../schema-packages/json-schema/v1/schema/json-schema.cem"),
        ),
        "cem-dom-projection" => (
            include_str!("../../schema-packages/cem-dom-projection/v1/package.cem"),
            include_str!(
                "../../schema-packages/cem-dom-projection/v1/schema/cem-dom-projection.cem"
            ),
        ),
        "cem-ast-projection" => (
            include_str!("../../schema-packages/cem-ast-projection/v1/package.cem"),
            include_str!(
                "../../schema-packages/cem-ast-projection/v1/schema/cem-ast-projection.cem"
            ),
        ),
        "cem-events-projection" => (
            include_str!("../../schema-packages/cem-events-projection/v1/package.cem"),
            include_str!(
                "../../schema-packages/cem-events-projection/v1/schema/cem-events-projection.cem"
            ),
        ),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::registry::{
        builtin_schema_descriptors, CEM_ML_SCHEMA_URI, CEM_SCHEMA_CONTENT_TYPE, CEM_SCHEMA_URI,
    };

    #[test]
    fn loads_bootstrap_package_by_schema_uri() {
        let package = load_builtin_schema_package(CEM_ML_SCHEMA_URI).unwrap();

        assert_eq!(package.descriptor.package_id, "cem-ml");
        assert!(package.manifest_source.contains(r#"{package @id="cem-ml""#));
        assert!(package
            .schema_source
            .contains(r#"{schema @name="cem-ml-generic""#));
    }

    #[test]
    fn loads_package_by_content_type() {
        let package =
            load_builtin_schema_package_for_content_type(CEM_SCHEMA_CONTENT_TYPE).unwrap();

        assert_eq!(package.descriptor.schema_uri, CEM_SCHEMA_URI);
        assert!(package.manifest_source.contains(r#"{package @id="schema""#));
        assert!(package
            .schema_source
            .contains(r#"{schema @name="cem-schema-definition""#));
    }

    #[test]
    fn every_builtin_descriptor_has_embedded_sources() {
        for descriptor in builtin_schema_descriptors() {
            let package = load_builtin_schema_package(&descriptor.schema_uri)
                .unwrap_or_else(|err| panic!("{}: {err}", descriptor.schema_uri));
            assert_eq!(package.descriptor.schema_uri, descriptor.schema_uri);
            assert!(
                package.manifest_source.contains("@doc cem-ml 1"),
                "{} manifest must be CEM-ML",
                descriptor.package_id
            );
            assert!(
                package.schema_source.contains("@doc cem-ml 1"),
                "{} schema must be CEM-ML",
                descriptor.package_id
            );
        }
    }

    #[test]
    fn content_type_parameters_are_accepted() {
        let package = load_builtin_schema_package_for_content_type(
            "application/vnd.cem.schema+cem; charset=utf-8",
        )
        .unwrap();

        assert_eq!(package.descriptor.schema_uri, CEM_SCHEMA_URI);
        assert_eq!(
            package.descriptor.content_types[0].value,
            CEM_SCHEMA_CONTENT_TYPE
        );
    }

    #[test]
    fn unknown_schema_uri_is_reported() {
        let err = load_builtin_schema_package("https://example.test/ns/unknown/1").unwrap_err();

        assert!(matches!(
            err,
            BuiltinSchemaPackageLoadError::UnknownSchemaUri { .. }
        ));
    }
}
