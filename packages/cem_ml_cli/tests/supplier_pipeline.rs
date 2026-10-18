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
