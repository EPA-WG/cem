//! Pure deployment-map contracts shared by source tooling and graph planning.
//! No resolver, filesystem access, pair matching or JavaScript scan occurs here.
use crate::diagnostics::{Diagnostic, Severity};
use crate::resolver::uri_scheme;
use crate::schema::registry::{
    content_type_essence, MODULE_MAP_CONTENT_TYPE, MODULE_MAP_SCHEMA_URI, MODULE_MAP_V2_SCHEMA_URI,
    MODULE_MAP_V3_SCHEMA_URI,
};
use crate::validation::json::{JsonDocumentAst, JsonSourceRange, JsonValueAst};
use std::{collections::BTreeMap, ffi::OsStr, path::Path};

#[derive(Debug)]
pub(crate) struct ModuleMapError {
    pub(crate) uri: String,
    pub(crate) code: String,
    pub(crate) message: String,
    path: Vec<String>,
    key: bool,
}
impl ModuleMapError {
    fn at(mut self, path: &[&str], key: bool) -> Self {
        self.path = path.iter().map(|s| (*s).into()).collect();
        self.key = key;
        self
    }
    fn prepend(mut self, prefix: &[&str]) -> Self {
        let mut path = prefix.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        path.append(&mut self.path);
        self.path = path;
        self
    }
}
fn config_error(uri: &str, code: &str, message: impl Into<String>) -> ModuleMapError {
    ModuleMapError {
        uri: uri.into(),
        code: code.into(),
        message: message.into(),
        path: Vec::new(),
        key: false,
    }
}

fn validate_document(
    doc: &ModuleMapDocument,
    uri: &str,
    value: &serde_json::Value,
) -> Result<(), ModuleMapError> {
    let schema = doc.schema.as_deref().expect("selected deployment schema");
    if let Some(object) = value.as_object() {
        if let Some(key) = object
            .keys()
            .find(|key| !matches!(key.as_str(), "$schema" | "imports" | "resources"))
        {
            return Err(config_error(
                uri,
                "cem.module_map.schema_invalid",
                format!("module map field `{key}` is not declared by `{schema}`"),
            )
            .at(&[key], true));
        }
    }
    validate_assets(doc, uri)
}

/// Called by planning only after paired schema/key identities are confirmed.
pub(crate) fn validate_assets(doc: &ModuleMapDocument, uri: &str) -> Result<(), ModuleMapError> {
    let v3 = doc.schema.as_deref() == Some(MODULE_MAP_V3_SCHEMA_URI);
    for (name, entry) in &doc.imports {
        validate_module_specifier(uri, name).map_err(|e| e.at(&["imports", name], true))?;
        validate_module_import_content_type(uri, name, &entry.path, &entry.content_type, v3)
            .map_err(|e| {
                let field = if e.code.ends_with("mismatch")
                    || e.code == "cem.module_map.asset_type_unsupported"
                {
                    "path"
                } else {
                    "contentType"
                };
                e.at(&["imports", name, field], false)
            })?;
    }
    for (name, entry) in &doc.resources {
        validate_module_specifier(uri, name).map_err(|e| e.at(&["resources", name], true))?;
        if doc.imports.contains_key(name) {
            return Err(config_error(uri,"cem.module_map.asset_identity_duplicate",format!("module-map asset identity `{name}` cannot be declared in both `imports` and `resources`")).at(&["resources",name],true));
        }
        validate_module_resource_content_type(uri, name, &entry.path, &entry.content_type, v3)
            .map_err(|e| {
                let field = if e.code.ends_with("mismatch") {
                    "path"
                } else {
                    "contentType"
                };
                e.at(&["resources", name, field], false)
            })?;
    }
    for (collection, name, content_type, edges) in doc
        .imports
        .iter()
        .map(|(name, e)| ("imports", name, &e.content_type, &e.module_imports))
        .chain(
            doc.resources
                .iter()
                .map(|(name, e)| ("resources", name, &e.content_type, &e.module_imports)),
        )
    {
        if !edges.is_empty() && content_type_essence(content_type) != "text/javascript" {
            return Err(config_error(
                uri,
                "cem.module_map.module_import_target_invalid",
                format!(
                    "module-map asset `{name}` may declare `moduleImports` only for JavaScript"
                ),
            )
            .at(&[collection, name, "moduleImports"], false));
        }
        for (specifier, target) in edges {
            let target_type = doc
                .imports
                .get(target)
                .map(|e| &e.content_type)
                .or_else(|| doc.resources.get(target).map(|e| &e.content_type));
            let Some(target_type) = target_type else {
                return Err(config_error(uri,"cem.module_map.module_import_target_missing",format!("module-map asset `{name}` maps `{specifier}` to undeclared asset `{target}`")).at(&[collection,name,"moduleImports",specifier],false));
            };
            if !matches!(
                content_type_essence(target_type).as_str(),
                "text/javascript" | "application/json"
            ) {
                return Err(config_error(uri,"cem.module_map.module_import_target_invalid",format!("module-map asset `{name}` maps `{specifier}` to non-module asset `{target}`")).at(&[collection,name,"moduleImports",specifier],false));
            }
        }
    }
    Ok(())
}

