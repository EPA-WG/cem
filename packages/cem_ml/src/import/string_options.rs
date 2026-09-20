//! Format/profile selection for already-decoded strings belongs to import.
use super::{
    CsvHeader, CsvImportOptions, ImportStringProfile, JsonXmlDuplicates, JsonXmlProjectionOptions,
};
use std::collections::BTreeMap;

/// Native scalar control metadata, never a parsed document representation.
#[derive(Debug, Clone)]
pub enum ImportStringOption {
    String(String),
    Boolean(bool),
}

pub struct ImportStringConfig {
    pub profile: ImportStringProfile,
    pub base_uri: Option<String>,
}

/// Resolve the same profiles used by XPath parsing without putting external
/// format names, option semantics or decoding in a consuming query evaluator.
/// Unknown options fail explicitly rather than silently changing interpretation.
pub fn resolve_string_import(
    format: &str,
    options: &BTreeMap<String, ImportStringOption>,
) -> Result<ImportStringConfig, String> {
    let mut profile = match format {
        "xml" | "application/xml" | "text/xml" => ImportStringProfile::Xml,
        "json" | "application/json" => {
            ImportStringProfile::JsonXml(JsonXmlProjectionOptions::default())
        }
        "csv" | "text/csv" => ImportStringProfile::CsvWithOptions(CsvImportOptions::default()),
        "yaml" | "application/yaml" | "text/yaml" => ImportStringProfile::Yaml,
        _ => return Err(format!("Unsupported string import format: {format}")),
    };
    let mut base_uri = None;
    for (name, value) in options {
        match (name.as_str(), value, &mut profile) {
            ("base-uri", ImportStringOption::String(uri), _) => base_uri = Some(uri.clone()),
            (
                "escape",
                ImportStringOption::Boolean(escape),
                ImportStringProfile::JsonXml(options),
            ) => options.escape = *escape,
            (
                "duplicates",
                ImportStringOption::String(value),
                ImportStringProfile::JsonXml(options),
            ) => {
                options.duplicates = match value.as_str() {
                    "retain" => JsonXmlDuplicates::Retain,
                    "use-first" => JsonXmlDuplicates::UseFirst,
                    "reject" => JsonXmlDuplicates::Reject,
                    _ => return Err("duplicates must be retain, use-first or reject".into()),
                };
            }
            (
                "header",
                ImportStringOption::String(value),
                ImportStringProfile::CsvWithOptions(options),
            ) => {
                options.header = match value.as_str() {
                    "absent" => CsvHeader::Absent,
                    "present" => CsvHeader::Present,
                    _ => return Err("header must be absent or present".into()),
                };
            }
            _ => return Err(format!("Invalid string import option {name} for {format}")),
        }
    }
    Ok(ImportStringConfig { profile, base_uri })
}
