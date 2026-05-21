//! RELAX NG XML mirror emitter (AC-S-2).
//!
//! Produces a single `<grammar>` document for the active CEM-native
//! schema. For cem-core/1 the grammar describes:
//!
//! - `cem-host` — any element accepting pass-through non-CEM attributes,
//!   known CEM annotations, annotation-scoped `cem:state` lists, and
//!   child `cem-host` elements (RELAX NG `<anyName/>`).
//! - One `<define name="cem-attr-{annotation}"/>` per annotation, with
//!   `<choice>` over `<value>` literals for enum-valued annotations and
//!   `<text/>` for free-form annotations.
//! - `<define name="cem-attr-state-{annotation}"/>` mapped to that
//!   annotation's allowed state list; `<define name="cem-attr-state"/>`
//!   remains the schema-wide state-only fallback.
//!
//! Determinism notes (§13.2.4):
//! - UTF-8, LF, single trailing newline, no trailing whitespace.
//! - Annotation order = `BTreeMap` iteration (alphabetical).
//! - Value choice order = the order recorded on `AnnotationDef`
//!   (cem-core/1 authors list values in a stable order; the spec
//!   leaves further sort policy to the schema source).

use super::byte_stability::{xml_escape, DeterministicWriter};
use super::emitter::{
    reject_non_streamable_constraints, relative_path, EmissionCursor, SchemaEmitter,
};
use super::error::EmitError;
use super::output::{ArtifactKind, EmittedArtifact};
use super::CompilerOptions;
use crate::schema::ir::CompiledSchema;

pub struct RngXmlEmitter;

impl SchemaEmitter for RngXmlEmitter {
    const KIND: ArtifactKind = ArtifactKind::RelaxNgXml;
    const EXTENSION: &'static str = "rng";
    const EMITTER_NAME: &'static str = "rng_xml";

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

