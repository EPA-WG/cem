//! Rebuild stylesheet occurrences from the admitted native inputs.
use super::{compare, fail, island, whitespace, HTML_NAMESPACE_URI};
use crate::{
    eval::Item,
    render::{DomStylesheetSource, RenderPlanNode, TemplateArtifact},
    retained_template::RetainedTemplate,
    ssr_browser_resources::{
        SourceReceipt, StylesheetReceipt, DECLARATION_ATTRIBUTE, MAX_STYLESHEET_MANIFEST_BYTES,
        STYLESHEETS_ATTRIBUTE,
    },
    ssr_stylesheets::{stylesheet_markup, stylesheet_markup_with_identity},
};
use cem_ml::{
    css_emission::{css_stylesheet_cache_key, emit_css_import_closure, CssManagedScope},
    css_imports::{CssImportClosure, CssImportLimits, CssImportResponsePolicy},
    css_resources::CssResourceKind,
    module_resolution::{
        CemModuleUrlContext, CemModuleUrlFrame, CemModuleUrlResolutionCapability,
        CemResolutionContextHandle, CemScopedModuleUrlResolver,
    },
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    source_map::SourceMapStack,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Default)]
pub(super) struct StylesheetReads {
    base: Option<String>,
    bytes: BTreeMap<String, Vec<u8>>,
    used: BTreeSet<String>,
    consumed: usize,
}

impl StylesheetReads {
    pub(super) fn new(
        document: &str,
        base: &str,
        bytes: BTreeMap<String, Vec<u8>>,
        cross_origin_grants: &BTreeSet<String>,
    ) -> Result<Self, String> {
        use crate::ssr_browser_resources::{MAX_STYLESHEET_BYTES, MAX_STYLESHEET_TOTAL_BYTES};
        if document.len() > 4096
            || base.len() > 4096
            || !url::Url::parse(base).is_ok_and(|u| matches!(u.scheme(), "https" | "http"))
            || bytes.len() > 64
            || bytes
                .iter()
                .any(|(url, bytes)| url.len() > 4096 || bytes.len() > MAX_STYLESHEET_BYTES)
            || bytes.values().map(Vec::len).sum::<usize>() > MAX_STYLESHEET_TOTAL_BYTES
        {
            return Err(fail());
        }
        let origin = url::Url::parse(document).map_err(|_| fail())?.origin();
        if !cross_origin_grants.iter().all(|uri| bytes.contains_key(uri)) {
            return Err(fail());
        }
        for uri in bytes.keys() {
            let url = url::Url::parse(uri).map_err(|_| fail())?;
            let cross_origin = url.origin() != origin;
            if cross_origin != cross_origin_grants.contains(uri)
                || (cross_origin && !cross_origin_stylesheet_url(uri, &url))
            {
                return Err(fail());
            }
        }
        Ok(Self {
            base: Some(base.into()),
            bytes,
            ..Default::default()
        })
    }

    pub(super) fn all_used(&self) -> bool {
        self.used.len() == self.bytes.len()
    }

    fn read(&mut self, receipt: &SourceReceipt) -> Result<Vec<u8>, String> {
        use crate::ssr_browser_resources::{MAX_STYLESHEET_BYTES, MAX_STYLESHEET_TOTAL_BYTES};
        let bytes = self.bytes.get(&receipt.url).ok_or_else(fail)?;
        self.consumed += bytes.len();
        if self.consumed > MAX_STYLESHEET_TOTAL_BYTES {
            return Err(fail());
        }
        if receipt.content_type.len() > 128
            || cem_ml::schema::registry::content_type_essence(&receipt.content_type) != "text/css"
        {
            return Err(fail());
        }
        receipt.check_bytes(bytes, &receipt.content_type, MAX_STYLESHEET_BYTES)?;
        self.used.insert(receipt.url.clone());
        Ok(bytes.clone())
    }
}

fn cross_origin_stylesheet_url(uri: &str, url: &url::Url) -> bool {
    let allowed_scheme = match (url.scheme(), url.host()) {
        ("https", _) => true,
        ("http", Some(url::Host::Ipv4(ip))) => ip.is_loopback(),
        ("http", Some(url::Host::Ipv6(ip))) => ip.is_loopback(),
        _ => false,
    };
    allowed_scheme
        && url.query().is_none()
        && !uri.contains('*')
        && cem_ml::resolver::ssr_resource_url(uri).as_deref() == Ok(uri)
}

#[derive(Default)]
pub(super) struct PayloadStyles {
    sources: Vec<DomStylesheetSource>,
    pub maps: Vec<SourceMapStack>,
}

