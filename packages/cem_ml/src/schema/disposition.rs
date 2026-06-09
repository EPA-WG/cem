//! AC-P-6.7 / AC-P-V-6 unknown-namespace disposition — decision-core.
//!
//! When a region's namespace resolves to **no metadata, no explicit schema,
//! and no rule**, the effective scope policy MUST select one defined behavior:
//!
//! - [`Disposition::Reject`] — a hard error; the region is not accepted.
//! - [`Disposition::Allow`] — accept as unvalidated foreign content.
//! - [`Disposition::Ignore`] — drop the region with a report event.
//!
//! The **default** is mode-selected (the BR-VC-9 run-mode disposition): an
//! application run rejects unknown data/security namespaces and allows unknown
//! presentation namespaces; build/SSR rejects all; development allows all. A
//! scope-policy rule MAY override within the mode, and the outcome MUST be
//! deterministic. `allow` and `ignore` are non-execution modes unless a
//! separate handler is explicitly selected.
//!
//! This module is the pure decision-core only. Detecting an unresolved-namespace
//! region and applying the outcome in the schema machine — plus threading the
//! run mode through the machine — is a follow-up slice. Pure, total over the
//! typed domain, and no I/O, so it is exhaustively unit-testable. It is the
//! parser-side counterpart to the cem-elements contract disposition that backs
//! fitness function FF-4; this side is the literal AC-P-V-6 verifier (tracked
//! in `docs/todo.md`).

/// Run mode selecting the default disposition for an unresolved-namespace region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Application,
    BuildSsr,
    Development,
}

/// Defined behavior for a region whose namespace resolves to nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// Hard error — the region is not accepted.
    Reject,
    /// Accept as unvalidated foreign content.
    Allow,
    /// Drop the region with a report event.
    Ignore,
}

/// Class hint for an unknown namespace, when a scope-policy rule (or host
/// metadata) provides one. Absent a hint the namespace is unclassified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceClass {
    Presentation,
    DataSecurity,
}

/// Where the resolved disposition came from — recorded for auditability
/// (BR-VC-9 "record the active mode … so the disposition is auditable").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispositionSource {
    RunModeDefault,
    ScopePolicyOverride,
}

/// A single, auditable disposition decision for an unresolved-namespace region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispositionDecision {
    pub mode: RunMode,
    pub class: Option<NamespaceClass>,
    pub disposition: Disposition,
    pub source: DispositionSource,
}

/// The mode-selected default disposition per AC-P-6.7 / BR-VC-9.
///
/// Build/SSR rejects every unknown namespace; development allows every unknown
/// namespace; an application run rejects unknown data/security namespaces and
/// allows unknown presentation namespaces. An **unclassified** unknown namespace
/// in an application run is rejected conservatively (treated as potentially
/// data/security until a scope-policy rule classifies it otherwise).
pub fn default_disposition(mode: RunMode, class: Option<NamespaceClass>) -> Disposition {
    match mode {
        RunMode::BuildSsr => Disposition::Reject,
        RunMode::Development => Disposition::Allow,
        RunMode::Application => match class {
            Some(NamespaceClass::Presentation) => Disposition::Allow,
            Some(NamespaceClass::DataSecurity) => Disposition::Reject,
            None => Disposition::Reject,
        },
    }
}

/// Resolve the effective disposition. A scope-policy `policy_override` wins over
/// the mode default (AC-P-6.7 "Scope policy MAY override within the mode"); the
/// outcome is deterministic for a given `(mode, class, policy_override)`.
pub fn resolve_disposition(
    mode: RunMode,
    class: Option<NamespaceClass>,
    policy_override: Option<Disposition>,
) -> DispositionDecision {
    match policy_override {
        Some(disposition) => DispositionDecision {
            mode,
            class,
            disposition,
            source: DispositionSource::ScopePolicyOverride,
        },
        None => DispositionDecision {
            mode,
            class,
            disposition: default_disposition(mode, class),
            source: DispositionSource::RunModeDefault,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLASSES: [Option<NamespaceClass>; 3] = [
        None,
        Some(NamespaceClass::Presentation),
        Some(NamespaceClass::DataSecurity),
    ];

    #[test]
    fn build_ssr_rejects_every_class() {
        for class in CLASSES {
            assert_eq!(default_disposition(RunMode::BuildSsr, class), Disposition::Reject);
        }
    }

    #[test]
    fn development_allows_every_class() {
        for class in CLASSES {
            assert_eq!(default_disposition(RunMode::Development, class), Disposition::Allow);
        }
    }

    #[test]
    fn application_is_per_class() {
        assert_eq!(
            default_disposition(RunMode::Application, Some(NamespaceClass::Presentation)),
            Disposition::Allow
        );
        assert_eq!(
            default_disposition(RunMode::Application, Some(NamespaceClass::DataSecurity)),
            Disposition::Reject
        );
        // Unclassified → conservative reject.
        assert_eq!(default_disposition(RunMode::Application, None), Disposition::Reject);
    }

    #[test]
    fn scope_policy_override_wins_in_every_mode() {
        let modes = [RunMode::Application, RunMode::BuildSsr, RunMode::Development];
        let overrides = [Disposition::Reject, Disposition::Allow, Disposition::Ignore];
        for mode in modes {
            for class in CLASSES {
                for over in overrides {
                    let decision = resolve_disposition(mode, class, Some(over));
                    assert_eq!(decision.disposition, over);
                    assert_eq!(decision.source, DispositionSource::ScopePolicyOverride);
                }
            }
        }
    }

    #[test]
    fn no_override_uses_the_mode_default() {
        let decision = resolve_disposition(RunMode::Application, Some(NamespaceClass::Presentation), None);
        assert_eq!(decision.disposition, Disposition::Allow);
        assert_eq!(decision.source, DispositionSource::RunModeDefault);

        let rejected = resolve_disposition(RunMode::BuildSsr, Some(NamespaceClass::Presentation), None);
        assert_eq!(rejected.disposition, Disposition::Reject);
        assert_eq!(rejected.source, DispositionSource::RunModeDefault);
    }

    #[test]
    fn decision_record_echoes_inputs() {
        let decision = resolve_disposition(RunMode::Application, Some(NamespaceClass::DataSecurity), None);
        assert_eq!(decision.mode, RunMode::Application);
        assert_eq!(decision.class, Some(NamespaceClass::DataSecurity));
    }

    #[test]
    fn ignore_is_only_reachable_via_an_explicit_override() {
        // The mode defaults never select Ignore; it is an opt-in scope-policy choice.
        for mode in [RunMode::Application, RunMode::BuildSsr, RunMode::Development] {
            for class in CLASSES {
                assert_ne!(default_disposition(mode, class), Disposition::Ignore);
            }
        }
        assert_eq!(
            resolve_disposition(RunMode::Application, None, Some(Disposition::Ignore)).disposition,
            Disposition::Ignore
        );
    }
}
