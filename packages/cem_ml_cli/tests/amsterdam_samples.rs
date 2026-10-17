use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

fn sample() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../examples/projects/login/example.json"
    ))
    .unwrap()
}

#[test]
fn presentation_login_commands_preserve_named_fields_and_report_existing_warnings() {
    let sample = sample();
    for page in sample["configurations"].as_array().unwrap() {
        let source = sample["sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|source| source["id"] == page["source"])
            .unwrap();
        let args: Vec<_> = page["argv"]
            .as_array()
            .unwrap()
            .iter()
            .map(|arg| match arg.as_str().unwrap() {
                "$source" => source["source"].as_str().unwrap(),
                value if value.starts_with('$') => sample["sources"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|source| source["id"] == &value[1..])
                    .unwrap()["source"]
                    .as_str()
                    .unwrap(),
                value => value,
            })
            .collect();
        let output = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .current_dir(workspace())
            .args(args)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(page["expectedExit"].as_i64().unwrap_or(0) as i32),
            "{}: {}",
            page["id"],
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(
            !text.contains('\u{1b}'),
            "saved presentation output is plain"
        );
        if let Some(expected) = page["expectedText"].as_array() {
            for expected in expected {
                assert!(
                    text.contains(expected.as_str().unwrap()),
                    "{}: {}",
                    page["id"],
                    expected
                );
            }
        } else {
            let report: serde_json::Value = serde_json::from_str(&text).unwrap();
            let expected = &page["expectedValidation"];
            assert_eq!(report["summary"]["errorCount"], expected["errorCount"]);
            assert_eq!(report["summary"]["warningCount"], expected["warningCount"]);
            assert!(report["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .all(|diagnostic| diagnostic["code"] == expected["warningCode"]
                    || diagnostic["code"] == expected["errorCode"]));
        }
    }
}

#[test]
fn presentation_schema_violation_has_a_retained_form_span_and_exact_repair() {
    let root = workspace();
    let fixture = root.join("examples/projects/login");
    let original = fs::read_to_string(root.join("examples/cem-ml/login.cem")).unwrap();
    let invalid = fs::read_to_string(fixture.join("login-invalid.cem")).unwrap();
    assert_eq!(invalid, original.replacen(" @method=post", "", 1));
    let parsed = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .args([
            "parse",
            fixture.join("login-invalid.cem").to_str().unwrap(),
            "--content-type",
            "application/cem",
            "--schema",
            "https://cem.dev/ns/cem-ml/1",
            "--format",
            "ast",
            "--no-color",
        ])
        .output()
        .unwrap();
    assert!(
        parsed.status.success(),
        "the failure must remain syntax-valid"
    );
    let validate = |path: &Path| {
        Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .args([
                "validate",
                path.to_str().unwrap(),
                "--content-type",
                "application/cem",
                "--schema",
                "urn:demo:login",
                "--schema-package",
                fixture.join("login-schema-package.cem").to_str().unwrap(),
                "--format",
                "json",
                "--no-color",
            ])
            .output()
            .unwrap()
    };
    let failed = validate(&fixture.join("login-invalid.cem"));
    assert_eq!(failed.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&failed.stdout).unwrap();
    assert_eq!(report["summary"]["errorCount"], 1);
    assert_eq!(report["summary"]["warningCount"], 3);
    let diagnostic = report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["code"] == "demo.login.method_required")
        .unwrap();
    assert_eq!(diagnostic["severity"], "error");
    assert_eq!(
        diagnostic["details"]["missingFields"],
        serde_json::json!(["method"])
    );
    let frame = diagnostic["sourceMap"]["frames"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    let range = &frame["span"]["ranges"];
    assert_eq!(range["start"], invalid.find("{form").unwrap());
    assert_eq!(range["len"], 5);
    assert_eq!(
        diagnostic["details"]["sourceMapCoordinates"]["frames"][1]["ranges"][0]["start"]["line"],
        8
    );
    let definition = sample();
    let repair = &definition["configurations"].as_array().unwrap().last().unwrap()["repair"];
    let repaired = invalid.replacen(
        repair["find"].as_str().unwrap(),
        repair["replace"].as_str().unwrap(),
        1,
    );
    assert_eq!(
        repaired, original,
        "repair restores only the missing attribute"
    );
    let directory = std::env::temp_dir().join(format!(
        "cem-amsterdam-repair-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("login-repaired.cem");
    fs::write(&path, repaired).unwrap();
    let success = validate(&path);
    fs::remove_dir_all(directory).unwrap();
    assert!(
        success.status.success(),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let repaired: serde_json::Value = serde_json::from_slice(&success.stdout).unwrap();
    assert_eq!(repaired["summary"]["errorCount"], 0);
    assert_eq!(repaired["summary"]["warningCount"], 3);
    assert!(repaired["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| item["code"] == "cem.ast.unresolved_reference"));
}

#[test]
fn presentation_heading_edit_stays_valid_and_keeps_both_form_controls() {
    let source = fs::read_to_string(workspace().join("examples/cem-ml/login.cem")).unwrap();
    let edited = source.replacen("Sign in}", "Amsterdam sign in}", 1);
    assert_ne!(edited, source);
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("cem-amsterdam-edit-{unique}"));
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("login.cem");
    fs::write(&path, edited).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .args([
            "parse",
            path.to_str().unwrap(),
            "--content-type",
            "application/cem",
            "--schema",
            "https://cem.dev/ns/cem-ml/1",
            "--fail-level",
            "validate",
            "--format",
            "ast",
            "--no-color",
        ])
        .output()
        .unwrap();
    fs::remove_dir_all(directory).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Amsterdam sign in"));
    assert_eq!(text.matches("@name=input ").count(), 2);
    assert!(text.contains("@name=name @value=email"));
    assert!(text.contains("@name=name @value=password"));
}
