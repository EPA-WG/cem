//! Behavior evaluation over explicit consumed placements of original AST nodes.
use super::*;
use cem_ml::schema::input_references::{RetainedBehaviorValidation, RetainedValidationStructure};
use cem_ql::validation_structure::{RetainedValidationQueryTree, ValidationPlacementNode};

pub(super) fn validate(
    structure: RetainedValidationStructure<'_>,
    model: &SchemaDocumentModel,
) -> RetainedBehaviorValidation {
    if !structure.complete || structure.nodes.iter().any(|node| !node.children_complete) {
        return RetainedBehaviorValidation::default();
    }
    let tree = match RetainedValidationQueryTree::new(structure) {
        Ok(tree) => tree,
        Err(message) => {
            return RetainedBehaviorValidation {
                complete: true,
                diagnostics: vec![schema_behavior_diagnostic(
                    SCHEMA_BEHAVIOR_RESULT_INVALID_CODE,
                    Severity::Error,
                    message,
                    &SourceMapStack::default(),
                    json!({"schemaUri": model.schema_uri}),
                )],
            }
        }
    };
    // Only scalar diagnostic/binding metadata is materialized here; candidates
    // passed into queries and function bodies remain native placement nodes.
    let candidates: Vec<_> = structure
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            let CemAstNode::Element {
                node_id,
                expanded_name,
                attributes,
                source,
                ..
            } = node.source.node()
            else {
                return None;
            };
            let name = &expanded_name.local_name;
            if name.is_empty() || name == "$" || name.starts_with('@') {
                return None;
            }
            let attributes = attributes
                .iter()
                .filter_map(|id| {
                    let CemAstNode::Attribute {
                        expanded_name,
                        value,
                        ..
                    } = node.source.document().get(*id)?
                    else {
                        return None;
                    };
                    Some((
                        expanded_name.local_name.clone(),
                        value.clone().unwrap_or_default(),
                    ))
                })
                .collect();
            Some((
                index,
                SchemaBehaviorCandidate {
                    node_id: *node_id,
                    element: name.clone(),
                    attributes,
                    source_map: source.clone(),
                },
            ))
        })
        .collect();
    let metadata: Vec<_> = candidates
        .iter()
        .map(|(_, candidate)| candidate.clone())
        .collect();
    let select_names = select_binding_names(model);
    let match_names = match_binding_names(model, &metadata);
    let mut diagnostics = Vec::new();
    for diagnostic in model
        .diagnostic_behaviors
        .values()
        .filter(|d| d.function.is_some())
    {
        let Some(definition) = diagnostic.definition.as_ref() else {
            continue;
        };
        let (Some(select), Some(match_query)) = (
            definition.select.as_deref(),
            definition.match_query.as_deref(),
        ) else {
            continue;
        };
        if let Some(function) = diagnostic_behavior_function(diagnostic, definition) {
            if function.params.iter().any(|param| {
                param.name == "candidate" && param.value_type.rsplit(':').next() != Some("node")
            }) {
                diagnostics.push(schema_behavior_diagnostic(
                    SCHEMA_BEHAVIOR_FUNCTION_FAILED_CODE, Severity::Error,
                    "retained behavior candidate parameters require explicit `node` type; `object` signatures belong to whole-document evaluation".to_owned(),
                    &definition.source_map, json!({"schemaUri":model.schema_uri,"behavior":diagnostic.behavior}),
                ));
                continue;
            }
        }
        let mut bindings: BTreeMap<String, ItemStream> = select_names
            .iter()
            .map(|name| (name.clone(), ItemStream::empty()))
            .collect();
        for (index, candidate) in &candidates {
            let item = tree.node(*index).unwrap();
            bindings.get_mut("nodes").unwrap().items.push(item.clone());
            if let Some(stream) = bindings.get_mut(&candidate.element) {
                stream.items.push(item);
            }
        }
        let selected = evaluate_cem_ql_behavior_query(select, &select_names, bindings)
            .map_err(|message| (SCHEMA_BEHAVIOR_QUERY_FAILED_CODE, message))
            .and_then(|stream| {
                let mut indices = BTreeSet::new();
                for item in stream.items {
                    let node = item
                        .view()
                        .and_then(|view| view.downcast_ref::<ValidationPlacementNode>())
                        .ok_or_else(|| {
                            (
                                SCHEMA_BEHAVIOR_RESULT_INVALID_CODE,
                                "select must return native element placements".to_owned(),
                            )
                        })?;
                    if !Arc::ptr_eq(node.owner(), &tree)
                        || !matches!(node.source_node().node(), CemAstNode::Element { .. })
                    {
                        return Err((
                            SCHEMA_BEHAVIOR_RESULT_INVALID_CODE,
                            "select returned a node outside this stage or a non-element node"
                                .to_owned(),
                        ));
                    }
                    indices.insert(node.placement());
                }
                Ok(indices)
            });
        let selected = match selected {
            Ok(selected) => selected,
            Err((code, message)) => {
                diagnostics.push(schema_behavior_diagnostic(code, Severity::Error,
                    format!("retained behavior select failed: {message}"), &definition.source_map,
                    json!({"schemaUri":model.schema_uri,"behavior":diagnostic.behavior,"query":select})));
                continue;
            }
        };
        for (index, candidate) in candidates
            .iter()
            .filter(|(index, _)| selected.contains(index))
        {
            let item = tree.node(*index).unwrap();
            let matched = evaluate_cem_ql_behavior_query(
                match_query,
                &match_names,
                candidate_match_bindings_with_item(candidate, &match_names, item.clone()),
            );
            match matched {
                Ok(stream) if !stream_truthy(&stream) => continue,
                Ok(_) => {}
                Err(message) => {
                    diagnostics.push(schema_behavior_diagnostic(SCHEMA_BEHAVIOR_QUERY_FAILED_CODE,Severity::Error,
                        format!("retained behavior match failed: {message}"),&candidate.source_map,
                        json!({"schemaUri":model.schema_uri,"behavior":diagnostic.behavior,"placement":index})));
                    continue;
                }
            }
            if let Some(mut result) = execute(model, diagnostic, definition, candidate, item) {
                if let Some(Value::Object(details)) = result.details.as_mut() {
                    details.insert("placement".to_owned(), json!(index));
                }
                diagnostics.push(result);
            }
        }
    }
    RetainedBehaviorValidation {
        complete: true,
        diagnostics,
    }
}

