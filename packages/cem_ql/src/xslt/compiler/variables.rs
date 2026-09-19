//! Select the outer bindings used by one XPath slot without rewriting its AST.
use cem_ml::validation::xpath::*;
use std::collections::BTreeSet;

type Names = BTreeSet<XPathExpandedName>;

/// A bare reference forwards the existing sequence unchanged. No focus,
/// atomization, conversion or XPath operation is involved.
pub(super) fn bare_reference(sequence: &XPathExpressionSequence) -> Option<XPathExpandedName> {
    let [XPathExpressionNode {
        expression: XPathExpression::Path(path),
        ..
    }] = sequence.expressions.as_slice()
    else {
        return None;
    };
    if path.root != XPathPathRoot::Relative {
        return None;
    }
    let [XPathStepNode {
        step: XPathStep::Primary(XPathPrimaryExpression::VariableReference(name)),
        ..
    }] = path.steps.as_slice()
    else {
        return None;
    };
    Some(expanded(name))
}

pub(super) fn empty_sequence(sequence: &XPathExpressionSequence) -> bool {
    let [XPathExpressionNode {
        expression: XPathExpression::Path(path),
        ..
    }] = sequence.expressions.as_slice()
    else {
        return false;
    };
    path.root == XPathPathRoot::Relative
        && matches!(
            path.steps.as_slice(),
            [XPathStepNode {
                step: XPathStep::Primary(XPathPrimaryExpression::Parenthesized(None)),
                ..
            }]
        )
}

fn expanded(name: &XPathName) -> XPathExpandedName {
    XPathExpandedName::new(name.namespace_uri.as_deref(), &name.local_name)
}

pub(super) fn referenced(sequence: &XPathExpressionSequence) -> Names {
    let mut names = Names::new();
    visit_sequence(sequence, &Names::new(), &mut names);
    names
}
fn visit_sequence(sequence: &XPathExpressionSequence, bound: &Names, names: &mut Names) {
    for node in &sequence.expressions {
        visit(node, bound, names);
    }
}
fn lookup(key: &XPathLookupKey, bound: &Names, names: &mut Names) {
    if let XPathLookupKey::Expression(Some(sequence)) = key {
        visit_sequence(sequence, bound, names);
    }
}
fn primary(value: &XPathPrimaryExpression, bound: &Names, names: &mut Names) {
    match value {
        XPathPrimaryExpression::VariableReference(name) => {
            let name = expanded(name);
            if !bound.contains(&name) {
                names.insert(name);
            }
        }
        XPathPrimaryExpression::FunctionCall { arguments, .. } => {
            for argument in arguments {
                visit(argument, bound, names);
            }
        }
        XPathPrimaryExpression::Parenthesized(Some(sequence))
        | XPathPrimaryExpression::ArrayConstructor(XPathArrayConstructor::Curly(Some(sequence))) => {
            visit_sequence(sequence, bound, names)
        }
        XPathPrimaryExpression::ArrayConstructor(XPathArrayConstructor::Square(sequence)) => {
            visit_sequence(sequence, bound, names)
        }
        XPathPrimaryExpression::InlineFunction {
            parameters,
            body: Some(sequence),
            ..
        } => {
            let mut inner = bound.clone();
            inner.extend(parameters.iter().map(|p| expanded(&p.name)));
            visit_sequence(sequence, &inner, names);
        }
        XPathPrimaryExpression::MapConstructor { entries } => {
            for entry in entries {
                visit(&entry.key, bound, names);
                visit(&entry.value, bound, names);
            }
        }
        XPathPrimaryExpression::UnaryLookup(key) => lookup(key, bound, names),
        XPathPrimaryExpression::Literal(_)
        | XPathPrimaryExpression::ContextItem
        | XPathPrimaryExpression::Parenthesized(None)
        | XPathPrimaryExpression::ArrayConstructor(XPathArrayConstructor::Curly(None))
        | XPathPrimaryExpression::InlineFunction { body: None, .. }
        | XPathPrimaryExpression::Unsupported { .. } => (),
    }
}
fn visit(node: &XPathExpressionNode, bound: &Names, names: &mut Names) {
    match &node.expression {
        XPathExpression::Path(path) => {
            for step in &path.steps {
                match &step.step {
                    XPathStep::Primary(value) => primary(value, bound, names),
                    XPathStep::Postfix {
                        primary: value,
                        postfixes,
                    } => {
                        primary(value, bound, names);
                        for postfix in postfixes {
                            match postfix {
                                XPathPostfixExpression::Predicate(sequence) => {
                                    visit_sequence(sequence, bound, names)
                                }
                                XPathPostfixExpression::ArgumentList(arguments) => {
                                    for argument in arguments {
                                        visit(argument, bound, names);
                                    }
                                }
                                XPathPostfixExpression::Lookup { key } => lookup(key, bound, names),
                            }
                        }
                    }
                    XPathStep::Axis { predicates, .. } => {
                        for sequence in predicates {
                            visit_sequence(sequence, bound, names);
                        }
                    }
                }
            }
        }
        XPathExpression::Unary { operand, .. }
        | XPathExpression::CastAs { operand, .. }
        | XPathExpression::CastableAs { operand, .. }
        | XPathExpression::TreatAs { operand, .. }
        | XPathExpression::InstanceOf { operand, .. } => visit(operand, bound, names),
        XPathExpression::Binary { left, right, .. } => {
            visit(left, bound, names);
            visit(right, bound, names);
        }
        XPathExpression::SimpleMap { input, mappings } => {
            visit(input, bound, names);
            for mapping in mappings {
                visit(mapping, bound, names);
            }
        }
        XPathExpression::For {
            binding,
            binding_expression,
            return_expression,
        }
        | XPathExpression::Let {
            binding,
            binding_expression,
            return_expression,
        } => {
            visit(binding_expression, bound, names);
            let mut inner = bound.clone();
            inner.insert(expanded(binding));
            visit(return_expression, &inner, names);
        }
        XPathExpression::Quantified {
            binding,
            binding_expression,
            satisfies_expression,
            ..
        } => {
            visit(binding_expression, bound, names);
            let mut inner = bound.clone();
            inner.insert(expanded(binding));
            visit(satisfies_expression, &inner, names);
        }
        XPathExpression::If {
            condition,
            then_expression,
            else_expression,
        } => {
            visit_sequence(condition, bound, names);
            visit(then_expression, bound, names);
            visit(else_expression, bound, names);
        }
        XPathExpression::Unsupported { .. } => (),
    }
}
