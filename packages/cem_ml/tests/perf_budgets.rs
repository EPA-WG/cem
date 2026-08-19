//! Performance budgets and memory-bounding proofs (AC-N-1 / AC-N-2).
//!
//! - AC-N-1 (Tier A): parse + validate + transform any canonical fixture
//!   in `examples/cem-ml/`, any HTML parity fixture in
//!   `examples/semantic/`, and all 40 source sides declared by the
//!   `<cem-element>` legacy and material parity manifests under **150 ms**,
//!   single-thread, cold cache.
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
use cem_ml::legacy_custom_element::{
    convert_template_source, extract_html_template_fragments, HtmlTemplateFragment,
};
use std::path::{Path, PathBuf};

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

fn external_template_reference(html: &str) -> Option<(String, String)> {
    for marker in ["src=\"", "src='"] {
        let mut cursor = 0;
        while let Some(offset) = html[cursor..].find(marker) {
            let value_start = cursor + offset + marker.len();
            let quote = marker.chars().last()?;
            let value_end = html[value_start..].find(quote)? + value_start;
            let value = &html[value_start..value_end];
            if let Some((source, fragment)) = value.split_once('#') {
                if !source.trim().is_empty() && !fragment.trim().is_empty() {
                    return Some((source.to_owned(), fragment.to_owned()));
                }
            }
            cursor = value_end + quote.len_utf8();
        }
    }
    None
}

fn parity_template_fragments(path: &Path, html: &str) -> Vec<HtmlTemplateFragment> {
    let fragments = extract_html_template_fragments(html);
    if !fragments.is_empty() {
        return fragments;
    }

    let (source, fragment_id) = external_template_reference(html).unwrap_or_else(|| {
        panic!(
            "{} has neither an inline template nor an external fragment reference",
            path.display()
        )
    });
    let source_path = path.parent().unwrap_or_else(|| Path::new("")).join(source);
    let source_html = std::fs::read_to_string(&source_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", source_path.display()));
    let fragment = extract_html_template_fragments(&source_html)
        .into_iter()
        .find(|template| {
            template.attributes.get("id").map(String::as_str) == Some(fragment_id.as_str())
        })
        .unwrap_or_else(|| {
            panic!(
                "{} does not contain referenced template `#{fragment_id}`",
                source_path.display()
            )
        });
    vec![fragment]
}

fn cem_element_parity_sources() -> Vec<(PathBuf, Vec<u8>)> {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let inventories = [
        "../../packages/cem-elements/tests/parity/legacy",
        "../../packages/cem-elements/tests/parity/material",
    ];
    let mut sources = Vec::new();

    for inventory in inventories {
        let directory = crate_root.join(inventory);
        let manifest_path = directory.join("manifest.json");
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&manifest_path)
                .unwrap_or_else(|error| panic!("read {}: {error}", manifest_path.display())),
        )
        .unwrap_or_else(|error| panic!("parse {}: {error}", manifest_path.display()));
        let fixtures = manifest["fixtures"]
            .as_array()
            .expect("parity manifest must contain a `fixtures` array");

        for fixture in fixtures {
            for (key, legacy) in [("legacy", true), ("cemMl", false)] {
                let relative = fixture[key]
                    .as_str()
                    .unwrap_or_else(|| panic!("parity fixture must declare `{key}`"));
                let path = directory.join(relative);
                let html = std::fs::read_to_string(&path)
                    .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
                let mut aggregate = String::new();
                for fragment in parity_template_fragments(&path, &html) {
                    let source = if legacy {
                        let lowered = convert_template_source(&fragment.body);
                        assert!(
                            lowered.diagnostics.is_empty(),
                            "{} emitted legacy lowering diagnostics: {:?}",
                            path.display(),
                            lowered.diagnostics
                        );
                        lowered.source
                    } else {
                        fragment.body
                    };
                    aggregate.push_str(&source);
                    aggregate.push('\n');
                }
                assert!(!aggregate.trim().is_empty(), "{} is empty", path.display());
                sources.push((path, aggregate.into_bytes()));
            }
        }
    }

    sources
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

/// AC-N-1 / Phase 3 aggregate gate: every file-backed legacy and material
/// parity source side parses + validates + transforms under the same budget
/// envelope as the base canonical fixtures. Semantic acceptance is owned by
/// the scoped CLI fixture-validation gate; this test owns the parser/schema-
/// machine/AST-builder first-paint budget proof.
#[test]
fn ac_n_1_cem_element_parity_fixtures_under_budget() {
    if perf_skipped_for_build() {
        return;
    }
    let budget = BenchmarkBudget::default_ac_n_1();
    let sources = cem_element_parity_sources();
    assert_eq!(
        sources.len(),
        40,
        "12 legacy pairs plus 8 material pairs must contribute all 40 source sides"
    );
    for (path, bytes) in &sources {
        let run = run_pipeline_iterations_bare(&bytes, InputFormat::Cem, ITERATIONS);
        assert!(
            run.within(&budget),
            "AC-N-1 CEM Element parity fail for {path:?}: median {} ns > effective budget {} ns",
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
