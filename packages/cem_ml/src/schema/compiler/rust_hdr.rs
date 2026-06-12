//! Rust `.rs` header emitter (AC-S-4).
//!
//! Tier A code, Tier B gate per OQ-SC-3 (resolved). The emitter lands
//! in the Tier A drop but `SchemaCompiler::emit_all` only invokes it
//! when `CompilerOptions.emit_rust = true` (default `false`). The
//! verification fixture
//! `packages/cem_ml/tests/schema_emit/rust_hdr_compiles.rs` runs only
//! when the flag is on (or `CEM_ML_EMIT_RUST=1`).
//!
//! Output shape (mirrors `cem-ml-stack-design-impl.md` §3.4.2.4):
//!
//! ```text
//! //! AUTO-GENERATED. CEM-native source: <schema-uri> @<embedded-version>
//! #![allow(non_camel_case_types, dead_code)]
//!
//! pub mod schema {
//!     pub const SCHEMA_URI: &str = "<schema-uri>";
//!     pub const EMBEDDED_VERSION: &str = "<semver>";
//!
//!     #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
//!     pub enum Action { Primary, Secondary }
//!
//!     // ... one enum per enum-typed annotation, plus CemState
//! }
//! ```
//!
//! Free-form annotations (those with `allowed_values = None`) carry no
//! Rust enum — their value type at the call site is `&str`. Only
//! enum-typed annotations and the schema-wide `state_matrix` lower
//! into emitted Rust types.
//!
//! Determinism notes (§13.2.4):
//! - UTF-8, LF, final newline, no trailing whitespace.
//! - Annotation iteration via `EmissionCursor::annotations()` is sorted
//!   (`BTreeMap`).
//! - The emitter writes pre-rustfmt-formatted output; matching against
//!   `rustfmt --check` is on the Tier B roadmap (vendored config).

use super::byte_stability::DeterministicWriter;
use super::emitter::{
    reject_non_streamable_constraints, relative_path, EmissionCursor, SchemaEmitter,
};
use super::error::EmitError;
use super::output::{ArtifactKind, EmittedArtifact};
use super::CompilerOptions;
use crate::schema::ir::CompiledSchema;

pub struct RustHdrEmitter;

impl SchemaEmitter for RustHdrEmitter {
    const KIND: ArtifactKind = ArtifactKind::RustHeader;
    const EXTENSION: &'static str = "rs";
    const EMITTER_NAME: &'static str = "rust_hdr";

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
                "//! AUTO-GENERATED. CEM-native source: {uri} @{ver}",
                uri = schema.version_identity.uri,
                ver = schema
                    .version_identity
                    .embedded_version
                    .to_canonical_string(),
            ))?;
        }
        w.line("#![allow(non_camel_case_types, dead_code)]")?;
        w.blank();

        w.line("pub mod schema {")?;
        w.indent();

        w.line(&format!(
            r#"pub const SCHEMA_URI: &str = "{}";"#,
            escape_rust_string_literal(&schema.version_identity.uri)
        ))?;
        w.line(&format!(
            r#"pub const EMBEDDED_VERSION: &str = "{}";"#,
            escape_rust_string_literal(
                &schema
                    .version_identity
                    .embedded_version
                    .to_canonical_string()
            )
        ))?;

        // One enum per enum-typed annotation. Free-form annotations
        // (allowed_values=None) lower to `&str` at the call site, so
        // they emit no Rust type.
        for (local, def) in cursor.annotations() {
            if let Some(values) = &def.allowed_values {
                w.blank();
                emit_annotation_enum(&mut w, local, values)?;
            }
        }

        // Schema-wide state matrix.
        w.blank();
        emit_state_enum(&mut w, cursor.state_matrix())?;

        w.dedent();
        w.line("}")?;

        let (bytes, content_hash) = w.finalize()?;
        Ok(EmittedArtifact {
            kind: ArtifactKind::RustHeader,
            relative_path: relative_path(schema, ArtifactKind::RustHeader)?,
            bytes,
            content_hash,
            source_map: Default::default(),
        })
    }
}