impl PayloadStyles {
    pub fn push(&mut self, style: &Item) -> Result<(), String> {
        if self.sources.len() >= 64 || !island::attrs(style)?.is_empty() {
            return Err(fail());
        }
        let mut css = String::new();
        for child in island::children(style)? {
            if !matches!(super::kind(&child).as_deref(), Some("text" | "whitespace")) {
                return Err(fail());
            }
            css.push_str(&super::text(&child)?);
        }
        if css.len() > 32_768 {
            return Err(fail());
        }
        self.maps.push(style.source_map().ok_or_else(fail)?);
        self.sources.push(DomStylesheetSource {
            css,
            scope: None,
            content_type: None,
        });
        Ok(())
    }

    pub fn verify(
        &self,
        instance: &str,
        suffix: &[Item],
        manifest: Option<&str>,
        reads: &mut StylesheetReads,
        c: &OperationControl,
    ) -> Result<bool, String> {
        let mut owner = RetainedTemplate::new_instance_stylesheets(&self.sources, instance)?;
        if owner.artifact().stylesheets.len() != self.sources.len()
            || !owner.artifact().diagnostics.is_empty()
        {
            return Err(fail());
        }
        let base = reads.base.clone();
        let (expected, _) = emit(
            &mut owner,
            instance,
            false,
            None,
            manifest,
            base.as_deref(),
            reads,
            c,
        )?;
        verify(&expected, suffix, c)?;
        Ok(!expected.is_empty())
    }
}

pub(super) fn declaration_scope(declaration: &Item) -> Result<Option<String>, String> {
    Ok(island::attrs(declaration)?
        .remove("scope")
        .filter(|scope| !scope.is_empty()))
}

pub(super) fn verify_declaration(
    declaration: Option<&Item>,
    tag: &str,
    scope: Option<&str>,
    artifact: &TemplateArtifact,
    reads: &mut StylesheetReads,
    c: &OperationControl,
) -> Result<Option<String>, String> {
    let Some(declaration) = declaration else {
        return if artifact.stylesheets.is_empty() {
            Ok(None)
        } else {
            Err(fail())
        };
    };
    let actual = island::elements(declaration)?
        .into_iter()
        .filter(|node| island::is_element(node, "style", HTML_NAMESPACE_URI))
        .collect::<Vec<_>>();
    let mut owner = RetainedTemplate::new(TemplateArtifact {
        nodes: vec![],
        stylesheets: artifact.stylesheets.clone(),
        module_map: None,
        diagnostics: vec![],
    });
    let attrs = island::attrs(declaration)?;
    let manifest = attrs.get(STYLESHEETS_ATTRIBUTE).map(String::as_str);
    let base = manifest
        .map(|_| declaration_base(tag, &attrs, reads))
        .transpose()?;
    let (expected, context) = emit(
        &mut owner,
        tag,
        true,
        scope,
        manifest,
        base.as_deref(),
        reads,
        c,
    )?;
    verify(&expected, &actual, c)?;
    Ok(context)
}

fn declaration_base(
    tag: &str,
    attrs: &BTreeMap<String, String>,
    reads: &StylesheetReads,
) -> Result<String, String> {
    let base = reads.base.as_ref().ok_or_else(fail)?;
    let Some(src) = attrs.get("src") else {
        return Ok(base.clone());
    };
    let raw = attrs
        .get(DECLARATION_ATTRIBUTE)
        .filter(|raw| raw.len() <= 8192)
        .ok_or_else(fail)?;
    let receipt: SourceReceipt = serde_json::from_str(raw).map_err(|_| fail())?;
    let mut url = url::Url::parse(base)
        .and_then(|base| base.join(src))
        .map_err(|_| fail())?;
    if url.fragment() != Some(tag) {
        return Err(fail());
    }
    url.set_fragment(None);
    if url.as_str() != receipt.url {
        return Err(fail());
    }
    Ok(receipt.url)
}

