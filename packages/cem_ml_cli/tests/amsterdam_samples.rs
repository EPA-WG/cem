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
        "../../cem-studio/fixtures/amsterdam-login/sample.json"
    ))
    .unwrap()
}

#[test]
fn presentation_login_commands_preserve_named_fields_and_report_existing_warnings() {
    let sample = sample();
    for page in sample["pages"].as_array().unwrap() {
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
                value => value,
            })
            .collect();
        let output = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .current_dir(workspace())
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
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
                .all(|diagnostic| diagnostic["code"] == expected["warningCode"]));
        }
    }
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
