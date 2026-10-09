//! Explicit host-selected shipped contracts. Names never select these capabilities.
//! Keep compatibility predicates here shared with document validation; grammar
//! availability and conversion availability are independent of lexical acceptance.
use super::*;
use crate::schema::{
    datatype_registry::DatatypeKind,
    datatype_validation::{ScalarRepresentation as S, ValueRepresentation as V},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShippedDatatype {
    Identifier,
    QualifiedName,
    SymbolReference,
    WildcardName,
    String,
    Boolean,
    ReferenceUnresolvedDisposition,
    Integer,
    Number,
    Uri,
    Semver,
    MediaType,
    Path,
    NameList,
    WildcardNameList,
    ContentModel,
    TypeReference,
    WildcardTypeReference,
}
impl ShippedDatatype {
    pub const ALL: [Self; 18] = [
        Self::Identifier,
        Self::QualifiedName,
        Self::SymbolReference,
        Self::WildcardName,
        Self::String,
        Self::Boolean,
        Self::ReferenceUnresolvedDisposition,
        Self::Integer,
        Self::Number,
        Self::Uri,
        Self::Semver,
        Self::MediaType,
        Self::Path,
        Self::NameList,
        Self::WildcardNameList,
        Self::ContentModel,
        Self::TypeReference,
        Self::WildcardTypeReference,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Identifier => "identifier",
            Self::QualifiedName => "qualified-name",
            Self::SymbolReference => "symbol-reference",
            Self::WildcardName => "wildcard-name",
            Self::String => "string",
            Self::Boolean => "boolean",
            Self::ReferenceUnresolvedDisposition => "reference-unresolved-disposition",
            Self::Integer => "integer",
            Self::Number => "number",
            Self::Uri => "uri",
            Self::Semver => "semver",
            Self::MediaType => "media-type",
            Self::Path => "path",
            Self::NameList => "name-list",
            Self::WildcardNameList => "wildcard-name-list",
            Self::ContentModel => "content-model",
            Self::TypeReference => "type-reference",
            Self::WildcardTypeReference => "wildcard-type-reference",
        }
    }
    pub fn kind(self) -> DatatypeKind {
        match self {
            Self::Identifier | Self::QualifiedName | Self::SymbolReference | Self::WildcardName => {
                DatatypeKind::Lexical
            }
            Self::NameList | Self::WildcardNameList => DatatypeKind::List,
            Self::ContentModel => DatatypeKind::Grammar,
            Self::TypeReference | Self::WildcardTypeReference => DatatypeKind::Reference,
            _ => DatatypeKind::Scalar,
        }
    }
    pub fn representation(self) -> V {
        match self {
            Self::Boolean => V::Scalar(S::Boolean),
            Self::Integer => V::Scalar(S::Integer),
            Self::Number => V::Scalar(S::Decimal),
            Self::NameList | Self::WildcardNameList => V::List(S::String),
            _ => V::Scalar(S::String),
        }
    }
    pub fn item(self) -> Option<Self> {
        match self {
            Self::NameList => Some(Self::Identifier),
            Self::WildcardNameList => Some(Self::WildcardName),
            _ => None,
        }
    }
    /// Literal validation preserves existing whitespace and boolean-presence rules.
    /// None means no executable grammar implementation, never successful validation.
    pub fn validate_lexical(self, value: &str) -> Option<bool> {
        let value = value.trim();
        Some(match self {
            Self::Identifier => is_cem_local_name(value),
            Self::QualifiedName | Self::TypeReference => is_cem_qualified_name(value),
            Self::SymbolReference => is_cem_symbol_reference(value),
            Self::WildcardName => is_cem_wildcard_name(value),
            Self::String => true,
            Self::Boolean => matches!(value, "" | "true" | "false"),
            Self::ReferenceUnresolvedDisposition => {
                matches!(value, "neutral" | "mandatory" | "warning" | "ignore")
            }
            Self::Integer => is_signed_decimal_integer(value),
            Self::Number => is_finite_decimal_number(value),
            Self::Uri => is_absolute_uri(value),
            Self::Semver => is_semver(value),
            Self::MediaType => is_media_type(value),
            Self::Path => is_scoped_path_specifier(value),
            Self::NameList => is_cem_name_list(value),
            Self::WildcardNameList => is_cem_wildcard_name_list(value),
            Self::WildcardTypeReference => is_cem_wildcard_type_reference(value),
            Self::ContentModel => return None,
        })
    }
    /// Reuse the shipped explicit normalization contract. Unsupported conversion
    /// has no capability; it is not a failed attempt to parse some supplied value.
    pub fn convert_lexical<E>(
        self,
        value: &str,
        source: &SourceMapStack,
        check: &mut impl FnMut() -> Result<(), E>,
    ) -> Option<Result<TypedAttributeValue, AttributeValueConversionError<E>>> {
        if !matches!(
            self,
            Self::String | Self::Boolean | Self::Integer | Self::Number
        ) {
            return None;
        }
        Some(convert_attribute_value_with_check(
            value,
            &AttributeValueContract {
                model: AttributeModel {
                    value_type: Some(self.name().into()),
                    ..Default::default()
                },
                ..Default::default()
            },
            source,
            check,
        ))
    }
}
