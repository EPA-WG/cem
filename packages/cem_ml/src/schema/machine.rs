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
use crate::schema::vocab::CompiledSchema;
use crate::schema::{FramePhase, SchemaFrame, SchemaMachine, SchemaVersionIdentity, ScopeId};
use crate::source::ByteRange;
use crate::source_map::SourceMapStack;

pub struct CemSchemaMachine<E: EventNormalizer> {
    schema: CompiledSchema,
    events: E,
    frames: Vec<SchemaFrame>,
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

    /// Drain the entire event stream. Returns the diagnostics produced;
    /// the final frame stack is available via [`frames`].
    pub fn run(mut self) -> SchemaMachineOutcome {
        while !self.finished {
            match self.events.next_event() {
                Some(ev) => self.consume(ev),
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
            NormalizedEvent::Error { code, byte_range, severity } => {
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
            schema_version: SchemaVersionIdentity {
                schema_id: self.schema.schema_id,
                major: 1,
                minor: 0,
                patch: 0,
            },
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
        self.pending_attr = None;
        self.pending_states.clear();
        self.pending_annotation = None;
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
        // Pop every handoff owned by the frame that's closing. A child
        // parser cannot consume past the parent's close — this enforces
        // that bound at the schema layer.
        if let Some(prior_depth) = self.handoff_depths.pop() {
            while self.handoffs.depth() > prior_depth {
                self.handoffs.pop();
            }
        }
        // States collected for this scope are validated at close, against
        // the annotation seen on this same frame. (Annotation validation
        // already happened at value-time.)
        let active_annotation = self.pending_annotation.as_ref().map(|ann| ann.local.clone());
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
        let Some(attr) = self.pending_attr.take() else {
            // Values outside an attribute name → content text. Ignored at
            // schema layer; the parser layer keeps them on AST nodes.
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
                    message: format!("`cem:{}` is not part of the active CEM Core vocabulary", ann.local),
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
        let out =
            run_schema(r#"{button @cem:action=primary @cem:state="loading" | Save}"#);
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
        let out =
            run_schema(r#"{button @cem:action=primary @cem:state="selected" | Save}"#);
        assert!(out
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.schema.state_not_allowed_for_role"));
    }

    #[test]
    fn multiple_states_in_one_attribute_are_validated_independently() {
        let out = run_schema(
            r#"{button @cem:action=primary @cem:state="loading hover" | Save}"#,
        );
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
        assert!(out.frames.is_empty(), "frames not drained: {:?}", out.frames);
    }

    #[test]
    fn all_canonical_fixtures_schema_validate_clean() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/cem-ml");
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
        use crate::events::{
            HandoffRecord, InheritedContext, ReturnCondition,
        };
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
        assert!(!stack.within_bounds(40), "40 is the close boundary; not consumable");
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
    fn non_streamable_constraints_emit_unsupported_constraint() {
        use crate::schema::vocab::{NonStreamableConstraint, NonStreamableKind};
        let mut schema = CompiledSchema::cem_core();
        schema.non_streamable_constraints.push(NonStreamableConstraint {
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
