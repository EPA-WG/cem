use cem_ml::{
    parser::tree::CemTreeNodeKind,
    source_map::TransformKind,
    ssr_artifact::{SsrLifecycleState, SsrPartialReason},
    validation::html::HtmlDocumentAst,
};
use cem_ql::ssr_preparation::{prepare_inline_dce, MAX_INLINE_TEMPLATE_BYTES};
use std::sync::Arc;

const DECL: &str = "<cem-element tag='x-inline' version='1'><template id='x-inline' type='text/cem-ml'>{p | Hello &amp; welcome}</template></cem-element>";
const INSTANCE: &str = "<x-inline label='World'><b>Original payload</b></x-inline>";
#[path = "support/ssr_page.rs"]
mod fixture;
use fixture::{control, owner, page};
fn error(body: &str) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let owner = owner(body);
    prepare_inline_dce(page(&owner, "memory:output"), &control()).unwrap_err()
}

#[test]
fn prepares_one_inline_declaration_and_instance_with_original_owners() {
    let owner = owner(&format!("{DECL}{INSTANCE}"));
    let original = (*owner).clone();
    let prepared = prepare_inline_dce(page(&owner, "file:///dist/index.html"), &control()).unwrap();
    assert!(Arc::ptr_eq(prepared.page().source_document(), &owner));
    let imported_owner = prepared
        .elements()
        .tree
        .native_owner()
        .unwrap()
        .clone()
        .downcast::<HtmlDocumentAst>()
        .unwrap();
    assert!(Arc::ptr_eq(&imported_owner, &owner));
    let tree = &prepared.elements().tree;
    for (id, name) in [
        (prepared.declaration_node(), "cem-element"),
        (prepared.template_node(), "template"),
        (prepared.instance_node(), "x-inline"),
    ] {
        assert_eq!(
            tree.node(id).unwrap().name.as_ref().unwrap().local_name,
            name
        );
    }
    assert_eq!(tree.base_uri(), Some("https://ssr.example.test/guide/"));
    assert_eq!(tree.document_uri(), Some("https://ssr.example.test/guide/"));
    assert_eq!(tree.source_uri(), "file:///source/page.html");
    assert_eq!(prepared.tag(), "x-inline");
    assert_eq!(
        prepared.page().lifecycle(),
        SsrLifecycleState::Partial {
            reason: SsrPartialReason::Incomplete
        }
    );
    assert_eq!(prepared.page().usage().instances, 1);
    assert_eq!(prepared.page().usage().render_passes, 0);
    assert!(!prepared.page().adoption_identity().hydration_ready());
    assert!(!prepared.page().static_fallback_candidate());
    assert_eq!(*owner, original);
}

#[test]
fn decoded_template_uses_existing_native_compiler_and_preserves_page_offsets() {
    let owner = owner(&format!("{DECL}{INSTANCE}"));
    let prepared = prepare_inline_dce(page(&owner, "memory:output"), &control()).unwrap();
    let artifact = prepared.declaration_owner().artifact();
    let plan = cem_ql::render::render_compiled_template(artifact, &Default::default());
    assert!(plan
        .diagnostics
        .iter()
        .all(|diagnostic| !diagnostic.severity.is_hard_violation()));
    let rendered = cem_ql::render::render_plan_to_html(&plan);
    assert_eq!(rendered, "<p>Hello &amp; welcome</p>");
    let body_offset = owner.source_text.find("{p |").unwrap() as u64;
    let cem_ml::value::CemValueNode::Element {
        source_map: map, ..
    } = &plan.nodes[0]
    else {
        panic!("native element plan")
    };
    assert!(map.frames.iter().any(|frame| matches!(&frame.transform, TransformKind::SourceBoundary { uri } if uri == "file:///source/page.html")));
    assert!(match map.origin().unwrap().span {
        cem_ml::source_map::FrameSpan::Single(range) => range.start >= body_offset,
        _ => false,
    });
    assert!(!prepared.page().adoption_identity().hydration_ready());
}

#[test]
fn inert_templates_and_foreign_namespaces_do_not_register_declarations_or_instances() {
    let body = format!("<template>{DECL}{INSTANCE}</template><svg><cem-element tag='foreign-tag'></cem-element><x-inline/></svg>{DECL}{INSTANCE}");
    let owner = owner(&body);
    let prepared = prepare_inline_dce(page(&owner, "memory:output"), &control()).unwrap();
    let tree = &prepared.elements().tree;
    assert_eq!(
        tree.node(prepared.instance_node()).unwrap().kind,
        CemTreeNodeKind::Element
    );
    assert_eq!(prepared.page().usage().instances, 1);
}

