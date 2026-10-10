use super::*;
use cem_ql::datatype_compilation::ExecutableDatatype;

fn derived(
    compilation: &cem_ml::schema::datatype_contracts::DatatypeCompilation,
) -> &ExecutableDatatype {
    compilation
        .contracts
        .iter()
        .find_map(|entry| {
            entry
                .as_any()
                .downcast_ref::<ExecutableDatatype>()
                .filter(|d| d.base().is_some())
        })
        .unwrap()
}

#[test]
fn whole_list_package_replacement_waits_for_ancestors_and_preserves_active_contracts() {
    let authored = SOURCE.replace("{elements |", "{types | {type @name=item @kind=scalar} {type @name=names @kind=list @base=item} {type @name=derived @list-base=names @max-items=3}} {elements |");
    let mut context = context(&authored);
    context.schema_package_compiler = Some(Arc::new(datatype_compiler_with_serializer(
        "old",
        true,
        Some(true),
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
    let request = cem_ql::datatype_validation::ValidationInput {
        value: vec![cem_ql::eval::Item::Atomic(cem_ql::eval::AtomValue::String("a".into())); 2],
        candidate: vec![],
        fallback: Default::default(),
    };
    let control = cem_ml::operation_control::OperationControl::default();
    let runtime = cem_ql::datatype_validation::ValidationRuntime {
        control: &control,
        scope: cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    assert_eq!(
        derived(&old)
            .serialize_list(&request, &runtime, Default::default())
            .text
            .as_deref(),
        Some("a a")
    );
    set_source(
        &mut context,
        &authored.replace("@max-items=3", "@max-items=1"),
    );
    let mut replacement = input();
    replacement.bytes = MANIFEST
        .replace("runtime-converter", "new-converter")
        .replace("old.cemt", "new.cemt")
        .into_bytes();
    context.schema_package_compiler = Some(Arc::new(datatype_compiler_with_serializer(
        "new",
        true,
        Some(false),
    )));
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
    assert!(pending
        .issues
        .iter()
        .any(|i| i.code == "datatype-list-serializer-unavailable"));
    assert!(Arc::ptr_eq(
        &old,
        context
            .schema_document_models
            .resolve_for_identity(Some(SCHEMA_URI), None, None)
            .unwrap()
            .datatype_compilation
            .as_ref()
            .unwrap()
    ));
    context.schema_package_compiler = Some(Arc::new(datatype_compiler_with_serializer(
        "new",
        true,
        Some(true),
    )));
    load(&mut context, &replacement);
    assert_active(&context, "new", "new-converter", "new.cemt");
    assert!(Arc::ptr_eq(
        &candidate,
        context.schema_package_sources.get(SOURCE_URI).unwrap()
    ));
    let new = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap()
        .datatype_compilation
        .as_ref()
        .unwrap();
    assert_eq!(
        derived(new)
            .serialize_list(&request, &runtime, Default::default())
            .accepted,
        Some(false)
    );
    assert_eq!(
        derived(&old)
            .serialize_list(&request, &runtime, Default::default())
            .text
            .as_deref(),
        Some("a a")
    );
    set_source(
        &mut context,
        &authored.replace("@list-base=names", "@list-base=item"),
    );
    let diagnostics =
        load_schema_package_manifest_into_context(&mut context, &replacement).unwrap();
    assert!(diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation() && d.message.contains("incompatible-list-base")));
    assert_active(&context, "new", "new-converter", "new.cemt");
}

fn attribute_compiler() -> CemQlSchemaPackageCompiler {
    use cem_ml::schema::{
        datatype_contracts::RegisteredTokenizer,
        datatype_registry::DatatypeKind,
        datatype_validation::{ScalarRepresentation, ValueRepresentation},
        declaration_references::SchemaDeclarationNode,
        document_model::{attribute_facets::FacetFamily, shipped_datatypes::ShippedDatatype as T},
    };
    use cem_ql::{
        datatype_compilation::{
            compile_datatypes, DatatypeImplementation, DatatypeImplementations, TokenizerBinding,
        },
        datatype_facets::{FacetProfileBinding, RegisteredFacetProfile},
        datatype_preparation::{PreparationBinding, RegisteredLexicalPreparation},
    };
    CemQlSchemaPackageCompiler::new(|request| {
        let policy = ReferenceScopePolicy::schema_defaults().unwrap();
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(
            request.source.clone(),
            Some(Default::default()),
            policy.clone(),
        );
        host.attach_captured_names(&request.lexical_scopes).unwrap();
        Ok((host, policy.limits))
    })
    .with_datatype_discovery(
        Default::default(),
        |request, _| {
            let schema = request
                .source
                .ast()
                .nodes
                .iter()
                .find_map(|node| match node {
                    CemAstNode::Element {
                        node_id,
                        expanded_name,
                        ..
                    } if expanded_name.local_name == "schema" => {
                        SchemaDeclarationNode::new(request.source.ast_owner().clone(), *node_id)
                    }
                    _ => None,
                })
                .unwrap();
            Ok(vec![cem_ql::datatype_names::DatatypeSchemaSource {
                schema,
                captured: request.lexical_scopes.clone(),
                imports: vec![],
            }])
        },
        |request, host, sources, limits| {
            let mut implementations = DatatypeImplementations::default();
            for source in sources {
                if source.attribute("list-base").is_some() {
                    continue;
                }
                let list = source.attribute("base").is_some();
                implementations
                    .register(DatatypeImplementation {
                        source: source.clone(),
                        kind: if list {
                            DatatypeKind::List
                        } else {
                            DatatypeKind::Scalar
                        },
                        representation: if list {
                            ValueRepresentation::List(ScalarRepresentation::String)
                        } else {
                            ValueRepresentation::Scalar(ScalarRepresentation::String)
                        },
                        accepted_bases: vec![],
                        bounds: Default::default(),
                        validator: None,
                        tokenizer: if list {
                            TokenizerBinding::Ready(RegisteredTokenizer::whitespace())
                        } else {
                            TokenizerBinding::Absent
                        },
                    })
                    .unwrap();
                implementations
                    .select_preparation(
                        source.clone(),
                        PreparationBinding::Ready(if list {
                            RegisteredLexicalPreparation::list_items(
                                source.clone(),
                                "items",
                                ScalarRepresentation::String,
                            )
                            .unwrap()
                        } else {
                            cem_ql::datatype_shipped::lexical_preparation(source.clone(), T::String)
                                .unwrap()
                        }),
                    )
                    .unwrap();
                if list {
                    implementations
                        .select_facets(
                            source.clone(),
                            FacetProfileBinding::Ready(
                                RegisteredFacetProfile::new(
                                    source.clone(),
                                    "list",
                                    FacetFamily::List(ScalarRepresentation::String),
                                )
                                .unwrap(),
                            ),
                        )
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
fn whole_list_attributes_activate_validate_defaults_and_consume_native_preparation() {
    use cem_ml::{
        operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
        schema::{attribute_datatypes::*, document_model::validate_document_model},
    };
    let authored = format!("{{schema @name=runtime @namespace=\"{SCHEMA_URI}\" @version=1.0.0 | {{types | {{type @name=item @kind=scalar}} {{type @name=names @kind=list @base=item}} {{type @name=pair @list-base=names @max-items=2}}}} {{attributes | {{attribute @name=value @type=pair @itemCount=2 @default='a a'}}}} {{elements | {{element @name=sample @optional-attributes=value}}}}}}");
    let mut context = context(&authored);
    context.schema_package_compiler = Some(Arc::new(attribute_compiler()));
    load(&mut context, &input());
    let active = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap()
        .clone();
    assert!(active.is_ready_for_validation());
    for (text, valid) in [("a a", true), ("a", false), ("a b c", false), ("", false)] {
        let original = tree(&format!("{{sample @value='{text}'}}"));
        let diagnostics = validate_document_model(original.ast(), &active);
        assert_eq!(
            !diagnostics.iter().any(|d| d.severity.is_hard_violation()),
            valid,
            "{diagnostics:?}"
        );
        let source = original.ast().nodes.iter().find(|node| matches!(node, CemAstNode::Attribute { expanded_name, .. } if expanded_name.local_name == "value")).unwrap();
        let epoch = AttributeDatatypeContext::default();
        let control = OperationControl::default();
        let values = std::collections::BTreeMap::new();
        let args = |value| AttributeDatatypeInput {
            value,
            source,
            source_tree: Some(original.clone()),
            element_name: "sample",
            attribute_values: &values,
            control: &control,
            context: Some(&epoch),
        };
        let contract = &active.attribute_datatypes["value"];
        let prepared = contract.prepare(args(AttributeDatatypeValue::Lexical(text)));
        assert_eq!(prepared.accepted, None);
        let handle = prepared.handle.unwrap();
        assert_eq!(
            contract
                .validate(args(AttributeDatatypeValue::Prepared(&handle)))
                .accepted,
            Some(valid)
        );
        control.complete_scope(ROOT_EXECUTION_SCOPE_ID).unwrap();
        assert_eq!(
            contract
                .validate(args(AttributeDatatypeValue::Prepared(&handle)))
                .accepted,
            None
        );
    }
    set_source(
        &mut context,
        &authored.replace("@default='a a'", "@default='a'"),
    );
    let errors = load_schema_package_manifest_into_context(&mut context, &input()).unwrap();
    assert!(errors.iter().any(|d| d.severity.is_hard_violation()));
    assert!(Arc::ptr_eq(
        active.datatype_compilation.as_ref().unwrap(),
        context
            .schema_document_models
            .resolve_for_identity(Some(SCHEMA_URI), None, None)
            .unwrap()
            .datatype_compilation
            .as_ref()
            .unwrap()
    ));
}
