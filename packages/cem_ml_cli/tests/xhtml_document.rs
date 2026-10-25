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

fn definition() -> serde_json::Value {
    serde_json::from_slice(
        &fs::read(workspace().join("examples/projects/xhtml-document/example.json")).unwrap(),
    )
    .unwrap()
}

fn resource_path(root: &Path, definition: &serde_json::Value, id: &str) -> PathBuf {
    let source = definition["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|source| source["id"] == id)
        .unwrap();
    root.join(source["path"].as_str().unwrap())
}

fn project() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "cem-xhtml-project-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    for source in definition()["sources"].as_array().unwrap() {
        let path = root.join(source["path"].as_str().unwrap());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::copy(workspace().join(source["source"].as_str().unwrap()), path).unwrap();
    }
    root
}

fn run(root: &Path, id: &str) -> Output {
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
                    resource_path(root, &definition, id)
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

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn outline(root: &Path) -> serde_json::Value {
    let output = run(root, "query-outline");
    success(&output);
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["language"], "xpath");
    assert_eq!(
        report["result"]["staticContext"]["namespaces"]["xhtml"],
        "http://www.w3.org/1999/xhtml"
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
fn xhtml_project_authored_inspection_validation_query_and_build_keep_their_identities() {
    let root = project();
    let definition = definition();
    for configuration in definition["configurations"].as_array().unwrap() {
        let id = configuration["id"].as_str().unwrap();
        let output = run(&root, id);
        assert_eq!(
            output.status.code(),
            Some(configuration["expectedExit"].as_i64().unwrap_or(0) as i32),
            "{id}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if let Some(expected) = configuration["expectedText"].as_array() {
            let text = String::from_utf8(output.stdout).unwrap();
            assert!(!text.contains('\u{1b}'));
            for expected in expected {
                assert!(text.contains(expected.as_str().unwrap()), "{id}: {text}");
            }
        } else if let Some(expected_id) = configuration["expectedSemanticContract"].as_str() {
            let expected: serde_json::Value = serde_json::from_slice(
                &fs::read(resource_path(&root, &definition, expected_id)).unwrap(),
            )
            .unwrap();
            assert_eq!(outline(&root), expected);
        } else if let Some(outputs) = configuration["expectedGraphOutputs"].as_object() {
            assert_eq!(configuration["entryArtifact"], "generated/article.html");
            for (path, expected_id) in outputs {
                assert_eq!(
                    fs::read(root.join(path)).unwrap(),
                    fs::read(resource_path(
                        &root,
                        &definition,
                        expected_id.as_str().unwrap()
                    ))
                    .unwrap(),
                    "graph output bytes match the authored expected artifact"
                );
            }
        } else {
            let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            let expected = &configuration["expectedValidation"];
            assert_eq!(report["summary"]["errorCount"], expected["errorCount"]);
            assert_eq!(report["summary"]["warningCount"], expected["warningCount"]);
            assert!(report["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .all(|diagnostic| diagnostic["code"] == expected["errorCode"]));
        }
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn xhtml_outline_uses_expanded_names_while_preserving_foreign_svg_content() {
    let root = project();
    let path = root.join("data/article.xhtml");
    let original = fs::read_to_string(&path).unwrap();
    let expected: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("expected/outline.json")).unwrap()).unwrap();
    assert_eq!(outline(&root), expected);

    let foreign_start = original.find("      <svg ").unwrap();
    let foreign_end = original.find("      </svg>").unwrap() + "      </svg>".len();
    let prefix = |source: &str| {
        let mut result = source.replace(
            "xmlns=\"http://www.w3.org/1999/xhtml\"",
            "xmlns:h=\"http://www.w3.org/1999/xhtml\" xmlns:xhtml=\"urn:foreign\"",
        );
        for name in ["html", "head", "title", "body", "main", "h1", "p", "a"] {
            result = result
                .replace(&format!("<{name}"), &format!("<h:{name}"))
                .replace(&format!("</{name}>"), &format!("</h:{name}>"));
        }
        result
    };
    let prefixed = format!(
        "{}{}{}",
        prefix(&original[..foreign_start]),
        &original[foreign_start..foreign_end],
        prefix(&original[foreign_end..])
    );
    assert!(prefixed.contains(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" role=\"img\" aria-label=\"Namespace example\">"
    ));
    assert!(prefixed.contains("<title>A foreign title is not the document title</title>"));
    fs::write(&path, prefixed).unwrap();
    success(&run(&root, "validate-xhtml"));
    assert_eq!(outline(&root), expected);
    success(&run(&root, "build-html"));
    assert_eq!(
        fs::read(root.join("generated/article.html")).unwrap(),
        fs::read(root.join("expected/article.html")).unwrap()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn xhtml_namespace_failure_retains_the_original_resource_and_repairs_exactly() {
    let root = project();
    let path = root.join("data/missing-namespace.xhtml");
    let original = fs::read_to_string(&path).unwrap();
    let output = run(&root, "repair-namespace");
    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["summary"]["errorCount"], 1);
    let diagnostic = &report["diagnostics"][0];
    assert_eq!(diagnostic["code"], "cem.xhtml.namespace_missing");
    assert_eq!(diagnostic["uri"], path.to_str().unwrap());
    assert_eq!(diagnostic["byteOffset"], original.find("<html").unwrap());
    assert_eq!(diagnostic["line"], 2);
    assert_eq!(diagnostic["column"], 1);

    let definition = definition();
    let repair = &definition["configurations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|configuration| configuration["id"] == "repair-namespace")
        .unwrap()["repair"];
    let repaired = original.replacen(
        repair["find"].as_str().unwrap(),
        repair["replace"].as_str().unwrap(),
        1,
    );
    assert_eq!(
        repaired,
        fs::read_to_string(root.join("data/article.xhtml")).unwrap()
    );
    fs::write(&path, repaired).unwrap();
    let output = run(&root, "repair-namespace");
    success(&output);
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["summary"]["errorCount"], 0);
    assert_eq!(report["summary"]["warningCount"], 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn xhtml_html_publication_preserves_template_provenance_and_isolated_saved_edits() {
    let root = project();
    let other = project();
    let original = fs::read_to_string(root.join("data/article.xhtml")).unwrap();
    let expected = fs::read_to_string(root.join("expected/article.html")).unwrap();
    success(&run(&root, "build-html"));
    success(&run(&other, "build-html"));
    assert_eq!(
        fs::read_to_string(root.join("generated/article.html")).unwrap(),
        expected
    );
    let map: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("generated/article.html.map")).unwrap())
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

    let edit = |source: &str| {
        source
            .replace("An XHTML field guide", "An edited field guide")
            .replace("Review before publishing.", "Review the saved article.")
    };
    fs::write(root.join("data/article.xhtml"), edit(&original)).unwrap();
    success(&run(&root, "build-html"));
    assert_eq!(
        fs::read_to_string(root.join("generated/article.html")).unwrap(),
        edit(&expected)
    );
    assert_eq!(outline(&root)[1], "heading=An edited field guide");
    assert_eq!(
        fs::read_to_string(other.join("generated/article.html")).unwrap(),
        expected
    );
    assert_eq!(
        fs::read_to_string(workspace().join("examples/projects/xhtml-document/data/article.xhtml"))
            .unwrap(),
        original
    );
    for root in [root, other] {
        fs::remove_dir_all(root).unwrap();
    }
}
