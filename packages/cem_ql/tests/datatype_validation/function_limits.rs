use super::*;
use cem_ml::schema::function_references::FunctionSelectionBudget;
use cem_ql::{
    datatype_compilation::compile_datatypes_with_budget,
    datatype_results::DatatypeResultError,
    datatype_validation::ValidationStopReason,
    eval::{BudgetAxis, EvalError},
};

#[test]
fn datatype_and_function_binding_share_one_compilation_allowance() {
    let original = source(
        &query_declaration("{accepted: true, diagnostics: ()}")
            .replace("@function=check-body", "@function={#chosen}"),
    );
    let (selection, mut budget) = selected_function(&original);
    let contract =
        DatatypeBehaviorContract::compile_selected(&selection, signature(false), &mut budget)
            .unwrap();
    let spent = budget.work_used();
    let (mut host, sources) = types_fixture_source(original);
    let mut implementations = DatatypeImplementations::default();
    let mut implementation = implementation(
        &sources[0],
        DatatypeKind::Scalar,
        ValueRepresentation::Scalar(ScalarRepresentation::String),
    );
    implementation.validator = Some((contract.owner().clone(), contract.behavior().clone()));
    implementations.register(implementation).unwrap();
    let mut registry = DatatypeValidationRegistry::default();
    registry.register_query(contract, adapter(), None).unwrap();
    let compile =
        |budget: &mut FunctionSelectionBudget,
         host: &mut cem_ql::schema_references::CemQlSchemaDeclarationHost| {
            compile_datatypes_with_budget(
                sources[0].scope().document().clone(),
                &sources,
                host,
                &implementations,
                &registry,
                budget,
                None,
                Default::default(),
            )
        };
    assert!(compile(&mut budget, &mut host).is_ready());
    let datatype_work = budget.work_used() - spent;
    assert!(datatype_work > 0);
    // Each phase fits separately; the combined attempt must exhaust this limit.
    let mut tight = FunctionSelectionBudget::new(ReferenceTraversalLimits {
        max_depth: 64,
        max_work: spent + datatype_work - 1,
    })
    .unwrap();
    tight.spend(spent, sources[0].declaration()).unwrap();
    let failed = compile(&mut tight, &mut host);
    assert!(!failed.is_ready());
    assert!(failed
        .issues
        .iter()
        .any(|i| i.code == "datatype-work-limit" || i.code == "datatype-dependencies-incomplete"));
    assert_eq!(tight.work_used(), spent + datatype_work - 1);
    assert!(
        DatatypeBehaviorContract::compile_selected(&selection, signature(false), &mut tight)
            .is_err()
    );
    let control = OperationControl::default();
    control.cancel_root(None, None).unwrap();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let cancelled = compile_datatypes_with_budget(
        sources[0].scope().document().clone(),
        &sources,
        &mut host,
        &implementations,
        &registry,
        &mut budget,
        Some(&runtime),
        Default::default(),
    );
    assert!(!cancelled.is_ready());
    assert!(cancelled
        .issues
        .iter()
        .any(|i| i.code == "datatype-compilation-control"
            && i.source.identity() == sources[0].declaration().identity()));
}

#[test]
fn selected_rules_share_query_work_and_cannot_catch_exhaustion() {
    let body = "try { (seq:map((1,2,3,4,5,6,7,8), fn(v) => ()), {accepted: true, diagnostics: ()}) } catch (code, message) { {accepted: true, diagnostics: ()} }";
    let original =
        source(&query_declaration(body).replace("@function=check-body", "@function={#chosen}"));
    let (selected, mut budget) = selected_function(&original);
    let contract =
        DatatypeBehaviorContract::compile_selected(&selected, signature(false), &mut budget)
            .unwrap();
    let mut registry = DatatypeValidationRegistry::default();
    registry.register_query(contract, adapter(), None).unwrap();
    let rule = registry
        .bind(
            &original.schema,
            &node(&original, "behavior"),
            native(&node(&original, "type")),
            DatatypeKind::Scalar,
        )
        .unwrap();
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: EvaluationContext {
            scope_policy: cem_ml::scheduler::ScopePolicy::host_root().with_queue_size(16),
            ..Default::default()
        },
    };
    assert_eq!(
        validate_rules(&[rule.clone()], &input(), &runtime, Default::default()).accepted,
        Some(true)
    );
    let failed = validate_rules(
        &vec![rule.clone(); 40],
        &input(),
        &runtime,
        Default::default(),
    );
    assert_eq!(failed.accepted, None);
    let ValidationStopReason::Result(DatatypeResultError::Execution(stream)) =
        failed.stopped.unwrap().reason
    else {
        panic!("expected query budget failure")
    };
    assert_eq!(
        stream.error,
        Some(EvalError::BudgetExceeded(BudgetAxis::FunctionCalls))
    );
    assert!(stream
        .diagnostics
        .iter()
        .any(|d| d.source_map.as_ref().is_some_and(|m| m.frames.len() > 1)));
    // A separate invocation gets a fresh budget, even with the same control.
    assert_eq!(
        validate_rules(&[rule], &input(), &runtime, Default::default()).accepted,
        Some(true)
    );
}

