//! Retained-document inline CSS analysis through native import and resolution.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::module_resolution::*;
    fn capability() -> CemModuleUrlResolutionCapability {
        let handle = CemResolutionContextHandle::new("page");
        let mut frame = CemModuleUrlFrame::new("page", "https://example.test/app/page.xhtml");
        frame
            .specifiers
            .resources
            .insert("icon".into(), CemModuleUrlMapping::target("./mapped.svg"));
        frame
            .specifiers
            .resources
            .insert("blocked".into(), CemModuleUrlMapping::blocked());
        CemModuleUrlResolutionCapability::new(
            std::sync::Arc::new(CemScopedModuleUrlResolver::new().with_context(
                handle.clone(),
                CemModuleUrlContext {
                    identity: "page".into(),
                    resolver_identity: "resolver".into(),
                    resource_policy_stamp: "policy-v1".into(),
                    frames: vec![frame],
                },
            )),
            handle,
        )
    }
    fn document(source: &str) -> std::sync::Arc<crate::parser::tree::RetainedCemTree> {
        crate::import::import_data_bytes(
            source.as_bytes(),
            "application/xml",
            "cem",
            "urn:synthetic:source",
        )
        .unwrap()
    }
    #[test]
    fn document_stylesheet_inventory_preserves_owners_and_resolution_policy() {
        let tree = document(
            r#"<html xmlns="http://www.w3.org/1999/xhtml"><style>body { background: url(icon) }</style><p style="background: url('images/a.svg?x=1&amp;y=2')"/></html>"#,
        );
        let result = inspect_document_stylesheets(
            tree.clone(),
            &capability(),
            "https://example.test/output/page.xhtml",
            100,
        );
        assert_eq!(result.stylesheets.len(), 2);
        assert!(std::sync::Arc::ptr_eq(&tree, &result.document.tree));
        for sheet in &result.stylesheets {
            assert!(tree.node(sheet.source_node).is_some());
            assert!(!sheet.host_source.frames.is_empty());
            assert_eq!(
                sheet.resources.as_ref().unwrap().references[0]
                    .resolution
                    .as_ref()
                    .unwrap()
                    .resource_policy_stamp,
                "policy-v1"
            );
        }
        assert_eq!(
            result.stylesheets[0].resources.as_ref().unwrap().references[0]
                .resolution
                .as_ref()
                .unwrap()
                .resolved_url,
            "https://example.test/app/mapped.svg"
        );
        assert_eq!(
            result.stylesheets[1].resources.as_ref().unwrap().references[0]
                .resolution
                .as_ref()
                .unwrap()
                .resolved_url,
            "https://example.test/output/images/a.svg?x=1&y=2"
        );
        assert!(result.document.body_analysis_required);
    }
    #[test]
    fn document_stylesheet_inventory_blocks_unresolved_bases_and_truncated_scans() {
        for source in [
            r#"<html xmlns="http://www.w3.org/1999/xhtml"><style>p { background: url(a.svg) }</style><base href="other/"/></html>"#,
            r#"<html xmlns="http://www.w3.org/1999/xhtml" xml:base="other/"><p style="background: url(a.svg)"/></html>"#,
        ] {
            let result = inspect_document_stylesheets(
                document(source),
                &capability(),
                "https://example.test/page.xhtml",
                100,
            );
            assert!(result.stylesheets.is_empty());
            assert!(!result.blockers.is_empty());
        }
        let tree = document(
            r#"<html xmlns="http://www.w3.org/1999/xhtml"><p style="background: url(a.svg)"/></html>"#,
        );
        let result = inspect_document_stylesheets(
            tree.clone(),
            &capability(),
            "https://example.test/page.xhtml",
            1,
        );
        assert!(result.stylesheets.is_empty());
        let result = inspect_document_stylesheets(tree, &capability(), "relative/page.xhtml", 100);
        assert!(result.stylesheets.is_empty());
    }
    #[test]
    fn document_stylesheet_inventory_keeps_denial_and_rejects_foreign_or_structural_css() {
        let tree = document(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><path style="fill: url(blocked)"/><style><nested/></style><other xmlns="urn:custom" style="fill: url(secret)"/></svg>"#,
        );
        let result = inspect_document_stylesheets(
            tree,
            &capability(),
            "https://example.test/image.svg",
            100,
        );
        assert_eq!(result.stylesheets.len(), 2);
        let error = result.stylesheets[0].resources.as_ref().unwrap().references[0]
            .resolution
            .as_ref()
            .unwrap_err();
        assert_eq!(error.reason, CemModuleUrlResolutionErrorReason::Blocked);
        assert_eq!(error.resource_policy_stamp, "policy-v1");
        assert!(result.stylesheets[1].resources.is_err());
    }
    #[test]
    fn xml_stylesheet_inventory_uses_import_owned_bases() {
        let source = br#"<svg xmlns="http://www.w3.org/2000/svg" xml:base="../assets/"><g xml:base="icons/"><style>path { fill: url(./paint.svg) }</style><path style="fill: url(./other.svg)"/></g></svg>"#;
        let result = inspect_xml_document_stylesheets(
            source,
            &capability(),
            "https://example.test/docs/a.svg",
            100,
        )
        .unwrap();
        assert_eq!(result.stylesheets.len(), 2);
        let urls: Vec<_> = result
            .stylesheets
            .iter()
            .map(|s| {
                s.resources.as_ref().unwrap().references[0]
                    .resolution
                    .as_ref()
                    .unwrap()
                    .resolved_url
                    .as_str()
            })
            .collect();
        assert_eq!(
            urls,
            vec![
                "https://example.test/assets/icons/paint.svg",
                "https://example.test/assets/icons/other.svg"
            ]
        );
        assert!(result.document.body_analysis_required);
        let bad = br#"<svg xmlns="http://www.w3.org/2000/svg" xml:base="http://["><path style="fill: url(./paint.svg)"/></svg>"#;
        let result =
            inspect_xml_document_stylesheets(bad, &capability(), "https://example.test/a.svg", 100)
                .unwrap();
        assert!(result.stylesheets[0].resources.is_err());
    }
    #[test]
    fn document_stylesheet_inventory_keeps_imports_and_rejects_non_css_types() {
        let tree = document(
            r#"<html xmlns="http://www.w3.org/1999/xhtml"><style>@import "theme.css";</style><style type="application/unknown">load(secret)</style></html>"#,
        );
        let result = inspect_document_stylesheets(
            tree,
            &capability(),
            "https://example.test/out/page.xhtml",
            100,
        );
        assert_eq!(result.stylesheets.len(), 2);
        let plan = result.stylesheets[0].resources.as_ref().unwrap();
        assert!(matches!(
            plan.references[0].kind,
            crate::css_resources::CssResourceKind::Import { .. }
        ));
        assert!(result.stylesheets[1].resources.is_err());
    }
}

