//! XML reference provenance from the authoritative native import boundary.
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xml_reference_provenance_uses_native_nested_bases_and_source_keys() {
        let bytes = br#"<svg xmlns="http://www.w3.org/2000/svg" xml:base="../assets/"><g xml:base="icons/"><image href="one.svg?x=1&amp;y=2"/></g><image xml:base="https://cdn.test/" href="two.svg"/></svg>"#;
        let result =
            inspect_xml_reference_provenance(bytes, "https://example.test/docs/image.svg", 100)
                .unwrap();
        assert_eq!(result.references.len(), 2);
        assert_eq!(
            result.references[0].resolved_uri.as_ref().unwrap(),
            "https://example.test/assets/icons/one.svg?x=1&y=2"
        );
        assert_eq!(
            result.references[1].resolved_uri.as_ref().unwrap(),
            "https://cdn.test/two.svg"
        );
        for r in &result.references {
            assert!(r.source_key.is_some());
            assert_eq!(
                r.source_uri.as_deref(),
                Some("https://example.test/docs/image.svg")
            );
            assert!(result.document.tree.node(r.attribute).is_some());
        }
        assert!(result.document.body_analysis_required);
        let same =
            inspect_xml_reference_provenance(bytes, "https://example.test/docs/image.svg", 100)
                .unwrap();
        assert_eq!(
            result.references[0].source_key,
            same.references[0].source_key
        );
    }
    #[test]
    fn xml_reference_provenance_binds_exact_sources_without_claiming_editability() {
        let bytes = br#"<svg xmlns="http://www.w3.org/2000/svg"><image href="asset.svg"/></svg>"#;
        let original =
            inspect_xml_reference_provenance(bytes, "https://example.test/a.svg", 100).unwrap();
        let second =
            inspect_xml_reference_provenance(bytes, "https://example.test/a.svg", 100).unwrap();
        assert_eq!(
            bind_reference_source(
                &second.document.tree,
                second.references[0].attribute,
                &original
            )
            .unwrap(),
            0
        );
        let changed = inspect_xml_reference_provenance(
            br#"<svg xmlns="http://www.w3.org/2000/svg"><image href="other.svg"/></svg>"#,
            "https://example.test/a.svg",
            100,
        )
        .unwrap();
        assert!(bind_reference_source(
            &changed.document.tree,
            changed.references[0].attribute,
            &original
        )
        .is_err());
        assert!(bind_reference_source(&second.document.tree, 0, &original).is_err());
        let synthetic = RetainedCemTree::from_shared(
            original.document.tree.ast_owner().clone(),
            "https://example.test/a.svg",
            "",
            Default::default(),
            None,
        )
        .unwrap();
        assert!(
            bind_reference_source(&synthetic, original.references[0].attribute, &original).is_err()
        );
    }
    #[test]
    fn xml_reference_provenance_blocks_html_base_truncation_and_bad_base() {
        let html = br#"<html xmlns="http://www.w3.org/1999/xhtml"><img src="a.svg"/><base href="other/"/></html>"#;
        let result =
            inspect_xml_reference_provenance(html, "https://example.test/page.xhtml", 100).unwrap();
        assert!(result.references.is_empty());
        assert!(result.blockers.iter().any(|b| b.contains("html-base")));
        let result =
            inspect_xml_reference_provenance(html, "https://example.test/page.xhtml", 1).unwrap();
        assert!(result.references.is_empty());
        let malformed = br#"<svg xmlns="http://www.w3.org/2000/svg" xml:base="http://["><image href="a.svg"/></svg>"#;
        let result =
            inspect_xml_reference_provenance(malformed, "https://example.test/a.svg", 100).unwrap();
        assert!(result.references[0].resolved_uri.is_err());
        assert!(inspect_xml_reference_provenance(html, "relative.xhtml", 100).is_err());
    }
}

use crate::{
    document_dependencies::{inspect_document_dependencies, DocumentDependencyInspection},
    parser::{tree::RetainedCemTree, AstNodeId},
};

