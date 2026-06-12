#![allow(clippy::items_after_test_module)]

//! Event stream → typed CEM AST.
//!
//! Tier A `InputDomAstBuilder` body per `cem-ml-stack-design-impl.md` §3.8.
//! Consumes the `NormalizedEvent` stream produced by Layer 3 and accumulates
//! a flat `CemDocument` arena addressed by `AstNodeId`. Every node carries a
//! `SourceMapStack` rooted in the originating tokenizer frame plus a
//! `TransformKind::CemAstBuilder` frame appended by this layer.

use crate::diagnostics::{Diagnostic, Severity};
use crate::events::{EventNormalizer, NormalizedEvent, ScalarValue, TriviaKind};
use crate::parser::document::CemDocument;
use crate::parser::format;
use crate::parser::{AstNodeId, CemAstNode, ExpandedName, NameSlot};
use crate::source::ByteRange;
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};

/// One parent slot on the build stack.
#[derive(Debug)]
enum Frame {
    Document,
    Element {
        id: AstNodeId,
        #[allow(dead_code)]
        name: String,
    },
}

pub struct CemAstBuilder<E: EventNormalizer> {
    events: E,
    doc: CemDocument,
    stack: Vec<Frame>,
    /// While walking attributes, holds the pending `Name` event so the
    /// following `Value` event finalizes the attribute. Cleared on
    /// `Separator(ElementBoundary)`, on `CloseScope`, or when another
    /// `Name` arrives (last-writer-wins per `cem-ml-stack-design-impl.md`
    /// §3.4 attribute semantics).
    pending_attr: Option<PendingAttr>,
    /// When `true`, this builder is parsing a persisted top-level
    /// canonical CEM-ML document and `finalize` enforces the AC-F-8
    /// `@doc cem-ml <version>` requirement. When `false` (the default),
    /// the builder is parsing an embedded fragment that inherits the
    /// parent's document-format identity, so no `cem.doc.*` diagnostic
    /// is emitted. Toggle with `top_level(true)` at the call site that
    /// knows it owns a persisted document.
    is_top_level: bool,
}

#[derive(Debug)]
struct PendingAttr {
    name: String,
    name_range: ByteRange,
    source_map: SourceMapStack,
}

impl<E: EventNormalizer> CemAstBuilder<E> {
    pub fn new(events: E) -> Self {
        let mut doc = CemDocument::default();
        let root = CemAstNode::Document {
            node_id: 0,
            root_children: Vec::new(),
            source: SourceMapStack::default(),
        };
        doc.nodes.push(root);
        Self {
            events,
            doc,
            stack: vec![Frame::Document],
            pending_attr: None,
            is_top_level: false,
        }
    }

    /// Mark this build as a persisted top-level canonical document so
    /// `finalize` enforces the AC-F-8 `@doc cem-ml <version>` directive
    /// and records the resolved format identity on the document root.
    /// Fragments parsed inside an established CEM-ML scope leave the
    /// default (`false`) so they inherit the parent's identity.
    pub fn top_level(mut self, yes: bool) -> Self {
        self.is_top_level = yes;
        self
    }

    pub fn build(mut self) -> CemDocument {
        while let Some(event) = self.events.next_event() {
            self.consume(event);
        }
        self.finalize();
        self.doc
    }

