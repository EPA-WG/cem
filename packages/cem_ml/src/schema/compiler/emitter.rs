//! `SchemaEmitter` trait + `EmissionCursor` walk helper.
//!
//! The cursor walks `CompiledSchema` in a fixed order — namespace bindings,
//! annotations (alphabetical by local name), states, semantic rules (by
//! `rule_id`), open-content rules (by content model then namespace), and
//! finally the schema-version identity record — so every emitter that
//! consumes the cursor produces deterministic output without re-sorting.
//!
//! Reference: `cem-ml-stack-design-impl.md` §3.4.2.2.

use std::collections::BTreeMap;

use crate::schema::ir::{AnnotationDef, CompiledSchema, SchemaStateDef, SemanticRule};

use super::error::EmitError;
use super::output::{ArtifactKind, EmittedArtifact};
use super::CompilerOptions;

pub struct EmissionCursor<'a> {
    schema: &'a CompiledSchema,
}

impl<'a> EmissionCursor<'a> {
    pub fn new(schema: &'a CompiledSchema) -> Self {
        Self { schema }
    }

    pub fn schema(&self) -> &'a CompiledSchema {
        self.schema
    }

    /// Annotations in alphabetical order (`BTreeMap` iteration). The
    /// only allowed source of ordered annotation output.
    pub fn annotations(&self) -> &'a BTreeMap<&'static str, AnnotationDef> {
        &self.schema.annotations
    }

    /// Structural state defs in `StructuralSchemaIr` order — populated
    /// by `cem_core()` via sorted-`BTreeMap` traversal, so this is
    /// safe to consume directly.
    pub fn structural_states(&self) -> &'a [SchemaStateDef] {
        &self.schema.structural.states
    }

    /// Semantic rules in declared order. `cem_core()` emits one entry
    /// per `RuleRegistry::with_tier_a_rules` registration in
    /// declaration order; emitters that need stable cross-version
    /// output sort by `rule_id` before writing.
    pub fn semantic_rules(&self) -> &'a [SemanticRule] {
        &self.schema.semantic_rules
    }

    /// State matrix in declared order (matches the cem-core/1 doc).
    pub fn state_matrix(&self) -> &'a [&'static str] {
        &self.schema.state_matrix
    }
}

pub trait SchemaEmitter {
    const KIND: ArtifactKind;
    const EXTENSION: &'static str;
    const EMITTER_NAME: &'static str;

    fn emit(
        &self,
        schema: &CompiledSchema,
        options: &CompilerOptions,
        cursor: &mut EmissionCursor<'_>,
    ) -> Result<EmittedArtifact, EmitError>;
}

/// Emitters must not publish a weakened structural mirror. A schema carrying
/// non-streamable constraints is rejected before any artifact bytes are built,
/// both through `SchemaCompiler::emit_all` and through direct emitter calls.
pub(crate) fn reject_non_streamable_constraints(schema: &CompiledSchema) -> Result<(), EmitError> {
    if let Some(constraint) = schema.non_streamable_constraints.first() {
        return Err(EmitError::UnsupportedConstraint {
            kind: format!("{:?}", constraint.kind),
            schema_uri: schema.version_identity.uri.clone(),
        });
    }
    Ok(())
}

/// Compute the on-disk relative path for an artifact under
/// `dist/lib/schema/<namespace-tail>/<embedded-version>/<stem>.<ext>`.
///
/// Per §13.2.5: the tail is the URI's path after the well-known
/// `https://cem.dev/ns/` prefix, with `/` segments preserved as
/// directories. For `https://cem.dev/ns/core/1` the tail is `core` —
/// the `/1` MAJOR constraint is recorded in URI metadata but is **not**
/// a directory.
pub fn relative_path(schema: &CompiledSchema, kind: ArtifactKind) -> Result<String, EmitError> {
    let uri = schema.version_identity.uri.as_str();
    if uri.is_empty() {
        return Err(EmitError::MissingIrField {
            field: "version_identity.uri",
        });
    }
    let tail = namespace_tail(uri).ok_or(EmitError::MissingIrField {
        field: "version_identity.uri (not a cem.dev/ns/* URI)",
    })?;
    let version = schema
        .version_identity
        .embedded_version
        .to_canonical_string();
    // The manifest is always `manifest.json`; every other artifact is
    // `<stem>.<ext>` (§13.2.5).
    let file_name = match kind {
        ArtifactKind::Manifest => "manifest.json".to_owned(),
        _ => format!(
            "{stem}.{ext}",
            stem = artifact_stem_from_tail(&tail),
            ext = kind.extension()
        ),
    };
    Ok(format!("{tail}/{version}/{file_name}"))
}

/// Strip the well-known prefix and drop the trailing major-version
/// segment. `https://cem.dev/ns/core/1` → `Some("core")`,
/// `https://cem.dev/ns/component-mvp/2` → `Some("component-mvp")`,
/// `https://other.example/ns/core/1` → `None`.
pub(crate) fn namespace_tail(uri: &str) -> Option<String> {
    const PREFIX: &str = "https://cem.dev/ns/";
    let rest = uri.strip_prefix(PREFIX)?;
    // Drop the final `/<digits>` MAJOR-version segment, if any.
    let trimmed = match rest.rsplit_once('/') {
        Some((head, tail)) if tail.chars().all(|c| c.is_ascii_digit()) && !tail.is_empty() => {
            head.to_owned()
        }
        _ => rest.to_owned(),
    };
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// `cem-core` for tail `core`, `cem-{tail}` otherwise. Keeps the stem
/// short and consistent with §13.2.5's `cem-core.rng` examples.
pub(crate) fn artifact_stem_from_tail(tail: &str) -> String {
    if tail == "core" {
        "cem-core".to_owned()
    } else {
        format!("cem-{tail}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::ir::CompiledSchema;

    #[test]
    fn cem_core_relative_path() {
        let schema = CompiledSchema::cem_core();
        let path = relative_path(&schema, ArtifactKind::RelaxNgXml).unwrap();
        assert_eq!(path, "core/1.0.0/cem-core.rng");
        let path = relative_path(&schema, ArtifactKind::RelaxNgCompact).unwrap();
        assert_eq!(path, "core/1.0.0/cem-core.rnc");
    }

    #[test]
    fn namespace_tail_strips_well_known_prefix_and_major() {
        assert_eq!(
            namespace_tail("https://cem.dev/ns/core/1"),
            Some("core".to_owned())
        );
        assert_eq!(
            namespace_tail("https://cem.dev/ns/component-mvp/2"),
            Some("component-mvp".to_owned())
        );
        assert_eq!(namespace_tail("https://other.example/ns/core/1"), None);
    }

    #[test]
    fn unrecognised_uri_surfaces_missing_field_error() {
        let mut schema = CompiledSchema::cem_core();
        schema.version_identity.uri = "ftp://nope/whatever".to_owned();
        let err = relative_path(&schema, ArtifactKind::RelaxNgXml).unwrap_err();
        assert!(matches!(err, EmitError::MissingIrField { .. }));
    }
}
