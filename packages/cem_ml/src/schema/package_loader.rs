//! Embedded schema-package source loader.
//!
//! The registry owns identity lookup. This module owns loading the matching
//! built-in package sources so validators and tooling can resolve a schema URL
//! or content type to the actual `package.cem` manifest plus schema document.

use crate::schema::package_sources::builtin_schema_package_source;
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
    let Some(source) = builtin_schema_package_source(&descriptor.package_id) else {
        return Err(BuiltinSchemaPackageLoadError::MissingEmbeddedSource {
            package_id: descriptor.package_id.clone(),
            schema_uri: descriptor.schema_uri.clone(),
        });
    };
    Ok(BuiltinSchemaPackage {
        descriptor: descriptor.clone(),
        manifest_source: source.manifest_source,
        schema_source: source.schema_source,
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
