use super::*;

fn datatype_compiler(name: &str, tokenizer_ready: bool) -> CemQlSchemaPackageCompiler {
    datatype_compiler_with_serializer(name, tokenizer_ready, None)
}
fn datatype_compiler_with_serializer(
    name: &str,
    tokenizer_ready: bool,
    serializer_ready: Option<bool>,
) -> CemQlSchemaPackageCompiler {
    compiler(Some(name)).with_datatypes(move |request,host,limits| {
        use cem_ml::schema::{datatype_registry::{DatatypeRegistry,DatatypeKind},datatype_validation::{ScalarRepresentation,ValueRepresentation},declaration_references::SchemaDeclarationNode};
        use cem_ql::datatype_compilation::{DatatypeImplementation,DatatypeImplementations,TokenizerBinding,compile_datatypes};
        let ast=request.source.ast_owner();
        let schema=ast.nodes.iter().find_map(|n|match n {CemAstNode::Element{node_id,expanded_name,..} if expanded_name.local_name=="schema"=>SchemaDeclarationNode::new(ast.clone(),*node_id),_=>None}).unwrap();
        let mut sources=vec![];let mut registry=DatatypeRegistry::default();let mut implementations=DatatypeImplementations::default();
        for n in &ast.nodes {if let CemAstNode::Element{node_id,expanded_name,attributes,..}=n {if expanded_name.local_name=="type" {
            let declaration=SchemaDeclarationNode::new(ast.clone(),*node_id).unwrap();registry.insert(schema.clone(),declaration).unwrap();
            let name=attributes.iter().find_map(|id|match ast.get(*id){Some(CemAstNode::Attribute{expanded_name,value,..}) if expanded_name.local_name=="name"=>value.as_deref(),_=>None}).unwrap();
            let source=registry.source(&schema,name).unwrap();host.register_datatype_source(source.clone()).unwrap();host.bind_literal_datatype(&schema,name,source.declaration().clone()).unwrap();
            if source.attribute("list-base").is_some() { sources.push(source); continue; }
            let list=name=="names";
            let mut entry = if list {
                cem_ql::datatype_shipped::list_implementation(
                    source.clone(),
                    cem_ml::schema::document_model::shipped_datatypes::ShippedDatatype::NameList,
                )
                .unwrap()
            } else {
                DatatypeImplementation {
                    source: source.clone(),
                    kind: DatatypeKind::Scalar,
                    representation: ValueRepresentation::Scalar(ScalarRepresentation::String),
                    accepted_bases: vec![],
                    bounds: Default::default(),
                    tokenizer: TokenizerBinding::Absent,
                    validator: None,
                }
            };
            if list {
                if !tokenizer_ready {
                    entry.tokenizer = TokenizerBinding::Unavailable;
                }
                let converter = cem_ql::datatype_shipped::converter(
                    source.clone(),
                    cem_ml::schema::document_model::shipped_datatypes::ShippedDatatype::NameList,
                ).unwrap();
                implementations.select_converter(
                    source.clone(), cem_ql::datatype_conversion::ConverterBinding::Ready(converter),
                ).unwrap();
                if let Some(ready) = serializer_ready {
                    use cem_ql::datatype_serialization::ListSerializerBinding;
                    let binding = if ready {
                        ListSerializerBinding::Ready(cem_ql::datatype_shipped::list_serializer(
                            source.clone(),
                            cem_ml::schema::document_model::shipped_datatypes::ShippedDatatype::NameList,
                        ).unwrap())
                    } else {
                        ListSerializerBinding::Unavailable
                    };
                    implementations.select_list_serializer(source.clone(), binding).unwrap();
                }
            }
            implementations.register(entry).unwrap();
            sources.push(source);
        }}}
        Ok(compile_datatypes(ast.clone(),&sources,host,&implementations,&Default::default(),limits))
    })
}
#[path = "list_serialization.rs"]
mod list_serialization;
#[path = "whole_list.rs"]
mod whole_list;
#[test]
fn incomplete_datatype_capability_preserves_package_and_retries_original_candidate() {
    let authored=SOURCE.replace("{elements |", "{types | {type @name=item @kind=scalar} {type @name=names @kind=list @base=item}} {elements |");
    let mut context = context(&authored);
    context.schema_package_compiler = Some(Arc::new(datatype_compiler("old", true)));
    load(&mut context, &input());
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    let original = context
        .schema_package_sources
        .get(SOURCE_URI)
        .unwrap()
        .clone();
    let active = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap()
        .datatype_compilation
        .clone()
        .unwrap();
    assert!(active.is_ready());
    let list = active
        .contracts
        .iter()
        .find_map(|entry| {
            entry
                .as_any()
                .downcast_ref::<cem_ql::datatype_compilation::ExecutableDatatype>()
                .filter(|d| d.kind() == cem_ml::schema::datatype_registry::DatatypeKind::List)
        })
        .unwrap();
    let control = cem_ml::operation_control::OperationControl::default();
    let runtime = cem_ql::datatype_validation::ValidationRuntime {
        control: &control,
        scope: cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let result = list.convert(
        &cem_ql::datatype_conversion::ConversionInput {
            value: cem_ql::datatype_conversion::ConversionValue::Lexical(
                cem_ml::schema::datatype_contracts::LexicalInput::new(
                    Arc::from("a b a"),
                    Default::default(),
                ),
            ),
            candidate: vec![],
            fallback: Default::default(),
        },
        &runtime,
        Default::default(),
    );
    assert_eq!(result.accepted, Some(true), "{result:?}");
    assert_eq!(result.value.unwrap().len(), 3);
    set_source(
        &mut context,
        &authored.replace("@kind=list", "@kind=list @max-items=2"),
    );
    let mut replacement = input();
    replacement.bytes = MANIFEST
        .replace("runtime-converter", "new-converter")
        .replace("old.cemt", "new.cemt")
        .into_bytes();
    context.schema_package_compiler = Some(Arc::new(datatype_compiler("new", false)));
    load(&mut context, &replacement);
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    assert!(context
        .converter_registry
        .converter("new-converter")
        .is_none());
    let candidate = context
        .schema_package_sources
        .get(SOURCE_URI)
        .unwrap()
        .clone();
    assert!(!Arc::ptr_eq(&original, &candidate));
    let pending = context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .datatype_compilation
        .as_ref()
        .unwrap();
    assert!(!pending.is_ready());
    assert_eq!(pending.issues[0].code, "datatype-tokenizer-unavailable");
    assert!(pending.matches_owner(candidate.ast_owner()));
    assert!(active.matches_owner(original.ast_owner()));
    context.schema_package_compiler = Some(Arc::new(datatype_compiler("new", true)));
    load(&mut context, &replacement);
    assert_active(&context, "new", "new-converter", "new.cemt");
    assert!(context
        .converter_registry
        .converter("runtime-converter")
        .is_none());
    assert!(Arc::ptr_eq(
        &candidate,
        context.schema_package_sources.get(SOURCE_URI).unwrap()
    ));
    set_source(
        &mut context,
        &authored.replace("@kind=list", "@kind=list @max-items=-1"),
    );
    let invalid = load_schema_package_manifest_into_context(&mut context, &replacement).unwrap();
    assert!(
        invalid.iter().any(|d| d.severity.is_hard_violation()
            && d.message.contains("invalid-item-bound")
            && d.source_map.is_some()),
        "{invalid:?}"
    );
    assert_active(&context, "new", "new-converter", "new.cemt");
    // A successful snapshot from the previous owner cannot authorize another source.
    context.schema_package_compiler = Some(Arc::new(
        compiler(Some("untrusted")).with_datatypes(move |_, _, _| Ok((*active).clone())),
    ));
    let diagnostics =
        load_schema_package_manifest_into_context(&mut context, &replacement).unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation() && d.message.contains("another source owner")),
        "{diagnostics:?}"
    );
    assert_active(&context, "new", "new-converter", "new.cemt");
}

