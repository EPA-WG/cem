//! Exclusive host observations over exact retained CSS occurrences. The host
//! adapter must compare real load/installation state; descriptor equality alone
//! does not prove browser behavior. No loading, CSS emission or edits occur here.
use crate::{
    css_import_snapshot::SnapshotCssImportClosure,
    css_imports::CssImportState,
    css_resources::CssResourceKind,
    parser::{tree::RetainedCemTree, AstNodeId},
};
use std::{collections::BTreeSet, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StylesheetOwner {
    Document {
        owner: AstNodeId,
        source_node: AstNodeId,
    },
    Link {
        owner: AstNodeId,
    },
    Body {
        uri: String,
        index: usize,
        scope: Option<String>,
    },
}
pub struct StylesheetExecutionRoot<'a> {
    pub owner: StylesheetOwner,
    pub closure: &'a SnapshotCssImportClosure,
    pub media: String,
    pub title: String,
    pub disabled: bool,
    pub alternate: bool,
}
pub struct StylesheetExecutionSlot<'a> {
    pub root: usize,
    pub sheet: usize,
    pub owner: &'a StylesheetOwner,
    pub tree: &'a Arc<RetainedCemTree>,
    pub source_key: &'a str,
    pub url: &'a str,
    pub parent_sheet: Option<usize>,
    pub conditions: Option<&'a CssResourceKind>,
    pub media: &'a str,
    pub title: &'a str,
    pub disabled: bool,
    pub alternate: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StylesheetObservation {
    /// Effective selection, including ancestors for imported sheets.
    pub selected: bool,
    /// Effective condition result supplied by the execution engine, not parsed here.
    pub conditions_match: bool,
    /// Import-owned source key after the adapter verifies the actual loaded source.
    pub loaded_source: Option<String>,
    /// Stable nonzero identity for this installed occurrence, even when inactive.
    pub installation: Option<u64>,
}
pub trait StylesheetExecutionObserver {
    /// Changes before any selection, CSSOM, load, viewport/media/supports or
    /// installation mutation, including same-value transitions. Must cover all roots.
    fn environment_revision(&mut self) -> Result<String, String>;
    fn observe(
        &mut self,
        slot: StylesheetExecutionSlot<'_>,
    ) -> Result<StylesheetObservation, String>;
}
#[derive(Default)]
pub struct StylesheetExecutionHost {
    identity: Arc<()>,
    epoch: u64,
    revoked: bool,
}
pub struct StylesheetExecutionEvidence {
    identity: Arc<()>,
    epoch: u64,
    environment: String,
    roots: Vec<RecordedRoot>,
    observations: Vec<StylesheetObservation>,
}
struct RecordedRoot {
    owner: StylesheetOwner,
    tree: Arc<RetainedCemTree>,
    media: String,
    title: String,
    disabled: bool,
    alternate: bool,
}
impl StylesheetExecutionEvidence {
    pub fn observations(&self) -> &[StylesheetObservation] {
        &self.observations
    }
}
impl StylesheetExecutionHost {
    pub fn invalidate(&mut self) -> Result<(), String> {
        self.revoked = true;
        self.epoch = self
            .epoch
            .checked_add(1)
            .ok_or("stylesheet execution epoch exhausted")?;
        Ok(())
    }
    pub fn capture(
        &mut self,
        roots: &[StylesheetExecutionRoot<'_>],
        observer: &mut impl StylesheetExecutionObserver,
        budget: usize,
    ) -> Result<StylesheetExecutionEvidence, String> {
        self.invalidate()?; // every new capture supersedes previous evidence
        let result = (|| {
            let environment = revision(observer)?;
            let observations = observe(roots, observer, budget)?;
            if revision(observer)? != environment
                || observe(roots, observer, budget)? != observations
                || revision(observer)? != environment
            {
                return Err("stylesheet environment or observations changed during capture".into());
            }
            let owners = roots
                .iter()
                .map(|r| RecordedRoot {
                    owner: r.owner.clone(),
                    tree: r.closure.closure().sheets()[0].resources.tree.clone(),
                    media: r.media.clone(),
                    title: r.title.clone(),
                    disabled: r.disabled,
                    alternate: r.alternate,
                })
                .collect();
            Ok(StylesheetExecutionEvidence {
                identity: self.identity.clone(),
                epoch: self.epoch,
                environment,
                roots: owners,
                observations,
            })
        })();
        if result.is_ok() {
            self.revoked = false;
        }
        result
    }
    pub fn validate(
        &mut self,
        evidence: &StylesheetExecutionEvidence,
        roots: &[StylesheetExecutionRoot<'_>],
        observer: &mut impl StylesheetExecutionObserver,
    ) -> Result<(), String> {
        if !Arc::ptr_eq(&evidence.identity, &self.identity) {
            return Err("foreign stylesheet execution host".into());
        }
        if self.revoked || self.epoch != evidence.epoch {
            return Err("stale stylesheet execution epoch".into());
        }
        let result = (|| {
            if roots.len() != evidence.roots.len()
                || roots.iter().zip(&evidence.roots).any(|(r, recorded)| {
                    r.owner != recorded.owner
                        || r.media != recorded.media
                        || r.title != recorded.title
                        || r.disabled != recorded.disabled
                        || r.alternate != recorded.alternate
                        || !Arc::ptr_eq(
                            &recorded.tree,
                            &r.closure.closure().sheets()[0].resources.tree,
                        )
                })
            {
                return Err("stylesheet execution roots changed".into());
            }
            if revision(observer)? != evidence.environment
                || observe(roots, observer, evidence.observations.len())? != evidence.observations
                || revision(observer)? != evidence.environment
            {
                return Err("stylesheet execution observations changed".into());
            }
            Ok(())
        })();
        if result.is_err() {
            self.invalidate()?;
        }
        result
    }
}
fn revision(observer: &mut impl StylesheetExecutionObserver) -> Result<String, String> {
    let value = observer.environment_revision()?;
    if value.trim().is_empty() || value.len() > 4096 {
        return Err("bounded stylesheet environment revision required".into());
    }
    Ok(value)
}
fn observe(
    roots: &[StylesheetExecutionRoot<'_>],
    observer: &mut impl StylesheetExecutionObserver,
    budget: usize,
) -> Result<Vec<StylesheetObservation>, String> {
    let count = roots
        .iter()
        .try_fold(0usize, |n, r| {
            n.checked_add(r.closure.closure().sheets().len())
        })
        .ok_or("stylesheet occurrence count overflow")?;
    if count > budget {
        return Err("stylesheet observation budget exceeded".into());
    }
    let mut result = Vec::new();
    let mut identities = BTreeSet::new();
    for (root_index, root) in roots.iter().enumerate() {
        let closure = root.closure.closure();
        let offset = result.len();
        if closure.state() != CssImportState::Ready {
            return Err("stylesheet closure is stale".into());
        }
        for (sheet_index, sheet) in closure.sheets().iter().enumerate() {
            let source_key = sheet
                .resources
                .tree
                .source_key(0)
                .ok_or("stylesheet source fingerprint required")?;
            let edge = closure
                .edges()
                .iter()
                .find(|e| e.child_sheet == sheet_index);
            let observation = observer.observe(StylesheetExecutionSlot {
                root: root_index,
                sheet: sheet_index,
                owner: &root.owner,
                tree: &sheet.resources.tree,
                source_key: &source_key,
                url: &sheet.url,
                parent_sheet: edge.map(|e| e.parent_sheet),
                conditions: edge.map(|e| &e.conditions),
                media: &root.media,
                title: &root.title,
                disabled: root.disabled,
                alternate: root.alternate,
            })?;
            if observation
                .loaded_source
                .as_ref()
                .is_some_and(|key| key != &source_key)
                || (observation.installation.is_some() && observation.loaded_source.is_none())
            {
                return Err("stylesheet load/source correspondence failed".into());
            }
            if let Some(id) = observation.installation {
                if id == 0 || !identities.insert(id) {
                    return Err(
                        "stylesheet installation identity must be unique and nonzero".into(),
                    );
                }
            }
            if root.disabled && observation.selected {
                return Err("disabled stylesheet cannot be selected".into());
            }
            if let Some(edge) = edge {
                let parent: &StylesheetObservation = &result[offset + edge.parent_sheet];
                if observation.selected && !(parent.selected && parent.conditions_match) {
                    return Err("import selection contradicts parent conditions".into());
                }
            }
            if observation.selected
                && observation.conditions_match
                && (observation.loaded_source.is_none() || observation.installation.is_none())
            {
                return Err("active stylesheet lacks load/installation evidence".into());
            }
            result.push(observation);
        }
        if closure.state() != CssImportState::Ready {
            return Err("stylesheet policy changed during observation".into());
        }
    }
    // A later observation can mutate an earlier root's policy.
    if roots
        .iter()
        .any(|r| r.closure.closure().state() != CssImportState::Ready)
    {
        return Err("stylesheet policy changed during observation".into());
    }
    Ok(result)
}
/// Only a complete execution observation discharges these history requirements.
/// Runtime resource reads and source-edit admission remain separate.
pub fn remaining_blockers(blockers: &[String]) -> Vec<String> {
    blockers
        .iter()
        .filter(|b| {
            !matches!(
                b.as_str(),
                "stylesheet-execution-history-required"
                    | "css-conditions-and-execution-history-required"
            )
        })
        .cloned()
        .collect()
}
