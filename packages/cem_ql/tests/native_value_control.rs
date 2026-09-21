use cem_ml::operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
use cem_ml::scheduler::ScopePolicy;
use cem_ml::value::artifact::CemValueArtifactLimits;
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{
    AtomValue, Item, ItemStream, QueryContextScope, QueryItemView, QueryItemViewKind,
    QueryNodeAccessError, QueryNodeTextIterator,
};
use cem_ql::render::{RenderPlan, RenderPlanAttribute, RenderPlanNode};

#[derive(Debug)]
struct ScopedText;
impl QueryItemView for ScopedText {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "test.scoped-text"
    }
    fn identity(&self) -> String {
        "scoped".into()
    }
    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Node
    }
    fn field(&self, name: &str) -> Option<Vec<Item>> {
        Some(vec![Item::Atomic(AtomValue::String(
            match name {
                "kind" => "text",
                "value" => "secret",
                _ => return None,
            }
            .into(),
        ))])
    }
    fn parent(&self, scope: QueryContextScope) -> Result<Option<Item>, QueryNodeAccessError> {
        if scope.0 == 7 {
            Ok(None)
        } else {
            Err(QueryNodeAccessError::ScopeViolation)
        }
    }
    fn text_fragments(
        &self,
        scope: QueryContextScope,
    ) -> Result<QueryNodeTextIterator<'_>, QueryNodeAccessError> {
        if scope.0 == 7 {
            Ok(Box::new(std::iter::once(Ok("secret"))))
        } else {
            Ok(Box::new(
                [Ok("prefix"), Err(QueryNodeAccessError::ScopeViolation)].into_iter(),
            ))
        }
    }
}

fn attribute(items: Vec<Item>) -> RenderPlanAttribute {
    RenderPlanAttribute {
        name: "label".into(),
        namespace: None,
        qualified_name: None,
        value: String::new(),
        value_stream: ItemStream::from_items(items),
        contract: None,
        source_map: Default::default(),
    }
}

