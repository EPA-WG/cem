use cem_ml::ssr_artifact::SsrLifecycleState;
use cem_ql::{
    ssr_island::prepare_inline_island, ssr_preparation::prepare_inline_dce,
    ssr_render::render_and_apply_inline_dce,
};
use std::sync::Arc;
#[path = "support/ssr_page.rs"]
mod fixture;
use fixture::{control, owner, page};
fn input(
    payload: &str,
    c: &cem_ml::operation_control::OperationControl,
) -> cem_ql::ssr_island::PreparedInlineIsland {
    input_with_type(payload, "text/cem-ml", c)
}
fn input_with_type(
    payload: &str,
    kind: &str,
    c: &cem_ml::operation_control::OperationControl,
) -> cem_ql::ssr_island::PreparedInlineIsland {
    let source=owner(&format!("<cem-element tag='x-inline'><template id='x-inline' type='text/cem-ml'>{{attribute @name=label | Committed}}{{p | Visible}}</template></cem-element><x-inline><template type='{kind}'>{payload}</template></x-inline>"));
    prepare_inline_island(
        prepare_inline_dce(page(&source, "memory:output"), c).unwrap(),
        "one",
        c,
    )
    .unwrap()
}
#[test]
fn source_payload_keeps_one_authority_and_reuses_its_native_ast_through_host_publication() {
    let c = control();
    let input = input("{item @key=original | {$ 1 div 0}}", &c);
    let admitted = input
        .bindings()
        .native_payload_source
        .as_ref()
        .unwrap()
        .clone();
    let envelope = input.payload_envelope().unwrap();
    assert!(Arc::ptr_eq(admitted.source_tree(), envelope.owner()));
    assert_eq!(admitted.template_node(), envelope.node_id());
    assert_eq!(input.payload().len(), 1);
    assert_eq!(input.payload()[0].node_id(), envelope.node_id());
    let result = render_and_apply_inline_dce(input, &c).unwrap();
    assert!(Arc::ptr_eq(
        result
            .committed_island()
            .bindings()
            .native_payload_source
            .as_ref()
            .unwrap(),
        &admitted
    ));
    assert_eq!(
        result.page().lifecycle(),
        SsrLifecycleState::Partial {
            reason: cem_ml::ssr_artifact::SsrPartialReason::Incomplete
        }
    );
    assert!(!result.page().adoption_identity().hydration_ready());
    assert_eq!(
        cem_ql::render::render_plan_to_html(result.projected_plan()),
        "<p>Visible</p>"
    );
    assert!(result
        .page()
        .source_document()
        .source_text
        .contains("{$ 1 div 0}"));
}
#[test]
fn invalid_cem_source_is_rejected_before_canonical_publication() {
    let c = control();
    for payload in ["{p | Unclosed", "{unknown:p | Unbound}"] {
        let source=owner(&format!("<cem-element tag='x-inline'><template id='x-inline' type='text/cem-ml'>{{p | Visible}}</template></cem-element><x-inline><template type='text/cem-ml'>{payload}</template></x-inline>"));
        let prepared = prepare_inline_dce(page(&source, "memory:output"), &c).unwrap();
        let errors = prepare_inline_island(prepared, "one", &control()).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|d| d.code == "cem.ssr.payload_source_invalid"),
            "{errors:?}"
        );
    }
}

#[test]
fn source_admission_cannot_be_restarted_under_a_new_operation() {
    let c = control();
    let input = input("{item | Original}", &c);
    let admitted = input
        .bindings()
        .native_payload_source
        .as_ref()
        .unwrap()
        .clone();
    let errors = render_and_apply_inline_dce(input, &control()).unwrap_err();
    assert_eq!(errors[0].code, "cem.ssr.payload_source_stale");
    assert!(
        c.memory_charged(cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID)
            .unwrap()
            > 0
    );
    drop(admitted);
    assert_eq!(
        c.memory_charged(cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID)
            .unwrap(),
        0
    );
}

