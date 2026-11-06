use super::*;
use crate::{
    parser::tree::CemDocumentMetadata,
    validation::html::{html_document_ast_from_source_bytes, HtmlSourceValidationRequest},
};
fn document(source: &str) -> Arc<html_parser::HtmlDocumentAst> {
    Arc::new(
        html_document_ast_from_source_bytes(HtmlSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "file:///source/page.html",
            content_type: Some("text/html"),
        })
        .0
        .unwrap(),
    )
}
fn imported(owner: Arc<html_parser::HtmlDocumentAst>) -> RetainedElementImport {
    retain_html_elements(
        owner,
        CemDocumentMetadata {
            base_uri: Some("https://ssr.example.test/guide/".into()),
            document_uri: Some("https://ssr.example.test/guide/".into()),
        },
    )
    .unwrap()
}
fn template(import: &RetainedElementImport) -> AstNodeId {
    import
        .elements
        .iter()
        .map(|entry| entry.0)
        .find(|id| {
            import
                .tree
                .node(*id)
                .unwrap()
                .name
                .as_ref()
                .unwrap()
                .local_name
                == "template"
        })
        .unwrap()
}

#[test]
fn ssr_element_import_retains_original_arc_and_browser_template_text_vectors() {
    // Explicit JSON fixture-control boundary, never an AST/data handoff.
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/inline-cem-source.json")).unwrap();
    for vector in vectors.as_array().unwrap() {
        let owner = document(vector["html"].as_str().unwrap());
        let import = imported(Arc::clone(&owner));
        let retained = import
            .tree
            .native_owner()
            .unwrap()
            .clone()
            .downcast::<html_parser::HtmlDocumentAst>()
            .unwrap();
        assert!(Arc::ptr_eq(&retained, &owner));
        assert_eq!(import.tree.source_uri(), owner.source.uri);
        assert_eq!(
            import.tree.base_uri(),
            Some("https://ssr.example.test/guide/")
        );
        let source = import
            .inline_cem_template_source(template(&import))
            .unwrap();
        assert_eq!(
            source.text(),
            vector["text"].as_str().unwrap(),
            "{}",
            vector["name"]
        );
        let map = source
            .source_map_for_range(ByteRange::new(0, source.text().len() as u32))
            .unwrap();
        assert!(map.frames.iter().any(|frame| matches!(&frame.transform, TransformKind::SourceBoundary {uri} if uri=="file:///source/page.html")));
    }
}

#[test]
fn ssr_element_import_preserves_transformed_and_absent_origins() {
    let native = document(
        "<html><body><template type='text/cem-ml'>{p | Original}</template></body></html>",
    );
    for mapped in [true, false] {
        let mut owner = (*native).clone();
        let origin = SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(7),
                span: FrameSpan::Single(ByteRange::new(77, 11)),
                transform: TransformKind::CemAstBuilder,
            }],
        }
        .with_source_uri("memory:authored.cem");
        owner.origin_spans = Some(crate::source_map::SourceMapIndex::new(if mapped {
            vec![crate::interpreter::OutputSpan {
                output_range: ByteRange::new(0, owner.source_text.len() as u32),
                origin: origin.clone(),
            }]
        } else {
            vec![]
        }));
        let import = imported(Arc::new(owner));
        let id = template(&import);
        let source = import.inline_cem_template_source(id).unwrap();
        let map = source.source_map_for_range(ByteRange::new(0, source.text().len() as u32));
        if mapped {
            assert_eq!(map.as_ref(), Some(&origin));
        } else {
            assert_eq!(map, None);
            assert!(import.tree.node_source_map(id).unwrap().frames.is_empty());
        }
    }
}

#[test]
fn ssr_element_import_does_not_grant_script_resource_or_markup_handoffs() {
    for markup in [
        "<html><script>alert(1)</script></html>",
        "<html><img src='https://remote.test/picture'></html>",
    ] {
        assert!(retain_html_elements(document(markup), CemDocumentMetadata::default()).is_err());
    }
    let import = imported(document(
        "<html><body><template type='text/cem-ml'><b>HTML markup</b></template></body></html>",
    ));
    assert!(import
        .inline_cem_template_source(template(&import))
        .is_err());
}

#[test]
fn ssr_element_source_refuses_character_references_outside_native_decode_profile() {
    for text in ["{p | A &nbsp; B}", "{p | A & B}"] {
        let import = imported(document(&format!(
            "<html><body><template type='text/cem-ml'>{text}</template></body></html>"
        )));
        assert!(import
            .inline_cem_template_source(template(&import))
            .is_err());
    }
}