#[test]
fn datatype_diamonds_reuse_selected_function_owners_but_cycles_stay_incomplete() {
    for cycle in [false, true] {
        let base = if cycle {
            "{type @name=base @kind=scalar @base=left}"
        } else {
            "{type @name=base @kind=scalar}"
        };
        let text = query_declaration("{accepted: true, diagnostics: ()}")
            .replace("@function=check-body", "@function={#chosen}")
            .replace("{type @name=sample @kind=scalar}", &format!("{base} {{type @name=left @kind=scalar @base=base}} {{type @name=right @kind=scalar @base=base}}"));
        let original = source(&text);
        let (selection, mut budget) = selected_function(&original);
        let contract =
            DatatypeBehaviorContract::compile_selected(&selection, signature(false), &mut budget)
                .unwrap();
        let mut validators = DatatypeValidationRegistry::default();
        validators
            .register_query(contract, adapter(), None)
            .unwrap();
        let (mut host, sources) = types_fixture_source(original.clone());
        let mut implementations = DatatypeImplementations::default();
        for (index, source) in sources.iter().enumerate() {
            let mut entry = implementation(
                source,
                DatatypeKind::Scalar,
                ValueRepresentation::Scalar(ScalarRepresentation::String),
            );
            entry.accepted_bases.push(BaseCompatibility {
                kind: entry.kind,
                representation: entry.representation,
            });
            if index == 0 {
                entry.validator = Some((original.schema.clone(), node(&original, "behavior")));
            }
            implementations.register(entry).unwrap();
        }
        let result = compile_datatypes_with_budget(
            original.schema.document().clone(),
            &sources,
            &mut host,
            &implementations,
            &validators,
            &mut budget,
            None,
            Default::default(),
        );
        assert_eq!(result.is_ready(), !cycle);
        if !cycle {
            let left = compiled(&result, &sources[1]);
            let right = compiled(&result, &sources[2]);
            assert!(Arc::ptr_eq(left.base().unwrap(), right.base().unwrap()));
            assert_eq!(
                left.rules()[0].behavior().identity(),
                right.rules()[0].behavior().identity()
            );
        } else {
            assert!(result.reference_issues.iter().any(|i| i.kind
                == cem_ml::value::reference_resolution::ReferenceResolutionIssueKind::Cycle));
        }
    }
}

#[test]
fn selected_query_cancellation_survives_native_success_and_query_catch() {
    use cem_ql::native::{NativeQueryFunction, NativeQueryRequest};
    #[derive(Debug)]
    struct Cancel;
    impl NativeQueryFunction for Cancel {
        fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
            request.control.cancel_root(None, None).unwrap();
            query("{accepted: true, diagnostics: ()}")
        }
    }
    let original = source(&query_declaration("try { native:call(\"cancel\") } catch (code, message) { {accepted: true, diagnostics: ()} }").replace("@function=check-body", "@function={#chosen}"));
    let (selection, mut budget) = selected_function(&original);
    let contract =
        DatatypeBehaviorContract::compile_selected(&selection, signature(false), &mut budget)
            .unwrap();
    let mut validators = DatatypeValidationRegistry::default();
    validators
        .register_query(contract, adapter(), None)
        .unwrap();
    let rule = validators
        .bind(
            &original.schema,
            &node(&original, "behavior"),
            native(&node(&original, "type")),
            DatatypeKind::Scalar,
        )
        .unwrap();
    let control = OperationControl::default();
    let mut context = EvaluationContext::default();
    context
        .native_functions
        .register("cancel", 0, Cancel)
        .unwrap();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: context,
    };
    let result = validate_rules(&[rule], &input(), &runtime, Default::default());
    assert_eq!(result.accepted, None);
    assert!(matches!(
        result.stopped.unwrap().reason,
        ValidationStopReason::Control(_)
    ));
}

