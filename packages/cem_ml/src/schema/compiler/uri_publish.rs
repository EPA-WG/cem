//! URI publication (AC-S-5): byte-stable `PublicationManifest` JSON
//! serialization, `.hash` sidecar bodies, and AC-V-10 URI-tail
//! resolution helpers.
//!
//! Reference: `cem-ml-stack-design.md` §13.2.5 / §13.2.6 / §13.2.11 and
//! `cem-ml-stack-design-impl.md` §3.4.2.4.
//!
//! Unlike the four content emitters, `uri_publish` is not a
//! `SchemaEmitter` — the manifest records the *other* artifacts' hashes
//! and so is built after them, with no `EmissionCursor` walk.
//! `emit_manifest_artifact` projects an already-assembled
//! `PublicationManifest` into byte-stable JSON; `resolve_uri` is the
//! AC-S-5 resolution surface a schema loader calls.

use std::cmp::Ordering;

use crate::schema::ir::{
    CompiledSchema, SchemaVersionConstraint, SchemaVersionMatchRule, SemVer,
};

use super::byte_stability::DeterministicWriter;
use super::emitter::relative_path;
use super::error::EmitError;
use super::output::{ArtifactKind, ContentHash, EmittedArtifact, PublicationManifest};

/// Well-known schema-URI prefix. Every published CEM schema URI begins
/// with this; the remainder is the namespace tail plus an optional
/// version segment (§13.2.5).
pub const WELL_KNOWN_PREFIX: &str = "https://cem.dev/ns/";

// ---------------------------------------------------------------------------
// Manifest JSON serialization (byte-stable; §13.2.11)
// ---------------------------------------------------------------------------

/// Project a `PublicationManifest` into a byte-stable `manifest.json`
/// artifact. The JSON encoding is fixed by §13.2.11: 2-space indent,
/// LF endings, one trailing newline, member order is the §13.2.3 wire
/// order, and `artifacts` is keyed in `ArtifactKind` ordinal order.
pub fn emit_manifest_artifact(
    schema: &CompiledSchema,
    manifest: &PublicationManifest,
) -> Result<EmittedArtifact, EmitError> {
    let mut writer = DeterministicWriter::new();
    write_manifest_json(&mut writer, manifest)?;
    let (bytes, content_hash) = writer.finalize()?;
    Ok(EmittedArtifact {
        kind: ArtifactKind::Manifest,
        relative_path: relative_path(schema, ArtifactKind::Manifest)?,
        bytes,
        content_hash,
        source_map: Default::default(),
    })
}

fn write_manifest_json(
    writer: &mut DeterministicWriter,
    manifest: &PublicationManifest,
) -> Result<(), EmitError> {
    writer.line("{")?;
    writer.indent();
    writer.line(&format!(
        "\"schema_uri\": {},",
        json_string(&manifest.schema_uri)
    ))?;
    writer.line(&format!(
        "\"embedded_version\": {},",
        json_string(&manifest.embedded_version.to_canonical_string())
    ))?;
    write_artifacts_object(writer, manifest)?;
    writer.line(&format!(
        "\"hash_scheme\": {}",
        json_string(manifest.hash_scheme)
    ))?;
    writer.dedent();
    writer.line("}")?;
    Ok(())
}

