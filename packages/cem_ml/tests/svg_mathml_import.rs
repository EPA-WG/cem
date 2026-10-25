//! Family policy and source ownership survive the shared retained-tree boundary.
use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    import::{import_data, import_data_bytes, retain_lifecycle, try_retain_lifecycle},
    lifecycle::LoadedInputAstStream,
    parser::{tree::RetainedCemTree, AstNodeId, CemAstNode},
    validation::{
        mathml::{
            mathml_document_ast_from_source_bytes, MathMlMediaProfile,
            MathMlSourceValidationRequest,
        },
        svg::{svg_document_ast_from_source_bytes, SvgSourceValidationRequest},
    },
};
use std::sync::Arc;

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const MATH_NS: &str = "http://www.w3.org/1998/Math/MathML";
const SVG_MIME: &str = "image/svg+xml;charset=utf-8";
const MATH_MIME: &str = "application/mathml+xml;charset=utf-8";
const PRESENTATION_MIME: &str = "application/mathml-presentation+xml;charset=utf-8";
const CONTENT_MIME: &str = "application/mathml-content+xml;charset=utf-8";
const URI: &str = "memory://documents/retained-source.xml";
const SVG: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<s:svg xmlns:s="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" role="img" viewBox="0 0 24 24">
  <s:title>Café &amp; diagrams</s:title>
  <s:defs><s:path id="stroke" d="M0 0L24 24"/></s:defs>
  <s:use xlink:href="#stroke"/>
  <s:text xml:space="preserve">A &amp; B<![CDATA[ < C]]></s:text>
</s:svg>
"##;
const MATH: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<m:math xmlns:m="http://www.w3.org/1998/Math/MathML" alttext="x squared">
  <m:msup><m:mi>x</m:mi><m:mn>2</m:mn></m:msup>
</m:math>
"#;
const CONTENT_MATH: &str = r#"<m:math xmlns:m="http://www.w3.org/1998/Math/MathML" alttext="x squared">
  <m:apply><m:power/><m:ci>x</m:ci><m:cn>2</m:cn></m:apply>
</m:math>
"#;

fn lifecycle(source: &str, mime: &str) -> (Arc<LoadedInputAstStream>, Vec<Diagnostic>) {
    if mime.starts_with("image/svg+xml") {
        let (document, diagnostics) =
            svg_document_ast_from_source_bytes(SvgSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: URI,
                content_type: Some(mime),
            });
        (
            Arc::new(LoadedInputAstStream::SvgDocument(document.unwrap())),
            diagnostics,
        )
    } else {
        let (document, diagnostics) =
            mathml_document_ast_from_source_bytes(MathMlSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: URI,
                content_type: Some(mime),
            });
        (
            Arc::new(LoadedInputAstStream::MathMlDocument(document.unwrap())),
            diagnostics,
        )
    }
}