/// Selected source identity is authoritative; a JSON `$schema` value cannot
/// upgrade a MIME-only v1 request or reclassify ordinary JSON data.
pub(crate) fn selected_schema(identity: &crate::engine::FormatIdentity) -> Option<&str> {
    match identity.schema.as_deref().map(str::trim) {
        Some(schema) if is_module_map_schema(schema) => Some(schema),
        Some(_) => None,
        None if identity
            .content_type
            .as_deref()
            .is_some_and(|s| content_type_essence(s) == MODULE_MAP_CONTENT_TYPE) =>
        {
            Some(MODULE_MAP_SCHEMA_URI)
        }
        None => None,
    }
}

pub(crate) fn validate_source(
    bytes: &[u8],
    document: &JsonDocumentAst,
    schema: &str,
) -> Vec<Diagnostic> {
    let Err(error) = parse_document(
        bytes,
        &document.source.uri,
        &document.source.uri,
        Some(schema),
        true,
    ) else {
        return Vec::new();
    };
    let mut path = error.path;
    if path.is_empty() && error.code.starts_with("cem.module_map.schema_") {
        path.push("$schema".into());
    }
    let range = document
        .root
        .as_ref()
        .map(|root| source_range(root, &path, error.key));
    let code = match error.code.as_str() {
        "cem.transform_config.importmap_imports_missing"
        | "cem.transform_config.importmap_json_invalid" => "cem.module_map.schema_invalid".into(),
        "cem.transform_config.importmap_entry_invalid" => {
            "cem.module_map.import_entry_invalid".into()
        }
        _ => error.code,
    };
    vec![Diagnostic {
        uri: Some(error.uri.clone()),
        code,
        severity: Severity::Error,
        message: error.message,
        line: range.map(|r| r.start.line),
        column: range.map(|r| r.start.column),
        byte_offset: range.map(|r| r.start.byte_offset),
        source_map: range.map(|r| r.source_map().with_source_uri(&error.uri)),
        details: Some(serde_json::json!({"fieldPath":path})),
        ..Default::default()
    }]
}
fn source_range(root: &JsonValueAst, path: &[String], key: bool) -> JsonSourceRange {
    let mut value = root;
    for (index, part) in path.iter().enumerate() {
        let JsonValueAst::Object { members, .. } = value else {
            break;
        };
        let Some(member) = members.iter().find(|m| &m.name == part) else {
            break;
        };
        if index + 1 == path.len() && key {
            return member.name_range;
        }
        value = &member.value;
    }
    value.range()
}

