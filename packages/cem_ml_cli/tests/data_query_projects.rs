//! Ordinary data-query projects retain native values, source owners and controls.
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

const PROJECTS: [&str; 2] = ["cemql-orders", "xpath-catalog"];
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
            "cem-data-query-{id}-{}-{}",
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
            .unwrap()
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
            .unwrap()
    }
    fn run(&self, config: &Value) -> Output {
        let args = config["argv"].as_array().unwrap().iter().map(|arg| {
            let arg = arg.as_str().unwrap();
            match if arg == "$source" {
                config["source"].as_str()
            } else {
                arg.strip_prefix('$')
            } {
                Some(id) => self.path(id).to_string_lossy().into_owned(),
                None => arg.to_owned(),
            }
        });
        Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn xpath_values(project: &Project, configuration: &str) -> Value {
    let output = project.run(project.config(configuration));
    success(&output);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["language"], "xpath");
    Value::Array(
        native_items(&report)
            .as_array()
            .unwrap()
            .iter()
            .map(|item| {
                assert_eq!(item["kind"], "atomic");
                item["value"]["lexicalValue"].clone()
            })
            .collect(),
    )
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
fn native_items(report: &Value) -> &Value {
    if report["language"] == "xpath" {
        &report["result"]["sequence"]["items"]
    } else if report["language"] == "cem-ql" {
        &report["result"]["items"]
    } else {
        panic!("unexpected query result identity: {report}")
    }
}
fn original_slice<'a>(source: &'a str, start: u64, len: u64) -> &'a str {
    source
        .get(start as usize..(start + len) as usize)
        .expect("native range stays within original UTF-8 boundaries")
}
fn assert_native_source_nodes(project: &Project, config: &Value, report: &Value) {
    let source_id = config["source"].as_str().unwrap();
    let source = String::from_utf8(project.read(source_id)).unwrap();
    let source_uri = project.path(source_id).to_string_lossy().into_owned();
    let items = native_items(report).as_array().unwrap();
    if let Some(expected) = config["expectedNativeNodes"].as_str() {
        let selected: Vec<Value> = items.iter().map(|item| {
            assert_eq!(item["kind"], "node");
            assert_eq!(item["sourceUri"], source_uri);
            let start = item["sourceRange"]["start"]["byteOffset"].as_u64().unwrap();
            let len = item["sourceRange"]["byteLength"].as_u64().unwrap();
            let original = original_slice(&source, start, len);
            assert!(original.starts_with('<') && original.ends_with('>') && original.len() < source.len());
            assert!(item["sourceMap"]["frames"].as_array().unwrap().iter().any(|frame| frame["transform"]["kind"] == "SourceBoundary" && frame["transform"]["uri"] == source_uri
                && frame["span"]["ranges"]["start"] == start && frame["span"]["ranges"]["len"] == len));
            json!({ "kind": item["kind"], "nodeId": item["nodeId"], "nodeKind": item["nodeKind"],
                "expandedName": item["expandedName"], "sourceRange": item["sourceRange"] })
        }).collect();
        assert_eq!(
            Value::Array(selected),
            serde_json::from_slice::<Value>(&project.read(expected)).unwrap()
        );
    }
    if let Some(expected) = config["expectedNativeRanges"].as_str() {
        let ranges: Vec<Value> = items
            .iter()
            .map(|item| {
                assert_eq!(item["kind"], "native");
                assert_eq!(item["representation"], "cem.ql.imported-cem-ast");
                let frame = item["sourceMap"]["frames"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|frame| {
                        frame["transform"]["kind"] == "SourceBoundary"
                            && frame["transform"]["uri"] == source_uri
                    })
                    .unwrap();
                let range = &frame["span"]["ranges"];
                let original = original_slice(
                    &source,
                    range["start"].as_u64().unwrap(),
                    range["len"].as_u64().unwrap(),
                );
                assert!(
                    original.starts_with('{')
                        && original.ends_with('}')
                        && original.len() < source.len()
                );
                range.clone()
            })
            .collect();
        assert_eq!(
            Value::Array(ranges),
            serde_json::from_slice::<Value>(&project.read(expected)).unwrap()
        );
    }
}

