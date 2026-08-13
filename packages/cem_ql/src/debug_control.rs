//! Bounded, side-effect-free CEM-QL conditions for common debug control.

use std::collections::BTreeMap;

use cem_ml::debug_control::{
    DebugConditionContext, DebugConditionEvaluator, DebugValueCapture, DebugVariableCapture,
};

use crate::api::{
    compile_expression, evaluate_expression, StandaloneExpressionBinding,
    StandaloneExpressionContext,
};
use crate::eval::{AtomValue, Item, ItemStream};
use crate::types::{AtomType, Type};

/// CEM-QL adapter used by debug sessions. It exposes only captured immutable
/// values and registers no host functions, resolver, or mutation capability.
#[derive(Debug, Clone, Copy, Default)]
pub struct CemQlDebugConditionEvaluator;

impl DebugConditionEvaluator for CemQlDebugConditionEvaluator {
    fn validate(&self, expression: &str) -> Result<(), String> {
        compile_expression(expression, &condition_context(None))
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn evaluate(&self, expression: &str, context: &DebugConditionContext) -> Result<bool, String> {
        let evaluated = evaluate_expression(expression, &condition_context(Some(context)))
            .map_err(|error| error.to_string())?;
        if let Some(error) = evaluated.result.error {
            return Err(format!("CEM-QL condition evaluation failed: {error:?}"));
        }
        match evaluated.result.items.as_slice() {
            [Item::Atomic(AtomValue::Boolean(value))] => Ok(*value),
            values => Err(format!(
                "CEM-QL condition returned {} items instead of one boolean",
                values.len()
            )),
        }
    }
}

fn condition_context(context: Option<&DebugConditionContext>) -> StandaloneExpressionContext {
    let (frame, scope, task, lexical) = match context {
        Some(context) => {
            let frame = context.frames.first();
            let frame_record = Item::Record(BTreeMap::from([
                (
                    "name".to_owned(),
                    vec![Item::Atomic(AtomValue::String(
                        frame.map(|frame| frame.name.clone()).unwrap_or_default(),
                    ))],
                ),
                (
                    "phase".to_owned(),
                    vec![Item::Atomic(AtomValue::String(
                        frame.map(|frame| frame.phase.clone()).unwrap_or_default(),
                    ))],
                ),
                (
                    "depth".to_owned(),
                    vec![Item::Atomic(AtomValue::Integer(
                        i64::try_from(context.frames.len()).unwrap_or(i64::MAX),
                    ))],
                ),
            ]));
            let scope_record = Item::Record(BTreeMap::from([(
                "id".to_owned(),
                vec![Item::Atomic(AtomValue::Integer(
                    i64::try_from(context.scope.get()).unwrap_or(i64::MAX),
                ))],
            )]));
            let task_record = Item::Record(BTreeMap::from([(
                "id".to_owned(),
                vec![Item::Atomic(AtomValue::Integer(
                    i64::try_from(context.task.get()).unwrap_or(i64::MAX),
                ))],
            )]));
            let lexical = frame
                .into_iter()
                .flat_map(|frame| &frame.variable_scopes)
                .flat_map(|scope| &scope.variables)
                .map(|variable| (variable.name.clone(), captured_variable(variable)))
                .collect();
            (frame_record, scope_record, task_record, lexical)
        }
        None => (
            Item::Record(BTreeMap::new()),
            Item::Record(BTreeMap::new()),
            Item::Record(BTreeMap::new()),
            BTreeMap::new(),
        ),
    };

    let mut expression_context = StandaloneExpressionContext::default()
        .with_binding(
            "frame",
            StandaloneExpressionBinding::any(ItemStream::once(frame)),
        )
        .with_binding(
            "scope",
            StandaloneExpressionBinding::any(ItemStream::once(scope)),
        )
        .with_binding(
            "task",
            StandaloneExpressionBinding::any(ItemStream::once(task)),
        );
    for (name, value) in lexical {
        expression_context = expression_context.with_binding(
            name,
            StandaloneExpressionBinding::any(ItemStream::once(value)),
        );
    }
    expression_context.expected_type = Some(Type::atom(AtomType::Boolean));
    expression_context
}

fn captured_variable(variable: &DebugVariableCapture) -> Item {
    captured_value(&variable.value, 0)
}

fn captured_value(value: &DebugValueCapture, depth: usize) -> Item {
    if depth >= 16 {
        return Item::Atomic(AtomValue::String(value.preview.clone()));
    }
    match value.type_name.as_str() {
        "boolean" => value
            .preview
            .parse::<bool>()
            .map(AtomValue::Boolean)
            .map(Item::Atomic)
            .unwrap_or_else(|_| Item::Atomic(AtomValue::String(value.preview.clone()))),
        "integer" => value
            .preview
            .parse::<i64>()
            .map(AtomValue::Integer)
            .map(Item::Atomic)
            .unwrap_or_else(|_| Item::Atomic(AtomValue::String(value.preview.clone()))),
        "decimal" => Item::Atomic(AtomValue::Decimal(value.preview.clone())),
        "double" => value
            .preview
            .parse::<f64>()
            .map(AtomValue::Double)
            .map(Item::Atomic)
            .unwrap_or_else(|_| Item::Atomic(AtomValue::String(value.preview.clone()))),
        "null" => Item::Atomic(AtomValue::Null),
        _ if !value.named.is_empty() => Item::Record(
            value
                .named
                .iter()
                .map(|child| {
                    (
                        child.name.clone(),
                        vec![captured_value(&child.value, depth + 1)],
                    )
                })
                .collect(),
        ),
        _ if !value.indexed.is_empty() => Item::Array(
            value
                .indexed
                .iter()
                .map(|child| captured_value(child, depth + 1))
                .collect(),
        ),
        _ => Item::Atomic(AtomValue::String(value.preview.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cem_ml::debug_control::{DebugConditionContext, LogicalFrameCapture};
    use cem_ml::operation_control::{ExecutionScopeId, TaskId};

    #[test]
    fn conditions_are_boolean_cem_ql_over_read_only_debug_records() {
        let evaluator = CemQlDebugConditionEvaluator;
        let context = DebugConditionContext {
            task: TaskId::from_raw(3),
            scope: ExecutionScopeId::from_raw(2),
            location: None,
            frame_names: vec!["render".to_owned()],
            frames: vec![LogicalFrameCapture {
                name: "render".to_owned(),
                phase: "evaluate".to_owned(),
                location: None,
                execution_scope: ExecutionScopeId::from_raw(2),
                variable_scopes: Vec::new(),
            }],
        };
        evaluator.validate("true").unwrap();
        assert!(evaluator.evaluate("true", &context).unwrap());
        assert!(evaluator
            .evaluate("frame.name == \"render\"", &context)
            .unwrap());
        assert!(evaluator.validate("1").is_err());
    }
}
