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
}
