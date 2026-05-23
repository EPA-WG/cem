//! Performance benchmark harness (AC-N-1, AC-N-2, AC-N-3).
//!
//! Tier A perf budgets:
//!
//! - **AC-N-1** parse + validate + transform any canonical CEM-ML
//!   fixture or HTML parity fixture in under **150 ms** on a
//!   developer-class machine (single-thread, cold cache).
//! - **AC-N-2** keep tokenizer memory bounded — accumulators must
//!   scale with the current token / open-scope depth, not the
//!   document byte length.
//! - **AC-N-3** publish a Rust benchmark suite reachable from Nx.
//!
//! ## CI tolerance policy
//!
//! Wall-clock budgets vary across hosts. The harness multiplies the
//! base budget by a tolerance factor before asserting; the default
//! tolerance is `3.0` so a CI runner that is 3× slower than a
//! developer machine still passes. Callers MAY override the tolerance
//! through the `CEM_ML_PERF_TOLERANCE` environment variable. Setting
//! `CEM_ML_PERF_SKIP=1` opts the suite out entirely (useful on
//! constrained virtualised CI runners where the budget is meaningless).
//!
//! ## Budget ownership
//!
//! [`BenchmarkBudget::default_ac_n_1`] returns the AC-N-1 budget so
//! perf tests stay in lockstep with the AC document. Future ACs that
//! impose other budgets should add their own constructors here so
//! tolerance policy stays in one place.

use crate::engine::InputFormat;
use crate::real::observe_pipeline;
use std::time::{Duration, Instant};

/// AC-N-1 budget container.
#[derive(Debug, Clone, Copy)]
pub struct BenchmarkBudget {
    pub budget: Duration,
    pub tolerance: f64,
}

impl BenchmarkBudget {
    /// AC-N-1: 150 ms wall-clock budget per fixture, single-thread,
    /// cold cache. The tolerance multiplier accommodates slow CI
    /// runners; override via `CEM_ML_PERF_TOLERANCE`.
    pub fn default_ac_n_1() -> Self {
        let tolerance = std::env::var("CEM_ML_PERF_TOLERANCE")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(3.0);
        Self {
            budget: Duration::from_millis(150),
            tolerance: tolerance.max(1.0),
        }
    }

    pub fn effective_budget(&self) -> Duration {
        Duration::from_secs_f64(self.budget.as_secs_f64() * self.tolerance)
    }
}

/// Statistical summary of `iterations` runs of the pipeline.
#[derive(Debug, Clone)]
pub struct BenchmarkRun {
    pub iterations: u32,
    pub per_iter_ns: Vec<u128>,
    pub min_ns: u128,
    pub max_ns: u128,
    pub mean_ns: u128,
    pub median_ns: u128,
    pub p95_ns: u128,
    pub p99_ns: u128,
}

impl BenchmarkRun {
    pub fn within(&self, budget: &BenchmarkBudget) -> bool {
        let limit = budget.effective_budget().as_nanos();
        self.median_ns <= limit
    }
}

/// Run `iterations` measured passes of the canonical pipeline over
/// `bytes` in the supplied format. The first pass is the
/// always-included warm-up: it is recorded but the caller can ignore
/// `per_iter_ns[0]` when comparing against AC-N-1 (the AC names "cold
/// cache" so the cold pass is meaningful, but the steady-state
/// numbers come from the rest).
pub fn run_pipeline_iterations(
    bytes: &[u8],
    from_format: InputFormat,
    iterations: u32,
) -> BenchmarkRun {
    let observer = crate::observability::BufferingObserver::new();
    let iterations = iterations.max(1);
    let mut per_iter_ns: Vec<u128> = Vec::with_capacity(iterations as usize);
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = observe_pipeline(bytes, from_format, &observer);
        per_iter_ns.push(start.elapsed().as_nanos());
        observer.drain();
    }
    summarise(iterations, per_iter_ns)
}

