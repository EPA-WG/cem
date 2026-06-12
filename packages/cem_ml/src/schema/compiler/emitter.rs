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
use super::uri_publish;
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
    uri_publish::artifact_relative_path(schema, kind)
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
    fn relative_path_uses_the_shared_schema_uri_parser() {
        let mut schema = CompiledSchema::cem_core();
        schema.version_identity.uri = "https://cem.dev/ns/component-mvp/1.2.3-rc.1".to_owned();
        let path = relative_path(&schema, ArtifactKind::RelaxNgXml).unwrap();
        assert_eq!(path, "component-mvp/1.0.0/cem-component-mvp.rng");
    }

    #[test]
    fn nested_namespace_tail_uses_directories_but_not_nested_file_stems() {
        let mut schema = CompiledSchema::cem_core();
        schema.version_identity.uri = "https://cem.dev/ns/ui/core/1.2".to_owned();
        let path = relative_path(&schema, ArtifactKind::TypeScriptDts).unwrap();
        assert_eq!(path, "ui/core/1.0.0/cem-ui-core.d.ts");
    }

    #[test]
    fn unrecognised_uri_surfaces_missing_field_error() {
        let mut schema = CompiledSchema::cem_core();
        schema.version_identity.uri = "ftp://nope/whatever".to_owned();
        let err = relative_path(&schema, ArtifactKind::RelaxNgXml).unwrap_err();
        assert!(matches!(err, EmitError::UnresolvableUri { .. }));
    }
}
