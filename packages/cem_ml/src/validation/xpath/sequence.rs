//! Bounded sequence selection and duplicate removal over retained XDM items.
use super::*;

const CODEPOINT_COLLATION: &str = "http://www.w3.org/2005/xpath-functions/collation/codepoint";

pub(super) fn evaluate(
    function: XPathNativeFunction,
    expression: &XPathExpressionAst,
    arguments: &[XPathExpressionNode],
    focus: XPathFocus<'_>,
    bindings: &XPathVariableBindings,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>, XPathEvaluationError> {
    let first = &arguments[0];
    let mut items = xpath_evaluate_expression_node(expression, first, focus, bindings, runtime)?;
    match function {
        XPathNativeFunction::Head => {
            items.truncate(1);
            Ok(items)
        }
        XPathNativeFunction::Tail => {
            runtime.charge_work(items.len() as u64, range)?;
            Ok(items.into_iter().skip(1).collect())
        }
        XPathNativeFunction::Reverse => {
            runtime.charge_work(items.len() as u64, range)?;
            items.reverse();
            Ok(items)
        }
        XPathNativeFunction::Subsequence => {
            let mut bounds = Vec::with_capacity(2);
            for argument in &arguments[1..] {
                let value =
                    xpath_evaluate_expression_node(expression, argument, focus, bindings, runtime)?;
                bounds.push(round_bound(double_argument(
                    &value,
                    runtime,
                    argument.source_range,
                )?));
            }
            let start = bounds[0];
            let end = bounds.get(1).map(|length| start + length);
            let mut result = Vec::new();
            for (index, item) in items.into_iter().enumerate() {
                runtime.poll(range)?;
                let position = index as f64 + 1.0;
                if start <= position && end.is_none_or(|end| position < end) {
                    result.push(item);
                }
            }
            Ok(result)
        }
        XPathNativeFunction::DistinctValues => {
            if let Some(argument) = arguments.get(1) {
                let collation =
                    xpath_evaluate_expression_node(expression, argument, focus, bindings, runtime)?;
                let values = xpath_atomized_items(
                    &collation,
                    argument.source_range,
                    runtime,
                    "fn:distinct-values",
                    "cem.xpath.distinct_values_function_item",
                )?;
                match values.as_slice() {
                    [XPathResultItem::Atomic { value, .. }]
                        if matches!(
                            value.type_name.as_str(),
                            "xs:string" | "xs:untypedAtomic" | "xs:anyURI"
                        ) =>
                    {
                        if value.lexical_value != CODEPOINT_COLLATION {
                            return Err(XPathEvaluationError::dynamic("cem.xpath.collation_unsupported",
                                "err:FOCH0002: this XPath slice supports only the Unicode codepoint collation", argument.source_range));
                        }
                    }
                    _ => {
                        return Err(type_error(
                            "fn:distinct-values collation expects one xs:string",
                            argument.source_range,
                        ))
                    }
                }
            }
            distinct(items, runtime, first.source_range)
        }
        _ => unreachable!("sequence dispatch is restricted to sequence functions"),
    }
}

fn type_error(message: &str, range: XPathSourceRange) -> XPathEvaluationError {
    XPathEvaluationError::dynamic(
        "cem.xpath.sequence_argument_type_error",
        format!("err:XPTY0004: {message}"),
        range,
    )
}

fn double_argument(
    items: &[XPathResultItem],
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<f64, XPathEvaluationError> {
    let items = xpath_atomized_items(
        items,
        range,
        runtime,
        "fn:subsequence",
        "cem.xpath.subsequence_function_item",
    )?;
    let [XPathResultItem::Atomic { value, .. }] = items.as_slice() else {
        return Err(type_error(
            "fn:subsequence bounds expect exactly one xs:double",
            range,
        ));
    };
    // Function conversion promotes numerics and casts untypedAtomic; it does not
    // provide fn:number's permissive string/boolean conversion or NaN recovery.
    if value.type_name == "xs:untypedAtomic" {
        let lexical = value.lexical_value.trim_matches([' ', '\t', '\r', '\n']);
        return xpath_parse_cast_double(lexical).ok_or_else(|| {
            XPathEvaluationError::dynamic(
                "cem.xpath.subsequence_bound_invalid",
                "err:FORG0001: fn:subsequence untyped bound is not a valid xs:double",
                range,
            )
        });
    }
    let value = xpath_comparable_atomic(value, range)?;
    if !value.is_numeric() {
        return Err(type_error(
            "fn:subsequence bounds expect xs:double or a promotable numeric value",
            range,
        ));
    }
    xpath_numeric_as_double(&value, range)
}

fn round_bound(value: f64) -> f64 {
    if !value.is_finite() {
        return value;
    }
    let floor = value.floor();
    // Adding 0.5 before flooring incorrectly rounds the representable number
    // immediately below 0.5. XPath's ties go toward positive infinity.
    if value - floor < 0.5 {
        floor
    } else {
        floor + 1.0
    }
}

fn distinct(
    items: Vec<XPathResultItem>,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>, XPathEvaluationError> {
    let items = xpath_atomized_items(
        &items,
        range,
        runtime,
        "fn:distinct-values",
        "cem.xpath.distinct_values_function_item",
    )?;
    let mut seen = Vec::new();
    let mut result = Vec::new();
    for item in items {
        runtime.poll(range)?;
        let XPathResultItem::Atomic { value, .. } = &item else {
            unreachable!("atomization returns atomics")
        };
        runtime.read_text(&value.lexical_value, range)?;
        let candidate = match xpath_comparable_atomic(value, range)? {
            XPathComparableAtomic::Untyped(text) => XPathComparableAtomic::String(text),
            value => value,
        };
        let mut duplicate = false;
        for previous in &seen {
            runtime.poll(range)?;
            text::charge_atomic_pair(previous, &candidate, runtime, range)?;
            let comparable = matches!(
                (previous, &candidate),
                (
                    XPathComparableAtomic::String(_),
                    XPathComparableAtomic::String(_)
                ) | (
                    XPathComparableAtomic::Boolean(_),
                    XPathComparableAtomic::Boolean(_)
                )
            ) || (previous.is_numeric() && candidate.is_numeric());
            if (is_nan(previous) && is_nan(&candidate))
                || (comparable
                    && xpath_compare_atomic(
                        previous,
                        &candidate,
                        XPathComparisonRelation::Equal,
                        range,
                    )?)
            {
                duplicate = true;
                break;
            }
        }
        if !duplicate {
            // Keep original atomic type/provenance. First encountered representatives
            // are this implementation's choice, not a portable display-order contract.
            seen.push(candidate);
            result.push(item);
        }
    }
    Ok(result)
}
fn is_nan(value: &XPathComparableAtomic) -> bool {
    match value {
        XPathComparableAtomic::Float(v) => v.is_nan(),
        XPathComparableAtomic::Double(v) => v.is_nan(),
        _ => false,
    }
}
