use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        Self(std::env::temp_dir().join(format!("cem-page-binding-{}-{id}", std::process::id())))
    }
    fn write(&self, name: &str, content: &str) {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    fn run(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .args(["transform", "--config"])
            .arg(self.0.join("graph.cem"))
            .arg("--report-json")
            .arg(self.0.join("report.json"))
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const TEMPLATE: &str = r#"{@doc cem-ml 1}
{module | {template @name=page @visibility=public |
  {param @name=pageName @required=true}{param @name=legacyStem @required=true}{param @name=pageUrl @required=true}
  {body | {p | {$pageName}}{p | {$legacyStem}}{a @href="{$pageUrl}" | Route}}
}}"#;

#[test]
fn page_binding_routes_compound_names_through_the_actual_cli_without_changing_stem() {
    let fixture = Fixture::new();
    for (file, page, _) in [
        ("guide.cem.md", "guide", "guide.cem"),
        ("news.html", "news", "news"),
        ("plain.md", "plain.md", "plain"),
    ] {
        fixture.write(&format!("inputs/{file}"), "{article | Input}");
        fixture.write(&format!("templates/{page}.cem"), TEMPLATE);
    }
    for directory in ["out", "another-output-root"] {
        fixture.write("graph.cem", &r#"{run | {import @id=pages @src="inputs/*" @content-type="text/cem-ml" |
          {transform @id=route @src="templates/{page}.cem" @template-content-type="text/cem-ml" @template-schema="https://cem.dev/ns/template/cem-native/1" @entrypoint=page |
            {param @name=pageName @value="{page}"}{param @name=legacyStem @value="{stem}"}
            {param @name=pageUrl @value="https://docs.example.test/{page}/"}
            {export @id=html @out="OUTPUT/{page}/index.html" @content-type="text/html"}
          }
        }}"#.replace("OUTPUT", directory));
        let output = fixture.run();
        assert_eq!(
            output.status.code(),
            Some(0),
            "stderr={} report={}",
            String::from_utf8_lossy(&output.stderr),
            fs::read_to_string(fixture.0.join("report.json")).unwrap_or_default()
        );
        assert!(output.stdout.is_empty());
        for (page, stem) in [
            ("guide", "guide.cem"),
            ("news", "news"),
            ("plain.md", "plain"),
        ] {
            assert_eq!(fs::read_to_string(fixture.0.join(format!("{directory}/{page}/index.html"))).unwrap(),
                format!("<p>{page}</p><p>{stem}</p><a href=\"https://docs.example.test/{page}/\">Route</a>"));
        }
    }
}

#[test]
fn page_binding_collisions_and_unknown_names_preserve_existing_files() {
    let fixture = Fixture::new();
    fixture.write("inputs/guide.html", "{p | First}");
    fixture.write("inputs/guide.cem.md", "{p | Second}");
    fixture.write("out/guide/index.cem", "prior publication");
    fixture.write(
        "graph.cem",
        r#"{run | {import @id=pages @src="inputs/*" @content-type="text/cem-ml" |
        {export @id=page @out="out/{page}/index.cem" @content-type="text/cem-ml"}
    }}"#,
    );
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.0.join("report.json")).unwrap()).unwrap();
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "cem.transform_runtime.duplicate_destination"),
        "{report}"
    );
    assert_eq!(
        fs::read_to_string(fixture.0.join("out/guide/index.cem")).unwrap(),
        "prior publication"
    );
    let prior_report = fs::read(fixture.0.join("report.json")).unwrap();
    fixture.write(
        "graph.cem",
        r#"{run | {import @id=pages @src="inputs/*" @content-type="text/cem-ml" |
        {export @id=page @out="out/{pages}/index.cem" @content-type="text/cem-ml"}
    }}"#,
    );
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("cem.transform_config.output_binding_unknown"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(fixture.0.join("report.json")).unwrap(),
        prior_report
    );
    assert_eq!(
        fs::read_to_string(fixture.0.join("out/guide/index.cem")).unwrap(),
        "prior publication"
    );
}
