//! Group construction stays in compiler-owned, typed XPath; CEMT only loops.
use super::*;
use cem_ml::validation::xpath::*;

// Inspect typed syntax, including unreachable branches and function bodies.
// Pattern errors are static and must not depend on predicate evaluation.
pub(super) fn group_function(
    sequence: &XPathExpressionSequence,
) -> Option<(&'static str, XPathSourceRange)> {
    fn key<'a>(key: &'a XPathLookupKey, stack: &mut Vec<&'a XPathExpressionNode>) {
        if let XPathLookupKey::Expression(Some(sequence)) = key {
            stack.extend(&sequence.expressions);
        }
    }
    fn primary<'a>(
        value: &'a XPathPrimaryExpression,
        stack: &mut Vec<&'a XPathExpressionNode>,
    ) -> Option<(&'static str, XPathSourceRange)> {
        match value {
            XPathPrimaryExpression::FunctionCall { name, arguments } => {
                if name.namespace_uri.as_deref() == Some("http://www.w3.org/2005/xpath-functions") {
                    let code = match name.local_name.as_str() {
                        "current-group" => Some("XTSE1060"),
                        "current-grouping-key" => Some("XTSE1070"),
                        _ => None,
                    };
                    if let Some(code) = code {
                        return Some((code, name.source_range));
                    }
                }
                stack.extend(arguments);
            }
            XPathPrimaryExpression::Parenthesized(Some(sequence))
            | XPathPrimaryExpression::ArrayConstructor(XPathArrayConstructor::Curly(Some(
                sequence,
            )))
            | XPathPrimaryExpression::InlineFunction {
                body: Some(sequence),
                ..
            } => stack.extend(&sequence.expressions),
            XPathPrimaryExpression::ArrayConstructor(XPathArrayConstructor::Square(sequence)) => {
                stack.extend(&sequence.expressions)
            }
            XPathPrimaryExpression::MapConstructor { entries } => {
                for entry in entries {
                    stack.extend([&entry.key, &entry.value]);
                }
            }
            XPathPrimaryExpression::UnaryLookup(value) => key(value, stack),
            _ => (),
        }
        None
    }
    let mut stack: Vec<_> = sequence.expressions.iter().collect();
    while let Some(node) = stack.pop() {
        match &node.expression {
            XPathExpression::Path(path) => {
                for step in &path.steps {
                    match &step.step {
                        XPathStep::Primary(value) => {
                            if let Some(found) = primary(value, &mut stack) {
                                return Some(found);
                            }
                        }
                        XPathStep::Postfix {
                            primary: value,
                            postfixes,
                        } => {
                            if let Some(found) = primary(value, &mut stack) {
                                return Some(found);
                            }
                            for postfix in postfixes {
                                match postfix {
                                    XPathPostfixExpression::Predicate(sequence) => {
                                        stack.extend(&sequence.expressions)
                                    }
                                    XPathPostfixExpression::ArgumentList(arguments) => {
                                        stack.extend(arguments)
                                    }
                                    XPathPostfixExpression::Lookup { key: value } => {
                                        key(value, &mut stack)
                                    }
                                }
                            }
                        }
                        XPathStep::Axis { predicates, .. } => {
                            for predicate in predicates {
                                stack.extend(&predicate.expressions);
                            }
                        }
                    }
                }
            }
            XPathExpression::Unary { operand, .. }
            | XPathExpression::CastAs { operand, .. }
            | XPathExpression::CastableAs { operand, .. }
            | XPathExpression::TreatAs { operand, .. }
            | XPathExpression::InstanceOf { operand, .. } => stack.push(operand),
            XPathExpression::Binary { left, right, .. } => {
                stack.extend([left.as_ref(), right.as_ref()])
            }
            XPathExpression::SimpleMap { input, mappings } => {
                stack.push(input);
                stack.extend(mappings);
            }
            XPathExpression::For {
                binding_expression,
                return_expression,
                ..
            }
            | XPathExpression::Let {
                binding_expression,
                return_expression,
                ..
            } => stack.extend([binding_expression.as_ref(), return_expression.as_ref()]),
            XPathExpression::Quantified {
                binding_expression,
                satisfies_expression,
                ..
            } => stack.extend([binding_expression.as_ref(), satisfies_expression.as_ref()]),
            XPathExpression::If {
                condition,
                then_expression,
                else_expression,
            } => {
                stack.extend(&condition.expressions);
                stack.extend([then_expression.as_ref(), else_expression.as_ref()]);
            }
            XPathExpression::Unsupported { .. } => (),
        }
    }
    None
}

