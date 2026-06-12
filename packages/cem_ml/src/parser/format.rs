//! Document-format directive resolution for canonical CEM-ML
//! (`@doc cem-ml <version>`).
//!
//! AC-F-8 / AC-F-V-6: a persisted top-level `.cem` document MUST start
//! with `@doc cem-ml <version>` before any non-trivia directive or
//! item. This module parses the directive value, resolves the version
//! constraint against the embedded Tier A supported version, and maps
//! each failure mode to the canonical diagnostic code listed in
//! AC-F-8.

use crate::schema::ir::{SchemaVersionConstraint, SchemaVersionMatchRule, SemVer};

/// Tier A canonical document-format identity recorded on the document
/// root scope after successful resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentFormatIdentity {
    pub format_id: String,
    pub content_type: String,
    /// Authoritative full SemVer of the loaded document-format profile
    /// (never the URI-tail / directive-tail partial constraint).
    pub format_version: SemVer,
}

/// Tier A defines exactly one canonical document format (AC-F-8).
pub const SUPPORTED_FORMAT_ID: &str = "cem-ml";
pub const SUPPORTED_CONTENT_TYPE: &str = "text/cem-ml";
pub const SUPPORTED_VERSION: SemVer = SemVer::new(1, 0, 0);

/// Diagnostic code for a missing top-level `@doc` directive.
pub const VERSION_MISSING_CODE: &str = "cem.doc.version_missing";
/// Info-level diagnostic code recorded on successful resolution
/// (AC-F-8 / AC-O-3).
pub const VERSION_RESOLVED_CODE: &str = "cem.doc.version_resolved";

/// Failure modes for `resolve_doc_directive`. Each variant maps to
/// exactly one of the diagnostic codes listed in AC-F-8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocDirectiveError {
    /// The directive body could not be split into
    /// `<format-id> <version>` or the version segment is not a valid
    /// SemVer constraint.
    SemverInvalid { value: String },
    /// The format id is anything other than `cem-ml`.
    FormatUnknown { format_id: String },
    /// The constraint is well-formed but no embedded version satisfies
    /// it (major mismatch or future minor / patch).
    VersionUnsupported { declared: String },
    /// The constraint names a pre-release that the embedded version
    /// does not match.
    PrereleaseUnmatched { declared: String },
}

impl DocDirectiveError {
    pub fn code(&self) -> &'static str {
        match self {
            DocDirectiveError::SemverInvalid { .. } => "cem.doc.semver_invalid",
            DocDirectiveError::FormatUnknown { .. } => "cem.doc.format_unknown",
            DocDirectiveError::VersionUnsupported { .. } => "cem.doc.version_unsupported",
            DocDirectiveError::PrereleaseUnmatched { .. } => "cem.doc.prerelease_unmatched",
        }
    }

    pub fn message(&self) -> String {
        match self {
            DocDirectiveError::SemverInvalid { value } => format!(
                "`@doc` directive value `{value}` is not `<format-id> <version>` with a valid SemVer constraint"
            ),
            DocDirectiveError::FormatUnknown { format_id } => format!(
                "`@doc` format id `{format_id}` is not supported (expected `{SUPPORTED_FORMAT_ID}`)"
            ),
            DocDirectiveError::VersionUnsupported { declared } => format!(
                "`@doc cem-ml {declared}` is not satisfied by the embedded Tier A version `{}`",
                SUPPORTED_VERSION
            ),
            DocDirectiveError::PrereleaseUnmatched { declared } => format!(
                "`@doc cem-ml {declared}` declares a pre-release that does not match the embedded version `{}`",
                SUPPORTED_VERSION
            ),
        }
    }
}

