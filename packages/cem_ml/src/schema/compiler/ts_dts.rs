//! TypeScript `.d.ts` header emitter (AC-S-3, AC-S-6).
//!
//! Reference: `cem-ml-stack-design-impl.md` §3.4.2.4 and AC-S-V-1..AC-S-V-5
//! in `cem-ml-ac.md`. Resolutions threaded through:
//!
//! - OQ-SC-6: runtime `asValidated` / `tryValidated` / `Validated<T>` come
//!   from `@epa-wg/cem-ml/wasm` via re-export. No `.js` sibling is emitted;
//!   no host-stub `declare function` lines.
//! - OQ-SC-7: per-version `.d.ts` under the per-version on-disk path
//!   (see `emitter::relative_path`); the `package.json` `exports` field
//!   maps that path to `@epa-wg/cem-ml/schema/<tail>/<version>/<stem>`.
//! - OQ-SC-8: header carries CEM-native source URI and embedded SemVer
//!   only. The content hash lives in the `.hash` sidecar, never in the
//!   header.
//!
//! Determinism notes (§13.2.4):
//! - UTF-8, LF, single trailing newline, no trailing whitespace.
//! - Annotations are walked through `EmissionCursor::annotations()` which
//!   is `BTreeMap` iteration (alphabetical by local name).

use super::byte_stability::DeterministicWriter;
use super::emitter::{relative_path, EmissionCursor, SchemaEmitter};
use super::error::EmitError;
use super::output::{ArtifactKind, EmittedArtifact};
use super::CompilerOptions;
use crate::schema::ir::{AnnotationDef, CompiledSchema};

const WASM_SUBPATH: &str = "@epa-wg/cem-ml/wasm";

pub struct TsDtsEmitter;

impl SchemaEmitter for TsDtsEmitter {
    const KIND: ArtifactKind = ArtifactKind::TypeScriptDts;
    const EXTENSION: &'static str = "d.ts";
    const EMITTER_NAME: &'static str = "ts_dts";

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

        let mut w = DeterministicWriter::new();

        if options.embed_source_header {
            w.line(&format!(
                "// AUTO-GENERATED. CEM-native source: {uri} @{ver}",
                uri = schema.version_identity.uri,
                ver = schema.version_identity.embedded_version.to_canonical_string(),
            ))?;
        }

