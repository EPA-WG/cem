//! The authored vector/formula projects use the same native contracts as Studio.
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

const PROJECTS: [&str; 2] = ["svg-diagram", "mathml-formula"];
fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}
struct Project {
    root: PathBuf,
    definition: Value,
}
impl Project {
    fn new(id: &str) -> Self {
        let definition: Value = serde_json::from_slice(
            &fs::read(workspace().join(format!("examples/projects/{id}/example.json"))).unwrap(),
        )
        .unwrap();
        let root = std::env::temp_dir().join(format!(
            "cem-{id}-{}-{}",
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
    fn configurations(&self) -> &[Value] {
        self.definition["configurations"].as_array().unwrap()
    }
    fn kind(&self, kind: &str) -> &Value {
        self.configurations()
            .iter()
            .find(|c| c["kind"] == kind)
            .unwrap()
    }
    fn primary(&self) -> &str {
        self.configurations()
            .iter()
            .find(|c| c["expectedSemanticContract"].is_string())
            .unwrap()["source"]
            .as_str()
            .unwrap()
    }
    fn run(&self, config: &Value) -> Output {
        let args = config["argv"].as_array().unwrap().iter().map(|v| {
            let value = v.as_str().unwrap();
            if value == "$source" {
                self.path(config["source"].as_str().unwrap())
                    .to_string_lossy()
                    .into_owned()
            } else if let Some(id) = value.strip_prefix('$') {
                self.path(id).to_string_lossy().into_owned()
            } else {
                value.to_owned()
            }
        });
        Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .unwrap()
    }
    fn json(&self, config: &Value) -> Value {
        let output = self.run(config);
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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr
            .lines()
            .any(|line| line.contains("error:") || line.contains("fatal:")),
        "successful commands must not hide hard diagnostics: {stderr}"
    );
}
fn values(report: &Value) -> Value {
    assert_eq!(report["language"], "xpath");
    Value::Array(
        report["result"]["sequence"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["value"]["lexicalValue"].clone())
            .collect(),
    )
}

#[test]
fn authored_svg_and_mathml_commands_keep_exact_results_and_family_contracts() {
    for id in PROJECTS {
        let project = Project::new(id);
        assert_eq!(
            project.configurations().len(),
            if id == "svg-diagram" { 6 } else { 7 }
        );
        for config in project.configurations() {
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
                    "{id}/{} exact formatter output",
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
                assert_eq!(
                    values(&serde_json::from_slice(&output.stdout).unwrap()),
                    serde_json::from_slice::<Value>(&fs::read(project.path(expected)).unwrap())
                        .unwrap()
                );
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
fn family_namespace_failures_identify_original_bytes_and_repair_before_building() {
    for id in PROJECTS {
        let project = Project::new(id);
        let config = project
            .configurations()
            .iter()
            .find(|c| c["repair"].is_object())
            .unwrap();
        let path = project.path(config["source"].as_str().unwrap());
        let invalid = fs::read_to_string(&path).unwrap();
        let report: Value = serde_json::from_slice(&project.run(config).stdout).unwrap();
        let expected = &config["expectedValidation"];
        let diagnostic = report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["code"] == expected["errorCode"])
            .unwrap();
        assert_eq!(diagnostic["severity"], "error");
        assert_eq!(diagnostic["uri"], path.to_str().unwrap());
        for (key, value) in expected["location"].as_object().unwrap() {
            assert_eq!(&diagnostic[key], value, "{id}/{key}");
        }
        assert!(diagnostic["sourceMap"]["frames"]
            .as_array()
            .is_some_and(|frames| !frames.is_empty()));
        let repair = &config["repair"];
        let repaired = invalid.replacen(
            repair["find"].as_str().unwrap(),
            repair["replace"].as_str().unwrap(),
            1,
        );
        assert_ne!(invalid, repaired);
        let source = project.path(project.primary());
        assert_eq!(repaired, fs::read_to_string(&source).unwrap());
        fs::write(&source, &invalid).unwrap();
        let graph = project.kind("transformation-graph");
        let failed = project.run(graph);
        assert!(
            !failed.status.success(),
            "invalid {id} must not enter the template through generic XML"
        );
        assert!(!project
            .root
            .join(graph["entryArtifact"].as_str().unwrap())
            .exists());
        fs::write(&path, &repaired).unwrap();
        assert_eq!(project.json(config)["summary"]["errorCount"], 0);
        fs::write(source, repaired).unwrap();
        success(&project.run(graph));
    }
}

fn assert_origins(project: &Project, graph: &Value) {
    let entry = project.root.join(graph["entryArtifact"].as_str().unwrap());
    let output = fs::read(&entry).unwrap();
    let map: Value =
        serde_json::from_slice(&fs::read(format!("{}.map", entry.display())).unwrap()).unwrap();
    let primary = project.source(project.primary())["path"].as_str().unwrap();
    let template = graph["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| project.source(id.as_str().unwrap()))
        .find(|s| s["role"] == "template")
        .unwrap();
    let template = template["path"].as_str().unwrap();
    for path in [primary, template] {
        let authored = fs::read_to_string(project.root.join(path)).unwrap();
        let mut bounded_origin = false;
        for span in map["outputSpans"].as_array().unwrap() {
            assert!(
                span["outputRange"]["start"].as_u64().unwrap()
                    + span["outputRange"]["len"].as_u64().unwrap()
                    <= output.len() as u64
            );
            for frame in span["origin"]["frames"].as_array().unwrap() {
                if frame["transform"]["kind"] != "SourceBoundary"
                    || !frame["transform"]["uri"]
                        .as_str()
                        .is_some_and(|uri| uri.ends_with(path))
                    || frame["span"]["kind"] != "Single"
                {
                    continue;
                }
                let range = &frame["span"]["ranges"];
                let start = range["start"].as_u64().unwrap() as usize;
                let len = range["len"].as_u64().unwrap() as usize;
                assert!(
                    authored.get(start..start + len).is_some(),
                    "{id} original UTF-8 source range",
                    id = project.definition["id"]
                );
                bounded_origin |= len > 0 && len < authored.len();
            }
        }
        assert!(
            bounded_origin,
            "{} retains a partial original source origin for {path}",
            project.definition["id"]
        );
    }
}

#[test]
fn vector_and_formula_builds_preserve_namespace_origins_and_independent_saved_copies() {
    for id in PROJECTS {
        let project = Project::new(id);
        let independent = Project::new(id);
        let graph = project.kind("transformation-graph");
        success(&project.run(graph));
        assert_origins(&project, graph);
        let entry = graph["entryArtifact"].as_str().unwrap();
        let baseline = fs::read(project.root.join(entry)).unwrap();
        let source = project.path(project.primary());
        let original = fs::read_to_string(&source).unwrap();
        let (find, replacement) = if id == "svg-diagram" {
            (">Source<", ">Café source<")
        } else {
            (">x<", ">y<")
        };
        let edited = original.replacen(find, replacement, 1);
        let edited = if id == "svg-diagram" {
            edited.replacen("width=\"80\"", "width=\"72\"", 1)
        } else {
            edited.replacen("x squared", "y squared", 1)
        };
        assert_ne!(edited, original, "{id} edit must exercise actual data");
        fs::write(&source, &edited).unwrap();
        success(&project.run(graph));
        assert_ne!(fs::read(project.root.join(entry)).unwrap(), baseline);
        assert_origins(&project, graph);
        success(&independent.run(independent.kind("transformation-graph")));
        assert_eq!(fs::read(independent.root.join(entry)).unwrap(), baseline);
        assert_eq!(
            fs::read_to_string(independent.path(independent.primary())).unwrap(),
            original
        );
        fs::write(&source, &original).unwrap();
        success(&project.run(graph));
        assert_eq!(fs::read(project.root.join(entry)).unwrap(), baseline);
    }
}
