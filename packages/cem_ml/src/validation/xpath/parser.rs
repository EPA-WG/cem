use super::lexer::{XPathLexicalToken, XPathLexicalTokenKind};
use super::{
    XPathArrayConstructor, XPathAttachment, XPathAxis, XPathBinaryOperator, XPathExpression,
    XPathExpressionNode, XPathExpressionSequence, XPathKindTest, XPathLiteral, XPathLiteralKind,
    XPathMapConstructorEntry, XPathName, XPathNameTest, XPathNodeTest, XPathPathExpression,
    XPathPathRoot, XPathPostfixExpression, XPathPrimaryExpression, XPathQuantifier,
    XPathSourceRange, XPathSourceRangeResolver, XPathStep, XPathStepNode, XPathSyntaxAst,
    XPathUnaryOperator,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum XPathParseErrorKind {
    Syntax,
    UnknownNamespacePrefix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct XPathParseError {
    pub(super) kind: XPathParseErrorKind,
    pub(super) expected: Vec<String>,
    pub(super) found: Option<String>,
    pub(super) namespace_prefix: Option<String>,
    pub(super) token_index: Option<usize>,
    pub(super) start: usize,
    pub(super) end: usize,
}

impl XPathParseError {
    pub(super) fn message(&self) -> String {
        match self.kind {
            XPathParseErrorKind::UnknownNamespacePrefix => format!(
                "XPath namespace prefix `{}` is not declared in the static context",
                self.namespace_prefix.as_deref().unwrap_or_default()
            ),
            XPathParseErrorKind::Syntax => {
                let expected = self.expected.join(" or ");
                match self.found.as_deref() {
                    Some(found) => format!("expected {expected}, found `{found}`"),
                    None => format!("expected {expected}, found end of XPath expression"),
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum XPathNameUse {
    Element,
    Attribute,
    Function,
    Variable,
}

enum XPathArrowFunctionSpecifier {
    Static(XPathName),
    Dynamic(XPathPrimaryExpression),
}

/// Parses an already scanned XPath token stream. Source text is accepted only
/// for its byte length; this parser never tokenizes or slices it.
pub(super) fn parse_xpath(
    source: &str,
    tokens: &[XPathLexicalToken<'_>],
    range_resolver: &XPathSourceRangeResolver,
    attachment: &XPathAttachment,
) -> Result<XPathSyntaxAst, XPathParseError> {
    XPathParser {
        tokens,
        raw_index: 0,
        source_len: source.len(),
        range_resolver,
        attachment,
    }
    .parse()
}

struct XPathParser<'tokens, 'source, 'context> {
    tokens: &'tokens [XPathLexicalToken<'source>],
    raw_index: usize,
    source_len: usize,
    range_resolver: &'context XPathSourceRangeResolver,
    attachment: &'context XPathAttachment,
}

impl<'tokens, 'source, 'context> XPathParser<'tokens, 'source, 'context> {
    fn parse(mut self) -> Result<XPathSyntaxAst, XPathParseError> {
        let root = self.parse_expression_sequence()?;
        if let Some((token_index, token)) = self.peek() {
            return Err(self.syntax_error_at(token_index, token, &["end of expression"]));
        }
        Ok(XPathSyntaxAst::new(root))
    }

    fn parse_expression_sequence(&mut self) -> Result<XPathExpressionSequence, XPathParseError> {
        let first = self.parse_expression_single()?;
        let start = self.node_start(&first);
        let mut end = self.node_end(&first);
        let mut expressions = vec![first];
        while self.consume_if(",").is_some() {
            let expression = self.parse_expression_single()?;
            end = self.node_end(&expression);
            expressions.push(expression);
        }
        Ok(XPathExpressionSequence {
            expressions,
            source_range: self.range(start, end),
        })
    }

    fn parse_expression_single(&mut self) -> Result<XPathExpressionNode, XPathParseError> {
        let Some((_, token)) = self.peek() else {
            return Err(self.syntax_error(&["expression"]));
        };
        match token.lexeme {
            "for" => self.parse_for_expression(),
            "let" => self.parse_let_expression(),
            "if" => self.parse_if_expression(),
            "some" | "every" => self.parse_quantified_expression(),
            "switch" => self.parse_unsupported_expression("switch-expression"),
            "typeswitch" => self.parse_unsupported_expression("typeswitch-expression"),
            _ => {
                let start = token.start;
                let expression = self.parse_binary_expression(1)?;
                if let Some((_, suffix)) = self.peek() {
                    if let Some(production) = unsupported_suffix_production(suffix.lexeme) {
                        return self.parse_unsupported_from(start, production);
                    }
                }
                Ok(expression)
            }
        }
    }

    fn parse_for_expression(&mut self) -> Result<XPathExpressionNode, XPathParseError> {
        let (_, start_token) = self.expect("for")?;
        let mut bindings = Vec::new();
        loop {
            let (_, binding_start) = self.expect("$")?;
            let (name_index, name_token) = self.expect_name("variable name")?;
            let binding = self.resolve_name(name_index, name_token, XPathNameUse::Variable)?;
            self.expect("in")?;
            let binding_expression = self.parse_expression_single()?;
            bindings.push((binding_start.start, binding, binding_expression));
            if self.consume_if(",").is_none() {
                break;
            }
        }
        self.expect("return")?;
        let mut return_expression = self.parse_expression_single()?;
        let end = self.node_end(&return_expression);

        for (index, (binding_start, binding, binding_expression)) in
            bindings.into_iter().enumerate().rev()
        {
            let start = if index == 0 {
                start_token.start
            } else {
                binding_start
            };
            return_expression = XPathExpressionNode {
                expression: XPathExpression::For {
                    binding,
                    binding_expression: Box::new(binding_expression),
                    return_expression: Box::new(return_expression),
                },
                source_range: self.range(start, end),
            };
        }

        Ok(return_expression)
    }

    fn parse_let_expression(&mut self) -> Result<XPathExpressionNode, XPathParseError> {
        let (_, start_token) = self.expect("let")?;
        let mut bindings = Vec::new();
        loop {
            let (_, binding_start) = self.expect("$")?;
            let (name_index, name_token) = self.expect_name("variable name")?;
            let binding = self.resolve_name(name_index, name_token, XPathNameUse::Variable)?;
            self.expect(":=")?;
            let binding_expression = self.parse_expression_single()?;
            bindings.push((binding_start.start, binding, binding_expression));
            if self.consume_if(",").is_none() {
                break;
            }
        }
        self.expect("return")?;
        let mut return_expression = self.parse_expression_single()?;
        let end = self.node_end(&return_expression);

        for (index, (binding_start, binding, binding_expression)) in
            bindings.into_iter().enumerate().rev()
        {
            let start = if index == 0 {
                start_token.start
            } else {
                binding_start
            };
            return_expression = XPathExpressionNode {
                expression: XPathExpression::Let {
                    binding,
                    binding_expression: Box::new(binding_expression),
                    return_expression: Box::new(return_expression),
                },
                source_range: self.range(start, end),
            };
        }

        Ok(return_expression)
    }

    fn parse_if_expression(&mut self) -> Result<XPathExpressionNode, XPathParseError> {
        let (_, start_token) = self.expect("if")?;
        self.expect("(")?;
        let condition = self.parse_expression_sequence()?;
        self.expect(")")?;
        self.expect("then")?;
        let then_expression = self.parse_expression_single()?;
        self.expect("else")?;
        let else_expression = self.parse_expression_single()?;
        let end = self.node_end(&else_expression);

        Ok(XPathExpressionNode {
            expression: XPathExpression::If {
                condition: Box::new(condition),
                then_expression: Box::new(then_expression),
                else_expression: Box::new(else_expression),
            },
            source_range: self.range(start_token.start, end),
        })
    }

    fn parse_quantified_expression(&mut self) -> Result<XPathExpressionNode, XPathParseError> {
        let (_, start_token) = self.next().expect("peeked quantified expression");
        let quantifier = match start_token.lexeme {
            "some" => XPathQuantifier::Some,
            "every" => XPathQuantifier::Every,
            _ => unreachable!("quantified parser is selected only for some or every"),
        };
        let mut bindings = Vec::new();
        loop {
            let (_, binding_start) = self.expect("$")?;
            let (name_index, name_token) = self.expect_name("variable name")?;
            let binding = self.resolve_name(name_index, name_token, XPathNameUse::Variable)?;
            self.expect("in")?;
            let binding_expression = self.parse_expression_single()?;
            bindings.push((binding_start.start, binding, binding_expression));
            if self.consume_if(",").is_none() {
                break;
            }
        }
        self.expect("satisfies")?;
        let mut satisfies_expression = self.parse_expression_single()?;
        let end = self.node_end(&satisfies_expression);

        for (index, (binding_start, binding, binding_expression)) in
            bindings.into_iter().enumerate().rev()
        {
            let start = if index == 0 {
                start_token.start
            } else {
                binding_start
            };
            satisfies_expression = XPathExpressionNode {
                expression: XPathExpression::Quantified {
                    quantifier,
                    binding,
                    binding_expression: Box::new(binding_expression),
                    satisfies_expression: Box::new(satisfies_expression),
                },
                source_range: self.range(start, end),
            };
        }

        Ok(satisfies_expression)
    }

    fn parse_binary_expression(
        &mut self,
        minimum_precedence: u8,
    ) -> Result<XPathExpressionNode, XPathParseError> {
        let mut left = self.parse_arrow_expression()?;
        loop {
            let Some((operator, precedence)) = self.peek_binary_operator() else {
                break;
            };
            if precedence < minimum_precedence {
                break;
            }
            self.next();
            let right = self.parse_binary_expression(precedence + 1)?;
            let start = self.node_start(&left);
            let end = self.node_end(&right);
            left = XPathExpressionNode {
                expression: XPathExpression::Binary {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                source_range: self.range(start, end),
            };
        }
        Ok(left)
    }

    fn parse_arrow_expression(&mut self) -> Result<XPathExpressionNode, XPathParseError> {
        let mut expression = self.parse_unary_expression()?;
        while self.consume_if("=>").is_some() {
            let specifier = self.parse_arrow_function_specifier()?;
            self.expect("(")?;
            let mut arguments = self.parse_argument_list_after_open()?;
            let start = self.node_start(&expression);
            arguments.insert(0, expression);
            let end = self
                .previous_semantic()
                .map_or(start, |(_, token)| token.end);
            let source_range = self.range(start, end);
            let step = match specifier {
                XPathArrowFunctionSpecifier::Static(name) => {
                    XPathStep::Primary(XPathPrimaryExpression::FunctionCall { name, arguments })
                }
                XPathArrowFunctionSpecifier::Dynamic(primary) => XPathStep::Postfix {
                    primary,
                    postfixes: vec![XPathPostfixExpression::ArgumentList(arguments)],
                },
            };
            expression = XPathExpressionNode {
                expression: XPathExpression::Path(XPathPathExpression {
                    root: XPathPathRoot::Relative,
                    steps: vec![XPathStepNode { step, source_range }],
                    source_range,
                }),
                source_range,
            };
        }
        Ok(expression)
    }

    fn parse_arrow_function_specifier(
        &mut self,
    ) -> Result<XPathArrowFunctionSpecifier, XPathParseError> {
        let Some((token_index, token)) = self.next() else {
            return Err(self.syntax_error(&["arrow function specifier"]));
        };
        if token.lexeme == "$" {
            let (name_index, name_token) = self.expect_name("variable name")?;
            return self
                .resolve_name(name_index, name_token, XPathNameUse::Variable)
                .map(XPathPrimaryExpression::VariableReference)
                .map(XPathArrowFunctionSpecifier::Dynamic);
        }
        if token.lexeme == "(" {
            let primary = if self.consume_if(")").is_some() {
                XPathPrimaryExpression::Parenthesized(None)
            } else {
                let expression = self.parse_expression_sequence()?;
                self.expect(")")?;
                XPathPrimaryExpression::Parenthesized(Some(Box::new(expression)))
            };
            return Ok(XPathArrowFunctionSpecifier::Dynamic(primary));
        }
        if is_name_token(token) {
            return self
                .resolve_name(token_index, token, XPathNameUse::Function)
                .map(XPathArrowFunctionSpecifier::Static);
        }
        Err(self.syntax_error_at(token_index, token, &["arrow function specifier"]))
    }

    fn parse_unary_expression(&mut self) -> Result<XPathExpressionNode, XPathParseError> {
        let mut operators = Vec::new();
        while let Some((_, token)) = self.peek() {
            let operator = match token.lexeme {
                "+" => XPathUnaryOperator::Plus,
                "-" => XPathUnaryOperator::Minus,
                _ => break,
            };
            let (_, token) = self.next().expect("peeked unary operator");
            operators.push((operator, token.start));
        }

        let mut operand = self.parse_simple_map_expression()?;
        let end = self.node_end(&operand);
        for (operator, start) in operators.into_iter().rev() {
            operand = XPathExpressionNode {
                expression: XPathExpression::Unary {
                    operator,
                    operand: Box::new(operand),
                },
                source_range: self.range(start, end),
            };
        }
        Ok(operand)
    }

    fn parse_simple_map_expression(&mut self) -> Result<XPathExpressionNode, XPathParseError> {
        let input = self.parse_path_expression()?;
        let start = self.node_start(&input);
        let mut mappings = Vec::new();
        while self.consume_if("!").is_some() {
            mappings.push(self.parse_path_expression()?);
        }
        let Some(last_mapping) = mappings.last() else {
            return Ok(input);
        };
        let end = self.node_end(last_mapping);
        Ok(XPathExpressionNode {
            expression: XPathExpression::SimpleMap {
                input: Box::new(input),
                mappings,
            },
            source_range: self.range(start, end),
        })
    }

    fn parse_path_expression(&mut self) -> Result<XPathExpressionNode, XPathParseError> {
        let (root, start, require_first_step) = if let Some((_, token)) = self.consume_if("//") {
            (XPathPathRoot::RootedDescendant, token.start, true)
        } else if let Some((_, token)) = self.consume_if("/") {
            (XPathPathRoot::Rooted, token.start, false)
        } else {
            let Some((_, token)) = self.peek() else {
                return Err(self.syntax_error(&["path step"]));
            };
            (XPathPathRoot::Relative, token.start, true)
        };

        let mut steps = Vec::new();
        if require_first_step || self.peek().is_some_and(|(_, token)| can_start_step(token)) {
            steps.push(self.parse_step()?);
        }
        while let Some((_, separator)) = self.peek() {
            if !matches!(separator.lexeme, "/" | "//") {
                break;
            }
            let (_, separator) = self.next().expect("peeked path separator");
            if separator.lexeme == "//" {
                steps.push(XPathStepNode {
                    step: XPathStep::Axis {
                        axis: XPathAxis::DescendantOrSelf,
                        node_test: XPathNodeTest::Kind {
                            kind: XPathKindTest::AnyNode,
                            lexical: "node()".to_owned(),
                        },
                        predicates: Vec::new(),
                    },
                    source_range: self.range(separator.start, separator.end),
                });
            }
            steps.push(self.parse_step()?);
        }

        let end = steps
            .last()
            .map(|step| self.range_end(step.source_range))
            .or_else(|| self.previous_semantic().map(|(_, token)| token.end))
            .unwrap_or(start);
        let path = XPathPathExpression {
            root,
            steps,
            source_range: self.range(start, end),
        };
        Ok(XPathExpressionNode {
            source_range: path.source_range,
            expression: XPathExpression::Path(path),
        })
    }

    fn parse_step(&mut self) -> Result<XPathStepNode, XPathParseError> {
        let Some((_, first)) = self.peek() else {
            return Err(self.syntax_error(&["path step"]));
        };
        let start = first.start;

        if first.lexeme == ".." {
            self.next().expect("peeked parent step");
            let predicates = self.parse_predicates()?;
            let end = self
                .previous_semantic()
                .map_or(start, |(_, token)| token.end);
            return Ok(XPathStepNode {
                step: XPathStep::Axis {
                    axis: XPathAxis::Parent,
                    node_test: XPathNodeTest::Kind {
                        kind: XPathKindTest::AnyNode,
                        lexical: "node()".to_owned(),
                    },
                    predicates,
                },
                source_range: self.range(start, end),
            });
        }

        let explicit_axis = self.peek_explicit_axis();
        let is_kind_test = self.peek_kind_test().is_some();
        let next_lexeme = self.peek_nth(1).map(|(_, token)| token.lexeme);
        let is_constructor = matches!(first.lexeme, "map" | "array") && next_lexeme == Some("{");
        let is_named_function_reference = is_name_token(first) && next_lexeme == Some("#");
        let is_name_test = is_name_test_token(first)
            && next_lexeme != Some("(")
            && !is_constructor
            && !is_named_function_reference;
        if first.lexeme == "@" || explicit_axis.is_some() || is_kind_test || is_name_test {
            return self.parse_axis_step();
        }

        let primary = self.parse_primary_expression()?;
        let postfixes = self.parse_postfixes()?;
        let end = self
            .previous_semantic()
            .map_or(start, |(_, token)| token.end);
        let step = if postfixes.is_empty() {
            XPathStep::Primary(primary)
        } else {
            XPathStep::Postfix { primary, postfixes }
        };
        Ok(XPathStepNode {
            step,
            source_range: self.range(start, end),
        })
    }

    fn parse_axis_step(&mut self) -> Result<XPathStepNode, XPathParseError> {
        let (_, first) = self.peek().expect("axis step requires a token");
        let start = first.start;
        let axis = if self.consume_if("@").is_some() {
            XPathAxis::Attribute
        } else if let Some(axis) = self.peek_explicit_axis() {
            self.next();
            self.expect("::")?;
            axis
        } else {
            XPathAxis::Child
        };
        let node_test = self.parse_node_test(axis)?;
        let predicates = self.parse_predicates()?;
        let end = self
            .previous_semantic()
            .map_or(start, |(_, token)| token.end);
        Ok(XPathStepNode {
            step: XPathStep::Axis {
                axis,
                node_test,
                predicates,
            },
            source_range: self.range(start, end),
        })
    }

    fn parse_node_test(&mut self, axis: XPathAxis) -> Result<XPathNodeTest, XPathParseError> {
        if let Some(kind) = self.peek_kind_test() {
            let (_, start) = self.next().expect("peeked kind test");
            self.expect("(")?;
            self.consume_balanced_until(")")?;
            let (_, end) = self.expect(")")?;
            return Ok(XPathNodeTest::Kind {
                kind,
                lexical: format!("{}", self.lexical_between(start.start, end.end)),
            });
        }

        let (token_index, token) = self
            .next()
            .ok_or_else(|| self.syntax_error(&["node test"]))?;
        let name_test = match token.lexeme {
            "*" => XPathNameTest::Any,
            lexical if lexical.starts_with("*:") => XPathNameTest::AnyNamespace {
                local_name: lexical[2..].to_owned(),
            },
            lexical if lexical.starts_with("Q{") && lexical.ends_with("}*") => {
                XPathNameTest::Namespace {
                    namespace_uri: lexical[2..lexical.len() - 2].to_owned(),
                }
            }
            lexical if lexical.ends_with(":*") => {
                let prefix = &lexical[..lexical.len() - 2];
                XPathNameTest::Namespace {
                    namespace_uri: self.resolve_prefix(token_index, token, prefix)?,
                }
            }
            _ if is_name_token(token) => XPathNameTest::Name(self.resolve_name(
                token_index,
                token,
                if axis == XPathAxis::Attribute {
                    XPathNameUse::Attribute
                } else {
                    XPathNameUse::Element
                },
            )?),
            _ => return Err(self.syntax_error_at(token_index, token, &["node test"])),
        };
        Ok(XPathNodeTest::Name(name_test))
    }

    fn parse_predicates(&mut self) -> Result<Vec<XPathExpressionSequence>, XPathParseError> {
        let mut predicates = Vec::new();
        while self.consume_if("[").is_some() {
            if self.peek().is_none() {
                return Err(self.syntax_error(&["]"]));
            }
            let expression = self.parse_expression_sequence()?;
            self.expect("]")?;
            predicates.push(expression);
        }
        Ok(predicates)
    }

    fn parse_primary_expression(&mut self) -> Result<XPathPrimaryExpression, XPathParseError> {
        let (token_index, token) = self
            .next()
            .ok_or_else(|| self.syntax_error(&["primary expression"]))?;
        match token.kind {
            XPathLexicalTokenKind::IntegerLiteral
            | XPathLexicalTokenKind::DecimalLiteral
            | XPathLexicalTokenKind::DoubleLiteral => {
                Ok(XPathPrimaryExpression::Literal(self.numeric_literal(token)))
            }
            XPathLexicalTokenKind::StringLiteral => {
                Ok(XPathPrimaryExpression::Literal(XPathLiteral {
                    kind: XPathLiteralKind::String,
                    lexical: token.lexeme.to_owned(),
                    value: string_literal_value(token.lexeme),
                }))
            }
            _ if token.lexeme == "$" => {
                let (name_index, name_token) = self.expect_name("variable name")?;
                Ok(XPathPrimaryExpression::VariableReference(
                    self.resolve_name(name_index, name_token, XPathNameUse::Variable)?,
                ))
            }
            _ if token.lexeme == "." => Ok(XPathPrimaryExpression::ContextItem),
            _ if token.lexeme == "(" => {
                if self.consume_if(")").is_some() {
                    return Ok(XPathPrimaryExpression::Parenthesized(None));
                }
                let expression = self.parse_expression_sequence()?;
                self.expect(")")?;
                Ok(XPathPrimaryExpression::Parenthesized(Some(Box::new(
                    expression,
                ))))
            }
            _ if token.lexeme == "[" => {
                if self.consume_if("]").is_some() {
                    return Ok(XPathPrimaryExpression::ArrayConstructor(
                        XPathArrayConstructor::Square(XPathExpressionSequence {
                            expressions: Vec::new(),
                            source_range: self.range(token.end, token.end),
                        }),
                    ));
                }
                let expression = self.parse_expression_sequence()?;
                self.expect("]")?;
                Ok(XPathPrimaryExpression::ArrayConstructor(
                    XPathArrayConstructor::Square(expression),
                ))
            }
            _ if token.lexeme == "map" && self.consume_if("{").is_some() => {
                self.parse_map_constructor()
            }
            _ if token.lexeme == "array" && self.consume_if("{").is_some() => {
                let expression = if self.consume_if("}").is_some() {
                    None
                } else {
                    let expression = self.parse_expression_sequence()?;
                    self.expect("}")?;
                    Some(Box::new(expression))
                };
                Ok(XPathPrimaryExpression::ArrayConstructor(
                    XPathArrayConstructor::Curly(expression),
                ))
            }
            _ if token.lexeme == "function" => {
                self.consume_unsupported_tail();
                Ok(XPathPrimaryExpression::Unsupported {
                    production: "inline-function-expression".to_owned(),
                })
            }
            _ if is_name_token(token) && self.consume_if("(").is_some() => {
                let name = self.resolve_name(token_index, token, XPathNameUse::Function)?;
                let arguments = self.parse_argument_list_after_open()?;
                Ok(XPathPrimaryExpression::FunctionCall { name, arguments })
            }
            _ if is_name_token(token) && self.consume_if("#").is_some() => {
                self.expect_number("function arity")?;
                Ok(XPathPrimaryExpression::Unsupported {
                    production: "named-function-reference".to_owned(),
                })
            }
            _ if token.lexeme == "?" => {
                self.parse_lookup_key()?;
                Ok(XPathPrimaryExpression::Unsupported {
                    production: "unary-lookup".to_owned(),
                })
            }
            _ => Err(self.syntax_error_at(token_index, token, &["primary expression"])),
        }
    }

    fn parse_map_constructor(&mut self) -> Result<XPathPrimaryExpression, XPathParseError> {
        let mut entries = Vec::new();
        if self.consume_if("}").is_some() {
            return Ok(XPathPrimaryExpression::MapConstructor { entries });
        }
        loop {
            let key = self.parse_expression_single()?;
            let start = self.node_start(&key);
            self.expect(":")?;
            let value = self.parse_expression_single()?;
            let end = self.node_end(&value);
            entries.push(XPathMapConstructorEntry {
                key,
                value,
                source_range: self.range(start, end),
            });
            if self.consume_if(",").is_none() {
                break;
            }
        }
        self.expect("}")?;
        Ok(XPathPrimaryExpression::MapConstructor { entries })
    }

    fn parse_postfixes(&mut self) -> Result<Vec<XPathPostfixExpression>, XPathParseError> {
        let mut postfixes = Vec::new();
        loop {
            if self.consume_if("[").is_some() {
                let expression = self.parse_expression_sequence()?;
                self.expect("]")?;
                postfixes.push(XPathPostfixExpression::Predicate(expression));
                continue;
            }
            if self.consume_if("(").is_some() {
                postfixes.push(XPathPostfixExpression::ArgumentList(
                    self.parse_argument_list_after_open()?,
                ));
                continue;
            }
            if self.consume_if("?").is_some() {
                postfixes.push(XPathPostfixExpression::Lookup {
                    lexical: self.parse_lookup_key()?,
                });
                continue;
            }
            break;
        }
        Ok(postfixes)
    }

    fn parse_argument_list_after_open(
        &mut self,
    ) -> Result<Vec<XPathExpressionNode>, XPathParseError> {
        let mut arguments = Vec::new();
        if self.consume_if(")").is_some() {
            return Ok(arguments);
        }
        loop {
            arguments.push(self.parse_expression_single()?);
            if self.consume_if(",").is_none() {
                break;
            }
        }
        self.expect(")")?;
        Ok(arguments)
    }

    fn parse_lookup_key(&mut self) -> Result<String, XPathParseError> {
        if let Some((_, open)) = self.consume_if("(") {
            let start = open.end;
            let expression = self.parse_expression_sequence()?;
            let end = self.node_end(
                expression
                    .expressions
                    .last()
                    .expect("non-empty lookup expression"),
            );
            self.expect(")")?;
            return Ok(self.lexical_between(start, end));
        }
        let (token_index, token) = self
            .next()
            .ok_or_else(|| self.syntax_error(&["lookup key"]))?;
        if is_name_token(token)
            || token.kind == XPathLexicalTokenKind::IntegerLiteral
            || token.lexeme == "*"
        {
            Ok(token.lexeme.to_owned())
        } else {
            Err(self.syntax_error_at(token_index, token, &["lookup key"]))
        }
    }

    fn parse_unsupported_expression(
        &mut self,
        production: &'static str,
    ) -> Result<XPathExpressionNode, XPathParseError> {
        let start = self
            .peek()
            .map_or(self.source_len, |(_, token)| token.start);
        self.parse_unsupported_from(start, production)
    }

    fn parse_unsupported_from(
        &mut self,
        start: usize,
        production: &'static str,
    ) -> Result<XPathExpressionNode, XPathParseError> {
        let end = self.consume_unsupported_tail();
        if end == start {
            return Err(self.syntax_error(&["expression"]));
        }
        Ok(XPathExpressionNode {
            expression: XPathExpression::Unsupported {
                production: production.to_owned(),
            },
            source_range: self.range(start, end),
        })
    }

    fn consume_unsupported_tail(&mut self) -> usize {
        let mut delimiters = Vec::new();
        let mut end = self
            .previous_semantic()
            .map_or(self.source_len, |(_, token)| token.end);
        while let Some((_, token)) = self.peek() {
            if delimiters.is_empty() && matches!(token.lexeme, "," | ")" | "]" | "}") {
                break;
            }
            let (_, token) = self.next().expect("peeked unsupported token");
            match token.lexeme {
                "(" | "[" | "{" => delimiters.push(token.lexeme),
                ")" => {
                    if delimiters.last() == Some(&"(") {
                        delimiters.pop();
                    }
                }
                "]" => {
                    if delimiters.last() == Some(&"[") {
                        delimiters.pop();
                    }
                }
                "}" => {
                    if delimiters.last() == Some(&"{") {
                        delimiters.pop();
                    }
                }
                _ => {}
            }
            end = token.end;
        }
        end
    }

    fn consume_balanced_until(&mut self, close: &str) -> Result<(), XPathParseError> {
        let mut delimiters = Vec::new();
        while let Some((_, token)) = self.peek() {
            if delimiters.is_empty() && token.lexeme == close {
                return Ok(());
            }
            let (_, token) = self.next().expect("peeked balanced token");
            match token.lexeme {
                "(" | "[" | "{" => delimiters.push(token.lexeme),
                ")" if delimiters.last() == Some(&"(") => {
                    delimiters.pop();
                }
                "]" if delimiters.last() == Some(&"[") => {
                    delimiters.pop();
                }
                "}" if delimiters.last() == Some(&"{") => {
                    delimiters.pop();
                }
                _ => {}
            }
        }
        Err(self.syntax_error(&[close]))
    }

    fn numeric_literal(&self, token: XPathLexicalToken<'_>) -> XPathLiteral {
        let (kind, value) = match token.kind {
            XPathLexicalTokenKind::IntegerLiteral => {
                (XPathLiteralKind::Integer, normalize_integer(token.lexeme))
            }
            XPathLexicalTokenKind::DecimalLiteral => {
                (XPathLiteralKind::Decimal, normalize_decimal(token.lexeme))
            }
            XPathLexicalTokenKind::DoubleLiteral => (
                XPathLiteralKind::Double,
                token
                    .lexeme
                    .parse::<f64>()
                    .map_or_else(|_| token.lexeme.to_owned(), |value| value.to_string()),
            ),
            _ => unreachable!("numeric literal requires a numeric token"),
        };
        XPathLiteral {
            kind,
            lexical: token.lexeme.to_owned(),
            value,
        }
    }

    fn resolve_name(
        &self,
        token_index: usize,
        token: XPathLexicalToken<'_>,
        name_use: XPathNameUse,
    ) -> Result<XPathName, XPathParseError> {
        let lexical = token.lexeme;
        let (prefix, local_name, explicit_namespace) =
            if let Some(rest) = lexical.strip_prefix("Q{") {
                let Some((namespace, local_name)) = rest.split_once('}') else {
                    return Err(self.syntax_error_at(token_index, token, &["EQName"]));
                };
                (None, local_name.to_owned(), Some(namespace.to_owned()))
            } else if let Some((prefix, local_name)) = lexical.split_once(':') {
                (Some(prefix.to_owned()), local_name.to_owned(), None)
            } else {
                (None, lexical.to_owned(), None)
            };
        let namespace_uri = match (explicit_namespace, prefix.as_deref()) {
            (Some(namespace), _) => Some(namespace),
            (None, Some(prefix)) => Some(self.resolve_prefix(token_index, token, prefix)?),
            (None, None) => self.default_namespace(name_use),
        };
        Ok(XPathName {
            lexical: lexical.to_owned(),
            prefix,
            local_name,
            namespace_uri,
            source_range: self.range(token.start, token.end),
        })
    }

    fn resolve_prefix(
        &self,
        token_index: usize,
        token: XPathLexicalToken<'_>,
        prefix: &str,
    ) -> Result<String, XPathParseError> {
        self.namespace_for_prefix(prefix)
            .ok_or_else(|| XPathParseError {
                kind: XPathParseErrorKind::UnknownNamespacePrefix,
                expected: Vec::new(),
                found: Some(token.lexeme.to_owned()),
                namespace_prefix: Some(prefix.to_owned()),
                token_index: Some(token_index),
                start: token.start,
                end: token.end,
            })
    }

    fn namespace_for_prefix(&self, prefix: &str) -> Option<String> {
        let host_namespace = match self.attachment {
            XPathAttachment::Host(host) => host.static_context.namespaces.get(prefix).cloned(),
            XPathAttachment::Standalone { .. } => None,
        };
        host_namespace.or_else(|| {
            match prefix {
                "xml" => Some("http://www.w3.org/XML/1998/namespace"),
                "xs" => Some("http://www.w3.org/2001/XMLSchema"),
                "fn" => Some("http://www.w3.org/2005/xpath-functions"),
                "math" => Some("http://www.w3.org/2005/xpath-functions/math"),
                "map" => Some("http://www.w3.org/2005/xpath-functions/map"),
                "array" => Some("http://www.w3.org/2005/xpath-functions/array"),
                "err" => Some("http://www.w3.org/2005/xqt-errors"),
                "output" => Some("http://www.w3.org/2010/xslt-xquery-serialization"),
                _ => None,
            }
            .map(str::to_owned)
        })
    }

    fn default_namespace(&self, name_use: XPathNameUse) -> Option<String> {
        let static_context = match self.attachment {
            XPathAttachment::Host(host) => Some(&host.static_context),
            XPathAttachment::Standalone { .. } => None,
        };
        match name_use {
            XPathNameUse::Element => {
                static_context.and_then(|context| context.default_element_namespace.clone())
            }
            XPathNameUse::Function => static_context
                .and_then(|context| context.default_function_namespace.clone())
                .or_else(|| Some("http://www.w3.org/2005/xpath-functions".to_owned())),
            XPathNameUse::Attribute | XPathNameUse::Variable => None,
        }
    }

    fn peek_binary_operator(&self) -> Option<(XPathBinaryOperator, u8)> {
        let (_, token) = self.peek()?;
        binary_operator(token.lexeme)
    }

    fn peek_explicit_axis(&self) -> Option<XPathAxis> {
        let (_, token) = self.peek()?;
        self.peek_nth(1)
            .is_some_and(|(_, next)| next.lexeme == "::")
            .then(|| axis(token.lexeme))?
    }

    fn peek_kind_test(&self) -> Option<XPathKindTest> {
        let (_, token) = self.peek()?;
        self.peek_nth(1)
            .is_some_and(|(_, next)| next.lexeme == "(")
            .then(|| kind_test(token.lexeme))?
    }

    fn expect(
        &mut self,
        expected: &'static str,
    ) -> Result<(usize, XPathLexicalToken<'source>), XPathParseError> {
        let Some((token_index, token)) = self.next() else {
            return Err(self.syntax_error(&[expected]));
        };
        if token.lexeme == expected {
            Ok((token_index, token))
        } else {
            Err(self.syntax_error_at(token_index, token, &[expected]))
        }
    }

    fn expect_name(
        &mut self,
        expected: &'static str,
    ) -> Result<(usize, XPathLexicalToken<'source>), XPathParseError> {
        let Some((token_index, token)) = self.next() else {
            return Err(self.syntax_error(&[expected]));
        };
        if is_name_token(token) {
            Ok((token_index, token))
        } else {
            Err(self.syntax_error_at(token_index, token, &[expected]))
        }
    }

    fn expect_number(
        &mut self,
        expected: &'static str,
    ) -> Result<(usize, XPathLexicalToken<'source>), XPathParseError> {
        let Some((token_index, token)) = self.next() else {
            return Err(self.syntax_error(&[expected]));
        };
        if matches!(
            token.kind,
            XPathLexicalTokenKind::IntegerLiteral
                | XPathLexicalTokenKind::DecimalLiteral
                | XPathLexicalTokenKind::DoubleLiteral
        ) {
            Ok((token_index, token))
        } else {
            Err(self.syntax_error_at(token_index, token, &[expected]))
        }
    }

    fn consume_if(&mut self, lexeme: &str) -> Option<(usize, XPathLexicalToken<'source>)> {
        let (token_index, token) = self.peek()?;
        if token.lexeme != lexeme {
            return None;
        }
        self.raw_index = token_index + 1;
        Some((token_index, token))
    }

    fn next(&mut self) -> Option<(usize, XPathLexicalToken<'source>)> {
        let (token_index, token) = self.peek()?;
        self.raw_index = token_index + 1;
        Some((token_index, token))
    }

    fn peek(&self) -> Option<(usize, XPathLexicalToken<'source>)> {
        self.peek_from(self.raw_index)
    }

    fn peek_nth(&self, nth: usize) -> Option<(usize, XPathLexicalToken<'source>)> {
        let mut raw_index = self.raw_index;
        for _ in 0..nth {
            let (index, _) = self.peek_from(raw_index)?;
            raw_index = index + 1;
        }
        self.peek_from(raw_index)
    }

    fn peek_from(&self, mut raw_index: usize) -> Option<(usize, XPathLexicalToken<'source>)> {
        while let Some(token) = self.tokens.get(raw_index) {
            if !is_trivia(*token) {
                return Some((raw_index, *token));
            }
            raw_index += 1;
        }
        None
    }

    fn previous_semantic(&self) -> Option<(usize, XPathLexicalToken<'source>)> {
        let mut index = self.raw_index;
        while index > 0 {
            index -= 1;
            let token = &self.tokens[index];
            if !is_trivia(*token) {
                return Some((index, *token));
            }
        }
        None
    }

    fn syntax_error(&self, expected: &[&str]) -> XPathParseError {
        match self.peek() {
            Some((token_index, token)) => self.syntax_error_at(token_index, token, expected),
            None => XPathParseError {
                kind: XPathParseErrorKind::Syntax,
                expected: expected.iter().map(|value| (*value).to_owned()).collect(),
                found: None,
                namespace_prefix: None,
                token_index: None,
                start: self.source_len,
                end: self.source_len,
            },
        }
    }

    fn syntax_error_at(
        &self,
        token_index: usize,
        token: XPathLexicalToken<'_>,
        expected: &[&str],
    ) -> XPathParseError {
        XPathParseError {
            kind: XPathParseErrorKind::Syntax,
            expected: expected.iter().map(|value| (*value).to_owned()).collect(),
            found: Some(token.lexeme.to_owned()),
            namespace_prefix: None,
            token_index: Some(token_index),
            start: token.start,
            end: token.end,
        }
    }

    fn node_start(&self, node: &XPathExpressionNode) -> usize {
        self.range_resolver.decoded_start(node.source_range)
    }

    fn node_end(&self, node: &XPathExpressionNode) -> usize {
        self.range_resolver.decoded_end(node.source_range)
    }

    fn range_end(&self, range: XPathSourceRange) -> usize {
        self.range_resolver.decoded_end(range)
    }

    fn range(&self, start: usize, end: usize) -> XPathSourceRange {
        self.range_resolver.range(start, end)
    }

    fn lexical_between(&self, start: usize, end: usize) -> String {
        self.tokens
            .iter()
            .filter(|token| token.start >= start && token.end <= end)
            .map(|token| token.lexeme)
            .collect()
    }
}

fn is_trivia(token: XPathLexicalToken<'_>) -> bool {
    matches!(
        token.kind,
        XPathLexicalTokenKind::Comment | XPathLexicalTokenKind::Whitespace
    )
}

fn is_name_token(token: XPathLexicalToken<'_>) -> bool {
    matches!(
        token.kind,
        XPathLexicalTokenKind::Name
            | XPathLexicalTokenKind::DelimitingName
            | XPathLexicalTokenKind::BracedUriLiteral
            | XPathLexicalTokenKind::Keyword
    )
}

fn is_name_test_token(token: XPathLexicalToken<'_>) -> bool {
    is_name_token(token) || token.lexeme == "*"
}

fn can_start_step(token: XPathLexicalToken<'_>) -> bool {
    is_name_test_token(token)
        || matches!(
            token.kind,
            XPathLexicalTokenKind::IntegerLiteral
                | XPathLexicalTokenKind::DecimalLiteral
                | XPathLexicalTokenKind::DoubleLiteral
                | XPathLexicalTokenKind::StringLiteral
        )
        || matches!(token.lexeme, "$" | "(" | "[" | "." | ".." | "@" | "?")
}

fn axis(lexeme: &str) -> Option<XPathAxis> {
    match lexeme {
        "ancestor" => Some(XPathAxis::Ancestor),
        "ancestor-or-self" => Some(XPathAxis::AncestorOrSelf),
        "attribute" => Some(XPathAxis::Attribute),
        "child" => Some(XPathAxis::Child),
        "descendant" => Some(XPathAxis::Descendant),
        "descendant-or-self" => Some(XPathAxis::DescendantOrSelf),
        "following" => Some(XPathAxis::Following),
        "following-sibling" => Some(XPathAxis::FollowingSibling),
        "namespace" => Some(XPathAxis::Namespace),
        "parent" => Some(XPathAxis::Parent),
        "preceding" => Some(XPathAxis::Preceding),
        "preceding-sibling" => Some(XPathAxis::PrecedingSibling),
        "self" => Some(XPathAxis::SelfAxis),
        _ => None,
    }
}

fn kind_test(lexeme: &str) -> Option<XPathKindTest> {
    match lexeme {
        "document-node" => Some(XPathKindTest::Document),
        "element" => Some(XPathKindTest::Element),
        "attribute" => Some(XPathKindTest::Attribute),
        "schema-element" => Some(XPathKindTest::SchemaElement),
        "schema-attribute" => Some(XPathKindTest::SchemaAttribute),
        "processing-instruction" => Some(XPathKindTest::ProcessingInstruction),
        "comment" => Some(XPathKindTest::Comment),
        "text" => Some(XPathKindTest::Text),
        "namespace-node" => Some(XPathKindTest::NamespaceNode),
        "node" => Some(XPathKindTest::AnyNode),
        _ => None,
    }
}

fn binary_operator(lexeme: &str) -> Option<(XPathBinaryOperator, u8)> {
    let (operator, precedence) = match lexeme {
        "or" => (XPathBinaryOperator::Or, 1),
        "and" => (XPathBinaryOperator::And, 2),
        "eq" => (XPathBinaryOperator::ValueEqual, 3),
        "ne" => (XPathBinaryOperator::ValueNotEqual, 3),
        "lt" => (XPathBinaryOperator::ValueLessThan, 3),
        "le" => (XPathBinaryOperator::ValueLessThanOrEqual, 3),
        "gt" => (XPathBinaryOperator::ValueGreaterThan, 3),
        "ge" => (XPathBinaryOperator::ValueGreaterThanOrEqual, 3),
        "=" => (XPathBinaryOperator::GeneralEqual, 3),
        "!=" => (XPathBinaryOperator::GeneralNotEqual, 3),
        "<" => (XPathBinaryOperator::GeneralLessThan, 3),
        "<=" => (XPathBinaryOperator::GeneralLessThanOrEqual, 3),
        ">" => (XPathBinaryOperator::GeneralGreaterThan, 3),
        ">=" => (XPathBinaryOperator::GeneralGreaterThanOrEqual, 3),
        "is" => (XPathBinaryOperator::NodeIs, 3),
        "<<" => (XPathBinaryOperator::NodePrecedes, 3),
        ">>" => (XPathBinaryOperator::NodeFollows, 3),
        "||" => (XPathBinaryOperator::Concatenate, 4),
        "to" => (XPathBinaryOperator::Range, 5),
        "+" => (XPathBinaryOperator::Add, 6),
        "-" => (XPathBinaryOperator::Subtract, 6),
        "*" => (XPathBinaryOperator::Multiply, 7),
        "div" => (XPathBinaryOperator::Divide, 7),
        "idiv" => (XPathBinaryOperator::IntegerDivide, 7),
        "mod" => (XPathBinaryOperator::Modulo, 7),
        "union" | "|" => (XPathBinaryOperator::Union, 8),
        "intersect" => (XPathBinaryOperator::Intersect, 9),
        "except" => (XPathBinaryOperator::Except, 9),
        _ => return None,
    };
    Some((operator, precedence))
}

fn unsupported_suffix_production(lexeme: &str) -> Option<&'static str> {
    match lexeme {
        "cast" => Some("cast-expression"),
        "castable" => Some("castable-expression"),
        "treat" => Some("treat-expression"),
        "instance" => Some("instance-of-expression"),
        _ => None,
    }
}

fn normalize_integer(lexical: &str) -> String {
    let normalized = lexical.trim_start_matches('0');
    if normalized.is_empty() {
        "0".to_owned()
    } else {
        normalized.to_owned()
    }
}

fn normalize_decimal(lexical: &str) -> String {
    let (integer, fraction) = lexical.split_once('.').unwrap_or((lexical, ""));
    let integer = if integer.is_empty() {
        "0".to_owned()
    } else {
        normalize_integer(integer)
    };
    let fraction = fraction.trim_end_matches('0');
    if fraction.is_empty() {
        integer
    } else {
        format!("{integer}.{fraction}")
    }
}

fn string_literal_value(lexical: &str) -> String {
    if lexical.len() < 2 {
        return lexical.to_owned();
    }
    let quote = &lexical[..1];
    lexical[1..lexical.len() - 1].replace(&format!("{quote}{quote}"), quote)
}
