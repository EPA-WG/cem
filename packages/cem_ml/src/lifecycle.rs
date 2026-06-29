//! Structural data lifecycle adapters.
//!
//! The CLI selects input and output identities; this module owns the library
//! side of input identity dispatch into the internal CEM event/AST pipeline.

use crate::diagnostics::{Diagnostic, Severity};
use crate::engine::{EngineContext, EngineInput, FormatIdentity, InputFormat, LayerFormat};
use crate::schema::ir::CEM_CORE_NAMESPACE;
use crate::schema::registry::{
    CEM_AST_JSON_PROJECTION_CONTENT_TYPE, CEM_AST_PROJECTION_CONTENT_TYPE,
    CEM_AST_PROJECTION_SCHEMA_URI, CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
    CEM_DOM_PROJECTION_CONTENT_TYPE, CEM_DOM_PROJECTION_SCHEMA_URI,
    CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE, CEM_EVENTS_PROJECTION_CONTENT_TYPE,
    CEM_EVENTS_PROJECTION_SCHEMA_URI, CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI,
    CEM_NATIVE_TEMPLATE_CONTENT_TYPE, CEM_NATIVE_TEMPLATE_SCHEMA_URI, CEM_SCHEMA_CONTENT_TYPE,
    CEM_SCHEMA_PACKAGE_CONTENT_TYPE, CEM_SCHEMA_PACKAGE_URI, CEM_SCHEMA_URI,
    CEM_TRANSFORM_CONTENT_TYPE, CEM_TRANSFORM_SCHEMA_URI, MATHML_CONTENT_TYPE,
    MATHML_NAMESPACE_URI, MATHML_SCHEMA_URI, SVG_CONTENT_TYPE, SVG_NAMESPACE_URI, SVG_SCHEMA_URI,
    XHTML_CONTENT_TYPE, XHTML_NAMESPACE_URI, XHTML_SCHEMA_URI, XML_CONTENT_TYPE, XML_SCHEMA_URI,
};
use crate::transform_config::TRANSFORM_CONFIG_SCHEMA_URI;

pub const ADAPTER_AMBIGUOUS_CODE: &str = "cem.lifecycle.adapter_ambiguous";
pub const ADAPTER_UNSUPPORTED_CODE: &str = "cem.lifecycle.adapter_unsupported";
pub const TARGET_ADAPTER_AMBIGUOUS_CODE: &str = "cem.lifecycle.target_adapter_ambiguous";
pub const TARGET_ADAPTER_UNSUPPORTED_CODE: &str = "cem.lifecycle.target_adapter_unsupported";
pub const DOM_JSON_PROJECTION_SCHEMA: &str = "https://cem.dev/ns/projection/dom-json/1";
pub const DOM_PROJECTION_SCHEMA: &str = CEM_DOM_PROJECTION_SCHEMA_URI;
pub const AST_PROJECTION_SCHEMA: &str = CEM_AST_PROJECTION_SCHEMA_URI;
pub const EVENTS_PROJECTION_SCHEMA: &str = CEM_EVENTS_PROJECTION_SCHEMA_URI;

const HTML_NAMESPACE: &str = XHTML_NAMESPACE_URI;
const SVG_NAMESPACE: &str = SVG_NAMESPACE_URI;
const MATHML_NAMESPACE: &str = MATHML_NAMESPACE_URI;
const XSLT_NAMESPACE: &str = "http://www.w3.org/1999/XSL/Transform";
const HTML_ADAPTER_SCHEMA_IDENTITIES: &[&str] = &[
    HTML_NAMESPACE,
    XHTML_SCHEMA_URI,
    SVG_NAMESPACE,
    MATHML_NAMESPACE,
];
const XML_ADAPTER_SCHEMA_IDENTITIES: &[&str] = &[XML_SCHEMA_URI, SVG_SCHEMA_URI, MATHML_SCHEMA_URI];
const CEM_ML_SCHEMA_IDENTITIES: &[&str] = &[
    CEM_ML_SCHEMA_URI,
    CEM_SCHEMA_URI,
    CEM_SCHEMA_PACKAGE_URI,
    CEM_CORE_NAMESPACE,
    TRANSFORM_CONFIG_SCHEMA_URI,
    CEM_NATIVE_TEMPLATE_SCHEMA_URI,
    CEM_TRANSFORM_SCHEMA_URI,
];

#[derive(Debug, Clone)]
pub struct LoadedInput {
    pub bytes: Vec<u8>,
    pub from_format: InputFormat,
    pub diagnostics: Vec<Diagnostic>,
    pub adapter_id: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct ExportSelection {
    pub to_format: LayerFormat,
    pub diagnostics: Vec<Diagnostic>,
    pub adapter_id: Option<&'static str>,
}

pub trait LifecycleAdapter: Send + Sync {
    fn id(&self) -> &'static str;
    fn matches_input(&self, identity: &FormatIdentity) -> bool;
    fn load(&self, input: &EngineInput, identity: &FormatIdentity) -> LoadedInput;
    fn matches_target(&self, _: &FormatIdentity) -> bool {
        false
    }
    fn target_format(&self) -> Option<LayerFormat> {
        None
    }
}

#[derive(Default)]
pub struct LifecycleRegistry {
    adapters: Vec<Box<dyn LifecycleAdapter>>,
}

impl LifecycleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtin_adapters() -> Self {
        let mut registry = Self::new();
        registry.register(CemMlAdapter);
        registry.register(HtmlAdapter);
        registry.register(XmlAdapter);
        registry.register(LegacyCustomElementXsltAdapter);
        registry.register(DomBinaryProjectionAdapter);
        registry.register(AstBinaryProjectionAdapter);
        registry.register(EventsBinaryProjectionAdapter);
        registry.register(DomJsonProjectionAdapter);
        registry.register(AstProjectionAdapter);
        registry.register(EventsProjectionAdapter);
        registry
    }

    pub fn register(&mut self, adapter: impl LifecycleAdapter + 'static) {
        self.adapters.push(Box::new(adapter));
    }

