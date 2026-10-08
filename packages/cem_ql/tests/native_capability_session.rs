use cem_ml::{ast::reload::ReloadLimits, schema::reference_policy::ReferenceScopePolicy};
use cem_ql::{
    api::{
        element_references::{ElementReferenceBinding, ElementReferenceSource},
        native_capability_session::NativeCapabilitySession,
        reference_transport::RetainedReferenceSource,
    },
    eval::{retained_cem_node, AtomValue},
    render::{compile_template, render_plan_to_html, CompileTemplateOptions, TemplateData},
};
use std::sync::Arc;
fn source(text: &str) -> ElementReferenceSource {
    ElementReferenceSource {
        source: RetainedReferenceSource::parse(
            text.as_bytes(),
            "text/cem-ml",
            "memory:options.cem",
            ReloadLimits::default(),
        )
        .unwrap(),
        context: true,
        policy: ReferenceScopePolicy::schema_defaults().unwrap(),
    }
}
fn prepare(
    sources: Vec<ElementReferenceSource>,
    grants: &[(usize, usize)],
    data: TemplateData,
) -> Result<NativeCapabilitySession, String> {
    NativeCapabilitySession::prepare(
        sources,
        0,
        &[
            ElementReferenceBinding {
                source: 0,
                name: "relation".into(),
                select: "input.children".into(),
            },
            ElementReferenceBinding {
                source: 1,
                name: "options".into(),
                select:
                    "seq:where(input.children, fn(n) => n.kind == \"element\" && n.name != \"@ns\")"
                        .into(),
            },
        ],
        grants,
        data,
        "relation",
        true,
        Default::default(),
    )
}
#[test]
fn source_selection_preserves_identity_descendants_and_native_label_frame() {
    let relation = source("{#datadom.slices.options}");
    let options = source("@ns v = urn:vendor\n{v:option @value=same | Label {#datadom.slices.other}}{v:option @value=same | Second}");
    let owner = options.source.ingress().source().clone();
    let before = options
        .source
        .export_bundle(ReloadLimits::default())
        .unwrap();
    let session = prepare(
        vec![relation, options.clone()],
        &[(0, 1)],
        TemplateData::default(),
    )
    .unwrap();
    assert_eq!(session.len(), 2);
    let selected = session.evaluate("input", None).unwrap();
    assert!(Arc::ptr_eq(
        retained_cem_node(&selected.items[0])
            .unwrap()
            .owner()
            .ast_owner(),
        owner.ast_owner()
    ));
    let descendant = session.evaluate("input.children.kind", Some(0)).unwrap();
    assert!(descendant
        .items
        .iter()
        .any(|n| n.atom() == Some(AtomValue::String("reference".into()))));
    assert!(session.export("input", None).is_err());
    assert!(session.export("input.name", None).is_ok());
    let template = compile_template(
        "{span | {$input.name}|{$input.children.kind}|{$input.children.expression}}",
        &CompileTemplateOptions {
            host_bindings: vec!["input".into()],
            ..Default::default()
        },
    );
    assert!(
        template.diagnostics.is_empty(),
        "{:?}",
        template.diagnostics
    );
    let html = render_plan_to_html(&session.render(&template, Some(0)).unwrap());
    assert!(
        html.contains("reference") && html.contains("#datadom.slices.other"),
        "{html}"
    );
    assert_eq!(
        options
            .source
            .export_bundle(ReloadLimits::default())
            .unwrap(),
        before
    );
}
#[test]
fn contexts_are_independent_and_pending_or_denied_selection_never_becomes_empty() {
    let shared = source("{#datadom.slices.options}");
    let a = prepare(
        vec![shared.clone(), source("{option | A}")],
        &[(0, 1)],
        TemplateData::default(),
    )
    .unwrap();
    let b = prepare(
        vec![shared.clone(), source("{option | B}")],
        &[(0, 1)],
        TemplateData::default(),
    )
    .unwrap();
    assert_ne!(
        a.evaluate("dom:text(input)", None).unwrap().items,
        b.evaluate("dom:text(input)", None).unwrap().items
    );
    assert!(prepare(
        vec![shared.clone(), source("{option}")],
        &[],
        TemplateData::default()
    )
    .is_err());
    let mut pending = shared;
    pending.context = false;
    assert!(prepare(
        vec![pending, source("{option}")],
        &[(0, 1)],
        TemplateData::default()
    )
    .is_err());
}
#[test]
fn destination_bounds_and_indices_apply_without_resetting_request_budget() {
    let mut target = source("{option}{option}");
    target.policy.limits.max_work = 1;
    assert!(prepare(
        vec![source("{#datadom.slices.options}"), target],
        &[(0, 1)],
        TemplateData::default()
    )
    .is_err());
    let empty = NativeCapabilitySession::prepare(
        vec![source("{option}")],
        0,
        &[],
        &[],
        TemplateData::default(),
        "()",
        true,
        Default::default(),
    )
    .unwrap();
    assert_eq!(empty.len(), 0);
    assert!(empty.evaluate("input", Some(0)).is_err());
    let invalid = NativeCapabilitySession::prepare(
        vec![source("{option}")],
        0,
        &[],
        &[],
        TemplateData::default(),
        "'record substitute'",
        true,
        Default::default(),
    );
    assert!(invalid.is_err());
}
#[test]
fn namespace_names_complete_per_session_without_evaluating_descendant_references() {
    let relation = source("{#datadom.slices.options}");
    let options = source(
        "{host @xmlns:v={#datadom.slices.namespace} | {v:option | Label {#datadom.slices.other}}}",
    );
    let before = options
        .source
        .export_bundle(ReloadLimits::default())
        .unwrap();
    for uri in ["urn:first", "urn:second"] {
        let namespace = source(&format!("@ns public = {uri}\n{{public:item}}"));
        let bindings = [
            ElementReferenceBinding { source: 0, name: "relation".into(), select: "input.children".into() },
            ElementReferenceBinding { source: 1, name: "options".into(), select: "input.children".into() },
            ElementReferenceBinding { source: 2, name: "namespace".into(), select: "seq:first(seq:where(input.children, fn(n) => n.kind == \"element\" && n.name == \"@ns\"))".into() },
        ];
        let session = NativeCapabilitySession::prepare(
            vec![relation.clone(), options.clone(), namespace.clone()],
            0,
            &bindings,
            &[(0, 1), (1, 2)],
            TemplateData::default(),
            "relation",
            true,
            Default::default(),
        )
        .unwrap();
        assert_eq!(
            session
                .evaluate("input.children.namespace", None)
                .unwrap()
                .items[0]
                .atom(),
            Some(AtomValue::String(uri.into()))
        );
        assert!(session
            .evaluate("input.children.children.expression", None)
            .unwrap()
            .items
            .iter()
            .any(|i| i.atom() == Some(AtomValue::String("#datadom.slices.other".into()))));
        assert!(NativeCapabilitySession::prepare(
            vec![relation.clone(), options.clone(), namespace],
            0,
            &bindings,
            &[(0, 1)],
            TemplateData::default(),
            "relation",
            true,
            Default::default()
        )
        .is_err());
    }
    assert_eq!(
        options
            .source
            .export_bundle(ReloadLimits::default())
            .unwrap(),
        before
    );
}
#[test]
fn selection_container_does_not_add_reference_depth() {
    let mut relation = source("{#datadom.slices.options}");
    relation.policy.limits.max_depth = 1;
    assert!(prepare(
        vec![relation, source("{option}")],
        &[(0, 1)],
        TemplateData::default()
    )
    .is_ok());
}
#[test]
fn label_body_inserts_completed_native_nodes_without_a_portable_round_trip() {
    let session = prepare(
        vec![
            source("{#datadom.slices.options}"),
            source("@ns v = urn:labels\n{v:option @v:code=one | First}{v:option | Second}"),
        ],
        &[(0, 1)],
        TemplateData::default(),
    )
    .unwrap();
    let template = compile_template(
        "{span | {$input}}",
        &CompileTemplateOptions {
            host_bindings: vec!["input".into()],
            ..Default::default()
        },
    );
    let plan = session.render(&template, Some(0)).unwrap();
    let html = render_plan_to_html(&plan);
    assert!(
        html.contains("urn:labels") && html.contains("First"),
        "{html}"
    );
}