#[test]
fn refuses_ambiguous_declarations_and_instances_before_preparation() {
    for (body, code) in [
        (format!("{INSTANCE}"), "cem.ssr.inline_declaration_count"),
        (
            format!("{DECL}{DECL}{INSTANCE}"),
            "cem.ssr.inline_declaration_count",
        ),
        (format!("{DECL}"), "cem.ssr.inline_instance_count"),
        (
            format!("{DECL}{INSTANCE}{INSTANCE}"),
            "cem.ssr.inline_instance_count",
        ),
    ] {
        assert_eq!(error(&body)[0].code, code, "{body}");
    }
}

#[test]
fn refuses_external_capability_mismatched_template_and_unknown_participant_shapes() {
    for (body, code) in [
        (
            format!(
                "{}{INSTANCE}",
                DECL.replace(
                    "tag='x-inline'",
                    "tag='x-inline' capability='action-command'"
                )
            ),
            "cem.ssr.inline_declaration_unsupported",
        ),
        (
            format!(
                "{}{INSTANCE}",
                DECL.replace("id='x-inline'", "id='other-template'")
            ),
            "cem.ssr.inline_template_unsupported",
        ),
        (
            format!("{DECL}{INSTANCE}<unknown-widget></unknown-widget>"),
            "cem.ssr.participant_unavailable",
        ),
        (
            format!("{DECL}{INSTANCE}<button is='unknown-widget'>Unknown participant</button>"),
            "cem.ssr.participant_unavailable",
        ),
        (
            format!(
                "{}{INSTANCE}",
                DECL.replace("Hello &amp; welcome", "{$missing}")
            ),
            "cem.ssr.inline_template_invalid",
        ),
    ] {
        assert_eq!(error(&body)[0].code, code, "{body}");
    }
}

#[test]
fn marked_island_selects_resume_refusal_without_recapturing_rendered_siblings() {
    let instance = "<x-inline><template data-cem-island='instance'><p>old island</p></template><p>Retained output</p></x-inline>";
    assert_eq!(
        error(&format!("{DECL}{instance}"))[0].code,
        "cem.ssr.resume_unavailable"
    );
}

#[test]
fn cancelled_preparation_and_oversized_template_leave_source_untouched() {
    let owner = owner(&format!("{DECL}{INSTANCE}"));
    let original = (*owner).clone();
    let cancelled = control();
    cancelled.abort_signal().abort();
    assert_eq!(
        prepare_inline_dce(page(&owner, "memory:output"), &cancelled).unwrap_err()[0].code,
        "host-cancellation"
    );
    assert_eq!(*owner, original);
    let large = DECL.replace(
        "Hello &amp; welcome",
        &"x".repeat(MAX_INLINE_TEMPLATE_BYTES + 1),
    );
    assert_eq!(
        error(&format!("{large}{INSTANCE}"))[0].code,
        "cem.ssr.inline_template_bounds"
    );
}

#[test]
fn preparation_errors_keep_original_source_provenance() {
    let errors = error(&format!("{DECL}{INSTANCE}{INSTANCE}"));
    assert_eq!(errors[0].uri.as_deref(), Some("file:///source/page.html"));
    assert!(errors[0].source_map.as_ref().unwrap().frames.iter().any(|frame| matches!(&frame.transform, TransformKind::SourceBoundary {uri} if uri=="file:///source/page.html")));
}

#[test]
fn physical_output_roots_do_not_change_preparation_or_compiled_template() {
    let owner = owner(&format!("{DECL}{INSTANCE}"));
    let first = prepare_inline_dce(page(&owner, "file:///one/index.html"), &control()).unwrap();
    let second = prepare_inline_dce(page(&owner, "file:///two/index.html"), &control()).unwrap();
    assert_eq!(first.instance_node(), second.instance_node());
    let render = |prepared: &cem_ql::ssr_preparation::PreparedInlineDce| {
        cem_ql::render::render_plan_to_html(&cem_ql::render::render_compiled_template(
            prepared.declaration_owner().artifact(),
            &Default::default(),
        ))
    };
    assert_eq!(render(&first), render(&second));
    assert_ne!(
        first.page().route().output_uri,
        second.page().route().output_uri
    );
}