    pub fn load(&self, input: &EngineInput, context: &EngineContext) -> LoadedInput {
        let identity = input
            .identity
            .clone()
            .unwrap_or_else(|| FormatIdentity::from(context));
        let matches: Vec<&dyn LifecycleAdapter> = self
            .adapters
            .iter()
            .map(|adapter| adapter.as_ref())
            .filter(|adapter| adapter.matches_input(&identity))
            .collect();

        match matches.as_slice() {
            [adapter] => adapter.load(input, &identity),
            [] => {
                let mut loaded =
                    passthrough_load(input, input.from_format.unwrap_or(InputFormat::Cem), None);
                if let Some(diagnostic) = unsupported_input_identity_diagnostic(input, &identity) {
                    loaded.diagnostics.push(diagnostic);
                }
                loaded
            }
            adapters => {
                let ids = adapters
                    .iter()
                    .map(|adapter| adapter.id())
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut loaded =
                    passthrough_load(input, input.from_format.unwrap_or(InputFormat::Cem), None);
                loaded.diagnostics.push(Diagnostic {
                    uri: Some(input.uri.clone()),
                    code: ADAPTER_AMBIGUOUS_CODE.to_owned(),
                    severity: Severity::Fatal,
                    message: format!("content type matched multiple lifecycle adapters: {ids}"),
                    ..Diagnostic::default()
                });
                loaded
            }
        }
    }

    pub fn select_export(
        &self,
        target: Option<&FormatIdentity>,
        fallback: LayerFormat,
    ) -> ExportSelection {
        let Some(identity) = target else {
            return export_selection(fallback, None);
        };
        if identity_is_empty(identity) {
            return export_selection(fallback, None);
        }

        let matches: Vec<&dyn LifecycleAdapter> = self
            .adapters
            .iter()
            .map(|adapter| adapter.as_ref())
            .filter(|adapter| adapter.matches_target(identity))
            .collect();

        match matches.as_slice() {
            [adapter] => export_selection(
                adapter.target_format().unwrap_or(fallback),
                Some(adapter.id()),
            ),
            [] => {
                let mut selection = export_selection(fallback, None);
                if identity.content_type.is_some() {
                    selection.diagnostics.push(Diagnostic {
                        code: TARGET_ADAPTER_UNSUPPORTED_CODE.to_owned(),
                        severity: Severity::Warning,
                        message: unsupported_target_content_type_message(identity),
                        ..Diagnostic::default()
                    });
                } else if let Some(schema) = identity.schema.as_deref().map(str::trim) {
                    if !schema.is_empty() {
                        selection.diagnostics.push(Diagnostic {
                            code: TARGET_ADAPTER_UNSUPPORTED_CODE.to_owned(),
                            severity: Severity::Warning,
                            message: format!(
                                "no lifecycle export adapter matched target schema `{schema}`"
                            ),
                            ..Diagnostic::default()
                        });
                    }
                } else if let Some(namespace) = namespace_identity_summary(identity) {
                    selection.diagnostics.push(Diagnostic {
                        code: TARGET_ADAPTER_UNSUPPORTED_CODE.to_owned(),
                        severity: Severity::Warning,
                        message: format!(
                            "no lifecycle export adapter matched target namespace `{namespace}`"
                        ),
                        ..Diagnostic::default()
                    });
                }
                selection
            }
            adapters => {
                let ids = adapters
                    .iter()
                    .map(|adapter| adapter.id())
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut selection = export_selection(fallback, None);
                selection.diagnostics.push(Diagnostic {
                    code: TARGET_ADAPTER_AMBIGUOUS_CODE.to_owned(),
                    severity: Severity::Fatal,
                    message: format!(
                        "target identity matched multiple lifecycle export adapters: {ids}"
                    ),
                    ..Diagnostic::default()
                });
                selection
            }
        }
    }
}

fn export_selection(to_format: LayerFormat, adapter_id: Option<&'static str>) -> ExportSelection {
    ExportSelection {
        to_format,
        diagnostics: Vec::new(),
        adapter_id,
    }
}

fn passthrough_load(
    input: &EngineInput,
    from_format: InputFormat,
    adapter_id: Option<&'static str>,
) -> LoadedInput {
    LoadedInput {
        bytes: input.bytes.clone(),
        from_format,
        diagnostics: Vec::new(),
        adapter_id,
    }
}

fn matches_content_type(identity: &FormatIdentity, allowed: &[&str]) -> bool {
    identity
        .content_type
        .as_deref()
        .map(content_type_essence)
        .map(|essence| allowed.contains(&essence.as_str()))
        .unwrap_or(false)
}

fn has_content_type(identity: &FormatIdentity) -> bool {
    identity
        .content_type
        .as_deref()
        .map(|content_type| !content_type.trim().is_empty())
        .unwrap_or(false)
}

fn matches_schema(identity: &FormatIdentity, allowed: &[&str]) -> bool {
    identity
        .schema
        .as_deref()
        .map(str::trim)
        .map(|schema| allowed.contains(&schema))
        .unwrap_or(false)
}

fn matches_schema_without_content_type(identity: &FormatIdentity, allowed: &[&str]) -> bool {
    !has_content_type(identity) && matches_schema(identity, allowed)
}

fn matches_projection_json_view(
    identity: &FormatIdentity,
    allowed_schemas: &[&str],
    allowed_content_types: &[&str],
) -> bool {
    if matches_content_type(identity, allowed_content_types) {
        return identity
            .schema
            .as_deref()
            .map(str::trim)
            .filter(|schema| !schema.is_empty())
            .map(|schema| allowed_schemas.contains(&schema))
            .unwrap_or(true);
    }

    matches_schema(identity, allowed_schemas)
        && (!has_content_type(identity)
            || matches_content_type(identity, &["application/json", "text/json"]))
}

fn matches_projection_binary_artifact(
    identity: &FormatIdentity,
    allowed_schemas: &[&str],
    allowed_content_types: &[&str],
) -> bool {
    matches_content_type(identity, allowed_content_types)
        && identity
            .schema
            .as_deref()
            .map(str::trim)
            .filter(|schema| !schema.is_empty())
            .map(|schema| allowed_schemas.contains(&schema))
            .unwrap_or(true)
}

fn identity_is_empty(identity: &FormatIdentity) -> bool {
    identity.content_type.is_none()
        && identity.schema.is_none()
        && identity.default_namespace.is_none()
        && identity.namespaces.is_empty()
        && identity.base_uri.is_none()
}

fn matches_namespace_without_content_type_or_schema(
    identity: &FormatIdentity,
    allowed: &[&str],
) -> bool {
    !has_content_type(identity)
        && identity
            .schema
            .as_deref()
            .map(str::trim)
            .filter(|schema| !schema.is_empty())
            .is_none()
        && namespace_values(identity).any(|namespace| allowed.contains(&namespace))
}

fn namespace_values(identity: &FormatIdentity) -> impl Iterator<Item = &str> {
    identity
        .default_namespace
        .as_deref()
        .into_iter()
        .chain(identity.namespaces.values().map(String::as_str))
        .map(str::trim)
        .filter(|namespace| !namespace.is_empty())
}