    fn consume(&mut self, event: NormalizedEvent) {
        match event {
            NormalizedEvent::OpenScope {
                name,
                byte_range,
                source_map,
            } => self.on_open(name.lexical_name, byte_range, source_map),
            NormalizedEvent::CloseScope { .. } => self.on_close(),
            NormalizedEvent::Name { name, byte_range } => {
                // Flush a pending attribute that never received a value
                // (boolean attribute).
                self.flush_pending_attr(None, byte_range);
                self.pending_attr = Some(PendingAttr {
                    name: name.lexical_name,
                    name_range: byte_range,
                    source_map: self.current_source_map(byte_range, TransformKind::CemAstBuilder),
                });
            }
            NormalizedEvent::Value { value, byte_range } => {
                let text = match value {
                    ScalarValue::Text(t) => t,
                    ScalarValue::Int(i) => i.to_string(),
                    ScalarValue::Float(f) => f.to_string(),
                    ScalarValue::Bool(b) => b.to_string(),
                    ScalarValue::Null => String::new(),
                };
                if self.pending_attr.is_some() {
                    self.flush_pending_attr(Some((text, byte_range)), byte_range);
                } else {
                    self.append_text(text, byte_range);
                }
            }
            NormalizedEvent::Trivia {
                kind,
                data,
                byte_range,
            } => {
                // Tokenizer-level whitespace is syntax trivia. Keep the
                // node for source-map continuity, but leave rendering
                // semantics to actual Value events.
                let data = if matches!(kind, TriviaKind::Whitespace) {
                    String::new()
                } else {
                    data
                };
                self.append_trivia_kind(kind, byte_range, data);
            }
            NormalizedEvent::Separator { kind, .. } => {
                // Content-boundary marker: finalize any unflushed pending
                // boolean attribute (e.g. `{input @required | ...}`) and
                // record that this element used an explicit `|` (or `▷`)
                // boundary. The `cem.lint.relaxed_content_boundary` rule
                // reads the flag from the AST.
                if let Some(pending) = self.pending_attr.take() {
                    let range = pending.name_range;
                    self.flush_attr(pending, None, range);
                }
                if matches!(kind, crate::events::SeparatorKind::ElementBoundary) {
                    self.mark_explicit_boundary();
                }
            }
            NormalizedEvent::ProcessingInstruction {
                target,
                data,
                byte_range,
            } => self.append_pi(target, data, byte_range),
            NormalizedEvent::ModeSwitch { .. } => {
                // The handoff stack (Layer 5) and schema machine (Layer 4)
                // handle this; the AST builder doesn't need a node for it.
            }
            NormalizedEvent::Error {
                code, byte_range, ..
            } => {
                self.append_error(code, byte_range);
            }
        }
    }

    fn on_open(&mut self, name: String, byte_range: ByteRange, source_map: SourceMapStack) {
        // Flush any dangling boolean attribute before opening a child.
        if let Some(pending) = self.pending_attr.take() {
            let range = pending.name_range;
            self.flush_attr(pending, None, range);
        }
        let node_id = self.doc.nodes.len() as AstNodeId;
        let mut combined = source_map;
        combined.push(SourceMapFrame {
            source_id: combined
                .frames
                .last()
                .map(|f| f.source_id)
                .unwrap_or(crate::source::SourceId(0)),
            span: FrameSpan::Single(byte_range),
            transform: TransformKind::CemAstBuilder,
        });
        let expanded = expand_name(&name);
        let element = CemAstNode::Element {
            node_id,
            expanded_name: expanded,
            attributes: Vec::new(),
            children: Vec::new(),
            // Set to `true` when a `Separator(ElementBoundary)` event
            // arrives while this element is on top of the build stack.
            has_explicit_boundary: false,
            source: combined,
        };
        self.doc.nodes.push(element);
        // Link into the parent's children.
        self.attach_child(node_id);
        self.stack.push(Frame::Element { id: node_id, name });
    }