fn node(tree: &RetainedCemTree, namespace: &str, name: &str, attribute: bool) -> AstNodeId {
    tree.ast()
        .iter()
        .find_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if !attribute
                && expanded_name.namespace_uri == namespace
                && expanded_name.local_name == name =>
            {
                Some(*node_id)
            }
            CemAstNode::Attribute {
                node_id,
                expanded_name,
                ..
            } if attribute
                && expanded_name.namespace_uri == namespace
                && expanded_name.local_name == name =>
            {
                Some(*node_id)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing {{{namespace}}}{name}"))
}

fn retained_owner(tree: &RetainedCemTree) -> Arc<LoadedInputAstStream> {
    tree.native_owner()
        .unwrap()
        .clone()
        .downcast::<LoadedInputAstStream>()
        .unwrap()
}

#[test]
fn svg_lifecycle_import_preserves_owner_names_values_case_and_source_coordinates() {
    let (owner, diagnostics) = lifecycle(SVG, SVG_MIME);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let weak = Arc::downgrade(&owner);
    let tree = try_retain_lifecycle(owner.clone())
        .unwrap()
        .expect("SVG has a retained import");
    let kept = retained_owner(&tree);
    assert!(Arc::ptr_eq(&owner, &kept));
    let LoadedInputAstStream::SvgDocument(document) = kept.as_ref() else {
        panic!("SVG owner was replaced")
    };
    assert_eq!(document.source.content_type, SVG_MIME);
    assert_eq!(document.xml_document.source.media_type, "image/svg+xml");
    assert_eq!(
        document
            .source
            .parameters
            .get("charset")
            .map(String::as_str),
        Some("utf-8")
    );
    assert_eq!(tree.source_uri(), URI);
    let title = node(&tree, SVG_NS, "title", false);
    let range = tree.source_node_range(title).unwrap();
    assert_eq!(range.offset, SVG.find("<s:title>").unwrap() as u64);
    assert_eq!((range.line, range.column), (3, 3));
    assert_eq!(tree.node_source_uri(title), Some(URI));
    assert_eq!(range.length, "<s:title>".len() as u64);
    let view_box = node(&tree, "", "viewBox", true);
    assert_eq!(tree.node(view_box).unwrap().value, "0 0 24 24");
    assert_eq!(
        tree.source_node_range(view_box).unwrap().offset,
        SVG.find("0 0 24 24").unwrap() as u64
    );
    let reference = node(&tree, "http://www.w3.org/1999/xlink", "href", true);
    assert_eq!(tree.node(reference).unwrap().value, "#stroke");
    assert_eq!(
        tree.source_node_range(reference).unwrap().offset,
        SVG.find("#stroke").unwrap() as u64
    );
    assert!(tree
        .ast()
        .iter()
        .any(|node| matches!(node, CemAstNode::Text { data, .. } if data == "Café & diagrams")));
    assert!(tree
        .ast()
        .iter()
        .any(|node| matches!(node, CemAstNode::Cdata { data, .. } if data == " < C")));
    assert!(!tree.node(title).unwrap().source.frames.is_empty());
    drop(owner);
    drop(kept);
    assert!(weak.upgrade().is_some());
    let last = tree.clone();
    drop(tree);
    assert!(
        weak.upgrade().is_some(),
        "the last tree keeps the original owner alive"
    );
    drop(last);
    assert!(
        weak.upgrade().is_none(),
        "last-tree disposal releases the original owner"
    );
}

#[test]
fn mathml_lifecycle_import_preserves_all_media_profiles_and_original_owners() {
    for (mime, source, profile, token) in [
        (MATH_MIME, MATH, MathMlMediaProfile::Generic, "mi"),
        (
            PRESENTATION_MIME,
            MATH,
            MathMlMediaProfile::Presentation,
            "mi",
        ),
        (
            CONTENT_MIME,
            CONTENT_MATH,
            MathMlMediaProfile::Content,
            "ci",
        ),
    ] {
        let (owner, diagnostics) = lifecycle(source, mime);
        assert!(diagnostics.is_empty(), "{mime}: {diagnostics:?}");
        let weak = Arc::downgrade(&owner);
        let tree = retain_lifecycle(owner.clone()).unwrap();
        let kept = retained_owner(&tree);
        assert!(Arc::ptr_eq(&owner, &kept));
        let LoadedInputAstStream::MathMlDocument(document) = kept.as_ref() else {
            panic!("MathML owner was replaced")
        };
        assert_eq!(document.media_profile, profile);
        assert_eq!(document.xml_document.source.content_type, mime);
        let token_id = node(&tree, MATH_NS, token, false);
        assert_eq!(
            tree.source_node_range(token_id).unwrap().offset,
            source.find(&format!("<m:{token}>")).unwrap() as u64
        );
        assert_eq!(
            tree.source_node_range(token_id).unwrap().column,
            if token == "mi" { 11 } else { 22 }
        );
        assert_eq!(tree.node_source_uri(token_id), Some(URI));
        drop(owner);
        drop(kept);
        let last = tree.clone();
        drop(tree);
        assert!(weak.upgrade().is_some());
        drop(last);
        assert!(weak.upgrade().is_none());
    }
}

#[test]
fn explicit_family_byte_and_string_imports_precede_generic_xml_suffix_selection() {
    for (source, mime, namespace, root) in [
        (SVG, SVG_MIME, SVG_NS, "svg"),
        (MATH, MATH_MIME, MATH_NS, "math"),
        (MATH, PRESENTATION_MIME, MATH_NS, "math"),
        (CONTENT_MATH, CONTENT_MIME, MATH_NS, "math"),
    ] {
        for projection in ["cem", "xpath"] {
            for tree in [
                import_data_bytes(source.as_bytes(), mime, projection, URI).unwrap(),
                import_data(source, mime, projection, URI).unwrap(),
            ] {
                node(&tree, namespace, root, false);
                assert_eq!(tree.source_uri(), URI);
                assert!(matches!(
                    (mime, retained_owner(&tree).as_ref()),
                    (SVG_MIME, LoadedInputAstStream::SvgDocument(_))
                        | (_, LoadedInputAstStream::MathMlDocument(_))
                ));
            }
        }
        assert!(
            import_data_bytes(b"<catalog/>", mime, "cem", URI).is_err(),
            "{mime} enforces its own root contract"
        );
        let generic =
            import_data_bytes(source.as_bytes(), "application/example+xml", "cem", URI).unwrap();
        assert!(matches!(
            retained_owner(&generic).as_ref(),
            LoadedInputAstStream::XmlDocument(_)
        ));
        let family = import_data_bytes(source.as_bytes(), mime, "cem", URI).unwrap();
        assert!(family.source_key(0).is_some());
        assert_ne!(
            family.source_key(0),
            generic.source_key(0),
            "family admission participates in source identity"
        );
    }
}

#[test]
fn svg_hard_family_facts_cannot_be_bypassed_at_any_import_boundary() {
    for (source, code) in [
        (
            SVG.replace(SVG_NS, "urn:wrong"),
            "cem.svg.namespace_missing",
        ),
        (
            SVG.replace("<s:defs>", "<s:script>run()</s:script><s:defs>"),
            "cem.svg.script_rejected",
        ),
        (
            SVG.replace("role=\"img\"", "role=\"img\" onload=\"run()\""),
            "cem.svg.script_rejected",
        ),
        (
            SVG.replace("#stroke", "https://example.test/icon.svg#stroke"),
            "cem.svg.external_resource_rejected",
        ),
        (
            SVG.replace("<s:defs>", "<foreign xmlns=\"urn:foreign\"/><s:defs>"),
            "cem.svg.foreign_content_rejected",
        ),
        (
            SVG.replace(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
                "<!DOCTYPE s:svg SYSTEM 'file:///not-admitted.dtd'>",
            ),
            "cem.svg.dtd_rejected",
        ),
        (
            SVG.replace("Café &amp; diagrams", "&external;"),
            "cem.svg.external_entity_rejected",
        ),
        (
            SVG.replace("0 0 24 24", "0 0 -1 24"),
            "cem.svg.view_box_invalid",
        ),
    ] {
        let (owner, diagnostics) = lifecycle(&source, SVG_MIME);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == code && d.severity.is_hard_violation()),
            "{code}: {diagnostics:?}"
        );
        for result in [
            retain_lifecycle(owner),
            import_data_bytes(source.as_bytes(), SVG_MIME, "cem", URI),
            import_data(&source, SVG_MIME, "xpath", URI),
        ] {
            assert!(result.is_err(), "{code} must refuse import");
        }
    }
}

