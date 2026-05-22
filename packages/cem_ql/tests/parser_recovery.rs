use cem_ql::lexer::{CookedTokenPayload, Lexer, TokenKind};

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
