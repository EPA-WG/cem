use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new(source: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "cem-yaml-export-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("source.yml"), source).unwrap();
        Self(root)
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .current_dir(&self.0)
            .env_remove("NO_COLOR")
            .env_remove("FORCE_COLOR")
            .args(["convert", "source.yml"])
            .args(args)
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn plain(bytes: &[u8]) -> String {
    let text = std::str::from_utf8(bytes).unwrap();
    let mut result = String::new();
    let mut chars = text.chars();
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
#[test]
fn yaml_cem_export_stdout_uses_tabular_terminal_and_saves_nothing() {
    let source = "service:\n  name: 'Café {$never()} {#never}'\n  enabled: true\n  retries: 3\n";
    let fixture = Fixture::new(source);
    let output = fixture.run(&[
        "--content-type",
        "application/yaml",
        "--to-format",
        "cem",
        "--tabular",
    ]);
    success(&output);
    assert!(output.stdout.windows(2).any(|bytes| bytes == b"\x1b["));
    let text = plain(&output.stdout);
    assert!(text.starts_with("@doc cem-ml 1\n"), "{text}");
    assert!(
        text.contains("cem:generic-data")
            && text.contains("data:object")
            && text.contains("data:boolean"),
        "{text}"
    );
    assert!(
        !text.contains("cemast:") && !text.contains("{ast"),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|line| line.starts_with("\t") || line.starts_with("    ")),
        "{text}"
    );
    assert_eq!(
        fs::read_to_string(fixture.0.join("source.yml")).unwrap(),
        source
    );
    let files: Vec<_> = fs::read_dir(&fixture.0)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(files, ["source.yml"]);
}
#[test]
fn yaml_cem_export_target_identity_and_explicit_profiles_match_shortcut() {
    let fixture = Fixture::new("name: Ada\nactive: true\n");
    let shortcut = fixture.run(&["--to-format", "cem", "--tabular"]);
    success(&shortcut);
    let explicit = fixture.run(&[
        "--to-content-type",
        cem_ml::schema::registry::CEM_ML_CONTENT_TYPE,
        "--to-schema",
        cem_ml::schema::registry::CEM_ML_SCHEMA_URI,
        "--cemt-formatter-profile",
        "tabular",
        "--cemt-color-profile",
        "terminal",
        "--output-color-type",
        "terminal",
    ]);
    success(&explicit);
    assert_eq!(explicit.stdout, shortcut.stdout);
    for flag in ["--no-color", "--output-color-type"] {
        let mut args = vec!["--to-format", "cem", "--tabular", flag];
        if flag == "--output-color-type" {
            args.push("none");
        }
        let output = fixture.run(&args);
        success(&output);
        assert!(!output.stdout.contains(&0x1b));
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            plain(&shortcut.stdout)
        );
    }
}
#[test]
fn yaml_cem_export_file_is_plain_and_errors_do_not_publish_partial_output() {
    let fixture = Fixture::new("name: Ada\nactive: true\n");
    let output = fixture.run(&["--to-format", "cem", "--tabular", "--out", "result.cem"]);
    success(&output);
    assert!(output.stdout.is_empty());
    let bytes = fs::read(fixture.0.join("result.cem")).unwrap();
    assert!(!bytes.contains(&0x1b));
    assert!(bytes.starts_with(b"@doc cem-ml 1\n"));
    for source in [
        "items: [\n",
        "value: !unsafe literal\n",
        "a: &copy value\nb: *copy\n",
    ] {
        fs::write(fixture.0.join("source.yml"), source).unwrap();
        for out in [false, true] {
            let mut args = vec!["--to-format", "cem", "--tabular"];
            if out {
                args.extend(["--out", "result.cem"]);
            }
            let output = fixture.run(&args);
            assert!(!output.status.success(), "{source}");
            assert!(output.stdout.is_empty(), "{source}");
            assert!(!String::from_utf8_lossy(&output.stderr)
                .contains("internal_ast_target_unsupported"));
            assert_eq!(fs::read(fixture.0.join("result.cem")).unwrap(), bytes);
        }
    }
}