#[test]
fn shorthand_preserves_authored_template_but_exposes_one_canonical_native_view() {
    let c = control();
    let input = input_with_type("{item | {$ 1 div 0}}", "cem-ml", &c);
    let source = input.bindings().native_payload_source.as_ref().unwrap();
    let original = source.source_tree().node(source.template_node()).unwrap();
    let type_value = |tree: &cem_ml::parser::tree::RetainedCemTree, id: u32| {
        tree.node(id)
            .unwrap()
            .attributes
            .iter()
            .find_map(|id| {
                let a = tree.node(*id).unwrap();
                (a.name.as_ref().unwrap().local_name == "type").then(|| a.value.clone())
            })
            .unwrap()
    };
    assert_eq!(
        type_value(source.source_tree(), source.template_node()),
        "cem-ml"
    );
    assert_eq!(
        type_value(source.canonical_tree(), source.canonical_template_node()),
        "text/cem-ml"
    );
    assert_eq!(
        original.children.len(),
        source
            .canonical_tree()
            .node(source.canonical_template_node())
            .unwrap()
            .children
            .len()
    );
    let [cem_ql::eval::Item::Record(frame)] = input.bindings().bindings["datadom"].items.as_slice()
    else {
        panic!()
    };
    let [cem_ql::eval::Item::Record(slots)] = frame["slots"].as_slice() else {
        panic!()
    };
    let root = cem_ql::eval::retained_cem_node(&slots[""][0]).unwrap();
    assert!(Arc::ptr_eq(root.owner(), source.canonical_tree()));
    assert_eq!(type_value(root.owner(), root.node_id()), "text/cem-ml");
    let result = render_and_apply_inline_dce(input, &c).unwrap();
    assert!(result
        .page()
        .source_document()
        .source_text
        .contains("type='cem-ml'"));
    assert!(!result.page().adoption_identity().hydration_ready());
}

#[test]
fn external_native_adapter_keeps_exact_resource_ranges_and_the_original_envelope() {
    use cem_ml::{
        import::element_tree::retain_external_cem_payload,
        parser::CemAstNode,
        source_map::{FrameSpan, TransformKind},
    };
    let c = control();
    let source = owner(
        r#"<cem-element tag="x-inline"><template id="x-inline" type="text/cem-ml">{p | Visible}</template></cem-element><x-inline><template type="text/cem-ml" data-cem-payload-src="./payload.cem"></template></x-inline>"#,
    );
    let prepared = prepare_inline_dce(page(&source, "memory:output"), &c).unwrap();
    let tree = &prepared.elements().tree;
    let stub = tree.node(prepared.instance_node()).unwrap().children[0];
    let text = "\n{item | {#nodes[1 div 0]}}";
    let uri = "https://ssr.test/payload.cem";
    let admitted = retain_external_cem_payload(
        tree.clone(),
        stub,
        uri,
        text.as_bytes().to_vec(),
        &c,
        &Default::default(),
    )
    .unwrap();
    assert!(admitted.matches_source(tree, stub));
    assert!(admitted.matches_operation(&c));
    assert!(!admitted.matches_operation(&control()));
    assert_eq!(admitted.tree().source_uri(), uri);
    assert_eq!(admitted.tree().base_uri(), Some(uri));
    let (id, _) = admitted.tree().ast().nodes.iter().enumerate().find(|(_, node)| {
        matches!(node, CemAstNode::Reference { expression, targets: None, .. } if expression == "#nodes[1 div 0]")
    }).expect("CEM reference remains inert");
    let map = admitted.tree().node_source_map(id as u32).unwrap();
    let offset = text.find("1 div 0").unwrap() as u64;
    assert!(map.frames.iter().any(|frame| {
        matches!(&frame.transform, TransformKind::SourceBoundary { uri: source } if source == uri)
            && matches!(frame.span, FrameSpan::Single(range) if range.start <= offset && offset < range.end())
    }), "{map:?}");
    let canonical = admitted.canonical_tree();
    let template = canonical.node(admitted.canonical_template_node()).unwrap();
    assert_eq!(canonical.node(template.children[0]).unwrap().value, text);
    assert_eq!(template.attributes.len(), 1);
    assert_eq!(admitted.external_input().unwrap().byte_length, text.len());
}