fn write_artifacts_object(
    writer: &mut DeterministicWriter,
    manifest: &PublicationManifest,
) -> Result<(), EmitError> {
    if manifest.artifacts.is_empty() {
        writer.line("\"artifacts\": {},")?;
        return Ok(());
    }
    writer.line("\"artifacts\": {")?;
    writer.indent();
    let count = manifest.artifacts.len();
    // BTreeMap iteration is `ArtifactKind` ordinal order (§13.2.4 rule 4).
    for (index, (kind, descriptor)) in manifest.artifacts.iter().enumerate() {
        let trailing = if index + 1 < count { "," } else { "" };
        writer.line(&format!("{}: {{", json_string(kind.manifest_key())))?;
        writer.indent();
        writer.line(&format!(
            "\"relative_path\": {},",
            json_string(&descriptor.relative_path)
        ))?;
        writer.line(&format!(
            "\"content_hash\": {},",
            json_string(&content_hash_field(&descriptor.content_hash))
        ))?;
        writer.line(&format!("\"byte_length\": {},", descriptor.byte_length))?;
        writer.line("\"emitted_by\": {")?;
        writer.indent();
        writer.line(&format!(
            "\"crate_version\": {},",
            json_string(descriptor.emitted_by.crate_version)
        ))?;
        writer.line(&format!(
            "\"emitter_name\": {}",
            json_string(descriptor.emitted_by.emitter_name)
        ))?;
        writer.dedent();
        writer.line("},")?;
        match &descriptor.validated_by {
            None => writer.line("\"validated_by\": null")?,
            Some(validator) => {
                writer.line("\"validated_by\": {")?;
                writer.indent();
                writer.line(&format!("\"name\": {},", json_string(validator.name)))?;
                match &validator.version {
                    Some(version) => {
                        writer.line(&format!("\"version\": {}", json_string(version)))?
                    }
                    None => writer.line("\"version\": null")?,
                }
                writer.dedent();
                writer.line("}")?;
            }
        }
        writer.dedent();
        writer.line(&format!("}}{trailing}"))?;
    }
    writer.dedent();
    writer.line("},")?;
    Ok(())
}

/// `ContentHash` rendered for the manifest: the same string the `.hash`
/// sidecar carries, minus the trailing newline — `{scheme}:{hex}`.
fn content_hash_field(hash: &ContentHash) -> String {
    format!("{}:{}", hash.scheme, hash.hex)
}

/// Minimal RFC 8259 string escaper. CEM schema URIs, relative paths,
/// hex digests, and emitter names are ASCII without quotes or
/// backslashes, so in practice no byte is rewritten — the escaper is
/// here so a future non-ASCII namespace tail still produces valid JSON.
fn json_string(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 2);
    out.push('"');
    for ch in raw.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            ch if (ch as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", ch as u32))
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// Sidecar path for an artifact: `<artifact-path>.hash`. The sidecar
/// body is `ContentHash::to_sidecar_string()`.
pub fn sidecar_relative_path(artifact_relative_path: &str) -> String {
    format!("{artifact_relative_path}.hash")
}

// ---------------------------------------------------------------------------
// URI-tail resolution (AC-S-5 / AC-V-10)
// ---------------------------------------------------------------------------

/// A parsed schema URI: the namespace tail (the publication-tree
/// directory key) plus the version-tail constraint and the AC-V-10
/// match rule its form implies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaUriRef {
    pub namespace_tail: String,
    pub constraint: SchemaVersionConstraint,
    pub match_rule: SchemaVersionMatchRule,
}

/// Outcome of resolving a declared URI against the published
/// manifests. Carries everything an AC-V-13 `cem.v.semver_resolved`
/// event needs: the URI as declared, the schema's own URI, the chosen
/// embedded version, and which AC-V-10 rule fired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UriResolution {
    pub declared_uri: String,
    pub schema_uri: String,
    pub embedded_version: SemVer,
    pub match_rule: SchemaVersionMatchRule,
}

/// Parse a well-known schema URI into its namespace tail and
/// version-tail constraint. Returns `None` for URIs outside the
/// `https://cem.dev/ns/` scheme or with no namespace segment.
pub fn parse_schema_uri(uri: &str) -> Option<SchemaUriRef> {
    let rest = uri.strip_prefix(WELL_KNOWN_PREFIX)?;
    let segments: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return None;
    }
    if let Some((constraint, match_rule)) =
        parse_version_segment(segments[segments.len() - 1])
    {
        let tail = &segments[..segments.len() - 1];
        if tail.is_empty() {
            return None; // a version with no namespace, e.g. `.../ns/1`
        }
        Some(SchemaUriRef {
            namespace_tail: tail.join("/"),
            constraint,
            match_rule,
        })
    } else {
        Some(SchemaUriRef {
            namespace_tail: segments.join("/"),
            constraint: SchemaVersionConstraint::Unconstrained,
            match_rule: SchemaVersionMatchRule::Unconstrained,
        })
    }
}