fn converter_compiler(name: &str, ready: bool) -> CemQlSchemaPackageCompiler {
    capability_compiler(name, ready, false)
}
fn preparation_compiler(name: &str, ready: bool) -> CemQlSchemaPackageCompiler {
    capability_compiler(name, ready, true)
}
fn capability_compiler(name: &str, ready: bool, preparation: bool) -> CemQlSchemaPackageCompiler {
    compiler(Some(name)).with_datatypes(move |request, host, limits| {
        use cem_ml::schema::{
            datatype_registry::{DatatypeKind, DatatypeRegistry},
            datatype_validation::{CandidateRequirement, ValueRepresentation},
            declaration_references::SchemaDeclarationNode,
        };
        use cem_ql::{
            datatype_compilation::{
                compile_datatypes, DatatypeImplementation, DatatypeImplementations,
                TokenizerBinding,
            },
            datatype_conversion::*,
        };
        #[derive(Debug)]
        struct Identity;
        impl NativeDatatypeConverter for Identity {
            fn convert(&self, call: ConversionCall<'_>) -> ConversionExecution {
                let ConversionValue::Values(value) = call.value else {
                    panic!()
                };
                ConversionExecution::Converted {
                    value: value.clone(),
                    diagnostics: vec![],
                }
            }
        }
        let ast = request.source.ast_owner();
        let find = |name: &str| {
            ast.nodes
                .iter()
                .find_map(|n| match n {
                    CemAstNode::Element {
                        node_id,
                        expanded_name,
                        ..
                    } if expanded_name.local_name == name => {
                        SchemaDeclarationNode::new(ast.clone(), *node_id)
                    }
                    _ => None,
                })
                .unwrap()
        };
        let schema = find("schema");
        let mut registry = DatatypeRegistry::default();
        registry.insert(schema.clone(), find("type")).unwrap();
        let source = registry.source(&schema, "sample").unwrap();
        host.register_datatype_source(source.clone()).unwrap();
        let mut implementations = DatatypeImplementations::default();
        implementations
            .register(DatatypeImplementation {
                source: source.clone(),
                kind: if preparation {
                    DatatypeKind::Scalar
                } else {
                    DatatypeKind::Node
                },
                representation: if preparation {
                    ValueRepresentation::Scalar(
                        cem_ml::schema::datatype_validation::ScalarRepresentation::String,
                    )
                } else {
                    ValueRepresentation::Nodes
                },
                accepted_bases: vec![],
                bounds: Default::default(),
                tokenizer: TokenizerBinding::Absent,
                validator: None,
            })
            .unwrap();
        if preparation {
            use cem_ql::datatype_preparation::PreparationBinding;
            let binding = if ready {
                PreparationBinding::Ready(
                    cem_ql::datatype_shipped::lexical_preparation(
                        source.clone(),
                        cem_ml::schema::document_model::shipped_datatypes::ShippedDatatype::String,
                    )
                    .unwrap(),
                )
            } else {
                PreparationBinding::Unavailable
            };
            implementations
                .select_preparation(source.clone(), binding)
                .unwrap();
        } else {
            let binding = if ready {
                ConverterBinding::Ready(
                    RegisteredDatatypeConverter::new(
                        source.clone(),
                        "retained-identity",
                        ConversionSignature {
                            kind: DatatypeKind::Node,
                            input: ConversionRepresentation::Values(ValueRepresentation::Nodes),
                            output: ValueRepresentation::Nodes,
                            candidate: CandidateRequirement::Optional,
                        },
                        Identity,
                    )
                    .unwrap(),
                )
            } else {
                ConverterBinding::Unavailable
            };
            implementations
                .select_converter(source.clone(), binding)
                .unwrap();
        }
        Ok(compile_datatypes(
            ast.clone(),
            &[source],
            host,
            &implementations,
            &Default::default(),
            limits,
        ))
    })
}
#[test]
fn selected_unavailable_converter_preserves_active_package_until_ready() {
    let authored = SOURCE.replace(
        "{elements |",
        "{types | {type @name=sample @kind=node}} {elements |",
    );
    let mut context = context(&authored);
    context.schema_package_compiler = Some(Arc::new(converter_compiler("old", true)));
    load(&mut context, &input());
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    let owner = context
        .schema_package_sources
        .get(SOURCE_URI)
        .unwrap()
        .clone();
    let mut replacement = input();
    replacement.bytes = MANIFEST
        .replace("runtime-converter", "new-converter")
        .replace("old.cemt", "new.cemt")
        .into_bytes();
    context.schema_package_compiler = Some(Arc::new(converter_compiler("new", false)));
    load(&mut context, &replacement);
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    assert!(context
        .converter_registry
        .converter("new-converter")
        .is_none());
    let pending = context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .datatype_compilation
        .as_ref()
        .unwrap();
    assert!(!pending.is_ready());
    assert_eq!(pending.issues[0].code, "datatype-converter-unavailable");
    context.schema_package_compiler = Some(Arc::new(converter_compiler("new", true)));
    load(&mut context, &replacement);
    assert_active(&context, "new", "new-converter", "new.cemt");
    assert!(Arc::ptr_eq(
        &owner,
        context.schema_package_sources.get(SOURCE_URI).unwrap()
    ));
}

