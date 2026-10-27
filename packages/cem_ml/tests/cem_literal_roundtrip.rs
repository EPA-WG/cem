use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::{builder::CemAstBuilder, CemAstNode},
    source::{BytesSource, SourceId},
    tokenizer::{cem::CemTokenizer, AttributeValueSyntax, SchemaTokenKind, SchemaTokenizer},
    transform_template::{
        transform_template_encode_cem_attribute_value, transform_template_encode_cem_string_literal,
    },
};

fn check_value(value: &str) {
    let encoded =
        transform_template_encode_cem_string_literal(value).expect("admitted scalar literal");
    let prefix = "{row @prefix=\"Café 🧬\" @value=";
    let source = format!("{prefix}{encoded} @after=ok}}");
    let mut tokenizer =
        CemTokenizer::from_source(BytesSource::new(SourceId(31), source.as_bytes().to_vec()));
    let mut found = false;
    while let Some(token) = tokenizer.next_token() {
        if let SchemaTokenKind::Attribute {
            name,
            value: actual,
            value_range,
            value_syntax,
            ..
        } = token.kind
        {
            if name == "value" {
                assert_eq!(actual.as_deref(), Some(value));
                assert_eq!(value_syntax, AttributeValueSyntax::Literal);
                let range = value_range.unwrap();
                assert_eq!(range.start as usize, prefix.len());
                assert_eq!(
                    &source.as_bytes()[range.start as usize..range.end() as usize],
                    encoded.as_bytes()
                );
                found = true;
            }
        }
    }
    assert!(found);
    assert!(tokenizer.take_diagnostics().is_empty());
    let document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(31), source.as_bytes().to_vec()),
    )))
    .build();
    assert!(
        document.diagnostics.is_empty(),
        "{:?}",
        document.diagnostics
    );
    assert!(document.nodes.iter().any(|node| matches!(node,
        CemAstNode::Attribute { expanded_name, value: Some(actual), value_nodes, .. }
        if expanded_name.local_name == "value" && actual == value && value_nodes.is_empty())));
    assert!(document.nodes.iter().any(|node| matches!(node,
        CemAstNode::Attribute { expanded_name, value: Some(actual), .. }
        if expanded_name.local_name == "after" && actual == "ok")));
    assert_eq!(
        transform_template_encode_cem_attribute_value(value, "test attribute").unwrap(),
        encoded
    );
}

#[test]
fn mixed_quote_literals_roundtrip_without_expression_or_fence_interpretation() {
    for value in [
        "can't \"quote\"",
        "\"'",
        "'\"```",
        "```\"'",
        "can't \"quote\" ```retained template source```",
        "\"'\n````\n{#missing} {$never()} @src=not-read | \\path\n```\n",
        "\"'\r",
        "\"'\r\n",
        "\"'\n",
        "\n\"'",
        "\"'\tCafé 🧬",
    ] {
        check_value(value);
    }
}

#[test]
fn ordinary_quotes_backslashes_and_control_refusals_keep_their_contract() {
    for value in [
        "",
        "text with spaces",
        "C:\\temp\\new\\",
        "say \"yes\"",
        "owner's",
        "line\r\nnext",
    ] {
        let expected = if value.contains('"') {
            format!("'{value}'")
        } else {
            format!("\"{value}\"")
        };
        assert_eq!(
            transform_template_encode_cem_string_literal(value).unwrap(),
            expected
        );
        // Attribute bare values retain their separate canonical policy.
        if value.is_empty() || value.contains([' ', '"', '\'', '\\', '\r', '\n']) {
            check_value(value);
        }
    }
    for value in ["\"'\0", "\"'\u{b}", "\"'\u{7f}"] {
        assert!(transform_template_encode_cem_string_literal(value)
            .unwrap_err()
            .contains("control characters"));
    }
}

#[test]
fn authored_line_endings_and_unterminated_fences_preserve_literal_admission() {
    for (newline, value) in [
        ("\n", ""),
        ("\r\n", ""),
        ("\n", "\"'\r"),
        ("\r\n", "\"'\r"),
        ("\r\n", "\r\n\"'\r\n"),
    ] {
        let source = format!("{{row @value=````{newline}{value}{newline}```` @after=ok}}");
        let document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
            BytesSource::new(SourceId(31), source.as_bytes().to_vec()),
        )))
        .build();
        assert!(
            document.diagnostics.is_empty(),
            "{:?}",
            document.diagnostics
        );
        assert!(document.nodes.iter().any(|node| matches!(node,
            CemAstNode::Attribute { expanded_name, value: Some(actual), value_nodes, .. }
            if expanded_name.local_name == "value" && actual == value && value_nodes.is_empty())));
    }
    for source in [
        "{row @value=```\nunterminated",
        "{row @value=````\nshort\n```}",
        "{row @value=```\r\nwrong boundary\n```}",
    ] {
        let mut tokenizer =
            CemTokenizer::from_source(BytesSource::new(SourceId(31), source.as_bytes().to_vec()));
        while tokenizer.next_token().is_some() {}
        let diagnostics = tokenizer.take_diagnostics();
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.code
                == "cem.tokenizer.unterminated_string"
                && diagnostic.severity.is_hard_violation()),
            "{source}: {diagnostics:?}"
        );
    }
    let source = "{row @value=```inline```}";
    let document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(31), source.as_bytes().to_vec()),
    )))
    .build();
    assert!(document.diagnostics.is_empty());
    assert!(document.nodes.iter().any(|node| matches!(node,
        CemAstNode::Attribute { value: Some(value), .. } if value == "```inline```")));
}

#[test]
fn fenced_literal_highlighting_keeps_payload_in_one_native_string_span() {
    let value = "can't \"quote\" ```{#literal} {$never()} ``` Café 🧬\r\n";
    let encoded = transform_template_encode_cem_string_literal(value).unwrap();
    let prefix = "{row @value=";
    let source = format!("{prefix}{encoded} @after=ok | {{after}}}}");
    let result: serde_json::Value = serde_json::from_str(
        &cem_ml::api::source_highlight::highlight_source_to_html_v1_json(
            &serde_json::json!({"source": source, "contentType": "application/cem"}).to_string(),
        ),
    )
    .unwrap();
    assert_eq!(result["status"], "highlighted");
    assert_eq!(result["diagnostics"], serde_json::json!([]));
    assert!(result["spans"]
        .as_array()
        .unwrap()
        .iter()
        .any(|span| span["role"] == "syntax.string"
            && span["byteOffset"] == prefix.len()
            && span["byteLength"] == encoded.len()));
    assert!(result["html"].as_str().unwrap().contains("{<b>after</b>}"));
}