fn emit(
    owner: &mut RetainedTemplate,
    id: &str,
    declaration: bool,
    shared_scope: Option<&str>,
    manifest: Option<&str>,
    base: Option<&str>,
    reads: &mut StylesheetReads,
    c: &OperationControl,
) -> Result<(Vec<RenderPlanNode>, Option<String>), String> {
    if owner.artifact().stylesheets.len() > 64 {
        return Err(fail());
    }
    let capability = CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new()),
        CemResolutionContextHandle::new("cem-ssr-closed-css"),
    );
    let manifests = if let Some(raw) = manifest {
        if raw.len() > MAX_STYLESHEET_MANIFEST_BYTES {
            return Err(fail());
        }
        let entries: Vec<Option<StylesheetReceipt>> =
            serde_json::from_str(raw).map_err(|_| fail())?;
        if entries.len() != owner.artifact().stylesheets.len()
            || entries.iter().all(Option::is_none)
        {
            return Err(fail());
        }
        entries
    } else {
        (0..owner.artifact().stylesheets.len())
            .map(|_| None)
            .collect()
    };
    let mut context = None;
    let output = manifests
        .into_iter()
        .enumerate()
        .map(|(index, manifest)| {
            c.check_scope(ROOT_EXECUTION_SCOPE_ID).map_err(|_| fail())?;
            let source = &owner.artifact().stylesheets[index];
            let scope = if declaration {
                match source.scope.as_deref() {
                    None => CssManagedScope::Private {
                        tag: id.into(),
                        context: None,
                    },
                    Some(name) if Some(name) == shared_scope => CssManagedScope::Shared {
                        name: name.into(),
                        context: None,
                    },
                    Some(_) => return Err(fail()),
                }
            } else {
                CssManagedScope::Instance
            };
            if let Some(manifest) = manifest {
                let closure = replay_imports(source, base.ok_or_else(fail)?, &manifest, reads, c)?;
                let scope = match scope {
                    CssManagedScope::Private { tag, .. } => CssManagedScope::Private {
                        tag,
                        context: Some(manifest.context.clone()),
                    },
                    CssManagedScope::Shared { name, .. } => CssManagedScope::Shared {
                        name,
                        context: Some(manifest.context.clone()),
                    },
                    CssManagedScope::Instance => CssManagedScope::Instance,
                };
                if declaration
                    && context
                        .replace(manifest.context.clone())
                        .is_some_and(|previous| previous != manifest.context)
                {
                    return Err(fail());
                }
                let emission = emit_css_import_closure(&closure, &scope, &manifest.owner_key)
                    .map_err(|_| fail())?;
                if !emission.diagnostics.is_empty() {
                    return Err(fail());
                }
                let key = if declaration {
                    css_stylesheet_cache_key(&closure, &manifest.owner_key)?
                } else {
                    String::new()
                };
                return stylesheet_markup_with_identity(
                    emission.css(),
                    &scope,
                    &key,
                    index,
                    declaration,
                    &SourceMapStack::default(),
                );
            }
            let closure = CssImportClosure::new(
                source.css_tree().clone(),
                "cem-ssr-css://inline/",
                capability.clone(),
                CssImportLimits {
                    max_sheets: 1,
                    max_depth: 0,
                },
                c.abort_signal().clone(),
            )
            .map_err(|_| fail())?;
            let record = owner.retain_closed_ssr_stylesheet(
                "resume",
                index,
                Arc::new(closure),
                &scope,
                id,
            )?;
            if !record.emission()?.diagnostics.is_empty() {
                return Err(fail());
            }
            stylesheet_markup(&record, index, declaration, &SourceMapStack::default())
        })
        .collect::<Result<_, _>>()?;
    Ok((output, context))
}

fn replay_imports(
    source: &crate::render::TemplateStylesheetArtifact,
    base: &str,
    manifest: &StylesheetReceipt,
    reads: &mut StylesheetReads,
    c: &OperationControl,
) -> Result<CssImportClosure, String> {
    let digest = |key: &str, prefix| {
        key.strip_prefix(prefix).is_some_and(|hash| {
            hash.len() == 64
                && hash
                    .bytes()
                    .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        })
    };
    if !digest(&manifest.owner_key, "cem-style-1-")
        || !digest(&manifest.context, "cem-context-1-")
        || manifest.sources.is_empty()
        || manifest.sources.len() >= 64
    {
        return Err(fail());
    }
    let mut receipts = BTreeMap::new();
    for receipt in &manifest.sources {
        if receipts.insert(receipt.url.as_str(), receipt).is_some() {
            return Err(fail());
        }
    }
    let handle = CemResolutionContextHandle::new("cem-ssr-css-replay");
    let resolver = CemScopedModuleUrlResolver::new().with_context(
        handle.clone(),
        CemModuleUrlContext {
            identity: handle.0.clone(),
            resolver_identity: handle.0.clone(),
            resource_policy_stamp: handle.0.clone(),
            frames: vec![CemModuleUrlFrame::new("page", base)],
        },
    );
    let capability = CemModuleUrlResolutionCapability::new(Arc::new(resolver), handle);
    let mut closure = CssImportClosure::new(
        source.css_tree().clone(),
        base,
        capability,
        CssImportLimits::default(),
        c.abort_signal().clone(),
    )
    .map_err(|_| fail())?;
    let policy = CssImportResponsePolicy {
        max_response_bytes: crate::ssr_browser_resources::MAX_STYLESHEET_BYTES,
        max_total_bytes: crate::ssr_browser_resources::MAX_STYLESHEET_TOTAL_BYTES,
    };
    let mut used = BTreeSet::new();
    while let Some(request) = closure.next_import().map_err(|_| fail())? {
        c.check_scope(ROOT_EXECUTION_SCOPE_ID).map_err(|_| fail())?;
        let uri = request.resolution.resolved_url;
        let receipt = receipts.get(uri.as_str()).ok_or_else(fail)?;
        let bytes = reads.read(receipt)?;
        used.insert(receipt.url.as_str());
        closure
            .complete_response_with_control(
                request.id,
                cem_ml::resolver::ResolvedRead {
                    uri,
                    bytes,
                    content_type: Some(receipt.content_type.clone()),
                },
                &policy,
                c,
            )
            .map_err(|_| fail())?;
    }
    if used.len() != receipts.len()
        || closure.sheets().iter().any(|sheet| {
            sheet
                .resources
                .references
                .iter()
                .any(|reference| !matches!(reference.kind, CssResourceKind::Import { .. }))
        })
    {
        return Err(fail());
    }
    Ok(closure)
}

