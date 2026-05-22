//! AC-S-5 / AC-V-10 fixture: stable-URI publication and URI-tail
//! resolution.
//!
//! Two concerns:
//!   1. `write_to_disk` lays the §13.2.5 publication tree — every
//!      artifact, every `.hash` sidecar, and `manifest.json` — and the
//!      sidecar bytes match a recomputed blake3 digest.
//!   2. `resolve_uri` matches a declared schema URI against a set of
//!      published manifests per the AC-V-10 table, excluding
//!      pre-releases from loose matches (resolved decision; §13.2.6).
//!
//! The design names `cem_ml::loader` as the eventual caller; that
//! document-loading pipeline does not exist yet. The AC-S-5 surface
//! per impl §3.4.2.1 is the `uri_publish` resolution helper, which is
//! what this fixture drives directly.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cem_ml::schema::compiler::{
    match_rule_label, resolve_uri, CompilerOptions, ContentHash, PublicationManifest,
    SchemaCompiler,
};
use cem_ml::schema::ir::{CompiledSchema, SchemaVersionMatchRule, SemVer};

fn semver(major: u64, minor: u64, patch: u64) -> SemVer {
    SemVer::new(major, minor, patch)
}

fn prerelease(major: u64, minor: u64, patch: u64, pre: &str) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
        prerelease: Some(pre.to_owned()),
        build: None,
    }
}

fn build(major: u64, minor: u64, patch: u64, build: &str) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
        prerelease: None,
        build: Some(build.to_owned()),
    }
}

/// A manifest carrying only the fields resolution reads — the schema
/// URI and the embedded version. `artifacts` is irrelevant to URI-tail
/// matching, so it is left empty here.
fn manifest_for(schema_uri: &str, version: SemVer) -> PublicationManifest {
    PublicationManifest {
        schema_uri: schema_uri.to_owned(),
        embedded_version: version,
        hash_scheme: ContentHash::SCHEME,
        artifacts: BTreeMap::new(),
    }
}

/// A published-manifest corpus for the `core` namespace: five stable
/// releases across two majors plus two pre-releases. The pre-releases
/// must never surface for a loose URI.
fn corpus() -> Vec<PublicationManifest> {
    vec![
        manifest_for("https://cem.dev/ns/core/1", semver(1, 0, 0)),
        manifest_for("https://cem.dev/ns/core/1", semver(1, 2, 0)),
        manifest_for("https://cem.dev/ns/core/1", prerelease(1, 2, 3, "rc.1")),
        manifest_for("https://cem.dev/ns/core/1", semver(1, 2, 5)),
        manifest_for("https://cem.dev/ns/core/1", semver(1, 3, 0)),
        manifest_for("https://cem.dev/ns/core/2", semver(2, 0, 0)),
        manifest_for("https://cem.dev/ns/core/2", prerelease(2, 1, 0, "rc.1")),
    ]
}

#[test]
fn unconstrained_uri_resolves_to_the_highest_stable_release() {
    let corpus = corpus();
    let (_, resolution) =
        resolve_uri("https://cem.dev/ns/core", &corpus).expect("bare URI resolves");
    // The highest published version is 2.1.0-rc.1, but a bare URI must
    // not surface a pre-release — it resolves to stable 2.0.0.
    assert_eq!(resolution.embedded_version, semver(2, 0, 0));
    assert_eq!(resolution.match_rule, SchemaVersionMatchRule::Unconstrained);
    assert_eq!(match_rule_label(resolution.match_rule), "unconstrained");
}

#[test]
fn major_uri_resolves_to_the_highest_stable_within_the_major() {
    let corpus = corpus();
    let (_, resolution) =
        resolve_uri("https://cem.dev/ns/core/1", &corpus).expect("/1 resolves");
    assert_eq!(resolution.embedded_version, semver(1, 3, 0));
    assert_eq!(resolution.match_rule, SchemaVersionMatchRule::Major);
    assert_eq!(match_rule_label(resolution.match_rule), "major");
}

