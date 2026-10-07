use cem_ml::{
    ast::reload::{ReferenceReloadBundle, ReloadLimits},
    value::artifact::{CemValueArtifactLimits, CemValueGraph},
};
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn cli_exports_source_bundle_reloads_without_parsing_and_exports_materialized_results() {
    let root = std::env::temp_dir().join(format!(
        "cem-reference-cli-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("source.cem");
    let bundle = root.join("source.reload");
    fs::write(&source, "{div}{#input}").unwrap();
    let produce = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .args(["query"])
        .arg(&source)
        .args([
            "--content-type",
            "text/cem-ml",
            "--query-content-type",
            "application/vnd.cem.query-expression+cem-ql",
            "--query",
            "#()",
            "--output",
            "cemv",
            "--export-reload-bundle",
        ])
        .arg(&bundle)
        .output()
        .unwrap();
    assert!(
        produce.status.success(),
        "{}",
        String::from_utf8_lossy(&produce.stderr)
    );
    let _ = ReferenceReloadBundle::decode(&fs::read(&bundle).unwrap(), ReloadLimits::default())
        .unwrap();
    let result =
        CemValueGraph::decode(&produce.stdout, &CemValueArtifactLimits::default()).unwrap();
    assert_eq!(result.records[result.roots[0] as usize].kind, "reference");
    assert!(result.records[result.roots[0] as usize].targets.is_empty());
    fs::remove_file(&source).unwrap();
    let consume = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .arg("query")
        .arg(&bundle)
        .args([
            "--reload-bundle",
            "--query-content-type",
            "application/vnd.cem.query-expression+cem-ql",
            "--query",
            "input.children.kind",
            "--output",
            "json",
        ])
        .output()
        .unwrap();
    assert!(
        consume.status.success(),
        "{}",
        String::from_utf8_lossy(&consume.stderr)
    );
    assert!(String::from_utf8_lossy(&consume.stdout).contains("reference"));
    let report_path = root.join("rejected.json");
    let rejected = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .arg("query")
        .arg(&bundle)
        .args([
            "--reload-bundle",
            "--query-content-type",
            "application/vnd.cem.query-expression+cem-ql",
            "--query",
            "input",
            "--output",
            "cemv",
        ])
        .arg("--report-json")
        .arg(&report_path)
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(report_path).unwrap()).unwrap();
    let diagnostic = report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["code"] == "cem.value.unsupported_source_reference")
        .unwrap();
    assert_eq!(diagnostic["details"]["kind"], "UnsupportedSourceReference");
    assert!(!diagnostic["sourceMap"]["frames"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(diagnostic["uri"].as_str().unwrap().ends_with("source.cem"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cli_requires_explicit_bundle_admission_and_source_identity() {
    use cem_ml_cli::cli::Cli;
    use clap::Parser;
    let base = [
        "cem-ml",
        "query",
        "input.reload",
        "--query",
        "input",
        "--query-content-type",
        "application/vnd.cem.query-expression+cem-ql",
    ];
    assert!(Cli::try_parse_from(base.into_iter().chain(["--reload-source-id", "2"])).is_err());
    assert!(Cli::try_parse_from(base.into_iter().chain([
        "--reload-bundle",
        "--reload-source-id",
        "2"
    ]))
    .is_ok());
}
