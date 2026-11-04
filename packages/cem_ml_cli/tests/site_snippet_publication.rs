use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn site_snippets_preserve_inner_indentation_without_template_padding_or_live_execution() {
    let root = Scratch(
        std::env::temp_dir().join(format!(
            "cem-site-snippets-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )),
    );
    fs::create_dir_all(&root.0).unwrap();
    fs::write(
        root.0.join("page.cemt"),
        include_str!("../../../apps/cem-site/layouts/page.cemt"),
    )
    .unwrap();
    fs::write(root.0.join("source.md"), "# Snippets\n\n```js\nconst label = \"keep  two spaces\";\nif (label) {\n    console.log(label);\n}\n```\n\n```html\n<p data-note=\"two spaces\">Hello & goodbye</p>\n```\n").unwrap();
    fs::write(root.0.join("site.cem"), r#"{run |
      {import @id=source @src="source.md" @content-type="text/markdown; charset=utf-8; variant=CommonMark" @schema="https://cem.dev/ns/data/markdown/1" |
        {convert @id=html @content-type="text/html" @schema="https://cem.dev/ns/data/html/1" @converter="markdown-to-html-rust" |
          {convert @id=dom @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1" @converter="html-to-cem-dom-projection-rust" |
            {transform @id=page @src="page.cemt" @template-content-type="application/vnd.cem.transform+cem" @template-schema="https://cem.dev/ns/transform/cem/1" @entrypoint="page" |
              {param @name=title @value="Snippet fixture"}
              {export @id=output @out="page.html" @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }}}}}"#).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .current_dir(&root.0)
        .args(["transform", "--config", "site.cem"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let html = fs::read_to_string(root.0.join("page.html")).unwrap();
    assert_eq!(
        html.matches("<cem-demo-element").count(),
        2,
        "code blocks must use the shared viewer"
    );
    assert_eq!(
        html.matches("demo=\"false\"").count(),
        2,
        "documentation examples must remain source-only"
    );
    assert!(html.contains("source=\"const label = &quot;keep  two spaces&quot;;\nif (label) {\n    console.log(label);\n}\""), "source whitespace drifted: {html}");
    assert!(
        html.contains(
            "source=\"&lt;p data-note=&quot;two spaces&quot;&gt;Hello &amp; goodbye&lt;/p&gt;\""
        ),
        "markup must remain escaped source: {html}"
    );
    assert!(
        !html.contains("<pre>"),
        "template padding must not survive inside preformatted output"
    );
}
