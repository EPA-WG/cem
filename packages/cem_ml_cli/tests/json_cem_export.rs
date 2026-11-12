use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture {
    root: PathBuf,
    mime: &'static str,
    schema: &'static str,
}
impl Fixture {
    fn new(mime: &'static str, schema: &'static str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "cem-json-export-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        fs::write(
            root.join("source.json"),
            r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","literal":"Café {$never()} {#never}","active":true,"number":1.2300e+04}"#,
        )
        .unwrap();
        Self { root, mime, schema }
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .current_dir(&self.root)
            .env_remove("NO_COLOR")
            .env_remove("FORCE_COLOR")
            .args([
                "convert",
                "source.json",
                "--content-type",
                self.mime,
                "--schema",
                self.schema,
            ])
            .args(args)
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn plain(bytes: &[u8]) -> String {
    let mut result = String::new();
    let mut chars = std::str::from_utf8(bytes).unwrap().chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}
fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
const IDENTITIES: [(&str, &str); 3] = [
    ("application/json", "https://cem.dev/ns/data/json/1"),
    (
        "application/schema+json",
        "https://cem.dev/ns/data/json-schema/1",
    ),
    ("application/json", "https://cem.dev/ns/data/json-schema/1"),
];

#[test]
fn json_cem_export_colored_stdout_and_explicit_targets_preserve_typed_literals() {
    for (mime, schema) in IDENTITIES {
        let fixture = Fixture::new(mime, schema);
        let output = fixture.run(&["--to-format", "cem", "--tabular"]);
        success(&output);
        assert!(output.stdout.windows(2).any(|bytes| bytes == b"\x1b["));
        let text = plain(&output.stdout);
        assert!(text.starts_with("@doc cem-ml 1\n"), "{text}");
        for expected in [
            "cem:generic-data",
            "data:object",
            "data:string",
            "data:boolean",
            "data:number",
            "{$never()}",
            "{#never}",
            "1.2300e+04",
        ] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
        assert!(!text.contains("cemast:") && !text.contains("{ast"));
        assert_eq!(fs::read_dir(&fixture.root).unwrap().count(), 1);
        let explicit = fixture.run(&[
            "--to-content-type",
            "application/cem",
            "--to-schema",
            "https://cem.dev/ns/cem-ml/1",
            "--cemt-formatter-profile",
            "tabular",
            "--cemt-color-profile",
            "terminal",
            "--output-color-type",
            "terminal",
        ]);
        success(&explicit);
        assert_eq!(explicit.stdout, output.stdout);
        let uncolored = fixture.run(&["--to-format", "cem", "--tabular", "--no-color"]);
        success(&uncolored);
        assert_eq!(uncolored.stdout, text.as_bytes());
    }
}

#[test]
fn json_cem_export_plain_file_and_invalid_input_publication_are_atomic() {
    for (mime, schema) in IDENTITIES {
        let fixture = Fixture::new(mime, schema);
        let output = fixture.run(&["--to-format", "cem", "--tabular", "--out", "result.cem"]);
        success(&output);
        assert!(output.stdout.is_empty());
        let original = fs::read(fixture.root.join("result.cem")).unwrap();
        assert!(!original.contains(&0x1b));
        for source in [
            b"{\"x\":[1,}".as_slice(),
            b"{} {}",
            b"{\"x\":NaN}",
            b"{\"x\":\"\xff\"}",
        ] {
            fs::write(fixture.root.join("source.json"), source).unwrap();
            for out in [false, true] {
                let mut args = vec!["--to-format", "cem", "--tabular"];
                if out {
                    args.extend(["--out", "result.cem"]);
                }
                let output = fixture.run(&args);
                assert!(!output.status.success());
                assert!(output.stdout.is_empty());
                assert!(!String::from_utf8_lossy(&output.stderr)
                    .contains("internal_ast_target_unsupported"));
                assert_eq!(fs::read(fixture.root.join("result.cem")).unwrap(), original);
            }
        }
    }
}