fn namespace_identity_summary(identity: &FormatIdentity) -> Option<String> {
    identity
        .default_namespace
        .as_deref()
        .map(str::trim)
        .filter(|namespace| !namespace.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            identity.namespaces.iter().find_map(|(prefix, namespace)| {
                let namespace = namespace.trim();
                (!namespace.is_empty()).then(|| format!("{prefix}={namespace}"))
            })
        })
}

fn content_type_essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

fn unsupported_target_content_type_message(identity: &FormatIdentity) -> String {
    let content_type = identity.content_type.as_deref().unwrap_or_default();
    match identity.schema.as_deref().map(str::trim) {
        Some(schema) if !schema.is_empty() => format!(
            "no lifecycle export adapter matched target content type `{content_type}` with schema `{schema}`"
        ),
        _ => format!("no lifecycle export adapter matched target content type `{content_type}`"),
    }
}

fn unsupported_input_identity_diagnostic(
    input: &EngineInput,
    identity: &FormatIdentity,
) -> Option<Diagnostic> {
    if let Some(content_type) = identity.content_type.as_deref().map(str::trim) {
        if !content_type.is_empty() {
            return Some(Diagnostic {
                uri: Some(input.uri.clone()),
                code: ADAPTER_UNSUPPORTED_CODE.to_owned(),
                severity: Severity::Warning,
                message: unsupported_input_content_type_message(identity),
                ..Diagnostic::default()
            });
        }
    }

    identity
        .schema
        .as_deref()
        .map(str::trim)
        .filter(|schema| !schema.is_empty())
        .map(|schema| Diagnostic {
            uri: Some(input.uri.clone()),
            code: ADAPTER_UNSUPPORTED_CODE.to_owned(),
            severity: Severity::Warning,
            message: format!("no lifecycle input adapter matched schema `{schema}`"),
            ..Diagnostic::default()
        })
        .or_else(|| {
            namespace_identity_summary(identity).map(|namespace| Diagnostic {
                uri: Some(input.uri.clone()),
                code: ADAPTER_UNSUPPORTED_CODE.to_owned(),
                severity: Severity::Warning,
                message: format!("no lifecycle input adapter matched namespace `{namespace}`"),
                ..Diagnostic::default()
            })
        })
}

fn unsupported_input_content_type_message(identity: &FormatIdentity) -> String {
    let content_type = identity.content_type.as_deref().unwrap_or_default();
    match identity.schema.as_deref().map(str::trim) {
        Some(schema) if !schema.is_empty() => format!(
            "no lifecycle input adapter matched content type `{content_type}` with schema `{schema}`"
        ),
        _ => format!("no lifecycle input adapter matched content type `{content_type}`"),
    }
}

struct CemMlAdapter;

impl LifecycleAdapter for CemMlAdapter {
    fn id(&self) -> &'static str {
        "cem-ml"
    }

    fn matches_input(&self, identity: &FormatIdentity) -> bool {
        matches_content_type(
            identity,
            &[
                "application/cem+xml",
                CEM_ML_CONTENT_TYPE,
                CEM_SCHEMA_CONTENT_TYPE,
                CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
                CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
                CEM_TRANSFORM_CONTENT_TYPE,
                "text/cem",
                "text/cem-ml",
            ],
        ) || matches_schema_without_content_type(identity, CEM_ML_SCHEMA_IDENTITIES)
            || matches_namespace_without_content_type_or_schema(identity, &[CEM_CORE_NAMESPACE])
    }

    fn load(&self, input: &EngineInput, _: &FormatIdentity) -> LoadedInput {
        passthrough_load(input, InputFormat::Cem, Some(self.id()))
    }

    fn matches_target(&self, identity: &FormatIdentity) -> bool {
        self.matches_input(identity)
    }

    fn target_format(&self) -> Option<LayerFormat> {
        Some(LayerFormat::Cem)
    }
}

struct HtmlAdapter;

impl LifecycleAdapter for HtmlAdapter {
    fn id(&self) -> &'static str {
        "html"
    }

    fn matches_input(&self, identity: &FormatIdentity) -> bool {
        matches_content_type(identity, &["text/html", XHTML_CONTENT_TYPE])
            || matches_schema_without_content_type(identity, HTML_ADAPTER_SCHEMA_IDENTITIES)
            || matches_namespace_without_content_type_or_schema(
                identity,
                &[HTML_NAMESPACE, SVG_NAMESPACE, MATHML_NAMESPACE],
            )
    }

    fn load(&self, input: &EngineInput, _: &FormatIdentity) -> LoadedInput {
        passthrough_load(input, InputFormat::Html, Some(self.id()))
    }

    fn matches_target(&self, identity: &FormatIdentity) -> bool {
        self.matches_input(identity)
    }

    fn target_format(&self) -> Option<LayerFormat> {
        Some(LayerFormat::Html)
    }
}

struct XmlAdapter;

impl LifecycleAdapter for XmlAdapter {
    fn id(&self) -> &'static str {
        "xml"
    }

    fn matches_input(&self, identity: &FormatIdentity) -> bool {
        matches_content_type(
            identity,
            &[
                XML_CONTENT_TYPE,
                "text/xml",
                SVG_CONTENT_TYPE,
                MATHML_CONTENT_TYPE,
                "application/mathml-presentation+xml",
                "application/mathml-content+xml",
            ],
        ) || matches_schema_without_content_type(identity, XML_ADAPTER_SCHEMA_IDENTITIES)
    }

    fn load(&self, input: &EngineInput, _: &FormatIdentity) -> LoadedInput {
        passthrough_load(input, InputFormat::Xml, Some(self.id()))
    }

    fn matches_target(&self, identity: &FormatIdentity) -> bool {
        self.matches_input(identity)
    }

    fn target_format(&self) -> Option<LayerFormat> {
        Some(LayerFormat::Xml)
    }
}

struct LegacyCustomElementXsltAdapter;

impl LifecycleAdapter for LegacyCustomElementXsltAdapter {
    fn id(&self) -> &'static str {
        "legacy-custom-element-xslt"
    }

    fn matches_input(&self, identity: &FormatIdentity) -> bool {
        identity
            .content_type
            .as_deref()
            .map(crate::legacy_custom_element::is_legacy_custom_element_content_type)
            .unwrap_or(false)
            || matches_namespace_without_content_type_or_schema(identity, &[XSLT_NAMESPACE])
    }

    fn load(&self, input: &EngineInput, _: &FormatIdentity) -> LoadedInput {
        let legacy_source = String::from_utf8_lossy(&input.bytes);
        let converted =
            crate::legacy_custom_element::convert_template_source(legacy_source.as_ref());
        LoadedInput {
            bytes: converted.source.into_bytes(),
            from_format: InputFormat::Cem,
            diagnostics: converted
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.to_engine_diagnostic(Some(input.uri.clone())))
                .collect(),
            adapter_id: Some(self.id()),
        }
    }
}