/// Recognise a trailing URI segment as a version constraint. Returns
/// `None` when the segment is a namespace component rather than a
/// version, so `.../ns/3d/1` keeps `3d` in the namespace tail.
fn parse_version_segment(
    segment: &str,
) -> Option<(SchemaVersionConstraint, SchemaVersionMatchRule)> {
    if !segment.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    // Peel a `+build` tail, then a `-prerelease` tail, off the numeric core.
    let (without_build, build) = match segment.split_once('+') {
        Some((core, meta)) => (core, Some(meta.to_owned())),
        None => (segment, None),
    };
    let (numeric, prerelease) = match without_build.split_once('-') {
        Some((core, pre)) => (core, Some(pre.to_owned())),
        None => (without_build, None),
    };
    let parts: Option<Vec<u64>> =
        numeric.split('.').map(parse_version_number).collect();
    let parts = parts?;
    match parts.as_slice() {
        // `/1` — MAJOR. A bare major never carries prerelease/build.
        [major] if prerelease.is_none() && build.is_none() => Some((
            SchemaVersionConstraint::Major(*major),
            SchemaVersionMatchRule::Major,
        )),
        // `/1.2` — MAJOR.MINOR.
        [major, minor] if prerelease.is_none() && build.is_none() => Some((
            SchemaVersionConstraint::MajorMinor(*major, *minor),
            SchemaVersionMatchRule::MajorMinor,
        )),
        // `/1.2.3`, `/1.2.3-rc.1`, `/1.2.3+sha.abc` — full SemVer.
        [major, minor, patch] => {
            let match_rule = if prerelease.is_some() {
                SchemaVersionMatchRule::PrereleaseExact
            } else {
                SchemaVersionMatchRule::Full
            };
            Some((
                SchemaVersionConstraint::Full(SemVer {
                    major: *major,
                    minor: *minor,
                    patch: *patch,
                    prerelease,
                    build,
                }),
                match_rule,
            ))
        }
        _ => None,
    }
}

/// Parse one numeric SemVer field. Rejects leading zeros per SemVer
/// 2.0 §2, so a malformed tail falls back to a namespace segment
/// rather than silently resolving.
fn parse_version_number(field: &str) -> Option<u64> {
    if field.is_empty() || (field.len() > 1 && field.starts_with('0')) {
        return None;
    }
    if !field.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    field.parse::<u64>().ok()
}

/// Resolve a declared schema URI against a set of published manifests
/// per AC-V-10. On success, returns the winning manifest and a
/// `UriResolution` describing the match. Pre-release embedded versions
/// are excluded from `unconstrained` / `major` / `major-minor` / `full`
/// matches — a pre-release is reachable only through a URI whose tail
/// names it exactly (resolved decision; see `cem-ml-stack-design.md`
/// §13.2.6).
pub fn resolve_uri<'m>(
    declared_uri: &str,
    manifests: &'m [PublicationManifest],
) -> Result<(&'m PublicationManifest, UriResolution), EmitError> {
    let uri_ref =
        parse_schema_uri(declared_uri).ok_or_else(|| EmitError::UnresolvableUri {
            uri: declared_uri.to_owned(),
            reason: "not a well-known https://cem.dev/ns/ schema URI",
        })?;
    let winner = manifests
        .iter()
        .filter(|m| {
            manifest_namespace_tail(m).as_deref()
                == Some(uri_ref.namespace_tail.as_str())
        })
        .filter(|m| constraint_matches(&uri_ref.constraint, &m.embedded_version))
        .max_by(|a, b| cmp_precedence(&a.embedded_version, &b.embedded_version));
    match winner {
        Some(manifest) => Ok((
            manifest,
            UriResolution {
                declared_uri: declared_uri.to_owned(),
                schema_uri: manifest.schema_uri.clone(),
                embedded_version: manifest.embedded_version.clone(),
                match_rule: uri_ref.match_rule,
            },
        )),
        None => Err(EmitError::UnresolvableUri {
            uri: declared_uri.to_owned(),
            reason: "no published embedded version satisfies the URI",
        }),
    }
}

fn manifest_namespace_tail(manifest: &PublicationManifest) -> Option<String> {
    parse_schema_uri(&manifest.schema_uri).map(|r| r.namespace_tail)
}

