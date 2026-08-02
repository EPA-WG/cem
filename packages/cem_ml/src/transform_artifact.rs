use crate::engine::FormatIdentity;
use crate::interpreter::OutputSpan;
use crate::lifecycle::LoadedInputAstStream;
use crate::parser::document::CemDocument;
use crate::projection::CemTreeAstStream;
use crate::schema::registry::{content_type_essence, JSON_CONTENT_TYPE};
use crate::source_map::SourceMapStack;
use crate::validation::generic_data::GenericDataDocumentAst;
use crate::validation::xpath::XPathResultArtifact;
use std::any::Any;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TransformDataArtifact {
    pub artifact_id: String,
    pub uri: Option<String>,
    pub identity: Option<FormatIdentity>,
    pub body: TransformArtifactBody,
}

impl TransformDataArtifact {
    pub fn new(
        artifact_id: impl Into<String>,
        uri: Option<String>,
        identity: Option<FormatIdentity>,
        body: TransformArtifactBody,
    ) -> Self {
        Self {
            artifact_id: artifact_id.into(),
            uri,
            identity,
            body,
        }
    }

    pub fn encoded(
        artifact_id: impl Into<String>,
        uri: Option<String>,
        identity: FormatIdentity,
        encoding: TransformEncoding,
        bytes: impl Into<Arc<[u8]>>,
    ) -> Result<Self, TransformEncodedArtifactError> {
        let encoded = TransformEncodedArtifact::new(identity.clone(), encoding, bytes)?;
        Ok(Self::new(
            artifact_id,
            uri,
            Some(identity),
            TransformArtifactBody::Encoded(Arc::new(encoded)),
        ))
    }

    pub fn explicit_json(
        artifact_id: impl Into<String>,
        uri: Option<String>,
        identity: FormatIdentity,
        value: &serde_json::Value,
    ) -> Result<Self, TransformEncodedArtifactError> {
        let bytes = serde_json::to_vec(value).map_err(|error| {
            TransformEncodedArtifactError::JsonEncodingFailed {
                message: error.to_string(),
            }
        })?;
        Self::encoded(artifact_id, uri, identity, TransformEncoding::Json, bytes)
    }

    pub fn explicit_json_bytes(&self) -> Result<&[u8], TransformArtifactAccessError> {
        let TransformArtifactBody::Encoded(encoded) = &self.body else {
            return Err(TransformArtifactAccessError::UnsupportedRepresentation {
                expected: "explicit encoded JSON",
                actual: self.body.representation_id(),
            });
        };
        if encoded.encoding != TransformEncoding::Json
            || !identity_has_json_content_type(&encoded.identity)
        {
            return Err(TransformArtifactAccessError::UnsupportedRepresentation {
                expected: "explicit encoded JSON",
                actual: self.body.representation_id(),
            });
        }
        Ok(encoded.bytes.as_ref())
    }

    pub fn explicit_json_value(&self) -> Result<serde_json::Value, TransformArtifactAccessError> {
        serde_json::from_slice(self.explicit_json_bytes()?).map_err(|error| {
            TransformArtifactAccessError::InvalidJson {
                message: error.to_string(),
            }
        })
    }

    pub fn encoded_text(&self) -> Result<&str, TransformArtifactAccessError> {
        let TransformArtifactBody::Encoded(encoded) = &self.body else {
            return Err(TransformArtifactAccessError::UnsupportedRepresentation {
                expected: "encoded UTF-8 text",
                actual: self.body.representation_id(),
            });
        };
        if encoded.encoding != TransformEncoding::Text {
            return Err(TransformArtifactAccessError::UnsupportedRepresentation {
                expected: "encoded UTF-8 text",
                actual: self.body.representation_id(),
            });
        }
        std::str::from_utf8(encoded.bytes.as_ref()).map_err(|error| {
            TransformArtifactAccessError::InvalidUtf8 {
                message: error.to_string(),
            }
        })
    }
}

#[derive(Clone)]
pub enum TransformArtifactBody {
    Lifecycle(Arc<LoadedInputAstStream>),
    CemDocument(Arc<CemDocument>),
    GenericData(Arc<GenericDataDocumentAst>),
    CemTree(Arc<CemTreeAstStream>),
    XPathResult(Arc<XPathResultArtifact>),
    Collection(Arc<TransformArtifactCollection>),
    Extension(Arc<dyn TransformNativeArtifact>),
    Encoded(Arc<TransformEncodedArtifact>),
}

