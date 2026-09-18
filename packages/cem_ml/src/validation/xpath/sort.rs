//! Stable lexicographic sorting. Keys are evaluated once; source items are moved
//! by permutation, preserving their native owners and source maps.
use super::*;
type Result<T> = std::result::Result<T, XPathEvaluationError>;
const CODEPOINT: &str = "http://www.w3.org/2005/xpath-functions/collation/codepoint";

fn type_error(message: &str, range: XPathSourceRange) -> XPathEvaluationError {
    XPathEvaluationError::dynamic(
        "cem.xpath.sort_type_error",
        format!("err:XPTY0004: {message}"),
        range,
    )
}

pub(super) fn evaluate(
    expression: &XPathExpressionAst,
    arguments: &[XPathExpressionNode],
    focus: XPathFocus<'_>,
    bindings: &XPathVariableBindings,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>> {
    let input =
        xpath_evaluate_expression_node(expression, &arguments[0], focus, bindings, runtime)?;
    if let Some(argument) = arguments.get(1) {
        let collation =
            xpath_evaluate_expression_node(expression, argument, focus, bindings, runtime)?;
        let collation = xpath_atomized_items(
            &collation,
            argument.source_range,
            runtime,
            "fn:sort collation",
            "cem.xpath.sort_function_item",
        )?;
        match collation.as_slice() {
            [] => {}
            [XPathResultItem::Atomic { value, .. }]
                if matches!(
                    value.type_name.as_str(),
                    "xs:string" | "xs:untypedAtomic" | "xs:anyURI"
                ) =>
            {
                if value.lexical_value != CODEPOINT {
                    return Err(XPathEvaluationError::dynamic("cem.xpath.collation_unsupported", "err:FOCH0002: this XPath slice supports only the Unicode codepoint collation", argument.source_range));
                }
            }
            _ => {
                return Err(type_error(
                    "fn:sort collation requires zero or one xs:string",
                    argument.source_range,
                ))
            }
        }
    }
    let key_function = if let Some(argument) = arguments.get(2) {
        let mut key =
            xpath_evaluate_expression_node(expression, argument, focus, bindings, runtime)?;
        match key.as_slice() {
            [XPathResultItem::Function {
                arity: 1,
                native_function: Some(_),
                ..
            }]
            | [XPathResultItem::Map { .. }]
            | [XPathResultItem::Array { .. }] => Some(key.remove(0)),
            [XPathResultItem::Function {
                native_function: None,
                ..
            }] => {
                return Err(XPathEvaluationError::dynamic(
                    "cem.xpath.native_function_missing",
                    "fn:sort requires a retained native key function",
                    argument.source_range,
                ))
            }
            _ => {
                return Err(type_error(
                    "fn:sort key must be a function of arity one",
                    argument.source_range,
                ))
            }
        }
    } else {
        None
    };
    let mut keys = Vec::with_capacity(input.len());
    let mut key_items = 0usize;
    let mut key_bytes = 0usize;
    for item in &input {
        runtime.poll(range)?;
        let values = if let Some(key) = &key_function {
            text::charge_items_copy(std::slice::from_ref(item), runtime, range)?;
            functions::invoke(key, vec![vec![item.clone()]], runtime, range)?
        } else {
            vec![item.clone()]
        };
        let atoms = xpath_atomized_items(
            &values,
            range,
            runtime,
            "fn:sort key",
            "cem.xpath.sort_function_item",
        )?;
        key_items = key_items.saturating_add(atoms.len());
        runtime.enforce_sequence_items(key_items, range)?;
        key_bytes = key_bytes.saturating_add(runtime.items_text_bytes(&atoms, range)?);
        runtime.check_text_size(key_bytes, range)?;
        let mut key = Vec::with_capacity(atoms.len());
        for atom in atoms {
            let XPathResultItem::Atomic { value, .. } = atom else {
                unreachable!()
            };
            key.push(xpath_untyped_to_string(xpath_comparable_atomic(
                &value, range,
            )?));
        }
        keys.push(key);
    }
    // Bottom-up merge sort permits comparison diagnostics/cancellation without
    // giving a fallible or inconsistent comparator to Rust's infallible sort.
    let mut order: Vec<usize> = (0..input.len()).collect();
    let mut scratch = vec![0; order.len()];
    let mut width = 1usize;
    while width < order.len() {
        for start in (0..order.len()).step_by(width.saturating_mul(2)) {
            let mid = start.saturating_add(width).min(order.len());
            let end = mid.saturating_add(width).min(order.len());
            let (mut left, mut right) = (start, mid);
            for slot in &mut scratch[start..end] {
                runtime.poll(range)?;
                if left < mid
                    && (right == end
                        || compare(&keys[order[left]], &keys[order[right]], runtime, range)?
                            != Ordering::Greater)
                {
                    *slot = order[left];
                    left += 1;
                } else {
                    *slot = order[right];
                    right += 1;
                }
            }
        }
        std::mem::swap(&mut order, &mut scratch);
        width = width.saturating_mul(2);
    }
    let mut input: Vec<_> = input.into_iter().map(Some).collect();
    let mut result = Vec::with_capacity(input.len());
    for index in order {
        runtime.poll(range)?;
        result.push(input[index].take().expect("permutation"));
    }
    Ok(result)
}

fn nan(value: &XPathComparableAtomic) -> bool {
    match value {
        XPathComparableAtomic::Float(v) => v.is_nan(),
        XPathComparableAtomic::Double(v) => v.is_nan(),
        _ => false,
    }
}
fn compare(
    a: &[XPathComparableAtomic],
    b: &[XPathComparableAtomic],
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Ordering> {
    for (a, b) in a.iter().zip(b) {
        runtime.poll(range)?;
        text::charge_atomic_pair(a, b, runtime, range)?;
        let comparable = matches!(
            (a, b),
            (
                XPathComparableAtomic::String(_),
                XPathComparableAtomic::String(_)
            ) | (
                XPathComparableAtomic::Boolean(_),
                XPathComparableAtomic::Boolean(_)
            )
        ) || (a.is_numeric() && b.is_numeric());
        if !comparable {
            return Err(type_error(
                "fn:sort keys contain incomparable atomic types",
                range,
            ));
        }
        if nan(a) && nan(b) {
            continue;
        }
        if nan(a) {
            return Ok(Ordering::Less);
        }
        if nan(b) {
            return Ok(Ordering::Greater);
        }
        if xpath_compare_atomic(a, b, XPathComparisonRelation::Equal, range)? {
            continue;
        }
        return Ok(
            if xpath_compare_atomic(a, b, XPathComparisonRelation::LessThan, range)? {
                Ordering::Less
            } else {
                Ordering::Greater
            },
        );
    }
    Ok(a.len().cmp(&b.len()))
}
