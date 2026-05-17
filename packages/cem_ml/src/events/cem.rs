//! CEM event normalizer.
//!
//! Lowers `SchemaToken` (from any profile) into the shared `NormalizedEvent`
//! stream defined in `cem-ml-stack-design-impl.md` §3.3. Tier A maps tokens
//! to the eight shared event categories: open scope, close scope, name,
//! value, trivia, separator, mode switch, and error. Directives lower as
//! `OpenScope` named `@<directive>` followed by a `Value` and a synthetic
//! `CloseScope`; this keeps directives addressable through the same
//! traversal as elements without a directive-specific event variant.

use crate::events::{
    EventNormalizer, HandoffRecord, InheritedContext, NormalizedEvent, QName, ReturnCondition,
    ScalarValue, SeparatorKind, Synthesis, TriviaKind,
};
use crate::source::ByteRange;
use crate::source_map::SourceMapStack;
use crate::tokenizer::{SchemaToken, SchemaTokenKind, SchemaTokenizer};
use std::collections::VecDeque;

pub struct CemEventNormalizer<T: SchemaTokenizer> {
    tokenizer: T,
    pending: VecDeque<NormalizedEvent>,
}

impl<T: SchemaTokenizer> CemEventNormalizer<T> {
    pub fn new(tokenizer: T) -> Self {
        Self {
            tokenizer,
            pending: VecDeque::new(),
        }
    }

    fn fill(&mut self) {
        while self.pending.is_empty() {
            let Some(token) = self.tokenizer.next_token() else {
                return;
            };
            self.lower(token);
        }
    }

