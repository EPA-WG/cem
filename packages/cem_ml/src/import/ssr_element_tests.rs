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

#[test]
fn ssr_cem_payload_import_keeps_source_envelope_and_maps_decoded_namespaced_ast() {
    let original = document("<html><body><template type='text/cem-ml'>@ns h = &quot;http://www.w3.org/1999/xhtml&quot;\n{h:b @slot=heading | Title &amp;amp; text}</template></body></html>");
    let imported = imported(original.clone());
    let id = template(&imported);
    let control =
        crate::operation_control::OperationControl::new(crate::scheduler::AbortSignal::new());
    let payload = retain_inline_cem_payload(
        imported.tree.clone(),
        id,
        &control,
        &crate::value::artifact::CemValueArtifactLimits::default(),
    )
    .unwrap();
    assert!(Arc::ptr_eq(payload.source_tree(), &imported.tree));
    assert_eq!(payload.template_node(), id);
    let tree = payload.tree();
    let b = tree
        .ast()
        .nodes
        .iter()
        .enumerate()
        .find_map(|(id, _)| {
            tree.node(id as u32)
                .and_then(|n| n.name.as_ref())
                .filter(|n| n.local_name == "b")
                .map(|_| id as u32)
        })
        .unwrap();
    assert_eq!(
        tree.node(b).unwrap().name.as_ref().unwrap().namespace_uri,
        "http://www.w3.org/1999/xhtml"
    );
    assert!(tree.node_source_map(b).unwrap().frames.iter().any(|f|matches!(&f.transform,TransformKind::SourceBoundary{uri} if uri=="file:///source/page.html")));
    assert_eq!(
        tree.text_fragments(b).unwrap().collect::<String>().trim(),
        "Title &amp; text"
    );
    let offset = original.source_text.find("{h:b").unwrap() as u64;
    assert!(tree
        .node_source_map(b)
        .unwrap()
        .frames
        .iter()
        .any(|f| matches!(f.span,FrameSpan::Single(r) if r.start<=offset && r.end()>offset)));
    assert!(original.source_text.contains("Title &amp;amp; text"));
}

#[test]
fn ssr_cem_payload_import_bounds_cancellation_and_invalid_source_are_atomic() {
    use crate::{
        operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
        scheduler::AbortSignal,
        value::artifact::CemValueArtifactLimits,
    };
    for text in [
        "{p | Unclosed",
        "{unbound:p | Invalid}",
        "@ns h = &quot;urn:unadmitted&quot;\n{h:p | Unadmitted}",
        "{p @name=\"Unclosed}",
        "<b>HTML child</b>",
    ] {
        let imported = imported(document(&format!(
            "<html><template type='text/cem-ml'>{text}</template></html>"
        )));
        let c = OperationControl::new(AbortSignal::new());
        assert!(
            retain_inline_cem_payload(
                imported.tree.clone(),
                template(&imported),
                &c,
                &Default::default()
            )
            .is_err(),
            "{text}"
        );
        assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    }
    let imported = imported(document(
        "<html><template type='text/cem-ml'>{p | {b | Original}}</template></html>",
    ));
    for limits in [
        CemValueArtifactLimits {
            max_bytes: 1,
            ..Default::default()
        },
        CemValueArtifactLimits {
            max_values: 1,
            ..Default::default()
        },
        CemValueArtifactLimits {
            max_depth: 1,
            ..Default::default()
        },
    ] {
        let c = OperationControl::new(AbortSignal::new());
        assert!(
            retain_inline_cem_payload(imported.tree.clone(), template(&imported), &c, &limits)
                .is_err()
        );
        assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    }
    for policy in [
        crate::scheduler::ScopePolicy::host_root().with_memory_bytes(1),
        crate::scheduler::ScopePolicy::host_root().with_stack_depth(1),
    ] {
        let c = OperationControl::with_root_policy(AbortSignal::new(), policy).unwrap();
        assert!(retain_inline_cem_payload(
            imported.tree.clone(),
            template(&imported),
            &c,
            &Default::default()
        )
        .is_err());
        assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    }
    let signal = AbortSignal::new();
    signal.abort();
    let c = OperationControl::new(signal);
    assert!(retain_inline_cem_payload(
        imported.tree.clone(),
        template(&imported),
        &c,
        &Default::default()
    )
    .is_err());
}
#[test]
fn ssr_cem_payload_import_retains_memory_and_distinguishes_identical_source_boundaries() {
    use crate::operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
    let imported=imported(document("<html><template type='text/cem-ml'>{p | Same}</template><template type='text/cem-ml'>{p | Same}</template></html>"));
    let ids = imported
        .elements
        .iter()
        .filter_map(|(id, _, _)| {
            (imported.tree.node(*id)?.name.as_ref()?.local_name == "template").then_some(*id)
        })
        .collect::<Vec<_>>();
    let c = OperationControl::new(crate::scheduler::AbortSignal::new());
    let first =
        retain_inline_cem_payload(imported.tree.clone(), ids[0], &c, &Default::default()).unwrap();
    let second =
        retain_inline_cem_payload(imported.tree.clone(), ids[1], &c, &Default::default()).unwrap();
    assert_ne!(first.tree().source_key(1), second.tree().source_key(1));
    assert!(!first.matches_source(&imported.tree, ids[1]));
    assert!(!first.matches_operation(&OperationControl::new(crate::scheduler::AbortSignal::new())));
    let retained = first.tree().clone();
    drop(first);
    drop(second);
    assert!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap() > 0);
    drop(retained);
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}