fn verify(
    expected: &[RenderPlanNode],
    actual: &[Item],
    c: &OperationControl,
) -> Result<(), String> {
    let actual = actual
        .iter()
        .filter(|node| !whitespace(node))
        .cloned()
        .collect::<Vec<_>>();
    compare(expected, &actual, 0, c, None, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cem_ml::content_cache::ContentHash;

    #[test]
    fn off_origin_stylesheet_reads_require_an_exact_host_grant() {
        let document = "https://page.test/main.html";
        let source = "https://cdn.test/styles/main.css";
        let bytes = BTreeMap::from([(source.into(), b"article {color:red}".to_vec())]);
        let admit = |grants| StylesheetReads::new(document, document, bytes.clone(), &grants);
        assert!(admit(BTreeSet::new()).is_err());
        assert!(admit(BTreeSet::from(["https://cdn.test/styles/other.css".into()])).is_err());
        assert!(admit(BTreeSet::from([source.into()])).is_ok());

        for (uri, allowed) in [
            ("http://127.0.0.1:6789/main.css", true),
            ("http://[::1]:6789/main.css", true),
            ("http://cdn.test/main.css", false),
            ("https://cdn.test/main.css?", false),
        ] {
            let bytes = BTreeMap::from([(uri.into(), vec![])]);
            let grants = BTreeSet::from([uri.into()]);
            assert_eq!(StylesheetReads::new(document, document, bytes, &grants).is_ok(), allowed);
        }
    }

    #[test]
    fn replay_requires_exact_nested_import_responses() {
        let base = "https://ssr.test/assets/page.html";
        let first = "https://ssr.test/assets/styles/main.css";
        let second = "https://ssr.test/assets/theme.css";
        let main = b"@import '../theme.css'; article { padding: 3px }".to_vec();
        let theme = b"article { color: red }".to_vec();
        let receipt = |url, bytes: &[u8]| {
            SourceReceipt::new(
                url,
                "text/css",
                bytes.len(),
                &ContentHash::from_blake3(bytes),
            )
        };
        let mut manifest = StylesheetReceipt {
            owner_key: format!("cem-style-1-{}", "a".repeat(64)),
            context: format!("cem-context-1-{}", "b".repeat(64)),
            sources: vec![receipt(first, &main), receipt(second, &theme)],
        };
        let root = crate::render::TemplateStylesheetArtifact::adopt(
            "@import './styles/main.css' layer(theme);".into(),
            None,
        )
        .unwrap();
        let control = OperationControl::default();
        let replay = |manifest: &StylesheetReceipt, inputs| {
            let mut reads = StylesheetReads::new(base, base, inputs, &BTreeSet::new()).unwrap();
            replay_imports(&root, base, manifest, &mut reads, &control)
        };
        let inputs =
            std::collections::BTreeMap::from([(first.into(), main), (second.into(), theme)]);
        assert!(StylesheetReads::new("https://different.test/", base, inputs.clone(), &BTreeSet::new()).is_err());
        let closure = replay(&manifest, inputs.clone()).unwrap();
        assert_eq!(
            closure
                .sheets()
                .iter()
                .map(|s| s.url.as_str())
                .collect::<Vec<_>>(),
            [base, first, second]
        );
        let mut changed = inputs.clone();
        changed.insert(second.into(), b"article { color: tan }".to_vec());
        assert!(replay(&manifest, changed).is_err());
        let mut missing = inputs.clone();
        missing.remove(second);
        assert!(replay(&manifest, missing).is_err());
        manifest
            .sources
            .push(receipt("https://ssr.test/unrelated.css", b""));
        assert!(replay(&manifest, inputs).is_err());
    }
}
