//! Serializable run/root-scope configuration shared by library hosts,
//! WASM adapters, and the CLI.
//!
//! The model is intentionally an array shape: build/CI callers can
//! validate or transform several documents in one run while preserving
//! each document root as scope zero for diagnostics, source maps, schema
//! selection, and resource policy accounting.

use crate::diagnostics::Diagnostic;
use crate::engine::FormatIdentity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunConfig {
    #[serde(default)]
    pub inputs: Vec<InputSpec>,
    #[serde(default)]
    pub outputs: Vec<OutputSpec>,
    #[serde(default)]
    pub scheduler: SchedulerConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputSpec {
    pub uri: String,
    #[serde(default)]
    pub root_scope: ScopeConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputSpec {
    #[serde(default)]
    pub input_ref: Option<String>,
    #[serde(default)]
    pub destination: Option<String>,
    #[serde(default)]
    pub root_scope: ScopeConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeConfig {
    #[serde(default)]
    pub default_content_type: Option<String>,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub version_pins: BTreeMap<String, String>,
    #[serde(default)]
    pub default_namespace: Option<String>,
    #[serde(default)]
    pub namespaces: BTreeMap<String, String>,
    #[serde(default)]
    pub module_map: Option<String>,
    #[serde(default)]
    pub base_uri: Option<String>,
    #[serde(default)]
    pub policy: Option<String>,
    #[serde(default)]
    pub budgets: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerConfig {
    #[serde(default)]
    pub thread_pool: Option<String>,
    #[serde(default)]
    pub max_parallel_documents: Option<u32>,
}

impl ScopeConfig {
    pub fn format_identity(&self) -> FormatIdentity {
        FormatIdentity {
            content_type: self.default_content_type.clone(),
            schema: self.schema.clone(),
            base_uri: self.base_uri.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfigParseRequest {
    pub bytes: Vec<u8>,
    pub identity: FormatIdentity,
    pub base_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfigParseResponse {
    pub config: RunConfig,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfigError {
    pub code: &'static str,
    pub message: String,
}

impl std::fmt::Display for RunConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for RunConfigError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecParseError {
    pub message: String,
}

impl std::fmt::Display for SpecParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SpecParseError {}

pub fn parse_run_config(
    request: RunConfigParseRequest,
) -> Result<RunConfigParseResponse, RunConfigError> {
    let content_type = request
        .identity
        .content_type
        .as_deref()
        .map(content_type_essence)
        .unwrap_or_else(|| "application/json".to_owned());

    match content_type.as_str() {
        "application/json" | "text/json" => {
            let config = serde_json::from_slice::<RunConfig>(&request.bytes).map_err(|error| {
                run_config_error(
                    "cem.run_config.invalid_json",
                    format!("run config JSON could not be parsed: {error}"),
                )
            })?;
            Ok(RunConfigParseResponse {
                config,
                diagnostics: Vec::new(),
            })
        }
        other => Err(run_config_error(
            "cem.run_config.unsupported_content_type",
            format!("run config content type `{other}` is not supported yet; use application/json"),
        )),
    }
}

pub fn parse_input_spec_record(record: &str) -> Result<InputSpec, SpecParseError> {
    let fields = parse_key_value_record(record)?;
    let mut spec = InputSpec::default();

    for (key, value) in fields {
        match normalize_key(&key).as_str() {
            "uri" | "path" => spec.uri = value,
            key => apply_scope_field(&mut spec.root_scope, key, value)?,
        }
    }

    if spec.uri.trim().is_empty() {
        return Err(parse_error("input spec requires `uri` or `path`"));
    }

    Ok(spec)
}

pub fn parse_output_spec_record(record: &str) -> Result<OutputSpec, SpecParseError> {
    let fields = parse_key_value_record(record)?;
    let mut spec = OutputSpec::default();

    for (key, value) in fields {
        match normalize_key(&key).as_str() {
            "input" | "inputref" => spec.input_ref = Some(value),
            "dest" | "destination" | "out" => spec.destination = Some(value),
            key => apply_scope_field(&mut spec.root_scope, key, value)?,
        }
    }

    Ok(spec)
}

fn apply_scope_field(
    scope: &mut ScopeConfig,
    normalized_key: &str,
    value: String,
) -> Result<(), SpecParseError> {
    match normalized_key {
        "contenttype" | "defaultcontenttype" => scope.default_content_type = Some(value),
        "schema" => scope.schema = Some(value),
        "defaultnamespace" | "defaultns" => scope.default_namespace = Some(value),
        "modulemap" => scope.module_map = Some(value),
        "baseuri" => scope.base_uri = Some(value),
        "policy" => scope.policy = Some(value),
        "namespaces" | "ns" => scope.namespaces = parse_map_field(&value)?,
        "versions" | "versionpins" => scope.version_pins = parse_map_field(&value)?,
        "budgets" => scope.budgets = parse_map_field(&value)?,
        other => {
            return Err(parse_error(format!(
                "unsupported spec field `{other}`; use config files for nested data"
            )));
        }
    }
    Ok(())
}

fn parse_key_value_record(record: &str) -> Result<Vec<(String, String)>, SpecParseError> {
    let mut fields = Vec::new();
    for field in split_escaped(record, ',')? {
        if field.trim().is_empty() {
            continue;
        }
        let Some((key, value)) = split_key_value(&field)? else {
            return Err(parse_error(format!(
                "spec field `{}` is missing `=`",
                field.trim()
            )));
        };
        fields.push((key.trim().to_owned(), value.trim().to_owned()));
    }
    Ok(fields)
}

fn split_key_value(field: &str) -> Result<Option<(String, String)>, SpecParseError> {
    let mut in_quote = false;
    let mut escape = false;

    for (idx, ch) in field.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match ch {
            '\\' => escape = true,
            '"' => in_quote = !in_quote,
            '=' if !in_quote => {
                let key = field[..idx].to_owned();
                let value = unquote(field[idx + 1..].trim())?;
                return Ok(Some((key, value)));
            }
            _ => {}
        }
    }

    if in_quote {
        return Err(parse_error("unterminated quoted spec field"));
    }
    Ok(None)
}

fn parse_map_field(value: &str) -> Result<BTreeMap<String, String>, SpecParseError> {
    let mut map = BTreeMap::new();
    if value.trim().is_empty() {
        return Ok(map);
    }

    for pair in split_escaped(value, '|')? {
        let Some((key, value)) = pair.split_once(':') else {
            return Err(parse_error(format!(
                "map entry `{pair}` is missing `:` separator"
            )));
        };
        map.insert(key.trim().to_owned(), value.trim().to_owned());
    }

    Ok(map)
}

fn split_escaped(input: &str, delimiter: char) -> Result<Vec<String>, SpecParseError> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut escape = false;

    for ch in input.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }

        match ch {
            '\\' => escape = true,
            '"' => {
                in_quote = !in_quote;
                current.push(ch);
            }
            c if c == delimiter && !in_quote => {
                parts.push(current.trim().to_owned());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if escape {
        current.push('\\');
    }
    if in_quote {
        return Err(parse_error("unterminated quoted spec record"));
    }

    parts.push(current.trim().to_owned());
    Ok(parts)
}

fn unquote(value: &str) -> Result<String, SpecParseError> {
    let trimmed = value.trim();
    if !(trimmed.starts_with('"') || trimmed.ends_with('"')) {
        return Ok(trimmed.to_owned());
    }
    if !(trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2) {
        return Err(parse_error(format!("malformed quoted value `{trimmed}`")));
    }
    Ok(trimmed[1..trimmed.len() - 1].to_owned())
}

fn normalize_key(key: &str) -> String {
    key.chars()
        .filter(|ch| *ch != '-' && *ch != '_' && !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn parse_error(message: impl Into<String>) -> SpecParseError {
    SpecParseError {
        message: message.into(),
    }
}

fn run_config_error(code: &'static str, message: impl Into<String>) -> RunConfigError {
    RunConfigError {
        code,
        message: message.into(),
    }
}

fn content_type_essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_spec_record_maps_identity_and_scope_fields() {
        let spec = parse_input_spec_record(
            r#"uri=src/a.cem,contentType=application/cem+xml,schema=https://cem.dev/ns/core/1,defaultNs=https://cem.dev/ns/core,namespaces=html:https://www.w3.org/1999/xhtml|svg:http://www.w3.org/2000/svg,moduleMap=cem.modules.json"#,
        )
        .unwrap();

        assert_eq!(spec.uri, "src/a.cem");
        assert_eq!(
            spec.root_scope.default_content_type.as_deref(),
            Some("application/cem+xml")
        );
        assert_eq!(
            spec.root_scope.schema.as_deref(),
            Some("https://cem.dev/ns/core/1")
        );
        assert_eq!(
            spec.root_scope.namespaces.get("html").map(String::as_str),
            Some("https://www.w3.org/1999/xhtml")
        );
        assert_eq!(
            spec.root_scope.module_map.as_deref(),
            Some("cem.modules.json")
        );
    }

    #[test]
    fn spec_record_supports_quoted_commas() {
        let spec = parse_input_spec_record(
            r#"uri="src/a,one.cem",contentType="text/custom-element-xslt""#,
        )
        .unwrap();

        assert_eq!(spec.uri, "src/a,one.cem");
        assert_eq!(
            spec.root_scope.default_content_type.as_deref(),
            Some("text/custom-element-xslt")
        );
    }

    #[test]
    fn output_spec_record_maps_target_scope() {
        let spec = parse_output_spec_record(
            "input=src/a.cem,dest=dist/a.cem,contentType=application/cem+xml,schema=core",
        )
        .unwrap();

        assert_eq!(spec.input_ref.as_deref(), Some("src/a.cem"));
        assert_eq!(spec.destination.as_deref(), Some("dist/a.cem"));
        assert_eq!(
            spec.root_scope.default_content_type.as_deref(),
            Some("application/cem+xml")
        );
        assert_eq!(spec.root_scope.schema.as_deref(), Some("core"));
    }

    #[test]
    fn json_run_config_parses_by_content_type() {
        let response = parse_run_config(RunConfigParseRequest {
            bytes: br#"{"inputs":[{"uri":"src/a.cem","rootScope":{"defaultContentType":"application/cem+xml"}}]}"#.to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json; charset=utf-8".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .unwrap();

        assert_eq!(response.config.inputs.len(), 1);
        assert_eq!(response.config.inputs[0].uri, "src/a.cem");
        assert_eq!(
            response.config.inputs[0]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("application/cem+xml")
        );
        assert!(response.diagnostics.is_empty());
    }

    #[test]
    fn unsupported_run_config_content_type_is_rejected_before_document_work() {
        let error = parse_run_config(RunConfigParseRequest {
            bytes: b"{}".to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/cem+xml".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .unwrap_err();

        assert_eq!(error.code, "cem.run_config.unsupported_content_type");
    }
}
