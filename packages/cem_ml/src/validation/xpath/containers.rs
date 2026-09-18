//! Native XDM containers. Member sequences stay intact until an explicit lookup.
use super::*;

type Result<T> = std::result::Result<T, XPathEvaluationError>;
fn type_error(message: &str, range: XPathSourceRange) -> XPathEvaluationError {
    XPathEvaluationError::dynamic(
        "cem.xpath.container_type_error",
        format!("err:XPTY0004: {message}"),
        range,
    )
}
fn sequence(items: Vec<XPathResultItem>) -> XPathResultSequence {
    XPathResultSequence {
        sequence_type: xpath_result_sequence_type(&items),
        items,
    }
}
fn origin(expression: &XPathExpressionAst, range: XPathSourceRange) -> SourceMapStack {
    range.source_map(
        expression.attachment.source_id(),
        &expression.source.media_type,
    )
}
fn atomics(
    items: &[XPathResultItem],
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>> {
    xpath_atomized_items(
        items,
        range,
        runtime,
        "map/array key",
        "cem.xpath.container_key_atomization",
    )
}
fn one_key(
    items: &[XPathResultItem],
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<XPathAtomicValue> {
    let mut items = atomics(items, runtime, range)?;
    if items.len() != 1 {
        return Err(type_error(
            "a map/array key must atomize to exactly one atomic value",
            range,
        ));
    }
    let XPathResultItem::Atomic { value, .. } = items.remove(0) else {
        unreachable!()
    };
    Ok(value)
}

// op:same-key must be transitive: do not reuse promotion-based eq/distinct-values.
#[derive(PartialEq)]
enum Key {
    String(String),
    Boolean(bool),
    Decimal(XPathExactDecimal),
    NaN,
    PositiveInfinity,
    NegativeInfinity,
}
fn key(
    value: &XPathAtomicValue,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Key> {
    runtime.read_text(&value.lexical_value, range)?;
    Ok(match xpath_comparable_atomic(value, range)? {
        XPathComparableAtomic::String(v) | XPathComparableAtomic::Untyped(v) => Key::String(v),
        XPathComparableAtomic::Boolean(v) => Key::Boolean(v),
        XPathComparableAtomic::Integer(v) | XPathComparableAtomic::Decimal(v) => Key::Decimal(v),
        XPathComparableAtomic::Float(v) => floating_key(f64::from(v), runtime, range)?,
        XPathComparableAtomic::Double(v) => floating_key(v, runtime, range)?,
    })
}
fn floating_key(
    value: f64,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Key> {
    if value.is_nan() {
        return Ok(Key::NaN);
    }
    if value == f64::INFINITY {
        return Ok(Key::PositiveInfinity);
    }
    if value == f64::NEG_INFINITY {
        return Ok(Key::NegativeInfinity);
    }
    if value == 0.0 {
        return Ok(Key::Decimal(XPathExactDecimal::from_u64(0)));
    }
    let encoded = ((value.to_bits() >> 52) & 0x7ff) as i32;
    let steps = if encoded == 0 {
        1074
    } else {
        (encoded - 1023 - 52).unsigned_abs()
    } as u64;
    // Bound repeated multiplication before expanding the finite IEEE value.
    runtime.charge_work(steps.saturating_mul(steps + 20), range)?;
    Ok(Key::Decimal(
        xpath_exact_decimal_from_f64(value).expect("finite IEEE decimal"),
    ))
}
fn same_key(
    a: &Key,
    b: &Key,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<bool> {
    let size = |v: &Key| match v {
        Key::String(s) => s.len(),
        Key::Decimal(d) => d.coefficient.len(),
        _ => 1,
    };
    runtime.charge_work(size(a).saturating_add(size(b)) as u64, range)?;
    Ok(a == b)
}
fn find<'a>(
    entries: &'a [XPathMapEntry],
    value: &XPathAtomicValue,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Option<&'a XPathResultSequence>> {
    let candidate = key(value, runtime, range)?;
    for entry in entries {
        runtime.poll(range)?;
        let previous = key(&entry.key, runtime, range)?;
        if same_key(&candidate, &previous, runtime, range)? {
            return Ok(Some(&entry.value));
        }
    }
    Ok(None)
}
fn index(
    value: &XPathAtomicValue,
    size: usize,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<usize> {
    runtime.read_text(&value.lexical_value, range)?;
    let integer = if value.type_name == "xs:untypedAtomic" {
        let lexical = value.lexical_value.trim_matches([' ', '\t', '\r', '\n']);
        if lexical.chars().any(char::is_whitespace) {
            None
        } else {
            XPathExactDecimal::parse(lexical, false)
        }
        .ok_or_else(|| {
            XPathEvaluationError::dynamic(
                "cem.xpath.array_index_cast_invalid",
                "err:FORG0001: array index is not an xs:integer",
                range,
            )
        })?
    } else {
        let XPathComparableAtomic::Integer(value) = xpath_comparable_atomic(value, range)? else {
            return Err(type_error("an array index must be xs:integer", range));
        };
        value
    };
    integer
        .to_usize()
        .filter(|i| *i >= 1 && *i <= size)
        .map(|i| i - 1)
        .ok_or_else(|| {
            XPathEvaluationError::dynamic(
                "cem.xpath.array_index_out_of_bounds",
                "err:FOAY0001: array index is outside 1 to array:size",
                range,
            )
        })
}
fn get<'a>(
    container: &'a XPathResultItem,
    value: &XPathAtomicValue,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Option<&'a XPathResultSequence>> {
    match container {
        XPathResultItem::Map { entries, .. } => find(entries, value, runtime, range),
        XPathResultItem::Array { members, .. } => {
            Ok(Some(&members[index(value, members.len(), runtime, range)?]))
        }
        _ => Err(type_error("lookup requires a map or array", range)),
    }
}
fn append(
    result: &mut Vec<XPathResultItem>,
    bytes: &mut usize,
    member: &XPathResultSequence,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<()> {
    runtime.enforce_sequence_items(result.len().saturating_add(member.items.len()), range)?;
    text::charge_items_copy(&member.items, runtime, range)?;
    runtime.append_items(result, bytes, member.items.clone(), range)
}

pub(super) fn primary(
    expression: &XPathExpressionAst,
    primary: &XPathPrimaryExpression,
    focus: XPathFocus<'_>,
    bindings: &XPathVariableBindings,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>> {
    let source_map = origin(expression, range);
    let mut bytes = 0usize;
    let item = match primary {
        XPathPrimaryExpression::MapConstructor { entries: authored } => {
            let mut entries = Vec::new();
            let mut keys = Vec::new();
            for entry in authored {
                runtime.poll(entry.source_range)?;
                let items = xpath_evaluate_expression_node(
                    expression, &entry.key, focus, bindings, runtime,
                )?;
                let value = one_key(&items, runtime, entry.key.source_range)?;
                let candidate = key(&value, runtime, entry.key.source_range)?;
                for previous in &keys {
                    if same_key(previous, &candidate, runtime, entry.key.source_range)? {
                        return Err(XPathEvaluationError::dynamic(
                            "cem.xpath.map_duplicate_key",
                            "err:XQDY0137: duplicate map key",
                            entry.key.source_range,
                        ));
                    }
                }
                bytes = bytes.saturating_add(value.lexical_value.len());
                runtime.check_text_size(bytes, entry.key.source_range)?;
                let items = xpath_evaluate_expression_node(
                    expression,
                    &entry.value,
                    focus,
                    bindings,
                    runtime,
                )?;
                bytes = bytes
                    .saturating_add(runtime.items_text_bytes(&items, entry.value.source_range)?);
                runtime.check_text_size(bytes, entry.value.source_range)?;
                keys.push(candidate);
                entries.push(XPathMapEntry {
                    key: value,
                    value: sequence(items),
                });
            }
            XPathResultItem::Map {
                entries,
                source_map,
            }
        }
        XPathPrimaryExpression::ArrayConstructor(array) => {
            let mut members = Vec::new();
            match array {
                XPathArrayConstructor::Square(expressions) => {
                    for node in &expressions.expressions {
                        runtime.poll(node.source_range)?;
                        let items = xpath_evaluate_expression_node(
                            expression, node, focus, bindings, runtime,
                        )?;
                        bytes = bytes
                            .saturating_add(runtime.items_text_bytes(&items, node.source_range)?);
                        runtime.check_text_size(bytes, node.source_range)?;
                        members.push(sequence(items));
                    }
                }
                XPathArrayConstructor::Curly(Some(expressions)) => {
                    let items = xpath_evaluate_expression_sequence(
                        expression,
                        expressions,
                        focus,
                        bindings,
                        runtime,
                    )?
                    .items;
                    for item in items {
                        runtime.poll(range)?;
                        members.push(sequence(vec![item]));
                    }
                }
                XPathArrayConstructor::Curly(None) => {}
            }
            XPathResultItem::Array {
                members,
                source_map,
            }
        }
        XPathPrimaryExpression::UnaryLookup(key) => {
            let item = focus.context_item.ok_or_else(|| {
                XPathEvaluationError::dynamic(
                    "cem.xpath.context_item_missing",
                    "err:XPDY0002: unary lookup requires a context item",
                    range,
                )
            })?;
            return lookup(
                expression,
                std::slice::from_ref(item),
                key,
                focus,
                bindings,
                runtime,
                range,
            );
        }
        _ => unreachable!(),
    };
    Ok(vec![item])
}

pub(super) fn lookup(
    expression: &XPathExpressionAst,
    containers: &[XPathResultItem],
    key: &XPathLookupKey,
    focus: XPathFocus<'_>,
    bindings: &XPathVariableBindings,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>> {
    let keys = match key {
        XPathLookupKey::Name(value) | XPathLookupKey::Integer(value) => {
            vec![xpath_atomic_result_item(
                expression,
                range,
                XPathAtomicValue {
                    type_name: if matches!(key, XPathLookupKey::Name(_)) {
                        "xs:string"
                    } else {
                        "xs:integer"
                    }
                    .into(),
                    lexical_value: runtime.copy_text(value, range)?,
                    namespace_uri: None,
                    local_name: None,
                },
            )]
        }
        XPathLookupKey::Expression(Some(expr)) => {
            let items =
                xpath_evaluate_expression_sequence(expression, expr, focus, bindings, runtime)?
                    .items;
            atomics(&items, runtime, expr.source_range)?
        }
        XPathLookupKey::Expression(None) | XPathLookupKey::Wildcard => Vec::new(),
    };
    let mut result = Vec::new();
    let mut bytes = 0;
    for container in containers {
        runtime.poll(range)?;
        if !matches!(
            container,
            XPathResultItem::Map { .. } | XPathResultItem::Array { .. }
        ) {
            return Err(type_error("lookup requires maps or arrays", range));
        }
        if matches!(key, XPathLookupKey::Wildcard) {
            match container {
                XPathResultItem::Map { entries, .. } => {
                    for entry in entries {
                        runtime.poll(range)?;
                        append(&mut result, &mut bytes, &entry.value, runtime, range)?;
                    }
                }
                XPathResultItem::Array { members, .. } => {
                    for member in members {
                        runtime.poll(range)?;
                        append(&mut result, &mut bytes, member, runtime, range)?;
                    }
                }
                _ => unreachable!(),
            }
        } else {
            for item in &keys {
                runtime.poll(range)?;
                let XPathResultItem::Atomic { value, .. } = item else {
                    unreachable!()
                };
                if let Some(member) = get(container, value, runtime, range)? {
                    append(&mut result, &mut bytes, member, runtime, range)?;
                }
            }
        }
    }
    Ok(result)
}

pub(super) fn call(
    expression: &XPathExpressionAst,
    containers: &[XPathResultItem],
    arguments: &[XPathExpressionNode],
    focus: XPathFocus<'_>,
    bindings: &XPathVariableBindings,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>> {
    let [container] = containers else {
        return Err(type_error(
            "container call requires one map or array",
            range,
        ));
    };
    if matches!(container, XPathResultItem::Function { .. }) {
        return Err(XPathEvaluationError::unsupported(
            "General dynamic function calls are outside this container slice",
            range,
        ));
    }
    if !matches!(
        container,
        XPathResultItem::Map { .. } | XPathResultItem::Array { .. }
    ) {
        return Err(type_error("container call requires a map or array", range));
    }
    let [argument] = arguments else {
        return Err(type_error(
            "a map/array function takes exactly one key argument",
            range,
        ));
    };
    let items = xpath_evaluate_expression_node(expression, argument, focus, bindings, runtime)?;
    call_value(container, &items, runtime, argument.source_range)
}

pub(super) fn call_value(container: &XPathResultItem, items: &[XPathResultItem], runtime: &mut XPathEvaluationRuntime, range: XPathSourceRange) -> Result<Vec<XPathResultItem>> {
    let key = one_key(items, runtime, range)?;
    let mut result = Vec::new();
    if let Some(member) = get(container, &key, runtime, range)? {
        append(&mut result, &mut 0, member, runtime, range)?;
    }
    Ok(result)
}

pub(super) fn evaluate(
    function: XPathNativeFunction,
    expression: &XPathExpressionAst,
    arguments: &[XPathExpressionNode],
    focus: XPathFocus<'_>,
    bindings: &XPathVariableBindings,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>> {
    let first = &arguments[0];
    let items = xpath_evaluate_expression_node(expression, first, focus, bindings, runtime)?;
    let [container] = items.as_slice() else {
        return Err(type_error(
            "expected exactly one map or array",
            first.source_range,
        ));
    };
    let map_function = matches!(
        function,
        XPathNativeFunction::MapContains
            | XPathNativeFunction::MapGet
            | XPathNativeFunction::MapKeys
    );
    if !matches!(
        (container, map_function),
        (XPathResultItem::Map { .. }, true) | (XPathResultItem::Array { .. }, false)
    ) {
        return Err(type_error(
            if map_function {
                "expected map(*)"
            } else {
                "expected array(*)"
            },
            first.source_range,
        ));
    }
    if let XPathNativeFunction::ArraySize = function {
        let XPathResultItem::Array { members, .. } = container else {
            unreachable!()
        };
        return Ok(vec![xpath_numeric_result_item(
            expression,
            range,
            XPathComparableAtomic::Integer(XPathExactDecimal::from_u64(members.len() as u64)),
            runtime,
        )?]);
    }
    if let XPathNativeFunction::MapKeys = function {
        let XPathResultItem::Map {
            entries,
            source_map,
        } = container
        else {
            unreachable!()
        };
        runtime.enforce_sequence_items(entries.len(), range)?;
        let mut result = Vec::new();
        for entry in entries {
            runtime.poll(range)?;
            runtime.read_text(&entry.key.lexical_value, range)?;
            result.push(XPathResultItem::Atomic {
                value: entry.key.clone(),
                source_map: source_map.clone(),
            });
        }
        return Ok(result);
    }
    let argument = &arguments[1];
    let keys = xpath_evaluate_expression_node(expression, argument, focus, bindings, runtime)?;
    let key = one_key(&keys, runtime, argument.source_range)?;
    let member = get(container, &key, runtime, argument.source_range)?;
    if function == XPathNativeFunction::MapContains {
        return Ok(vec![xpath_boolean_result_item(
            expression,
            range,
            member.is_some(),
        )]);
    }
    let mut result = Vec::new();
    if let Some(member) = member {
        append(&mut result, &mut 0, member, runtime, range)?;
    }
    Ok(result)
}
