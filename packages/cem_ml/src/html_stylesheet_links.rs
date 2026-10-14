//! Retained HTML link occurrences. Selection/installation are observed separately.
use crate::{
    css_import_snapshot::{
        canonical_url, capture_css_import_snapshot, CssSnapshotCaptureLimits, CssSnapshotSource,
        CssSourceSnapshot, SnapshotCssImportClosure,
    },
    module_resolution::*,
    parser::{tree::RetainedCemTree, AstNodeId},
    scheduler::AbortSignal,
    source_map::SourceMapStack,
};
use std::sync::Arc;
#[derive(Clone)]
pub struct HtmlStylesheetLink {
    pub owner: AstNodeId,
    pub href_attribute: Option<AstNodeId>,
    pub href: String,
    pub media: String,
    pub title: String,
    pub alternate: bool,
    pub disabled: bool,
    pub content_type: String,
    pub integrity: String,
    pub source: SourceMapStack,
}
pub struct HtmlLinkCssImports {
    pub link: HtmlStylesheetLink,
    pub source: Arc<CssSnapshotSource>,
    pub closure: SnapshotCssImportClosure,
}
pub(crate) fn attribute<'a>(
    tree: &'a RetainedCemTree,
    owner: AstNodeId,
    name: &str,
) -> Option<(AstNodeId, &'a str)> {
    tree.node(owner)?.attributes.iter().find_map(|id| {
        let node = tree.node(*id)?;
        node.name
            .as_ref()
            .filter(|n| n.namespace_uri.is_empty() && n.local_name == name)
            .map(|_| (*id, node.value.as_str()))
    })
}
pub(crate) fn collect(
    tree: &RetainedCemTree,
    budget: usize,
) -> Result<Vec<HtmlStylesheetLink>, String> {
    let mut pending = vec![0];
    let mut remaining = budget;
    let mut result = vec![];
    while let Some(id) = pending.pop() {
        remaining = remaining
            .checked_sub(1)
            .ok_or("stylesheet link inventory budget exceeded")?;
        let node = tree.node(id).ok_or("invalid stylesheet link node")?;
        let html = node
            .name
            .as_ref()
            .is_some_and(|n| n.namespace_uri == "http://www.w3.org/1999/xhtml");
        if html
            && node
                .name
                .as_ref()
                .is_some_and(|n| n.local_name == "template")
        {
            continue;
        }
        if node.children.len() > remaining {
            return Err("stylesheet link inventory budget exceeded".into());
        }
        pending.extend(node.children.iter().rev().copied());
        if !html || !node.name.as_ref().is_some_and(|n| n.local_name == "link") {
            continue;
        }
        let get = |name| attribute(tree, id, name).map(|(_, v)| v).unwrap_or("");
        remaining = remaining
            .checked_sub(node.attributes.len())
            .and_then(|n| n.checked_sub(get("rel").len()))
            .ok_or("stylesheet link attribute budget exceeded")?;
        let rel: Vec<_> = get("rel")
            .split(|c| matches!(c, '\t' | '\n' | '\u{c}' | '\r' | ' '))
            .filter(|v| !v.is_empty())
            .collect();
        if !rel.iter().any(|v| v.eq_ignore_ascii_case("stylesheet")) {
            continue;
        }
        let bytes = ["href", "media", "title", "type", "integrity"]
            .iter()
            .try_fold(0usize, |n, field| n.checked_add(get(field).len()))
            .ok_or("stylesheet link metadata count overflow")?;
        remaining = remaining
            .checked_sub(bytes)
            .ok_or("stylesheet link metadata budget exceeded")?;
        let link = HtmlStylesheetLink {
            owner: id,
            href_attribute: attribute(tree, id, "href").map(|(id, _)| id),
            href: get("href").into(),
            media: get("media").into(),
            title: get("title").into(),
            alternate: rel.iter().any(|v| v.eq_ignore_ascii_case("alternate")),
            disabled: attribute(tree, id, "disabled").is_some(),
            content_type: get("type").into(),
            integrity: get("integrity").into(),
            source: attribute(tree, id, "href")
                .and_then(|(id, _)| tree.node(id))
                .map(|node| node.source.clone())
                .unwrap_or_else(|| node.source.clone()),
        };
        result.push(link);
    }
    Ok(result)
}
pub(crate) fn capture(
    link: &HtmlStylesheetLink,
    base: &str,
    capability: &CemModuleUrlResolutionCapability,
    snapshot: Arc<CssSourceSnapshot>,
    limits: CssSnapshotCaptureLimits,
    abort: AbortSignal,
) -> Result<HtmlLinkCssImports, String> {
    if abort.is_aborted() || limits.imports.max_sheets == 0 {
        return Err("stylesheet link capture cancelled".into());
    }
    if link.href_attribute.is_none()
        || link
            .href
            .trim_matches(|c: char| c.is_ascii_whitespace())
            .is_empty()
        || (link.alternate && link.title.is_empty())
    {
        return Err("stylesheet link requires href and alternate title".into());
    }
    if !link.content_type.is_empty()
        && crate::schema::registry::content_type_essence(&link.content_type) != "text/css"
    {
        return Err("unsupported stylesheet link type".into());
    }
    let requested = canonical_url(
        url::Url::parse(base)
            .map_err(|e| e.to_string())?
            .join(&link.href)
            .map_err(|e| e.to_string())?
            .as_str(),
    )?;
    let check = |url: &str, referrer: &str| -> Result<CemModuleUrlResolution, String> {
        let r = capability
            .resolve_with_referrer(
                CemModuleUrlResolutionPurpose::Css,
                url,
                CemModuleUrlReferrer::Url(referrer.into()),
                link.source.clone(),
            )
            .map_err(|e| e.to_string())?;
        if canonical_url(&r.resolved_url)? != url {
            return Err("HTML stylesheet URLs cannot be remapped".into());
        }
        Ok(r)
    };
    let resolution = check(&requested, base)?;
    let source = snapshot
        .source(&requested)
        .ok_or("stylesheet response absent from immutable snapshot")?
        .clone();
    if source.bytes.len()
        > limits
            .responses
            .max_response_bytes
            .min(crate::import::MAX_DOCUMENT_BYTES)
        || source.bytes.len() > limits.responses.max_total_bytes
    {
        return Err(
            "cem.css.import_byte_limit: external stylesheet response limit exceeded".into(),
        );
    }
    check(&source.final_url, &requested)?;
    if !link.integrity.is_empty() {
        crate::resource_integrity::verify_resource_integrity(&source.bytes, &link.integrity)?;
    }
    let (root, bytes) = crate::css_imports::import_css_response(
        &crate::resolver::ResolvedRead {
            uri: source.final_url.clone(),
            bytes: source.bytes.clone(),
            content_type: Some(source.content_type.clone()),
        },
        &resolution,
        &limits.responses,
        0,
    )
    .map_err(|e| e.to_string())?;
    let mut remaining = limits;
    remaining.responses.max_total_bytes -= bytes;
    let closure = capture_css_import_snapshot(
        root,
        &source.final_url,
        capability,
        snapshot,
        remaining,
        abort,
    )
    .map_err(|e| e.to_string())?;
    Ok(HtmlLinkCssImports {
        link: link.clone(),
        source,
        closure,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn document_inert_template_links_are_not_roots() {
        let tree=crate::import::import_data_bytes(b"<html xmlns='http://www.w3.org/1999/xhtml'><template><link rel='stylesheet' href='inert.css'/></template></html>","application/xml","cem","urn:test").unwrap();
        assert!(super::collect(&tree, 1000).unwrap().is_empty());
        assert!(super::collect(&tree, 1).is_err());
    }
}
