use cem_ql::api::parse;
use cem_ql::lexer::{CookedTokenPayload, Lexer, TokenKind};
use cem_ql::parser::{
    BinaryOp, Expression, LiteralValue, PipelineStep, QuantifierKind, SurfaceNode,
};

fn kinds(source: &str) -> Vec<TokenKind> {
    Lexer::new(source)
        .scan_all()
        .into_iter()
        .filter(|token| token.kind != TokenKind::Whitespace)
        .map(|token| token.kind)
        .collect()
}

#[test]
fn lexer_recognises_tier_a_keywords_and_reserved_boolean_forms() {
    assert_eq!(
        kinds("let if then else for return some every satisfies import as declare variable function module instance of cast treat is fn"),
        vec![
            TokenKind::Let,
            TokenKind::If,
            TokenKind::Then,
            TokenKind::Else,
            TokenKind::For,
            TokenKind::ReturnKw,
            TokenKind::Some,
            TokenKind::Every,
            TokenKind::Satisfies,
            TokenKind::Import,
            TokenKind::As,
            TokenKind::Declare,
            TokenKind::Variable,
            TokenKind::Function,
            TokenKind::Module,
            TokenKind::InstanceKw,
            TokenKind::OfKw,
            TokenKind::CastKw,
            TokenKind::TreatKw,
            TokenKind::IsKw,
            TokenKind::FnKw,
            TokenKind::EndOfInput,
        ]
    );
    assert_eq!(
        kinds("a && b || c and d or not e"),
        vec![
            TokenKind::Ident,
            TokenKind::AmpAmpReserved,
            TokenKind::Ident,
            TokenKind::PipePipeReserved,
            TokenKind::Ident,
            TokenKind::AndKw,
            TokenKind::Ident,
            TokenKind::OrKw,
            TokenKind::NotKw,
            TokenKind::Ident,
            TokenKind::EndOfInput,
        ]
    );
}

