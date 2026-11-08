#![cfg(any(target_os = "linux", windows))]
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
fn fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "cem-ssr-cli-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("page.html"),r#"<!doctype html><html><head><title>SSR</title></head><body><cem-element tag="x-card" version="1.2.3"><template id="x-card" type="text/cem-ml">{article | Published}</template></cem-element><x-card></x-card></body></html>"#).unwrap();
    fs::write(
        root.join("ssr.cemt"),
        include_str!("../../cem_ml_transform_cem_ql/templates/ssr-page.cemt"),
    )
    .unwrap();
    root
}
fn params() -> [&'static str; 9] {
    [
        "page-url=https://ssr.test/guide/",
        "consumer-versions=x-card=1.9.0",
        "max-depth=8",
        "max-instances=8",
        "max-render-passes=8",
        "max-resources=16",
        "max-bytes=1048576",
        "wall-ms=60000",
        "cpu-ms=10000",
    ]
}
fn direct(root: &std::path::Path, target: &str, extra: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cem-ml"));
    cmd.current_dir(root).args([
        "transform",
        "page.html",
        "--template",
        "ssr.cemt",
        "--template-content-type",
        "text/cem-ml",
        "--template-entrypoint",
        "ssr:render-page",
        "--to-content-type",
        target,
        "--out",
        "out.html",
    ]);
    for param in &params() {
        cmd.args(["--param", param]);
    }
    cmd.args(extra).output().unwrap()
}
#[test]
fn direct_cli_lowers_to_native_proof_and_publishes_uncolored_html() {
    let root = fixture();
    let out = direct(&root, "text/html", &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let html = fs::read_to_string(root.join("out.html")).unwrap();
    assert!(html.contains("<article>Published</article>"), "{html}");
    assert!(html.contains("data-cem-island="));
    assert!(!html.contains('\u{1b}'));
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn direct_cli_refuses_coloring_without_replacing_existing_output() {
    let root = fixture();
    fs::write(root.join("out.html"), "existing").unwrap();
    let out = direct(&root, "text/html", &["--output-color-type", "html"]);
    assert!(!out.status.success());
    assert_eq!(
        fs::read_to_string(root.join("out.html")).unwrap(),
        "existing"
    );
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn config_cli_shares_proof_for_named_html_and_report_exports_atomically() {
    let root = fixture();
    let mut bindings = String::new();
    for param in &params() {
        let (k, v) = param.split_once('=').unwrap();
        bindings.push_str(&format!("{{param @name=\"{k}\" @value='{v}'}}"));
    }
    let config = format!(
        r#"{{run | {{import @id=page @src="page.html" @content-type="text/html" | {{transform @id=ready @src="ssr.cemt" @template-content-type="text/cem-ml" @entrypoint="ssr:render-page" | {bindings} {{export @id=html @out="page-out.html" @content-type="text/html"}} {{export @id=report @out="page-out.json" @content-type="application/vnd.cem.ssr-report+json"}} }} }} }}"#
    );
    fs::write(root.join("run.cem"), config).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .current_dir(&root)
        .args(["transform", "--config", "run.cem"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(fs::read_to_string(root.join("page-out.html"))
        .unwrap()
        .contains("<article>Published</article>"));
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("page-out.json")).unwrap()).unwrap();
    assert_eq!(report["readiness"], "native-ready");
    assert_eq!(report["hydrationReady"], false);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_graph_export_leaves_all_destinations_unpublished() {
    for export in [
        r#"{export @id=bad @out="bad.json" @content-type="application/json"}"#,
        r#"{export @id=bad @out="bad.html" @content-type="text/html" @style-policy="omit"}"#,
    ] {
        let root = fixture();
        fs::write(root.join("page-out.html"), "existing").unwrap();
        let mut bindings = String::new();
        for param in params() {
            let (key, value) = param.split_once('=').unwrap();
            bindings.push_str(&format!("{{param @name='{key}' @value='{value}'}}"));
        }
        fs::write(root.join("run.cem"),format!(r#"{{run | {{import @id=page @src="page.html" @content-type="text/html" | {{transform @id=ready @src="ssr.cemt" @template-content-type="text/cem-ml" @entrypoint="ssr:render-page" | {bindings} {{export @id=html @out="page-out.html" @content-type="text/html"}} {export} }} }} }}"#)).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .current_dir(&root)
            .args(["transform", "--config", "run.cem"])
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert_eq!(
            fs::read_to_string(root.join("page-out.html")).unwrap(),
            "existing"
        );
        assert!(!root.join("bad.json").exists());
        assert!(!root.join("bad.html").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
