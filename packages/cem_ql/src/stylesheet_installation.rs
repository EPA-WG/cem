//! Sealed correspondence between native compiled closures and installed CSSOM.
//! Flattened imports share one real installation; inner conditional groups keep
//! their runtime blocker instead of inventing separate browser stylesheet IDs.
use crate::retained_template::{RetainedStylesheet, StylesheetConsumerLease};
use cem_ml::css_emission::CssStylesheetIdentity;
use std::{collections::BTreeSet, sync::Arc};
pub struct CompiledStylesheetSet {
    identity: Arc<()>,
    sheets: Vec<Arc<RetainedStylesheet>>,
    leases: Vec<StylesheetConsumerLease>,
}
impl CompiledStylesheetSet {
    pub(crate) fn new(
        sheets: Vec<Arc<RetainedStylesheet>>,
        leases: Vec<StylesheetConsumerLease>,
    ) -> Self {
        Self {
            identity: Arc::new(()),
            sheets,
            leases,
        }
    }
    pub(crate) fn join(sets: Vec<Self>) -> Result<Self, String> {
        let mut sheets = Vec::new();
        let mut leases = Vec::new();
        for set in sets {
            sheets.extend(set.sheets);
            leases.extend(set.leases);
        }
        if sheets.len() > 256 {
            return Err("compiled stylesheet occurrence bound exceeded".into());
        }
        Ok(Self::new(sheets, leases))
    }
    pub fn len(&self) -> usize {
        self.sheets.len()
    }
    pub fn is_empty(&self) -> bool {
        self.sheets.is_empty()
    }
    pub fn validate_current(&self) -> Result<(), String> {
        if self.leases.iter().any(|lease| !lease.active()) {
            return Err("compiled stylesheet consumer or source lifetime changed".into());
        }
        for sheet in &self.sheets {
            if !sheet.emission()?.diagnostics.is_empty() {
                return Err("incomplete compiled CSS emission cannot prove installation".into());
            }
        }
        Ok(())
    }
    /// Native emitted text for installation/export. Its string value is not proof.
    pub fn css(&self, index: usize) -> Result<String, String> {
        self.validate_current()?;
        Ok(self
            .sheets
            .get(index)
            .ok_or("unknown compiled stylesheet occurrence")?
            .emission()?
            .css())
    }
    pub(crate) fn emission(
        &self,
        index: usize,
    ) -> Result<&cem_ml::css_emission::CssClosureEmission, String> {
        self.validate_current()?;
        self.sheets
            .get(index)
            .ok_or("unknown compiled stylesheet occurrence")?
            .emission()
            .map_err(str::to_string)
    }
    pub fn identity(&self, index: usize) -> Result<&CssStylesheetIdentity, String> {
        self.validate_current()?;
        Ok(self
            .sheets
            .get(index)
            .ok_or("unknown compiled stylesheet occurrence")?
            .identity())
    }
}
pub struct CompiledStylesheetSlot<'a> {
    pub index: usize,
    pub css: &'a str,
    pub identity: &'a CssStylesheetIdentity,
}
#[derive(Clone, PartialEq, Eq)]
pub struct CompiledStylesheetObservation {
    pub installation: u32,
    pub selected: bool,
    pub source_matches: bool,
}
pub trait CompiledStylesheetObserver {
    fn environment_revision(&mut self) -> Result<String, String>;
    fn observe(
        &mut self,
        slot: CompiledStylesheetSlot<'_>,
    ) -> Result<CompiledStylesheetObservation, String>;
}
#[derive(Default)]
pub struct CompiledStylesheetHost {
    identity: Arc<()>,
    epoch: u64,
    revoked: bool,
}
pub struct CompiledStylesheetEvidence {
    host: Arc<()>,
    owner: Arc<()>,
    epoch: u64,
    environment: String,
    observations: Vec<CompiledStylesheetObservation>,
    max_bytes: usize,
}
impl CompiledStylesheetEvidence {
    pub fn observations(&self) -> &[CompiledStylesheetObservation] {
        &self.observations
    }
    pub fn blockers(&self) -> &[&str] {
        &[
            "compiled-stylesheet-conditions-runtime-required",
            "css-runtime-dependency-closure-required",
            "atomic-host-transaction-required",
        ]
    }
}
impl CompiledStylesheetHost {
    pub fn invalidate(&mut self) -> Result<(), String> {
        self.revoked = true;
        self.epoch = self
            .epoch
            .checked_add(1)
            .ok_or("compiled stylesheet epoch exhausted")?;
        Ok(())
    }
    pub fn capture(
        &mut self,
        sheets: &CompiledStylesheetSet,
        observer: &mut impl CompiledStylesheetObserver,
        max_sheets: usize,
        max_bytes: usize,
    ) -> Result<CompiledStylesheetEvidence, String> {
        self.invalidate()?;
        let environment = revision(observer)?;
        let observations = observe(sheets, observer, max_sheets, max_bytes)?;
        if revision(observer)? != environment
            || observe(sheets, observer, max_sheets, max_bytes)? != observations
            || revision(observer)? != environment
        {
            return Err("compiled stylesheet observation changed during capture".into());
        }
        sheets.validate_current()?;
        self.revoked = false;
        Ok(CompiledStylesheetEvidence {
            host: self.identity.clone(),
            owner: sheets.identity.clone(),
            epoch: self.epoch,
            environment,
            observations,
            max_bytes,
        })
    }
    pub fn validate(
        &mut self,
        evidence: &CompiledStylesheetEvidence,
        sheets: &CompiledStylesheetSet,
        observer: &mut impl CompiledStylesheetObserver,
    ) -> Result<(), String> {
        if !Arc::ptr_eq(&evidence.host, &self.identity)
            || !Arc::ptr_eq(&evidence.owner, &sheets.identity)
        {
            return Err("foreign compiled stylesheet host or source set".into());
        }
        if self.revoked || evidence.epoch != self.epoch {
            return Err("stale compiled stylesheet evidence".into());
        }
        let result = (|| {
            if revision(observer)? != evidence.environment
                || observe(
                    sheets,
                    observer,
                    evidence.observations.len(),
                    evidence.max_bytes,
                )? != evidence.observations
                || revision(observer)? != evidence.environment
            {
                return Err("compiled stylesheet observations changed".into());
            }
            sheets.validate_current()
        })();
        if result.is_err() {
            self.invalidate()?;
        }
        result
    }
}
fn revision(observer: &mut impl CompiledStylesheetObserver) -> Result<String, String> {
    let r = observer.environment_revision()?;
    if r.trim().is_empty() || r.len() > 4096 {
        return Err("bounded compiled stylesheet revision required".into());
    }
    Ok(r)
}
fn observe(
    set: &CompiledStylesheetSet,
    observer: &mut impl CompiledStylesheetObserver,
    max_sheets: usize,
    max_bytes: usize,
) -> Result<Vec<CompiledStylesheetObservation>, String> {
    set.validate_current()?;
    if set.len() > max_sheets {
        return Err("compiled stylesheet occurrence bound exceeded".into());
    }
    // Check the aggregate emitted fragments before allocating comparison strings.
    let mut bytes = 0usize;
    for sheet in &set.sheets {
        for f in &sheet.emission()?.fragments {
            bytes = bytes
                .checked_add(f.fragment.text.len())
                .ok_or("compiled stylesheet bytes overflow")?;
            if bytes > max_bytes {
                return Err("compiled stylesheet byte bound exceeded".into());
            }
        }
    }
    let mut seen = BTreeSet::new();
    let mut observations = Vec::new();
    for (index, sheet) in set.sheets.iter().enumerate() {
        let css = sheet.emission()?.css();
        let observation = observer.observe(CompiledStylesheetSlot {
            index,
            css: &css,
            identity: sheet.identity(),
        })?;
        if !observation.source_matches
            || observation.installation == 0
            || !seen.insert(observation.installation)
        {
            return Err(
                "compiled source correspondence or unique installation identity missing".into(),
            );
        }
        observations.push(observation);
        set.validate_current()?;
    }
    Ok(observations)
}
