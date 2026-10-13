//! Exclusive execution-host sessions. DOM predicates compare live slots against
//! native semantics; they do not import, serialize or reconstruct a document AST.
use crate::{
    html_base::*,
    html_base_evidence::*,
    parser::{
        tree::{CemTreeNode, RetainedCemTree},
        AstNodeId,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

static NEXT_HOST: AtomicU64 = AtomicU64::new(1);

/// A read-only predicate request from the retained native owner. The host checks
/// kind, expanded name, value, parent, ordinal and both child/attribute counts,
/// then returns a stable, nonzero DOM identity for that exact slot.
pub struct HtmlBaseDomSlot<'a> {
    pub id: AstNodeId,
    pub node: &'a CemTreeNode,
    pub index: usize,
}

pub struct HtmlBaseExecutionHost {
    identity: String,
    revision: u64,
    epoch: u64,
    profile: HtmlBaseCompatibilityProfile,
    browser: HtmlBaseBrowser,
    limits: HtmlBaseLimits,
    revoked: bool,
    slots: BTreeMap<AstNodeId, u32>,
    pending: Option<PreparedHtmlBaseFreeze>,
    current: Option<Arc<CapturedHtmlBaseEvidence>>,
}

impl HtmlBaseExecutionHost {
    pub fn new(
        profile: HtmlBaseCompatibilityProfile,
        browser: HtmlBaseBrowser,
        limits: HtmlBaseLimits,
    ) -> Result<Self, String> {
        if browser.engine.trim().is_empty()
            || browser.version.trim().is_empty()
            || limits.nodes == 0
            || limits.url_bytes == 0
        {
            return Err("HTML base host requires browser identity and positive bounds".into());
        }
        let identity = NEXT_HOST
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| "HTML base host identity exhausted")?;
        Ok(Self {
            identity: format!("html-base-host:{identity}"),
            revision: 0,
            epoch: 0,
            profile,
            browser,
            limits,
            revoked: false,
            slots: BTreeMap::new(),
            pending: None,
            current: None,
        })
    }

    /// Call before every controlled context/document change. Old evidence never
    /// becomes current again, including URL, policy and DOM A-B-A transitions.
    pub fn invalidate(&mut self) -> Result<(), String> {
        self.live()?;
        self.current = None;
        self.pending = None;
        self.slots.clear();
        self.revision = self.revision.checked_add(1).ok_or_else(|| {
            self.revoked = true;
            "HTML base host revision exhausted".to_string()
        })?;
        Ok(())
    }

    pub fn revoke(&mut self) {
        self.revoked = true;
        self.current = None;
        self.pending = None;
        self.slots.clear();
    }

    /// Validate the complete document read set before policy or browser mutation.
    /// The host must immediately set the returned href Attr to its current value
    /// (a real freeze), then confirm the browser URL. None captures a no-base
    /// fallback snapshot. This API never guesses a preexisting frozen history.
    pub fn prepare(
        &mut self,
        tree: Arc<RetainedCemTree>,
        context: HtmlBaseContext,
        matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        decide: impl FnOnce(&HtmlBasePolicyRequest<'_>) -> Result<HtmlBasePolicyDecision, String>,
    ) -> Result<Option<u32>, String> {
        self.live()?;
        if self.pending.is_some() {
            self.revoke();
            return Err("HTML base host preparation already pending".into());
        }
        self.invalidate()?;
        let result = (|| {
            let slots = compare_slots(&tree, self.limits.nodes, matches)?;
            self.epoch = self
                .epoch
                .checked_add(1)
                .ok_or("HTML base host epoch exhausted")?;
            // Selection is native; the host does not independently parse names
            // or choose a base. This scan cannot invoke execution policy.
            let selection =
                resolve_html_base_snapshot(tree.clone(), context.clone(), self.limits, |_| {
                    Ok(HtmlBasePolicyDecision::Allow)
                })?;
            let prepared = prepare_html_base_freeze(
                HtmlBaseFreezeRequest {
                    tree,
                    context,
                    profile: self.profile,
                    limits: self.limits,
                    binding: HtmlBaseEpoch {
                        document_id: self.identity.clone(),
                        revision: self.revision.to_string(),
                        epoch: self.epoch,
                        kind: if selection.selected.is_some() {
                            HtmlBaseEpochKind::HrefFreeze
                        } else {
                            HtmlBaseEpochKind::FallbackSnapshot
                        },
                    },
                },
                decide,
            )?;
            let selected = prepared
                .native_resolution()
                .selected
                .as_ref()
                .map(|s| slots[&s.attribute]);
            self.slots = slots;
            self.pending = Some(prepared);
            Ok(selected)
        })();
        if result.is_err() {
            self.revoke();
        }
        result
    }

    pub fn confirm(&mut self, base_uri: String) -> Result<Arc<CapturedHtmlBaseEvidence>, String> {
        self.live()?;
        let result = (|| {
            let prepared = self
                .pending
                .take()
                .ok_or("HTML base host has no pending freeze")?;
            let observed = HtmlBaseObservation {
                binding: prepared.binding().clone(),
                browser: self.browser.clone(),
                selected_attribute: prepared
                    .native_resolution()
                    .selected
                    .as_ref()
                    .map(|s| s.attribute),
                policy_stamp: prepared.native_resolution().context.policy_stamp.clone(),
                base_uri,
            };
            let evidence = Arc::new(prepared.confirm(observed)?);
            self.current = Some(evidence.clone());
            Ok(evidence)
        })();
        if result.is_err() {
            self.revoke();
        }
        result
    }

    pub fn current(&self) -> Option<&Arc<CapturedHtmlBaseEvidence>> {
        self.current.as_ref()
    }

    /// Recheck live identities and all native predicates, including body slots.
    /// Host event observation and exclusive context ownership are also required.
    pub fn validate(
        &mut self,
        evidence: &Arc<CapturedHtmlBaseEvidence>,
        matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base_uri: &str,
    ) -> Result<(), String> {
        self.live()?;
        if !self
            .current
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, evidence))
        {
            return Err("HTML base host evidence is stale or foreign".into());
        }
        let result = (|| {
            if base_uri != evidence.effective_url() {
                return Err("HTML base host observed URL changed".into());
            }
            let slots = compare_slots(
                &evidence.native_resolution().tree,
                self.limits.nodes,
                matches,
            )?;
            if slots != self.slots {
                return Err("HTML base host DOM identity changed".into());
            }
            Ok(())
        })();
        if result.is_err() {
            self.revoke();
        }
        result
    }

    fn live(&self) -> Result<(), String> {
        if self.revoked {
            Err("HTML base host session revoked".into())
        } else {
            Ok(())
        }
    }
}