#[test]
fn compiler_preserves_transformed_origins_and_does_not_invent_generated_origins() {
    let original = owner(&format!("{DECL}{INSTANCE}"));
    for mapped in [true, false] {
        let mut native = (*original).clone();
        let origin = cem_ml::source_map::SourceMapStack {
            frames: vec![cem_ml::source_map::SourceMapFrame {
                source_id: cem_ml::source::SourceId(7),
                span: cem_ml::source_map::FrameSpan::Single(cem_ml::source::ByteRange::new(77, 11)),
                transform: TransformKind::CemAstBuilder,
            }],
        }
        .with_source_uri("memory:authored.cem");
        native.origin_spans = Some(cem_ml::source_map::SourceMapIndex::new(if mapped {
            vec![cem_ml::interpreter::OutputSpan {
                output_range: cem_ml::source::ByteRange::new(0, native.source_text.len() as u32),
                origin: origin.clone(),
            }]
        } else {
            vec![]
        }));
        let native = Arc::new(native);
        let prepared = prepare_inline_dce(page(&native, "memory:output"), &control()).unwrap();
        let cem_ql::render::TemplateNode::Element { source_map, .. } =
            &prepared.declaration_owner().artifact().nodes[0]
        else {
            panic!("native template node")
        };
        if mapped {
            assert_eq!(source_map.origin(), origin.origin());
            assert!(source_map
                .frames
                .iter()
                .any(|frame| matches!(frame.transform, TransformKind::HandoffBoundary { .. })));
        } else {
            assert!(source_map.frames.is_empty());
        }
        assert!(!prepared.page().adoption_identity().hydration_ready());
    }
}

#[test]
fn preparation_preserves_completed_control_and_refuses_already_partial_pages() {
    let owner = owner(&format!("{DECL}{INSTANCE}"));
    let completed = control();
    completed
        .complete_scope(cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID)
        .unwrap();
    let errors = prepare_inline_dce(page(&owner, "memory:output"), &completed).unwrap_err();
    assert_eq!(errors[0].code, "cem.control.scope_completed");
    let initial = page(&owner, "memory:output");
    let usage = initial.usage();
    let partial = initial
        .into_partial(SsrPartialReason::Incomplete, usage, vec![])
        .unwrap();
    assert_eq!(
        prepare_inline_dce(partial, &control()).unwrap_err()[0].code,
        "cem.ssr.preparation_state"
    );
}

#[test]
fn stylesheet_module_and_import_work_is_not_admitted_by_preparation() {
    for body in [
        "{style | ``` p { color: red; } ```}{p | Text}",
        "{module-map | {resource @specifier=card @target=./card.cem @content-type=text/cem-ml}}{p | Text}",
    ] {
        let declaration = DECL.replace("{p | Hello &amp; welcome}", body);
        let errors = error(&format!("{declaration}{INSTANCE}"));
        assert_eq!(errors[0].code, "cem.ssr.inline_template_unsupported");
    }
    assert_eq!(
        error(&format!("{DECL}{INSTANCE}<script>alert(1)</script>"))[0].code,
        "cem.ssr.page_import_refused"
    );
}

#[test]
fn tokenizer_diagnostics_retain_handoff_origins_without_fabricated_coordinates() {
    use cem_ml::{
        source::{ByteRange, SourceId},
        source_map::{FrameSpan, SourceMapFrame, SourceMapStack},
    };
    for mapped in [true, false] {
        let origin = SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(7),
                span: FrameSpan::Single(ByteRange::new(77, 11)),
                transform: TransformKind::CemAstBuilder,
            }],
        }
        .with_source_uri("memory:authored.cem");
        for (source, code) in [
            ("{p Hello {.name}}", "cem.tokenizer.bare_brace_text"),
            ("{p | {$missing}}", "cem.ql.render.compile_failed"),
        ] {
            let artifact = cem_ql::render::compile_template_with_source_map(
                source,
                &Default::default(),
                &|_| mapped.then(|| origin.clone()),
            );
            let diagnostic = artifact
                .diagnostics
                .iter()
                .find(|d| d.code == code)
                .unwrap();
            if mapped {
                assert_eq!(
                    diagnostic.source_map.as_ref().unwrap().origin(),
                    origin.origin()
                );
                assert_eq!(diagnostic.uri.as_deref(), Some("memory:authored.cem"));
                assert_eq!(diagnostic.byte_offset, Some(77));
            } else {
                assert!(diagnostic
                    .source_map
                    .as_ref()
                    .is_none_or(|map| map.frames.is_empty()));
                assert_eq!(diagnostic.uri, None);
                assert_eq!(diagnostic.byte_offset, None);
            }
            assert_eq!(diagnostic.line, None);
            assert_eq!(diagnostic.column, None);
        }
    }
}