/// Same as [`run_pipeline_iterations`] but without observer wiring
/// (lower per-iteration overhead). Suitable for budget tests that
/// need to match production behaviour.
pub fn run_pipeline_iterations_bare(
    bytes: &[u8],
    from_format: InputFormat,
    iterations: u32,
) -> BenchmarkRun {
    use crate::events::cem::CemEventNormalizer;
    use crate::parser::builder::CemAstBuilder;
    use crate::source::{BytesSource, SourceId};
    use crate::tokenizer::cem::CemTokenizer;
    use crate::tokenizer::html::HtmlTokenizer;
    use crate::tokenizer::xml::XmlTokenizer;
    use crate::schema::machine::CemSchemaMachine;
    use crate::schema::vocab::CompiledSchema;

    let iterations = iterations.max(1);
    let mut per_iter_ns: Vec<u128> = Vec::with_capacity(iterations as usize);
    for _ in 0..iterations {
        let start = Instant::now();
        match from_format {
            InputFormat::Cem => {
                let src = BytesSource::new(SourceId(1), bytes.to_vec());
                let tok = CemTokenizer::from_source(src);
                let _ = CemSchemaMachine::new(CompiledSchema::cem_core(), CemEventNormalizer::new(tok)).run();
                let src = BytesSource::new(SourceId(1), bytes.to_vec());
                let tok = CemTokenizer::from_source(src);
                let _ = CemAstBuilder::new(CemEventNormalizer::new(tok)).build();
            }
            InputFormat::Html => {
                let src = BytesSource::new(SourceId(1), bytes.to_vec());
                let tok = HtmlTokenizer::from_source(src);
                let _ = CemSchemaMachine::new(CompiledSchema::cem_core(), CemEventNormalizer::new(tok)).run();
                let src = BytesSource::new(SourceId(1), bytes.to_vec());
                let tok = HtmlTokenizer::from_source(src);
                let _ = CemAstBuilder::new(CemEventNormalizer::new(tok)).build();
            }
            InputFormat::Xml => {
                let src = BytesSource::new(SourceId(1), bytes.to_vec());
                let tok = XmlTokenizer::from_source(src);
                let _ = CemSchemaMachine::new(CompiledSchema::cem_core(), CemEventNormalizer::new(tok)).run();
                let src = BytesSource::new(SourceId(1), bytes.to_vec());
                let tok = XmlTokenizer::from_source(src);
                let _ = CemAstBuilder::new(CemEventNormalizer::new(tok)).build();
            }
        }
        per_iter_ns.push(start.elapsed().as_nanos());
    }
    summarise(iterations, per_iter_ns)
}

fn summarise(iterations: u32, mut per_iter_ns: Vec<u128>) -> BenchmarkRun {
    let mut sorted = per_iter_ns.clone();
    sorted.sort_unstable();
    let n = sorted.len();
    let min_ns = *sorted.first().unwrap_or(&0);
    let max_ns = *sorted.last().unwrap_or(&0);
    let sum: u128 = sorted.iter().sum();
    let mean_ns = if n > 0 { sum / n as u128 } else { 0 };
    let median_ns = if n == 0 { 0 } else { sorted[n / 2] };
    let p95_ns = percentile(&sorted, 95);
    let p99_ns = percentile(&sorted, 99);
    // Restore caller order for the per-iteration list.
    let _ = per_iter_ns.as_mut_slice();
    BenchmarkRun {
        iterations,
        per_iter_ns,
        min_ns,
        max_ns,
        mean_ns,
        median_ns,
        p95_ns,
        p99_ns,
    }
}

fn percentile(sorted: &[u128], p: u32) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() as f64) * (p as f64) / 100.0).ceil() as usize;
    sorted[idx.saturating_sub(1).min(sorted.len() - 1)]
}

/// Helper for tests that wants to bail out when running on a perf-
/// constrained CI runner.
pub fn perf_suite_skipped() -> bool {
    matches!(
        std::env::var("CEM_ML_PERF_SKIP").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_budget_is_150ms_with_tolerance_floor() {
        std::env::remove_var("CEM_ML_PERF_TOLERANCE");
        let b = BenchmarkBudget::default_ac_n_1();
        assert_eq!(b.budget, Duration::from_millis(150));
        assert!(b.tolerance >= 1.0);
    }

    #[test]
    fn effective_budget_scales_with_tolerance() {
        let b = BenchmarkBudget {
            budget: Duration::from_millis(100),
            tolerance: 2.5,
        };
        assert_eq!(b.effective_budget(), Duration::from_millis(250));
    }

    #[test]
    fn within_returns_true_when_median_under_effective_budget() {
        let b = BenchmarkBudget {
            budget: Duration::from_millis(10),
            tolerance: 2.0,
        };
        let run = BenchmarkRun {
            iterations: 5,
            per_iter_ns: vec![1, 2, 3, 4, 5],
            min_ns: 1,
            max_ns: 5,
            mean_ns: 3,
            median_ns: 3,
            p95_ns: 5,
            p99_ns: 5,
        };
        assert!(run.within(&b));
    }

    #[test]
    fn summarise_computes_min_mean_median_p95() {
        let r = summarise(5, vec![10, 20, 30, 40, 50]);
        assert_eq!(r.iterations, 5);
        assert_eq!(r.min_ns, 10);
        assert_eq!(r.max_ns, 50);
        assert_eq!(r.mean_ns, 30);
        assert_eq!(r.median_ns, 30);
        assert!(r.p95_ns >= 40);
    }

    #[test]
    fn run_pipeline_iterations_runs_at_least_once() {
        let r = run_pipeline_iterations_bare(b"{p | hi}", InputFormat::Cem, 1);
        assert_eq!(r.iterations, 1);
        assert!(r.per_iter_ns[0] > 0);
    }
}