#[test]
fn selected_unavailable_preparation_preserves_active_package_until_ready() {
    let authored = SOURCE.replace(
        "{elements |",
        "{types | {type @name=sample @kind=scalar}} {elements |",
    );
    let mut context = context(&authored);
    context.schema_package_compiler = Some(Arc::new(preparation_compiler("old", true)));
    load(&mut context, &input());
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    let owner = context
        .schema_package_sources
        .get(SOURCE_URI)
        .unwrap()
        .clone();
    let mut replacement = input();
    replacement.bytes = MANIFEST
        .replace("runtime-converter", "new-converter")
        .replace("old.cemt", "new.cemt")
        .into_bytes();
    context.schema_package_compiler = Some(Arc::new(preparation_compiler("new", false)));
    load(&mut context, &replacement);
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    assert!(context
        .converter_registry
        .converter("new-converter")
        .is_none());
    let pending = context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .datatype_compilation
        .as_ref()
        .unwrap();
    assert!(!pending.is_ready());
    assert_eq!(pending.issues[0].code, "datatype-preparation-unavailable");
    context.schema_package_compiler = Some(Arc::new(preparation_compiler("new", true)));
    load(&mut context, &replacement);
    assert_active(&context, "new", "new-converter", "new.cemt");
    assert!(Arc::ptr_eq(
        &owner,
        context.schema_package_sources.get(SOURCE_URI).unwrap()
    ));
}

