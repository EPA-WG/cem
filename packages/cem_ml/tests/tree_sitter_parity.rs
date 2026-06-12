//! Parity check: every canonical `examples/cem-ml/*.cem` fixture must
//! parse equivalently in the Rust pipeline
//! (`CemTokenizer` → `CemEventNormalizer` → `CemAstBuilder`) and in the
//! tree-sitter grammar shipped at `grammar/tree-sitter-cem`.
//!
//! Equivalence is structural: the test reduces each parse to a
//! sequence of [`StructuralEvent`]s — element opens/closes with
//! qualified names, attribute names and values, directive heads,
//! expression-node markers, rich content as text, and presence of
//! non-whitespace text. Whitespace, line/block comments, and
//! source-position metadata are deliberately excluded because they are
//! trivia in both engines.

use cem_ml::events::cem::CemEventNormalizer;
use cem_ml::parser::builder::CemAstBuilder;
use cem_ml::parser::document::CemDocument;
use cem_ml::parser::{AstNodeId, CemAstNode};
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;

use tree_sitter::{Node, Parser};

#[derive(Debug, Clone, PartialEq, Eq)]
enum StructuralEvent {
    Directive(String),
    OpenElement(String),
    OpenAnonymousScope,
    OpenExpressionNode,
    CloseScope,
    Attribute { name: String, value: Option<String> },
    Text,
}

fn fixture_paths() -> Vec<std::path::PathBuf> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/cem-ml");
    let mut out = Vec::new();
    walk_dir(&root, &mut out);
    out.sort();
    out
}

fn walk_dir(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            walk_dir(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("cem") {
            out.push(path);
        }
    }
}

/// Reduce a `CemDocument` to its parity projection.
fn project_rust(doc: &CemDocument) -> Vec<StructuralEvent> {
    let mut out = Vec::new();
    if let Some(CemAstNode::Document { root_children, .. }) = doc.root() {
        for child in root_children {
            visit_rust(doc, *child, &mut out);
        }
    }
    out
}

fn visit_rust(doc: &CemDocument, id: AstNodeId, out: &mut Vec<StructuralEvent>) {
    let Some(node) = doc.get(id) else { return };
    match node {
        CemAstNode::Element {
            expanded_name,
            attributes,
            children,
            ..
        } => {
            let qname = qualified_name(&expanded_name.namespace_uri, &expanded_name.local_name);
            // Directives lower into the event stream as scopes named
            // `@doc` / `@ns` / `@default` / `@schema` whose body is a
            // single Text child. Project them as `Directive(name)` and
            // ignore the body — its segmentation differs across
            // engines and is compared elsewhere.
            if let Some(name) = qname.strip_prefix('@') {
                out.push(StructuralEvent::Directive(name.to_owned()));
                return;
            }
            if qname == "$" {
                out.push(StructuralEvent::OpenExpressionNode);
                out.push(StructuralEvent::CloseScope);
                return;
            }
            if qname.is_empty() {
                out.push(StructuralEvent::OpenAnonymousScope);
            } else {
                out.push(StructuralEvent::OpenElement(qname));
            }
            for a in attributes {
                if let Some(CemAstNode::Attribute {
                    expanded_name,
                    value,
                    ..
                }) = doc.get(*a)
                {
                    let aname =
                        qualified_name(&expanded_name.namespace_uri, &expanded_name.local_name);
                    out.push(StructuralEvent::Attribute {
                        name: aname,
                        value: value.clone(),
                    });
                }
            }
            for c in children {
                visit_rust(doc, *c, out);
            }
            out.push(StructuralEvent::CloseScope);
        }
        CemAstNode::Text { data, .. } => {
            if !data.trim().is_empty() {
                out.push(StructuralEvent::Text);
            }
        }
        CemAstNode::Whitespace { .. } | CemAstNode::Comment { .. } => {}
        CemAstNode::ProcessingInstruction { .. } => {}
        CemAstNode::Cdata { .. } | CemAstNode::RawText { .. } => {
            out.push(StructuralEvent::Text);
        }
        CemAstNode::Document { .. } | CemAstNode::Error { .. } | CemAstNode::Attribute { .. } => {}
    }
}

fn qualified_name(prefix: &str, local: &str) -> String {
    if prefix.is_empty() {
        local.to_owned()
    } else {
        format!("{prefix}:{local}")
    }
}

/// Reduce a tree-sitter parse to its parity projection.
fn project_tree_sitter(source: &str) -> Vec<StructuralEvent> {
    let mut parser = Parser::new();
    let language: tree_sitter::Language = tree_sitter_cem::LANGUAGE.into();
    parser
        .set_language(&language)
        .expect("tree-sitter cem language loads");
    let tree = parser
        .parse(source, None)
        .expect("tree-sitter parses CEM fixture");
    let root = tree.root_node();
    assert!(
        !root.has_error(),
        "tree-sitter reported parse errors in fixture: {}",
        root.to_sexp()
    );
    let mut out = Vec::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        visit_ts(child, source.as_bytes(), false, &mut out);
    }
    out
}

