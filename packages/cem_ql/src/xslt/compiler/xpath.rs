//! XSLT expression adaptations made from typed XPath nodes. The only parsed
//! text below is a compiler-owned standard-XPath macro or declaration name,
//! never rewritten authored XPath and never a runtime document.
use cem_ml::validation::xpath::*;
use std::collections::BTreeMap;

pub(super) enum ResultKind {
    Sequence,
    Boolean,
    Text(String),
}
const FN: &str = "http://www.w3.org/2005/xpath-functions";

// XSLT 3.0 §5.7.2: remove empty text nodes, merge adjacent text nodes,
// atomize, cast to strings, then join. Arrays retain their member boundaries
// until fn:data atomizes them; an empty array still separates text-node runs.
// Index arrays avoid repeated predicate scans for text-node groups.
const SIMPLE_CONTENT: &str = r#"
let $s := array { $input[not(. instance of text() and string(.) = '')] }
return let $starts := array {
    (for $i in 1 to array:size($s)
     return if ($i = 1 or not($s?($i) instance of text() and $s?($i - 1) instance of text()))
            then $i else ()),
    array:size($s) + 1
}
return string-join(
    for $g in 1 to (array:size($starts) - 1)
    return let $first := $starts?($g)
    return if ($s?($first) instance of text())
           then string-join(for $j in $first to ($starts?($g + 1) - 1)
                            return string($s?($j)), '')
           else data($s?($first)) ! string(.),
    $separator)
"#;

pub(super) fn expanded(name: &XPathExpandedName) -> String {
    name.namespace_uri
        .as_ref()
        .map(|uri| format!("Q{{{uri}}}{}", name.local_name))
        .unwrap_or_else(|| name.local_name.clone())
}
pub(super) fn variable_name(
    lexical: &str,
    expression: &XPathExpressionAst,
) -> Result<XPathExpandedName, String> {
    // Ask the XPath grammar to validate and expand the declaration's name.
    // This is an authoring scalar, not rewriting an authored XPath program.
    let source = format!("${}", lexical.trim());
    let parsed = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: source.as_bytes(),
            source_uri: &expression.source.uri,
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        expression.attachment.clone(),
    );
    let syntax = parsed
        .syntax_ast
        .ok_or("variable name must be a declared EQName")?;
    if let [XPathExpressionNode {
        expression:
            XPathExpression::Path(XPathPathExpression {
                root: XPathPathRoot::Relative,
                steps,
                ..
            }),
        ..
    }] = syntax.root.expressions.as_slice()
    {
        if let [XPathStepNode {
            step: XPathStep::Primary(XPathPrimaryExpression::VariableReference(name)),
            ..
        }] = steps.as_slice()
        {
            return Ok(XPathExpandedName::new(
                name.namespace_uri.clone(),
                name.local_name.clone(),
            ));
        }
    }
    Err("variable name must be a declared EQName".into())
}

pub(super) fn primary(
    value: XPathPrimaryExpression,
    range: XPathSourceRange,
) -> XPathExpressionNode {
    XPathExpressionNode {
        source_range: range,
        expression: XPathExpression::Path(XPathPathExpression {
            root: XPathPathRoot::Relative,
            source_range: range,
            steps: vec![XPathStepNode {
                step: XPathStep::Primary(value),
                source_range: range,
            }],
        }),
    }
}
fn sequence_node(sequence: XPathExpressionSequence) -> XPathExpressionNode {
    let range = sequence.source_range;
    primary(
        XPathPrimaryExpression::Parenthesized(Some(Box::new(sequence))),
        range,
    )
}
fn name(local: &str, range: XPathSourceRange) -> XPathName {
    XPathName {
        lexical: local.into(),
        prefix: None,
        local_name: local.into(),
        namespace_uri: None,
        source_range: range,
    }
}
fn bind(
    local: &str,
    value: XPathExpressionNode,
    body: XPathExpressionNode,
    range: XPathSourceRange,
) -> XPathExpressionNode {
    XPathExpressionNode {
        source_range: range,
        expression: XPathExpression::Let {
            binding: name(local, range),
            binding_expression: Box::new(value),
            return_expression: Box::new(body),
        },
    }
}

