use cem_ml::{
    operation_control::ROOT_EXECUTION_SCOPE_ID, parser::tree::RetainedCemTree,
    ssr_artifact::SsrLifecycleState,
};
use cem_ql::{
    ssr_island::prepare_inline_island,
    ssr_preparation::prepare_inline_dce,
    ssr_render::{render_and_apply_inline_dce, AppliedInlineDce},
};
use std::{collections::BTreeMap, sync::Arc};
#[path = "support/ssr_page.rs"]
mod fixture;
use fixture::{control, owner, page};
fn prepare(template: &str, attrs: &str) -> cem_ql::ssr_island::PreparedInlineIsland {
    let source=owner(&format!("<cem-element tag='x-inline'><template id='x-inline' type='text/cem-ml'>{template}</template></cem-element><x-inline {attrs}><template><b slot='title'>Original &amp; stable</b></template></x-inline>"));
    prepare_inline_island(
        prepare_inline_dce(page(&source, "memory:output"), &control()).unwrap(),
        "inline-one",
        &control(),
    )
    .unwrap()
}
fn apply(template: &str, attrs: &str) -> AppliedInlineDce {
    render_and_apply_inline_dce(prepare(template, attrs), &control()).unwrap()
}
fn attrs(tree: &RetainedCemTree, id: u32) -> Vec<(String, String)> {
    tree.node(id)
        .unwrap()
        .attributes
        .iter()
        .map(|id| {
            let a = tree.node(*id).unwrap();
            (a.name.as_ref().unwrap().local_name.clone(), a.value.clone())
        })
        .collect()
}
fn sections(r: &AppliedInlineDce) -> (BTreeMap<String, String>, BTreeMap<String, String>) {
    let t = r.application().tree();
    let template = r.application().inserted_children()[0];
    let root = t.node(template).unwrap().children[0];
    let parts = &t.node(root).unwrap().children;
    let entries = |id: u32, key: &str| {
        t.node(id)
            .unwrap()
            .children
            .iter()
            .map(|id| {
                let a = attrs(t, *id).into_iter().collect::<BTreeMap<_, _>>();
                (a[key].clone(), a["value"].clone())
            })
            .collect()
    };
    (entries(parts[1], "name"), entries(parts[2], "key"))
}
#[test]
fn missing_defaults_commit_to_host_and_canonical_attributes() {
    let r = apply("{attribute @name=label | Default}{p | {$ label}}", "");
    assert_eq!(
        attrs(r.application().tree(), r.application().output_target()),
        vec![("label".into(), "Default".into())]
    );
    assert_eq!(
        sections(&r).0,
        BTreeMap::from([("label".into(), "Default".into())])
    );
    assert_eq!(
        cem_ql::render::render_plan_to_html(r.projected_plan()),
        "<p>Default</p>"
    );
    assert_eq!(r.page().usage().render_passes, 1);
    assert!(matches!(
        r.page().lifecycle(),
        SsrLifecycleState::Partial { .. }
    ));
    assert!(!r.page().adoption_identity().hydration_ready());
}
#[test]
fn supplied_empty_attribute_wins_over_default() {
    let r = apply(
        "{attribute @name=label | Default}{p | {$ label}}",
        "label=''",
    );
    assert!(r.plan().host_attribute_updates.is_empty());
    assert_eq!(sections(&r).0["label"], "");
}
#[test]
fn data_defaults_are_visible_to_the_same_render_and_committed_dataset() {
    let r = apply(
        "{attribute @name=data-theme | native}{p | {$ datadom.dataset.theme}}",
        "",
    );
    assert_eq!(
        cem_ql::render::render_plan_to_html(r.projected_plan()),
        "<p>native</p>"
    );
    assert_eq!(
        sections(&r).1,
        BTreeMap::from([("theme".into(), "native".into())])
    );
}
#[test]
fn public_theme_selectors_publish_as_host_attributes_and_dataset_defaults() {
    for (input, spacing) in [("", "normal"), ("data-cem-spacing='sparse'", "sparse")] {
        let r = apply(
            "{attribute @name=data-cem-density | super-compact}{attribute @name=data-cem-spacing | normal}{p | {$ datadom.dataset.cemDensity}}",
            input,
        );
        let host = attrs(r.application().tree(), r.application().output_target())
            .into_iter()
            .collect::<BTreeMap<_, _>>();
        assert_eq!(host["data-cem-density"], "super-compact");
        assert_eq!(host["data-cem-spacing"], spacing);
        assert_eq!(sections(&r).1["cemDensity"], "super-compact");
        assert_eq!(sections(&r).1["cemSpacing"], spacing);
        assert_eq!(
            cem_ql::render::render_plan_to_html(r.projected_plan()),
            "<p>super-compact</p>"
        );
    }
}
#[test]
fn selected_writes_replace_in_place_and_append_new_names_once() {
    let r = apply(
        r#"{attribute @name=label @select='"Selected"'}{attribute @name=data-mode @select='"new"'}{p | {$ datadom.attributes.label}}"#,
        "id='original' label='old' data-mode='old'",
    );
    assert_eq!(
        attrs(r.application().tree(), r.application().output_target()),
        vec![
            ("id".into(), "original".into()),
            ("label".into(), "Selected".into()),
            ("data-mode".into(), "new".into())
        ]
    );
    assert_eq!(sections(&r).0["label"], "Selected");
    assert_eq!(sections(&r).1["mode"], "new");
}
#[test]
fn repeated_updates_have_one_final_host_and_dataset_value() {
    let r = apply(
        r#"{attribute @name=data-mode | default}{attribute @name=data-mode @select='"selected"'}{p | {$ datadom.dataset.mode}}"#,
        "",
    );
    assert_eq!(
        attrs(r.application().tree(), r.application().output_target()),
        vec![("data-mode".into(), "selected".into())]
    );
    assert_eq!(sections(&r).1["mode"], "selected");
    assert_eq!(
        cem_ql::render::render_plan_to_html(r.projected_plan()),
        "<p>selected</p>"
    );
}
#[test]
fn original_source_and_payload_identity_survive_committed_defaults() {
    let input = prepare(
        "{attribute @name=label | Default}{p | Hello}",
        "id='original'",
    );
    let tree = input.prepared().elements().tree.clone();
    let source = input.prepared().page().source_document().clone();
    let identity = input.payload()[0].query_item().identity();
    let bytes = source.source_text.clone();
    let r = render_and_apply_inline_dce(input, &control()).unwrap();
    assert!(Arc::ptr_eq(r.page().source_document(), &source));
    assert_eq!(source.source_text, bytes);
    assert!(Arc::ptr_eq(r.application().source(), &tree));
    assert_eq!(r.input().payload()[0].query_item().identity(), identity);
    assert_eq!(
        attrs(&tree, r.input().prepared().instance_node()),
        vec![("id".into(), "original".into())]
    );
}
#[test]
fn unsafe_host_writes_refuse_without_publishing_either_half() {
    for template in [
        "{attribute @name=onclick | run}{p | Hello}",
        "{attribute @name=src | https://remote.test/file}{p | Hello}",
        "{attribute @name=type | password}{p | Hello}",
        "{attribute @name=dataset | reserved}{p | Hello}",
        "{attribute @name=data-cem-island | instance}{p | Hello}",
        "{attribute @name=data-cem-css-context | forged}{p | Hello}",
    ] {
        let input = prepare(template, "");
        let tree = input.prepared().elements().tree.clone();
        let id = input.prepared().instance_node();
        let bytes = input
            .prepared()
            .page()
            .source_document()
            .source_text
            .clone();
        let error = render_and_apply_inline_dce(input, &control()).unwrap_err();
        assert_eq!(error[0].code, "cem.ssr.host_commit_refused", "{template}");
        assert!(attrs(&tree, id).is_empty());
        assert!(bytes.contains("<x-inline"));
    }
}
#[test]
fn abort_and_completed_scope_refuse_before_any_commit() {
    for complete in [false, true] {
        let input = prepare("{attribute @name=label | Default}{p | Hello}", "");
        let c = control();
        if complete {
            c.complete_scope(ROOT_EXECUTION_SCOPE_ID).unwrap()
        } else {
            c.abort_signal().abort()
        };
        let errors = render_and_apply_inline_dce(input, &c).unwrap_err();
        assert_eq!(
            errors[0].code,
            if complete {
                "cem.control.scope_completed"
            } else {
                "host-cancellation"
            }
        );
        assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    }
}