#[test]
fn authored_query_projects_keep_typed_values_exact_exports_and_owned_node_ranges() {
    for id in PROJECTS {
        let project = Project::new(id);
        assert_eq!(
            project.configs().len(),
            if id == "cemql-orders" { 14 } else { 7 }
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
            if let Some(expected) = config["expectedNativeItems"].as_str() {
                let report: Value = serde_json::from_slice(&output.stdout).unwrap();
                assert_eq!(
                    native_items(&report),
                    &serde_json::from_slice::<Value>(&project.read(expected)).unwrap()
                );
            }
            if let Some(expected) = config["expectedSemanticContract"].as_str() {
                let report: Value = serde_json::from_slice(&output.stdout).unwrap();
                let values: Vec<Value> = native_items(&report)
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|item| {
                        assert_eq!(item["kind"], "atomic");
                        item["value"]["lexicalValue"].clone()
                    })
                    .collect();
                assert_eq!(
                    Value::Array(values),
                    serde_json::from_slice::<Value>(&project.read(expected)).unwrap()
                );
            }
            if config["expectedNativeNodes"].is_string()
                || config["expectedNativeRanges"].is_string()
            {
                let report = serde_json::from_slice(&output.stdout).unwrap();
                assert_native_source_nodes(&project, config, &report);
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
                        project.read(expected.as_str().unwrap()),
                        "{id}/{path}"
                    );
                }
            }
        }
    }
}

#[test]
fn native_query_diagnostics_repair_the_original_source_without_changing_catalog_bytes() {
    for id in PROJECTS {
        let project = Project::new(id);
        let config = project
            .configs()
            .iter()
            .find(|config| config["repair"].is_object())
            .unwrap();
        let source_id = config["source"].as_str().unwrap();
        let invalid = String::from_utf8(project.read(source_id)).unwrap();
        let output = project.run(config);
        assert!(!output.status.success());
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        let expected = &config["expectedValidation"];
        let diagnostic = report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["code"] == expected["errorCode"])
            .unwrap();
        assert_eq!(diagnostic["severity"], "error");
        assert_eq!(
            diagnostic["uri"],
            project.path(source_id).to_string_lossy().as_ref()
        );
        for (key, value) in expected["location"].as_object().unwrap() {
            assert_eq!(&diagnostic[key], value, "{id}/{key}");
        }
        let repaired = invalid.replacen(
            config["repair"]["find"].as_str().unwrap(),
            config["repair"]["replace"].as_str().unwrap(),
            1,
        );
        assert_ne!(repaired, invalid);
        fs::write(project.path(source_id), repaired).unwrap();
        let fixed = project.run(config);
        success(&fixed);
        let report: Value = serde_json::from_slice(&fixed.stdout).unwrap();
        assert_eq!(report["summary"]["hardViolationCount"], 0);
        let catalog = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(project.source(source_id)["source"].as_str().unwrap());
        assert_eq!(fs::read_to_string(catalog).unwrap(), invalid);
    }
}