#[test]
fn ssr_cem_payload_shorthand_canonicalizes_a_native_view_without_replacing_source() {
    for kind in [
        "cem-ml",
        " CeM-mL ",
        "\u{feff}TEXT/CEM-ML\u{a0}",
        "text/cem-ml",
    ] {
        let imported = imported(document(&format!(
            "<html><template type='{kind}'>{{item | Original &amp;amp; text}}</template></html>"
        )));
        let id = template(&imported);
        let c =
            crate::operation_control::OperationControl::new(crate::scheduler::AbortSignal::new());
        let admitted =
            retain_inline_cem_payload(imported.tree.clone(), id, &c, &Default::default()).unwrap();
        let value = |tree: &RetainedCemTree, id: u32| {
            tree.node(id)
                .unwrap()
                .attributes
                .iter()
                .find_map(|a| {
                    let a = tree.node(*a).unwrap();
                    (a.name.as_ref().unwrap().local_name == "type").then(|| a.value.clone())
                })
                .unwrap()
        };
        assert_eq!(value(admitted.source_tree(), id), kind);
        assert_eq!(
            value(
                admitted.canonical_tree(),
                admitted.canonical_template_node()
            ),
            "text/cem-ml"
        );
        assert_eq!(
            Arc::ptr_eq(admitted.source_tree(), admitted.canonical_tree()),
            kind == "text/cem-ml"
        );
        assert!(admitted.matches_source(&imported.tree, id));
        assert!(admitted.matches_canonical_source(
            admitted.canonical_tree(),
            admitted.canonical_template_node()
        ));
        for (old, new) in imported.tree.node(id).unwrap().children.iter().zip(
            &admitted
                .canonical_tree()
                .node(admitted.canonical_template_node())
                .unwrap()
                .children,
        ) {
            assert_eq!(
                imported.tree.source_key(*old),
                admitted.canonical_tree().source_key(*new)
            );
            assert_eq!(
                imported.tree.node_source_map(*old),
                admitted.canonical_tree().node_source_map(*new)
            );
        }
    }
    for kind in [
        "\u{85}cem-ml",
        "application/cem",
        "cem-ml; charset=utf-8",
        "text/html",
    ] {
        assert!(canonical_cem_payload_type(kind).is_none());
    }
}

#[test]
fn ssr_element_continuation_preserves_original_handles_and_refuses_stale_views() {
    use crate::{
        operation_control::OperationControl, parser::tree_apply::apply_children,
        scheduler::AbortSignal, value::CemValueNode,
    };
    let owner=document("<!doctype html><html><body><template id='x-a' type='text/cem-ml'>{p | A &amp; B}</template><div>Old</div></body></html>");
    let original = imported(owner);
    let t = template(&original);
    let target = original
        .elements
        .iter()
        .map(|e| e.0)
        .find(|id| {
            original
                .tree
                .node(*id)
                .unwrap()
                .name
                .as_ref()
                .unwrap()
                .local_name
                == "div"
        })
        .unwrap();
    let c = OperationControl::new(AbortSignal::new());
    let text = || {
        Arc::new(vec![CemValueNode::<(), ()>::Text {
            text: "Derived".into(),
            source_map: Default::default(),
        }])
    };
    let first = apply_children(
        original.tree.clone(),
        target,
        text(),
        Arc::new(()),
        Default::default(),
        &c,
    )
    .unwrap();
    let next = original.after_application(&first).unwrap();
    next.validate_html_text_projection().unwrap();
    let remapped = first.output_for_source(t).unwrap();
    assert_eq!(
        next.inline_cem_template_source(remapped).unwrap().text(),
        "{p | A & B}"
    );
    let second = apply_children(
        next.tree.clone(),
        first.output_target(),
        text(),
        Arc::new(()),
        Default::default(),
        &c,
    )
    .unwrap();
    assert!(original.after_application(&second).is_err());
    assert!(next.after_application(&second).is_ok());
}
