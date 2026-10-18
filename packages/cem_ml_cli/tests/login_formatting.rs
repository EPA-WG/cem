use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn login_format_configurations_preserve_semantics_and_saved_edits() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let definition: serde_json::Value = serde_json::from_str(include_str!(
        "../../../examples/projects/login/example.json"
    ))
    .unwrap();
    let original = fs::read_to_string(root.join("examples/cem-ml/login.cem")).unwrap();
    let directory = std::env::temp_dir().join(format!(
        "cem-login-format-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let source = directory.join("login.cem");
    let formatted = directory.join("formatted.cem");
    let mut layouts = Vec::new();
    for profile in ["pretty", "tabular"] {
        let id = format!("format-{profile}");
        let configuration = definition["configurations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == id)
            .expect("Login must expose both format configurations");
        assert_eq!(configuration["source"], "login-cem");
        assert_eq!(configuration["kind"], "conversion");
        let format = |path: &std::path::Path| {
            let args: Vec<String> = configuration["argv"]
                .as_array()
                .unwrap()
                .iter()
                .map(|arg| {
                    if arg == "$source" {
                        path.to_string_lossy().into_owned()
                    } else {
                        arg.as_str().unwrap().into()
                    }
                })
                .collect();
            let output = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
                .current_dir(&root)
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let text = String::from_utf8(output.stdout).unwrap();
            assert!(!text.contains('\u{1b}'));
            assert!(!text.contains("<span"));
            text
        };
        for heading in ["Sign in", "Bienvenue 🍒"] {
            let edited = original.replacen("Sign in", heading, 1);
            fs::write(&source, &edited).unwrap();
            let output = format(&source);
            assert_eq!(fs::read_to_string(&source).unwrap(), edited);
            assert!(output.contains(heading));
            fs::write(&formatted, &output).unwrap();
            assert_eq!(format(&formatted), output, "{profile} must be idempotent");
            let query = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
                .current_dir(&root)
                .args([
                    "query",
                    formatted.to_str().unwrap(),
                    "--content-type",
                    "application/cem",
                    "--schema",
                    "https://cem.dev/ns/cem-ml/1",
                    "--query-file",
                    "examples/projects/login/queries/semantics.xpath",
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
                query.status.success(),
                "{}",
                String::from_utf8_lossy(&query.stderr)
            );
            let report: serde_json::Value = serde_json::from_slice(&query.stdout).unwrap();
            let values: Vec<_> = report["result"]["sequence"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| item["value"]["lexicalValue"].as_str().unwrap().to_owned())
                .collect();
            let mut expected: Vec<String> = serde_json::from_str(include_str!(
                "../../../examples/projects/login/expected/semantics.json"
            ))
            .unwrap();
            expected[2] = format!("title={heading}");
            assert_eq!(values, expected, "{profile}");
            if heading == "Sign in" {
                layouts.push(output);
            }
        }
    }
    assert_ne!(
        layouts[0], layouts[1],
        "the two configurations must demonstrate different layouts"
    );
    fs::remove_dir_all(directory).unwrap();
}