use crate::{
    css_resources::{resolve_css_resources, CssResourcePlan},
    document_dependencies::{inspect_document_dependencies, DocumentDependencyInspection},
    module_resolution::CemModuleUrlResolutionCapability,
    parser::{
        tree::{CemTreeNodeKind, RetainedCemTree},
        AstNodeId,
    },
    source_map::SourceMapStack,
};
use std::sync::Arc;

pub struct DocumentStylesheetOccurrence {
    pub owner: AstNodeId,
    pub source_node: AstNodeId,
    /// Coordinates in the document; resource maps remain CSS-local.
    pub host_source: SourceMapStack,
    pub resources: Result<CssResourcePlan, String>,
}
pub struct DocumentStylesheetInventory {
    pub document: DocumentDependencyInspection,
    pub stylesheets: Vec<DocumentStylesheetOccurrence>,
    pub presentations: Vec<crate::import::svg_presentation::SvgPresentationPlan>,
    pub blockers: Vec<String>,
}
/// `document_url` is the host's effective final document URL, never a synthetic
/// import URI. The capability must be the actual execution context. Unresolved
/// HTML/XML base declarations block this bounded integration. Plans authorize no
/// acquisition, installation or source edit; import closures remain outstanding.
pub fn inspect_document_stylesheets(
    tree: Arc<RetainedCemTree>,
    capability: &CemModuleUrlResolutionCapability,
    document_url: &str,
    budget: usize,
) -> DocumentStylesheetInventory {
    inspect_stylesheets(
        tree,
        capability,
        document_url,
        budget,
        StylesheetBase::Fixed,
    )
}
/// Import exact XML at its final URL and use import-owned per-node base chains.
/// This explicit XML path does not emulate HTML base-element behavior.
pub fn inspect_xml_document_stylesheets(
    bytes: &[u8],
    capability: &CemModuleUrlResolutionCapability,
    document_url: &str,
    budget: usize,
) -> Result<DocumentStylesheetInventory, String> {
    url::Url::parse(document_url).map_err(|_| "absolute XML document URL required")?;
    let tree = crate::import::import_data_bytes(bytes, "application/xml", "cem", document_url)?;
    Ok(inspect_stylesheets(
        tree,
        capability,
        document_url,
        budget,
        StylesheetBase::ImportedXml,
    ))
}

/// Only the execution-host inventory calls this after validating current native
/// ownership and live DOM correspondence. No arbitrary URL can opt into it.
pub(crate) fn inspect_html_execution_stylesheets(
    evidence: &crate::html_base_evidence::CapturedHtmlBaseEvidence,
    capability: &CemModuleUrlResolutionCapability,
    budget: usize,
) -> DocumentStylesheetInventory {
    inspect_stylesheets(
        evidence.native_resolution().tree.clone(),
        capability,
        evidence.effective_url(),
        budget,
        StylesheetBase::HtmlExecution,
    )
}

#[derive(PartialEq, Eq)]
enum StylesheetBase {
    Fixed,
    ImportedXml,
    HtmlExecution,
}

