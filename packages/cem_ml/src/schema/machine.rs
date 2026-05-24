#![allow(clippy::items_after_test_module)]

//! `CemSchemaMachine` — Tier A streaming validator for the CEM Core vocab.
//!
//! Consumes a `NormalizedEvent` stream (Layer 3) and emits `SchemaFrame`
//! transitions (Layer 4) plus diagnostics per the codes in
//! `packages/cem_ml/schema/cem-core.md`.
//!
//! Tier A streaming guarantee: every diagnostic is decidable from the
//! current frame + the incoming event. The machine never buffers unbounded
//! event history; pending state is bounded by the depth of open scopes.

use crate::diagnostics::{Diagnostic, Severity};
use crate::events::{EventNormalizer, HandoffRecord, NormalizedEvent, ScalarValue};
use crate::handoff::{is_supported_content_type, HandoffStack};
use crate::schema::namespace::NsContext;
use crate::schema::scoping::{
    inline_cache_identity, InlineSchemaDeclaration, SchemaScopeContext, SchemaSource,
};
use crate::schema::vocab::CompiledSchema;
use crate::schema::{FramePhase, SchemaFrame, SchemaMachine, ScopeId};
use crate::source::ByteRange;
use crate::source_map::SourceMapStack;

pub struct CemSchemaMachine<E: EventNormalizer> {
    schema: CompiledSchema,
    events: E,
    frames: Vec<SchemaFrame>,
    /// Scope-chain namespace contexts, one per non-directive open frame.
    /// `ns_contexts[0]` is the document root. Directive scopes (`@doc`,
    /// `@ns`, `@default`, `@schema`) do *not* push a context; their
    /// bindings register into the enclosing scope's context.
    ns_contexts: Vec<NsContext>,
    /// Scope-chain schema-scoping context tracking inline `cem:schema`
    /// declarations, mid-document `src`/`select` switches, and host-node
    /// attribute-form switches per AC-F-2 / `cem-ml-stack-design.md`
    /// §13.1. Pushed/popped alongside `ns_contexts` for non-directive
    /// scopes.
    schema_scopes: SchemaScopeContext,
    /// While processing `cem:schema` elements, accumulates declared
    /// attributes per open frame so nested declarations do not overwrite
    /// their parent frame's pending state.
    pending_schema_elements: Vec<Option<PendingSchemaElement>>,
    /// Host-node `cem:schema-src` / `cem:schema-select` state per open
    /// frame. Exclusivity is host-local; inherited active sources must
    /// not make a child host look invalid.
    pending_host_switches: Vec<PendingHostSwitch>,
    /// Tracks the directive currently being consumed (e.g. `@ns`,
    /// `@default`). Cleared on close. The value events between open and
    /// close go into `pending_directive_body`.
    active_directive: Option<DirectiveKind>,
    pending_directive_body: String,
    pending_directive_open: Option<ByteRange>,
    handoffs: HandoffStack,
    /// One entry per open frame: depth of `handoffs` when the frame opened,
    /// so close can pop any handoffs the frame owned without losing track
    /// of outer ones.
    handoff_depths: Vec<usize>,
    diagnostics: Vec<Diagnostic>,
    next_scope_id: ScopeId,
    /// While walking an element's attributes, this holds the
    /// pending-annotation lookup (annotation local name + value range +
    /// optional value) so we can attach it to the frame when the element's
    /// content starts.
    pending_attr: Option<PendingAttr>,
    /// State attribute values queued for the active frame before its
    /// `phase` flips to `Content`.
    pending_states: Vec<PendingState>,
    /// Tracks the annotation currently being assembled on the open frame.
    pending_annotation: Option<PendingAnnotation>,
    finished: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirectiveKind {
    Ns,
    Default,
    Doc,
    Schema,
    Other,
}

#[derive(Debug, Clone)]
struct PendingAttr {
    name: String,
    name_range: ByteRange,
}

#[derive(Debug, Clone)]
struct PendingState {
    value: String,
    byte_range: ByteRange,
}

#[derive(Debug, Clone)]
struct PendingSchemaElement {
    open_byte_range: ByteRange,
    cem_name: Option<String>,
    src: Option<String>,
    select: Option<String>,
    /// Tracks whether the element introduced its own scope (always true
    /// in Tier A — the schema machine treats every `cem:schema` element
    /// as a scope; sibling-form vs wrapping-form is decided by whether
    /// the element has body content). Reserved for the Phase 11
    /// self-closing form.
    #[allow(dead_code)]
    is_self_closing: bool,
}

#[derive(Debug, Clone)]
struct PendingHostSwitch {
    src: Option<String>,
    select: Option<String>,
}

impl PendingHostSwitch {
    fn empty() -> Self {
        Self {
            src: None,
            select: None,
        }
    }
}

#[derive(Debug, Clone)]
struct PendingAnnotation {
    local: String,
    value: Option<String>,
    name_range: ByteRange,
    value_range: Option<ByteRange>,
}

impl<E: EventNormalizer> CemSchemaMachine<E> {
    pub fn new(schema: CompiledSchema, events: E) -> Self {
        Self {
            schema,
            events,
            frames: Vec::new(),
            ns_contexts: vec![NsContext::new(0)],
            schema_scopes: SchemaScopeContext::new(),
            pending_schema_elements: Vec::new(),
            pending_host_switches: Vec::new(),
            active_directive: None,
            pending_directive_body: String::new(),
            pending_directive_open: None,
            handoffs: HandoffStack::default(),
            handoff_depths: Vec::new(),
            diagnostics: Vec::new(),
            next_scope_id: 1,
            pending_attr: None,
            pending_states: Vec::new(),
            pending_annotation: None,
            finished: false,
        }
    }

    /// Returns the active `NsContext` (the top of the scope chain).
    /// Available for downstream layers that need namespace resolution.
    pub fn current_ns_context(&self) -> &NsContext {
        self.ns_contexts
            .last()
            .expect("ns_contexts has document root")
    }

    /// Returns the active schema-scoping context (the top of the chain).
    /// Use this to query the active schema source on the current scope
    /// or to resolve a `cem:name` reference walking outward.
    pub fn schema_scopes(&self) -> &SchemaScopeContext {
        &self.schema_scopes
    }

    /// Drain the entire event stream. Returns the diagnostics produced;
    /// the final frame stack is available via [`frames`].
    pub fn run(self) -> SchemaMachineOutcome {
        self.run_with_observer(|_| {})
    }

    /// Drain the event stream, invoking `observe` after every event is
    /// consumed. Useful for integration tests that need to inspect
    /// schema/namespace/scope state mid-stream.
    pub fn run_with_observer<F>(mut self, mut observe: F) -> SchemaMachineOutcome
    where
        F: FnMut(&Self),
    {
        while !self.finished {
            match self.events.next_event() {
                Some(ev) => {
                    self.consume(ev);
                    observe(&self);
                }
                None => {
                    self.finalize();
                    break;
                }
            }
        }
        SchemaMachineOutcome {
            frames: self.frames,
            handoffs_at_eof: self.handoffs.depth(),
            diagnostics: self.diagnostics,
        }
    }

