use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn direct_cli_executes_xpath_over_native_xml_and_exports_registered_result_json() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("cem-ml-cli-xpath-transform-{nanos}"));
    fs::create_dir_all(&root).expect("create XPath transform fixture directory");
    let data = root.join("catalog.xml");
    let template = root.join("books.xpath");
    fs::write(&data, "<catalog><book id=\"a\"/><book id=\"b\"/></catalog>")
        .expect("write XPath transform XML fixture");
    fs::write(&template, "/catalog/book").expect("write XPath transform template fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .args([
            "transform",
            data.to_str().expect("data path is UTF-8"),
            "--data-content-type",
            "application/xml",
            "--template",
            template.to_str().expect("template path is UTF-8"),
            "--template-content-type",
            "application/vnd.cem.xpath",
            "--to-content-type",
            "application/vnd.cem.xpath-result+json",
        ])
        .output()
        .expect("run cem-ml XPath transform");

    assert!(
        output.status.success(),
        "XPath transform stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "XPath transform stderr must stay empty: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout)
        .expect("XPath transform stdout is explicit result JSON");
    let items = result["sequence"]["items"]
        .as_array()
        .expect("XPath result item sequence");
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|item| item["kind"] == "node"));
}
