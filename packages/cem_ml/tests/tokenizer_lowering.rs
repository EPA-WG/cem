//! AC-F-9 / AC-P-1 / AC-P-8 tokenizer-lowering coverage.
//!
//! Token-level and event-level shape tests live alongside their layers
//! in `src/tokenizer/cem.rs` and `src/events/cem.rs`. This fixture runs
//! the full pipeline (bytes -> tokenizer -> normalizer -> AST builder)
//! and asserts the source-preserving AST + diagnostic surface for each
//! construct the AC list calls out:
//!
//!   1. `{name @attributes | content...}` element form
//!   2. `{$ ...}` expression nodes
//!   3. Anonymous typed scopes  —  `{@type="..." | ... }`
//!   4. Comments (line `//` and block `/* */`)
//!   5. Rich-content enclosures  —  triple-backtick fences
//!   6. Rejection of bare `{...}` text interpolation in content (the
//!      explicit `$` expression node is required).

use cem_ml::diagnostics::Diagnostic;
use cem_ml::events::cem::CemEventNormalizer;
use cem_ml::parser::builder::CemAstBuilder;
use cem_ml::parser::document::CemDocument;
use cem_ml::parser::CemAstNode;
use cem_ml::query;
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;

fn parse(input: &str) -> CemDocument {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tok);
    CemAstBuilder::new(normalizer).build()
}

fn assert_no_hard_violations(doc: &CemDocument) {
    let hard: Vec<&Diagnostic> = doc
        .diagnostics
        .iter()
        .filter(|d| d.severity.is_hard_violation())
        .collect();
    assert!(
        hard.is_empty(),
        "unexpected hard-violation diagnostics: {hard:?}"
    );
}

fn text_children(doc: &CemDocument, element: &CemAstNode) -> Vec<String> {
    let children = match element {
        CemAstNode::Element { children, .. } => children.clone(),
        _ => return Vec::new(),
    };
    children
        .iter()
        .filter_map(|id| match doc.get(*id) {
            Some(CemAstNode::Text { data, .. }) => Some(data.clone()),
            _ => None,
        })
        .collect()
}

fn comment_data(doc: &CemDocument) -> Vec<String> {
    doc.iter()
        .filter_map(|n| match n {
            CemAstNode::Comment { data, .. } => Some(data.clone()),
            _ => None,
        })
        .collect()
}