#[test]
fn committed_bindings_and_payload_references_remain_native_and_current() {
    use cem_ql::eval::output::OutputView;
    fn payload_reference(template: &cem_ql::eval::Item) -> String {
        let view = template
            .view()
            .unwrap()
            .downcast_ref::<OutputView>()
            .unwrap();
        let cem_ql::render::RenderPlanNode::Element { children, .. } = view.node() else {
            panic!()
        };
        let cem_ql::render::RenderPlanNode::Element { children, .. } = &children[0] else {
            panic!()
        };
        let cem_ql::render::RenderPlanNode::Element { children, .. } = &children[3] else {
            panic!()
        };
        let cem_ql::render::RenderPlanNode::Reference { reference, .. } = &children[0] else {
            panic!()
        };
        reference.identity()
    }
    let input = prepare(
        "{attribute @name=data-theme | native}{p | {$ datadom.dataset.theme}}",
        "data-other='kept'",
    );
    let reference = payload_reference(input.template());
    let r = render_and_apply_inline_dce(input, &control()).unwrap();
    assert_eq!(
        payload_reference(r.committed_island().template()),
        reference
    );
    let plan = cem_ql::render::render_template(
        "{p | {$ datadom.dataset.theme}-{$ datadom.dataset.other}}",
        r.committed_island().bindings(),
    );
    assert!(plan.diagnostics.is_empty());
    assert_eq!(plan.rendered, "<p>native-kept</p>");
    assert!(r.committed_island().writes()[0]
        .source_map
        .frames
        .iter()
        .any(|f| matches!(
            &f.transform,
            cem_ml::source_map::TransformKind::HandoffBoundary { .. }
        )));
    let t = r.application().tree();
    let target = t.node(r.application().output_target()).unwrap();
    let written = target
        .attributes
        .iter()
        .find(|id| t.node(**id).unwrap().name.as_ref().unwrap().local_name == "data-theme")
        .unwrap();
    assert_eq!(t.source_line_number(*written), None);
    assert_eq!(
        t.node_source_map(*written).unwrap(),
        r.committed_island().writes()[0].source_map
    );
}

