//! Ordinary module-map and portable-configuration projects use native contracts.
use cem_ml::command_service::sha256_hex;
use cem_ml::studio_project::{
    parse_studio_project, studio_project_resource_uri, STUDIO_PROJECT_CEM_CONTENT_TYPE,
    STUDIO_PROJECT_JSON_CONTENT_TYPE, STUDIO_PROJECT_SCHEMA_URI,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

const PROJECTS: [&str; 2] = ["module-maps", "portable-config"];

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
            "cem-module-config-{id}-{}-{}",
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
    fn source_at(&self, path: &str) -> &Value {
        self.definition["sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|source| source["path"] == path)
            .unwrap_or_else(|| panic!("missing source path {path}"))
    }
    fn path(&self, id: &str) -> PathBuf {
        self.root.join(self.source(id)["path"].as_str().unwrap())
    }
    fn read(&self, id: &str) -> Vec<u8> {
        fs::read(self.path(id)).unwrap()
    }
    fn text_at(&self, path: &str) -> String {
        fs::read_to_string(self.root.join(path)).unwrap()
    }
    fn configs(&self) -> &[Value] {
        self.definition["configurations"].as_array().unwrap()
    }
    fn graph_at(&self, path: &str) -> &Value {
        let id = self.source_at(path)["id"].as_str().unwrap();
        self.configs()
            .iter()
            .find(|config| config["source"] == id && config["kind"] == "transformation-graph")
            .unwrap_or_else(|| panic!("missing graph configuration for {path}"))
    }
    fn args(&self, config: &Value) -> Vec<String> {
        config["argv"]
            .as_array()
            .unwrap()
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
            .collect()
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
    fn graph(&self, config: &Value) -> (Output, Value) {
        let path = self.root.join(format!(
            "native-{}.report.json",
            config["id"].as_str().unwrap()
        ));
        let mut args = self.args(config);
        args.extend(["--report-json".into(), path.to_string_lossy().into_owned()]);
        let output = self.run_args(&args);
        let report = fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or(Value::Null);
        (output, report)
    }
    fn clear_outputs(&self) {
        let _ = fs::remove_dir_all(self.root.join("generated"));
    }
    fn owned_path(&self, uri: &str) -> PathBuf {
        let path = Path::new(uri.strip_prefix("file://").unwrap_or(uri));
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        fs::canonicalize(path).unwrap()
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
        "successful commands must not conceal hard source diagnostics: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("@kind=error"),
        "source inspection must not conceal error nodes"
    );
}
fn report(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "invalid native report {error}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}
fn expected_outputs(project: &Project, config: &Value) {
    let outputs = config["expectedGraphOutputs"].as_object().unwrap();
    let entry = config["entryArtifact"].as_str().unwrap();
    assert!(outputs.contains_key(entry));
    for (path, fixture) in outputs {
        assert_eq!(
            fs::read(project.root.join(path)).unwrap(),
            project.read(fixture.as_str().unwrap()),
            "{path}"
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
        let map: Value =
            serde_json::from_slice(&fs::read(project.root.join(format!("{entry}.map"))).unwrap())
                .unwrap();
        assert!(!map["outputSpans"].as_array().unwrap().is_empty());
        for path in config["expectedProvenance"].as_array().unwrap() {
            assert!(map
                .to_string()
                .contains(&format!("/{}", path.as_str().unwrap())));
        }
    }
    let folder = project.root.join(Path::new(entry).parent().unwrap());
    let mut actual_files = Vec::new();
    collect_files(&folder, &project.root, &mut actual_files);
    let mut expected_files = outputs.keys().cloned().collect::<Vec<_>>();
    for path in outputs.keys() {
        let map = format!("{path}.map");
        if project.root.join(&map).is_file() {
            expected_files.push(map);
        }
    }
    actual_files.sort();
    expected_files.sort();
    assert_eq!(
        actual_files, expected_files,
        "only declared assets and native sidecars may be written"
    );
}
fn collect_files(folder: &Path, root: &Path, files: &mut Vec<String>) {
    for entry in fs::read_dir(folder).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            collect_files(&entry.path(), root, files);
        } else {
            files.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
}
fn expected_assets(project: &Project, config: &Value, report: &Value) {
    let Some(expected) = config["expectedNativeAssets"].as_array() else {
        return;
    };
    let manifest = &report["reportAst"]["transformGraph"]["moduleAssetManifest"];
    assert_eq!(manifest["hashScheme"], "sha256");
    assert_eq!(
        manifest["assetCount"].as_u64().unwrap(),
        expected.len() as u64
    );
    let assets = manifest["assets"].as_array().unwrap();
    assert_eq!(assets.len(), expected.len());
    if let Some(hash) = config["expectedNativeAssetHash"].as_str() {
        assert_eq!(manifest["hash"], hash);
    }
    for expected in expected {
        let asset = assets
            .iter()
            .find(|asset| asset["specifier"] == expected["specifier"])
            .unwrap();
        for key in ["target", "contentType"] {
            assert_eq!(asset[key], expected[key]);
        }
        let source = project.read(expected["source"].as_str().unwrap());
        assert_eq!(
            project.owned_path(asset["sourceUri"].as_str().unwrap()),
            fs::canonicalize(project.path(expected["source"].as_str().unwrap())).unwrap()
        );
        assert_eq!(
            project.owned_path(asset["sourceMapUri"].as_str().unwrap()),
            fs::canonicalize(project.path(expected["sourceMap"].as_str().unwrap())).unwrap()
        );
        let output =
            fs::read(project.root.join(expected["destination"].as_str().unwrap())).unwrap();
        assert_eq!(
            project.owned_path(asset["destination"].as_str().unwrap()),
            fs::canonicalize(project.root.join(expected["destination"].as_str().unwrap())).unwrap()
        );
        assert_eq!(
            asset["sourceByteLength"].as_u64().unwrap(),
            source.len() as u64
        );
        assert_eq!(asset["sourceSha256"], sha256_hex(&source));
        assert_eq!(asset["byteLength"].as_u64().unwrap(), output.len() as u64);
        assert_eq!(asset["sha256"], sha256_hex(&output));
        assert_eq!(
            output,
            project.read(expected["expectedOutput"].as_str().unwrap())
        );
    }
}

#[test]
fn authored_module_and_configuration_projects_keep_native_source_and_artifact_contracts() {
    for id in PROJECTS {
        let project = Project::new(id);
        assert!(!project.configs().is_empty());
        for config in project.configs() {
            let (output, graph_report) = if config["kind"] == "transformation-graph" {
                project.graph(config)
            } else {
                (project.run(config), Value::Null)
            };
            let exit = config["expectedExit"].as_i64().unwrap_or(0) as i32;
            assert_eq!(
                output.status.code(),
                Some(exit),
                "{id}/{}: {} {}",
                config["id"],
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            if exit == 0 {
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
                    "{id}/{} misses{text}",
                    config["id"]
                );
            }
            if let Some(expected) = config["expectedValidation"].as_object() {
                let result = report(&output);
                for key in ["errorCount", "warningCount"] {
                    assert_eq!(result["summary"][key], expected[key]);
                }
                assert_eq!(
                    result["summary"]["fatalCount"],
                    expected.get("fatalCount").cloned().unwrap_or(json!(0))
                );
            }
            if config["expectedGraphOutputs"].is_object() {
                expected_outputs(&project, config);
                expected_assets(&project, config, &graph_report);
                let (repeated, repeated_report) = project.graph(config);
                success(&repeated);
                expected_outputs(&project, config);
                assert_eq!(
                    graph_report["reportAst"]["transformGraph"]["moduleAssetManifest"],
                    repeated_report["reportAst"]["transformGraph"]["moduleAssetManifest"]
                );
            }
        }
    }
}

#[test]
fn original_configuration_diagnostics_repair_without_changing_catalog_sources() {
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
            assert_eq!(
                output.status.code(),
                Some(config["expectedExit"].as_i64().unwrap_or(1) as i32)
            );
            let result = report(&output);
            let expected = &config["expectedValidation"];
            let code = expected["errorCode"]
                .as_str()
                .or(expected["warningCode"].as_str())
                .unwrap();
            let diagnostic = result["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .find(|diagnostic| diagnostic["code"] == code)
                .unwrap();
            assert_eq!(
                diagnostic["severity"],
                expected["severity"]
                    .as_str()
                    .unwrap_or(if expected["errorCode"].is_string() {
                        "error"
                    } else {
                        "warning"
                    })
            );
            let owner = expected["source"].as_str().unwrap_or(source);
            assert_eq!(
                project.owned_path(diagnostic["uri"].as_str().unwrap()),
                fs::canonicalize(project.path(owner)).unwrap()
            );
            for (key, value) in expected["location"].as_object().into_iter().flatten() {
                assert_eq!(&diagnostic[key], value, "{id}/{key}");
            }
            let repaired = original.replacen(
                config["repair"]["find"].as_str().unwrap(),
                config["repair"]["replace"].as_str().unwrap(),
                1,
            );
            assert_ne!(repaired, original);
            fs::write(project.path(source), repaired).unwrap();
            let fixed = project.run(config);
            success(&fixed);
            assert_eq!(report(&fixed)["summary"]["hardViolationCount"], 0);
            let catalog = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(project.source(source)["source"].as_str().unwrap());
            assert_eq!(fs::read_to_string(catalog).unwrap(), original);
        }
    }
}

#[test]
fn deployment_maps_keep_native_hashes_rewrites_and_no_publication_on_contract_refusal() {
    let project = Project::new("module-maps");
    let independent = Project::new("module-maps");
    for version in 1..=3 {
        let config = project.graph_at(&format!("graphs/v{version}.cem"));
        let (baseline, baseline_report) = project.graph(config);
        success(&baseline);
        expected_assets(&project, config, &baseline_report);
        let (copy, copy_report) = independent.graph(config);
        success(&copy);
        expected_outputs(&independent, config);
        let manifest = &baseline_report["reportAst"]["transformGraph"]["moduleAssetManifest"];
        assert_eq!(
            manifest["hash"],
            copy_report["reportAst"]["transformGraph"]["moduleAssetManifest"]["hash"]
        );
        let mut key_args = project.args(config);
        key_args.push("--module-asset-cache-key".into());
        project.clear_outputs();
        let key = project.run_args(&key_args);
        success(&key);
        assert_eq!(
            String::from_utf8(key.stdout).unwrap().trim(),
            format!("sha256:{}", manifest["hash"].as_str().unwrap())
        );
        assert!(!project.root.join("generated").exists());
        for refusal in config["expectedGraphRefusals"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let source = refusal["source"].as_str().unwrap();
            let replacement = refusal["replacement"].as_str().unwrap();
            let original = project.read(source);
            fs::write(project.path(source), project.read(replacement)).unwrap();
            // Standalone module-map loading checks JSON syntax, while graph
            // planning owns paired-map version and deployment semantics.
            let resource = project.source(source);
            let syntax = project.run_args(&[
                "validate".into(),
                project.path(source).to_string_lossy().into_owned(),
                "--content-type".into(),
                resource["contentType"].as_str().unwrap().into(),
                "--schema".into(),
                resource["schema"].as_str().unwrap().into(),
                "--format".into(),
                "json".into(),
                "--no-color".into(),
            ]);
            success(&syntax);
            assert_eq!(report(&syntax)["summary"]["hardViolationCount"], 0);
            let denied = project.run(config);
            assert_eq!(
                denied.status.code(),
                Some(refusal["exitCode"].as_i64().unwrap() as i32),
                "{} {}",
                String::from_utf8_lossy(&denied.stdout),
                String::from_utf8_lossy(&denied.stderr)
            );
            let combined = format!(
                "{} {}",
                String::from_utf8_lossy(&denied.stdout),
                String::from_utf8_lossy(&denied.stderr)
            );
            assert!(combined.contains(refusal["code"].as_str().unwrap()));
            for path in config["expectedGraphOutputs"].as_object().unwrap().keys() {
                assert!(!project.root.join(path).exists());
            }
            fs::write(project.path(source), original).unwrap();
        }
        let (restored, _) = project.graph(config);
        success(&restored);
        expected_outputs(&project, config);
    }
    let config = project.graph_at("graphs/v3.cem");
    let target = project.root.join("maps/target-v3.json");
    let original = fs::read_to_string(&target).unwrap();
    let changed = original.replace("./assets/metadata.json", "./data/metadata.json");
    assert_ne!(changed, original);
    fs::write(&target, changed).unwrap();
    project.clear_outputs();
    let (updated, updated_report) = project.graph(config);
    success(&updated);
    assert!(project
        .text_at("generated/v3/assets/app.js")
        .contains("../data/metadata.json"));
    assert_eq!(
        fs::read(project.root.join("generated/v3/data/metadata.json")).unwrap(),
        fs::read(project.root.join("modules/metadata.json")).unwrap()
    );
    let (_, original_report) = independent.graph(config);
    assert_ne!(
        updated_report["reportAst"]["transformGraph"]["moduleAssetManifest"]["hash"],
        original_report["reportAst"]["transformGraph"]["moduleAssetManifest"]["hash"]
    );
    assert!(independent
        .text_at("generated/v3/assets/app.js")
        .contains("'./metadata.json'"));
    assert!(project
        .text_at("modules/app.js")
        .contains("'@demo/metadata'"));
}

#[test]
fn deployment_mapping_does_not_grant_custom_uri_reads() {
    let project = Project::new("module-maps");
    let config = project.graph_at("graphs/v1.cem");
    let uri = "cem+fixture://owned/caption.js";
    for path in ["maps/source-v1.json", "pages/v1.html"] {
        let original = project.text_at(path);
        let changed = original.replace("../modules/caption.js", uri);
        assert_ne!(changed, original);
        fs::write(project.root.join(path), changed).unwrap();
    }
    project.clear_outputs();
    let denied = project.run(config);
    assert_eq!(denied.status.code(), Some(6));
    assert!(denied.stdout.is_empty());
    assert!(String::from_utf8_lossy(&denied.stderr).contains("unsupported resolver"));
    assert!(!project.root.join("generated").exists());
    let mut args = project.args(config);
    args.extend([
        "--resolver-read-map".into(),
        format!(
            "cem+fixture://owned={}",
            project.root.join("modules").display()
        ),
    ]);
    let allowed = project.run_args(&args);
    success(&allowed);
    assert_eq!(
        fs::read(project.root.join("generated/v1/assets/caption.js")).unwrap(),
        fs::read(project.root.join("modules/caption.js")).unwrap()
    );
    assert!(project
        .text_at("generated/v1/index.html")
        .contains("./assets/caption.js"));
    assert!(!project.root.join("generated/v1/index.html.map").exists());
}

#[test]
fn portable_project_encodings_keep_logical_identities_hashes_and_actual_authored_argv() {
    let project = Project::new("portable-config");
    let json_bytes = fs::read(project.root.join("project/card.project.json")).unwrap();
    let cem_bytes = fs::read(project.root.join("project/card.project.cem")).unwrap();
    let model = parse_studio_project(
        &json_bytes,
        STUDIO_PROJECT_JSON_CONTENT_TYPE,
        STUDIO_PROJECT_SCHEMA_URI,
    )
    .unwrap();
    assert_eq!(
        model,
        parse_studio_project(
            &cem_bytes,
            STUDIO_PROJECT_CEM_CONTENT_TYPE,
            STUDIO_PROJECT_SCHEMA_URI
        )
        .unwrap()
    );
    let roundtrip = fs::read(project.root.join("expected/roundtrip.project.json")).unwrap();
    assert_eq!(
        model,
        parse_studio_project(
            &roundtrip,
            STUDIO_PROJECT_JSON_CONTENT_TYPE,
            STUDIO_PROJECT_SCHEMA_URI
        )
        .unwrap()
    );
    for resource in &model.resources {
        assert_eq!(
            studio_project_resource_uri(&model, &resource.id).unwrap(),
            format!(
                "{}{}",
                model.root_uri,
                resource.reference_path.as_deref().unwrap_or(&resource.path)
            )
        );
        assert!(!Path::new(&resource.path).is_absolute());
        assert_eq!(
            resource.sha256,
            sha256_hex(&fs::read(project.root.join(&resource.path)).unwrap())
        );
    }
    let command_bytes = fs::read(project.root.join("authored/build.command.json")).unwrap();
    let command = cem_ml::cli_command::parse_cli_command(
        &command_bytes,
        cem_ml::cli_command::CLI_COMMAND_CONTENT_TYPE,
        cem_ml::cli_command::CLI_COMMAND_SCHEMA_URI,
    )
    .unwrap();
    let built = project.run_args(&command.argv);
    success(&built);
    assert_eq!(
        fs::read(project.root.join("generated/index.html")).unwrap(),
        fs::read(project.root.join("expected/index.html")).unwrap()
    );
    let mut invalid_command = command.clone();
    invalid_command.argv = vec!["not-a-native-command".into()];
    cem_ml::cli_command::validate_cli_command(&invalid_command).unwrap();
    project.clear_outputs();
    let refused = project.run_args(&invalid_command.argv);
    assert_eq!(refused.status.code(), Some(2));
    assert!(!project.root.join("generated").exists());
    let graph_config = project.graph_at("graphs/card.cem");
    let original_data = project.text_at("data/card.xml");
    let changed = original_data.replace("Café Noord", "Café Haven");
    assert_ne!(changed, original_data);
    fs::write(project.root.join("data/card.xml"), changed).unwrap();
    let (updated, _) = project.graph(graph_config);
    success(&updated);
    assert!(project
        .text_at("generated/index.html")
        .contains("Café Haven"));
    let independent = Project::new("portable-config");
    success(&independent.run_args(&command.argv));
    assert!(independent
        .text_at("generated/index.html")
        .contains("Café Noord"));
    let graph = project.text_at("graphs/card.cem");
    let heading = graph.replace("Dispatch board", "Release board");
    assert_ne!(heading, graph);
    fs::write(project.root.join("graphs/card.cem"), heading).unwrap();
    success(&project.run_args(&command.argv));
    assert!(project
        .text_at("generated/index.html")
        .contains("Release board"));
    for pair in graph_config["expectedGraphFailures"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let source = pair[0].as_str().unwrap();
        let bad = pair[1].as_str().unwrap();
        let saved = project.read(source);
        fs::write(project.path(source), project.read(bad)).unwrap();
        project.clear_outputs();
        let refused = project.run_args(&command.argv);
        assert_eq!(refused.status.code(), Some(1));
        assert!(!project.root.join("generated/index.html").exists());
        fs::write(project.path(source), saved).unwrap();
    }
}

#[test]
fn run_config_controls_relative_inputs_and_rejects_duplicates_before_any_input_read() {
    let project = Project::new("portable-config");
    let command_bytes = fs::read(project.root.join("authored/check.command.json")).unwrap();
    let command = cem_ml::cli_command::parse_cli_command(
        &command_bytes,
        cem_ml::cli_command::CLI_COMMAND_CONTENT_TYPE,
        cem_ml::cli_command::CLI_COMMAND_SCHEMA_URI,
    )
    .unwrap();
    let baseline = project.run_args(&command.argv);
    success(&baseline);
    assert!(report(&baseline)["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|uri| uri.as_str().unwrap().ends_with("data/card.xml")));
    let config_path = project.root.join("authored/check.run.json");
    let original = fs::read_to_string(&config_path).unwrap();
    let redirected = original.replace("data/card.xml", "data/alternate.xml");
    assert_ne!(redirected, original);
    fs::write(&config_path, redirected).unwrap();
    let changed = project.run_args(&command.argv);
    success(&changed);
    assert!(report(&changed)["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|uri| uri.as_str().unwrap().ends_with("data/alternate.xml")));
    assert!(!report(&changed)["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|uri| uri.as_str().unwrap().ends_with("data/card.xml")));
    fs::write(&config_path, project.read("invalid-run")).unwrap();
    fs::remove_file(project.root.join("data/card.xml")).unwrap();
    let refused = project.run_args(&command.argv);
    assert_eq!(refused.status.code(), Some(2));
    let failure = report(&refused);
    let diagnostic = failure["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "cem.run_config.input_uri_duplicate")
        .unwrap();
    assert_eq!(diagnostic["severity"], "fatal");
    assert!(
        diagnostic["line"].is_null()
            && diagnostic["column"].is_null()
            && diagnostic["byteOffset"].is_null()
    );
    assert!(!String::from_utf8_lossy(&refused.stderr).contains("I/O error"));
    assert!(!project.root.join("generated").exists());
}