fn emit_annotation_enum(
    w: &mut DeterministicWriter,
    local: &str,
    values: &[&'static str],
) -> Result<(), EmitError> {
    let enum_name = to_pascal_case(local);
    w.line("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")?;
    w.line(&format!("pub enum {enum_name} {{"))?;
    w.indent();
    for value in values {
        w.line(&format!("{},", to_pascal_case(value)))?;
    }
    w.dedent();
    w.line("}")?;
    Ok(())
}

fn emit_state_enum(
    w: &mut DeterministicWriter,
    state_matrix: &[&'static str],
) -> Result<(), EmitError> {
    w.line("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")?;
    w.line("pub enum CemState {")?;
    w.indent();
    for state in state_matrix {
        w.line(&format!("{},", to_pascal_case(state)))?;
    }
    w.dedent();
    w.line("}")?;
    Ok(())
}

/// `"screen"` → `"Screen"`, `"focus-visible"` → `"FocusVisible"`.
/// Splits on `-` / `_`; preserves ASCII-only annotation names (every
/// cem-core/1 value is ASCII).
fn to_pascal_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut next_upper = true;
    for c in input.chars() {
        if c == '-' || c == '_' {
            next_upper = true;
            continue;
        }
        if next_upper {
            for upper in c.to_uppercase() {
                out.push(upper);
            }
            next_upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Escape a Rust double-quoted string literal. cem-core/1 URIs and
/// version strings are ASCII so this is defensive but worth keeping
/// for schemas with quoted special characters.
fn escape_rust_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::ir::AnnotationDef;

    fn emit_cem_core() -> EmittedArtifact {
        let schema = CompiledSchema::cem_core();
        let opts = CompilerOptions {
            emit_rust: true,
            ..Default::default()
        };
        let mut cursor = EmissionCursor::new(&schema);
        RustHdrEmitter.emit(&schema, &opts, &mut cursor).unwrap()
    }

    fn body_of(artifact: &EmittedArtifact) -> String {
        String::from_utf8(artifact.bytes.clone()).unwrap()
    }

    #[test]
    fn header_uses_module_doc_comment_with_uri_and_version() {
        let body = body_of(&emit_cem_core());
        assert!(body.starts_with(
            "//! AUTO-GENERATED. CEM-native source: https://cem.dev/ns/core/1 @1.0.0"
        ));
        // OQ-SC-8: no content hash in header.
        assert!(!body.contains("Content hash"));
    }

    #[test]
    fn header_omitted_when_disabled_but_attribute_remains() {
        let schema = CompiledSchema::cem_core();
        let opts = CompilerOptions {
            emit_rust: true,
            embed_source_header: false,
            ..Default::default()
        };
        let mut cursor = EmissionCursor::new(&schema);
        let body = body_of(&RustHdrEmitter.emit(&schema, &opts, &mut cursor).unwrap());
        assert!(!body.contains("AUTO-GENERATED"));
        assert!(body.starts_with("#![allow(non_camel_case_types, dead_code)]"));
    }

    #[test]
    fn module_carries_schema_uri_and_embedded_version_consts() {
        let body = body_of(&emit_cem_core());
        assert!(body.contains("pub mod schema {"));
        assert!(body.contains(r#"pub const SCHEMA_URI: &str = "https://cem.dev/ns/core/1";"#));
        assert!(body.contains(r#"pub const EMBEDDED_VERSION: &str = "1.0.0";"#));
    }

    #[test]
    fn enum_typed_annotations_lower_to_rust_enums() {
        let body = body_of(&emit_cem_core());
        // Action — primary | secondary
        assert!(body.contains("pub enum Action {"));
        assert!(body.contains("Primary,"));
        assert!(body.contains("Secondary,"));
        // Badge — success | info | warning | error
        assert!(body.contains("pub enum Badge {"));
        for variant in ["Success", "Info", "Warning", "Error"] {
            assert!(
                body.contains(&format!("{variant},")),
                "Badge variant missing: {variant}"
            );
        }
        // Message — sent | received
        assert!(body.contains("pub enum Message {"));
        for variant in ["Sent", "Received"] {
            assert!(
                body.contains(&format!("{variant},")),
                "Message variant missing: {variant}"
            );
        }
    }

    #[test]
    fn free_form_annotations_emit_no_enum() {
        let body = body_of(&emit_cem_core());
        // Free-form annotations: screen, form, card, list, row, thread.
        // The emitter must NOT produce a `pub enum Screen` (etc.) —
        // the call-site type is `&str`.
        for free_form in ["Screen", "Form", "Card", "List", "Row", "Thread"] {
            assert!(
                !body.contains(&format!("pub enum {free_form} {{")),
                "unexpected pub enum {free_form} — free-form annotations should not lower to Rust enums"
            );
        }
    }

    #[test]
    fn cem_state_enum_carries_full_state_matrix() {
        let body = body_of(&emit_cem_core());
        assert!(body.contains("pub enum CemState {"));
        for variant in [
            "Default",
            "Hover",
            "FocusVisible",
            "Active",
            "Selected",
            "Disabled",
            "Invalid",
            "Required",
            "Loading",
            "Empty",
        ] {
            assert!(
                body.contains(&format!("{variant},")),
                "CemState variant missing: {variant}"
            );
        }
    }

    #[test]
    fn every_enum_derives_the_standard_set() {
        let body = body_of(&emit_cem_core());
        let derive_count = body
            .matches("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")
            .count();
        // 3 annotation enums (Action / Badge / Message) + 1 CemState.
        assert_eq!(
            derive_count, 4,
            "expected 4 derive blocks (3 enum-typed annotations + CemState):\n{body}"
        );
    }

    #[test]
    fn byte_stability_two_emits_equal() {
        let a = emit_cem_core();
        let b = emit_cem_core();
        assert_eq!(a.bytes, b.bytes, "rust_hdr is not byte-stable");
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
        assert_eq!(emit_cem_core().relative_path, "core/1.0.0/cem-core.rs");
    }

    #[test]
    fn to_pascal_case_handles_kebab_input() {
        assert_eq!(to_pascal_case("primary"), "Primary");
        assert_eq!(to_pascal_case("focus-visible"), "FocusVisible");
    }

    #[test]
    fn escape_rust_string_literal_handles_backslash_and_quote() {
        assert_eq!(escape_rust_string_literal("a\\b\"c"), "a\\\\b\\\"c");
        assert_eq!(escape_rust_string_literal("plain"), "plain");
    }

    /// Smoke test: try to compile the emitted module as a standalone
    /// Rust source through `syn`-style validation. We don't pull in
    /// the `syn` crate (no dep added), so this test checks the shape
    /// invariants instead: opening / closing brace counts match, the
    /// `pub mod schema` block is balanced, and no enum body is left
    /// open.
    #[test]
    fn emitted_braces_balance() {
        let body = body_of(&emit_cem_core());
        let opens = body.matches('{').count();
        let closes = body.matches('}').count();
        assert_eq!(opens, closes, "unbalanced braces in emitted .rs:\n{body}");
    }

    /// The annotation `def: AnnotationDef` arg path also handles an
    /// empty allowed_values vec correctly (defensive — the cem-core/1
    /// vocabulary never declares one, but the emitter must not panic
    /// on it either).
    #[test]
    fn empty_allowed_values_vec_emits_empty_enum_body() {
        let def = AnnotationDef {
            local_name: "synthetic",
            allowed_values: Some(Vec::new()),
            known_values: Vec::new(),
            allowed_states: Vec::new(),
        };
        let mut w = DeterministicWriter::new();
        emit_annotation_enum(&mut w, def.local_name, def.allowed_values.as_ref().unwrap()).unwrap();
        let (bytes, _) = w.finalize().unwrap();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("pub enum Synthetic {"));
        assert!(s.contains("}"));
    }
}
