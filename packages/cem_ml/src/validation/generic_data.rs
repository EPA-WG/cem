use crate::source_map::SourceMapStack;
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericDataDocumentAst {
    pub source: GenericDataSourceAst,
    pub documents: Vec<GenericDataStreamDocumentAst>,
    pub line_ending: Option<String>,
}

impl GenericDataDocumentAst {
    pub fn source_line_ending(&self) -> Option<&str> {
        self.line_ending.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericDataSourceAst {
    pub uri: String,
    pub content_type: String,
    pub media_type: String,
    pub parameters: BTreeMap<String, String>,
    pub byte_length: usize,
}

impl GenericDataSourceAst {
    pub fn to_cemt_subject(&self) -> Value {
        json!({
            "uri": self.uri,
            "contentType": self.content_type,
            "mediaType": self.media_type,
            "parameters": self.parameters,
            "byteLength": self.byte_length,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericDataStreamDocumentAst {
    pub index: usize,
    pub source_range: GenericDataSourceRangeAst,
    pub root: Option<GenericDataValueAst>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericDataMappingEntryAst {
    pub index: usize,
    pub key: GenericDataValueAst,
    pub value: GenericDataValueAst,
    pub source_range: GenericDataSourceRangeAst,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericDataValueAst {
    Mapping {
        source_range: GenericDataSourceRangeAst,
        entries: Vec<GenericDataMappingEntryAst>,
    },
    Sequence {
        source_range: GenericDataSourceRangeAst,
        items: Vec<GenericDataValueAst>,
    },
    String {
        source_range: GenericDataSourceRangeAst,
        value: String,
        lexeme: Option<String>,
        style: Option<String>,
    },
    Number {
        source_range: GenericDataSourceRangeAst,
        lexeme: String,
        number_kind: GenericDataNumberKind,
    },
    Boolean {
        source_range: GenericDataSourceRangeAst,
        value: bool,
    },
    Null {
        source_range: GenericDataSourceRangeAst,
    },
    Alias {
        source_range: GenericDataSourceRangeAst,
        alias: Option<String>,
    },
}

impl GenericDataValueAst {
    pub fn source_range(&self) -> &GenericDataSourceRangeAst {
        match self {
            Self::Mapping { source_range, .. }
            | Self::Sequence { source_range, .. }
            | Self::String { source_range, .. }
            | Self::Number { source_range, .. }
            | Self::Boolean { source_range, .. }
            | Self::Null { source_range }
            | Self::Alias { source_range, .. } => source_range,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericDataNumberKind {
    Integer,
    Decimal,
    Exponent,
}

impl GenericDataNumberKind {
    pub fn as_json_number_kind(self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Decimal => "decimal",
            Self::Exponent => "exponent",
        }
    }

    pub fn as_yaml_implicit_kind(self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Decimal | Self::Exponent => "float",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericDataSourceRangeAst {
    pub byte_offset: u64,
    pub byte_length: u64,
    pub line: u32,
    pub column: u32,
    pub source_map: Option<SourceMapStack>,
}

impl GenericDataSourceRangeAst {
    pub fn generated() -> Self {
        Self {
            byte_offset: 0,
            byte_length: 0,
            line: 1,
            column: 1,
            source_map: None,
        }
    }

    pub fn to_cemt_subject(&self) -> Value {
        json!({
            "byteOffset": self.byte_offset,
            "byteLength": self.byte_length,
            "line": self.line,
            "column": self.column,
        })
    }

    pub fn source_map_subject(&self) -> Value {
        self.source_map
            .as_ref()
            .map(|source_map| json!(source_map))
            .unwrap_or(Value::Null)
    }
}