// Only compiler-owned macro nodes are reanchored. Authored nodes are spliced
// afterward and retain their exact ranges. Fail closed if the macro gains a
// construct this small visitor does not support.
pub(super) fn anchor_sequence(
    sequence: &mut XPathExpressionSequence,
    range: XPathSourceRange,
) -> Result<(), String> {
    sequence.source_range = range;
    for node in &mut sequence.expressions {
        anchor_node(node, range)?;
    }
    Ok(())
}
fn anchor_primary(
    primary: &mut XPathPrimaryExpression,
    range: XPathSourceRange,
) -> Result<(), String> {
    match primary {
        XPathPrimaryExpression::Literal(_) | XPathPrimaryExpression::ContextItem => (),
        XPathPrimaryExpression::VariableReference(name) => name.source_range = range,
        XPathPrimaryExpression::Parenthesized(sequence)
        | XPathPrimaryExpression::ArrayConstructor(XPathArrayConstructor::Curly(sequence)) => {
            if let Some(sequence) = sequence {
                anchor_sequence(sequence, range)?;
            }
        }
        XPathPrimaryExpression::FunctionCall { name, arguments } => {
            name.source_range = range;
            for argument in arguments {
                anchor_node(argument, range)?;
            }
        }
        XPathPrimaryExpression::ArrayConstructor(XPathArrayConstructor::Square(sequence)) => {
            anchor_sequence(sequence, range)?;
        }
        XPathPrimaryExpression::InlineFunction {
            parameters,
            result_type: None,
            body,
        } => {
            for parameter in parameters {
                if parameter.sequence_type.is_some() {
                    return Err("typed compiler macro parameter is unsupported".into());
                }
                parameter.name.source_range = range;
            }
            if let Some(body) = body {
                anchor_sequence(body, range)?;
            }
        }
        _ => return Err("unsupported compiler macro primary".into()),
    }
    Ok(())
}
fn anchor_node(node: &mut XPathExpressionNode, range: XPathSourceRange) -> Result<(), String> {
    node.source_range = range;
    match &mut node.expression {
        XPathExpression::Path(path) => {
            path.source_range = range;
            for step in &mut path.steps {
                step.source_range = range;
                match &mut step.step {
                    XPathStep::Primary(primary) => anchor_primary(primary, range)?,
                    XPathStep::Postfix { primary, postfixes } => {
                        anchor_primary(primary, range)?;
                        for postfix in postfixes {
                            match postfix {
                                XPathPostfixExpression::Predicate(sequence) => {
                                    anchor_sequence(sequence, range)?
                                }
                                XPathPostfixExpression::Lookup {
                                    key: XPathLookupKey::Expression(sequence),
                                } => {
                                    if let Some(sequence) = sequence {
                                        anchor_sequence(sequence, range)?;
                                    }
                                }
                                XPathPostfixExpression::Lookup { .. } => (),
                                _ => return Err("unsupported compiler macro postfix".into()),
                            }
                        }
                    }
                    XPathStep::Axis {
                        node_test,
                        predicates,
                        ..
                    } => {
                        if let XPathNodeTest::Name(XPathNameTest::Name(name)) = node_test {
                            name.source_range = range;
                        }
                        for predicate in predicates {
                            anchor_sequence(predicate, range)?;
                        }
                    }
                }
            }
        }
        XPathExpression::Binary { left, right, .. } => {
            anchor_node(left, range)?;
            anchor_node(right, range)?;
        }
        XPathExpression::SimpleMap { input, mappings } => {
            anchor_node(input, range)?;
            for mapping in mappings {
                anchor_node(mapping, range)?;
            }
        }
        XPathExpression::Let {
            binding,
            binding_expression,
            return_expression,
        }
        | XPathExpression::For {
            binding,
            binding_expression,
            return_expression,
        } => {
            binding.source_range = range;
            anchor_node(binding_expression, range)?;
            anchor_node(return_expression, range)?;
        }
        XPathExpression::If {
            condition,
            then_expression,
            else_expression,
        } => {
            anchor_sequence(condition, range)?;
            anchor_node(then_expression, range)?;
            anchor_node(else_expression, range)?;
        }
        XPathExpression::InstanceOf {
            operand,
            sequence_type:
                XPathSequenceType::Item {
                    item_type: XPathSequenceItemType::Atomic(name),
                    source_range,
                    ..
                },
        } => {
            name.source_range = range;
            *source_range = range;
            anchor_node(operand, range)?;
        }
        XPathExpression::InstanceOf {
            operand,
            sequence_type:
                XPathSequenceType::Item {
                    item_type:
                        XPathSequenceItemType::Kind {
                            source_range: item_range,
                            ..
                        },
                    source_range,
                    ..
                },
        } => {
            *source_range = range;
            *item_range = range;
            anchor_node(operand, range)?;
        }
        _ => return Err("unsupported compiler macro expression".into()),
    }
    Ok(())
}

