//! Structural data lifecycle adapters.
//!
//! The CLI selects input and output identities; this module owns the library
//! side of input identity dispatch into the internal CEM event/AST pipeline.

use crate::diagnostics::{Diagnostic, Severity};
use crate::engine::{EngineContext, EngineInput, FormatIdentity, InputFormat, LayerFormat};

pub const ADAPTER_AMBIGUOUS_CODE: &str = "cem.lifecycle.adapter_ambiguous";
pub const TARGET_ADAPTER_AMBIGUOUS_CODE: &str = "cem.lifecycle.target_adapter_ambiguous";
pub const TARGET_ADAPTER_UNSUPPORTED_CODE: &str = "cem.lifecycle.target_adapter_unsupported";

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
        registry
    }

    pub fn register(&mut self, adapter: impl LifecycleAdapter + 'static) {
        self.adapters.push(Box::new(adapter));
    }

    pub fn load(&self, input: &EngineInput, context: &EngineContext) -> LoadedInput {
        let identity = FormatIdentity::from(context);
        let matches: Vec<&dyn LifecycleAdapter> = self
            .adapters
            .iter()
            .map(|adapter| adapter.as_ref())
            .filter(|adapter| adapter.matches_input(&identity))
            .collect();

        match matches.as_slice() {
            [adapter] => adapter.load(input, &identity),
            [] => passthrough_load(input, input.from_format.unwrap_or(InputFormat::Cem), None),
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
        if identity.content_type.is_none() && identity.schema.is_none() {
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
                        message: format!(
                            "no lifecycle export adapter matched target content type `{}`",
                            identity.content_type.as_deref().unwrap_or_default()
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

fn content_type_essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
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
                "application/cem",
                "text/cem",
                "text/cem-ml",
            ],
        )
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
        matches_content_type(identity, &["text/html", "application/xhtml+xml"])
    }

    fn load(&self, input: &EngineInput, _: &FormatIdentity) -> LoadedInput {
        passthrough_load(input, InputFormat::Html, Some(self.id()))
    }
}

struct XmlAdapter;

impl LifecycleAdapter for XmlAdapter {
    fn id(&self) -> &'static str {
        "xml"
    }

    fn matches_input(&self, identity: &FormatIdentity) -> bool {
        matches_content_type(identity, &["application/xml", "text/xml"])
    }

    fn load(&self, input: &EngineInput, _: &FormatIdentity) -> LoadedInput {
        passthrough_load(input, InputFormat::Xml, Some(self.id()))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn input(bytes: &[u8]) -> EngineInput {
        EngineInput {
            uri: "test-input".to_owned(),
            bytes: bytes.to_vec(),
            from_format: None,
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
    fn unknown_content_type_falls_back_to_input_format() {
        let mut source = input(b"<p>Hi</p>");
        source.from_format = Some(InputFormat::Html);
        let loaded = LifecycleRegistry::with_builtin_adapters()
            .load(&source, &context("application/unknown"));
        assert_eq!(loaded.from_format, InputFormat::Html);
        assert_eq!(loaded.adapter_id, None);
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
}
