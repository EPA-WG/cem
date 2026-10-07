use cem_ml::{
    ast::reload::ReloadLimits,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    schema::reference_policy::ReferenceScopePolicy,
};
use cem_ql::{
    api::{
        element_references::{
            ElementReferenceBinding, ElementReferenceExecution, ElementReferenceSource,
        },
        reference_transport::RetainedReferenceSource,
    },
    render::{
        compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
        TemplateData,
    },
};
fn source(text: &str, uri: &str) -> RetainedReferenceSource {
    RetainedReferenceSource::parse(text.as_bytes(), "text/cem-ml", uri, ReloadLimits::default())
        .unwrap()
}
fn execute_with(
    a: RetainedReferenceSource,
    b: RetainedReferenceSource,
    context: bool,
    grant: bool,
    instance: &str,
) -> Result<String, String> {
    let owner = a.ingress().source().clone();
    let mut data = TemplateData::default();
    let execution = ElementReferenceExecution::prepare(
        vec![
            ElementReferenceSource {
                source: a,
                context,
                policy: ReferenceScopePolicy::schema_defaults().unwrap(),
            },
            ElementReferenceSource {
                source: b,
                context: true,
                policy: ReferenceScopePolicy::schema_defaults().unwrap(),
            },
        ],
        0,
        &[
            ElementReferenceBinding {
                source: 0,
                name: "relation".into(),
                select: "input.children".into(),
            },
            ElementReferenceBinding {
                source: 1,
                name: "destination".into(),
                select: "input.children".into(),
            },
        ],
        if grant { &[(0, 1)] } else { &[] },
        &mut data,
    )?;
    let artifact = compile_template(
        "{slice @name=relation}{slice @name=destination}{button @commandfor={#relation}}{$destination}",
        &CompileTemplateOptions::default(),
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    let plan = render_compiled_template(&artifact, &data);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    let result = execution
        .project(
            &plan,
            instance,
            &Default::default(),
            &OperationControl::default(),
            ROOT_EXECUTION_SCOPE_ID,
        )
        .map_err(|e| e.to_string());
    assert!(owner.ast().nodes.iter().all(|n| !matches!(
        n,
        cem_ml::parser::CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
    result.map(|p| render_plan_to_html(&p))
}
fn execute(context: bool, grant: bool, instance: &str) -> Result<String, String> {
    execute_with(
        source("{#datadom.slices.destination}", "memory:relationship.cem"),
        source("{dialog}", "memory:target.cem"),
        context,
        grant,
        instance,
    )
}
#[test]
fn shared_authored_reference_uses_each_invocations_datadom() {
    let shared = source("{#datadom.slices.destination}", "memory:shared.cem");
    let a = execute_with(
        shared.clone(),
        source("{dialog | First}", "memory:first.cem"),
        true,
        true,
        "a",
    )
    .unwrap();
    let b = execute_with(
        shared.clone(),
        source("{dialog | Second}", "memory:second.cem"),
        true,
        true,
        "b",
    )
    .unwrap();
    assert!(a.contains("First") && !a.contains("Second"), "{a}");
    assert!(b.contains("Second") && !b.contains("First"), "{b}");
    assert!(execute_with(
        shared,
        source("{dialog}", "memory:pending.cem"),
        false,
        true,
        "pending"
    )
    .is_err());
}
#[test]
fn source_capture_context_and_grants_are_independent_per_execution() {
    assert!(execute(false, true, "a").is_err());
    assert!(execute(true, false, "a").is_err());
    let a = execute(true, true, "a").unwrap();
    let b = execute(true, true, "b").unwrap();
    assert!(a.contains("commandfor=\"a-ref-1\""), "{a}");
    assert!(b.contains("commandfor=\"b-ref-1\""), "{b}");
    assert_eq!(a, execute(true, true, "a").unwrap());
}
#[test]
fn malformed_lifecycle_metadata_fails_before_mutating_template_inputs() {
    let s = ElementReferenceSource {
        source: source("{dialog}", "memory:source.cem"),
        context: true,
        policy: ReferenceScopePolicy::schema_defaults().unwrap(),
    };
    for (request, bindings, grants) in [
        (1, vec![], vec![]),
        (0, vec![], vec![(0, 1)]),
        (
            0,
            vec![ElementReferenceBinding {
                source: 0,
                name: "datadom".into(),
                select: "input".into(),
            }],
            vec![],
        ),
        (
            0,
            vec![ElementReferenceBinding {
                source: 1,
                name: "destination".into(),
                select: "input".into(),
            }],
            vec![],
        ),
    ] {
        let mut data = TemplateData::default();
        assert!(ElementReferenceExecution::prepare(
            vec![s.clone()],
            request,
            &bindings,
            &grants,
            &mut data
        )
        .is_err());
        assert!(data.bindings.is_empty());
    }
}
