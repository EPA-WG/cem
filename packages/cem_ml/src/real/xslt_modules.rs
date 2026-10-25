//! XSLT authoring dependency preflight through the existing host resolver.
//! CEMT module aliases/visibility/include semantics do not apply to XSLT.
use super::*;
use crate::schema::registry::{XSLT_CONTENT_TYPE, XSLT_NAMESPACE_URI, XSLT_SCHEMA_URI};
use crate::validation::xml::XmlEventKind;

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
        // Use the same original-owned parser contract as document loading.
        // Contextless parsing can neither observe an admitted package nor
        // safely read a schema body excluded from the optional binary.
        let loaded = load_source_input_through_lifecycle(
            &EngineInput {
                uri: template.uri.clone(),
                bytes: template.bytes.clone(),
                from_format: None,
                identity: Some(FormatIdentity {
                    content_type: Some(XSLT_CONTENT_TYPE.into()),
                    schema: Some(XSLT_SCHEMA_URI.into()),
                    ..Default::default()
                }),
                root_scope: template.root_scope.clone(),
            },
            self.context,
        );
        let stylesheet = match loaded.ast_stream {
            Some(LoadedInputAstStream::XsltStylesheet(stylesheet)) => Some(stylesheet),
            _ => None,
        };
        let parsed = loaded.diagnostics;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        document_model::{
            compile_schema_document_model, compile_schema_document_model_from_retained,
        },
        package_compilation::SchemaPackageSourceRegistry,
        registry::SchemaPackageOrigin,
    };

    fn visit(context: &EngineContext, diagnostics: &mut Vec<Diagnostic>) -> bool {
        let mut state = Preflight {
            context,
            modules: Vec::new(),
            active: BTreeSet::new(),
            completed: BTreeSet::new(),
            dependency_hash_input: Vec::new(),
            bytes: 0,
            max_depth: 32,
        };
        state.visit(&TemplateInput { uri: "memory:owned-preflight.xsl".into(),
            bytes: br#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:template match="/"/></xsl:stylesheet>"#.to_vec(),
            identity: None, root_scope: Default::default() }, diagnostics)
    }

    #[test]
    fn xslt_preflight_uses_the_original_owned_parser_contract() {
        let source = include_str!("../../schema-packages/xslt/v1/schema/xslt.cem").replace(
            "cem.xslt.version_missing",
            "fixture.xslt.owned_version_missing",
        );
        let owner = SchemaPackageSourceRegistry::default()
            .retain("memory:owned-preflight-schema.cem", &source)
            .unwrap();
        let mut context = EngineContext::default();
        context.schema_registry.set_package_origin(
            "xslt",
            SchemaPackageOrigin::Manifest("memory:owned-preflight-schema.cem".into()),
        );
        context
            .schema_document_models
            .register(compile_schema_document_model_from_retained(
                XSLT_SCHEMA_URI,
                &owner,
            ));
        let mut diagnostics = Vec::new();
        assert!(!visit(&context, &mut diagnostics));
        let diagnostic = diagnostics
            .iter()
            .find(|d| d.code == "fixture.xslt.owned_version_missing")
            .expect("preflight must use the retained schema's diagnostic");
        assert_eq!(
            diagnostic.uri.as_deref(),
            Some("memory:owned-preflight.xsl")
        );
    }

    #[test]
    fn xslt_preflight_refuses_an_inspection_only_parser_contract() {
        let mut context = EngineContext::default();
        context.schema_registry.set_package_origin(
            "xslt",
            SchemaPackageOrigin::Manifest("memory:unbound-preflight-schema.cem".into()),
        );
        context
            .schema_document_models
            .inspect_candidate(compile_schema_document_model(
                XSLT_SCHEMA_URI,
                include_str!("../../schema-packages/xslt/v1/schema/xslt.cem"),
            ));
        let mut diagnostics = Vec::new();
        assert!(!visit(&context, &mut diagnostics));
        assert!(diagnostics.iter().any(|d| d.code
            == if cfg!(feature = "external-schema-assets") {
                "cem.schema_asset.missing"
            } else {
                crate::schema::document_model::MODEL_NOT_READY_CODE
            }));
        assert!(!diagnostics
            .iter()
            .any(|d| d.code == "cem.xslt.version_missing"));
    }
}
