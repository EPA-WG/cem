//! CSS syntax is interpreted at import, never by retained-tree consumers.
use super::{ImportBuilder, MAX_DEPTH, MAX_VALUES};
use crate::{
    parser::{
        AstNodeId,
        document::CemDocument,
        tree::{CemTreeRange, CemTreeSemantics},
    },
    schema::registry::CSS_SCHEMA_URI,
    validation::css::{
        CssDocumentAst, CssEntryMode, CssEventAst, CssSemanticKindAst, validate_css_document_ast,
    },
};

// Import retains unresolved references without accessing them. Resource-policy
// facts remain on the native owner and still reject ordinary CSS validation;
// the eventual resolver/loader must authorize access before installing styles.
fn pending_resource_policy(d: &crate::diagnostics::Diagnostic) -> bool {
    matches!(
        d.code.as_str(),
        "cem.css.import_rejected" | "cem.css.url_rejected"
    )
}

pub(super) fn parse(
    bytes: &[u8],
    source_uri: &str,
    content_type: &str,
) -> Result<CssDocumentAst, String> {
    use crate::validation::css::{CssSourceValidationRequest, css_document_ast_from_source_bytes};
    let (document, diagnostics) = css_document_ast_from_source_bytes(CssSourceValidationRequest {
        bytes,
        source_uri,
        content_type: Some(content_type),
    });
    super::checked((
        document,
        diagnostics
            .into_iter()
            .filter(|d| !pending_resource_policy(d))
            .collect(),
    ))
}

pub(super) fn project(doc: &CssDocumentAst) -> Result<(CemDocument, CemTreeSemantics), String> {
    if let Some(error) = validate_css_document_ast(doc)
        .iter()
        .find(|d| d.severity.is_hard_violation() && !pending_resource_policy(d))
    {
        return Err(error.message.clone());
    }
    if doc.events.len() > MAX_VALUES || doc.events.iter().any(|e| e.depth > MAX_DEPTH) {
        return Err("CSS exceeds the 64-level / 4096-event import limit.".into());
    }
    let mut p = CssImport {
        b: ImportBuilder::new(),
        events: &doc.events,
    };
    let name = match doc.entry_mode {
        CssEntryMode::Stylesheet => "stylesheet",
        CssEntryMode::ScopedStyleBlock => "style-block",
        CssEntryMode::DeclarationList => "style-attribute",
    };
    let root = p.node(0, name, 0, doc.events.len());
    if doc.entry_mode == CssEntryMode::Stylesheet {
        p.attr(root, "encoding", &doc.encoding_report.normalized_encoding);
    } else {
        p.attr(
            root,
            "host-kind",
            if doc.entry_mode == CssEntryMode::ScopedStyleBlock {
                "style-element"
            } else {
                "unknown"
            },
        );
    }
    p.items(
        root,
        0,
        doc.events.len(),
        doc.entry_mode == CssEntryMode::DeclarationList,
    )?;
    Ok((p.b.ast, p.b.semantics))
}

struct CssImport<'a> {
    b: ImportBuilder,
    events: &'a [CssEventAst],
}

