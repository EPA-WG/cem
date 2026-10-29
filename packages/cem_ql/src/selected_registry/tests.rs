use super::*;
use crate::{
    api::{compile, CompileContext, EvaluationContext},
    eval::Evaluator,
};
use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    scheduler::AbortSignal,
};
use std::sync::Arc;
fn registry(entries: &[SelectedBuiltin]) -> Arc<SelectedFunctionRegistry> {
    Arc::new(SelectedFunctionRegistry::new(entries.iter().copied()).unwrap())
}
fn evaluate(source: &str, entries: &[SelectedBuiltin]) -> crate::eval::ItemStream {
    let query = compile(source, &CompileContext::default()).unwrap();
    Evaluator::evaluate_with_selected_registry(
        &query,
        &EvaluationContext::default(),
        registry(entries),
        &OperationControl::default(),
        ROOT_EXECUTION_SCOPE_ID,
    )
}
#[test]
fn reviewed_functions_match_the_full_evaluator_without_catalogue_assembly() {
    use SelectedBuiltin::*;
    let cases = [
        ("seq:first((1,2))", SequenceFirst),
        ("seq:last((1,2))", SequenceLast),
        ("seq:count((1,2))", SequenceCount),
        ("seq:take((1,2),1)", SequenceTake),
        ("seq:drop((1,2),1)", SequenceDrop),
        ("seq:map((1,2),fn(x)=>x+1)", SequenceMap),
        ("seq:flat_map((1,2),fn(x)=>(x,x))", SequenceFlatMap),
        ("seq:where((1,2),fn(x)=>x==2)", SequenceWhere),
        ("seq:any((1,2),fn(x)=>x==2)", SequenceAny),
        ("seq:all((1,2),fn(x)=>x>0)", SequenceAll),
        ("seq:peek((1,2),fn(x)=>x)", SequencePeek),
        ("str:length(\"café😀\")", StringLength),
        ("str:lower(\"CAFÉ\")", StringLower),
        ("str:upper(\"straße\")", StringUpper),
        ("str:contains(\"café\",\"fé\")", StringContains),
    ];
    for (source, entry) in cases {
        let query = compile(source, &CompileContext::default()).unwrap();
        let original = Evaluator::evaluate(&query, &EvaluationContext::default());
        let (selected, stages) = crate::compile_profile::measure(|| {
            Evaluator::evaluate_with_selected_registry(
                &query,
                &EvaluationContext::default(),
                registry(&[entry]),
                &OperationControl::default(),
                ROOT_EXECUTION_SCOPE_ID,
            )
        });
        assert_eq!(
            format!("{:?}", original),
            format!("{:?}", selected),
            "{source}"
        );
        assert!(!stages.contains_key("stdlib/assemble-registry"));
    }
}
#[test]
fn unselected_special_routes_callbacks_and_arguments_do_not_fall_back() {
    use SelectedBuiltin::*;
    for source in [
        "str:upper(1/0)",
        "native:call(\"not-admitted\")",
        "url:parse(\"https://example.test/\")",
        "seq:map((1,2),fn(x)=>str:upper(\"x\"))",
        "(1,2).seq:take(1)",
    ] {
        let output = evaluate(source, &[SequenceMap, StringLower]);
        assert!(output.error.is_some(), "{source}");
        assert!(
            output
                .diagnostics
                .iter()
                .any(|d| d.message.contains("selected registry")),
            "{source}: {:?}",
            output.diagnostics
        );
        assert!(
            !output
                .diagnostics
                .iter()
                .any(|d| d.message.contains("zero")),
            "denied arguments ran"
        );
    }
    let output = evaluate("(1,2).seq:take(1)", &[SequenceTake]);
    assert!(output.error.is_none());
    let output = evaluate("dom:chain(()).count()", &[]);
    assert!(output.error.is_some());
}
#[test]
fn cancellation_remains_terminal_and_selection_does_not_assemble_catalogues() {
    let query = compile("seq:count((1,2))", &CompileContext::default()).unwrap();
    let abort = AbortSignal::new();
    let control = OperationControl::new(abort.clone());
    abort.abort();
    let output = Evaluator::evaluate_with_selected_registry(
        &query,
        &EvaluationContext::default(),
        registry(&[SelectedBuiltin::SequenceCount]),
        &control,
        ROOT_EXECUTION_SCOPE_ID,
    );
    assert!(output.error.is_some());
    assert!(output.items.is_empty());
}
#[test]
fn registry_and_transitive_execution_contracts_are_exact_and_deterministic() {
    use NativeExecutionUnit::*;
    use SelectedBuiltin::*;
    assert!(SelectedFunctionRegistry::new([StringLower, StringLower]).is_err());
    assert!(SelectedBuiltin::from_key("cem:stdlib/strings", "lower", 2).is_err());
    assert!(SelectedBuiltin::from_key("cem:stdlib/native", "call", 1).is_err());
    let units = SequenceWhere.execution_units();
    for unit in [
        SelectedDispatch,
        OperationControl,
        RetainedValues,
        Provenance,
        LocalLambdas,
        BooleanCoercion,
    ] {
        assert!(units.contains(&unit));
        for path in unit.source_paths() {
            assert!(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join(path)
                    .is_file(),
                "{path}"
            );
        }
    }
    assert_eq!(units, SequenceWhere.execution_units());
    assert!(!units.contains(&UnicodeCase));
}

