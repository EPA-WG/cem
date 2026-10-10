//! Explicit function registration participates in the existing package transaction.
use super::*;
use cem_ml::schema::{
    datatype_contracts::{DatatypeCompilation, DatatypeCompilationIssue, DatatypeIssueState},
    datatype_registry::{DatatypeKind, DatatypeRegistry},
    datatype_validation::{
        CandidateRequirement, DatatypeBehaviorContract, ResultRepresentation, ScalarRepresentation,
        ValidationSignature, ValueRepresentation,
    },
    declaration_references::SchemaDeclarationNode,
    function_references::{FunctionCatalog, ScalarCompilationBudget},
    machine::CemSchemaMachine,
    registry::CEM_SCHEMA_URI,
    value_contracts::{ContractName, ValueContractSource, ValueContracts},
    vocab::CompiledSchema,
};
use cem_ql::{
    datatype_compilation::{
        compile_datatypes_with_budget, DatatypeImplementation, DatatypeImplementations,
        ExecutableDatatype, TokenizerBinding,
    },
    datatype_results::DatatypeResultAdapter,
    datatype_validation::{DatatypeValidationRegistry, ValidationInput, ValidationRuntime},
    eval::{AtomValue, Item},
};

fn declaration(tree: &RetainedCemTree, local: &str) -> SchemaDeclarationNode {
    tree.ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == local => {
                SchemaDeclarationNode::new(tree.ast_owner().clone(), *node_id)
            }
            _ => None,
        })
        .unwrap()
}
fn retained(text: &str, uri: &str) -> (Arc<RetainedCemTree>, ValueContractSource) {
    let captured = Arc::new(
        CemSchemaMachine::new(
            CompiledSchema::cem_core(),
            CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
                SourceId(17),
                text.as_bytes().to_vec(),
            ))),
        )
        .build_with_lexical_scopes(),
    );
    assert!(captured.document().diagnostics.is_empty());
    let tree = RetainedCemTree::from_shared(
        captured.document().clone(),
        uri,
        text,
        Default::default(),
        None,
    )
    .unwrap();
    let source = ValueContractSource::new(
        declaration(&tree, "schema"),
        CEM_SCHEMA_URI,
        [("schema".into(), CEM_SCHEMA_URI.into())].into(),
    )
    .with_captured_names(captured)
    .unwrap();
    (tree, source)
}
#[derive(Clone, Copy)]
enum Mode {
    Old,
    New,
    Pending,
    Invalid,
    Denied,
}
fn compiler_with_functions(
    name: &str,
    caller: (Arc<RetainedCemTree>, ValueContractSource),
    library: (Arc<RetainedCemTree>, ValueContractSource),
    mode: Mode,
) -> CemQlSchemaPackageCompiler {
    compiler(Some(name)).with_datatypes(move |request, host, limits| {
        let owner = request.source.ast_owner().clone();
        let schema = declaration(&request.source, "schema");
        let mut types = DatatypeRegistry::default();
        types.insert(schema.clone(), declaration(&request.source, "type")).unwrap();
        let datatype = types.source(&schema, "sample").unwrap();
        host.register_datatype_source(datatype.clone()).unwrap();
        let target_name = match mode { Mode::Old => "old", Mode::Invalid => "bad", _ => "new" };
        let target = library.0.ast().nodes.iter().find_map(|n| match n {
            CemAstNode::Element { node_id, expanded_name, attributes, .. } if expanded_name.local_name == "function" && attributes.iter().any(|id| matches!(library.0.ast().get(*id), Some(CemAstNode::Attribute { expanded_name, value: Some(value), .. }) if expanded_name.local_name == "name" && value == target_name)) => Some(*node_id), _ => None,
        }).unwrap();
        let evaluation = (!matches!(mode, Mode::Pending)).then(|| StandaloneExpressionContext::default().with_binding("chosen", StandaloneExpressionBinding::any(ItemStream::once(RetainedCemNode::new(library.0.clone(), target).unwrap().query_item()))));
        let from = host.register_scope(caller.0.clone(), evaluation, ReferenceScopePolicy::schema_defaults().unwrap());
        let to = host.register_scope(library.0.clone(), None, ReferenceScopePolicy::schema_defaults().unwrap());
        if !matches!(mode, Mode::Denied) { host.allow_scope_crossing(from, to); }
        let mut budget = ScalarCompilationBudget::new(limits).unwrap();
        let catalog = FunctionCatalog::collect(&[caller.1.clone(), library.1.clone()], &mut budget).unwrap();
        let behavior = declaration(&caller.0, "behavior");
        let selection = catalog.select(&behavior, host, &mut budget).unwrap();
        let incomplete = |code, state, source| {
            let mut output = DatatypeCompilation::new(owner.clone());
            output.sources.push(datatype.clone());
            output.issues.push(DatatypeCompilationIssue { code, state, source, related: None });
            output
        };
        if selection.target().is_none() {
            let mut output = incomplete("function-selection-incomplete", DatatypeIssueState::Pending, selection.attribute.clone());
            output.diagnostics.extend(selection.resolution.diagnostics);
            return Ok(output);
        }
        let signature = ValidationSignature { kind: DatatypeKind::Scalar, value: ValueRepresentation::Scalar(ScalarRepresentation::String), candidate: CandidateRequirement::Optional, result: ResultRepresentation::Accepted(ContractName::new(CEM_SCHEMA_URI, "datatype-validation-result")) };
        let contract = match DatatypeBehaviorContract::compile_selected(&selection, signature, &mut budget) {
            Ok(contract) => contract,
            Err(error) => return Ok(incomplete(error.code, DatatypeIssueState::Invalid, error.source.unwrap())),
        };
        let builtin = retained(cem_ml::schema::package_sources::builtin_schema_package_source("schema").unwrap().schema_source, "builtin.cem").1;
        let adapter = DatatypeResultAdapter::new(Arc::new(ValueContracts::compile(&[builtin], Default::default()).unwrap()), ContractName::new(CEM_SCHEMA_URI, "datatype-validation-result"), ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic")).unwrap();
        let mut validators = DatatypeValidationRegistry::default();
        validators.register_query(contract, adapter, None).unwrap();
        let mut implementations = DatatypeImplementations::default();
        implementations.register(DatatypeImplementation { source: datatype.clone(), kind: DatatypeKind::Scalar, representation: ValueRepresentation::Scalar(ScalarRepresentation::String), accepted_bases: vec![], bounds: Default::default(), tokenizer: TokenizerBinding::Absent, validator: Some((caller.1.schema.clone(), behavior)) }).unwrap();
        Ok(compile_datatypes_with_budget(owner, &[datatype], host, &implementations, &validators, &mut budget, None, Default::default()))
    })
}
fn accepts(compilation: &DatatypeCompilation) -> Option<bool> {
    let descriptor = compilation.contracts[0]
        .as_any()
        .downcast_ref::<ExecutableDatatype>()
        .unwrap();
    let control = cem_ml::operation_control::OperationControl::default();
    descriptor
        .validate(
            &ValidationInput {
                value: vec![Item::Atomic(AtomValue::String("same".into()))],
                candidate: vec![],
                fallback: Default::default(),
            },
            &ValidationRuntime {
                control: &control,
                scope: cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
                query: Default::default(),
            },
            Default::default(),
        )
        .accepted
}
#[test]
fn function_replacement_preserves_active_bindings_converters_and_artifacts_until_retry() {
    let prelude = format!("@ns schema = \"{CEM_SCHEMA_URI}\"\n@default schema\n");
    let caller = retained(&format!("{prelude}{{schema | {{behaviors | {{behavior @name=check @implementation=function @execution=datatype-validation @function={{#chosen}} | {{inputs | {{input-binding @name=value @source=value @type=string @required=true}} {{input-binding @name=datatype @source=datatype @type=node @required=true}} {{input-binding @name=candidate @source=candidate @type=node @required=false @cardinality=zero-or-one}}}} {{result @type=datatype-validation-result}}}}}}}}"), "caller.cem");
    let functions = [("old", "datatype-validation-result", true), ("new", "datatype-validation-result", false), ("bad", "object", false)].iter().map(|(name, returns, accepted)| format!("{{function @name={name} @visibility=public @returns={returns} | {{param @name=value @type=string @required=true}} {{param @name=datatype @type=node @required=true}} {{param @name=candidate @type=node @required=false @cardinality=zero-or-one}} {{body | {{$ {{accepted: {accepted}, diagnostics: ()}} }}}}}}")).collect::<String>();
    let library = retained(
        &format!("{prelude}{{schema | {{behaviors | {{behavior @name=library | {functions}}}}}}}"),
        "functions.cem",
    );
    let authored = SOURCE.replace(
        "{elements |",
        "{types | {type @name=sample @kind=scalar}} {elements |",
    );
    let mut context = context(&authored);
    context.schema_package_compiler = Some(Arc::new(compiler_with_functions(
        "old",
        caller.clone(),
        library.clone(),
        Mode::Old,
    )));
    load(&mut context, &input());
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    let old = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap()
        .datatype_compilation
        .clone()
        .unwrap();
    assert_eq!(accepts(&old), Some(true));
    set_source(&mut context, &format!("{authored}\n"));
    let mut replacement = input();
    replacement.bytes = MANIFEST
        .replace("runtime-converter", "new-converter")
        .replace("old.cemt", "new.cemt")
        .into_bytes();
    let mut candidate = None;
    for mode in [Mode::Pending, Mode::Denied, Mode::Invalid] {
        context.schema_package_compiler = Some(Arc::new(compiler_with_functions(
            "new",
            caller.clone(),
            library.clone(),
            mode,
        )));
        let diagnostics =
            load_schema_package_manifest_into_context(&mut context, &replacement).unwrap();
        assert_active(&context, "old", "runtime-converter", "old.cemt");
        assert!(context
            .converter_registry
            .converter("new-converter")
            .is_none());
        assert_eq!(accepts(&old), Some(true));
        let retained = context
            .schema_package_sources
            .get(SOURCE_URI)
            .unwrap()
            .clone();
        if let Some(previous) = &candidate {
            assert!(Arc::ptr_eq(previous, &retained));
        }
        candidate = Some(retained);
        if matches!(mode, Mode::Invalid) {
            assert!(diagnostics
                .iter()
                .any(|d| d.message.contains("function-result-mismatch")
                    && d.uri.as_deref() == Some("functions.cem")
                    && d.source_map.is_some()));
        }
    }
    context.schema_package_compiler = Some(Arc::new(compiler_with_functions(
        "new",
        caller.clone(),
        library,
        Mode::New,
    )));
    load(&mut context, &replacement);
    assert_active(&context, "new", "new-converter", "new.cemt");
    assert!(Arc::ptr_eq(
        candidate.as_ref().unwrap(),
        context.schema_package_sources.get(SOURCE_URI).unwrap()
    ));
    let active = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap()
        .datatype_compilation
        .as_ref()
        .unwrap();
    assert_eq!(accepts(active), Some(false));
    assert_eq!(accepts(&old), Some(true));
    assert!(caller
        .0
        .ast()
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { .. })));
    assert!(caller.0.ast().nodes.iter().all(|n| !matches!(
        n,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}

fn automatic_source(slot: &str) -> String {
    format!("@ns schema = \"{CEM_SCHEMA_URI}\"\n@default schema\n{{schema @name=runtime @namespace=\"{SCHEMA_URI}\" @version=1.0.0 | {{elements | {{element @name=sample}}}} {{types | {{type @name=sample @kind=scalar}}}} {{behaviors | {{behavior @name=check @implementation=function @execution=datatype-validation @function={slot} | {{inputs | {{input-binding @name=value @source=value @type=string @required=true}} {{input-binding @name=datatype @source=datatype @type=node @required=true}} {{input-binding @name=candidate @source=candidate @type=node @required=false @cardinality=zero-or-one}}}} {{result @type=schema:datatype-validation-result}} {{function @name=check @returns=datatype-validation-result | {{param @name=value @type=string @required=true}} {{param @name=datatype @type=node @required=true}} {{param @name=candidate @type=node @required=false @cardinality=zero-or-one}} {{body | {{$ {{accepted: true, diagnostics: ()}} }}}}}}}}}}}}")
}
fn automatic(admit: bool, count: Option<usize>, work: usize) -> CemQlSchemaPackageCompiler {
    CemQlSchemaPackageCompiler::new(move |request| {
        let mut host = CemQlSchemaDeclarationHost::new();
        let target = declaration(&request.source, "function");
        let item = RetainedCemNode::new(request.source.clone(), target.node_id())
            .unwrap()
            .query_item();
        let context = count.map(|n| {
            StandaloneExpressionContext::default().with_binding(
                "chosen",
                StandaloneExpressionBinding::any(ItemStream::from_items(vec![item; n])),
            )
        });
        let policy = ReferenceScopePolicy::schema_defaults().unwrap();
        host.register_scope(request.source.clone(), context, policy.clone());
        host.attach_captured_names(&request.lexical_scopes).unwrap();
        Ok((
            host,
            cem_ml::schema::reference_traversal::ReferenceTraversalLimits {
                max_work: work,
                ..policy.limits
            },
        ))
    })
    .with_scalar_datatypes(move |request, host| {
        let schema = declaration(&request.source, "schema");
        let source = ValueContractSource::new(
            schema.clone(),
            SCHEMA_URI,
            [("schema".into(), CEM_SCHEMA_URI.into())].into(),
        )
        .with_captured_names(request.lexical_scopes.clone())
        .unwrap();
        automatic_plan(
            request,
            host,
            admit,
            vec![source],
            schema,
            declaration(&request.source, "behavior"),
        )
    })
}
#[test]
fn automatic_functions_activate_authored_native_and_existing_inline_profiles() {
    for slot in ["{#chosen}", "check"] {
        let mut context = context(&automatic_source(slot));
        context.schema_package_compiler = Some(Arc::new(automatic(true, Some(1), 100_000)));
        load(&mut context, &input());
        let active = context
            .schema_document_models
            .resolve_for_identity(Some(SCHEMA_URI), None, None)
            .expect("automatic checked function activates");
        assert!(active.is_ready_for_validation());
        assert_eq!(
            accepts(active.datatype_compilation.as_ref().unwrap()),
            Some(true)
        );
        assert!(active.function_bindings.is_some());
    }
}
#[test]
fn automatic_function_retry_preserves_the_complete_snapshot() {
    let text = automatic_source("{#chosen}");
    let mut context = context(&text);
    context.schema_package_compiler = Some(Arc::new(automatic(true, Some(1), 100_000)));
    load(&mut context, &input());
    let old = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap();
    let old = old.datatype_compilation.clone().unwrap();
    set_source(
        &mut context,
        &text.replace("accepted: true", "accepted: false"),
    );
    let mut replacement = input();
    replacement.bytes = MANIFEST
        .replace("runtime-converter", "new-converter")
        .replace("old.cemt", "new.cemt")
        .into_bytes();
    let mut candidate = None;
    for (admit, count, work) in [
        (false, Some(1), 100_000),
        (true, None, 100_000),
        (true, Some(0), 100_000),
        (true, Some(2), 100_000),
        (true, Some(1), 10),
    ] {
        context.schema_package_compiler = Some(Arc::new(automatic(admit, count, work)));
        let _ = load_schema_package_manifest_into_context(&mut context, &replacement).unwrap();
        assert_active(&context, "sample", "runtime-converter", "old.cemt");
        assert!(!context
            .schema_document_models
            .get(SCHEMA_URI)
            .unwrap()
            .is_ready_for_validation());
        assert!(context
            .converter_registry
            .converter("new-converter")
            .is_none());
        let retained = context
            .schema_package_sources
            .get(SOURCE_URI)
            .unwrap()
            .clone();
        if let Some(previous) = &candidate {
            assert!(Arc::ptr_eq(previous, &retained));
        }
        candidate = Some(retained);
    }
    context.schema_package_compiler = Some(Arc::new(automatic(true, Some(1), 100_000)));
    load(&mut context, &replacement);
    assert_active(&context, "sample", "new-converter", "new.cemt");
    assert!(Arc::ptr_eq(
        candidate.as_ref().unwrap(),
        context.schema_package_sources.get(SOURCE_URI).unwrap()
    ));
    let active = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap();
    assert_eq!(
        accepts(active.datatype_compilation.as_ref().unwrap()),
        Some(false)
    );
    assert_eq!(accepts(&old), Some(true));
}

fn automatic_plan(
    request: &cem_ml::schema::package_compilation::SchemaPackageCompilationRequest,
    host: &mut CemQlSchemaDeclarationHost,
    admit: bool,
    sources: Vec<ValueContractSource>,
    caller: SchemaDeclarationNode,
    behavior: SchemaDeclarationNode,
) -> Result<
    cem_ml_transform_cem_ql::schema_packages::ScalarPackagePlan,
    Vec<cem_ml::diagnostics::Diagnostic>,
> {
    use cem_ml_transform_cem_ql::schema_packages::ScalarPackagePlan;
    use cem_ql::function_activation::{FunctionAdmission, FunctionProfile};
    let schema = declaration(&request.source, "schema");
    let mut types = DatatypeRegistry::default();
    types
        .insert(schema.clone(), declaration(&request.source, "type"))
        .unwrap();
    let datatype = types.source(&schema, "sample").unwrap();
    host.register_datatype_source(datatype.clone()).unwrap();
    let mut plan = ScalarPackagePlan::default();
    plan.sources = sources;
    plan.roots.push(datatype.clone());
    plan.implementations
        .register(DatatypeImplementation {
            source: datatype,
            kind: DatatypeKind::Scalar,
            representation: ValueRepresentation::Scalar(ScalarRepresentation::String),
            accepted_bases: vec![],
            bounds: Default::default(),
            tokenizer: TokenizerBinding::Absent,
            validator: Some((caller, behavior.clone())),
        })
        .unwrap();
    if admit {
        let builtin = retained(
            cem_ml::schema::package_sources::builtin_schema_package_source("schema")
                .unwrap()
                .schema_source,
            "builtin.cem",
        )
        .1;
        let adapter = DatatypeResultAdapter::new(
            Arc::new(ValueContracts::compile(&[builtin], Default::default()).unwrap()),
            ContractName::new(CEM_SCHEMA_URI, "datatype-validation-result"),
            ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"),
        )
        .unwrap();
        plan.admissions.push(FunctionAdmission {
            behavior,
            profile: FunctionProfile::Validation {
                signature: ValidationSignature {
                    kind: DatatypeKind::Scalar,
                    value: ValueRepresentation::Scalar(ScalarRepresentation::String),
                    candidate: CandidateRequirement::Optional,
                    result: ResultRepresentation::Accepted(ContractName::new(
                        CEM_SCHEMA_URI,
                        "datatype-validation-result",
                    )),
                },
                adapter,
                legacy: None,
            },
        });
    }
    Ok(plan)
}

#[test]
fn automatic_reused_behavior_slots_keep_original_owners_and_require_current_grants() {
    let library = retained(&automatic_source("{#chosen}"), "reused.cem");
    let text = format!("@ns schema = \"{CEM_SCHEMA_URI}\"\n@default schema\n{{schema @name=runtime @namespace=\"{SCHEMA_URI}\" @version=1.0.0 | {{elements | {{element @name=sample}}}} {{types | {{type @name=sample @kind=scalar}}}} {{behaviors | {{#chosen}}}}}}");
    let make = |grant: bool| {
        let (tree, source) = library.clone();
        let owner = tree.clone();
        let behavior = declaration(&tree, "behavior");
        CemQlSchemaPackageCompiler::new(move |request| {
            let mut host = CemQlSchemaDeclarationHost::new();
            let policy = ReferenceScopePolicy::schema_defaults().unwrap();
            let caller = host.register_scope(
                request.source.clone(),
                Some(
                    StandaloneExpressionContext::default().with_binding(
                        "chosen",
                        StandaloneExpressionBinding::any(ItemStream::once(
                            RetainedCemNode::new(tree.clone(), behavior.node_id())
                                .unwrap()
                                .query_item(),
                        )),
                    ),
                ),
                policy.clone(),
            );
            let function = declaration(&tree, "function");
            let target = host.register_scope(
                tree.clone(),
                Some(
                    StandaloneExpressionContext::default().with_binding(
                        "chosen",
                        StandaloneExpressionBinding::any(ItemStream::once(
                            RetainedCemNode::new(tree.clone(), function.node_id())
                                .unwrap()
                                .query_item(),
                        )),
                    ),
                ),
                policy.clone(),
            );
            host.attach_captured_names(&request.lexical_scopes).unwrap();
            if grant {
                host.allow_scope_crossing(caller, target);
            }
            Ok((host, policy.limits))
        })
        .with_scalar_datatypes(move |request, host| {
            let own = ValueContractSource::new(
                declaration(&request.source, "schema"),
                SCHEMA_URI,
                Default::default(),
            )
            .with_captured_names(request.lexical_scopes.clone())
            .unwrap();
            automatic_plan(
                request,
                host,
                true,
                vec![own, source.clone()],
                source.schema.clone(),
                declaration(&owner, "behavior"),
            )
        })
    };
    let mut context = context(&text);
    context.schema_package_compiler = Some(Arc::new(make(false)));
    let _ = load_schema_package_manifest_into_context(&mut context, &input()).unwrap();
    assert!(context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .is_none());
    let candidate = context
        .schema_package_sources
        .get(SOURCE_URI)
        .unwrap()
        .clone();
    context.schema_package_compiler = Some(Arc::new(make(true)));
    load(&mut context, &input());
    let active = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .expect("admitted reused behavior activates");
    assert_eq!(
        accepts(active.datatype_compilation.as_ref().unwrap()),
        Some(true)
    );
    let bindings = active
        .function_bindings
        .as_ref()
        .unwrap()
        .as_any()
        .downcast_ref::<cem_ql::function_activation::FunctionBindings>()
        .unwrap();
    let (caller, selected) = bindings.selections().next().unwrap();
    assert!(Arc::ptr_eq(caller.document(), library.0.ast_owner()));
    assert!(Arc::ptr_eq(
        selected.target().unwrap().function().document(),
        library.0.ast_owner()
    ));
    assert!(Arc::ptr_eq(
        &candidate,
        context.schema_package_sources.get(SOURCE_URI).unwrap()
    ));
}

fn named(tree: &RetainedCemTree, local: &str, name: &str) -> SchemaDeclarationNode {
    tree.ast().nodes.iter().find_map(|node| match node {
        CemAstNode::Element { node_id, expanded_name, attributes, .. } if expanded_name.local_name == local && attributes.iter().any(|id| matches!(tree.ast().get(*id), Some(CemAstNode::Attribute { expanded_name, value: Some(value), .. }) if expanded_name.local_name == "name" && value == name)) => SchemaDeclarationNode::new(tree.ast_owner().clone(), *node_id), _ => None,
    }).unwrap()
}
fn profile_source() -> String {
    let profiles = [
        ("validation", "{accepted: true, diagnostics: ()}"),
        (
            "conversion",
            "{status: \"converted\", value: value, diagnostics: ()}",
        ),
        ("equality", "{equal: left == right, diagnostics: ()}"),
        (
            "constant",
            "{status: \"prepared\", value: value, diagnostics: ()}",
        ),
        ("diagnostic", "()"),
    ];
    let behaviors = profiles.iter().map(|(kind, body)| {
        let required_candidate = *kind == "constant";
        let roles = if *kind == "equality" { vec![("left", "string", true), ("right", "string", true), ("datatype", "node", true)] } else { vec![("value", "string", true), ("datatype", "node", true), ("candidate", "node", required_candidate)] };
        let inputs = roles.iter().map(|(name, ty, required)| format!("{{input-binding @name={name} @source={name} @type={ty} @required={required} @cardinality={}}}", if *required { "one" } else { "zero-or-one" })).collect::<String>();
        let params = roles.iter().map(|(name, ty, required)| format!("{{param @name={name} @type={ty} @required={required} @cardinality={}}}", if *required { "one" } else { "zero-or-one" })).collect::<String>();
        let (execution, result, returns) = if *kind == "diagnostic" { ("validation".into(), "schema:datatype-diagnostic @cardinality=zero-or-more".into(), "diagnostic-sequence".into()) } else { (kind.to_string(), format!("schema:datatype-{kind}-result"), format!("datatype-{kind}-result")) };
        format!("{{behavior @name={kind} @implementation=function @execution=datatype-{execution} @function={{#q_{kind}}} | {{inputs | {inputs}}} {{result @type={result}}} {{function @name={kind} @returns={returns} | {params} {{body | {{$ {body} }}}}}}}}")
    }).collect::<String>();
    format!("@ns schema = \"{CEM_SCHEMA_URI}\"\n@default schema\n{{schema @name=runtime @namespace=\"{SCHEMA_URI}\" @version=1.0.0 | {{elements | {{element @name=sample}}}} {{types | {{type @name=sample @kind=scalar @values=good}}}} {{behaviors | {behaviors}}}}}")
}
fn all_profiles(legacy: bool) -> CemQlSchemaPackageCompiler {
    CemQlSchemaPackageCompiler::new(|request| {
        let mut host = CemQlSchemaDeclarationHost::new();
        let policy = ReferenceScopePolicy::schema_defaults().unwrap();
        let mut context = StandaloneExpressionContext::default();
        for kind in [
            "validation",
            "conversion",
            "equality",
            "constant",
            "diagnostic",
        ] {
            context = context.with_binding(
                format!("q_{kind}"),
                StandaloneExpressionBinding::any(ItemStream::once(
                    RetainedCemNode::new(
                        request.source.clone(),
                        named(&request.source, "function", kind).node_id(),
                    )
                    .unwrap()
                    .query_item(),
                )),
            );
        }
        host.register_scope(request.source.clone(), Some(context), policy.clone());
        host.attach_captured_names(&request.lexical_scopes).unwrap();
        Ok((host, policy.limits))
    })
    .with_scalar_datatypes(move |request, host| {
        use cem_ql::{
            datatype_conversion::{
                ConversionRepresentation, ConversionSignature, DatatypeConversionResultAdapter,
            },
            datatype_enumeration::{DatatypeConstantResultAdapter, DatatypeEqualityResultAdapter},
            datatype_validation::LegacyAcceptance,
            function_activation::{FunctionAdmission, FunctionProfile},
        };
        let schema = declaration(&request.source, "schema");
        let source = ValueContractSource::new(
            schema.clone(),
            SCHEMA_URI,
            [("schema".into(), CEM_SCHEMA_URI.into())].into(),
        )
        .with_captured_names(request.lexical_scopes.clone())
        .unwrap();
        let mut plan = automatic_plan(
            request,
            host,
            false,
            vec![source],
            schema,
            named(&request.source, "behavior", "validation"),
        )?;
        let builtin = retained(
            cem_ml::schema::package_sources::builtin_schema_package_source("schema")
                .unwrap()
                .schema_source,
            "builtin.cem",
        )
        .1;
        let contracts = Arc::new(ValueContracts::compile(&[builtin], Default::default()).unwrap());
        let result =
            |kind: &str| ContractName::new(CEM_SCHEMA_URI, format!("datatype-{kind}-result"));
        let diagnostic = ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic");
        let adapter =
            DatatypeResultAdapter::new(contracts.clone(), result("validation"), diagnostic.clone())
                .unwrap();
        let value = ValueRepresentation::Scalar(ScalarRepresentation::String);
        let signature = ValidationSignature {
            kind: DatatypeKind::Scalar,
            value,
            candidate: CandidateRequirement::Optional,
            result: ResultRepresentation::Accepted(result("validation")),
        };
        let datatype = plan.roots[0].clone();
        let profiles = [
            (
                "validation",
                FunctionProfile::Validation {
                    signature: signature.clone(),
                    adapter: adapter.clone(),
                    legacy: None,
                },
            ),
            (
                "conversion",
                FunctionProfile::Conversion {
                    datatype: datatype.clone(),
                    signature: ConversionSignature {
                        kind: DatatypeKind::Scalar,
                        input: ConversionRepresentation::Values(value),
                        output: value,
                        candidate: CandidateRequirement::Optional,
                    },
                    adapter: DatatypeConversionResultAdapter::new(
                        contracts.clone(),
                        result("conversion"),
                        diagnostic.clone(),
                    )
                    .unwrap(),
                },
            ),
            (
                "equality",
                FunctionProfile::Equality {
                    datatype: datatype.clone(),
                    representation: ScalarRepresentation::String,
                    adapter: DatatypeEqualityResultAdapter::new(
                        contracts.clone(),
                        result("equality"),
                        diagnostic.clone(),
                    )
                    .unwrap(),
                },
            ),
            (
                "constant",
                FunctionProfile::Constant {
                    datatype,
                    representation: ScalarRepresentation::String,
                    adapter: DatatypeConstantResultAdapter::new(
                        contracts,
                        result("constant"),
                        diagnostic.clone(),
                    )
                    .unwrap(),
                },
            ),
            (
                "diagnostic",
                FunctionProfile::Validation {
                    signature: ValidationSignature {
                        result: ResultRepresentation::Diagnostics(diagnostic),
                        ..signature
                    },
                    adapter,
                    legacy: legacy.then_some(LegacyAcceptance::NoDiagnostics),
                },
            ),
        ];
        for (kind, profile) in profiles {
            plan.admissions.push(FunctionAdmission {
                behavior: named(&request.source, "behavior", kind),
                profile,
            });
        }
        Ok(plan)
    })
}
#[test]
fn automatic_profiles_prepare_enumerations_validate_convert_and_require_diagnostic_mapping() {
    let mut context = context(&profile_source());
    context.schema_package_compiler = Some(Arc::new(all_profiles(false)));
    let errors = load_schema_package_manifest_into_context(&mut context, &input()).unwrap();
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("result-registration-mismatch")),
        "{errors:?}"
    );
    assert!(context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .is_none());
    context.schema_package_compiler = Some(Arc::new(all_profiles(true)));
    load(&mut context, &input());
    let active = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap();
    let bindings = active
        .function_bindings
        .as_ref()
        .unwrap()
        .as_any()
        .downcast_ref::<cem_ql::function_activation::FunctionBindings>()
        .unwrap();
    assert_eq!(bindings.selections().count(), 5);
    let descriptor = active.datatype_compilation.as_ref().unwrap().contracts[0]
        .as_any()
        .downcast_ref::<ExecutableDatatype>()
        .unwrap();
    assert_eq!(descriptor.enumerations().len(), 1);
    let control = cem_ml::operation_control::OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    for (text, accepted) in [("good", true), ("bad", false)] {
        let value = vec![Item::Atomic(AtomValue::String(text.into()))];
        let result = descriptor.validate(
            &ValidationInput {
                value: value.clone(),
                candidate: vec![],
                fallback: Default::default(),
            },
            &runtime,
            Default::default(),
        );
        assert_eq!(result.accepted, Some(accepted), "{result:?}");
        let result = descriptor.convert(
            &cem_ql::datatype_conversion::ConversionInput {
                value: cem_ql::datatype_conversion::ConversionValue::Values(value),
                candidate: vec![],
                fallback: Default::default(),
            },
            &runtime,
            Default::default(),
        );
        assert_eq!(result.accepted, Some(accepted), "{result:?}");
    }
}

