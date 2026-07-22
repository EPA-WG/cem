//! Layer 2: surface parser and recovery.

pub mod module;
pub mod pratt;

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::source::ByteRange;

use crate::api::ParseResult;
use crate::diagnostics::{
    self as ql_diagnostics, DiagnosticCode, PARSE_ERROR, USE_RUST_BOOLEAN_OPS,
};
use crate::lexer::{CookedTokenPayload, Lexer, Token, TokenKind};
use pratt::{infix_operator, InfixOperator, PREC_CALL, PREC_DOT, PREC_TYPE, PREC_UNARY};

#[derive(Debug, Clone)]
pub struct Parser<'src> {
    pub source: &'src str,
    tokens: Vec<Token>,
    cursor: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'src> Parser<'src> {
    pub fn new(source: &'src str) -> Self {
        let mut diagnostics = Vec::new();
        let mut tokens = Vec::new();
        for token in Lexer::new(source).scan_all() {
            match token.kind {
                TokenKind::Whitespace | TokenKind::LineComment | TokenKind::BlockComment => {}
                TokenKind::Invalid => {
                    diagnostics.push(diagnostic(PARSE_ERROR, "invalid token", token.range))
                }
                _ => tokens.push(token),
            }
        }
        Self {
            source,
            tokens,
            cursor: 0,
            diagnostics,
        }
    }

    pub fn parse_module(mut self) -> ParseResult {
        let mut nodes = Vec::new();
        while !self.at(TokenKind::EndOfInput) {
            let before = self.cursor;
            if let Some(node) = self.parse_surface_node() {
                nodes.push(node);
                continue;
            }
            self.synchronize(before);
        }
        ParseResult {
            module: SurfaceModule {
                source: self.source.to_owned(),
                nodes,
            },
            diagnostics: self.diagnostics,
        }
    }

    fn parse_surface_node(&mut self) -> Option<SurfaceNode> {
        match self.current().kind {
            TokenKind::Module => self.parse_module_decl().map(SurfaceNode::Module),
            TokenKind::Import => self.parse_import_decl().map(SurfaceNode::Import),
            TokenKind::Declare => self.parse_declare(),
            _ => self.parse_expression(0).map(SurfaceNode::Expression),
        }
    }

    fn parse_module_decl(&mut self) -> Option<ModuleDecl> {
        let start = self.expect(TokenKind::Module, "`module`")?;
        let uri = self.expect_string("module URI")?;
        let range = join_ranges(start.range, uri.range);
        Some(ModuleDecl {
            uri: string_value(&uri),
            range,
        })
    }

    fn parse_import_decl(&mut self) -> Option<ImportDecl> {
        let start = self.expect(TokenKind::Import, "`import`")?;
        let uri = self.expect_string("import URI")?;
        let mut range = join_ranges(start.range, uri.range);
        let alias = if self.match_kind(TokenKind::As).is_some() {
            let alias = self.parse_qname()?;
            range = join_ranges(range, alias.range);
            Some(alias.local)
        } else {
            None
        };
        Some(ImportDecl {
            uri: string_value(&uri),
            alias,
            range,
        })
    }

    fn parse_declare(&mut self) -> Option<SurfaceNode> {
        let start = self.expect(TokenKind::Declare, "`declare`")?;
        if self.match_kind(TokenKind::Let).is_some() {
            let name = self.parse_qname()?;
            self.expect(TokenKind::Assign, "`=` after binding name")?;
            let value = self.parse_expression(0)?;
            let range = join_ranges(start.range, value.range());
            return Some(SurfaceNode::DeclareVariable(VariableDecl {
                name,
                value,
                range,
            }));
        }
        if self.match_kind(TokenKind::Function).is_some() {
            let name = self.parse_qname()?;
            self.expect(TokenKind::LParen, "`(` after function name")?;
            let params = self.parse_function_params()?;
            self.expect(TokenKind::RParen, "`)` after function parameters")?;
            let body = if self.match_kind(TokenKind::LBrace).is_some() {
                let body = self.parse_expression(0)?;
                self.expect(TokenKind::RBrace, "`}` after function body")?;
                body
            } else {
                self.parse_expression(0)?
            };
            let range = join_ranges(start.range, body.range());
            return Some(SurfaceNode::DeclareFunction(FunctionDecl {
                name,
                params,
                body,
                range,
            }));
        }
        if self.current().kind == TokenKind::XPathCompatWord
            && self.compat_word(self.current()) == Some("variable")
        {
            let token = self.bump();
            self.error_at(
                PARSE_ERROR,
                "use `declare let name = expression` instead of `declare variable name := expression`",
                token.range,
            );
            return None;
        }
        self.error_current("expected `let` or `function` after `declare`");
        None
    }

    fn parse_function_params(&mut self) -> Option<Vec<FunctionParam>> {
        let mut params = Vec::new();
        if self.at(TokenKind::RParen) {
            return Some(params);
        }
        loop {
            let name = self.parse_qname()?;
            let type_annotation = if self.match_kind(TokenKind::As).is_some() {
                Some(self.parse_type_expr()?)
            } else {
                None
            };
            params.push(FunctionParam {
                name,
                type_annotation,
            });
            if self.match_kind(TokenKind::Comma).is_none() {
                return Some(params);
            }
        }
    }

    fn parse_expression(&mut self, min_prec: u8) -> Option<Expression> {
        let mut lhs = self.parse_prefix()?;
        loop {
            if self.is_expression_boundary() {
                break;
            }
            if self.consume_compat_word() {
                continue;
            }
            if self.at(TokenKind::LParen) {
                if PREC_CALL < min_prec {
                    break;
                }
                lhs = self.parse_call(lhs)?;
                continue;
            }
            if self.at(TokenKind::Dot) {
                if PREC_DOT < min_prec {
                    break;
                }
                lhs = self.parse_pipeline(lhs)?;
                continue;
            }
            if matches!(self.current().kind, TokenKind::As | TokenKind::IsKw) {
                if PREC_TYPE < min_prec {
                    break;
                }
                lhs = self.parse_type_postfix(lhs)?;
                continue;
            }
            let Some((prec, op)) = infix_operator(self.current().kind) else {
                break;
            };
            if prec < min_prec {
                break;
            }
            let op_token = self.bump();
            let rhs = self.parse_expression(prec + 1)?;
            let range = join_ranges(lhs.range(), rhs.range());
            lhs = match op {
                InfixOperator::Binary(op) => Expression::BinaryOp {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    range,
                },
                InfixOperator::Set(op) => Expression::SetOp {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    range: join_ranges(op_token.range, range),
                },
            };
        }
        Some(lhs)
    }

    fn parse_prefix(&mut self) -> Option<Expression> {
        let token = self.bump();
        match token.kind {
            TokenKind::StringLit
            | TokenKind::IntLit
            | TokenKind::DecimalLit
            | TokenKind::DoubleLit
            | TokenKind::BoolLit
            | TokenKind::NullLit => Some(Expression::Literal(literal_value(&token), token.range)),
            TokenKind::Ident | TokenKind::PrefixedName => {
                Some(Expression::Name(qname_from_token(&token)?, token.range))
            }
            TokenKind::Dot => Some(Expression::LeadingDot(token.range)),
            TokenKind::Slash => {
                self.error_at(
                    PARSE_ERROR,
                    "XPath path syntax is not CEM-QL syntax; use dot pipelines or named axis helpers",
                    token.range,
                );
                None
            }
            TokenKind::LParen => self.parse_group_or_sequence(token),
            TokenKind::LBrace => self.parse_block_or_record(token),
            TokenKind::If => self.parse_if(token),
            TokenKind::Let => self.parse_let(token),
            TokenKind::For => self.parse_for(token),
            TokenKind::FnKw => self.parse_lambda(token),
            TokenKind::Bang => {
                let operand = self.parse_expression(PREC_UNARY)?;
                let range = join_ranges(token.range, operand.range());
                Some(Expression::UnaryOp {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    range,
                })
            }
            TokenKind::Minus => {
                let operand = self.parse_expression(PREC_UNARY)?;
                let range = join_ranges(token.range, operand.range());
                Some(Expression::UnaryOp {
                    op: UnaryOp::Negate,
                    operand: Box::new(operand),
                    range,
                })
            }
            TokenKind::Dollar => {
                self.error_at(PARSE_ERROR, dollar_name_message(), token.range);
                None
            }
            TokenKind::XPathCompatWord => {
                self.error_at(
                    self.compat_code(&token),
                    self.compat_message(&token),
                    token.range,
                );
                None
            }
            _ => {
                self.error_at(PARSE_ERROR, "expected expression", token.range);
                None
            }
        }
    }

    fn parse_if(&mut self, start: Token) -> Option<Expression> {
        let cond = self.parse_expression(0)?;
        let then_branch = self.parse_braced_expression("if branch")?;
        self.expect(TokenKind::Else, "`else` after then branch")?;
        let else_branch = self.parse_braced_expression("else branch")?;
        let range = join_ranges(start.range, else_branch.range());
        Some(Expression::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
            range,
        })
    }

    fn parse_let(&mut self, start: Token) -> Option<Expression> {
        let name = self.parse_qname()?;
        self.expect(TokenKind::Assign, "`=` after let binding")?;
        let value = self.parse_expression(0)?;
        self.expect(TokenKind::Semicolon, "`;` after let binding value")?;
        let body = self.parse_expression(0)?;
        let range = join_ranges(start.range, body.range());
        Some(Expression::Let {
            name,
            value: Box::new(value),
            body: Box::new(body),
            range,
        })
    }

    fn parse_for(&mut self, start: Token) -> Option<Expression> {
        let var = self.parse_qname()?;
        self.expect(TokenKind::In, "`in` after for variable")?;
        let source = self.parse_expression(0)?;
        let body = self.parse_braced_expression("for body")?;
        let range = join_ranges(start.range, body.range());
        Some(Expression::For {
            var,
            source: Box::new(source),
            body: Box::new(body),
            range,
        })
    }

    fn parse_lambda(&mut self, start: Token) -> Option<Expression> {
        self.expect(TokenKind::LParen, "`(` after `fn`")?;
        let params = self.parse_function_params()?;
        self.expect(TokenKind::RParen, "`)` after lambda parameters")?;
        self.expect(TokenKind::FatArrow, "`=>` after lambda parameters")?;
        let body = self.parse_expression(0)?;
        let range = join_ranges(start.range, body.range());
        Some(Expression::Lambda {
            params,
            body: Box::new(body),
            range,
        })
    }

    fn parse_group_or_sequence(&mut self, start: Token) -> Option<Expression> {
        if let Some(end) = self.match_kind(TokenKind::RParen) {
            return Some(Expression::Sequence {
                items: Vec::new(),
                range: join_ranges(start.range, end.range),
            });
        }
        let first = self.parse_expression(0)?;
        if self.match_kind(TokenKind::Comma).is_none() {
            self.expect(TokenKind::RParen, "`)` after grouped expression")?;
            return Some(first);
        }
        let mut items = vec![first];
        while !self.at(TokenKind::RParen) && !self.at(TokenKind::EndOfInput) {
            items.push(self.parse_expression(0)?);
            if self.match_kind(TokenKind::Comma).is_none() {
                break;
            }
        }
        let end = self.expect(TokenKind::RParen, "`)` after sequence")?;
        Some(Expression::Sequence {
            items,
            range: join_ranges(start.range, end.range),
        })
    }

    fn parse_braced_expression(&mut self, label: &'static str) -> Option<Expression> {
        self.expect(TokenKind::LBrace, "`{` before expression block")?;
        let expr = self.parse_expression(0)?;
        self.expect(TokenKind::RBrace, "`}` after expression block")?;
        if expr.range().len == 0 {
            self.error_current(format!("expected {label}"));
        }
        Some(expr)
    }

    fn parse_block_or_record(&mut self, start: Token) -> Option<Expression> {
        if self.at(TokenKind::Let) {
            return self.parse_block(start);
        }
        self.parse_record(start)
    }

    fn parse_block(&mut self, start: Token) -> Option<Expression> {
        let mut bindings = Vec::new();
        while self.match_kind(TokenKind::Let).is_some() {
            let name = self.parse_qname()?;
            self.expect(TokenKind::Assign, "`=` after let binding")?;
            let value = self.parse_expression(0)?;
            self.expect(TokenKind::Semicolon, "`;` after let binding value")?;
            bindings.push((name, value));
        }
        let mut body = self.parse_expression(0)?;
        let end = self.expect(TokenKind::RBrace, "`}` after block expression")?;
        while let Some((name, value)) = bindings.pop() {
            body = Expression::Let {
                range: join_ranges(start.range, end.range),
                name,
                value: Box::new(value),
                body: Box::new(body),
            };
        }
        Some(body)
    }

    fn parse_record(&mut self, start: Token) -> Option<Expression> {
        let mut entries = Vec::new();
        if let Some(end) = self.match_kind(TokenKind::RBrace) {
            return Some(Expression::Record {
                entries,
                range: join_ranges(start.range, end.range),
            });
        }
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::EndOfInput) {
            let key = self.parse_record_key()?;
            self.expect(TokenKind::Colon, "`:` after record key")?;
            let value = self.parse_expression(0)?;
            let range = join_ranges(key.range(), value.range());
            entries.push(RecordEntry { key, value, range });
            if self.match_kind(TokenKind::Comma).is_none() {
                break;
            }
        }
        let end = self.expect(TokenKind::RBrace, "`}` after record literal")?;
        Some(Expression::Record {
            entries,
            range: join_ranges(start.range, end.range),
        })
    }

    fn parse_record_key(&mut self) -> Option<RecordKey> {
        let token = self.bump();
        match token.kind {
            TokenKind::Ident | TokenKind::PrefixedName => {
                let name = qname_from_token(&token)?;
                let key = if let Some(prefix) = name.prefix {
                    format!("{prefix}:{}", name.local)
                } else {
                    name.local
                };
                Some(RecordKey::Static {
                    value: key,
                    range: token.range,
                })
            }
            TokenKind::StringLit => Some(RecordKey::Static {
                value: string_value(&token),
                range: token.range,
            }),
            TokenKind::LBracket => {
                let expr = self.parse_expression(0)?;
                let close = self.expect(TokenKind::RBracket, "`]` after computed record key")?;
                Some(RecordKey::Computed {
                    range: join_ranges(token.range, close.range),
                    expr: Box::new(expr),
                })
            }
            _ => {
                self.error_at(
                    PARSE_ERROR,
                    "expected bare identifier, quoted string, or `[expr]` record key",
                    token.range,
                );
                None
            }
        }
    }

    fn parse_call(&mut self, callee: Expression) -> Option<Expression> {
        self.expect(TokenKind::LParen, "`(` after callee")?;
        let args = self.parse_arguments(TokenKind::RParen)?;
        let close = self.expect(TokenKind::RParen, "`)` after call arguments")?;
        let range = join_ranges(callee.range(), close.range);
        if let Expression::Name(name, _) = &callee {
            if name.prefix.is_none() && name.local == "treat_as" {
                if args.len() == 2 {
                    if let Expression::Name(type_name, type_range) = &args[1] {
                        return Some(Expression::TreatAs {
                            range,
                            value: Box::new(args[0].clone()),
                            ty: TypeExpr {
                                name: type_name.clone(),
                                range: *type_range,
                            },
                        });
                    }
                    self.error_at(
                        PARSE_ERROR,
                        "expected type name as second `treat_as` argument",
                        args[1].range(),
                    );
                } else {
                    self.error_at(
                        PARSE_ERROR,
                        "`treat_as` expects exactly two arguments: `treat_as(expr, Type)`",
                        range,
                    );
                }
            }
            if name.prefix.is_none() && name.local == "same_node" {
                if args.len() == 2 {
                    return Some(Expression::BinaryOp {
                        op: BinaryOp::Is,
                        lhs: Box::new(args[0].clone()),
                        rhs: Box::new(args[1].clone()),
                        range,
                    });
                }
                self.error_at(
                    PARSE_ERROR,
                    "`same_node` expects exactly two arguments: `same_node(a, b)`",
                    range,
                );
            }
        }
        Some(Expression::Call {
            range,
            callee: Box::new(callee),
            args,
        })
    }

    fn parse_arguments(&mut self, close: TokenKind) -> Option<Vec<Expression>> {
        let mut args = Vec::new();
        if self.at(close) {
            return Some(args);
        }
        while !self.at(close) && !self.at(TokenKind::EndOfInput) {
            args.push(self.parse_expression(0)?);
            if self.match_kind(TokenKind::Comma).is_none() {
                break;
            }
        }
        Some(args)
    }

    fn parse_pipeline(&mut self, source: Expression) -> Option<Expression> {
        let dot = self.expect(TokenKind::Dot, "`.` before pipeline step")?;
        let step = if self.match_kind(TokenKind::LBrace).is_some() {
            let lambda = self.parse_expression(0)?;
            let close = self.expect(TokenKind::RBrace, "`}` after pipeline lambda")?;
            PipelineStep::Lambda {
                range: join_ranges(dot.range, close.range),
                lambda,
            }
        } else {
            let name = self.parse_qname()?;
            let mut range = join_ranges(dot.range, name.range);
            let args = if self.match_kind(TokenKind::LParen).is_some() {
                let args = self.parse_arguments(TokenKind::RParen)?;
                let close = self.expect(TokenKind::RParen, "`)` after pipeline step arguments")?;
                range = join_ranges(dot.range, close.range);
                args
            } else {
                Vec::new()
            };
            PipelineStep::Named { name, args, range }
        };
        let range = join_ranges(source.range(), step.range());
        match source {
            Expression::Pipeline {
                source,
                mut steps,
                range: original,
            } => {
                steps.push(step);
                Some(Expression::Pipeline {
                    source,
                    steps,
                    range: join_ranges(original, range),
                })
            }
            other => Some(Expression::Pipeline {
                source: Box::new(other),
                steps: vec![step],
                range,
            }),
        }
    }

    fn parse_type_postfix(&mut self, value: Expression) -> Option<Expression> {
        let token = self.bump();
        match token.kind {
            TokenKind::IsKw => {
                let ty = self.parse_type_expr()?;
                let range = join_ranges(value.range(), ty.range);
                Some(Expression::InstanceOf {
                    value: Box::new(value),
                    ty,
                    range,
                })
            }
            TokenKind::As => {
                let ty = self.parse_type_expr()?;
                let range = join_ranges(value.range(), ty.range);
                Some(Expression::CastAs {
                    value: Box::new(value),
                    ty,
                    range,
                })
            }
            _ => None,
        }
    }

    fn parse_qname(&mut self) -> Option<QName> {
        let token = self.bump();
        match token.kind {
            TokenKind::Ident | TokenKind::PrefixedName => qname_from_token(&token),
            TokenKind::Dollar => {
                let range = if matches!(
                    self.current().kind,
                    TokenKind::Ident | TokenKind::PrefixedName
                ) {
                    join_ranges(token.range, self.bump().range)
                } else {
                    token.range
                };
                self.error_at(PARSE_ERROR, dollar_name_message(), range);
                None
            }
            _ => {
                self.error_at(PARSE_ERROR, "expected name", token.range);
                None
            }
        }
    }

    fn parse_type_expr(&mut self) -> Option<TypeExpr> {
        let name = self.parse_qname()?;
        Some(TypeExpr {
            range: name.range,
            name,
        })
    }

    fn consume_compat_word(&mut self) -> bool {
        if self.current().kind != TokenKind::XPathCompatWord {
            return false;
        }
        let token = self.bump();
        self.error_at(
            self.compat_code(&token),
            self.compat_message(&token),
            token.range,
        );
        if !self.is_expression_boundary() {
            let _ = self.parse_expression(pratt::PREC_AND + 1);
        }
        true
    }

    fn compat_word<'tok>(&self, token: &'tok Token) -> Option<&'tok str> {
        match &token.cooked {
            Some(CookedTokenPayload::Name(word)) => Some(word.as_str()),
            _ => None,
        }
    }

    fn compat_code(&self, token: &Token) -> DiagnosticCode {
        match self.compat_word(token) {
            Some("and" | "or" | "not") => USE_RUST_BOOLEAN_OPS,
            _ => PARSE_ERROR,
        }
    }

    fn compat_message(&self, token: &Token) -> String {
        match self.compat_word(token) {
            Some("and") => "use `&&` instead of XPath `and`".to_owned(),
            Some("or") => "use `||` instead of XPath `or`".to_owned(),
            Some("not") => "use prefix `!` instead of XPath `not(...)`".to_owned(),
            Some("eq") => "use `==` instead of XPath `eq`".to_owned(),
            Some("ne") => "use `!=` instead of XPath `ne`".to_owned(),
            Some("lt") => "use `<` instead of XPath `lt`".to_owned(),
            Some("le") => "use `<=` instead of XPath `le`".to_owned(),
            Some("gt") => "use `>` instead of XPath `gt`".to_owned(),
            Some("ge") => "use `>=` instead of XPath `ge`".to_owned(),
            Some("div") => "use `/` instead of XPath `div`".to_owned(),
            Some("mod") => "use `%` instead of XPath `mod`".to_owned(),
            Some("then") => "use `if condition { then_expr } else { else_expr }`".to_owned(),
            Some("return") => "use `for name in stream { expr }`".to_owned(),
            Some("some") => "use `any(stream, fn)` instead of `some ... satisfies`".to_owned(),
            Some("every") => "use `all(stream, fn)` instead of `every ... satisfies`".to_owned(),
            Some("satisfies") => "use `any(stream, fn)` or `all(stream, fn)`".to_owned(),
            Some("variable") => "use `declare let name = expression`".to_owned(),
            Some("instance") => "use `expr is Type` instead of `instance of`".to_owned(),
            Some("of") => "use `expr is Type` instead of `instance of`".to_owned(),
            Some("cast") => "use `expr as Type` instead of `cast as`".to_owned(),
            Some("treat") => "use `treat_as(expr, Type)` instead of `treat as`".to_owned(),
            Some(word) => format!("`{word}` is not canonical CEM-QL syntax"),
            None => "non-canonical CEM-QL syntax".to_owned(),
        }
    }

    fn synchronize(&mut self, before: usize) {
        let start = self.current().range;
        while !self.at(TokenKind::EndOfInput)
            && !self.is_top_level_anchor()
            && !matches!(
                self.current().kind,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace
            )
            && !self.pipeline_boundary()
        {
            self.bump();
        }
        if self.cursor == before && !self.at(TokenKind::EndOfInput) {
            self.bump();
        } else if self.cursor > before {
            let end = self.previous().range;
            self.diagnostics.push(diagnostic(
                PARSE_ERROR,
                "recovered at parser synchronization point",
                join_ranges(start, end),
            ));
        }
    }

    fn is_top_level_anchor(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Module | TokenKind::Import | TokenKind::Declare
        )
    }

    fn pipeline_boundary(&self) -> bool {
        self.current().kind == TokenKind::Dot
            && self.tokens.get(self.cursor + 1).is_some_and(|token| {
                matches!(token.kind, TokenKind::Ident | TokenKind::PrefixedName)
            })
    }

    fn is_expression_boundary(&self) -> bool {
        if self.current().kind == TokenKind::XPathCompatWord
            && matches!(
                self.compat_word(self.current()),
                Some("then" | "return" | "satisfies")
            )
        {
            return true;
        }
        matches!(
            self.current().kind,
            TokenKind::EndOfInput
                | TokenKind::RParen
                | TokenKind::RBracket
                | TokenKind::RBrace
                | TokenKind::LBrace
                | TokenKind::Comma
                | TokenKind::Semicolon
                | TokenKind::Else
                | TokenKind::In
                | TokenKind::Module
                | TokenKind::Import
                | TokenKind::Declare
        )
    }

    fn expect_string(&mut self, label: &'static str) -> Option<Token> {
        let token = self.bump();
        if token.kind == TokenKind::StringLit {
            Some(token)
        } else {
            self.error_at(PARSE_ERROR, format!("expected {label}"), token.range);
            None
        }
    }

    fn expect(&mut self, kind: TokenKind, label: &'static str) -> Option<Token> {
        let token = self.bump();
        if token.kind == kind {
            Some(token)
        } else {
            self.error_at(PARSE_ERROR, format!("expected {label}"), token.range);
            None
        }
    }

    fn match_kind(&mut self, kind: TokenKind) -> Option<Token> {
        if self.at(kind) {
            Some(self.bump())
        } else {
            None
        }
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current().kind == kind
    }

    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.cursor.saturating_sub(1)]
    }

    fn bump(&mut self) -> Token {
        let token = self.current().clone();
        if token.kind != TokenKind::EndOfInput {
            self.cursor += 1;
        }
        token
    }

    fn error_current(&mut self, message: impl Into<String>) {
        let range = self.current().range;
        self.error_at(PARSE_ERROR, message, range);
    }

    fn error_at(&mut self, code: DiagnosticCode, message: impl Into<String>, range: ByteRange) {
        self.diagnostics.push(diagnostic(code, message, range));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceModule {
    pub source: String,
    pub nodes: Vec<SurfaceNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceNode {
    Module(ModuleDecl),
    Import(ImportDecl),
    DeclareVariable(VariableDecl),
    DeclareFunction(FunctionDecl),
    Expression(Expression),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDecl {
    pub uri: String,
    pub range: ByteRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDecl {
    pub uri: String,
    pub alias: Option<String>,
    pub range: ByteRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDecl {
    pub name: QName,
    pub value: Expression,
    pub range: ByteRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: QName,
    pub params: Vec<FunctionParam>,
    pub body: Expression,
    pub range: ByteRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionParam {
    pub name: QName,
    pub type_annotation: Option<TypeExpr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(LiteralValue, ByteRange),
    Name(QName, ByteRange),
    LeadingDot(ByteRange),
    Path {
        steps: Vec<PathStep>,
        range: ByteRange,
    },
    Pipeline {
        source: Box<Expression>,
        steps: Vec<PipelineStep>,
        range: ByteRange,
    },
    BinaryOp {
        op: BinaryOp,
        lhs: Box<Expression>,
        rhs: Box<Expression>,
        range: ByteRange,
    },
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expression>,
        range: ByteRange,
    },
    SetOp {
        op: SetOp,
        lhs: Box<Expression>,
        rhs: Box<Expression>,
        range: ByteRange,
    },
    If {
        cond: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Box<Expression>,
        range: ByteRange,
    },
    Let {
        name: QName,
        value: Box<Expression>,
        body: Box<Expression>,
        range: ByteRange,
    },
    For {
        var: QName,
        source: Box<Expression>,
        body: Box<Expression>,
        range: ByteRange,
    },
    Quantified {
        kind: QuantifierKind,
        var: QName,
        source: Box<Expression>,
        predicate: Box<Expression>,
        range: ByteRange,
    },
    Record {
        entries: Vec<RecordEntry>,
        range: ByteRange,
    },
    Sequence {
        items: Vec<Expression>,
        range: ByteRange,
    },
    Call {
        callee: Box<Expression>,
        args: Vec<Expression>,
        range: ByteRange,
    },
    Lambda {
        params: Vec<FunctionParam>,
        body: Box<Expression>,
        range: ByteRange,
    },
    InstanceOf {
        value: Box<Expression>,
        ty: TypeExpr,
        range: ByteRange,
    },
    CastAs {
        value: Box<Expression>,
        ty: TypeExpr,
        range: ByteRange,
    },
    TreatAs {
        value: Box<Expression>,
        ty: TypeExpr,
        range: ByteRange,
    },
}

impl Expression {
    pub fn range(&self) -> ByteRange {
        match self {
            Expression::Literal(_, range)
            | Expression::Name(_, range)
            | Expression::LeadingDot(range)
            | Expression::Path { range, .. }
            | Expression::Pipeline { range, .. }
            | Expression::BinaryOp { range, .. }
            | Expression::UnaryOp { range, .. }
            | Expression::SetOp { range, .. }
            | Expression::If { range, .. }
            | Expression::Let { range, .. }
            | Expression::For { range, .. }
            | Expression::Quantified { range, .. }
            | Expression::Record { range, .. }
            | Expression::Sequence { range, .. }
            | Expression::Call { range, .. }
            | Expression::Lambda { range, .. }
            | Expression::InstanceOf { range, .. }
            | Expression::CastAs { range, .. }
            | Expression::TreatAs { range, .. } => *range,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordEntry {
    pub key: RecordKey,
    pub value: Expression,
    pub range: ByteRange,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecordKey {
    Static {
        value: String,
        range: ByteRange,
    },
    Computed {
        expr: Box<Expression>,
        range: ByteRange,
    },
}

impl RecordKey {
    pub fn range(&self) -> ByteRange {
        match self {
            RecordKey::Static { range, .. } | RecordKey::Computed { range, .. } => *range,
        }
    }

    pub fn static_value(&self) -> Option<&str> {
        match self {
            RecordKey::Static { value, .. } => Some(value),
            RecordKey::Computed { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    String(String),
    Integer(i64),
    Decimal(String),
    Double(f64),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PathStep {
    Axis {
        axis: Axis,
        name_test: NameTest,
        predicates: Vec<Expression>,
        range: ByteRange,
    },
    Parent(ByteRange),
    Self_(ByteRange),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PipelineStep {
    Named {
        name: QName,
        args: Vec<Expression>,
        range: ByteRange,
    },
    Lambda {
        lambda: Expression,
        range: ByteRange,
    },
}

impl PipelineStep {
    fn range(&self) -> ByteRange {
        match self {
            PipelineStep::Named { range, .. } | PipelineStep::Lambda { range, .. } => *range,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOp {
    Union,
    Intersect,
    Difference,
    SymmetricDifference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    EqEq,
    BangEq,
    Lt,
    Le,
    Gt,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    And,
    Or,
    /// `??` — null/empty-sequence coalescing: the left operand unless it is empty
    /// or its first item is `null`, otherwise the right operand.
    Coalesce,
    Is,
}

impl BinaryOp {
    pub fn symbol(self) -> &'static str {
        match self {
            BinaryOp::EqEq => "==",
            BinaryOp::BangEq => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::Plus => "+",
            BinaryOp::Minus => "-",
            BinaryOp::Star => "*",
            BinaryOp::Slash => "/",
            BinaryOp::Percent => "%",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
            BinaryOp::Coalesce => "??",
            BinaryOp::Is => "is",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantifierKind {
    Some,
    Every,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QName {
    pub prefix: Option<String>,
    pub local: String,
    pub range: ByteRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameTest {
    pub prefix: Option<String>,
    pub local: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Self_,
    Child,
    Parent,
    Descendants,
    DescendantsOrSelf,
    Ancestors,
    AncestorsOrSelf,
    FollowingSibling,
    PrecedingSibling,
    Attributes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeExpr {
    pub name: QName,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub diagnostic: Diagnostic,
}

fn dollar_name_message() -> &'static str {
    "use bare CEM-QL names without the XPath `$` variable prefix"
}

fn qname_from_token(token: &Token) -> Option<QName> {
    match &token.cooked {
        Some(CookedTokenPayload::Name(local)) => Some(QName {
            prefix: None,
            local: local.clone(),
            range: token.range,
        }),
        Some(CookedTokenPayload::PrefixedName { prefix, local }) => Some(QName {
            prefix: Some(prefix.clone()),
            local: local.clone(),
            range: token.range,
        }),
        _ => None,
    }
}

fn literal_value(token: &Token) -> LiteralValue {
    match &token.cooked {
        Some(CookedTokenPayload::StringValue(value)) => LiteralValue::String(value.clone()),
        Some(CookedTokenPayload::IntValue(value)) => LiteralValue::Integer(*value),
        Some(CookedTokenPayload::DecimalValue(value)) => LiteralValue::Decimal(value.clone()),
        Some(CookedTokenPayload::DoubleValue(value)) => LiteralValue::Double(*value),
        Some(CookedTokenPayload::BoolValue(value)) => LiteralValue::Boolean(*value),
        _ => LiteralValue::Null,
    }
}

fn string_value(token: &Token) -> String {
    match &token.cooked {
        Some(CookedTokenPayload::StringValue(value)) => value.clone(),
        _ => String::new(),
    }
}

fn join_ranges(a: ByteRange, b: ByteRange) -> ByteRange {
    let start = a.start.min(b.start);
    let end = a.end().max(b.end());
    ByteRange::new(start, (end - start).try_into().unwrap_or(u32::MAX))
}

fn diagnostic(code: DiagnosticCode, message: impl Into<String>, range: ByteRange) -> Diagnostic {
    ql_diagnostics::spanned(code, message, range, Severity::Error)
}
