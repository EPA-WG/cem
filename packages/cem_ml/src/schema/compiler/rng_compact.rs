//! RELAX NG compact-syntax mirror emitter (AC-S-2).
//!
//! Structurally equivalent to `rng_xml.rs`; the surface form is
//! RELAX NG compact (`.rnc`). The two emitters share the same
//! `EmissionCursor`, so the same annotation/state ordering applies.
//!
//! `cem-host` is a single `element *` pattern with every CEM
//! annotation as an optional attribute and `cem:state` validated
//! against the schema-wide state matrix. See `rng_xml.rs` for the
//! AC-S-2 vs AC-S-8 scope boundary (per-annotation state narrowing is
//! a semantic rule, not a structural one) and the single-host
//! rationale (libxml2 cannot disambiguate a choice of `element *`
//! variants).

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

        // `cem-host` — a single `element *` pattern. Attributes in a
        // compact-syntax group (`,`) are order-independent, so no `&`
        // (interleave) is needed; using `,` also keeps the converted
        // grammar inside libxml2's validator subset.
        let mut host_parts = vec!["host-pass-through-attrs".to_owned()];
        host_parts.extend(
            cursor
                .annotations()
                .keys()
                .map(|local| format!("cem-attr-{local}?")),
        );
        host_parts.push("cem-attr-state?".to_owned());
        host_parts.push("cem-host-children".to_owned());
        w.line(&format!(
            "cem-host = element * {{ {} }}",
            host_parts.join(", ")
        ))?;
        w.blank();

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

        // `cem:state` is a whitespace-separated token list; each token
        // must be a known state from the schema-wide matrix.
        // Per-annotation narrowing is an AC-S-8 semantic rule.
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
    fn host_is_a_single_element_pattern_with_optional_annotations() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        assert!(body.contains("start = cem-host"));
        // No per-annotation `cem-host-{x}` variants — single host.
        assert!(!body.contains("cem-host-unannotated"));
        assert!(!body.contains("cem-host-badge ="));

        let host = body
            .lines()
            .find(|l| l.starts_with("cem-host = "))
            .expect("cem-host define line");
        assert!(host.starts_with("cem-host = element * {"));
        assert!(host.contains("host-pass-through-attrs"));
        for local in ["action", "badge", "message", "screen"] {
            assert!(
                host.contains(&format!("cem-attr-{local}?")),
                "cem-host missing optional cem-attr-{local}: {host}"
            );
        }
        assert!(host.contains("cem-attr-state?"));
        assert!(host.contains("cem-host-children"));
        assert!(body.contains("cem-host-children = (cem-host | text)*"));
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
        assert!(define.contains("list {"));
    }

    #[test]
    fn no_annotation_scoped_state_defines_are_emitted() {
        // Per-annotation state narrowing is an AC-S-8 semantic rule.
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        assert!(!body.contains("cem-attr-state-badge"));
        assert!(!body.contains("cem-attr-state-action"));
    }

    #[test]
    fn pass_through_attrs_exclude_active_cem_namespace() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        assert!(body.contains("host-pass-through-attrs = host-pass-through-attr*"));
        assert!(body.contains("host-pass-through-attr = attribute (* - cem:*) { text }"));
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
