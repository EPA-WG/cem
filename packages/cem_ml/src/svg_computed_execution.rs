//! Current computed values for retained SVG presentation slots. These opaque CSSOM
//! strings identify neither winning declarations nor fetched resources. The caller
//! must separately validate its document/base/style execution evidence.
use crate::import::svg_presentation::RetainedSvgPresentationAttribute;
use std::{collections::BTreeSet, sync::Arc};

pub struct SvgComputedSlot<'a> {
    pub source: &'a RetainedSvgPresentationAttribute,
    pub property: &'a str,
}
pub trait SvgComputedObserver {
    /// Changes before any style, animation, environment or owner transition,
    /// including transitions that restore the same computed value.
    fn environment_revision(&mut self) -> Result<String, String>;
    /// Verify the exact live element correspondence before reading its computed
    /// property. Detached/replaced owners and unsupported states must fail.
    fn observe(&mut self, slot: SvgComputedSlot<'_>) -> Result<String, String>;
    /// Verify the exact consumer, acquired instance and native-listed paint leaves.
    /// Geometry must use the consumer's local user space; sample coverage must
    /// be exhaustive at one position in the selected instance's content space.
    /// Include layout, viewport, animation and coverage transitions in revision.
    /// DOM snapshots or fixture coordinates alone cannot establish correspondence.
    fn observe_calculation(
        &mut self,
        _slot: crate::svg_resource_acquisition::calculations::CalculationSlot<'_>,
    ) -> Result<crate::svg_resource_acquisition::calculations::CalculationObservation, String> {
        Err("verified SVG calculation observer required".into())
    }
}
#[derive(Default)]
pub struct SvgComputedHost {
    identity: Arc<()>,
    epoch: u64,
    revoked: bool,
}
pub struct SvgComputedEvidence {
    identity: Arc<()>,
    epoch: u64,
    environment: String,
    sources: Vec<RetainedSvgPresentationAttribute>,
    values: Vec<String>,
}
impl SvgComputedEvidence {
    pub fn sources(&self) -> &[RetainedSvgPresentationAttribute] {
        &self.sources
    }
    pub fn values(&self) -> &[String] {
        &self.values
    }
    /// Observation never discharges source selection, acquisition or full coverage.
    pub fn blockers(&self) -> &'static [&'static str] {
        &[
            "svg-computed-winning-source-required",
            "svg-computed-resource-acquisition-required",
            "general-runtime-resource-coverage-required",
        ]
    }
}
impl SvgComputedHost {
    pub fn invalidate(&mut self) -> Result<(), String> {
        self.revoked = true;
        self.epoch = self
            .epoch
            .checked_add(1)
            .ok_or("SVG computed epoch exhausted")?;
        Ok(())
    }
    pub fn capture(
        &mut self,
        sources: &[RetainedSvgPresentationAttribute],
        observer: &mut impl SvgComputedObserver,
    ) -> Result<SvgComputedEvidence, String> {
        self.invalidate()?;
        check_sources(sources)?;
        let environment = revision(observer)?;
        let values = observe(sources, observer)?;
        if revision(observer)? != environment
            || observe(sources, observer)? != values
            || revision(observer)? != environment
        {
            return Err("SVG computed state changed during capture".into());
        }
        self.revoked = false;
        Ok(SvgComputedEvidence {
            identity: self.identity.clone(),
            epoch: self.epoch,
            environment,
            sources: sources.to_vec(),
            values,
        })
    }
    pub fn validate(
        &mut self,
        evidence: &SvgComputedEvidence,
        sources: &[RetainedSvgPresentationAttribute],
        observer: &mut impl SvgComputedObserver,
    ) -> Result<(), String> {
        if !Arc::ptr_eq(&self.identity, &evidence.identity) {
            return Err("foreign SVG computed host".into());
        }
        if self.revoked || self.epoch != evidence.epoch {
            return Err("stale SVG computed evidence".into());
        }
        let result = (|| {
            if sources.len() != evidence.sources.len()
                || sources.iter().zip(&evidence.sources).any(|(a, b)| {
                    !Arc::ptr_eq(a.document(), b.document())
                        || !Arc::ptr_eq(a.css(), b.css())
                        || a.owner() != b.owner()
                        || a.attribute() != b.attribute()
                })
            {
                return Err("SVG computed source ownership changed".into());
            }
            if revision(observer)? != evidence.environment
                || observe(sources, observer)? != evidence.values
                || revision(observer)? != evidence.environment
            {
                return Err("SVG computed observations changed".into());
            }
            Ok(())
        })();
        if result.is_err() {
            self.invalidate()?;
        }
        result
    }
}
fn check_sources(sources: &[RetainedSvgPresentationAttribute]) -> Result<(), String> {
    if sources.len() > 256 {
        return Err("SVG computed slot budget exceeded".into());
    }
    let mut attributes = BTreeSet::new();
    for source in sources {
        if !Arc::ptr_eq(source.document(), sources[0].document())
            || !attributes.insert(source.attribute())
        {
            return Err("SVG computed slots require one document and unique attributes".into());
        }
    }
    Ok(())
}
fn revision(observer: &mut impl SvgComputedObserver) -> Result<String, String> {
    let value = observer.environment_revision()?;
    if value.trim().is_empty() || value.len() > 4096 {
        return Err("bounded SVG computed environment required".into());
    }
    Ok(value)
}
fn observe(
    sources: &[RetainedSvgPresentationAttribute],
    observer: &mut impl SvgComputedObserver,
) -> Result<Vec<String>, String> {
    sources
        .iter()
        .map(|source| {
            let property = &source
                .document()
                .node(source.attribute())
                .ok_or("missing SVG attribute")?
                .name
                .as_ref()
                .ok_or("missing SVG property")?
                .local_name;
            let value = observer.observe(SvgComputedSlot { source, property })?;
            if value.trim().is_empty() || value.len() > 4096 {
                return Err("bounded nonempty SVG computed value required".into());
            }
            Ok(value)
        })
        .collect()
}
