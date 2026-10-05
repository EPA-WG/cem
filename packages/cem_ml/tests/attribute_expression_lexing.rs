use cem_ml::{
    source::{BytesSource, SourceId},
    tokenizer::{
        cem::CemTokenizer, xml::XmlTokenizer, AttributeValueSyntax, SchemaToken, SchemaTokenKind,
        SchemaTokenizer,
    },
};
fn tokens(text: &str) -> Vec<SchemaToken> {
    let mut tokenizer =
        CemTokenizer::from_source(BytesSource::new(SourceId(1), text.as_bytes().to_vec()));
    let mut tokens = Vec::new();
    while let Some(token) = tokenizer.next_token() {
        tokens.push(token);
    }
    assert!(tokenizer.take_diagnostics().is_empty());
    tokens
}
fn attributes(tokens: &[SchemaToken]) -> Vec<(&str, Option<&str>, AttributeValueSyntax)> {
    tokens
        .iter()
        .filter_map(|token| match &token.kind {
            SchemaTokenKind::Attribute {
                name,
                value,
                value_syntax,
                ..
            } => Some((name.as_str(), value.as_deref(), *value_syntax)),
            _ => None,
        })
        .collect()
}
#[test]
fn quoted_literals_and_unquoted_brace_expressions_keep_distinct_syntax() {
    let source =
        r#"{item @native={#nodes} @quoted="{#nodes}" @single='{#nodes}' @bare=#nodes @boolean}"#;
    let tokens = tokens(source);
    assert_eq!(
        attributes(&tokens),
        vec![
            ("native", Some("{#nodes}"), AttributeValueSyntax::Expression),
            ("quoted", Some("{#nodes}"), AttributeValueSyntax::Literal),
            ("single", Some("{#nodes}"), AttributeValueSyntax::Literal),
            ("bare", Some("#nodes"), AttributeValueSyntax::Literal),
            ("boolean", None, AttributeValueSyntax::Literal),
        ]
    );
    let native = tokens
        .iter()
        .find(|token| matches!(&token.kind,SchemaTokenKind::Attribute{name,..}if name=="native"))
        .unwrap();
    let SchemaTokenKind::Attribute {
        value_range: Some(range),
        ..
    } = &native.kind
    else {
        panic!()
    };
    assert_eq!(
        &source[range.start as usize..range.end() as usize],
        "{#nodes}"
    );
}
#[test]
fn query_strings_nested_comments_and_records_do_not_close_the_attribute_span() {
    let expressions = [
        r#"{#seq:where(nodes, fn(x) => x.name == "}")}"#,
        r#"{#seq:where(nodes, fn(x) => x.name == 'it''s}') }"#,
        r#"{#seq:where(nodes, fn(x) => x.name == "a\"}b")}"#,
        "{#nodes (: } (: { :) still } :) }",
        r#"{#choose({nested: {value: "}"}}, nodes)}"#,
    ];
    for expression in expressions {
        let source = format!("{{item @target={expression} @after=kept}}{{$ #next}}");
        let tokens = tokens(&source);
        assert_eq!(
            attributes(&tokens),
            vec![
                ("target", Some(expression), AttributeValueSyntax::Expression),
                ("after", Some("kept"), AttributeValueSyntax::Literal)
            ],
            "{source}"
        );
        assert!(tokens.iter().any(
            |token| matches!(&token.kind,SchemaTokenKind::ExpressionNode(body) if body=="#next")
        ));
    }
}
#[test]
fn unterminated_query_attribute_span_retains_the_body_and_reports_the_boundary() {
    let source = r#"{item @target={#nodes (: } still in comment"#;
    let mut tokenizer =
        CemTokenizer::from_source(BytesSource::new(SourceId(1), source.as_bytes().to_vec()));
    let mut tokens = Vec::new();
    while let Some(token) = tokenizer.next_token() {
        tokens.push(token);
    }
    assert_eq!(
        attributes(&tokens)[0],
        (
            "target",
            Some("{#nodes (: } still in comment}"),
            AttributeValueSyntax::Expression
        )
    );
    assert!(tokenizer
        .take_diagnostics()
        .iter()
        .any(|d| d.message.contains("unterminated") && d.message.contains("attribute")));
}
#[test]
fn xml_braced_attribute_text_remains_literal_without_an_expression_contract() {
    let source = r#"<item target="{#nodes}"/>"#;
    let mut tokenizer =
        XmlTokenizer::from_source(BytesSource::new(SourceId(1), source.as_bytes().to_vec()));
    let mut tokens = Vec::new();
    while let Some(token) = tokenizer.next_token() {
        tokens.push(token);
    }
    assert_eq!(
        attributes(&tokens),
        vec![("target", Some("{#nodes}"), AttributeValueSyntax::Literal)]
    );
    assert!(tokenizer.take_diagnostics().is_empty());
}