impl Compiler<'_> {
    pub(super) fn grouping(
        &mut self,
        node: &AuthorNode<'_>,
        scope: &Scope,
    ) -> CompileResult<String> {
        let event = node.event;
        self.attributes(event, &["select", "group-by", "composite", "collation"])?;
        if self.attribute(event, "composite").is_some_and(|a| {
            !matches!(
                a.entity_decoded_value.as_deref(),
                Some("no" | "false" | "0")
            )
        }) {
            return Err(self.error(
                event,
                "cem.xslt.compile_unsupported",
                "this grouping profile supports non-composite group-by",
            ));
        }
        if self.attribute(event, "collation").is_some_and(|a| {
            a.entity_decoded_value.as_deref()
                != Some("http://www.w3.org/2005/xpath-functions/collation/codepoint")
        }) {
            return Err(self.error(
                event,
                "cem.xslt.compile_unsupported",
                "this grouping profile supports the literal Unicode codepoint collation",
            ));
        }
        let select = self.expression(event, "select")?;
        let key = self.expression(event, "group-by")?;
        for expression in [&select, &key] {
            if expression.syntax_ast.as_ref().is_some_and(|syntax| {
                syntax.events.iter().any(|e| {
                    matches!(
                        e.node_kind,
                        XPathSyntaxNodeKind::UnsupportedExpression
                            | XPathSyntaxNodeKind::UnsupportedPrimary
                    )
                })
            }) {
                return Err(self.error(
                    event,
                    "cem.xslt.compile_unsupported",
                    "unsupported grouping XPath syntax",
                ));
            }
        }
        let mut expression = self.generated_xpath(event, "0")?;
        xpath::group_by(&mut expression, select, key)
            .map_err(|message| self.error(event, "cem.xslt.compile_xpath", message))?;
        let select = self.program(event, expression, scope, xpath::ResultKind::Sequence)?;
        let (sorting, select, children) = self.sorted(node, scope, select, true)?;
        let groups = self.fresh();
        let size = self.fresh();
        let group = self.fresh();
        let position = self.fresh();
        let members = self.fresh();
        let key = self.fresh();
        let item = self.fresh();
        let loop_scope = Scope {
            item: group.clone(),
            position: position.clone(),
            size: size.clone(),
            ..scope.clone()
        };
        let mut extraction = |code| {
            self.program(
                event,
                self.generated_xpath(event, code)?,
                &loop_scope,
                xpath::ResultKind::Sequence,
            )
        };
        let members_select = extraction(".?(2)")?;
        let key_select = extraction(".?(1)")?;
        let item_select = extraction("head(.?(2))")?;
        let inner = Scope {
            item: item.clone(),
            position: position.clone(),
            size: size.clone(),
            groups: Some(["true".into(), members.clone(), "true".into(), key.clone()]),
            ..scope.clone()
        };
        let body = self.sequence(&children, inner)?;
        Ok(format!(
            "{sorting}{}{}{{for-each @select={} @as={} |{}{}{}{}{body}}}",
            emit_variable(&groups, &select),
            emit_variable(&size, &format!("seq:count({groups})")),
            quote(&groups),
            quote(&group),
            emit_variable(&position, "position"),
            emit_variable(&members, &members_select),
            emit_variable(&key, &key_select),
            emit_variable(&item, &item_select)
        ))
    }
}
