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
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::registry::CEM_ML_SCHEMA_URI;
use crate::source::ByteRange;
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::OnceLock;

const CEM_ML_PACKAGE_ID: &str = "cem-ml";
const CEM_ML_AST_REPORT_BEHAVIOR: &str = "cem-ml-ast-report-fact";
const CEM_ML_DOC_REPORT_BEHAVIOR: &str = "cem-ml-doc-report-fact";
#[cfg(test)]
const AST_UNBALANCED_CLOSE_CONTRACT: &str = "ast-unbalanced-close";
#[cfg(test)]
const AST_UNCLOSED_SCOPE_CONTRACT: &str = "ast-unclosed-scope";
#[cfg(test)]
const AST_UNRESOLVED_REFERENCE_CONTRACT: &str = "ast-unresolved-reference";
#[cfg(test)]
const DOC_VERSION_MISSING_CONTRACT: &str = "doc-version-missing";
#[cfg(test)]
const DOC_SEMVER_INVALID_CONTRACT: &str = "doc-semver-invalid";
#[cfg(test)]
const DOC_FORMAT_UNKNOWN_CONTRACT: &str = "doc-format-unknown";
#[cfg(test)]
const DOC_VERSION_UNSUPPORTED_CONTRACT: &str = "doc-version-unsupported";
#[cfg(test)]
const DOC_PRERELEASE_UNMATCHED_CONTRACT: &str = "doc-prerelease-unmatched";
#[cfg(test)]
const DOC_VERSION_RESOLVED_CONTRACT: &str = "doc-version-resolved";

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
    cem_ml_parser_diagnostics: Option<CemMlParserDiagnosticCatalog>,
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
            cem_ml_parser_diagnostics: None,
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

    #[cfg(test)]
    fn with_cem_ml_parser_diagnostic_catalog(
        mut self,
        catalog: CemMlParserDiagnosticCatalog,
    ) -> Self {
        self.cem_ml_parser_diagnostics = Some(catalog);
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
            NormalizedEvent::CloseScope {
                byte_range,
                source_map,
                ..
            } => self.on_close(byte_range, source_map),
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

    fn on_close(&mut self, byte_range: ByteRange, source_map: SourceMapStack) {
        // Flush any dangling attribute before closing.
        if let Some(pending) = self.pending_attr.take() {
            let range = pending.name_range;
            self.flush_attr(pending, None, range);
        }
        // Pop the topmost element frame.
        if self.stack.len() <= 1 {
            self.push_parser_fact_diagnostic(
                CemMlParserFactKind::AstUnbalancedClose,
                Some(byte_range.start),
                "close-scope event with no matching open element".to_owned(),
                Some(source_map),
            );
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
            self.push_parser_fact_diagnostic(
                CemMlParserFactKind::AstUnclosedScope,
                None,
                format!(
                    "{} scope(s) still open at end of input",
                    self.stack.len() - 1
                ),
                None,
            );
        }
        // Emit a Warning for each unresolved name slot, per AC reference
        // slots semantics.
        let unresolved = std::mem::take(&mut self.doc.unresolved_slots);
        for slot in &unresolved {
            self.push_parser_fact_diagnostic(
                CemMlParserFactKind::AstUnresolvedReference,
                slot.source.frames.last().and_then(|f| match &f.span {
                    FrameSpan::Single(r) => Some(r.start),
                    FrameSpan::Multi(rs) => rs.first().map(|r| r.start),
                }),
                format!(
                    "reference `{}` did not match any element id",
                    slot.target_name
                ),
                Some(slot.source.clone()),
            );
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
            self.push_parser_fact_diagnostic(
                CemMlParserFactKind::DocVersionMissing,
                Some(0),
                "persisted top-level CEM-ML document must begin with `@doc cem-ml <version>`"
                    .to_owned(),
                None,
            );
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
                self.push_parser_fact_diagnostic(
                    CemMlParserFactKind::DocVersionResolved,
                    byte_offset,
                    message,
                    Some(source_map),
                );
            }
            Err(err) => {
                self.push_parser_fact_diagnostic(
                    CemMlParserFactKind::from_doc_directive_error(&err),
                    byte_offset,
                    err.message(),
                    Some(source_map),
                );
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

    fn push_parser_fact_diagnostic(
        &mut self,
        kind: CemMlParserFactKind,
        byte_offset: Option<u64>,
        message: String,
        source_map: Option<SourceMapStack>,
    ) {
        let binding = if let Some(catalog) = self.cem_ml_parser_diagnostics.as_ref() {
            catalog.binding_for_fact(kind).cloned()
        } else {
            builtin_cem_ml_parser_diagnostic_catalog()
                .binding_for_fact(kind)
                .cloned()
        };
        let Some(binding) = binding else {
            return;
        };
        self.doc.diagnostics.push(cem_ml_parser_fact_diagnostic(
            kind,
            &binding,
            byte_offset,
            message,
            source_map,
        ));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CemMlParserFactKind {
    AstUnbalancedClose,
    AstUnclosedScope,
    AstUnresolvedReference,
    DocVersionMissing,
    DocSemverInvalid,
    DocFormatUnknown,
    DocVersionUnsupported,
    DocPrereleaseUnmatched,
    DocVersionResolved,
}

impl CemMlParserFactKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::AstUnbalancedClose => "ast-unbalanced-close",
            Self::AstUnclosedScope => "ast-unclosed-scope",
            Self::AstUnresolvedReference => "ast-unresolved-reference",
            Self::DocVersionMissing => "doc-version-missing",
            Self::DocSemverInvalid => "doc-semver-invalid",
            Self::DocFormatUnknown => "doc-format-unknown",
            Self::DocVersionUnsupported => "doc-version-unsupported",
            Self::DocPrereleaseUnmatched => "doc-prerelease-unmatched",
            Self::DocVersionResolved => "doc-version-resolved",
        }
    }

    fn from_doc_directive_error(error: &format::DocDirectiveError) -> Self {
        match error {
            format::DocDirectiveError::SemverInvalid { .. } => Self::DocSemverInvalid,
            format::DocDirectiveError::FormatUnknown { .. } => Self::DocFormatUnknown,
            format::DocDirectiveError::VersionUnsupported { .. } => Self::DocVersionUnsupported,
            format::DocDirectiveError::PrereleaseUnmatched { .. } => Self::DocPrereleaseUnmatched,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CemMlParserDiagnosticCatalog {
    fact_bindings: BTreeMap<String, CemMlParserDiagnosticBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CemMlParserDiagnosticBinding {
    fact_kind: String,
    contract: String,
    behavior: Option<String>,
    diagnostic_code: String,
    severity: Severity,
    policy: Option<String>,
}

impl CemMlParserDiagnosticCatalog {
    fn from_builtin() -> Self {
        let source =
            crate::schema::package_sources::builtin_schema_package_source(CEM_ML_PACKAGE_ID)
                .expect("built-in CEM-ML schema package source must be registered");
        Self::from_schema_source(source.schema_source)
    }

    fn from_schema_source(schema_source: &str) -> Self {
        let model = compile_schema_document_model(CEM_ML_SCHEMA_URI, schema_source);
        let fact_bindings = model
            .constraints
            .values()
            .filter_map(|constraint| {
                let behavior = constraint.behavior.as_deref()?.trim();
                if !matches!(
                    behavior,
                    CEM_ML_AST_REPORT_BEHAVIOR | CEM_ML_DOC_REPORT_BEHAVIOR
                ) {
                    return None;
                }
                let fact_kind = constraint.fact_kind.as_deref()?.trim();
                if fact_kind.is_empty() {
                    return None;
                }
                let diagnostic_code = constraint.diagnostic.as_deref()?.trim();
                if diagnostic_code.is_empty() {
                    return None;
                }
                let diagnostic = model.diagnostics.get(diagnostic_code)?;
                Some((
                    fact_kind.to_owned(),
                    CemMlParserDiagnosticBinding {
                        fact_kind: fact_kind.to_owned(),
                        contract: constraint.kind.clone(),
                        behavior: constraint.behavior.clone(),
                        diagnostic_code: diagnostic.code.clone(),
                        severity: diagnostic.severity,
                        policy: constraint.policy.clone(),
                    },
                ))
            })
            .collect();

        Self { fact_bindings }
    }

    fn binding_for_fact(&self, kind: CemMlParserFactKind) -> Option<&CemMlParserDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

fn builtin_cem_ml_parser_diagnostic_catalog() -> &'static CemMlParserDiagnosticCatalog {
    static CATALOG: OnceLock<CemMlParserDiagnosticCatalog> = OnceLock::new();
    CATALOG.get_or_init(CemMlParserDiagnosticCatalog::from_builtin)
}

fn cem_ml_parser_fact_diagnostic(
    kind: CemMlParserFactKind,
    binding: &CemMlParserDiagnosticBinding,
    byte_offset: Option<u64>,
    message: String,
    source_map: Option<SourceMapStack>,
) -> Diagnostic {
    Diagnostic {
        uri: None,
        line: None,
        column: None,
        byte_offset,
        code: binding.diagnostic_code.clone(),
        severity: binding.severity,
        message,
        node: None,
        details: Some(json!({
            "contract": binding.contract,
            "behavior": binding.behavior,
            "factKind": kind.as_str(),
            "policy": binding.policy,
            "sourceRange": {
                "byteOffset": byte_offset,
            },
        })),
        source_map,
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

    fn parse_top_level(input: &str) -> CemDocument {
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        CemAstBuilder::new(normalizer).top_level(true).build()
    }

    fn parse_with_catalog(input: &str, catalog: CemMlParserDiagnosticCatalog) -> CemDocument {
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        CemAstBuilder::new(normalizer)
            .with_cem_ml_parser_diagnostic_catalog(catalog)
            .build()
    }

    fn parse_top_level_with_catalog(
        input: &str,
        catalog: CemMlParserDiagnosticCatalog,
    ) -> CemDocument {
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        CemAstBuilder::new(normalizer)
            .top_level(true)
            .with_cem_ml_parser_diagnostic_catalog(catalog)
            .build()
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
        let diagnostic = doc
            .diagnostics
            .iter()
            .find(|d| d.code == "cem.ast.unresolved_reference")
            .expect("schema-owned unresolved reference diagnostic");
        assert_eq!(diagnostic.severity, Severity::Warning);
        assert_eq!(
            diagnostic.details.as_ref().unwrap()["factKind"],
            "ast-unresolved-reference"
        );
    }

    #[test]
    fn cem_ml_ast_fact_diagnostic_bindings_are_schema_declared_by_fact_kind() {
        let catalog = CemMlParserDiagnosticCatalog::from_builtin();
        let cases = [
            (
                CemMlParserFactKind::AstUnbalancedClose,
                AST_UNBALANCED_CLOSE_CONTRACT,
                "cem.ast.unbalanced_close",
                Severity::Error,
            ),
            (
                CemMlParserFactKind::AstUnclosedScope,
                AST_UNCLOSED_SCOPE_CONTRACT,
                "cem.ast.unclosed_scope",
                Severity::Error,
            ),
            (
                CemMlParserFactKind::AstUnresolvedReference,
                AST_UNRESOLVED_REFERENCE_CONTRACT,
                "cem.ast.unresolved_reference",
                Severity::Warning,
            ),
        ];

        for (fact_kind, contract, diagnostic_code, severity) in cases {
            let binding = catalog
                .binding_for_fact(fact_kind)
                .unwrap_or_else(|| panic!("schema binding for {}", fact_kind.as_str()));
            assert_eq!(binding.fact_kind, fact_kind.as_str());
            assert_eq!(binding.contract, contract);
            assert_eq!(
                binding.behavior.as_deref(),
                Some(CEM_ML_AST_REPORT_BEHAVIOR)
            );
            assert_eq!(binding.diagnostic_code, diagnostic_code);
            assert_eq!(binding.severity, severity);
        }
    }

    #[test]
    fn cem_ml_ast_diagnostic_code_and_severity_are_schema_owned() {
        let source = crate::schema::package_sources::builtin_schema_package_source(CEM_ML_PACKAGE_ID)
            .expect("CEM-ML package source")
            .schema_source
            .replace(
                r#"{constraint @kind="ast-unresolved-reference" @target="attribute" @diagnostic="cem.ast.unresolved_reference" @behavior="cem-ml-ast-report-fact" @fact-kind="ast-unresolved-reference" @policy="AST reference slots remain neutral facts until schema policy decides their diagnostic disposition"}"#,
                r#"{constraint @kind="ast-unresolved-reference" @target="attribute" @diagnostic="example.ast.reference" @behavior="cem-ml-ast-report-fact" @fact-kind="ast-unresolved-reference" @policy="AST reference slots remain neutral facts until schema policy decides their diagnostic disposition"}"#,
            )
            .replace(
                r#"{diagnostic @code="cem.ast.unresolved_reference" @severity="warning"}"#,
                r#"{diagnostic @code="example.ast.reference" @severity="error"}"#,
            );
        let catalog = CemMlParserDiagnosticCatalog::from_schema_source(&source);
        let doc = parse_with_catalog(r#"{label @for=missing | Missing}"#, catalog);
        let diagnostic = doc
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "example.ast.reference")
            .expect("mutated schema-owned AST diagnostic");

        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(
            diagnostic.details.as_ref().unwrap()["contract"],
            AST_UNRESOLVED_REFERENCE_CONTRACT
        );
        assert_eq!(
            diagnostic.details.as_ref().unwrap()["factKind"],
            "ast-unresolved-reference"
        );
    }

    #[test]
    fn cem_ml_ast_unbound_fact_stays_neutral() {
        let source =
            crate::schema::package_sources::builtin_schema_package_source(CEM_ML_PACKAGE_ID)
                .expect("CEM-ML package source")
                .schema_source
                .replace(
                    r#"@fact-kind="ast-unresolved-reference""#,
                    r#"@fact-kind="schema-ignored-ast-reference""#,
                );
        let catalog = CemMlParserDiagnosticCatalog::from_schema_source(&source);
        assert!(catalog
            .binding_for_fact(CemMlParserFactKind::AstUnresolvedReference)
            .is_none());

        let doc = parse_with_catalog(r#"{label @for=missing | Missing}"#, catalog);
        assert!(
            doc.diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "cem.ast.unresolved_reference"),
            "unbound AST facts stay neutral instead of falling back to Rust-owned diagnostics: {:?}",
            doc.diagnostics
        );
    }

    #[test]
    fn cem_ml_doc_fact_diagnostic_bindings_are_schema_declared_by_fact_kind() {
        let catalog = CemMlParserDiagnosticCatalog::from_builtin();
        let cases = [
            (
                CemMlParserFactKind::DocVersionMissing,
                DOC_VERSION_MISSING_CONTRACT,
                format::VERSION_MISSING_CODE,
                Severity::Error,
            ),
            (
                CemMlParserFactKind::DocSemverInvalid,
                DOC_SEMVER_INVALID_CONTRACT,
                "cem.doc.semver_invalid",
                Severity::Error,
            ),
            (
                CemMlParserFactKind::DocFormatUnknown,
                DOC_FORMAT_UNKNOWN_CONTRACT,
                "cem.doc.format_unknown",
                Severity::Error,
            ),
            (
                CemMlParserFactKind::DocVersionUnsupported,
                DOC_VERSION_UNSUPPORTED_CONTRACT,
                "cem.doc.version_unsupported",
                Severity::Error,
            ),
            (
                CemMlParserFactKind::DocPrereleaseUnmatched,
                DOC_PRERELEASE_UNMATCHED_CONTRACT,
                "cem.doc.prerelease_unmatched",
                Severity::Error,
            ),
            (
                CemMlParserFactKind::DocVersionResolved,
                DOC_VERSION_RESOLVED_CONTRACT,
                format::VERSION_RESOLVED_CODE,
                Severity::Info,
            ),
        ];

        for (fact_kind, contract, diagnostic_code, severity) in cases {
            let binding = catalog
                .binding_for_fact(fact_kind)
                .unwrap_or_else(|| panic!("schema binding for {}", fact_kind.as_str()));
            assert_eq!(binding.fact_kind, fact_kind.as_str());
            assert_eq!(binding.contract, contract);
            assert_eq!(
                binding.behavior.as_deref(),
                Some(CEM_ML_DOC_REPORT_BEHAVIOR)
            );
            assert_eq!(binding.diagnostic_code, diagnostic_code);
            assert_eq!(binding.severity, severity);
        }
    }

    #[test]
    fn top_level_doc_resolution_emits_schema_owned_fact_details() {
        let doc = parse_top_level("@doc cem-ml 1\n{p | ok}");
        let diagnostic = doc
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == format::VERSION_RESOLVED_CODE)
            .expect("schema-owned @doc success diagnostic");

        assert_eq!(diagnostic.severity, Severity::Info);
        assert_eq!(
            diagnostic.details.as_ref().unwrap()["contract"],
            DOC_VERSION_RESOLVED_CONTRACT
        );
        assert_eq!(
            diagnostic.details.as_ref().unwrap()["factKind"],
            "doc-version-resolved"
        );
    }

    #[test]
    fn cem_ml_doc_diagnostic_code_and_severity_are_schema_owned() {
        let source = crate::schema::package_sources::builtin_schema_package_source(CEM_ML_PACKAGE_ID)
            .expect("CEM-ML package source")
            .schema_source
            .replace(
                r#"{constraint @kind="doc-version-unsupported" @target="directive" @diagnostic="cem.doc.version_unsupported" @behavior="cem-ml-doc-report-fact" @fact-kind="doc-version-unsupported" @policy="@doc version constraints must resolve against the embedded CEM-ML document profile"}"#,
                r#"{constraint @kind="doc-version-unsupported" @target="directive" @diagnostic="example.doc.version" @behavior="cem-ml-doc-report-fact" @fact-kind="doc-version-unsupported" @policy="@doc version constraints must resolve against the embedded CEM-ML document profile"}"#,
            )
            .replace(
                r#"{diagnostic @code="cem.doc.version_unsupported" @severity="error"}"#,
                r#"{diagnostic @code="example.doc.version" @severity="warning"}"#,
            );
        let catalog = CemMlParserDiagnosticCatalog::from_schema_source(&source);
        let doc = parse_top_level_with_catalog("@doc cem-ml 2\n{p | no}", catalog);
        let diagnostic = doc
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "example.doc.version")
            .expect("mutated schema-owned @doc diagnostic");

        assert_eq!(diagnostic.severity, Severity::Warning);
        assert_eq!(
            diagnostic.details.as_ref().unwrap()["contract"],
            DOC_VERSION_UNSUPPORTED_CONTRACT
        );
        assert_eq!(
            diagnostic.details.as_ref().unwrap()["factKind"],
            "doc-version-unsupported"
        );
    }

    #[test]
    fn cem_ml_doc_unbound_fact_stays_neutral() {
        let source =
            crate::schema::package_sources::builtin_schema_package_source(CEM_ML_PACKAGE_ID)
                .expect("CEM-ML package source")
                .schema_source
                .replace(
                    r#"@fact-kind="doc-version-unsupported""#,
                    r#"@fact-kind="schema-ignored-doc-version""#,
                );
        let catalog = CemMlParserDiagnosticCatalog::from_schema_source(&source);
        assert!(catalog
            .binding_for_fact(CemMlParserFactKind::DocVersionUnsupported)
            .is_none());

        let doc = parse_top_level_with_catalog("@doc cem-ml 2\n{p | no}", catalog);
        assert!(
            doc.diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "cem.doc.version_unsupported"),
            "unbound @doc facts stay neutral instead of falling back to Rust-owned diagnostics: {:?}",
            doc.diagnostics
        );
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