// First-seen distinct-values selects representatives with XPath promotion,
// unlike map same-key or CEM identity. Assign each key to the FIRST matching
// representative: it existed by that key's population position, including
// non-transitive numeric cases (§14.5). Per-record indices remove duplicate
// keys without removing duplicate population positions or native node owners.
const GROUP_BY: &str = r#"
let $keys := array { distinct-values(for $record in $records?* return $record?(2)) }
return let $assigned := array {
    for $record in $records?*
    return [$record?(1), distinct-values(
        for $key in $record?(2)
        return head(for $i in 1 to array:size($keys)
                    return if (count(distinct-values(($key, $keys?($i)))) = 1)
                           then $i else ()))]
}
return for $i in 1 to array:size($keys)
       return [$keys?($i), for $record in $assigned?*
                          return if ($record?(2) = $i) then $record?(1) else ()]
"#;

fn macro_node(code: &str, range: XPathSourceRange) -> Result<XPathExpressionNode, String> {
    let parsed = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: code.as_bytes(),
            source_uri: "memory:xslt-macro",
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        XPathAttachment::StandaloneStaticContext {
            source_id: 1,
            static_context: XPathStaticContext::default(),
        },
    );
    let mut sequence = parsed
        .syntax_ast
        .ok_or_else(|| format!("invalid grouping macro: {:?}", parsed.facts))?
        .root;
    anchor_sequence(&mut sequence, range)?;
    Ok(sequence_node(sequence))
}

// Compiler-owned programs use fixed standard namespaces, independently of
// authored prefix rebinding. Their arguments are native values, not XPath text.
pub(super) fn compiler_macro(target: &mut XPathExpressionAst, code: &str) -> Result<(), String> {
    let range = target
        .syntax_ast
        .as_ref()
        .ok_or("missing compiler XPath")?
        .root
        .source_range;
    target.syntax_ast = Some(XPathSyntaxAst {
        root: XPathExpressionSequence {
            expressions: vec![macro_node(code, range)?],
            source_range: range,
        },
        events: Vec::new(),
    });
    target.tokens.clear();
    target.events.clear();
    target.facts.clear();
    target.source_text = None;
    Ok(())
}

