//! Native lexical closures. Only program syntax is portable; captured values
//! and executable owners are never serialized or recovered from function IDs.
use super::*;

type Result<T> = std::result::Result<T, XPathEvaluationError>;
const MAX_CALL_DEPTH: usize = 32;

#[derive(Clone)]
pub struct XPathNativeFunctionItem(Arc<Closure>);

struct Closure {
    expression: XPathExpressionAst,
    parameters: Vec<XPathFunctionParameter>,
    result_type: Option<XPathSequenceType>,
    body: Option<XPathExpressionSequence>,
    bindings: XPathVariableBindings,
    static_context: XPathStaticContext,
    default_language: String,
    retained_text_bytes: usize,
    xslt_host: bool,
}

impl std::fmt::Debug for XPathNativeFunctionItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XPathNativeFunctionItem")
            .field("arity", &self.0.parameters.len())
            .finish_non_exhaustive()
    }
}
impl PartialEq for XPathNativeFunctionItem {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for XPathNativeFunctionItem {}
impl XPathNativeFunctionItem {
    pub(super) fn retained_text_bytes(&self) -> usize {
        self.0.retained_text_bytes
    }
}

fn type_error(message: impl Into<String>, range: XPathSourceRange) -> XPathEvaluationError {
    XPathEvaluationError::dynamic(
        "cem.xpath.function_type_error",
        format!("err:XPTY0004: {}", message.into()),
        range,
    )
}

#[allow(clippy::too_many_arguments)] // The typed declaration and evaluator context stay explicit.
pub(super) fn create(
    expression: &XPathExpressionAst,
    parameters: &[XPathFunctionParameter],
    result_type: Option<&XPathSequenceType>,
    body: Option<&XPathExpressionSequence>,
    focus: XPathFocus<'_>,
    bindings: &XPathVariableBindings,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>> {
    let mut names = BTreeSet::new();
    for parameter in parameters {
        if !names.insert(XPathExpandedName::from_syntax_name(&parameter.name)) {
            return Err(XPathEvaluationError::dynamic(
                "cem.xpath.function_duplicate_parameter",
                "err:XQST0039: duplicate inline function parameter",
                parameter.name.source_range,
            ));
        }
        if let Some(ty) = &parameter.sequence_type {
            xpath_validate_sequence_type_supported(ty)?;
        }
    }
    if let Some(ty) = result_type {
        xpath_validate_sequence_type_supported(ty)?;
    }
    // Collect referenced names without retaining unrelated caller documents.
    let mut references = BTreeSet::new();
    if let Some(body) = body {
        collect(body, &mut references, runtime, 0)?;
    }
    let mut captured = BTreeMap::new();
    let mut count = 0usize;
    let mut bytes = 0usize;
    for name in references.difference(&names) {
        if let Some(value) = bindings.get(name) {
            count = count.saturating_add(value.items.len());
            runtime.enforce_sequence_items(count, range)?;
            bytes = bytes.saturating_add(runtime.items_text_bytes(&value.items, range)?);
            runtime.check_text_size(bytes, range)?;
            text::charge_items_copy(&value.items, runtime, range)?;
            captured.insert(name.clone(), value.clone());
        }
    }
    let signature = format!(
        "function({}) as {}",
        parameters
            .iter()
            .map(|p| p
                .sequence_type
                .as_ref()
                .map(xpath_sequence_type_display)
                .unwrap_or_else(|| "item()*".into()))
            .collect::<Vec<_>>()
            .join(", "),
        result_type
            .map(xpath_sequence_type_display)
            .unwrap_or_else(|| "item()*".into())
    );
    let owner = XPathExpressionAst {
        source: expression.source.clone(),
        attachment: expression.attachment.clone(),
        source_text: None,
        tokens: Vec::new(),
        events: Vec::new(),
        syntax_ast: None,
        facts: Vec::new(),
        line_ending: None,
    };
    Ok(vec![XPathResultItem::Function {
        evaluator_id: "cem.xpath.native".into(),
        function_id: format!(
            "inline:{}:{}",
            expression.source.uri, range.start.byte_offset
        ),
        name: None,
        arity: parameters.len(),
        signature,
        source_map: range.source_map(
            expression.attachment.source_id(),
            &expression.source.media_type,
        ),
        native_function: Some(XPathNativeFunctionItem(Arc::new(Closure {
            expression: owner,
            parameters: parameters.to_vec(),
            result_type: result_type.cloned(),
            body: body.cloned(),
            bindings: captured,
            static_context: focus.static_context.clone(),
            default_language: focus.default_language.to_owned(),
            retained_text_bytes: bytes,
            xslt_host: focus.xslt_host,
        }))),
    }])
}

pub(super) fn call(
    expression: &XPathExpressionAst,
    functions: &[XPathResultItem],
    arguments: &[XPathExpressionNode],
    focus: XPathFocus<'_>,
    bindings: &XPathVariableBindings,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>> {
    let [item] = functions else {
        return Err(type_error("a dynamic call requires one function", range));
    };
    if !matches!(item, XPathResultItem::Function { .. }) {
        return containers::call(
            expression, functions, arguments, focus, bindings, runtime, range,
        );
    }
    let mut values = Vec::new();
    for argument in arguments {
        values.push(xpath_evaluate_expression_node(
            expression, argument, focus, bindings, runtime,
        )?);
    }
    invoke(item, values, runtime, range)
}

pub(super) fn invoke(
    item: &XPathResultItem,
    arguments: Vec<Vec<XPathResultItem>>,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>> {
    if matches!(
        item,
        XPathResultItem::Map { .. } | XPathResultItem::Array { .. }
    ) {
        let [argument] = arguments.as_slice() else {
            return Err(type_error("map/array function arity mismatch", range));
        };
        return containers::call_value(item, argument, runtime, range);
    }
    let XPathResultItem::Function {
        native_function: Some(native),
        ..
    } = item
    else {
        return Err(XPathEvaluationError::dynamic(
            "cem.xpath.native_function_missing",
            "XPath function invocation requires its retained native executable owner",
            range,
        ));
    };
    let closure = &native.0;
    if closure.parameters.len() != arguments.len() {
        return Err(type_error("dynamic function arity mismatch", range));
    }
    if runtime.function_depth >= MAX_CALL_DEPTH {
        return Err(XPathEvaluationError::dynamic(
            "cem.xpath.function_depth_exceeded",
            "XPath inline function call depth exceeds 32",
            range,
        ));
    }
    runtime.poll(range)?;
    runtime.function_depth += 1;
    let result = (|| {
        let mut bindings = BTreeMap::new();
        for (name, value) in &closure.bindings {
            runtime.check_items_text(&value.items, range)?;
            text::charge_items_copy(&value.items, runtime, range)?;
            bindings.insert(name.clone(), value.clone());
        }
        for (parameter, values) in closure.parameters.iter().zip(arguments) {
            let items = convert(
                values,
                parameter.sequence_type.as_ref(),
                runtime,
                parameter.name.source_range,
            )?;
            bindings.insert(
                XPathExpandedName::from_syntax_name(&parameter.name),
                XPathResultSequence {
                    sequence_type: xpath_result_sequence_type(&items),
                    items,
                },
            );
        }
        let items = if let Some(body) = &closure.body {
            xpath_evaluate_expression_sequence(
                &closure.expression,
                body,
                XPathFocus {
                    xslt_host: closure.xslt_host,
                    ..XPathFocus::outer(None, &closure.default_language, &closure.static_context)
                },
                &bindings,
                runtime,
            )?
            .items
        } else {
            Vec::new()
        };
        convert(
            items,
            closure.result_type.as_ref(),
            runtime,
            closure
                .result_type
                .as_ref()
                .map_or(range, XPathSequenceType::source_range),
        )
    })();
    runtime.function_depth -= 1;
    result.map_err(|mut error: XPathEvaluationError| {
        if error.diagnostic.is_none() {
            error.diagnostic = Some(Box::new(error.clone().into_diagnostic(&closure.expression)));
        }
        error
    })
}

fn atomic_type(ty: &XPathSequenceItemType) -> Option<&XPathName> {
    match ty {
        XPathSequenceItemType::Atomic(name) => Some(name),
        XPathSequenceItemType::Parenthesized { item_type, .. } => atomic_type(item_type),
        _ => None,
    }
}

fn convert(
    mut items: Vec<XPathResultItem>,
    ty: Option<&XPathSequenceType>,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>> {
    let Some(ty) = ty else {
        return Ok(items);
    };
    xpath_validate_sequence_type_supported(ty)?;
    if let XPathSequenceType::Item { item_type, .. } = ty {
        if let Some(target) = atomic_type(item_type) {
            items = xpath_atomized_items(
                &items,
                range,
                runtime,
                "inline function conversion",
                "cem.xpath.function_atomization_error",
            )?;
            for item in &mut items {
                let XPathResultItem::Atomic { value, .. } = item else {
                    unreachable!()
                };
                if xpath_atomic_value_matches_type(value, target) {
                    continue;
                }
                let target = target.local_name.as_str();
                let promotable = matches!(
                    (value.type_name.as_str(), target),
                    ("xs:untypedAtomic", _)
                        | ("xs:anyURI", "string")
                        | ("xs:integer" | "xs:decimal", "float" | "double")
                        | ("xs:float", "double")
                );
                if promotable {
                    let target = if target == "numeric" {
                        "double"
                    } else {
                        target
                    };
                    *value = xpath_cast_atomic(
                        xpath_cast_atomic_value(value, range)?,
                        target,
                        runtime,
                        range,
                    )
                    .map_err(|error| {
                        XPathEvaluationError::dynamic(
                            error.diagnostic_code(),
                            format!("err:FORG0001: {}", error.message),
                            range,
                        )
                    })?;
                }
            }
        }
    }
    if !xpath_sequence_matches_type(&items, ty) {
        return Err(type_error(
            format!("value does not match {}", xpath_sequence_type_display(ty)),
            range,
        ));
    }
    Ok(items)
}

// Walk typed program nodes only. Bounds apply before recursively cloning a body.
fn collect(
    sequence: &XPathExpressionSequence,
    names: &mut BTreeSet<XPathExpandedName>,
    runtime: &mut XPathEvaluationRuntime,
    depth: usize,
) -> Result<()> {
    for node in &sequence.expressions {
        collect_node(node, names, runtime, depth)?;
    }
    Ok(())
}
fn collect_node(
    node: &XPathExpressionNode,
    names: &mut BTreeSet<XPathExpandedName>,
    runtime: &mut XPathEvaluationRuntime,
    depth: usize,
) -> Result<()> {
    if depth >= 128 {
        return Err(XPathEvaluationError::dynamic(
            "cem.xpath.function_depth_exceeded",
            "XPath closure program nesting exceeds 128",
            node.source_range,
        ));
    }
    runtime.poll(node.source_range)?;
    let next = depth + 1;
    match &node.expression {
        XPathExpression::Path(path) => {
            for step in &path.steps {
                match &step.step {
                    XPathStep::Axis { predicates, .. } => {
                        for seq in predicates {
                            collect(seq, names, runtime, next)?;
                        }
                    }
                    XPathStep::Primary(primary) => collect_primary(primary, names, runtime, next)?,
                    XPathStep::Postfix { primary, postfixes } => {
                        collect_primary(primary, names, runtime, next)?;
                        for postfix in postfixes {
                            match postfix {
                                XPathPostfixExpression::Predicate(seq) => {
                                    collect(seq, names, runtime, next)?
                                }
                                XPathPostfixExpression::ArgumentList(args) => {
                                    for arg in args {
                                        collect_node(arg, names, runtime, next)?;
                                    }
                                }
                                XPathPostfixExpression::Lookup { key } => {
                                    collect_key(key, names, runtime, next)?
                                }
                            }
                        }
                    }
                }
            }
        }
        XPathExpression::Unary { operand, .. }
        | XPathExpression::CastAs { operand, .. }
        | XPathExpression::CastableAs { operand, .. }
        | XPathExpression::TreatAs { operand, .. }
        | XPathExpression::InstanceOf { operand, .. } => {
            collect_node(operand, names, runtime, next)?
        }
        XPathExpression::Binary { left, right, .. } => {
            collect_node(left, names, runtime, next)?;
            collect_node(right, names, runtime, next)?;
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
            collect_node(binding_expression, names, runtime, next)?;
            let mut locals = BTreeSet::new();
            collect_node(return_expression, &mut locals, runtime, next)?;
            locals.remove(&XPathExpandedName::from_syntax_name(binding));
            names.extend(locals);
        }
        XPathExpression::Quantified {
            binding,
            binding_expression,
            satisfies_expression,
            ..
        } => {
            collect_node(binding_expression, names, runtime, next)?;
            let mut locals = BTreeSet::new();
            collect_node(satisfies_expression, &mut locals, runtime, next)?;
            locals.remove(&XPathExpandedName::from_syntax_name(binding));
            names.extend(locals);
        }
        XPathExpression::If {
            condition,
            then_expression,
            else_expression,
        } => {
            collect(condition, names, runtime, next)?;
            collect_node(then_expression, names, runtime, next)?;
            collect_node(else_expression, names, runtime, next)?;
        }
        XPathExpression::SimpleMap { input, mappings } => {
            collect_node(input, names, runtime, next)?;
            for node in mappings {
                collect_node(node, names, runtime, next)?;
            }
        }
        XPathExpression::Unsupported { .. } => {}
    }
    Ok(())
}
fn collect_key(
    key: &XPathLookupKey,
    names: &mut BTreeSet<XPathExpandedName>,
    runtime: &mut XPathEvaluationRuntime,
    depth: usize,
) -> Result<()> {
    if let XPathLookupKey::Expression(Some(seq)) = key {
        collect(seq, names, runtime, depth)?;
    }
    Ok(())
}
fn collect_primary(
    primary: &XPathPrimaryExpression,
    names: &mut BTreeSet<XPathExpandedName>,
    runtime: &mut XPathEvaluationRuntime,
    depth: usize,
) -> Result<()> {
    match primary {
        XPathPrimaryExpression::VariableReference(name) => {
            names.insert(XPathExpandedName::from_syntax_name(name));
        }
        XPathPrimaryExpression::Parenthesized(Some(seq))
        | XPathPrimaryExpression::ArrayConstructor(XPathArrayConstructor::Curly(Some(seq))) => {
            collect(seq, names, runtime, depth)?
        }
        XPathPrimaryExpression::FunctionCall { arguments, .. } => {
            for node in arguments {
                collect_node(node, names, runtime, depth)?;
            }
        }
        XPathPrimaryExpression::InlineFunction {
            parameters, body, ..
        } => {
            let mut locals = BTreeSet::new();
            if let Some(body) = body {
                collect(body, &mut locals, runtime, depth)?;
            }
            for parameter in parameters {
                locals.remove(&XPathExpandedName::from_syntax_name(&parameter.name));
            }
            names.extend(locals);
        }
        XPathPrimaryExpression::MapConstructor { entries } => {
            for entry in entries {
                collect_node(&entry.key, names, runtime, depth)?;
                collect_node(&entry.value, names, runtime, depth)?;
            }
        }
        XPathPrimaryExpression::ArrayConstructor(XPathArrayConstructor::Square(seq)) => {
            collect(seq, names, runtime, depth)?
        }
        XPathPrimaryExpression::UnaryLookup(key) => collect_key(key, names, runtime, depth)?,
        _ => {}
    }
    Ok(())
}