struct DomJsonProjectionAdapter;

struct DomBinaryProjectionAdapter;

impl LifecycleAdapter for DomBinaryProjectionAdapter {
    fn id(&self) -> &'static str {
        "dom-binary-projection"
    }

    fn matches_input(&self, _: &FormatIdentity) -> bool {
        false
    }

    fn load(&self, input: &EngineInput, _: &FormatIdentity) -> LoadedInput {
        passthrough_load(
            input,
            input.from_format.unwrap_or(InputFormat::Cem),
            Some(self.id()),
        )
    }

    fn matches_target(&self, identity: &FormatIdentity) -> bool {
        matches_projection_binary_artifact(
            identity,
            &[DOM_PROJECTION_SCHEMA],
            &[CEM_DOM_PROJECTION_CONTENT_TYPE],
        )
    }

    fn target_format(&self) -> Option<LayerFormat> {
        Some(LayerFormat::DomBin)
    }
}

impl LifecycleAdapter for DomJsonProjectionAdapter {
    fn id(&self) -> &'static str {
        "dom-json-projection"
    }

    fn matches_input(&self, _: &FormatIdentity) -> bool {
        false
    }

    fn load(&self, input: &EngineInput, _: &FormatIdentity) -> LoadedInput {
        passthrough_load(
            input,
            input.from_format.unwrap_or(InputFormat::Cem),
            Some(self.id()),
        )
    }

    fn matches_target(&self, identity: &FormatIdentity) -> bool {
        matches_projection_json_view(
            identity,
            &[DOM_PROJECTION_SCHEMA, DOM_JSON_PROJECTION_SCHEMA],
            &[CEM_DOM_JSON_PROJECTION_CONTENT_TYPE],
        )
    }

    fn target_format(&self) -> Option<LayerFormat> {
        Some(LayerFormat::DomJson)
    }
}

struct AstProjectionAdapter;

struct AstBinaryProjectionAdapter;

impl LifecycleAdapter for AstBinaryProjectionAdapter {
    fn id(&self) -> &'static str {
        "ast-binary-projection"
    }

    fn matches_input(&self, _: &FormatIdentity) -> bool {
        false
    }

    fn load(&self, input: &EngineInput, _: &FormatIdentity) -> LoadedInput {
        passthrough_load(
            input,
            input.from_format.unwrap_or(InputFormat::Cem),
            Some(self.id()),
        )
    }

    fn matches_target(&self, identity: &FormatIdentity) -> bool {
        matches_projection_binary_artifact(
            identity,
            &[AST_PROJECTION_SCHEMA],
            &[CEM_AST_PROJECTION_CONTENT_TYPE],
        )
    }

    fn target_format(&self) -> Option<LayerFormat> {
        Some(LayerFormat::AstBin)
    }
}

impl LifecycleAdapter for AstProjectionAdapter {
    fn id(&self) -> &'static str {
        "ast-projection"
    }

    fn matches_input(&self, _: &FormatIdentity) -> bool {
        false
    }

    fn load(&self, input: &EngineInput, _: &FormatIdentity) -> LoadedInput {
        passthrough_load(
            input,
            input.from_format.unwrap_or(InputFormat::Cem),
            Some(self.id()),
        )
    }

    fn matches_target(&self, identity: &FormatIdentity) -> bool {
        matches_projection_json_view(
            identity,
            &[AST_PROJECTION_SCHEMA],
            &[CEM_AST_JSON_PROJECTION_CONTENT_TYPE],
        )
    }

    fn target_format(&self) -> Option<LayerFormat> {
        Some(LayerFormat::Ast)
    }
}

struct EventsProjectionAdapter;

struct EventsBinaryProjectionAdapter;

impl LifecycleAdapter for EventsBinaryProjectionAdapter {
    fn id(&self) -> &'static str {
        "events-binary-projection"
    }

    fn matches_input(&self, _: &FormatIdentity) -> bool {
        false
    }

    fn load(&self, input: &EngineInput, _: &FormatIdentity) -> LoadedInput {
        passthrough_load(
            input,
            input.from_format.unwrap_or(InputFormat::Cem),
            Some(self.id()),
        )
    }

    fn matches_target(&self, identity: &FormatIdentity) -> bool {
        matches_projection_binary_artifact(
            identity,
            &[EVENTS_PROJECTION_SCHEMA],
            &[CEM_EVENTS_PROJECTION_CONTENT_TYPE],
        )
    }

    fn target_format(&self) -> Option<LayerFormat> {
        Some(LayerFormat::EventsBin)
    }
}