#[derive(Debug)]
pub(crate) struct ModuleMapDocument {
    pub(crate) uri: String,
    pub(crate) schema: Option<String>,
    pub(crate) imports: BTreeMap<String, ModuleMapImport>,
    pub(crate) resources: BTreeMap<String, ModuleMapResource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModuleMapImport {
    pub(crate) path: String,
    pub(crate) content_type: String,
    pub(crate) module_imports: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModuleMapResource {
    pub(crate) path: String,
    pub(crate) content_type: String,
    pub(crate) module_imports: BTreeMap<String, String>,
}

pub(crate) fn parse_document(
    bytes: &[u8],
    config_uri: &str,
    map_uri: &str,
    expected_schema: Option<&str>,
    required: bool,
) -> Result<ModuleMapDocument, ModuleMapError> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|error| {
        config_error(
            config_uri,
            "cem.transform_config.importmap_json_invalid",
            format!("importmap `{}` is not valid JSON: {error}", map_uri),
        )
    })?;
    let schema = match value.get("$schema") {
        Some(serde_json::Value::String(schema)) if is_module_map_schema(schema) => {
            Some(schema.clone())
        }
        Some(serde_json::Value::String(schema)) => {
            return Err(config_error(
                    config_uri,
                    "cem.module_map.schema_unsupported",
                    format!(
                        "module map `{}` uses unsupported `$schema` `{schema}`; expected `{MODULE_MAP_SCHEMA_URI}`, `{MODULE_MAP_V2_SCHEMA_URI}`, or `{MODULE_MAP_V3_SCHEMA_URI}`",
                        map_uri
                    ),
                ));
        }
        Some(_) => {
            return Err(config_error(
                config_uri,
                "cem.module_map.schema_invalid",
                format!("module map `{}` `$schema` must be a string", map_uri),
            ));
        }
        None => None,
    };
    if let Some(expected) = expected_schema {
        match schema.as_deref() {
            Some(actual) if actual == expected => {}
            _ => {
                return Err(config_error(
                    config_uri,
                    "cem.module_map.schema_invalid",
                    format!("module map `{map_uri}` must declare `$schema` `{expected}`"),
                )
                .at(&["$schema"], false))
            }
        }
    }
    let required = required || schema.is_some() || expected_schema.is_some();
    let Some(imports) = value.get("imports").and_then(serde_json::Value::as_object) else {
        if required {
            return Err(config_error(
                config_uri,
                "cem.transform_config.importmap_imports_missing",
                format!("importmap `{}` requires an object `imports` map", map_uri),
            ));
        }
        return Ok(ModuleMapDocument {
            uri: map_uri.to_owned(),
            schema,
            imports: BTreeMap::new(),
            resources: BTreeMap::new(),
        });
    };
    let imports = imports
        .iter()
        .map(|(key, value)| {
            if schema.as_deref() == Some(MODULE_MAP_V3_SCHEMA_URI) {
                parse_module_map_v3_asset_entry(config_uri, &map_uri, "import", key, value)
                    .map_err(|e| e.prepend(&["imports", key]))
                    .map(|entry| {
                        (
                            key.clone(),
                            ModuleMapImport {
                                path: entry.path,
                                content_type: entry.content_type,
                                module_imports: entry.module_imports,
                            },
                        )
                    })
            } else {
                value
                    .as_str()
                    .map(|value| {
                        (
                            key.clone(),
                            ModuleMapImport {
                                path: value.to_owned(),
                                content_type: "text/javascript".to_owned(),
                                module_imports: BTreeMap::new(),
                            },
                        )
                    })
                    .ok_or_else(|| {
                        config_error(
                            config_uri,
                            "cem.transform_config.importmap_entry_invalid",
                            format!("importmap `{}` entry `{key}` must be a string", map_uri),
                        )
                    })
            }
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let resources = match value.get("resources") {
            None => BTreeMap::new(),
            Some(_)
                if !matches!(
                    schema.as_deref(),
                    Some(MODULE_MAP_V2_SCHEMA_URI | MODULE_MAP_V3_SCHEMA_URI)
                ) =>
            {
                return Err(config_error(
                    config_uri,
                    "cem.module_map.resources_unsupported",
                    format!(
                        "module map `{}` may declare `resources` only with schema `{MODULE_MAP_V2_SCHEMA_URI}` or `{MODULE_MAP_V3_SCHEMA_URI}`",
                        map_uri
                    ),
                ));
            }
            Some(serde_json::Value::Object(resources)) => resources
                .iter()
                .map(|(name, value)| {
                    if schema.as_deref() == Some(MODULE_MAP_V3_SCHEMA_URI) {
                        return parse_module_map_v3_asset_entry(
                            config_uri,
                            &map_uri,
                            "resource",
                            name,
                            value,
                        )
                        .map_err(|e| e.prepend(&["resources", name]))
                        .map(|entry| {
                            (
                                name.clone(),
                                ModuleMapResource {
                                    path: entry.path,
                                    content_type: entry.content_type,
                                    module_imports: entry.module_imports,
                                },
                            )
                        });
                    }
                    let entry = value.as_object().ok_or_else(|| {
                        config_error(
                            config_uri,
                            "cem.module_map.resource_entry_invalid",
                            format!(
                                "module map `{}` resource `{name}` must be an object",
                                map_uri
                            ),
                        )
                    })?;
                    let path = entry
                        .get("path")
                        .and_then(serde_json::Value::as_str)
                        .filter(|value| !value.trim().is_empty())
                        .ok_or_else(|| {
                            config_error(
                                config_uri,
                                "cem.module_map.resource_entry_invalid",
                                format!(
                                    "module map `{}` resource `{name}` requires a non-empty string `path`",
                                    map_uri
                                ),
                            )
                        })?;
                    let content_type = entry
                        .get("contentType")
                        .and_then(serde_json::Value::as_str)
                        .filter(|value| !value.trim().is_empty())
                        .ok_or_else(|| {
                            config_error(
                                config_uri,
                                "cem.module_map.resource_entry_invalid",
                                format!(
                                    "module map `{}` resource `{name}` requires a non-empty string `contentType`",
                                    map_uri
                                ),
                            )
                        })?;
                    if entry
                        .keys()
                        .any(|key| !matches!(key.as_str(), "path" | "contentType"))
                    {
                        return Err(config_error(
                            config_uri,
                            "cem.module_map.resource_entry_invalid",
                            format!(
                                "module map `{}` resource `{name}` permits only `path` and `contentType`",
                                map_uri
                            ),
                        ));
                    }
                    Ok((
                        name.clone(),
                        ModuleMapResource {
                            path: path.to_owned(),
                            content_type: content_type.to_owned(),
                            module_imports: BTreeMap::new(),
                        },
                    ))
                })
                .collect::<Result<BTreeMap<_, _>, _>>()?,
            Some(_) => {
                return Err(config_error(
                    config_uri,
                    "cem.module_map.resources_invalid",
                    format!(
                        "module map `{}` `resources` must be an object",
                        map_uri
                    ),
                ));
            }
        };
    let document = ModuleMapDocument {
        uri: map_uri.to_owned(),
        schema,
        imports,
        resources,
    };
    if expected_schema.is_some() {
        validate_document(&document, config_uri, &value)?;
    }
    Ok(document)
}

struct ParsedModuleMapV3AssetEntry {
    path: String,
    content_type: String,
    module_imports: BTreeMap<String, String>,
}

fn parse_module_map_v3_asset_entry(
    config_uri: &str,
    map_uri: &str,
    kind: &str,
    identity: &str,
    value: &serde_json::Value,
) -> Result<ParsedModuleMapV3AssetEntry, ModuleMapError> {
    let diagnostic = if kind == "import" {
        "cem.module_map.import_entry_invalid"
    } else {
        "cem.module_map.resource_entry_invalid"
    };
    let entry = value.as_object().ok_or_else(|| {
        config_error(
            config_uri,
            diagnostic,
            format!("module map `{map_uri}` {kind} `{identity}` must be an object"),
        )
    })?;
    if entry
        .keys()
        .any(|key| !matches!(key.as_str(), "path" | "contentType" | "moduleImports"))
    {
        return Err(config_error(
            config_uri,
            diagnostic,
            format!(
                "module map `{map_uri}` {kind} `{identity}` permits only `path`, `contentType`, and `moduleImports`"
            ),
        ));
    }
    let path = entry
        .get("path")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            config_error(
                config_uri,
                diagnostic,
                format!(
                    "module map `{map_uri}` {kind} `{identity}` requires a non-empty string `path`"
                ),
            )
        })?;
    let content_type = entry
        .get("contentType")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            config_error(
                config_uri,
                diagnostic,
                format!(
                    "module map `{map_uri}` {kind} `{identity}` requires a non-empty string `contentType`"
                ),
            )
        })?;
    let module_imports = match entry.get("moduleImports") {
        None => BTreeMap::new(),
        Some(serde_json::Value::Object(edges)) => edges
            .iter()
            .map(|(specifier, asset)| {
                validate_module_specifier(config_uri, specifier)?;
                let asset = asset
                    .as_str()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| {
                        config_error(
                            config_uri,
                            "cem.module_map.module_imports_invalid",
                            format!(
                                "module map `{map_uri}` {kind} `{identity}` module import `{specifier}` must name a declared asset"
                            ),
                        )
                    })?;
                validate_module_specifier(config_uri, asset)?;
                Ok::<_, ModuleMapError>((specifier.clone(), asset.to_owned()))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?,
        Some(_) => {
            return Err(config_error(
                config_uri,
                "cem.module_map.module_imports_invalid",
                format!(
                    "module map `{map_uri}` {kind} `{identity}` `moduleImports` must be an object"
                ),
            ));
        }
    };
    Ok(ParsedModuleMapV3AssetEntry {
        path: path.to_owned(),
        content_type: content_type.to_owned(),
        module_imports,
    })
}

