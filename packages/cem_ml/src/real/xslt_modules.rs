//! XSLT authoring dependency preflight through the existing host resolver.
//! CEMT module aliases/visibility/include semantics do not apply to XSLT.
use super::*;
use crate::schema::registry::{XSLT_CONTENT_TYPE, XSLT_NAMESPACE_URI};
use crate::validation::{
    xml::XmlEventKind,
    xslt::{xslt_stylesheet_ast_from_source_bytes, XsltSourceValidationRequest},
};

#[allow(clippy::too_many_arguments)]
pub(super) fn preflight(
    context: &EngineContext,
    adapter: &str,
    template: &TemplateInput,
    entrypoint: &TransformTemplateEntrypoint,
    options: &TransformTemplateModuleOptions,
    policy: TransformExecutionPolicy,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<TransformTemplateModulePreflight> {
    // CEMT overlays cannot silently modify an XSLT declaration graph.
    let mut declarations = options.clone();
    declarations.limits = Default::default();
    if !declarations.is_empty() {
        diagnostics.push(template_module_diagnostic(
            Some(&template.uri),
            "cem.xslt.compile_host",
            "XSLT modules and parameters must be declared in stylesheet source",
        ));
        return None;
    }
    let mut state = Preflight {
        context,
        modules: Vec::new(),
        active: BTreeSet::new(),
        completed: BTreeSet::new(),
        dependency_hash_input: Vec::new(),
        bytes: 0,
        max_depth: options.limits.max_import_depth.min(32) as usize,
    };
    if !state.visit(template, diagnostics) {
        return None;
    }
    Some(TransformTemplateModulePreflight {
        resolved_imports: state.modules,
        cache_key: Some(TransformTemplateModuleCacheKey::new(
            adapter,
            template.uri.clone(),
            template.identity.clone(),
            content_hash(&template.bytes),
            entrypoint.clone(),
            policy,
            content_hash(&state.dependency_hash_input),
        )),
    })
}
struct Preflight<'a> {
    context: &'a EngineContext,
    modules: Vec<TransformTemplateResolvedModule>,
    active: BTreeSet<String>,
    completed: BTreeSet<String>,
    dependency_hash_input: Vec<u8>,
    bytes: usize,
    max_depth: usize,
}
impl Preflight<'_> {
    fn visit(&mut self, template: &TemplateInput, diagnostics: &mut Vec<Diagnostic>) -> bool {
        if self.active.contains(&template.uri) || self.active.len() > self.max_depth {
            diagnostics.push(template_module_diagnostic(
                Some(&template.uri),
                "cem.xslt.compile_import",
                "cyclic or excessive XSLT dependency nesting",
            ));
            return false;
        }
        if self.completed.contains(&template.uri) {
            return true;
        }
        self.bytes = self.bytes.saturating_add(template.bytes.len());
        if self.bytes > 128 * 1024 || self.completed.len() + self.active.len() >= 64 {
            diagnostics.push(template_module_diagnostic(
                Some(&template.uri),
                "cem.xslt.compile_import",
                "XSLT stylesheet closure exceeds limits",
            ));
            return false;
        }
        let (stylesheet, parsed) =
            xslt_stylesheet_ast_from_source_bytes(XsltSourceValidationRequest {
                bytes: &template.bytes,
                source_uri: &template.uri,
                content_type: Some(XSLT_CONTENT_TYPE),
            });
        // Discovery performs no execution. URI diagnostics are revalidated by the
        // compiler against the exact resolved closure; all other errors block I/O.
        let errors: Vec<_> = parsed
            .into_iter()
            .filter(|d| {
                d.severity.is_hard_violation() && d.code != "cem.xslt.external_uri_rejected"
            })
            .collect();
        if !errors.is_empty() {
            diagnostics.extend(errors);
            return false;
        }
        let Some(stylesheet) = stylesheet else {
            diagnostics.push(template_module_diagnostic(
                Some(&template.uri),
                "cem.xslt.compile_import",
                "invalid stylesheet source",
            ));
            return false;
        };
        self.active.insert(template.uri.clone());
        let mut seen = BTreeSet::new();
        for event in stylesheet.xml_document.events.iter().filter(|event| {
            event.depth == 1
                && matches!(
                    event.kind,
                    XmlEventKind::StartElement | XmlEventKind::EmptyElement
                )
                && event.namespace_uri.as_deref() == Some(XSLT_NAMESPACE_URI)
                && matches!(event.local_name.as_deref(), Some("import" | "include"))
        }) {
            let Some(href) = event
                .attributes
                .iter()
                .find(|a| a.namespace_uri.is_none() && a.local_name == "href")
                .and_then(|a| a.entity_decoded_value.as_deref())
                .filter(|s| !s.trim().is_empty())
            else {
                diagnostics.push(template_module_diagnostic(
                    Some(&template.uri),
                    "cem.xslt.compile_import",
                    "stylesheet dependency requires href",
                ));
                return false;
            };
            if !seen.insert(href) {
                continue;
            }
            if self.modules.len() >= 128 {
                diagnostics.push(template_module_diagnostic(
                    Some(&template.uri),
                    "cem.xslt.compile_import",
                    "XSLT dependency edge limit exceeded",
                ));
                return false;
            }
            let import = TransformTemplateModuleImport {
                alias: href.into(),
                uri: href.into(),
                identity: Some(FormatIdentity {
                    content_type: Some(XSLT_CONTENT_TYPE.into()),
                    ..Default::default()
                }),
                kind: TransformTemplateModuleDependencyKind::Import,
                source_range: Some(ByteRange::new(
                    event.source_range.start.byte_offset,
                    event.source_range.byte_length as u32,
                )),
            };
            let module = match read_template_module_import(
                self.context,
                template,
                &import,
                Some(&template.uri),
            ) {
                Ok(module) => module,
                Err(error) => {
                    diagnostics.push(template_import_resolution_diagnostic(
                        template,
                        &import,
                        Some(&template.uri),
                        &error,
                    ));
                    return false;
                }
            };
            for part in [
                template.uri.as_str(),
                href,
                module.uri.as_str(),
                module.content_hash.as_str(),
                module.resolver_policy_stamp.as_deref().unwrap_or_default(),
            ] {
                self.dependency_hash_input
                    .extend_from_slice(part.as_bytes());
                self.dependency_hash_input.push(0);
            }
            let child = TemplateInput {
                uri: module.uri.clone(),
                bytes: module.bytes.clone(),
                identity: module.identity.clone(),
                root_scope: ScopeConfig::default(),
            };
            self.modules.push(module);
            if !self.visit(&child, diagnostics) {
                return false;
            }
        }
        self.active.remove(&template.uri);
        self.completed.insert(template.uri.clone());
        true
    }
}
