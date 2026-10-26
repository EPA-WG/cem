//! Import the registered HTML parser's decoded events into the shared CEM tree.
//! HTML mirrors stay literal attributes; application mappings are explicit queries.
use super::{html_range, ImportBuilder, MAX_DEPTH};
use crate::{
    parser::{tree::CemTreeSemantics, CemAstNode},
    validation::html::{HtmlDocumentAst, HtmlEventKind, HtmlSchemaContractCatalog},
};

pub(super) fn validate(document: &HtmlDocumentAst, max_values: usize) -> Result<(), String> {
    let catalog = HtmlSchemaContractCatalog::from_builtin();
    if let Some(fact) = document.facts.iter().find(|fact| {
        catalog.severity_for_fact(fact.kind).is_hard_violation()
            && !document.defers_import_policy_fact(fact.kind)
    }) {
        return Err(fact.message.clone());
    }
    let nodes = document.events.iter().try_fold(1usize, |count, event| {
        let depth = event.depth + usize::from(event.kind == HtmlEventKind::StartElement);
        if depth > MAX_DEPTH {
            return Err("HTML exceeds the 64-level import limit.");
        }
        let count = count
            .saturating_add(1)
            .saturating_add(event.attributes.len());
        if count > max_values {
            return Err("HTML exceeds the document node import limit.");
        }
        Ok(count)
    });
    nodes.map(|_| ()).map_err(str::to_owned)
}

pub(super) fn project(
    document: &HtmlDocumentAst,
) -> Result<(crate::parser::document::CemDocument, CemTreeSemantics), String> {
    let (ast, semantics, _) = project_tracked(document)?;
    Ok((ast, semantics))
}