/// Resolve a `@doc cem-ml <version>` directive body. `text` is the
/// directive value as authored — e.g. `"cem-ml 1"`,
/// `"cem-ml 1.0.0"`, `"cem-ml 1.0.0-rc.1"`. Whitespace around either
/// token is tolerated.
pub fn resolve_doc_directive(text: &str) -> Result<DocumentFormatIdentity, DocDirectiveError> {
    let trimmed = text.trim();
    let mut parts = trimmed.split_ascii_whitespace();
    let format_id = parts.next().unwrap_or("");
    let version_str = parts.next().unwrap_or("");
    let trailing = parts.next();
    if format_id.is_empty() || version_str.is_empty() || trailing.is_some() {
        return Err(DocDirectiveError::SemverInvalid {
            value: trimmed.to_owned(),
        });
    }
    if format_id != SUPPORTED_FORMAT_ID {
        return Err(DocDirectiveError::FormatUnknown {
            format_id: format_id.to_owned(),
        });
    }
    let (constraint, _rule) = parse_version_constraint(version_str).ok_or_else(|| {
        DocDirectiveError::SemverInvalid {
            value: version_str.to_owned(),
        }
    })?;
    resolve_against_supported(version_str, constraint)
}

fn resolve_against_supported(
    declared: &str,
    constraint: SchemaVersionConstraint,
) -> Result<DocumentFormatIdentity, DocDirectiveError> {
    let supported = &SUPPORTED_VERSION;
    let satisfied = match &constraint {
        // Unreachable from `resolve_doc_directive` — a present
        // `<version>` always parses into one of the three concrete
        // forms below. Kept exhaustive so a future caller cannot
        // silently bypass the supported-version check.
        SchemaVersionConstraint::Unconstrained => false,
        SchemaVersionConstraint::Major(m) => *m == supported.major,
        SchemaVersionConstraint::MajorMinor(m, n) => {
            *m == supported.major && supported.minor >= *n
        }
        SchemaVersionConstraint::Full(v) => {
            if v.prerelease.is_some() {
                // Prerelease-exact path: AC-V-10 / AC-F-8 require an
                // exact pre-release match. The embedded Tier A version
                // has no pre-release tag, so any prerelease constraint
                // fails with the dedicated code.
                return Err(DocDirectiveError::PrereleaseUnmatched {
                    declared: declared.to_owned(),
                });
            }
            v.major == supported.major
                && (supported.minor, supported.patch) >= (v.minor, v.patch)
        }
    };
    if satisfied {
        Ok(DocumentFormatIdentity {
            format_id: SUPPORTED_FORMAT_ID.to_owned(),
            content_type: SUPPORTED_CONTENT_TYPE.to_owned(),
            format_version: SUPPORTED_VERSION,
        })
    } else {
        Err(DocDirectiveError::VersionUnsupported {
            declared: declared.to_owned(),
        })
    }
}

/// Parse a version-tail constraint string into a
/// `SchemaVersionConstraint`. AC-F-8 reuses the AC-V-10 constraint
/// grammar — `MAJOR`, `MAJOR.MINOR`, or `MAJOR.MINOR.PATCH` with
/// optional `-prerelease` and `+build` suffixes.
fn parse_version_constraint(
    raw: &str,
) -> Option<(SchemaVersionConstraint, SchemaVersionMatchRule)> {
    if raw.is_empty() || !raw.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let (without_build, build) = match raw.split_once('+') {
        Some((core, meta)) => (core, Some(meta.to_owned())),
        None => (raw, None),
    };
    let (numeric, prerelease) = match without_build.split_once('-') {
        Some((core, pre)) => (core, Some(pre.to_owned())),
        None => (without_build, None),
    };
    let parts: Option<Vec<u64>> = numeric.split('.').map(parse_numeric_field).collect();
    let parts = parts?;
    match parts.as_slice() {
        [major] if prerelease.is_none() && build.is_none() => Some((
            SchemaVersionConstraint::Major(*major),
            SchemaVersionMatchRule::Major,
        )),
        [major, minor] if prerelease.is_none() && build.is_none() => Some((
            SchemaVersionConstraint::MajorMinor(*major, *minor),
            SchemaVersionMatchRule::MajorMinor,
        )),
        [major, minor, patch] => {
            let rule = if prerelease.is_some() {
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
                rule,
            ))
        }
        _ => None,
    }
}