#[test]
fn selected_template_rendering_refuses_cache_receipts_and_old_receipt_matching() {
    use crate::render::{read_dependencies::*, *};
    use cem_ml::{
        content_cache::ContentHash,
        dependency_identity::{DependencyVersion, RENDER_DEPENDENCY_INPUTS},
    };
    let source = "{p | {$seq:count((1,2))}}";
    let artifact = compile_template(source, &Default::default());
    let freshness = RenderFreshness {
        environment: DependencyVersion::Content(ContentHash::from_blake3(b"env")),
        inputs: RENDER_DEPENDENCY_INPUTS
            .iter()
            .map(|axis| {
                (
                    *axis,
                    DependencyVersion::Content(ContentHash::from_blake3(b"input")),
                )
            })
            .collect(),
    };
    let ordinary = TemplateData::default();
    let normal = render_compiled_template_with_dependencies(&artifact, &ordinary, &freshness);
    let receipt = normal.dependencies.unwrap();
    assert!(receipt.matches(&artifact, &ordinary, &freshness));
    let selected = TemplateData {
        selected_registry: Some(registry(&[SelectedBuiltin::SequenceCount])),
        ..Default::default()
    };
    assert!(!receipt.matches(&artifact, &selected, &freshness));
    assert!(crate::render::read_dependencies::reuse_identity(
        &artifact,
        &selected,
        &freshness,
        cem_ml::scheduler::ScopePolicy::host_root().with_queue_size(128)
    )
    .is_none());
    let output = render_compiled_template_with_dependencies(&artifact, &selected, &freshness);
    assert!(matches!(
        output.dependencies,
        Err(RenderTrackingError::OpaqueRuntimeScope)
    ));
    assert_eq!(
        render_plan_to_html(&normal.output.plan),
        render_plan_to_html(&output.output.plan)
    );
}

#[test]
fn generated_recipe_is_pinned_deterministic_and_refuses_unreviewed_or_open_contracts() {
    use crate::{
        application_requirements::*, declaration_closure::*,
        template_artifact::TemplateArtifactSourceMapMode,
    };
    let build = |body: &str| {
        build_declaration_closure(&["cem-probe".into()],TemplateArtifactSourceMapMode::Prod,|tag|Ok(DeclarationBuildSource {
        source:format!("<cem-element tag=\"{tag}\"><template id=\"{tag}\" type=\"text/cem-ml\">{body}</template></cem-element>"),uri:"https://app.test/probe.xhtml".into()})).unwrap()
    };
    let closure = build("{p @title=\"{str:lower(datadom.attributes.label)}\" | Probe}");
    let selected = closure.requirements.selected_registry_plan().unwrap();
    let (bytes, pin) = selected.pinned_json().unwrap();
    assert_eq!(
        pin,
        cem_ml::content_cache::ContentHash::from_blake3(&bytes).header_value()
    );
    assert_eq!(
        selected.pinned_json().unwrap(),
        closure
            .requirements
            .selected_registry_plan()
            .unwrap()
            .pinned_json()
            .unwrap()
    );
    assert!(selected
        .rust_source()
        .contains("SelectedBuiltin::StringLower"));
    assert!(!selected.rust_source().contains("datadom"));
    assert_eq!(
        selected.registry().entries().collect::<Vec<_>>(),
        vec![SelectedBuiltin::StringLower]
    );
    for body in ["{slot}", "{p @title=\"{num:abs(1)}\"}"] {
        assert!(build(body).requirements.selected_registry_plan().is_err());
    }
    let mut app = closure.requirements;
    let mut function = app
        .declarations
        .get_mut("cem-probe")
        .unwrap()
        .functions
        .pop_first()
        .unwrap();
    function.implementation = "unknown".into();
    app.declarations
        .get_mut("cem-probe")
        .unwrap()
        .functions
        .insert(function);
    assert!(app.selected_registry_plan().is_err());
    let _ = ApplicationRequirements::default();
}

#[test]
fn preexisting_chain_receivers_cannot_bypass_the_selected_method_guard() {
    use crate::eval::{AtomValue, Item, ItemStream};
    let mut stream = ItemStream::once(Item::Atomic(AtomValue::Integer(1)));
    stream.chain = true;
    let mut context = EvaluationContext::default();
    context
        .policy_bindings
        .insert("input".into(), stream.clone());
    let mut compile_context = CompileContext::default();
    compile_context
        .policy_bindings
        .insert("input".into(), stream);
    let query = compile("input.count()", &compile_context).unwrap();
    let output = Evaluator::evaluate_with_selected_registry(
        &query,
        &context,
        registry(&[]),
        &OperationControl::default(),
        ROOT_EXECUTION_SCOPE_ID,
    );
    assert!(output.error.is_some());
    assert!(output
        .diagnostics
        .iter()
        .any(|d| d.message.contains("selected execution contract")));
}