        w.line(r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;

        if options.embed_source_header {
            let header = format!(
                "<!-- AUTO-GENERATED. CEM-native source: {uri} @{ver} -->",
                uri = xml_escape(&schema.version_identity.uri),
                ver = xml_escape(
                    &schema
                        .version_identity
                        .embedded_version
                        .to_canonical_string()
                ),
            );
            w.line(&header)?;
        }

        // Grammar open tag. Attribute order per §13.2.4 rule 6 — fixed
        // preamble (`xmlns`, `xmlns:cem`, `ns`, then alphabetised
        // remainder).
        w.line(&format!(
            r#"<grammar xmlns="http://relaxng.org/ns/structure/1.0" xmlns:cem="{cem}" ns="{ns}" datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes">"#,
            cem = xml_escape(&schema.version_identity.uri),
            ns = xml_escape(&schema.version_identity.uri),
        ))?;
        w.indent();

        // Entry point.
        w.line(r#"<start>"#)?;
        w.indent();
        w.line(r#"<ref name="cem-host"/>"#)?;
        w.dedent();
        w.line(r#"</start>"#)?;

        // `cem-host` — choice of an unannotated host, or a host anchored
        // by each known CEM annotation. Anchored variants validate
        // `cem:state` against that annotation's allowed state set while
        // still permitting other known CEM annotations on the same host.
        w.line(r#"<define name="cem-host">"#)?;
        w.indent();
        w.line(r#"<choice>"#)?;
        w.indent();
        w.line(r#"<ref name="cem-host-unannotated"/>"#)?;
        for local in cursor.annotations().keys() {
            w.line(&format!(r#"<ref name="cem-host-{local}"/>"#))?;
        }
        w.dedent();
        w.line(r#"</choice>"#)?;
        w.dedent();
        w.line(r#"</define>"#)?;

        emit_host_variant(&mut w, "cem-host-unannotated", None, cursor)?;
        for (local, def) in cursor.annotations() {
            emit_host_variant(&mut w, &format!("cem-host-{local}"), Some(local), cursor)?;
            emit_state_attr_define(
                &mut w,
                &format!("cem-attr-state-{local}"),
                &def.allowed_states,
            )?;
        }

        w.line(r#"<define name="cem-host-children">"#)?;
        w.indent();
        w.line(r#"<zeroOrMore>"#)?;
        w.indent();
        w.line(r#"<choice>"#)?;
        w.indent();
        w.line(r#"<ref name="cem-host"/>"#)?;
        w.line(r#"<text/>"#)?;
        w.dedent();
        w.line(r#"</choice>"#)?;
        w.dedent();
        w.line(r#"</zeroOrMore>"#)?;
        w.dedent();
        w.line(r#"</define>"#)?;

        // Non-CEM attributes are pass-through. Unknown active-CEM
        // namespace attributes are intentionally excluded and therefore
        // rejected unless a known `cem-attr-*` pattern consumes them.
        w.line(r#"<define name="host-pass-through-attrs">"#)?;
        w.indent();
        w.line(r#"<zeroOrMore><ref name="host-pass-through-attr"/></zeroOrMore>"#)?;
        w.dedent();
        w.line(r#"</define>"#)?;

        w.line(r#"<define name="host-pass-through-attr">"#)?;
        w.indent();
        w.line(r#"<attribute>"#)?;
        w.indent();
        w.line(r#"<anyName>"#)?;
        w.indent();
        w.line(r#"<except>"#)?;
        w.indent();
        w.line(&format!(
            r#"<nsName ns="{}"/>"#,
            xml_escape(&schema.version_identity.uri)
        ))?;
        w.dedent();
        w.line(r#"</except>"#)?;
        w.dedent();
        w.line(r#"</anyName>"#)?;
        w.line(r#"<text/>"#)?;
        w.dedent();
        w.line(r#"</attribute>"#)?;
        w.dedent();
        w.line(r#"</define>"#)?;

        // Per-annotation defines.
        for (local, def) in cursor.annotations() {
            w.line(&format!(r#"<define name="cem-attr-{local}">"#))?;
            w.indent();
            w.line(&format!(r#"<attribute name="cem:{local}">"#))?;
            w.indent();
            match &def.allowed_values {
                Some(values) => {
                    w.line(r#"<choice>"#)?;
                    w.indent();
                    for value in values {
                        w.line(&format!(r#"<value>{}</value>"#, xml_escape(value)))?;
                    }
                    w.dedent();
                    w.line(r#"</choice>"#)?;
                }
                None => {
                    w.line(r#"<text/>"#)?;
                }
            }
            w.dedent();
            w.line(r#"</attribute>"#)?;
            w.dedent();
            w.line(r#"</define>"#)?;
        }

        // State-only host fallback mirrors the native machine's current
        // behavior: `cem:state` with no active annotation is checked only
        // against the global state matrix.
        emit_state_attr_define(&mut w, "cem-attr-state", cursor.state_matrix())?;

        w.dedent();
        w.line(r#"</grammar>"#)?;

        let (bytes, content_hash) = w.finalize()?;
        Ok(EmittedArtifact {
            kind: ArtifactKind::RelaxNgXml,
            relative_path: relative_path(schema, ArtifactKind::RelaxNgXml)?,
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
    w.line(&format!(r#"<define name="{define_name}">"#))?;
    w.indent();
    w.line(r#"<element>"#)?;
    w.indent();
    w.line(r#"<anyName/>"#)?;
    w.line(r#"<interleave>"#)?;
    w.indent();
    w.line(r#"<ref name="host-pass-through-attrs"/>"#)?;
    if let Some(local) = required_annotation {
        w.line(&format!(r#"<ref name="cem-attr-{local}"/>"#))?;
        for other in cursor.annotations().keys().filter(|other| **other != local) {
            w.line(&format!(
                r#"<optional><ref name="cem-attr-{other}"/></optional>"#
            ))?;
        }
        w.line(&format!(
            r#"<optional><ref name="cem-attr-state-{local}"/></optional>"#
        ))?;
    } else {
        w.line(r#"<optional><ref name="cem-attr-state"/></optional>"#)?;
    }
    w.dedent();
    w.line(r#"</interleave>"#)?;
    w.line(r#"<ref name="cem-host-children"/>"#)?;
    w.dedent();
    w.line(r#"</element>"#)?;
    w.dedent();
    w.line(r#"</define>"#)?;
    Ok(())
}

fn emit_state_attr_define(
    w: &mut DeterministicWriter,
    define_name: &str,
    states: &[&'static str],
) -> Result<(), EmitError> {
    w.line(&format!(r#"<define name="{define_name}">"#))?;
    w.indent();
    w.line(r#"<attribute name="cem:state">"#)?;
    w.indent();
    w.line(r#"<list>"#)?;
    w.indent();
    w.line(r#"<oneOrMore>"#)?;
    w.indent();
    w.line(r#"<choice>"#)?;
    w.indent();
    for state in states {
        w.line(&format!(r#"<value>{}</value>"#, xml_escape(state)))?;
    }
    w.dedent();
    w.line(r#"</choice>"#)?;
    w.dedent();
    w.line(r#"</oneOrMore>"#)?;
    w.dedent();
    w.line(r#"</list>"#)?;
    w.dedent();
    w.line(r#"</attribute>"#)?;
    w.dedent();
    w.line(r#"</define>"#)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emit_cem_core() -> EmittedArtifact {
        let schema = CompiledSchema::cem_core();
        let opts = CompilerOptions::default();
        let mut cursor = EmissionCursor::new(&schema);
        RngXmlEmitter.emit(&schema, &opts, &mut cursor).unwrap()
    }

    #[test]
    fn output_starts_with_xml_decl_then_header() {
        let a = emit_cem_core();
        let body = std::str::from_utf8(&a.bytes).unwrap();
        assert!(body.starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
        assert!(
            body.contains("AUTO-GENERATED. CEM-native source: https://cem.dev/ns/core/1 @1.0.0")
        );
        // No content-hash line — OQ-SC-8 (resolved).
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
        let a = RngXmlEmitter.emit(&schema, &opts, &mut cursor).unwrap();
        let body = std::str::from_utf8(&a.bytes).unwrap();
        assert!(!body.contains("AUTO-GENERATED"));
    }

    #[test]
    fn grammar_declares_cem_namespace_and_default_ns() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        assert!(body.contains(r#"xmlns="http://relaxng.org/ns/structure/1.0""#));
        assert!(body.contains(r#"xmlns:cem="https://cem.dev/ns/core/1""#));
        assert!(body.contains(r#"ns="https://cem.dev/ns/core/1""#));
    }

    #[test]
    fn every_annotation_has_a_define() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        for local in [
            "screen", "form", "action", "badge", "card", "list", "row", "thread", "message",
        ] {
            assert!(
                body.contains(&format!(r#"<define name="cem-attr-{local}">"#)),
                "missing define for cem-attr-{local}"
            );
            assert!(body.contains(&format!(r#"<attribute name="cem:{local}">"#)));
        }
    }

    #[test]
    fn enum_annotation_emits_choice_over_values() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        // `cem:action` is enum-typed: primary | secondary.
        assert!(body.contains("<value>primary</value>"));
        assert!(body.contains("<value>secondary</value>"));
    }

    #[test]
    fn free_form_annotation_emits_text() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        // `cem:screen` is free-form (allowed_values = None) → `<text/>`.
        let screen_block_start = body
            .find(r#"<define name="cem-attr-screen">"#)
            .expect("define for cem-attr-screen");
        let screen_block_end = body[screen_block_start..]
            .find("</define>")
            .map(|i| screen_block_start + i)
            .unwrap();
        let screen_block = &body[screen_block_start..screen_block_end];
        assert!(
            screen_block.contains("<text/>"),
            "free-form screen annotation should emit <text/>:\n{screen_block}"
        );
    }

    #[test]
    fn state_attribute_lists_every_state_in_state_matrix() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
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
            assert!(
                body.contains(&format!("<value>{state}</value>")),
                "state matrix value missing from cem-attr-state: {state}"
            );
        }
        assert!(body.contains(r#"<attribute name="cem:state">"#));
    }

    #[test]
    fn pass_through_attrs_exclude_active_cem_namespace() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        let pass_through = extract_define_block(&body, "host-pass-through-attr")
            .expect("host-pass-through-attr define");
        assert!(pass_through.contains(r#"<anyName>"#));
        assert!(pass_through.contains(r#"<except>"#));
        assert!(pass_through.contains(r#"<nsName ns="https://cem.dev/ns/core/1"/>"#));
        assert!(pass_through.contains(r#"<text/>"#));
    }

    #[test]
    fn state_attributes_are_annotation_scoped_lists() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();

        let badge_state =
            extract_define_block(&body, "cem-attr-state-badge").expect("badge state define");
        assert!(badge_state.contains("<list>"));
        assert!(badge_state.contains("<oneOrMore>"));
        assert!(badge_state.contains("<value>default</value>"));
        assert!(
            !badge_state.contains("<value>loading</value>"),
            "badge must not accept loading state:\n{badge_state}"
        );

        let action_state =
            extract_define_block(&body, "cem-attr-state-action").expect("action state define");
        assert!(action_state.contains("<value>loading</value>"));
        assert!(action_state.contains("<value>hover</value>"));
        assert!(
            !action_state.contains("<value>selected</value>"),
            "action must not accept selected state:\n{action_state}"
        );
    }

    #[test]
    fn host_variants_anchor_state_to_present_annotation() {
        let body = String::from_utf8(emit_cem_core().bytes).unwrap();
        let action_host =
            extract_define_block(&body, "cem-host-action").expect("cem-host-action define");
        assert!(action_host.contains(r#"<ref name="host-pass-through-attrs"/>"#));
        assert!(action_host.contains(r#"<ref name="cem-attr-action"/>"#));
        assert!(action_host.contains(r#"<ref name="cem-attr-state-action"/>"#));
        assert!(action_host.contains(r#"<optional><ref name="cem-attr-badge"/></optional>"#));

        let unannotated = extract_define_block(&body, "cem-host-unannotated")
            .expect("cem-host-unannotated define");
        assert!(unannotated.contains(r#"<ref name="cem-attr-state"/>"#));
        assert!(!unannotated.contains(r#"<ref name="cem-attr-action"/>"#));
    }

    #[test]
    fn byte_stability_two_emits_equal() {
        let a = emit_cem_core();
        let b = emit_cem_core();
        assert_eq!(a.bytes, b.bytes, "rng_xml is not byte-stable");
        assert_eq!(a.content_hash, b.content_hash);
    }

    #[test]
    fn output_is_utf8_lf_no_trailing_whitespace_final_newline() {
        let bytes = emit_cem_core().bytes;
        // Must end with LF.
        assert_eq!(*bytes.last().unwrap(), b'\n');
        // No CR bytes.
        assert!(!bytes.contains(&b'\r'));
        // No trailing whitespace on any line (excluding the final LF).
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
        let a = emit_cem_core();
        assert_eq!(a.relative_path, "core/1.0.0/cem-core.rng");
    }

    fn extract_define_block<'a>(body: &'a str, name: &str) -> Option<&'a str> {
        let head = format!(r#"<define name="{name}">"#);
        let start = body.find(&head)?;
        let after_head = start + head.len();
        let next = body[after_head..].find("\n  <define name=");
        let end = next.map(|i| after_head + i).unwrap_or_else(|| {
            body[start..]
                .find("\n</grammar>")
                .map(|i| start + i)
                .unwrap_or(body.len())
        });
        Some(&body[start..end])
    }
}