pub(super) fn project_tracked(
    document: &HtmlDocumentAst,
) -> Result<(crate::parser::document::CemDocument, CemTreeSemantics, Vec<Option<u32>>), String> {
    let mut event_nodes = vec![None; document.events.len()];
    let mut builder = ImportBuilder::new();
    let mut semantics = CemTreeSemantics::default();
    let mut stack = vec![0];
    for (index, event) in document.events.iter().enumerate() {
        // Native event depths already describe closing-tag recovery. Do not
        // reconstruct a second HTML parsing policy from tag names here.
        if event.depth + 1 > stack.len() {
            return Err("Invalid HTML event depth.".into());
        }
        stack.truncate(event.depth + 1);
        let parent = *stack.last().ok_or("Unbalanced HTML event stream.")?;
        let source = event.source_range.source_map();
        let range = html_range(event.source_range);
        let id = match event.kind {
            HtmlEventKind::Doctype => continue,
            HtmlEventKind::StartElement => {
                let id = builder.element(
                    parent,
                    &event.namespace_uri,
                    event
                        .local_name
                        .as_deref()
                        .ok_or("HTML element name missing.")?,
                    source.clone(),
                );
                for attribute in event
                    .attributes
                    .iter()
                    .filter(|attribute| !attribute.duplicate)
                {
                    let aid = builder.ast.nodes.len() as u32;
                    // HTML foreign attributes use their registered expanded names;
                    // ordinary HTML data-* and colon names remain literal.
                    let (namespace, name) = if event.namespace_uri != "http://www.w3.org/1999/xhtml"
                    {
                        match attribute.local_name.split_once(':') {
                            Some(("xlink", local)) => ("http://www.w3.org/1999/xlink", local),
                            Some(("xml", local)) => ("http://www.w3.org/XML/1998/namespace", local),
                            Some(("xmlns", local)) => ("http://www.w3.org/2000/xmlns/", local),
                            _ if attribute.local_name == "xmlns" => {
                                ("http://www.w3.org/2000/xmlns/", "xmlns")
                            }
                            _ => ("", attribute.local_name.as_str()),
                        }
                    } else {
                        ("", attribute.local_name.as_str())
                    };
                    let value = attribute.value.clone().unwrap_or_default();
                    builder.attribute(
                        id,
                        namespace,
                        name,
                        value.clone(),
                        attribute.source_range.source_map(),
                    );
                    semantics.values.insert(aid, value);
                    semantics
                        .ranges
                        .insert(aid, html_range(attribute.source_range));
                    if namespace == "http://www.w3.org/2000/xmlns/" {
                        semantics.omitted.insert(aid);
                    }
                }
                if !event.void_element && !event.self_closing {
                    stack.push(id);
                }
                id
            }
            HtmlEventKind::EndElement => {
                continue;
            }
            HtmlEventKind::Text | HtmlEventKind::Rcdata | HtmlEventKind::RawText => {
                let id = builder.ast.nodes.len() as u32;
                let value = event.value.clone().unwrap_or_default();
                // Retain lexical text for inspection, decoded values for queries.
                let node = if event.kind == HtmlEventKind::RawText {
                    CemAstNode::RawText {
                        node_id: id,
                        data: event.lexeme.clone(),
                        source: source.clone(),
                    }
                } else if event.whitespace_only {
                    CemAstNode::Whitespace {
                        node_id: id,
                        data: event.lexeme.clone(),
                        source: source.clone(),
                    }
                } else {
                    CemAstNode::Text {
                        node_id: id,
                        data: event.lexeme.clone(),
                        source: source.clone(),
                    }
                };
                builder.push(parent, node);
                semantics.values.insert(id, value);
                id
            }
            HtmlEventKind::Comment => {
                let id = builder.ast.nodes.len() as u32;
                builder.push(
                    parent,
                    CemAstNode::Comment {
                        node_id: id,
                        data: event.value.clone().unwrap_or_default(),
                        source: source.clone(),
                    },
                );
                id
            }
        };
        event_nodes[index] = Some(id);
        semantics.sources.insert(id, source);
        semantics.ranges.insert(id, range);
    }
    // Imported HTML never evaluates expressions, applies styles or runs scripts.
    Ok((builder.ast, semantics, event_nodes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::html::{
        html_document_ast_from_passive_projection, html_document_ast_from_source_bytes,
        HtmlFactKind, HtmlSourceValidationRequest,
    };

    #[test]
    fn passive_projection_preserves_facts_without_granting_authored_source_admission() {
        let request = HtmlSourceValidationRequest {
            bytes: b"<main><img src='../images/harbor.svg'><script src='https://example.test/no-fetch.js'>neverRun()</script></main>",
            source_uri: "memory:generated.html",
            content_type: Some("text/html"),
        };
        let (source, diagnostics) = html_document_ast_from_source_bytes(request);
        assert!(diagnostics
            .iter()
            .any(|item| item.severity.is_hard_violation()));
        let mut source = source.unwrap();
        assert!(validate(&source, 4096).is_err());
        // Source-map annotations are provenance, never an admission grant.
        source.origin_spans = Some(crate::source_map::SourceMapIndex::new(vec![]));
        assert!(validate(&source, 4096).is_err());

        let (projection, diagnostics) = html_document_ast_from_passive_projection(request);
        assert!(!diagnostics
            .iter()
            .any(|item| item.severity.is_hard_violation()));
        let projection = projection.unwrap();
        for kind in [
            HtmlFactKind::ExternalResourceRejected,
            HtmlFactKind::ScriptRejected,
        ] {
            assert!(projection.facts.iter().any(|fact| fact.kind == kind));
        }
        validate(&projection, 4096).unwrap();
        validate(&projection.clone(), 4096).unwrap();
        project(&projection).unwrap();
        // Reparse of exported bytes starts as strict source again.
        let reparsed = html_document_ast_from_source_bytes(request).0.unwrap();
        assert!(validate(&reparsed, 4096).is_err());
    }

    #[test]
    fn passive_projection_keeps_unrelated_hard_policy_and_structural_bounds() {
        for source in [
            "<img src='../image.svg' onload='neverRun()'>".to_owned(),
            format!("{}{}", "<div>".repeat(66), "</div>".repeat(66)),
            "<input>".repeat(4100),
        ] {
            let (projection, _) =
                html_document_ast_from_passive_projection(HtmlSourceValidationRequest {
                    bytes: source.as_bytes(),
                    source_uri: "memory:bounded-projection.html",
                    content_type: Some("text/html"),
                });
            assert!(validate(&projection.unwrap(), 4096).is_err());
        }
    }
}
