//! Native retained-document reference candidates. No fetch, execution or rewrite.
use crate::import::srcset::{
    import_srcset_attribute, srcset_slot, RetainedSrcsetAttribute, SrcsetLimits,
};
use crate::parser::{tree::RetainedCemTree, AstNodeId};
use crate::source_map::SourceMapStack;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentReferenceRole {
    Resource,
    Navigation,
    Base,
}
#[derive(Debug, Clone)]
pub struct DocumentReference {
    pub owner: AstNodeId,
    pub attribute: AstNodeId,
    pub role: DocumentReferenceRole,
    pub authored_uri: String,
    pub source: SourceMapStack,
}
pub struct DocumentDependencyInspection {
    pub tree: Arc<RetainedCemTree>,
    pub references: Vec<DocumentReference>,
    pub srcsets: Vec<RetainedSrcsetAttribute>,
    pub presentations: Vec<crate::import::svg_presentation::RetainedSvgPresentationAttribute>,
    pub blockers: Vec<String>,
    pub body_analysis_required: bool,
}
/// Inventory native decoded HTML/SVG direct URI slots and HTML image candidates.
/// Work charges nodes, attributes and decoded srcset bytes against one budget.
/// The retained owner keeps node IDs/source maps alive. This does not resolve bases,
/// apply module maps, execute templates or claim a complete document dependency set.
pub fn inspect_document_dependencies(
    tree: Arc<RetainedCemTree>,
    budget: usize,
) -> DocumentDependencyInspection {
    use crate::parser::tree::CemTreeNodeKind;
    use crate::validation::{
        html::{html_direct_resource_slot, html_uri_requires_policy},
        svg::{svg_direct_resource_reference_requires_policy, svg_direct_resource_slot},
    };
    const HTML: &str = "http://www.w3.org/1999/xhtml";
    const SVG: &str = "http://www.w3.org/2000/svg";
    let mut references = Vec::new();
    let mut srcsets = Vec::new();
    let mut presentations = Vec::new();
    let mut blockers = Vec::new();
    let mut remaining = budget;
    let mut pending = vec![0];
    'nodes: while let Some(id) = pending.pop() {
        if !charge(&mut remaining, &mut blockers) {
            break;
        }
        let Some(node) = tree.node(id) else {
            block(&mut blockers, "invalid-retained-node");
            continue;
        };
        if node.children.len() > remaining {
            block(&mut blockers, "inspection-budget-exceeded");
        }
        pending.extend(node.children.iter().take(remaining).rev().copied());
        match node.kind {
            CemTreeNodeKind::ProcessingInstruction | CemTreeNodeKind::Reference => block(
                &mut blockers,
                "runtime-or-processing-instruction-analysis-required",
            ),
            CemTreeNodeKind::Element => {
                let Some(name) = &node.name else {
                    block(&mut blockers, "missing-element-name");
                    continue;
                };
                let namespace = name.namespace_uri.as_str();
                let local = name.local_name.as_str();
                if namespace == SVG
                    && matches!(
                        local,
                        "animate" | "animateMotion" | "animateTransform" | "set" | "discard"
                    )
                {
                    block(&mut blockers, "svg-animation-runtime-analysis-required");
                }
                if !matches!(namespace, HTML | SVG) {
                    block(&mut blockers, "unknown-document-vocabulary");
                }
                if matches!(namespace, HTML | SVG) {
                    if local == "script" {
                        block(&mut blockers, "script-runtime-analysis-required");
                    }
                    if local == "style" {
                        block(&mut blockers, "embedded-css-analysis-required");
                    }
                    if matches!(
                        local,
                        "template" | "foreignObject" | "meta" | "object" | "embed"
                    ) {
                        block(&mut blockers, "embedded-document-runtime-analysis-required");
                    }
                    if namespace == HTML && local.contains('-') {
                        block(&mut blockers, "custom-element-analysis-required");
                    }
                }
                for attribute in &node.attributes {
                    if !charge(&mut remaining, &mut blockers) {
                        break 'nodes;
                    }
                    let Some(attr) = tree.node(*attribute) else {
                        block(&mut blockers, "invalid-retained-attribute");
                        continue;
                    };
                    let Some(attr_name) = &attr.name else {
                        block(&mut blockers, "missing-attribute-name");
                        continue;
                    };
                    let attr_ns = attr_name.namespace_uri.as_str();
                    let field = attr_name.local_name.as_str();
                    if attr_ns == "http://www.w3.org/XML/1998/namespace" && field == "base" {
                        block(&mut blockers, "document-base-context-required");
                    }
                    let supported_attribute = attr_ns.is_empty()
                        || (namespace == SVG
                            && attr_ns == "http://www.w3.org/1999/xlink"
                            && field == "href");
                    if !supported_attribute {
                        if !matches!(
                            attr_ns,
                            "http://www.w3.org/XML/1998/namespace"
                                | "http://www.w3.org/2000/xmlns/"
                        ) {
                            block(&mut blockers, "foreign-attribute-vocabulary");
                        }
                        continue;
                    }
                    if field == "style" {
                        block(&mut blockers, "embedded-css-analysis-required");
                    }
                    if matches!(field, "srcset" | "imagesrcset") {
                        if srcset_slot(namespace, local, attr_ns, field) {
                            let defaults = SrcsetLimits::default();
                            let limits = SrcsetLimits {
                                max_bytes: remaining.min(defaults.max_bytes),
                                max_candidates: defaults.max_candidates,
                            };
                            match import_srcset_attribute(tree.clone(), id, *attribute, limits) {
                                Ok(srcset) => {
                                    remaining -= srcset.value().len();
                                    if !srcset.parsed().complete {
                                        block(&mut blockers, "srcset-analysis-required");
                                        block(&mut blockers, "srcset-numeric-profile-incomplete");
                                    }
                                    if !srcset.parsed().issues.is_empty() {
                                        block(&mut blockers, "srcset-parse-errors");
                                    }
                                    if !srcset.parsed().candidates.is_empty() {
                                        block(&mut blockers, "srcset-base-and-policy-required");
                                        block(
                                            &mut blockers,
                                            "srcset-selection-and-load-history-required",
                                        );
                                    }
                                    srcsets.push(srcset);
                                }
                                Err(_) => {
                                    block(&mut blockers, "inspection-budget-exceeded");
                                    break 'nodes;
                                }
                            }
                        } else {
                            block(&mut blockers, "srcset-analysis-required");
                        }
                    }
                    if field.starts_with("on") {
                        block(&mut blockers, "event-runtime-analysis-required");
                    }
                    // SVG presentation values can contain CSS resource references.
                    // Do not parse CSS syntax by searching document strings.
                    if namespace == SVG
                        && attr_ns.is_empty()
                        && crate::import::svg_presentation::presentation_slot(local, field)
                    {
                        if attr.value.len() > remaining {
                            block(&mut blockers, "inspection-budget-exceeded");
                            break 'nodes;
                        }
                        remaining -= attr.value.len();
                        match crate::import::svg_presentation::import_svg_presentation_attribute(
                            tree.clone(),
                            id,
                            *attribute,
                            attr.value.len(),
                        ) {
                            Ok(source) => {
                                presentations.push(source);
                                block(&mut blockers, "svg-presentation-base-and-policy-required");
                                block(&mut blockers, "svg-presentation-runtime-analysis-required");
                            }
                            Err(_) => block(&mut blockers, "svg-presentation-analysis-incomplete"),
                        }
                    } else if namespace == SVG && matches!(field, "color-profile" | "marker") {
                        block(&mut blockers, "svg-presentation-analysis-required");
                    }
                    let role = if namespace == HTML
                        && html_direct_resource_slot(local, field)
                        && html_uri_requires_policy(&attr.value)
                    {
                        Some(if local == "base" && field == "href" {
                            block(&mut blockers, "document-base-context-required");
                            DocumentReferenceRole::Base
                        } else {
                            DocumentReferenceRole::Resource
                        })
                    } else if namespace == HTML
                        && matches!(local, "a" | "area")
                        && field == "href"
                        && html_uri_requires_policy(&attr.value)
                    {
                        Some(DocumentReferenceRole::Navigation)
                    } else if namespace == SVG
                        && svg_direct_resource_slot(field)
                        && svg_direct_resource_reference_requires_policy(&attr.value)
                    {
                        Some(DocumentReferenceRole::Resource)
                    } else {
                        None
                    };
                    if let Some(role) = role {
                        block(&mut blockers, "reference-base-and-policy-required");
                        references.push(DocumentReference {
                            owner: id,
                            attribute: *attribute,
                            role,
                            authored_uri: attr.value.clone(),
                            source: attr.source.clone(),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    if tree
        .ast_owner()
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation())
    {
        block(&mut blockers, "document-import-diagnostics");
    }
    DocumentDependencyInspection {
        tree,
        references,
        srcsets,
        presentations,
        blockers,
        body_analysis_required: true,
    }
}
fn block(blockers: &mut Vec<String>, reason: &str) {
    if !blockers.iter().any(|item| item == reason) {
        blockers.push(reason.into());
    }
}
fn charge(remaining: &mut usize, blockers: &mut Vec<String>) -> bool {
    if *remaining == 0 {
        block(blockers, "inspection-budget-exceeded");
        false
    } else {
        *remaining -= 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tree(source: &str) -> Arc<RetainedCemTree> {
        crate::import::import_data_bytes(
            source.as_bytes(),
            "application/xml",
            "cem",
            "studio://p/document.xhtml",
        )
        .unwrap()
    }
    #[test]
    fn document_dependencies_keep_decoded_slots_and_native_owners() {
        let tree = tree(
            r#"<html xmlns="http://www.w3.org/1999/xhtml"><img src="images/a.svg?x=1&amp;y=2"/><a href="next.xhtml">Next</a><link href="theme.css"/></html>"#,
        );
        let result = inspect_document_dependencies(tree.clone(), 100);
        assert!(Arc::ptr_eq(&tree, &result.tree));
        assert_eq!(result.references.len(), 3);
        assert_eq!(result.references[0].authored_uri, "images/a.svg?x=1&y=2");
        assert_eq!(result.references[1].role, DocumentReferenceRole::Navigation);
        assert!(result
            .references
            .iter()
            .all(|r| !r.source.frames.is_empty() && result.tree.node(r.attribute).is_some()));
        assert!(result.body_analysis_required);
        assert!(result
            .blockers
            .iter()
            .any(|b| b.contains("base-and-policy")));
    }
    #[test]
    fn document_dependencies_respect_namespace_and_local_references() {
        let result = inspect_document_dependencies(
            tree(
                r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><use href="#local"/><image xlink:href="asset.svg"/><image href="data:image/svg+xml,empty"/><foreign xmlns="urn:custom" src="not-an-owned-slot"/></svg>"##,
            ),
            100,
        );
        assert_eq!(result.references.len(), 1);
        assert_eq!(result.references[0].authored_uri, "asset.svg");
        assert!(result.blockers.iter().any(|b| b.contains("vocabulary")));
    }
    #[test]
    fn document_dependencies_keep_base_css_script_and_selector_followups() {
        let result = inspect_document_dependencies(
            tree(
                r#"<html xmlns="http://www.w3.org/1999/xhtml" xml:base="../"><base href="other/"/><script src="app.js"/><style>body { background: url(image.svg) }</style><img srcset="a.svg 1x, b.svg 2x" style="background: red" onclick="go()"/><cem-loader src="data.xml"/></html>"#,
            ),
            100,
        );
        assert!(result
            .references
            .iter()
            .any(|r| r.role == DocumentReferenceRole::Base));
        for word in ["base-context", "runtime", "css", "srcset", "custom-element"] {
            assert!(
                result.blockers.iter().any(|b| b.contains(word)),
                "{word}: {:?}",
                result.blockers
            );
        }
    }
    #[test]
    fn document_dependencies_budget_and_unknown_xml_do_not_certify_leaves() {
        let result = inspect_document_dependencies(tree("<data><src>hidden.xml</src></data>"), 100);
        assert!(result.references.is_empty());
        assert!(result.body_analysis_required);
        assert!(result.blockers.iter().any(|b| b.contains("vocabulary")));
        let result = inspect_document_dependencies(
            tree(r#"<img xmlns="http://www.w3.org/1999/xhtml" src="a.svg"/>"#),
            1,
        );
        assert!(result.blockers.iter().any(|b| b.contains("budget")));
    }
}
