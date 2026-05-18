use crate::source_map::SourceMapStack;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Fatal,
}

impl Severity {
    pub fn is_hard_violation(self) -> bool {
        matches!(self, Severity::Error | Severity::Fatal)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub uri: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    #[serde(rename = "byteOffset")]
    pub byte_offset: Option<u64>,
    pub code: String,
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    /// Origin-first source-map stack, projected on demand into `line`/
    /// `column`. Per `cem-ml-cli-contract.md` §Output Shapes the JSON key
    /// is `sourceMap`.
    #[serde(rename = "sourceMap", skip_serializing_if = "Option::is_none")]
    pub source_map: Option<SourceMapStack>,
}

impl Default for Diagnostic {
    fn default() -> Self {
        Self {
            uri: None,
            line: None,
            column: None,
            byte_offset: None,
            code: String::new(),
            severity: Severity::Info,
            message: String::new(),
            node: None,
            source_map: None,
        }
    }
}
