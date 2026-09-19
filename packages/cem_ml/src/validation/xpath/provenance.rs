//! Metadata over common native CEM nodes. No source syntax is interpreted here.
use super::*;

pub(super) fn evaluate(
    function: XPathNativeFunction,
    expression: &XPathExpressionAst,
    argument: &XPathExpressionNode,
    focus: XPathFocus<'_>,
    bindings: &XPathVariableBindings,
    runtime: &mut XPathEvaluationRuntime,
    range: XPathSourceRange,
) -> Result<Vec<XPathResultItem>, XPathEvaluationError> {
    let items = xpath_evaluate_expression_node(expression, argument, focus, bindings, runtime)?;
    let invalid = || {
        parsing::error(
            expression,
            "cem.xpath.source_argument",
            "XPTY0004",
            "Source metadata requires zero or one native node",
            range,
        )
    };
    let node = match items.as_slice() {
        [] => return Ok(vec![]),
        [item] => item.native_node().ok_or_else(invalid)?,
        _ => return Err(invalid()),
    };
    let (kind, value) = match function {
        XPathNativeFunction::SourceNodeKey => ("xs:string", node.source_key()),
        XPathNativeFunction::SourceLineNumber => (
            "xs:integer",
            node.source_line_number().map(|line| line.to_string()),
        ),
        _ => unreachable!(),
    };
    let Some(value) = value else {
        return Ok(vec![]);
    };
    runtime.read_text(&value, range)?;
    runtime.enforce_sequence_items(1, range)?;
    Ok(vec![XPathResultItem::Atomic {
        value: XPathAtomicValue {
            type_name: kind.into(),
            lexical_value: value,
            namespace_uri: None,
            local_name: None,
        },
        source_map: node.source_map(),
    }])
}
