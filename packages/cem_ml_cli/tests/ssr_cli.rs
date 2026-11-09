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

fn stylesheet_fixture() -> PathBuf {
    let root = fixture();
    let page = root.join("page.html");
    let text = fs::read_to_string(&page).unwrap();
    fs::write(
        page,
        text.replace(
            "{article | Published}",
            "{style |```@import './styles/theme.css';```}{article | Published}",
        ),
    )
    .unwrap();
    fs::create_dir_all(root.join("mirror/styles/nested")).unwrap();
    fs::write(
        root.join("mirror/styles/theme.css"),
        "@import './nested/color.css'; article { padding: 1px }",
    )
    .unwrap();
    fs::write(
        root.join("mirror/styles/nested/color.css"),
        "article { color: red }",
    )
    .unwrap();
    root
}

#[test]
fn direct_cli_loads_mapped_stylesheets_with_nested_public_bases() {
    let root = stylesheet_fixture();
    let read_map = format!("https://ssr.test/guide/={}", root.join("mirror").display());
    let out = direct(&root, "text/html", &["--resolver-read-map", &read_map]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let html = fs::read_to_string(root.join("out.html")).unwrap();
    assert!(
        html.contains("color: red") || html.contains("color:red"),
        "{html}"
    );
    assert!(
        html.contains("padding: 1px") || html.contains("padding:1px"),
        "{html}"
    );
    assert!(
        !html.split("<style ").skip(1).any(|style| style
            .split("</style>")
            .next()
            .unwrap()
            .contains("@import")),
        "{html}"
    );
    assert!(
        !html.contains(root.join("mirror").to_str().unwrap()),
        "{html}"
    );
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
fn config_cli(root: &std::path::Path, args: &[&str]) -> std::process::Output {
    let mut bindings = String::new();
    for param in &params() {
        let (k, v) = param.split_once('=').unwrap();
        bindings.push_str(&format!("{{param @name=\"{k}\" @value='{v}'}}"));
    }
    let config = format!(
        r#"{{run | {{import @id=page @src="page.html" @content-type="text/html" | {{transform @id=ready @src="ssr.cemt" @template-content-type="text/cem-ml" @entrypoint="ssr:render-page" | {bindings} {{export @id=html @out="page-out.html" @content-type="text/html"}} {{export @id=report @out="page-out.json" @content-type="application/vnd.cem.ssr-report+json"}} }} }} }}"#
    );
    fs::write(root.join("run.cem"), config).unwrap();
    Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .current_dir(root)
        .args(["transform", "--config", "run.cem"])
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn config_cli_shares_proof_for_named_html_and_report_exports_atomically() {
    let root = fixture();
    let out = config_cli(&root, &[]);
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
fn config_cli_hashes_acquired_bytes_and_keeps_both_previous_exports_on_refusal() {
    let root = stylesheet_fixture();
    let report = |dir| {
        let mapping = format!("https://ssr.test/guide/={}", root.join(dir).display());
        let out = config_cli(&root, &["--resolver-read-map", &mapping]);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&fs::read(root.join("page-out.json")).unwrap())
            .unwrap()
    };
    let first = report("mirror");
    let input_bytes = [
        "page.html",
        "mirror/styles/theme.css",
        "mirror/styles/nested/color.css",
    ]
    .iter()
    .map(|path| fs::metadata(root.join(path)).unwrap().len())
    .sum::<u64>();
    assert_eq!(first["usage"]["totalBytes"], input_bytes);
    assert_eq!(first["usage"]["resources"], 3);
    fs::rename(root.join("mirror"), root.join("relocated")).unwrap();
    let moved = report("relocated");
    assert_eq!(first["renderInputIdentity"], moved["renderInputIdentity"]);
    let leaf = root.join("relocated/styles/nested/color.css");
    fs::write(&leaf, "article { color: blue }").unwrap();
    let changed = report("relocated");
    assert_ne!(first["renderInputIdentity"], changed["renderInputIdentity"]);
    let published =
        ["page-out.html", "page-out.json"].map(|path| fs::read(root.join(path)).unwrap());
    fs::write(leaf, b"\xff\xfe").unwrap();
    let mapping = format!(
        "https://ssr.test/guide/={}",
        root.join("relocated").display()
    );
    let failed = config_cli(&root, &["--resolver-read-map", &mapping]);
    assert!(!failed.status.success());
    let diagnostics = String::from_utf8_lossy(&failed.stderr);
    assert!(
        diagnostics.contains("cem.css.import_parse_failed"),
        "{diagnostics}"
    );
    assert_eq!(
        ["page-out.html", "page-out.json"].map(|path| fs::read(root.join(path)).unwrap()),
        published
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cli_refuses_unmapped_or_oversized_resources_without_replacing_output() {
    let root = stylesheet_fixture();
    fs::write(root.join("out.html"), "existing").unwrap();
    let unmapped = direct(&root, "text/html", &[]);
    assert!(!unmapped.status.success());
    let diagnostics = String::from_utf8_lossy(&unmapped.stderr);
    assert!(
        diagnostics.contains("unsupported resolver")
            && diagnostics.contains("https://ssr.test/guide/styles/theme.css"),
        "{diagnostics}"
    );
    let leaf = fs::File::create(root.join("mirror/styles/nested/color.css")).unwrap();
    leaf.set_len(1_048_576).unwrap();
    drop(leaf);
    let mapping = format!("https://ssr.test/guide/={}", root.join("mirror").display());
    let oversized = direct(&root, "text/html", &["--resolver-read-map", &mapping]);
    assert!(!oversized.status.success());
    let diagnostics = String::from_utf8_lossy(&oversized.stderr);
    assert!(
        diagnostics.contains("Original SSR resource byte budget exceeded"),
        "{diagnostics}"
    );
    let page = root.join("page.html");
    let text = fs::read_to_string(&page).unwrap();
    fs::write(
        page,
        text.replace("./styles/theme.css", "https://ssr.test/guide-lookalike.css"),
    )
    .unwrap();
    fs::write(root.join("mirror/-lookalike.css"), "article { color: red }").unwrap();
    let prefix = format!("https://ssr.test/guide={}", root.join("mirror").display());
    let lookalike = direct(&root, "text/html", &["--resolver-read-map", &prefix]);
    assert!(!lookalike.status.success());
    assert!(String::from_utf8_lossy(&lookalike.stderr).contains("unsupported resolver"));
    assert_eq!(
        fs::read_to_string(root.join("out.html")).unwrap(),
        "existing"
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn cli_refuses_stylesheet_symlinks_outside_the_read_map_root() {
    let root = stylesheet_fixture();
    let leaf = root.join("mirror/styles/nested/color.css");
    let outside = root.join("outside.css");
    fs::rename(&leaf, &outside).unwrap();
    std::os::unix::fs::symlink(outside, leaf).unwrap();
    let mapping = format!("https://ssr.test/guide/={}", root.join("mirror").display());
    let refused = direct(&root, "text/html", &["--resolver-read-map", &mapping]);
    assert!(!refused.status.success());
    let diagnostics = String::from_utf8_lossy(&refused.stderr);
    assert!(
        diagnostics.contains("SSR resource escapes its read-map root"),
        "{diagnostics}"
    );
    assert!(!root.join("out.html").exists());
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