fn compare_slots(
    tree: &RetainedCemTree,
    mut remaining: usize,
    mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
) -> Result<BTreeMap<AstNodeId, u32>, String> {
    let mut pending = vec![(0, 0)];
    let mut slots = BTreeMap::new();
    let mut identities = BTreeSet::new();
    while let Some((id, index)) = pending.pop() {
        remaining = remaining
            .checked_sub(1)
            .ok_or("HTML base host topology budget exceeded")?;
        let node = tree.node(id).ok_or("HTML base host native slot missing")?;
        let handle = matches(HtmlBaseDomSlot { id, node, index })?;
        if handle == 0 || !identities.insert(handle) || slots.insert(id, handle).is_some() {
            return Err("HTML base host requires unique nonzero DOM slot identities".into());
        }
        let count = node
            .children
            .len()
            .checked_add(node.attributes.len())
            .ok_or("HTML base host topology budget exceeded")?;
        if count > remaining.saturating_sub(pending.len()) {
            return Err("HTML base host topology budget exceeded".into());
        }
        pending.extend(
            node.children
                .iter()
                .enumerate()
                .rev()
                .map(|(i, id)| (*id, i)),
        );
        pending.extend(
            node.attributes
                .iter()
                .enumerate()
                .rev()
                .map(|(i, id)| (*id, i)),
        );
    }
    Ok(slots)
}
