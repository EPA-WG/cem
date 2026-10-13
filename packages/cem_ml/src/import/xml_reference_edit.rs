//! Candidate edits of native-import-owned XML reference values.
#[cfg(test)]
mod tests {
    use super::*;
    const URI: &str = "https://example.test/docs/a.svg";
    fn key(bytes: &[u8]) -> String {
        crate::document_reference_provenance::inspect_xml_reference_provenance(bytes, URI, 100)
            .unwrap()
            .references[0]
            .source_key
            .clone()
            .unwrap()
    }
    #[test]
    fn xml_reference_edit_preserves_quotes_entities_and_other_bytes() {
        let bytes = br#"<svg xmlns="http://www.w3.org/2000/svg" xml:base="../assets/">
  <!-- keep --> <image href = 'old.svg?x=1&amp;y=2'/><image href="stay.svg"/>
</svg>"#;
        let plan = plan_xml_reference_edit(
            bytes,
            URI,
            &key(bytes),
            "https://cdn.test/new.svg?x=3&y=4",
            100,
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(plan.rewritten_bytes).unwrap(),
            String::from_utf8(bytes.to_vec()).unwrap().replace(
                "old.svg?x=1&amp;y=2",
                "https://cdn.test/new.svg?x=3&amp;y=4"
            )
        );
        assert!(plan.body_analysis_required);
        assert!(!plan.blockers.is_empty());
    }
    #[test]
    fn xml_reference_edit_from_retained_occurrence_requires_source_binding() {
        let bytes = br#"<svg xmlns="http://www.w3.org/2000/svg"><image href="old.svg"/></svg>"#;
        let source =
            crate::document_reference_provenance::inspect_xml_reference_provenance(bytes, URI, 100)
                .unwrap();
        let plan = plan_bound_xml_reference_edit(
            &source.document.tree,
            source.references[0].attribute,
            bytes,
            URI,
            "https://example.test/new.svg",
            100,
        )
        .unwrap();
        assert_ne!(plan.before_hash, plan.after_hash);
        assert!(plan_bound_xml_reference_edit(
            &source.document.tree,
            0,
            bytes,
            URI,
            "https://example.test/new.svg",
            100
        )
        .is_err());
        let bad = br#"<svg xmlns="http://www.w3.org/2000/svg" xml:base="http://["><image href="old.svg"/></svg>"#;
        assert!(
            plan_xml_reference_edit(bad, URI, &key(bad), "https://example.test/new.svg", 100)
                .is_err()
        );
    }
    #[test]
    fn xml_reference_edit_noop_preserves_exact_authored_bytes() {
        let bytes =
            br#"<svg xmlns="http://www.w3.org/2000/svg"><image href="../old.svg?x=&#49;"/></svg>"#;
        let plan = plan_xml_reference_edit(
            bytes,
            URI,
            &key(bytes),
            "https://example.test/old.svg?x=1",
            100,
        )
        .unwrap();
        assert_eq!(plan.rewritten_bytes, bytes);
    }
    #[test]
    fn xml_reference_edit_rejects_stale_keys_bases_and_nonabsolute_targets() {
        let bytes = br#"<svg xmlns="http://www.w3.org/2000/svg"><image href="old.svg"/></svg>"#;
        let old_key = key(bytes);
        let mut changed = bytes.to_vec();
        changed.push(b' ');
        assert!(plan_xml_reference_edit(
            &changed,
            URI,
            &old_key,
            "https://example.test/new.svg",
            100
        )
        .is_err());
        assert!(plan_xml_reference_edit(bytes, URI, &old_key, "new.svg", 100).is_err());
        assert!(
            plan_xml_reference_edit(bytes, URI, &old_key, "https://example.test/new.svg", 1)
                .is_err()
        );
    }
}

use crate::document_reference_provenance::{
    inspect_xml_reference_provenance, XmlReferenceInventory,
};

