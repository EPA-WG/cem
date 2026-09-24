//! Lexically scoped CEM template whitespace controls, before instruction lowering.
//!
//! Keep the tokenizer's byte ranges. Suppressed layout becomes zero-width text
//! so its provenance survives the existing portable IR without emitting spaces.
use super::{frame_for, render_diagnostic, Diagnostic, SchemaToken, SchemaTokenKind};

#[derive(Clone, Copy, Default)]
enum Policy {
    #[default]
    Legacy,
    Layout,
    Preserve,
}

struct Scope {
    policy: Policy,
    header: bool,
    declared: bool,
}

pub(super) fn lower(tokens: &mut [SchemaToken]) -> Vec<Diagnostic> {
    let mut scopes: Vec<Scope> = Vec::new();
    let mut diagnostics = Vec::new();
    for token in tokens {
        match &token.kind {
            SchemaTokenKind::NodeStart { .. } | SchemaTokenKind::AnonymousScopeStart => {
                let policy = scopes.last_mut().map_or(Policy::Legacy, |scope| {
                    scope.header = false;
                    scope.policy
                });
                scopes.push(Scope {
                    policy,
                    header: true,
                    declared: false,
                });
            }
            SchemaTokenKind::NodeEnd { .. } => {
                scopes.pop();
            }
            SchemaTokenKind::Attribute { name, value, .. } if name == "cem:whitespace" => {
                let Some(scope) = scopes.last_mut() else {
                    continue;
                };
                let policy = match value.as_deref() {
                    Some("layout") => Some(Policy::Layout),
                    Some("preserve") => Some(Policy::Preserve),
                    _ => None,
                };
                if scope.declared || !scope.header || policy.is_none() {
                    diagnostics.push(render_diagnostic(
                        "cem.ql.render.whitespace_policy_invalid",
                        "@cem:whitespace requires one static layout or preserve value per node"
                            .into(),
                        token.byte_range.start,
                        frame_for(token),
                    ));
                } else if let Some(policy) = policy {
                    scope.policy = policy;
                }
                scope.declared = true;
                // Every instruction's header reader already consumes trivia.
                // The control never becomes a runtime/output attribute.
                token.kind = SchemaTokenKind::Trivia(String::new());
            }
            SchemaTokenKind::Attribute { .. } => {}
            SchemaTokenKind::Trivia(text) if text == "|" => {
                if let Some(scope) = scopes.last_mut() {
                    scope.header = false;
                }
            }
            SchemaTokenKind::Trivia(text) => {
                if let Some(scope) = scopes.last().filter(|scope| !scope.header) {
                    match scope.policy {
                        Policy::Preserve => token.kind = SchemaTokenKind::Text(text.clone()),
                        Policy::Layout if is_layout(text) => {
                            token.kind = SchemaTokenKind::Text(String::new());
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                if let Some(scope) = scopes.last_mut() {
                    scope.header = false;
                }
            }
        }
    }
    diagnostics
}

fn is_layout(text: &str) -> bool {
    text.contains(['\r', '\n'])
        && text
            .bytes()
            .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
}
