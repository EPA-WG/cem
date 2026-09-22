use super::*;

fn renderer() -> PlanRenderer<'static> {
    PlanRenderer {
        evaluation_context: EvaluationContext::default(),
        diagnostics: Vec::new(),
        templates: BTreeMap::new(),
        match_rules: Vec::new(),
        call_depth: 0,
        max_call_depth: MAX_TEMPLATE_CALL_DEPTH,
        safe_points: None,
        control: None,
        control_failed: false,
        recovery_depth: 0,
        failure: None,
        calls: None,
        result_work: 0,
        result_bytes: 0,
        result_depth: 0,
        text_memory: Vec::new(),
        hook_scopes: vec![Vec::new()],
        active_hooks: Vec::new(),
        capture_depth: None,
        render_scope_depth: 0,
        value_types: BTreeMap::new(),
        attribute_contracts: BTreeMap::new(),
        expression_target: ExpressionTarget::Content,
    }
}

fn ineligible_hook(into: &str) -> ExpressionHook {
    ExpressionHook {
        id: "fixture".into(),
        into: into.into(),
        test: None,
        priority: 0,
        returns: None,
        body: Vec::new(),
        bindings: BTreeMap::new(),
    }
}

// Allocation identity checks the performance contract deterministically, without
// elapsed-time thresholds. None of these calls may save/replace the environment
// or clone the consumed input just to return it unchanged.
#[test]
fn ineligible_hooks_preserve_input_and_binding_allocations_and_stream_metadata() {
    let node = crate::eval::imported_cem_tree(
        cem_ml::import::import_data("<name>ivy</name>", "xml", "cem", "memory:hook-input").unwrap(),
    );
    for into in ["content", "attribute"] {
        for case in ["empty", "opposite", "active"] {
            let mut renderer = renderer();
            let context = &mut renderer.evaluation_context;
            context.current_item = Some(node.clone());
            context.policy_bindings.insert(
                "island".into(),
                ItemStream::once(Item::Record(BTreeMap::from([(
                    "label".into(),
                    vec![Item::Atomic(AtomValue::String("retained".into()))],
                )]))),
            );
            let binding_pointer = context.policy_bindings["island"].items.as_ptr();
            if case != "empty" {
                let target = if case == "active" {
                    into
                } else if into == "content" {
                    "attribute"
                } else {
                    "content"
                };
                renderer.hook_scopes[0].push(ineligible_hook(target));
                if case == "active" {
                    renderer.active_hooks.push("fixture".into());
                    renderer.call_depth = MAX_TEMPLATE_CALL_DEPTH;
                }
            }
            let mut input = ItemStream::from_items(vec![
                Item::Atomic(AtomValue::Integer(1)),
                node.clone(),
                crate::eval::values::reference(vec![node.clone()]),
            ]);
            input.chain = true;
            input.error = Some(EvalError::TypeError("retained failure"));
            input.diagnostics.push(Diagnostic {
                code: "fixture.warning".into(),
                severity: Severity::Warning,
                source_map: Some(SourceMapStack::default()),
                ..Default::default()
            });
            assert_eq!(
                input.next_item(),
                Some(Ok(Item::Atomic(AtomValue::Integer(1))))
            );
            let expected = input.clone();
            let input_pointer = input.items.as_ptr();
            let mut output = renderer.apply_expression_hook(
                input,
                into,
                (into == "attribute").then_some("label"),
                &SourceMapStack::default(),
            );
            assert_eq!(output, expected, "{into}/{case}");
            assert_eq!(output.diagnostics, expected.diagnostics);
            assert!(output.chain);
            assert_eq!(output.items[1].identity(), node.identity());
            assert_eq!(output.next_item(), Some(Ok(expected.items[1].clone())));
            assert_eq!(output.next_item(), Some(Ok(expected.items[2].clone())));
            assert_eq!(
                output.next_item(),
                Some(Err(EvalError::TypeError("retained failure")))
            );
            assert_eq!(output.next_item(), None);
            assert_eq!(renderer.evaluation_context.current_item, Some(node.clone()));
            assert!(renderer.diagnostics.is_empty());
            assert!(renderer.failure.is_none());
            assert!(!renderer.control_failed);
            assert_eq!(
                output.items.as_ptr(),
                input_pointer,
                "input was copied: {into}/{case}"
            );
            assert_eq!(
                renderer.evaluation_context.policy_bindings["island"]
                    .items
                    .as_ptr(),
                binding_pointer,
                "bindings were copied: {into}/{case}"
            );
        }
    }
}
