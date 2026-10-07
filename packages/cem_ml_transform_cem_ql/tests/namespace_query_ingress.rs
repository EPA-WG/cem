use cem_ml::{
    engine::{EngineInput, FormatIdentity, InputFormat},
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    parser::CemAstNode,
    query::{
        run_query_with_source_owner, QueryRunError, QueryRunRequest, QueryRunResponse, QuerySource,
        QuerySourceOwner,
    },
    run_config::ScopeConfig,
    schema::{
        declaration_references::SchemaDeclarationNode,
        namespace_references::{
            admit_namespace_scope_target, NamespaceNameCompletion, NamespaceNameCompletionError,
        },
        registry::CEM_QL_EXPRESSION_CONTENT_TYPE,
        vocab::CompiledSchema,
    },
};
use cem_ml_transform_cem_ql::{
    engine_context_with_cem_ql_template_adapter, CemQlNativeItemsOwner, CemQlQueryResultArtifact,
};
use cem_ql::eval::{retained_cem_node, AtomValue, Item, QueryContextScope};
use std::{collections::BTreeMap, sync::Arc};

fn import(text: &str) -> ScopedCemImport {
    import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "memory:namespace-input.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
}
fn element(input: &ScopedCemImport, local: &str) -> u32 {
    input
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == local => Some(*node_id),
            _ => None,
        })
        .unwrap()
}
fn declaration(input: &ScopedCemImport) -> u32 {
    input
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Attribute {
                node_id,
                expanded_name,
                value_nodes,
                ..
            } if expanded_name.namespace_uri == "xmlns" && !value_nodes.is_empty() => {
                Some(*node_id)
            }
            _ => None,
        })
        .unwrap()
}
fn completed(input: &ScopedCemImport, roots: &[u32], uri: &str) -> Arc<NamespaceNameCompletion> {
    let vendor = import(&format!("@ns public = {uri}\n{{public:item}}"));
    let target = admit_namespace_scope_target(
        SchemaDeclarationNode::new(vendor.tree.ast_owner().clone(), element(&vendor, "@ns"))
            .unwrap(),
        &vendor.captured,
    )
    .unwrap();
    Arc::new(
        NamespaceNameCompletion::new(
            input.captured.clone(),
            roots,
            BTreeMap::from([(declaration(input), target)]),
        )
        .unwrap(),
    )
}
fn request(expression: &str) -> QueryRunRequest {
    QueryRunRequest {
        // Retained-owner execution must not try to parse these bytes.
        data: EngineInput {
            uri: "memory:namespace-input.cem".into(),
            bytes: vec![0xff],
            from_format: Some(InputFormat::Cem),
            identity: Some(FormatIdentity {
                content_type: Some("text/cem-ml".into()),
                ..Default::default()
            }),
            root_scope: ScopeConfig::default(),
        },
        query: QuerySource {
            uri: "memory:namespace-query.cemql".into(),
            bytes: expression.as_bytes().to_vec(),
            identity: FormatIdentity {
                content_type: Some(CEM_QL_EXPRESSION_CONTENT_TYPE.into()),
                ..Default::default()
            },
        },
        context: engine_context_with_cem_ql_template_adapter(),
        context_item: None,
        bindings: Default::default(),
        limits: None,
    }
}
fn run(
    input: &ScopedCemImport,
    names: Arc<NamespaceNameCompletion>,
    expression: &str,
) -> QueryRunResponse {
    run_query_with_source_owner(
        request(expression),
        QuerySourceOwner::NamespaceCompleted {
            source: input.tree.clone(),
            completion: names,
        },
    )
    .unwrap()
}
fn items(response: &QueryRunResponse) -> &[Item] {
    &response
        .result
        .native_result
        .as_any()
        .downcast_ref::<CemQlQueryResultArtifact>()
        .unwrap()
        .stream()
        .items
}
fn strings(values: &[&str]) -> Vec<Item> {
    values
        .iter()
        .map(|value| Item::Atomic(AtomValue::String((*value).into())))
        .collect()
}