fn parse_numeric_field(field: &str) -> Option<u64> {
    if field.is_empty() || (field.len() > 1 && field.starts_with('0')) {
        return None;
    }
    if !field.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    field.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn major_constraint_resolves_to_supported_full_semver() {
        let id = resolve_doc_directive("cem-ml 1").unwrap();
        assert_eq!(id.format_id, "cem-ml");
        assert_eq!(id.content_type, "text/cem-ml");
        assert_eq!(id.format_version, SUPPORTED_VERSION);
    }

    #[test]
    fn major_minor_constraint_resolves() {
        let id = resolve_doc_directive("cem-ml 1.0").unwrap();
        assert_eq!(id.format_version, SUPPORTED_VERSION);
    }

    #[test]
    fn full_constraint_resolves_when_exactly_at_embedded() {
        let id = resolve_doc_directive("cem-ml 1.0.0").unwrap();
        assert_eq!(id.format_version, SUPPORTED_VERSION);
    }

    #[test]
    fn surrounding_whitespace_is_tolerated() {
        let id = resolve_doc_directive("  cem-ml   1.0.0  ").unwrap();
        assert_eq!(id.format_version, SUPPORTED_VERSION);
    }

    #[test]
    fn unknown_format_id_yields_format_unknown() {
        let err = resolve_doc_directive("widget 1").unwrap_err();
        assert!(matches!(err, DocDirectiveError::FormatUnknown { .. }));
        assert_eq!(err.code(), "cem.doc.format_unknown");
    }

    #[test]
    fn malformed_version_token_yields_semver_invalid() {
        let err = resolve_doc_directive("cem-ml not-a-version").unwrap_err();
        assert!(matches!(err, DocDirectiveError::SemverInvalid { .. }));
        assert_eq!(err.code(), "cem.doc.semver_invalid");
    }

    #[test]
    fn missing_version_token_yields_semver_invalid() {
        let err = resolve_doc_directive("cem-ml").unwrap_err();
        assert!(matches!(err, DocDirectiveError::SemverInvalid { .. }));
    }

    #[test]
    fn trailing_garbage_yields_semver_invalid() {
        let err = resolve_doc_directive("cem-ml 1 something-else").unwrap_err();
        assert!(matches!(err, DocDirectiveError::SemverInvalid { .. }));
    }

    #[test]
    fn major_mismatch_yields_version_unsupported() {
        let err = resolve_doc_directive("cem-ml 2").unwrap_err();
        assert!(matches!(err, DocDirectiveError::VersionUnsupported { .. }));
        assert_eq!(err.code(), "cem.doc.version_unsupported");
    }

    #[test]
    fn future_minor_yields_version_unsupported() {
        let err = resolve_doc_directive("cem-ml 1.2").unwrap_err();
        assert!(matches!(err, DocDirectiveError::VersionUnsupported { .. }));
    }

    #[test]
    fn future_full_version_yields_version_unsupported() {
        let err = resolve_doc_directive("cem-ml 1.0.1").unwrap_err();
        assert!(matches!(err, DocDirectiveError::VersionUnsupported { .. }));
    }

    #[test]
    fn prerelease_constraint_yields_prerelease_unmatched() {
        let err = resolve_doc_directive("cem-ml 1.0.0-rc.1").unwrap_err();
        assert!(matches!(err, DocDirectiveError::PrereleaseUnmatched { .. }));
        assert_eq!(err.code(), "cem.doc.prerelease_unmatched");
    }

    #[test]
    fn leading_zero_numeric_field_yields_semver_invalid() {
        let err = resolve_doc_directive("cem-ml 01").unwrap_err();
        assert!(matches!(err, DocDirectiveError::SemverInvalid { .. }));
    }

    #[test]
    fn build_metadata_only_constraint_resolves() {
        // /1.0.0+sha.abc has no prerelease and same (minor, patch); the
        // embedded version (no build) is matched per AC-F-8 / AC-V-10
        // (build metadata is ignored for satisfaction precedence; the
        // embedded full version is what's recorded).
        let id = resolve_doc_directive("cem-ml 1.0.0+sha.abc").unwrap();
        assert_eq!(id.format_version, SUPPORTED_VERSION);
    }
}
