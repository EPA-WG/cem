//! Selector benchmark suite.
//!
//! Implements verification item §13.4 from `docs/cem-ql-ac.md`:
//! "selector benchmarks shared with the host `cem_ml_cli:bench`
//! budget per AC-QR-5." Reuses [`BenchmarkBudget::default_ac_n_1`] so
//! the same 150 ms Tier A wall-clock budget — and the same
//! `CEM_ML_PERF_TOLERANCE` / `CEM_ML_PERF_SKIP` knobs — apply to
//! cem-ql selector evaluations.
//!
//! Each benchmark compiles a Tier A selector once and replays it
//! across the canonical fixtures, measuring per-iteration wall-clock
//! time. Selector + transform end-to-end stays under the host's 150 ms
//! Tier A budget per AC-QR-5.

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use cem_ml::benchmark::{perf_suite_skipped, BenchmarkBudget, BenchmarkRun};
use cem_ml::scheduler::ScopePolicy;
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{ItemStream, QueryContextScope};

const FIXTURE_NAMES: &[&str] = &[
    "assets-list",
    "login",
    "message-thread",
    "profile",
    "registration",
];

const ITERATIONS: u32 = 12;

fn workspace_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path
}

fn load_fixture(dir: &str, name: &str, ext: &str) -> String {
    let path = workspace_root()
        .join("examples")
        .join(dir)
        .join(format!("{name}.{ext}"));
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn evaluate_once(source: &str) -> ItemStream {
    let compiled = compile(source, &CompileContext::default())
        .unwrap_or_else(|err| panic!("compile failed: {err}"));
    evaluate(
        &compiled,
        &EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: ScopePolicy::host_root().with_queue_size(4096),
            diagnostics: Vec::new(),
            policy_bindings: Default::default(),
        },
    )
}

fn summarise(per_iter_ns: Vec<u128>) -> BenchmarkRun {
    let iterations = per_iter_ns.len() as u32;
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

fn run_selector(selector_template: impl Fn(&str) -> String, iterations: u32) -> BenchmarkRun {
    let inputs: Vec<(String, String)> = FIXTURE_NAMES
        .iter()
        .map(|name| (name.to_string(), load_fixture("cem-ml", name, "cem")))
        .collect();

    let mut per_iter_ns = Vec::with_capacity(iterations as usize);
    for _ in 0..iterations {
        let start = Instant::now();
        for (_, fixture) in &inputs {
            let query = selector_template(fixture);
            let stream = evaluate_once(&query);
            std::hint::black_box(stream.items.len());
        }
        per_iter_ns.push(start.elapsed().as_nanos());
    }
    summarise(per_iter_ns)
}

#[test]
fn selector_benchmark_target_uses_shared_budget_contract() {
    let budget = BenchmarkBudget::default_ac_n_1();
    assert!(budget.effective_budget() >= budget.budget);
}

#[test]
fn cemml_parse_selector_runs_under_ac_n_1_budget_across_fixtures() {
    if perf_suite_skipped() {
        eprintln!("CEM_ML_PERF_SKIP set; skipping selector benchmark");
        return;
    }
    let budget = BenchmarkBudget::default_ac_n_1();
    let run = run_selector(
        |fixture| {
            let escaped = escape_query_string(fixture);
            format!(r#"cemml:parse("{escaped}")"#)
        },
        ITERATIONS,
    );

    eprintln!(
        "cemml:parse selector: iterations={} median={}ms mean={}ms p95={}ms",
        run.iterations,
        run.median_ns / 1_000_000,
        run.mean_ns / 1_000_000,
        run.p95_ns / 1_000_000,
    );
    assert!(
        run.within(&budget),
        "median {}ns exceeded AC-N-1 effective budget {}ns; raise CEM_ML_PERF_TOLERANCE or fix the regression",
        run.median_ns,
        budget.effective_budget().as_nanos(),
    );
}

#[test]
fn string_pipeline_selector_runs_under_ac_n_1_budget_across_fixtures() {
    if perf_suite_skipped() {
        eprintln!("CEM_ML_PERF_SKIP set; skipping selector benchmark");
        return;
    }
    let budget = BenchmarkBudget::default_ac_n_1();
    let run = run_selector(
        |fixture| {
            let escaped = escape_query_string(fixture);
            format!(r#"str:length(str:lower(str:slice("{escaped}", 0, 64)))"#)
        },
        ITERATIONS,
    );

    eprintln!(
        "string-pipeline selector: iterations={} median={}ms mean={}ms p95={}ms",
        run.iterations,
        run.median_ns / 1_000_000,
        run.mean_ns / 1_000_000,
        run.p95_ns / 1_000_000,
    );
    assert!(
        run.within(&budget),
        "median {}ns exceeded AC-N-1 effective budget {}ns",
        run.median_ns,
        budget.effective_budget().as_nanos(),
    );
}

fn escape_query_string(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 2);
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str(r"\\"),
            '"' => out.push_str(r#"\""#),
            '\n' => out.push_str(r"\n"),
            '\r' => out.push_str(r"\r"),
            '\t' => out.push_str(r"\t"),
            _ => out.push(ch),
        }
    }
    out
}