#[test]
fn lexer_cooks_names_literals_and_numbers() {
    let tokens = Lexer::new(r#"str:length "a\n\u{41}" 42 3.14 6.02e23 true null"#).scan_all();
    assert_eq!(tokens[0].kind, TokenKind::PrefixedName);
    assert_eq!(
        tokens[0].cooked,
        Some(CookedTokenPayload::PrefixedName {
            prefix: "str".to_owned(),
            local: "length".to_owned(),
        })
    );
    assert_eq!(tokens[2].kind, TokenKind::StringLit);
    assert_eq!(
        tokens[2].cooked,
        Some(CookedTokenPayload::StringValue("a\nA".to_owned()))
    );
    assert_eq!(tokens[4].cooked, Some(CookedTokenPayload::IntValue(42)));
    assert_eq!(
        tokens[6].cooked,
        Some(CookedTokenPayload::DecimalValue("3.14".to_owned()))
    );
    assert_eq!(
        tokens[8].cooked,
        Some(CookedTokenPayload::DoubleValue(6.02e23))
    );
    assert_eq!(tokens[10].cooked, Some(CookedTokenPayload::BoolValue(true)));
    assert_eq!(tokens[12].kind, TokenKind::NullLit);
}

#[test]
fn lexer_tracks_byte_ranges_for_trivia_comments_and_punctuation() {
    let tokens = Lexer::new("a;; comment\n(* block *) .. := :: != <= >").scan_all();
    assert_eq!(tokens[0].range.start, 0);
    assert_eq!(tokens[0].range.len, 1);
    assert_eq!(tokens[1].kind, TokenKind::LineComment);
    assert_eq!(tokens[1].range.start, 1);
    assert_eq!(tokens[1].range.len, 10);
    assert_eq!(tokens[3].kind, TokenKind::BlockComment);
    assert_eq!(
        kinds(".. := :: != <= >"),
        vec![
            TokenKind::DotDot,
            TokenKind::Assign,
            TokenKind::ColonColon,
            TokenKind::NeqOp,
            TokenKind::Le,
            TokenKind::Gt,
            TokenKind::EndOfInput,
        ]
    );
}

#[test]
fn lexer_returns_invalid_tokens_for_unclosed_or_malformed_spans() {
    assert_eq!(
        kinds("\"unterminated"),
        vec![TokenKind::Invalid, TokenKind::EndOfInput]
    );
    assert_eq!(
        kinds("1e+"),
        vec![TokenKind::Invalid, TokenKind::EndOfInput]
    );
    assert_eq!(
        kinds("(* unterminated"),
        vec![TokenKind::Invalid, TokenKind::EndOfInput]
    );
}

fn first_expr(source: &str) -> Expression {
    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    match result.module.nodes.into_iter().next().expect("node") {
        SurfaceNode::Expression(expr) => expr,
        other => panic!("expected expression, got {other:?}"),
    }
}

#[test]
fn parser_recognises_module_import_and_declarations() {
    let result = parse(
        r#"module "urn:demo"
           import "cem:stdlib/strings" as str
           declare variable answer := 42
           declare function local:greet(name as string) { str:concat("hi", name) }"#,
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.module.nodes.len(), 4);
    assert!(matches!(
        &result.module.nodes[0],
        SurfaceNode::Module(module) if module.uri == "urn:demo"
    ));
    assert!(matches!(
        &result.module.nodes[1],
        SurfaceNode::Import(import) if import.uri == "cem:stdlib/strings" && import.alias.as_deref() == Some("str")
    ));
    assert!(matches!(
        &result.module.nodes[2],
        SurfaceNode::DeclareVariable(var)
            if var.name.local == "answer"
                && matches!(var.value, Expression::Literal(LiteralValue::Integer(42), _))
    ));
    assert!(matches!(
        &result.module.nodes[3],
        SurfaceNode::DeclareFunction(fun)
            if fun.name.prefix.as_deref() == Some("local")
                && fun.name.local == "greet"
                && fun.params.len() == 1
    ));
}

#[test]
fn parser_applies_pratt_precedence_for_boolean_and_arithmetic_ops() {
    let expr = first_expr("a or b and c + d * e");
    let Expression::BinaryOp {
        op: BinaryOp::Or,
        rhs,
        ..
    } = expr
    else {
        panic!("expected top-level or expression");
    };
    let Expression::BinaryOp {
        op: BinaryOp::And,
        rhs,
        ..
    } = *rhs
    else {
        panic!("expected and to bind under or");
    };
    let Expression::BinaryOp {
        op: BinaryOp::Plus,
        rhs,
        ..
    } = *rhs
    else {
        panic!("expected plus to bind under and");
    };
    assert!(matches!(
        *rhs,
        Expression::BinaryOp {
            op: BinaryOp::Star,
            ..
        }
    ));
}

#[test]
fn parser_builds_pipeline_call_and_path_shapes() {
    let expr = first_expr(r#"items.filter(true).map("x")"#);
    let Expression::Pipeline { source, steps, .. } = expr else {
        panic!("expected pipeline");
    };
    assert!(matches!(*source, Expression::Name(_, _)));
    assert_eq!(steps.len(), 2);
    assert!(matches!(
        &steps[0],
        PipelineStep::Named { name, args, .. } if name.local == "filter" && args.len() == 1
    ));
    assert!(matches!(
        &steps[1],
        PipelineStep::Named { name, args, .. } if name.local == "map" && args.len() == 1
    ));

    let path = first_expr("root/child[true]/..");
    assert!(matches!(path, Expression::Path { ref steps, .. } if steps.len() == 3));
}

#[test]
fn parser_builds_control_flow_record_sequence_and_quantified_forms() {
    assert!(matches!(
        first_expr(r#"if ok then {"name": "x"} else (1, 2)"#),
        Expression::If { .. }
    ));
    assert!(matches!(
        first_expr("let item := source in item"),
        Expression::Let { .. }
    ));
    assert!(matches!(
        first_expr("for item in source return item"),
        Expression::For { .. }
    ));
    assert!(matches!(
        first_expr("some item in source satisfies true"),
        Expression::Quantified {
            kind: QuantifierKind::Some,
            ..
        }
    ));
}

#[test]
fn parser_recovers_to_top_level_anchors_after_errors() {
    let result = parse(r#"declare variable := 1 import "cem:stdlib/strings" as str"#);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.ql.parse_error"),
        "{:?}",
        result.diagnostics
    );
    assert!(matches!(
        result.module.nodes.last(),
        Some(SurfaceNode::Import(import)) if import.alias.as_deref() == Some("str")
    ));
}

#[test]
fn parser_reports_reserved_c_family_boolean_operators() {
    let result = parse("a && b || c");
    let use_and_or = result
        .diagnostics
        .iter()
        .filter(|diag| diag.code == "cem.ql.use_and_or")
        .count();
    assert_eq!(use_and_or, 2, "{:?}", result.diagnostics);
}