    fn lower(&mut self, token: SchemaToken) {
        let SchemaToken {
            kind,
            byte_range,
            source_map,
            ..
        } = token;
        match kind {
            SchemaTokenKind::NodeStart { name } => {
                self.pending.push_back(NormalizedEvent::OpenScope {
                    name: qname(&name, byte_range),
                    byte_range,
                    source_map,
                });
            }
            SchemaTokenKind::NodeEnd { name } => {
                self.pending.push_back(NormalizedEvent::CloseScope {
                    name: qname(name.as_deref().unwrap_or(""), byte_range),
                    byte_range,
                    synthesis: Synthesis::Real,
                    source_map,
                });
            }
            SchemaTokenKind::Attribute {
                name,
                value,
                name_range,
                value_range,
            } => {
                self.pending.push_back(NormalizedEvent::Name {
                    name: qname(&name, name_range),
                    byte_range: name_range,
                });
                if let Some(v) = value.clone() {
                    self.pending.push_back(NormalizedEvent::Value {
                        value: ScalarValue::Text(v),
                        byte_range: value_range.unwrap_or(byte_range),
                    });
                }
                // `@type=...` selects the active child content type
                // (`cem-ml-syntax.md` §"Content-Type Handoffs Stay Schema-Owned").
                // Emit a ModeSwitch alongside the Name/Value pair so the
                // schema machine can act on it without rescanning attribute
                // values.
                if name == "type" {
                    if let Some(v) = value {
                        let ct = v.trim_matches('"').to_owned();
                        self.pending.push_back(NormalizedEvent::ModeSwitch {
                            content_type: ct.clone(),
                            handoff: HandoffRecord {
                                content_type: ct,
                                schema_id: None,
                                source_span: byte_range,
                                inherited_context: InheritedContext {
                                    schema_id: None,
                                    namespace_uri: None,
                                    // The schema machine fills the actual
                                    // parent close offset when it owns the
                                    // open frame for this handoff.
                                    parent_close_byte_offset: None,
                                },
                                return_condition: ReturnCondition::ParentScopeClose,
                            },
                        });
                    }
                }
            }
            SchemaTokenKind::Text(data) => {
                if !data.is_empty() {
                    self.pending.push_back(NormalizedEvent::Value {
                        value: ScalarValue::Text(data),
                        byte_range,
                    });
                }
            }
            SchemaTokenKind::Trivia(data) => {
                // `|` content-boundary marker lowers to a Separator; pure
                // whitespace lowers to Trivia(Whitespace).
                if data == "|" {
                    self.pending.push_back(NormalizedEvent::Separator {
                        kind: SeparatorKind::ElementBoundary,
                        byte_range,
                    });
                } else {
                    self.pending.push_back(NormalizedEvent::Trivia {
                        kind: TriviaKind::Whitespace,
                        byte_range,
                    });
                }
                let _ = data;
            }
            SchemaTokenKind::Comment(_) => {
                self.pending.push_back(NormalizedEvent::Trivia {
                    kind: TriviaKind::Comment,
                    byte_range,
                });
            }
            SchemaTokenKind::ProcessingInstruction { target, data } => {
                self.pending
                    .push_back(NormalizedEvent::ProcessingInstruction {
                        target,
                        data,
                        byte_range,
                    });
            }
            SchemaTokenKind::ExpressionNode(body) => {
                // The wrapping `{$` NodeStart and matching `}` NodeEnd are
                // already in the token stream — they bracket this Value.
                self.pending.push_back(NormalizedEvent::Value {
                    value: ScalarValue::Text(body),
                    byte_range,
                });
            }
            SchemaTokenKind::AnonymousScopeStart => {
                // Anonymous scopes lower as OpenScope with an empty local
                // name; the schema machine treats them as parser/policy
                // boundaries.
                self.pending.push_back(NormalizedEvent::OpenScope {
                    name: qname("", byte_range),
                    byte_range,
                    source_map,
                });
            }
            SchemaTokenKind::Directive { name, data } => {
                let dir_name = format!("@{name}");
                let head_source = source_map.clone();
                self.pending.push_back(NormalizedEvent::OpenScope {
                    name: qname(&dir_name, byte_range),
                    byte_range,
                    source_map: head_source,
                });
                if !data.is_empty() {
                    self.pending.push_back(NormalizedEvent::Value {
                        value: ScalarValue::Text(data),
                        byte_range,
                    });
                }
                self.pending.push_back(NormalizedEvent::CloseScope {
                    name: qname(&dir_name, byte_range),
                    byte_range,
                    synthesis: Synthesis::ImpliedByEof,
                    source_map,
                });
            }
            SchemaTokenKind::RichContent { data } => {
                self.pending.push_back(NormalizedEvent::Value {
                    value: ScalarValue::Text(data),
                    byte_range,
                });
            }
            SchemaTokenKind::Error { code } => {
                self.pending.push_back(NormalizedEvent::Error {
                    code,
                    byte_range,
                    severity: crate::diagnostics::Severity::Error,
                });
            }
        }
    }
}

impl<T: SchemaTokenizer> EventNormalizer for CemEventNormalizer<T> {
    fn next_event(&mut self) -> Option<NormalizedEvent> {
        self.fill();
        self.pending.pop_front()
    }
}

fn qname(raw: &str, source_range: ByteRange) -> QName {
    let (prefix, local) = match raw.split_once(':') {
        Some((p, l)) => (Some(p.to_owned()), l.to_owned()),
        None => (None, raw.to_owned()),
    };
    QName {
        lexical_name: raw.to_owned(),
        prefix,
        local_name: local,
        source_range,
    }
}

/// Convenience helper for tests and the future schema machine: collect every
/// event from a normalizer.
pub fn collect_events<T: SchemaTokenizer>(
    mut normalizer: CemEventNormalizer<T>,
) -> Vec<NormalizedEvent> {
    let mut out = Vec::new();
    while let Some(ev) = normalizer.next_event() {
        out.push(ev);
    }
    out
}

