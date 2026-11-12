use super::*;
use crate::api::{compile, CompileContext, EvaluationContext};
use crate::eval::Evaluator;
use crate::selected_registry::{SelectedBuiltin, SelectedFunctionRegistry};
use cem_ml::operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
use std::sync::Arc;

fn name(local: &str) -> QName {
    QName {
        prefix: None,
        local: local.into(),
        range: cem_ml::source::ByteRange { start: 0, len: 0 },
    }
}

#[test]
fn selected_link_mask_refuses_registered_but_unlinked_calls_and_pipelines() {
    use crate::eval::selected::{call_linked, pipeline_step_linked};
    const FIRST: u32 = 1 << SelectedBuiltin::SequenceFirst as u32;
    let query = compile("(1, 2, 3)", &CompileContext::default()).unwrap();
    let context = EvaluationContext::default();
    let control = OperationControl::default();
    let mut ctx = EvalCtx::new(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID);
    ctx.selected_registry = Some(Arc::new(
        SelectedFunctionRegistry::new([
            SelectedBuiltin::SequenceFirst,
            SelectedBuiltin::SequenceCount,
            SelectedBuiltin::SequenceLast,
        ])
        .unwrap(),
    ));
    let args = [query.tree.root];
    let result = call_linked::<FIRST>("cem:stdlib/sequence", "first", &args, &mut ctx);
    assert!(result.error.is_none());
    assert_eq!(result.items, [Item::Atomic(AtomValue::Integer(1))]);
    let refused = call_linked::<FIRST>("cem:stdlib/sequence", "count", &args, &mut ctx);
    assert!(refused.error.is_some());
    assert!(refused.items.is_empty());
    let input = ItemStream::once(Item::Atomic(AtomValue::Integer(3)));
    let step = IrStep::NamedStdlib {
        module: ModuleUri("cem:stdlib/sequence".into()),
        name: name("last"),
        args: vec![],
    };
    let refused = pipeline_step_linked::<FIRST>(input.clone(), &step, &mut ctx).unwrap();
    assert!(refused.error.is_some());
    assert!(refused.items.is_empty());
    let ordinary = pipeline_step_linked::<{ u32::MAX }>(input, &step, &mut ctx).unwrap();
    assert!(ordinary.error.is_none());
    assert_eq!(ordinary.items, [Item::Atomic(AtomValue::Integer(3))]);
}

#[test]
fn selected_unowned_steps_refuse_builtin_and_source_dispatch() {
    for local in ["first", "last", "take", "drop", "nth", "target", "where"] {
        let query = compile("1", &CompileContext::default()).unwrap();
        let context = EvaluationContext::default();
        let control = OperationControl::default();
        let mut ctx = EvalCtx::new(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID);
        let result = apply_unowned_named_step::<true>(
            ItemStream::once(Item::Atomic(AtomValue::Integer(1))),
            &name(local),
            &[],
            &mut ctx,
        );
        assert!(result.error.is_some(), "{local}");
        assert!(result.items.is_empty());
        assert!(result
            .diagnostics
            .iter()
            .any(|item| item.code == "cem.ql.unknown_function"));
    }
}

#[test]
fn selected_unowned_field_navigation_keeps_native_values() {
    let query = compile("1", &CompileContext::default()).unwrap();
    let context = EvaluationContext::default();
    let control = OperationControl::default();
    let mut ctx = EvalCtx::new(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID);
    let input = ItemStream::once(Item::Record(
        [("amount".into(), vec![Item::Atomic(AtomValue::Integer(42))])]
            .into_iter()
            .collect(),
    ));
    let result = apply_unowned_named_step::<true>(input, &name("amount"), &[], &mut ctx);
    assert!(result.error.is_none());
    assert_eq!(result.items, [Item::Atomic(AtomValue::Integer(42))]);
}

#[test]
fn ordinary_unowned_builtins_keep_the_full_profile_behavior() {
    let query = compile("1", &CompileContext::default()).unwrap();
    let context = EvaluationContext::default();
    let control = OperationControl::default();
    let mut ctx = EvalCtx::new(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID);
    let input = ItemStream {
        items: vec![
            Item::Atomic(AtomValue::Integer(1)),
            Item::Atomic(AtomValue::Integer(2)),
        ],
        ..Default::default()
    };
    let result = apply_unowned_named_step::<false>(input, &name("first"), &[], &mut ctx);
    assert!(result.error.is_none());
    assert_eq!(result.items, [Item::Atomic(AtomValue::Integer(1))]);
}

#[test]
fn selected_registered_sequence_steps_preserve_the_ordinary_results() {
    use crate::ir::{CompiledQuery, IrNode, IrTree};
    let registry = Arc::new(
        SelectedFunctionRegistry::new([
            SelectedBuiltin::SequenceFirst,
            SelectedBuiltin::SequenceLast,
            SelectedBuiltin::SequenceTake,
            SelectedBuiltin::SequenceDrop,
            SelectedBuiltin::SequenceWhere,
        ])
        .unwrap(),
    );
    for (local, args, expected) in [
        ("first", vec![], vec![1]),
        ("last", vec![], vec![3]),
        ("take", vec![IrId(4)], vec![1, 2]),
        ("drop", vec![IrId(4)], vec![3]),
        ("where", vec![IrId(6)], vec![3]),
    ] {
        for qualified in [false, true] {
            let step = if qualified {
                IrStep::NamedStdlib {
                    module: ModuleUri("cem:stdlib/sequence".into()),
                    name: name(local),
                    args: args.clone(),
                }
            } else {
                IrStep::Named {
                    binding: None,
                    name: name(local),
                    args: args.clone(),
                }
            };
            let query = CompiledQuery {
                tree: IrTree {
                    nodes: vec![
                        IrNode::LitInt(1),
                        IrNode::LitInt(2),
                        IrNode::LitInt(3),
                        IrNode::Sequence(vec![IrId(0), IrId(1), IrId(2)]),
                        IrNode::LitInt(2),
                        IrNode::LeadingDot,
                        IrNode::BinaryOp {
                            op: crate::parser::BinaryOp::Gt,
                            lhs: IrId(5),
                            rhs: IrId(4),
                        },
                        IrNode::Pipeline {
                            source: IrId(3),
                            steps: vec![step],
                        },
                    ],
                    root: IrId(7),
                    source_maps: vec![Default::default(); 8],
                    types: vec![crate::types::Type::Any; 8],
                    resolutions: vec![None; 8],
                },
                policy_bindings: Default::default(),
                source: String::new(),
            };
            let result = Evaluator::evaluate_with_selected_registry(
                &query,
                &EvaluationContext::default(),
                registry.clone(),
                &OperationControl::default(),
                ROOT_EXECUTION_SCOPE_ID,
            );
            assert!(result.error.is_none(), "{local}: {:?}", result.diagnostics);
            assert_eq!(
                result.items,
                expected
                    .iter()
                    .map(|value| Item::Atomic(AtomValue::Integer(*value)))
                    .collect::<Vec<_>>()
            );
            let refused = Evaluator::evaluate_with_selected_registry(
                &query,
                &EvaluationContext::default(),
                Arc::new(SelectedFunctionRegistry::new([]).unwrap()),
                &OperationControl::default(),
                ROOT_EXECUTION_SCOPE_ID,
            );
            assert!(refused.error.is_some(), "unregistered {local}");
            assert!(refused.items.is_empty());
        }
    }
}
