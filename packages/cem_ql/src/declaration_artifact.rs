//! Build-time declaration headers and bounded runtime metadata admission.
//! JSON is control metadata, never a serialized CEM data/AST substitute.
use crate::{
    render::{CompileTemplateOptions, TemplateModuleMapArtifact},
    template_artifact::{
        compile_template_artifact, CompiledTemplateArtifact, PinnedTemplateArtifactContext,
        TemplateArtifactSourceMapMode,
    },
};
use cem_ml::{
    content_cache::ContentHash,
    source::{BytesSource, SourceId},
    tokenizer::{html::HtmlTokenizer, SchemaTokenKind, SchemaTokenizer},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DECLARATION_METADATA_VERSION: &str = "cem-declaration-metadata/1";
const MAX_METADATA_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeclarationArtifactMetadata {
    pub contract: String,
    pub tag: String,
    pub declared_tag: String,
    pub version: Option<String>,
    pub capability: Option<String>,
    pub scope: Option<String>,
    pub link_base: String,
    pub source_uri: String,
    pub resource_base_url: String,
    pub declaration_source_hash: String,
    pub template: PinnedTemplateArtifactContext,
    pub module_map: Option<TemplateModuleMapArtifact>,
}

pub struct BuiltDeclarationArtifact {
    pub metadata_json: String,
    pub metadata_hash: String,
    pub template: CompiledTemplateArtifact,
}

fn fail(message: impl Into<String>) -> String {
    format!("cem.declaration_artifact.invalid: {}", message.into())
}
fn valid_hash(value: &str) -> bool {
    value.strip_prefix("cem-bin/1+blake3:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

/// Pins come from the admitted application build, not from this metadata file.
pub fn admit_declaration_metadata(
    json: &str,
    expected_hash: &str,
) -> Result<DeclarationArtifactMetadata, String> {
    if json.len() > MAX_METADATA_BYTES
        || !valid_hash(expected_hash)
        || ContentHash::from_blake3(json.as_bytes()).header_value() != expected_hash
    {
        return Err(fail("metadata hash/size mismatch"));
    }
    let metadata: DeclarationArtifactMetadata =
        serde_json::from_str(json).map_err(|error| fail(error.to_string()))?;
    if metadata.contract != DECLARATION_METADATA_VERSION
        || !valid_tag(&metadata.tag)
        || !valid_tag(&metadata.declared_tag)
        || !["document", "source"].contains(&metadata.link_base.as_str())
        || metadata.template.cem_ml_version != cem_ml::VERSION
        || metadata.template.cem_ql_version != crate::VERSION
        || !valid_hash(&metadata.declaration_source_hash)
        || !valid_hash(&metadata.template.source_hash)
        || !valid_hash(&metadata.template.content_hash)
        || metadata.template.host_bindings.len() > 256
        || metadata
            .template
            .host_bindings
            .iter()
            .any(|s| s.is_empty() || s.len() > 1024)
        || [&metadata.source_uri, &metadata.resource_base_url]
            .iter()
            .any(|uri| url::Url::parse(uri).is_err())
        || [
            metadata.version.as_ref(),
            metadata.capability.as_ref(),
            metadata.scope.as_ref(),
        ]
        .into_iter()
        .flatten()
        .any(|value| value.is_empty() || value.len() > 1024)
    {
        return Err(fail(
            "unsupported declaration metadata or compiler contract",
        ));
    }
    Ok(metadata)
}

fn valid_tag(tag: &str) -> bool {
    tag.len() <= 256
        && tag.contains('-')
        && tag.starts_with(|c: char| c.is_ascii_lowercase())
        && tag.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_' | b'.')
        })
        && ![
            "annotation-xml",
            "color-profile",
            "font-face",
            "font-face-src",
            "font-face-uri",
            "font-face-format",
            "font-face-name",
            "missing-glyph",
        ]
        .contains(&tag)
}

/// Compile one named canonical declaration. Source parsing belongs to the build.
pub fn build_declaration_artifact(
    source: &str,
    source_uri: &str,
    mode: TemplateArtifactSourceMapMode,
    alias: Option<&str>,
) -> Result<BuiltDeclarationArtifact, String> {
    if source.len() > 1024 * 1024 {
        return Err(fail("declaration exceeds build input limit"));
    }
    let fragments = cem_ml::legacy_custom_element::extract_html_template_fragments(source);
    let [fragment] = fragments.as_slice() else {
        return Err(fail("one canonical template is required"));
    };
    let mut tokenizer = HtmlTokenizer::from_document_source(BytesSource::new(
        SourceId(1),
        source[..fragment.start_byte].as_bytes().to_vec(),
    ));
    let mut root = BTreeMap::new();
    let mut opened = false;
    while let Some(token) = tokenizer.next_token() {
        match token.kind {
            SchemaTokenKind::NodeStart { name } if name == "cem-element" && !opened => {
                opened = true
            }
            SchemaTokenKind::Attribute { name, value, .. } if opened => {
                if ![
                    "xmlns",
                    "tag",
                    "version",
                    "capability",
                    "scope",
                    "link-base",
                    "xml:base",
                ]
                .contains(&name.as_str())
                    || root.insert(name, value.unwrap_or_default()).is_some()
                {
                    return Err(fail("unsupported/duplicate declaration attribute"));
                }
            }
            SchemaTokenKind::Trivia(_) | SchemaTokenKind::Comment(_) => {}
            SchemaTokenKind::Text(text) if text.trim().is_empty() => {}
            _ => return Err(fail("expected one canonical cem-element header")),
        }
    }
    if !opened || !tokenizer.take_diagnostics().is_empty() {
        return Err(fail("invalid declaration header"));
    }
    let declared_tag = root
        .get("tag")
        .ok_or_else(|| fail("named declaration required"))?
        .clone();
    if fragment.attributes.get("id") != Some(&declared_tag)
        || fragment.attributes.get("type").map(String::as_str) != Some("text/cem-ml")
        || fragment
            .attributes
            .keys()
            .any(|key| !["id", "type", "capability", "xml:base"].contains(&key.as_str()))
        || !source[fragment.end_byte..].trim().eq("</cem-element>")
    {
        return Err(fail("unsupported declaration/template shape"));
    }
    let capability = fragment
        .attributes
        .get("capability")
        .or_else(|| root.get("capability"))
        .cloned();
    // Match runtime declaration precedence when a host supplies a capability.
    let capability = root.get("capability").cloned().or(capability);
    let mut base = url::Url::parse(source_uri).map_err(|error| fail(error.to_string()))?;
    for value in [root.get("xml:base"), fragment.attributes.get("xml:base")]
        .into_iter()
        .flatten()
    {
        base = base.join(value).map_err(|error| fail(error.to_string()))?;
    }
    let options = CompileTemplateOptions {
        host_bindings: vec![
            "datadom".into(),
            "island".into(),
            "instanceID".into(),
            "suggestions".into(),
        ],
        source_uri: Some(format!("{source_uri}#{declared_tag}")),
        ..Default::default()
    };
    let template = compile_template_artifact(&fragment.body, &options, mode);
    let pin = PinnedTemplateArtifactContext {
        content_hash: template.content_hash.header_value(),
        source_hash: template.identity.source_hash.header_value(),
        cem_ml_version: cem_ml::VERSION.into(),
        cem_ql_version: crate::VERSION.into(),
        host_bindings: options.host_bindings,
        source_map_mode: mode,
    };
    let loaded = template
        .reload_pinned(&pin)
        .map_err(|error| fail(error.to_string()))?;
    if loaded.requires_legacy_runtime_resources() {
        return Err(fail(
            "legacy resource routing requires a source declaration",
        ));
    }
    let parsed = cem_ml::transform_template::parse_cem_native_template_module_options(
        cem_ml::transform_template::TransformTemplateModuleParseRequest {
            template: cem_ml::engine::TemplateInput {
                uri: source_uri.into(),
                bytes: fragment.body.as_bytes().to_vec(),
                identity: None,
                root_scope: cem_ml::run_config::ScopeConfig::default(),
            },
        },
    );
    if !parsed.module_options.imports.is_empty() {
        return Err(fail("external template modules require a compiled closure"));
    }
    let scope = root.get("scope").cloned();
    if loaded
        .stylesheets
        .iter()
        .any(|sheet| sheet.scope.is_some() && sheet.scope != scope)
    {
        return Err(fail("stylesheet/declaration scope mismatch"));
    }
    let metadata = DeclarationArtifactMetadata {
        contract: DECLARATION_METADATA_VERSION.into(),
        tag: alias.unwrap_or(&declared_tag).into(),
        declared_tag,
        version: root.get("version").cloned(),
        capability,
        scope,
        link_base: root
            .get("link-base")
            .cloned()
            .unwrap_or_else(|| "document".into()),
        source_uri: source_uri.into(),
        resource_base_url: base.to_string(),
        declaration_source_hash: ContentHash::from_blake3(source.as_bytes()).header_value(),
        template: pin,
        module_map: loaded.module_map,
    };
    let metadata_json =
        serde_json::to_string(&metadata).map_err(|error| fail(error.to_string()))?;
    let metadata_hash = ContentHash::from_blake3(metadata_json.as_bytes()).header_value();
    admit_declaration_metadata(&metadata_json, &metadata_hash)?;
    Ok(BuiltDeclarationArtifact {
        metadata_json,
        metadata_hash,
        template,
    })
}
