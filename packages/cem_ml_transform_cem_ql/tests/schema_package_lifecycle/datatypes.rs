use super::*;

fn datatype_compiler(name: &str, tokenizer_ready: bool) -> CemQlSchemaPackageCompiler {
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
            let list=name=="names";
            implementations.register(DatatypeImplementation{source:source.clone(),kind:if list {DatatypeKind::List}else{DatatypeKind::Scalar},representation:if list {ValueRepresentation::List(ScalarRepresentation::String)}else{ValueRepresentation::Scalar(ScalarRepresentation::String)},accepted_bases:vec![],bounds:Default::default(),tokenizer:if !list {TokenizerBinding::Absent}else if tokenizer_ready {TokenizerBinding::Ready(cem_ml::schema::datatype_contracts::RegisteredTokenizer::whitespace())}else{TokenizerBinding::Unavailable},validator:None}).unwrap();sources.push(source);
        }}}
        Ok(compile_datatypes(ast.clone(),&sources,host,&implementations,&Default::default(),limits))
    })
}
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
                kind: DatatypeKind::Node,
                representation: ValueRepresentation::Nodes,
                accepted_bases: vec![],
                bounds: Default::default(),
                tokenizer: TokenizerBinding::Absent,
                validator: None,
            })
            .unwrap();
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

fn discovered_compiler(name: &str) -> CemQlSchemaPackageCompiler {
    discovered_compiler_mode(name, false)
}
fn discovered_compiler_mode(name: &str, omit: bool) -> CemQlSchemaPackageCompiler {
    use cem_ql::datatype_names::DatatypeSchemaSource;
    compiler(Some(name)).with_datatype_discovery(
        Default::default(),
        |request, _| {
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
    let replacement_source = authored.replace("{type @name=base @kind=scalar}", "{#types}");
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
    assert_eq!(
        pending.issues[0].code,
        "datatype-collection-selection-unavailable"
    );
    assert!(pending.matches_owner(candidate.ast_owner()));
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