#[test]
fn list_items_share_the_selected_functions_query_allowance() {
    let body = "(seq:map((1,2,3,4,5,6,7,8), fn(v) => ()), {accepted: true, diagnostics: ()})";
    let text = query_declaration(body)
        .replace("@function=check-body", "@function={#chosen}")
        .replace(
            "{type @name=sample @kind=scalar}",
            "{type @name=item @kind=scalar} {type @name=list @kind=list @base=item}",
        );
    let original = source(&text);
    let (selected, mut budget) = selected_function(&original);
    let contract =
        DatatypeBehaviorContract::compile_selected(&selected, signature(false), &mut budget)
            .unwrap();
    let mut validators = DatatypeValidationRegistry::default();
    validators
        .register_query(contract, adapter(), None)
        .unwrap();
    let (mut host, sources) = types_fixture_source(original.clone());
    let mut implementations = DatatypeImplementations::default();
    let mut item = implementation(
        &sources[0],
        DatatypeKind::Scalar,
        ValueRepresentation::Scalar(ScalarRepresentation::String),
    );
    item.validator = Some((original.schema.clone(), node(&original, "behavior")));
    implementations.register(item).unwrap();
    let mut list = implementation(
        &sources[1],
        DatatypeKind::List,
        ValueRepresentation::List(ScalarRepresentation::String),
    );
    list.accepted_bases.push(BaseCompatibility {
        kind: DatatypeKind::Scalar,
        representation: ValueRepresentation::Scalar(ScalarRepresentation::String),
    });
    list.tokenizer = TokenizerBinding::Ready(
        cem_ml::schema::datatype_contracts::RegisteredTokenizer::whitespace(),
    );
    implementations.register(list).unwrap();
    let compilation = compile_datatypes_with_budget(
        original.schema.document().clone(),
        &sources,
        &mut host,
        &implementations,
        &validators,
        &mut budget,
        None,
        Default::default(),
    );
    assert!(compilation.is_ready());
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: EvaluationContext {
            scope_policy: cem_ml::scheduler::ScopePolicy::host_root().with_queue_size(16),
            ..Default::default()
        },
    };
    let mut request = input();
    let descriptor = compiled(&compilation, &sources[1]);
    assert_eq!(
        descriptor
            .validate(&request, &runtime, Default::default())
            .accepted,
        Some(true)
    );
    request.value = vec![request.value[0].clone(); 40];
    let failed = descriptor.validate(&request, &runtime, Default::default());
    assert_eq!(failed.accepted, None);
    let ValidationStopReason::Result(DatatypeResultError::Execution(stream)) =
        failed.stopped.unwrap().reason
    else {
        panic!("expected shared query budget failure")
    };
    assert_eq!(
        stream.error,
        Some(EvalError::BudgetExceeded(BudgetAxis::FunctionCalls))
    );
}

#[test]
fn native_validation_cannot_accept_after_exhausting_its_enclosing_query_budget() {
    #[derive(Debug)]
    struct Swallow;
    impl NativeDatatypeValidator for Swallow {
        fn validate(&self, call: ValidationCall<'_>) -> RuleExecution {
            let query = compile(
                "declare function down(n) { if n == 0 { () } else { down(n - 1) } } down(100)",
                &CompileContext::default(),
            )
            .unwrap();
            let failed = cem_ql::api::evaluate_with_control(
                &query,
                &call.runtime.query,
                call.runtime.control,
                call.runtime.scope,
            );
            assert!(failed.error.is_some());
            RuleExecution::Complete(super::super::query("{accepted: true, diagnostics: ()}"))
        }
    }
    let original = source(&declaration(false));
    let mut validators = DatatypeValidationRegistry::default();
    validators
        .register_native(
            "urn:test:validate",
            contract(&original, false),
            adapter(),
            None,
            Swallow,
        )
        .unwrap();
    let rule = validators
        .bind(
            &original.schema,
            &node(&original, "behavior"),
            native(&node(&original, "type")),
            DatatypeKind::Scalar,
        )
        .unwrap();
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: EvaluationContext {
            scope_policy: cem_ml::scheduler::ScopePolicy::host_root().with_cpu_workers(1),
            ..Default::default()
        },
    }
    .with_query_budget();
    assert_eq!(
        validate_rules(&[rule], &input(), &runtime, Default::default()).accepted,
        None
    );
    assert_eq!(
        validate_rules(&[], &input(), &runtime, Default::default()).accepted,
        None
    );
}
