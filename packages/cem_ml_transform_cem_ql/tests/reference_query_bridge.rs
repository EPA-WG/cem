use cem_ml::{
    engine::{EngineInput, FormatIdentity, InputFormat},
    lifecycle::LoadedInputAstStream,
    parser::CemAstNode,
    query::{run_query, QueryRunRequest, QueryRunResponse, QuerySource},
    run_config::ScopeConfig,
    schema::registry::CEM_QL_EXPRESSION_CONTENT_TYPE,
};
use cem_ml_transform_cem_ql::{
    engine_context_with_cem_ql_template_adapter, CemQlNativeItemsOwner, CemQlQueryResultArtifact,
};
use cem_ql::eval::{retained_cem_node, Item, QueryContextScope, QueryItemViewKind};
use std::sync::Arc;

fn run(
    format: Option<InputFormat>,
    media: &str,
    source: &str,
    expression: &str,
) -> QueryRunResponse {
    run_query(QueryRunRequest {
        data: EngineInput {
            uri: "memory:reference-bridge".into(),
            bytes: source.as_bytes().to_vec(),
            from_format: format,
            identity: Some(FormatIdentity {
                content_type: Some(media.into()),
                ..Default::default()
            }),
            root_scope: ScopeConfig {
                default_content_type: Some(media.into()),
                ..Default::default()
            },
        },
        query: QuerySource {
            uri: "memory:reference-query.cemql".into(),
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
    })
    .unwrap()
}
fn targets(item: &Item) -> Vec<Item> {
    let view = item.view().expect("native reference");
    assert_eq!(view.kind(), QueryItemViewKind::Node);
    assert_eq!(
        view.field("kind").unwrap()[0].atom(),
        Some(cem_ql::eval::AtomValue::String("reference".into()))
    );
    assert!(view
        .children(QueryContextScope(0))
        .unwrap()
        .next()
        .is_none());
    view.field("targets").expect("constructed targets")
}
#[test]
fn lifecycle_query_bridge_constructs_references_to_original_native_owners_across_formats() {
    for (format, media, source) in [
        (Some(InputFormat::Xml), "application/xml", "<root xmlns:r='https://cem.dev/ns/cem-ml/1'><r:expr>#missing</r:expr><value ref='{#missing}'>#missing</value></root>"),
        (None, "application/json", r##"{"value":"#missing","ref":"{#missing}"}"##),
        (None, "application/yaml", "value: '#missing'\nref: '{#missing}'\n"),
        (None, "text/csv", "value,ref\n#missing,{#missing}\n"),
    ] {
        let response = run(format, media, source, "(input, #(input, input), ##input, #())");
        assert!(response.diagnostics.iter().all(|d| !d.severity.is_hard_violation()), "{:?}", response.diagnostics);
        let input = response.result.input_ast_owner.as_any().downcast_ref::<CemQlNativeItemsOwner>().unwrap();
        let result = response.result.native_result.as_any().downcast_ref::<CemQlQueryResultArtifact>().unwrap();
        let items = &result.stream().items;
        assert_eq!(items.len(), 4);
        let original = retained_cem_node(&input.stream().items[0]).unwrap();
        let returned = retained_cem_node(&items[0]).unwrap();
        assert!(Arc::ptr_eq(original.owner(), returned.owner()));
        assert_eq!(returned.node_id(), 0);
        let native = original.owner().native_owner().unwrap().downcast_ref::<LoadedInputAstStream>().unwrap();
        assert!(std::ptr::eq(native, input.lifecycle_owner().as_ref()));
        assert_eq!(original.owner().source_uri(), "memory:reference-bridge");
        assert!(!items[0].source_map().unwrap().frames.is_empty());
        let repeated = targets(&items[1]);
        assert_eq!(repeated.len(), 2);
        for item in repeated {
            let target = retained_cem_node(&item).unwrap();
            assert!(Arc::ptr_eq(target.owner(), original.owner()));
            assert_eq!(target.node_id(), original.node_id());
        }
        let nested = targets(&items[2]);
        assert_eq!(nested.len(), 1);
        let inner = targets(&nested[0]);
        assert_eq!(inner.len(), 1);
        assert!(Arc::ptr_eq(retained_cem_node(&inner[0]).unwrap().owner(), original.owner()));
        assert!(targets(&items[3]).is_empty());
        let references: Vec<_> = original.owner().ast().nodes.iter().filter(|node| matches!(node, CemAstNode::Reference { .. })).collect();
        assert_eq!(references.len(), usize::from(format == Some(InputFormat::Xml)));
        for node in references {
            assert!(matches!(node, CemAstNode::Reference { expression, targets: None, .. } if expression == "#missing"));
        }
        assert!(original.owner().ast().nodes.iter().any(|node| matches!(node,
            CemAstNode::Text { data, .. } if data == "#missing")));
    }
}
#[test]
fn lifecycle_query_bridge_can_reference_an_authored_reference_without_evaluating_it() {
    let response = run(
        Some(InputFormat::Xml),
        "application/xml",
        "<root xmlns:r='https://cem.dev/ns/cem-ml/1'><r:expr>#unavailable</r:expr><target/></root>",
        "#(input.children.children, input.children.children)",
    );
    let input = response
        .result
        .input_ast_owner
        .as_any()
        .downcast_ref::<CemQlNativeItemsOwner>()
        .unwrap();
    let original = retained_cem_node(&input.stream().items[0]).unwrap();
    let result = response
        .result
        .native_result
        .as_any()
        .downcast_ref::<CemQlQueryResultArtifact>()
        .unwrap();
    let selected = targets(&result.stream().items[0]);
    assert_eq!(selected.len(), 4);
    for item in &selected {
        let target = retained_cem_node(item).unwrap();
        assert!(Arc::ptr_eq(target.owner(), original.owner()));
    }
    assert_eq!(
        selected[0].view().unwrap().identity(),
        selected[2].view().unwrap().identity()
    );
    let authored = retained_cem_node(&selected[0]).unwrap();
    assert!(
        matches!(authored.node(), CemAstNode::Reference { expression, targets: None, .. } if expression == "#unavailable")
    );
    assert!(selected[0].view().unwrap().field("targets").is_none());
}