#[test]
fn major_minor_uri_resolves_within_the_minor() {
    let corpus = corpus();
    let (_, resolution) =
        resolve_uri("https://cem.dev/ns/core/1.2", &corpus).expect("/1.2 resolves");
    // 1.2.0 and 1.2.5 are candidates; 1.2.3-rc.1 is excluded.
    assert_eq!(resolution.embedded_version, semver(1, 2, 5));
    assert_eq!(resolution.match_rule, SchemaVersionMatchRule::MajorMinor);
    assert_eq!(match_rule_label(resolution.match_rule), "major-minor");
}

#[test]
fn full_uri_resolves_forgiving_at_or_above_the_tail() {
    let corpus = corpus();
    // /1.2.3 — same major, (minor, patch) >= (2, 3): 1.2.5 and 1.3.0.
    let (_, resolution) =
        resolve_uri("https://cem.dev/ns/core/1.2.3", &corpus).expect("/1.2.3 resolves");
    assert_eq!(resolution.embedded_version, semver(1, 3, 0));
    assert_eq!(resolution.match_rule, SchemaVersionMatchRule::Full);
    assert_eq!(match_rule_label(resolution.match_rule), "full");
}

#[test]
fn prerelease_uri_resolves_to_the_named_prerelease_only() {
    let corpus = corpus();
    let (_, resolution) = resolve_uri("https://cem.dev/ns/core/1.2.3-rc.1", &corpus)
        .expect("/1.2.3-rc.1 resolves");
    assert_eq!(resolution.embedded_version, prerelease(1, 2, 3, "rc.1"));
    assert_eq!(resolution.match_rule, SchemaVersionMatchRule::PrereleaseExact);
    assert_eq!(match_rule_label(resolution.match_rule), "prerelease-exact");
}

#[test]
fn buildless_uri_prefers_the_unbuilt_release_when_build_variants_tie() {
    let corpus = vec![
        manifest_for("https://cem.dev/ns/core/1", build(1, 2, 3, "sha.a")),
        manifest_for("https://cem.dev/ns/core/1", semver(1, 2, 3)),
        manifest_for("https://cem.dev/ns/core/1", build(1, 2, 3, "sha.b")),
    ];
    let (_, resolution) =
        resolve_uri("https://cem.dev/ns/core/1.2.3", &corpus).expect("buildless URI resolves");
    assert_eq!(resolution.embedded_version, semver(1, 2, 3));
}

#[test]
fn buildless_uri_rejects_ambiguous_build_only_ties() {
    let corpus = vec![
        manifest_for("https://cem.dev/ns/core/1", build(1, 2, 3, "sha.a")),
        manifest_for("https://cem.dev/ns/core/1", build(1, 2, 3, "sha.b")),
    ];
    assert!(resolve_uri("https://cem.dev/ns/core/1.2.3", &corpus).is_err());
}

#[test]
fn explicit_build_uri_resolves_the_named_build() {
    let corpus = vec![
        manifest_for("https://cem.dev/ns/core/1", build(1, 2, 3, "sha.a")),
        manifest_for("https://cem.dev/ns/core/1", build(1, 2, 3, "sha.b")),
    ];
    let (_, resolution) =
        resolve_uri("https://cem.dev/ns/core/1.2.3+sha.b", &corpus).expect("+build resolves");
    assert_eq!(resolution.embedded_version, build(1, 2, 3, "sha.b"));
}

#[test]
fn unmatched_prerelease_uri_does_not_resolve() {
    let corpus = corpus();
    // No `rc.9` is published — a pre-release tail matches exactly only.
    assert!(resolve_uri("https://cem.dev/ns/core/1.2.3-rc.9", &corpus).is_err());
}

#[test]
fn major_with_no_published_version_does_not_resolve() {
    let corpus = corpus();
    assert!(resolve_uri("https://cem.dev/ns/core/9", &corpus).is_err());
}

#[test]
fn uri_outside_the_well_known_scheme_does_not_resolve() {
    let corpus = corpus();
    assert!(resolve_uri("https://example.com/ns/core", &corpus).is_err());
}

