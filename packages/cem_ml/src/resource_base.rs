//! Resolve retained declaration source bases without fetching or module-map lookup.

/// Apply XML Base values in ancestor-to-descendant order. The caller retains
/// acquisition provenance separately from this effective resource location.
pub fn resolve_resource_base(document_url: &str, bases: &[String]) -> Result<String, String> {
    let mut base = url::Url::parse(document_url)
        .map_err(|error| format!("invalid declaration document URL: {error}"))?;
    base.set_fragment(None);
    for authored in bases {
        let value = authored.trim();
        if value.is_empty() { continue; }
        let resolved = base.join(value)
            .map_err(|error| format!("invalid declaration xml:base: {error}"))?;
        if resolved.fragment().is_some() {
            return Err("declaration xml:base must not contain a fragment".into());
        }
        base = resolved;
    }
    Ok(base.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_ancestor_chain_against_final_document_url() {
        assert_eq!(resolve_resource_base(
            "https://example.test/release/components.xhtml",
            &["../src/".into(), "components/card/card.xhtml".into(), "".into()],
        ).unwrap(), "https://example.test/src/components/card/card.xhtml");
    }

    #[test]
    fn absolute_override_and_empty_chain_keep_url_semantics() {
        assert_eq!(resolve_resource_base("https://example.test/bundle.xhtml", &[]).unwrap(),
            "https://example.test/bundle.xhtml");
        assert_eq!(resolve_resource_base("https://example.test/bundle.xhtml",
            &["https://cdn.test/card.xhtml?v=2".into()]).unwrap(),
            "https://cdn.test/card.xhtml?v=2");
    }

    #[test]
    fn rejects_invalid_or_fragment_bases_without_fallback() {
        for value in ["http://[", "card.xhtml#fragment", "#fragment"] {
            assert!(resolve_resource_base("https://example.test/bundle.xhtml", &[value.into()]).is_err());
        }
        assert!(resolve_resource_base("relative.xhtml", &[]).is_err());
    }
}
