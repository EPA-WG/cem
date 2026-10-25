//! HTML/CEM parity, shared declaration inspection and native card publication.
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
        &fs::read(workspace().join("examples/projects/html-cem-card/example.json")).unwrap(),
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
            "cem-html-cem-card-project-{}-{}",
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
fn html_cem_card_authored_commands_keep_semantic_parity_and_exact_publication() {
    let project = Project::new();
    let configurations = project.definition["configurations"].as_array().unwrap();
    assert_eq!(configurations.len(), 8);
    for configuration in configurations {
        let id = configuration["id"].as_str().unwrap();
        let output = project.run(id);
        assert_eq!(
            output.status.code(),
            Some(configuration["expectedExit"].as_i64().unwrap_or(0) as i32),
            "{id}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if configuration["expectedExit"].as_i64().unwrap_or(0) == 0 {
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                !stderr
                    .lines()
                    .any(|line| line.contains("error:") || line.contains("fatal:")),
                "{id} must not hide hard native diagnostics behind a successful exit: {stderr}"
            );
        }
        if id == "live-view" {
            let text = String::from_utf8_lossy(&output.stdout);
            assert!(!text.contains("@severity=error") && !text.contains("@severity=fatal"));
            let resource = project.definition["sources"]
                .as_array()
                .unwrap()
                .iter()
                .find(|resource| resource["id"] == configuration["source"])
                .unwrap();
            assert_eq!(
                resource["contentType"],
                "application/vnd.cem.element-template+cem"
            );
            assert_eq!(
                resource["schema"],
                "https://cem.dev/ns/template/cem-element/1"
            );
        }
        if let Some(expected_id) = configuration["expectedSemanticContract"].as_str() {
            let expected: serde_json::Value =
                serde_json::from_slice(&fs::read(project.path(expected_id)).unwrap()).unwrap();
            let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(xpath_values(&report), expected, "{id}");
        }
        if let Some(markers) = configuration["expectedText"].as_array() {
            let text = String::from_utf8_lossy(&output.stdout);
            assert!(!text.contains('\u{1b}'));
            for marker in markers {
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
            assert_eq!(configuration["entryArtifact"], "generated/card.html");
            for (path, expected_id) in outputs {
                assert_eq!(
                    fs::read(project.root.join(path)).unwrap(),
                    fs::read(project.path(expected_id.as_str().unwrap())).unwrap(),
                    "{id}: {path}"
                );
            }
        }
    }
    assert_eq!(
        xpath_values(&project.json("semantics-html")),
        xpath_values(&project.json("semantics-cem"))
    );
}

