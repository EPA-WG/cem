//! Numeric aggregate subset, retaining the shared atomic and limit contracts.
use super::*;

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
    let items = xpath_evaluate_expression_node(expression, first, focus, bindings, runtime)?;
    let items = xpath_atomized_items(
        &items,
        first.source_range,
        runtime,
        "numeric aggregate",
        "cem.xpath.aggregate_function_item",
    )?;
    let mut zero = None;
    if let Some(argument) = arguments.get(1) {
        let value = xpath_evaluate_expression_node(expression, argument, focus, bindings, runtime)?;
        let values = xpath_atomized_items(
            &value,
            argument.source_range,
            runtime,
            "numeric aggregate",
            "cem.xpath.aggregate_function_item",
        )?;
        if function == XPathNativeFunction::Sum {
            if values.len() > 1 {
                return Err(argument_error(
                    "fn:sum zero must be zero or one atomic value",
                    argument.source_range,
                ));
            }
            zero = Some(values);
        } else {
            // F&O ignores the collation for non-string inputs. Still apply its
            // xs:string function-conversion/cardinality contract, even when empty.
            if !matches!(values.as_slice(), [XPathResultItem::Atomic { value, .. }]
                if matches!(value.type_name.as_str(), "xs:string" | "xs:anyURI" | "xs:untypedAtomic"))
            {
                return Err(argument_error(
                    "fn:min/max collation must be one xs:string",
                    argument.source_range,
                ));
            }
        }
    }
    if items.is_empty() {
        return if function == XPathNativeFunction::Sum {
            match zero {
                Some(value) => Ok(value),
                None => Ok(vec![xpath_numeric_result_item(
                    expression,
                    range,
                    XPathComparableAtomic::Integer(XPathExactDecimal::from_u64(0)),
                    runtime,
                )?]),
            }
        } else {
            Ok(Vec::new())
        };
    }

    let mut values = Vec::with_capacity(items.len());
    let mut numeric = false;
    let mut nonnumeric = None;
    let mut mixed_nonnumeric = false;
    let mut rank = 0;
    for item in items {
        runtime.poll(first.source_range)?;
        let XPathResultItem::Atomic { value, .. } = item else {
            unreachable!("atomized")
        };
        runtime.read_text(&value.lexical_value, first.source_range)?;
        let value = if value.type_name == "xs:untypedAtomic" {
            XPathComparableAtomic::Double(
                xpath_parse_cast_double(value.lexical_value.trim_matches([' ', '\t', '\r', '\n']))
                    .ok_or_else(|| {
                        XPathEvaluationError::dynamic(
                            "cem.xpath.aggregate_cast_invalid",
                            "err:FORG0001: aggregate untyped value cannot be cast to xs:double",
                            first.source_range,
                        )
                    })?,
            )
        } else {
            xpath_comparable_atomic(&value, first.source_range)?
        };
        if value.is_numeric() {
            numeric = true;
            rank = rank.max(match value {
                XPathComparableAtomic::Double(_) => 2,
                XPathComparableAtomic::Float(_) => 1,
                _ => 0,
            });
        } else {
            mixed_nonnumeric |= nonnumeric.is_some_and(|ty| ty != value.type_name());
            nonnumeric = Some(value.type_name());
        }
        values.push(value);
    }
    if nonnumeric.is_some() {
        return Err(
            if numeric
                || mixed_nonnumeric
                || matches!(
                    function,
                    XPathNativeFunction::Sum | XPathNativeFunction::Avg
                )
            {
                XPathEvaluationError::dynamic("cem.xpath.aggregate_type_error",
                "err:FORG0006: aggregate values have incompatible types or do not support numeric addition", first.source_range)
            } else {
                XPathEvaluationError::dynamic("cem.xpath.aggregate_type_unsupported",
                "This fn:min/max slice supports numeric values only; standard nonnumeric aggregate types are not implemented", first.source_range)
            },
        );
    }

    // Promote the complete sequence before reducing. Pairwise promotion during
    // a fold could lose precision or overflow before a later double is seen.
    let count = values.len();
    let mut converted_bytes = 0usize;
    for value in &mut values {
        runtime.poll(first.source_range)?;
        text::charge_atomic_pair(value, value, runtime, first.source_range)?;
        *value = match rank {
            2 => XPathComparableAtomic::Double(xpath_numeric_as_double(value, first.source_range)?),
            1 => XPathComparableAtomic::Float(xpath_numeric_as_float(value, first.source_range)?),
            _ => continue,
        };
        converted_bytes = converted_bytes.saturating_add(numeric_bytes(value));
        runtime.check_text_size(converted_bytes, first.source_range)?;
    }
    let mut values = values.into_iter();
    let mut result = values.next().expect("nonempty");
    for value in values {
        runtime.poll(range)?;
        text::charge_atomic_pair(&result, &value, runtime, range)?;
        result = match function {
            XPathNativeFunction::Sum | XPathNativeFunction::Avg => {
                let value = xpath_numeric_binary(
                    result,
                    value,
                    XPathBinaryOperator::Add,
                    first.source_range,
                    first.source_range,
                    range,
                )?;
                runtime.check_text_size(numeric_bytes(&value), range)?;
                value
            }
            XPathNativeFunction::Min | XPathNativeFunction::Max => {
                let relation = if function == XPathNativeFunction::Min {
                    XPathComparisonRelation::LessThan
                } else {
                    XPathComparisonRelation::GreaterThan
                };
                if is_nan(&value)
                    || (!is_nan(&result) && xpath_compare_atomic(&value, &result, relation, range)?)
                {
                    value
                } else {
                    result
                }
            }
            _ => unreachable!("aggregate dispatch"),
        };
    }
    if function == XPathNativeFunction::Avg {
        let divisor = XPathComparableAtomic::Integer(XPathExactDecimal::from_usize(count));
        // The denominator is the bounded item count, not an arbitrary decimal.
        // Account for long division, its remainder scans and rounding passes
        // before entering the existing exact kernel.
        let work = numeric_bytes(&result)
            .saturating_add(XPATH_DECIMAL_DIVISION_PRECISION)
            .saturating_add(numeric_bytes(&divisor))
            .saturating_mul(numeric_bytes(&divisor).saturating_add(1))
            .saturating_mul(20);
        runtime.charge_work(work as u64, range)?;
        result = xpath_numeric_binary(
            result,
            divisor,
            XPathBinaryOperator::Divide,
            range,
            range,
            range,
        )?;
    }
    Ok(vec![xpath_numeric_result_item(
        expression, range, result, runtime,
    )?])
}

fn numeric_bytes(value: &XPathComparableAtomic) -> usize {
    match value {
        XPathComparableAtomic::Integer(value) | XPathComparableAtomic::Decimal(value) => {
            text::decimal_bytes(value)
        }
        XPathComparableAtomic::Float(value) => xpath_float_string_value(*value).len(),
        XPathComparableAtomic::Double(value) => xpath_double_string_value(*value).len(),
        _ => unreachable!("numeric only"),
    }
}

fn is_nan(value: &XPathComparableAtomic) -> bool {
    match value {
        XPathComparableAtomic::Float(value) => value.is_nan(),
        XPathComparableAtomic::Double(value) => value.is_nan(),
        _ => false,
    }
}

fn argument_error(message: &str, range: XPathSourceRange) -> XPathEvaluationError {
    XPathEvaluationError::dynamic(
        "cem.xpath.aggregate_argument_type_error",
        format!("err:XPTY0004: {message}"),
        range,
    )
}
