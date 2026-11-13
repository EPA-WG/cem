//! Shared bootstrap requests produce runnable, movable Studio projects.
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Output, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

fn request(value: &Value) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .arg("project-bootstrap")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(value.to_string().as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cem-bootstrap-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn every_starter_builds_after_export_with_original_and_relocated_paths() {
    let catalog = success(request(&json!({"action":"catalog"})));
    for preset in catalog["presets"].as_array().unwrap() {
        for relocate in [false, true] {
            let dir = Directory::new();
            let mut config = success(request(&json!({"action":"preset", "preset":preset["id"],
                "projectId":"starter", "name":"Starter", "createdAt":"2026-11-13T00:00:00Z"})))
                ["configuration"]
                .clone();
            if relocate {
                for resource in config["resources"].as_array_mut().unwrap() {
                    resource["path"] =
                        format!("moved/it's a {}.source", resource["id"].as_str().unwrap()).into();
                }
            }
            let output = success(request(
                &json!({"action":"generate", "configuration":config}),
            ));
            fs::write(
                dir.0.join("project.cem"),
                output["manifest"].as_str().unwrap(),
            )
            .unwrap();
            for resource in config["resources"].as_array().unwrap() {
                let path = dir.0.join(resource["path"].as_str().unwrap());
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, resource["text"].as_str().unwrap()).unwrap();
            }
            for entry in output["bundle"]["project"]["entries"].as_array().unwrap() {
                let Some(id) = entry["runConfigResourceId"].as_str() else {
                    continue;
                };
                let resource = config["resources"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["id"] == id)
                    .unwrap();
                let command: Value =
                    serde_json::from_str(resource["text"].as_str().unwrap()).unwrap();
                let build = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
                    .args(
                        command["argv"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|v| v.as_str().unwrap()),
                    )
                    .args(["--resolver-project", "project.cem"])
                    .current_dir(&dir.0)
                    .output()
                    .unwrap();
                assert!(
                    build.status.success(),
                    "{} relocated={relocate}: {}",
                    preset["id"],
                    String::from_utf8_lossy(&build.stderr)
                );
                if let Some(artifact) = entry["entryArtifact"].as_str() {
                    let bytes = fs::read(dir.0.join(artifact)).unwrap();
                    assert!(!bytes.is_empty());
                    let text = String::from_utf8(bytes).unwrap();
                    assert!(
                        !text.contains('\u{1b}'),
                        "saved artifact contains terminal escapes"
                    );
                    if preset["id"] == "markdown-site" {
                        assert!(text.contains(">Welcome</h1>"), "{text}");
                    }
                    if preset["id"] == "transformation-pipeline" {
                        assert!(text.contains("CEM pipeline"), "{text}");
                        assert!(text.contains("Hello from CEM"), "{text}");
                    }
                    if preset["id"] == "simple-conversion" {
                        assert!(text.contains("Hello from CEM"), "{text}");
                    }
                }
            }
            for resource in config["resources"].as_array().unwrap() {
                assert_eq!(
                    fs::read(dir.0.join(resource["path"].as_str().unwrap())).unwrap(),
                    resource["text"].as_str().unwrap().as_bytes()
                );
            }
        }
    }
}

#[test]
fn invalid_request_has_structured_error_exit_and_no_partial_archive() {
    for request_value in [
        json!({"action":"preset","preset":"unknown","projectId":"test","name":"Test","createdAt":"2026-11-13T00:00:00Z"}),
        json!({"action":"generate","configuration":{}}),
        json!({"action":"catalog","extra":true}),
    ] {
        let output = request(&request_value);
        assert_eq!(output.status.code(), Some(2));
        let response: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(response.get("error").is_some());
        assert!(response.get("bundle").is_none());
    }
}

