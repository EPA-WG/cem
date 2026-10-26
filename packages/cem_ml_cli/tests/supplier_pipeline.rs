use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

fn project() -> PathBuf {
    let definition: serde_json::Value = serde_json::from_slice(
        &fs::read(workspace().join("examples/projects/supplier-pipeline/example.json"))
            .expect("Supplier pipeline must be an ordinary packaged Studio project"),
    )
    .unwrap();
    let root = std::env::temp_dir().join(format!(
        "cem-supplier-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    for source in definition["sources"].as_array().unwrap() {
        let path = root.join(source["path"].as_str().unwrap());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::copy(workspace().join(source["source"].as_str().unwrap()), path).unwrap();
    }
    root
}

fn run(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .args([
            "transform",
            "--config",
            root.join("graphs/availability.cem").to_str().unwrap(),
            "--scope-budget",
            "queueSize=1024",
        ])
        .output()
        .unwrap()
}

fn success(output: Output) {
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn values(root: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(root.join("generated/availability.json")).unwrap()).unwrap()
}

fn definition() -> serde_json::Value {
    serde_json::from_slice(
        &fs::read(workspace().join("examples/projects/supplier-pipeline/example.json")).unwrap(),
    )
    .unwrap()
}

fn run_configuration(root: &Path, id: &str) -> Output {
    let definition = definition();
    let configuration = definition["configurations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|configuration| configuration["id"] == id)
        .unwrap();
    let arguments: Vec<_> = configuration["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|argument| {
            let argument = argument.as_str().unwrap();
            let resource_id = if argument == "$source" {
                configuration["source"].as_str()
            } else {
                argument.strip_prefix('$')
            };
            resource_id
                .map(|id| {
                    let source = definition["sources"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|source| source["id"] == id)
                        .unwrap();
                    root.join(source["path"].as_str().unwrap())
                        .to_str()
                        .unwrap()
                        .to_owned()
                })
                .unwrap_or_else(|| argument.to_owned())
        })
        .collect();
    Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .args(arguments)
        .output()
        .unwrap()
}

fn query_products(root: &Path) -> serde_json::Value {
    let output = run_configuration(root, "query-products");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["language"], "xpath");
    assert_eq!(
        report["result"]["staticContext"]["namespaces"]["supplier"],
        "urn:demo:supplier"
    );
    serde_json::Value::Array(
        report["result"]["sequence"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["value"]["lexicalValue"].clone())
            .collect(),
    )
}

