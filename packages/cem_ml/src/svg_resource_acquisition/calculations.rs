//! Consumer-bound preparation observations, not engine paint/consumption receipts.
//! The observer verifies live correspondence; native code owns the source join,
//! repeated observations, calculation validation and revocation.
use super::*;
use geometry::{GeometryContext, GradientEvaluation, ViewBoxMapping};

pub struct CalculationSlot<'a> {
    pub consumer: &'a crate::import::svg_presentation::RetainedSvgPresentationAttribute,
    pub binding: &'a SvgResourceBinding,
    pub plan: &'a paint::AcquiredPaintContent,
    /// Native-owned order. The adapter returns one coverage bit per source;
    /// it cannot substitute source coordinates or omit uncovered leaves.
    pub paint_sources: &'a [AstNodeId],
}
#[derive(Clone, Debug, PartialEq)]
/// Mapping for the consumer's nearest SVG viewport. The context viewport
/// dimensions are the viewBox dimensions in that viewport's user space.
pub struct ViewBoxObservation {
    pub view_box: [f64; 4],
    pub viewport: [f64; 4],
    pub aspect: String,
    pub observed_matrix: [f64; 6],
}
#[derive(Clone, Debug, PartialEq)]
pub struct SampleObservation {
    /// In the selected pattern content's user space, after instance mapping.
    pub point: [f64; 2],
    pub covered: Vec<bool>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CalculationObservation {
    pub context: GeometryContext,
    pub view_box: Option<ViewBoxObservation>,
    pub sample: Option<SampleObservation>,
}
pub(super) struct Calculations {
    observations: Vec<CalculationObservation>,
    pub(super) cancel: AbortSignal,
}
impl SvgResourceRecord {
    /// Opt-in, atomic capture for every acquired paint binding. Observations
    /// must include layout/coverage changes in the computed environment revision,
    /// even when the same values are later restored. No browser default adapter
    /// is installed: an adapter without verified correspondence must reject.
    pub fn prepare_calculations(
        &mut self,
        host: &mut SvgComputedHost,
        observer: &mut impl SvgComputedObserver,
        cancel: AbortSignal,
    ) -> Result<(), String> {
        let result = (|| {
            self.validate(host, observer)?;
            if self.calculations.is_some() {
                return Err("fresh calculation capture required".into());
            }
            active(&cancel)?;
            let observations = self.observe_calculations(observer)?;
            self.validate(host, observer)?;
            if observations != self.observe_calculations(observer)? {
                return Err("SVG calculation observations changed during capture".into());
            }
            self.validate(host, observer)?;
            active(&cancel)?;
            self.calculations = Some(Calculations {
                observations,
                cancel,
            });
            Ok(())
        })();
        if result.is_err() {
            self.close();
        }
        result
    }
    pub(super) fn validate_calculations(
        &self,
        observer: &mut impl SvgComputedObserver,
    ) -> Result<(), String> {
        if let Some(capture) = &self.calculations {
            active(&capture.cancel)?;
            if self.observe_calculations(observer)? != capture.observations {
                return Err("SVG calculation observations changed".into());
            }
            active(&capture.cancel)?;
        }
        Ok(())
    }
    fn observe_calculations(
        &self,
        observer: &mut impl SvgComputedObserver,
    ) -> Result<Vec<CalculationObservation>, String> {
        let plans = self
            .content
            .as_ref()
            .ok_or("prepared paint content required")?;
        if plans.len() != self.bindings.len() || plans.is_empty() {
            return Err("complete calculation bindings required".into());
        }
        let mut observations = Vec::new();
        let mut work = 8192usize;
        for (binding, plan) in self.bindings.iter().zip(plans) {
            active(&self.cancel)?;
            let pattern = crate::svg_paint_dependencies::kind(binding.body.tree(), plan.host)
                == Some("pattern");
            let sources: Vec<_> = if pattern {
                plan.colors
                    .as_ref()
                    .map_err(Clone::clone)?
                    .iter()
                    .map(|p| p.source)
                    .collect()
            } else {
                Vec::new()
            };
            work = work
                .checked_sub(sources.len() + 1)
                .ok_or("calculation observation budget exceeded")?;
            let observation = observer.observe_calculation(CalculationSlot {
                consumer: self
                    .computed
                    .sources()
                    .get(binding.slot)
                    .ok_or("calculation consumer missing")?,
                binding,
                plan,
                paint_sources: &sources,
            })?;
            check_context(&observation)?;
            if pattern {
                let sample = observation
                    .sample
                    .as_ref()
                    .ok_or("pattern coverage observation required")?;
                if sample.covered.len() != sources.len()
                    || !sample.point.iter().all(|n| n.is_finite())
                {
                    return Err("complete finite sample coverage required".into());
                }
                let coverage: Vec<_> = sources
                    .iter()
                    .zip(&sample.covered)
                    .filter_map(|(id, hit)| hit.then_some(*id))
                    .collect();
                plan.composite_pattern_sample(&coverage)?;
            } else {
                if observation.sample.is_some() {
                    return Err("unexpected gradient sample coverage".into());
                }
                plan.resolve_gradient(observation.context)?;
            }
            observations.push(observation);
        }
        Ok(observations)
    }
    /// Inspection under the record lifetime, not a transferable execution seal.
    /// Revalidate the record and enclosing preparation immediately before use.
    pub fn gradient_calculation(
        &self,
        index: usize,
    ) -> Option<(&SvgResourceBinding, GradientEvaluation<'_>)> {
        let (binding, plan) = self.paint_content(index)?;
        let observation = self.calculations.as_ref()?.observations.get(index)?;
        Some((binding, plan.resolve_gradient(observation.context).ok()?))
    }
    /// The sample position and viewBox observations remain available with the
    /// exact binding; coverage cannot be supplied by the result consumer.
    pub fn calculation_observation(
        &self,
        index: usize,
    ) -> Option<(&SvgResourceBinding, &CalculationObservation)> {
        let (binding, _) = self.paint_content(index)?;
        Some((
            binding,
            self.calculations.as_ref()?.observations.get(index)?,
        ))
    }
    pub fn pattern_calculation(
        &self,
        index: usize,
    ) -> Option<(&SvgResourceBinding, composite::PatternSample<'_>)> {
        let (binding, plan) = self.paint_content(index)?;
        let sample = self
            .calculations
            .as_ref()?
            .observations
            .get(index)?
            .sample
            .as_ref()?;
        let coverage: Vec<_> = plan
            .colors
            .as_ref()
            .ok()?
            .iter()
            .zip(&sample.covered)
            .filter_map(|(p, hit)| hit.then_some(p.source))
            .collect();
        Some((binding, plan.composite_pattern_sample(&coverage).ok()?))
    }
}
fn check_context(observation: &CalculationObservation) -> Result<(), String> {
    let c = observation.context;
    if !c
        .viewport
        .iter()
        .chain(c.bounding_box.iter())
        .all(|n| n.is_finite())
        || c.viewport.iter().any(|n| *n < 0.)
        || c.bounding_box[2..].iter().any(|n| *n < 0.)
    {
        return Err("invalid observed geometry context".into());
    }
    if let Some(v) = &observation.view_box {
        if c.viewport != [v.view_box[2], v.view_box[3]] {
            return Err("viewBox and geometry viewport disagree".into());
        }
        if v.aspect.len() > 64 {
            return Err("viewBox aspect bound exceeded".into());
        }
        let expected = ViewBoxMapping::resolve(v.view_box, v.viewport, &v.aspect)?;
        if expected.disabled
            || v.observed_matrix
                .iter()
                .zip(expected.matrix.coefficients())
                .any(|(actual, expected)| {
                    !actual.is_finite() || (actual - expected).abs() > 1e-5 * expected.abs().max(1.)
                })
        {
            return Err("observed viewBox matrix mismatch or disabled viewport".into());
        }
    }
    Ok(())
}