    fn consume(&mut self, event: NormalizedEvent) {
        match event {
            NormalizedEvent::OpenScope {
                name,
                byte_range,
                source_map,
            } => self.on_open(&name.lexical_name, byte_range, source_map),
            NormalizedEvent::CloseScope { name, .. } => {
                self.commit_pending_annotation();
                self.on_close(&name.lexical_name);
            }
            NormalizedEvent::Name { name, byte_range } => {
                // If we were collecting an annotation, the new Name event
                // means the prior attribute is done — finalize it before
                // starting the next.
                self.commit_pending_annotation();
                self.pending_attr = Some(PendingAttr {
                    name: name.lexical_name,
                    name_range: byte_range,
                });
            }
            NormalizedEvent::Value { value, byte_range } => {
                self.on_value(value, byte_range);
            }
            NormalizedEvent::Separator { .. } => {
                self.commit_pending_annotation();
                self.mark_current_schema_element_has_body();
                if let Some(frame) = self.frames.last_mut() {
                    if frame.phase == FramePhase::Attribute || frame.phase == FramePhase::Header {
                        frame.phase = FramePhase::Content;
                    }
                }
            }
            NormalizedEvent::Trivia { .. } | NormalizedEvent::ProcessingInstruction { .. } => {
                // Trivia + PIs are reported but don't change schema state.
            }
            NormalizedEvent::ModeSwitch {
                content_type,
                handoff,
            } => self.on_mode_switch(content_type, handoff),
            NormalizedEvent::Error {
                code,
                byte_range,
                severity,
            } => {
                self.diagnostics.push(Diagnostic {
                    uri: None,
                    line: None,
                    column: None,
                    byte_offset: Some(byte_range.start),
                    code,
                    severity,
                    message: "tokenizer-reported error surfaced into schema stream".to_owned(),
                    node: None,
                    source_map: None,
                });
            }
        }
    }

    fn on_open(&mut self, name: &str, byte_range: ByteRange, source_map: SourceMapStack) {
        self.mark_current_schema_element_has_body();
        let scope_id = self.next_scope_id;
        self.next_scope_id += 1;
        // Tier A applies the active CEM Core schema universally; one schema
        // per frame. Directive scopes (names starting with `@`) carry the
        // directive name as language_id so downstream layers can identify
        // them.
        let language_id = if let Some(rest) = name.strip_prefix('@') {
            format!("directive/{rest}")
        } else {
            "cem-core".to_owned()
        };
        let frame = SchemaFrame {
            scope_id,
            schema_id: self.schema.schema_id,
            schema_version: self.schema.version_identity.clone(),
            language_id,
            phase: FramePhase::Attribute,
            source_span: byte_range,
            source_map_stack: source_map,
            expected_close: if name.is_empty() {
                None
            } else {
                Some(name.to_owned())
            },
        };
        self.frames.push(frame);
        self.handoff_depths.push(self.handoffs.depth());
        self.pending_host_switches.push(PendingHostSwitch::empty());
        let pending_schema = if !name.starts_with('@') && name == "cem:schema" {
            Some(PendingSchemaElement {
                open_byte_range: byte_range,
                cem_name: None,
                src: None,
                select: None,
                is_self_closing: true,
            })
        } else {
            None
        };
        self.pending_schema_elements.push(pending_schema);
        self.pending_attr = None;
        self.pending_states.clear();
        self.pending_annotation = None;
        // Push an NsContext for non-directive scopes; directives don't
        // shift the namespace context, they declare into the enclosing
        // scope.
        if let Some(rest) = name.strip_prefix('@') {
            self.active_directive = Some(directive_kind(rest));
            self.pending_directive_body.clear();
            self.pending_directive_open = Some(byte_range);
        } else {
            let child = match self.ns_contexts.last() {
                Some(parent) => NsContext::child_of(parent, scope_id),
                None => NsContext::new(scope_id),
            };
            self.ns_contexts.push(child);
            self.schema_scopes.push(scope_id);
        }
    }

    fn on_close(&mut self, _name: &str) {
        if self.frames.is_empty() {
            self.diagnostics.push(Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: None,
                code: "cem.schema.unbalanced_close".to_owned(),
                severity: Severity::Error,
                message: "close-scope event with no matching open frame".to_owned(),
                node: None,
                source_map: None,
            });
            return;
        }
        let frame = self.frames.pop().expect("frames non-empty");
        let pending_schema_element = self.pending_schema_elements.pop().unwrap_or(None);
        let _pending_host_switch = self.pending_host_switches.pop();
        // Pop every handoff owned by the frame that's closing. A child
        // parser cannot consume past the parent's close — this enforces
        // that bound at the schema layer.
        if let Some(prior_depth) = self.handoff_depths.pop() {
            while self.handoffs.depth() > prior_depth {
                self.handoffs.pop();
            }
        }
        // Namespace-context bookkeeping. A directive scope closes by
        // committing its declaration into the surrounding scope's
        // NsContext. A non-directive scope pops its own context.
        let is_directive = frame
            .expected_close
            .as_deref()
            .map(|n| n.starts_with('@'))
            .unwrap_or(false);
        if is_directive {
            self.commit_directive(&frame);
            self.active_directive = None;
            self.pending_directive_body.clear();
            self.pending_directive_open = None;
        } else {
            // Commit any pending `cem:schema` element before popping
            // this scope. The pending element's effects apply to the
            // surrounding scope (declaration / src / select switches);
            // the schema-scoping context is then popped.
            if let Some(pending) = pending_schema_element {
                self.commit_schema_element(pending, &frame);
            }
            if self.ns_contexts.len() > 1 {
                self.ns_contexts.pop();
            }
            self.schema_scopes.pop();
        }
        // States collected for this scope are validated at close, against
        // the annotation seen on this same frame. (Annotation validation
        // already happened at value-time.)
        let active_annotation = self
            .pending_annotation
            .as_ref()
            .map(|ann| ann.local.clone());
        for state in std::mem::take(&mut self.pending_states) {
            self.validate_state(&state, active_annotation.as_deref());
        }
        let _ = frame;
        self.pending_annotation = None;
    }