fn enumeration_compiler(name: &str, ready: bool) -> CemQlSchemaPackageCompiler {
    compiler(Some(name)).with_datatypes(move |request, host, limits| {
        use cem_ml::{
            operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
            schema::{
                datatype_registry::{DatatypeKind, DatatypeRegistry},
                datatype_validation::{ScalarRepresentation, ValueRepresentation},
                declaration_references::SchemaDeclarationNode,
            },
        };
        use cem_ql::{
            datatype_compilation::{
                compile_datatypes_with_runtime, DatatypeImplementation, DatatypeImplementations,
                TokenizerBinding,
            },
            datatype_enumeration::*,
            datatype_validation::ValidationRuntime,
            eval::{AtomValue, Item},
        };
        #[derive(Debug)]
        struct Interpret;
        impl NativeConstantInterpreter for Interpret {
            fn interpret(&self, call: ConstantCall<'_>) -> ConstantExecution {
                ConstantExecution::Prepared {
                    value: vec![Item::Atomic(AtomValue::String(call.token.text().into()))],
                    diagnostics: vec![],
                }
            }
        }
        #[derive(Debug)]
        struct Equal;
        impl NativeScalarEquality for Equal {
            fn compare(&self, call: EqualityCall<'_>) -> EqualityExecution {
                EqualityExecution::Complete {
                    equal: call.left == call.right,
                    diagnostics: vec![],
                }
            }
        }
        let ast = request.source.ast_owner();
        let find = |name: &str| {
            ast.nodes
                .iter()
                .find_map(|n| match n {
                    CemAstNode::Element {
                        node_id,
                        expanded_name,
                        ..
                    } if expanded_name.local_name == name => {
                        SchemaDeclarationNode::new(ast.clone(), *node_id)
                    }
                    _ => None,
                })
                .unwrap()
        };
        let schema = find("schema");
        let mut registry = DatatypeRegistry::default();
        registry.insert(schema.clone(), find("type")).unwrap();
        let source = registry.source(&schema, "sample").unwrap();
        host.register_datatype_source(source.clone()).unwrap();
        let mut implementations = DatatypeImplementations::default();
        implementations
            .register(DatatypeImplementation {
                source: source.clone(),
                kind: DatatypeKind::Scalar,
                representation: ValueRepresentation::Scalar(ScalarRepresentation::String),
                accepted_bases: vec![],
                bounds: Default::default(),
                tokenizer: TokenizerBinding::Absent,
                validator: None,
            })
            .unwrap();
        implementations
            .select_equality(
                source.clone(),
                EqualityBinding::Ready(
                    RegisteredScalarEquality::new(
                        source.clone(),
                        "test-equality",
                        ScalarRepresentation::String,
                        Equal,
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
        implementations
            .select_constant_interpreter(
                source.clone(),
                if ready {
                    ConstantBinding::Ready(
                        RegisteredConstantInterpreter::new(
                            source.clone(),
                            "test-constant",
                            ScalarRepresentation::String,
                            Interpret,
                        )
                        .unwrap(),
                    )
                } else {
                    ConstantBinding::Unavailable
                },
            )
            .unwrap();
        let control = OperationControl::default();
        let runtime = ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        };
        Ok(compile_datatypes_with_runtime(
            ast.clone(),
            &[source],
            host,
            &implementations,
            &Default::default(),
            limits,
            &runtime,
            Default::default(),
        ))
    })
}
#[test]
fn unprepared_vocabulary_preserves_active_package_and_retries_original_owner() {
    let authored = SOURCE.replace(
        "{elements |",
        "{types | {type @name=sample @kind=scalar @values=\"3 5\"}} {elements |",
    );
    let mut context = context(&authored);
    context.schema_package_compiler = Some(Arc::new(enumeration_compiler("old", true)));
    load(&mut context, &input());
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    let owner = context
        .schema_package_sources
        .get(SOURCE_URI)
        .unwrap()
        .clone();
    let mut replacement = input();
    replacement.bytes = MANIFEST
        .replace("runtime-converter", "new-converter")
        .replace("old.cemt", "new.cemt")
        .into_bytes();
    context.schema_package_compiler = Some(Arc::new(enumeration_compiler("new", false)));
    load(&mut context, &replacement);
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    let pending = context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .datatype_compilation
        .as_ref()
        .unwrap();
    assert!(!pending.is_ready());
    assert_eq!(pending.issues[0].code, "datatype-enumeration-unavailable");
    assert!(pending.matches_owner(owner.ast_owner()));
    context.schema_package_compiler = Some(Arc::new(enumeration_compiler("new", true)));
    load(&mut context, &replacement);
    assert_active(&context, "new", "new-converter", "new.cemt");
    assert!(Arc::ptr_eq(
        &owner,
        context.schema_package_sources.get(SOURCE_URI).unwrap()
    ));
    set_source(
        &mut context,
        &authored.replace("@values=\"3 5\"", "@values=\" \""),
    );
    let diagnostics =
        load_schema_package_manifest_into_context(&mut context, &replacement).unwrap();
    assert!(
        diagnostics.iter().any(|d| d.severity.is_hard_violation()
            && d.message.contains("datatype-empty-vocabulary")
            && d.source_map.is_some()),
        "{diagnostics:?}"
    );
    assert_active(&context, "new", "new-converter", "new.cemt");
}

#[test]
fn retained_constant_vocabulary_preserves_active_package_until_ready_replacement() {
    let authored = SOURCE.replace("{elements |", r#"{types | {type @name=sample @kind=scalar | {constant @value="In progress"} {constant @value=""}}} {elements |"#);
    let mut context = context(&authored);
    context.schema_package_compiler = Some(Arc::new(enumeration_compiler("old", true)));
    load(&mut context, &input());
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    let owner = context.schema_package_sources.get(SOURCE_URI).unwrap().clone();
    let mut replacement = input();
    replacement.bytes = MANIFEST.replace("runtime-converter", "new-converter").replace("old.cemt", "new.cemt").into_bytes();
    context.schema_package_compiler = Some(Arc::new(enumeration_compiler("new", false)));
    load(&mut context, &replacement);
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    let pending = context.schema_document_models.get(SCHEMA_URI).unwrap().datatype_compilation.as_ref().unwrap();
    assert!(!pending.is_ready());
    assert!(pending.matches_owner(owner.ast_owner()));
    context.schema_package_compiler = Some(Arc::new(enumeration_compiler("new", true)));
    load(&mut context, &replacement);
    assert_active(&context, "new", "new-converter", "new.cemt");
    let ready = context.schema_document_models.get(SCHEMA_URI).unwrap().datatype_compilation.as_ref().unwrap();
    let descriptor = ready.contracts[0].as_any().downcast_ref::<cem_ql::datatype_compilation::ExecutableDatatype>().unwrap();
    let constants = descriptor.enumerations()[0].constants();
    assert_eq!(constants.iter().map(|c| c.token.text()).collect::<Vec<_>>(), ["In progress", ""]);
    assert!(constants.iter().all(|c| Arc::ptr_eq(c.token.source.document(), owner.ast_owner())));
    set_source(&mut context, &authored.replace("@kind=scalar |", "@kind=scalar @values=old |"));
    let diagnostics = load_schema_package_manifest_into_context(&mut context, &replacement).unwrap();
    assert!(diagnostics.iter().any(|d| d.severity.is_hard_violation() && d.message.contains("datatype-mixed-vocabulary")), "{diagnostics:?}");
    assert_active(&context, "new", "new-converter", "new.cemt");
}

fn discovered_compiler(name: &str) -> CemQlSchemaPackageCompiler {
    discovered_compiler_mode(name, false)
}
fn discovered_compiler_mode(name: &str, omit: bool) -> CemQlSchemaPackageCompiler {
    discovered_native_compiler(name, omit, false)
}
fn discovered_native_compiler(name: &str, omit: bool, ready: bool) -> CemQlSchemaPackageCompiler {
    use cem_ql::datatype_names::DatatypeSchemaSource;
    compiler(Some(name)).with_datatype_discovery(
        Default::default(),
        move |request, host| {
            let owner = request.source.ast_owner();
            let schema = owner
                .nodes
                .iter()
                .find_map(|n| match n {
                    CemAstNode::Element {
                        node_id,
                        expanded_name,
                        ..
                    } if expanded_name.local_name == "schema" => {
                        cem_ml::schema::declaration_references::SchemaDeclarationNode::new(
                            owner.clone(),
                            *node_id,
                        )
                    }
                    _ => None,
                })
                .unwrap();
            use cem_ml::{schema::declaration_references::SchemaDeclarationHost, value::reference_resolution::ReferenceResolutionHost};
            let parent = host.scope(&host.source_reference(schema.clone())).unwrap();
            for node in &owner.nodes {
                if let CemAstNode::Reference { node_id, expression, .. } = node {
                    if expression != "#types" { continue; }
                    let evaluation = ready.then(|| {
                        let target = owner.nodes.iter().find_map(|node| match node {
                            CemAstNode::Element {node_id, expanded_name, attributes, ..}
                                if expanded_name.local_name == "type" && attributes.iter().any(|id| matches!(owner.get(*id), Some(CemAstNode::Attribute {value: Some(v), ..}) if v == "base")) => Some(*node_id),
                            _ => None,
                        }).unwrap();
                        StandaloneExpressionContext::default().with_binding("types", StandaloneExpressionBinding::any(ItemStream::once(RetainedCemNode::new(request.source.clone(), target).unwrap().query_item())))
                    });
                    let lexical = host.register_lexical_scope(parent, evaluation, ReferenceScopePolicy::schema_defaults().unwrap()).unwrap();
                    assert!(host.assign_subtree_scope(&request.source, *node_id, lexical));
                }
            }
            Ok(vec![DatatypeSchemaSource {
                schema,
                captured: request.lexical_scopes.clone(),
                imports: vec![],
            }])
        },
        move |request, host, sources, limits| {
            if omit {
                return Ok(
                    cem_ml::schema::datatype_contracts::DatatypeCompilation::new(
                        request.source.ast_owner().clone(),
                    ),
                );
            }
            use cem_ml::schema::{
                datatype_registry::DatatypeKind,
                datatype_validation::{ScalarRepresentation, ValueRepresentation},
            };
            use cem_ql::datatype_compilation::{
                compile_datatypes, DatatypeImplementation, DatatypeImplementations,
                TokenizerBinding,
            };
            let mut implementations = DatatypeImplementations::default();
            for source in sources {
                if source.attribute("kind").is_some() {
                    implementations
                        .register(DatatypeImplementation {
                            source: source.clone(),
                            kind: DatatypeKind::Scalar,
                            representation: ValueRepresentation::Scalar(
                                ScalarRepresentation::String,
                            ),
                            accepted_bases: vec![],
                            bounds: Default::default(),
                            tokenizer: TokenizerBinding::Absent,
                            validator: None,
                        })
                        .unwrap();
                }
            }
            Ok(compile_datatypes(
                request.source.ast_owner().clone(),
                sources,
                host,
                &implementations,
                &Default::default(),
                limits,
            ))
        },
    )
}
#[test]
fn discovered_source_bindings_gate_package_publication_and_ready_retry() {
    let authored = SOURCE.replace(
        "{elements |",
        "{types | {type @name=derived @base=p:base} {type @name=base @kind=scalar}} {elements |",
    );
    let authored = format!("@ns p = https://example.test/schema/runtime/1\n{authored}");
    let mut context = context(&authored);
    context.schema_package_compiler = Some(Arc::new(discovered_compiler("old")));
    load(&mut context, &input());
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    let replacement_source = authored
        .replace("{type @name=base @kind=scalar}", "{#types}")
        .replace(
            "{elements |",
            "{library | {type @name=base @kind=scalar}} {elements |",
        );
    set_source(&mut context, &replacement_source);
    let mut replacement = input();
    replacement.bytes = MANIFEST
        .replace("runtime-converter", "new-converter")
        .replace("old.cemt", "new.cemt")
        .into_bytes();
    context.schema_package_compiler = Some(Arc::new(discovered_compiler("new")));
    load(&mut context, &replacement);
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    let candidate = context
        .schema_package_sources
        .get(SOURCE_URI)
        .unwrap()
        .clone();
    let pending = context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .datatype_compilation
        .as_ref()
        .unwrap();
    assert!(!pending.is_ready());
    assert_eq!(pending.issues[0].code, "datatype-selection-pending");
    assert!(pending.matches_owner(candidate.ast_owner()));
    context.schema_package_compiler =
        Some(Arc::new(discovered_native_compiler("new", false, true)));
    load(&mut context, &replacement);
    assert!(Arc::ptr_eq(
        &candidate,
        context.schema_package_sources.get(SOURCE_URI).unwrap()
    ));
    assert_active(&context, "new", "new-converter", "new.cemt");
    set_source(&mut context, &authored);
    load(&mut context, &replacement);
    assert_active(&context, "new", "new-converter", "new.cemt");
    context.schema_package_compiler = Some(Arc::new(discovered_compiler_mode("ignored", true)));
    load(&mut context, &replacement);
    assert_active(&context, "new", "new-converter", "new.cemt");
    let omitted = context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .datatype_compilation
        .as_ref()
        .unwrap();
    assert_eq!(omitted.sources.len(), 2);
    assert!(!omitted.is_ready());
    context.schema_package_compiler = Some(Arc::new(discovered_compiler("new")));
    set_source(&mut context, &replacement_source.replace("#types", "#1 +"));
    let diagnostics =
        load_schema_package_manifest_into_context(&mut context, &replacement).unwrap();
    let invalid = context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .datatype_compilation
        .as_ref()
        .unwrap();
    assert!(!invalid.is_ready());
    assert!(!invalid.diagnostics.is_empty());
    for original in &invalid.diagnostics {
        assert!(diagnostics.iter().any(|d| d.code == original.code
            && d.message == original.message
            && d.uri.as_deref() == Some(SOURCE_URI)));
    }
    assert_active(&context, "new", "new-converter", "new.cemt");
    set_source(
        &mut context,
        &authored.replace(
            "{type @name=base @kind=scalar}",
            "{type @name=base @kind=scalar}{type @name=base @kind=scalar}",
        ),
    );
    let diagnostics =
        load_schema_package_manifest_into_context(&mut context, &replacement).unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("duplicate-datatype-name") && d.source_map.is_some()),
        "{diagnostics:?}"
    );
    assert_active(&context, "new", "new-converter", "new.cemt");
}
