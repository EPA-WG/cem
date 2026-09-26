//! CSS syntax is interpreted at import, never by retained-tree consumers.
mod selectors;
use super::{ImportBuilder, MAX_DEPTH, MAX_VALUES};
use crate::{
    parser::{
        document::CemDocument,
        tree::{CemTreeRange, CemTreeSemantics},
        AstNodeId,
    },
    schema::registry::CSS_SCHEMA_URI,
    validation::css::{
        validate_css_document_ast, CssDocumentAst, CssEntryMode, CssEventAst, CssSemanticKindAst,
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
    use crate::validation::css::{css_document_ast_from_source_bytes, CssSourceValidationRequest};
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
        false,
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
        nested: bool,
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
                    self.attr(
                        id,
                        "has-block",
                        if open.is_some() { "true" } else { "false" },
                    );
                    self.attr(id, "name", name);
                    self.attr(id, "prelude", self.text(pos + 1, prelude_end).trim());
                    let at_rule = self.node(id, "at-rule", pos, next);
                    self.attr(at_rule, "name", name);
                    self.attr(at_rule, "prelude", self.text(pos + 1, prelude_end).trim());
                    match name.to_ascii_lowercase().as_str() {
                        "media" => {
                            self.media_list(at_rule, "group-media", pos + 1, prelude_end, true)
                        }
                        "supports" => {
                            let condition =
                                self.node(at_rule, "group-supports", pos + 1, prelude_end);
                            let valid =
                                self.supports_form(pos + 1, prelude_end) == Some("condition");
                            self.attr(
                                condition,
                                "syntax-valid",
                                if valid { "true" } else { "false" },
                            );
                            self.components(condition, pos + 1, prelude_end);
                        }
                        _ => self.components(at_rule, pos + 1, prelude_end),
                    }
                    if let Some(open) = open {
                        match name.to_ascii_lowercase().as_str() {
                            "media" | "supports" | "layer" | "container" | "starting-style" => {
                                self.items(at_rule, open + 1, close, true, nested)?;
                            }
                            // Authored @scope changes the meaning of & to its
                            // scoping root; it cannot inherit a style-rule parent.
                            // Scope-relative selector support remains deferred.
                            "scope"
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
                                self.items(at_rule, open + 1, close, true, false)?;
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
                self.attr(
                    id,
                    "selector-context",
                    if nested { "nested" } else { "root" },
                );
                self.selectors(id, pos, open, nested);
                self.items(id, open + 1, close, true, true)?;
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
                        if name == "layer" {
                            self.import_layer(id, i, close)?;
                        } else {
                            self.import_supports(id, i, close)?;
                        }
                        self.attr(id, name, self.text(i + 1, close).trim());
                        pos = (close + 1).min(end);
                    } else if name == "layer" && e.token_kind == "ident" {
                        self.attr(id, name, "");
                        let layer = self.node(id, "import-layer", i, i + 1);
                        self.attr(layer, "anonymous", "true");
                        pos = i + 1;
                    }
                }
            }
        }
        if let Some(i) = self.significant(pos, end) {
            self.attr(id, "media", self.text(i, end).trim());
            self.import_media(id, i, end);
        }
        Ok(())
    }

    /// Decode the layer path at the import boundary. Keep segments separate:
    /// `base\.theme` names one layer, whereas `base.theme` names a nested layer.
    fn import_layer(&mut self, parent: AstNodeId, open: usize, close: usize) -> Result<(), String> {
        let error = || {
            format!(
                concat!(
                    "CSS import layer() at byte {} requires a nonempty dotted identifier path ",
                    "without internal whitespace or CSS-wide keywords."
                ),
                self.events[open].source_range.start.byte_offset,
            )
        };
        let significant: Vec<_> = (open + 1..close)
            .filter(|i| {
                !matches!(
                    self.events[*i].token_kind.as_str(),
                    "whitespace" | "comment" | "presentation-gap"
                )
            })
            .collect();
        let (Some(&first), Some(&last)) = (significant.first(), significant.last()) else {
            return Err(error());
        };
        if significant.len() % 2 == 0
            || self.events[first..=last]
                .iter()
                .any(|e| matches!(e.token_kind.as_str(), "whitespace" | "presentation-gap"))
        {
            return Err(error());
        }
        for (index, &i) in significant.iter().enumerate() {
            let e = &self.events[i];
            if index % 2 == 0 {
                if e.token_kind != "ident"
                    || e.value.as_deref().is_none_or(|value| {
                        ["initial", "inherit", "unset", "revert", "revert-layer"]
                            .iter()
                            .any(|keyword| value.eq_ignore_ascii_case(keyword))
                    })
                {
                    return Err(error());
                }
            } else if e.token_kind != "delimiter" || e.value.as_deref() != Some(".") {
                return Err(error());
            }
        }
        let layer = self.node(parent, "import-layer", open, close + 1);
        self.attr(layer, "anonymous", "false");
        for i in significant.into_iter().step_by(2) {
            let segment = self.node(layer, "layer-segment", i, i + 1);
            self.attr(segment, "value", self.events[i].value.as_deref().unwrap());
        }
        Ok(())
    }

    fn import_supports(
        &mut self,
        parent: AstNodeId,
        open: usize,
        close: usize,
    ) -> Result<(), String> {
        let form = self.supports_form(open + 1, close).ok_or_else(|| {
            format!(
                "CSS import supports() at byte {} requires a declaration or a valid supports condition.",
                self.events[open].source_range.start.byte_offset,
            )
        })?;
        let supports = self.node(parent, "import-supports", open, close + 1);
        self.attr(supports, "condition-form", form);
        self.components(supports, open + 1, close);
        Ok(())
    }

    /// Validate syntax only, never evaluate feature support. Balanced functions
    /// and parentheses admit general-enclosed syntax for forward compatibility;
    /// their contents must be retained even when this implementation cannot
    /// interpret a feature. The emitting browser evaluates the unchanged query.
    fn supports_form(&self, start: usize, end: usize) -> Option<&'static str> {
        let top = self.condition_tokens(start, end)?;
        let first = *top.first()?;
        let keyword = |i, value| self.keyword(i, value);
        let bang = |i: usize| {
            self.events[i].token_kind == "delimiter" && self.events[i].value.as_deref() == Some("!")
        };
        if self.events[first].token_kind == "ident"
            && top
                .get(1)
                .is_some_and(|i| self.events[*i].token_kind == "colon")
        {
            // A declaration permits an empty value and a final !important,
            // but no top-level semicolon or other exclamation delimiter.
            let mut value_end = top.len();
            if value_end >= 4
                && bang(top[value_end - 2])
                && keyword(top[value_end - 1], "important")
            {
                value_end -= 2;
            }
            return top[2..value_end]
                .iter()
                .all(|i| self.events[*i].token_kind != "semicolon" && !bang(*i))
                .then_some("declaration");
        }
        self.boolean_condition(&top, true).then_some("condition")
    }

    fn keyword(&self, i: usize, value: &str) -> bool {
        self.events[i].token_kind == "ident"
            && self.events[i]
                .value
                .as_deref()
                .is_some_and(|v| v.eq_ignore_ascii_case(value))
    }

    /// Top-level component indices, with nested blocks retained as single operands.
    fn condition_tokens(&self, start: usize, end: usize) -> Option<Vec<usize>> {
        if self.events[start..end].iter().any(|e| e.recovered) {
            return None;
        }
        let mut top = Vec::new();
        let mut pos = start;
        while let Some(i) = self.significant(pos, end) {
            top.push(i);
            pos = if self.events[i].kind == "block-open" {
                let close = self.close(i, end);
                if close == end {
                    return None;
                }
                close + 1
            } else {
                i + 1
            };
        }
        Some(top)
    }

    fn boolean_condition(&self, top: &[usize], allow_or: bool) -> bool {
        let Some(&first) = top.first() else {
            return false;
        };
        let atom = |i: usize| {
            matches!(
                self.events[i].token_kind.as_str(),
                "parenthesis-open" | "function-open"
            )
        };
        if self.keyword(first, "not") {
            return top.len() == 2 && atom(top[1]);
        }
        if !atom(first) || top.len() % 2 == 0 {
            return false;
        }
        let operator = if top.get(1).is_some_and(|i| self.keyword(*i, "or")) {
            if !allow_or {
                return false;
            }
            "or"
        } else {
            "and"
        };
        top[1..]
            .chunks_exact(2)
            .all(|pair| self.keyword(pair[0], operator) && atom(pair[1]))
    }

    fn media_query_valid(&self, start: usize, end: usize) -> bool {
        let Some(top) = self.condition_tokens(start, end) else {
            return false;
        };
        if self.boolean_condition(&top, true) {
            return true;
        }
        let mut pos = 0;
        if top
            .first()
            .is_some_and(|i| self.keyword(*i, "not") || self.keyword(*i, "only"))
        {
            pos += 1;
        }
        let Some(&media_type) = top.get(pos) else {
            return false;
        };
        if self.events[media_type].token_kind != "ident"
            || ["not", "only", "and", "or", "layer"]
                .iter()
                .any(|word| self.keyword(media_type, word))
        {
            return false;
        }
        pos += 1;
        pos == top.len()
            || (self.keyword(top[pos], "and") && self.boolean_condition(&top[pos + 1..], false))
    }

    fn import_media(&mut self, parent: AstNodeId, start: usize, end: usize) {
        self.media_list(parent, "import-media", start, end, false);
    }

    fn media_list(
        &mut self,
        parent: AstNodeId,
        name: &str,
        start: usize,
        end: usize,
        allow_empty: bool,
    ) {
        let media = self.node(parent, name, start, end);
        if allow_empty && self.significant(start, end).is_none() {
            self.attr(media, "empty", "true");
            return;
        }
        let depth = self.events[start].depth;
        let separators: Vec<_> = (start..end)
            .filter(|i| self.events[*i].depth == depth && self.events[*i].token_kind == "comma")
            .chain(std::iter::once(end))
            .collect();
        let mut query_start = start;
        for query_end in separators {
            let valid = self.media_query_valid(query_start, query_end);
            let anchor = query_start.min(self.events.len() - 1);
            let query = self.node(media, "media-query", anchor, query_end);
            if query_start == query_end {
                // Empty entries are invalid, with a zero-width diagnostic range.
                // At EOF the preceding event is the final comma.
                let event = &self.events[anchor];
                let extra = if query_start == self.events.len() {
                    event.source_range.byte_length
                } else {
                    0
                };
                self.b.semantics.ranges.insert(
                    query,
                    CemTreeRange {
                        line: event.source_range.start.line,
                        column: event.source_range.start.column + extra as u32,
                        offset: event.source_range.start.byte_offset + extra,
                        length: 0,
                    },
                );
            }
            self.attr(query, "syntax-valid", if valid { "true" } else { "false" });
            self.components(query, query_start, query_end);
            query_start = query_end + 1;
        }
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