    fn on_mode_switch(&mut self, content_type: String, mut handoff: HandoffRecord) {
        // `@type="..."` only opens a content-type handoff on an anonymous
        // scope per `cem-ml-syntax.md` §"Content-Type Handoffs Stay
        // Schema-Owned". On a named element (`<input type="email">`) it's
        // an ordinary HTML attribute and not a handoff. Detect by
        // checking the active frame's `expected_close`: anonymous scopes
        // have `None`.
        let active_anonymous = self
            .frames
            .last()
            .map(|f| f.expected_close.is_none())
            .unwrap_or(false);
        if !active_anonymous {
            return;
        }
        // Fill the inherited context from the active parent frame. The
        // parent close byte offset is the upper bound the child parser
        // must respect; in Tier A the frame's `source_span.end()` is the
        // best available approximation until the parser fills the
        // expected-close offset.
        let parent_close_byte_offset = self.frames.last().map(|f| f.source_span.end());
        handoff.inherited_context.parent_close_byte_offset = parent_close_byte_offset;
        handoff.inherited_context.schema_id = self.frames.last().map(|f| f.schema_id);

        let span = handoff.source_span;
        if !is_supported_content_type(&content_type) {
            self.diagnostics.push(Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: Some(span.start),
                code: "cem.handoff.unsupported_content_type".to_owned(),
                severity: Severity::Error,
                message: format!(
                    "content type `{content_type}` has no Tier A handoff; region is bounded but not interpreted"
                ),
                node: None,
                source_map: None,
            });
        } else {
            self.diagnostics.push(Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: Some(span.start),
                code: "cem.handoff.child_parser_deferred".to_owned(),
                severity: Severity::Info,
                message: format!(
                    "child parser for `{content_type}` lands in Phase 11; region preserved as opaque text bounded by the parent scope's close"
                ),
                node: None,
                source_map: None,
            });
        }
        self.handoffs.push(handoff);
    }

    fn on_value(&mut self, value: ScalarValue, byte_range: ByteRange) {
        // Directive bodies arrive as Value events; capture them for
        // commit at directive-close time.
        if self.active_directive.is_some() && self.pending_attr.is_none() {
            if let ScalarValue::Text(t) = &value {
                if !self.pending_directive_body.is_empty() {
                    self.pending_directive_body.push(' ');
                }
                self.pending_directive_body.push_str(t);
            }
            return;
        }
        let Some(attr) = self.pending_attr.take() else {
            // Values outside an attribute name → content text. Ignored at
            // schema layer; the parser layer keeps them on AST nodes.
            self.mark_current_schema_element_has_body();
            return;
        };
        let text = match value {
            ScalarValue::Text(t) => t,
            ScalarValue::Int(i) => i.to_string(),
            ScalarValue::Float(f) => f.to_string(),
            ScalarValue::Bool(b) => b.to_string(),
            ScalarValue::Null => String::new(),
        };
        self.handle_attribute(attr, text, byte_range);
    }

    fn handle_attribute(&mut self, attr: PendingAttr, value: String, value_range: ByteRange) {
        // Namespace attribute forms: `xmlns="uri"` rebinds the default
        // binding on the current scope; `xmlns:prefix="uri"` declares a
        // prefix binding. Mirrors XML 1.0 §"Namespaces in XML" and the
        // HTML5 foreign-content handling.
        if attr.name == "xmlns" {
            if let Some(ctx) = self.ns_contexts.last_mut() {
                ctx.declare(
                    "",
                    value,
                    attr.name_range,
                    value_range,
                    SourceMapStack::default(),
                );
            }
            return;
        }
        if let Some(prefix) = attr.name.strip_prefix("xmlns:") {
            if let Some(ctx) = self.ns_contexts.last_mut() {
                ctx.declare(
                    prefix.to_owned(),
                    value,
                    attr.name_range,
                    value_range,
                    SourceMapStack::default(),
                );
            }
            return;
        }
        // Host-node attribute-form schema switches: `cem:schema-src` /
        // `cem:schema-select`. These apply to the *current* scope (the
        // host element's scope, which was already opened) and are
        // mutually exclusive.
        if attr.name == "cem:schema-src" {
            self.apply_host_node_schema_switch(SchemaSource::Uri(value), false, attr.name_range);
            return;
        }
        if attr.name == "cem:schema-select" {
            self.apply_host_node_schema_switch(SchemaSource::Select(value), true, attr.name_range);
            return;
        }
        // `cem:schema` element attributes — record for commit-on-close
        // (so we can validate exclusivity + missing-source at the
        // element boundary) AND apply the switch to the current scope
        // immediately so the element's body sees the new active source.
        if self
            .pending_schema_elements
            .last()
            .and_then(|p| p.as_ref())
            .is_some()
        {
            match attr.name.as_str() {
                "src" => {
                    let already_select = self
                        .pending_schema_elements
                        .last()
                        .and_then(|p| p.as_ref())
                        .and_then(|p| p.select.as_ref())
                        .is_some();
                    if let Some(Some(p)) = self.pending_schema_elements.last_mut() {
                        p.src = Some(value.clone());
                    }
                    if already_select {
                        // commit_schema_element will report the
                        // exclusivity error at close.
                    } else {
                        self.schema_scopes
                            .current_mut()
                            .set_active(SchemaSource::Uri(value));
                    }
                    return;
                }
                "select" => {
                    let already_uri = self
                        .pending_schema_elements
                        .last()
                        .and_then(|p| p.as_ref())
                        .and_then(|p| p.src.as_ref())
                        .is_some();
                    if let Some(Some(p)) = self.pending_schema_elements.last_mut() {
                        p.select = Some(value.clone());
                    }
                    if already_uri {
                        // commit_schema_element will report exclusivity.
                    } else {
                        self.schema_scopes
                            .current_mut()
                            .set_active(SchemaSource::Select(value));
                    }
                    return;
                }
                "cem:name" => {
                    if let Some(Some(p)) = self.pending_schema_elements.last_mut() {
                        p.cem_name = Some(value.clone());
                    }
                    // `cem:name` is the schema-scoping declaration
                    // identifier per AC-F-2 — not a CEM-Core annotation.
                    // Don't route it through the annotation path.
                    return;
                }
                _ => {}
            }
        }

        if let Some(rest) = attr.name.strip_prefix("cem:") {
            if rest == "state" {
                // `cem:state="a b"` may carry multiple state names.
                for part in value.split_whitespace() {
                    self.pending_states.push(PendingState {
                        value: part.to_owned(),
                        byte_range: value_range,
                    });
                }
                return;
            }
            // A CEM annotation.
            self.commit_pending_annotation();
            self.pending_annotation = Some(PendingAnnotation {
                local: rest.to_owned(),
                value: Some(value),
                name_range: attr.name_range,
                value_range: Some(value_range),
            });
        }
        // Host-element attributes (e.g. `id`, `href`, `aria-*`) are not the
        // schema's concern at this layer; the semantic-rule catalog
        // (`AC-V-6`) handles them.
    }

    fn apply_host_node_schema_switch(
        &mut self,
        source: SchemaSource,
        is_select: bool,
        name_range: ByteRange,
    ) {
        // If a switch was already applied at this scope, the second one
        // is the mutual-exclusivity error.
        let Some(host) = self.pending_host_switches.last_mut() else {
            return;
        };
        let existing_is_select = host.select.is_some();
        let existing_is_uri = host.src.is_some();
        if (existing_is_select && !is_select) || (existing_is_uri && is_select) {
            self.diagnostics.push(Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: Some(name_range.start),
                code: "cem.schema.scoping.exclusive_src_select".to_owned(),
                severity: Severity::Error,
                message:
                    "`cem:schema-src` and `cem:schema-select` are mutually exclusive on the same host"
                        .to_owned(),
                node: None,
                source_map: None,
            });
            return;
        }
        match &source {
            SchemaSource::Uri(value) => host.src = Some(value.clone()),
            SchemaSource::Select(value) => host.select = Some(value.clone()),
            _ => {}
        }
        self.schema_scopes.current_mut().set_active(source);
    }

    fn commit_schema_element(&mut self, pending: PendingSchemaElement, frame: &SchemaFrame) {
        let _ = frame;
        // 1. Validate exclusivity: src and select cannot both appear.
        if pending.src.is_some() && pending.select.is_some() {
            self.diagnostics.push(Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: Some(pending.open_byte_range.start),
                code: "cem.schema.scoping.exclusive_src_select".to_owned(),
                severity: Severity::Error,
                message: "`cem:schema` element may carry `src` or `select`, not both".to_owned(),
                node: None,
                source_map: None,
            });
            return;
        }
        let switch_source = match (&pending.src, &pending.select) {
            (Some(uri), None) => Some(SchemaSource::Uri(uri.clone())),
            (None, Some(expr)) => Some(SchemaSource::Select(expr.clone())),
            _ => None,
        };
        // 2. Inline declaration form: `cem:schema cem:name="..."`. The
        // declaration registers into the *parent* scope so descendants
        // can reference it. We pop the schema scope on close after this
        // commit runs (see `on_close`).
        if let Some(name) = pending.cem_name.clone() {
            // Body byte range approximates the inline-schema content;
            // Tier A uses the frame's source_span minus the open header.
            let cache_identity = inline_cache_identity(name.as_bytes());
            let decl = InlineSchemaDeclaration {
                name: name.clone(),
                body_byte_range: pending.open_byte_range,
                cache_identity,
                source_map: frame.source_map_stack.clone(),
            };
            // Register at the parent scope so descendants resolve it.
            // (When the cem:schema element closes, the inner scope is
            // popped; we need to declare into what *will be* the
            // current scope after pop.)
            if self.schema_scopes.depth() >= 2 {
                let parent_idx = self.schema_scopes.depth() - 2;
                // Safe access to parent frame via private API.
                self.declare_inline_at(parent_idx, decl);
            } else {
                self.schema_scopes.current_mut().declare_inline(decl);
            }
        }
        // 3. The src/select switch was already applied to the current
        // scope in `handle_attribute` when the attribute value arrived.
        // The wrapping-form behavior follows naturally from `on_close`
        // popping the schema scope. The no-body form is the sibling
        // switch: apply the same source to the parent scope before this
        // frame pops so subsequent siblings inherit it until parent close.
        if pending.is_self_closing {
            if let Some(source) = switch_source {
                if self.schema_scopes.depth() >= 2 {
                    let parent_idx = self.schema_scopes.depth() - 2;
                    self.set_active_at(parent_idx, source);
                } else {
                    self.schema_scopes.current_mut().set_active(source);
                }
            }
        }
        //
        // 4. Missing-source error: a `cem:schema` element with no `src`,
        // `select`, *or* `cem:name` is a schema-compilation error per
        // AC-F-2.
        if pending.cem_name.is_none() && pending.src.is_none() && pending.select.is_none() {
            self.diagnostics.push(Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: Some(pending.open_byte_range.start),
                code: "cem.schema.scoping.missing_source".to_owned(),
                severity: Severity::Error,
                message: "`cem:schema` element must declare `cem:name`, `src`, or `select`"
                    .to_owned(),
                node: None,
                source_map: None,
            });
        }
    }

    fn mark_current_schema_element_has_body(&mut self) {
        if let Some(Some(pending)) = self.pending_schema_elements.last_mut() {
            pending.is_self_closing = false;
        }
    }

    fn declare_inline_at(&mut self, idx: usize, decl: InlineSchemaDeclaration) {
        // The SchemaScopeContext doesn't expose per-index mutation;
        // implement it via a swap-and-rebuild pattern. For Tier A we
        // pop down to the index, declare, then push back the popped
        // frames. This is O(depth) but acceptable since schema scopes
        // are bounded by AC limits.
        let mut popped = Vec::new();
        while self.schema_scopes.depth() > idx + 1 {
            let frame = self.schema_scopes.current().clone();
            self.schema_scopes.pop();
            popped.push(frame);
        }
        self.schema_scopes.current_mut().declare_inline(decl);
        for frame in popped.into_iter().rev() {
            self.schema_scopes.push(frame.scope_id);
            // Reapply the frame's state (best-effort restore).
            *self.schema_scopes.current_mut() = frame;
        }
    }

    fn set_active_at(&mut self, idx: usize, source: SchemaSource) {
        let mut popped = Vec::new();
        while self.schema_scopes.depth() > idx + 1 {
            let frame = self.schema_scopes.current().clone();
            self.schema_scopes.pop();
            popped.push(frame);
        }
        self.schema_scopes.current_mut().set_active(source);
        for frame in popped.into_iter().rev() {
            self.schema_scopes.push(frame.scope_id);
            *self.schema_scopes.current_mut() = frame;
        }
    }

    fn commit_pending_annotation(&mut self) {
        let Some(ann) = self.pending_annotation.take() else {
            return;
        };
        let def = match self.schema.annotation(&ann.local) {
            Some(def) => def,
            None => {
                self.diagnostics.push(Diagnostic {
                    uri: None,
                    line: None,
                    column: None,
                    byte_offset: Some(ann.name_range.start),
                    code: "cem.schema.unknown_annotation".to_owned(),
                    severity: Severity::Error,
                    message: format!(
                        "`cem:{}` is not part of the active CEM Core vocabulary",
                        ann.local
                    ),
                    node: None,
                    source_map: None,
                });
                return;
            }
        };
        if let Some(value) = &ann.value {
            if let Some(allowed) = &def.allowed_values {
                if !allowed.iter().any(|v| *v == value) {
                    self.diagnostics.push(Diagnostic {
                        uri: None,
                        line: None,
                        column: None,
                        byte_offset: ann
                            .value_range
                            .map(|r| r.start)
                            .or(Some(ann.name_range.start)),
                        code: "cem.schema.unknown_annotation_value".to_owned(),
                        severity: Severity::Error,
                        message: format!(
                            "value `{value}` is not in the Tier A enum for `cem:{}` (allowed: {})",
                            ann.local,
                            allowed.join(", ")
                        ),
                        node: None,
                        source_map: None,
                    });
                }
            }
        }
        // Put the annotation back so closer can read its name for state
        // checking.
        self.pending_annotation = Some(ann);
    }

    fn validate_state(&mut self, state: &PendingState, active_annotation: Option<&str>) {
        if !self.schema.is_known_state(&state.value) {
            self.diagnostics.push(Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: Some(state.byte_range.start),
                code: "cem.schema.disallowed_state".to_owned(),
                severity: Severity::Error,
                message: format!(
                    "`cem:state` value `{}` is not part of the CEM state matrix",
                    state.value
                ),
                node: None,
                source_map: None,
            });
            return;
        }
        let Some(ann) = active_annotation else {
            return;
        };
        let Some(def) = self.schema.annotation(ann) else {
            return;
        };
        if !def.allowed_states.iter().any(|s| *s == state.value) {
            self.diagnostics.push(Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: Some(state.byte_range.start),
                code: "cem.schema.state_not_allowed_for_role".to_owned(),
                severity: Severity::Error,
                message: format!(
                    "state `{}` is not allowed on `cem:{}` (allowed: {})",
                    state.value,
                    ann,
                    def.allowed_states.join(", ")
                ),
                node: None,
                source_map: None,
            });
        }
    }

    fn commit_directive(&mut self, frame: &SchemaFrame) {
        let kind = match self.active_directive {
            Some(k) => k,
            None => return,
        };
        let body = self.pending_directive_body.trim().to_owned();
        if body.is_empty() {
            return;
        }
        let declared_at = self.pending_directive_open.unwrap_or(frame.source_span);
        let source_map = frame.source_map_stack.clone();
        match kind {
            DirectiveKind::Ns => {
                // `prefix = "uri"` or `prefix = uri`
                if let Some((prefix, uri)) = parse_ns_body(&body) {
                    if let Some(ctx) = self.ns_contexts.last_mut() {
                        ctx.declare(
                            prefix,
                            uri,
                            declared_at,
                            ByteRange::new(declared_at.end(), 0),
                            source_map,
                        );
                    }
                } else {
                    self.diagnostics.push(Diagnostic {
                        uri: None,
                        line: None,
                        column: None,
                        byte_offset: Some(declared_at.start),
                        code: "cem.ns.invalid_ns_directive".to_owned(),
                        severity: Severity::Error,
                        message: format!("`@ns` directive body could not be parsed: `{body}`"),
                        node: None,
                        source_map: None,
                    });
                }
            }
            DirectiveKind::Default => {
                // `@default <prefix|uri>` — if the token is a known
                // prefix in the current context, copy its URI; otherwise
                // treat the token as a literal URI.
                let token = body.trim_matches('"').trim().to_owned();
                let uri = self
                    .ns_contexts
                    .last()
                    .and_then(|ctx| ctx.binding(&token).map(|b| b.namespace_uri.clone()))
                    .unwrap_or(token);
                if let Some(ctx) = self.ns_contexts.last_mut() {
                    ctx.declare(
                        "",
                        uri,
                        declared_at,
                        ByteRange::new(declared_at.end(), 0),
                        source_map,
                    );
                }
            }
            DirectiveKind::Schema => match parse_schema_source_body(&body) {
                Ok(source) => self.schema_scopes.current_mut().set_active(source),
                Err(SchemaDirectiveError::ExclusiveSrcSelect) => {
                    self.diagnostics.push(Diagnostic {
                        uri: None,
                        line: None,
                        column: None,
                        byte_offset: Some(declared_at.start),
                        code: "cem.schema.scoping.exclusive_src_select".to_owned(),
                        severity: Severity::Error,
                        message: "`@schema` directive may carry `src` or `select`, not both"
                            .to_owned(),
                        node: None,
                        source_map: None,
                    });
                }
                Err(SchemaDirectiveError::MissingSource) => {
                    self.diagnostics.push(Diagnostic {
                        uri: None,
                        line: None,
                        column: None,
                        byte_offset: Some(declared_at.start),
                        code: "cem.schema.scoping.missing_source".to_owned(),
                        severity: Severity::Error,
                        message: "`@schema` directive must declare `src` or `select`".to_owned(),
                        node: None,
                        source_map: None,
                    });
                }
            },
            DirectiveKind::Doc | DirectiveKind::Other => {
                // Handled elsewhere (document-format identity, future
                // directives).
            }
        }
    }

    fn finalize(&mut self) {
        // Any frames still on the stack at EOF mean unbalanced opens.
        for frame in self.frames.iter() {
            self.diagnostics.push(Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: Some(frame.source_span.start),
                code: "cem.schema.unclosed_scope".to_owned(),
                severity: Severity::Error,
                message: match &frame.expected_close {
                    Some(name) => format!("scope `{}` did not close before EOF", name),
                    None => "anonymous scope did not close before EOF".to_owned(),
                },
                node: None,
                source_map: None,
            });
        }
        // Reject non-streamable constraints at finalize so the diagnostic
        // surfaces even when no real input was consumed.
        for c in &self.schema.non_streamable_constraints {
            self.diagnostics.push(Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: None,
                code: "cem.schema.unsupported_constraint".to_owned(),
                severity: Severity::Error,
                message: format!(
                    "constraint on `cem:{}` is not streamable: {} ({:?})",
                    c.annotation, c.reason, c.kind
                ),
                node: None,
                source_map: None,
            });
        }
        self.finished = true;
    }
}

