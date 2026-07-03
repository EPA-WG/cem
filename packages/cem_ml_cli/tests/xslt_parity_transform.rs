use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const EXIT_OK: i32 = 0;
const EXIT_HARD_FAILURE: i32 = 1;
fn cem_ml(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .args(args)
        .output()
        .expect("run cem-ml binary")
}

fn fixture_root(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("cem-ml-cli-xslt-parity-{name}-{nanos}"))
}

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|err| panic!("create fixture directory {}: {err}", parent.display()));
    }
    fs::write(path, text).unwrap_or_else(|err| panic!("write fixture {}: {err}", path.display()));
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is utf-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is utf-8")
}

fn report(path: &Path) -> serde_json::Value {
    serde_json::from_str(
        &fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("read report {}: {err}", path.display())),
    )
    .unwrap_or_else(|err| panic!("parse report {}: {err}", path.display()))
}

fn has_diagnostic(report: &serde_json::Value, code: &str) -> bool {
    report["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .any(|diagnostic| diagnostic["code"] == code)
}

#[test]
fn direct_cli_executes_xslt_parity_for_login_profile_shape() {
    let root = fixture_root("direct-login-profile");
    let data = root.join("login.cem");
    let template = root.join("profile.xsl");
    let report_path = root.join("report.json");
    write(&data, r#"{main @id="login"}"#);
    write(
        &template,
        r#"<xsl:stylesheet version="1.0"><xsl:template match="/"><main class="login"><h1>Sign in</h1><section class="profile"><xsl:call-template name="row"><xsl:with-param name="label" select="'Display name'"/></xsl:call-template></section></main></xsl:template><xsl:template name="row"><p><xsl:value-of select="$label"/></p></xsl:template></xsl:stylesheet>"#,
    );

    let output = cem_ml(&[
        "transform",
        data.to_str().expect("data path is utf-8"),
        "--data-content-type",
        "text/cem-ml",
        "--template",
        template.to_str().expect("template path is utf-8"),
        "--template-content-type",
        "application/xslt+xml",
        "--to-content-type",
        "text/html",
        "--report-json",
        report_path.to_str().expect("report path is utf-8"),
    ]);

    assert_eq!(output.status.code(), Some(EXIT_OK));
    assert_eq!(
        stdout(&output),
        r#"<main class="login"><h1>Sign in</h1><section class="profile"><p>Display name</p></section></main>"#
    );
    assert!(stderr(&output).trim().is_empty(), "{}", stderr(&output));
    let report = report(&report_path);
    assert_eq!(report["summary"]["hardViolationCount"], 0);
    assert_eq!(report["reportAst"]["transform"]["hasSourceMap"], true);
}

#[test]
fn direct_cli_executes_xslt_named_entrypoint_and_params() {
    let root = fixture_root("direct-named-entrypoint");
    let data = root.join("profile.cem");
    let template = root.join("profile.xsl");
    let report_path = root.join("report.json");
    write(&data, r#"{section @id="ada"}"#);
    write(
        &template,
        r#"<xsl:stylesheet version="1.0"><xsl:template match="/"><section>default</section></xsl:template><xsl:template name="profile"><section class="profile"><p><xsl:value-of select="$label"/></p></section></xsl:template></xsl:stylesheet>"#,
    );

    let output = cem_ml(&[
        "transform",
        data.to_str().expect("data path is utf-8"),
        "--data-content-type",
        "text/cem-ml",
        "--template",
        template.to_str().expect("template path is utf-8"),
        "--template-content-type",
        "application/xslt+xml",
        "--template-entrypoint",
        "profile",
        "--param",
        "label=Display name",
        "--to-content-type",
        "text/html",
        "--report-json",
        report_path.to_str().expect("report path is utf-8"),
    ]);

    assert_eq!(output.status.code(), Some(EXIT_OK));
    assert_eq!(
        stdout(&output),
        r#"<section class="profile"><p>Display name</p></section>"#
    );
    assert!(stderr(&output).trim().is_empty(), "{}", stderr(&output));
    let report = report(&report_path);
    assert_eq!(report["summary"]["hardViolationCount"], 0);
    assert_eq!(report["reportAst"]["transform"]["hasSourceMap"], true);
}

#[test]
fn direct_cli_reports_missing_xslt_named_entrypoint() {
    let root = fixture_root("direct-missing-entrypoint");
    let data = root.join("profile.cem");
    let template = root.join("profile.xsl");
    let report_path = root.join("report.json");
    write(&data, r#"{section @id="ada"}"#);
    write(
        &template,
        r#"<xsl:stylesheet version="1.0"><xsl:template match="/"><section>default</section></xsl:template></xsl:stylesheet>"#,
    );

    let output = cem_ml(&[
        "transform",
        data.to_str().expect("data path is utf-8"),
        "--data-content-type",
        "text/cem-ml",
        "--template",
        template.to_str().expect("template path is utf-8"),
        "--template-content-type",
        "application/xslt+xml",
        "--template-entrypoint",
        "missing",
        "--to-content-type",
        "text/html",
        "--report-json",
        report_path.to_str().expect("report path is utf-8"),
    ]);

    assert_eq!(output.status.code(), Some(EXIT_HARD_FAILURE));
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    assert!(stderr(&output).trim().is_empty(), "{}", stderr(&output));
    let report = report(&report_path);
    assert!(has_diagnostic(
        &report,
        "cem.transform_template.call_unknown"
    ));
    assert_eq!(report["summary"]["hardViolationCount"], 1);
}

#[test]
fn direct_cli_reports_unsupported_xslt_construct_without_output() {
    let root = fixture_root("direct-unsupported-construct");
    let data = root.join("profile.cem");
    let template = root.join("profile.xsl");
    let report_path = root.join("report.json");
    let out = root.join("out/profile.html");
    write(&data, r#"{section @id="ada"}"#);
    write(
        &template,
        r#"<xsl:stylesheet version="1.0"><xsl:template match="/"><msxsl:script language="JScript">function run(){return 1;}</msxsl:script></xsl:template></xsl:stylesheet>"#,
    );

    let output = cem_ml(&[
        "transform",
        data.to_str().expect("data path is utf-8"),
        "--data-content-type",
        "text/cem-ml",
        "--template",
        template.to_str().expect("template path is utf-8"),
        "--template-content-type",
        "application/xslt+xml",
        "--to-content-type",
        "text/html",
        "--out",
        out.to_str().expect("output path is utf-8"),
        "--report-json",
        report_path.to_str().expect("report path is utf-8"),
    ]);

    assert_eq!(output.status.code(), Some(EXIT_HARD_FAILURE));
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    assert!(stderr(&output).trim().is_empty(), "{}", stderr(&output));
    assert!(
        !out.exists(),
        "failed direct transform must not write output"
    );
    assert!(
        !PathBuf::from(format!("{}.map", out.display())).exists(),
        "failed direct transform must not write source-map sidecar"
    );
    let report = report(&report_path);
    assert!(has_diagnostic(&report, "legacy_xslt.unsupported_construct"));
    assert_eq!(report["summary"]["hardViolationCount"], 1);
}

#[test]
fn graph_config_executes_xslt_parity_asset_list_and_writes_sidecar() {
    let root = fixture_root("graph-asset-list");
    let data = root.join("asset.cem");
    let template = root.join("assets.xsl");
    let graph = root.join("graph.cem");
    let report_path = root.join("report.json");
    let out = root.join("out/assets.html");
    write(&data, r#"{article @id="asset"}"#);
    write(
        &template,
        r#"<xsl:stylesheet version="1.0"><xsl:variable name="assets"><asset>Logo</asset><asset>Hero</asset></xsl:variable><xsl:template match="/"><ul><li>default</li></ul></xsl:template><xsl:template name="assets"><ul><xsl:apply-templates select="exsl:node-set($assets)/*"/><li><xsl:value-of select="$suffix"/></li></ul></xsl:template><xsl:template match="asset"><li><xsl:value-of select="."/></li></xsl:template></xsl:stylesheet>"#,
    );
    write(
        &graph,
        r#"{run |
  {import @id=asset @src="asset.cem" @content-type="text/cem-ml" |
    {transform @id=html @src="assets.xsl" @template-content-type="application/xslt+xml" @entrypoint="assets" |
      {param @name="suffix" @value="{stem}"}
      {export @id=main @out="out/assets.html" @content-type="text/html"}
    }
  }
}"#,
    );

    let output = cem_ml(&[
        "transform",
        "--config",
        graph.to_str().expect("graph path is utf-8"),
        "--report-json",
        report_path.to_str().expect("report path is utf-8"),
    ]);

    assert_eq!(output.status.code(), Some(EXIT_OK));
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    assert!(stderr(&output).trim().is_empty(), "{}", stderr(&output));
    assert_eq!(
        fs::read_to_string(&out).expect("read export"),
        "<ul><li>Logo</li><li>Hero</li><li>asset</li></ul>"
    );
    let sidecar = format!("{}.map", out.display());
    let sidecar_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&sidecar).unwrap_or_else(|err| panic!("read sidecar {sidecar}: {err}")),
    )
    .unwrap_or_else(|err| panic!("parse sidecar {sidecar}: {err}"));
    assert_eq!(sidecar_json["exportId"], "main");
    assert_eq!(sidecar_json["destination"], out.display().to_string());
    let report = report(&report_path);
    assert_eq!(report["summary"]["hardViolationCount"], 0);
    assert_eq!(
        report["reportAst"]["transformGraph"]["exports"][0]["sourceMapRef"],
        sidecar
    );
    assert_eq!(
        report["reportAst"]["transformGraph"]["exports"][0]["hasSourceMap"],
        true
    );
}

#[test]
fn graph_config_reports_missing_xslt_named_entrypoint_without_export() {
    let root = fixture_root("graph-missing-entrypoint");
    let data = root.join("asset.cem");
    let template = root.join("assets.xsl");
    let graph = root.join("graph.cem");
    let report_path = root.join("report.json");
    let out = root.join("out/assets.html");
    write(&data, r#"{article @id="asset"}"#);
    write(
        &template,
        r#"<xsl:stylesheet version="1.0"><xsl:template match="/"><ul><li>default</li></ul></xsl:template></xsl:stylesheet>"#,
    );
    write(
        &graph,
        r#"{run |
  {import @id=asset @src="asset.cem" @content-type="text/cem-ml" |
    {transform @id=html @src="assets.xsl" @template-content-type="application/xslt+xml" @entrypoint="missing" |
      {export @id=main @out="out/assets.html" @content-type="text/html"}
    }
  }
}"#,
    );

    let output = cem_ml(&[
        "transform",
        "--config",
        graph.to_str().expect("graph path is utf-8"),
        "--report-json",
        report_path.to_str().expect("report path is utf-8"),
    ]);

    assert_eq!(output.status.code(), Some(EXIT_HARD_FAILURE));
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    assert!(stderr(&output).trim().is_empty(), "{}", stderr(&output));
    assert!(!out.exists(), "failed graph stage must not write export");
    assert!(
        !PathBuf::from(format!("{}.map", out.display())).exists(),
        "failed graph stage must not write source-map sidecar"
    );
    let report = report(&report_path);
    assert!(has_diagnostic(
        &report,
        "cem.transform_template.call_unknown"
    ));
    assert_eq!(report["summary"]["hardViolationCount"], 1);
    assert_eq!(report["reportAst"]["transformGraph"]["exportCount"], 0);
    assert!(report["reportAst"]["transformGraph"]["exports"]
        .as_array()
        .expect("exports array")
        .is_empty());
}

#[test]
fn graph_config_reports_unsupported_xslt_construct_without_export() {
    let root = fixture_root("graph-unsupported-construct");
    let data = root.join("asset.cem");
    let template = root.join("assets.xsl");
    let graph = root.join("graph.cem");
    let report_path = root.join("report.json");
    let out = root.join("out/assets.html");
    write(&data, r#"{article @id="asset"}"#);
    write(
        &template,
        r#"<xsl:stylesheet version="1.0"><xsl:template match="/"><msxsl:script language="JScript">function run(){return 1;}</msxsl:script></xsl:template></xsl:stylesheet>"#,
    );
    write(
        &graph,
        r#"{run |
  {import @id=asset @src="asset.cem" @content-type="text/cem-ml" |
    {transform @id=html @src="assets.xsl" @template-content-type="application/xslt+xml" |
      {export @id=main @out="out/assets.html" @content-type="text/html"}
    }
  }
}"#,
    );

    let output = cem_ml(&[
        "transform",
        "--config",
        graph.to_str().expect("graph path is utf-8"),
        "--report-json",
        report_path.to_str().expect("report path is utf-8"),
    ]);

    assert_eq!(output.status.code(), Some(EXIT_HARD_FAILURE));
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    assert!(stderr(&output).trim().is_empty(), "{}", stderr(&output));
    assert!(!out.exists(), "failed graph stage must not write export");
    assert!(
        !PathBuf::from(format!("{}.map", out.display())).exists(),
        "failed graph stage must not write source-map sidecar"
    );
    let report = report(&report_path);
    assert!(has_diagnostic(&report, "legacy_xslt.unsupported_construct"));
    assert_eq!(report["summary"]["hardViolationCount"], 1);
    assert_eq!(report["reportAst"]["transformGraph"]["exportCount"], 0);
    assert!(report["reportAst"]["transformGraph"]["exports"]
        .as_array()
        .expect("exports array")
        .is_empty());
}

#[test]
fn graph_config_executes_mixed_cem_native_and_xslt_stage_policies() {
    let root = fixture_root("graph-mixed-runtime");
    let data = root.join("asset.cem");
    let native_template = root.join("card.cem");
    let xslt_template = root.join("shell.xsl");
    let graph = root.join("graph.cem");
    let report_path = root.join("report.json");
    let native_out = root.join("out/card.html");
    let xslt_out = root.join("out/shell.html");
    write(&data, r#"{article @id="asset"}"#);
    write(
        &native_template,
        r#"{@doc cem-ml 1}
{module |
  {template @name="card" @visibility="public" |
    {param @name="title" @required="true"}
    {body | {article | {$ title }}}
  }
}"#,
    );
    write(
        &xslt_template,
        r#"<xsl:stylesheet version="1.0"><xsl:template match="/"><main><h1>Sign in</h1></main></xsl:template></xsl:stylesheet>"#,
    );
    write(
        &graph,
        r#"{run |
  {import @id=asset @src="asset.cem" @content-type="text/cem-ml" |
    {transform @id=card @src="card.cem" @template-content-type="text/cem-ml" @template-schema="https://cem.dev/ns/template/cem-native/1" @entrypoint="card" |
      {param @name="title" @value="{stem}"}
      {export @id=cardOut @out="out/card.html" @content-type="text/html"}
    }
    {transform @id=shell @src="shell.xsl" @template-content-type="application/xslt+xml" |
      {export @id=shellOut @out="out/shell.html" @content-type="text/html"}
    }
  }
}"#,
    );

    let output = cem_ml(&[
        "transform",
        "--config",
        graph.to_str().expect("graph path is utf-8"),
        "--report-json",
        report_path.to_str().expect("report path is utf-8"),
    ]);

    assert_eq!(output.status.code(), Some(EXIT_OK));
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    assert!(stderr(&output).trim().is_empty(), "{}", stderr(&output));
    assert_eq!(
        fs::read_to_string(&native_out).expect("read native export"),
        "<article>asset</article>"
    );
    assert_eq!(
        fs::read_to_string(&xslt_out).expect("read xslt export"),
        "<main><h1>Sign in</h1></main>"
    );
    let report = report(&report_path);
    assert_eq!(report["summary"]["hardViolationCount"], 0);
    assert_eq!(report["reportAst"]["transformGraph"]["exportCount"], 2);
}

#[test]
fn graph_config_projects_inline_style_export_to_css_and_links_html() {
    let root = fixture_root("graph-inline-style-css-export");
    let data = root.join("asset.cem");
    let template = root.join("page.xsl");
    let graph = root.join("graph.cem");
    let report_path = root.join("report.json");
    let html_out = root.join("out/page.html");
    let inline_out = root.join("out/page-inline.html");
    let omit_out = root.join("out/page-omit.html");
    let css_out = root.join("out/page.css");
    let html_map = root.join("out/page.html.map");
    let omit_map = root.join("out/page-omit.html.map");
    let css_map = root.join("out/page.css.map");
    write(&data, r#"{article @id="asset"}"#);
    write(
        &template,
        r#"<xsl:stylesheet version="1.0"><xsl:template match="/"><html><head><style>.card { color: red; }</style></head><body><main class="card"><h1>Asset</h1></main></body></html></xsl:template></xsl:stylesheet>"#,
    );
    write(
        &graph,
        r#"{run |
  {import @id=asset @src="asset.cem" @content-type="text/cem-ml" |
    {transform @id=page @src="page.xsl" @template-content-type="application/xslt+xml" |
      {export @id=htmlOut @out="out/page.html" @content-type="text/html"}
      {export @id=inlineOut @out="out/page-inline.html" @content-type="text/html" @style-policy="inline"}
      {export @id=omitOut @out="out/page-omit.html" @content-type="text/html" @style-policy="omit"}
      {export @id=cssOut @out="out/page.css" @content-type="text/css" @schema="https://cem.dev/ns/data/css/1"}
    }
  }
}"#,
    );

    let output = cem_ml(&[
        "transform",
        "--config",
        graph.to_str().expect("graph path is utf-8"),
        "--report-json",
        report_path.to_str().expect("report path is utf-8"),
    ]);

    assert_eq!(output.status.code(), Some(EXIT_OK));
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    assert!(stderr(&output).trim().is_empty(), "{}", stderr(&output));
    let html = fs::read_to_string(&html_out).expect("read html export");
    assert!(
        html.contains(r#"<link rel="stylesheet" href="page.css">"#),
        "{html}"
    );
    assert!(!html.contains("<style>"), "{html}");
    assert!(
        html.contains(r#"<main class="card"><h1>Asset</h1></main>"#),
        "{html}"
    );
    let inline = fs::read_to_string(&inline_out).expect("read inline html export");
    assert!(
        inline.contains("<style>.card { color: red; }</style>"),
        "{inline}"
    );
    assert!(!inline.contains(r#"rel="stylesheet""#), "{inline}");
    let omitted = fs::read_to_string(&omit_out).expect("read omitted html export");
    assert!(!omitted.contains("<style>"), "{omitted}");
    assert!(!omitted.contains(r#"rel="stylesheet""#), "{omitted}");
    assert!(
        omitted.contains(r#"<main class="card"><h1>Asset</h1></main>"#),
        "{omitted}"
    );
    assert_eq!(
        fs::read_to_string(&css_out).expect("read css export"),
        ".card { color: red; }\n"
    );
    let graph_report = report(&report_path);
    assert_eq!(graph_report["summary"]["hardViolationCount"], 0);
    assert_eq!(
        graph_report["reportAst"]["transformGraph"]["exportCount"],
        4
    );
    let exports = graph_report["reportAst"]["transformGraph"]["exports"]
        .as_array()
        .expect("exports array");
    let html_export = exports
        .iter()
        .find(|export| {
            export["exportId"] == "htmlOut"
                && export["contentType"] == "text/html"
                && export["destination"] == html_out.display().to_string()
        })
        .expect("htmlOut export report");
    assert_eq!(html_export["hasSourceMap"], true);
    assert!(html_export["outputSpanCount"].as_u64().unwrap() > 0);
    assert_eq!(html_export["sourceMapRef"], html_map.display().to_string());
    assert!(html_map.exists());
    assert!(!report(&html_map)["outputSpans"]
        .as_array()
        .unwrap()
        .is_empty());
    let omit_export = exports
        .iter()
        .find(|export| {
            export["exportId"] == "omitOut"
                && export["contentType"] == "text/html"
                && export["destination"] == omit_out.display().to_string()
        })
        .expect("omitOut export report");
    assert_eq!(omit_export["hasSourceMap"], true);
    assert!(omit_export["outputSpanCount"].as_u64().unwrap() > 0);
    assert_eq!(omit_export["sourceMapRef"], omit_map.display().to_string());
    assert!(omit_map.exists());
    assert!(!report(&omit_map)["outputSpans"]
        .as_array()
        .unwrap()
        .is_empty());
    let css_export = exports
        .iter()
        .find(|export| {
            export["exportId"] == "cssOut"
                && export["contentType"] == "text/css"
                && export["destination"] == css_out.display().to_string()
        })
        .expect("cssOut export report");
    assert_eq!(css_export["schema"], "https://cem.dev/ns/data/css/1");
    assert_eq!(css_export["hasSourceMap"], true);
    assert!(css_export["outputSpanCount"].as_u64().unwrap() > 0);
    assert_eq!(css_export["sourceMapRef"], css_map.display().to_string());
    assert!(css_map.exists());
    assert!(!report(&css_map)["outputSpans"]
        .as_array()
        .unwrap()
        .is_empty());
}