impl LifecycleAdapter for EventsProjectionAdapter {
    fn id(&self) -> &'static str {
        "events-projection"
    }

    fn matches_input(&self, _: &FormatIdentity) -> bool {
        false
    }

    fn load(&self, input: &EngineInput, _: &FormatIdentity) -> LoadedInput {
        passthrough_load(
            input,
            input.from_format.unwrap_or(InputFormat::Cem),
            Some(self.id()),
        )
    }

    fn matches_target(&self, identity: &FormatIdentity) -> bool {
        matches_projection_json_view(
            identity,
            &[EVENTS_PROJECTION_SCHEMA],
            &[CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE],
        )
    }

    fn target_format(&self) -> Option<LayerFormat> {
        Some(LayerFormat::Events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(bytes: &[u8]) -> EngineInput {
        EngineInput {
            uri: "test-input".to_owned(),
            bytes: bytes.to_vec(),
            from_format: None,
            identity: None,
            root_scope: Default::default(),
        }
    }

    fn context(content_type: &str) -> EngineContext {
        EngineContext {
            content_type: Some(content_type.to_owned()),
            ..EngineContext::default()
        }
    }

    #[test]
    fn builtins_load_html_content_type_as_html() {
        let loaded = LifecycleRegistry::with_builtin_adapters()
            .load(&input(b"<p>Hi</p>"), &context("text/html; charset=utf-8"));
        assert_eq!(loaded.from_format, InputFormat::Html);
        assert_eq!(loaded.adapter_id, Some("html"));
    }

    #[test]
    fn builtins_load_svg_content_type_as_xml() {
        let loaded = LifecycleRegistry::with_builtin_adapters().load(
            &input(b"<svg><title>Hi</title></svg>"),
            &context("image/svg+xml"),
        );
        assert_eq!(loaded.from_format, InputFormat::Xml);
        assert_eq!(loaded.adapter_id, Some("xml"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn builtins_load_mathml_content_type_as_xml() {
        let loaded = LifecycleRegistry::with_builtin_adapters().load(
            &input(br#"<math xmlns="http://www.w3.org/1998/Math/MathML"><mi>x</mi></math>"#),
            &context(MATHML_CONTENT_TYPE),
        );
        assert_eq!(loaded.from_format, InputFormat::Xml);
        assert_eq!(loaded.adapter_id, Some("xml"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn builtins_load_legacy_custom_element_xslt_to_cem() {
        let loaded = LifecycleRegistry::with_builtin_adapters().load(
            &input(br#"<if test="$ready"><button>Go</button></if>"#),
            &context("custom-element-xslt"),
        );
        assert_eq!(loaded.from_format, InputFormat::Cem);
        assert_eq!(loaded.adapter_id, Some("legacy-custom-element-xslt"));
        assert!(String::from_utf8(loaded.bytes)
            .unwrap()
            .contains("{cem:if @test=\"ready\""));
    }

    #[test]
    fn builtins_load_standard_xslt_content_type_to_cem() {
        let loaded = LifecycleRegistry::with_builtin_adapters().load(
            &input(br#"<xsl:if test="$ready"><button>Go</button></xsl:if>"#),
            &context("application/xslt+xml"),
        );
        assert_eq!(loaded.from_format, InputFormat::Cem);
        assert_eq!(loaded.adapter_id, Some("legacy-custom-element-xslt"));
        assert!(String::from_utf8(loaded.bytes)
            .unwrap()
            .contains("{cem:if @test=\"ready\""));
    }

    #[test]
    fn unknown_content_type_falls_back_to_input_format() {
        let mut source = input(b"<p>Hi</p>");
        source.from_format = Some(InputFormat::Html);
        let loaded = LifecycleRegistry::with_builtin_adapters()
            .load(&source, &context("application/unknown"));
        assert_eq!(loaded.from_format, InputFormat::Html);
        assert_eq!(loaded.adapter_id, None);
        assert_eq!(loaded.diagnostics.len(), 1);
        assert_eq!(loaded.diagnostics[0].code, ADAPTER_UNSUPPORTED_CODE);
        assert!(loaded.diagnostics[0]
            .message
            .contains("content type `application/unknown`"));
    }

    #[test]
    fn unknown_content_type_with_schema_reports_full_input_identity() {
        let mut source = input(b"<p>Hi</p>");
        source.from_format = Some(InputFormat::Html);
        let loaded = LifecycleRegistry::with_builtin_adapters().load(
            &source,
            &EngineContext {
                content_type: Some("application/unknown".to_owned()),
                schema: Some("https://example.test/ns/widgets/1".to_owned()),
                ..EngineContext::default()
            },
        );
        assert_eq!(loaded.from_format, InputFormat::Html);
        assert_eq!(loaded.adapter_id, None);
        assert_eq!(loaded.diagnostics.len(), 1);
        assert_eq!(loaded.diagnostics[0].code, ADAPTER_UNSUPPORTED_CODE);
        assert!(loaded.diagnostics[0]
            .message
            .contains("content type `application/unknown`"));
        assert!(loaded.diagnostics[0]
            .message
            .contains("schema `https://example.test/ns/widgets/1`"));
    }

    #[test]
    fn unknown_schema_falls_back_to_input_format_with_warning() {
        let mut source = input(b"<p>Hi</p>");
        source.from_format = Some(InputFormat::Html);
        let loaded = LifecycleRegistry::with_builtin_adapters().load(
            &source,
            &EngineContext {
                schema: Some("https://example.test/ns/widgets/1".to_owned()),
                ..EngineContext::default()
            },
        );
        assert_eq!(loaded.from_format, InputFormat::Html);
        assert_eq!(loaded.adapter_id, None);
        assert_eq!(loaded.diagnostics.len(), 1);
        assert_eq!(loaded.diagnostics[0].code, ADAPTER_UNSUPPORTED_CODE);
        assert!(loaded.diagnostics[0]
            .message
            .contains("schema `https://example.test/ns/widgets/1`"));
    }

    #[test]
    fn cem_target_content_type_selects_cem_export() {
        let target = FormatIdentity {
            content_type: Some("application/cem+xml; charset=utf-8".to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Cem);
        assert_eq!(selected.adapter_id, Some("cem-ml"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn cem_core_schema_selects_cem_input_when_content_type_absent() {
        let loaded = LifecycleRegistry::with_builtin_adapters().load(
            &input(b"@doc cem-ml 1\n{p | Hi}"),
            &EngineContext {
                schema: Some(CEM_CORE_NAMESPACE.to_owned()),
                ..EngineContext::default()
            },
        );
        assert_eq!(loaded.from_format, InputFormat::Cem);
        assert_eq!(loaded.adapter_id, Some("cem-ml"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn transform_config_schema_selects_cem_input_when_content_type_absent() {
        let loaded = LifecycleRegistry::with_builtin_adapters().load(
            &input(br#"{run}{import @src="input.cem"}"#),
            &EngineContext {
                schema: Some(TRANSFORM_CONFIG_SCHEMA_URI.to_owned()),
                ..EngineContext::default()
            },
        );
        assert_eq!(loaded.from_format, InputFormat::Cem);
        assert_eq!(loaded.adapter_id, Some("cem-ml"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn native_template_schema_selects_cem_input_when_content_type_absent() {
        let loaded = LifecycleRegistry::with_builtin_adapters().load(
            &input(br#"{module}{template @name="card"}{body | Hi}"#),
            &EngineContext {
                schema: Some(CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
                ..EngineContext::default()
            },
        );
        assert_eq!(loaded.from_format, InputFormat::Cem);
        assert_eq!(loaded.adapter_id, Some("cem-ml"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn transform_schema_selects_cem_input_when_content_type_absent() {
        let loaded = LifecycleRegistry::with_builtin_adapters().load(
            &input(br#"{transform @template="view.cem" @to-content-type="text/html"}"#),
            &EngineContext {
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                ..EngineContext::default()
            },
        );
        assert_eq!(loaded.from_format, InputFormat::Cem);
        assert_eq!(loaded.adapter_id, Some("cem-ml"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn html_schema_selects_html_input_when_content_type_absent() {
        let loaded = LifecycleRegistry::with_builtin_adapters().load(
            &input(b"<p>Hi</p>"),
            &EngineContext {
                schema: Some(HTML_NAMESPACE.to_owned()),
                ..EngineContext::default()
            },
        );
        assert_eq!(loaded.from_format, InputFormat::Html);
        assert_eq!(loaded.adapter_id, Some("html"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn xhtml_schema_selects_html_input_when_content_type_absent() {
        let loaded = LifecycleRegistry::with_builtin_adapters().load(
            &input(b"<html xmlns=\"http://www.w3.org/1999/xhtml\"><body>Hi</body></html>"),
            &EngineContext {
                schema: Some(XHTML_SCHEMA_URI.to_owned()),
                ..EngineContext::default()
            },
        );
        assert_eq!(loaded.from_format, InputFormat::Html);
        assert_eq!(loaded.adapter_id, Some("html"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn svg_schema_selects_html_input_when_content_type_absent() {
        let loaded = LifecycleRegistry::with_builtin_adapters().load(
            &input(b"<svg><title>Hi</title></svg>"),
            &EngineContext {
                schema: Some(SVG_NAMESPACE.to_owned()),
                ..EngineContext::default()
            },
        );
        assert_eq!(loaded.from_format, InputFormat::Html);
        assert_eq!(loaded.adapter_id, Some("html"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn mathml_schema_selects_html_input_when_content_type_absent() {
        let loaded = LifecycleRegistry::with_builtin_adapters().load(
            &input(b"<math><mi>x</mi></math>"),
            &EngineContext {
                schema: Some(MATHML_NAMESPACE.to_owned()),
                ..EngineContext::default()
            },
        );
        assert_eq!(loaded.from_format, InputFormat::Html);
        assert_eq!(loaded.adapter_id, Some("html"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn svg_package_schema_selects_xml_input_when_content_type_absent() {
        let loaded = LifecycleRegistry::with_builtin_adapters().load(
            &input(b"<svg xmlns=\"http://www.w3.org/2000/svg\"><title>Hi</title></svg>"),
            &EngineContext {
                schema: Some(SVG_SCHEMA_URI.to_owned()),
                ..EngineContext::default()
            },
        );
        assert_eq!(loaded.from_format, InputFormat::Xml);
        assert_eq!(loaded.adapter_id, Some("xml"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn mathml_package_schema_selects_xml_input_when_content_type_absent() {
        let loaded = LifecycleRegistry::with_builtin_adapters().load(
            &input(br#"<math xmlns="http://www.w3.org/1998/Math/MathML"><mi>x</mi></math>"#),
            &EngineContext {
                schema: Some(MATHML_SCHEMA_URI.to_owned()),
                ..EngineContext::default()
            },
        );
        assert_eq!(loaded.from_format, InputFormat::Xml);
        assert_eq!(loaded.adapter_id, Some("xml"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn cem_core_namespace_selects_cem_input_when_content_type_and_schema_absent() {
        let mut source = input(b"@doc cem-ml 1\n{p | Hi}");
        source.identity = Some(FormatIdentity {
            default_namespace: Some(CEM_CORE_NAMESPACE.to_owned()),
            ..FormatIdentity::default()
        });

        let loaded =
            LifecycleRegistry::with_builtin_adapters().load(&source, &EngineContext::default());

        assert_eq!(loaded.from_format, InputFormat::Cem);
        assert_eq!(loaded.adapter_id, Some("cem-ml"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn html_namespace_selects_html_input_when_content_type_and_schema_absent() {
        let mut source = input(b"<p>Hi</p>");
        source.identity = Some(FormatIdentity {
            default_namespace: Some(HTML_NAMESPACE.to_owned()),
            ..FormatIdentity::default()
        });

        let loaded =
            LifecycleRegistry::with_builtin_adapters().load(&source, &EngineContext::default());

        assert_eq!(loaded.from_format, InputFormat::Html);
        assert_eq!(loaded.adapter_id, Some("html"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn svg_namespace_selects_html_input_when_content_type_and_schema_absent() {
        let mut source = input(b"<svg><title>Hi</title></svg>");
        source.identity = Some(FormatIdentity {
            namespaces: std::collections::BTreeMap::from([(
                "svg".to_owned(),
                SVG_NAMESPACE.to_owned(),
            )]),
            ..FormatIdentity::default()
        });

        let loaded =
            LifecycleRegistry::with_builtin_adapters().load(&source, &EngineContext::default());

        assert_eq!(loaded.from_format, InputFormat::Html);
        assert_eq!(loaded.adapter_id, Some("html"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn mathml_namespace_selects_html_input_when_content_type_and_schema_absent() {
        let mut source = input(b"<math><mi>x</mi></math>");
        source.identity = Some(FormatIdentity {
            namespaces: std::collections::BTreeMap::from([(
                "mathml".to_owned(),
                MATHML_NAMESPACE.to_owned(),
            )]),
            ..FormatIdentity::default()
        });

        let loaded =
            LifecycleRegistry::with_builtin_adapters().load(&source, &EngineContext::default());

        assert_eq!(loaded.from_format, InputFormat::Html);
        assert_eq!(loaded.adapter_id, Some("html"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn xslt_namespace_selects_legacy_xslt_input_when_content_type_and_schema_absent() {
        let mut source = input(br#"<xsl:if test="$ready"><button>Go</button></xsl:if>"#);
        source.identity = Some(FormatIdentity {
            namespaces: std::collections::BTreeMap::from([(
                "xsl".to_owned(),
                XSLT_NAMESPACE.to_owned(),
            )]),
            ..FormatIdentity::default()
        });

        let loaded =
            LifecycleRegistry::with_builtin_adapters().load(&source, &EngineContext::default());

        assert_eq!(loaded.from_format, InputFormat::Cem);
        assert_eq!(loaded.adapter_id, Some("legacy-custom-element-xslt"));
        assert!(loaded.diagnostics.is_empty());
        assert!(String::from_utf8(loaded.bytes)
            .unwrap()
            .contains("{cem:if @test=\"ready\""));
    }

    #[test]
    fn content_type_takes_precedence_over_xslt_namespace_for_input() {
        let mut source = input(b"<p>Hi</p>");
        source.identity = Some(FormatIdentity {
            content_type: Some("text/html".to_owned()),
            namespaces: std::collections::BTreeMap::from([(
                "xsl".to_owned(),
                XSLT_NAMESPACE.to_owned(),
            )]),
            ..FormatIdentity::default()
        });

        let loaded =
            LifecycleRegistry::with_builtin_adapters().load(&source, &EngineContext::default());

        assert_eq!(loaded.from_format, InputFormat::Html);
        assert_eq!(loaded.adapter_id, Some("html"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn unknown_namespace_falls_back_to_input_format_with_warning() {
        let mut source = input(b"<p>Hi</p>");
        source.from_format = Some(InputFormat::Html);
        source.identity = Some(FormatIdentity {
            namespaces: std::collections::BTreeMap::from([(
                "widget".to_owned(),
                "https://example.test/ns/widgets/1".to_owned(),
            )]),
            ..FormatIdentity::default()
        });

        let loaded =
            LifecycleRegistry::with_builtin_adapters().load(&source, &EngineContext::default());

        assert_eq!(loaded.from_format, InputFormat::Html);
        assert_eq!(loaded.adapter_id, None);
        assert_eq!(loaded.diagnostics.len(), 1);
        assert_eq!(loaded.diagnostics[0].code, ADAPTER_UNSUPPORTED_CODE);
        assert!(loaded.diagnostics[0]
            .message
            .contains("namespace `widget=https://example.test/ns/widgets/1`"));
    }

    #[test]
    fn content_type_takes_precedence_over_cem_core_schema_for_input() {
        let loaded = LifecycleRegistry::with_builtin_adapters().load(
            &input(b"<p>Hi</p>"),
            &EngineContext {
                content_type: Some("text/html".to_owned()),
                schema: Some(CEM_CORE_NAMESPACE.to_owned()),
                ..EngineContext::default()
            },
        );
        assert_eq!(loaded.from_format, InputFormat::Html);
        assert_eq!(loaded.adapter_id, Some("html"));
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn html_target_content_type_selects_html_export() {
        let target = FormatIdentity {
            content_type: Some("text/html; charset=utf-8".to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Html);
        assert_eq!(selected.adapter_id, Some("html"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn xhtml_target_content_type_selects_html_export() {
        let target = FormatIdentity {
            content_type: Some(XHTML_CONTENT_TYPE.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Html);
        assert_eq!(selected.adapter_id, Some("html"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn xml_target_content_type_selects_xml_export() {
        let target = FormatIdentity {
            content_type: Some("application/xml; charset=utf-8".to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Xml);
        assert_eq!(selected.adapter_id, Some("xml"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn svg_target_content_type_selects_xml_export() {
        let target = FormatIdentity {
            content_type: Some(SVG_CONTENT_TYPE.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Xml);
        assert_eq!(selected.adapter_id, Some("xml"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn mathml_target_content_type_selects_xml_export() {
        let target = FormatIdentity {
            content_type: Some(MATHML_CONTENT_TYPE.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Xml);
        assert_eq!(selected.adapter_id, Some("xml"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn svg_package_schema_selects_xml_export_when_content_type_absent() {
        let target = FormatIdentity {
            schema: Some(SVG_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Xml);
        assert_eq!(selected.adapter_id, Some("xml"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn mathml_package_schema_selects_xml_export_when_content_type_absent() {
        let target = FormatIdentity {
            schema: Some(MATHML_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Xml);
        assert_eq!(selected.adapter_id, Some("xml"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn text_xml_target_content_type_selects_xml_export() {
        let target = FormatIdentity {
            content_type: Some("text/xml".to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Xml);
        assert_eq!(selected.adapter_id, Some("xml"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn cem_core_schema_selects_cem_export_when_content_type_absent() {
        let target = FormatIdentity {
            schema: Some(CEM_CORE_NAMESPACE.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Cem);
        assert_eq!(selected.adapter_id, Some("cem-ml"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn transform_config_schema_selects_cem_export_when_content_type_absent() {
        let target = FormatIdentity {
            schema: Some(TRANSFORM_CONFIG_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Cem);
        assert_eq!(selected.adapter_id, Some("cem-ml"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn native_template_schema_selects_cem_export_when_content_type_absent() {
        let target = FormatIdentity {
            schema: Some(CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Cem);
        assert_eq!(selected.adapter_id, Some("cem-ml"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn transform_schema_selects_cem_export_when_content_type_absent() {
        let target = FormatIdentity {
            schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Cem);
        assert_eq!(selected.adapter_id, Some("cem-ml"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn html_schema_selects_html_export_when_content_type_absent() {
        let target = FormatIdentity {
            schema: Some(HTML_NAMESPACE.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Html);
        assert_eq!(selected.adapter_id, Some("html"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn xhtml_schema_selects_html_export_when_content_type_absent() {
        let target = FormatIdentity {
            schema: Some(XHTML_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Html);
        assert_eq!(selected.adapter_id, Some("html"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn svg_schema_selects_html_export_when_content_type_absent() {
        let target = FormatIdentity {
            schema: Some(SVG_NAMESPACE.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Html);
        assert_eq!(selected.adapter_id, Some("html"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn mathml_schema_selects_html_export_when_content_type_absent() {
        let target = FormatIdentity {
            schema: Some(MATHML_NAMESPACE.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Html);
        assert_eq!(selected.adapter_id, Some("html"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn cem_core_namespace_selects_cem_export_when_content_type_and_schema_absent() {
        let target = FormatIdentity {
            default_namespace: Some(CEM_CORE_NAMESPACE.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Cem);
        assert_eq!(selected.adapter_id, Some("cem-ml"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn html_namespace_selects_html_export_when_content_type_and_schema_absent() {
        let target = FormatIdentity {
            default_namespace: Some(HTML_NAMESPACE.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Html);
        assert_eq!(selected.adapter_id, Some("html"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn svg_namespace_selects_html_export_when_content_type_and_schema_absent() {
        let target = FormatIdentity {
            namespaces: std::collections::BTreeMap::from([(
                "svg".to_owned(),
                SVG_NAMESPACE.to_owned(),
            )]),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Html);
        assert_eq!(selected.adapter_id, Some("html"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn mathml_namespace_selects_html_export_when_content_type_and_schema_absent() {
        let target = FormatIdentity {
            namespaces: std::collections::BTreeMap::from([(
                "mathml".to_owned(),
                MATHML_NAMESPACE.to_owned(),
            )]),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Html);
        assert_eq!(selected.adapter_id, Some("html"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn dom_json_projection_schema_selects_dom_json_export() {
        let target = FormatIdentity {
            schema: Some(DOM_JSON_PROJECTION_SCHEMA.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::Cem);
        assert_eq!(selected.to_format, LayerFormat::DomJson);
        assert_eq!(selected.adapter_id, Some("dom-json-projection"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn dom_json_projection_schema_with_json_content_type_selects_dom_json_export() {
        let target = FormatIdentity {
            content_type: Some("application/json; charset=utf-8".to_owned()),
            schema: Some(DOM_JSON_PROJECTION_SCHEMA.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::Cem);
        assert_eq!(selected.to_format, LayerFormat::DomJson);
        assert_eq!(selected.adapter_id, Some("dom-json-projection"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn semantic_dom_projection_schema_with_json_view_content_type_selects_dom_json_export() {
        let target = FormatIdentity {
            content_type: Some(format!(
                "{CEM_DOM_JSON_PROJECTION_CONTENT_TYPE}; charset=utf-8"
            )),
            schema: Some(DOM_PROJECTION_SCHEMA.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::Cem);
        assert_eq!(selected.to_format, LayerFormat::DomJson);
        assert_eq!(selected.adapter_id, Some("dom-json-projection"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn dom_json_projection_content_type_selects_dom_json_export() {
        let target = FormatIdentity {
            content_type: Some(CEM_DOM_JSON_PROJECTION_CONTENT_TYPE.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::Cem);
        assert_eq!(selected.to_format, LayerFormat::DomJson);
        assert_eq!(selected.adapter_id, Some("dom-json-projection"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn dom_binary_projection_content_type_selects_dom_binary_export() {
        let target = FormatIdentity {
            content_type: Some(CEM_DOM_PROJECTION_CONTENT_TYPE.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::Cem);
        assert_eq!(selected.to_format, LayerFormat::DomBin);
        assert_eq!(selected.adapter_id, Some("dom-binary-projection"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn semantic_dom_projection_schema_with_binary_content_type_selects_dom_binary_export() {
        let target = FormatIdentity {
            content_type: Some(format!("{CEM_DOM_PROJECTION_CONTENT_TYPE}; charset=utf-8")),
            schema: Some(DOM_PROJECTION_SCHEMA.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::Cem);
        assert_eq!(selected.to_format, LayerFormat::DomBin);
        assert_eq!(selected.adapter_id, Some("dom-binary-projection"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn ast_projection_schema_selects_ast_export() {
        let target = FormatIdentity {
            schema: Some(AST_PROJECTION_SCHEMA.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::Cem);
        assert_eq!(selected.to_format, LayerFormat::Ast);
        assert_eq!(selected.adapter_id, Some("ast-projection"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn ast_json_projection_content_type_selects_ast_export() {
        let target = FormatIdentity {
            content_type: Some(CEM_AST_JSON_PROJECTION_CONTENT_TYPE.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::Cem);
        assert_eq!(selected.to_format, LayerFormat::Ast);
        assert_eq!(selected.adapter_id, Some("ast-projection"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn ast_binary_projection_content_type_selects_ast_binary_export() {
        let target = FormatIdentity {
            content_type: Some(CEM_AST_PROJECTION_CONTENT_TYPE.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::Cem);
        assert_eq!(selected.to_format, LayerFormat::AstBin);
        assert_eq!(selected.adapter_id, Some("ast-binary-projection"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn events_projection_schema_selects_events_export() {
        let target = FormatIdentity {
            schema: Some(EVENTS_PROJECTION_SCHEMA.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::Cem);
        assert_eq!(selected.to_format, LayerFormat::Events);
        assert_eq!(selected.adapter_id, Some("events-projection"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn events_binary_projection_content_type_selects_events_binary_export() {
        let target = FormatIdentity {
            content_type: Some(CEM_EVENTS_PROJECTION_CONTENT_TYPE.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::Cem);
        assert_eq!(selected.to_format, LayerFormat::EventsBin);
        assert_eq!(selected.adapter_id, Some("events-binary-projection"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn events_json_projection_content_type_selects_events_export() {
        let target = FormatIdentity {
            content_type: Some(CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::Cem);
        assert_eq!(selected.to_format, LayerFormat::Events);
        assert_eq!(selected.adapter_id, Some("events-projection"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn projection_schema_does_not_override_known_document_content_type() {
        let target = FormatIdentity {
            content_type: Some("text/html".to_owned()),
            schema: Some(DOM_JSON_PROJECTION_SCHEMA.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::Cem);
        assert_eq!(selected.to_format, LayerFormat::Html);
        assert_eq!(selected.adapter_id, Some("html"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn content_type_takes_precedence_over_cem_core_schema_for_export() {
        let target = FormatIdentity {
            content_type: Some("text/html".to_owned()),
            schema: Some(CEM_CORE_NAMESPACE.to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::DomJson);
        assert_eq!(selected.to_format, LayerFormat::Html);
        assert_eq!(selected.adapter_id, Some("html"));
        assert!(selected.diagnostics.is_empty());
    }

    #[test]
    fn unknown_target_content_type_preserves_fallback_with_warning() {
        let target = FormatIdentity {
            content_type: Some("application/unknown".to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::Ast);
        assert_eq!(selected.to_format, LayerFormat::Ast);
        assert_eq!(selected.adapter_id, None);
        assert_eq!(selected.diagnostics.len(), 1);
        assert_eq!(
            selected.diagnostics[0].code,
            TARGET_ADAPTER_UNSUPPORTED_CODE
        );
    }

    #[test]
    fn unknown_target_content_type_with_schema_reports_full_target_identity() {
        let target = FormatIdentity {
            content_type: Some("application/unknown".to_owned()),
            schema: Some("https://example.test/ns/widgets/1".to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::Events);
        assert_eq!(selected.to_format, LayerFormat::Events);
        assert_eq!(selected.adapter_id, None);
        assert_eq!(selected.diagnostics.len(), 1);
        assert_eq!(
            selected.diagnostics[0].code,
            TARGET_ADAPTER_UNSUPPORTED_CODE
        );
        assert!(selected.diagnostics[0]
            .message
            .contains("target content type `application/unknown`"));
        assert!(selected.diagnostics[0]
            .message
            .contains("schema `https://example.test/ns/widgets/1`"));
    }

    #[test]
    fn unknown_target_schema_preserves_fallback_with_warning() {
        let target = FormatIdentity {
            schema: Some("https://example.test/ns/widgets/1".to_owned()),
            ..FormatIdentity::default()
        };
        let selected = LifecycleRegistry::with_builtin_adapters()
            .select_export(Some(&target), LayerFormat::Events);
        assert_eq!(selected.to_format, LayerFormat::Events);
        assert_eq!(selected.adapter_id, None);
        assert_eq!(selected.diagnostics.len(), 1);
        assert_eq!(
            selected.diagnostics[0].code,
            TARGET_ADAPTER_UNSUPPORTED_CODE
        );
        assert!(selected.diagnostics[0]
            .message
            .contains("target schema `https://example.test/ns/widgets/1`"));
    }
}
