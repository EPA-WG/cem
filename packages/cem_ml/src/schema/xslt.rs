//! AC-P-6.8 / AC-P-V-4 / AC-P-V-7 XSLT region dispatch — decision-core.
//!
//! The XSLT namespace `http://www.w3.org/1999/XSL/Transform` is dispatchable as
//! an embedded content type (AC-P-6.1–6.7): on explicit opt-in the parser opens
//! an isolated Layer-5 handoff, does **not** interpret XSLT constructs as
//! CEM-ML, and the surrounding content type resumes on return. Key properties:
//!
//! - **Version-pinning (AC-P-V-4):** the dispatched region's version comes from
//!   the document's native `xsl:stylesheet/@version`, resolved against a
//!   CEM-owned XSLT **adapter SemVer line** — *not* from the version-stable
//!   namespace URI, and *not* from the CEM-ML core version. So a CEM-ML core
//!   MAJOR bump leaves a dispatched region's resolved version (and expanded
//!   names) unchanged.
//! - **Explicit opt-in (AC-P-V-7):** absent host namespace metadata or a
//!   scope-policy rule, an `xsl:` subtree follows the AC-P-6.7 unknown-namespace
//!   default ([`crate::schema::disposition`]); adding an explicit XSLT dispatch
//!   rule opens the isolated handoff.
//!
//! Dispatch/isolation/version-pinning do **not** depend on which XSLT versions
//! the engine can *execute* — execution is capability-gated and deferred
//! (AC-P-6.9). This module is the pure decision-core; recognizing the `xsl:`
//! namespace in the schema machine and opening the handoff is a follow-up slice.

use crate::schema::ir::SemVer;

/// The version-stable XSLT namespace URI. It is NOT a version source.
pub const XSL_NAMESPACE: &str = "http://www.w3.org/1999/XSL/Transform";

/// The CEM-owned XSLT adapter SemVer line. Dispatch / isolation / version-pinning
/// (AC-P-6.8) do not depend on which XSLT versions the adapter can *execute*
/// (AC-P-6.9, capability-gated + deferred); this is the adapter-contract version.
pub const ADAPTER_LINE: SemVer = SemVer::new(1, 0, 0);

/// An XSLT compatibility version as declared by `xsl:stylesheet/@version`
/// (an `xs:decimal` such as `1.0`, `2.0`, `3.0`). Tier A records MAJOR.MINOR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XsltVersion {
    pub major: u64,
    pub minor: u64,
}

/// Parse an `xsl:stylesheet/@version` value (`MAJOR` or `MAJOR.MINOR`, numeric).
/// `1` → `1.0`. Returns `None` for empty, non-numeric, or 3+-component input.
pub fn parse_xslt_version(raw: &str) -> Option<XsltVersion> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut parts = trimmed.split('.');
    let major = numeric_field(parts.next()?)?;
    let minor = match parts.next() {
        Some(field) => numeric_field(field)?,
        None => 0,
    };
    if parts.next().is_some() {
        return None; // XSLT versions are MAJOR or MAJOR.MINOR, never MAJOR.MINOR.PATCH.
    }
    Some(XsltVersion { major, minor })
}

fn numeric_field(field: &str) -> Option<u64> {
    if field.is_empty() || (field.len() > 1 && field.starts_with('0')) {
        return None;
    }
    if !field.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    field.parse::<u64>().ok()
}

/// A pinned XSLT dispatch identity. `requested` is the document's
/// `xsl:stylesheet/@version`; `adapter_line` is the CEM XSLT adapter version.
/// Neither field references the CEM-ML core version, so the identity is stable
/// across a CEM-ML core MAJOR bump (AC-P-V-4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XsltDispatch {
    pub requested: XsltVersion,
    pub adapter_line: SemVer,
}

/// Why an opted-in `xsl:` region could not be version-pinned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XsltVersionError {
    /// No `xsl:stylesheet/@version` was declared.
    Missing,
    /// `@version` is present but not a valid XSLT version.
    Malformed,
}

/// Pin the requested XSLT version against the adapter line. The namespace URI is
/// not consulted (it is version-stable).
pub fn resolve_xslt_dispatch(
    requested_version: Option<&str>,
) -> Result<XsltDispatch, XsltVersionError> {
    let raw = requested_version.ok_or(XsltVersionError::Missing)?;
    let requested = parse_xslt_version(raw).ok_or(XsltVersionError::Malformed)?;
    Ok(XsltDispatch {
        requested,
        adapter_line: ADAPTER_LINE,
    })
}

/// Outcome for an `xsl:`-namespace region (AC-P-6.8 / AC-P-V-7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XsltRegionOutcome {
    /// Explicit opt-in + a valid `@version`: open an isolated, version-pinned
    /// XSLT handoff. No CEM-ML interpretation; execution is gated separately
    /// (AC-P-6.9).
    Dispatch(XsltDispatch),
    /// Explicit opt-in but the `@version` is missing/malformed → diagnose+reject.
    VersionError(XsltVersionError),
    /// No explicit opt-in → the region follows the AC-P-6.7 unknown-namespace
    /// default disposition (see [`crate::schema::disposition`]).
    UnknownNamespaceDefault,
}

