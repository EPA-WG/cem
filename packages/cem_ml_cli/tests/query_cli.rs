use std::fs;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const CSS_SELECTOR_CONTENT_TYPE: &str = "application/vnd.cem.query-expression+css-selector";
const CSS_SELECTOR_SCHEMA: &str = "https://cem.dev/ns/query/css-selector/1";
const CEM_QL_CONTENT_TYPE: &str = "application/vnd.cem.query-expression+cem-ql";
const CEM_QL_SCHEMA: &str = "https://cem.dev/ns/query/cem-ql/1#expression";
const XPATH_CONTENT_TYPE: &str = "application/vnd.cem.xpath";
const XPATH_SCHEMA: &str = "https://cem.dev/ns/query/xpath/1";

fn query_fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/query")
        .join(name)
}

fn workspace_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("CLI crate is nested under workspace packages")
        .join(relative)
}

fn query_data() -> (std::path::PathBuf, std::path::PathBuf) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("cem-ml-cli-query-{nanos}"));
    fs::create_dir_all(&root).expect("create query test directory");
    let data = root.join("catalog.xml");
    fs::write(
        &data,
        "<catalog><book id=\"a\"/><book id=\"b\"/><magazine/></catalog>",
    )
    .expect("write query XML data");
    (root, data)
}

fn run_query(data: &std::path::Path, args: &[&str]) -> Output {
    run_query_with_data_type(data, "application/xml", args)
}

fn run_query_with_data_type(
    data: &std::path::Path,
    data_content_type: &str,
    args: &[&str],
) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .arg("query")
        .arg(data)
        .args(["--content-type", data_content_type])
        .args(args)
        .output()
        .expect("run cem-ml query")
}

fn diagnostic_codes(report: &serde_json::Value) -> Vec<&str> {
    report["diagnostics"]
        .as_array()
        .expect("report diagnostics")
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .collect()
}

fn json_object_keys(value: &serde_json::Value) -> Vec<String> {
    let mut keys = value
        .as_object()
        .expect("JSON object")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    keys
}

fn assert_success(output: &Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "query stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "successful query stderr must stay empty: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("query JSON output")
}

#[test]
fn query_markdown_path_uses_its_implicitly_loaded_html_dom() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("cem-ml-cli-markdown-query-{nanos}"));
    fs::create_dir_all(&root).expect("create Markdown query test directory");
    let data = root.join("guide.md");
    fs::write(&data, "# Guide\n\nRead the **documentation**.\n")
        .expect("write Markdown query data");

    let output = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .arg("query")
        .arg(&data)
        .args([
            "--query",
            "h1",
            "--query-content-type",
            CSS_SELECTOR_CONTENT_TYPE,
            "--query-schema",
            CSS_SELECTOR_SCHEMA,
            "--output",
            "json",
        ])
        .output()
        .expect("query implicitly loaded Markdown HTML DOM");
    let result = assert_success(&output);

    assert_eq!(result["language"], "css-selector");
    assert_eq!(result["matches"].as_array().map(Vec::len), Some(1));
    assert_eq!(result["matches"][0]["localName"], "h1");
}

#[test]
fn query_executes_inline_css_selector_through_explicit_identity() {
    let (_root, data) = query_data();
    let result = assert_success(&run_query(
        &data,
        &[
            "--query",
            "book",
            "--query-content-type",
            "application/vnd.cem.query-expression+css-selector",
            "--query-schema",
            "https://cem.dev/ns/query/css-selector/1",
            "--output",
            "json",
        ],
    ));
    assert_eq!(result["language"], "css-selector");
    assert_eq!(result["matches"].as_array().map(Vec::len), Some(2));
}

