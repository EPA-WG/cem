//! Explicit HTML execution bases over retained, decoded CEM snapshots.

use crate::parser::{
    tree::{CemTreeNodeKind, RetainedCemTree},
    AstNodeId,
};
use crate::source_map::SourceMapStack;
use std::sync::Arc;
use url::Url;

/// Host-owned fallback at the time the selected base is frozen (or at inspection
/// when no href base exists). Never inferred from an import/source-map URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlFallbackBase {
    DocumentUrl(String),
    Srcdoc {
        about_base_url: String,
    },
    AboutBlank {
        document_url: String,
        about_base_url: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlBaseEncoding {
    Utf8,
    /// Unsupported encodings fail explicitly, including non-UTF-8 query encoding.
    Other(String),
}

#[derive(Debug, Clone)]
pub struct HtmlBaseContext {
    pub fallback: HtmlFallbackBase,
    pub encoding: HtmlBaseEncoding,
    /// Identity of the host policy applied at this freeze point, not a grant.
    pub policy_stamp: String,
}

#[derive(Debug, Clone, Copy)]
pub struct HtmlBaseLimits {
    /// Counts document/element/other nodes, attributes and import diagnostics.
    pub nodes: usize,
    /// Maximum UTF-8 bytes in each input or serialized URL.
    pub url_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlBasePolicyDecision {
    Allow,
    Deny,
}

pub struct HtmlBasePolicyRequest<'a> {
    pub fallback_url: &'a Url,
    pub candidate_url: &'a Url,
    pub selected: &'a HtmlBaseSelection,
    pub policy_stamp: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlBaseOutcome {
    NoHrefBase,
    Resolved,
    InvalidHref,
    ForbiddenScheme,
    PolicyDenied,
}

#[derive(Debug)]
pub struct HtmlBaseSelection {
    pub element: AstNodeId,
    pub attribute: AstNodeId,
    pub authored_href: String,
    pub source_uri: Option<String>,
    pub source_key: Option<String>,
    pub source: SourceMapStack,
}

#[derive(Debug)]
pub struct HtmlBaseResolution {
    pub tree: Arc<RetainedCemTree>,
    pub context: HtmlBaseContext,
    pub fallback_url: String,
    pub effective_url: String,
    pub selected: Option<HtmlBaseSelection>,
    pub outcome: HtmlBaseOutcome,
    /// Current URL semantics cannot establish earlier resource loads or edit safety.
    pub body_analysis_required: bool,
}

/// Resolve a fresh HTML base freeze from a retained UTF-8 document snapshot.
///
/// The caller owns the snapshot's correspondence to the execution document, the
/// fallback at the freeze point and the policy decision (including CSP base-uri).
/// This is not a text/html parser or a reconstruction of an arbitrary live DOM's
/// frozen-base history. Names/values must already be decoded by native import.
/// XML base chains are deliberately ignored under this explicit HTML profile.
///
/// The first HTML-namespace base with an unqualified href wins, even if invalid
/// or denied. target-only bases do not participate. Full bounded inspection must
/// succeed before policy runs. Opaque references and template children whose
/// document-vs-content ownership is not represented by the retained tree fail.
/// Hosts may supply a document tree with inert template contents omitted.
///
/// Results are candidate URL evidence only; no fetch, rewrite or edit admission.
pub fn resolve_html_base_snapshot(
    tree: Arc<RetainedCemTree>,
    context: HtmlBaseContext,
    limits: HtmlBaseLimits,
    decide: impl FnOnce(&HtmlBasePolicyRequest<'_>) -> Result<HtmlBasePolicyDecision, String>,
) -> Result<HtmlBaseResolution, String> {
    const HTML: &str = "http://www.w3.org/1999/xhtml";
    if context.encoding != HtmlBaseEncoding::Utf8 {
        return Err("HTML base resolution requires an explicit UTF-8 execution profile".into());
    }
    if context.policy_stamp.trim().is_empty() {
        return Err("HTML base policy identity required".into());
    }
    let fallback = fallback_url(&context.fallback, limits.url_bytes)?;
    let mut remaining = limits.nodes;
    for diagnostic in &tree.ast_owner().diagnostics {
        charge(&mut remaining)?;
        if diagnostic.severity.is_hard_violation() {
            return Err("HTML base snapshot has hard import diagnostics".into());
        }
    }
    let mut pending = vec![0];
    let mut selected = None;
    while let Some(id) = pending.pop() {
        charge(&mut remaining)?;
        let node = tree.node(id).ok_or("invalid retained HTML base node")?;
        if node.kind == CemTreeNodeKind::Reference {
            return Err("HTML base snapshot requires expanded document ownership".into());
        }
        let name = node.name.as_ref();
        if node.kind == CemTreeNodeKind::Element && name.is_none() {
            return Err("HTML base snapshot requires expanded element names".into());
        }
        let html_name = name
            .filter(|n| n.namespace_uri == HTML)
            .map(|n| n.local_name.as_str());
        if html_name == Some("template") && !node.children.is_empty() {
            return Err(
                "HTML template content ownership must be separated from document children".into(),
            );
        }
        for &attribute in &node.attributes {
            charge(&mut remaining)?;
            let attr = tree
                .node(attribute)
                .ok_or("invalid retained HTML base attribute")?;
            let attr_name = attr
                .name
                .as_ref()
                .ok_or("HTML base attribute name required")?;
            if attr.kind != CemTreeNodeKind::Attribute || attr.parent != Some(id) {
                return Err("invalid retained HTML base attribute ownership".into());
            }
            if tree
                .attribute_value_nodes(attribute)
                .is_some_and(|values| !values.is_empty())
            {
                return Err("HTML base snapshot requires materialized attribute values".into());
            }
            if selected.is_none()
                && html_name == Some("base")
                && attr_name.namespace_uri.is_empty()
                && attr_name.local_name == "href"
            {
                check_url_size(&attr.value, limits.url_bytes)?;
                selected = Some(HtmlBaseSelection {
                    element: id,
                    attribute,
                    authored_href: attr.value.clone(),
                    source_uri: tree.node_source_uri(attribute).map(str::to_owned),
                    source_key: tree.source_key(attribute),
                    source: attr.source.clone(),
                });
            }
        }
        // Reserve at least one unit for every queued node before growing the stack.
        if node.children.len() > remaining.saturating_sub(pending.len()) {
            return Err("HTML base inspection budget exceeded".into());
        }
        pending.extend(node.children.iter().rev().copied());
    }
    let mut effective = fallback.clone();
    let outcome = if let Some(selection) = &selected {
        // URL parsing owns ASCII control/space handling. Unicode trim() would
        // incorrectly remove authored non-ASCII spaces and change URL semantics.
        match fallback.join(&selection.authored_href) {
            Err(_) => HtmlBaseOutcome::InvalidHref,
            Ok(candidate) => {
                check_url_size(candidate.as_str(), limits.url_bytes)?;
                if matches!(candidate.scheme(), "data" | "javascript") {
                    HtmlBaseOutcome::ForbiddenScheme
                } else {
                    match decide(&HtmlBasePolicyRequest {
                        fallback_url: &fallback,
                        candidate_url: &candidate,
                        selected: selection,
                        policy_stamp: &context.policy_stamp,
                    })? {
                        HtmlBasePolicyDecision::Deny => HtmlBaseOutcome::PolicyDenied,
                        HtmlBasePolicyDecision::Allow => {
                            effective = candidate;
                            HtmlBaseOutcome::Resolved
                        }
                    }
                }
            }
        }
    } else {
        HtmlBaseOutcome::NoHrefBase
    };
    Ok(HtmlBaseResolution {
        tree,
        context,
        fallback_url: fallback.into(),
        effective_url: effective.into(),
        selected,
        outcome,
        body_analysis_required: true,
    })
}

fn charge(remaining: &mut usize) -> Result<(), String> {
    *remaining = remaining
        .checked_sub(1)
        .ok_or("HTML base inspection budget exceeded")?;
    Ok(())
}
fn check_url_size(value: &str, limit: usize) -> Result<(), String> {
    if value.len() > limit {
        Err("HTML base URL byte budget exceeded".into())
    } else {
        Ok(())
    }
}
fn parse_absolute(value: &str, limit: usize) -> Result<Url, String> {
    check_url_size(value, limit)?;
    let url = Url::parse(value).map_err(|_| "absolute HTML fallback URL required")?;
    check_url_size(url.as_str(), limit)?;
    Ok(url)
}
fn fallback_url(context: &HtmlFallbackBase, limit: usize) -> Result<Url, String> {
    match context {
        HtmlFallbackBase::DocumentUrl(value) => {
            let url = parse_absolute(value, limit)?;
            if url.scheme() == "about"
                && url.host().is_none()
                && (url.path() == "blank" || (url.path() == "srcdoc" && url.query().is_none()))
            {
                return Err(
                    "embedded HTML documents require an explicit about-base context".into(),
                );
            }
            Ok(url)
        }
        HtmlFallbackBase::Srcdoc { about_base_url } => parse_absolute(about_base_url, limit),
        HtmlFallbackBase::AboutBlank {
            document_url,
            about_base_url,
        } => {
            let document = parse_absolute(document_url, limit)?;
            if document.scheme() != "about"
                || document.host().is_some()
                || document.path() != "blank"
            {
                return Err("about:blank context requires an about:blank document URL".into());
            }
            match about_base_url {
                Some(value) => parse_absolute(value, limit),
                None => Ok(document),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::import_data_bytes;
    use std::cell::Cell;

    fn tree(source: &str) -> Arc<RetainedCemTree> {
        import_data_bytes(
            source.as_bytes(),
            "application/xml",
            "cem",
            "studio://source/page.xhtml",
        )
        .unwrap()
    }
    fn context() -> HtmlBaseContext {
        HtmlBaseContext {
            fallback: HtmlFallbackBase::DocumentUrl("https://example.test/docs/page.html".into()),
            encoding: HtmlBaseEncoding::Utf8,
            policy_stamp: "policy:1".into(),
        }
    }
    fn limits() -> HtmlBaseLimits {
        HtmlBaseLimits {
            nodes: 100,
            url_bytes: 4096,
        }
    }
    fn allow(_: &HtmlBasePolicyRequest<'_>) -> Result<HtmlBasePolicyDecision, String> {
        Ok(HtmlBasePolicyDecision::Allow)
    }

    #[test]
    fn html_base_first_href_keeps_native_owner_and_ignores_imported_xml_base() {
        let tree = tree(
            r#"<html xmlns="http://www.w3.org/1999/xhtml" xml:base="https://wrong.test/"><head><base target="_blank"/><base href="../assets/?a=1&amp;b=2"/><base href="https://later.test/"/></head></html>"#,
        );
        let calls = Cell::new(0);
        let result = resolve_html_base_snapshot(tree.clone(), context(), limits(), |request| {
            calls.set(calls.get() + 1);
            assert_eq!(
                request.fallback_url.as_str(),
                "https://example.test/docs/page.html"
            );
            assert_eq!(
                request.candidate_url.as_str(),
                "https://example.test/assets/?a=1&b=2"
            );
            assert_eq!(request.policy_stamp, "policy:1");
            allow(request)
        })
        .unwrap();
        assert_eq!(calls.get(), 1);
        assert!(Arc::ptr_eq(&result.tree, &tree));
        assert_eq!(result.effective_url, "https://example.test/assets/?a=1&b=2");
        assert_eq!(result.outcome, HtmlBaseOutcome::Resolved);
        assert!(result.body_analysis_required);
        let selected = result.selected.unwrap();
        assert_eq!(selected.authored_href, "../assets/?a=1&b=2");
        assert_eq!(
            selected.source_uri.as_deref(),
            Some("studio://source/page.xhtml")
        );
        assert_eq!(selected.source_key, tree.source_key(selected.attribute));
        assert!(selected.source_key.is_some());
        assert!(!selected.source.frames.is_empty());
        assert_eq!(
            tree.node(selected.attribute).unwrap().parent,
            Some(selected.element)
        );
    }

    #[test]
    fn html_base_invalid_and_forbidden_first_href_never_use_later_bases_or_policy() {
        for (href, outcome) in [
            ("http://[", HtmlBaseOutcome::InvalidHref),
            ("DaTa:text/plain,base", HtmlBaseOutcome::ForbiddenScheme),
            ("java&#x9;script:alert(1)", HtmlBaseOutcome::ForbiddenScheme),
        ] {
            let tree = tree(&format!(
                r#"<html xmlns="http://www.w3.org/1999/xhtml"><base href="{href}"/><base href="https://later.test/"/></html>"#
            ));
            let result = resolve_html_base_snapshot(tree, context(), limits(), |_| {
                panic!("policy must not run")
            })
            .unwrap();
            assert_eq!(result.effective_url, result.fallback_url);
            assert_eq!(result.outcome, outcome);
            assert!(result.selected.is_some());
        }
    }

    #[test]
    fn html_base_policy_denial_falls_back_and_unknown_policy_fails() {
        let tree = tree(
            r#"<html xmlns="http://www.w3.org/1999/xhtml"><base href="https://cdn.test/"/><base href="https://later.test/"/></html>"#,
        );
        let result = resolve_html_base_snapshot(tree.clone(), context(), limits(), |_| {
            Ok(HtmlBasePolicyDecision::Deny)
        })
        .unwrap();
        assert_eq!(result.outcome, HtmlBaseOutcome::PolicyDenied);
        assert_eq!(result.effective_url, result.fallback_url);
        assert!(
            resolve_html_base_snapshot(tree, context(), limits(), |_| Err(
                "policy unavailable".into()
            ))
            .unwrap_err()
            .contains("policy unavailable")
        );
    }

    #[test]
    fn html_base_namespace_and_ambiguous_template_content_ownership() {
        let retained = tree(
            r#"<html xmlns="http://www.w3.org/1999/xhtml" xmlns:f="urn:foreign"><head><template></template><f:base href="https://foreign.test/"/><base f:href="https://foreign-attr.test/" target="_blank"/></head><body><base href=""/></body></html>"#,
        );
        let result = resolve_html_base_snapshot(retained, context(), limits(), allow).unwrap();
        assert_eq!(result.outcome, HtmlBaseOutcome::Resolved);
        assert_eq!(result.selected.unwrap().authored_href, "");
        assert_eq!(result.effective_url, result.fallback_url);
        let ambiguous = tree(
            r#"<html xmlns="http://www.w3.org/1999/xhtml"><template><base href="https://inert.test/"/></template></html>"#,
        );
        assert!(
            resolve_html_base_snapshot(ambiguous, context(), limits(), |_| panic!(
                "ambiguous ownership"
            ))
            .unwrap_err()
            .contains("template content ownership")
        );
    }

    #[test]
    fn html_base_fallback_contexts_are_explicit_and_utf8_only() {
        let tree =
            tree(r#"<html xmlns="http://www.w3.org/1999/xhtml"><base target="_blank"/></html>"#);
        for (fallback, expected) in [
            (
                HtmlFallbackBase::Srcdoc {
                    about_base_url: "https://parent.test/dir/".into(),
                },
                "https://parent.test/dir/",
            ),
            (
                HtmlFallbackBase::AboutBlank {
                    document_url: "about:blank".into(),
                    about_base_url: Some("https://creator.test/".into()),
                },
                "https://creator.test/",
            ),
            (
                HtmlFallbackBase::AboutBlank {
                    document_url: "about:blank?x#y".into(),
                    about_base_url: None,
                },
                "about:blank?x#y",
            ),
        ] {
            let result = resolve_html_base_snapshot(
                tree.clone(),
                HtmlBaseContext {
                    fallback,
                    ..context()
                },
                limits(),
                |_| panic!("no href base"),
            )
            .unwrap();
            assert_eq!(result.effective_url, expected);
            assert_eq!(result.outcome, HtmlBaseOutcome::NoHrefBase);
            assert!(result.selected.is_none());
        }
        for url in ["relative.html", "", "about:srcdoc", "about:blank"] {
            assert!(resolve_html_base_snapshot(
                tree.clone(),
                HtmlBaseContext {
                    fallback: HtmlFallbackBase::DocumentUrl(url.into()),
                    ..context()
                },
                limits(),
                allow
            )
            .is_err());
        }
        assert!(resolve_html_base_snapshot(
            tree.clone(),
            HtmlBaseContext {
                encoding: HtmlBaseEncoding::Other("windows-1252".into()),
                ..context()
            },
            limits(),
            allow
        )
        .is_err());
        assert!(resolve_html_base_snapshot(
            tree,
            HtmlBaseContext {
                policy_stamp: "".into(),
                ..context()
            },
            limits(),
            allow
        )
        .is_err());
    }

    #[test]
    fn html_base_complete_bounded_scan_precedes_policy_and_resolution() {
        let retained = tree(
            r#"<html xmlns="http://www.w3.org/1999/xhtml"><base href="/chosen/"/><body><div><p>late</p></div></body></html>"#,
        );
        assert!(resolve_html_base_snapshot(
            retained.clone(),
            context(),
            HtmlBaseLimits {
                nodes: 7,
                ..limits()
            },
            |_| panic!("incomplete inventory")
        )
        .unwrap_err()
        .contains("budget"));
        assert!(resolve_html_base_snapshot(
            retained,
            context(),
            HtmlBaseLimits {
                url_bytes: 2,
                ..limits()
            },
            |_| panic!("URL budget")
        )
        .unwrap_err()
        .contains("budget"));
        let late = tree(
            r#"<html xmlns="http://www.w3.org/1999/xhtml"><body><div>before</div><base href="late/"/></body></html>"#,
        );
        assert_eq!(
            resolve_html_base_snapshot(late, context(), limits(), allow)
                .unwrap()
                .effective_url,
            "https://example.test/docs/late/"
        );
    }

    #[test]
    fn html_base_unknown_nodes_and_hard_diagnostics_are_not_empty_documents() {
        use crate::parser::{document::CemDocument, CemAstNode};
        for opaque in [true, false] {
            let mut ast = CemDocument::default();
            ast.nodes.push(CemAstNode::Document {
                node_id: 0,
                root_children: if opaque { vec![1] } else { vec![] },
                source: Default::default(),
            });
            if opaque {
                ast.nodes.push(CemAstNode::Reference {
                    node_id: 1,
                    expression: "$output".into(),
                    context: 0,
                    targets: None,
                    source: Default::default(),
                });
            } else {
                ast.diagnostics.push(crate::diagnostics::Diagnostic {
                    severity: crate::diagnostics::Severity::Error,
                    ..Default::default()
                });
            }
            let retained =
                RetainedCemTree::new(ast, "studio://synthetic", "", Default::default(), None)
                    .unwrap();
            let error = resolve_html_base_snapshot(retained, context(), limits(), |_| {
                panic!("incomplete ownership")
            })
            .unwrap_err();
            assert!(error.contains(if opaque {
                "expanded document ownership"
            } else {
                "hard import diagnostics"
            }));
        }
    }

    #[test]
    fn html_base_href_size_and_serialized_size_are_bounded_without_fallback() {
        for href in ["x".repeat(101), "é".repeat(30)] {
            let retained = tree(&format!(
                r#"<html xmlns="http://www.w3.org/1999/xhtml"><base href="{href}"/></html>"#
            ));
            let error = resolve_html_base_snapshot(
                retained,
                context(),
                HtmlBaseLimits {
                    url_bytes: 100,
                    ..limits()
                },
                |_| panic!("oversized URL"),
            )
            .unwrap_err();
            assert!(error.contains("byte budget"));
        }
    }
}
