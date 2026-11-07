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
    let source=owner(&format!("<cem-element tag='x-inline'><template id='x-inline' type='text/cem-ml'>{{attribute @name=label | Committed}}{{p | Visible}}</template></cem-element><x-inline><template type='text/cem-ml'>{payload}</template></x-inline>"));
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