#[test]
fn html_cem_card_inspects_the_canonical_shared_declaration_without_expanding_dces() {
    let project = Project::new();
    let configuration = project.configuration("inspect-component");
    let resource_id = configuration["source"].as_str().unwrap();
    let canonical =
        workspace().join("packages/cem-components/src/components/cem-card/cem-card.xhtml");
    let canonical_source = fs::read_to_string(&canonical).unwrap();
    assert_eq!(
        fs::read(project.path(resource_id)).unwrap(),
        canonical_source.as_bytes()
    );
    let resource = project.definition["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|resource| resource["id"] == resource_id)
        .unwrap();
    assert_eq!(resource["contentType"], "application/xml");
    assert_eq!(resource["schema"], "https://cem.dev/ns/data/xml/1");
    assert!(
        configuration["argv"]
            .as_array()
            .unwrap()
            .windows(2)
            .any(|pair| { pair[0] == "--format" && pair[1] == "json" }),
        "JSON source inspection must be an explicitly named presentation boundary"
    );
    let output = project.run("inspect-component");
    success(&output);
    let nodes: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let declaration = nodes
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["kind"] == "element" && node["name"] == "cem-element")
        .unwrap();
    assert!(declaration["attributes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|attribute| { attribute["name"] == "tag" && attribute["value"] == "cem-card" }));
    let template = declaration["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["kind"] == "element" && node["name"] == "template")
        .unwrap();
    for (name, value) in [("id", "cem-card"), ("type", "text/cem-ml")] {
        assert!(template["attributes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|attribute| { attribute["name"] == name && attribute["value"] == value }));
    }
    let text = template["children"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["kind"] == "text")
        .map(|node| node["value"].as_str().unwrap())
        .collect::<String>();
    let expected = canonical_source
        .split_once("<template id=\"cem-card\" type=\"text/cem-ml\">")
        .unwrap()
        .1
        .split_once("</template>")
        .unwrap()
        .0;
    assert_eq!(
        text, expected,
        "inspection retains the complete authored CEM-ML as literal text"
    );
    assert!(text.contains("attribute @name=label"));
    assert!(text.contains("var(--cem-palette-comfort)"));
    assert!(text.contains('\'') && text.contains('"') && text.contains("```"));
    // This explicitly exported source AST never becomes a renderer's data handoff.
    assert_eq!(template["children"].as_array().unwrap().len(), 1);
}

#[test]
fn html_event_handler_policy_identifies_original_bytes_and_repairs_before_publication() {
    let project = Project::new();
    let configuration = project.configuration("repair-html-policy");
    let path = project.path(configuration["source"].as_str().unwrap());
    let original = fs::read_to_string(&path).unwrap();
    let output = project.run("repair-html-policy");
    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["summary"]["errorCount"], 1);
    assert_eq!(report["summary"]["warningCount"], 0);
    let diagnostic = report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["code"] == "cem.html.event_handler_rejected")
        .unwrap();
    assert_eq!(diagnostic["uri"], path.to_str().unwrap());
    assert_eq!(diagnostic["severity"], "error");
    for (key, value) in configuration["expectedValidation"]["location"]
        .as_object()
        .unwrap()
    {
        assert_eq!(&diagnostic[key], value, "original diagnostic {key}");
    }
    let offset = diagnostic["byteOffset"].as_u64().unwrap() as usize;
    assert!(offset < original.len());
    assert!(diagnostic["sourceMap"]["frames"]
        .as_array()
        .is_some_and(|frames| !frames.is_empty()));

    let source_id = project.configuration("semantics-html")["source"]
        .as_str()
        .unwrap();
    let source_path = project.path(source_id);
    let valid_source = fs::read_to_string(&source_path).unwrap();
    fs::write(&source_path, &original).unwrap();
    let failed_build = project.run("build-html");
    assert!(!failed_build.status.success());
    assert!(!project.root.join("generated/card.html").exists());
    let repair = &configuration["repair"];
    let repaired = original.replacen(
        repair["find"].as_str().unwrap(),
        repair["replace"].as_str().unwrap(),
        1,
    );
    assert_ne!(original, repaired);
    assert_eq!(repaired, valid_source);
    fs::write(path, &repaired).unwrap();
    let fixed = project.json("repair-html-policy");
    assert_eq!(fixed["summary"]["errorCount"], 0);
    assert_eq!(fixed["summary"]["warningCount"], 0);
    fs::write(source_path, repaired).unwrap();
    success(&project.run("build-html"));
}

fn assert_template_literal_origin(project: &Project, template_path: &Path, literal: &str) {
    let html = fs::read_to_string(project.root.join("generated/card.html")).unwrap();
    let map: serde_json::Value =
        serde_json::from_slice(&fs::read(project.root.join("generated/card.html.map")).unwrap())
            .unwrap();
    let template = fs::read_to_string(template_path).unwrap();
    let output_offset = html.find(literal).unwrap() as u64;
    let template_name = template_path
        .strip_prefix(&project.root)
        .unwrap()
        .to_str()
        .unwrap();
    let mut mapped = false;
    for span in map["outputSpans"].as_array().unwrap() {
        let start = span["outputRange"]["start"].as_u64().unwrap();
        let length = span["outputRange"]["len"].as_u64().unwrap();
        assert!(start + length <= html.len() as u64);
        if !(start <= output_offset && output_offset < start + length) {
            continue;
        }
        for frame in span["origin"]["frames"].as_array().unwrap() {
            if frame["transform"]["kind"] != "SourceBoundary"
                || !frame["transform"]["uri"]
                    .as_str()
                    .is_some_and(|uri| uri.ends_with(template_name))
                || frame["span"]["kind"] != "Single"
            {
                continue;
            }
            let range = &frame["span"]["ranges"];
            let from = range["start"].as_u64().unwrap() as usize;
            let to = from + range["len"].as_u64().unwrap() as usize;
            let authored = template
                .get(from..to)
                .expect("original UTF-8 template range");
            if authored.contains(literal) && to - from < template.len() {
                mapped = true;
            }
        }
    }
    assert!(
        mapped,
        "the literal needs an original template instruction range"
    );
}

#[test]
fn html_cem_card_saved_source_and_template_edits_keep_copies_and_provenance_independent() {
    let project = Project::new();
    let other = Project::new();
    let html_id = project.configuration("semantics-html")["source"]
        .as_str()
        .unwrap();
    let cem_id = project.configuration("semantics-cem")["source"]
        .as_str()
        .unwrap();
    let html_path = project.path(html_id);
    let cem_path = project.path(cem_id);
    let html_source = fs::read_to_string(&html_path).unwrap();
    let cem_source = fs::read_to_string(&cem_path).unwrap();
    let template_resource = project.definition["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|resource| {
            resource["role"] == "template"
                && resource["path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with(".cemt"))
        })
        .unwrap();
    let template_path = project.path(template_resource["id"].as_str().unwrap());
    let original_template = fs::read_to_string(&template_path).unwrap();
    let output_id = project.configuration("build-html")["expectedGraphOutputs"]
        ["generated/card.html"]
        .as_str()
        .unwrap();
    let expected = fs::read_to_string(project.path(output_id)).unwrap();
    let original_contract = xpath_values(&project.json("semantics-html"));
    assert_eq!(
        xpath_values(&project.json("semantics-cem")),
        original_contract
    );
    success(&project.run("build-html"));
    success(&other.run("build-html"));
    let caption = "Rendered from saved HTML data.";
    assert_template_literal_origin(&project, &template_path, caption);

    let edited_html = html_source.replace("Harbor field notes", "Café &amp; friends");
    let edited_cem = cem_source.replace("Harbor field notes", "Café & friends");
    assert_ne!(edited_html, html_source);
    assert_ne!(edited_cem, cem_source);
    fs::write(&html_path, &edited_html).unwrap();
    fs::write(&cem_path, &edited_cem).unwrap();
    let contract = xpath_values(&project.json("semantics-html"));
    assert_ne!(contract, original_contract);
    assert_eq!(xpath_values(&project.json("semantics-cem")), contract);
    assert!(contract.as_array().unwrap().iter().any(|item| {
        item.as_str()
            .is_some_and(|value| value.contains("Café & friends"))
    }));
    let edited_template = original_template.replace(caption, "Rendered from this saved project.");
    assert_ne!(edited_template, original_template);
    fs::write(&template_path, &edited_template).unwrap();
    success(&project.run("build-html"));
    let output = fs::read_to_string(project.root.join("generated/card.html")).unwrap();
    assert_eq!(
        output,
        expected
            .replace("Harbor field notes", "Café &amp; friends")
            .replace(caption, "Rendered from this saved project.")
    );
    assert_template_literal_origin(
        &project,
        &template_path,
        "Rendered from this saved project.",
    );
    assert_eq!(
        xpath_values(&other.json("semantics-html")),
        original_contract
    );
    assert_eq!(
        fs::read_to_string(other.path(html_id)).unwrap(),
        html_source
    );
    assert_eq!(fs::read_to_string(other.path(cem_id)).unwrap(), cem_source);
    assert_eq!(
        fs::read_to_string(other.root.join("generated/card.html")).unwrap(),
        expected
    );
    for (id, original) in [(html_id, html_source), (cem_id, cem_source)] {
        let resource = project.definition["sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|resource| resource["id"] == id)
            .unwrap();
        assert_eq!(
            fs::read_to_string(workspace().join(resource["source"].as_str().unwrap())).unwrap(),
            original
        );
    }
    let first = fs::read(project.root.join("generated/card.html")).unwrap();
    success(&project.run("build-html"));
    assert_eq!(
        fs::read(project.root.join("generated/card.html")).unwrap(),
        first
    );
}
