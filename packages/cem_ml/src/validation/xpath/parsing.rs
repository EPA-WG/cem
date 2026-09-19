//! Standard function contracts; external syntax is resolved only by import.
use super::*;
use crate::import::{
    import_string, CsvHeader, CsvImportOptions, ImportFailureKind, ImportStringProfile, ImportStringRequest, JsonXmlDuplicates,
    JsonXmlProjectionOptions,
};

pub(super) const ERRORS: &str = "http://www.w3.org/2005/xqt-errors";

pub(super) fn error(
    expression: &XPathExpressionAst,
    code: &'static str,
    local: &str,
    message: impl Into<String>,
    range: XPathSourceRange,
) -> XPathEvaluationError {
    let message = message.into();
    let diagnostic = xpath_evaluation_diagnostic(expression, code, &message, Some(range))
        .with_error_name(ERRORS, local);
    XPathEvaluationError {
        code,
        message,
        source_range: Some(range),
        diagnostic: Some(Box::new(diagnostic)),
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
    let mut args = Vec::new();
    for argument in arguments {
        args.push(xpath_evaluate_expression_node(
            expression, argument, focus, bindings, runtime,
        )?);
    }
    let type_error = || {
        error(
            expression,
            "cem.xpath.parse_argument",
            "XPTY0004",
            "Parsing/URI function argument has the wrong type or cardinality",
            range,
        )
    };
    if matches!(
        function,
        XPathNativeFunction::BaseUri | XPathNativeFunction::DocumentUri
    ) {
        let items = if let Some(first) = args.first() {
            first.as_slice()
        } else {
            std::slice::from_ref(focus.context_item.ok_or_else(|| {
                error(
                    expression,
                    "cem.xpath.context_item_missing",
                    "XPDY0002",
                    "URI function requires an available context item",
                    range,
                )
            })?)
        };
        if items.is_empty() {
            return Ok(vec![]);
        }
        let [item] = items else {
            return Err(type_error());
        };
        let node = item.native_node().ok_or_else(type_error)?;
        let uri = if function == XPathNativeFunction::DocumentUri {
            if node.result_node_kind() != XPathResultNodeKind::Document {
                None
            } else {
                node.owner().document_uri()
            }
        } else {
            node.base_uri()
        };
        if let Some(uri) = uri {
            runtime.read_text(uri, range)?;
        }
        return Ok(uri
            .map(|uri| XPathResultItem::Atomic {
                value: XPathAtomicValue {
                    type_name: "xs:anyURI".into(),
                    lexical_value: uri.into(),
                    namespace_uri: None,
                    local_name: None,
                },
                source_map: node.source_map(),
            })
            .into_iter()
            .collect());
    }
    let items = xpath_atomized_items(
        &args[0],
        range,
        runtime,
        "parsing function",
        "cem.xpath.parse_function_item",
    )?;
    let source = match items.as_slice() {
        [] => None,
        [XPathResultItem::Atomic { value, .. }]
            if matches!(
                value.type_name.as_str(),
                "xs:string" | "xs:anyURI" | "xs:untypedAtomic"
            ) =>
        {
            Some(runtime.copy_text(&value.lexical_value, range)?)
        }
        _ => return Err(type_error()),
    };
    let options = if let Some(items) = args.get(1) {
        let [XPathResultItem::Map { entries, .. }] = items.as_slice() else {
            return Err(type_error());
        };
        Some(entries.as_slice())
    } else {
        None
    };
    let profile = match function {
        XPathNativeFunction::ParseXml => ImportStringProfile::Xml,
        XPathNativeFunction::JsonToXml => ImportStringProfile::JsonXml(
            options.map(|entries| json_options(expression, entries, range)).transpose()?.unwrap_or_default()
        ),
        XPathNativeFunction::ParseCsv => ImportStringProfile::CsvWithOptions(
            options.map(|entries| csv_options(expression, entries, range)).transpose()?.unwrap_or_default()
        ),
        XPathNativeFunction::ParseYaml => ImportStringProfile::Yaml,
        _ => unreachable!(),
    };
    let Some(source) = source else {
        return Ok(vec![]);
    };
    runtime.charge_work(source.len() as u64, range)?;
    runtime.force(range)?;
    let uri = format!(
        "{}#parsed-{}",
        expression.source.uri, range.start.byte_offset
    );
    let result = import_string(ImportStringRequest {
        source: &source,
        source_uri: &uri,
        base_uri: Some(&expression.source.uri),
        profile,
    });
    runtime.force(range)?;
    let tree = result.map_err(|failure| {
        let (code, local) = match failure.kind {
            ImportFailureKind::Limit => {
                return XPathEvaluationError::dynamic(
                    "cem.xpath.import_limit_exceeded",
                    failure.message,
                    range,
                )
            }
            ImportFailureKind::Unsupported | ImportFailureKind::Internal => {
                return XPathEvaluationError::unsupported(failure.message, range)
            }
            ImportFailureKind::DuplicateKey => ("cem.xpath.json_duplicate_key", "FOJS0003"),
            ImportFailureKind::Malformed if function == XPathNativeFunction::ParseXml => {
                ("cem.xpath.parse_xml", "FODC0006")
            }
            ImportFailureKind::Malformed if function == XPathNativeFunction::JsonToXml => {
                ("cem.xpath.json_to_xml", "FOJS0001")
            }
            ImportFailureKind::Malformed => ("cem.xpath.import_invalid", ""),
        };
        let mut failed = error(expression, code, local, failure.message, range);
        if let Some(diagnostic) = failed.diagnostic.as_mut() {
            if local.is_empty() {
                **diagnostic = diagnostic
                    .clone()
                    .with_error_name("urn:cem:import", "invalid-source");
            }
            // Explicit diagnostic metadata retains parser locations alongside
            // the stylesheet call location; no runtime tree serialization.
            if let Some(details) = diagnostic.details.as_mut().and_then(|d| d.as_object_mut()) {
                details.insert(
                    "importDiagnostics".into(),
                    serde_json::to_value(failure.diagnostics).expect("diagnostic metadata"),
                );
            }
        }
        failed
    })?;
    let items = vec![XPathResultItem::from_native_node(
        XPathNativeNode::cem_document(tree),
    )];
    runtime.enforce_sequence_items(items.len(), range)?;
    runtime.check_items_text(&items, range)?;
    Ok(items)
}

fn csv_options(
    expression: &XPathExpressionAst,
    entries: &[XPathMapEntry],
    range: XPathSourceRange,
) -> Result<CsvImportOptions, XPathEvaluationError> {
    let invalid = || {
        let mut failure = error(expression, "cem.xpath.csv_options", "", "Invalid CSV import options", range);
        if let Some(diagnostic) = failure.diagnostic.as_mut() {
            **diagnostic = diagnostic.clone().with_error_name("urn:cem:import", "invalid-options");
        }
        failure
    };
    let mut options = CsvImportOptions::default();
    for entry in entries {
        if entry.key.type_name != "xs:string" || entry.key.lexical_value != "header" {
            return Err(invalid());
        }
        let [XPathResultItem::Atomic { value, .. }] = entry.value.items.as_slice() else {
            return Err(invalid());
        };
        if value.type_name != "xs:string" { return Err(invalid()); }
        options.header = match value.lexical_value.as_str() {
            "present" => CsvHeader::Present,
            "absent" => CsvHeader::Absent,
            _ => return Err(invalid()),
        };
    }
    Ok(options)
}

fn json_options(
    expression: &XPathExpressionAst,
    entries: &[XPathMapEntry],
    range: XPathSourceRange,
) -> Result<JsonXmlProjectionOptions, XPathEvaluationError> {
    let invalid = || {
        error(
            expression,
            "cem.xpath.json_options",
            "FOJS0005",
            "Invalid json-to-xml option",
            range,
        )
    };
    let mut options = JsonXmlProjectionOptions::default();
    let mut validate = false;
    let mut fallback = false;
    let mut explicit_duplicates = false;
    for entry in entries {
        if !matches!(
            entry.key.type_name.as_str(),
            "xs:string" | "xs:anyURI" | "xs:untypedAtomic"
        ) {
            continue;
        }
        match entry.key.lexical_value.as_str() {
            "escape" | "validate" | "liberal" => {
                let [XPathResultItem::Atomic { value, .. }] = entry.value.items.as_slice() else {
                    return Err(invalid());
                };
                if value.type_name != "xs:boolean" {
                    return Err(invalid());
                }
                let value = match value.lexical_value.as_str() {
                    "true" | "1" => true,
                    "false" | "0" => false,
                    _ => return Err(invalid()),
                };
                match entry.key.lexical_value.as_str() {
                    "escape" => options.escape = value,
                    "validate" => validate = value,
                    _ => {}
                }
            }
            "duplicates" => {
                let [XPathResultItem::Atomic { value, .. }] = entry.value.items.as_slice() else {
                    return Err(invalid());
                };
                if !matches!(
                    value.type_name.as_str(),
                    "xs:string" | "xs:anyURI" | "xs:untypedAtomic"
                ) {
                    return Err(invalid());
                }
                options.duplicates = match value.lexical_value.as_str() {
                    "retain" => JsonXmlDuplicates::Retain,
                    "use-first" => JsonXmlDuplicates::UseFirst,
                    "reject" => JsonXmlDuplicates::Reject,
                    _ => return Err(invalid()),
                };
                explicit_duplicates = true;
            }
            "fallback" => {
                if !matches!(
                    entry.value.items.as_slice(),
                    [XPathResultItem::Function { arity: 1, .. }]
                ) {
                    return Err(invalid());
                }
                fallback = true;
            }
            _ => {} // Unknown option keys are ignored by the standard convention.
        }
    }
    if (fallback && options.escape)
        || (validate && explicit_duplicates && options.duplicates == JsonXmlDuplicates::Retain)
    {
        return Err(invalid());
    }
    if validate {
        return Err(error(
            expression,
            "cem.xpath.json_validate",
            "FOJS0004",
            "Schema-typed JSON-to-XML is not supported",
            range,
        ));
    }
    if fallback {
        return Err(XPathEvaluationError::unsupported(
            "Custom JSON fallback callbacks are outside the parsing profile",
            range,
        ));
    }
    Ok(options)
}