pub(super) fn group_by(
    target: &mut XPathExpressionAst,
    select: XPathExpressionAst,
    key: XPathExpressionAst,
) -> Result<(), String> {
    let range = target
        .syntax_ast
        .as_ref()
        .ok_or("missing compiler XPath")?
        .root
        .source_range;
    let select = select.syntax_ast.ok_or("missing typed population")?.root;
    let key = key.syntax_ast.ok_or("missing typed key")?.root;
    let key_range = key.source_range;
    let normalized = bind(
        "input",
        sequence_node(key),
        macro_node(
            "data($input) ! (if (. instance of xs:untypedAtomic) then string(.) else .)",
            key_range,
        )?,
        key_range,
    );
    let record = primary(
        XPathPrimaryExpression::ArrayConstructor(XPathArrayConstructor::Square(
            XPathExpressionSequence {
                source_range: range,
                expressions: vec![
                    primary(XPathPrimaryExpression::ContextItem, range),
                    normalized,
                ],
            },
        )),
        range,
    );
    let records = primary(
        XPathPrimaryExpression::ArrayConstructor(XPathArrayConstructor::Curly(Some(Box::new(
            XPathExpressionSequence {
                source_range: range,
                expressions: vec![XPathExpressionNode {
                    source_range: range,
                    expression: XPathExpression::SimpleMap {
                        input: Box::new(sequence_node(select)),
                        mappings: vec![record],
                    },
                }],
            },
        )))),
        range,
    );
    // Every authored expression is evaluated before compiler-local variables
    // enter scope; generated names cannot capture authored variable references.
    let body = bind("records", records, macro_node(GROUP_BY, range)?, range);
    target.syntax_ast = Some(XPathSyntaxAst {
        root: XPathExpressionSequence {
            expressions: vec![body],
            source_range: range,
        },
        events: Vec::new(),
    });
    target.tokens.clear();
    target.events.clear();
    target.facts.clear();
    target.source_text = None;
    Ok(())
}

pub(super) fn adapt(expression: &mut XPathExpressionAst, kind: ResultKind) -> Result<(), String> {
    if matches!(kind, ResultKind::Sequence) {
        return Ok(());
    }
    let original = expression
        .syntax_ast
        .as_ref()
        .ok_or("missing typed XPath")?
        .root
        .clone();
    let range = original.source_range;
    let input = sequence_node(original);
    let body = match kind {
        ResultKind::Sequence => unreachable!(),
        ResultKind::Boolean => primary(
            XPathPrimaryExpression::FunctionCall {
                name: XPathName {
                    namespace_uri: Some(FN.into()),
                    lexical: format!("Q{{{FN}}}boolean"),
                    ..name("boolean", range)
                },
                arguments: vec![input],
            },
            range,
        ),
        ResultKind::Text(separator) => {
            let parsed = xpath_expression_ast_from_source_bytes(
                XPathSourceRequest {
                    bytes: SIMPLE_CONTENT.as_bytes(),
                    source_uri: &expression.source.uri,
                    content_type: Some(XPATH_CONTENT_TYPE),
                    source_range_projector: None,
                },
                XPathAttachment::StandaloneStaticContext {
                    source_id: 1,
                    static_context: XPathStaticContext {
                        namespaces: BTreeMap::from([("array".into(), format!("{FN}/array"))]),
                        variable_bindings: BTreeMap::from([
                            ("input".into(), "item()*".into()),
                            ("separator".into(), "xs:string".into()),
                        ]),
                        ..Default::default()
                    },
                },
            );
            let mut root = parsed
                .syntax_ast
                .ok_or_else(|| {
                    format!("invalid compiler simple-content macro: {:?}", parsed.facts)
                })?
                .root;
            anchor_sequence(&mut root, range)?;
            let body = sequence_node(root);
            let separator = primary(
                XPathPrimaryExpression::Literal(XPathLiteral {
                    kind: XPathLiteralKind::String,
                    lexical: format!("'{}'", separator.replace('\'', "''")),
                    value: separator,
                }),
                range,
            );
            // Input is evaluated before these compiler-local bindings exist,
            // so authored variables with these spellings are not captured.
            bind(
                "input",
                input,
                bind("separator", separator, body, range),
                range,
            )
        }
    };
    expression.syntax_ast = Some(XPathSyntaxAst {
        root: XPathExpressionSequence {
            expressions: vec![body],
            source_range: range,
        },
        events: vec![],
    });
    // No token/tree disagreement survives into the compiler-owned program.
    expression.source_text = None;
    expression.tokens.clear();
    expression.events.clear();
    expression.facts.clear();
    Ok(())
}
