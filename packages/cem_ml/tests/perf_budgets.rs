//! Performance budgets and memory-bounding proofs (AC-N-1 / AC-N-2).
//!
//! - AC-N-1 (Tier A): parse + validate + transform any canonical fixture
//!   in `examples/cem-ml/` and any HTML parity fixture in
//!   `examples/semantic/` under **150 ms**, single-thread, cold cache.
//!   The `<cem-element>` material parity substrate fixtures in
//!   `examples/cem-elements/material-*.cem` ride the same budget
//!   discipline for the Phase 3.1 production-ready gate.
//!   CI tolerance is owned by [`cem_ml::benchmark::BenchmarkBudget`].
//! - AC-N-2 (Tier A): tokenizer accumulators must scale with current
//!   token / open-scope depth, not with document byte length. The proof
//!   is a 10 MB synthetic fixture that parses inside the same per-byte
//!   envelope as the small fixtures.
//! - AC-N-3 (Tier B): the suite is reachable from Nx via
//!   `nx run cem_ml:bench`, which lifts this file through the standard
//!   test runner under `--release`.
//!
//! Set `CEM_ML_PERF_SKIP=1` on constrained virtualised CI runners where
//! the wall-clock budget is meaningless. Debug builds are auto-skipped:
//! AC-N-1 names release wall-clock budgets and debug builds run 5-10×
//! slower without representing the shipped artifact. Run with
//! `cargo test --release --test perf_budgets` or
//! `nx run cem_ml:bench`.

use cem_ml::benchmark::{perf_suite_skipped, run_pipeline_iterations_bare, BenchmarkBudget};
use cem_ml::engine::InputFormat;

const ITERATIONS: u32 = 8;

fn perf_skipped_for_build() -> bool {
    perf_suite_skipped() || cfg!(debug_assertions)
}

fn read(path: &str) -> Vec<u8> {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    std::fs::read(&p).unwrap_or_else(|e| panic!("read {p:?}: {e}"))
}

fn list_fixtures(rel: &str, ext: &str) -> Vec<std::path::PathBuf> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read_dir {dir:?}: {e}"))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some(ext))
        .collect();
    paths.sort();
    paths
}

fn list_fixtures_with_prefix(rel: &str, ext: &str, prefix: &str) -> Vec<std::path::PathBuf> {
    list_fixtures(rel, ext)
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(prefix))
        })
        .collect()
}

/// AC-N-1: every canonical CEM-ML fixture parses + validates under the
/// effective 150 ms budget (tolerance from `CEM_ML_PERF_TOLERANCE`).
#[test]
fn ac_n_1_cem_fixtures_under_budget() {
    if perf_skipped_for_build() {
        return;
    }
    let budget = BenchmarkBudget::default_ac_n_1();
    let paths = list_fixtures("../../examples/cem-ml", "cem");
    assert!(paths.len() >= 5, "expected >= 5 canonical CEM-ML fixtures");
    for path in &paths {
        let bytes = std::fs::read(path).unwrap();
        let run = run_pipeline_iterations_bare(&bytes, InputFormat::Cem, ITERATIONS);
        assert!(
            run.within(&budget),
            "AC-N-1 fail for {path:?}: median {} ns > effective budget {} ns",
            run.median_ns,
            budget.effective_budget().as_nanos()
        );
    }
}

/// AC-N-1: every HTML parity fixture passes the same envelope.
#[test]
fn ac_n_1_html_parity_fixtures_under_budget() {
    if perf_skipped_for_build() {
        return;
    }
    let budget = BenchmarkBudget::default_ac_n_1();
    let paths = list_fixtures("../../examples/semantic", "html");
    assert!(paths.len() >= 5, "expected >= 5 HTML parity fixtures");
    for path in &paths {
        let bytes = std::fs::read(path).unwrap();
        let run = run_pipeline_iterations_bare(&bytes, InputFormat::Html, ITERATIONS);
        assert!(
            run.within(&budget),
            "AC-N-1 fail for {path:?}: median {} ns > effective budget {} ns",
            run.median_ns,
            budget.effective_budget().as_nanos()
        );
    }
}

/// AC-N-1 / Phase 3.1 production-ready gate: material parity substrate
/// fixtures parse + validate + transform under the same budget envelope
/// as the base canonical fixtures. These fixtures use `<cem-element>`
/// render-time vocabulary, so semantic acceptance remains owned by
/// `cem-elements:verify-substrate`; this test owns first-paint budget
/// proof for the parser/schema-machine/AST-builder pipeline.
#[test]
fn ac_n_1_cem_element_material_parity_fixtures_under_budget() {
    if perf_skipped_for_build() {
        return;
    }
    let budget = BenchmarkBudget::default_ac_n_1();
    let paths = list_fixtures_with_prefix("../../examples/cem-elements", "cem", "material-");
    assert!(
        !paths.is_empty(),
        "expected >= 1 material parity substrate fixture"
    );
    for path in &paths {
        let bytes = std::fs::read(path).unwrap();
        let run = run_pipeline_iterations_bare(&bytes, InputFormat::Cem, ITERATIONS);
        assert!(
            run.within(&budget),
            "AC-N-1 material parity fail for {path:?}: median {} ns > effective budget {} ns",
            run.median_ns,
            budget.effective_budget().as_nanos()
        );
    }
}

