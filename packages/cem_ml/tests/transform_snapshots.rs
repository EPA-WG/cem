//! Snapshot tests for the light-DOM transform.
//!
//! For each `examples/cem-ml/*.cem` fixture, runs the transform and
//! compares the rendered HTML against the corresponding file in
//! `packages/cem_ml/tests/__snapshots__/`. When a snapshot is missing or
//! out of date, set `CEM_ML_UPDATE_SNAPSHOTS=1` to regenerate it.

use cem_ml::interpreter::light_dom::render_html;

#[test]
fn every_canonical_fixture_matches_snapshot() {
    let fixtures_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/cem-ml");
    let snapshots_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/__snapshots__");
    let update = std::env::var("CEM_ML_UPDATE_SNAPSHOTS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let mut entries: Vec<_> = std::fs::read_dir(&fixtures_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| x == "cem")
                .unwrap_or(false)
        })
        .collect();
    entries.sort_by_key(|e| e.path());
    assert!(
        !entries.is_empty(),
        "no canonical fixtures found in {}",
        fixtures_dir.display()
    );

    for entry in entries {
        let cem_path = entry.path();
        let stem = cem_path.file_stem().unwrap().to_string_lossy().into_owned();
        let snapshot_path = snapshots_dir.join(format!("{stem}.html"));

        let input = std::fs::read_to_string(&cem_path).unwrap();
        let output = render_html(&input);

        // Hard violations from upstream layers fail the snapshot test;
        // snapshots are only meaningful for parseable fixtures.
        let hard: Vec<_> = output
            .diagnostics
            .iter()
            .filter(|d| {
                matches!(
                    d.severity,
                    cem_ml::diagnostics::Severity::Error | cem_ml::diagnostics::Severity::Fatal
                )
            })
            .collect();
        assert!(
            hard.is_empty(),
            "fixture `{}` produced hard violations: {hard:?}",
            cem_path.display()
        );

        if update || !snapshot_path.exists() {
            std::fs::create_dir_all(&snapshots_dir).unwrap();
            std::fs::write(&snapshot_path, &output.rendered).unwrap();
            if !update {
                panic!(
                    "wrote new snapshot `{}`; re-run tests to verify",
                    snapshot_path.display()
                );
            }
            continue;
        }

        let expected = std::fs::read_to_string(&snapshot_path).unwrap();
        assert_eq!(
            output.rendered,
            expected,
            "snapshot mismatch for fixture `{}` (set CEM_ML_UPDATE_SNAPSHOTS=1 to regenerate)",
            cem_path.display(),
        );
    }
}