#[test]
fn supplier_catalog_query_uses_expanded_names_independent_of_source_prefixes() {
    let root = project();
    let original = fs::read_to_string(root.join("data/catalog.xml")).unwrap();
    let expected: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("expected/products.json")).unwrap()).unwrap();
    assert_eq!(query_products(&root), expected);
    success(run_configuration(&root, "validate-catalog"));

    let prefixed = original
        .replace("<catalog xmlns=", "<p:catalog xmlns:p=")
        .replace("</catalog>", "</p:catalog>")
        .replace("<product", "<p:product")
        .replace("</product>", "</p:product>")
        .replace("<name>", "<p:name>")
        .replace("</name>", "</p:name>")
        .replace(
            "supplier=\"Canal Supplies\"",
            "xmlns:supplier=\"urn:foreign\" supplier=\"Canal Supplies\"",
        )
        .replace(
            "</p:catalog>",
            "  <supplier:product id=\"IGNORE\"><supplier:name>Foreign product</supplier:name></supplier:product>\n</p:catalog>",
        );
    fs::write(root.join("data/catalog.xml"), prefixed).unwrap();
    success(run_configuration(&root, "validate-catalog"));
    assert_eq!(query_products(&root), expected);
    success(run(&root));
    let expected_availability: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("expected/availability.json")).unwrap())
            .unwrap();
    assert_eq!(values(&root), expected_availability);
    assert_eq!(
        fs::read_to_string(
            workspace().join("examples/projects/supplier-pipeline/data/catalog.xml")
        )
        .unwrap(),
        original,
        "namespace edits belong only to the temporary project copy"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn supplier_catalog_validation_retains_original_xml_ranges_and_repairs() {
    let root = project();
    let path = root.join("data/catalog-invalid.xml");
    let invalid = fs::read_to_string(&path).unwrap();
    let output = run_configuration(&root, "repair-catalog");
    assert!(!output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["summary"]["errorCount"], 2);
    assert_eq!(report["summary"]["warningCount"], 0);
    let diagnostic = report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "cem.xml.parse_error")
        .expect("malformed XML retains its owning parser diagnostic");
    assert_eq!(diagnostic["uri"], path.to_str().unwrap());
    assert_eq!(
        diagnostic["byteOffset"],
        invalid.find("</product-broken>").unwrap(),
        "range is in the original UTF-8 XML, including the earlier Café text"
    );
    assert_eq!(diagnostic["line"], 6);
    assert_eq!(diagnostic["column"], 49);
    assert_eq!(diagnostic["details"]["xml"]["phase"], "parse");
    assert_eq!(
        diagnostic["details"]["xml"]["contentType"],
        "application/xml"
    );

    let definition = definition();
    let repair = &definition["configurations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|configuration| configuration["id"] == "repair-catalog")
        .unwrap()["repair"];
    let repaired = invalid.replacen(
        repair["find"].as_str().unwrap(),
        repair["replace"].as_str().unwrap(),
        1,
    );
    assert_eq!(
        repaired,
        fs::read_to_string(root.join("data/catalog.xml")).unwrap()
    );
    fs::write(&path, repaired).unwrap();
    let output = run_configuration(&root, "repair-catalog");
    assert!(output.status.success());
    let repaired_report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(repaired_report["summary"]["errorCount"], 0);
    assert_eq!(repaired_report["summary"]["warningCount"], 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn supplier_pipeline_reconciles_named_inputs_and_branches_to_three_formats() {
    let root = project();
    success(run(&root));
    let expected: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("expected/availability.json")).unwrap())
            .unwrap();
    assert_eq!(values(&root), expected);
    assert_eq!(expected[0]["quantity"], 7);
    assert_eq!(expected[0]["available"], true);
    assert_eq!(expected[1]["status"], "out-of-stock");
    assert_eq!(expected[2]["status"], "unknown");
    assert_eq!(expected[3]["status"], "ambiguous");
    for extension in ["yaml", "html"] {
        assert_eq!(
            fs::read_to_string(root.join(format!("generated/availability.{extension}")))
                .unwrap()
                .trim(),
            fs::read_to_string(root.join(format!("expected/availability.{extension}")))
                .unwrap()
                .trim()
        );
    }
    let yaml = root.join("generated/availability.yaml");
    let converted = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .args([
            "convert",
            yaml.to_str().unwrap(),
            "--content-type",
            "application/yaml",
            "--schema",
            "https://cem.dev/ns/data/yaml/1",
            "--to-content-type",
            "application/json",
            "--to-schema",
            "https://cem.dev/ns/data/json/1",
            "--output-color-type",
            "none",
        ])
        .output()
        .unwrap();
    assert!(
        converted.status.success(),
        "{}",
        String::from_utf8_lossy(&converted.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&converted.stdout).unwrap(),
        expected
    );
    // Computed values point to authored template instructions, without claiming cell-level lineage.
    let map: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("generated/availability.html.map")).unwrap())
            .unwrap();
    assert!(map["outputSpans"]
        .as_array()
        .unwrap()
        .iter()
        .any(|span| span["origin"]["frames"]
            .as_array()
            .unwrap()
            .iter()
            .any(|frame| frame["transform"]["uri"]
                .as_str()
                .is_some_and(|uri| uri.ends_with("templates/view.cemt")))));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn supplier_pipeline_matches_ids_independent_of_row_order_and_preserves_isolation() {
    let root = project();
    let other = project();
    let path = root.join("data/stock.csv");
    let stock = fs::read_to_string(&path).unwrap();
    let mut lines = stock.lines();
    let header = lines.next().unwrap();
    let rows: Vec<_> = lines.collect();
    fs::write(
        &path,
        format!(
            "{header}\n{}\n",
            rows.into_iter().rev().collect::<Vec<_>>().join("\n")
        ),
    )
    .unwrap();
    success(run(&root));
    success(run(&other));
    assert_eq!(values(&root), values(&other));
    let reordered = fs::read_to_string(&path).unwrap();
    fs::write(&path, reordered.replace("SKU-A,7", "SKU-A,0")).unwrap();
    success(run(&root));
    assert_eq!(values(&root)[0]["available"], false);
    assert_eq!(values(&root)[0]["quantity"], 0);
    assert_eq!(values(&other)[0]["quantity"], 7);
    fs::write(&path, stock.replace("SKU-A,7", "SKU-A,-2")).unwrap();
    success(run(&root));
    assert_eq!(values(&root)[0]["status"], "invalid");
    assert_eq!(values(&root)[0]["available"], false);
    for root in [root, other] {
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn supplier_pipeline_reports_invalid_quantity_without_publication_and_repairs() {
    let root = project();
    let path = root.join("data/stock.csv");
    let original = fs::read_to_string(&path).unwrap();
    fs::write(&path, original.replace("SKU-A,7", "SKU-A,many")).unwrap();
    let failure = run(&root);
    assert!(!failure.status.success());
    let diagnostic = String::from_utf8_lossy(&failure.stderr);
    assert!(diagnostic.contains("invalid data number"), "{diagnostic}");
    for extension in ["json", "yaml", "html"] {
        assert!(!root
            .join(format!("generated/availability.{extension}"))
            .exists());
    }
    fs::write(path, original).unwrap();
    success(run(&root));
    assert_eq!(values(&root)[0]["quantity"], 7);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn supplier_json_publication_matches_exact_shared_byte_fixture() {
    let root = project();
    success(run(&root));
    assert_eq!(fs::read(root.join("generated/availability.json")).unwrap(),
        fs::read(root.join("expected/availability.json")).unwrap());
    fs::remove_dir_all(root).unwrap();
}
