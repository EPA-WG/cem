//! Build-pinned CSS dependency responses, replayed through the current native
//! resolver and response policy. The bundle conveys no authority to read URLs.
use crate::{
    compiled_css::{self, AdmittedCss},
    content_cache::ContentHash,
    css_imports::{CssImportClosure, CssImportLimits, CssImportRequest, CssImportResponsePolicy},
    module_resolution::CemModuleUrlResolutionCapability,
    parser::tree::RetainedCemTree,
    resolver::ResolvedRead,
    scheduler::AbortSignal,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledCssImportsArtifact {
    base_url: String,
    responses: Vec<CompiledCssResponse>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompiledCssResponse {
    requested_url: String,
    hash: String,
    bytes: Vec<u8>,
}
#[derive(Debug)]
pub struct CompiledCssImports {
    artifact: CompiledCssImportsArtifact,
    responses: Vec<AdmittedCss>,
}
fn canonical_url(value: &str) -> Result<String, String> {
    let mut url = url::Url::parse(value).map_err(|e| e.to_string())?;
    url.set_fragment(None);
    Ok(url.into())
}

/// The caller provides an explicit build-time resolver/loader. There is no
/// implicit filesystem/network access. Repeated occurrences preserve ordering.
pub fn compile(
    root: Arc<RetainedCemTree>,
    base_url: &str,
    capability: CemModuleUrlResolutionCapability,
    load: &mut dyn FnMut(&CssImportRequest) -> Result<ResolvedRead, String>,
) -> Result<CompiledCssImportsArtifact, String> {
    let mut closure = CssImportClosure::new(
        root,
        base_url,
        capability,
        CssImportLimits::default(),
        AbortSignal::new(),
    )
    .map_err(|e| e.to_string())?;
    let mut responses = Vec::new();
    while let Some(request) = closure.next_import().map_err(|e| e.to_string())? {
        let response = load(&request)?;
        if response.bytes.len() > crate::import::MAX_DOCUMENT_BYTES {
            return Err("CSS build response exceeds the document byte limit".into());
        }
        let source = std::str::from_utf8(&response.bytes)
            .map_err(|e| e.to_string())?
            .to_owned();
        let mime = response
            .content_type
            .as_deref()
            .or(request.resolution.content_type_hint.as_deref())
            .unwrap_or("text/css")
            .to_owned();
        closure
            .complete_response(request.id, response, &Default::default())
            .map_err(|e| e.to_string())?;
        let tree = &closure
            .sheets()
            .last()
            .ok_or("missing imported stylesheet")?
            .resources
            .tree;
        let bytes = compiled_css::encode_response(tree, &source, &mime)?;
        responses.push(CompiledCssResponse {
            requested_url: canonical_url(&request.resolution.resolved_url)?,
            hash: ContentHash::from_blake3(&bytes).header_value(),
            bytes,
        });
    }
    let artifact = CompiledCssImportsArtifact {
        base_url: canonical_url(base_url)?,
        responses,
    };
    // Apply the same decoded allocation bound before publishing build output.
    CompiledCssImports::admit(artifact.clone())?;
    Ok(artifact)
}
impl CompiledCssImports {
    pub fn admit(artifact: CompiledCssImportsArtifact) -> Result<Self, String> {
        if artifact.responses.len() >= CssImportLimits::default().max_sheets
            || canonical_url(&artifact.base_url)? != artifact.base_url
        {
            return Err("compiled CSS closure base/count mismatch".into());
        }
        let mut responses = Vec::new();
        let mut expanded = 0usize;
        let mut source_bytes = 0usize;
        let policy = CssImportResponsePolicy::default();
        for response in &artifact.responses {
            if canonical_url(&response.requested_url)? != response.requested_url {
                return Err("compiled CSS request URL mismatch".into());
            }
            let admitted = compiled_css::admit(&response.bytes, &response.hash)?;
            // Imports are full stylesheet responses, never scoped inline blocks.
            let root = admitted.tree.node(0).unwrap().children[0];
            if admitted
                .tree
                .node(root)
                .unwrap()
                .name
                .as_ref()
                .unwrap()
                .local_name
                != "stylesheet"
                || canonical_url(admitted.tree.source_uri())? != admitted.tree.source_uri()
            {
                return Err("compiled CSS response root/URL mismatch".into());
            }
            expanded = expanded
                .checked_add(admitted.expansion_bytes)
                .ok_or("CSS expansion limit")?;
            source_bytes = source_bytes
                .checked_add(admitted.source.len())
                .ok_or("CSS byte limit")?;
            if expanded > policy.max_total_bytes || source_bytes > policy.max_total_bytes {
                return Err("compiled CSS closure allocation/byte limit".into());
            }
            responses.push(admitted);
        }
        Ok(Self {
            artifact,
            responses,
        })
    }
    pub fn check_base(&self, base: &str) -> Result<(), String> {
        if canonical_url(base)? != self.artifact.base_url {
            return Err("compiled CSS closure base changed".into());
        }
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.responses.len()
    }
    pub fn is_empty(&self) -> bool {
        self.responses.is_empty()
    }
    pub fn complete(
        &self,
        closure: &mut CssImportClosure,
        index: usize,
        request: &CssImportRequest,
        policy: &CssImportResponsePolicy,
    ) -> Result<(), crate::css_imports::CssImportFailure> {
        if self.artifact.responses.get(index).is_none_or(|response| {
            canonical_url(&request.resolution.resolved_url)
                .ok()
                .as_ref()
                != Some(&response.requested_url)
        }) {
            closure.fail_import(
                request.id,
                "cem.css.compiled_import_mismatch",
                "compiled stylesheet dependency is missing or its resolved URL changed",
            )?;
            return Err(closure.failure().unwrap());
        }
        closure.complete_compiled_response(request.id, &self.responses[index], policy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        css_emission::{derive_css_stylesheet_identity, emit_css_import_closure, CssManagedScope},
        module_resolution::{
            CemModuleUrlContext, CemModuleUrlFrame, CemModuleUrlMapping,
            CemResolutionContextHandle, CemScopedModuleUrlResolver,
        },
    };
    const BASE: &str = "https://app.test/ui/view.xhtml";
    fn capability(
        mapping: Option<(&str, CemModuleUrlMapping)>,
    ) -> CemModuleUrlResolutionCapability {
        let mut frame = CemModuleUrlFrame::new("page", BASE);
        if let Some((key, value)) = mapping {
            frame.specifiers.resources.insert(key.into(), value);
        }
        let handle = CemResolutionContextHandle::new("imports");
        let context = CemModuleUrlContext {
            identity: "page".into(),
            resolver_identity: "resolver".into(),
            resource_policy_stamp: "policy".into(),
            frames: vec![frame],
        };
        CemModuleUrlResolutionCapability::new(
            Arc::new(CemScopedModuleUrlResolver::new().with_context(handle.clone(), context)),
            handle,
        )
    }
    fn root() -> Arc<RetainedCemTree> {
        crate::import::import_data_bytes(
            b"@import './main.css' layer(first); @import './main.css' screen; :scope {color:red}",
            "text/css; mode=scoped-style-block",
            "cem",
            "urn:root",
        )
        .unwrap()
    }
    fn response(request: &CssImportRequest) -> Result<ResolvedRead, String> {
        let (uri, text) = if request.resolution.resolved_url.ends_with("main.css") {
            ("https://app.test/cdn/main.css", "@import './child.css' supports(display:block); :scope {background:url('../img/a.svg');animation:pulse 1s} @keyframes pulse {to{opacity:.5}} #forbidden{color:red}")
        } else {
            (
                "https://app.test/cdn/child.css",
                ":scope { --title:'café'; color:blue }",
            )
        };
        Ok(ResolvedRead {
            uri: uri.into(),
            bytes: text.as_bytes().to_vec(),
            content_type: Some("text/css; charset=utf-8".into()),
        })
    }
    fn closure(
        root: Arc<RetainedCemTree>,
        capability: CemModuleUrlResolutionCapability,
    ) -> CssImportClosure {
        CssImportClosure::new(
            root,
            BASE,
            capability,
            Default::default(),
            AbortSignal::new(),
        )
        .unwrap()
    }
    fn replay(
        bundle: &CompiledCssImports,
        graph: &mut CssImportClosure,
        policy: CssImportResponsePolicy,
    ) -> Result<(), String> {
        let mut index = 0;
        while let Some(request) = graph.next_import().map_err(|e| e.to_string())? {
            bundle
                .complete(graph, index, &request, &policy)
                .map_err(|e| e.to_string())?;
            index += 1;
        }
        if index != bundle.len() {
            return Err("unused compiled CSS responses".into());
        }
        Ok(())
    }
    #[test]
    fn imported_capsules_preserve_provenance_emission_and_policy_without_parsing() {
        let root = root();
        let artifact = compile(root.clone(), BASE, capability(None), &mut response).unwrap();
        let again = compile(root.clone(), BASE, capability(None), &mut response).unwrap();
        assert_eq!(
            rmp_serde::to_vec(&artifact).unwrap(),
            rmp_serde::to_vec(&again).unwrap()
        );
        assert_eq!(artifact.responses.len(), 4); // repeated imports remain distinct
        assert!(compiled_css::admit_inline(&artifact.responses[0].bytes, &artifact.responses[0].hash).is_err());
        let mut source = closure(root.clone(), capability(None));
        while let Some(request) = source.next_import().unwrap() {
            source
                .complete_response(request.id, response(&request).unwrap(), &Default::default())
                .unwrap();
        }
        let before = crate::validation::css::CSS_SOURCE_PARSE_CALLS.with(|n| n.get());
        let admitted = CompiledCssImports::admit(artifact).unwrap();
        let mut compiled = closure(root, capability(None));
        replay(&admitted, &mut compiled, Default::default()).unwrap();
        assert_eq!(
            before,
            crate::validation::css::CSS_SOURCE_PARSE_CALLS.with(|n| n.get())
        );
        assert_eq!(source.received_bytes(), compiled.received_bytes());
        for (left, right) in source.sheets().iter().zip(compiled.sheets()) {
            assert_eq!(left.url, right.url);
            for id in 0..left.resources.tree.ast().nodes.len() {
                assert_eq!(
                    format!("{:?}", left.resources.tree.node(id as u32)),
                    format!("{:?}", right.resources.tree.node(id as u32))
                );
                assert_eq!(
                    left.resources.tree.source_key(id as u32),
                    right.resources.tree.source_key(id as u32)
                );
            }
        }
        for scope in [
            CssManagedScope::Private {
                tag: "cem-import".into(),
                context: None,
            },
            CssManagedScope::Shared {
                name: "group".into(),
                context: None,
            },
            CssManagedScope::Instance,
        ] {
            let a = derive_css_stylesheet_identity(&source, &scope, "decl", "style:0").unwrap();
            let b = derive_css_stylesheet_identity(&compiled, &scope, "decl", "style:0").unwrap();
            assert_eq!(a, b);
            let a = emit_css_import_closure(&source, &a.scope, &a.owner_key).unwrap();
            let b = emit_css_import_closure(&compiled, &b.scope, &b.owner_key).unwrap();
            assert_eq!(a.css(), b.css());
            assert!(b.css().contains("https://app.test/img/a.svg"));
            assert_eq!(
                format!("{:?}", a.diagnostics),
                format!("{:?}", b.diagnostics)
            );
            assert!(!b.diagnostics.is_empty());
        }
    }
    #[test]
    fn current_url_redirect_mime_integrity_and_byte_policy_are_required() {
        let root = root();
        let artifact = compile(root.clone(), BASE, capability(None), &mut response).unwrap();
        let admitted = CompiledCssImports::admit(artifact).unwrap();
        assert!(admitted
            .check_base("https://other.test/view.xhtml")
            .is_err());
        // A valid caller pin is checked over the original source bytes, not IR.
        use base64::Engine;
        use sha2::Digest;
        let mut probe = closure(root.clone(), capability(None));
        let request = probe.next_import().unwrap().unwrap();
        let digest = sha2::Sha256::digest(&response(&request).unwrap().bytes);
        let integrity = format!(
            "sha256-{}",
            base64::engine::general_purpose::STANDARD.encode(digest)
        );
        replay(
            &admitted,
            &mut closure(
                root.clone(),
                capability(Some((
                    "./main.css",
                    CemModuleUrlMapping::target("./main.css").with_integrity(integrity),
                ))),
            ),
            Default::default(),
        )
        .unwrap();
        for (key, mapping, code) in [
            (
                "./main.css",
                CemModuleUrlMapping::target("./changed.css"),
                "compiled_import_mismatch",
            ),
            (
                "https://app.test/cdn/main.css",
                CemModuleUrlMapping::blocked(),
                "import_redirect_denied",
            ),
            (
                "./main.css",
                CemModuleUrlMapping::target("./main.css").with_content_type("application/json"),
                "import_content_type",
            ),
            (
                "./main.css",
                CemModuleUrlMapping::target("./main.css")
                    .with_integrity("sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="),
                "import_integrity",
            ),
        ] {
            let mut graph = closure(root.clone(), capability(Some((key, mapping))));
            let error = replay(&admitted, &mut graph, Default::default()).unwrap_err();
            assert!(error.contains(code), "{error}");
            assert!(graph.failure().unwrap().location.is_some());
        }
        for policy in [
            CssImportResponsePolicy {
                max_response_bytes: 1,
                ..Default::default()
            },
            CssImportResponsePolicy {
                max_total_bytes: 1,
                ..Default::default()
            },
        ] {
            assert!(replay(
                &admitted,
                &mut closure(root.clone(), capability(None)),
                policy
            )
            .unwrap_err()
            .contains("import_byte_limit"));
        }
    }
    #[test]
    fn corrupt_missing_extra_and_cyclic_closures_fail_closed() {
        let root = root();
        let artifact = compile(root.clone(), BASE, capability(None), &mut response).unwrap();
        let mut corrupt = artifact.clone();
        corrupt.responses[0].bytes[0] ^= 1;
        assert!(CompiledCssImports::admit(corrupt).is_err());
        let mut missing = artifact.clone();
        missing.responses.pop();
        let admitted = CompiledCssImports::admit(missing).unwrap();
        assert!(replay(
            &admitted,
            &mut closure(root.clone(), capability(None)),
            Default::default()
        )
        .unwrap_err()
        .contains("compiled_import_mismatch"));
        let mut extra = artifact.clone();
        extra.responses.push(artifact.responses[0].clone());
        assert!(replay(
            &CompiledCssImports::admit(extra).unwrap(),
            &mut closure(root.clone(), capability(None)),
            Default::default()
        )
        .unwrap_err()
        .contains("unused"));
        let mut oversized = artifact.clone();
        oversized.responses = vec![artifact.responses[0].clone(); 64];
        assert!(CompiledCssImports::admit(oversized).is_err());
        let cycle = compile(root, BASE, capability(None), &mut |r| {
            Ok(ResolvedRead {
                uri: r.resolution.resolved_url.clone(),
                bytes: b"@import './main.css';".to_vec(),
                content_type: Some("text/css".into()),
            })
        })
        .unwrap_err();
        assert!(cycle.contains("import_cycle"), "{cycle}");
    }
}
