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
fn project() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "cem-login-graph-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    for directory in ["data", "templates", "graphs"] {
        fs::create_dir_all(root.join(directory)).unwrap();
    }
    fs::copy(
        workspace().join("examples/cem-ml/login.cem"),
        root.join("data/login.cem"),
    )
    .unwrap();
    fs::copy(
        workspace().join("examples/semantic/login.html"),
        root.join("data/login.html"),
    )
    .unwrap();
    let template =
        fs::read_to_string(workspace().join("examples/projects/login/templates/view.cemt"))
            .unwrap();
    fs::write(
        root.join("templates/view.cemt"),
        template.replacen("@doc cem-ml 1", "@doc cem-ml 1\n// café Amsterdam", 1),
    )
    .unwrap();
    root
}
fn graph(media: &str, schema: &str, source: &str) -> String {
    format!(
        r#"{{run |
  {{import @id=source @src="../data/{source}" @content-type="{media}" @schema="{schema}" |
    {{transform @id=view @src="../templates/view.cemt" @template-content-type="application/vnd.cem.template+cem" @template-schema="https://cem.dev/ns/template/cem-native/1" @entrypoint=main |
      {{export @id=page @out="../generated/login.html" @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}}
    }}
  }}
}}
"#
    )
}
fn run(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .args([
            "transform",
            "--config",
            root.join("graphs/view.cem").to_str().unwrap(),
            "--scope-budget",
            "queueSize=1024",
        ])
        .output()
        .unwrap()
}
fn success(output: Output) {
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
fn assert_provenance(root: &Path, template_path: &str, instruction: &str) {
    let html = fs::read_to_string(root.join("generated/login.html")).unwrap();
    let map: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("generated/login.html.map")).unwrap()).unwrap();
    let template = fs::read_to_string(root.join(template_path)).unwrap();
    let spans = map["outputSpans"].as_array().unwrap();
    assert!(!spans.is_empty());
    let offset = html.find(instruction).unwrap() as u64;
    let mut instruction_mapped = false;
    for span in spans {
        let start = span["outputRange"]["start"].as_u64().unwrap();
        let len = span["outputRange"]["len"].as_u64().unwrap();
        assert!(start + len <= html.len() as u64);
        for frame in span["origin"]["frames"].as_array().unwrap() {
            if frame["transform"]["kind"] != "SourceBoundary" {
                continue;
            }
            assert!(
                frame["transform"]["uri"]
                    .as_str()
                    .unwrap()
                    .ends_with(template_path),
                "{frame}"
            );
            let range = &frame["span"]["ranges"];
            let from = range["start"].as_u64().unwrap() as usize;
            let to = from + range["len"].as_u64().unwrap() as usize;
            let authored = template
                .get(from..to)
                .expect("authored range must respect original UTF-8 bytes");
            if start <= offset && offset < start + len {
                let expected = format!("```{instruction}```");
                assert_eq!(
                    authored, expected,
                    "select the authored token, not a document-wide frame"
                );
                assert_eq!(from, template.find(&expected).unwrap());
                instruction_mapped = true;
            }
        }
    }
    assert!(
        instruction_mapped,
        "generated instruction needs an explicit native template URI/range"
    );
}

#[test]
fn login_graph_preserves_html_and_template_source_ranges_across_edits() {
    let expected =
        fs::read_to_string(workspace().join("examples/projects/login/expected/view.html")).unwrap();
    for (media, schema, source) in [
        (
            "application/cem",
            "https://cem.dev/ns/cem-ml/1",
            "login.cem",
        ),
        ("text/html", "https://cem.dev/ns/data/html/1", "login.html"),
    ] {
        let root = project();
        let configuration = graph(media, schema, source);
        fs::write(root.join("graphs/view.cem"), &configuration).unwrap();
        success(run(&root));
        assert_eq!(
            fs::read_to_string(root.join("generated/login.html"))
                .unwrap()
                .trim(),
            expected.trim()
        );
        assert_provenance(&root, "templates/view.cemt", "Use your email and password.");
        let source_path = root.join("data").join(source);
        let original = fs::read_to_string(&source_path).unwrap();
        fs::write(
            &source_path,
            original.replacen("Sign in", "Amsterdam sign in", 1),
        )
        .unwrap();
        let template = fs::read_to_string(root.join("templates/view.cemt")).unwrap();
        let edited = template.replace(
            "Use your email and password.",
            "Use your Amsterdam account.",
        );
        fs::write(root.join("templates/edited.cemt"), &edited).unwrap();
        fs::write(
            root.join("graphs/view.cem"),
            configuration.replace("templates/view.cemt", "templates/edited.cemt"),
        )
        .unwrap();
        success(run(&root));
        assert_eq!(
            fs::read_to_string(root.join("generated/login.html"))
                .unwrap()
                .trim(),
            expected
                .trim()
                .replacen("Sign in", "Amsterdam sign in", 1)
                .replace(
                    "Use your email and password.",
                    "Use your Amsterdam account."
                )
        );
        assert_provenance(
            &root,
            "templates/edited.cemt",
            "Use your Amsterdam account.",
        );
        assert_eq!(
            fs::read_to_string(root.join("templates/view.cemt")).unwrap(),
            template
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn invalid_login_graphs_and_missing_templates_publish_no_html() {
    for broken in [false, true] {
        let root = project();
        let configuration = graph(
            "application/cem",
            "https://cem.dev/ns/cem-ml/1",
            "login.cem",
        );
        if broken {
            fs::write(
                root.join("graphs/view.cem"),
                configuration.replace("@id=view", "@id=source"),
            )
            .unwrap();
        } else {
            fs::write(root.join("graphs/view.cem"), configuration).unwrap();
            fs::remove_file(root.join("templates/view.cemt")).unwrap();
        }
        let output = run(&root);
        assert!(!output.status.success());
        assert!(!root.join("generated/login.html").exists());
        assert!(!root.join("generated/login.html.map").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