#[test]
fn lowered_text_budget_rejects_the_entire_value() {
    let mut context = EvaluationContext::default();
    context.scope_policy = context.scope_policy.with_memory_bytes(4);
    let query = compile(r#"dom:text("long value")"#, &CompileContext::default()).unwrap();
    let result = evaluate(&query, &context);
    assert!(result.error.is_some(), "{result:?}");
    assert!(result.items.is_empty());
}

#[test]
fn deferred_text_preserves_atomic_segments_and_rejects_partial_access() {
    let limits = CemValueArtifactLimits::default();
    let control = OperationControl::default();
    let value = attribute(vec![
        Item::Atomic(AtomValue::Integer(42)),
        Item::Atomic(AtomValue::Boolean(true)),
    ]);
    let output = cem_ql::eval::output::output_attribute(value.clone());
    let mut context = EvaluationContext::default();
    context
        .policy_bindings
        .insert("value".into(), ItemStream::once(output));
    let query = compile(
        "dom:text(value)",
        &CompileContext {
            policy_bindings: context.policy_bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        evaluate(&query, &context).items[0].atom(),
        Some(AtomValue::String("42true".into()))
    );
    let denied = attribute(vec![Item::native(ScopedText)]);
    let result = cem_ql::render::project_attribute_value_with_control(
        &denied,
        QueryContextScope(0),
        &limits,
        &control,
        ROOT_EXECUTION_SCOPE_ID,
    );
    assert!(result.is_err());
}

#[test]
fn native_export_obeys_query_scope_cancellation_and_memory() {
    use cem_ql::eval::portable::encode_values_with_control;
    let values = ItemStream::once(Item::native(ScopedText));
    let limits = CemValueArtifactLimits::default();
    let control = OperationControl::default();
    assert!(encode_values_with_control(
        &values,
        &limits,
        QueryContextScope(7),
        &control,
        ROOT_EXECUTION_SCOPE_ID
    )
    .is_ok());
    assert!(encode_values_with_control(
        &values,
        &limits,
        QueryContextScope(0),
        &control,
        ROOT_EXECUTION_SCOPE_ID
    )
    .is_err());
    let control = OperationControl::default();
    control.abort_signal().abort();
    assert!(encode_values_with_control(
        &values,
        &limits,
        QueryContextScope(7),
        &control,
        ROOT_EXECUTION_SCOPE_ID
    )
    .is_err());
    let control = OperationControl::with_root_policy(
        Default::default(),
        ScopePolicy::host_root().with_memory_bytes(8),
    )
    .unwrap();
    assert!(encode_values_with_control(
        &values,
        &limits,
        QueryContextScope(7),
        &control,
        ROOT_EXECUTION_SCOPE_ID
    )
    .is_err());
}

#[test]
fn final_markup_projection_discards_prefix_on_deferred_failure() {
    let plan = RenderPlan {
        nodes: vec![RenderPlanNode::Element {
            tag: "p".into(),
            namespace: None,
            qualified_name: None,
            attributes: vec![attribute(vec![Item::native(ScopedText)])],
            children: Vec::new(),
            source_map: Default::default(),
        }],
        host_attribute_updates: Vec::new(),
        diagnostics: Vec::new(),
    };
    let result = cem_ql::render::render_plan_to_html_with_control(
        &plan,
        &OperationControl::default(),
        ROOT_EXECUTION_SCOPE_ID,
    );
    assert!(result.is_err());
}

#[test]
fn cancellation_during_text_projection_discards_the_prefix() {
    #[derive(Debug)]
    struct CancellingText(OperationControl);
    impl QueryItemView for CancellingText {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "test.cancel-text"
        }
        fn identity(&self) -> String {
            "cancel".into()
        }
        fn kind(&self) -> QueryItemViewKind {
            QueryItemViewKind::Node
        }
        fn text_fragments(
            &self,
            _: QueryContextScope,
        ) -> Result<QueryNodeTextIterator<'_>, QueryNodeAccessError> {
            Ok(Box::new(["prefix", "suffix"].into_iter().enumerate().map(
                |(i, fragment)| {
                    if i == 1 {
                        self.0.abort_signal().abort();
                    }
                    Ok(fragment)
                },
            )))
        }
    }
    let control = OperationControl::default();
    let value = attribute(vec![Item::native(CancellingText(control.clone()))]);
    assert!(cem_ql::render::project_attribute_value_with_control(
        &value,
        QueryContextScope(0),
        &CemValueArtifactLimits::default(),
        &control,
        ROOT_EXECUTION_SCOPE_ID
    )
    .is_err());
    assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}

#[test]
fn execution_child_can_only_lower_text_memory() {
    use cem_ml::operation_control::{ExecutionScopeKind, ExecutionScopeRegistration};
    let control = OperationControl::default();
    let child = control
        .register_scope(
            ROOT_EXECUTION_SCOPE_ID,
            ExecutionScopeRegistration::inherited(
                ExecutionScopeKind::Template,
                "small",
                ScopePolicy::host_root().with_memory_bytes(4),
            ),
        )
        .unwrap();
    let query = compile(r#"dom:text("12345")"#, &CompileContext::default()).unwrap();
    let result =
        cem_ql::api::evaluate_with_control(&query, &EvaluationContext::default(), &control, child);
    assert!(result.error.is_some());
    assert!(result.items.is_empty());
    assert!(control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_ok());
}

#[test]
fn retained_metadata_is_part_of_the_native_graph_memory_budget() {
    use cem_ml::value::artifact::{CemValueGraph, CemValueProvenance, CemValueRecord};
    let plain = CemValueGraph {
        roots: vec![0],
        records: vec![CemValueRecord::default()],
    };
    let mut rich = plain.clone();
    rich.records[0].provenance = Some(CemValueProvenance {
        source_uri: Some("x".repeat(131072)),
        ..Default::default()
    });
    assert!(rich.accounted_bytes() >= plain.accounted_bytes() + 131072);
    let mut contract = cem_ml::schema::document_model::AttributeValueContract::default();
    contract.model.pattern = Some("x".repeat(131072));
    rich.records[0].contract = Some(contract);
    assert!(rich.accounted_bytes() >= plain.accounted_bytes() + 262144);
}
