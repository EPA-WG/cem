//! Ordinary transformation and schema projects exercise the native authoring contract.
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

const PROJECTS: [&str; 5] = [
    "cem-transform",
    "xslt-transform",
    "cem-schema",
    "json-schema",
    "relax-ng",
];

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
            "cem-transform-schema-{id}-{}-{}",
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
            .find(|source| source["id"] == id)
            .unwrap_or_else(|| panic!("missing source {id}"))
    }
    fn path(&self, id: &str) -> PathBuf {
        self.root.join(self.source(id)["path"].as_str().unwrap())
    }
    fn read(&self, id: &str) -> Vec<u8> {
        fs::read(self.path(id)).unwrap()
    }
    fn configs(&self) -> &[Value] {
        self.definition["configurations"].as_array().unwrap()
    }
    fn config(&self, id: &str) -> &Value {
        self.configs()
            .iter()
            .find(|config| config["id"] == id)
            .unwrap_or_else(|| panic!("missing configuration {id}"))
    }
    fn args(&self, config: &Value) -> Vec<String> {
        if let Some(argv) = config["argv"].as_array() {
            return argv
                .iter()
                .map(|arg| {
                    let arg = arg.as_str().unwrap();
                    match if arg == "$source" {
                        config["source"].as_str()
                    } else {
                        arg.strip_prefix('$')
                    } {
                        Some(id) => self.path(id).to_string_lossy().into_owned(),
                        None => arg.to_owned(),
                    }
                })
                .collect();
        }
        assert_eq!(config["kind"], "validation");
        let source = self.source(config["source"].as_str().unwrap());
        let mut args = vec![
            "validate".into(),
            self.path(source["id"].as_str().unwrap())
                .to_string_lossy()
                .into_owned(),
            "--content-type".into(),
            source["contentType"].as_str().unwrap().into(),
            "--schema".into(),
            source["schema"].as_str().unwrap().into(),
            "--format".into(),
            "json".into(),
            "--no-color".into(),
        ];
        for dependency in config["dependencies"].as_array().into_iter().flatten() {
            let id = dependency.as_str().unwrap();
            if self.source(id)["schema"] == "https://cem.dev/ns/schema-package/1" {
                args.extend([
                    "--schema-package".into(),
                    self.path(id).to_string_lossy().into_owned(),
                ]);
            }
        }
        args
    }
    fn run_args(&self, args: &[String]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .unwrap()
    }
    fn run(&self, config: &Value) -> Output {
        self.run_args(&self.args(config))
    }
    fn graph(&self) -> &Value {
        self.configs()
            .iter()
            .find(|config| config["kind"] == "transformation-graph")
            .expect("transformation project has an ordinary graph")
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
fn report(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "invalid native report: {error}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn all_authored_transformation_and_schema_configurations_keep_native_results() {
    for id in PROJECTS {
        let project = Project::new(id);
        assert!(!project.configs().is_empty());
        for config in project.configs() {
            let output = project.run(config);
            let expected_exit = config["expectedExit"].as_i64().unwrap_or(0) as i32;
            assert_eq!(
                output.status.code(),
                Some(expected_exit),
                "{id}/{}: {} {}",
                config["id"],
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            if expected_exit == 0 {
                success(&output);
            }
            if let Some(expected) = config["expectedOutput"].as_str() {
                assert_eq!(
                    output.stdout,
                    project.read(expected),
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
            if let Some(expected) = config["expectedValidation"].as_object() {
                let report = report(&output);
                assert_eq!(report["summary"]["errorCount"], expected["errorCount"]);
                assert_eq!(report["summary"]["warningCount"], expected["warningCount"]);
                assert_eq!(report["summary"]["fatalCount"], 0);
            }
            if let Some(outputs) = config["expectedGraphOutputs"].as_object() {
                let entry = config["entryArtifact"].as_str().unwrap();
                assert!(outputs.contains_key(entry));
                for (path, expected) in outputs {
                    assert_eq!(
                        fs::read(project.root.join(path)).unwrap(),
                        project.read(expected.as_str().unwrap()),
                        "{id}/{path}"
                    );
                }
                for path in config["expectedUnmappedOutputs"]
                    .as_array()
                    .into_iter()
                    .flatten()
                {
                    assert!(outputs.contains_key(path.as_str().unwrap()));
                    assert!(!project
                        .root
                        .join(format!("{}.map", path.as_str().unwrap()))
                        .exists());
                }
                if !config["expectedUnmappedOutputs"]
                    .as_array()
                    .is_some_and(|paths| paths.iter().any(|path| path == entry))
                {
                    let map: Value = serde_json::from_slice(
                        &fs::read(project.root.join(format!("{entry}.map"))).unwrap(),
                    )
                    .unwrap();
                    assert!(!map["outputSpans"].as_array().unwrap().is_empty());
                    let text = map.to_string();
                    let paths = config["expectedProvenance"]
                        .as_array()
                        .expect("mapped entry names actual native provenance owners");
                    assert!(!paths.is_empty());
                    for path in paths {
                        assert!(
                            text.contains(&format!("/{}", path.as_str().unwrap())),
                            "{id} missing {path} in native map"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn schema_and_template_repairs_retain_original_diagnostics_and_catalog_bytes() {
    for id in PROJECTS {
        let definition = Project::new(id);
        for config in definition
            .configs()
            .iter()
            .filter(|config| config["repair"].is_object())
        {
            let project = Project::new(id);
            let source = config["source"].as_str().unwrap();
            let original = String::from_utf8(project.read(source)).unwrap();
            let output = project.run(config);
            assert_eq!(output.status.code(), Some(1));
            let result = report(&output);
            let expected = &config["expectedValidation"];
            let diagnostic = result["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .find(|diagnostic| diagnostic["code"] == expected["errorCode"])
                .unwrap_or_else(|| panic!("missing {} in {result}", expected["errorCode"]));
            assert_eq!(diagnostic["severity"], "error");
            let identity = expected["source"].as_str().unwrap_or(source);
            assert_eq!(
                diagnostic["uri"],
                project.path(identity).to_string_lossy().as_ref()
            );
            if let Some(location) = expected["location"].as_object() {
                for (key, value) in location {
                    assert_eq!(&diagnostic[key], value, "{id}/{key}");
                }
            } else {
                assert!(diagnostic["line"].as_u64().unwrap() > 0);
                assert!(diagnostic["column"].as_u64().unwrap() > 0);
                assert!(diagnostic["byteOffset"].as_u64().is_some());
            }
            let fixed = original.replacen(
                config["repair"]["find"].as_str().unwrap(),
                config["repair"]["replace"].as_str().unwrap(),
                1,
            );
            assert_ne!(fixed, original);
            fs::write(project.path(source), fixed).unwrap();
            let repaired = project.run(config);
            success(&repaired);
            let repaired = report(&repaired);
            assert_eq!(repaired["summary"]["hardViolationCount"], 0);
            let catalog = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(project.source(source)["source"].as_str().unwrap());
            assert_eq!(fs::read_to_string(catalog).unwrap(), original);
        }
    }
}

#[test]
fn cem_instance_validation_uses_the_owned_schema_and_refuses_missing_package_inputs() {
    let project = Project::new("cem-schema");
    let independent = Project::new("cem-schema");
    let valid = project.config("validate-instance");
    success(&project.run(valid));
    let invalid = project.config("repair-instance");
    let original = report(&project.run(invalid));
    assert_eq!(original["summary"]["errorCount"], 1);
    let original_diagnostic = original["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["code"] == "example.note.id_required")
        .unwrap();
    assert_eq!(original_diagnostic["severity"], "error");
    let original_message = "A note must declare its id: element `note` failed field contract `note-id` (required-fields) with missing fields: id";
    assert_eq!(original_diagnostic["message"], original_message);

    let schema = project.path("note-schema");
    let source = fs::read_to_string(&schema).unwrap();
    let edited = source.replace(
        "A note must declare its id",
        "Saved schema requires a note id",
    );
    assert_ne!(edited, source);
    fs::write(&schema, &edited).unwrap();
    success(&project.run(project.config("validate-schema")));
    let changed = report(&project.run(invalid));
    let diagnostic = changed["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["code"] == "example.note.id_required")
        .unwrap();
    assert_eq!(diagnostic["severity"], "error");
    assert_eq!(
        diagnostic["message"],
        original_message.replace(
            "A note must declare its id",
            "Saved schema requires a note id"
        )
    );
    let unchanged = report(&independent.run(independent.config("repair-instance")));
    assert!(unchanged["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["code"] == "example.note.id_required"
            && item["message"] == original_message));

    let instance = project.path("note-instance");
    let original_instance = fs::read_to_string(&instance).unwrap();
    let edited_instance = original_instance.replace("Café notes", "Saved notes");
    assert_ne!(edited_instance, original_instance);
    fs::write(&instance, edited_instance).unwrap();
    success(&project.run(valid));
    fs::write(&instance, project.read("invalid-instance")).unwrap();
    let failure = project.run(valid);
    assert_eq!(failure.status.code(), Some(1));
    assert!(report(&failure)["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["code"] == "example.note.id_required" && item["severity"] == "error"));
    fs::write(&instance, original_instance).unwrap();
    success(&project.run(valid));

    let mut without_package = project.args(valid);
    let at = without_package
        .iter()
        .position(|arg| arg == "--schema-package")
        .unwrap();
    without_package.drain(at..at + 2);
    let missing = project.run_args(&without_package);
    assert_eq!(
        missing.status.code(),
        Some(1),
        "unloaded domain schema cannot silently validate: {} {}",
        String::from_utf8_lossy(&missing.stdout),
        String::from_utf8_lossy(&missing.stderr)
    );
    assert!(
        report(&missing)["summary"]["hardViolationCount"]
            .as_u64()
            .unwrap()
            > 0
    );

    fs::remove_file(&schema).unwrap();
    let missing = project.run(valid);
    assert_eq!(missing.status.code(), Some(1));
    assert!(report(&missing)["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(
            |item| item["code"] == "cem.schema_package.schema_source_unreadable"
                && item["severity"] == "error"
        ));
    fs::write(&schema, edited).unwrap();
    success(&project.run(valid));
    fs::remove_file(project.path("note-package")).unwrap();
    let missing = project.run(valid);
    assert!(
        !missing.status.success(),
        "missing package must refuse, not infer a domain contract"
    );
    let text = format!(
        "{} {}",
        String::from_utf8_lossy(&missing.stdout),
        String::from_utf8_lossy(&missing.stderr)
    );
    assert!(
        text.contains("package.cem"),
        "missing-package refusal retains its resource identity: {text}"
    );
}

#[test]
fn json_schema_and_relax_ng_source_authoring_keep_their_explicit_semantic_limits() {
    let json = Project::new("json-schema");
    let independent = Project::new("json-schema");
    let schema = json.path("note-schema");
    let original = fs::read_to_string(&schema).unwrap();
    let edited = original.replace("\"title\": \"Note\"", "\"title\": \"Saved note\"");
    assert_ne!(edited, original);
    fs::write(&schema, edited).unwrap();
    success(&json.run(json.config("validate-schema")));
    let formatted = json.run(json.config("format-schema"));
    success(&formatted);
    assert_eq!(
        serde_json::from_slice::<Value>(&formatted.stdout).unwrap()["title"],
        "Saved note"
    );
    let original_formatted = independent.run(independent.config("format-schema"));
    success(&original_formatted);
    assert_eq!(
        serde_json::from_slice::<Value>(&original_formatted.stdout).unwrap()["title"],
        "Note"
    );
    // Source validation does not assert domain keywords or traverse $ref edges.
    fs::write(json.path("note-data"), br#"{"title":42}"#).unwrap();
    success(&json.run(json.config("validate-data-syntax")));
    let remote_reference = original.replace("#/$defs/label", "https://unavailable.invalid/schema");
    assert_ne!(remote_reference, original);
    fs::write(&schema, remote_reference).unwrap();
    success(&json.run(json.config("validate-schema")));

    let rng = Project::new("relax-ng");
    let independent = Project::new("relax-ng");
    for (id, operation, from, to) in [
        (
            "xml-schema",
            "validate-xml",
            "name=\"note\"",
            "name=\"memo\"",
        ),
        ("compact-schema", "validate-compact", "note", "memo"),
    ] {
        let path = rng.path(id);
        let source = fs::read_to_string(&path).unwrap();
        let changed = source.replace(from, to);
        assert_ne!(changed, source);
        fs::write(path, &changed).unwrap();
        success(&rng.run(rng.config(operation)));
        assert_eq!(String::from_utf8(independent.read(id)).unwrap(), source);
        assert_eq!(String::from_utf8(rng.read(id)).unwrap(), changed);
    }
    // The selected operation validates XML syntax, not the RELAX NG instance.
    fs::write(rng.path("note-data"), "<unrelated><value/></unrelated>\n").unwrap();
    success(&rng.run(rng.config("validate-data-syntax")));
    let refusal = rng.run(rng.config("repair-reference-policy"));
    assert_eq!(refusal.status.code(), Some(1));
    let result = report(&refusal);
    assert_eq!(result["summary"]["errorCount"], 1);
    assert!(result["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["code"] == "cem.relax_ng.external_ref_rejected"
            && item["severity"] == "error"));
    let xml = String::from_utf8(independent.read("xml-schema")).unwrap();
    let external = xml.replace(
        "<start><ref name=\"note\"/></start>",
        "<start><externalRef href=\"remote.rng\"/></start>",
    );
    assert_ne!(external, xml);
    fs::write(rng.path("xml-schema"), external).unwrap();
    let refusal = rng.run(rng.config("validate-xml"));
    assert_eq!(refusal.status.code(), Some(1));
    let result = report(&refusal);
    assert_eq!(result["summary"]["errorCount"], 1);
    assert!(result["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["code"] == "cem.relax_ng.external_ref_rejected"
            && item["severity"] == "error"));
}

#[test]
fn transformation_graph_edits_keep_independent_artifacts_and_refuse_partial_publication() {
    for id in ["cem-transform", "xslt-transform"] {
        let project = Project::new(id);
        let independent = Project::new(id);
        let graph = project.graph();
        let entry = graph["entryArtifact"].as_str().unwrap();
        assert_eq!(entry, "generated/index.html");
        success(&project.run(graph));
        let baseline = fs::read(project.root.join(entry)).unwrap();
        success(&project.run(graph));
        assert_eq!(fs::read(project.root.join(entry)).unwrap(), baseline);

        let source_path = project.path("orders-source");
        let original_source = fs::read_to_string(&source_path).unwrap();
        let saved_source = original_source.replace("Café Noord", "Café Haven");
        assert_ne!(saved_source, original_source);
        fs::write(&source_path, &saved_source).unwrap();
        success(&project.run(graph));
        let edited_data = fs::read_to_string(project.root.join(entry)).unwrap();
        assert!(edited_data.contains("Café Haven"));
        assert!(!edited_data.contains("Café Noord"));

        let template_path = project.path("page-template");
        let original_template = fs::read_to_string(&template_path).unwrap();
        let saved_template = original_template.replace("@name=ul", "@name=ol");
        assert_ne!(saved_template, original_template);
        fs::write(&template_path, &saved_template).unwrap();
        success(&project.run(graph));
        let edited_template = fs::read_to_string(project.root.join(entry)).unwrap();
        assert!(edited_template.contains("<ol>"));
        assert!(edited_template.contains("Café Haven"));

        let graph_path = project.path("report-graph");
        let original_graph = fs::read_to_string(&graph_path).unwrap();
        let saved_graph = original_graph.replace("@value=30", "@value=45");
        assert_ne!(saved_graph, original_graph);
        fs::write(&graph_path, &saved_graph).unwrap();
        success(&project.run(graph));
        let edited_graph = fs::read_to_string(project.root.join(entry)).unwrap();
        assert!(edited_graph.contains("O-101"));
        assert!(!edited_graph.contains("O-103"));
        assert!(edited_graph.contains("<ol>") && edited_graph.contains("Café Haven"));
        if id == "cem-transform" {
            let data: Value = serde_json::from_slice(
                &fs::read(project.root.join("generated/report.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(data.as_array().unwrap().len(), 1);
            assert_eq!(data[0]["id"], "O-101");
            assert!(!project.root.join("generated/report.json.map").exists());
        }
        success(&independent.run(independent.graph()));
        assert_eq!(fs::read(independent.root.join(entry)).unwrap(), baseline);

        for (source, invalid) in [
            ("orders-source", "invalid-orders"),
            ("report-template", "invalid-template"),
        ] {
            let source_path = project.path(source);
            let before = fs::read(&source_path).unwrap();
            fs::remove_dir_all(project.root.join("generated")).unwrap();
            fs::write(&source_path, project.read(invalid)).unwrap();
            let refused = project.run(graph);
            assert_eq!(
                refused.status.code(),
                Some(1),
                "{id}/{invalid}: {} {}",
                String::from_utf8_lossy(&refused.stdout),
                String::from_utf8_lossy(&refused.stderr)
            );
            assert!(
                refused.stdout.is_empty(),
                "failed graph must not publish a replacement primary"
            );
            for output in graph["expectedGraphOutputs"].as_object().unwrap().keys() {
                assert!(
                    !project.root.join(output).exists(),
                    "{id}/{invalid} published partial {output}"
                );
            }
            fs::write(source_path, before).unwrap();
            success(&project.run(graph));
            assert_eq!(
                fs::read_to_string(project.root.join(entry)).unwrap(),
                edited_graph
            );
        }
        fs::write(graph_path, original_graph).unwrap();
        success(&project.run(graph));
        assert_eq!(
            fs::read_to_string(project.root.join(entry)).unwrap(),
            edited_template
        );
        assert_eq!(fs::read(source_path).unwrap(), saved_source.as_bytes());
        assert_eq!(fs::read(template_path).unwrap(), saved_template.as_bytes());
    }
}
