//! Conservative resource-use inspection of retained native XPath syntax.
use crate::validation::xpath::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathResourceUse {
    pub function: String,
    pub literal_uri: Option<String>,
    pub source_range: XPathSourceRange,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathResourceInspection {
    pub loads: Vec<XPathResourceUse>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateXPathResourceInspection {
    pub template_uri: String,
    pub invocation_id: String,
    pub inspection: XPathResourceInspection,
}

pub fn inspect_template_xpath_slots(
    template_uri: &str,
    options: &crate::transform_template::TransformTemplateModuleOptions,
) -> Vec<TemplateXPathResourceInspection> {
    options
        .xpath_invocations
        .iter()
        .map(|invocation| TemplateXPathResourceInspection {
            template_uri: template_uri.into(),
            invocation_id: invocation.id.clone(),
            inspection: inspect_xpath_resource_uses(
                &invocation.expression,
                &invocation.static_context,
                10_000,
            ),
        })
        .collect()
}

/// Inspect native syntax without evaluating variables, functions or URI bases.
/// Literal references are candidates only; resolution/policy is always outstanding.
/// This covers the supplied XPath slot, not all template or document load mechanisms.
pub fn inspect_xpath_resource_uses(
    ast: &XPathExpressionAst,
    context: &XPathStaticContext,
    max_nodes: usize,
) -> XPathResourceInspection {
    let mut result = XPathResourceInspection::default();
    let Some(syntax) = &ast.syntax_ast else {
        result.blockers.push("invalid-xpath-syntax".into());
        return result;
    };
    enum Work<'a> {
        Sequence(&'a XPathExpressionSequence),
        Expression(&'a XPathExpressionNode),
        Primary(&'a XPathPrimaryExpression),
        Lookup(&'a XPathLookupKey),
    }
    let mut work = vec![Work::Sequence(&syntax.root)];
    let mut count = 0;
    while let Some(item) = work.pop() {
        if count >= max_nodes {
            block(&mut result, "inspection-budget-exceeded");
            break;
        }
        count += 1;
        match item {
            Work::Sequence(sequence) => {
                work.extend(sequence.expressions.iter().rev().map(Work::Expression))
            }
            Work::Lookup(XPathLookupKey::Expression(Some(sequence))) => {
                work.push(Work::Sequence(sequence))
            }
            Work::Lookup(_) => {}
            Work::Expression(node) => match &node.expression {
                XPathExpression::Path(path) => {
                    for step in path.steps.iter().rev() {
                        match &step.step {
                            XPathStep::Axis { predicates, .. } => {
                                work.extend(predicates.iter().rev().map(Work::Sequence))
                            }
                            XPathStep::Primary(primary) => work.push(Work::Primary(primary)),
                            XPathStep::Postfix { primary, postfixes } => {
                                for postfix in postfixes.iter().rev() {
                                    match postfix {
                                        XPathPostfixExpression::Predicate(sequence) => {
                                            work.push(Work::Sequence(sequence))
                                        }
                                        XPathPostfixExpression::Lookup { key } => {
                                            work.push(Work::Lookup(key))
                                        }
                                        XPathPostfixExpression::ArgumentList(arguments) => {
                                            block(&mut result, "dynamic-function-call");
                                            work.extend(
                                                arguments.iter().rev().map(Work::Expression),
                                            );
                                        }
                                    }
                                }
                                work.push(Work::Primary(primary));
                            }
                        }
                    }
                }
                XPathExpression::Unary { operand, .. }
                | XPathExpression::CastAs { operand, .. }
                | XPathExpression::CastableAs { operand, .. }
                | XPathExpression::TreatAs { operand, .. }
                | XPathExpression::InstanceOf { operand, .. } => {
                    work.push(Work::Expression(operand))
                }
                XPathExpression::Binary { left, right, .. } => {
                    work.push(Work::Expression(right));
                    work.push(Work::Expression(left));
                }
                XPathExpression::SimpleMap { input, mappings } => {
                    work.extend(mappings.iter().rev().map(Work::Expression));
                    work.push(Work::Expression(input));
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
                } => {
                    work.push(Work::Expression(return_expression));
                    work.push(Work::Expression(binding_expression));
                }
                XPathExpression::Quantified {
                    binding_expression,
                    satisfies_expression,
                    ..
                } => {
                    work.push(Work::Expression(satisfies_expression));
                    work.push(Work::Expression(binding_expression));
                }
                XPathExpression::If {
                    condition,
                    then_expression,
                    else_expression,
                } => {
                    work.push(Work::Expression(else_expression));
                    work.push(Work::Expression(then_expression));
                    work.push(Work::Sequence(condition));
                }
                XPathExpression::Unsupported { .. } => {
                    block(&mut result, "unsupported-xpath-syntax")
                }
            },
            Work::Primary(primary) => match primary {
                XPathPrimaryExpression::FunctionCall { name, arguments } => {
                    let namespace =
                        name.namespace_uri
                            .as_deref()
                            .or_else(|| match name.prefix.as_deref() {
                                Some(prefix) => context
                                    .namespaces
                                    .get(prefix)
                                    .map(String::as_str)
                                    .or_else(|| (prefix == "fn").then_some(FUNCTIONS)),
                                None => context
                                    .default_function_namespace
                                    .as_deref()
                                    .or(Some(FUNCTIONS)),
                            });
                    if namespace != Some(FUNCTIONS)
                        || context.function_bindings.contains_key(&name.lexical)
                    {
                        block(&mut result, "extension-function");
                    } else if matches!(
                        name.local_name.as_str(),
                        "doc"
                            | "doc-available"
                            | "collection"
                            | "uri-collection"
                            | "json-doc"
                            | "unparsed-text"
                            | "unparsed-text-lines"
                            | "unparsed-text-available"
                    ) {
                        let literal_uri = arguments
                            .first()
                            .and_then(literal_string)
                            .map(str::to_owned);
                        block(
                            &mut result,
                            if literal_uri.is_some() {
                                "resource-resolution-required"
                            } else {
                                "computed-resource-uri"
                            },
                        );
                        result.loads.push(XPathResourceUse {
                            function: name.lexical.clone(),
                            literal_uri,
                            source_range: name.source_range,
                        });
                    } else if matches!(
                        name.local_name.as_str(),
                        "function-lookup" | "apply" | "transform" | "load-xquery-module"
                    ) {
                        block(&mut result, "dynamic-function-or-module");
                    }
                    work.extend(arguments.iter().rev().map(Work::Expression));
                }
                XPathPrimaryExpression::Parenthesized(Some(sequence)) => {
                    work.push(Work::Sequence(sequence))
                }
                XPathPrimaryExpression::InlineFunction { body, .. } => {
                    block(&mut result, "inline-function");
                    if let Some(sequence) = body {
                        work.push(Work::Sequence(sequence));
                    }
                }
                XPathPrimaryExpression::MapConstructor { entries } => {
                    for entry in entries.iter().rev() {
                        work.push(Work::Expression(&entry.value));
                        work.push(Work::Expression(&entry.key));
                    }
                }
                XPathPrimaryExpression::ArrayConstructor(XPathArrayConstructor::Square(
                    sequence,
                )) => work.push(Work::Sequence(sequence)),
                XPathPrimaryExpression::ArrayConstructor(XPathArrayConstructor::Curly(Some(
                    sequence,
                ))) => work.push(Work::Sequence(sequence)),
                XPathPrimaryExpression::UnaryLookup(key) => work.push(Work::Lookup(key)),
                XPathPrimaryExpression::Unsupported { .. } => {
                    block(&mut result, "unsupported-xpath-syntax")
                }
                XPathPrimaryExpression::Literal(_)
                | XPathPrimaryExpression::VariableReference(_)
                | XPathPrimaryExpression::ContextItem
                | XPathPrimaryExpression::Parenthesized(None)
                | XPathPrimaryExpression::ArrayConstructor(XPathArrayConstructor::Curly(None)) => {}
            },
        }
    }
    result
}
const FUNCTIONS: &str = "http://www.w3.org/2005/xpath-functions";
fn block(result: &mut XPathResourceInspection, code: &str) {
    if !result.blockers.iter().any(|item| item == code) {
        result.blockers.push(code.into());
    }
}
fn literal_string(node: &XPathExpressionNode) -> Option<&str> {
    let XPathExpression::Path(path) = &node.expression else {
        return None;
    };
    if path.root != XPathPathRoot::Relative || path.steps.len() != 1 {
        return None;
    }
    match &path.steps[0].step {
        XPathStep::Primary(XPathPrimaryExpression::Literal(literal))
            if literal.kind == XPathLiteralKind::String =>
        {
            Some(&literal.value)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inspect(
        source: &str,
        context: XPathStaticContext,
        budget: usize,
    ) -> XPathResourceInspection {
        let ast = xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: source.as_bytes(),
                source_uri: "studio://p/template.cemt",
                content_type: None,
                source_range_projector: None,
            },
            XPathAttachment::StandaloneStaticContext {
                source_id: 1,
                static_context: context.clone(),
            },
        );
        inspect_xpath_resource_uses(&ast, &context, budget)
    }
    #[test]
    fn records_literal_and_computed_loads_without_evaluation() {
        let report = inspect(
            "(doc('data.xml'), doc(concat('a', '.xml')), count(doc('b.xml')))",
            XPathStaticContext::default(),
            1000,
        );
        assert_eq!(report.loads.len(), 3);
        assert_eq!(report.loads[0].literal_uri.as_deref(), Some("data.xml"));
        assert!(report.loads[1].literal_uri.is_none());
        assert_eq!(report.loads[2].literal_uri.as_deref(), Some("b.xml"));
        assert!(report.blockers.iter().any(|b| b == "computed-resource-uri"));
        assert!(report
            .blockers
            .iter()
            .any(|b| b == "resource-resolution-required"));
    }
    #[test]
    fn traverses_conditional_and_container_branches_without_executing_them() {
        let report = inspect("(if (true()) then doc('a') else doc('b'), map{'x':doc('c')}, [doc('d')], (1)[doc('e')])", XPathStaticContext::default(), 1000);
        let mut uris: Vec<_> = report
            .loads
            .iter()
            .filter_map(|load| load.literal_uri.as_deref())
            .collect();
        uris.sort();
        assert_eq!(uris, vec!["a", "b", "c", "d", "e"]);
    }

    #[test]
    fn namespace_identity_and_higher_order_calls_remain_explicit() {
        let mut context = XPathStaticContext::default();
        context
            .namespaces
            .insert("custom".into(), "urn:custom".into());
        let report = inspect("custom:doc('data.xml')", context, 1000);
        assert!(report.loads.is_empty());
        assert!(report.blockers.contains(&"extension-function".into()));
        let report = inspect(
            "function-lookup(QName('http://www.w3.org/2005/xpath-functions', 'doc'), 1)('a.xml')",
            XPathStaticContext::default(),
            1000,
        );
        assert!(!report.blockers.is_empty());
    }
    #[test]
    fn string_contents_are_not_calls_and_budget_exhaustion_is_blocked() {
        let report = inspect("'doc(\"fake.xml\")'", XPathStaticContext::default(), 1000);
        assert!(report.loads.is_empty());
        assert!(report.blockers.is_empty());
        let report = inspect("doc('a.xml')", XPathStaticContext::default(), 0);
        assert!(report
            .blockers
            .contains(&"inspection-budget-exceeded".into()));
        let report = inspect("doc(", XPathStaticContext::default(), 1000);
        assert!(report.blockers.contains(&"invalid-xpath-syntax".into()));
    }
}
