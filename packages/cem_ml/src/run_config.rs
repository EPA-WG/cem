//! Serializable run/root-scope configuration shared by library hosts,
//! WASM adapters, and the CLI.
//!
//! The model is intentionally an array shape: build/CI callers can
//! validate or transform several documents in one run while preserving
//! each document root as scope zero for diagnostics, source maps, schema
//! selection, and resource policy accounting.

use crate::diagnostics::{Diagnostic, Severity};
use crate::engine::FormatIdentity;
use crate::resolver::{has_uri_scheme, local_file_uri_to_path};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const RUN_CONFIG_SCHEMA_URI: &str = "https://cem.dev/ns/cli/run-config/1";
pub const RUN_CONFIG_NAMESPACE_URI: &str = RUN_CONFIG_SCHEMA_URI;
pub const RUN_CONFIG_JSON_SCHEMA_URI: &str = "https://cem.dev/schema/cli/run-config.schema.json";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunConfig {
    #[serde(default)]
    pub inputs: Vec<InputSpec>,
    #[serde(default)]
    pub outputs: Vec<OutputSpec>,
    #[serde(default)]
    pub resolvers: Vec<ResolverSpec>,
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
pub struct ResolverSpec {
    pub uri_prefix: String,
    pub local_root: String,
    #[serde(default)]
    pub read: bool,
    #[serde(default)]
    pub write: bool,
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
            default_namespace: self.default_namespace.clone(),
            namespaces: self.namespaces.clone(),
            base_uri: self.base_uri.clone(),
        }
    }

    pub fn format_identity_option(&self) -> Option<FormatIdentity> {
        let identity = self.format_identity();
        (identity.content_type.is_some()
            || identity.schema.is_some()
            || identity.default_namespace.is_some()
            || !identity.namespaces.is_empty()
            || identity.base_uri.is_some())
        .then_some(identity)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunConfigDefaults {
    #[serde(default)]
    pub input_scope: ScopeConfig,
    #[serde(default)]
    pub output_scope: ScopeConfig,
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
    validate_run_config_identity(&request.identity)?;

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

fn validate_run_config_identity(identity: &FormatIdentity) -> Result<(), RunConfigError> {
    if let Some(schema) = identity.schema.as_deref().map(str::trim) {
        if !schema.is_empty() && schema != RUN_CONFIG_SCHEMA_URI {
            return Err(run_config_error(
                "cem.run_config.unsupported_schema_identity",
                format!(
                    "run config schema `{schema}` is not supported; expected `{RUN_CONFIG_SCHEMA_URI}`"
                ),
            ));
        }
    }

    if let Some(default_namespace) = identity.default_namespace.as_deref().map(str::trim) {
        if !default_namespace.is_empty() && default_namespace != RUN_CONFIG_NAMESPACE_URI {
            return Err(run_config_error(
                "cem.run_config.unsupported_schema_identity",
                format!(
                    "run config namespace `{default_namespace}` is not supported; expected `{RUN_CONFIG_NAMESPACE_URI}`"
                ),
            ));
        }
    }

    Ok(())
}

pub fn normalize_run_config(
    mut config: RunConfig,
    defaults: RunConfigDefaults,
    base_uri: Option<&str>,
) -> RunConfigParseResponse {
    for input in &mut config.inputs {
        merge_scope_defaults(&mut input.root_scope, &defaults.input_scope);
        resolve_scope_module_map(&mut input.root_scope, base_uri);
        if input.root_scope.default_content_type.is_none() {
            input.root_scope.default_content_type = infer_content_type_from_path(&input.uri);
        }
    }

    for output in &mut config.outputs {
        merge_scope_defaults(&mut output.root_scope, &defaults.output_scope);
        resolve_scope_module_map(&mut output.root_scope, base_uri);
        resolve_output_destination(output, base_uri);
        if output.root_scope.default_content_type.is_none() {
            if let Some(destination) = output.destination.as_deref() {
                output.root_scope.default_content_type = infer_content_type_from_path(destination);
            }
        }
    }

    let mut diagnostics = validate_run_config_defaults(&defaults, base_uri);
    diagnostics.extend(validate_run_config(&config, base_uri));
    RunConfigParseResponse {
        config,
        diagnostics,
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

pub fn validate_run_config(config: &RunConfig, base_uri: Option<&str>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut input_uris = std::collections::BTreeSet::new();

    for (index, input) in config.inputs.iter().enumerate() {
        if input.uri.trim().is_empty() {
            diagnostics.push(config_diagnostic(
                "cem.run_config.input_uri_missing",
                format!("input spec at index {index} requires `uri`"),
                base_uri,
            ));
        } else if !input_uris.insert(input.uri.clone()) {
            diagnostics.push(config_diagnostic(
                "cem.run_config.input_uri_duplicate",
                format!("input URI `{}` is declared more than once", input.uri),
                base_uri,
            ));
        }
        validate_scope_config(
            &input.root_scope,
            "input",
            index,
            base_uri,
            &mut diagnostics,
        );
    }

    for (index, output) in config.outputs.iter().enumerate() {
        if let Some(input_ref) = output.input_ref.as_deref() {
            if !input_uris.contains(input_ref) {
                diagnostics.push(config_diagnostic(
                    "cem.run_config.output_input_ref_unknown",
                    format!("output spec at index {index} references unknown input `{input_ref}`"),
                    base_uri,
                ));
            }
        }
        validate_scope_config(
            &output.root_scope,
            "output",
            index,
            base_uri,
            &mut diagnostics,
        );
    }

    diagnostics
}

fn validate_run_config_defaults(
    defaults: &RunConfigDefaults,
    base_uri: Option<&str>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    validate_scope_config(
        &defaults.input_scope,
        "input default",
        0,
        base_uri,
        &mut diagnostics,
    );
    validate_scope_config(
        &defaults.output_scope,
        "output default",
        0,
        base_uri,
        &mut diagnostics,
    );
    diagnostics
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
        "namespaces" | "ns" => scope.namespaces = parse_map_field("namespaces", &value)?,
        "versions" | "versionpins" => scope.version_pins = parse_map_field("versionPins", &value)?,
        "budgets" => scope.budgets = parse_map_field("budgets", &value)?,
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

fn parse_map_field(
    field_name: &str,
    value: &str,
) -> Result<BTreeMap<String, String>, SpecParseError> {
    let mut map = BTreeMap::new();
    if value.trim().is_empty() {
        return Ok(map);
    }

    for pair in split_escaped(value, '|')? {
        let Some((key, value)) = pair.split_once(':') else {
            return Err(parse_error(format!(
                "{field_name} map entry `{pair}` is missing `:` separator"
            )));
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() {
            return Err(parse_error(format!(
                "{field_name} map entries require a non-empty key"
            )));
        }
        if value.is_empty() {
            return Err(parse_error(format!(
                "{field_name} map entry `{key}` requires a non-empty value"
            )));
        }
        map.insert(key.to_owned(), value.to_owned());
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

fn config_diagnostic(code: &str, message: String, base_uri: Option<&str>) -> Diagnostic {
    Diagnostic {
        uri: base_uri.map(str::to_owned),
        code: code.to_owned(),
        severity: Severity::Fatal,
        message,
        ..Diagnostic::default()
    }
}

fn validate_scope_config(
    scope: &ScopeConfig,
    direction: &str,
    index: usize,
    base_uri: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(module_map) = scope.module_map.as_deref() {
        if module_map.trim().is_empty() {
            diagnostics.push(config_diagnostic(
                "cem.run_config.scope_module_map_invalid",
                format!("{direction} scope at index {index} has an empty moduleMap"),
                base_uri,
            ));
        }
    }

    if let Some(default_namespace) = scope.default_namespace.as_deref() {
        validate_namespace_uri(
            "defaultNamespace",
            default_namespace,
            direction,
            index,
            base_uri,
            diagnostics,
        );
    }

    for (prefix, uri) in &scope.namespaces {
        validate_namespace_prefix(prefix, direction, index, base_uri, diagnostics);
        validate_namespace_uri(
            &format!("namespaces.{prefix}"),
            uri,
            direction,
            index,
            base_uri,
            diagnostics,
        );
        if prefix == "xml" && uri != "http://www.w3.org/XML/1998/namespace" {
            diagnostics.push(config_diagnostic(
                "cem.run_config.scope_namespace_invalid",
                format!(
                    "{direction} scope at index {index} binds reserved prefix `xml` to `{uri}`"
                ),
                base_uri,
            ));
        }
        if prefix == "xmlns" {
            diagnostics.push(config_diagnostic(
                "cem.run_config.scope_namespace_invalid",
                format!("{direction} scope at index {index} uses reserved prefix `xmlns`"),
                base_uri,
            ));
        }
    }

    for (name, constraint) in &scope.version_pins {
        if name.trim().is_empty() {
            diagnostics.push(config_diagnostic(
                "cem.run_config.scope_version_pin_invalid",
                format!("{direction} scope at index {index} has an empty versionPins key"),
                base_uri,
            ));
        }
        if constraint.trim().is_empty() {
            diagnostics.push(config_diagnostic(
                "cem.run_config.scope_version_pin_invalid",
                format!(
                    "{direction} scope at index {index} has an empty versionPins value for `{name}`"
                ),
                base_uri,
            ));
        }
    }
}

fn validate_namespace_prefix(
    prefix: &str,
    direction: &str,
    index: usize,
    base_uri: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let valid = !prefix.trim().is_empty()
        && !prefix.contains(':')
        && !prefix.chars().any(char::is_whitespace)
        && prefix
            .chars()
            .next()
            .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
        && prefix
            .chars()
            .all(|ch| ch == '_' || ch == '-' || ch == '.' || ch.is_ascii_alphanumeric());
    if !valid {
        diagnostics.push(config_diagnostic(
            "cem.run_config.scope_namespace_invalid",
            format!("{direction} scope at index {index} has invalid namespace prefix `{prefix}`"),
            base_uri,
        ));
    }
}

fn validate_namespace_uri(
    field: &str,
    uri: &str,
    direction: &str,
    index: usize,
    base_uri: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if uri.trim().is_empty() {
        diagnostics.push(config_diagnostic(
            "cem.run_config.scope_namespace_invalid",
            format!("{direction} scope at index {index} has an empty {field} URI"),
            base_uri,
        ));
    }
}

fn merge_scope_defaults(scope: &mut ScopeConfig, defaults: &ScopeConfig) {
    if scope.default_content_type.is_none() {
        scope.default_content_type = defaults.default_content_type.clone();
    }
    if scope.schema.is_none() {
        scope.schema = defaults.schema.clone();
    }
    if scope.default_namespace.is_none() {
        scope.default_namespace = defaults.default_namespace.clone();
    }
    if scope.module_map.is_none() {
        scope.module_map = defaults.module_map.clone();
    }
    if scope.base_uri.is_none() {
        scope.base_uri = defaults.base_uri.clone();
    }
    if scope.policy.is_none() {
        scope.policy = defaults.policy.clone();
    }

    let mut version_pins = defaults.version_pins.clone();
    version_pins.extend(scope.version_pins.clone());
    scope.version_pins = version_pins;

    let mut namespaces = defaults.namespaces.clone();
    namespaces.extend(scope.namespaces.clone());
    scope.namespaces = namespaces;

    let mut budgets = defaults.budgets.clone();
    budgets.extend(scope.budgets.clone());
    scope.budgets = budgets;
}

fn resolve_scope_module_map(scope: &mut ScopeConfig, base_uri: Option<&str>) {
    let Some(module_map) = scope.module_map.as_deref() else {
        return;
    };
    let Some(resolved) = resolve_relative_path_like(module_map, base_uri) else {
        return;
    };
    scope.module_map = Some(resolved);
}

fn resolve_output_destination(output: &mut OutputSpec, base_uri: Option<&str>) {
    let Some(destination) = output.destination.as_deref() else {
        return;
    };
    let Some(resolved) = resolve_relative_path_like(destination, base_uri) else {
        return;
    };
    output.destination = Some(resolved);
}

fn resolve_relative_path_like(value: &str, base_uri: Option<&str>) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || has_uri_scheme(trimmed) || std::path::Path::new(trimmed).is_absolute()
    {
        return None;
    }
    let base = base_uri?.trim();
    if base.is_empty() {
        return None;
    }

    let local_file_base;
    let base_path = if has_uri_scheme(base) {
        local_file_base = local_file_uri_to_path(base)?;
        local_file_base.as_path()
    } else {
        std::path::Path::new(base)
    };
    let base_dir = if base.ends_with('/') {
        base_path
    } else {
        base_path.parent()?
    };
    Some(base_dir.join(trimmed).to_string_lossy().into_owned())
}

pub fn infer_content_type_from_path(path: &str) -> Option<String> {
    let lower_path = path.to_ascii_lowercase();
    if lower_path.ends_with(".schema.json") {
        return Some(crate::schema::registry::JSON_SCHEMA_CONTENT_TYPE.to_owned());
    }

    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())?
        .to_ascii_lowercase();
    match ext.as_str() {
        "cem" => Some("application/cem+xml".to_owned()),
        "html" | "htm" => Some("text/html".to_owned()),
        "xhtml" => Some("application/xhtml+xml".to_owned()),
        "xml" => Some(crate::schema::registry::XML_CONTENT_TYPE.to_owned()),
        "svg" => Some("image/svg+xml".to_owned()),
        "xsl" | "xslt" => Some("application/xslt+xml".to_owned()),
        "cemt" => Some(crate::schema::registry::CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
        "cemql" => Some(crate::schema::registry::CEM_QL_CONTENT_TYPE.to_owned()),
        "jsonschema" => Some(crate::schema::registry::JSON_SCHEMA_CONTENT_TYPE.to_owned()),
        "json" => Some(crate::schema::registry::JSON_CONTENT_TYPE.to_owned()),
        "yaml" | "yml" => Some(crate::schema::registry::YAML_CONTENT_TYPE.to_owned()),
        "csv" => Some(crate::schema::registry::CSV_CONTENT_TYPE.to_owned()),
        "md" | "markdown" => Some(crate::schema::registry::MARKDOWN_CONTENT_TYPE.to_owned()),
        _ => None,
    }
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
    fn spec_record_rejects_empty_scope_map_entries() {
        let error = parse_input_spec_record("uri=src/a.cem,namespaces=:urn:widgets").unwrap_err();

        assert!(error
            .message
            .contains("namespaces map entries require a non-empty key"));
    }

    #[test]
    fn output_spec_record_maps_target_scope() {
        let spec = parse_output_spec_record(
            "input=src/a.cem,dest=dist/a.cem,contentType=application/cem+xml,schema=core,defaultNs=https://cem.dev/ns/core/1,namespaces=html:http://www.w3.org/1999/xhtml",
        )
        .unwrap();

        assert_eq!(spec.input_ref.as_deref(), Some("src/a.cem"));
        assert_eq!(spec.destination.as_deref(), Some("dist/a.cem"));
        assert_eq!(
            spec.root_scope.default_content_type.as_deref(),
            Some("application/cem+xml")
        );
        assert_eq!(spec.root_scope.schema.as_deref(), Some("core"));
        assert_eq!(
            spec.root_scope.default_namespace.as_deref(),
            Some("https://cem.dev/ns/core/1")
        );
        assert_eq!(
            spec.root_scope.namespaces.get("html").map(String::as_str),
            Some("http://www.w3.org/1999/xhtml")
        );
    }

    #[test]
    fn cemt_extension_infers_transform_content_type() {
        assert_eq!(
            infer_content_type_from_path("templates/page.cemt").as_deref(),
            Some(crate::schema::registry::CEM_TRANSFORM_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("templates/PAGE.CEMT").as_deref(),
            Some(crate::schema::registry::CEM_TRANSFORM_CONTENT_TYPE)
        );
    }

    #[test]
    fn cemql_extension_infers_query_content_type() {
        assert_eq!(
            infer_content_type_from_path("queries/module.cemql").as_deref(),
            Some(crate::schema::registry::CEM_QL_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("queries/MODULE.CEMQL").as_deref(),
            Some(crate::schema::registry::CEM_QL_CONTENT_TYPE)
        );
    }

    #[test]
    fn json_extension_infers_json_content_type() {
        assert_eq!(
            infer_content_type_from_path("data/item.json").as_deref(),
            Some(crate::schema::registry::JSON_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("data/ITEM.JSON").as_deref(),
            Some(crate::schema::registry::JSON_CONTENT_TYPE)
        );
    }

    #[test]
    fn yaml_extensions_infer_yaml_content_type() {
        assert_eq!(
            infer_content_type_from_path("data/item.yaml").as_deref(),
            Some(crate::schema::registry::YAML_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("data/ITEM.YML").as_deref(),
            Some(crate::schema::registry::YAML_CONTENT_TYPE)
        );
    }

    #[test]
    fn csv_extension_infers_csv_content_type() {
        assert_eq!(
            infer_content_type_from_path("data/table.csv").as_deref(),
            Some(crate::schema::registry::CSV_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("data/TABLE.CSV").as_deref(),
            Some(crate::schema::registry::CSV_CONTENT_TYPE)
        );
    }

    #[test]
    fn markdown_extensions_infer_markdown_content_type() {
        assert_eq!(
            infer_content_type_from_path("docs/readme.md").as_deref(),
            Some(crate::schema::registry::MARKDOWN_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("docs/README.MARKDOWN").as_deref(),
            Some(crate::schema::registry::MARKDOWN_CONTENT_TYPE)
        );
    }

    #[test]
    fn xml_extension_infers_xml_content_type() {
        assert_eq!(
            infer_content_type_from_path("data/document.xml").as_deref(),
            Some(crate::schema::registry::XML_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("data/DOCUMENT.XML").as_deref(),
            Some(crate::schema::registry::XML_CONTENT_TYPE)
        );
    }

    #[test]
    fn json_schema_paths_infer_json_schema_content_type() {
        assert_eq!(
            infer_content_type_from_path("schema/run-config.schema.json").as_deref(),
            Some(crate::schema::registry::JSON_SCHEMA_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("schema/REPORT.SCHEMA.JSON").as_deref(),
            Some(crate::schema::registry::JSON_SCHEMA_CONTENT_TYPE)
        );
        assert_eq!(
            infer_content_type_from_path("schema/root.jsonschema").as_deref(),
            Some(crate::schema::registry::JSON_SCHEMA_CONTENT_TYPE)
        );
    }

    #[test]
    fn run_config_schema_identity_constants_are_stable() {
        assert_eq!(RUN_CONFIG_SCHEMA_URI, "https://cem.dev/ns/cli/run-config/1");
        assert_eq!(RUN_CONFIG_NAMESPACE_URI, RUN_CONFIG_SCHEMA_URI);
        assert_eq!(
            RUN_CONFIG_JSON_SCHEMA_URI,
            "https://cem.dev/schema/cli/run-config.schema.json"
        );
    }

    #[test]
    fn run_config_json_schema_artifact_matches_constants() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../schema/cli/run-config.schema.json"))
                .expect("run config JSON Schema parses");

        assert_eq!(
            schema.get("$id").and_then(serde_json::Value::as_str),
            Some(RUN_CONFIG_JSON_SCHEMA_URI)
        );
        assert_eq!(
            schema
                .pointer("/properties/inputs/items/$ref")
                .and_then(serde_json::Value::as_str),
            Some("#/$defs/inputSpec")
        );
        assert_eq!(
            schema
                .pointer("/properties/outputs/items/$ref")
                .and_then(serde_json::Value::as_str),
            Some("#/$defs/outputSpec")
        );
        assert_eq!(
            schema
                .pointer("/$defs/scopeConfig/properties/defaultContentType/type")
                .and_then(serde_json::Value::as_str),
            Some("string")
        );
        assert_eq!(
            schema
                .pointer("/$defs/resolverSpec/required/0")
                .and_then(serde_json::Value::as_str),
            Some("uriPrefix")
        );
    }

    #[test]
    fn json_run_config_parses_by_content_type() {
        let response = parse_run_config(RunConfigParseRequest {
            bytes: br#"{"resolvers":[{"uriPrefix":"cem+vfs://workspace","localRoot":"/tmp/cem-vfs","read":true}],"inputs":[{"uri":"src/a.cem","rootScope":{"defaultContentType":"application/cem+xml"}}]}"#.to_vec(),
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
        assert_eq!(response.config.resolvers.len(), 1);
        assert_eq!(
            response.config.resolvers[0].uri_prefix,
            "cem+vfs://workspace"
        );
        assert!(response.config.resolvers[0].read);
        assert!(!response.config.resolvers[0].write);
        assert!(response.diagnostics.is_empty());
    }

    #[test]
    fn json_run_config_accepts_run_config_schema_identity() {
        let response = parse_run_config(RunConfigParseRequest {
            bytes: b"{}".to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                schema: Some(RUN_CONFIG_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .expect("run config schema identity accepted");

        assert!(response.config.inputs.is_empty());
        assert!(response.config.outputs.is_empty());
    }

    #[test]
    fn json_run_config_accepts_run_config_namespace_identity() {
        let response = parse_run_config(RunConfigParseRequest {
            bytes: b"{}".to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                default_namespace: Some(RUN_CONFIG_NAMESPACE_URI.to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .expect("run config namespace identity accepted");

        assert!(response.config.inputs.is_empty());
        assert!(response.config.outputs.is_empty());
    }

    #[test]
    fn unsupported_run_config_schema_identity_is_rejected() {
        let error = parse_run_config(RunConfigParseRequest {
            bytes: b"{}".to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                schema: Some("https://cem.dev/ns/core/1".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .expect_err("CEM core schema is not run config schema");

        assert_eq!(error.code, "cem.run_config.unsupported_schema_identity");
        assert!(error.message.contains(RUN_CONFIG_SCHEMA_URI));
    }

    #[test]
    fn unsupported_run_config_namespace_identity_is_rejected() {
        let error = parse_run_config(RunConfigParseRequest {
            bytes: b"{}".to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                default_namespace: Some("https://cem.dev/ns/core/1".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .expect_err("CEM core namespace is not run config namespace");

        assert_eq!(error.code, "cem.run_config.unsupported_schema_identity");
        assert!(error.message.contains(RUN_CONFIG_NAMESPACE_URI));
    }

    #[test]
    fn normalize_run_config_applies_defaults_and_infers_content_type() {
        let response = normalize_run_config(
            RunConfig {
                inputs: vec![
                    InputSpec {
                        uri: "src/a.cem".to_owned(),
                        ..InputSpec::default()
                    },
                    InputSpec {
                        uri: "src/b.html".to_owned(),
                        root_scope: ScopeConfig {
                            schema: Some("explicit-schema".to_owned()),
                            ..ScopeConfig::default()
                        },
                    },
                    InputSpec {
                        uri: "src/icon.svg".to_owned(),
                        ..InputSpec::default()
                    },
                    InputSpec {
                        uri: "src/page.xhtml".to_owned(),
                        ..InputSpec::default()
                    },
                ],
                outputs: vec![
                    OutputSpec {
                        destination: Some("dist/a.cem".to_owned()),
                        ..OutputSpec::default()
                    },
                    OutputSpec {
                        destination: Some("dist/icon.svg".to_owned()),
                        ..OutputSpec::default()
                    },
                    OutputSpec {
                        destination: Some("dist/page.xhtml".to_owned()),
                        ..OutputSpec::default()
                    },
                ],
                resolvers: Vec::new(),
                scheduler: SchedulerConfig::default(),
            },
            RunConfigDefaults {
                input_scope: ScopeConfig {
                    schema: Some("default-schema".to_owned()),
                    ..ScopeConfig::default()
                },
                output_scope: ScopeConfig {
                    schema: Some("target-schema".to_owned()),
                    ..ScopeConfig::default()
                },
            },
            None,
        );

        assert!(response.diagnostics.is_empty());
        assert_eq!(
            response.config.inputs[0]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("application/cem+xml")
        );
        assert_eq!(
            response.config.inputs[0].root_scope.schema.as_deref(),
            Some("default-schema")
        );
        assert_eq!(
            response.config.inputs[1]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("text/html")
        );
        assert_eq!(
            response.config.inputs[1].root_scope.schema.as_deref(),
            Some("explicit-schema")
        );
        assert_eq!(
            response.config.inputs[2]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("image/svg+xml")
        );
        assert_eq!(
            response.config.inputs[2].root_scope.schema.as_deref(),
            Some("default-schema")
        );
        assert_eq!(
            response.config.inputs[3]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("application/xhtml+xml")
        );
        assert_eq!(
            response.config.inputs[3].root_scope.schema.as_deref(),
            Some("default-schema")
        );
        assert_eq!(
            response.config.outputs[0]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("application/cem+xml")
        );
        assert_eq!(
            response.config.outputs[0].root_scope.schema.as_deref(),
            Some("target-schema")
        );
        assert_eq!(
            response.config.outputs[1]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("image/svg+xml")
        );
        assert_eq!(
            response.config.outputs[1].root_scope.schema.as_deref(),
            Some("target-schema")
        );
        assert_eq!(
            response.config.outputs[2]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("application/xhtml+xml")
        );
        assert_eq!(
            response.config.outputs[2].root_scope.schema.as_deref(),
            Some("target-schema")
        );
    }

    #[test]
    fn normalize_run_config_resolves_relative_module_map_against_config_path() {
        let response = normalize_run_config(
            RunConfig {
                inputs: vec![InputSpec {
                    uri: "src/a.cem".to_owned(),
                    root_scope: ScopeConfig {
                        module_map: Some("cem.modules.json".to_owned()),
                        ..ScopeConfig::default()
                    },
                }],
                outputs: vec![OutputSpec {
                    destination: Some("dist/a.cem".to_owned()),
                    root_scope: ScopeConfig {
                        module_map: Some("out.modules.json".to_owned()),
                        ..ScopeConfig::default()
                    },
                    ..OutputSpec::default()
                }],
                resolvers: Vec::new(),
                scheduler: SchedulerConfig::default(),
            },
            RunConfigDefaults::default(),
            Some("/workspace/configs/run.json"),
        );

        assert!(response.diagnostics.is_empty());
        assert_eq!(
            response.config.inputs[0].root_scope.module_map.as_deref(),
            Some("/workspace/configs/cem.modules.json")
        );
        assert_eq!(
            response.config.outputs[0].root_scope.module_map.as_deref(),
            Some("/workspace/configs/out.modules.json")
        );
    }

    #[test]
    fn normalize_run_config_leaves_absolute_and_uri_module_maps_unchanged() {
        let response = normalize_run_config(
            RunConfig {
                inputs: vec![
                    InputSpec {
                        uri: "src/a.cem".to_owned(),
                        root_scope: ScopeConfig {
                            module_map: Some("/workspace/cem.modules.json".to_owned()),
                            ..ScopeConfig::default()
                        },
                    },
                    InputSpec {
                        uri: "src/b.cem".to_owned(),
                        root_scope: ScopeConfig {
                            module_map: Some("https://example.test/cem.modules.json".to_owned()),
                            ..ScopeConfig::default()
                        },
                    },
                ],
                ..RunConfig::default()
            },
            RunConfigDefaults::default(),
            Some("/workspace/configs/run.json"),
        );

        assert!(response.diagnostics.is_empty());
        assert_eq!(
            response.config.inputs[0].root_scope.module_map.as_deref(),
            Some("/workspace/cem.modules.json")
        );
        assert_eq!(
            response.config.inputs[1].root_scope.module_map.as_deref(),
            Some("https://example.test/cem.modules.json")
        );
    }

    #[test]
    fn normalize_run_config_preserves_uri_shaped_inputs_outputs_and_module_maps() {
        let response = normalize_run_config(
            RunConfig {
                inputs: vec![InputSpec {
                    uri: "https://example.test/src/a.cem".to_owned(),
                    root_scope: ScopeConfig {
                        module_map: Some("cem+vfs://workspace/cem.modules.json".to_owned()),
                        ..ScopeConfig::default()
                    },
                }],
                outputs: vec![OutputSpec {
                    input_ref: Some("https://example.test/src/a.cem".to_owned()),
                    destination: Some("cem+vfs://workspace/dist/a.json".to_owned()),
                    root_scope: ScopeConfig {
                        module_map: Some("file://example.test/out.modules.json".to_owned()),
                        ..ScopeConfig::default()
                    },
                }],
                ..RunConfig::default()
            },
            RunConfigDefaults::default(),
            Some("/workspace/configs/run.json"),
        );

        assert!(response.diagnostics.is_empty());
        assert_eq!(
            response.config.inputs[0].uri,
            "https://example.test/src/a.cem"
        );
        assert_eq!(
            response.config.inputs[0].root_scope.module_map.as_deref(),
            Some("cem+vfs://workspace/cem.modules.json")
        );
        assert_eq!(
            response.config.outputs[0].destination.as_deref(),
            Some("cem+vfs://workspace/dist/a.json")
        );
        assert_eq!(
            response.config.outputs[0].root_scope.module_map.as_deref(),
            Some("file://example.test/out.modules.json")
        );
    }

    #[test]
    fn normalize_run_config_resolves_relative_output_destination_against_config_path() {
        let response = normalize_run_config(
            RunConfig {
                inputs: vec![InputSpec {
                    uri: "src/a.cem".to_owned(),
                    ..InputSpec::default()
                }],
                outputs: vec![OutputSpec {
                    destination: Some("dist/a.cem".to_owned()),
                    ..OutputSpec::default()
                }],
                ..RunConfig::default()
            },
            RunConfigDefaults::default(),
            Some("/workspace/configs/run.json"),
        );

        assert!(response.diagnostics.is_empty());
        assert_eq!(
            response.config.outputs[0].destination.as_deref(),
            Some("/workspace/configs/dist/a.cem")
        );
        assert_eq!(
            response.config.outputs[0]
                .root_scope
                .default_content_type
                .as_deref(),
            Some("application/cem+xml")
        );
    }

    #[test]
    fn normalize_run_config_resolves_relative_paths_against_local_file_uri_config_path() {
        let response = normalize_run_config(
            RunConfig {
                inputs: vec![InputSpec {
                    uri: "src/a.cem".to_owned(),
                    root_scope: ScopeConfig {
                        module_map: Some("cem.modules.json".to_owned()),
                        ..ScopeConfig::default()
                    },
                }],
                outputs: vec![OutputSpec {
                    destination: Some("dist/a.cem".to_owned()),
                    root_scope: ScopeConfig {
                        module_map: Some("out.modules.json".to_owned()),
                        ..ScopeConfig::default()
                    },
                    ..OutputSpec::default()
                }],
                ..RunConfig::default()
            },
            RunConfigDefaults::default(),
            Some("file:///workspace/configs/with%20space/run.json"),
        );

        assert!(response.diagnostics.is_empty());
        assert_eq!(
            response.config.inputs[0].root_scope.module_map.as_deref(),
            Some("/workspace/configs/with space/cem.modules.json")
        );
        assert_eq!(
            response.config.outputs[0].root_scope.module_map.as_deref(),
            Some("/workspace/configs/with space/out.modules.json")
        );
        assert_eq!(
            response.config.outputs[0].destination.as_deref(),
            Some("/workspace/configs/with space/dist/a.cem")
        );
    }

    #[test]
    fn run_config_validation_reports_unknown_output_input_ref() {
        let parsed = parse_run_config(RunConfigParseRequest {
            bytes: br#"{"inputs":[{"uri":"src/a.cem"}],"outputs":[{"inputRef":"missing.cem"}]}"#
                .to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: Some("file:///run-config.json".to_owned()),
        })
        .unwrap();
        let response = normalize_run_config(
            parsed.config,
            RunConfigDefaults::default(),
            Some("file:///run-config.json"),
        );

        assert_eq!(response.diagnostics.len(), 1);
        assert_eq!(
            response.diagnostics[0].code,
            "cem.run_config.output_input_ref_unknown"
        );
        assert_eq!(
            response.diagnostics[0].uri.as_deref(),
            Some("file:///run-config.json")
        );
    }

    #[test]
    fn run_config_validation_reports_duplicate_inputs() {
        let parsed = parse_run_config(RunConfigParseRequest {
            bytes: br#"{"inputs":[{"uri":"src/a.cem"},{"uri":"src/a.cem"}]}"#.to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .unwrap();
        let response = normalize_run_config(parsed.config, RunConfigDefaults::default(), None);

        assert!(response
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.run_config.input_uri_duplicate"));
    }

    #[test]
    fn run_config_validation_reports_invalid_scope_fields() {
        let parsed = parse_run_config(RunConfigParseRequest {
            bytes: br#"{
                "inputs": [{
                    "uri": "src/a.cem",
                    "rootScope": {
                        "defaultNamespace": "",
                        "namespaces": {
                            "1bad": "urn:widgets",
                            "xml": "urn:not-xml"
                        },
                        "versionPins": {
                            "core": ""
                        },
                        "moduleMap": ""
                    }
                }]
            }"#
            .to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: Some("file:///run-config.json".to_owned()),
        })
        .unwrap();
        let response = normalize_run_config(
            parsed.config,
            RunConfigDefaults::default(),
            Some("file:///run-config.json"),
        );

        let codes: Vec<_> = response
            .diagnostics
            .iter()
            .map(|diag| diag.code.as_str())
            .collect();
        assert!(codes.contains(&"cem.run_config.scope_module_map_invalid"));
        assert!(codes.contains(&"cem.run_config.scope_namespace_invalid"));
        assert!(codes.contains(&"cem.run_config.scope_version_pin_invalid"));
        assert!(response
            .diagnostics
            .iter()
            .all(|diag| diag.uri.as_deref() == Some("file:///run-config.json")));
    }

    #[test]
    fn normalize_run_config_validates_default_scope_fields() {
        let response = normalize_run_config(
            RunConfig::default(),
            RunConfigDefaults {
                input_scope: ScopeConfig {
                    namespaces: BTreeMap::from([("xml".to_owned(), "urn:not-xml".to_owned())]),
                    ..ScopeConfig::default()
                },
                output_scope: ScopeConfig {
                    default_namespace: Some(String::new()),
                    ..ScopeConfig::default()
                },
            },
            Some("file:///run-config.json"),
        );

        assert_eq!(response.diagnostics.len(), 2);
        assert!(response
            .diagnostics
            .iter()
            .all(|diag| diag.code == "cem.run_config.scope_namespace_invalid"));
        assert!(response
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("input default scope")));
        assert!(response
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("output default scope")));
        assert!(response
            .diagnostics
            .iter()
            .all(|diag| diag.uri.as_deref() == Some("file:///run-config.json")));
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