#[test]
fn native_query_data_and_parameter_changes_stay_independent_and_keep_mapping_limits() {
    let project = Project::new("cemql-orders");
    let independent = Project::new("cemql-orders");
    let ready = project.config("report-ready");
    let draft = project.config("report-draft");
    success(&project.run(ready));
    success(&project.run(draft));
    let ready_path = ready["entryArtifact"].as_str().unwrap();
    let baseline = fs::read(project.root.join(ready_path)).unwrap();
    let draft_bytes =
        fs::read(project.root.join(draft["entryArtifact"].as_str().unwrap())).unwrap();
    assert_ne!(baseline, draft_bytes);
    for config in [ready, draft] {
        let paths = config["expectedUnmappedOutputs"]
            .as_array()
            .expect("computed JSON outputs declare native mapping availability");
        assert!(!paths.is_empty());
        for path in paths {
            let target = project.root.join(path.as_str().unwrap());
            assert!(target.exists());
            assert!(
                !PathBuf::from(format!("{}.map", target.display())).exists(),
                "the native JSON writer does not publish invented per-character lineage"
            );
        }
    }
    let source = project.path("order-data");
    let original = fs::read_to_string(&source).unwrap();
    let changed = original.replace("Café Noord", "Café Noord edited");
    assert_ne!(changed, original);
    fs::write(&source, changed).unwrap();
    success(&project.run(ready));
    assert_ne!(fs::read(project.root.join(ready_path)).unwrap(), baseline);
    success(&independent.run(independent.config("report-ready")));
    assert_eq!(
        fs::read(independent.root.join(ready_path)).unwrap(),
        baseline
    );
    fs::write(source, original).unwrap();
    success(&project.run(ready));
    assert_eq!(fs::read(project.root.join(ready_path)).unwrap(), baseline);
    let graph_path = project.path(ready["source"].as_str().unwrap());
    let graph_source = fs::read_to_string(&graph_path).unwrap();
    let updated = graph_source
        .replace("@value=ready", "@value=draft")
        .replace("@value=30", "@value=0");
    assert_ne!(updated, graph_source);
    fs::write(&graph_path, updated).unwrap();
    success(&project.run(ready));
    assert_eq!(
        fs::read(project.root.join(ready_path)).unwrap(),
        draft_bytes
    );
    fs::write(graph_path, graph_source).unwrap();
    for pair in ready["expectedGraphFailures"]
        .as_array()
        .into_iter()
        .flatten()
    {
        fs::remove_dir_all(project.root.join("generated")).unwrap();
        let primary = project.path(pair[0].as_str().unwrap());
        let original = fs::read(&primary).unwrap();
        fs::write(&primary, project.read(pair[1].as_str().unwrap())).unwrap();
        assert!(!project.run(ready).status.success());
        for path in ready["expectedGraphOutputs"].as_object().unwrap().keys() {
            assert!(
                !project.root.join(path).exists(),
                "invalid input cannot publish {path}"
            );
        }
        fs::write(primary, original).unwrap();
        success(&project.run(ready));
    }
}

#[test]
fn xpath_owned_parameter_maps_and_query_edits_rebuild_from_their_original_sources() {
    let project = Project::new("xpath-catalog");
    let independent = Project::new("xpath-catalog");
    let baseline = xpath_values(&project, "query-parameters");
    assert_eq!(
        baseline,
        serde_json::from_slice::<Value>(&project.read("parameter-summary")).unwrap()
    );
    let source = project.path("catalog-source");
    let original = fs::read_to_string(&source).unwrap();
    let updated = original.replace("minimum-stock=\"4\"", "minimum-stock=\"6\"");
    assert_ne!(updated, original);
    fs::write(&source, updated).unwrap();
    assert_eq!(
        xpath_values(&project, "query-parameters"),
        json!([
            "minimum-stock=6",
            "region=west",
            "tea:Green tea:Café Harbor"
        ])
    );
    assert_eq!(xpath_values(&independent, "query-parameters"), baseline);
    // The independent literal filter remains independent from the owned map.
    assert_eq!(
        xpath_values(&project, "query-filter"),
        serde_json::from_slice::<Value>(&project.read("filter-summary")).unwrap()
    );
    fs::write(&source, &original).unwrap();
    assert_eq!(xpath_values(&project, "query-parameters"), baseline);
    let query = project.path("filter-query");
    let original_query = fs::read_to_string(&query).unwrap();
    let updated_query = original_query.replace("@stock >= 4", "@stock >= 6");
    assert_ne!(updated_query, original_query);
    fs::write(&query, updated_query).unwrap();
    assert_eq!(
        xpath_values(&project, "query-filter"),
        json!(["tea:Green tea"])
    );
    assert_eq!(
        xpath_values(&independent, "query-filter"),
        serde_json::from_slice::<Value>(&project.read("filter-summary")).unwrap()
    );
    fs::write(&query, original_query).unwrap();
    let invalid = project.read("invalid-query");
    fs::write(&query, invalid).unwrap();
    let refused = project.run(project.config("query-filter"));
    assert!(!refused.status.success());
    assert!(
        refused.stdout.is_empty(),
        "invalid query source must not publish a replacement result"
    );
}