/// Decide the fate of an `xsl:` region. `opt_in` is true when host namespace
/// metadata or a scope-policy rule explicitly dispatches XSLT.
pub fn xslt_region_outcome(opt_in: bool, requested_version: Option<&str>) -> XsltRegionOutcome {
    if !opt_in {
        return XsltRegionOutcome::UnknownNamespaceDefault;
    }
    match resolve_xslt_dispatch(requested_version) {
        Ok(dispatch) => XsltRegionOutcome::Dispatch(dispatch),
        Err(error) => XsltRegionOutcome::VersionError(error),
    }
}

/// Whether a resolved namespace URI is the XSLT namespace.
pub fn is_xslt_namespace(uri: &str) -> bool {
    uri == XSL_NAMESPACE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_major_and_major_minor() {
        assert_eq!(parse_xslt_version("1.0"), Some(XsltVersion { major: 1, minor: 0 }));
        assert_eq!(parse_xslt_version("3.0"), Some(XsltVersion { major: 3, minor: 0 }));
        assert_eq!(parse_xslt_version("1.1"), Some(XsltVersion { major: 1, minor: 1 }));
        assert_eq!(parse_xslt_version("2"), Some(XsltVersion { major: 2, minor: 0 }));
        assert_eq!(parse_xslt_version("  3.0  "), Some(XsltVersion { major: 3, minor: 0 }));
    }

    #[test]
    fn rejects_malformed_versions() {
        assert_eq!(parse_xslt_version(""), None);
        assert_eq!(parse_xslt_version("x"), None);
        assert_eq!(parse_xslt_version("1.0.0"), None); // no PATCH in XSLT
        assert_eq!(parse_xslt_version("1.x"), None);
        assert_eq!(parse_xslt_version("01"), None); // leading zero
    }

    #[test]
    fn resolves_dispatch_pinned_to_the_adapter_line() {
        let dispatch = resolve_xslt_dispatch(Some("1.0")).unwrap();
        assert_eq!(dispatch.requested, XsltVersion { major: 1, minor: 0 });
        assert_eq!(dispatch.adapter_line, ADAPTER_LINE);
    }

    #[test]
    fn missing_and_malformed_versions_are_distinguished() {
        assert_eq!(resolve_xslt_dispatch(None), Err(XsltVersionError::Missing));
        assert_eq!(resolve_xslt_dispatch(Some("nope")), Err(XsltVersionError::Malformed));
    }

    #[test]
    fn no_opt_in_falls_to_the_unknown_namespace_default() {
        assert_eq!(
            xslt_region_outcome(false, Some("1.0")),
            XsltRegionOutcome::UnknownNamespaceDefault
        );
        // Even without a version, no opt-in means the unknown-namespace path.
        assert_eq!(
            xslt_region_outcome(false, None),
            XsltRegionOutcome::UnknownNamespaceDefault
        );
    }

    #[test]
    fn opt_in_dispatches_a_version_pinned_handoff() {
        match xslt_region_outcome(true, Some("3.0")) {
            XsltRegionOutcome::Dispatch(d) => {
                assert_eq!(d.requested, XsltVersion { major: 3, minor: 0 });
                assert_eq!(d.adapter_line, ADAPTER_LINE);
            }
            other => panic!("expected Dispatch, got {other:?}"),
        }
    }

    #[test]
    fn opt_in_without_a_valid_version_is_a_version_error() {
        assert_eq!(
            xslt_region_outcome(true, None),
            XsltRegionOutcome::VersionError(XsltVersionError::Missing)
        );
        assert_eq!(
            xslt_region_outcome(true, Some("1.0.0")),
            XsltRegionOutcome::VersionError(XsltVersionError::Malformed)
        );
    }

    #[test]
    fn version_pinning_is_independent_of_cem_ml_core_version() {
        // AC-P-V-4: the resolution consults only `@version` + the adapter line —
        // never the CEM-ML core version — so a core MAJOR bump cannot change a
        // dispatched region's pinned identity. Re-resolving the same `@version`
        // is byte-stable.
        let first = resolve_xslt_dispatch(Some("2.0")).unwrap();
        let second = resolve_xslt_dispatch(Some("2.0")).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.requested, XsltVersion { major: 2, minor: 0 });
    }

    #[test]
    fn recognizes_the_version_stable_namespace() {
        assert!(is_xslt_namespace(XSL_NAMESPACE));
        assert!(!is_xslt_namespace("urn:example:widgets:1"));
        assert!(!is_xslt_namespace("http://www.w3.org/2000/svg"));
    }
}