#[test]
fn query_executes_file_backed_xpath_through_explicit_identity() {
    let (root, data) = query_data();
    let query = root.join("books.xpath");
    fs::write(&query, "/catalog/book").expect("write XPath query");
    let result = assert_success(&run_query(
        &data,
        &[
            "--query-file",
            query.to_str().expect("query path is UTF-8"),
            "--query-content-type",
            "application/vnd.cem.xpath",
            "--query-schema",
            "https://cem.dev/ns/query/xpath/1",
            "--output",
            "json",
        ],
    ));
    assert_eq!(result["language"], "xpath");
    assert_eq!(
        result["result"]["sequence"]["items"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
}

#[test]
fn query_executes_inline_cem_ql_without_text_shape_detection() {
    let (_root, data) = query_data();
    let result = assert_success(&run_query(
        &data,
        &[
            "--query",
            "input",
            "--query-content-type",
            "application/vnd.cem.query-expression+cem-ql",
            "--query-schema",
            "https://cem.dev/ns/query/cem-ql/1#expression",
            "--output",
            "json",
        ],
    ));
    assert_eq!(result["language"], "cem-ql");
    assert_eq!(result["result"]["items"].as_array().map(Vec::len), Some(1));
}

#[test]
fn query_requires_exactly_one_source_and_an_explicit_content_type() {
    let (root, data) = query_data();
    let query = root.join("books.xpath");
    fs::write(&query, "/catalog/book").expect("write XPath query");

    let missing_source = run_query(
        &data,
        &["--query-content-type", "application/vnd.cem.xpath"],
    );
    assert!(!missing_source.status.success());

    let duplicate_source = run_query(
        &data,
        &[
            "--query",
            "/catalog/book",
            "--query-file",
            query.to_str().expect("query path is UTF-8"),
            "--query-content-type",
            "application/vnd.cem.xpath",
        ],
    );
    assert!(!duplicate_source.status.success());

    let missing_identity = run_query(&data, &["--query", "book"]);
    assert!(!missing_identity.status.success());
}

#[test]
fn query_exports_terminal_and_cem_only_when_requested() {
    let (_root, data) = query_data();
    let terminal = run_query(
        &data,
        &[
            "--query",
            "book",
            "--query-content-type",
            "application/vnd.cem.query-expression+css-selector",
        ],
    );
    assert!(
        terminal.status.success(),
        "{}",
        String::from_utf8_lossy(&terminal.stderr)
    );
    let terminal = String::from_utf8(terminal.stdout).expect("terminal output is UTF-8");
    assert!(
        terminal.starts_with("CSS selector: 2 matches\n"),
        "{terminal}"
    );
    assert!(!terminal.trim_start().starts_with('{'));

    let cem = run_query(
        &data,
        &[
            "--query",
            "book",
            "--query-content-type",
            "application/vnd.cem.query-expression+css-selector",
            "--output",
            "cem",
        ],
    );
    assert!(
        cem.status.success(),
        "{}",
        String::from_utf8_lossy(&cem.stderr)
    );
    let cem = String::from_utf8(cem.stdout).expect("CEM output is UTF-8");
    assert!(
        cem.starts_with("{query-result @language=\"css-selector\" @count=2 |"),
        "{cem}"
    );
    assert!(cem.contains("{match @node-id=\"xml:event:1\""), "{cem}");
}

#[test]
fn query_applies_common_result_budget_and_writes_report() {
    let (root, data) = query_data();
    let report = root.join("query-report.json");
    let output = run_query(
        &data,
        &[
            "--query",
            "book",
            "--query-content-type",
            "application/vnd.cem.query-expression+css-selector",
            "--output",
            "json",
            "--scope-budget",
            "queryItems=1",
            "--report-json",
            report.to_str().expect("report path is UTF-8"),
        ],
    );
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(report).expect("query report must be written on evaluation failure"),
    )
    .expect("query report JSON");
    assert_eq!(report["summary"]["hardViolationCount"], 1);
    assert!(report["diagnostics"][0]["code"]
        .as_str()
        .is_some_and(|code| code.contains("budget")));
}

#[test]
fn query_fixture_runs_all_languages_with_the_same_native_nodes_and_report_shape() {
    let data = query_fixture("catalog.xml");
    let (temp, _) = query_data();
    let cases = [
        (
            "css-selector",
            "catalog.css-selector",
            CSS_SELECTOR_CONTENT_TYPE,
            CSS_SELECTOR_SCHEMA,
        ),
        (
            "cem-ql",
            "catalog.cem-ql",
            CEM_QL_CONTENT_TYPE,
            CEM_QL_SCHEMA,
        ),
        ("xpath", "catalog.xpath", XPATH_CONTENT_TYPE, XPATH_SCHEMA),
    ];
    let mut sources_by_language = std::collections::BTreeMap::new();
    let mut report_shapes = Vec::new();

    for (language, query_name, content_type, schema) in cases {
        let query = query_fixture(query_name);
        let report = temp.join(format!("{language}.report.json"));
        let output = run_query(
            &data,
            &[
                "--query-file",
                query.to_str().expect("query fixture path is UTF-8"),
                "--query-content-type",
                content_type,
                "--query-schema",
                schema,
                "--namespace",
                "cat=urn:cem:query-catalog",
                "--output",
                "json",
                "--report-json",
                report.to_str().expect("report path is UTF-8"),
            ],
        );
        let result = assert_success(&output);
        let items = match language {
            "css-selector" => result["matches"].as_array().expect("CSS matches"),
            "cem-ql" => result["result"]["items"].as_array().expect("CEM-QL items"),
            "xpath" => result["result"]["sequence"]["items"]
                .as_array()
                .expect("XPath items"),
            _ => unreachable!(),
        };
        let ids = items
            .iter()
            .map(|item| {
                item.get("nodeId")
                    .or_else(|| item.get("identity"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_else(|| panic!("{language} item retains native identity: {item}"))
                    .to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(ids.len(), 2, "{language} selected two catalog books");
        assert!(
            items.iter().all(|item| item["sourceMap"]["frames"]
                .as_array()
                .is_some_and(|frames| !frames.is_empty())),
            "{language} items retain source maps: {items:#?}"
        );
        sources_by_language.insert(
            language,
            items
                .iter()
                .map(|item| item["sourceMap"].clone())
                .collect::<Vec<_>>(),
        );

        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(&report).expect("query report fixture output"))
                .expect("query report JSON");
        assert_eq!(report["summary"]["inputCount"], 2);
        assert_eq!(report["summary"]["hardViolationCount"], 0);
        report_shapes.push((
            json_object_keys(&report),
            json_object_keys(&report["summary"]),
        ));
    }

    assert_eq!(
        sources_by_language["css-selector"],
        sources_by_language["cem-ql"]
    );
    assert_eq!(
        sources_by_language["css-selector"],
        sources_by_language["xpath"]
    );
    assert!(report_shapes.windows(2).all(|pair| pair[0] == pair[1]));
}

#[test]
fn query_negative_fixtures_cover_identity_input_context_resolver_budget_and_exporter_errors() {
    let data = query_fixture("catalog.xml");
    let (temp, _) = query_data();

    let mismatch = run_query(
        &data,
        &[
            "--query-file",
            query_fixture("catalog.css-selector")
                .to_str()
                .expect("query fixture path is UTF-8"),
            "--query-content-type",
            CSS_SELECTOR_CONTENT_TYPE,
            "--query-schema",
            XPATH_SCHEMA,
        ],
    );
    assert!(!mismatch.status.success());
    assert!(String::from_utf8_lossy(&mismatch.stderr).contains("did not match"));

    let unsupported = run_query_with_data_type(
        &query_fixture("unsupported-data.json"),
        "application/json",
        &[
            "--query-file",
            query_fixture("catalog.css-selector")
                .to_str()
                .expect("query fixture path is UTF-8"),
            "--query-content-type",
            CSS_SELECTOR_CONTENT_TYPE,
            "--report-json",
            temp.join("unsupported.report.json")
                .to_str()
                .expect("report path is UTF-8"),
        ],
    );
    assert!(!unsupported.status.success());
    let unsupported_report: serde_json::Value = serde_json::from_slice(
        &fs::read(temp.join("unsupported.report.json")).expect("unsupported input report"),
    )
    .expect("unsupported input report JSON");
    assert!(diagnostic_codes(&unsupported_report)
        .iter()
        .any(|code| code.contains("input_unsupported")));

    for (language, file, content_type, schema, expected_code) in [
        (
            "css-selector",
            "invalid.css-selector",
            CSS_SELECTOR_CONTENT_TYPE,
            CSS_SELECTOR_SCHEMA,
            "css-selector.parse.invalid",
        ),
        (
            "cem-ql",
            "invalid.cem-ql",
            CEM_QL_CONTENT_TYPE,
            CEM_QL_SCHEMA,
            "cem.ql.parse_error",
        ),
        (
            "xpath",
            "invalid.xpath",
            XPATH_CONTENT_TYPE,
            XPATH_SCHEMA,
            "cem.xpath.parse_error",
        ),
    ] {
        let report = temp.join(format!("invalid-{language}.report.json"));
        let output = run_query(
            &data,
            &[
                "--query-file",
                query_fixture(file)
                    .to_str()
                    .expect("query fixture path is UTF-8"),
                "--query-content-type",
                content_type,
                "--query-schema",
                schema,
                "--report-json",
                report.to_str().expect("report path is UTF-8"),
            ],
        );
        assert!(
            !output.status.success(),
            "{language} invalid query must fail"
        );
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(report).expect("invalid query report"))
                .expect("invalid query report JSON");
        assert!(
            diagnostic_codes(&report).contains(&expected_code),
            "{language} expected {expected_code}: {report:#}"
        );
    }

    let missing_context_report = temp.join("missing-context.report.json");
    let missing_context = run_query(
        &data,
        &[
            "--query-file",
            query_fixture("catalog.css-selector")
                .to_str()
                .expect("query fixture path is UTF-8"),
            "--query-content-type",
            CSS_SELECTOR_CONTENT_TYPE,
            "--report-json",
            missing_context_report
                .to_str()
                .expect("report path is UTF-8"),
        ],
    );
    assert!(!missing_context.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(missing_context_report).expect("missing context report"))
            .expect("missing context report JSON");
    assert!(diagnostic_codes(&report).contains(&"css-selector.namespace.unbound"));

    let resolver_report = temp.join("resolver-denied.report.json");
    let resolver_query = workspace_path(
        "packages/cem_ml/schema-packages/xpath/v1/examples/external-resource-denied.xpath",
    );
    let resolver_denied = run_query(
        &data,
        &[
            "--query-file",
            resolver_query
                .to_str()
                .expect("resolver query path is UTF-8"),
            "--query-content-type",
            XPATH_CONTENT_TYPE,
            "--query-schema",
            XPATH_SCHEMA,
            "--report-json",
            resolver_report.to_str().expect("report path is UTF-8"),
        ],
    );
    assert!(!resolver_denied.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(resolver_report).expect("resolver denial report"))
            .expect("resolver denial report JSON");
    assert!(diagnostic_codes(&report).contains(&"cem.xpath.external_resource_denied"));

    let exhausted = run_query(
        &data,
        &[
            "--query-file",
            query_fixture("catalog.css-selector")
                .to_str()
                .expect("query fixture path is UTF-8"),
            "--query-content-type",
            CSS_SELECTOR_CONTENT_TYPE,
            "--namespace",
            "cat=urn:cem:query-catalog",
            "--scope-budget",
            "queryItems=1",
        ],
    );
    assert!(!exhausted.status.success());
    assert!(String::from_utf8_lossy(&exhausted.stderr).contains("budget"));

    let unavailable_exporter = run_query(
        &data,
        &[
            "--query-file",
            query_fixture("catalog.xpath")
                .to_str()
                .expect("query fixture path is UTF-8"),
            "--query-content-type",
            XPATH_CONTENT_TYPE,
            "--output",
            "xml",
        ],
    );
    assert!(!unavailable_exporter.status.success());
    assert!(String::from_utf8_lossy(&unavailable_exporter.stderr).contains("invalid value"));
}

#[test]
fn query_url_matrix_matches_native_results_and_preserves_reports() {
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../cem_ql/fixtures/url/query-matrix.json")).unwrap();
    let (temp, data) = query_data();
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let module = case["sourceKind"] == "module";
        let source = if module {
            format!(
                "module \"urn:test:url:{id}\"\n{}",
                case["query"].as_str().unwrap()
            )
        } else {
            case["query"].as_str().unwrap().to_owned()
        };
        let (content_type, schema) = if module {
            (
                "application/vnd.cem.query+cem-ql",
                "https://cem.dev/ns/query/cem-ql/1",
            )
        } else {
            (CEM_QL_CONTENT_TYPE, CEM_QL_SCHEMA)
        };
        let report_path = temp.join(format!("{id}.report.json"));
        let output = run_query(
            &data,
            &[
                "--query",
                &source,
                "--query-content-type",
                content_type,
                "--query-schema",
                schema,
                "--output",
                "json",
                "--report-json",
                report_path.to_str().unwrap(),
            ],
        );
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
        let codes = diagnostic_codes(&report);
        let mut expected_codes = case["diagnosticCodes"].as_array().unwrap().clone();
        if case["error"] == true {
            expected_codes.push(serde_json::json!("cem.ql.query_evaluation_failed"));
        }
        assert_eq!(
            serde_json::json!(codes),
            serde_json::json!(expected_codes),
            "{id}: {report}"
        );
        if case["error"] == true {
            assert!(!output.status.success(), "{id}");
            assert!(output.stdout.is_empty(), "{id}");
        } else {
            assert!(
                output.status.success(),
                "{id}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(result["result"]["items"], case["items"], "{id}");
            assert_eq!(result["result"]["error"], serde_json::Value::Null, "{id}");
        }
        for diagnostic in report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["code"] != "cem.ql.query_evaluation_failed")
        {
            assert!(
                diagnostic["sourceMap"]["frames"]
                    .as_array()
                    .is_some_and(|frames| !frames.is_empty()),
                "{id}: {diagnostic}"
            );
        }
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn query_url_setter_warnings_reach_the_cli_report() {
    let (temp, data) = query_data();
    let report = temp.join("warnings.json");
    let output = run_query(
        &data,
        &[
            "--query",
            r#"url:with_parts("https://h:8443/a", {protocol: "mailto:", host: "other:70000"})"#,
            "--query-content-type",
            CEM_QL_CONTENT_TYPE,
            "--query-schema",
            CEM_QL_SCHEMA,
            "--output",
            "json",
            "--report-json",
            report.to_str().unwrap(),
        ],
    );
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
    assert_eq!(
        diagnostic_codes(&report),
        vec!["cem.ql.url_setter_ignored", "cem.ql.url_setter_ignored"]
    );
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn query_url_modules_work_inline_and_from_files_without_changing_expression_mode() {
    let (temp, data) = query_data();
    let module_file = temp.join("links.cemql");
    let source = r#"module "urn:test:links"
import "cem:stdlib/url" as u
declare let base = "https://h/"
declare function local:link(path as string) { u:href(path, base) }
(local:link("next"), input)"#;
    fs::write(&module_file, source).unwrap();
    for media in ["application/vnd.cem.query+cem-ql", "text/cem-ql"] {
        for (flag, value) in [
            ("--query", source),
            ("--query-file", module_file.to_str().unwrap()),
        ] {
            let result = assert_success(&run_query(
                &data,
                &[
                    flag,
                    value,
                    "--query-content-type",
                    media,
                    "--output",
                    "json",
                ],
            ));
            assert_eq!(result["result"]["items"][0]["type"], "any-uri");
            assert_eq!(result["result"]["items"][0]["value"], "https://h/next");
            assert_eq!(result["result"]["items"].as_array().unwrap().len(), 2);
        }
    }
    let denied = run_query(
        &data,
        &[
            "--query",
            source,
            "--query-content-type",
            CEM_QL_CONTENT_TYPE,
            "--output",
            "json",
        ],
    );
    assert!(!denied.status.success());
    assert!(denied.stdout.is_empty());
    for (media, schema) in [
        (CEM_QL_CONTENT_TYPE, "https://cem.dev/ns/query/cem-ql/1"),
        ("application/vnd.cem.query+cem-ql", CEM_QL_SCHEMA),
    ] {
        let mismatch = run_query(
            &data,
            &[
                "--query",
                "1",
                "--query-content-type",
                media,
                "--query-schema",
                schema,
            ],
        );
        assert!(!mismatch.status.success());
        assert!(String::from_utf8_lossy(&mismatch.stderr).contains("did not match"));
    }
    fs::remove_dir_all(temp).unwrap();
}
