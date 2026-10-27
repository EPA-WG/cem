use cem_ml::{
    engine::{
        CemMlEngine, EngineContext, EngineInput, FormatIdentity, InspectRequest, InspectView,
    },
    events::cem::CemEventNormalizer,
    parser::{builder::CemAstBuilder, CemAstNode},
    real::RealCemMlEngine,
    run_config::ScopeConfig,
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};

#[test]
fn typed_xml_inspection_reimports_mixed_quotes_and_embedded_rich_content_losslessly() {
    let literal = "can't \"quote\" ```{#not-evaluated} {$never()} ``` Café 🧬\r\n";
    let source = format!("<note><![CDATA[{literal}]]></note>");
    for show in [InspectView::Ast, InspectView::Tree] {
        let response = RealCemMlEngine::new()
            .inspect(InspectRequest {
                input: EngineInput {
                    uri: "memory:mixed.xml".into(),
                    bytes: source.as_bytes().to_vec(),
                    from_format: None,
                    identity: Some(FormatIdentity {
                        content_type: Some("application/xml".into()),
                        ..Default::default()
                    }),
                    root_scope: Default::default(),
                },
                show,
                presentation_scope: Some(ScopeConfig {
                    cemt_formatter_profile: Some("tabular".into()),
                    output_color_type: Some("none".into()),
                    ..Default::default()
                }),
                context: EngineContext::default(),
            })
            .expect("admitted typed CEM inspection");
        let bytes = response.primary_bytes.unwrap().bytes;
        let document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
            BytesSource::new(SourceId(19), bytes),
        )))
        .build();
        assert!(
            document.diagnostics.is_empty(),
            "{:?}",
            document.diagnostics
        );
        // Source inspection preserves authored CDATA bytes, including CRLF,
        // and never evaluates its reference-looking payload.
        let row = document
            .nodes
            .iter()
            .find_map(|node| {
                let CemAstNode::Element { attributes, .. } = node else {
                    return None;
                };
                let values: std::collections::BTreeMap<_, _> = attributes
                    .iter()
                    .filter_map(|id| match document.get(*id) {
                        Some(CemAstNode::Attribute {
                            expanded_name,
                            value: Some(value),
                            value_nodes,
                            ..
                        }) => {
                            assert!(value_nodes.is_empty(), "inspection attributes stay literal");
                            Some((expanded_name.local_name.as_str(), value.as_str()))
                        }
                        _ => None,
                    })
                    .collect();
                (values.get("kind") == Some(&"cdata")).then_some(values)
            })
            .expect("original CDATA inspection row");
        assert_eq!(row.get("value"), Some(&literal));
        assert_eq!(row.get("source-id"), Some(&"memory:mixed.xml"));
        assert_eq!(row.get("byte-offset"), Some(&"6"));
        assert_eq!(
            row["byte-length"].parse::<usize>().unwrap(),
            literal.len() + "<![CDATA[]]>".len()
        );
    }
}

#[test]
fn canonical_component_ast_is_admitted_as_native_cem_text() {
    let input = include_bytes!("../../cem-components/src/components/cem-card/cem-card.xhtml");
    let response = RealCemMlEngine::new()
        .inspect(InspectRequest {
            input: EngineInput {
                uri: "memory:cem-card.xhtml".into(),
                bytes: input.to_vec(),
                from_format: None,
                identity: Some(FormatIdentity {
                    content_type: Some("application/xml".into()),
                    ..Default::default()
                }),
                root_scope: Default::default(),
            },
            show: InspectView::Ast,
            presentation_scope: Some(ScopeConfig {
                cemt_formatter_profile: Some("tabular".into()),
                output_color_type: Some("none".into()),
                ..Default::default()
            }),
            context: EngineContext::default(),
        })
        .expect("canonical declaration uses default CEM inspection");
    let bytes = response.primary_bytes.unwrap().bytes;
    let document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(19), bytes),
    )))
    .build();
    assert!(
        document.diagnostics.is_empty(),
        "{:?}",
        document.diagnostics
    );
    assert!(document.nodes.iter().any(|node| matches!(node,
        CemAstNode::Attribute { value: Some(value), .. } if value == "cem-card")));
}
