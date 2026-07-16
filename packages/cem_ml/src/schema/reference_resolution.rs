use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use crate::schema::registry::{SchemaDescriptor, SchemaRegistry};
use crate::transform_template::{
    TransformTemplateArtifactFunctionContract, TransformTemplateOutputFunctionDescriptor,
    TransformTemplateOutputFunctionKind, TransformTemplateOutputFunctionRegistry,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceNormalizer {
    ScalarExact,
    IdentifierToken,
    MediaType,
    MediaTypeEssence,
    MediaTypeEssenceSet,
    SchemaUri,
    DocumentUri,
    NamespaceUri,
    ArtifactName,
    FunctionName,
    ContentCategory,
    ProfileName,
}

impl ReferenceNormalizer {
    pub fn reference(self) -> &'static str {
        match self {
            Self::ScalarExact => "schema:scalar-exact",
            Self::IdentifierToken => "schema:identifier-token",
            Self::MediaType => "schema:media-type",
            Self::MediaTypeEssence => "schema:media-type-essence",
            Self::MediaTypeEssenceSet => "schema:media-type-essence-set",
            Self::SchemaUri => "schema:schema-uri",
            Self::DocumentUri => "schema:document-uri",
            Self::NamespaceUri => "schema:namespace-uri",
            Self::ArtifactName => "schema:artifact-name",
            Self::FunctionName => "schema:function-name",
            Self::ContentCategory => "schema:content-category",
            Self::ProfileName => "schema:profile-name",
        }
    }

    pub fn placement(self) -> NormalizerPlacement {
        match self {
            Self::ScalarExact
            | Self::IdentifierToken
            | Self::NamespaceUri
            | Self::ArtifactName
            | Self::FunctionName
            | Self::ContentCategory
            | Self::ProfileName => NormalizerPlacement::Pure,
            Self::MediaType
            | Self::MediaTypeEssence
            | Self::MediaTypeEssenceSet
            | Self::SchemaUri
            | Self::DocumentUri => NormalizerPlacement::EngineAssisted,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizerPlacement {
    Pure,
    EngineAssisted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizerSupport {
    Required,
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceValueCardinality {
    One,
    Optional,
    Set,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceValueState {
    Valid,
    Missing,
    Invalid,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceValueReason {
    MissingValue,
    InvalidScalar,
    InvalidMediaType,
    UnresolvedSchema,
    UnresolvedDocument,
    UnresolvedNamespace,
    UnresolvedFunction,
    UnsupportedNormalizer,
}

impl ReferenceValueReason {
    pub fn reference(self) -> &'static str {
        match self {
            Self::MissingValue => "missing-value",
            Self::InvalidScalar => "invalid-scalar",
            Self::InvalidMediaType => "invalid-media-type",
            Self::UnresolvedSchema => "unresolved-schema",
            Self::UnresolvedDocument => "unresolved-document",
            Self::UnresolvedNamespace => "unresolved-namespace",
            Self::UnresolvedFunction => "unresolved-function",
            Self::UnsupportedNormalizer => "unsupported-normalizer",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedMediaType {
    pub essence: String,
    pub type_name: Option<String>,
    pub subtype: Option<String>,
    pub suffix: Option<String>,
    pub parameters: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizedReferenceValue {
    Scalar(String),
    StringSet(BTreeSet<String>),
    MediaType(NormalizedMediaType),
    UriRecord {
        declared_uri: String,
        resolved_uri: String,
    },
    Record(BTreeMap<String, NormalizedReferenceValue>),
}

impl NormalizedReferenceValue {
    fn as_scalar(&self) -> Option<&str> {
        match self {
            Self::Scalar(value) => Some(value),
            _ => None,
        }
    }

    fn as_string_set(&self) -> Option<&BTreeSet<String>> {
        match self {
            Self::StringSet(values) => Some(values),
            _ => None,
        }
    }

    fn to_json(&self) -> Value {
        match self {
            Self::Scalar(value) => Value::String(value.clone()),
            Self::StringSet(values) => values.iter().cloned().map(Value::String).collect(),
            Self::MediaType(media_type) => {
                let mut object = serde_json::Map::new();
                object.insert(
                    "essence".to_owned(),
                    Value::String(media_type.essence.clone()),
                );
                if let Some(type_name) = &media_type.type_name {
                    object.insert("type".to_owned(), Value::String(type_name.clone()));
                }
                if let Some(subtype) = &media_type.subtype {
                    object.insert("subtype".to_owned(), Value::String(subtype.clone()));
                }
                if let Some(suffix) = &media_type.suffix {
                    object.insert("suffix".to_owned(), Value::String(suffix.clone()));
                }
                object.insert(
                    "parameters".to_owned(),
                    serde_json::to_value(&media_type.parameters).unwrap_or(Value::Null),
                );
                Value::Object(object)
            }
            Self::UriRecord {
                declared_uri,
                resolved_uri,
            } => serde_json::json!({
                "declaredUri": declared_uri,
                "resolvedUri": resolved_uri,
            }),
            Self::Record(fields) => fields
                .iter()
                .map(|(name, value)| (name.clone(), value.to_json()))
                .collect(),
        }
    }

    fn project(&self, projection: Option<&str>) -> Option<NormalizedReferenceValue> {
        let Some(projection) = projection else {
            return Some(self.clone());
        };
        match self {
            Self::MediaType(media_type) => match projection {
                "essence" => Some(Self::Scalar(media_type.essence.clone())),
                "type" => media_type.type_name.clone().map(Self::Scalar),
                "subtype" => media_type.subtype.clone().map(Self::Scalar),
                "suffix" => media_type.suffix.clone().map(Self::Scalar),
                _ => None,
            },
            Self::UriRecord {
                declared_uri,
                resolved_uri,
            } => match projection {
                "declaredUri" => Some(Self::Scalar(declared_uri.clone())),
                "resolvedUri" => Some(Self::Scalar(resolved_uri.clone())),
                _ => None,
            },
            Self::Record(fields) => fields.get(projection).cloned(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceValue {
    pub name: String,
    pub normalizer: ReferenceNormalizer,
    pub cardinality: ReferenceValueCardinality,
    pub declared_value: Option<String>,
    pub normalized_value: Option<NormalizedReferenceValue>,
    pub state: ReferenceValueState,
    pub reason: Option<ReferenceValueReason>,
    pub support: NormalizerSupport,
}

impl ReferenceValue {
    pub fn valid(
        name: impl Into<String>,
        normalizer: ReferenceNormalizer,
        cardinality: ReferenceValueCardinality,
        declared_value: Option<String>,
        normalized_value: NormalizedReferenceValue,
    ) -> Self {
        Self {
            name: name.into(),
            normalizer,
            cardinality,
            declared_value,
            normalized_value: Some(normalized_value),
            state: ReferenceValueState::Valid,
            reason: None,
            support: NormalizerSupport::Required,
        }
    }

    pub fn missing(name: impl Into<String>, normalizer: ReferenceNormalizer) -> Self {
        Self::non_valid(
            name,
            normalizer,
            ReferenceValueState::Missing,
            ReferenceValueReason::MissingValue,
        )
    }

    pub fn invalid(
        name: impl Into<String>,
        normalizer: ReferenceNormalizer,
        declared_value: impl Into<String>,
        reason: ReferenceValueReason,
    ) -> Self {
        let mut value = Self::non_valid(name, normalizer, ReferenceValueState::Invalid, reason);
        value.declared_value = Some(declared_value.into());
        value
    }

    pub fn unresolved(
        name: impl Into<String>,
        normalizer: ReferenceNormalizer,
        declared_value: impl Into<String>,
        reason: ReferenceValueReason,
    ) -> Self {
        let mut value = Self::non_valid(name, normalizer, ReferenceValueState::Unresolved, reason);
        value.declared_value = Some(declared_value.into());
        value
    }

    pub fn unsupported(name: impl Into<String>, normalizer: ReferenceNormalizer) -> Self {
        Self::non_valid(
            name,
            normalizer,
            ReferenceValueState::Invalid,
            ReferenceValueReason::UnsupportedNormalizer,
        )
    }

    fn non_valid(
        name: impl Into<String>,
        normalizer: ReferenceNormalizer,
        state: ReferenceValueState,
        reason: ReferenceValueReason,
    ) -> Self {
        Self {
            name: name.into(),
            normalizer,
            cardinality: ReferenceValueCardinality::Optional,
            declared_value: None,
            normalized_value: None,
            state,
            reason: Some(reason),
            support: NormalizerSupport::Required,
        }
    }
}

pub fn normalize_scalar_exact(name: impl Into<String>, value: impl Into<String>) -> ReferenceValue {
    let value = value.into();
    ReferenceValue::valid(
        name,
        ReferenceNormalizer::ScalarExact,
        ReferenceValueCardinality::One,
        Some(value.clone()),
        NormalizedReferenceValue::Scalar(value),
    )
}

pub fn normalize_identifier_token(
    name: impl Into<String>,
    value: impl Into<String>,
) -> ReferenceValue {
    normalize_identifier_like(name, value, ReferenceNormalizer::IdentifierToken)
}

pub fn normalize_namespace_uri(
    name: impl Into<String>,
    value: impl Into<String>,
) -> ReferenceValue {
    let value = value.into();
    ReferenceValue::valid(
        name,
        ReferenceNormalizer::NamespaceUri,
        ReferenceValueCardinality::One,
        Some(value.clone()),
        NormalizedReferenceValue::Scalar(value),
    )
}

pub fn normalize_namespace_uri_set<I, S>(name: impl Into<String>, values: I) -> ReferenceValue
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let values = values
        .into_iter()
        .map(|value| value.as_ref().to_owned())
        .collect::<BTreeSet<_>>();
    ReferenceValue::valid(
        name,
        ReferenceNormalizer::NamespaceUri,
        ReferenceValueCardinality::Set,
        Some(values.iter().cloned().collect::<Vec<_>>().join(" ")),
        NormalizedReferenceValue::StringSet(values),
    )
}

pub fn normalize_artifact_name(
    name: impl Into<String>,
    value: impl Into<String>,
) -> ReferenceValue {
    normalize_non_empty_scalar(name, value, ReferenceNormalizer::ArtifactName)
}

pub fn normalize_function_name(
    name: impl Into<String>,
    value: impl Into<String>,
) -> ReferenceValue {
    normalize_non_empty_scalar(name, value, ReferenceNormalizer::FunctionName)
}

pub fn normalize_content_category(
    name: impl Into<String>,
    value: impl Into<String>,
) -> ReferenceValue {
    normalize_identifier_like(name, value, ReferenceNormalizer::ContentCategory)
}

pub fn normalize_profile_name(name: impl Into<String>, value: impl Into<String>) -> ReferenceValue {
    normalize_identifier_like(name, value, ReferenceNormalizer::ProfileName)
}

pub fn normalize_media_type(name: impl Into<String>, value: impl Into<String>) -> ReferenceValue {
    let value = value.into();
    match parse_media_type(&value) {
        Some(media_type) => ReferenceValue::valid(
            name,
            ReferenceNormalizer::MediaType,
            ReferenceValueCardinality::One,
            Some(value),
            NormalizedReferenceValue::MediaType(media_type),
        ),
        None => ReferenceValue::invalid(
            name,
            ReferenceNormalizer::MediaType,
            value,
            ReferenceValueReason::InvalidMediaType,
        ),
    }
}

pub fn normalize_media_type_essence(
    name: impl Into<String>,
    value: impl Into<String>,
) -> ReferenceValue {
    let name = name.into();
    let value = value.into();
    match parse_media_type(&value) {
        Some(media_type) => ReferenceValue::valid(
            name,
            ReferenceNormalizer::MediaTypeEssence,
            ReferenceValueCardinality::One,
            Some(value),
            NormalizedReferenceValue::Scalar(media_type.essence),
        ),
        None => ReferenceValue::invalid(
            name,
            ReferenceNormalizer::MediaTypeEssence,
            value,
            ReferenceValueReason::InvalidMediaType,
        ),
    }
}

pub fn normalize_media_type_essence_set<I, S>(name: impl Into<String>, values: I) -> ReferenceValue
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let name = name.into();
    let mut declared_values = Vec::new();
    let mut essences = BTreeSet::new();
    for value in values {
        let value = value.as_ref();
        declared_values.push(value.to_owned());
        let Some(media_type) = parse_media_type(value) else {
            return ReferenceValue::invalid(
                name,
                ReferenceNormalizer::MediaTypeEssenceSet,
                value,
                ReferenceValueReason::InvalidMediaType,
            );
        };
        essences.insert(media_type.essence);
    }
    ReferenceValue::valid(
        name,
        ReferenceNormalizer::MediaTypeEssenceSet,
        ReferenceValueCardinality::Set,
        Some(declared_values.join(" ")),
        NormalizedReferenceValue::StringSet(essences),
    )
}

pub fn normalize_schema_uri(
    name: impl Into<String>,
    declared_uri: impl Into<String>,
    resolved_schema_uri: Option<&str>,
) -> ReferenceValue {
    let declared_uri = declared_uri.into();
    match resolved_schema_uri {
        Some(resolved_schema_uri) => ReferenceValue::valid(
            name,
            ReferenceNormalizer::SchemaUri,
            ReferenceValueCardinality::One,
            Some(declared_uri),
            NormalizedReferenceValue::Scalar(resolved_schema_uri.to_owned()),
        ),
        None => ReferenceValue::unresolved(
            name,
            ReferenceNormalizer::SchemaUri,
            declared_uri,
            ReferenceValueReason::UnresolvedSchema,
        ),
    }
}

pub fn normalize_document_uri(
    name: impl Into<String>,
    declared_uri: impl Into<String>,
    resolved_uri: Option<&str>,
) -> ReferenceValue {
    let declared_uri = declared_uri.into();
    match resolved_uri {
        Some(resolved_uri) => ReferenceValue::valid(
            name,
            ReferenceNormalizer::DocumentUri,
            ReferenceValueCardinality::One,
            Some(declared_uri.clone()),
            NormalizedReferenceValue::UriRecord {
                declared_uri,
                resolved_uri: resolved_uri.to_owned(),
            },
        ),
        None => ReferenceValue::unresolved(
            name,
            ReferenceNormalizer::DocumentUri,
            declared_uri,
            ReferenceValueReason::UnresolvedDocument,
        ),
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PureReferenceNormalizerEvaluator;

impl PureReferenceNormalizerEvaluator {
    pub fn normalize(
        self,
        normalizer: ReferenceNormalizer,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> ReferenceValue {
        let name = name.into();
        let value = value.into();
        match normalizer {
            ReferenceNormalizer::ScalarExact => normalize_scalar_exact(name, value),
            ReferenceNormalizer::IdentifierToken => normalize_identifier_token(name, value),
            ReferenceNormalizer::NamespaceUri => normalize_namespace_uri(name, value),
            ReferenceNormalizer::ArtifactName => normalize_artifact_name(name, value),
            ReferenceNormalizer::FunctionName => normalize_function_name(name, value),
            ReferenceNormalizer::ContentCategory => normalize_content_category(name, value),
            ReferenceNormalizer::ProfileName => normalize_profile_name(name, value),
            ReferenceNormalizer::MediaType
            | ReferenceNormalizer::MediaTypeEssence
            | ReferenceNormalizer::MediaTypeEssenceSet
            | ReferenceNormalizer::SchemaUri
            | ReferenceNormalizer::DocumentUri => ReferenceValue::unsupported(name, normalizer),
        }
    }

    pub fn scalar_exact(self, name: impl Into<String>, value: impl Into<String>) -> ReferenceValue {
        normalize_scalar_exact(name, value)
    }

    pub fn identifier_token(
        self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> ReferenceValue {
        normalize_identifier_token(name, value)
    }

    pub fn namespace_uri(
        self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> ReferenceValue {
        normalize_namespace_uri(name, value)
    }

    pub fn artifact_name(
        self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> ReferenceValue {
        normalize_artifact_name(name, value)
    }

    pub fn function_name(
        self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> ReferenceValue {
        normalize_function_name(name, value)
    }

    pub fn content_category(
        self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> ReferenceValue {
        normalize_content_category(name, value)
    }

    pub fn profile_name(self, name: impl Into<String>, value: impl Into<String>) -> ReferenceValue {
        normalize_profile_name(name, value)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EngineAssistedReferenceNormalizerEvaluator;

impl EngineAssistedReferenceNormalizerEvaluator {
    pub fn media_type(self, name: impl Into<String>, value: impl Into<String>) -> ReferenceValue {
        normalize_media_type(name, value)
    }

    pub fn media_type_essence(
        self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> ReferenceValue {
        normalize_media_type_essence(name, value)
    }

    pub fn media_type_essence_set<I, S>(self, name: impl Into<String>, values: I) -> ReferenceValue
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        normalize_media_type_essence_set(name, values)
    }

    pub fn schema_uri_from_registry(
        self,
        name: impl Into<String>,
        declared_uri: impl Into<String>,
        registry: &SchemaRegistry,
    ) -> ReferenceValue {
        let declared_uri = declared_uri.into();
        let resolved_schema_uri = registry
            .schema(&declared_uri)
            .map(|descriptor| descriptor.schema_uri.as_str());
        normalize_schema_uri(name, declared_uri, resolved_schema_uri)
    }

    pub fn schema_descriptor_uri(
        self,
        name: impl Into<String>,
        descriptor: &SchemaDescriptor,
    ) -> ReferenceValue {
        normalize_schema_uri(
            name,
            descriptor.schema_uri.clone(),
            Some(descriptor.schema_uri.as_str()),
        )
    }

    pub fn schema_descriptor_content_type_essences(
        self,
        name: impl Into<String>,
        descriptor: &SchemaDescriptor,
    ) -> ReferenceValue {
        normalize_media_type_essence_set(name, descriptor.content_type_essences())
    }

    pub fn registry_content_type_essences(
        self,
        name: impl Into<String>,
        registry: &SchemaRegistry,
    ) -> ReferenceValue {
        normalize_media_type_essence_set(name, registry.content_type_essences())
    }

    pub fn schema_descriptor_namespace_uris(
        self,
        name: impl Into<String>,
        descriptor: &SchemaDescriptor,
    ) -> ReferenceValue {
        normalize_namespace_uri_set(name, descriptor.namespace_uris())
    }

    pub fn namespace_claiming_schema_uris(
        self,
        name: impl Into<String>,
        namespace_uri: impl Into<String>,
        registry: &SchemaRegistry,
    ) -> ReferenceValue {
        let name = name.into();
        let namespace_uri = namespace_uri.into();
        let schema_uris = registry
            .lookup_namespace(&namespace_uri)
            .into_iter()
            .map(|descriptor| descriptor.schema_uri.clone())
            .collect::<BTreeSet<_>>();
        if schema_uris.is_empty() {
            return ReferenceValue::unresolved(
                name,
                ReferenceNormalizer::SchemaUri,
                namespace_uri,
                ReferenceValueReason::UnresolvedNamespace,
            );
        }
        ReferenceValue::valid(
            name,
            ReferenceNormalizer::SchemaUri,
            ReferenceValueCardinality::Set,
            Some(namespace_uri),
            NormalizedReferenceValue::StringSet(schema_uris),
        )
    }

    pub fn document_uri(
        self,
        name: impl Into<String>,
        declared_uri: impl Into<String>,
        resolved_uri: Option<&str>,
    ) -> ReferenceValue {
        normalize_document_uri(name, declared_uri, resolved_uri)
    }

    pub fn cemt_declared_function_name(
        self,
        name: impl Into<String>,
        declared_name: impl Into<String>,
        registry: &TransformTemplateOutputFunctionRegistry,
    ) -> ReferenceValue {
        let declared_name = declared_name.into();
        if registry
            .functions()
            .iter()
            .any(|function| function.name == declared_name)
        {
            normalize_function_name(name, declared_name)
        } else {
            ReferenceValue::unresolved(
                name,
                ReferenceNormalizer::FunctionName,
                declared_name,
                ReferenceValueReason::UnresolvedFunction,
            )
        }
    }

    pub fn cemt_output_function_record(
        self,
        name: impl Into<String>,
        function: &TransformTemplateOutputFunctionDescriptor,
    ) -> ReferenceValue {
        let name = name.into();
        let Some(content_type) = parse_media_type(&function.content_type) else {
            return ReferenceValue::invalid(
                name,
                ReferenceNormalizer::FunctionName,
                function.content_type.clone(),
                ReferenceValueReason::InvalidMediaType,
            );
        };
        if !is_identifier_token(&function.category) {
            return ReferenceValue::invalid(
                name,
                ReferenceNormalizer::FunctionName,
                function.category.clone(),
                ReferenceValueReason::InvalidScalar,
            );
        }
        if let Some(profile) = function.profile.as_deref() {
            if !is_identifier_token(profile) {
                return ReferenceValue::invalid(
                    name,
                    ReferenceNormalizer::FunctionName,
                    profile,
                    ReferenceValueReason::InvalidScalar,
                );
            }
        }

        let mut fields = BTreeMap::from([
            (
                "kind".to_owned(),
                NormalizedReferenceValue::Scalar(output_function_kind_value(function.kind)),
            ),
            (
                "function-name".to_owned(),
                NormalizedReferenceValue::Scalar(function.name.clone()),
            ),
            (
                "target-content-type".to_owned(),
                NormalizedReferenceValue::Scalar(content_type.essence),
            ),
            (
                "target-schema".to_owned(),
                NormalizedReferenceValue::Scalar(function.schema.clone()),
            ),
            (
                "target-category".to_owned(),
                NormalizedReferenceValue::Scalar(function.category.clone()),
            ),
        ]);
        if let Some(profile) = function.profile.as_ref() {
            fields.insert(
                "function-profile".to_owned(),
                NormalizedReferenceValue::Scalar(profile.clone()),
            );
        }
        ReferenceValue::valid(
            name,
            ReferenceNormalizer::FunctionName,
            ReferenceValueCardinality::One,
            Some(function.name.clone()),
            NormalizedReferenceValue::Record(fields),
        )
    }

    pub fn cemt_artifact_function_contract_record(
        self,
        name: impl Into<String>,
        contract: TransformTemplateArtifactFunctionContract<'_>,
    ) -> ReferenceValue {
        let name = name.into();
        let mut fields = BTreeMap::new();
        if let Some(kind) = contract
            .artifact_kind
            .and_then(artifact_output_function_kind_value)
        {
            fields.insert("kind".to_owned(), NormalizedReferenceValue::Scalar(kind));
        }
        if let Some(target_content_type) = trimmed_non_empty(contract.target_content_type) {
            let Some(media_type) = parse_media_type(target_content_type) else {
                return ReferenceValue::invalid(
                    name,
                    ReferenceNormalizer::FunctionName,
                    target_content_type,
                    ReferenceValueReason::InvalidMediaType,
                );
            };
            fields.insert(
                "target-content-type".to_owned(),
                NormalizedReferenceValue::Scalar(media_type.essence),
            );
        }
        if let Some(target_schema) = trimmed_non_empty(contract.target_schema) {
            fields.insert(
                "target-schema".to_owned(),
                NormalizedReferenceValue::Scalar(target_schema.to_owned()),
            );
        }
        if let Some(target_category) = trimmed_non_empty(contract.target_category) {
            if !is_identifier_token(target_category) {
                return ReferenceValue::invalid(
                    name,
                    ReferenceNormalizer::FunctionName,
                    target_category,
                    ReferenceValueReason::InvalidScalar,
                );
            }
            fields.insert(
                "target-category".to_owned(),
                NormalizedReferenceValue::Scalar(target_category.to_owned()),
            );
        }
        if let Some(function_profile) = trimmed_non_empty(contract.function_profile) {
            if !is_identifier_token(function_profile) {
                return ReferenceValue::invalid(
                    name,
                    ReferenceNormalizer::FunctionName,
                    function_profile,
                    ReferenceValueReason::InvalidScalar,
                );
            }
            fields.insert(
                "function-profile".to_owned(),
                NormalizedReferenceValue::Scalar(function_profile.to_owned()),
            );
        }
        ReferenceValue::valid(
            name,
            ReferenceNormalizer::FunctionName,
            ReferenceValueCardinality::One,
            None,
            NormalizedReferenceValue::Record(fields),
        )
    }
}

fn normalize_identifier_like(
    name: impl Into<String>,
    value: impl Into<String>,
    normalizer: ReferenceNormalizer,
) -> ReferenceValue {
    let value = value.into();
    if is_identifier_token(&value) {
        ReferenceValue::valid(
            name,
            normalizer,
            ReferenceValueCardinality::One,
            Some(value.clone()),
            NormalizedReferenceValue::Scalar(value),
        )
    } else {
        ReferenceValue::invalid(name, normalizer, value, ReferenceValueReason::InvalidScalar)
    }
}

fn normalize_non_empty_scalar(
    name: impl Into<String>,
    value: impl Into<String>,
    normalizer: ReferenceNormalizer,
) -> ReferenceValue {
    let value = value.into();
    if value.is_empty() {
        ReferenceValue::invalid(name, normalizer, value, ReferenceValueReason::InvalidScalar)
    } else {
        ReferenceValue::valid(
            name,
            normalizer,
            ReferenceValueCardinality::One,
            Some(value.clone()),
            NormalizedReferenceValue::Scalar(value),
        )
    }
}

fn parse_media_type(value: &str) -> Option<NormalizedMediaType> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let mut parts = value.split(';');
    let essence = parts.next()?.trim();
    let (type_name, subtype, suffix) = if is_legacy_content_type_alias(essence) {
        (None, None, None)
    } else {
        let (type_name, subtype) = essence.split_once('/')?;
        if !is_http_token(type_name) || !is_http_token(subtype) {
            return None;
        }
        let type_name = type_name.to_ascii_lowercase();
        let subtype = subtype.to_ascii_lowercase();
        let suffix = subtype
            .rsplit_once('+')
            .map(|(_, suffix)| suffix)
            .filter(|suffix| !suffix.is_empty())
            .map(str::to_owned);
        (Some(type_name), Some(subtype), suffix)
    };

    let mut parameters = BTreeMap::<String, BTreeSet<String>>::new();
    for parameter in parts {
        let (name, param_value) = parameter.trim().split_once('=')?;
        let name = name.trim();
        if !is_http_token(name) {
            return None;
        }
        let param_value = normalized_media_type_parameter_value(param_value.trim())?;
        parameters
            .entry(name.to_ascii_lowercase())
            .or_default()
            .insert(param_value);
    }

    Some(NormalizedMediaType {
        essence: essence.to_ascii_lowercase(),
        type_name,
        subtype,
        suffix,
        parameters,
    })
}

fn normalized_media_type_parameter_value(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    if is_http_token(value) {
        return Some(value.to_owned());
    }
    let inner = value.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::new();
    let mut escaped = false;
    for ch in inner.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' || ch == '\r' || ch == '\n' {
            return None;
        } else {
            out.push(ch);
        }
    }
    (!escaped).then_some(out)
}

fn is_legacy_content_type_alias(value: &str) -> bool {
    value.eq_ignore_ascii_case("custom-element-xslt")
}

fn is_http_token(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
}

fn is_identifier_token(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(|ch| ch.is_ascii_alphabetic())
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

fn output_function_kind_value(kind: TransformTemplateOutputFunctionKind) -> String {
    match kind {
        TransformTemplateOutputFunctionKind::Encoding => "encoding",
        TransformTemplateOutputFunctionKind::Format => "format",
        TransformTemplateOutputFunctionKind::Color => "color",
    }
    .to_owned()
}

fn artifact_output_function_kind_value(artifact_kind: &str) -> Option<String> {
    match artifact_kind.trim() {
        "formatter" | "formatter-helper" => Some("format".to_owned()),
        "colorizer" | "colorizer-helper" => Some("color".to_owned()),
        "encoder" => Some("encoding".to_owned()),
        _ => None,
    }
}

fn trimmed_non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceOperandRole {
    Actual,
    Expected,
    Forbidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceStatePolicy {
    RequiredValid,
    OptionalAbsentOk,
    CompareWhenPresent,
    BothOrNone,
    UnresolvedFails,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceComparisonOperator {
    Equals,
    MemberOf,
    AllIn,
    ContainsAll,
    Intersects,
    Disjoint,
    Exists,
    RecordFieldsEqual,
    RecordFieldsMemberOf,
}

impl ReferenceComparisonOperator {
    pub fn reference(self) -> &'static str {
        match self {
            Self::Equals => "schema:equals",
            Self::MemberOf => "schema:member-of",
            Self::AllIn => "schema:all-in",
            Self::ContainsAll => "schema:contains-all",
            Self::Intersects => "schema:intersects",
            Self::Disjoint => "schema:disjoint",
            Self::Exists => "schema:exists",
            Self::RecordFieldsEqual => "schema:record-fields-equal",
            Self::RecordFieldsMemberOf => "schema:record-fields-member-of",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceOperand {
    pub role: ReferenceOperandRole,
    pub binding: String,
    pub value: ReferenceValue,
    pub projection: Option<String>,
}

impl ReferenceOperand {
    pub fn new(
        role: ReferenceOperandRole,
        binding: impl Into<String>,
        value: ReferenceValue,
    ) -> Self {
        Self {
            role,
            binding: binding.into(),
            value,
            projection: None,
        }
    }

    pub fn with_projection(mut self, projection: impl Into<String>) -> Self {
        self.projection = Some(projection.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceComparisonInput {
    pub operator: ReferenceComparisonOperator,
    pub actual: ReferenceOperand,
    pub expected: Option<ReferenceOperand>,
    pub forbidden: Option<ReferenceOperand>,
    pub state_policy: ReferenceStatePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceComparisonResult {
    pub passed: bool,
    pub operator: ReferenceComparisonOperator,
    pub expected_values: BTreeMap<String, Value>,
    pub invalid_values: BTreeMap<String, Value>,
    pub missing_values: BTreeMap<String, Value>,
    pub unresolved_values: BTreeMap<String, Value>,
    pub invalid_fields: BTreeSet<String>,
    pub comparison: Value,
}

impl ReferenceComparisonResult {
    fn passed(operator: ReferenceComparisonOperator, comparison: Value) -> Self {
        Self {
            passed: true,
            operator,
            expected_values: BTreeMap::new(),
            invalid_values: BTreeMap::new(),
            missing_values: BTreeMap::new(),
            unresolved_values: BTreeMap::new(),
            invalid_fields: BTreeSet::new(),
            comparison,
        }
    }

    fn failed(operator: ReferenceComparisonOperator, comparison: Value) -> Self {
        Self {
            passed: false,
            operator,
            expected_values: BTreeMap::new(),
            invalid_values: BTreeMap::new(),
            missing_values: BTreeMap::new(),
            unresolved_values: BTreeMap::new(),
            invalid_fields: BTreeSet::new(),
            comparison,
        }
    }
}

pub fn compare_references(input: ReferenceComparisonInput) -> ReferenceComparisonResult {
    let comparison = comparison_metadata(&input);
    let operands = comparison_operands(&input);
    if let Some(result) = evaluate_state_policy(
        input.operator,
        input.state_policy,
        comparison.clone(),
        &operands,
    ) {
        return result;
    }

    let mut result = ReferenceComparisonResult::passed(input.operator, comparison);
    match input.operator {
        ReferenceComparisonOperator::Equals => {
            let Some(expected) = input.expected.as_ref() else {
                return malformed_comparison(input.operator, result.comparison);
            };
            let actual = projected_value(&input.actual);
            let expected_value = projected_value(expected);
            if actual != expected_value {
                result.passed = false;
                add_expected_value(&mut result, expected, expected_value);
                add_invalid_value(&mut result, &input.actual, actual);
            }
        }
        ReferenceComparisonOperator::MemberOf => {
            let Some(expected) = input.expected.as_ref() else {
                return malformed_comparison(input.operator, result.comparison);
            };
            let actual = projected_value(&input.actual);
            let expected_value = projected_value(expected);
            if !scalar_in_set(actual.as_ref(), expected_value.as_ref()) {
                result.passed = false;
                add_expected_value(&mut result, expected, expected_value);
                add_invalid_value(&mut result, &input.actual, actual);
            }
        }
        ReferenceComparisonOperator::AllIn => {
            compare_sets(
                &mut result,
                &input.actual,
                input.expected.as_ref(),
                |actual, expected| actual.is_subset(expected),
            );
        }
        ReferenceComparisonOperator::ContainsAll => {
            compare_sets(
                &mut result,
                &input.actual,
                input.expected.as_ref(),
                |actual, expected| expected.is_subset(actual),
            );
        }
        ReferenceComparisonOperator::Intersects => {
            compare_sets(
                &mut result,
                &input.actual,
                input.expected.as_ref(),
                |actual, expected| !actual.is_disjoint(expected),
            );
        }
        ReferenceComparisonOperator::Disjoint => {
            compare_sets(
                &mut result,
                &input.actual,
                input.forbidden.as_ref(),
                |actual, forbidden| actual.is_disjoint(forbidden),
            );
        }
        ReferenceComparisonOperator::Exists => {
            let actual = projected_value(&input.actual);
            if actual.is_none() {
                result.passed = false;
                add_invalid_value(&mut result, &input.actual, actual);
            }
        }
        ReferenceComparisonOperator::RecordFieldsEqual => {
            let Some(expected) = input.expected.as_ref() else {
                return malformed_comparison(input.operator, result.comparison);
            };
            compare_record_fields(&mut result, &input.actual, expected, false);
        }
        ReferenceComparisonOperator::RecordFieldsMemberOf => {
            let Some(expected) = input.expected.as_ref() else {
                return malformed_comparison(input.operator, result.comparison);
            };
            compare_record_fields(&mut result, &input.actual, expected, true);
        }
    }
    result
}

fn comparison_operands<'a>(input: &'a ReferenceComparisonInput) -> Vec<&'a ReferenceOperand> {
    let mut operands = vec![&input.actual];
    if let Some(expected) = &input.expected {
        operands.push(expected);
    }
    if let Some(forbidden) = &input.forbidden {
        operands.push(forbidden);
    }
    operands
}

fn evaluate_state_policy(
    operator: ReferenceComparisonOperator,
    policy: ReferenceStatePolicy,
    comparison: Value,
    operands: &[&ReferenceOperand],
) -> Option<ReferenceComparisonResult> {
    match policy {
        ReferenceStatePolicy::OptionalAbsentOk | ReferenceStatePolicy::CompareWhenPresent
            if operands
                .iter()
                .any(|operand| operand.value.state == ReferenceValueState::Missing) =>
        {
            let mut result = ReferenceComparisonResult::failed(operator, comparison.clone());
            for operand in operands {
                if matches!(
                    operand.value.state,
                    ReferenceValueState::Invalid | ReferenceValueState::Unresolved
                ) {
                    add_state_failure(&mut result, operand);
                }
            }
            return if result.invalid_fields.is_empty() {
                Some(ReferenceComparisonResult::passed(operator, comparison))
            } else {
                Some(result)
            };
        }
        ReferenceStatePolicy::BothOrNone => {
            let missing_count = operands
                .iter()
                .filter(|operand| operand.value.state == ReferenceValueState::Missing)
                .count();
            if missing_count == operands.len() {
                return Some(ReferenceComparisonResult::passed(operator, comparison));
            }
            if missing_count > 0 {
                let mut result = ReferenceComparisonResult::failed(operator, comparison);
                for operand in operands {
                    if operand.value.state == ReferenceValueState::Missing {
                        add_state_failure(&mut result, operand);
                    }
                }
                return Some(result);
            }
        }
        _ => {}
    }

    let mut result = ReferenceComparisonResult::failed(operator, comparison);
    for operand in operands {
        if operand.value.state != ReferenceValueState::Valid {
            add_state_failure(&mut result, operand);
        }
    }
    (!result.invalid_fields.is_empty()
        || !result.missing_values.is_empty()
        || !result.unresolved_values.is_empty())
    .then_some(result)
}

fn add_state_failure(result: &mut ReferenceComparisonResult, operand: &ReferenceOperand) {
    let reason = operand
        .value
        .reason
        .map(ReferenceValueReason::reference)
        .unwrap_or("invalid-scalar");
    match operand.value.state {
        ReferenceValueState::Missing => {
            result
                .missing_values
                .insert(operand.binding.clone(), Value::String(reason.to_owned()));
        }
        ReferenceValueState::Unresolved => {
            result
                .unresolved_values
                .insert(operand.binding.clone(), Value::String(reason.to_owned()));
        }
        ReferenceValueState::Invalid => {
            result
                .invalid_values
                .insert(operand.binding.clone(), Value::String(reason.to_owned()));
        }
        ReferenceValueState::Valid => {}
    }
    result.invalid_fields.insert(operand.binding.clone());
}

fn projected_value(operand: &ReferenceOperand) -> Option<NormalizedReferenceValue> {
    operand
        .value
        .normalized_value
        .as_ref()
        .and_then(|value| value.project(operand.projection.as_deref()))
}

fn scalar_in_set(
    actual: Option<&NormalizedReferenceValue>,
    expected: Option<&NormalizedReferenceValue>,
) -> bool {
    let Some(actual) = actual.and_then(NormalizedReferenceValue::as_scalar) else {
        return false;
    };
    expected
        .and_then(NormalizedReferenceValue::as_string_set)
        .is_some_and(|expected| expected.contains(actual))
}

fn compare_sets<F>(
    result: &mut ReferenceComparisonResult,
    actual: &ReferenceOperand,
    expected: Option<&ReferenceOperand>,
    predicate: F,
) where
    F: FnOnce(&BTreeSet<String>, &BTreeSet<String>) -> bool,
{
    let Some(expected) = expected else {
        result.passed = false;
        return;
    };
    let actual_value = projected_value(actual);
    let expected_value = projected_value(expected);
    let passes = actual_value
        .as_ref()
        .and_then(NormalizedReferenceValue::as_string_set)
        .zip(
            expected_value
                .as_ref()
                .and_then(NormalizedReferenceValue::as_string_set),
        )
        .is_some_and(|(actual, expected)| predicate(actual, expected));
    if !passes {
        result.passed = false;
        add_expected_value(result, expected, expected_value);
        add_invalid_value(result, actual, actual_value);
    }
}

fn compare_record_fields(
    result: &mut ReferenceComparisonResult,
    actual: &ReferenceOperand,
    expected: &ReferenceOperand,
    member_of: bool,
) {
    let actual_value = projected_value(actual);
    let expected_value = projected_value(expected);
    let Some(NormalizedReferenceValue::Record(actual_fields)) = actual_value.as_ref() else {
        result.passed = false;
        add_invalid_value(result, actual, actual_value);
        add_expected_value(result, expected, expected_value);
        return;
    };
    let Some(NormalizedReferenceValue::Record(expected_fields)) = expected_value.as_ref() else {
        result.passed = false;
        add_invalid_value(result, actual, actual_value);
        add_expected_value(result, expected, expected_value);
        return;
    };
    for (field, expected_field_value) in expected_fields {
        let actual_field_value = actual_fields.get(field);
        let matches = if member_of {
            scalar_in_set(actual_field_value, Some(expected_field_value))
        } else {
            actual_field_value == Some(expected_field_value)
        };
        if !matches {
            result.passed = false;
            result
                .expected_values
                .insert(field.clone(), expected_field_value.to_json());
            result.invalid_values.insert(
                field.clone(),
                actual_field_value
                    .map(NormalizedReferenceValue::to_json)
                    .unwrap_or(Value::Null),
            );
            result.invalid_fields.insert(field.clone());
        }
    }
}

fn add_expected_value(
    result: &mut ReferenceComparisonResult,
    operand: &ReferenceOperand,
    value: Option<NormalizedReferenceValue>,
) {
    if let Some(value) = value {
        result
            .expected_values
            .insert(operand.binding.clone(), value.to_json());
    }
}

fn add_invalid_value(
    result: &mut ReferenceComparisonResult,
    operand: &ReferenceOperand,
    value: Option<NormalizedReferenceValue>,
) {
    result.invalid_values.insert(
        operand.binding.clone(),
        value
            .map(|value| value.to_json())
            .unwrap_or_else(|| Value::String("<not comparable>".to_owned())),
    );
    result.invalid_fields.insert(operand.binding.clone());
}

fn malformed_comparison(
    operator: ReferenceComparisonOperator,
    comparison: Value,
) -> ReferenceComparisonResult {
    let mut result = ReferenceComparisonResult::failed(operator, comparison);
    result.invalid_values.insert(
        "comparison".to_owned(),
        Value::String("missing required operand".to_owned()),
    );
    result.invalid_fields.insert("comparison".to_owned());
    result
}

fn comparison_metadata(input: &ReferenceComparisonInput) -> Value {
    let mut object = serde_json::Map::new();
    object.insert(
        "operator".to_owned(),
        Value::String(input.operator.reference().to_owned()),
    );
    object.insert(
        "actualBinding".to_owned(),
        Value::String(input.actual.binding.clone()),
    );
    object.insert(
        "actualNormalizer".to_owned(),
        Value::String(input.actual.value.normalizer.reference().to_owned()),
    );
    if let Some(expected) = &input.expected {
        object.insert(
            "expectedBinding".to_owned(),
            Value::String(expected.binding.clone()),
        );
        object.insert(
            "expectedNormalizer".to_owned(),
            Value::String(expected.value.normalizer.reference().to_owned()),
        );
    }
    if let Some(forbidden) = &input.forbidden {
        object.insert(
            "forbiddenBinding".to_owned(),
            Value::String(forbidden.binding.clone()),
        );
        object.insert(
            "forbiddenNormalizer".to_owned(),
            Value::String(forbidden.value.normalizer.reference().to_owned()),
        );
    }
    Value::Object(object)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::registry::{
        HTML_CONTENT_TYPE, HTML_NAMESPACE_URI, HTML_SCHEMA_URI, XHTML_SCHEMA_URI,
    };
    use crate::transform_template::{
        TransformTemplateModuleVisibility, TransformTemplateOutputFunctionImplementation,
        TransformTemplateOutputProducedKind,
    };

    fn set(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn actual(value: ReferenceValue) -> ReferenceOperand {
        ReferenceOperand::new(ReferenceOperandRole::Actual, value.name.clone(), value)
    }

    fn expected(value: ReferenceValue) -> ReferenceOperand {
        ReferenceOperand::new(ReferenceOperandRole::Expected, value.name.clone(), value)
    }

    fn output_function_descriptor() -> TransformTemplateOutputFunctionDescriptor {
        TransformTemplateOutputFunctionDescriptor {
            kind: TransformTemplateOutputFunctionKind::Format,
            owner: None,
            name: "demo.format".to_owned(),
            category: "html-document".to_owned(),
            subject: "cem-tree".to_owned(),
            produces: TransformTemplateOutputProducedKind::Text,
            content_type: "Text/HTML; Charset=UTF-8".to_owned(),
            schema: HTML_SCHEMA_URI.to_owned(),
            canonical: true,
            streamable: true,
            visibility: TransformTemplateModuleVisibility::Public,
            implementation: TransformTemplateOutputFunctionImplementation::Cemt,
            profile: Some("compact".to_owned()),
            extends: None,
            capability: None,
            deterministic: true,
            trusted: false,
            lossy: false,
            fallback: None,
            params: Vec::new(),
            body_declared: false,
            body_expression: None,
        }
    }

    #[test]
    fn normalizer_metadata_names_and_placements_are_stable() {
        assert_eq!(
            ReferenceNormalizer::MediaTypeEssence.reference(),
            "schema:media-type-essence"
        );
        assert_eq!(
            ReferenceNormalizer::SchemaUri.placement(),
            NormalizerPlacement::EngineAssisted
        );
        assert_eq!(
            ReferenceNormalizer::IdentifierToken.placement(),
            NormalizerPlacement::Pure
        );
    }

    #[test]
    fn normalizer_evaluators_keep_pure_and_engine_assisted_paths_separate() {
        let pure = PureReferenceNormalizerEvaluator;
        let identifier = pure.normalize(ReferenceNormalizer::IdentifierToken, "profile", "compact");
        assert_eq!(identifier.state, ReferenceValueState::Valid);

        let unsupported = pure.normalize(
            ReferenceNormalizer::MediaTypeEssence,
            "content-type",
            "text/html",
        );
        assert_eq!(unsupported.state, ReferenceValueState::Invalid);
        assert_eq!(
            unsupported.reason,
            Some(ReferenceValueReason::UnsupportedNormalizer)
        );

        let engine = EngineAssistedReferenceNormalizerEvaluator;
        let media_type = engine.media_type_essence("content-type", "Text/HTML; charset=utf-8");
        assert_eq!(
            media_type.normalized_value,
            Some(NormalizedReferenceValue::Scalar("text/html".to_owned()))
        );
    }

    #[test]
    fn pure_scalar_and_identifier_normalizers_preserve_explicit_semantics() {
        let exact = normalize_scalar_exact("exact", "  Keep Case  ");
        assert_eq!(exact.state, ReferenceValueState::Valid);
        assert_eq!(
            exact.normalized_value,
            Some(NormalizedReferenceValue::Scalar("  Keep Case  ".to_owned()))
        );

        let identifier = normalize_identifier_token("profile", "compact");
        assert_eq!(identifier.state, ReferenceValueState::Valid);

        let invalid_identifier = normalize_identifier_token("profile", "schema:type");
        assert_eq!(invalid_identifier.state, ReferenceValueState::Invalid);
        assert_eq!(
            invalid_identifier.reason,
            Some(ReferenceValueReason::InvalidScalar)
        );
    }

    #[test]
    fn media_type_normalizer_keeps_record_and_essence_forms_separate() {
        let media_type = normalize_media_type("content-type", r#"Text/HTML; Charset="UTF-8""#);
        let Some(NormalizedReferenceValue::MediaType(media_type)) = media_type.normalized_value
        else {
            panic!("media type record");
        };
        assert_eq!(media_type.essence, "text/html");
        assert_eq!(media_type.type_name.as_deref(), Some("text"));
        assert_eq!(media_type.subtype.as_deref(), Some("html"));
        assert_eq!(media_type.parameters.get("charset"), Some(&set(&["UTF-8"])));

        let essence = normalize_media_type_essence("content-type", "Application/CEM+XML; q=1");
        assert_eq!(
            essence.normalized_value,
            Some(NormalizedReferenceValue::Scalar(
                "application/cem+xml".to_owned()
            ))
        );

        let invalid = normalize_media_type("content-type", "text html");
        assert_eq!(invalid.state, ReferenceValueState::Invalid);
        assert_eq!(invalid.reason, Some(ReferenceValueReason::InvalidMediaType));
    }

    #[test]
    fn media_type_essence_set_is_sorted_and_duplicate_free() {
        let value = normalize_media_type_essence_set(
            "contentTypes",
            ["Text/HTML", "text/html; charset=utf-8", "Application/XML"],
        );
        assert_eq!(value.state, ReferenceValueState::Valid);
        assert_eq!(
            value.normalized_value,
            Some(NormalizedReferenceValue::StringSet(set(&[
                "application/xml",
                "text/html"
            ])))
        );
    }

    #[test]
    fn engine_assisted_normalizers_preserve_resolved_identity_and_state() {
        let schema = normalize_schema_uri(
            "schema",
            "schema:declared",
            Some("https://cem.dev/ns/schema/1"),
        );
        assert_eq!(
            schema.normalized_value,
            Some(NormalizedReferenceValue::Scalar(
                "https://cem.dev/ns/schema/1".to_owned()
            ))
        );

        let document = normalize_document_uri(
            "source",
            "schema/source.cem",
            Some("file:///workspace/schema/source.cem"),
        );
        assert_eq!(
            document.normalized_value,
            Some(NormalizedReferenceValue::UriRecord {
                declared_uri: "schema/source.cem".to_owned(),
                resolved_uri: "file:///workspace/schema/source.cem".to_owned(),
            })
        );

        let unresolved_schema = normalize_schema_uri("schema", "missing:", None);
        assert_eq!(unresolved_schema.state, ReferenceValueState::Unresolved);
        assert_eq!(
            unresolved_schema.reason,
            Some(ReferenceValueReason::UnresolvedSchema)
        );
    }

    #[test]
    fn domain_scalar_normalizers_cover_current_reference_value_kinds() {
        assert_eq!(
            normalize_namespace_uri("namespace", "https://example.test/ns")
                .normalized_value
                .unwrap(),
            NormalizedReferenceValue::Scalar("https://example.test/ns".to_owned())
        );
        assert_eq!(
            normalize_namespace_uri_set("namespaces", ["urn:z", "urn:a", "urn:a"])
                .normalized_value
                .unwrap(),
            NormalizedReferenceValue::StringSet(set(&["urn:a", "urn:z"]))
        );
        assert_eq!(
            normalize_artifact_name("artifact", "formatters/cem-format-tree.cemt").state,
            ReferenceValueState::Valid
        );
        assert_eq!(
            normalize_function_name("function", "formatDocument").state,
            ReferenceValueState::Valid
        );
        assert_eq!(
            normalize_content_category("category", "document").state,
            ReferenceValueState::Valid
        );
        assert_eq!(
            normalize_profile_name("profile", "compact").state,
            ReferenceValueState::Valid
        );
    }

    #[test]
    fn engine_assisted_registry_evaluator_lifts_schema_content_type_and_namespace_metadata() {
        let registry = SchemaRegistry::with_builtin_schemas();
        let engine = EngineAssistedReferenceNormalizerEvaluator;

        let schema = engine.schema_uri_from_registry("schema", HTML_SCHEMA_URI, &registry);
        assert_eq!(
            schema.normalized_value,
            Some(NormalizedReferenceValue::Scalar(HTML_SCHEMA_URI.to_owned()))
        );

        let unresolved = engine.schema_uri_from_registry(
            "schema",
            "https://example.test/ns/missing/1",
            &registry,
        );
        assert_eq!(unresolved.state, ReferenceValueState::Unresolved);
        assert_eq!(
            unresolved.reason,
            Some(ReferenceValueReason::UnresolvedSchema)
        );

        let html = registry.schema(HTML_SCHEMA_URI).expect("HTML descriptor");
        let content_types = engine.schema_descriptor_content_type_essences("contentTypes", html);
        assert_eq!(
            content_types.normalized_value,
            Some(NormalizedReferenceValue::StringSet(set(&[
                HTML_CONTENT_TYPE
            ])))
        );

        let namespaces = engine.schema_descriptor_namespace_uris("namespaces", html);
        let Some(NormalizedReferenceValue::StringSet(namespaces)) = namespaces.normalized_value
        else {
            panic!("namespace set");
        };
        assert!(namespaces.contains(HTML_SCHEMA_URI));
        assert!(namespaces.contains(HTML_NAMESPACE_URI));

        let claiming_schemas =
            engine.namespace_claiming_schema_uris("namespace", HTML_NAMESPACE_URI, &registry);
        let Some(NormalizedReferenceValue::StringSet(claiming_schema_uris)) =
            claiming_schemas.normalized_value
        else {
            panic!("claiming schema set");
        };
        assert!(claiming_schema_uris.contains(HTML_SCHEMA_URI));
        assert!(claiming_schema_uris.contains(XHTML_SCHEMA_URI));

        let missing_namespace =
            engine.namespace_claiming_schema_uris("namespace", "urn:missing", &registry);
        assert_eq!(missing_namespace.state, ReferenceValueState::Unresolved);
        assert_eq!(
            missing_namespace.reason,
            Some(ReferenceValueReason::UnresolvedNamespace)
        );
    }

    #[test]
    fn engine_assisted_resolver_evaluator_preserves_document_uri_projection() {
        let document = EngineAssistedReferenceNormalizerEvaluator.document_uri(
            "source",
            "schema/source.cem",
            Some("file:///workspace/schema/source.cem"),
        );
        assert_eq!(
            document.normalized_value,
            Some(NormalizedReferenceValue::UriRecord {
                declared_uri: "schema/source.cem".to_owned(),
                resolved_uri: "file:///workspace/schema/source.cem".to_owned(),
            })
        );

        let unresolved =
            EngineAssistedReferenceNormalizerEvaluator.document_uri("source", "missing.cem", None);
        assert_eq!(unresolved.state, ReferenceValueState::Unresolved);
        assert_eq!(
            unresolved.reason,
            Some(ReferenceValueReason::UnresolvedDocument)
        );
    }

    #[test]
    fn engine_assisted_cemt_evaluator_lifts_function_lookup_and_contract_metadata() {
        let engine = EngineAssistedReferenceNormalizerEvaluator;
        let descriptor = output_function_descriptor();
        let mut registry = TransformTemplateOutputFunctionRegistry::new();
        registry.register(descriptor.clone());

        let declared =
            engine.cemt_declared_function_name("function-name", "demo.format", &registry);
        assert_eq!(declared.state, ReferenceValueState::Valid);
        assert_eq!(
            declared.normalized_value,
            Some(NormalizedReferenceValue::Scalar("demo.format".to_owned()))
        );

        let missing =
            engine.cemt_declared_function_name("function-name", "demo.missing", &registry);
        assert_eq!(missing.state, ReferenceValueState::Unresolved);
        assert_eq!(
            missing.reason,
            Some(ReferenceValueReason::UnresolvedFunction)
        );

        let actual_record = engine.cemt_output_function_record("actualFunction", &descriptor);
        let expected_record = engine.cemt_artifact_function_contract_record(
            "expectedFunction",
            TransformTemplateArtifactFunctionContract {
                artifact_kind: Some("formatter"),
                target_content_type: Some("text/html"),
                target_schema: Some(HTML_SCHEMA_URI),
                target_category: Some("html-document"),
                function_profile: Some("compact"),
            },
        );
        let result = compare_references(ReferenceComparisonInput {
            operator: ReferenceComparisonOperator::RecordFieldsEqual,
            actual: actual(actual_record),
            expected: Some(expected(expected_record)),
            forbidden: None,
            state_policy: ReferenceStatePolicy::RequiredValid,
        });
        assert!(result.passed);

        let invalid_contract = engine.cemt_artifact_function_contract_record(
            "expectedFunction",
            TransformTemplateArtifactFunctionContract {
                target_content_type: Some("text html"),
                ..TransformTemplateArtifactFunctionContract::default()
            },
        );
        assert_eq!(invalid_contract.state, ReferenceValueState::Invalid);
        assert_eq!(
            invalid_contract.reason,
            Some(ReferenceValueReason::InvalidMediaType)
        );
    }

    #[test]
    fn non_valid_reference_values_have_stable_reasons() {
        assert_eq!(
            ReferenceValue::missing("field", ReferenceNormalizer::ScalarExact).reason,
            Some(ReferenceValueReason::MissingValue)
        );
        assert_eq!(
            ReferenceValue::unresolved(
                "function",
                ReferenceNormalizer::FunctionName,
                "missing",
                ReferenceValueReason::UnresolvedFunction
            )
            .reason,
            Some(ReferenceValueReason::UnresolvedFunction)
        );
        assert_eq!(
            ReferenceValue::unsupported("field", ReferenceNormalizer::DocumentUri).reason,
            Some(ReferenceValueReason::UnsupportedNormalizer)
        );
    }

    #[test]
    fn comparison_equality_and_membership_project_expected_and_invalid_values() {
        let equal = compare_references(ReferenceComparisonInput {
            operator: ReferenceComparisonOperator::Equals,
            actual: actual(normalize_schema_uri("schema", "declared", Some("schema:a"))),
            expected: Some(expected(normalize_schema_uri(
                "expectedSchema",
                "declared",
                Some("schema:a"),
            ))),
            forbidden: None,
            state_policy: ReferenceStatePolicy::RequiredValid,
        });
        assert!(equal.passed);

        let mismatch = compare_references(ReferenceComparisonInput {
            operator: ReferenceComparisonOperator::MemberOf,
            actual: actual(normalize_media_type_essence("content-type", "text/html")),
            expected: Some(expected(normalize_media_type_essence_set(
                "contentTypes",
                ["application/xml", "text/xml"],
            ))),
            forbidden: None,
            state_policy: ReferenceStatePolicy::RequiredValid,
        });
        assert!(!mismatch.passed);
        assert_eq!(
            mismatch.invalid_values["content-type"],
            serde_json::json!("text/html")
        );
        assert_eq!(
            mismatch.expected_values["contentTypes"],
            serde_json::json!(["application/xml", "text/xml"])
        );
        assert!(mismatch.invalid_fields.contains("content-type"));
    }

    #[test]
    fn comparison_set_operators_cover_all_required_set_shapes() {
        let actual_set = ReferenceValue::valid(
            "actual",
            ReferenceNormalizer::MediaTypeEssenceSet,
            ReferenceValueCardinality::Set,
            None,
            NormalizedReferenceValue::StringSet(set(&["a", "b"])),
        );
        let expected_set = ReferenceValue::valid(
            "expected",
            ReferenceNormalizer::MediaTypeEssenceSet,
            ReferenceValueCardinality::Set,
            None,
            NormalizedReferenceValue::StringSet(set(&["a", "b", "c"])),
        );
        let forbidden_set = ReferenceValue::valid(
            "forbidden",
            ReferenceNormalizer::MediaTypeEssenceSet,
            ReferenceValueCardinality::Set,
            None,
            NormalizedReferenceValue::StringSet(set(&["z"])),
        );

        assert!(
            compare_references(ReferenceComparisonInput {
                operator: ReferenceComparisonOperator::AllIn,
                actual: actual(actual_set.clone()),
                expected: Some(expected(expected_set.clone())),
                forbidden: None,
                state_policy: ReferenceStatePolicy::RequiredValid,
            })
            .passed
        );
        assert!(
            compare_references(ReferenceComparisonInput {
                operator: ReferenceComparisonOperator::ContainsAll,
                actual: actual(expected_set.clone()),
                expected: Some(expected(actual_set.clone())),
                forbidden: None,
                state_policy: ReferenceStatePolicy::RequiredValid,
            })
            .passed
        );
        assert!(
            compare_references(ReferenceComparisonInput {
                operator: ReferenceComparisonOperator::Intersects,
                actual: actual(actual_set.clone()),
                expected: Some(expected(expected_set)),
                forbidden: None,
                state_policy: ReferenceStatePolicy::RequiredValid,
            })
            .passed
        );
        assert!(
            compare_references(ReferenceComparisonInput {
                operator: ReferenceComparisonOperator::Disjoint,
                actual: actual(actual_set),
                expected: None,
                forbidden: Some(ReferenceOperand::new(
                    ReferenceOperandRole::Forbidden,
                    "forbidden",
                    forbidden_set
                )),
                state_policy: ReferenceStatePolicy::RequiredValid,
            })
            .passed
        );
    }

    #[test]
    fn comparison_existence_and_record_fields_cover_artifact_contract_shapes() {
        let exists = compare_references(ReferenceComparisonInput {
            operator: ReferenceComparisonOperator::Exists,
            actual: actual(normalize_function_name("function-name", "formatDocument")),
            expected: None,
            forbidden: None,
            state_policy: ReferenceStatePolicy::RequiredValid,
        });
        assert!(exists.passed);

        let actual_record = ReferenceValue::valid(
            "actualFunction",
            ReferenceNormalizer::FunctionName,
            ReferenceValueCardinality::One,
            None,
            NormalizedReferenceValue::Record(BTreeMap::from([
                (
                    "target-content-type".to_owned(),
                    NormalizedReferenceValue::Scalar("text/html".to_owned()),
                ),
                (
                    "target-category".to_owned(),
                    NormalizedReferenceValue::Scalar("document".to_owned()),
                ),
            ])),
        );
        let expected_record = ReferenceValue::valid(
            "expectedFunction",
            ReferenceNormalizer::FunctionName,
            ReferenceValueCardinality::One,
            None,
            NormalizedReferenceValue::Record(BTreeMap::from([
                (
                    "target-content-type".to_owned(),
                    NormalizedReferenceValue::Scalar("application/xml".to_owned()),
                ),
                (
                    "target-category".to_owned(),
                    NormalizedReferenceValue::Scalar("document".to_owned()),
                ),
            ])),
        );
        let result = compare_references(ReferenceComparisonInput {
            operator: ReferenceComparisonOperator::RecordFieldsEqual,
            actual: actual(actual_record),
            expected: Some(expected(expected_record)),
            forbidden: None,
            state_policy: ReferenceStatePolicy::RequiredValid,
        });
        assert!(!result.passed);
        assert_eq!(
            result.invalid_values["target-content-type"],
            serde_json::json!("text/html")
        );
        assert!(!result.invalid_fields.contains("target-category"));
    }

    #[test]
    fn comparison_state_policies_keep_missing_and_unresolved_explicit() {
        let required_missing = compare_references(ReferenceComparisonInput {
            operator: ReferenceComparisonOperator::Equals,
            actual: actual(ReferenceValue::missing(
                "profile",
                ReferenceNormalizer::ProfileName,
            )),
            expected: Some(expected(normalize_profile_name(
                "expectedProfile",
                "compact",
            ))),
            forbidden: None,
            state_policy: ReferenceStatePolicy::RequiredValid,
        });
        assert!(!required_missing.passed);
        assert_eq!(
            required_missing.missing_values["profile"],
            serde_json::json!("missing-value")
        );

        let optional_missing = compare_references(ReferenceComparisonInput {
            operator: ReferenceComparisonOperator::Equals,
            actual: actual(ReferenceValue::missing(
                "profile",
                ReferenceNormalizer::ProfileName,
            )),
            expected: Some(expected(normalize_profile_name(
                "expectedProfile",
                "compact",
            ))),
            forbidden: None,
            state_policy: ReferenceStatePolicy::CompareWhenPresent,
        });
        assert!(optional_missing.passed);

        let optional_missing_with_invalid_expected = compare_references(ReferenceComparisonInput {
            operator: ReferenceComparisonOperator::Equals,
            actual: actual(ReferenceValue::missing(
                "profile",
                ReferenceNormalizer::ProfileName,
            )),
            expected: Some(expected(normalize_profile_name(
                "expectedProfile",
                "bad value",
            ))),
            forbidden: None,
            state_policy: ReferenceStatePolicy::CompareWhenPresent,
        });
        assert!(!optional_missing_with_invalid_expected.passed);
        assert_eq!(
            optional_missing_with_invalid_expected.invalid_values["expectedProfile"],
            serde_json::json!("invalid-scalar")
        );

        let unresolved = compare_references(ReferenceComparisonInput {
            operator: ReferenceComparisonOperator::Equals,
            actual: actual(normalize_schema_uri("schema", "missing:", None)),
            expected: Some(expected(normalize_schema_uri(
                "expectedSchema",
                "declared",
                Some("schema:a"),
            ))),
            forbidden: None,
            state_policy: ReferenceStatePolicy::UnresolvedFails,
        });
        assert!(!unresolved.passed);
        assert_eq!(
            unresolved.unresolved_values["schema"],
            serde_json::json!("unresolved-schema")
        );
    }

    #[test]
    fn comparison_metadata_records_operator_and_normalizers() {
        let result = compare_references(ReferenceComparisonInput {
            operator: ReferenceComparisonOperator::MemberOf,
            actual: actual(normalize_media_type_essence("content-type", "text/html")),
            expected: Some(expected(normalize_media_type_essence_set(
                "contentTypes",
                ["text/html"],
            ))),
            forbidden: None,
            state_policy: ReferenceStatePolicy::RequiredValid,
        });
        assert!(result.passed);
        assert_eq!(result.comparison["operator"], "schema:member-of");
        assert_eq!(
            result.comparison["actualNormalizer"],
            "schema:media-type-essence"
        );
    }
}