fn inspect_stylesheets(
    tree: Arc<RetainedCemTree>,
    capability: &CemModuleUrlResolutionCapability,
    document_url: &str,
    budget: usize,
    base: StylesheetBase,
) -> DocumentStylesheetInventory {
    let document = inspect_document_dependencies(tree.clone(), budget);
    let mut result = DocumentStylesheetInventory {
        document,
        stylesheets: Vec::new(),
        presentations: Vec::new(),
        blockers: vec!["css-import-and-runtime-closure-required".into()],
    };
    if url::Url::parse(document_url).is_err() {
        result
            .blockers
            .push("absolute-document-url-required".into());
        return result;
    }
    if result.document.blockers.iter().any(|b| {
        b.contains("budget")
            || b.contains("invalid-retained")
            || b == "srcset-numeric-profile-incomplete"
            || b == "svg-presentation-analysis-incomplete"
            || (base == StylesheetBase::Fixed && b == "document-base-context-required")
            || b == "document-import-diagnostics"
    }) {
        result
            .blockers
            .push("complete-document-base-inventory-required".into());
        return result;
    }
    // First collect all owners so even a later <base> prevents earlier resolution.
    // The completed document inventory bounded these same nodes and attributes.
    let mut pending = vec![0];
    let mut candidates = Vec::new();
    while let Some(id) = pending.pop() {
        let Some(node) = tree.node(id) else {
            continue;
        };
        pending.extend(node.children.iter().rev().copied());
        let Some(name) = &node.name else {
            continue;
        };
        if node.kind != CemTreeNodeKind::Element
            || !matches!(
                name.namespace_uri.as_str(),
                "http://www.w3.org/1999/xhtml" | "http://www.w3.org/2000/svg"
            )
        {
            continue;
        }
        if base != StylesheetBase::HtmlExecution
            && name.namespace_uri == "http://www.w3.org/1999/xhtml"
            && name.local_name == "base"
        {
            result
                .blockers
                .push("document-base-context-required".into());
            return result;
        }
        if name.local_name == "style" {
            candidates.push((id, id, false));
        }
        for attr_id in &node.attributes {
            if tree
                .node(*attr_id)
                .and_then(|a| a.name.as_ref())
                .is_some_and(|n| n.namespace_uri.is_empty() && n.local_name == "style")
            {
                candidates.push((id, *attr_id, true));
            }
        }
    }
    for source in &result.document.presentations {
        let url = if base == StylesheetBase::ImportedXml {
            tree.node_base_uri(source.owner())
        } else {
            Some(document_url)
        };
        match url
            .ok_or_else(|| "missing SVG presentation base".to_string())
            .and_then(|url| resolve_css_resources(source.css().clone(), capability, url))
        {
            Ok(resources) => {
                result
                    .presentations
                    .push(crate::import::svg_presentation::SvgPresentationPlan {
                        source: source.clone(),
                        resources,
                    })
            }
            Err(_) => result
                .blockers
                .push("svg-presentation-base-unresolved".into()),
        }
    }
    for (owner, source_node, attribute) in candidates {
        let node = tree.node(source_node).unwrap();
        let resources = (|| {
            let source = if attribute {
                node.value.clone()
            } else {
                for id in &node.attributes {
                    let attr = tree.node(*id).ok_or("invalid style attribute")?;
                    if attr
                        .name
                        .as_ref()
                        .is_some_and(|n| n.namespace_uri.is_empty() && n.local_name == "type")
                        && !attr.value.trim().is_empty()
                        && !attr.value.trim().eq_ignore_ascii_case("text/css")
                    {
                        return Err("unsupported style content type".into());
                    }
                }
                let mut source = String::new();
                for id in &node.children {
                    let child = tree.node(*id).ok_or("invalid style child")?;
                    match child.kind {
                        CemTreeNodeKind::Text => source.push_str(&child.value),
                        CemTreeNodeKind::Comment => {}
                        _ => return Err("style content requires native text".into()),
                    }
                    if source.len() > crate::import::MAX_DOCUMENT_BYTES {
                        return Err("inline CSS import limit exceeded".into());
                    }
                }
                source
            };
            let content_type = if attribute {
                "text/css; mode=style-attribute"
            } else {
                "text/css"
            };
            // A synthetic URI identifies local CSS coordinates, never its URL base.
            let source_uri = format!(
                "urn:cem:document-css:{source_node}:{}",
                crate::content_cache::ContentHash::from_blake3(source.as_bytes()).header_value()
            );
            let css = crate::import::import_data_bytes(
                source.as_bytes(),
                content_type,
                "cem",
                &source_uri,
            )?;
            if css
                .ast_owner()
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation())
            {
                return Err("inline CSS import diagnostics".into());
            }
            let base = if base == StylesheetBase::ImportedXml {
                tree.node_base_uri(owner)
                    .ok_or("missing imported XML stylesheet base")?
            } else {
                document_url
            };
            resolve_css_resources(css, capability, base)
        })();
        result.stylesheets.push(DocumentStylesheetOccurrence {
            owner,
            source_node,
            host_source: node.source.clone(),
            resources,
        });
    }
    result
}