fn attribute_names(doc: &CemDocument, element: &CemAstNode) -> Vec<String> {
    let attrs = match element {
        CemAstNode::Element { attributes, .. } => attributes.clone(),
        _ => return Vec::new(),
    };
    attrs
        .iter()
        .filter_map(|id| match doc.get(*id) {
            Some(CemAstNode::Attribute { expanded_name, .. }) => {
                Some(expanded_name.local_name.clone())
            }
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 1. {name @attributes | content...}
// ---------------------------------------------------------------------------

#[test]
fn bare_node_lowers_to_a_named_element() {
    let doc = parse("{p}");
    assert_no_hard_violations(&doc);
    let p = query::find_by_local_name(&doc, "p")
        .next()
        .expect("bare {p} lowers to an Element named p");
    let CemAstNode::Element {
        attributes,
        children,
        has_explicit_boundary,
        ..
    } = p
    else {
        panic!("p was not an Element");
    };
    assert!(attributes.is_empty(), "bare node carries no attributes");
    assert!(children.is_empty(), "bare node carries no content");
    assert!(
        !has_explicit_boundary,
        "no `|` in source -> no explicit boundary"
    );
}

#[test]
fn node_with_attributes_and_content_lowers_with_explicit_boundary() {
    let doc = parse(r#"{field @name=email @label="Email" | edit me}"#);
    assert_no_hard_violations(&doc);
    let field = query::find_by_local_name(&doc, "field")
        .next()
        .expect("field element");
    let names = attribute_names(&doc, field);
    assert!(names.contains(&"name".to_owned()));
    assert!(names.contains(&"label".to_owned()));

    let text = text_children(&doc, field);
    assert_eq!(text, vec!["edit me".to_owned()]);

    let CemAstNode::Element {
        has_explicit_boundary,
        ..
    } = field
    else {
        unreachable!()
    };
    assert!(
        *has_explicit_boundary,
        "`|` in source must set has_explicit_boundary=true"
    );
}

#[test]
fn nested_nodes_lower_to_nested_elements() {
    let doc = parse("{a | {b | inner}}");
    assert_no_hard_violations(&doc);
    let a = query::find_by_local_name(&doc, "a")
        .next()
        .expect("outer a");
    let CemAstNode::Element { children, .. } = a else {
        unreachable!()
    };
    let inner_b = children.iter().any(|id| {
        matches!(
            doc.get(*id),
            Some(CemAstNode::Element { expanded_name, .. }) if expanded_name.local_name == "b"
        )
    });
    assert!(inner_b, "a must contain a nested b element");
}

#[test]
fn namespaced_node_and_attribute_preserve_prefixes() {
    let doc = parse(r#"{html:p @html:class="lead" | hi}"#);
    assert_no_hard_violations(&doc);
    let p = query::find_by_local_name(&doc, "p")
        .next()
        .expect("html:p element");
    let CemAstNode::Element {
        expanded_name,
        attributes,
        ..
    } = p
    else {
        unreachable!()
    };
    assert_eq!(expanded_name.local_name, "p");

    let attr_local = attributes
        .iter()
        .find_map(|id| match doc.get(*id) {
            Some(CemAstNode::Attribute { expanded_name, .. }) => Some(&expanded_name.local_name),
            _ => None,
        })
        .expect("the html:class attribute");
    assert_eq!(attr_local, "class");
}

// ---------------------------------------------------------------------------
// 2. {$ ...} expression nodes
// ---------------------------------------------------------------------------

#[test]
fn dollar_expression_node_with_boundary_lowers_to_dollar_element_with_text_body() {
    let doc = parse("{$ | .name}");
    assert_no_hard_violations(&doc);
    let expr = query::find_by_local_name(&doc, "$")
        .next()
        .expect("$ expression element");
    let text = text_children(&doc, expr);
    assert_eq!(text, vec![".name".to_owned()]);
}

#[test]
fn dollar_expression_node_without_boundary_lowers_inline_body() {
    let doc = parse("{$ count(.items)}");
    assert_no_hard_violations(&doc);
    let expr = query::find_by_local_name(&doc, "$")
        .next()
        .expect("$ expression element");
    let text = text_children(&doc, expr);
    assert_eq!(text, vec!["count(.items)".to_owned()]);
}

// ---------------------------------------------------------------------------
// 3. Anonymous typed scopes — {@type="..." | ...}
// ---------------------------------------------------------------------------

#[test]
fn anonymous_typed_scope_lowers_to_an_unnamed_element_with_content() {
    let doc = parse(r#"{@type="text/cem-ml" | inner text}"#);
    assert_no_hard_violations(&doc);

    // Anonymous scopes lower as elements with an empty local name —
    // not as a `cem:scope` host node. The schema machine treats them
    // as parser/policy boundaries (per `events/cem.rs::AnonymousScopeStart`).
    let anon = doc
        .iter()
        .find(|n| {
            matches!(
                n,
                CemAstNode::Element { expanded_name, .. } if expanded_name.local_name.is_empty()
            )
        })
        .expect("anonymous typed scope lowers to an empty-named Element");
    let text = text_children(&doc, anon);
    assert_eq!(text, vec!["inner text".to_owned()]);
}

// ---------------------------------------------------------------------------
// 4. Comments (line and block)
// ---------------------------------------------------------------------------

#[test]
fn line_comment_outside_content_is_preserved_as_comment_node() {
    let doc = parse("// leading note\n{p | x}");
    assert_no_hard_violations(&doc);
    let comments = comment_data(&doc);
    assert_eq!(
        comments.len(),
        1,
        "expected exactly one Comment node, got {comments:?}"
    );
}

#[test]
fn block_comment_inside_content_is_preserved_as_comment_node() {
    let doc = parse("{p | /* inline */ visible}");
    assert_no_hard_violations(&doc);
    let comments = comment_data(&doc);
    assert_eq!(comments.len(), 1, "got {comments:?}");

    // Surrounding visible text remains in the AST.
    let p = query::find_by_local_name(&doc, "p").next().unwrap();
    let text = text_children(&doc, p).join(" ");
    assert!(
        text.contains("visible"),
        "text after a block comment must remain in the AST, got `{text}`"
    );
}

// ---------------------------------------------------------------------------
// 5. Rich-content enclosures (``` ... ```)
// ---------------------------------------------------------------------------

#[test]
fn rich_content_enclosure_is_preserved_verbatim_as_text() {
    // The triple-backtick body becomes a Text child of the host node;
    // its angle brackets and inner `{x}` are not re-tokenized.
    let doc = parse("{code | ```<div>{x}</div>```}");
    assert_no_hard_violations(&doc);
    let code = query::find_by_local_name(&doc, "code")
        .next()
        .expect("code element");
    let text = text_children(&doc, code);
    assert_eq!(text, vec!["<div>{x}</div>".to_owned()]);
}

// ---------------------------------------------------------------------------
// 6. Rejection of bare {...} text interpolation
// ---------------------------------------------------------------------------

#[test]
fn bare_brace_text_interpolation_is_rejected_with_canonical_error_node() {
    // AC-T-7 prohibits bare `{.name}` in content — the explicit `$`
    // expression node is required. The tokenizer raises the canonical
    // code so the rejection reaches the AST as an Error node;
    // downstream consumers see the failure node-locally.
    //
    // The tokenizer's internal `Diagnostic` queue is not drained into
    // `CemDocument.diagnostics` today; routing tokenizer diagnostics
    // through the report tree is tracked separately (the AST Error
    // node is the canonical AST-level surface for parse-time
    // rejection).
    let doc = parse("{p Hello {.name}}");
    let has_error_node = doc.iter().any(|n| {
        matches!(
            n,
            CemAstNode::Error { code, .. } if code == "cem.tokenizer.bare_brace_text"
        )
    });
    assert!(
        has_error_node,
        "Error AST node with cem.tokenizer.bare_brace_text must be present"
    );
}

// ---------------------------------------------------------------------------
// Cross-cutting — source-map preservation through the lowering
// ---------------------------------------------------------------------------

#[test]
fn every_element_carries_an_origin_byte_range_after_lowering() {
    // AC-P-1 / AC-P-7: every node carries a source-map stack rooted in
    // Layer 1's source id, so a downstream consumer can locate the
    // original bytes for every Element produced by the tokenizer.
    let doc = parse("{a | {b | x}{c}}");
    assert_no_hard_violations(&doc);
    for node in doc.iter() {
        if matches!(node, CemAstNode::Element { .. }) {
            let range = query::origin_byte_range(node)
                .expect("every element must carry an origin byte range");
            assert!(range.end() <= 32, "byte range {:?} out of source", range);
        }
    }
}
