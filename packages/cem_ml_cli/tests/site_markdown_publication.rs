use cem_ml::{
    engine::FormatIdentity,
    transform_config::{
        parse_transform_graph_config, TransformGraphNode, TransformGraphNodeKind,
        TransformGraphParseRequest,
    },
    validation::markdown::{
        markdown_document_ast_from_source_bytes, MarkdownDocumentAst,
        MarkdownSourceValidationRequest,
    },
};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const FIXTURE: &str = "# Table fixture\n\n| Column | Value |\n| --- | --- |\n| `a\\|b` | **ready** and [docs](https://example.test/guide) |\n\n| Label |\n| --- |\n| café & tea |\n\n```text\n| Keep | Literal |\n| --- | --- |\n| a | b |\n```\n";

fn imports() -> Vec<TransformGraphNode> {
    let response = parse_transform_graph_config(TransformGraphParseRequest {
        bytes: include_bytes!("../../../apps/cem-site/site.cem").to_vec(),
        identity: FormatIdentity {
            content_type: Some("application/cem".into()),
            ..Default::default()
        },
        base_uri: None,
    })
    .unwrap();
    assert!(
        response.diagnostics.is_empty(),
        "{:?}",
        response.diagnostics
    );
    response
        .graph
        .nodes
        .into_iter()
        .filter(|node| {
            node.kind == TransformGraphNodeKind::Import
                && node
                    .content_type
                    .as_deref()
                    .is_some_and(|value| value.starts_with("text/markdown;"))
        })
        .collect()
}

fn markdown(source: &str, content_type: &str) -> MarkdownDocumentAst {
    let (document, diagnostics) =
        markdown_document_ast_from_source_bytes(MarkdownSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "site://table-fixture.md",
            content_type: Some(content_type),
        });
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    document.unwrap()
}

fn table_count(document: &MarkdownDocumentAst) -> usize {
    document
        .events
        .iter()
        .filter(|event| event.kind == "start" && event.tag.as_deref() == Some("table"))
        .count()
}