/// AC-N-2: a 10 MB synthetic fixture parses without retaining
/// proportional accumulator state. The proof is indirect — we measure
/// wall-clock per byte against a small fixture; if accumulators leaked
/// per-byte instead of per-token, parse time would scale super-linearly.
/// We assert per-byte time stays within an order of magnitude of the
/// small fixture, which is impossible if the tokenizer's state buffer
/// scales with document length.
#[test]
fn ac_n_2_ten_megabyte_fixture_bounded_per_byte() {
    if perf_skipped_for_build() {
        return;
    }
    // Build a ~10 MB well-formed CEM-ML document by repeating one
    // balanced scope so depth stays bounded (depth = 2 throughout).
    let header = b"@doc cem-ml 1\n@ns cem = \"https://cem.dev/ns/core/1\"\n@ns html = \"http://www.w3.org/1999/xhtml\"\n@default html\n\n{main |\n";
    let unit = b"  {span @class=cell | x}\n";
    let footer = b"}\n";
    let target = 10 * 1024 * 1024usize;
    let mut buf = Vec::with_capacity(target + 1024);
    buf.extend_from_slice(header);
    while buf.len() + footer.len() < target {
        buf.extend_from_slice(unit);
    }
    buf.extend_from_slice(footer);
    assert!(
        buf.len() >= target,
        "synthetic fixture size {} < 10 MB target",
        buf.len()
    );

    let small = read("../../examples/cem-ml/login.cem");
    let small_run = run_pipeline_iterations_bare(&small, InputFormat::Cem, 4);
    let big_run = run_pipeline_iterations_bare(&buf, InputFormat::Cem, 2);

    let small_ns_per_byte = small_run.median_ns as f64 / small.len() as f64;
    let big_ns_per_byte = big_run.median_ns as f64 / buf.len() as f64;

    // Floor the small-fixture per-byte rate so tiny inputs (where
    // fixed-cost overhead dominates) do not produce an artificially
    // strict ratio. Anything under 50 ns/byte gets clamped.
    let small_floor = small_ns_per_byte.max(50.0);

    // Sub-linear / linear accumulator scaling means big_ns_per_byte
    // stays in the same per-byte envelope as the small fixture. We
    // accept up to 10× to absorb cache misses on a 10 MB input.
    let ratio = big_ns_per_byte / small_floor;
    assert!(
        ratio <= 10.0,
        "AC-N-2: 10 MB per-byte rate {:.1} ns/byte > 10× small-fixture floor {:.1} ns/byte (ratio {:.2})",
        big_ns_per_byte,
        small_floor,
        ratio
    );

    // Sanity bound: 10 MB still completes inside an Nx job budget.
    // The AC-N-2 envelope is "bounded streaming", not the AC-N-1 wall
    // clock — we accept up to 30 s on the slowest CI runner.
    let budget_ns: u128 = 30 * 1_000_000_000;
    assert!(
        big_run.median_ns <= budget_ns,
        "AC-N-2: 10 MB median {} ns exceeds 30 s envelope",
        big_run.median_ns
    );
}

/// AC-N-3 / AC-N-1: deep-nesting fixture verifies that tokenizer state
/// scales with depth, not byte length. Depth = 200 still fits in the
/// default budget × tolerance.
#[test]
fn ac_n_2_deep_nesting_bounded() {
    if perf_skipped_for_build() {
        return;
    }
    let depth = 200usize;
    let mut buf: Vec<u8> = b"@doc cem-ml 1\n@ns cem = \"https://cem.dev/ns/core/1\"\n@ns html = \"http://www.w3.org/1999/xhtml\"\n@default html\n\n".to_vec();
    for _ in 0..depth {
        buf.extend_from_slice(b"{div |\n");
    }
    buf.extend_from_slice(b"  leaf\n");
    for _ in 0..depth {
        buf.extend_from_slice(b"}\n");
    }
    let run = run_pipeline_iterations_bare(&buf, InputFormat::Cem, 4);
    let budget = BenchmarkBudget::default_ac_n_1();
    assert!(
        run.within(&budget),
        "AC-N-2 (depth=200): median {} ns > effective budget {} ns",
        run.median_ns,
        budget.effective_budget().as_nanos()
    );
}
