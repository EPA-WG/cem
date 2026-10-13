//! Captured native HTML base evidence and explicit browser compatibility.

use crate::{
    html_base::*,
    parser::{tree::RetainedCemTree, AstNodeId},
};
use std::sync::Arc;

pub const CHROMIUM_BASE_PROFILE_VERSION: &str = "148.0.7778.96";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlBaseCompatibilityProfile {
    /// Admit only an observed URL equal to the standard native result.
    StandardAgreement,
    /// Explicit, version-pinned compatibility for the measured HTTP(S) cases.
    Chromium148,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlBaseCompatibility {
    StandardAgreement,
    ChromiumInvalidHref,
    ChromiumEmptyHref,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlBaseEpochKind {
    /// A first href base became active, or its href was set (even to the same value).
    HrefFreeze,
    /// No href base exists. Capture the current fallback; it is not a frozen base.
    FallbackSnapshot,
}

/// Trusted execution-host identities. The document ID identifies a document
/// lifetime, not its URL. Hosts advance revision for relevant document/context
/// changes and epoch for every base freeze/fallback capture, including A-B-A.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlBaseEpoch {
    pub document_id: String,
    pub revision: String,
    pub epoch: u64,
    pub kind: HtmlBaseEpochKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlBaseBrowser {
    pub engine: String,
    pub version: String,
}

/// Scalar host observation, bound to a native slot through the host's execution
/// correspondence. It must be captured at the named epoch, not inferred later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlBaseObservation {
    pub binding: HtmlBaseEpoch,
    pub browser: HtmlBaseBrowser,
    pub selected_attribute: Option<AstNodeId>,
    pub policy_stamp: String,
    pub base_uri: String,
}

pub struct HtmlBaseFreezeRequest {
    pub tree: Arc<RetainedCemTree>,
    pub context: HtmlBaseContext,
    pub binding: HtmlBaseEpoch,
    pub profile: HtmlBaseCompatibilityProfile,
    pub limits: HtmlBaseLimits,
}

/// Preparation is not confirmed execution evidence. Private state prevents a
/// caller from changing the captured fallback, policy decision or native owner.
#[derive(Debug)]
pub struct PreparedHtmlBaseFreeze {
    resolution: Arc<HtmlBaseResolution>,
    binding: HtmlBaseEpoch,
    profile: HtmlBaseCompatibilityProfile,
    decision: Option<HtmlBasePolicyDecision>,
    url_bytes: usize,
}

/// Confirmed base evidence for one host read set. It never certifies historical
/// resource loads, DOM observation completeness or mutation authorization.
#[derive(Debug)]
pub struct CapturedHtmlBaseEvidence {
    resolution: Arc<HtmlBaseResolution>,
    observation: HtmlBaseObservation,
    profile: HtmlBaseCompatibilityProfile,
    compatibility: HtmlBaseCompatibility,
    decision: Option<HtmlBasePolicyDecision>,
}

pub fn prepare_html_base_freeze(
    request: HtmlBaseFreezeRequest,
    decide: impl FnOnce(&HtmlBasePolicyRequest<'_>) -> Result<HtmlBasePolicyDecision, String>,
) -> Result<PreparedHtmlBaseFreeze, String> {
    let HtmlBaseFreezeRequest {
        tree,
        context,
        binding,
        profile,
        limits,
    } = request;
    if binding.document_id.trim().is_empty()
        || binding.revision.trim().is_empty()
        || binding.epoch == 0
    {
        return Err("HTML base document identity, revision and nonzero epoch required".into());
    }
    let mut decision = None;
    let resolution = resolve_html_base_snapshot(tree, context, limits, |query| {
        if binding.kind != HtmlBaseEpochKind::HrefFreeze {
            return Err("HTML base epoch kind does not match the selected href slot".into());
        }
        let captured = decide(query)?;
        decision = Some(captured);
        Ok(captured)
    })?;
    if resolution.selected.is_some() != (binding.kind == HtmlBaseEpochKind::HrefFreeze) {
        return Err("HTML base epoch kind does not match the selected href slot".into());
    }
    Ok(PreparedHtmlBaseFreeze {
        resolution: Arc::new(resolution),
        binding,
        profile,
        decision,
        url_bytes: limits.url_bytes,
    })
}

impl PreparedHtmlBaseFreeze {
    pub fn native_resolution(&self) -> &HtmlBaseResolution {
        &self.resolution
    }
    pub fn binding(&self) -> &HtmlBaseEpoch {
        &self.binding
    }

    /// Confirm an actual browser observation. No default profile, unknown-version
    /// fallback, URL normalization or arbitrary browser override is permitted.
    pub fn confirm(
        &self,
        observed: HtmlBaseObservation,
    ) -> Result<CapturedHtmlBaseEvidence, String> {
        if observed.binding != self.binding
            || observed.policy_stamp != self.resolution.context.policy_stamp
        {
            return Err("HTML base observation epoch or freeze policy changed".into());
        }
        if observed.selected_attribute != self.resolution.selected.as_ref().map(|s| s.attribute) {
            return Err("HTML base observation selected native slot changed".into());
        }
        if observed.browser.engine.trim().is_empty() || observed.browser.version.trim().is_empty() {
            return Err("HTML base observation browser identity required".into());
        }
        if observed.base_uri.len() > self.url_bytes {
            return Err("HTML base observation URL byte budget exceeded".into());
        }
        let (expected, compatibility) = self.expected_execution_url(&observed.browser)?;
        if observed.base_uri != expected {
            return Err(
                "HTML base observation disagrees with the selected compatibility profile".into(),
            );
        }
        Ok(CapturedHtmlBaseEvidence {
            resolution: self.resolution.clone(),
            observation: observed,
            profile: self.profile,
            compatibility,
            decision: self.decision,
        })
    }

    fn expected_execution_url<'a>(
        &'a self,
        browser: &HtmlBaseBrowser,
    ) -> Result<(&'a str, HtmlBaseCompatibility), String> {
        if self.profile == HtmlBaseCompatibilityProfile::Chromium148 {
            if browser.engine != "chromium" || browser.version != CHROMIUM_BASE_PROFILE_VERSION {
                return Err(
                    "unsupported browser identity for the pinned Chromium base profile".into(),
                );
            }
            // Compatibility is deliberately confined to ordinary HTTP(S)
            // documents, not every invalid URL or embedded-document context.
            let ordinary = matches!(
                &self.resolution.context.fallback,
                HtmlFallbackBase::DocumentUrl(_)
            ) && url::Url::parse(&self.resolution.fallback_url)
                .is_ok_and(|u| matches!(u.scheme(), "http" | "https"));
            if ordinary && self.resolution.outcome == HtmlBaseOutcome::InvalidHref {
                return Ok(("about:blank", HtmlBaseCompatibility::ChromiumInvalidHref));
            }
            if ordinary
                && self.resolution.outcome == HtmlBaseOutcome::Resolved
                && self
                    .resolution
                    .selected
                    .as_ref()
                    .is_some_and(|s| s.authored_href.is_empty())
                && self.resolution.fallback_url != self.resolution.effective_url
            {
                return Ok((
                    &self.resolution.fallback_url,
                    HtmlBaseCompatibility::ChromiumEmptyHref,
                ));
            }
        }
        Ok((
            &self.resolution.effective_url,
            HtmlBaseCompatibility::StandardAgreement,
        ))
    }
}

impl CapturedHtmlBaseEvidence {
    pub fn native_resolution(&self) -> &HtmlBaseResolution {
        &self.resolution
    }
    pub fn observation(&self) -> &HtmlBaseObservation {
        &self.observation
    }
    pub fn effective_url(&self) -> &str {
        &self.observation.base_uri
    }
    pub fn profile(&self) -> HtmlBaseCompatibilityProfile {
        self.profile
    }
    pub fn compatibility(&self) -> HtmlBaseCompatibility {
        self.compatibility
    }
    pub fn policy_decision(&self) -> Option<HtmlBasePolicyDecision> {
        self.decision
    }
    pub fn body_analysis_required(&self) -> bool {
        true
    }
    pub fn blockers(&self) -> &[&str] {
        &[
            "host-execution-observation-completeness-required",
            "runtime-dependency-closure-required",
            "atomic-host-transaction-required",
        ]
    }

    /// Verify an unchanged native owner and host execution read set. This does not
    /// rerun policy or substitute today's URL for the captured fallback. The host
    /// must invalidate revisions/epochs for missed events and document replacement.
    pub fn validate_current(
        &self,
        tree: &Arc<RetainedCemTree>,
        observed: &HtmlBaseObservation,
        current_policy_stamp: &str,
    ) -> Result<(), String> {
        if !Arc::ptr_eq(tree, &self.resolution.tree) {
            return Err("HTML base native snapshot owner changed".into());
        }
        if current_policy_stamp != self.resolution.context.policy_stamp {
            return Err("HTML base execution policy changed".into());
        }
        if observed != &self.observation {
            return Err("HTML base execution read set changed".into());
        }
        Ok(())
    }
}
