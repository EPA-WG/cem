use super::*;
use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    schema::{attribute_datatypes::*, document_model::attribute_facets::FacetFamily},
};
use std::collections::BTreeMap;

fn compiler(ready: bool) -> CemQlSchemaPackageCompiler {
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
                .find_map(|n| match n {
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
        move |request, host, sources, limits| {
            let mut registrations = DatatypeImplementations::default();
            for source in sources {
                let derived = source.attribute("base").is_some();
                if !derived {
                    registrations
                        .register(DatatypeImplementation {
                            source: source.clone(),
                            kind: DatatypeKind::Scalar,
                            representation: ShippedDatatype::String.representation(),
                            accepted_bases: vec![],
                            bounds: Default::default(),
                            tokenizer: TokenizerBinding::Absent,
                            validator: None,
                        })
                        .unwrap();
                    registrations
                        .select_preparation(
                            source.clone(),
                            PreparationBinding::Ready(
                                cem_ql::datatype_shipped::lexical_preparation(
                                    source.clone(),
                                    ShippedDatatype::String,
                                )
                                .unwrap(),
                            ),
                        )
                        .unwrap();
                }
                let profile = RegisteredFacetProfile::new(
                    source.clone(),
                    "same-id",
                    FacetFamily::Shipped(if derived {
                        ShippedDatatype::String
                    } else {
                        ShippedDatatype::Uri
                    }),
                )
                .unwrap();
                registrations
                    .select_facets(
                        source.clone(),
                        if !derived && !ready {
                            FacetProfileBinding::Unavailable
                        } else if derived {
                            FacetProfileBinding::CheckedReplacement(profile)
                        } else {
                            FacetProfileBinding::Ready(profile)
                        },
                    )
                    .unwrap();
            }
            Ok(compile_datatypes(
                request.source.ast_owner().clone(),
                sources,
                host,
                &registrations,
                &Default::default(),
                limits,
            ))
        },
    )
}

#[test]
fn checked_facet_profiles_preserve_defaults_receipts_and_atomic_package_replacement() {
    let authored = authored("derived", "@default=\"https://allowed.example/a\"")
        .replace(
            "{type @name=integer @kind=scalar}",
            "{type @name=base @kind=scalar} {type @name=derived @base=base}",
        )
        .replace("@maxInclusive=4", "");
    let mut context = context(&authored);
    context.schema_package_compiler = Some(Arc::new(compiler(true)));
    load(&mut context, &input());
    let active = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap()
        .clone();
    for (text, accepted) in [("https://allowed.example/a", true), ("not-uri", false)] {
        let original = tree(&format!("{{sample @count=\"{text}\"}}"));
        let diagnostics = validate_document_model(original.ast(), &active);
        assert_eq!(
            !diagnostics.iter().any(|d| d.severity.is_hard_violation()),
            accepted,
            "{diagnostics:?}"
        );
    }
    let original = tree("{sample @count=\"https://allowed.example/a\"}");
    let source = original.ast().nodes.iter().find(|n| matches!(n, CemAstNode::Attribute { expanded_name, .. } if expanded_name.local_name == "count")).unwrap();
    let values = BTreeMap::new();
    let control = OperationControl::default();
    let epoch = AttributeDatatypeContext::default();
    let args = |value| AttributeDatatypeInput {
        value,
        source,
        source_tree: Some(original.clone()),
        element_name: "sample",
        attribute_values: &values,
        control: &control,
        context: Some(&epoch),
    };
    let contract = &active.attribute_datatypes["count"];
    let handle = contract
        .prepare(args(AttributeDatatypeValue::Lexical(
            "https://allowed.example/a",
        )))
        .handle
        .unwrap();
    for mode in ["invalid-default", "unsupported-field", "unavailable"] {
        let candidate = match mode {
            "invalid-default" => {
                authored.replace("@default=\"https://allowed.example/a\"", "@default=not-uri")
            }
            "unsupported-field" => {
                authored.replace("@type=derived", "@type=derived @uriHosts=allowed.example")
            }
            _ => authored.replace("allowed.example/a", "allowed.example/b"),
        };
        set_source(&mut context, &candidate);
        context.schema_package_compiler = Some(Arc::new(compiler(mode != "unavailable")));
        load_schema_package_manifest_into_context(&mut context, &input()).unwrap();
        assert!(
            Arc::ptr_eq(
                active.datatype_compilation.as_ref().unwrap(),
                context
                    .schema_document_models
                    .resolve_for_identity(Some(SCHEMA_URI), None, None)
                    .unwrap()
                    .datatype_compilation
                    .as_ref()
                    .unwrap()
            ),
            "{mode}"
        );
        assert_eq!(
            contract
                .validate(args(AttributeDatatypeValue::Prepared(&handle)))
                .accepted,
            Some(true),
            "{mode}"
        );
    }
    context.schema_package_compiler = Some(Arc::new(compiler(true)));
    load(&mut context, &input());
    assert!(!Arc::ptr_eq(
        active.datatype_compilation.as_ref().unwrap(),
        context
            .schema_document_models
            .resolve_for_identity(Some(SCHEMA_URI), None, None)
            .unwrap()
            .datatype_compilation
            .as_ref()
            .unwrap()
    ));
    assert_eq!(
        contract
            .validate(args(AttributeDatatypeValue::Prepared(&handle)))
            .accepted,
        None
    );
    control.complete_scope(ROOT_EXECUTION_SCOPE_ID).unwrap();
}
