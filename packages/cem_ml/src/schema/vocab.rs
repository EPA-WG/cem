//! Compiled CEM Core schema vocabulary.
//!
//! Source of truth: [`../../schema/cem-core.md`](../../schema/cem-core.md).
//! Tier A constructs this `CompiledSchema` programmatically; a future
//! markdown-driven compiler reads `cem-core.md` and produces the same shape.

use std::collections::BTreeMap;

pub const CEM_CORE_NAMESPACE: &str = "https://cem.dev/ns/core/1";
pub const CEM_CORE_SCHEMA_ID: u32 = 1;

#[derive(Debug, Clone)]
pub struct AnnotationDef {
    pub local_name: &'static str,
    /// `Some(values)` means the value must be in the enum; `None` means any
    /// non-empty string is accepted (free-form id).
    pub allowed_values: Option<Vec<&'static str>>,
    /// Known values for autocomplete / tooling even when `allowed_values`
    /// is `None`. The compiler does not reject values absent from this
    /// list when `allowed_values` is `None`.
    pub known_values: Vec<&'static str>,
    pub allowed_states: Vec<&'static str>,
}

#[derive(Debug, Clone, Default)]
pub struct CompiledSchema {
    pub schema_id: u32,
    pub namespace_uri: &'static str,
    pub annotations: BTreeMap<&'static str, AnnotationDef>,
    pub state_matrix: Vec<&'static str>,
    /// Reserved: rules that would require non-streamable evaluation are
    /// rejected at compile time with `cem.schema.unsupported_constraint`.
    /// Tier A authors no such rules.
    pub non_streamable_constraints: Vec<NonStreamableConstraint>,
}

#[derive(Debug, Clone)]
pub struct NonStreamableConstraint {
    pub annotation: &'static str,
    pub kind: NonStreamableKind,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonStreamableKind {
    AttributeOrderNonAdjacent,
    CrossScopePredicate,
    FullDocumentBuffering,
}

impl CompiledSchema {
    /// Build the active Tier A schema mirroring `schema/cem-core.md`.
    pub fn cem_core() -> Self {
        let mut annotations: BTreeMap<&'static str, AnnotationDef> = BTreeMap::new();

        annotations.insert(
            "screen",
            AnnotationDef {
                local_name: "screen",
                allowed_values: None,
                known_values: vec![
                    "login",
                    "registration",
                    "profile",
                    "assets",
                    "message-thread",
                ],
                allowed_states: vec!["default", "loading", "empty"],
            },
        );
        annotations.insert(
            "form",
            AnnotationDef {
                local_name: "form",
                allowed_values: None,
                known_values: vec![
                    "sign-in",
                    "registration",
                    "asset-filter",
                    "profile-preferences",
                    "message-reply",
                ],
                allowed_states: vec!["default", "disabled", "invalid", "loading"],
            },
        );
        annotations.insert(
            "action",
            AnnotationDef {
                local_name: "action",
                allowed_values: Some(vec!["primary", "secondary"]),
                known_values: vec!["primary", "secondary"],
                allowed_states: vec![
                    "default",
                    "hover",
                    "focus-visible",
                    "active",
                    "disabled",
                    "loading",
                ],
            },
        );
        annotations.insert(
            "badge",
            AnnotationDef {
                local_name: "badge",
                allowed_values: Some(vec!["success", "info", "warning", "error"]),
                known_values: vec!["success", "info", "warning", "error"],
                allowed_states: vec!["default"],
            },
        );
        annotations.insert(
            "card",
            AnnotationDef {
                local_name: "card",
                allowed_values: None,
                known_values: vec!["identity", "preferences", "summary"],
                allowed_states: vec!["default", "selected", "loading", "empty"],
            },
        );
        annotations.insert(
            "list",
            AnnotationDef {
                local_name: "list",
                allowed_values: None,
                known_values: vec!["assets", "results", "notifications"],
                allowed_states: vec!["default", "loading", "empty"],
            },
        );
        annotations.insert(
            "row",
            AnnotationDef {
                local_name: "row",
                allowed_values: None,
                known_values: vec!["asset", "result", "notification"],
                allowed_states: vec![
                    "default",
                    "hover",
                    "focus-visible",
                    "selected",
                    "disabled",
                ],
            },
        );
        annotations.insert(
            "thread",
            AnnotationDef {
                local_name: "thread",
                allowed_values: None,
                known_values: vec!["support", "notifications"],
                allowed_states: vec!["default", "loading", "empty"],
            },
        );
        annotations.insert(
            "message",
            AnnotationDef {
                local_name: "message",
                allowed_values: Some(vec!["sent", "received"]),
                known_values: vec!["sent", "received"],
                allowed_states: vec!["default"],
            },
        );

        Self {
            schema_id: CEM_CORE_SCHEMA_ID,
            namespace_uri: CEM_CORE_NAMESPACE,
            annotations,
            state_matrix: vec![
                "default",
                "hover",
                "focus-visible",
                "active",
                "selected",
                "disabled",
                "invalid",
                "required",
                "loading",
                "empty",
            ],
            non_streamable_constraints: Vec::new(),
        }
    }

    /// Returns the annotation definition for the given local name (i.e.
    /// the part after `cem:`), or `None` if the name is not part of the
    /// active vocabulary.
    pub fn annotation(&self, local: &str) -> Option<&AnnotationDef> {
        self.annotations.get(local)
    }

    pub fn is_known_state(&self, state: &str) -> bool {
        self.state_matrix.contains(&state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cem_core_includes_every_fixture_annotation() {
        let s = CompiledSchema::cem_core();
        for name in [
            "screen", "form", "action", "badge", "card", "list", "row", "thread", "message",
        ] {
            assert!(s.annotation(name).is_some(), "missing annotation: {name}");
        }
    }

    #[test]
    fn enum_annotations_carry_allowed_values() {
        let s = CompiledSchema::cem_core();
        let action = s.annotation("action").unwrap();
        assert_eq!(
            action.allowed_values.as_deref(),
            Some(["primary", "secondary"].as_slice())
        );
    }

    #[test]
    fn state_matrix_matches_component_mvp() {
        let s = CompiledSchema::cem_core();
        for state in [
            "default",
            "hover",
            "focus-visible",
            "active",
            "selected",
            "disabled",
            "invalid",
            "required",
            "loading",
            "empty",
        ] {
            assert!(s.is_known_state(state), "state matrix missing: {state}");
        }
    }

    #[test]
    fn tier_a_declares_no_non_streamable_constraints() {
        let s = CompiledSchema::cem_core();
        assert!(s.non_streamable_constraints.is_empty());
    }
}
