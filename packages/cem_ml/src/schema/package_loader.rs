//! Owned schema-package source inputs with an explicit embedded default adapter.
//!
//! The registry owns identity lookup. This module owns loading the matching
//! built-in package sources so validators and tooling can resolve a schema URL
//! or content type to the actual `package.cem` manifest plus schema document.

use crate::schema::package_sources::builtin_schema_package_source;
use crate::schema::registry::{SchemaDescriptor, SchemaLookupError, SchemaRegistry};
use std::sync::Arc;

/// Source text is borrowed only at the complete embedded adapter; admitted
/// sources own shared storage, without requiring consumer-side static lifetimes.
#[derive(Debug, Clone)]
pub enum SchemaPackageText {
    Embedded(&'static str),
    Owned(Arc<str>),
}

impl std::ops::Deref for SchemaPackageText {
    type Target = str;
    fn deref(&self) -> &str {
        match self {
            Self::Embedded(text) => text,
            Self::Owned(text) => text,
        }
    }
}
impl AsRef<str> for SchemaPackageText {
    fn as_ref(&self) -> &str {
        self
    }
}
impl PartialEq for SchemaPackageText {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}
impl Eq for SchemaPackageText {}
impl From<Arc<str>> for SchemaPackageText {
    fn from(text: Arc<str>) -> Self {
        Self::Owned(text)
    }
}
impl From<String> for SchemaPackageText {
    fn from(text: String) -> Self {
        Self::Owned(Arc::from(text))
    }
}
impl From<&str> for SchemaPackageText {
    fn from(text: &str) -> Self {
        Self::Owned(Arc::from(text))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaPackageSourcePolicy {
    /// Default complete-catalog metadata checks retain the embedded adapter.
    Embedded,
    /// Metadata belongs to supplied sources; artifact acquisition must use its
    /// authoritative reader rather than a similarly named embedded template.
    Authoritative,
}

#[derive(Debug, Clone)]
pub struct SchemaPackage {
    pub descriptor: SchemaDescriptor,
    pub manifest_source: SchemaPackageText,
    pub schema_source: SchemaPackageText,
    pub source_policy: SchemaPackageSourcePolicy,
}

/// Compatibility name; package consumers no longer require static source text.
pub type BuiltinSchemaPackage = SchemaPackage;

impl SchemaPackage {
    /// Admit original native source inputs against an already selected bootstrap
    /// identity. A descriptor alone never authorizes different source metadata.
    pub fn from_sources(
        descriptor: SchemaDescriptor,
        manifest_source: impl Into<SchemaPackageText>,
        schema_source: impl Into<SchemaPackageText>,
    ) -> Result<Self, BuiltinSchemaPackageLoadError> {
        let manifest_source = manifest_source.into();
        let schema_source = schema_source.into();
        let derived = crate::schema::registry::schema_descriptor_from_manifest_and_schema_sources(
            &descriptor.package_id,
            &manifest_source,
            &descriptor.source,
            &schema_source,
        )
        .map_err(|error| BuiltinSchemaPackageLoadError::SourceIdentity {
            package_id: descriptor.package_id.clone(),
            message: error.to_string(),
        })?;
        if derived != descriptor {
            return Err(BuiltinSchemaPackageLoadError::SourceIdentity {
                package_id: descriptor.package_id.clone(),
                message: "source metadata differs from selected bootstrap identity".into(),
            });
        }
        Ok(Self {
            descriptor,
            manifest_source,
            schema_source,
            source_policy: SchemaPackageSourcePolicy::Authoritative,
        })
    }
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
    SourceIdentity {
        package_id: String,
        message: String,
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
            Self::SourceIdentity {
                package_id,
                message,
            } => write!(
                f,
                "schema package `{package_id}` source identity: {message}"
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
    load_builtin_schema_package_from_descriptor(descriptor)
}

pub fn load_builtin_schema_package_for_content_type(
    content_type: &str,
) -> Result<BuiltinSchemaPackage, BuiltinSchemaPackageLoadError> {
    let registry = SchemaRegistry::with_builtin_schemas();
    let descriptor = registry.resolve_content_type(content_type)?;
    load_builtin_schema_package_from_descriptor(descriptor)
}

/// Load embedded sources for an identity already resolved by a local registry.
pub(crate) fn load_builtin_schema_package_from_descriptor(
    descriptor: &SchemaDescriptor,
) -> Result<BuiltinSchemaPackage, BuiltinSchemaPackageLoadError> {
    let Some(source) = builtin_schema_package_source(&descriptor.package_id) else {
        return Err(BuiltinSchemaPackageLoadError::MissingEmbeddedSource {
            package_id: descriptor.package_id.clone(),
            schema_uri: descriptor.schema_uri.clone(),
        });
    };
    Ok(SchemaPackage {
        descriptor: descriptor.clone(),
        manifest_source: SchemaPackageText::Embedded(source.manifest_source),
        schema_source: SchemaPackageText::Embedded(source.schema_source),
        source_policy: SchemaPackageSourcePolicy::Embedded,
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
    #[test]
    fn owned_sources_survive_input_release_and_reject_bootstrap_mismatch() {
        let original = load_builtin_schema_package(CEM_ML_SCHEMA_URI).unwrap();
        let embedded = builtin_schema_package_source("cem-ml").unwrap();
        assert!(
            std::ptr::eq::<str>(original.schema_source.as_ref(), embedded.schema_source),
            "default adapter borrows compiled bytes"
        );
        let owned = SchemaPackage::from_sources(
            original.descriptor.clone(),
            original.manifest_source.to_string(),
            original.schema_source.to_string(),
        )
        .unwrap();
        let clone = owned.clone();
        assert!(std::ptr::eq::<str>(
            owned.manifest_source.as_ref(),
            clone.manifest_source.as_ref()
        ));
        drop(owned);
        assert_eq!(clone.manifest_source, original.manifest_source);
        let mut mismatch = original.descriptor.clone();
        mismatch.version = "99.0.0".into();
        assert!(matches!(
            SchemaPackage::from_sources(mismatch, original.manifest_source, original.schema_source),
            Err(BuiltinSchemaPackageLoadError::SourceIdentity { .. })
        ));
    }
}
