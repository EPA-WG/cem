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

fn scratch() -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "cem-login-template-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    directory
}

fn render(source: &Path, media: &str, schema: &str, template: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .args([
            "transform",
            source.to_str().unwrap(),
            "--data-content-type",
            media,
            "--data-schema",
            schema,
            "--template",
            template.to_str().unwrap(),
            "--template-content-type",
            "application/vnd.cem.template+cem",
            "--template-schema",
            "https://cem.dev/ns/template/cem-native/1",
            "--template-entrypoint",
            "main",
            "--to-content-type",
            "text/html",
            "--to-schema",
            "https://cem.dev/ns/data/html/1",
            "--output-color-type",
            "none",
            "--scope-budget",
            "queueSize=1024",
        ])
        .output()
        .unwrap()
}

fn html(output: std::process::Output) -> String {
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

#[test]
fn login_template_projects_both_sources_edits_presence_and_escaped_values() {
    let root = workspace();
    let template = root.join("examples/projects/login/templates/view.cemt");
    assert!(
        template.exists(),
        "Login view template must be a real project resource"
    );
    let expected = fs::read_to_string(root.join("examples/projects/login/expected/view.html"))
        .unwrap()
        .trim()
        .to_owned();
    let directory = scratch();
    for (canonical, media, schema, required, method) in [
        (
            "examples/semantic/login.html",
            "text/html",
            "https://cem.dev/ns/data/html/1",
            " required",
            "method=\"post\"",
        ),
        (
            "examples/cem-ml/login.cem",
            "application/cem",
            "https://cem.dev/ns/cem-ml/1",
            " @required",
            "@method=post",
        ),
    ] {
        let original = fs::read_to_string(root.join(canonical)).unwrap();
        let source = directory.join(if media == "text/html" {
            "login.html"
        } else {
            "login.cem"
        });
        let edited = original
            .replacen("Sign in", "Amsterdam sign in", 1)
            .replacen("Email", "Email address", 1)
            .replacen(
                method,
                if media == "text/html" {
                    "method=\"get\""
                } else {
                    "@method=get"
                },
                1,
            );
        let escaped = original.replacen(
            "Sign in",
            if media == "text/html" {
                "Welcome &lt;team&gt; &amp; friends"
            } else {
                "Welcome <team> & friends"
            },
            1,
        );
        let escaped_action = original.replacen(
            if media == "text/html" {
                "action=\"#session\""
            } else {
                "@action=\"#session\""
            },
            if media == "text/html" {
                "action=\"#session?next=&lt;login&gt;&amp;mode=&quot;demo&quot;\""
            } else {
                "@action='#session?next=<login>&mode=\"demo\"'"
            },
            1,
        );
        let mut variants = vec![
            (original.clone(), expected.clone()),
            (
                edited,
                expected
                    .replacen("Sign in", "Amsterdam sign in", 1)
                    .replacen("Email", "Email address", 1)
                    .replacen("method=\"post\"", "method=\"get\"", 1),
            ),
            (
                original.replacen(required, &format!("{required}=\"\""), 1),
                expected.clone(),
            ),
            (
                original.replacen(required, "", 1),
                expected.replacen(" required", "", 1),
            ),
            (
                escaped,
                expected.replacen("Sign in", "Welcome &lt;team&gt; &amp; friends", 1),
            ),
            (
                escaped_action,
                expected.replace(
                    "action=\"#session\"",
                    "action=\"#session?next=&lt;login&gt;&amp;mode=&quot;demo&quot;\"",
                ),
            ),
        ];
        if media == "application/cem" {
            variants.push((
                original
                    .replace("@ns html =", "@ns web =")
                    .replace("@default html", "@default web")
                    .replace("@ns cem =", "@ns core =")
                    .replace("@cem:", "@core:"),
                expected.clone(),
            ));
        }
        for (value, expected) in variants {
            fs::write(&source, &value).unwrap();
            assert_eq!(
                html(render(&source, media, schema, &template)),
                expected,
                "{media}"
            );
            assert_eq!(fs::read_to_string(&source).unwrap(), value);
        }
        assert_eq!(fs::read_to_string(root.join(canonical)).unwrap(), original);
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn login_template_edits_change_only_the_instruction_and_report_template_errors() {
    let root = workspace();
    let original =
        fs::read_to_string(root.join("examples/projects/login/templates/view.cemt")).unwrap();
    let expected = fs::read_to_string(root.join("examples/projects/login/expected/view.html"))
        .unwrap()
        .trim()
        .to_owned();
    let directory = scratch();
    let template = directory.join("view.cemt");
    let edited = original.replace(
        "Use your email and password.",
        "Use your Amsterdam account.",
    );
    assert_ne!(edited, original);
    fs::write(&template, &edited).unwrap();
    assert_eq!(
        html(render(
            &root.join("examples/semantic/login.html"),
            "text/html",
            "https://cem.dev/ns/data/html/1",
            &template
        )),
        expected.replace(
            "Use your email and password.",
            "Use your Amsterdam account."
        )
    );
    assert_eq!(fs::read_to_string(&template).unwrap(), edited);
    let broken = original.replacen("seq:where(", "seq:where((", 1);
    assert_ne!(broken, original);
    fs::write(&template, broken).unwrap();
    let output = render(
        &root.join("examples/semantic/login.html"),
        "text/html",
        "https://cem.dev/ns/data/html/1",
        &template,
    );
    assert!(!output.status.success());
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains("view.cemt"), "{diagnostic}");
    assert!(
        output.stdout.is_empty(),
        "failed template must not publish a partial view"
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn native_template_validation_admits_documented_whitespace_and_rejects_unknown_annotations() {
    let directory = scratch();
    let path = directory.join("layout.cemt");
    for (annotation, accepted, expected_code) in [
        ("@cem:whitespace=layout", true, ""),
        ("@cem:whitespace=preserve", true, ""),
        (
            "@cem:whitespace=unknown",
            false,
            "cem.schema.unknown_annotation_value",
        ),
        (
            "@cem:made-up=layout",
            false,
            "cem.schema.unknown_annotation",
        ),
    ] {
        fs::write(&path, format!("@doc cem-ml 1\n@ns cem = \"https://cem.dev/ns/core/1\"\n@ns template = \"https://cem.dev/ns/template/cem-native/1\"\n@default template\n{{template @name=main @visibility=public {annotation} | {{p | Example}}}}\n")).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .args([
                "validate",
                path.to_str().unwrap(),
                "--content-type",
                "application/vnd.cem.template+cem",
                "--schema",
                "https://cem.dev/ns/template/cem-native/1",
                "--format",
                "json",
            ])
            .output()
            .unwrap();
        let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(result.status.success(), accepted, "{annotation}: {report}");
        if !accepted {
            assert!(
                report["diagnostics"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|diagnostic| diagnostic["code"] == expected_code),
                "{annotation}: {report}"
            );
        }
    }
    fs::remove_dir_all(directory).unwrap();
}