#[test]
fn excessive_host_aliases_or_late_output_failure_publish_no_state() {
    let attributes = (0..64)
        .map(|i| format!("a{i}='old'"))
        .collect::<Vec<_>>()
        .join(" ");
    let p = prepare("{attribute @name=label | Default}{p | Hello}", &attributes);
    let tree = p.prepared().elements().tree.clone();
    let id = p.prepared().instance_node();
    let error = render_and_apply_inline_dce(p, &control()).unwrap_err();
    assert_eq!(error[0].code, "cem.ssr.host_commit_bounds");
    assert_eq!(tree.node(id).unwrap().attributes.len(), 64);
    let p = prepare("{attribute @name=label | Default}{script | Unsafe}", "");
    let tree = p.prepared().elements().tree.clone();
    let id = p.prepared().instance_node();
    let c = control();
    let error = render_and_apply_inline_dce(p, &c).unwrap_err();
    assert_eq!(error[0].code, "cem.ssr.output_profile_unsupported");
    assert!(tree.node(id).unwrap().attributes.is_empty());
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}

#[test]
fn shared_commit_refuses_foreign_host_owner_even_with_identical_attributes() {
    use cem_ql::{eval::RetainedCemNode, instance_island::host_commit::prepare_host_commit};
    let original = prepare("{p | Hello}", "label='same'");
    let foreign = prepare("{p | Hello}", "label='same'");
    let host = RetainedCemNode::new(
        foreign.prepared().elements().tree.clone(),
        foreign.prepared().instance_node(),
    )
    .unwrap();
    assert!(prepare_host_commit(
        original.template(),
        host,
        original.instance_id(),
        &[],
        &control()
    )
    .is_err());
}