#[test]
fn unknown_namespace_does_not_resolve() {
    let corpus = corpus();
    assert!(resolve_uri("https://cem.dev/ns/widget/1", &corpus).is_err());
}

#[test]
fn write_to_disk_lays_the_publication_tree_with_sidecars() {
    let schema = CompiledSchema::cem_core();
    let output =
        SchemaCompiler::emit_all(&schema, &CompilerOptions::default()).expect("emit_all");

    let root = unique_temp_dir("cem_ml_uri_publish");
    let _ = fs::remove_dir_all(&root);
    SchemaCompiler::write_to_disk(&output, &root).expect("write_to_disk");

    // Every artifact and its `.hash` sidecar is on disk; the on-disk
    // bytes match the emitted bytes, and the sidecar body is the
    // canonical `cem-bin/1+blake3:<hex>\n` over those bytes.
    for artifact in &output.artifacts {
        let path = root.join(&artifact.relative_path);
        let on_disk = fs::read(&path)
            .unwrap_or_else(|e| panic!("missing artifact {}: {e}", path.display()));
        assert_eq!(
            on_disk, artifact.bytes,
            "{:?} on-disk bytes differ from the emitted bytes",
            artifact.kind
        );

        let sidecar = PathBuf::from(format!("{}.hash", path.display()));
        let sidecar_body = fs::read_to_string(&sidecar)
            .unwrap_or_else(|e| panic!("missing sidecar {}: {e}", sidecar.display()));
        assert_eq!(
            sidecar_body,
            ContentHash::from_blake3(&on_disk).to_sidecar_string(),
            "{:?} sidecar does not match a recomputed digest",
            artifact.kind
        );
        assert!(sidecar_body.ends_with('\n'));
    }

    // The §13.2.5 layout: core/1.0.0/ holds the manifest plus mirrors.
    let version_dir = root.join("core").join("1.0.0");
    for name in ["cem-core.rng", "cem-core.rnc", "cem-core.d.ts", "manifest.json"] {
        assert!(
            version_dir.join(name).is_file(),
            "missing {name} in the publication tree"
        );
    }
    // The temp-then-rename adapter leaves no `.tmp` files behind.
    for entry in fs::read_dir(&version_dir).expect("read version dir") {
        let name = entry.expect("dir entry").file_name();
        assert!(
            !name.to_string_lossy().ends_with(".tmp"),
            "temp file left behind: {}",
            name.to_string_lossy()
        );
    }

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn write_to_disk_rejects_artifact_paths_that_escape_the_root() {
    let schema = CompiledSchema::cem_core();
    let mut output =
        SchemaCompiler::emit_all(&schema, &CompilerOptions::default()).expect("emit_all");
    output.artifacts[0].relative_path = "../escape.rng".to_owned();

    let root = unique_temp_dir("cem_ml_uri_publish_escape");
    let _ = fs::remove_dir_all(&root);
    let err = SchemaCompiler::write_to_disk(&output, &root).unwrap_err();
    assert!(
        err.to_string().contains("invalid artifact path"),
        "unexpected error: {err}"
    );
}

#[test]
fn the_published_cem_core_manifest_resolves_back_through_resolve_uri() {
    let schema = CompiledSchema::cem_core();
    let output =
        SchemaCompiler::emit_all(&schema, &CompilerOptions::default()).expect("emit_all");
    let manifests = [output.manifest.clone()];

    // cem-core publishes embedded 1.0.0 under the /1 identity URI.
    let (_, by_major) = resolve_uri("https://cem.dev/ns/core/1", &manifests)
        .expect("cem-core resolves from /1");
    assert_eq!(by_major.embedded_version, SemVer::new(1, 0, 0));
    assert_eq!(by_major.match_rule, SchemaVersionMatchRule::Major);

    let (_, by_bare) = resolve_uri("https://cem.dev/ns/core", &manifests)
        .expect("cem-core resolves from the bare URI");
    assert_eq!(by_bare.embedded_version, SemVer::new(1, 0, 0));
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("{prefix}_{}_{nanos}", std::process::id()))
}