#[test]
fn mathml_hard_profile_facts_refuse_import_but_annotation_warnings_remain_advisory() {
    for (source, mime, code) in [
        (
            MATH.replace(MATH_NS, "urn:wrong"),
            MATH_MIME,
            "cem.mathml.namespace_missing",
        ),
        (
            MATH.replace("m:math", "m:document"),
            MATH_MIME,
            "cem.mathml.root_not_math",
        ),
        (
            MATH.to_owned(),
            CONTENT_MIME,
            "cem.mathml.malformed_expression",
        ),
        (
            CONTENT_MATH.to_owned(),
            PRESENTATION_MIME,
            "cem.mathml.malformed_expression",
        ),
        (
            MATH.replace("<m:msup>", "<foreign xmlns=\"urn:foreign\"/><m:msup>"),
            MATH_MIME,
            "cem.mathml.foreign_content_rejected",
        ),
        (
            MATH.replace(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
                "<!DOCTYPE m:math SYSTEM 'file:///not-admitted.dtd'>",
            ),
            MATH_MIME,
            "cem.mathml.dtd_rejected",
        ),
        (
            MATH.replace("<m:mi>x", "<m:mi>&external;"),
            MATH_MIME,
            "cem.mathml.external_entity_rejected",
        ),
    ] {
        let (owner, diagnostics) = lifecycle(&source, mime);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == code && d.severity.is_hard_violation()),
            "{code}: {diagnostics:?}"
        );
        assert!(retain_lifecycle(owner).is_err(), "retained {code}");
        assert!(
            import_data_bytes(source.as_bytes(), mime, "cem", URI).is_err(),
            "bytes {code}"
        );
    }
    let source = MATH.replace(
        "</m:math>",
        "<m:annotation encoding=\"application/json\" src=\"formula.json\"/></m:math>",
    );
    let (owner, diagnostics) = lifecycle(&source, MATH_MIME);
    assert!(diagnostics
        .iter()
        .any(|d| d.code == "cem.mathml.external_annotation_rejected"
            && d.severity == Severity::Warning));
    assert!(!diagnostics.iter().any(|d| d.severity.is_hard_violation()));
    assert!(retain_lifecycle(owner).is_ok());
    assert!(import_data_bytes(source.as_bytes(), MATH_MIME, "cem", URI).is_ok());
}