/// AC-V-10 URI-tail matching. Pre-releases are excluded from every rule
/// except `prerelease-exact`.
fn constraint_matches(constraint: &SchemaVersionConstraint, embedded: &SemVer) -> bool {
    match constraint {
        SchemaVersionConstraint::Unconstrained => embedded.prerelease.is_none(),
        SchemaVersionConstraint::Major(major) => {
            embedded.major == *major && embedded.prerelease.is_none()
        }
        SchemaVersionConstraint::MajorMinor(major, minor) => {
            embedded.major == *major
                && embedded.minor == *minor
                && embedded.prerelease.is_none()
        }
        SchemaVersionConstraint::Full(uri_version) => {
            if uri_version.prerelease.is_some() {
                // prerelease-exact: identical core and prerelease.
                embedded.major == uri_version.major
                    && embedded.minor == uri_version.minor
                    && embedded.patch == uri_version.patch
                    && embedded.prerelease == uri_version.prerelease
                    && build_matches(uri_version, embedded)
            } else {
                // full forgiving: same major, (minor, patch) at or above
                // the URI's, stable releases only (AC-V-10 / AC-V-2).
                embedded.prerelease.is_none()
                    && embedded.major == uri_version.major
                    && (embedded.minor, embedded.patch)
                        >= (uri_version.minor, uri_version.patch)
                    && build_matches(uri_version, embedded)
            }
        }
    }
}

/// AC-V-10 build-metadata rule: a URI without `+build` matches any
/// embedded build; a URI with `+build` matches that build exactly.
fn build_matches(uri_version: &SemVer, embedded: &SemVer) -> bool {
    match &uri_version.build {
        None => true,
        Some(build) => embedded.build.as_deref() == Some(build.as_str()),
    }
}

/// SemVer 2.0 §11 precedence. Build metadata is ignored (§10 / AC-V-11).
fn cmp_precedence(a: &SemVer, b: &SemVer) -> Ordering {
    (a.major, a.minor, a.patch)
        .cmp(&(b.major, b.minor, b.patch))
        .then_with(|| match (&a.prerelease, &b.prerelease) {
            (None, None) => Ordering::Equal,
            // A pre-release has lower precedence than the release.
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(x), Some(y)) => cmp_prerelease(x, y),
        })
}

/// Dot-separated pre-release identifier comparison (SemVer §11).
fn cmp_prerelease(a: &str, b: &str) -> Ordering {
    let mut a_idents = a.split('.');
    let mut b_idents = b.split('.');
    loop {
        match (a_idents.next(), b_idents.next()) {
            (None, None) => return Ordering::Equal,
            // A larger set of identifiers outranks a smaller one when
            // every preceding identifier is equal.
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let ordering = cmp_prerelease_ident(x, y);
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
        }
    }
}

fn cmp_prerelease_ident(x: &str, y: &str) -> Ordering {
    let x_numeric = !x.is_empty() && x.bytes().all(|b| b.is_ascii_digit());
    let y_numeric = !y.is_empty() && y.bytes().all(|b| b.is_ascii_digit());
    match (x_numeric, y_numeric) {
        // Numeric identifiers compare numerically.
        (true, true) => x
            .parse::<u64>()
            .unwrap_or(0)
            .cmp(&y.parse::<u64>().unwrap_or(0)),
        // Numeric identifiers always rank lower than alphanumeric.
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => x.cmp(y),
    }
}

