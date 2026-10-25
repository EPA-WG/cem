//! Ordinary Markdown resources use the same native commands in Studio and CLI.
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
        &fs::read(workspace().join("examples/projects/markdown-site/example.json")).unwrap(),
    )
    .unwrap()
}

struct Project {
    root: PathBuf,
    definition: serde_json::Value,
}

impl Project {
    fn new() -> Self {
        let definition = definition();
        let root = std::env::temp_dir().join(format!(
            "cem-markdown-article-project-{}-{}",
            std::process::id(),
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
        Self { root, definition }
    }

    fn path(&self, id: &str) -> PathBuf {
        let source = self.definition["sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|source| source["id"] == id)
            .unwrap_or_else(|| panic!("missing authored resource {id}"));
        self.root.join(source["path"].as_str().unwrap())
    }

    fn configuration(&self, id: &str) -> &serde_json::Value {
        self.definition["configurations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|configuration| configuration["id"] == id)
            .unwrap_or_else(|| panic!("missing authored configuration {id}"))
    }

    fn run(&self, id: &str) -> Output {
        let configuration = self.configuration(id);
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
                    .map(|id| self.path(id).to_str().unwrap().to_owned())
                    .unwrap_or_else(|| argument.to_owned())
            })
            .collect();
        Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .args(arguments)
            .current_dir(&self.root)
            .output()
            .unwrap()
    }

    fn json(&self, id: &str) -> serde_json::Value {
        let output = self.run(id);
        success(&output);
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn xpath_values(report: &serde_json::Value) -> serde_json::Value {
    assert_eq!(report["language"], "xpath");
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
fn markdown_article_authored_configurations_preserve_native_results_and_exact_artifacts() {
    let project = Project::new();
    assert_eq!(project.definition["version"], "1.1.0");
    assert_eq!(
        project.definition["configurations"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
    for configuration in project.definition["configurations"].as_array().unwrap() {
        let id = configuration["id"].as_str().unwrap();
        let output = project.run(id);
        assert_eq!(
            output.status.code(),
            Some(configuration["expectedExit"].as_i64().unwrap_or(0) as i32),
            "{id}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if let Some(expected_id) = configuration["expectedOutput"].as_str() {
            assert_eq!(
                output.stdout,
                fs::read(project.path(expected_id)).unwrap(),
                "{id}"
            );
            assert!(!String::from_utf8_lossy(&output.stdout).contains("title:"));
        }
        if let Some(expected_id) = configuration["expectedSemanticContract"].as_str() {
            let expected: serde_json::Value =
                serde_json::from_slice(&fs::read(project.path(expected_id)).unwrap()).unwrap();
            let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(xpath_values(&report), expected);
        }
        if let Some(expected) = configuration["expectedText"].as_array() {
            let text = String::from_utf8_lossy(&output.stdout);
            assert!(!text.contains('\u{1b}'));
            for marker in expected {
                assert!(text.contains(marker.as_str().unwrap()), "{id}: {text}");
            }
        }
        if let Some(expected) = configuration["expectedValidation"].as_object() {
            let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(report["summary"]["errorCount"], expected["errorCount"]);
            assert_eq!(report["summary"]["warningCount"], expected["warningCount"]);
            assert_eq!(report["summary"]["fatalCount"], 0);
        }
        if let Some(outputs) = configuration["expectedGraphOutputs"].as_object() {
            assert_eq!(
                outputs.len(),
                7,
                "the existing four-page site and three assets remain intact"
            );
            for (path, expected_id) in outputs {
                assert_eq!(
                    fs::read(project.root.join(path)).unwrap(),
                    fs::read(project.path(expected_id.as_str().unwrap())).unwrap(),
                    "{path}"
                );
            }
        }
    }
}

#[test]
fn markdown_article_outline_queries_the_generated_html_without_exposing_front_matter() {
    let project = Project::new();
    let original = fs::read_to_string(project.path("content-articles-first-build-md")).unwrap();
    let expected: serde_json::Value =
        serde_json::from_slice(&fs::read(project.path("expected-article-outline")).unwrap())
            .unwrap();
    let report = project.json("query-article-outline");
    assert_eq!(xpath_values(&report), expected);
    assert!(report["result"]["staticContext"]["namespaces"]
        .as_object()
        .unwrap()
        .values()
        .any(|value| value == "http://www.w3.org/1999/xhtml"));

    // Metadata title belongs to the layout input; it does not become a body heading.
    fs::write(
        project.path("content-articles-first-build-md"),
        original.replacen(
            "title: Build a small site",
            "title: A separate publishing title",
            1,
        ),
    )
    .unwrap();
    assert_eq!(
        xpath_values(&project.json("query-article-outline")),
        expected
    );
    let preview = project.run("preview-article");
    success(&preview);
    let html = String::from_utf8(preview.stdout).unwrap();
    assert!(html.contains("Build a small site"));
    assert!(!html.contains("A separate publishing title"));
    assert!(!html.contains("title:"));
}

#[test]
fn markdown_metadata_failure_identifies_original_bytes_and_repairs_the_same_resource() {
    let project = Project::new();
    let configuration = project.configuration("repair-article-metadata");
    let path = project.path("invalid-article-metadata");
    assert!(
        !path.starts_with(project.root.join("content")),
        "failure examples must not join the site's content glob"
    );
    let original = fs::read_to_string(&path).unwrap();
    let output = project.run("repair-article-metadata");
    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["summary"]["errorCount"], 1);
    assert_eq!(report["summary"]["warningCount"], 0);
    let diagnostic = report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "cem.markdown.front_matter_invalid")
        .unwrap();
    assert_eq!(diagnostic["uri"], path.to_str().unwrap());
    assert_eq!(diagnostic["severity"], "error");
    assert_eq!(diagnostic["line"], 2);
    assert_eq!(diagnostic["column"], 1);
    assert_eq!(diagnostic["byteOffset"], original.find('\n').unwrap() + 1);
    assert!(diagnostic["message"].as_str().unwrap().contains("weight"));

    let repair = &configuration["repair"];
    let repaired = original.replacen(
        repair["find"].as_str().unwrap(),
        repair["replace"].as_str().unwrap(),
        1,
    );
    assert_ne!(repaired, original);
    fs::write(&path, repaired).unwrap();
    let fixed = project.json("repair-article-metadata");
    assert_eq!(fixed["summary"]["errorCount"], 0);
    assert_eq!(fixed["summary"]["warningCount"], 0);
    // A separately packaged failure does not prevent normal site publication.
    success(&project.run("build-site"));
}

#[test]
fn markdown_site_copies_keep_saved_metadata_body_and_static_assets_independent() {
    let project = Project::new();
    let other = Project::new();
    let source_id = "content-articles-first-build-md";
    let original = fs::read_to_string(project.path(source_id)).unwrap();
    let changed = original.replacen(
        "title: Build a small site",
        "title: Saved publishing title",
        1,
    );
    assert_ne!(changed, original);
    fs::write(project.path(source_id), &changed).unwrap();
    success(&project.run("build-site"));
    success(&other.run("build-site"));
    let path = "generated/articles/first-build.html";
    let html = fs::read_to_string(project.root.join(path)).unwrap();
    assert!(html.contains("<title>Saved publishing title</title>"));
    assert!(html.contains("<h1 id=\"build-a-small-site\" tabindex=\"-1\">Build a small site</h1>"));
    assert_eq!(
        fs::read(other.root.join(path)).unwrap(),
        fs::read(other.path("expected-articles-first-build-html")).unwrap()
    );
    assert_eq!(fs::read_to_string(other.path(source_id)).unwrap(), original);
    let repo_source =
        workspace().join("examples/projects/markdown-site/content/articles/first-build.md");
    assert_eq!(fs::read_to_string(repo_source).unwrap(), original);
    for (output, input) in [
        ("generated/assets/site.css", "assets-site-css"),
        ("generated/images/harbor.svg", "static-images-harbor-svg"),
        (
            "generated/downloads/notes.txt",
            "static-downloads-notes-txt",
        ),
    ] {
        assert_eq!(
            fs::read(project.root.join(output)).unwrap(),
            fs::read(project.path(input)).unwrap()
        );
        assert_eq!(
            fs::read(project.root.join(output)).unwrap(),
            fs::read(other.root.join(output)).unwrap()
        );
    }
    let first_build = fs::read(project.root.join(path)).unwrap();
    success(&project.run("build-site"));
    assert_eq!(fs::read(project.root.join(path)).unwrap(), first_build);
    assert_eq!(
        fs::read_to_string(project.path(source_id)).unwrap(),
        changed
    );
}