#[test]
fn automatic_invalid_function_replacement_retains_original_error_and_active_binding() {
    let text = automatic_source("{#chosen}");
    let mut context = context(&text);
    context.schema_package_compiler = Some(Arc::new(automatic(true, Some(1), 100_000)));
    load(&mut context, &input());
    let old = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap()
        .datatype_compilation
        .clone()
        .unwrap();
    set_source(
        &mut context,
        &text.replace("@returns=datatype-validation-result", "@returns=object"),
    );
    let errors = load_schema_package_manifest_into_context(&mut context, &input()).unwrap();
    let function = declaration(
        context.schema_package_sources.get(SOURCE_URI).unwrap(),
        "function",
    );
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("function-result-mismatch")
                && d.uri.as_deref() == Some(SOURCE_URI)
                && d.node.as_deref() == Some(function.identity().as_str())
                && d.source_map.is_some()),
        "{errors:?}"
    );
    assert_active(&context, "sample", "runtime-converter", "old.cemt");
    let active = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap();
    assert!(Arc::ptr_eq(
        &old,
        active.datatype_compilation.as_ref().unwrap()
    ));
    assert_eq!(accepts(&old), Some(true));
}

#[test]
fn foreign_admission_cannot_clear_an_equal_node_id_in_the_candidate() {
    let text = automatic_source("{#chosen}");
    let (foreign, source) = retained(&text, "same-text-different-owner.cem");
    let retained = foreign.clone();
    let compiler = CemQlSchemaPackageCompiler::new(move |request| {
        let mut host = CemQlSchemaDeclarationHost::new();
        let policy = ReferenceScopePolicy::schema_defaults().unwrap();
        for tree in [request.source.clone(), foreign.clone()] {
            let function = declaration(&tree, "function");
            host.register_scope(
                tree.clone(),
                Some(
                    StandaloneExpressionContext::default().with_binding(
                        "chosen",
                        StandaloneExpressionBinding::any(ItemStream::once(
                            RetainedCemNode::new(tree, function.node_id())
                                .unwrap()
                                .query_item(),
                        )),
                    ),
                ),
                policy.clone(),
            );
        }
        host.attach_captured_names(&request.lexical_scopes).unwrap();
        Ok((host, policy.limits))
    })
    .with_scalar_datatypes(move |request, host| {
        let own = declaration(&request.source, "behavior");
        let foreign = declaration(&retained, "behavior");
        assert_eq!(own.node_id(), foreign.node_id());
        assert_ne!(own.identity(), foreign.identity());
        let candidate = ValueContractSource::new(
            declaration(&request.source, "schema"),
            SCHEMA_URI,
            [("schema".into(), CEM_SCHEMA_URI.into())].into(),
        )
        .with_captured_names(request.lexical_scopes.clone())
        .unwrap();
        automatic_plan(
            request,
            host,
            true,
            vec![candidate, source.clone()],
            source.schema.clone(),
            foreign,
        )
    });
    let mut context = context(&text);
    context.schema_package_compiler = Some(Arc::new(compiler));
    let errors = load_schema_package_manifest_into_context(&mut context, &input()).unwrap();
    assert!(
        !errors.iter().any(|d| d.severity.is_hard_violation()),
        "{errors:?}"
    );
    let candidate = context.schema_document_models.get(SCHEMA_URI).unwrap();
    assert!(candidate.datatype_compilation.as_ref().unwrap().is_ready());
    assert!(!candidate.is_ready_for_validation());
    assert!(context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .is_none());
}