pub(crate) fn is_module_map_schema(schema: &str) -> bool {
    matches!(
        schema,
        MODULE_MAP_SCHEMA_URI | MODULE_MAP_V2_SCHEMA_URI | MODULE_MAP_V3_SCHEMA_URI
    )
}

pub(crate) fn validate_module_import_content_type(
    config_uri: &str,
    specifier: &str,
    path: &str,
    content_type: &str,
    v3: bool,
) -> Result<String, ModuleMapError> {
    let content_type = content_type_essence(content_type);
    let expected_extensions: &[&str] = match content_type.as_str() {
        "text/javascript" => &["js", "mjs"],
        "application/json" if v3 => &["json"],
        _ => {
            return Err(config_error(
                config_uri,
                if v3 {
                    "cem.module_map.import_type_unsupported"
                } else {
                    "cem.module_map.asset_type_unsupported"
                },
                format!(
                    "module-map import `{specifier}` content type `{content_type}` is unsupported"
                ),
            ));
        }
    };
    let extension = Path::new(path)
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase);
    if !extension
        .as_deref()
        .is_some_and(|extension| expected_extensions.contains(&extension))
    {
        return Err(config_error(
            config_uri,
            if v3 {
                "cem.module_map.import_type_mismatch"
            } else {
                "cem.module_map.asset_type_unsupported"
            },
            format!(
                "module-map import `{specifier}` path `{path}` does not match content type `{content_type}`"
            ),
        ));
    }
    Ok(content_type)
}

