//! Shared text accounting and the non-regex XPath text functions.
use super::*;

pub(super) fn charge_items_copy(
    items: &[XPathResultItem],
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<(), XPathEvaluationError> {
    for item in items {
        runtime.poll(range)?;
        match item {
            XPathResultItem::Atomic { value, .. } => {
                runtime.read_text(&value.lexical_value, range)?
            }
            XPathResultItem::Array { members, .. } => {
                for member in members {
                    runtime.poll(range)?;
                    charge_items_copy(&member.items, runtime, range)?;
                }
            }
            XPathResultItem::Map { entries, .. } => {
                for entry in entries {
                    runtime.poll(range)?;
                    runtime.read_text(&entry.key.lexical_value, range)?;
                    charge_items_copy(&entry.value.items, runtime, range)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn decimal_bytes(value: &XPathExactDecimal) -> usize {
    usize::from(value.negative).saturating_add(if value.scale == 0 {
        value.coefficient.len()
    } else if value.coefficient.len() > value.scale {
        value.coefficient.len().saturating_add(1)
    } else {
        value.scale.saturating_add(2)
    })
}

pub(super) fn check_number_mantissa(
    value: &XPathExactDecimal,
    picture: &XPathNumberSubPicture,
    format: &XPathDecimalFormat,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<(), XPathEvaluationError> {
    let integer = if value.is_zero() {
        0
    } else {
        value.coefficient.len().saturating_sub(value.scale)
    }
    .max(picture.minimum_integer_size);
    let fraction =
        if value.is_zero() { 0 } else { value.scale }.max(picture.minimum_fractional_size);
    let integer_groups = match &picture.integer_grouping {
        XPathNumberGrouping::None => 0,
        XPathNumberGrouping::Regular(size) => integer.saturating_sub(1) / size,
        XPathNumberGrouping::Irregular(positions) => positions
            .iter()
            .filter(|p| **p > 0 && **p < integer)
            .count(),
    };
    let fraction_groups = picture
        .fractional_grouping_positions
        .iter()
        .filter(|p| **p > 0 && **p < fraction)
        .count();
    let bytes = integer
        .saturating_add(fraction)
        .saturating_mul(format.zero_digit.len_utf8())
        .saturating_add(
            integer_groups
                .saturating_add(fraction_groups)
                .saturating_mul(format.grouping_separator.len_utf8()),
        )
        .saturating_add(if picture.decimal_separator_present && fraction > 0 {
            format.decimal_separator.len_utf8()
        } else {
            0
        });
    runtime.check_text_size(bytes, range)?;
    // Padding, grouping, and digit translation make bounded passes over this text.
    runtime.charge_work(bytes.saturating_mul(6) as u64, range)
}

pub(super) fn charge_atomic_pair(
    left: &XPathComparableAtomic,
    right: &XPathComparableAtomic,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<(), XPathEvaluationError> {
    for value in [left, right] {
        let size = match value {
            XPathComparableAtomic::String(value) | XPathComparableAtomic::Untyped(value) => {
                value.len()
            }
            XPathComparableAtomic::Integer(value) | XPathComparableAtomic::Decimal(value) => {
                decimal_bytes(value)
            }
            _ => 1,
        };
        runtime.charge_work(size as u64, range)?;
    }
    Ok(())
}

pub(super) fn atomic_string(
    value: XPathComparableAtomic,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<String, XPathEvaluationError> {
    if let XPathComparableAtomic::Integer(value) | XPathComparableAtomic::Decimal(value) = &value {
        runtime.check_text_size(decimal_bytes(value), range)?;
        runtime.charge_work(decimal_bytes(value) as u64, range)?;
    }
    let value = xpath_atomic_string_value(value);
    runtime.read_text(&value, range)?;
    Ok(value)
}

pub(super) fn work_limit(range: XPathSourceRange) -> XPathEvaluationError {
    XPathEvaluationError::dynamic(
        "cem.xpath.work_limit_exceeded",
        "XPath evaluation exceeded its xpathWorkUnits limit",
        range,
    )
}

impl XPathEvaluationRuntime {
    pub(super) fn check_text_size(
        &self,
        bytes: usize,
        range: XPathSourceRange,
    ) -> Result<(), XPathEvaluationError> {
        if self
            .limits
            .max_text_bytes
            .is_some_and(|limit| bytes as u64 > limit)
        {
            return Err(XPathEvaluationError::dynamic(
                "cem.xpath.text_byte_limit_exceeded",
                "XPath text exceeded its xpathTextBytes limit (UTF-8 lexical bytes)",
                range,
            ));
        }
        Ok(())
    }

    /// Counts atomic lexical values, including those inside supplied containers.
    /// Retained node owners are neither serialized nor traversed here.
    pub(super) fn check_items_text(
        &mut self,
        items: &[XPathResultItem],
        range: XPathSourceRange,
    ) -> Result<(), XPathEvaluationError> {
        self.items_text_bytes(items, range).map(|_| ())
    }

    pub(super) fn items_text_bytes(
        &mut self,
        items: &[XPathResultItem],
        range: XPathSourceRange,
    ) -> Result<usize, XPathEvaluationError> {
        fn visit(
            runtime: &mut XPathEvaluationRuntime,
            items: &[XPathResultItem],
            total: &mut usize,
            range: XPathSourceRange,
        ) -> Result<(), XPathEvaluationError> {
            for item in items {
                runtime.poll(range)?;
                match item {
                    XPathResultItem::Atomic { value, .. } => {
                        *total = total.saturating_add(value.lexical_value.len());
                        runtime.check_text_size(*total, range)?;
                    }
                    XPathResultItem::Array { members, .. } => {
                        for member in members {
                            runtime.poll(range)?;
                            visit(runtime, &member.items, total, range)?;
                        }
                    }
                    XPathResultItem::Map { entries, .. } => {
                        for entry in entries {
                            runtime.poll(range)?;
                            *total = total.saturating_add(entry.key.lexical_value.len());
                            runtime.check_text_size(*total, range)?;
                            visit(runtime, &entry.value.items, total, range)?;
                        }
                    }
                    _ => {}
                }
            }
            Ok(())
        }
        let mut total = 0;
        visit(self, items, &mut total, range)?;
        Ok(total)
    }

    pub(super) fn append_items(
        &mut self,
        items: &mut Vec<XPathResultItem>,
        total_bytes: &mut usize,
        values: Vec<XPathResultItem>,
        range: XPathSourceRange,
    ) -> Result<(), XPathEvaluationError> {
        self.enforce_sequence_items(items.len().saturating_add(values.len()), range)?;
        *total_bytes = total_bytes.saturating_add(self.items_text_bytes(&values, range)?);
        self.check_text_size(*total_bytes, range)?;
        items.extend(values);
        Ok(())
    }

    pub(super) fn read_text(
        &mut self,
        value: &str,
        range: XPathSourceRange,
    ) -> Result<(), XPathEvaluationError> {
        self.check_text_size(value.len(), range)?;
        self.charge_work(value.len() as u64, range)
    }

    pub(super) fn copy_text(
        &mut self,
        value: &str,
        range: XPathSourceRange,
    ) -> Result<String, XPathEvaluationError> {
        self.read_text(value, range)?;
        Ok(value.to_owned())
    }

    pub(super) fn append_text(
        &mut self,
        result: &mut String,
        value: &str,
        range: XPathSourceRange,
    ) -> Result<(), XPathEvaluationError> {
        self.check_text_size(result.len().saturating_add(value.len()), range)?;
        self.charge_work(value.len() as u64, range)?;
        result.push_str(value);
        Ok(())
    }

    pub(super) fn append_char(
        &mut self,
        result: &mut String,
        value: char,
        range: XPathSourceRange,
    ) -> Result<(), XPathEvaluationError> {
        self.append_text(result, value.encode_utf8(&mut [0; 4]), range)
    }
}

pub(super) fn node_string_value(
    node: &XPathNativeNode,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<String, XPathEvaluationError> {
    if !matches!(
        node.result_node_kind(),
        XPathResultNodeKind::Document | XPathResultNodeKind::Element
    ) {
        return runtime.copy_text(node.semantic_value(), range);
    }
    let mut result = String::new();
    let mut pending = node.child_nodes();
    pending.reverse();
    while let Some(node) = pending.pop() {
        runtime.poll(range)?;
        if node.result_node_kind() == XPathResultNodeKind::Text {
            runtime.append_text(&mut result, node.semantic_value(), range)?;
        } else {
            let mut children = node.child_nodes();
            children.reverse();
            pending.extend(children);
        }
    }
    Ok(result)
}

pub(super) fn node_typed_value(
    node: &XPathNativeNode,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<XPathAtomicValue, XPathEvaluationError> {
    Ok(XPathAtomicValue {
        type_name: match node.result_node_kind() {
            XPathResultNodeKind::Document
            | XPathResultNodeKind::Element
            | XPathResultNodeKind::Attribute
            | XPathResultNodeKind::Text => "xs:untypedAtomic",
            _ => "xs:string",
        }
        .to_owned(),
        lexical_value: node_string_value(node, runtime, range)?,
        namespace_uri: None,
        local_name: None,
    })
}

fn string_argument(
    items: &[XPathResultItem],
    optional: bool,
    range: XPathSourceRange,
    runtime: &mut XPathEvaluationRuntime,
) -> Result<String, XPathEvaluationError> {
    let items = xpath_atomized_items(
        items,
        range,
        runtime,
        "text function",
        "cem.xpath.text_function_item",
    )?;
    match items.as_slice() {
        [] if optional => Ok(String::new()),
        [XPathResultItem::Atomic { value, .. }] if matches!(value.type_name.as_str(), "xs:string" | "xs:untypedAtomic" | "xs:anyURI") => runtime.copy_text(&value.lexical_value, range),
        _ => Err(XPathEvaluationError::dynamic("cem.xpath.text_argument_type", "err:XPTY0004: XPath text argument requires a singleton string (or the empty sequence where optional)", range)),
    }
}

pub(super) fn evaluate(
    function: XPathNativeFunction,
    expression: &XPathExpressionAst,
    arguments: &[XPathExpressionNode],
    focus: XPathFocus<'_>,
    bindings: &XPathVariableBindings,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>, XPathEvaluationError> {
    let Some(argument) = arguments.first() else {
        let context = focus.context_item.ok_or_else(|| {
            XPathEvaluationError::dynamic(
                "cem.xpath.context_item_missing",
                "err:XPDY0002: XPath text function requires an available context item",
                range,
            )
        })?;
        let value = xpath_string_function_value(std::slice::from_ref(context), range, runtime)?;
        return evaluate_string(function, value, expression, runtime, range);
    };
    let items = xpath_evaluate_expression_node(expression, argument, focus, bindings, runtime)?;
    if function == XPathNativeFunction::StringJoin {
        let items = xpath_atomized_items(
            &items,
            argument.source_range,
            runtime,
            "fn:string-join",
            "cem.xpath.text_function_item",
        )?;
        let separator = if let Some(separator) = arguments.get(1) {
            let values =
                xpath_evaluate_expression_node(expression, separator, focus, bindings, runtime)?;
            string_argument(&values, false, separator.source_range, runtime)?
        } else {
            String::new()
        };
        let mut result = String::new();
        for (index, item) in items.iter().enumerate() {
            runtime.poll(range)?;
            if index > 0 {
                runtime.append_text(&mut result, &separator, range)?;
            }
            let value = xpath_string_function_value(
                std::slice::from_ref(item),
                argument.source_range,
                runtime,
            )?;
            runtime.append_text(&mut result, &value, range)?;
        }
        return Ok(vec![xpath_string_result_item(expression, range, result)]);
    }
    let value = string_argument(&items, true, argument.source_range, runtime)?;
    evaluate_string(function, value, expression, runtime, range)
}

fn evaluate_string(
    function: XPathNativeFunction,
    value: String,
    expression: &XPathExpressionAst,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>, XPathEvaluationError> {
    runtime.read_text(&value, range)?;
    if function == XPathNativeFunction::StringLength {
        return Ok(vec![xpath_numeric_result_item(
            expression,
            range,
            XPathComparableAtomic::Integer(XPathExactDecimal::from_usize(value.chars().count())),
            runtime,
        )?]);
    }
    if function == XPathNativeFunction::Tokenize {
        let mut items = Vec::new();
        for token in value
            .split([' ', '\t', '\r', '\n'])
            .filter(|token| !token.is_empty())
        {
            runtime.enforce_sequence_items(items.len().saturating_add(1), range)?;
            items.push(xpath_string_result_item(
                expression,
                range,
                runtime.copy_text(token, range)?,
            ));
        }
        return Ok(items);
    }
    let mut result = String::new();
    let mut space = false;
    for ch in value.chars() {
        runtime.poll(range)?;
        if matches!(ch, ' ' | '\t' | '\r' | '\n') {
            space = !result.is_empty();
        } else {
            if space {
                runtime.append_char(&mut result, ' ', range)?;
                space = false;
            }
            runtime.append_char(&mut result, ch, range)?;
        }
    }
    Ok(vec![xpath_string_result_item(expression, range, result)])
}