pub struct XmlReferenceEditPlan {
    pub source_uri: String,
    pub before_hash: String,
    pub after_hash: String,
    pub source_key: String,
    pub rewritten_bytes: Vec<u8>,
    pub blockers: Vec<String>,
    pub body_analysis_required: bool,
}
/// Plan one exact source occurrence. No file is written and no resource is loaded.
/// The source key binds acquisition URI, original bytes and native attribute owner.
/// Imported XML lexical coordinates are consumed only inside this import module.
/// Generated occurrence sharing, host revisions and complete dependency closure
/// must be admitted separately before any candidate is committed.
pub fn plan_xml_reference_edit(
    bytes: &[u8],
    document_url: &str,
    source_key: &str,
    to_uri: &str,
    budget: usize,
) -> Result<XmlReferenceEditPlan, String> {
    let target = url::Url::parse(to_uri)
        .map_err(|_| "absolute target URL required")?
        .to_string();
    let before = inspect_xml_reference_provenance(bytes, document_url, budget)?;
    crate::document_reference_provenance::complete_xml_base_inventory(&before.document)?;
    let matches: Vec<_> = before
        .references
        .iter()
        .enumerate()
        .filter(|(_, r)| r.source_key.as_deref() == Some(source_key))
        .collect();
    let [(selected, reference)] = matches.as_slice() else {
        return Err("source key must select exactly one original reference".into());
    };
    let selected = *selected;
    // A candidate cannot conceal unresolved neighboring references.
    for r in &before.references {
        r.resolved_uri.as_ref().map_err(|e| e.clone())?;
    }
    let mut rewritten_bytes = bytes.to_vec();
    if reference.resolved_uri.as_ref().unwrap() != &target {
        let native = before
            .document
            .tree
            .native_owner()
            .and_then(|o| o.downcast_ref::<crate::lifecycle::LoadedInputAstStream>())
            .ok_or("native XML owner required")?;
        let crate::lifecycle::LoadedInputAstStream::XmlDocument(document) = native else {
            return Err("XML source owner required".into());
        };
        let imported = super::import_xml_ast(document)?;
        let mut selected_range = None;
        for (event_index, ids) in imported.attribute_nodes.iter().enumerate() {
            for (attr_index, id) in ids.iter().enumerate() {
                if *id != reference.attribute {
                    continue;
                }
                if selected_range.is_some() {
                    return Err("ambiguous lexical owner".into());
                }
                let attr = document
                    .events
                    .get(event_index)
                    .and_then(|e| e.attributes.get(attr_index))
                    .ok_or("missing lexical attribute")?;
                let node = before
                    .document
                    .tree
                    .node(*id)
                    .ok_or("missing retained attribute")?;
                if attr.entity_decoded_value.as_deref() != Some(node.value.as_str()) {
                    return Err("lexical/semantic value mismatch".into());
                }
                let range = attr
                    .value_source_range
                    .ok_or("attribute has no lexical value range")?;
                let start =
                    usize::try_from(range.start.byte_offset).map_err(|_| "invalid value offset")?;
                let length =
                    usize::try_from(range.byte_length).map_err(|_| "invalid value length")?;
                let end = start.checked_add(length).ok_or("invalid value range")?;
                if start == 0 || bytes.get(start..end) != Some(attr.value.as_bytes()) {
                    return Err("source bytes disagree with lexical owner".into());
                }
                let quote = *bytes.get(start - 1).ok_or("missing value quote")?;
                if !matches!(quote, b'\'' | b'"') || bytes.get(end) != Some(&quote) {
                    return Err("quoted XML value required".into());
                }
                selected_range = Some((start, end));
            }
        }
        let (start, end) = selected_range.ok_or("source attribute has no lexical owner")?;
        // Escape both quote forms, preserving the original enclosing delimiter.
        let escaped = target
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;");
        rewritten_bytes.splice(start..end, escaped.bytes());
    }
    let after = inspect_xml_reference_provenance(&rewritten_bytes, document_url, budget)?;
    verify_targets(&before, &after, selected, &target)?;
    let hash = |bytes: &[u8]| crate::content_cache::ContentHash::from_blake3(bytes).header_value();
    Ok(XmlReferenceEditPlan {
        source_uri: document_url.into(),
        before_hash: hash(bytes),
        after_hash: hash(&rewritten_bytes),
        source_key: source_key.into(),
        rewritten_bytes,
        blockers: vec![
            "runtime-dependency-closure-required".into(),
            "generated-occurrence-ownership-required".into(),
            "host-policy-revision-and-transaction-required".into(),
        ],
        body_analysis_required: true,
    })
}
fn verify_targets(
    before: &XmlReferenceInventory,
    after: &XmlReferenceInventory,
    selected: usize,
    target: &str,
) -> Result<(), String> {
    crate::document_reference_provenance::complete_xml_base_inventory(&after.document)?;
    if before.references.len() != after.references.len() {
        return Err("reference inventory changed".into());
    }
    for (index, (old, new)) in before.references.iter().zip(&after.references).enumerate() {
        if old.attribute != new.attribute || old.effective_base != new.effective_base {
            return Err("reference ownership or base changed".into());
        }
        let expected = if index == selected {
            target
        } else {
            old.resolved_uri.as_ref().map_err(|e| e.clone())?
        };
        if new.resolved_uri.as_deref() != Ok(expected) {
            return Err("rewritten reference target mismatch".into());
        }
    }
    Ok(())
}

/// Bridge a retained occurrence to its original imported reference before planning.
/// This proves native source correspondence only; multiple produced occurrences can
/// share a source and therefore still require a host-owned edit-scope decision.
pub fn plan_bound_xml_reference_edit(
    output: &crate::parser::tree::RetainedCemTree,
    attribute: crate::parser::AstNodeId,
    bytes: &[u8],
    document_url: &str,
    to_uri: &str,
    budget: usize,
) -> Result<XmlReferenceEditPlan, String> {
    let source = inspect_xml_reference_provenance(bytes, document_url, budget)?;
    let index =
        crate::document_reference_provenance::bind_reference_source(output, attribute, &source)?;
    let key = source.references[index]
        .source_key
        .as_deref()
        .ok_or("source key required")?;
    plan_xml_reference_edit(bytes, document_url, key, to_uri, budget)
}
