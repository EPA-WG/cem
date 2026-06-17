use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const EXIT_OK: i32 = 0;
const EXIT_HARD_FAILURE: i32 = 1;
const CEM_NATIVE_SCHEMA: &str = "https://cem.dev/ns/template/cem-native/1";

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
    std::env::temp_dir().join(format!("cem-ml-cli-conformance-{name}-{nanos}"))
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
        .expect("report diagnostics array")
        .iter()
        .any(|diagnostic| diagnostic["code"] == code)
}

fn transform_args<'a>(
    data: &'a Path,
    template: &'a Path,
    report: &'a Path,
    extra: &[&'a str],
) -> Vec<&'a str> {
    let mut args = vec![
        "transform",
        data.to_str().expect("data path is utf-8"),
        "--data-content-type",
        "text/cem-ml",
        "--template",
        template.to_str().expect("template path is utf-8"),
        "--template-content-type",
        "text/cem-ml",
        "--template-schema",
        CEM_NATIVE_SCHEMA,
        "--to-content-type",
        "text/html",
        "--report-json",
        report.to_str().expect("report path is utf-8"),
    ];
    args.extend_from_slice(extra);
    args
}

#[test]
fn direct_cli_covers_implicit_and_named_entrypoints_for_login_view() {
    let root = fixture_root("entrypoints-login");
    let data = root.join("login.cem");
    let template = root.join("login-page.cem");
    let implicit_report = root.join("implicit-report.json");
    let named_report = root.join("named-report.json");
    let default_null_report = root.join("default-null-report.json");
    write(&data, r#"{main @id="login"}"#);
    write(
        &template,
        r#"{@doc cem-ml 1}
{module |
  {body | {main | Login}}
  {template @name="form" @visibility="public" |
    {param @name="title" @default="Sign in"}
    {param @name="subtitle" @nullable="true"}
    {body | {form | {call @template="heading" @with:text="{title}"}}}
  }
  {template @name="heading" |
    {param @name="text" @required="true"}
    {body | {h1 | {$ text }}}
  }
}"#,
    );

    let implicit = cem_ml(&transform_args(&data, &template, &implicit_report, &[]));

    assert_eq!(implicit.status.code(), Some(EXIT_OK));
    assert_eq!(stdout(&implicit), "<main>Login</main>");
    assert!(stderr(&implicit).trim().is_empty(), "{}", stderr(&implicit));
    assert_eq!(report(&implicit_report)["summary"]["hardViolationCount"], 0);

    let named = cem_ml(&transform_args(
        &data,
        &template,
        &named_report,
        &["--template-entrypoint", "form", "--param", "title=Register"],
    ));

    assert_eq!(named.status.code(), Some(EXIT_OK));
    assert_eq!(stdout(&named), "<form><h1>Register</h1></form>");
    assert!(stderr(&named).trim().is_empty(), "{}", stderr(&named));
    assert_eq!(report(&named_report)["summary"]["hardViolationCount"], 0);

    let default_null = cem_ml(&transform_args(
        &data,
        &template,
        &default_null_report,
        &["--template-entrypoint", "form", "--param", "subtitle=null"],
    ));

    assert_eq!(default_null.status.code(), Some(EXIT_OK));
    assert_eq!(stdout(&default_null), "<form><h1>Sign in</h1></form>");
    assert!(
        stderr(&default_null).trim().is_empty(),
        "{}",
        stderr(&default_null)
    );
    assert_eq!(
        report(&default_null_report)["summary"]["hardViolationCount"],
        0
    );
}

#[test]
fn direct_cli_reports_entrypoint_and_param_contract_failures() {
    let root = fixture_root("contract-failures");
    let data = root.join("profile.cem");
    let template = root.join("profile-page.cem");
    let private_report = root.join("private-report.json");
    let missing_report = root.join("missing-report.json");
    let valid_type_report = root.join("valid-type-report.json");
    let type_report = root.join("type-report.json");
    write(&data, r#"{section @id="ada"}"#);
    write(
        &template,
        r#"{@doc cem-ml 1}
{module |
  {template @name="helper" | {body | {span | Private}}}
  {template @name="stats" @visibility="public" |
    {param @name="count" @type="integer" @required="true"}
    {body | {output | {$ count }}}
  }
}"#,
    );

    let private = cem_ml(&transform_args(
        &data,
        &template,
        &private_report,
        &["--template-entrypoint", "helper"],
    ));
    assert_eq!(private.status.code(), Some(EXIT_HARD_FAILURE));
    assert!(stdout(&private).is_empty(), "{}", stdout(&private));
    assert!(stderr(&private).trim().is_empty(), "{}", stderr(&private));
    assert!(has_diagnostic(
        &report(&private_report),
        "cem.transform_template.entrypoint_not_public"
    ));

    let missing = cem_ml(&transform_args(
        &data,
        &template,
        &missing_report,
        &["--template-entrypoint", "missing"],
    ));
    assert_eq!(missing.status.code(), Some(EXIT_HARD_FAILURE));
    assert!(has_diagnostic(
        &report(&missing_report),
        "cem.transform_template.entrypoint_not_public"
    ));

    let valid_type = cem_ml(&transform_args(
        &data,
        &template,
        &valid_type_report,
        &["--template-entrypoint", "stats", "--param", "count=7"],
    ));
    assert_eq!(valid_type.status.code(), Some(EXIT_OK));
    assert_eq!(stdout(&valid_type), "<output>7</output>");
    assert_eq!(
        report(&valid_type_report)["summary"]["hardViolationCount"],
        0
    );

    let type_mismatch = cem_ml(&transform_args(
        &data,
        &template,
        &type_report,
        &[
            "--template-entrypoint",
            "stats",
            "--param",
            "count=not-an-integer",
        ],
    ));
    assert_eq!(type_mismatch.status.code(), Some(EXIT_HARD_FAILURE));
    assert!(has_diagnostic(
        &report(&type_report),
        "cem.transform_template.param_type"
    ));
}

#[test]
fn graph_config_covers_imported_module_asset_card_and_sidecar() {
    let root = fixture_root("graph-asset-card");
    let data = root.join("asset.cem");
    let page = root.join("page.cem");
    let stats = root.join("stats.cem");
    let ui = root.join("ui.cem");
    let graph = root.join("graph.cem");
    let report_path = root.join("report.json");
    let out = root.join("out/asset.html");
    write(&data, r#"{article @id="hero"}"#);
    write(
        &page,
        r#"{@doc cem-ml 1}
{module |
  {import @as="ui" @src="ui.cem" @content-type="text/cem-ml"}
  {template @name="page" @visibility="public" |
    {param @name="title" @required="true"}
    {body | {main | {call @from="ui" @template="card" @with:title="{title}"}{aside | {$stats}}}}
  }
}"#,
    );
    write(
        &stats,
        r#"{@doc cem-ml 1}
{module |
  {body | {span | {$datadom.attributes.kind}}}
}"#,
    );
    write(
        &ui,
        r#"{@doc cem-ml 1}
{module |
  {template @name="card" @visibility="public" |
    {param @name="title" @required="true"}
    {body | {article | {$ title }}}
  }
}"#,
    );
    write(
        &graph,
        r#"{run |
  {import @id=asset @src="asset.cem" @content-type="text/cem-ml" |
    {transform @id=stats @src="stats.cem" @template-content-type="text/cem-ml" @template-schema="https://cem.dev/ns/template/cem-native/1"}
    {transform @id=html @src="page.cem" @template-content-type="text/cem-ml" @template-schema="https://cem.dev/ns/template/cem-native/1" @entrypoint="page" @with:stats=stats |
      {param @name="title" @value="{stem}"}
      {export @id=main @out="out/{stem}.html" @content-type="text/html"}
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
        "<main><article>asset</article><aside>&lt;span&gt;document&lt;/span&gt;</aside></main>"
    );
    let sidecar = format!("{}.map", out.display());
    let sidecar_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&sidecar).unwrap_or_else(|err| panic!("read sidecar {sidecar}: {err}")),
    )
    .unwrap_or_else(|err| panic!("parse sidecar {sidecar}: {err}"));
    assert_eq!(sidecar_json["exportId"], "main");
    let report = report(&report_path);
    assert_eq!(report["summary"]["hardViolationCount"], 0);
    assert_eq!(
        report["reportAst"]["transformGraph"]["exports"][0]["sourceMapRef"],
        sidecar
    );
}

#[test]
fn direct_cli_reports_import_cycles_depth_limits_and_recursion_limits() {
    let root = fixture_root("preflight-limits");
    let data = root.join("source.cem");
    let cycle_page = root.join("cycle-page.cem");
    let cycle_ui = root.join("cycle-ui.cem");
    let depth_page = root.join("depth-page.cem");
    let recursive_page = root.join("recursive-page.cem");
    let cycle_report = root.join("cycle-report.json");
    let depth_report = root.join("depth-report.json");
    let recursion_report = root.join("recursion-report.json");
    write(&data, "{p | Source}");
    write(
        &cycle_page,
        r#"{@doc cem-ml 1}
{module |
  {import @as="ui" @src="cycle-ui.cem" @content-type="text/cem-ml"}
  {body | {main | Cycle}}
}"#,
    );
    write(
        &cycle_ui,
        r#"{@doc cem-ml 1}
{module |
  {import @as="page" @src="cycle-page.cem" @content-type="text/cem-ml"}
  {template @name="card" @visibility="public" | {body | {article | Card}}}
}"#,
    );
    write(
        &depth_page,
        r#"{@doc cem-ml 1}
{module |
  {import @as="next" @src="depth-00.cem" @content-type="text/cem-ml"}
  {body | {main | Depth}}
}"#,
    );
    for index in 0..34 {
        let path = root.join(format!("depth-{index:02}.cem"));
        if index == 33 {
            write(
                &path,
                r#"{@doc cem-ml 1}
{module |
  {body | {span | Leaf}}
}"#,
            );
        } else {
            write(
                &path,
                &format!(
                    "{{@doc cem-ml 1}}
{{module |
  {{import @as=\"next\" @src=\"depth-{next:02}.cem\" @content-type=\"text/cem-ml\"}}
}}",
                    next = index + 1
                ),
            );
        }
    }
    write(
        &recursive_page,
        r#"{@doc cem-ml 1}
{module |
  {template @name="loop" | {body | {span | Loop {call @template="loop"}}}}
  {body | {main | {call @template="loop"}}}
}"#,
    );

    let cycle = cem_ml(&transform_args(&data, &cycle_page, &cycle_report, &[]));
    assert_eq!(cycle.status.code(), Some(EXIT_HARD_FAILURE));
    assert!(has_diagnostic(
        &report(&cycle_report),
        "cem.transform_template.import_cycle"
    ));

    let depth = cem_ml(&transform_args(&data, &depth_page, &depth_report, &[]));
    assert_eq!(depth.status.code(), Some(EXIT_HARD_FAILURE));
    assert!(has_diagnostic(
        &report(&depth_report),
        "cem.transform_template.import_depth"
    ));

    let recursion = cem_ml(&transform_args(
        &data,
        &recursive_page,
        &recursion_report,
        &[],
    ));
    assert_eq!(recursion.status.code(), Some(EXIT_HARD_FAILURE));
    assert!(has_diagnostic(
        &report(&recursion_report),
        "cem.transform_template.recursion_limit"
    ));
}
