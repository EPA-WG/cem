use cem_ml::diagnostics::Severity;
use cem_ml::source_map::TransformKind;
use cem_ql::api::{compile, parse, CompileContext};
use cem_ql::ir::lower::IrLowerer;
use cem_ql::ir::{IrNode, IrStep};
use cem_ql::resolve::ModuleUri;
use cem_ql::types::{AtomType, Type};

fn lower(source: &str) -> cem_ql::ir::lower::LowerResult {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    IrLowerer::new().lower_module(&parsed.module)
}

#[test]
fn lowerer_builds_typed_control_flow_tree_with_query_source_maps() {
    let result = lower("if true then 1 else 2");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let tree = result.query.tree;

    assert!(matches!(tree.node(tree.root), Some(IrNode::If { .. })));
    assert_eq!(tree.ty(tree.root), Some(&Type::atom(AtomType::Integer)));
    assert_eq!(tree.nodes.len(), tree.types.len());
    assert_eq!(tree.nodes.len(), tree.source_maps.len());
    assert!(tree.source_maps.iter().all(|stack| matches!(
        stack.current().map(|frame| &frame.transform),
        Some(TransformKind::Query)
    )));
}

#[test]
fn public_compile_entrypoint_returns_lowered_ir() {
    let query = compile("if true then 1 else 2", &CompileContext::default()).unwrap();

    assert!(matches!(
        query.tree.node(query.tree.root),
        Some(IrNode::If { .. })
    ));
}

#[test]
fn lowerer_maps_import_alias_calls_to_stdlib_calls() {
    let result = lower(
        r#"import "cem:stdlib/strings" as str
           str:length("submit")"#,
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);

    assert!(matches!(
        result.query.tree.node(result.query.tree.root),
        Some(IrNode::StdlibCall {
            module,
            name,
            args,
        }) if *module == ModuleUri("cem:stdlib/strings".to_owned())
            && name.prefix.as_deref() == Some("str")
            && name.local == "length"
            && args.len() == 1
    ));
}

#[test]
fn lowerer_uses_function_refs_for_user_declared_calls() {
    let result = lower(
        r#"declare function local:echo(item as string) { item }
           local:echo("x")"#,
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let tree = result.query.tree;
    let Some(IrNode::Call { callee, args }) = tree.node(tree.root) else {
        panic!(
            "expected user function call, got {:?}",
            tree.node(tree.root)
        );
    };

    assert_eq!(args.len(), 1);
    assert!(matches!(tree.node(*callee), Some(IrNode::FunctionRef(_))));
    assert!(tree.resolutions[tree.root.0 as usize].is_some());
}

#[test]
fn lowerer_threads_path_steps_through_pipeline_nodes() {
    let result = lower("root/child[true]/..");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);

    let Some(IrNode::Pipeline { source, steps }) = result.query.tree.node(result.query.tree.root)
    else {
        panic!(
            "expected lowered path pipeline, got {:?}",
            result.query.tree.node(result.query.tree.root)
        );
    };
    assert!(matches!(
        result.query.tree.node(*source),
        Some(IrNode::AxisStep { .. })
    ));
    assert_eq!(steps.len(), 2);
}

#[test]
fn pipeline_lambda_records_captures_and_closure_detachment_diagnostic() {
    let parsed = parse(
        r#"declare variable items := ()
           declare variable host := "snapshot"
           items.{host}"#,
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let result = IrLowerer::new()
        .detach_captures(true)
        .lower_module(&parsed.module);

    assert!(result
        .diagnostics
        .iter()
        .any(|diag| { diag.code == "cem.ql.closure_detached" && diag.severity == Severity::Info }));
    let Some(IrNode::Pipeline { steps, .. }) = result.query.tree.node(result.query.tree.root)
    else {
        panic!(
            "expected pipeline root, got {:?}",
            result.query.tree.node(result.query.tree.root)
        );
    };
    let Some(IrStep::Lambda(lambda_id)) = steps.first() else {
        panic!("expected lambda step, got {steps:?}");
    };
    assert!(matches!(
        result.query.tree.node(*lambda_id),
        Some(IrNode::Lambda { captures, .. }) if !captures.is_empty()
    ));
    assert!(matches!(
        result.query.tree.source_maps[lambda_id.0 as usize]
            .current()
            .map(|frame| &frame.transform),
        Some(TransformKind::QueryStep)
    ));
}