#[derive(Clone)]
enum Argument {
    Control(Value),
    Native(Vec<Item>),
}
fn execute(
    model: &SchemaDocumentModel,
    diagnostic: &DiagnosticBehavior,
    definition: &BehaviorDefinition,
    candidate: &SchemaBehaviorCandidate,
    item: Item,
) -> Option<Diagnostic> {
    let name = diagnostic.function.as_deref()?;
    let evaluate = || -> Result<(String, Value), String> {
        let function = diagnostic_behavior_function(diagnostic, definition)
            .ok_or_else(|| format!("behavior function `{name}` is not declared"))?;
        let body = function
            .body_expression
            .as_deref()
            .ok_or_else(|| format!("behavior function `{name}` has no body expression"))?;
        let mut arguments = BTreeMap::new();
        for param in &function.params {
            if param.name == "candidate" {
                arguments.insert(param.name.clone(), Argument::Native(vec![item.clone()]));
                continue;
            }
            match schema_behavior_function_argument_value(param, diagnostic, definition, candidate)?
            {
                Some(value) => {
                    arguments.insert(param.name.clone(), Argument::Control(value));
                }
                None if param.required => {
                    return Err(format!(
                        "required behavior parameter `{}` was not bound",
                        param.name
                    ))
                }
                None => {}
            }
        }
        let expression = parse_schema_behavior_expression(body)?;
        Ok((
            function.returns.clone(),
            evaluate_expression(&expression, &arguments)?,
        ))
    };
    match evaluate() {
        Ok((returns, result)) => {
            schema_behavior_result_diagnostic(model, diagnostic, name, candidate, &returns, result)
        }
        Err(message) => Some(schema_behavior_function_failed_diagnostic(
            model, diagnostic, name, candidate, message,
        )),
    }
}

fn evaluate_expression(
    expression: &SchemaBehaviorExpression,
    bindings: &BTreeMap<String, Argument>,
) -> Result<Value, String> {
    match expression {
        SchemaBehaviorExpression::Null => Ok(Value::Null),
        SchemaBehaviorExpression::Bool(v) => Ok(Value::Bool(*v)),
        SchemaBehaviorExpression::Number(v) => Ok(Value::Number(v.clone())),
        SchemaBehaviorExpression::String(v) => Ok(Value::String(v.clone())),
        SchemaBehaviorExpression::Path(path) => {
            let (root, segments) = path.split_first().ok_or("empty behavior path")?;
            let argument = bindings
                .get(root)
                .ok_or_else(|| format!("unknown behavior binding `${root}`"))?;
            match argument {
                Argument::Control(value) => resolve_schema_behavior_path(
                    path,
                    &BTreeMap::from([(root.clone(), value.clone())]),
                ),
                Argument::Native(items) => {
                    let mut items = items.clone();
                    for segment in segments {
                        let mut next = Vec::new();
                        for item in items {
                            let fields = item
                                .view()
                                .and_then(|view| view.field(segment))
                                .ok_or_else(|| {
                                    format!(
                                        "native behavior path `${}` is unresolved",
                                        path.join(".")
                                    )
                                })?;
                            next.extend(fields);
                        }
                        items = next;
                    }
                    let values = items
                        .iter()
                        .map(atomic_output)
                        .collect::<Result<Vec<_>, _>>()?;
                    match values.len() {
                        0 => Ok(Value::Null),
                        1 => Ok(values.into_iter().next().unwrap()),
                        _ => Ok(Value::Array(values)),
                    }
                }
            }
        }
        SchemaBehaviorExpression::Array(items) => items
            .iter()
            .map(|item| evaluate_expression(item, bindings))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        SchemaBehaviorExpression::Object(fields) => {
            let mut object = Map::new();
            for (name, value) in fields {
                object.insert(name.clone(), evaluate_expression(value, bindings)?);
            }
            Ok(Value::Object(object))
        }
    }
}
fn atomic_output(item: &Item) -> Result<Value, String> {
    match item {
        Item::Atomic(AtomValue::String(v) | AtomValue::AnyUri(v)) => Ok(Value::String(v.clone())),
        Item::Atomic(AtomValue::Integer(v)) => Ok(json!(v)),
        Item::Atomic(AtomValue::Boolean(v)) => Ok(Value::Bool(*v)),
        Item::Atomic(AtomValue::Null) => Ok(Value::Null),
        Item::Atomic(AtomValue::Double(v)) => Number::from_f64(*v)
            .map(Value::Number)
            .ok_or_else(|| "non-finite diagnostic number".to_owned()),
        Item::Atomic(AtomValue::Decimal(v)) => v
            .parse::<Number>()
            .map(Value::Number)
            .map_err(|e| e.to_string()),
        _ => Err(
            "diagnostic output requires explicit scalar extraction from native nodes".to_owned(),
        ),
    }
}
