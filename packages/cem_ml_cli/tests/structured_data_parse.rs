//! Source Parse uses retained native data, including strict syntax refusals.
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
#[test]
fn structured_parse_and_inspect_preserve_identity_and_strict_diagnostics() {
    let directory = std::env::temp_dir().join(format!(
        "cem-data-source-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    for (name, mime, schema, valid, invalid, code) in [
        (
            "source.json",
            "application/json",
            "https://cem.dev/ns/data/json/1",
            r#"{"literal":"{#never} {$never()} Café 🧬"}"#,
            "{\r\n\"café\": [1, }",
            "cem.json.parse_error",
        ),
        (
            "source.yaml",
            "text/x-yaml; charset=utf-8",
            "https://cem.dev/ns/data/yaml/1",
            "literal: '{#never} {$never()} Café 🧬'\r\n",
            "café: [1, 2\r\n",
            "cem.yaml.parse_error",
        ),
        (
            "schema.json",
            "application/json",
            "https://cem.dev/ns/data/json-schema/1",
            r##"{"$schema":"https://json-schema.org/draft/2020-12/schema","$defs":{"note":{"type":"string"}},"$ref":"#/$defs/note"}"##,
            "{\r\n\"$defs\": [}",
            "cem.json_schema.parse_error",
        ),
    ] {
        let source = directory.join(name);
        fs::write(&source, valid).unwrap();
        for operation in ["parse", "inspect"] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_cem-ml"));
            command.args([
                operation,
                source.to_str().unwrap(),
                "--content-type",
                mime,
                "--schema",
                schema,
                "--no-color",
            ]);
            command.args(if operation == "parse" {
                ["--format", "ast"]
            } else {
                ["--show", "ast"]
            });
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let text = String::from_utf8(output.stdout).unwrap();
            assert!(text.contains(schema) && text.contains(mime), "{text}");
            assert!(
                text.contains("@namespace=cem:generic-data") && text.contains("@byte-offset="),
                "{text}"
            );
        }
        fs::write(&source, invalid).unwrap();
        let report = directory.join("diagnostics.json");
        let output = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .args([
                "parse",
                source.to_str().unwrap(),
                "--content-type",
                mime,
                "--schema",
                schema,
                "--format",
                "ast",
                "--no-color",
                "--fail-level",
                "validate",
                "--report-json",
                report.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.stdout.is_empty(),
            "invalid input must not publish a partial tree"
        );
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(&report).unwrap()).unwrap();
        let diagnostics = report["diagnostics"].as_array().unwrap();
        assert!(diagnostics.iter().any(|d| d["code"] == code), "{report}");
        assert!(diagnostics
            .iter()
            .all(|d| !d["code"].as_str().unwrap().starts_with("cem.tokenizer.")));
        assert!(
            diagnostics.iter().any(
                |d| d["line"] == 2 || d["byteOffset"].as_u64().is_some_and(|offset| offset > 0)
            ),
            "{report}"
        );
    }
    fs::remove_dir_all(directory).unwrap();
}