pub struct XmlReferenceProvenance {
    pub attribute: AstNodeId,
    pub effective_base: Option<String>,
    pub resolved_uri: Result<String, String>,
    pub source_uri: Option<String>,
    /// Opaque native selection identity, never a writable byte range.
    pub source_key: Option<String>,
}
pub struct XmlReferenceInventory {
    pub document: DocumentDependencyInspection,
    pub references: Vec<XmlReferenceProvenance>,
    pub blockers: Vec<String>,
}
/// Import exact XML bytes at their final acquired URL, then inspect native base
/// and source provenance. URL candidates are not acquisition grants or edit plans.
/// HTML base elements require a separate host contract even in XHTML input.
pub fn inspect_xml_reference_provenance(
    bytes: &[u8],
    document_url: &str,
    budget: usize,
) -> Result<XmlReferenceInventory, String> {
    url::Url::parse(document_url).map_err(|_| "absolute XML document URL required")?;
    let tree = crate::import::import_data_bytes(bytes, "application/xml", "cem", document_url)?;
    let document = inspect_document_dependencies(tree.clone(), budget);
    let mut result = XmlReferenceInventory {
        document,
        references: Vec::new(),
        blockers: vec!["resource-policy-and-source-edit-ownership-required".into()],
    };
    if let Err(reason) = complete_xml_base_inventory(&result.document) {
        result.blockers.push(reason);
        return Ok(result);
    }
    for reference in &result.document.references {
        let base = tree.node_base_uri(reference.owner);
        let resolved_uri =
            resolve_imported_reference(&tree, reference.owner, &reference.authored_uri);
        result.references.push(XmlReferenceProvenance {
            attribute: reference.attribute,
            effective_base: base.map(str::to_owned),
            resolved_uri,
            source_uri: tree.node_source_uri(reference.attribute).map(str::to_owned),
            source_key: tree.source_key(reference.attribute),
        });
    }
    Ok(result)
}
/// Used only after importing XML bytes at the acquired URL. A complete first pass
/// prevents a later HTML base from being missed before candidate resolution.
pub(crate) fn complete_xml_base_inventory(
    document: &DocumentDependencyInspection,
) -> Result<(), String> {
    if document.blockers.iter().any(|b| {
        b.contains("budget") || b.contains("invalid-retained") || b == "document-import-diagnostics"
    }) {
        return Err("complete-XML-document-inventory-required".into());
    }
    let mut pending = vec![0];
    while let Some(id) = pending.pop() {
        let node = document.tree.node(id).ok_or("invalid XML node")?;
        if node.name.as_ref().is_some_and(|n| {
            n.namespace_uri == "http://www.w3.org/1999/xhtml" && n.local_name == "base"
        }) {
            return Err("html-base-execution-contract-required".into());
        }
        pending.extend(node.children.iter().rev().copied());
    }
    Ok(())
}
fn resolve_imported_reference(
    tree: &RetainedCemTree,
    owner: AstNodeId,
    authored: &str,
) -> Result<String, String> {
    let base = tree
        .node_base_uri(owner)
        .ok_or("missing imported XML base")?;
    let base = url::Url::parse(base).map_err(|_| "invalid imported XML base")?;
    base.join(authored.trim())
        .map(|url| url.to_string())
        .map_err(|_| "unresolved XML reference".into())
}

/// Correlate a retained output/reference attribute with an original imported slot.
/// The returned index selects `source.references`. It is correspondence evidence
/// only: source keys do not authorize replacing a lexical byte range or editing a
/// shared source occurrence. Hosts still need source revision and mutation checks.
pub fn bind_reference_source(
    output: &RetainedCemTree,
    attribute: AstNodeId,
    source: &XmlReferenceInventory,
) -> Result<usize, String> {
    use crate::parser::tree::CemTreeNodeKind;
    let node = output.node(attribute).ok_or("missing output attribute")?;
    if node.kind != CemTreeNodeKind::Attribute {
        return Err("output must be an attribute".into());
    }
    let key = output
        .source_key(attribute)
        .ok_or("output has no native source key")?;
    let uri = output
        .node_source_uri(attribute)
        .ok_or("output has no source URI")?;
    let mut matched = None;
    for (index, candidate) in source.references.iter().enumerate() {
        if candidate.source_key.as_deref() != Some(&key)
            || candidate.source_uri.as_deref() != Some(uri)
        {
            continue;
        }
        let original = source
            .document
            .tree
            .node(candidate.attribute)
            .ok_or("source attribute no longer exists")?;
        if source
            .document
            .tree
            .source_key(candidate.attribute)
            .as_deref()
            != Some(&key)
            || source.document.tree.node_source_uri(candidate.attribute) != Some(uri)
        {
            return Err("source inventory provenance mismatch".into());
        }
        if original.kind != CemTreeNodeKind::Attribute
            || original
                .name
                .as_ref()
                .map(|n| (&n.namespace_uri, &n.local_name, n.schema_id))
                != node
                    .name
                    .as_ref()
                    .map(|n| (&n.namespace_uri, &n.local_name, n.schema_id))
            || original.value != node.value
        {
            return Err("source attribute identity or value changed".into());
        }
        if matched.replace(index).is_some() {
            return Err("ambiguous source occurrence".into());
        }
    }
    matched.ok_or_else(|| "no matching imported source slot".into())
}
