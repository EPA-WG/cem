//! Login live templates use the registered runtime-binding source contract.
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
const ELEMENT_TYPE: &str = "application/vnd.cem.element-template+cem";
const ELEMENT_SCHEMA: &str = "https://cem.dev/ns/template/cem-element/1";
fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .into()
}
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cem-login-source-identity-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn execute(
    operation: &str,
    source: &Path,
    media: &str,
    schema: &str,
    directory: &Path,
) -> (std::process::Output, Value) {
    let report = directory.join(format!("{operation}.report.json"));
    let result = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .args([
            operation,
            source.to_str().unwrap(),
            "--content-type",
            media,
            "--schema",
            schema,
            "--format",
            if operation == "parse" { "ast" } else { "json" },
            "--no-color",
            "--report-json",
            report.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let report = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
    (result, report)
}
#[test]
fn authored_login_live_source_and_command_use_the_element_template_contract() {
    let root = workspace();
    let definition: Value = serde_json::from_slice(
        &fs::read(root.join("examples/projects/login/example.json")).unwrap(),
    )
    .unwrap();
    let resource = definition["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "login-live-view")
        .unwrap();
    let configuration = definition["configurations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "live-view")
        .unwrap();
    let path = root.join(resource["source"].as_str().unwrap());
    let original = fs::read(&path).unwrap();
    let scratch = Scratch::new();
    // Execute the actual authored selectors first: zero Parse exit alone cannot
    // turn its original hard binding diagnostics into passing source validation.
    for operation in ["parse", "validate"] {
        let (output, report) = execute(
            operation,
            &path,
            resource["contentType"].as_str().unwrap(),
            resource["schema"].as_str().unwrap(),
            &scratch.0,
        );
        assert!(output.status.success(), "{operation}: {report}");
        let diagnostics = report["diagnostics"].as_array().unwrap();
        assert!(
            !diagnostics
                .iter()
                .any(|item| item["severity"] == "error" || item["severity"] == "fatal"),
            "{operation}: {report}"
        );
        assert_eq!(
            diagnostics.len(),
            15,
            "retain advisory open-content attribute diagnostics"
        );
        assert!(diagnostics.iter().all(|item| item["severity"] == "warning"
            && item["code"] == "cem.schema.unknown_html_attribute"));
        assert!(diagnostics
            .iter()
            .all(|item| item["uri"] == path.to_str().unwrap() && item["byteOffset"] == 0));
        if operation == "parse" {
            let text = String::from_utf8(output.stdout).unwrap();
            for expected in ["@name=cem-text-field", "@name=cem-action", "@byte-offset="] {
                assert!(text.contains(expected), "{text}");
            }
        }
    }
    assert_eq!(resource["contentType"], ELEMENT_TYPE);
    assert_eq!(resource["schema"], ELEMENT_SCHEMA);
    assert_eq!(resource["path"], "views/login.cem");
    assert_eq!(resource["role"], "template");
    let argv = configuration["argv"].as_array().unwrap();
    for (flag, value) in [
        ("--content-type", ELEMENT_TYPE),
        ("--schema", ELEMENT_SCHEMA),
    ] {
        assert!(argv
            .windows(2)
            .any(|pair| pair[0] == flag && pair[1] == value));
    }
    for (id, media, schema) in [
        (
            "login-cem",
            "application/cem",
            "https://cem.dev/ns/cem-ml/1",
        ),
        (
            "login-view-template",
            "application/vnd.cem.template+cem",
            "https://cem.dev/ns/template/cem-native/1",
        ),
    ] {
        let source = definition["sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == id)
            .unwrap();
        assert_eq!(source["contentType"], media);
        assert_eq!(source["schema"], schema);
    }
    assert_eq!(fs::read(path).unwrap(), original);
}
#[test]
fn legacy_generic_login_source_keeps_its_original_binding_errors_and_ranges() {
    let path = workspace().join("examples/projects/login/views/login.cem");
    let scratch = Scratch::new();
    for operation in ["parse", "validate"] {
        let (output, report) = execute(
            operation,
            &path,
            "application/cem",
            "https://cem.dev/ns/cem-ml/1",
            &scratch.0,
        );
        assert_eq!(
            output.status.code(),
            Some(if operation == "parse" { 0 } else { 1 })
        );
        let diagnostics = report["diagnostics"].as_array().unwrap();
        assert_eq!(diagnostics.len(), 4);
        for (item, (start, len)) in
            diagnostics
                .iter()
                .zip([(196, 184), (657, 38), (1147, 23), (1187, 48)])
        {
            assert_eq!(item["severity"], "error");
            assert_eq!(item["code"], "cem.ql.compile_failed");
            assert_eq!(item["uri"], path.to_str().unwrap());
            assert_eq!(item["byteOffset"], start);
            assert!(item["line"].is_null());
            assert!(item["column"].is_null());
            let frames = item["sourceMap"]["frames"].as_array().unwrap();
            let frame = frames.last().unwrap();
            assert_eq!(frame["transform"]["kind"], "TemplateEmbedding");
            assert_eq!(
                frame["span"]["ranges"],
                serde_json::json!({"start":start,"len":len})
            );
            assert_eq!(frame["transform"]["host"], frame["span"]["ranges"]);
        }
    }
}

#[test]
fn element_template_identity_still_rejects_an_undeclared_login_binding() {
    let scratch = Scratch::new();
    let path = scratch.0.join("login.cem");
    let original =
        fs::read_to_string(workspace().join("examples/projects/login/views/login.cem")).unwrap();
    let broken = original.replace("dom:text(heading)", "dom:text(not_declared)");
    assert_ne!(broken, original);
    fs::write(&path, &broken).unwrap();
    for operation in ["validate", "check"] {
        let (output, report) = execute(operation, &path, ELEMENT_TYPE, ELEMENT_SCHEMA, &scratch.0);
        assert_eq!(output.status.code(), Some(1));
        let errors: Vec<_> = report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["severity"] == "error")
            .collect();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0]["code"], "cem.ql.render.compile_failed");
        assert_eq!(errors[0]["uri"], path.to_str().unwrap());
        assert_eq!(
            errors[0]["byteOffset"],
            broken.find("{$str:normalize_space").unwrap()
        );
        assert!(errors[0]["message"]
            .as_str()
            .unwrap()
            .contains("not_declared"));
    }
    fs::write(&path, original).unwrap();
    assert!(
        execute("validate", &path, ELEMENT_TYPE, ELEMENT_SCHEMA, &scratch.0)
            .0
            .status
            .success()
    );
}