fn visit_ts(node: Node<'_>, src: &[u8], in_content: bool, out: &mut Vec<StructuralEvent>) {
    match node.kind() {
        "directive" => {
            // The `_directive_head` token (`@doc` / `@ns` / `@default`
            // / `@schema`) is hidden, so we read the directive's own
            // span and pull the name out of the source bytes directly.
            let raw = node.utf8_text(src).unwrap_or("");
            let after_at = raw.trim_start().trim_start_matches('@');
            let name = after_at
                .split(|c: char| c.is_ascii_whitespace())
                .next()
                .unwrap_or("")
                .to_owned();
            out.push(StructuralEvent::Directive(name));
        }
        "node" => {
            let name = node
                .child_by_field_name("name")
                .map(|c| c.utf8_text(src).unwrap_or(""))
                .unwrap_or("")
                .to_owned();
            out.push(StructuralEvent::OpenElement(name));
            walk_children(node, src, true, out);
            out.push(StructuralEvent::CloseScope);
        }
        "anonymous_scope" => {
            out.push(StructuralEvent::OpenAnonymousScope);
            walk_children(node, src, true, out);
            out.push(StructuralEvent::CloseScope);
        }
        "expression_node" => {
            out.push(StructuralEvent::OpenExpressionNode);
            out.push(StructuralEvent::CloseScope);
        }
        "attribute" => {
            let aname = node
                .child_by_field_name("name")
                .map(|c| c.utf8_text(src).unwrap_or(""))
                .unwrap_or("")
                .to_owned();
            let value = node
                .child_by_field_name("value")
                .map(|c| attribute_value_text(c, src));
            out.push(StructuralEvent::Attribute { name: aname, value });
        }
        "text" => {
            let raw = node.utf8_text(src).unwrap_or("");
            if !raw.trim().is_empty() {
                out.push(StructuralEvent::Text);
            }
        }
        "rich_content" => {
            out.push(StructuralEvent::Text);
        }
        // Comments inside an element body are text in the Rust pipeline
        // (`scan_content_text` keeps `//` as plain text so URLs survive
        // and `/*` is the only content-text interruption). Tree-sitter
        // captures them as separate comment nodes, so we re-project
        // them as `Text` events when we are inside content. At the
        // document level, comments are trivia in both engines and are
        // dropped.
        "line_comment" | "block_comment" if in_content => {
            out.push(StructuralEvent::Text);
        }
        "content_boundary" | "line_comment" | "block_comment" => {}
        _ => {}
    }
}

fn walk_children(node: Node<'_>, src: &[u8], in_content: bool, out: &mut Vec<StructuralEvent>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_ts(child, src, in_content, out);
    }
}

fn attribute_value_text(node: Node<'_>, src: &[u8]) -> String {
    match node.kind() {
        "bare_value" => node.utf8_text(src).unwrap_or("").to_owned(),
        "quoted_string" => {
            // Strip surrounding quotes to match the Rust event stream,
            // which records the inner scalar value only.
            let raw = node.utf8_text(src).unwrap_or("");
            let mut chars = raw.chars();
            match (chars.next(), raw.chars().last()) {
                (Some(open), Some(close)) if open == close && (open == '"' || open == '\'') => {
                    raw[open.len_utf8()..raw.len() - close.len_utf8()].to_owned()
                }
                _ => raw.to_owned(),
            }
        }
        "cem_ql_span" => node.utf8_text(src).unwrap_or("").to_owned(),
        _ => node.utf8_text(src).unwrap_or("").to_owned(),
    }
}

fn parse_rust(input: &str) -> CemDocument {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tok);
    CemAstBuilder::new(normalizer).build()
}

/// Collapse consecutive `Text` markers into a single event. The two
/// engines disagree on how a run of prose is split (by leading/trailing
/// whitespace, by adjacent comments, by mid-text backticks, …); the
/// parity check only cares whether non-empty text content exists in
/// the same position, not how many slices it is split into.
fn dedupe_text(events: Vec<StructuralEvent>) -> Vec<StructuralEvent> {
    let mut out: Vec<StructuralEvent> = Vec::with_capacity(events.len());
    for ev in events {
        if matches!(ev, StructuralEvent::Text) && matches!(out.last(), Some(StructuralEvent::Text))
        {
            continue;
        }
        out.push(ev);
    }
    out
}

#[test]
fn every_canonical_fixture_parses_equivalently_in_rust_and_tree_sitter() {
    let fixtures = fixture_paths();
    assert!(
        fixtures.len() >= 5,
        "expected at least the five top-level CEM-ML fixtures"
    );
    let mut failures = Vec::new();
    for path in &fixtures {
        let input = std::fs::read_to_string(path).unwrap_or_else(|e| {
            panic!("read fixture {}: {e}", path.display());
        });
        let rust = dedupe_text(project_rust(&parse_rust(&input)));
        let ts = dedupe_text(project_tree_sitter(&input));
        if rust != ts {
            failures.push((path.clone(), rust, ts));
        }
    }
    if !failures.is_empty() {
        let mut msg = String::new();
        for (path, rust, ts) in &failures {
            msg.push_str(&format!(
                "\n--- fixture: {}\n  rust ({} events): {:?}\n  ts   ({} events): {:?}\n",
                path.display(),
                rust.len(),
                rust,
                ts.len(),
                ts
            ));
        }
        panic!("rust/tree-sitter parity mismatches:{msg}");
    }
}