#[test]
fn independent_completion_ingress_exposes_names_and_original_authored_source() {
    let input = import("@ns v = urn:fixed\n{fixed @v:flag=yes}\n{host @xmlns:v={#library} | {v:item @v:flag=yes | {#related}}} {outside}");
    let root = element(&input, "item");
    let fixed = element(&input, "fixed");
    let mut identities = vec![];
    for uri in ["urn:one", "urn:two"] {
        let names = completed(&input, &[root, fixed], uri);
        let response = run(
            &input,
            names.clone(),
            "(input.namespace, input.attributes.namespace, input.source.namespace, input)",
        );
        let result = items(&response);
        assert_eq!(&result[..6], strings(&[uri, "", uri, "urn:fixed", "v", ""]));
        let owner = response
            .result
            .input_ast_owner
            .as_any()
            .downcast_ref::<CemQlNativeItemsOwner>()
            .unwrap();
        assert!(owner.lifecycle_owner().is_none());
        let QuerySourceOwner::NamespaceCompleted { source, completion } = owner.source_owner()
        else {
            panic!("completed owner")
        };
        assert!(Arc::ptr_eq(source, &input.tree));
        assert!(Arc::ptr_eq(completion, &names));
        assert_eq!(owner.stream().items.len(), 2);
        let projected = &result[6];
        identities.push(projected.identity());
        let original = retained_cem_node(projected).unwrap();
        assert!(Arc::ptr_eq(original.owner(), &input.tree));
        assert_eq!(original.node_id(), root);
        let raw = projected.view().unwrap().field("source").unwrap().remove(0);
        assert_eq!(projected.source_map(), raw.source_map());
        assert!(!projected.source_map().unwrap().frames.is_empty());
    }
    assert_ne!(identities[0], identities[1]);
    assert!(input
        .captured
        .expanded_name(input.tree.ast_owner(), root)
        .is_none());
    assert!(input.tree.ast().nodes.iter().all(|node| !matches!(
        node,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}

#[test]
fn reference_targets_and_axes_retain_selected_forest_and_authored_descendants() {
    let input = import("{host @xmlns:v={#library} | {v:item @v:flag=yes | content {#related}} {v:sibling}} {outside}");
    let root = element(&input, "item");
    let response = run(
        &input,
        completed(&input, &[root], "urn:ready"),
        "(input.parent, input, #input, input.children)",
    );
    let result = items(&response);
    let projected = &result[0];
    assert!(projected
        .view()
        .unwrap()
        .parent(QueryContextScope(0))
        .unwrap()
        .is_none());
    let targets = result[1].view().unwrap().field("targets").unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].identity(), projected.identity());
    assert!(Arc::ptr_eq(
        retained_cem_node(&targets[0]).unwrap().owner(),
        &input.tree
    ));
    let authored = result[2..]
        .iter()
        .find(|item| {
            matches!(
                retained_cem_node(item).unwrap().node(),
                CemAstNode::Reference { .. }
            )
        })
        .unwrap();
    assert!(
        matches!(retained_cem_node(authored).unwrap().node(), CemAstNode::Reference {expression, targets: None, ..} if expression == "#related")
    );
    assert!(authored.view().unwrap().field("targets").is_none());
    let response = run(&input, completed(&input, &[root], "urn:ready"), "(data:node_key(input), data:line_number(input), data:base_uri(input), data:document_uri(input))");
    let raw = run(&input, completed(&input, &[root], "urn:ready"), "(data:node_key(input.source), data:line_number(input.source), data:base_uri(input.source), data:document_uri(input.source))");
    assert_eq!(items(&response), items(&raw));
}

#[test]
fn mismatched_owner_rejects_before_query_compilation_and_pending_qnames_do_not_enter() {
    let input = import("{host @xmlns:v={#library} | {v:item}}");
    let foreign = import("{host @xmlns:v={#library} | {v:item}}");
    let root = element(&input, "item");
    assert!(
        matches!(NamespaceNameCompletion::new(input.captured.clone(), &[root], BTreeMap::new()), Err(NamespaceNameCompletionError::Pending {declaration: dep, ..}) if dep == declaration(&input))
    );
    let names = completed(&input, &[root], "urn:ready");
    let error = run_query_with_source_owner(
        request("("),
        QuerySourceOwner::NamespaceCompleted {
            source: foreign.tree,
            completion: names,
        },
    )
    .unwrap_err();
    let QueryRunError::Execution(failure) = error else {
        panic!("owner mismatch diagnostic")
    };
    assert_eq!(failure.diagnostics.len(), 1);
    assert_eq!(
        failure.diagnostics[0].code,
        "cem.ql.query_input_unsupported"
    );
    assert!(failure.diagnostics[0]
        .message
        .contains("another source owner"));
}

#[test]
fn empty_selected_forest_produces_empty_input_without_falling_back_to_whole_document() {
    let input = import("{host @xmlns:v={#library} | {v:item}}");
    let names = Arc::new(
        NamespaceNameCompletion::new(input.captured.clone(), &[], BTreeMap::new()).unwrap(),
    );
    let response = run(&input, names, "(input, #input)");
    assert_eq!(items(&response).len(), 1);
    assert!(items(&response)[0]
        .view()
        .unwrap()
        .field("targets")
        .unwrap()
        .is_empty());
    let owner = response
        .result
        .input_ast_owner
        .as_any()
        .downcast_ref::<CemQlNativeItemsOwner>()
        .unwrap();
    assert!(owner.stream().items.is_empty());
}

#[test]
fn retained_owner_runner_keeps_cancellation_budgets_and_language_admission() {
    use cem_ml::{
        query::QueryExecutionLimits,
        schema::registry::{CSS_SELECTOR_CONTENT_TYPE, XPATH_CONTENT_TYPE},
    };
    let input = import("{host @xmlns:v={#library} | {v:item}}");
    let names = completed(&input, &[element(&input, "item")], "urn:ready");
    let owner = || QuerySourceOwner::NamespaceCompleted {
        source: input.tree.clone(),
        completion: names.clone(),
    };
    let mut limited = request("(input, input)");
    limited.limits = Some(QueryExecutionLimits {
        max_result_items: Some(1),
        ..Default::default()
    });
    let error = run_query_with_source_owner(limited, owner()).unwrap_err();
    assert!(format!("{error:?}").contains("cem.ql.query_result_limit_exceeded"));
    let cancelled = request("(");
    cancelled.context.abort_signal().abort();
    let error = run_query_with_source_owner(cancelled, owner()).unwrap_err();
    assert!(format!("{error:?}").contains("cem.query.cancelled"));
    for (media, code) in [
        (
            CSS_SELECTOR_CONTENT_TYPE,
            "cem.css_selector.input_unsupported",
        ),
        (XPATH_CONTENT_TYPE, "cem.xpath.query_input_unsupported"),
    ] {
        let mut other_language = request("*");
        other_language.query.identity = FormatIdentity {
            content_type: Some(media.into()),
            ..Default::default()
        };
        let QueryRunError::Execution(failure) =
            run_query_with_source_owner(other_language, owner()).unwrap_err()
        else {
            panic!("language admission")
        };
        assert!(
            failure
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code),
            "{:?}",
            failure.diagnostics
        );
    }
}