// Silence "unused import": `SourceMapStack` is part of the public-API
// signature even when no in-module code constructs one.
#[allow(dead_code)]
fn _types_assert(_: SourceMapStack) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{BytesSource, SourceId};
    use crate::tokenizer::cem::CemTokenizer;

    fn events_for(input: &str) -> Vec<NormalizedEvent> {
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        collect_events(CemEventNormalizer::new(tok))
    }

    fn shapes(evts: &[NormalizedEvent]) -> Vec<&'static str> {
        evts.iter()
            .filter(|e| !matches!(e, NormalizedEvent::Trivia { .. }))
            .map(|e| match e {
                NormalizedEvent::OpenScope { .. } => "Open",
                NormalizedEvent::CloseScope { .. } => "Close",
                NormalizedEvent::Name { .. } => "Name",
                NormalizedEvent::Value { .. } => "Value",
                NormalizedEvent::Trivia { .. } => "Trivia",
                NormalizedEvent::ProcessingInstruction { .. } => "PI",
                NormalizedEvent::Separator { .. } => "Sep",
                NormalizedEvent::ModeSwitch { .. } => "Mode",
                NormalizedEvent::Error { .. } => "Error",
            })
            .collect()
    }

    #[test]
    fn node_with_attr_and_content_normalizes_to_open_name_value_sep_value_close() {
        let e = events_for(r#"{p @id=x | Hi}"#);
        assert_eq!(
            shapes(&e),
            vec!["Open", "Name", "Value", "Sep", "Value", "Close"]
        );
    }

    #[test]
    fn directive_lowers_to_open_value_close() {
        let e = events_for("@doc cem-ml 1");
        // shapes filters trivia; directives produce Open + Value + Close.
        let mut iter = e.iter();
        assert!(matches!(iter.next(), Some(NormalizedEvent::OpenScope { name, .. }) if name.lexical_name == "@doc"));
        assert!(matches!(
            iter.next(),
            Some(NormalizedEvent::Value {
                value: ScalarValue::Text(t),
                ..
            }) if t == "cem-ml 1"
        ));
        assert!(matches!(
            iter.next(),
            Some(NormalizedEvent::CloseScope { name, .. }) if name.lexical_name == "@doc"
        ));
    }

    #[test]
    fn type_directive_emits_mode_switch() {
        let e = events_for(r#"{@type="text/html" | x}"#);
        assert!(e
            .iter()
            .any(|ev| matches!(ev, NormalizedEvent::ModeSwitch { content_type, .. } if content_type == "text/html")));
    }

    #[test]
    fn bare_brace_text_yields_error_event() {
        let e = events_for("{p Hello {.x}}");
        assert!(e
            .iter()
            .any(|ev| matches!(ev, NormalizedEvent::Error { code, .. } if code == "cem.tokenizer.bare_brace_text")));
    }

    #[test]
    fn expression_node_emits_open_value_close() {
        let e = events_for("{$ .name}");
        let mut iter = e.iter().filter(|e| !matches!(e, NormalizedEvent::Trivia { .. }));
        assert!(matches!(iter.next(), Some(NormalizedEvent::OpenScope { name, .. }) if name.lexical_name == "$"));
        assert!(matches!(
            iter.next(),
            Some(NormalizedEvent::Value {
                value: ScalarValue::Text(t),
                ..
            }) if t == ".name"
        ));
        assert!(matches!(
            iter.next(),
            Some(NormalizedEvent::CloseScope { name, .. }) if name.lexical_name == "$"
        ));
    }

    #[test]
    fn source_map_frames_carry_through_to_events() {
        let e = events_for("{p x}");
        let first_open = e
            .iter()
            .find(|ev| matches!(ev, NormalizedEvent::OpenScope { .. }))
            .unwrap();
        if let NormalizedEvent::OpenScope { source_map, .. } = first_open {
            assert!(!source_map.frames.is_empty());
            assert!(matches!(
                source_map.frames[0].transform,
                crate::source_map::TransformKind::CemTokenizer
            ));
        }
    }
}