#[test]
fn authored_table_code_and_links_are_not_lost_to_unescaped_pipe_separators() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();
    for source in imports() {
        let text = fs::read_to_string(
            workspace
                .join("apps/cem-site")
                .join(source.src.as_deref().unwrap()),
        )
        .unwrap();
        let document = markdown(&text, "text/markdown; charset=utf-8; variant=GFM");
        for table in document
            .events
            .iter()
            .filter_map(|event| event.lexical_source.as_deref())
        {
            let authored = markdown(table, "text/markdown; charset=utf-8; variant=CommonMark");
            let parsed = markdown(table, "text/markdown; charset=utf-8; variant=GFM");
            for kind in ["code", "link"] {
                let values = |document: &MarkdownDocumentAst| {
                    document
                        .events
                        .iter()
                        .filter_map(|event| {
                            if kind == "code" && event.kind == "code" {
                                event.text.clone()
                            } else if kind == "link"
                                && event.kind == "start"
                                && event.tag.as_deref() == Some("link")
                            {
                                event.destination.clone()
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                };
                let mut retained = values(&parsed);
                for value in values(&authored) {
                    let expected = if kind == "code" {
                        value.replace("\\|", "|")
                    } else {
                        value
                    };
                    let position = retained.iter().position(|actual| actual == &expected);
                    assert!(position.is_some(), "{} loses table {kind} {expected:?}; escape literal pipes in table cells as \\|", source.src.as_deref().unwrap());
                    retained.remove(position.unwrap());
                }
            }
        }
    }
}

#[test]
fn every_site_markdown_import_preserves_tables_while_explicit_commonmark_stays_strict() {
    let sources = imports();
    assert!(!sources.is_empty());
    for source in sources {
        assert_eq!(table_count(&markdown(FIXTURE, source.content_type.as_deref().unwrap())), 2,
            "Site import {} must support tables without parsing the fenced example as a third table", source.id);
    }
    assert_eq!(
        table_count(&markdown(
            FIXTURE,
            "text/markdown; charset=utf-8; variant=CommonMark"
        )),
        0
    );
}

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn publish(source: &str, content_type: &str) -> String {
    let root = Scratch(std::env::temp_dir().join(format!(
            "cem-site-tables-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    fs::create_dir_all(&root.0).unwrap();
    fs::write(
        root.0.join("page.cemt"),
        include_str!("../../../apps/cem-site/layouts/page.cemt"),
    )
    .unwrap();
    fs::write(root.0.join("source.md"), source).unwrap();
    let graph = r#"{run |
      {import @id=source @src="source.md" @content-type="CONTENT_TYPE" @schema="https://cem.dev/ns/data/markdown/1" |
        {convert @id=html @content-type="text/html" @schema="https://cem.dev/ns/data/html/1" @converter="markdown-to-html-rust" |
          {convert @id=dom @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1" @converter="html-to-cem-dom-projection-rust" |
            {transform @id=page @src="page.cemt" @template-content-type="application/vnd.cem.transform+cem" @template-schema="https://cem.dev/ns/transform/cem/1" @entrypoint="page" |
              {param @name=title @value="Table fixture"}
              {export @id=output @out="page.html" @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }}}}}"#.replace("CONTENT_TYPE", content_type);
    fs::write(root.0.join("site.cem"), graph).unwrap();
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
    fs::read_to_string(root.0.join("page.html")).unwrap()
}

#[test]
fn native_site_page_preserves_table_cells_inline_markup_and_literal_fenced_source() {
    let content_type = imports()
        .into_iter()
        .find(|node| node.id == "transform-reference-source")
        .unwrap()
        .content_type
        .unwrap();
    let html = publish(FIXTURE, &content_type);
    assert_eq!(
        html.matches("<table>").count(),
        2,
        "Tables must survive the typed HTML/DOM/page pipeline: {html}"
    );
    assert_eq!(html.matches("<thead>").count(), 2);
    assert_eq!(html.matches("<tbody>").count(), 2);
    for expected in [
        "<th>Column</th>",
        "<td><code>a|b</code></td>",
        "<strong>ready</strong>",
        "href=\"https://example.test/guide\"",
        "café &amp; tea",
        "<cem-demo-element",
        "demo=\"false\"",
        "source=\"| Keep | Literal |\n| --- | --- |\n| a | b |\"",
    ] {
        assert!(html.contains(expected), "Missing {expected}: {html}");
    }
    assert!(!html.contains("&lt;table"));
}

#[test]
fn every_authored_site_source_keeps_its_native_tables_and_cells() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();
    let mut audited = Vec::new();
    for node in imports() {
        let source = node.src.as_deref().unwrap();
        let text = fs::read_to_string(workspace.join("apps/cem-site").join(source)).unwrap();
        let document = markdown(&text, "text/markdown; charset=utf-8; variant=GFM");
        let tables = table_count(&document);
        let cells = document
            .events
            .iter()
            .filter(|event| event.kind == "start" && event.tag.as_deref() == Some("table-cell"))
            .count();
        let html = publish(&text, node.content_type.as_deref().unwrap());
        assert_eq!(
            html.matches("<table>").count(),
            tables,
            "{} lost tables",
            node.id
        );
        let actual_cells = ["<th>", "<th ", "<td>", "<td "]
            .into_iter()
            .map(|tag| html.matches(tag).count())
            .sum::<usize>();
        assert_eq!(actual_cells, cells, "{} lost table cells", node.id);
        audited.push(
            serde_json::json!({"importId":node.id,"source":source,"tables":tables,"cells":cells,
                "sourceSha256":cem_ml::command_service::sha256_hex(text.as_bytes()),"contentType":node.content_type}),
        );
    }
    let report = serde_json::json!({"schemaVersion":1,"status":"passed","sourceCount":audited.len(),"sources":audited,
            "siteGraphSha256":cem_ml::command_service::sha256_hex(include_bytes!("../../../apps/cem-site/site.cem")),
            "pageLayoutSha256":cem_ml::command_service::sha256_hex(include_bytes!("../../../apps/cem-site/layouts/page.cemt"))});
    if let Some(path) = std::env::var_os("CEM_SITE_TABLE_REPORT") {
        // Cargo runs integration tests from the crate directory, while Nx
        // report paths are workspace-relative.
        let path = workspace.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            path,
            format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
        )
        .unwrap();
    }
    println!("{report}");
}