#[test]
fn svg_and_mathml_imports_keep_document_event_depth_and_string_limits() {
    for (mime, root, ns, leaf) in [
        (SVG_MIME, "svg", SVG_NS, "g"),
        (MATH_MIME, "math", MATH_NS, "mrow"),
    ] {
        let wrap = |body: String| {
            format!("<f:{root} xmlns:f=\"{ns}\" aria-label=\"Bounded fixture\">{body}</f:{root}>")
        };
        let large = wrap(format!("<f:{leaf}/>").repeat(8192));
        assert!(import_data_bytes(large.as_bytes(), mime, "cem", URI).is_ok());
        assert!(import_data(&large, mime, "cem", URI)
            .unwrap_err()
            .contains("32 KiB"));
        let nested = wrap(format!(
            "{}{}",
            format!("<f:{leaf}>").repeat(65),
            format!("</f:{leaf}>").repeat(65)
        ));
        let (owner, _) = lifecycle(&nested, mime);
        assert!(retain_lifecycle(owner).unwrap_err().contains("64-level"));
        assert!(import_data_bytes(nested.as_bytes(), mime, "cem", URI)
            .unwrap_err()
            .contains("64-level"));
        let over_events = wrap(format!("<f:{leaf}/>").repeat(65_536));
        let (owner, _) = lifecycle(&over_events, mime);
        assert!(retain_lifecycle(owner).unwrap_err().contains("65536-event"));
        assert!(import_data_bytes(over_events.as_bytes(), mime, "cem", URI)
            .unwrap_err()
            .contains("65536-event"));
    }
}

#[test]
fn family_cem_inspection_uses_imported_namespaces_and_original_token_coordinates() {
    use cem_ml::{
        engine::{
            CemMlEngine, EngineContext, EngineInput, FormatIdentity, InspectRequest, InspectView,
        },
        real::RealCemMlEngine,
        run_config::ScopeConfig,
    };
    for (source, mime, namespace, token) in [
        (SVG, SVG_MIME, SVG_NS, "s:title"),
        (MATH, MATH_MIME, MATH_NS, "m:mi"),
        (MATH, PRESENTATION_MIME, MATH_NS, "m:mi"),
        (CONTENT_MATH, CONTENT_MIME, MATH_NS, "m:ci"),
    ] {
        for show in [InspectView::Ast, InspectView::Tree] {
            let response = RealCemMlEngine::new()
                .inspect(InspectRequest {
                    input: EngineInput {
                        uri: URI.into(),
                        bytes: source.as_bytes().to_vec(),
                        from_format: None,
                        identity: Some(FormatIdentity {
                            content_type: Some(mime.into()),
                            ..Default::default()
                        }),
                        root_scope: Default::default(),
                    },
                    show,
                    presentation_scope: Some(ScopeConfig {
                        cemt_formatter_profile: Some("tabular".into()),
                        output_color_type: Some("none".into()),
                        ..Default::default()
                    }),
                    context: EngineContext::default(),
                })
                .unwrap();
            let primary = response.primary_bytes.unwrap();
            assert_eq!(
                primary.schema.as_deref(),
                Some("https://cem.dev/ns/projection/ast/1")
            );
            let output = std::str::from_utf8(&primary.bytes).unwrap();
            assert!(
                output.contains(&format!("@namespace={namespace}")),
                "{mime}: {output}"
            );
            assert!(
                output.contains(&format!(
                    "@byte-offset={}",
                    source.find(&format!("<{token}>")).unwrap()
                )),
                "{mime}: {output}"
            );
            assert!(output.contains(URI), "{mime}: {output}");
            assert!(!output.contains('\u{1b}'));
        }
    }
}
