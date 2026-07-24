use std::collections::BTreeMap;

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::scheduler::ScopePolicy;
use serde_json::{json, Map, Value};

use super::{compile, evaluate, CompileContext, EvaluationContext};
use crate::eval::{
    AtomValue, BudgetAxis, EvalError, Item, ItemStream, QueryContextScope, ResourceHandle,
};

const DEFAULT_WASM_QUEUE_SIZE: u32 = 256;

pub(crate) fn evaluate_query_source_json(source: &str, bindings_json: &str) -> String {
    let policy_bindings = match parse_policy_bindings(bindings_json) {
        Ok(bindings) => bindings,
        Err(message) => {
            return query_failure_json("input", "cem.ql.wasm.invalid_bindings", message).to_string()
        }
    };
    let query = match compile(
        source,
        &CompileContext {
            policy_bindings: policy_bindings.clone(),
            ..CompileContext::default()
        },
    ) {
        Ok(query) => query,
        Err(error) => return query_failure_json("compile", error.code, error.message).to_string(),
    };
    let stream = evaluate(
        &query,
        &EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: ScopePolicy::host_root().with_queue_size(DEFAULT_WASM_QUEUE_SIZE),
            diagnostics: Vec::new(),
            policy_bindings,
            current_item: None,
        },
    );
    query_result_json(&stream).to_string()
}