pub struct SchemaMachineOutcome {
    pub frames: Vec<SchemaFrame>,
    pub handoffs_at_eof: usize,
    pub diagnostics: Vec<Diagnostic>,
}

impl SchemaMachineOutcome {
    pub fn hard_violations(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
            .count()
    }
}

impl<E: EventNormalizer> SchemaMachine for CemSchemaMachine<E> {
    fn current(&self) -> Option<&SchemaFrame> {
        self.frames.last()
    }
    fn frames(&self) -> &[SchemaFrame] {
        &self.frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::cem::CemEventNormalizer;
    use crate::source::{BytesSource, SourceId};
    use crate::tokenizer::cem::CemTokenizer;

    fn run_schema(input: &str) -> SchemaMachineOutcome {
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer).run()
    }

    #[test]
    fn known_annotation_validates() {
        let out = run_schema(r#"{button @cem:action=primary | Save}"#);
        assert_eq!(
            out.hard_violations(),
            0,
            "expected no hard violations, got: {:?}",
            out.diagnostics
        );
    }

    #[test]
    fn unknown_annotation_value_is_flagged() {
        let out = run_schema(r#"{button @cem:action=bogus | Save}"#);
        assert!(out
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.schema.unknown_annotation_value"));
    }

    #[test]
    fn unknown_annotation_is_flagged() {
        let out = run_schema(r#"{button @cem:made-up="x" | Save}"#);
        assert!(out
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.schema.unknown_annotation"));
    }

    #[test]
    fn freeform_id_annotation_accepts_any_string() {
        let out = run_schema(r#"{main @cem:screen="custom-screen" | x}"#);
        assert_eq!(out.hard_violations(), 0, "{:?}", out.diagnostics);
    }

    #[test]
    fn allowed_state_validates() {
        let out = run_schema(r#"{button @cem:action=primary @cem:state="loading" | Save}"#);
        assert_eq!(out.hard_violations(), 0, "{:?}", out.diagnostics);
    }

    #[test]
    fn state_not_in_matrix_is_flagged() {
        let out = run_schema(r#"{button @cem:action=primary @cem:state="bogus" | Save}"#);
        assert!(out
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.schema.disallowed_state"));
    }

    #[test]
    fn state_not_allowed_for_role_is_flagged() {
        // `selected` is in the matrix but not allowed on `cem:action`.
        let out = run_schema(r#"{button @cem:action=primary @cem:state="selected" | Save}"#);
        assert!(out
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.schema.state_not_allowed_for_role"));
    }

    #[test]
    fn multiple_states_in_one_attribute_are_validated_independently() {
        let out = run_schema(r#"{button @cem:action=primary @cem:state="loading hover" | Save}"#);
        assert_eq!(out.hard_violations(), 0, "{:?}", out.diagnostics);
    }

    #[test]
    fn unclosed_scope_at_eof_is_reported() {
        let out = run_schema("{p Hello");
        // Tokenizer flags `cem.tokenizer.unterminated_node`; the schema
        // machine adds `cem.schema.unclosed_scope` for the still-open
        // frame at finalize.
        assert!(out
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.schema.unclosed_scope"));
    }

    #[test]
    fn streaming_frames_track_depth_through_nested_scopes() {
        // After running, the stack should be empty (all closes balanced).
        let out = run_schema("{a | {b | {c | x}}}");
        assert_eq!(out.hard_violations(), 0);
        assert!(
            out.frames.is_empty(),
            "frames not drained: {:?}",
            out.frames
        );
    }

    #[test]
    fn all_canonical_fixtures_schema_validate_clean() {
        let dir =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/cem-ml");
        let mut checked = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("cem") {
                continue;
            }
            let input = std::fs::read_to_string(&path).unwrap();
            let out = run_schema(&input);
            // Hard violations from the schema layer should be zero; we
            // still allow tokenizer-error surfaces if present (none expected
            // for the canonical fixtures).
            let schema_hard: Vec<&Diagnostic> = out
                .diagnostics
                .iter()
                .filter(|d| {
                    d.code.starts_with("cem.schema.")
                        && matches!(d.severity, Severity::Error | Severity::Fatal)
                })
                .collect();
            assert!(
                schema_hard.is_empty(),
                "fixture `{}` schema diagnostics: {schema_hard:?}",
                path.display()
            );
            checked += 1;
        }
        assert!(checked >= 5);
    }

    #[test]
    fn supported_content_type_emits_deferred_info_diag() {
        let out = run_schema(r#"{@type="text/html" | <p>hi</p>}"#);
        let deferred = out
            .diagnostics
            .iter()
            .find(|d| d.code == "cem.handoff.child_parser_deferred")
            .expect("expected child_parser_deferred diag");
        assert_eq!(deferred.severity, Severity::Info);
        assert!(deferred.message.contains("text/html"));
        // No hard violations from a supported handoff.
        let hard: Vec<&Diagnostic> = out
            .diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
            .collect();
        assert!(hard.is_empty(), "expected no hard violations: {hard:?}");
    }

    #[test]
    fn unsupported_content_type_emits_error_but_region_is_bounded() {
        let out = run_schema(r#"{@type="application/x-rocks" | totally opaque }"#);
        let bad = out
            .diagnostics
            .iter()
            .find(|d| d.code == "cem.handoff.unsupported_content_type")
            .expect("expected unsupported_content_type diag");
        assert_eq!(bad.severity, Severity::Error);
        // The scope still closed: no `cem.schema.unclosed_scope` should fire.
        assert!(out
            .diagnostics
            .iter()
            .all(|d| d.code != "cem.schema.unclosed_scope"));
    }

    #[test]
    fn handoff_records_carry_inherited_context_with_parent_close_offset() {
        // Drive the events directly so we can inspect the handoff record.
        use crate::events::{
            HandoffRecord, InheritedContext, NormalizedEvent, QName, ReturnCondition, Synthesis,
        };
        use crate::source::ByteRange;
        use crate::source_map::SourceMapStack;

        struct Replay(Vec<NormalizedEvent>);
        impl crate::events::EventNormalizer for Replay {
            fn next_event(&mut self) -> Option<NormalizedEvent> {
                if self.0.is_empty() {
                    None
                } else {
                    Some(self.0.remove(0))
                }
            }
        }

        let span = ByteRange::new(0, 40);
        let ctx = SourceMapStack::default();
        let qn = |s: &str| QName {
            lexical_name: s.to_owned(),
            prefix: None,
            local_name: s.to_owned(),
            source_range: ByteRange::new(0, s.len() as u32),
        };
        let evts = vec![
            NormalizedEvent::OpenScope {
                name: qn(""),
                byte_range: span,
                source_map: ctx.clone(),
            },
            NormalizedEvent::ModeSwitch {
                content_type: "text/html".into(),
                handoff: HandoffRecord {
                    content_type: "text/html".into(),
                    schema_id: None,
                    source_span: ByteRange::new(1, 16),
                    inherited_context: InheritedContext {
                        schema_id: None,
                        namespace_uri: None,
                        parent_close_byte_offset: None,
                    },
                    return_condition: ReturnCondition::ParentScopeClose,
                },
            },
            NormalizedEvent::CloseScope {
                name: qn(""),
                byte_range: ByteRange::new(39, 1),
                synthesis: Synthesis::Real,
                source_map: ctx,
            },
        ];

        let schema = CompiledSchema::cem_core();
        // Build the machine, but instrument by snooping handoffs via a
        // wrapper: consume events one at a time so we can inspect state
        // mid-stream.
        let mut machine = CemSchemaMachine::new(schema, Replay(evts));
        // Step 1: OpenScope.
        let ev = machine.events.next_event().unwrap();
        machine.consume(ev);
        assert_eq!(machine.frames.len(), 1);
        // Step 2: ModeSwitch.
        let ev = machine.events.next_event().unwrap();
        machine.consume(ev);
        assert_eq!(machine.handoffs.depth(), 1);
        let top = machine.handoffs.top().unwrap();
        assert_eq!(top.content_type, "text/html");
        assert_eq!(top.return_condition, ReturnCondition::ParentScopeClose);
        assert_eq!(
            top.inherited_context.parent_close_byte_offset,
            Some(40),
            "parent close offset should equal opening frame's source_span.end()"
        );
        // Step 3: CloseScope pops the handoff.
        let ev = machine.events.next_event().unwrap();
        machine.consume(ev);
        assert!(machine.handoffs.is_empty(), "handoff should pop on close");
    }

    #[test]
    fn child_parser_cannot_consume_past_parent_close() {
        use crate::events::{HandoffRecord, InheritedContext, ReturnCondition};
        use crate::handoff::HandoffStack;
        let mut stack = HandoffStack::default();
        stack.push(HandoffRecord {
            content_type: "text/html".into(),
            schema_id: None,
            source_span: crate::source::ByteRange::new(10, 30),
            inherited_context: InheritedContext {
                schema_id: None,
                namespace_uri: None,
                parent_close_byte_offset: Some(40),
            },
            return_condition: ReturnCondition::ParentScopeClose,
        });
        assert!(stack.within_bounds(39), "39 < 40 is inside the parent");
        assert!(
            !stack.within_bounds(40),
            "40 is the close boundary; not consumable"
        );
        assert!(!stack.within_bounds(41), "past the close is forbidden");
    }

    #[test]
    fn nested_scopes_pop_only_owned_handoffs() {
        // Outer scope does NOT switch content type; inner scope switches to
        // text/html. Closing the inner scope must pop the handoff;
        // closing the outer scope leaves zero handoffs.
        let input = r#"{outer | {@type="text/html" | body}}"#;
        let out = run_schema(input);
        assert_eq!(out.handoffs_at_eof, 0);
        assert!(out
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.handoff.child_parser_deferred"));
    }

    #[test]
    fn at_ns_directive_populates_ns_context() {
        let input = r#"@ns cem = "https://cem.dev/ns/core/1"
{button @cem:action=primary | Save}"#;
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        let mut machine = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer);
        // Step events until just before EOF so the ns_context still
        // reflects the active state.
        let mut last_ns_uri = None;
        while let Some(ev) = machine.events.next_event() {
            machine.consume(ev);
            if let Some(b) = machine.current_ns_context().binding("cem") {
                last_ns_uri = Some(b.namespace_uri.clone());
            }
        }
        machine.finalize();
        assert_eq!(last_ns_uri.as_deref(), Some("https://cem.dev/ns/core/1"));
    }

    #[test]
    fn at_default_directive_resolves_unprefixed_to_html() {
        let input = r#"@ns html = "http://www.w3.org/1999/xhtml"
@default html
{button | Save}"#;
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        let mut machine = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer);
        let mut default_uri = None;
        while let Some(ev) = machine.events.next_event() {
            machine.consume(ev);
            if let Some(b) = machine.current_ns_context().binding("") {
                default_uri = Some(b.namespace_uri.clone());
            }
        }
        machine.finalize();
        assert_eq!(default_uri.as_deref(), Some("http://www.w3.org/1999/xhtml"));
    }

    #[test]
    fn login_fixture_resolves_cem_and_default_prefixes() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/cem-ml/login.cem");
        let input = std::fs::read_to_string(&path).unwrap();
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        let mut machine = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer);
        let mut cem_uri = None;
        let mut html_uri = None;
        let mut default_uri = None;
        while let Some(ev) = machine.events.next_event() {
            machine.consume(ev);
            if let Some(b) = machine.current_ns_context().binding("cem") {
                cem_uri = Some(b.namespace_uri.clone());
            }
            if let Some(b) = machine.current_ns_context().binding("html") {
                html_uri = Some(b.namespace_uri.clone());
            }
            if let Some(b) = machine.current_ns_context().binding("") {
                default_uri = Some(b.namespace_uri.clone());
            }
        }
        machine.finalize();
        assert_eq!(cem_uri.as_deref(), Some("https://cem.dev/ns/core/1"));
        assert_eq!(html_uri.as_deref(), Some("http://www.w3.org/1999/xhtml"));
        assert_eq!(default_uri.as_deref(), Some("http://www.w3.org/1999/xhtml"));
    }

    #[test]
    fn cem_schema_element_with_src_switches_scope() {
        let input = r#"{section | {cem:schema @src="schema://x" | {p Hi}}}"#;
        let src_bytes = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src_bytes);
        let normalizer = CemEventNormalizer::new(tok);
        let mut machine = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer);

        // Capture active source after every event; the switch applies
        // once the `@src` Value arrives, not at scope open.
        let mut saw_uri = false;
        while let Some(ev) = machine.events.next_event() {
            machine.consume(ev);
            if matches!(
                machine.schema_scopes().current().active,
                SchemaSource::Uri(ref u) if u == "schema://x"
            ) {
                saw_uri = true;
            }
        }
        machine.finalize();
        assert!(
            saw_uri,
            "cem:schema @src=... should switch the active source to Uri"
        );
    }

    #[test]
    fn cem_schema_no_body_switch_applies_to_following_sibling_scope() {
        let input = r#"{section | {cem:schema @src="schema://x"} {p Hi}}"#;
        let src_bytes = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src_bytes);
        let normalizer = CemEventNormalizer::new(tok);
        let mut machine = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer);

        let mut saw_sibling_uri = false;
        while let Some(ev) = machine.events.next_event() {
            machine.consume(ev);
            if matches!(
                machine.schema_scopes().current().active,
                SchemaSource::Uri(ref u) if u == "schema://x"
            ) && machine.schema_scopes().depth() >= 3
            {
                saw_sibling_uri = true;
            }
        }
        machine.finalize();
        assert!(
            saw_sibling_uri,
            "no-body cem:schema @src=... should switch following sibling scope to Uri"
        );
    }

    #[test]
    fn cem_schema_element_with_select_records_select_source() {
        let input = r#"{section | {cem:schema @select=".pred" | {p Hi}}}"#;
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        let mut machine = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer);
        let mut saw_select = false;
        while let Some(ev) = machine.events.next_event() {
            machine.consume(ev);
            if matches!(
                machine.schema_scopes().current().active,
                SchemaSource::Select(_)
            ) {
                saw_select = true;
            }
        }
        machine.finalize();
        assert!(
            saw_select,
            "expected a Select source at some point during the parse"
        );
    }

    #[test]
    fn cem_schema_src_and_select_together_is_an_error() {
        let input = r#"{cem:schema @src="x" @select="y" | body}"#;
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        let outcome = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer).run();
        assert!(outcome
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.schema.scoping.exclusive_src_select"));
    }

    #[test]
    fn cem_schema_with_neither_src_select_nor_name_is_an_error() {
        let input = r#"{cem:schema | body}"#;
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        let outcome = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer).run();
        assert!(outcome
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.schema.scoping.missing_source"));
    }

    #[test]
    fn cem_schema_inline_declaration_registers_into_parent_scope() {
        let input = r#"{section | {cem:schema @cem:name="badge" | body} {p Hi}}"#;
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        let mut machine = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer);
        let mut saw_inline_in_sibling_scope = false;
        while let Some(ev) = machine.events.next_event() {
            machine.consume(ev);
            // After the cem:schema element closes, the sibling `p` scope
            // should see `badge` resolvable.
            if machine.schema_scopes().resolve_name("badge").is_some() {
                saw_inline_in_sibling_scope = true;
            }
        }
        machine.finalize();
        assert!(
            saw_inline_in_sibling_scope,
            "inline `cem:name=\"badge\"` should resolve in subsequent sibling scopes"
        );
    }

    #[test]
    fn host_node_schema_src_attribute_switches_scope() {
        let input = r#"{section @cem:schema-src="schema://x" | body}"#;
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        let mut machine = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer);
        let mut saw_uri = false;
        while let Some(ev) = machine.events.next_event() {
            machine.consume(ev);
            if matches!(
                machine.schema_scopes().current().active,
                SchemaSource::Uri(ref u) if u == "schema://x"
            ) {
                saw_uri = true;
            }
        }
        machine.finalize();
        assert!(
            saw_uri,
            "@cem:schema-src=... should switch the host's scope to Uri"
        );
    }

    #[test]
    fn host_node_schema_src_and_select_together_is_an_error() {
        let input = r#"{section @cem:schema-src="x" @cem:schema-select=".p" | body}"#;
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        let outcome = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer).run();
        assert!(outcome
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.schema.scoping.exclusive_src_select"));
    }

    #[test]
    fn host_node_switch_exclusivity_is_not_triggered_by_inherited_active_source() {
        let input =
            r#"{cem:schema @src="schema://outer" | {section @cem:schema-select=".local" | body}}"#;
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        let outcome = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer).run();
        assert!(
            outcome
                .diagnostics
                .iter()
                .all(|d| d.code != "cem.schema.scoping.exclusive_src_select"),
            "inherited URI source must not conflict with host-local schema-select: {:?}",
            outcome.diagnostics
        );
    }

    #[test]
    fn schema_directive_with_src_sets_document_scope_active_source() {
        let input = r#"@schema src="schema://document/default/1"
{section | body}"#;
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        let mut saw_document_uri = false;
        let outcome = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer)
            .run_with_observer(|m| {
                if matches!(
                    m.schema_scopes().current().active,
                    SchemaSource::Uri(ref u) if u == "schema://document/default/1"
                ) {
                    saw_document_uri = true;
                }
            });
        assert!(
            saw_document_uri,
            "@schema src should set the active source on the document scope"
        );
        assert!(
            outcome
                .diagnostics
                .iter()
                .all(|d| d.code != "cem.schema.scoping.missing_source"),
            "unexpected missing-source diagnostic: {:?}",
            outcome.diagnostics
        );
    }

    #[test]
    fn non_streamable_constraints_emit_unsupported_constraint() {
        use crate::schema::vocab::{NonStreamableConstraint, NonStreamableKind};
        let mut schema = CompiledSchema::cem_core();
        schema
            .non_streamable_constraints
            .push(NonStreamableConstraint {
                annotation: "form",
                kind: NonStreamableKind::FullDocumentBuffering,
                reason: "synthetic test rule",
            });
        let src = BytesSource::new(SourceId(1), b"{p x}".to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        let out = CemSchemaMachine::new(schema, normalizer).run();
        assert!(out
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.schema.unsupported_constraint"));
    }
}

fn directive_kind(name: &str) -> DirectiveKind {
    match name {
        "ns" => DirectiveKind::Ns,
        "default" => DirectiveKind::Default,
        "doc" => DirectiveKind::Doc,
        "schema" => DirectiveKind::Schema,
        _ => DirectiveKind::Other,
    }
}

/// Parse a `@ns` directive body of the form `prefix = "uri"` or
/// `prefix = uri`. Returns `(prefix, uri)` on success.
fn parse_ns_body(body: &str) -> Option<(String, String)> {
    let (left, right) = body.split_once('=')?;
    let prefix = left.trim().to_owned();
    if prefix.is_empty() {
        return None;
    }
    let mut uri = right.trim().to_owned();
    let is_double = uri.starts_with('"') && uri.ends_with('"') && uri.len() >= 2;
    let is_single = uri.starts_with('\'') && uri.ends_with('\'') && uri.len() >= 2;
    if is_double || is_single {
        uri = uri[1..uri.len() - 1].to_owned();
    }
    if uri.is_empty() {
        return None;
    }
    Some((prefix, uri))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchemaDirectiveError {
    ExclusiveSrcSelect,
    MissingSource,
}

fn parse_schema_source_body(body: &str) -> Result<SchemaSource, SchemaDirectiveError> {
    let mut src: Option<String> = None;
    let mut select: Option<String> = None;
    for (key, value) in parse_directive_attrs(body) {
        match key.as_str() {
            "src" => src = Some(value),
            "select" => select = Some(value),
            _ => {}
        }
    }
    match (src, select) {
        (Some(_), Some(_)) => Err(SchemaDirectiveError::ExclusiveSrcSelect),
        (Some(uri), None) => Ok(SchemaSource::Uri(uri)),
        (None, Some(expr)) => Ok(SchemaSource::Select(expr)),
        (None, None) => {
            let fallback = trim_directive_value(body);
            if fallback.is_empty() {
                Err(SchemaDirectiveError::MissingSource)
            } else {
                Ok(SchemaSource::Uri(fallback))
            }
        }
    }
}

fn parse_directive_attrs(body: &str) -> Vec<(String, String)> {
    let chars: Vec<char> = body.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let key_start = i;
        while i < chars.len()
            && !chars[i].is_whitespace()
            && chars[i] != '='
            && chars[i] != '"'
            && chars[i] != '\''
        {
            i += 1;
        }
        let key: String = chars[key_start..i].iter().collect();
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if key.is_empty() || i >= chars.len() || chars[i] != '=' {
            while i < chars.len() && !chars[i].is_whitespace() {
                i += 1;
            }
            continue;
        }
        i += 1;
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let value = if i < chars.len() && (chars[i] == '"' || chars[i] == '\'') {
            let quote = chars[i];
            i += 1;
            let value_start = i;
            while i < chars.len() && chars[i] != quote {
                i += 1;
            }
            let value: String = chars[value_start..i].iter().collect();
            if i < chars.len() {
                i += 1;
            }
            value
        } else {
            let value_start = i;
            while i < chars.len() && !chars[i].is_whitespace() {
                i += 1;
            }
            chars[value_start..i].iter().collect()
        };
        out.push((key, value));
    }
    out
}

fn trim_directive_value(body: &str) -> String {
    let mut value = body.trim().to_owned();
    let is_double = value.starts_with('"') && value.ends_with('"') && value.len() >= 2;
    let is_single = value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2;
    if is_double || is_single {
        value = value[1..value.len() - 1].to_owned();
    }
    value
}
