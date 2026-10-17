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
fn login_html_and_cem_queries_have_the_same_explicit_semantic_contract() {
    let root = workspace();
    let query = root.join("examples/projects/login/queries/semantics.xpath");
    let expected = [
        "screen=login",
        "title-id=login-title",
        "title=Sign in",
        "labelled-by=login-title",
        "form=sign-in",
        "method=post",
        "action=#session",
        "email-id=email",
        "email-type=email",
        "email-autocomplete=email",
        "email-required=true",
        "email-label=Email",
        "password-id=password",
        "password-type=password",
        "password-autocomplete=current-password",
        "password-required=true",
        "password-label=Password",
        "submit-type=submit",
        "submit-action=primary",
        "submit-text=Sign in",
        "inputs=2",
    ];
    for (path, content_type, schema) in [
        (
            "examples/semantic/login.html",
            "text/html",
            "https://cem.dev/ns/data/html/1",
        ),
        (
            "examples/cem-ml/login.cem",
            "application/cem",
            "https://cem.dev/ns/cem-ml/1",
        ),
    ] {
        let path = root.join(path);
        let original = fs::read(&path).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .args([
                "query",
                path.to_str().unwrap(),
                "--content-type",
                content_type,
                "--schema",
                schema,
                "--query-file",
                query.to_str().unwrap(),
                "--query-content-type",
                "application/vnd.cem.xpath",
                "--query-schema",
                "https://cem.dev/ns/query/xpath/1",
                "--output",
                "json",
            ])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}: {} {}",
            content_type,
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        let result: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        let values: Vec<_> = result["result"]["sequence"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["value"]["lexicalValue"].as_str().unwrap())
            .collect();
        assert_eq!(values, expected, "{content_type}");
        assert_eq!(fs::read(&path).unwrap(), original);
        let source = String::from_utf8(original).unwrap();
        let edited = source.replacen("Sign in", "Welcome", 1).replacen(
            if content_type == "text/html" {
                "method=\"post\""
            } else {
                "@method=post"
            },
            if content_type == "text/html" {
                "method=\"get\""
            } else {
                "@method=get"
            },
            1,
        );
        let directory = std::env::temp_dir().join(format!(
            "cem-login-compare-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let edited_path = directory.join(if content_type == "text/html" {
            "login.html"
        } else {
            "login.cem"
        });
        fs::write(&edited_path, &edited).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .args([
                "query",
                edited_path.to_str().unwrap(),
                "--content-type",
                content_type,
                "--schema",
                schema,
                "--query-file",
                query.to_str().unwrap(),
                "--query-content-type",
                "application/vnd.cem.xpath",
                "--query-schema",
                "https://cem.dev/ns/query/xpath/1",
                "--output",
                "json",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let changed: Vec<_> = result["result"]["sequence"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["value"]["lexicalValue"].as_str().unwrap())
            .collect();
        let expected_changed: Vec<_> = expected
            .iter()
            .map(|value| match *value {
                "title=Sign in" => "title=Welcome",
                "method=post" => "method=get",
                value => value,
            })
            .collect();
        assert_eq!(changed, expected_changed);
        assert_eq!(fs::read_to_string(&edited_path).unwrap(), edited);
        fs::remove_dir_all(directory).unwrap();
    }
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
        } else if let Some(contract) = page["expectedSemanticContract"].as_str() {
            let source = sample["sources"]
                .as_array()
                .unwrap()
                .iter()
                .find(|source| source["id"] == contract)
                .unwrap();
            let expected: serde_json::Value = serde_json::from_slice(
                &fs::read(workspace().join(source["source"].as_str().unwrap())).unwrap(),
            )
            .unwrap();
            let report: serde_json::Value = serde_json::from_str(&text).unwrap();
            let values: Vec<_> = report["result"]["sequence"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| item["value"]["lexicalValue"].clone())
                .collect();
            assert_eq!(serde_json::Value::Array(values), expected);
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
    let repair = &definition["configurations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|configuration| configuration["id"] == "validation-repair")
        .unwrap()["repair"];
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