#[test]
fn datalist_source_projection_retains_granted_scopes_and_independent_contexts() {
    let relation = source("{#datadom.slices.options}");
    let options = source("{cem-option @value=1 @label=One | {#datadom.slices.other}}");
    let owner = options.source.ingress().source().clone();
    let before = options
        .source
        .export_bundle(ReloadLimits::default())
        .unwrap();
    let a = prepare(
        vec![relation.clone(), options.clone()],
        &[(0, 1)],
        TemplateData::default(),
    )
    .unwrap();
    let b = prepare(
        vec![relation.clone(), source("{cem-option @value=2 | Two}")],
        &[(0, 1)],
        TemplateData::default(),
    )
    .unwrap();
    let av = a.datalist().unwrap();
    let bv = b.datalist().unwrap();
    let rows = av.root().view().unwrap().field("children").unwrap();
    let retained = rows[0].view().unwrap().field("source").unwrap();
    assert!(Arc::ptr_eq(
        retained_cem_node(&retained[0]).unwrap().owner().ast_owner(),
        owner.ast_owner()
    ));
    assert_ne!(
        av.root().view().unwrap().identity(),
        bv.root().view().unwrap().identity()
    );
    assert_eq!(
        av.root().view().unwrap().identity(),
        a.datalist().unwrap().root().view().unwrap().identity()
    );
    assert!(a
        .evaluate("input.children.kind", Some(0))
        .unwrap()
        .items
        .iter()
        .any(|n| n.atom() == Some(AtomValue::String("reference".into()))));
    assert_eq!(
        options
            .source
            .export_bundle(ReloadLimits::default())
            .unwrap(),
        before
    );
    assert!(prepare(
        vec![relation.clone(), options.clone()],
        &[],
        TemplateData::default()
    )
    .is_err());
    let mut bounded = options;
    bounded.policy.limits.max_work = 1;
    assert!(prepare(vec![relation, bounded], &[(0, 1)], TemplateData::default()).is_err());
}
