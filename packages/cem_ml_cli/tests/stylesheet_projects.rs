//! Authored Studio styles and selectors exercise native source/runtime contracts.
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

const PROJECTS: [&str; 3] = ["css-stylesheet", "css-selector", "scss-stylesheet"];
struct Project {
    root: PathBuf,
    definition: Value,
}
impl Project {
    fn new(id: &str) -> Self {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let definition: Value = serde_json::from_slice(
            &fs::read(workspace.join(format!("examples/projects/{id}/example.json"))).unwrap(),
        )
        .unwrap();
        let root = std::env::temp_dir().join(format!(
            "cem-styles-{id}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        for source in definition["sources"].as_array().unwrap() {
            let path = root.join(source["path"].as_str().unwrap());
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::copy(workspace.join(source["source"].as_str().unwrap()), path).unwrap();
        }
        Self { root, definition }
    }
    fn source(&self, id: &str) -> &Value {
        self.definition["sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == id)
            .unwrap()
    }
    fn path(&self, id: &str) -> PathBuf {
        self.root.join(self.source(id)["path"].as_str().unwrap())
    }
    fn configs(&self) -> &[Value] {
        self.definition["configurations"].as_array().unwrap()
    }
    fn run(&self, config: &Value) -> Output {
        self.run_with_module_grant(config, true)
    }
    fn run_with_module_grant(&self, config: &Value, admit_modules: bool) -> Output {
        let scss = self.definition["id"] == "scss-stylesheet";
        let mut args: Vec<String> = config["argv"]
            .as_array()
            .unwrap()
            .iter()
            .map(|arg| {
                let arg = arg.as_str().unwrap();
                let id = if arg == "$source" {
                    config["source"].as_str()
                } else {
                    arg.strip_prefix('$')
                };
                match id {
                    Some(id) if scss => reqwest::Url::from_file_path(self.path(id))
                        .unwrap()
                        .to_string(),
                    Some(id) => self.path(id).to_string_lossy().into_owned(),
                    None => arg.to_owned(),
                }
            })
            .collect();
        if scss && admit_modules {
            let prefix = reqwest::Url::from_directory_path(&self.root).unwrap();
            args.extend([
                "--resolver-read-map".into(),
                format!("{prefix}={}", self.root.display()),
            ]);
        }
        Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .unwrap()
    }
    fn graph(&self) -> Option<&Value> {
        self.configs()
            .iter()
            .find(|c| c["kind"] == "transformation-graph")
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
    assert!(
        !String::from_utf8_lossy(&output.stderr)
            .lines()
            .any(|line| line.contains("error:") || line.contains("fatal:")),
        "successful commands must not hide hard diagnostics: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn selection(report: &Value) -> Value {
    assert_eq!(report["language"], "css-selector");
    Value::Array(
        report["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| {
                json!({
                    "localName": item["localName"], "namespaceUri": item["namespaceUri"],
                    "documentOrder": item["documentOrder"], "nodeId": item["nodeId"],
                })
            })
            .collect(),
    )
}

#[test]
fn all_authored_stylesheet_and_selector_configurations_keep_exact_native_results() {
    for id in PROJECTS {
        let project = Project::new(id);
        assert_eq!(
            project.configs().len(),
            match id {
                "css-stylesheet" => 6,
                "css-selector" => 7,
                _ => 7,
            }
        );
        for config in project.configs() {
            let output = project.run(config);
            assert_eq!(
                output.status.code(),
                Some(config["expectedExit"].as_i64().unwrap_or(0) as i32),
                "{id}/{}: {} {}",
                config["id"],
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            if config["expectedExit"].as_i64().unwrap_or(0) == 0 {
                success(&output);
            }
            if let Some(expected) = config["expectedOutput"].as_str() {
                assert_eq!(
                    output.stdout,
                    fs::read(project.path(expected)).unwrap(),
                    "{id}/{}",
                    config["id"]
                );
            }
            for text in config["expectedText"].as_array().into_iter().flatten() {
                assert!(
                    String::from_utf8_lossy(&output.stdout).contains(text.as_str().unwrap()),
                    "{id}/{} missing {text}",
                    config["id"]
                );
            }
            if let Some(expected) = config["expectedSemanticContract"].as_str() {
                let report: Value = serde_json::from_slice(&output.stdout).unwrap();
                assert_eq!(
                    selection(&report),
                    serde_json::from_slice::<Value>(&fs::read(project.path(expected)).unwrap())
                        .unwrap()
                );
                let source =
                    fs::read_to_string(project.path(config["source"].as_str().unwrap())).unwrap();
                for item in report["matches"].as_array().unwrap() {
                    assert!(
                        item["sourceMap"]["frames"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|frame| {
                                let ranges = &frame["span"]["ranges"];
                                let Some(start) = ranges["start"].as_u64() else {
                                    return false;
                                };
                                let Some(len) = ranges["len"].as_u64() else {
                                    return false;
                                };
                                source
                                    .get(start as usize..(start + len) as usize)
                                    .is_some_and(|text| {
                                        text.starts_with('<') && text.len() < source.len()
                                    })
                            }),
                        "native match retains a bounded original element range"
                    );
                }
            }
            if let Some(expected) = config["expectedValidation"].as_object() {
                let report: Value = serde_json::from_slice(&output.stdout).unwrap();
                assert_eq!(report["summary"]["errorCount"], expected["errorCount"]);
                assert_eq!(report["summary"]["warningCount"], expected["warningCount"]);
                assert_eq!(report["summary"]["fatalCount"], 0);
            }
            if let Some(outputs) = config["expectedGraphOutputs"].as_object() {
                assert!(outputs.contains_key(config["entryArtifact"].as_str().unwrap()));
                for (path, expected) in outputs {
                    assert_eq!(
                        fs::read(project.root.join(path)).unwrap(),
                        fs::read(project.path(expected.as_str().unwrap())).unwrap(),
                        "{id}/{path}"
                    );
                }
            }
        }
    }
}

#[test]
fn native_failure_coordinates_and_repairs_keep_original_resources() {
    for id in PROJECTS {
        let project = Project::new(id);
        let config = project
            .configs()
            .iter()
            .find(|c| c["repair"].is_object())
            .unwrap();
        let path = project.path(config["source"].as_str().unwrap());
        let invalid = fs::read_to_string(&path).unwrap();
        let output = project.run(config);
        assert!(!output.status.success());
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        let expected = &config["expectedValidation"];
        let diagnostic = report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["code"] == expected["errorCode"])
            .unwrap();
        assert_eq!(diagnostic["severity"], "error");
        let uri = if id == "scss-stylesheet" {
            reqwest::Url::from_file_path(&path).unwrap().to_string()
        } else {
            path.to_string_lossy().into_owned()
        };
        assert_eq!(diagnostic["uri"], uri);
        for (key, value) in expected["location"].as_object().unwrap() {
            assert_eq!(&diagnostic[key], value, "{id}/{key}");
        }
        let repaired = invalid.replacen(
            config["repair"]["find"].as_str().unwrap(),
            config["repair"]["replace"].as_str().unwrap(),
            1,
        );
        assert_ne!(repaired, invalid);
        fs::write(&path, repaired).unwrap();
        let fixed = project.run(config);
        success(&fixed);
        assert_eq!(
            serde_json::from_slice::<Value>(&fixed.stdout).unwrap()["summary"]
                ["hardViolationCount"],
            0
        );
        assert_eq!(
            fs::read_to_string(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(
                    project.source(config["source"].as_str().unwrap())["source"]
                        .as_str()
                        .unwrap()
                )
            )
            .unwrap(),
            invalid,
            "repairing a saved copy must leave the catalog failure unchanged"
        );
    }
}

#[test]
fn stylesheet_builds_keep_owned_origins_edits_and_independent_outputs() {
    for (id, source_id, find, replace) in [
        (
            "css-stylesheet",
            "style-source",
            "rgb(24, 92, 128)",
            "rgb(128, 48, 24)",
        ),
        ("scss-stylesheet", "scss-tokens", "#036", "#630"),
    ] {
        let project = Project::new(id);
        let copy = Project::new(id);
        let graph = project.graph().unwrap();
        success(&project.run(graph));
        let outputs = graph["expectedGraphOutputs"].as_object().unwrap();
        let css_path = outputs.keys().find(|path| path.ends_with(".css")).unwrap();
        let baseline = fs::read(project.root.join(css_path)).unwrap();
        let map: Value = serde_json::from_slice(
            &fs::read(format!("{}.map", project.root.join(css_path).display())).unwrap(),
        )
        .unwrap();
        assert!(!map["outputSpans"].as_array().unwrap().is_empty());
        assert!(serde_json::to_string(&map)
            .unwrap()
            .contains(project.source(source_id)["path"].as_str().unwrap()));
        let source = project.path(source_id);
        let original = fs::read_to_string(&source).unwrap();
        let changed = original.replace(find, replace);
        assert_ne!(changed, original);
        fs::write(&source, changed).unwrap();
        success(&project.run(graph));
        assert_ne!(fs::read(project.root.join(css_path)).unwrap(), baseline);
        success(&copy.run(copy.graph().unwrap()));
        assert_eq!(fs::read(copy.root.join(css_path)).unwrap(), baseline);
        fs::write(&source, original).unwrap();
        success(&project.run(graph));
        assert_eq!(fs::read(project.root.join(css_path)).unwrap(), baseline);
        for pair in graph["expectedGraphFailures"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let source_id = pair[0].as_str().unwrap();
            let source = project.path(source_id);
            let original = fs::read(&source).unwrap();
            fs::remove_dir_all(project.root.join("generated")).unwrap();
            fs::write(
                &source,
                fs::read(project.path(pair[1].as_str().unwrap())).unwrap(),
            )
            .unwrap();
            assert!(!project.run(graph).status.success());
            for path in outputs.keys() {
                assert!(
                    !project.root.join(path).exists(),
                    "invalid {id} must not publish {path}"
                );
            }
            fs::write(source, original).unwrap();
            success(&project.run(graph));
        }
    }
}

#[test]
fn scss_file_uri_graph_uses_its_source_base_but_still_requires_module_read_grants() {
    let mut project = Project::new("scss-stylesheet");
    let original_root = project.root.clone();
    project.root = original_root.with_file_name(format!(
        "{} café % space",
        original_root.file_name().unwrap().to_string_lossy()
    ));
    fs::rename(original_root, &project.root).unwrap();
    let graph = project.graph().unwrap();
    let denied = project.run_with_module_grant(graph, false);
    assert!(!denied.status.success());
    let diagnostics = String::from_utf8_lossy(&denied.stderr);
    assert!(
        diagnostics.contains("cem.scss.resolver_denied"),
        "{diagnostics}"
    );
    assert!(
        diagnostics.contains(
            &reqwest::Url::from_file_path(project.path("scss-source"))
                .unwrap()
                .to_string()
        ),
        "{diagnostics}"
    );
    for path in graph["expectedGraphOutputs"].as_object().unwrap().keys() {
        assert!(
            !project.root.join(path).exists(),
            "a missing module grant cannot publish {path}"
        );
    }
    let admitted = project.run(graph);
    success(&admitted);
    for (path, expected) in graph["expectedGraphOutputs"].as_object().unwrap() {
        assert_eq!(
            fs::read(project.root.join(path)).unwrap(),
            fs::read(project.path(expected.as_str().unwrap())).unwrap()
        );
    }
    let map_path = project.root.join("generated/cards.css.map");
    let map: Value = serde_json::from_slice(&fs::read(map_path).unwrap()).unwrap();
    let encoded = serde_json::to_string(&map).unwrap();
    for id in ["scss-source", "scss-tokens"] {
        let uri = reqwest::Url::from_file_path(project.path(id))
            .unwrap()
            .to_string();
        assert!(
            encoded.contains(&uri),
            "source-map sidecar lost {uri}: {encoded}"
        );
    }
    assert!(!map["outputSpans"].as_array().unwrap().is_empty());
    let spans: Vec<cem_ml::interpreter::OutputSpan> =
        serde_json::from_value(map["outputSpans"].clone()).unwrap();
    let module_uri = reqwest::Url::from_file_path(project.path("scss-tokens"))
        .unwrap()
        .to_string();
    let module = fs::read_to_string(project.path("scss-tokens")).unwrap();
    let definition = "$space: 0.75rem;";
    assert!(spans.iter().flat_map(|span| &span.origin.frames).any(|frame| {
        matches!(&frame.transform, cem_ml::source_map::TransformKind::ScssOrigin { module_uri: uri, origin_kind: cem_ml::source_map::ScssOriginKind::Definition, name: Some(name) } if uri == &module_uri && name == "space")
            && frame.span == cem_ml::source_map::FrameSpan::Single(cem_ml::source::ByteRange::new(module.find(definition).unwrap() as u64, definition.len() as u32))
    }), "module definition must retain its original file URI and byte range");
}

#[test]
fn scss_file_uri_graph_retains_localhost_module_grants() {
    let project = Project::new("scss-stylesheet");
    let graph = project.graph().unwrap();
    let config_uri = reqwest::Url::from_file_path(project.path(graph["source"].as_str().unwrap()))
        .unwrap()
        .to_string()
        .replacen("file:///", "file://localhost/", 1);
    let prefix = reqwest::Url::from_directory_path(&project.root)
        .unwrap()
        .to_string()
        .replacen("file:///", "file://localhost/", 1);
    let output = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .args([
            "transform",
            "--config",
            &config_uri,
            "--resolver-read-map",
            &format!("{prefix}={}", project.root.display()),
        ])
        .current_dir(&project.root)
        .output()
        .unwrap();
    success(&output);
    for (path, expected) in graph["expectedGraphOutputs"].as_object().unwrap() {
        assert_eq!(
            fs::read(project.root.join(path)).unwrap(),
            fs::read(project.path(expected.as_str().unwrap())).unwrap()
        );
    }
}

#[test]
fn scss_module_defaults_keep_caller_shadows_separate_and_rebuild_from_saved_module_changes() {
    let project = Project::new("scss-stylesheet");
    let config = project
        .configs()
        .iter()
        .find(|entry| entry["id"] == "expand-css")
        .unwrap();
    let source_path = project.path("scss-source");
    let source = fs::read_to_string(&source_path).unwrap();
    assert!(source.contains("@include tokens.inset();"));
    let module_path = project.path("scss-tokens");
    let module = fs::read_to_string(&module_path).unwrap();
    assert!(module.contains("@mixin inset($amount: $space)"));
    let baseline = fs::read(project.path("expected-css")).unwrap();
    let shadow = source.replace(".demo-card {", "$space: 9rem;\n\n.demo-card {");
    assert_ne!(shadow, source);
    fs::write(&source_path, &shadow).unwrap();
    let omitted = project.run(config);
    success(&omitted);
    assert_eq!(omitted.stdout, baseline);
    let explicit = shadow.replace("tokens.inset()", "tokens.inset($space)");
    fs::write(&source_path, explicit).unwrap();
    let supplied = project.run(config);
    success(&supplied);
    assert_eq!(
        String::from_utf8(supplied.stdout).unwrap(),
        String::from_utf8(baseline.clone())
            .unwrap()
            .replace("padding: 0.75rem", "padding: 9rem")
    );
    fs::write(&source_path, shadow).unwrap();
    fs::write(module_path, module.replace("$space: 0.75rem", "$space: 1rem")).unwrap();
    let changed = project.run(config);
    success(&changed);
    assert_eq!(
        String::from_utf8(changed.stdout).unwrap(),
        String::from_utf8(baseline)
            .unwrap()
            .replace("padding: 0.75rem", "padding: 1rem")
    );
}