pub(crate) fn validate_module_resource_content_type(
    config_uri: &str,
    name: &str,
    path: &str,
    content_type: &str,
    v3: bool,
) -> Result<String, ModuleMapError> {
    let content_type = content_type_essence(content_type);
    let expected_extensions: &[&str] = match content_type.as_str() {
        "text/javascript" => &["js", "mjs"],
        "text/css" => &["css"],
        "application/wasm" => &["wasm"],
        "application/xhtml+xml" if v3 => &["xhtml"],
        _ => {
            return Err(config_error(
                config_uri,
                "cem.module_map.resource_type_unsupported",
                format!(
                    "module-map resource `{name}` content type `{content_type}` is unsupported; expected `text/javascript`, `text/css`, or `application/wasm`{}",
                    if v3 { ", or `application/xhtml+xml`" } else { "" }
                ),
            ));
        }
    };
    let extension = Path::new(path)
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase);
    if !extension
        .as_deref()
        .is_some_and(|extension| expected_extensions.contains(&extension))
    {
        return Err(config_error(
            config_uri,
            "cem.module_map.resource_type_mismatch",
            format!(
                "module-map resource `{name}` path `{path}` does not match content type `{content_type}`"
            ),
        ));
    }
    Ok(content_type)
}

pub(crate) fn validate_module_specifier(
    config_uri: &str,
    specifier: &str,
) -> Result<(), ModuleMapError> {
    if specifier.trim().is_empty()
        || specifier != specifier.trim()
        || specifier
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '\\' | '?' | '#'))
        || specifier
            .split('/')
            .any(|part| matches!(part, "" | "." | ".."))
        || (specifier.starts_with('@') && !specifier.contains('/'))
        || specifier.starts_with('.')
        || specifier.starts_with('/')
        || specifier.starts_with('#')
        || specifier.ends_with('/')
        || uri_scheme(specifier).is_some()
    {
        return Err(config_error(
            config_uri,
            "cem.module_map.specifier_invalid",
            format!(
                "module-map import key `{specifier}` must be an exact bare npm module specifier"
            ),
        ));
    }
    Ok(())
}
