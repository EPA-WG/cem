use std::fs;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

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
    Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .arg("query")
        .arg(data)
        .args(["--content-type", "application/xml"])
        .args(args)
        .output()
        .expect("run cem-ml query")
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
