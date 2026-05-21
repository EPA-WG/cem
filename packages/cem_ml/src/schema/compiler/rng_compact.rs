//! RELAX NG compact-syntax mirror emitter (AC-S-2).
//!
//! Structurally equivalent to `rng_xml.rs`; the surface form is
//! RELAX NG compact (`.rnc`). The two emitters share the same
//! `EmissionCursor`, so the same annotation/state ordering applies.

use super::byte_stability::{rnc_escape, DeterministicWriter};
use super::emitter::{
    reject_non_streamable_constraints, relative_path, EmissionCursor, SchemaEmitter,
};
use super::error::EmitError;
use super::output::{ArtifactKind, EmittedArtifact};
use super::CompilerOptions;
use crate::schema::ir::CompiledSchema;

pub struct RngCompactEmitter;

impl SchemaEmitter for RngCompactEmitter {
    const KIND: ArtifactKind = ArtifactKind::RelaxNgCompact;
    const EXTENSION: &'static str = "rnc";
    const EMITTER_NAME: &'static str = "rng_compact";

    fn emit(
        &self,
        schema: &CompiledSchema,
        options: &CompilerOptions,
        cursor: &mut EmissionCursor<'_>,
    ) -> Result<EmittedArtifact, EmitError> {
        if schema.version_identity.uri.is_empty() {
            return Err(EmitError::MissingIrField {
                field: "version_identity.uri",
            });
        }
        reject_non_streamable_constraints(schema)?;

        let mut w = DeterministicWriter::new();

        if options.embed_source_header {
            w.line(&format!(
                "# AUTO-GENERATED. CEM-native source: {uri} @{ver}",
                uri = schema.version_identity.uri,
                ver = schema
                    .version_identity
                    .embedded_version
                    .to_canonical_string(),
            ))?;
        }

        // Namespace preamble. `default namespace` so `element *` host
        // matches the same set the XML form's `ns=` attribute selects;
        // `namespace cem` exposes the CEM annotation prefix.
        w.line(&format!(
            "default namespace = \"{}\"",
            rnc_escape(&schema.version_identity.uri)
        ))?;
        w.line(&format!(
            "namespace cem = \"{}\"",
            rnc_escape(&schema.version_identity.uri)
        ))?;
        w.blank();

        // Entry pattern.
        w.line("start = cem-host")?;
        w.blank();

        // `cem-host` — choice of unannotated host or annotation-anchored
        // variants whose `cem:state` pattern is scoped to that
        // annotation's allowed states.
        let mut variants = vec!["cem-host-unannotated".to_owned()];
        variants.extend(
            cursor
                .annotations()
                .keys()
                .map(|local| format!("cem-host-{local}")),
        );
        w.line(&format!("cem-host = {}", variants.join(" | ")))?;
        w.blank();

        emit_host_variant(&mut w, "cem-host-unannotated", None, cursor)?;
        for (local, def) in cursor.annotations() {
            emit_host_variant(&mut w, &format!("cem-host-{local}"), Some(local), cursor)?;
            emit_state_attr_define(
                &mut w,
                &format!("cem-attr-state-{local}"),
                &def.allowed_states,
            )?;
        }

        w.line("cem-host-children = (cem-host | text)*")?;
        w.line("host-pass-through-attrs = host-pass-through-attr*")?;
        w.line("host-pass-through-attr = attribute (* - cem:*) { text }")?;
        w.blank();

        // Per-annotation defines.
        for (local, def) in cursor.annotations() {
            match &def.allowed_values {
                Some(values) => {
                    let joined = values
                        .iter()
                        .map(|v| format!("\"{}\"", rnc_escape(v)))
                        .collect::<Vec<_>>()
                        .join(" | ");
                    w.line(&format!(
                        "cem-attr-{local} = attribute cem:{local} {{ {joined} }}"
                    ))?;
                }
                None => {
                    w.line(&format!(
                        "cem-attr-{local} = attribute cem:{local} {{ text }}"
                    ))?;
                }
            }
        }

        // State-only host fallback mirrors the native machine's current
        // behavior: `cem:state` with no active annotation is checked only
        // against the global state matrix.
        emit_state_attr_define(&mut w, "cem-attr-state", cursor.state_matrix())?;

        let (bytes, content_hash) = w.finalize()?;
        Ok(EmittedArtifact {
            kind: ArtifactKind::RelaxNgCompact,
            relative_path: relative_path(schema, ArtifactKind::RelaxNgCompact)?,
            bytes,
            content_hash,
            source_map: Default::default(),
        })
    }
}

fn emit_host_variant(
    w: &mut DeterministicWriter,
    define_name: &str,
    required_annotation: Option<&str>,
    cursor: &EmissionCursor<'_>,
) -> Result<(), EmitError> {
    let attrs = if let Some(local) = required_annotation {
        let mut parts = vec![
            "host-pass-through-attrs".to_owned(),
            format!("cem-attr-{local}"),
        ];
        parts.extend(
            cursor
                .annotations()
                .keys()
                .filter(|other| **other != local)
                .map(|other| format!("cem-attr-{other}?")),
        );
        parts.push(format!("cem-attr-state-{local}?"));
        parts.join(", ")
    } else {
        "host-pass-through-attrs, cem-attr-state?".to_owned()
    };

    w.line(&format!(
        "{define_name} = element * {{ {attrs}, cem-host-children }}"
    ))?;
    Ok(())
}