impl CssImport<'_> {
    fn node(&mut self, parent: AstNodeId, name: &str, start: usize, end: usize) -> AstNodeId {
        let source = self
            .events
            .get(start)
            .map(|e| e.source_map.clone())
            .unwrap_or_default();
        let id = self.b.element(parent, CSS_SCHEMA_URI, name, source);
        if let Some(first) = self.events.get(start).filter(|_| start < end) {
            let last = &self.events[end - 1];
            self.b.semantics.ranges.insert(
                id,
                CemTreeRange {
                    line: first.source_range.start.line,
                    column: first.source_range.start.column,
                    offset: first.source_range.start.byte_offset,
                    length: last.source_range.start.byte_offset + last.source_range.byte_length
                        - first.source_range.start.byte_offset,
                },
            );
        }
        id
    }

    fn attr(&mut self, parent: AstNodeId, name: &str, value: &str) {
        let crate::parser::CemAstNode::Element { source, .. } = &self.b.ast.nodes[parent as usize]
        else {
            unreachable!("CSS attributes belong to elements")
        };
        let source = source.clone();
        self.b.attribute(parent, "", name, value.to_owned(), source);
    }

    fn text(&self, start: usize, end: usize) -> String {
        self.events[start..end]
            .iter()
            .map(|e| e.lexeme.as_str())
            .collect()
    }

    fn significant(&self, start: usize, end: usize) -> Option<usize> {
        (start..end).find(|i| {
            !matches!(
                self.events[*i].token_kind.as_str(),
                "whitespace" | "comment" | "presentation-gap"
            )
        })
    }

    fn close(&self, open: usize, end: usize) -> usize {
        (open + 1..end)
            .find(|i| {
                self.events[*i].depth == self.events[open].depth
                    && self.events[*i].kind == "block-close"
            })
            .unwrap_or(end)
    }

    fn items(
        &mut self,
        parent: AstNodeId,
        mut pos: usize,
        end: usize,
        body: bool,
    ) -> Result<(), String> {
        while pos < end {
            let e = &self.events[pos];
            match e.token_kind.as_str() {
                "whitespace" | "presentation-gap" | "semicolon" | "cdo" | "cdc" => {
                    pos += 1;
                    continue;
                }
                "comment" => {
                    let id = self.node(parent, "comment", pos, pos + 1);
                    self.attr(id, "value", e.value.as_deref().unwrap_or(""));
                    pos += 1;
                    continue;
                }
                _ => {}
            }
            let depth = e.depth;
            let semi = (pos..end)
                .find(|i| {
                    self.events[*i].depth == depth && self.events[*i].token_kind == "semicolon"
                })
                .unwrap_or(end);
            if body
                && matches!(
                    e.semantic_kind,
                    CssSemanticKindAst::Property | CssSemanticKindAst::CustomProperty
                )
            {
                let colon = (pos + 1..semi)
                    .find(|i| {
                        self.events[*i].depth == depth && self.events[*i].token_kind == "colon"
                    })
                    .ok_or("CSS declaration has no colon.")?;
                let id = self.node(parent, "declaration", pos, (semi + 1).min(end));
                self.attr(id, "name", e.value.as_deref().unwrap_or(""));
                let significant: Vec<_> = (colon + 1..semi)
                    .filter(|i| {
                        self.events[*i].depth == depth
                            && !matches!(
                                self.events[*i].token_kind.as_str(),
                                "whitespace" | "comment" | "presentation-gap"
                            )
                    })
                    .collect();
                let mut value_end = semi;
                if significant.len() >= 2 {
                    let bang = significant[significant.len() - 2];
                    let important = significant[significant.len() - 1];
                    if self.events[bang].lexeme == "!"
                        && self.events[important].token_kind == "ident"
                        && self.events[important]
                            .value
                            .as_deref()
                            .is_some_and(|v| v.eq_ignore_ascii_case("important"))
                    {
                        self.attr(id, "important", "true");
                        value_end = bang;
                    }
                }
                if e.semantic_kind == CssSemanticKindAst::CustomProperty {
                    self.attr(id, "custom-property", "true");
                }
                self.attr(id, "value", self.text(colon + 1, value_end).trim());
                self.components(id, colon + 1, value_end);
                pos = (semi + 1).min(end);
                continue;
            }
            let open = (pos..semi).find(|i| {
                self.events[*i].depth == depth && self.events[*i].token_kind == "curly-open"
            });
            let prelude_end = open.unwrap_or(semi);
            let close = open.map(|i| self.close(i, end)).unwrap_or(semi);
            let next = (close + 1).min(end);
            if e.token_kind == "at-keyword" {
                let name = e.value.as_deref().unwrap_or("");
                if name.eq_ignore_ascii_case("import") && open.is_none() {
                    self.import(parent, pos, prelude_end, next)?;
                } else if name.eq_ignore_ascii_case("charset") && open.is_none() {
                    let value = self
                        .significant(pos + 1, prelude_end)
                        .and_then(|i| self.events[i].value.as_deref())
                        .ok_or("CSS charset has no value.")?;
                    let id = self.node(parent, "charset", pos, next);
                    self.attr(id, "encoding", value);
                } else {
                    let id = self.node(parent, "rule", pos, next);
                    self.attr(id, "kind", "at");
                    self.attr(id, "name", name);
                    self.attr(id, "prelude", self.text(pos + 1, prelude_end).trim());
                    let at_rule = self.node(id, "at-rule", pos, next);
                    self.attr(at_rule, "name", name);
                    self.attr(at_rule, "prelude", self.text(pos + 1, prelude_end).trim());
                    self.components(at_rule, pos + 1, prelude_end);
                    if let Some(open) = open {
                        match name.to_ascii_lowercase().as_str() {
                            "media"
                            | "supports"
                            | "layer"
                            | "container"
                            | "scope"
                            | "starting-style"
                            | "keyframes"
                            | "-webkit-keyframes"
                            | "font-face"
                            | "page"
                            | "property"
                            | "counter-style"
                            | "font-feature-values"
                            | "font-palette-values"
                            | "position-try"
                            | "view-transition" => {
                                self.items(at_rule, open + 1, close, true)?;
                            }
                            // Unknown at-rule bodies have no known declaration
                            // grammar. Preserve their balanced component values.
                            _ => {
                                self.components(at_rule, open, (close + 1).min(end));
                            }
                        }
                    }
                }
            } else if let Some(open) = open {
                let id = self.node(parent, "rule", pos, next);
                self.attr(id, "kind", "style");
                self.attr(id, "selector", self.text(pos, open).trim());
                self.items(id, open + 1, close, true)?;
            } else {
                return Err(format!(
                    "CSS statement at byte {} has no rule block.",
                    e.source_range.start.byte_offset
                ));
            }
            pos = next;
        }
        Ok(())
    }

    fn import(
        &mut self,
        parent: AstNodeId,
        start: usize,
        end: usize,
        next: usize,
    ) -> Result<(), String> {
        let first = self
            .significant(start + 1, end)
            .ok_or("CSS import has no URL.")?;
        let event = &self.events[first];
        let (href, mut pos) = match event.token_kind.as_str() {
            "string" | "url" => (event.value.clone().unwrap_or_default(), first + 1),
            "function-open"
                if event
                    .value
                    .as_deref()
                    .is_some_and(|v| v.eq_ignore_ascii_case("url")) =>
            {
                let close = self.close(first, end);
                let value = self
                    .significant(first + 1, close)
                    .filter(|i| self.events[*i].token_kind == "string")
                    .ok_or("CSS import url() requires a string.")?;
                if self.significant(value + 1, close).is_some() {
                    return Err("CSS import url() has extra arguments.".into());
                }
                (
                    self.events[value].value.clone().unwrap_or_default(),
                    (close + 1).min(end),
                )
            }
            _ => return Err("CSS import requires a URL or string token.".into()),
        };
        let id = self.node(parent, "import", start, next);
        self.attr(id, "href", &href);
        for name in ["layer", "supports"] {
            if let Some(i) = self.significant(pos, end) {
                let e = &self.events[i];
                if e.value
                    .as_deref()
                    .is_some_and(|v| v.eq_ignore_ascii_case(name))
                {
                    if e.token_kind == "function-open" {
                        let close = self.close(i, end);
                        self.attr(id, name, self.text(i + 1, close).trim());
                        pos = (close + 1).min(end);
                    } else if name == "layer" && e.token_kind == "ident" {
                        self.attr(id, name, "");
                        pos = i + 1;
                    }
                }
            }
        }
        if let Some(i) = self.significant(pos, end) {
            self.attr(id, "media", self.text(i, end).trim());
        }
        Ok(())
    }

    fn components(&mut self, parent: AstNodeId, mut pos: usize, end: usize) {
        while pos < end {
            let e = &self.events[pos];
            let close = if e.kind == "block-open" {
                self.close(pos, end)
            } else {
                pos
            };
            let next = (close + 1).min(end);
            let id = self.node(parent, "component-value", pos, next);
            let kind = match e.token_kind.as_str() {
                "function-open" => "function",
                "curly-open" => "curly",
                "square-open" => "square",
                "parenthesis-open" => "paren",
                "ident" | "string" | "url" | "number" | "percentage" | "dimension" | "hash"
                | "whitespace" => e.token_kind.as_str(),
                "comment" => "comment",
                "presentation-gap" => "whitespace",
                "bad-url" | "bad-string" => "unknown",
                _ => "delimiter",
            };
            self.attr(id, "kind", kind);
            self.attr(id, "token", &self.text(pos, next));
            if let Some(value) = &e.value {
                self.attr(id, "value", value);
            }
            if e.kind == "block-open" {
                let inner = self.node(
                    id,
                    if kind == "function" {
                        "function"
                    } else {
                        "block"
                    },
                    pos,
                    next,
                );
                self.attr(
                    inner,
                    if kind == "function" { "name" } else { "kind" },
                    if kind == "function" {
                        e.value.as_deref().unwrap_or("")
                    } else {
                        kind
                    },
                );
                self.components(inner, pos + 1, close);
            }
            pos = next;
        }
    }
}
