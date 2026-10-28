//! Default Parse may inspect partial sources, but a refused import cannot succeed.
use serde_json::Value;
use std::{fs, process::Command};

#[test]
fn default_parse_csv_refusal_preserves_diagnostics_and_existing_output() {
    let directory = std::env::temp_dir().join(format!("cem-parse-refusal-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let input = directory.join("original.csv");
    let destination = directory.join("kept.out");
    let report = directory.join("report.json");
    for (text, code, offset, column) in [
        ("id,name\n1,\"Ada\n", "cem.csv.unclosed_quote", 10, 3),
        ("id,name\n1,A \"B\n", "cem.csv.invalid_quote_escape", 12, 5),
    ] {
        fs::write(&input, text).unwrap();
        for format in [
            "ast",
            "events",
            "ast-json",
            "events-json",
            "dom-json",
            "json",
        ] {
            for level in [None, Some("parse"), Some("validate"), Some("strict")] {
                fs::write(&destination, "previous output").unwrap();
                let mut command = Command::new(env!("CARGO_BIN_EXE_cem-ml"));
                command.args([
                    "parse",
                    input.to_str().unwrap(),
                    "--format",
                    format,
                    "--content-type",
                    "text/csv; header=present",
                    "--schema",
                    "https://cem.dev/ns/data/csv/1",
                    "--no-color",
                    "--out",
                    destination.to_str().unwrap(),
                    "--report-json",
                    report.to_str().unwrap(),
                ]);
                if let Some(level) = level {
                    command.args(["--fail-level", level]);
                }
                let output = command.output().unwrap();
                assert_eq!(
                    output.status.code(),
                    Some(1),
                    "{format}/{level:?}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(output.stdout.is_empty());
                assert_eq!(fs::read_to_string(&destination).unwrap(), "previous output");
                let report: Value = serde_json::from_slice(&fs::read(&report).unwrap()).unwrap();
                let diagnostics = report["diagnostics"].as_array().unwrap();
                assert_eq!(diagnostics.len(), 1, "{report}");
                let diagnostic = &diagnostics[0];
                assert_eq!(diagnostic["code"], code);
                assert_eq!(diagnostic["severity"], "error");
                assert_eq!(diagnostic["uri"], input.to_str().unwrap());
                assert_eq!(diagnostic["byteOffset"], offset);
                assert_eq!(diagnostic["line"], 2);
                assert_eq!(diagnostic["column"], column);
                assert_eq!(diagnostic["details"]["behavior"], "csv-parse-report-fact");
                assert!(!String::from_utf8_lossy(&output.stderr).contains("write failure"));
            }
        }
    }
    fs::write(&input, "id,name\n1,\"Café 🍒\"\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .args([
            "parse",
            input.to_str().unwrap(),
            "--no-color",
            "--content-type",
            "text/csv; header=present",
            "--out",
            destination.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fs::read_to_string(&destination)
        .unwrap()
        .contains("Café 🍒"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn default_parse_keeps_recoverable_query_source_inspection() {
    let directory = std::env::temp_dir().join(format!("cem-parse-partial-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let input = directory.join("expression.cemql");
    fs::write(&input, "\"🍒\" +").unwrap();
    for (level, exit) in [("parse", 0), ("validate", 1)] {
        let output = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .args([
                "parse",
                input.to_str().unwrap(),
                "--no-color",
                "--content-type",
                "application/vnd.cem.query-expression+cem-ql",
                "--schema",
                "https://cem.dev/ns/query/cem-ql/1#expression",
                "--fail-level",
                level,
            ])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(exit),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.stdout.is_empty(),
            "native partial-source inspection remains available"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("cem.ql.parse_error"));
    }
    fs::remove_dir_all(directory).unwrap();
}
