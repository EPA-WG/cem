use cem_ml::{
    import::import_data,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    schema::reference_policy::ReferenceScopePolicy,
    value::artifact::CemValueArtifactLimits,
};
use cem_ql::{
    eval::{imported_cem_tree, Item, ItemStream},
    render::{
        compile_template, project_element_reference_ids_with_host_and_placements,
        render_compiled_template, render_plan_to_html, CompileTemplateOptions,
        ElementPlacementAdmission, ElementPlacementGrant, ElementPlacementSnapshot,
        ElementReferenceProjection, ElementReferenceProjectionError, RenderPlan, TemplateData,
    },
    schema_references::CemQlSchemaDeclarationHost,
};
use std::collections::BTreeMap;

fn setup(
    body: &str,
    destination_work: usize,
) -> (
    RenderPlan,
    Item,
    CemQlSchemaDeclarationHost,
    cem_ql::schema_references::DeclarationScope,
    cem_ql::schema_references::DeclarationScope,
) {
    let source = import_data("<dialog/>", "xml", "cem", "memory:placement.xml").unwrap();
    let target = imported_cem_tree(source.clone())
        .view()
        .unwrap()
        .field("children")
        .unwrap()[0]
        .clone();
    let data = TemplateData::default().with_binding("target", ItemStream::once(target.clone()));
    let compiled = compile_template(
        body,
        &CompileTemplateOptions {
            host_bindings: vec!["target".into()],
            ..Default::default()
        },
    );
    assert!(
        compiled.diagnostics.is_empty(),
        "{:?}",
        compiled.diagnostics
    );
    let plan = render_compiled_template(&compiled, &data);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    let mut host = CemQlSchemaDeclarationHost::new();
    let request = host.register_scope(
        super::template(),
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    let mut policy = ReferenceScopePolicy::schema_defaults().unwrap();
    policy.limits.max_work = destination_work;
    let destination = host.register_scope(source, Some(Default::default()), policy);
    (plan, target, host, request, destination)
}
fn snapshot(target: Item) -> ElementPlacementSnapshot {
    ElementPlacementSnapshot {
        admissions: vec![ElementPlacementAdmission {
            token: "admission-1".into(),
            target,
            producer: "producer".into(),
            path: vec![0],
            revision: "1".into(),
            id: "producer-ref-0".into(),
        }],
        grants: vec![ElementPlacementGrant {
            requester: "consumer".into(),
            token: "admission-1".into(),
            properties: vec!["commandfor".into(), "aria-controls".into()],
        }],
        committed_revisions: BTreeMap::from([("producer".into(), "1".into())]),
        prepared_transaction: None,
    }
}
fn project(
    plan: &RenderPlan,
    host: &mut CemQlSchemaDeclarationHost,
    request: cem_ql::schema_references::DeclarationScope,
    snapshot: &ElementPlacementSnapshot,
    limits: CemValueArtifactLimits,
) -> Result<ElementReferenceProjection, ElementReferenceProjectionError> {
    project_element_reference_ids_with_host_and_placements(
        plan,
        "consumer",
        &limits,
        &OperationControl::default(),
        ROOT_EXECUTION_SCOPE_ID,
        &mut host.element_reference_host(request).unwrap(),
        snapshot,
    )
}
#[test]
fn foreign_placement_requires_source_crossing_and_specific_host_grant() {
    let (plan, target, mut host, request, destination) =
        setup("{button @commandfor={#target}}", 100);
    let mut placements = snapshot(target.clone());
    assert!(project(&plan, &mut host, request, &placements, Default::default()).is_err());
    host.allow_scope_crossing(request, destination);
    placements.grants.clear();
    assert_eq!(
        project(&plan, &mut host, request, &placements, Default::default())
            .unwrap_err()
            .code(),
        "cem.element_reference.placement_denied"
    );
    placements = snapshot(target);
    let output = project(&plan, &mut host, request, &placements, Default::default()).unwrap();
    assert_eq!(
        render_plan_to_html(&output.plan),
        "<button commandfor=\"producer-ref-0\" data-cem-placement-ref-commandfor></button>"
    );
    assert_eq!(output.placements.len(), 1);
    assert_eq!(output.placements[0].token, "admission-1");
    assert_eq!(output.placements[0].attribute, "commandfor");
    assert_eq!(output.placements[0].path, vec![0]);
    assert!(!render_plan_to_html(&plan).contains("producer-ref"));
}
#[test]
fn local_placement_wins_and_foreign_admission_cannot_repair_local_ambiguity() {
    for (body, expected) in [
        ("{button @commandfor={#target}}{$target}", None),
        (
            "{button @commandfor={#target}}{$target}{$target}",
            Some("cem.element_reference.target_ambiguous"),
        ),
    ] {
        let (plan, target, mut host, request, destination) = setup(body, 100);
        host.allow_scope_crossing(request, destination);
        let output = project(
            &plan,
            &mut host,
            request,
            &snapshot(target),
            Default::default(),
        );
        if let Some(code) = expected {
            assert_eq!(output.unwrap_err().code(), code);
        } else {
            let output = output.unwrap();
            assert!(output.placements.is_empty());
            assert!(render_plan_to_html(&output.plan).contains("commandfor=\"consumer-ref-1\""));
        }
    }
}
#[test]
fn admission_revision_token_property_and_reserved_id_checks_fail_atomically() {
    let (plan, target, mut host, request, destination) =
        setup("{button @commandfor={#target}}", 100);
    host.allow_scope_crossing(request, destination);
    for change in 0..7 {
        let mut placements = snapshot(target.clone());
        match change {
            0 => {
                placements.committed_revisions.clear();
            }
            1 => {
                placements
                    .committed_revisions
                    .insert("producer".into(), "2".into());
            }
            2 => {
                placements.grants[0].properties = vec!["anchor".into()];
            }
            3 => {
                placements.grants[0].requester = "other".into();
            }
            4 => {
                placements.admissions[0].id = "bad id".into();
            }
            5 => {
                placements.admissions.push(placements.admissions[0].clone());
            }
            _ => {
                placements.admissions[0].producer = "consumer".into();
            }
        }
        assert!(
            project(&plan, &mut host, request, &placements, Default::default()).is_err(),
            "case {change}"
        );
        assert!(!render_plan_to_html(&plan).contains("producer-ref"));
    }
}
#[test]
fn unique_grant_selects_one_placement_and_id_lists_preserve_repetitions() {
    let (plan, target, mut host, request, destination) =
        setup("{button @aria-controls={#(target, target)}}", 100);
    host.allow_scope_crossing(request, destination);
    let mut placements = snapshot(target);
    let mut second = placements.admissions[0].clone();
    second.token = "admission-2".into();
    second.path = vec![1];
    second.id = "producer-ref-1".into();
    placements.admissions.push(second);
    let output = project(&plan, &mut host, request, &placements, Default::default()).unwrap();
    assert!(render_plan_to_html(&output.plan)
        .contains("aria-controls=\"producer-ref-0 producer-ref-0\""));
    placements.grants.push(ElementPlacementGrant {
        token: "admission-2".into(),
        ..placements.grants[0].clone()
    });
    assert_eq!(
        project(&plan, &mut host, request, &placements, Default::default())
            .unwrap_err()
            .code(),
        "cem.element_reference.placement_ambiguous"
    );
}
#[test]
fn metadata_and_multiple_slot_destination_work_are_bounded() {
    for (work, limits) in [
        (1, CemValueArtifactLimits::default()),
        (
            100,
            CemValueArtifactLimits {
                max_values: 2,
                ..Default::default()
            },
        ),
    ] {
        let (plan, target, mut host, request, destination) = setup(
            "{button @commandfor={#target}}{button @commandfor={#target}}",
            work,
        );
        host.allow_scope_crossing(request, destination);
        assert!(project(&plan, &mut host, request, &snapshot(target), limits).is_err());
    }
}

#[test]
fn admitted_ids_cannot_collide_with_authored_or_generated_local_ids() {
    for body in [
        "{button @commandfor={#target}}{i @id=producer-ref-0}",
        "{button @commandfor={#target}}{i @data-cem-placement-ref-commandfor=bad}",
    ] {
        let (plan, target, mut host, request, destination) = setup(body, 100);
        host.allow_scope_crossing(request, destination);
        assert!(project(
            &plan,
            &mut host,
            request,
            &snapshot(target),
            Default::default()
        )
        .is_err());
    }
}

#[test]
fn cancelled_admission_preserves_source_owners_and_original_ids() {
    let (plan, target, mut host, request, destination) =
        setup("{button @commandfor={#target}}", 100);
    host.allow_scope_crossing(request, destination);
    let placements = snapshot(target.clone());
    let control = OperationControl::default();
    control.abort_signal().abort();
    assert!(project_element_reference_ids_with_host_and_placements(
        &plan,
        "consumer",
        &Default::default(),
        &control,
        ROOT_EXECUTION_SCOPE_ID,
        &mut host.element_reference_host(request).unwrap(),
        &placements
    )
    .is_err());
    drop(host);
    let original = cem_ql::eval::retained_cem_node(&target).unwrap();
    assert!(original.owner().ast().nodes.iter().all(|n| !matches!(n, cem_ml::parser::CemAstNode::Attribute { expanded_name, .. } if expanded_name.local_name == "id")));
    assert!(cem_ql::eval::retained_cem_node(&placements.admissions[0].target).is_some());
}

#[test]
fn prepared_placements_require_an_explicit_transaction_and_requester_membership() {
    use cem_ql::render::ElementPlacementTransaction;
    use std::collections::BTreeSet;
    let (plan, target, mut host, request, destination) =
        setup("{button @commandfor={#target}}", 100);
    host.allow_scope_crossing(request, destination);
    let mut placements = snapshot(target);
    placements.committed_revisions.clear();
    assert!(project(&plan, &mut host, request, &placements, Default::default()).is_err());
    placements.prepared_transaction = Some(ElementPlacementTransaction {
        token: "transaction".into(),
        participants: BTreeSet::from(["producer".into(), "consumer".into()]),
        producer_revisions: BTreeMap::from([("producer".into(), "1".into())]),
    });
    let prepared = project(&plan, &mut host, request, &placements, Default::default()).unwrap();
    assert_eq!(
        prepared.placements[0].transaction.as_deref(),
        Some("transaction")
    );
    placements
        .prepared_transaction
        .as_mut()
        .unwrap()
        .participants
        .remove("consumer");
    assert_eq!(
        project(&plan, &mut host, request, &placements, Default::default())
            .unwrap_err()
            .code(),
        "cem.element_reference.transaction_invalid"
    );
}