fn emit_state_attr_define(
    w: &mut DeterministicWriter,
    define_name: &str,
    states: &[&'static str],
) -> Result<(), EmitError> {
    let joined = states
        .iter()
        .map(|s| format!("\"{}\"", rnc_escape(s)))
        .collect::<Vec<_>>()
        .join(" | ");
    w.line(&format!(
        "{define_name} = attribute cem:state {{ list {{ ({joined})+ }} }}"
    ))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emit_cem_core() -> EmittedArtifact {
        let schema = CompiledSchema::cem_core();
        let opts = CompilerOptions::default();
        let mut cursor = EmissionCursor::new(&schema);
        RngCompactEmitter.emit(&schema, &opts, &mut cursor).unwrap()
    }

    #[test]
    fn header_uses_hash_comment_and_carries_uri_plus_version() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        assert!(body.starts_with("# AUTO-GENERATED"));
        assert!(body.contains("CEM-native source: https://cem.dev/ns/core/1 @1.0.0"));
        // No content-hash line per OQ-SC-8.
        assert!(!body.contains("Content hash"));
    }

    #[test]
    fn header_omitted_when_disabled() {
        let schema = CompiledSchema::cem_core();
        let opts = CompilerOptions {
            embed_source_header: false,
            ..Default::default()
        };
        let mut cursor = EmissionCursor::new(&schema);
        let body = String::from_utf8(
            RngCompactEmitter
                .emit(&schema, &opts, &mut cursor)
                .unwrap()
                .bytes,
        )
        .unwrap();
        assert!(!body.contains("AUTO-GENERATED"));
        assert!(body.starts_with("default namespace = "));
    }

    #[test]
    fn namespace_preamble_declares_default_and_cem_prefix() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        assert!(body.contains(r#"default namespace = "https://cem.dev/ns/core/1""#));
        assert!(body.contains(r#"namespace cem = "https://cem.dev/ns/core/1""#));
    }

    #[test]
    fn entry_pattern_and_host_definition_present() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        assert!(body.contains("start = cem-host"));
        assert!(body.contains("cem-host = cem-host-unannotated | cem-host-action"));
        assert!(body.contains("cem-host-action = element * {"));
        assert!(body.contains("(cem-host | text)*"));
    }

    #[test]
    fn enum_annotation_uses_pipe_separated_string_literals() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        assert!(
            body.contains(r#"cem-attr-action = attribute cem:action { "primary" | "secondary" }"#)
        );
        assert!(body.contains(
            r#"cem-attr-badge = attribute cem:badge { "success" | "info" | "warning" | "error" }"#
        ));
    }

    #[test]
    fn free_form_annotation_uses_text_token() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        assert!(body.contains("cem-attr-screen = attribute cem:screen { text }"));
        assert!(body.contains("cem-attr-card = attribute cem:card { text }"));
    }

    #[test]
    fn cem_state_define_lists_full_state_matrix() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        // Look for every state name in the cem-attr-state pattern.
        let define = body
            .lines()
            .find(|l| l.starts_with("cem-attr-state = "))
            .expect("cem-attr-state define line");
        for s in [
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
            assert!(
                define.contains(&format!("\"{s}\"")),
                "cem-attr-state line missing state {s}: {define}"
            );
        }
    }

    #[test]
    fn pass_through_attrs_exclude_active_cem_namespace() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        assert!(body.contains("host-pass-through-attrs = host-pass-through-attr*"));
        assert!(body.contains("host-pass-through-attr = attribute (* - cem:*) { text }"));
    }

    #[test]
    fn state_attributes_are_annotation_scoped_lists() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        let badge_state = body
            .lines()
            .find(|line| line.starts_with("cem-attr-state-badge = "))
            .expect("badge state define");
        assert!(badge_state.contains("list {"));
        assert!(badge_state.contains(r#""default""#));
        assert!(
            !badge_state.contains(r#""loading""#),
            "badge must not accept loading state: {badge_state}"
        );

        let action_state = body
            .lines()
            .find(|line| line.starts_with("cem-attr-state-action = "))
            .expect("action state define");
        assert!(action_state.contains(r#""loading""#));
        assert!(action_state.contains(r#""hover""#));
        assert!(
            !action_state.contains(r#""selected""#),
            "action must not accept selected state: {action_state}"
        );
    }

    #[test]
    fn host_variants_anchor_state_to_present_annotation() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        let action_host = body
            .lines()
            .find(|line| line.starts_with("cem-host-action = "))
            .expect("cem-host-action define");
        assert!(action_host.contains("host-pass-through-attrs"));
        assert!(action_host.contains("cem-attr-action"));
        assert!(action_host.contains("cem-attr-state-action?"));
        assert!(action_host.contains("cem-attr-badge?"));

        let unannotated = body
            .lines()
            .find(|line| line.starts_with("cem-host-unannotated = "))
            .expect("cem-host-unannotated define");
        assert!(unannotated.contains("cem-attr-state?"));
        assert!(!unannotated.contains("cem-attr-action"));
    }

    #[test]
    fn byte_stability_two_emits_equal() {
        let a = emit_cem_core();
        let b = emit_cem_core();
        assert_eq!(a.bytes, b.bytes, "rng_compact is not byte-stable");
        assert_eq!(a.content_hash, b.content_hash);
    }

    #[test]
    fn output_is_utf8_lf_no_trailing_whitespace_final_newline() {
        let bytes = emit_cem_core().bytes;
        assert_eq!(*bytes.last().unwrap(), b'\n');
        assert!(!bytes.contains(&b'\r'));
        for line in bytes.split(|&b| b == b'\n') {
            assert!(
                !line.ends_with(b" ") && !line.ends_with(b"\t"),
                "trailing whitespace on line: {:?}",
                std::str::from_utf8(line).unwrap_or("<non-utf8>")
            );
        }
    }

    #[test]
    fn relative_path_points_under_per_version_directory() {
        assert_eq!(emit_cem_core().relative_path, "core/1.0.0/cem-core.rnc");
    }
}