    fn on_close(&mut self) {
        // Flush any dangling attribute before closing.
        if let Some(pending) = self.pending_attr.take() {
            let range = pending.name_range;
            self.flush_attr(pending, None, range);
        }
        // Pop the topmost element frame.
        if self.stack.len() <= 1 {
            self.doc.diagnostics.push(Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: None,
                code: "cem.ast.unbalanced_close".to_owned(),
                severity: Severity::Error,
                message: "close-scope event with no matching open element".to_owned(),
                node: None,
                source_map: None,
            });
            return;
        }
        self.stack.pop();
    }

    fn flush_pending_attr(
        &mut self,
        value_and_range: Option<(String, ByteRange)>,
        fallback_range: ByteRange,
    ) {
        if let Some(pending) = self.pending_attr.take() {
            self.flush_attr(pending, value_and_range, fallback_range);
        }
    }

    fn flush_attr(
        &mut self,
        pending: PendingAttr,
        value_and_range: Option<(String, ByteRange)>,
        fallback_range: ByteRange,
    ) {
        let value = value_and_range.as_ref().map(|(v, _)| v.clone());
        let _ = fallback_range;
        let attr_id = self.doc.nodes.len() as AstNodeId;
        let mut source = pending.source_map.clone();
        source.push(SourceMapFrame {
            source_id: source
                .frames
                .last()
                .map(|f| f.source_id)
                .unwrap_or(crate::source::SourceId(0)),
            span: FrameSpan::Single(pending.name_range),
            transform: TransformKind::CemAstBuilder,
        });
        let attr = CemAstNode::Attribute {
            node_id: attr_id,
            expanded_name: expand_name(&pending.name),
            value: value.clone(),
            source,
        };
        self.doc.nodes.push(attr);
        // Append to current element's attribute list.
        if let Some(Frame::Element { id, .. }) = self.stack.last() {
            let parent_id = *id;
            if let Some(CemAstNode::Element { attributes, .. }) =
                self.doc.nodes.get_mut(parent_id as usize)
            {
                attributes.push(attr_id);
            }
        }
        // Reference tracking: `id=` populates the id_table, `for=` /
        // `aria-labelledby=` / `aria-describedby=` resolve through it.
        self.update_references(&pending.name, value.as_deref(), attr_id);
    }

    fn update_references(&mut self, name: &str, value: Option<&str>, attr_id: AstNodeId) {
        let Some(value) = value else { return };
        let parent_id = match self.stack.last() {
            Some(Frame::Element { id, .. }) => *id,
            _ => return,
        };
        match name {
            "id" => {
                self.doc.id_table.insert(value.to_owned(), parent_id);
            }
            "for" | "aria-labelledby" | "aria-describedby" | "aria-controls" => {
                let resolved = self.doc.id_table.get(value).copied();
                if resolved.is_none() {
                    self.doc.unresolved_slots.push(NameSlot {
                        owner_scope: parent_id,
                        target_name: value.to_owned(),
                        resolved: None,
                        source: self
                            .doc
                            .nodes
                            .get(attr_id as usize)
                            .and_then(|n| match n {
                                CemAstNode::Attribute { source, .. } => Some(source.clone()),
                                _ => None,
                            })
                            .unwrap_or_default(),
                    });
                }
            }
            _ => {}
        }
    }

    fn append_text(&mut self, data: String, byte_range: ByteRange) {
        if data.trim().is_empty() {
            // Whitespace-only Value events fold into Whitespace nodes.
            self.append_trivia_kind(TriviaKind::Whitespace, byte_range, data);
            return;
        }
        let node_id = self.doc.nodes.len() as AstNodeId;
        let source = self.current_source_map(byte_range, TransformKind::CemAstBuilder);
        self.doc.nodes.push(CemAstNode::Text {
            node_id,
            data,
            source,
        });
        self.attach_child(node_id);
    }

    fn append_trivia_kind(&mut self, kind: TriviaKind, byte_range: ByteRange, data: String) {
        let node_id = self.doc.nodes.len() as AstNodeId;
        let source = self.current_source_map(byte_range, TransformKind::CemAstBuilder);
        let node = match kind {
            TriviaKind::Whitespace => CemAstNode::Whitespace {
                node_id,
                data,
                source,
            },
            TriviaKind::Comment => CemAstNode::Comment {
                node_id,
                data,
                source,
            },
        };
        self.doc.nodes.push(node);
        self.attach_child(node_id);
    }

    fn append_pi(&mut self, target: String, data: String, byte_range: ByteRange) {
        let node_id = self.doc.nodes.len() as AstNodeId;
        let source = self.current_source_map(byte_range, TransformKind::CemAstBuilder);
        self.doc.nodes.push(CemAstNode::ProcessingInstruction {
            node_id,
            target,
            data,
            source,
        });
        self.attach_child(node_id);
    }

    fn append_error(&mut self, code: String, byte_range: ByteRange) {
        let node_id = self.doc.nodes.len() as AstNodeId;
        let source = self.current_source_map(byte_range, TransformKind::CemAstBuilder);
        self.doc.nodes.push(CemAstNode::Error {
            node_id,
            code,
            source,
        });
        self.attach_child(node_id);
    }

    fn mark_explicit_boundary(&mut self) {
        let Some(Frame::Element { id, .. }) = self.stack.last() else {
            return;
        };
        let parent_id = *id as usize;
        if let Some(CemAstNode::Element {
            has_explicit_boundary,
            ..
        }) = self.doc.nodes.get_mut(parent_id)
        {
            *has_explicit_boundary = true;
        }
    }

    fn attach_child(&mut self, child: AstNodeId) {
        match self.stack.last() {
            Some(Frame::Document) => {
                if let Some(CemAstNode::Document { root_children, .. }) = self.doc.nodes.get_mut(0)
                {
                    root_children.push(child);
                }
            }
            Some(Frame::Element { id, .. }) => {
                let parent = *id;
                if let Some(CemAstNode::Element { children, .. }) =
                    self.doc.nodes.get_mut(parent as usize)
                {
                    children.push(child);
                }
            }
            None => {}
        }
    }

    fn current_source_map(
        &self,
        byte_range: ByteRange,
        transform: TransformKind,
    ) -> SourceMapStack {
        let mut stack = match self.stack.last() {
            Some(Frame::Element { id, .. }) => {
                if let Some(CemAstNode::Element { source, .. }) = self.doc.nodes.get(*id as usize) {
                    source.clone()
                } else {
                    SourceMapStack::default()
                }
            }
            _ => SourceMapStack::default(),
        };
        let source_id = stack
            .frames
            .last()
            .map(|f| f.source_id)
            .unwrap_or(crate::source::SourceId(0));
        stack.push(SourceMapFrame {
            source_id,
            span: FrameSpan::Single(byte_range),
            transform,
        });
        stack
    }

    fn finalize(&mut self) {
        // Surface any dangling pending attribute (rare; should be caught
        // by Separator/CloseScope flushes).
        if let Some(pending) = self.pending_attr.take() {
            let range = pending.name_range;
            self.flush_attr(pending, None, range);
        }
        // Unbalanced opens (scopes still on the stack at EOF) are reported
        // by the schema machine; AST records this as a diagnostic too so a
        // caller using only the AST builder still sees the failure.
        if self.stack.len() > 1 {
            self.doc.diagnostics.push(Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: None,
                code: "cem.ast.unclosed_scope".to_owned(),
                severity: Severity::Error,
                message: format!(
                    "{} scope(s) still open at end of input",
                    self.stack.len() - 1
                ),
                node: None,
                source_map: None,
            });
        }
        // Emit a Warning for each unresolved name slot, per AC reference
        // slots semantics.
        let unresolved = std::mem::take(&mut self.doc.unresolved_slots);
        for slot in &unresolved {
            self.doc.diagnostics.push(Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: slot.source.frames.last().and_then(|f| match &f.span {
                    FrameSpan::Single(r) => Some(r.start),
                    FrameSpan::Multi(rs) => rs.first().map(|r| r.start),
                }),
                code: "cem.ast.unresolved_reference".to_owned(),
                severity: Severity::Warning,
                message: format!(
                    "reference `{}` did not match any element id",
                    slot.target_name
                ),
                node: None,
                source_map: None,
            });
        }
        self.doc.unresolved_slots = unresolved;
        // AC-F-8: a persisted top-level document MUST begin with
        // `@doc cem-ml <version>` before any non-trivia item. Fragments
        // (the default mode) inherit the parent identity and are not
        // checked here.
        if self.is_top_level {
            self.resolve_top_level_format_identity();
        }
    }

    /// Walk the document root for the leading `@doc cem-ml <version>`
    /// directive, resolve it, and emit either `cem.doc.version_resolved`
    /// (Info) on success or the documented `cem.doc.*` Error per
    /// AC-F-8 on failure. Missing entirely → `cem.doc.version_missing`.
    fn resolve_top_level_format_identity(&mut self) {
        let root_children: Vec<AstNodeId> = match self.doc.nodes.first() {
            Some(CemAstNode::Document { root_children, .. }) => root_children.clone(),
            _ => return,
        };
        let mut directive_id: Option<AstNodeId> = None;
        for child in root_children {
            match self.doc.nodes.get(child as usize) {
                // Trivia is allowed before `@doc`.
                Some(CemAstNode::Whitespace { .. }) | Some(CemAstNode::Comment { .. }) => continue,
                // The first non-trivia node MUST be the `@doc` element.
                Some(CemAstNode::Element { expanded_name, .. })
                    if expanded_name.local_name == "@doc" =>
                {
                    directive_id = Some(child);
                    break;
                }
                _ => break,
            }
        }

        let Some(directive_id) = directive_id else {
            self.doc.diagnostics.push(Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: Some(0),
                code: format::VERSION_MISSING_CODE.to_owned(),
                severity: Severity::Error,
                message:
                    "persisted top-level CEM-ML document must begin with `@doc cem-ml <version>`"
                        .to_owned(),
                node: None,
                source_map: None,
            });
            return;
        };

        let (text, source_map) = self.collect_directive_text(directive_id);
        let byte_offset = source_map.frames.last().and_then(|f| match &f.span {
            FrameSpan::Single(r) => Some(r.start),
            FrameSpan::Multi(rs) => rs.first().map(|r| r.start),
        });
        match format::resolve_doc_directive(&text) {
            Ok(identity) => {
                let message = format!(
                    "resolved @doc {} {} -> embedded {}",
                    identity.format_id, identity.content_type, identity.format_version
                );
                self.doc.format_identity = Some(identity);
                self.doc.diagnostics.push(Diagnostic {
                    uri: None,
                    line: None,
                    column: None,
                    byte_offset,
                    code: format::VERSION_RESOLVED_CODE.to_owned(),
                    severity: Severity::Info,
                    message,
                    node: None,
                    source_map: Some(source_map),
                });
            }
            Err(err) => {
                self.doc.diagnostics.push(Diagnostic {
                    uri: None,
                    line: None,
                    column: None,
                    byte_offset,
                    code: err.code().to_owned(),
                    severity: Severity::Error,
                    message: err.message(),
                    node: None,
                    source_map: Some(source_map),
                });
            }
        }
    }

    /// Concatenate the directive element's text children — the tokenizer
    /// emits the value as a single `Value(Text("cem-ml 1"))`, but this
    /// also handles fragmented value events (e.g. trivia interleaving).
    fn collect_directive_text(&self, directive_id: AstNodeId) -> (String, SourceMapStack) {
        let (children, source_map) = match self.doc.nodes.get(directive_id as usize) {
            Some(CemAstNode::Element {
                children, source, ..
            }) => (children.clone(), source.clone()),
            _ => return (String::new(), SourceMapStack::default()),
        };
        let mut text = String::new();
        for child in children {
            if let Some(CemAstNode::Text { data, .. }) = self.doc.nodes.get(child as usize) {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(data);
            }
        }
        (text, source_map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::cem::CemEventNormalizer;
    use crate::query;
    use crate::source::{BytesSource, SourceId};
    use crate::tokenizer::cem::CemTokenizer;

    fn parse(input: &str) -> CemDocument {
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        CemAstBuilder::new(normalizer).build()
    }

    #[test]
    fn document_root_is_node_zero() {
        let doc = parse("{p Hello}");
        assert!(matches!(
            doc.root(),
            Some(CemAstNode::Document { node_id: 0, .. })
        ));
    }

    #[test]
    fn nested_element_is_child_of_outer() {
        let doc = parse("{a | {b | x}}");
        let outer = query::find_by_local_name(&doc, "a").next().unwrap();
        let CemAstNode::Element { children, .. } = outer else {
            panic!()
        };
        // outer has the inner `b` element (plus possibly whitespace/text).
        let has_inner_b = children.iter().any(|child_id| {
            matches!(doc.get(*child_id), Some(CemAstNode::Element { expanded_name, .. }) if expanded_name.local_name == "b")
        });
        assert!(has_inner_b, "outer should contain inner element b");
    }

    #[test]
    fn attribute_value_is_recorded() {
        let doc = parse(r#"{field @name=email @label="Email"}"#);
        let field = query::find_by_local_name(&doc, "field").next().unwrap();
        let names: Vec<(String, Option<String>)> = match field {
            CemAstNode::Element { attributes, .. } => attributes
                .iter()
                .filter_map(|a| match doc.get(*a) {
                    Some(CemAstNode::Attribute {
                        expanded_name,
                        value,
                        ..
                    }) => Some((expanded_name.local_name.clone(), value.clone())),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        assert!(names.contains(&("name".into(), Some("email".into()))));
        assert!(names.contains(&("label".into(), Some("Email".into()))));
    }

    #[test]
    fn expanded_names_carry_cem_core_schema_id() {
        let doc = parse(r#"{button @cem:action=primary | Save}"#);
        let button = query::find_by_local_name(&doc, "button").next().unwrap();
        let CemAstNode::Element {
            expanded_name,
            attributes,
            ..
        } = button
        else {
            panic!()
        };
        assert_eq!(
            expanded_name.schema_id,
            Some(crate::schema::ir::CEM_CORE_SCHEMA_ID)
        );
        let action = attributes
            .iter()
            .find_map(|id| match doc.get(*id) {
                Some(CemAstNode::Attribute { expanded_name, .. })
                    if expanded_name.local_name == "action" =>
                {
                    Some(expanded_name)
                }
                _ => None,
            })
            .expect("action attribute");
        assert_eq!(
            action.schema_id,
            Some(crate::schema::ir::CEM_CORE_SCHEMA_ID)
        );
    }

    #[test]
    fn boolean_attribute_has_no_value() {
        let doc = parse("{input @required}");
        let input = query::find_by_local_name(&doc, "input").next().unwrap();
        let CemAstNode::Element { attributes, .. } = input else {
            panic!()
        };
        let req = attributes
            .iter()
            .find_map(|a| match doc.get(*a) {
                Some(CemAstNode::Attribute {
                    expanded_name,
                    value,
                    ..
                }) if expanded_name.local_name == "required" => Some(value.clone()),
                _ => None,
            })
            .unwrap();
        assert!(req.is_none(), "boolean attribute should have no value");
    }

    #[test]
    fn id_attribute_populates_id_table_and_resolves_for_attribute() {
        let doc = parse(r#"{form | {label @for=email | Email} {input @id=email}}"#);
        assert!(doc.id_table.contains_key("email"));
        // The `for=email` reference resolves to the `input` element.
        let label = query::find_by_local_name(&doc, "label").next().unwrap();
        let CemAstNode::Element { attributes, .. } = label else {
            panic!()
        };
        let for_attr = attributes
            .iter()
            .find_map(|a| match doc.get(*a) {
                Some(node @ CemAstNode::Attribute { expanded_name, .. })
                    if expanded_name.local_name == "for" =>
                {
                    Some(node)
                }
                _ => None,
            })
            .unwrap();
        let resolved = query::resolve_reference(&doc, for_attr).unwrap();
        assert!(
            matches!(resolved, CemAstNode::Element { expanded_name, .. } if expanded_name.local_name == "input"),
        );
    }

    #[test]
    fn unresolved_reference_emits_warning() {
        let doc = parse(r#"{label @for=missing | Missing}"#);
        assert!(doc
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.ast.unresolved_reference"));
    }

    #[test]
    fn every_node_carries_an_origin_byte_range() {
        let doc = parse("{p | Hello}");
        // Skip the synthetic Document root (no origin span; created from
        // the OpenScope event implicitly). Every element/attribute/text/
        // whitespace node must trace to an origin span.
        for node in doc.iter() {
            match node {
                CemAstNode::Document { .. } => {}
                _ => {
                    assert!(
                        query::origin_byte_range(node).is_some(),
                        "node has no origin byte range: {node:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn cem_annotations_filter_excludes_cem_state() {
        let doc = parse(r#"{button @cem:action=primary @cem:state="loading" | Save}"#);
        let button = query::find_by_local_name(&doc, "button").next().unwrap();
        let annotations: Vec<&str> = query::cem_annotations(&doc, button)
            .filter_map(|a| match a {
                CemAstNode::Attribute { expanded_name, .. } => {
                    Some(expanded_name.local_name.as_str())
                }
                _ => None,
            })
            .collect();
        assert_eq!(annotations, vec!["action"]);
        let states = query::state_of(&doc, button);
        assert_eq!(states, vec!["loading".to_owned()]);
    }

    #[test]
    fn elements_with_annotation_finds_every_screen() {
        let doc = parse(
            r#"@doc cem-ml 1
{main @cem:screen="login" | a}
{main @cem:screen="profile" | b}"#,
        );
        let screens: Vec<&str> = query::elements_with_annotation(&doc, "screen")
            .filter_map(|n| match n {
                CemAstNode::Element { attributes, .. } => {
                    attributes.iter().find_map(|a| match doc.get(*a) {
                        Some(CemAstNode::Attribute {
                            expanded_name,
                            value,
                            ..
                        }) if expanded_name.local_name == "screen" => value.as_deref(),
                        _ => None,
                    })
                }
                _ => None,
            })
            .collect();
        assert_eq!(screens, vec!["login", "profile"]);
    }

    #[test]
    fn fixture_login_cem_parses_into_expected_shape() {
        let dir =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/cem-ml");
        let input = std::fs::read_to_string(dir.join("login.cem")).unwrap();
        let doc = parse(&input);
        // Must have a main element with cem:screen="login".
        let logins: Vec<_> = query::elements_with_annotation(&doc, "screen").collect();
        assert!(
            !logins.is_empty(),
            "expected at least one cem:screen element"
        );
        // The login screen is wrapped in a `main`.
        let mains: Vec<_> = query::find_by_local_name(&doc, "main").collect();
        assert!(!mains.is_empty(), "expected at least one main element");
        // The sign-in form is present.
        let forms: Vec<_> = query::elements_with_annotation(&doc, "form").collect();
        assert!(!forms.is_empty(), "expected at least one cem:form element");
    }

    #[test]
    fn every_canonical_fixture_parses_without_ast_hard_violations() {
        let dir =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/cem-ml");
        let mut checked = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("cem") {
                continue;
            }
            let input = std::fs::read_to_string(&path).unwrap();
            let doc = parse(&input);
            let hard: Vec<&Diagnostic> = doc
                .diagnostics
                .iter()
                .filter(|d| {
                    d.code.starts_with("cem.ast.")
                        && matches!(d.severity, Severity::Error | Severity::Fatal)
                })
                .collect();
            assert!(
                hard.is_empty(),
                "fixture `{}` produced AST hard violations: {hard:?}",
                path.display()
            );
            checked += 1;
        }
        assert!(checked >= 5);
    }

    #[test]
    fn origin_byte_range_traces_to_source_bytes() {
        let input = "{p | Hello}";
        let doc = parse(input);
        let p = query::find_by_local_name(&doc, "p").next().unwrap();
        let r = query::origin_byte_range(p).unwrap();
        // The `p` opens at byte 0; the origin frame covers the opening
        // `{p` head span emitted by the tokenizer.
        assert_eq!(r.start, 0);
        // Verify text is positioned after the `|`.
        let texts: Vec<&CemAstNode> = doc
            .iter()
            .filter(|n| matches!(n, CemAstNode::Text { .. }))
            .collect();
        assert_eq!(texts.len(), 1);
        let r = query::origin_byte_range(texts[0]).unwrap();
        // "Hello" starts after `{p | ` — that's offset 5 (count "{p | ").
        let bytes = &input.as_bytes()[r.start as usize..(r.start + r.len as u64) as usize];
        assert!(
            std::str::from_utf8(bytes).unwrap().contains("Hello"),
            "byte range should point at the text source: {:?}",
            std::str::from_utf8(bytes)
        );
    }
}

fn expand_name(raw: &str) -> ExpandedName {
    let (prefix, local) = match raw.split_once(':') {
        Some((p, l)) => (Some(p), l),
        None => (None, raw),
    };
    // The tokenizer emits namespace prefixes lexically; namespace-URI
    // rebinding is tracked by `cem_ml::schema::namespace`. The parser
    // records the active Tier A schema id so downstream schema-frame
    // consumers can distinguish unvalidated decoded names from parsed
    // CEM-Core names.
    ExpandedName {
        namespace_uri: prefix.map(|p| p.to_owned()).unwrap_or_default(),
        local_name: local.to_owned(),
        schema_id: Some(crate::schema::ir::CEM_CORE_SCHEMA_ID),
    }
}