/// AC-V-13 `cem.v.semver_resolved` rule label.
pub fn match_rule_label(rule: SchemaVersionMatchRule) -> &'static str {
    match rule {
        SchemaVersionMatchRule::Unconstrained => "unconstrained",
        SchemaVersionMatchRule::Major => "major",
        SchemaVersionMatchRule::MajorMinor => "major-minor",
        SchemaVersionMatchRule::Full => "full",
        SchemaVersionMatchRule::PrereleaseExact => "prerelease-exact",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::compiler::{CompilerOptions, SchemaCompiler};

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

    fn manifest_for(schema_uri: &str, version: SemVer) -> PublicationManifest {
        PublicationManifest {
            schema_uri: schema_uri.to_owned(),
            embedded_version: version,
            hash_scheme: ContentHash::SCHEME,
            artifacts: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn parse_schema_uri_recognises_each_tail_form() {
        let unconstrained = parse_schema_uri("https://cem.dev/ns/core").unwrap();
        assert_eq!(unconstrained.namespace_tail, "core");
        assert_eq!(
            unconstrained.constraint,
            SchemaVersionConstraint::Unconstrained
        );
        assert_eq!(unconstrained.match_rule, SchemaVersionMatchRule::Unconstrained);

        let major = parse_schema_uri("https://cem.dev/ns/core/1").unwrap();
        assert_eq!(major.namespace_tail, "core");
        assert_eq!(major.constraint, SchemaVersionConstraint::Major(1));
        assert_eq!(major.match_rule, SchemaVersionMatchRule::Major);

        let major_minor = parse_schema_uri("https://cem.dev/ns/core/1.2").unwrap();
        assert_eq!(
            major_minor.constraint,
            SchemaVersionConstraint::MajorMinor(1, 2)
        );
        assert_eq!(major_minor.match_rule, SchemaVersionMatchRule::MajorMinor);

        let full = parse_schema_uri("https://cem.dev/ns/core/1.2.3").unwrap();
        assert_eq!(
            full.constraint,
            SchemaVersionConstraint::Full(semver(1, 2, 3))
        );
        assert_eq!(full.match_rule, SchemaVersionMatchRule::Full);

        let pre = parse_schema_uri("https://cem.dev/ns/core/1.2.3-rc.1").unwrap();
        assert_eq!(
            pre.constraint,
            SchemaVersionConstraint::Full(prerelease(1, 2, 3, "rc.1"))
        );
        assert_eq!(pre.match_rule, SchemaVersionMatchRule::PrereleaseExact);
    }

    #[test]
    fn parse_schema_uri_keeps_a_numeric_namespace_segment_in_the_tail() {
        // `3d` starts with a digit but is not a version segment.
        let parsed = parse_schema_uri("https://cem.dev/ns/3d/1").unwrap();
        assert_eq!(parsed.namespace_tail, "3d");
        assert_eq!(parsed.constraint, SchemaVersionConstraint::Major(1));
    }

    #[test]
    fn parse_schema_uri_rejects_foreign_scheme_and_empty_tail() {
        assert!(parse_schema_uri("https://example.com/ns/core").is_none());
        assert!(parse_schema_uri("https://cem.dev/ns/").is_none());
        assert!(parse_schema_uri("https://cem.dev/ns/1").is_none());
    }

    #[test]
    fn parse_version_number_rejects_leading_zeros() {
        assert_eq!(parse_version_number("0"), Some(0));
        assert_eq!(parse_version_number("10"), Some(10));
        assert_eq!(parse_version_number("01"), None);
        assert_eq!(parse_version_number("1a"), None);
        assert_eq!(parse_version_number(""), None);
    }

    #[test]
    fn constraint_matching_excludes_prereleases_from_loose_rules() {
        let stable = semver(1, 4, 0);
        let pre = prerelease(1, 4, 0, "rc.1");
        for constraint in [
            SchemaVersionConstraint::Unconstrained,
            SchemaVersionConstraint::Major(1),
            SchemaVersionConstraint::MajorMinor(1, 4),
            SchemaVersionConstraint::Full(semver(1, 4, 0)),
        ] {
            assert!(constraint_matches(&constraint, &stable), "{constraint:?}");
            assert!(
                !constraint_matches(&constraint, &pre),
                "pre-release leaked into {constraint:?}"
            );
        }
        // A prerelease-exact tail matches the named pre-release only.
        let exact = SchemaVersionConstraint::Full(prerelease(1, 4, 0, "rc.1"));
        assert!(constraint_matches(&exact, &pre));
        assert!(!constraint_matches(&exact, &stable));
    }

    #[test]
    fn full_constraint_is_forgiving_within_the_major() {
        let constraint = SchemaVersionConstraint::Full(semver(1, 2, 3));
        assert!(constraint_matches(&constraint, &semver(1, 2, 3)));
        assert!(constraint_matches(&constraint, &semver(1, 9, 0)));
        assert!(!constraint_matches(&constraint, &semver(1, 2, 2)));
        assert!(!constraint_matches(&constraint, &semver(2, 0, 0)));
    }

    #[test]
    fn cmp_precedence_orders_releases_above_prereleases() {
        assert_eq!(
            cmp_precedence(&semver(1, 2, 3), &prerelease(1, 2, 3, "rc.1")),
            Ordering::Greater
        );
        assert_eq!(
            cmp_precedence(&prerelease(1, 0, 0, "alpha"), &prerelease(1, 0, 0, "beta")),
            Ordering::Less
        );
        // Numeric identifiers compare numerically, not lexically.
        assert_eq!(
            cmp_precedence(&prerelease(1, 0, 0, "rc.9"), &prerelease(1, 0, 0, "rc.10")),
            Ordering::Less
        );
        assert_eq!(
            cmp_precedence(&semver(1, 2, 0), &semver(1, 10, 0)),
            Ordering::Less
        );
    }

    #[test]
    fn match_rule_labels_use_the_ac_v_13_spelling() {
        assert_eq!(
            match_rule_label(SchemaVersionMatchRule::Unconstrained),
            "unconstrained"
        );
        assert_eq!(match_rule_label(SchemaVersionMatchRule::Major), "major");
        assert_eq!(
            match_rule_label(SchemaVersionMatchRule::MajorMinor),
            "major-minor"
        );
        assert_eq!(match_rule_label(SchemaVersionMatchRule::Full), "full");
        assert_eq!(
            match_rule_label(SchemaVersionMatchRule::PrereleaseExact),
            "prerelease-exact"
        );
    }

    #[test]
    fn resolve_uri_rejects_a_uri_outside_the_well_known_scheme() {
        let manifests = [manifest_for("https://cem.dev/ns/core/1", semver(1, 0, 0))];
        let err = resolve_uri("ftp://nope/core", &manifests).unwrap_err();
        assert!(matches!(err, EmitError::UnresolvableUri { .. }));
    }

    #[test]
    fn json_string_escapes_quotes_backslashes_and_controls() {
        assert_eq!(json_string("plain"), "\"plain\"");
        assert_eq!(json_string("a\"b\\c"), "\"a\\\"b\\\\c\"");
        assert_eq!(json_string("tab\there"), "\"tab\\there\"");
        assert_eq!(json_string("\u{01}"), "\"\\u0001\"");
        // Schema-URI characters need no escaping.
        assert_eq!(
            json_string("https://cem.dev/ns/core/1"),
            "\"https://cem.dev/ns/core/1\""
        );
    }

    #[test]
    fn emit_manifest_artifact_produces_well_formed_json() {
        let schema = CompiledSchema::cem_core();
        let output =
            SchemaCompiler::emit_all(&schema, &CompilerOptions::default()).unwrap();
        let manifest_artifact =
            emit_manifest_artifact(&schema, &output.manifest).unwrap();

        assert_eq!(manifest_artifact.kind, ArtifactKind::Manifest);
        assert_eq!(manifest_artifact.relative_path, "core/1.0.0/manifest.json");

        let text = std::str::from_utf8(&manifest_artifact.bytes).unwrap();
        assert!(text.starts_with("{\n"));
        assert!(text.ends_with("}\n"));
        // Wire order from §13.2.3: schema_uri, embedded_version,
        // artifacts, then hash_scheme.
        let schema_at = text.find("\"schema_uri\"").unwrap();
        let version_at = text.find("\"embedded_version\"").unwrap();
        let artifacts_at = text.find("\"artifacts\"").unwrap();
        let scheme_at = text.find("\"hash_scheme\"").unwrap();
        assert!(schema_at < version_at);
        assert!(version_at < artifacts_at);
        assert!(artifacts_at < scheme_at);
        // The default emit publishes three content artifacts; the
        // manifest never describes itself.
        assert!(text.contains("\"relaxng-xml\""));
        assert!(text.contains("\"relaxng-compact\""));
        assert!(text.contains("\"typescript-dts\""));
        assert!(!text.contains("\"manifest\":"));
        assert!(text.contains("\"hash_scheme\": \"cem-bin/1+blake3\""));
    }

    #[test]
    fn emit_manifest_artifact_is_byte_stable() {
        let schema = CompiledSchema::cem_core();
        let output =
            SchemaCompiler::emit_all(&schema, &CompilerOptions::default()).unwrap();
        let first = emit_manifest_artifact(&schema, &output.manifest).unwrap();
        let second = emit_manifest_artifact(&schema, &output.manifest).unwrap();
        assert_eq!(first.bytes, second.bytes);
        assert_eq!(first.content_hash, second.content_hash);
        assert_eq!(first.content_hash, ContentHash::from_blake3(&first.bytes));
    }

    #[test]
    fn sidecar_relative_path_appends_hash() {
        assert_eq!(
            sidecar_relative_path("core/1.0.0/cem-core.rng"),
            "core/1.0.0/cem-core.rng.hash"
        );
    }
}