#[test]
fn planned_site_builds_nested_navigation_layout_css_and_exact_static_assets() {
    let plan = success(request(
        &json!({"action":"plan","preset":"markdown-site","projectId":"site-planned","name":"My site","createdAt":"2026-11-13T00:00:00Z", "answers":{"sourceFiles":"index.md\nabout.md\nguides/start.md","outputFolder":"public"}}),
    ));
    let dir = materialize_and_build(&plan["configuration"]);
    let nested = fs::read_to_string(dir.0.join("public/guides/start.html")).unwrap();
    assert!(
        nested.contains("<html") && nested.contains("<main>") && nested.contains("<header>"),
        "{nested}"
    );
    assert!(
        nested.contains("../index.html") && nested.contains("../guides/start.html"),
        "{nested}"
    );
    assert!(nested.contains("../assets/css/site.css"), "{nested}");
    assert!(nested.contains("Add your content here."), "{nested}");
    for path in ["assets/css/site.css", "static/logo.svg"] {
        assert_eq!(
            fs::read(dir.0.join(path)).unwrap(),
            fs::read(dir.0.join("public").join(path)).unwrap()
        );
    }
    assert!(dir.0.join("public/index.html").exists() && dir.0.join("public/about.html").exists());
}

#[test]
fn planned_pipeline_and_format_choices_build_all_targets() {
    for (preset, answers, targets) in [
        (
            "transformation-pipeline",
            json!({"sourceFiles":"in/a.xml\nin/nested/b.xml","targetFiles":"out/a.html\nout/nested/b.html"}),
            vec!["out/a.html", "out/nested/b.html"],
        ),
        (
            "simple-conversion",
            json!({"inputFormat":"json","outputFormat":"cem","sourceFiles":"in/source.json","targetFiles":"out/result.cem"}),
            vec!["out/result.cem"],
        ),
        (
            "simple-conversion",
            json!({"inputFormat":"yaml","outputFormat":"json","sourceFiles":"in/source.yaml","targetFiles":"out/result.json"}),
            vec!["out/result.json"],
        ),
        (
            "schema-validation",
            json!({"sourceFiles":"samples/a.xhtml\nsamples/b.xhtml"}),
            vec![],
        ),
    ] {
        let plan = success(request(
            &json!({"action":"plan","preset":preset,"projectId":"pipeline-planned","name":"Planned","createdAt":"2026-11-13T00:00:00Z","answers":answers}),
        ));
        let dir = materialize_and_build(&plan["configuration"]);
        for target in targets {
            assert!(!fs::read(dir.0.join(target)).unwrap().is_empty());
        }
    }
}

fn materialize_and_build(config: &Value) -> Directory {
    let generated = success(request(
        &json!({"action":"generate","configuration":config}),
    ));
    let dir = Directory::new();
    fs::write(
        dir.0.join("project.cem"),
        generated["manifest"].as_str().unwrap(),
    )
    .unwrap();
    for resource in config["resources"].as_array().unwrap() {
        let path = dir.0.join(resource["path"].as_str().unwrap());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, resource["text"].as_str().unwrap()).unwrap();
    }
    let command = config["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["role"] == "run-config")
        .unwrap();
    let command: Value = serde_json::from_str(command["text"].as_str().unwrap()).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .args(
            command["argv"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap()),
        )
        .args(["--resolver-project", "project.cem"])
        .current_dir(&dir.0)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    dir
}

#[test]
fn editable_commands_use_the_canonical_host_grammar_before_archive_publication() {
    let config = success(request(
        &json!({"action":"preset", "preset":"simple-conversion",
        "projectId":"starter", "name":"Starter", "createdAt":"2026-11-13T00:00:00Z"}),
    ))["configuration"]
        .clone();
    for argv in [
        json!(["convert", "--unknown-flag"]),
        json!(["convert", "--help"]),
        json!(["project-bootstrap"]),
        json!([
            "convert",
            ".resources/source",
            "--resolver-project",
            "project.cem"
        ]),
    ] {
        let mut invalid = config.clone();
        let command = invalid["resources"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["role"] == "run-config")
            .unwrap();
        let mut authored: Value = serde_json::from_str(command["text"].as_str().unwrap()).unwrap();
        authored["argv"] = argv;
        command["text"] = authored.to_string().into();
        let output = request(&json!({"action":"generate", "configuration":invalid}));
        assert_eq!(output.status.code(), Some(2));
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            result["error"]["code"],
            "cem.project_bootstrap.command_invalid"
        );
        assert!(result.get("bundle").is_none());
    }
}