impl TransformArtifactBody {
    pub fn representation_id(&self) -> &'static str {
        match self {
            Self::Lifecycle(_) => "cem.lifecycle-ast",
            Self::CemDocument(_) => "cem.document-ast",
            Self::GenericData(_) => "cem.generic-data-ast",
            Self::CemTree(_) => "cem.tree-ast",
            Self::XPathResult(_) => "cem.xpath-result",
            Self::Collection(_) => "cem.transform-collection",
            Self::Extension(artifact) => artifact.representation_id(),
            Self::Encoded(_) => "cem.encoded",
        }
    }
}

impl fmt::Debug for TransformArtifactBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransformArtifactBody")
            .field("representation_id", &self.representation_id())
            .finish_non_exhaustive()
    }
}

pub trait TransformNativeArtifact: Any + Send + Sync {
    fn representation_id(&self) -> &'static str;
    fn source_map(&self) -> Option<&SourceMapStack>;
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformArtifactCollectionMode {
    Collect,
    GroupBy,
    MatchBy,
    Zip,
}

#[derive(Debug, Clone)]
pub struct TransformArtifactCollection {
    pub mode: TransformArtifactCollectionMode,
    pub bindings: BTreeMap<String, String>,
    pub items: Vec<TransformArtifactCollectionItem>,
}

#[derive(Debug, Clone)]
pub struct TransformArtifactCollectionItem {
    pub input_name: String,
    pub destination: Option<String>,
    pub target: Option<FormatIdentity>,
    pub bindings: BTreeMap<String, String>,
    pub artifact: Arc<TransformDataArtifact>,
    pub source_map: Option<SourceMapStack>,
    pub output_spans: Vec<OutputSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformEncoding {
    Text,
    Json,
    Binary,
}

#[derive(Debug, Clone)]
pub struct TransformEncodedArtifact {
    pub identity: FormatIdentity,
    pub encoding: TransformEncoding,
    pub bytes: Arc<[u8]>,
}

impl TransformEncodedArtifact {
    pub fn new(
        identity: FormatIdentity,
        encoding: TransformEncoding,
        bytes: impl Into<Arc<[u8]>>,
    ) -> Result<Self, TransformEncodedArtifactError> {
        if encoding == TransformEncoding::Json && !identity_has_json_content_type(&identity) {
            return Err(TransformEncodedArtifactError::JsonIdentityRequired {
                content_type: identity.content_type.clone(),
            });
        }
        Ok(Self {
            identity,
            encoding,
            bytes: bytes.into(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransformEncodedArtifactError {
    JsonIdentityRequired { content_type: Option<String> },
    JsonEncodingFailed { message: String },
}

impl fmt::Display for TransformEncodedArtifactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JsonIdentityRequired { content_type } => write!(
                f,
                "JSON encoding requires an explicit JSON or +json content type, got {}",
                content_type.as_deref().unwrap_or("no content type")
            ),
            Self::JsonEncodingFailed { message } => {
                write!(f, "JSON transform artifact encoding failed: {message}")
            }
        }
    }
}

impl std::error::Error for TransformEncodedArtifactError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransformArtifactAccessError {
    UnsupportedRepresentation {
        expected: &'static str,
        actual: &'static str,
    },
    InvalidUtf8 {
        message: String,
    },
    InvalidJson {
        message: String,
    },
}

impl fmt::Display for TransformArtifactAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedRepresentation { expected, actual } => {
                write!(f, "transform artifact representation `{actual}` is unsupported; expected {expected}")
            }
            Self::InvalidUtf8 { message } => {
                write!(
                    f,
                    "encoded transform artifact is not valid UTF-8: {message}"
                )
            }
            Self::InvalidJson { message } => {
                write!(f, "encoded JSON transform artifact is invalid: {message}")
            }
        }
    }
}

impl std::error::Error for TransformArtifactAccessError {}

fn identity_has_json_content_type(identity: &FormatIdentity) -> bool {
    identity
        .content_type
        .as_deref()
        .is_some_and(|content_type| {
            let essence = content_type_essence(content_type);
            essence == JSON_CONTENT_TYPE || essence.ends_with("+json")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_encoded_artifact_requires_explicit_json_identity() {
        let error = TransformEncodedArtifact::new(
            FormatIdentity {
                content_type: Some("application/xml".to_owned()),
                ..FormatIdentity::default()
            },
            TransformEncoding::Json,
            Vec::<u8>::new(),
        )
        .expect_err("non-JSON identity must be rejected");
        assert!(matches!(
            error,
            TransformEncodedArtifactError::JsonIdentityRequired { .. }
        ));

        assert!(TransformEncodedArtifact::new(
            FormatIdentity {
                content_type: Some("application/vnd.cem.dom+json".to_owned()),
                ..FormatIdentity::default()
            },
            TransformEncoding::Json,
            Vec::<u8>::new(),
        )
        .is_ok());
    }
}
