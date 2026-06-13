//! AC-S-2 / §13.2.4 byte-stability fixture.
//!
//! Emits the cem-core schema twice and asserts every artifact — the
//! RELAX NG mirrors, the `.d.ts`, the optional `.rs`, and the
//! publication manifest — is byte-identical and hash-identical across
//! runs. This is the runnable surface for AC-S-2's "byte-stable for
//! unchanged input" requirement and §13.2.4's encoding rules.

use cem_ml::schema::compiler::{ArtifactKind, CompilerOptions, ContentHash, SchemaCompiler};
use cem_ml::schema::ir::CompiledSchema;

/// Emit the same schema twice with the same options and assert that
/// every artifact reproduces byte-for-byte.
fn assert_emit_is_byte_stable(options: &CompilerOptions) {
    let schema = CompiledSchema::cem_core();
    let first = SchemaCompiler::emit_all(&schema, options).expect("first emit");
    let second = SchemaCompiler::emit_all(&schema, options).expect("second emit");

    assert_eq!(
        first.artifacts.len(),
        second.artifacts.len(),
        "artifact count is not stable"
    );
    for (a, b) in first.artifacts.iter().zip(&second.artifacts) {
        assert_eq!(a.kind, b.kind, "artifact order is not stable");
        assert_eq!(
            a.relative_path, b.relative_path,
            "{:?} relative_path is not stable",
            a.kind
        );
        assert_eq!(a.bytes, b.bytes, "{:?} bytes are not byte-stable", a.kind);
        assert_eq!(
            a.content_hash, b.content_hash,
            "{:?} content hash is not stable",
            a.kind
        );
        // The recorded hash must be blake3 of the recorded bytes
        // (§13.2.4 rule 8).
        assert_eq!(
            a.content_hash,
            ContentHash::from_blake3(&a.bytes),
            "{:?} content hash does not match its bytes",
            a.kind
        );
        assert_eq!(a.content_hash.scheme, "cem-bin/1+blake3");
    }
}

#[test]
fn default_emit_is_byte_stable() {
    assert_emit_is_byte_stable(&CompilerOptions::default());
}

#[test]
fn emit_with_rust_header_is_byte_stable() {
    // Exercises the Tier-B-gated rust_hdr path as well.
    assert_emit_is_byte_stable(&CompilerOptions {
        emit_rust: true,
        ..Default::default()
    });
}

#[test]
fn manifest_is_emitted_last_and_is_byte_stable() {
    let schema = CompiledSchema::cem_core();
    let output = SchemaCompiler::emit_all(&schema, &CompilerOptions::default()).expect("emit_all");

    let manifest = output
        .artifacts
        .iter()
        .find(|a| a.kind == ArtifactKind::Manifest)
        .expect("manifest artifact is present");
    assert_eq!(manifest.relative_path, "core/1.0.0/manifest.json");
    // Written last on disk so a crash leaves the previous manifest in
    // place (§13.2.6 step 2).
    assert_eq!(
        output.artifacts.last().map(|a| a.kind),
        Some(ArtifactKind::Manifest)
    );
    assert_eq!(
        manifest.content_hash,
        ContentHash::from_blake3(&manifest.bytes)
    );
}

#[test]
fn every_artifact_obeys_the_lf_encoding_rules() {
    // §13.2.4 rule 1: UTF-8, no CR, single trailing newline, no
    // trailing whitespace on any line.
    let schema = CompiledSchema::cem_core();
    let output = SchemaCompiler::emit_all(
        &schema,
        &CompilerOptions {
            emit_rust: true,
            ..Default::default()
        },
    )
    .expect("emit_all");

    for artifact in &output.artifacts {
        let bytes = &artifact.bytes;
        assert!(!bytes.is_empty(), "{:?} is empty", artifact.kind);
        assert!(
            std::str::from_utf8(bytes).is_ok(),
            "{:?} is not valid UTF-8",
            artifact.kind
        );
        assert!(
            !bytes.contains(&b'\r'),
            "{:?} carries a CR byte",
            artifact.kind
        );
        assert_eq!(
            bytes.last(),
            Some(&b'\n'),
            "{:?} lacks a final newline",
            artifact.kind
        );
        for line in bytes.split(|&b| b == b'\n') {
            assert!(
                !line.ends_with(b" ") && !line.ends_with(b"\t"),
                "{:?} has a line with trailing whitespace",
                artifact.kind
            );
        }
    }
}
