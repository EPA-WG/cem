//! Resolve references from retained CSS CEM trees without inspecting parser ASTs.
//! This side-effect-free plan authorizes no fetch or browser installation. Hosts
//! must retain the resolution context and apply loader/import policy separately.
use std::sync::Arc;

use crate::{
    module_resolution::{
        CemModuleUrlReferrer, CemModuleUrlResolution, CemModuleUrlResolutionCapability,
        CemModuleUrlResolutionError, CemModuleUrlResolutionPurpose,
    },
    parser::{
        tree::{CemTreeRange, RetainedCemTree},
        AstNodeId,
    },
    schema::registry::CSS_SCHEMA_URI,
    source_map::SourceMapStack,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CssResourceKind {
    Import {
        layer: Option<String>,
        supports: Option<String>,
        media: Option<String>,
    },
    Url,
}

#[derive(Debug)]
pub struct CssResourceReference {
    pub node_id: AstNodeId,
    pub kind: CssResourceKind,
    pub authored_specifier: String,
    pub source: SourceMapStack,
    /// Range of the complete import statement or URL component, in source bytes.
    pub range: CemTreeRange,
    pub resolution: Result<CemModuleUrlResolution, CemModuleUrlResolutionError>,
}

#[derive(Debug)]
pub struct CssResourcePlan {
    /// Native source ownership outlives all reference node IDs and source ranges.
    pub tree: Arc<RetainedCemTree>,
    pub references: Vec<CssResourceReference>,
}

/// Resolve each imported stylesheet or explicit URL component in source order.
/// `stylesheet_url` is the owning template's URL (inline CSS), or the imported
/// sheet's final URL. Never use a synthetic retained-tree source URI as its base.
/// Resolution results are context-specific: do not cache this plan by tree alone.
/// String-valued grammars such as image-set() candidates are not yet covered.
pub fn resolve_css_resources(
    tree: Arc<RetainedCemTree>,
    capability: &CemModuleUrlResolutionCapability,
    stylesheet_url: &str,
) -> Result<CssResourcePlan, String> {
    // An explicit absolute base prevents an accidental bare-referrer map lookup.
    url::Url::parse(stylesheet_url).map_err(|e| format!("invalid CSS stylesheet URL: {e}"))?;
    let document = tree.node(0).ok_or("CSS tree has no document")?;
    if document.children.len() != 1
        || !["stylesheet", "style-block", "style-attribute"]
            .iter()
            .any(|name| named(&tree, document.children[0], name))
    {
        return Err("CSS resource resolution requires a retained CSS document".into());
    }
    let mut pending = document.children.clone();
    let mut references = Vec::new();
    while let Some(id) = pending.pop() {
        let node = tree.node(id).ok_or("CSS tree contains an invalid node")?;
        let reference = if named(&tree, id, "import") {
            Some((
                attribute(&tree, id, "href")
                    .ok_or("CSS import has no href")?
                    .to_owned(),
                CssResourceKind::Import {
                    layer: attribute(&tree, id, "layer").map(str::to_owned),
                    supports: attribute(&tree, id, "supports").map(str::to_owned),
                    media: attribute(&tree, id, "media").map(str::to_owned),
                },
            ))
        } else if named(&tree, id, "component-value") && attribute(&tree, id, "kind") == Some("url")
        {
            Some((
                attribute(&tree, id, "value")
                    .ok_or("CSS URL has no value")?
                    .to_owned(),
                CssResourceKind::Url,
            ))
        } else if named(&tree, id, "function")
            && attribute(&tree, id, "name").is_some_and(|s| s.eq_ignore_ascii_case("url"))
        {
            // Import has already decoded strings and escapes and identified trivia.
            // Recovered bad tokens remain unknown, never ignorable comments.
            let values: Vec<_> = node
                .children
                .iter()
                .copied()
                .filter(|child| {
                    !matches!(
                        attribute(&tree, *child, "kind"),
                        Some("whitespace" | "comment")
                    )
                })
                .collect();
            if values.len() != 1 || attribute(&tree, values[0], "kind") != Some("string") {
                return Err(format!(
                    "CSS url() at byte {} requires one static string",
                    node.range.offset
                ));
            }
            Some((
                attribute(&tree, values[0], "value")
                    .ok_or("CSS URL string has no value")?
                    .to_owned(),
                CssResourceKind::Url,
            ))
        } else {
            None
        };
        if let Some((authored_specifier, kind)) = reference {
            let resolution = capability.resolve_with_referrer(
                CemModuleUrlResolutionPurpose::Css,
                &authored_specifier,
                CemModuleUrlReferrer::Url(stylesheet_url.to_owned()),
                node.source.clone(),
            );
            references.push(CssResourceReference {
                node_id: id,
                kind,
                authored_specifier,
                source: node.source.clone(),
                range: node.range,
                resolution,
            });
        } else {
            pending.extend(node.children.iter().rev().copied());
        }
    }
    Ok(CssResourcePlan { tree, references })
}

pub(crate) fn named(tree: &RetainedCemTree, id: AstNodeId, local: &str) -> bool {
    tree.node(id)
        .and_then(|node| node.name.as_ref())
        .is_some_and(|name| name.namespace_uri == CSS_SCHEMA_URI && name.local_name == local)
}

pub(crate) fn attribute<'a>(
    tree: &'a RetainedCemTree,
    id: AstNodeId,
    local: &str,
) -> Option<&'a str> {
    tree.node(id)?.attributes.iter().find_map(|id| {
        let node = tree.node(*id)?;
        let name = node.name.as_ref()?;
        (name.namespace_uri.is_empty() && name.local_name == local).then_some(node.value.as_str())
    })
}