        // `Validated<T>` brand and constructors come from the WASM
        // build per OQ-SC-6. Structural-by-default consumers can skip
        // the import block entirely by setting
        // `CompilerOptions.include_validated_brand = false` (AC-S-6).
        if options.include_validated_brand {
            w.line(&format!(r#"export type {{ Validated }} from "{WASM_SUBPATH}";"#))?;
            w.line(&format!(
                r#"export {{ asValidated, tryValidated }} from "{WASM_SUBPATH}";"#
            ))?;
            w.blank();
        }

        // One structural interface per annotation. Interface names are
        // Pascal-case of the annotation's local name; the
        // annotation-value property is camelCase of `cem:{local}`
        // (`cem:action` → `cemAction`).
        let mut first_interface = true;
        for (local, def) in cursor.annotations() {
            if !first_interface {
                w.blank();
            }
            first_interface = false;
            emit_interface(&mut w, local, def)?;
        }

        let (bytes, content_hash) = w.finalize()?;
        Ok(EmittedArtifact {
            kind: ArtifactKind::TypeScriptDts,
            relative_path: relative_path(schema, ArtifactKind::TypeScriptDts)?,
            bytes,
            content_hash,
            source_map: Default::default(),
        })
    }
}

fn emit_interface(
    w: &mut DeterministicWriter,
    local: &str,
    def: &AnnotationDef,
) -> Result<(), EmitError> {
    let interface_name = to_pascal_case(local);
    let property_name = cem_property_name(local);
    let value_type = match &def.allowed_values {
        Some(values) => union_of_string_literals(values.iter().copied()),
        None => "string".to_owned(),
    };
    let state_type = if def.allowed_states.is_empty() {
        // Defensive: the active vocabulary never declares an empty
        // state set, but defaulting to `string` keeps consumers
        // forward-compatible if a future schema does.
        "string".to_owned()
    } else {
        union_of_string_literals(def.allowed_states.iter().copied())
    };

    // AC-S-V-1: every CEM element type extends the matching DOM base.
    // For cem-core/1 every annotation host is an HTML element; SVG /
    // XMLDocument bases come in when a non-HTML annotation lands.
    w.line(&format!(
        "export interface {interface_name} extends HTMLElement {{"
    ))?;
    w.indent();
    w.line(&format!("readonly {property_name}?: {value_type};"))?;
    w.line(&format!("readonly cemState?: {state_type};"))?;
    w.dedent();
    w.line("}")?;
    Ok(())
}

/// `"screen"` → `"Screen"`, `"focus-visible"` → `"FocusVisible"`.
/// Splits on `-` / `_`; preserves ASCII-only annotation names (every
/// cem-core/1 annotation is ASCII).
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

/// `"action"` → `"cemAction"`, `"focus-visible"` → `"cemFocusVisible"`.
fn cem_property_name(local: &str) -> String {
    format!("cem{}", to_pascal_case(local))
}

fn union_of_string_literals<'a, I>(values: I) -> String
where
    I: IntoIterator<Item = &'a str>,
{
    let parts: Vec<String> = values
        .into_iter()
        .map(|v| format!("\"{}\"", escape_double_quoted_ts(v)))
        .collect();
    if parts.is_empty() {
        "never".to_owned()
    } else {
        parts.join(" | ")
    }
}

/// Escape a TypeScript double-quoted string literal. cem-core/1 values
/// are all ASCII identifiers so this is defensive but worth keeping for
/// future schemas that allow quoted special characters.
fn escape_double_quoted_ts(s: &str) -> String {
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

    fn emit_cem_core() -> EmittedArtifact {
        let schema = CompiledSchema::cem_core();
        let opts = CompilerOptions::default();
        let mut cursor = EmissionCursor::new(&schema);
        TsDtsEmitter.emit(&schema, &opts, &mut cursor).unwrap()
    }

    fn body_of(artifact: &EmittedArtifact) -> String {
        String::from_utf8(artifact.bytes.clone()).unwrap()
    }

    #[test]
    fn header_carries_uri_and_version_no_hash() {
        let a = emit_cem_core();
        let body = body_of(&a);
        assert!(body.starts_with(
            "// AUTO-GENERATED. CEM-native source: https://cem.dev/ns/core/1 @1.0.0"
        ));
        // OQ-SC-8 (resolved): no content hash in header.
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
        let body = body_of(&TsDtsEmitter.emit(&schema, &opts, &mut cursor).unwrap());
        assert!(!body.contains("AUTO-GENERATED"));
        assert!(body.starts_with("export type { Validated }"));
    }

    #[test]
    fn validated_brand_re_exports_come_from_wasm_subpath() {
        let body = body_of(&emit_cem_core());
        assert!(body.contains(r#"export type { Validated } from "@epa-wg/cem-ml/wasm";"#));
        assert!(body
            .contains(r#"export { asValidated, tryValidated } from "@epa-wg/cem-ml/wasm";"#));
        // OQ-SC-6 (resolved): no host stubs declared.
        assert!(!body.contains("declare function asValidated"));
        assert!(!body.contains("declare function tryValidated"));
    }

    #[test]
    fn brand_block_dropped_when_include_validated_brand_false() {
        let schema = CompiledSchema::cem_core();
        let opts = CompilerOptions {
            include_validated_brand: false,
            ..Default::default()
        };
        let mut cursor = EmissionCursor::new(&schema);
        let body = body_of(&TsDtsEmitter.emit(&schema, &opts, &mut cursor).unwrap());
        assert!(!body.contains("Validated"));
        assert!(!body.contains("asValidated"));
        assert!(!body.contains("tryValidated"));
        // Structural interfaces still emit.
        assert!(body.contains("export interface Badge extends HTMLElement"));
    }

    #[test]
    fn every_annotation_becomes_an_interface_extending_html_element() {
        let body = body_of(&emit_cem_core());
        for (local, expected_name) in [
            ("screen", "Screen"),
            ("form", "Form"),
            ("action", "Action"),
            ("badge", "Badge"),
            ("card", "Card"),
            ("list", "List"),
            ("row", "Row"),
            ("thread", "Thread"),
            ("message", "Message"),
        ] {
            let needle = format!("export interface {expected_name} extends HTMLElement {{");
            assert!(
                body.contains(&needle),
                "missing interface for annotation `{local}`: expected `{needle}` in:\n{body}"
            );
        }
    }

    #[test]
    fn enum_annotation_emits_literal_union_for_value_property() {
        let body = body_of(&emit_cem_core());
        // cem:action — primary | secondary
        assert!(body.contains(r#"readonly cemAction?: "primary" | "secondary";"#));
        // cem:badge — success | info | warning | error
        assert!(
            body.contains(r#"readonly cemBadge?: "success" | "info" | "warning" | "error";"#)
        );
        // cem:message — sent | received
        assert!(body.contains(r#"readonly cemMessage?: "sent" | "received";"#));
    }

    #[test]
    fn free_form_annotation_emits_plain_string_for_value_property() {
        let body = body_of(&emit_cem_core());
        assert!(body.contains("readonly cemScreen?: string;"));
        assert!(body.contains("readonly cemCard?: string;"));
        assert!(body.contains("readonly cemList?: string;"));
        assert!(body.contains("readonly cemRow?: string;"));
        assert!(body.contains("readonly cemThread?: string;"));
        assert!(body.contains("readonly cemForm?: string;"));
    }

    #[test]
    fn state_union_reflects_annotations_allowed_states() {
        let body = body_of(&emit_cem_core());
        // badge — allowed_states = ["default"]
        let badge_block = extract_interface_block(&body, "Badge").expect("Badge interface block");
        assert!(
            badge_block.contains(r#"readonly cemState?: "default";"#),
            "badge state union should be exactly \"default\":\n{badge_block}"
        );
        // action — allowed_states = ["default","hover","focus-visible","active","disabled","loading"]
        let action_block = extract_interface_block(&body, "Action").expect("Action interface block");
        assert!(action_block.contains(
            r#"readonly cemState?: "default" | "hover" | "focus-visible" | "active" | "disabled" | "loading";"#
        ));
    }

    #[test]
    fn byte_stability_two_emits_equal() {
        let a = emit_cem_core();
        let b = emit_cem_core();
        assert_eq!(a.bytes, b.bytes, "ts_dts is not byte-stable");
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
        // OQ-SC-7 (resolved): the on-disk per-version directory backs
        // the `@epa-wg/cem-ml/schema/<tail>/<version>/<stem>` subpath.
        assert_eq!(emit_cem_core().relative_path, "core/1.0.0/cem-core.d.ts");
    }

    #[test]
    fn to_pascal_case_handles_kebab_and_snake() {
        assert_eq!(to_pascal_case("screen"), "Screen");
        assert_eq!(to_pascal_case("focus-visible"), "FocusVisible");
        assert_eq!(to_pascal_case("multi_word_name"), "MultiWordName");
    }

    #[test]
    fn cem_property_name_capitalises_and_prefixes() {
        assert_eq!(cem_property_name("action"), "cemAction");
        assert_eq!(cem_property_name("focus-visible"), "cemFocusVisible");
    }

    #[test]
    fn union_helper_quotes_each_value_and_pipes() {
        let s = union_of_string_literals(["a", "b", "c"]);
        assert_eq!(s, r#""a" | "b" | "c""#);
        let empty = union_of_string_literals(std::iter::empty());
        assert_eq!(empty, "never");
    }

    /// Helper: extract the body of `export interface {name}` block —
    /// from the opening `{` to its matching `}`. Single-level braces
    /// in the cem-core/1 output keep this trivial.
    fn extract_interface_block<'a>(body: &'a str, name: &str) -> Option<&'a str> {
        let head = format!("export interface {name} extends HTMLElement {{");
        let start = body.find(&head)?;
        let after_open = start + head.len();
        let close_rel = body[after_open..].find('}')?;
        Some(&body[after_open..after_open + close_rel])
    }
}