pub(crate) fn json_value_to_stream(value: Value) -> Result<ItemStream, String> {
    match value {
        Value::Object(mut map) if map.len() == 1 && map.contains_key("$stream") => {
            let value = map.remove("$stream").expect("checked reserved key");
            let Value::Array(items) = value else {
                return Err("`$stream` must contain an array of items".to_owned());
            };
            Ok(ItemStream::from_items(
                items
                    .into_iter()
                    .map(json_value_to_item)
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
        other => Ok(ItemStream::once(json_value_to_item(other)?)),
    }
}

pub(crate) fn diagnostics_json(diagnostics: &[Diagnostic]) -> Value {
    serde_json::to_value(diagnostics).unwrap_or_else(|_| Value::Array(Vec::new()))
}

fn parse_policy_bindings(input: &str) -> Result<BTreeMap<String, ItemStream>, String> {
    if input.trim().is_empty() {
        return Ok(BTreeMap::new());
    }
    let value: Value = serde_json::from_str(input).map_err(|err| err.to_string())?;
    let Value::Object(map) = value else {
        return Err("query bindings must be a JSON object".to_owned());
    };
    map.into_iter()
        .map(|(name, value)| json_value_to_stream(value).map(|stream| (name, stream)))
        .collect()
}

fn json_value_to_item(value: Value) -> Result<Item, String> {
    match value {
        Value::Null => Ok(Item::Atomic(AtomValue::Null)),
        Value::Bool(value) => Ok(Item::Atomic(AtomValue::Boolean(value))),
        Value::Number(value) => {
            if let Some(integer) = value.as_i64() {
                Ok(Item::Atomic(AtomValue::Integer(integer)))
            } else {
                Ok(Item::Atomic(AtomValue::Double(
                    value.as_f64().unwrap_or_default(),
                )))
            }
        }
        Value::String(value) => Ok(Item::Atomic(AtomValue::String(value))),
        Value::Array(items) => Ok(Item::Array(
            items
                .into_iter()
                .map(json_value_to_item)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        Value::Object(map) => object_to_item(map),
    }
}

fn object_to_item(mut map: Map<String, Value>) -> Result<Item, String> {
    if map.len() == 1 {
        if let Some(value) = map.remove("$node") {
            let Value::String(id) = value else {
                return Err("`$node` must contain a string node identity".to_owned());
            };
            return Ok(Item::Node(id));
        }
        if let Some(value) = map.remove("$atom") {
            return tagged_atom_to_item(value);
        }
        if let Some(value) = map.remove("$resource") {
            return tagged_resource_to_item(value);
        }
        if let Some(value) = map.remove("$array") {
            let Value::Array(items) = value else {
                return Err("`$array` must contain an array".to_owned());
            };
            return Ok(Item::Array(
                items
                    .into_iter()
                    .map(json_value_to_item)
                    .collect::<Result<Vec<_>, _>>()?,
            ));
        }
        if let Some(value) = map.remove("$record") {
            let Value::Object(fields) = value else {
                return Err("`$record` must contain an object".to_owned());
            };
            return record_fields_to_item(fields);
        }
    }
    record_fields_to_item(map)
}

fn record_fields_to_item(fields: Map<String, Value>) -> Result<Item, String> {
    Ok(Item::Record(
        fields
            .into_iter()
            .map(|(key, value)| {
                json_value_to_stream(value).map(|stream| {
                    let ItemStream { items, .. } = stream;
                    (key, items)
                })
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?,
    ))
}

fn tagged_atom_to_item(value: Value) -> Result<Item, String> {
    let Value::Object(mut atom) = value else {
        return Err("`$atom` must contain an object".to_owned());
    };
    let atom_type = take_string(&mut atom, "type")?;
    let value = atom.remove("value").unwrap_or(Value::Null);
    let atom = match atom_type.as_str() {
        "string" => AtomValue::String(expect_string(value, "string atom value")?),
        "integer" => AtomValue::Integer(expect_i64(value, "integer atom value")?),
        "decimal" => AtomValue::Decimal(expect_decimal_string(value)?),
        "double" => AtomValue::Double(expect_f64(value, "double atom value")?),
        "boolean" => AtomValue::Boolean(expect_bool(value, "boolean atom value")?),
        "any-uri" | "anyUri" => AtomValue::AnyUri(expect_string(value, "any-uri atom value")?),
        "null" => AtomValue::Null,
        other => return Err(format!("unsupported `$atom.type` `{other}`")),
    };
    Ok(Item::Atomic(atom))
}

fn tagged_resource_to_item(value: Value) -> Result<Item, String> {
    let Value::Object(mut resource) = value else {
        return Err("`$resource` must contain an object".to_owned());
    };
    let id = take_string(&mut resource, "id")?;
    let content_type = take_string(&mut resource, "contentType")?;
    let schema = match resource.remove("schema") {
        Some(Value::Null) | None => None,
        Some(Value::String(value)) => Some(value),
        Some(_) => return Err("`$resource.schema` must be a string or null".to_owned()),
    };
    let roles = match resource.remove("roles") {
        Some(Value::Array(values)) => values
            .into_iter()
            .map(|value| expect_string(value, "`$resource.roles` item"))
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => return Err("`$resource.roles` must be an array of strings".to_owned()),
        None => Vec::new(),
    };
    let fail_accessor = match resource.remove("failAccessor") {
        Some(value) => expect_bool(value, "`$resource.failAccessor`")?,
        None => false,
    };
    Ok(Item::Resource(ResourceHandle {
        id,
        content_type,
        schema,
        roles,
        fail_accessor,
    }))
}

fn take_string(map: &mut Map<String, Value>, field: &str) -> Result<String, String> {
    let Some(value) = map.remove(field) else {
        return Err(format!("missing required `{field}` field"));
    };
    expect_string(value, field)
}

fn expect_string(value: Value, label: &str) -> Result<String, String> {
    match value {
        Value::String(value) => Ok(value),
        _ => Err(format!("`{label}` must be a string")),
    }
}

fn expect_i64(value: Value, label: &str) -> Result<i64, String> {
    match value {
        Value::Number(value) => value
            .as_i64()
            .ok_or_else(|| format!("`{label}` must be an integer")),
        Value::String(value) => value
            .parse()
            .map_err(|_| format!("`{label}` must be an integer")),
        _ => Err(format!("`{label}` must be an integer")),
    }
}

fn expect_f64(value: Value, label: &str) -> Result<f64, String> {
    match value {
        Value::Number(value) => value
            .as_f64()
            .ok_or_else(|| format!("`{label}` must be a finite number")),
        Value::String(value) if value == "NaN" => Ok(f64::NAN),
        Value::String(value) if value == "Infinity" => Ok(f64::INFINITY),
        Value::String(value) if value == "-Infinity" => Ok(f64::NEG_INFINITY),
        Value::String(value) => value
            .parse()
            .map_err(|_| format!("`{label}` must be a number")),
        _ => Err(format!("`{label}` must be a number")),
    }
}

fn expect_bool(value: Value, label: &str) -> Result<bool, String> {
    match value {
        Value::Bool(value) => Ok(value),
        _ => Err(format!("`{label}` must be a boolean")),
    }
}

fn expect_decimal_string(value: Value) -> Result<String, String> {
    match value {
        Value::String(value) => Ok(value),
        Value::Number(value) => Ok(value.to_string()),
        _ => Err("decimal atom value must be a string or number".to_owned()),
    }
}

fn query_result_json(stream: &ItemStream) -> Value {
    json!({
        "items": stream.items.iter().map(item_json).collect::<Vec<_>>(),
        "diagnostics": diagnostics_json(&stream.diagnostics),
        "error": stream.error.as_ref().map(eval_error_json)
    })
}

fn query_failure_json(kind: &str, code: &str, message: impl Into<String>) -> Value {
    let message = message.into();
    json!({
        "items": [],
        "diagnostics": [{
            "code": code,
            "severity": Severity::Error,
            "message": message
        }],
        "error": {
            "kind": kind,
            "code": code,
            "message": message
        }
    })
}

fn item_json(item: &Item) -> Value {
    match item {
        Item::Node(id) => json!({
            "kind": "node",
            "id": id
        }),
        Item::Atomic(atom) => atom_json(atom),
        Item::Record(fields) => {
            let fields = fields
                .iter()
                .map(|(key, values)| {
                    (
                        key.clone(),
                        Value::Array(values.iter().map(item_json).collect::<Vec<_>>()),
                    )
                })
                .collect::<Map<_, _>>();
            json!({
                "kind": "record",
                "fields": fields
            })
        }
        Item::Array(items) => json!({
            "kind": "array",
            "items": items.iter().map(item_json).collect::<Vec<_>>()
        }),
        Item::Lambda(id) => json!({
            "kind": "lambda",
            "id": id.0
        }),
        Item::Resource(resource) => json!({
            "kind": "resource",
            "id": resource.id,
            "contentType": resource.content_type,
            "schema": resource.schema,
            "roles": resource.roles,
            "failAccessor": resource.fail_accessor
        }),
    }
}

fn atom_json(atom: &AtomValue) -> Value {
    match atom {
        AtomValue::String(value) => typed_atom_json("string", json!(value)),
        AtomValue::Integer(value) => typed_atom_json("integer", json!(value)),
        AtomValue::Decimal(value) => typed_atom_json("decimal", json!(value)),
        AtomValue::Double(value) => typed_atom_json("double", double_json(*value)),
        AtomValue::Boolean(value) => typed_atom_json("boolean", json!(value)),
        AtomValue::AnyUri(value) => typed_atom_json("any-uri", json!(value)),
        AtomValue::Null => typed_atom_json("null", Value::Null),
    }
}

fn typed_atom_json(atom_type: &str, value: Value) -> Value {
    json!({
        "kind": "atomic",
        "type": atom_type,
        "value": value
    })
}

fn double_json(value: f64) -> Value {
    if value.is_nan() {
        Value::String("NaN".to_owned())
    } else if value == f64::INFINITY {
        Value::String("Infinity".to_owned())
    } else if value == f64::NEG_INFINITY {
        Value::String("-Infinity".to_owned())
    } else {
        json!(value)
    }
}

fn eval_error_json(error: &EvalError) -> Value {
    match error {
        EvalError::BudgetExceeded(axis) => json!({
            "kind": "eval",
            "type": "budget-exceeded",
            "axis": budget_axis_json(*axis),
            "message": format!("budget exceeded for `{}`", axis.as_str())
        }),
        EvalError::Unsupported(message) => json!({
            "kind": "eval",
            "type": "unsupported",
            "message": message
        }),
        EvalError::TypeError(message) => json!({
            "kind": "eval",
            "type": "type-error",
            "message": message
        }),
    }
}

fn budget_axis_json(axis: BudgetAxis) -> Value {
    json!(axis.as_str())
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::evaluate_query_source_json;

    fn evaluate(source: &str, bindings: &str) -> Value {
        serde_json::from_str(&evaluate_query_source_json(source, bindings))
            .expect("query boundary result is JSON")
    }

    #[test]
    fn evaluates_source_with_plain_json_bindings() {
        let result = evaluate("left + right", r#"{"left":2,"right":3}"#);

        assert_eq!(
            result["items"],
            json!([{
                "kind": "atomic",
                "type": "integer",
                "value": 5
            }])
        );
        assert_eq!(result["diagnostics"], json!([]));
        assert_eq!(result["error"], Value::Null);
    }

    #[test]
    fn preserves_json_array_as_single_array_item_for_record_navigation() {
        let result = evaluate("rows.name", r#"{"rows":[{"name":"Ada"},{"name":"Lin"}]}"#);

        assert_eq!(
            result["items"],
            json!([
                {
                    "kind": "atomic",
                    "type": "string",
                    "value": "Ada"
                },
                {
                    "kind": "atomic",
                    "type": "string",
                    "value": "Lin"
                }
            ])
        );
    }

    #[test]
    fn accepts_explicit_stream_bindings_for_set_operators() {
        let result = evaluate(
            "left - right",
            r#"{"left":{"$stream":[1,2,3]},"right":{"$stream":[2,4]}}"#,
        );

        assert_eq!(
            result["items"],
            json!([
                {
                    "kind": "atomic",
                    "type": "integer",
                    "value": 1
                },
                {
                    "kind": "atomic",
                    "type": "integer",
                    "value": 3
                }
            ])
        );
    }

    #[test]
    fn accepts_explicit_node_bindings_for_node_identity() {
        let result = evaluate(
            "same_node(first, second)",
            r#"{"first":{"$node":"node-1"},"second":{"$node":"node-1"}}"#,
        );

        assert_eq!(
            result["items"],
            json!([{
                "kind": "atomic",
                "type": "boolean",
                "value": true
            }])
        );
    }

    #[test]
    fn reports_invalid_bindings_as_diagnostics() {
        let result = evaluate("1", "[]");

        assert_eq!(result["items"], json!([]));
        assert_eq!(
            result["diagnostics"][0]["code"],
            "cem.ql.wasm.invalid_bindings"
        );
        assert_eq!(result["error"]["kind"], "input");
    }

    #[test]
    fn reports_compile_errors_as_diagnostics() {
        let result = evaluate("1 + 1.0", "{}");

        assert_eq!(result["items"], json!([]));
        assert_eq!(result["diagnostics"][0]["code"], "cem.ql.compile_failed");
        assert_eq!(result["error"]["kind"], "compile");
    }
}
