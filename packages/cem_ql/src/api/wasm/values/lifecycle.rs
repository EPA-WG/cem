//! Opaque execution/snapshot handles. JSON is transport control metadata only.
use super::*;
use crate::api::reference_lifecycle::{
    query_snapshot::ReferenceQuerySnapshot, resources::ReferenceResourceExecution,
};
use std::sync::Arc;
thread_local! {
    static EXECUTIONS: RefCell<BTreeMap<u32, ReferenceResourceExecution>> = const { RefCell::new(BTreeMap::new()) };
    static SNAPSHOTS: RefCell<BTreeMap<u32, Arc<ReferenceQuerySnapshot>>> = const { RefCell::new(BTreeMap::new()) };
}
#[wasm_bindgen(js_name = "startReferenceResourceExecution")]
pub fn start(session_id: u32) -> Result<u32, JsValue> {
    let execution = VALIDATIONS.with(|sessions| {
        sessions
            .borrow()
            .get(&session_id)
            .ok_or_else(|| validation_error("Unknown validation session"))?
            .start_resources(Default::default())
            .map_err(validation_error)
    })?;
    let id = next_id().map_err(validation_error)?;
    EXECUTIONS.with(|runs| runs.borrow_mut().insert(id, execution));
    Ok(id)
}
#[wasm_bindgen(js_name = "advanceReferenceResourceExecution")]
pub fn advance(id: u32) -> Result<String, JsValue> {
    EXECUTIONS.with(|runs| {
        let progress = runs
            .borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| validation_error("Unknown resource execution"))?
            .advance()
            .map_err(validation_error)?;
        serde_json::to_string(&progress).map_err(validation_error)
    })
}
#[wasm_bindgen(js_name = "completeReferenceResource")]
pub fn complete(
    id: u32,
    request_id: u32,
    bytes: &[u8],
    content_type: &str,
    final_uri: &str,
) -> Result<String, JsValue> {
    if bytes.len() > cem_ml::ast::reload::ReloadLimits::default().max_bytes {
        return Err(validation_error("Resource byte limit exceeded"));
    }
    let source = EXECUTIONS.with(|runs| {
        runs.borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| validation_error("Unknown resource execution"))?
            .complete(
                request_id.into(),
                Ok(cem_ml::resolver::ResolvedRead {
                    uri: final_uri.into(),
                    bytes: bytes.into(),
                    content_type: (!content_type.is_empty()).then(|| content_type.into()),
                }),
            )
            .map_err(validation_error)
    })?;
    let Some(source) = source else {
        return Ok("null".into());
    };
    Ok(json!({"sourceIndex":source.index, "sourceId":retain_source(source.source)?}).to_string())
}
#[wasm_bindgen(js_name = "failReferenceResource")]
pub fn fail_resource(id: u32, request_id: u32, reason: &str) -> Result<(), JsValue> {
    EXECUTIONS.with(|runs| {
        runs.borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| validation_error("Unknown resource execution"))?
            .complete(
                request_id.into(),
                Err(cem_ml::diagnostics::Diagnostic {
                    code: "cem.reference.resource_unavailable".into(),
                    severity: cem_ml::diagnostics::Severity::Error,
                    message: reason.into(),
                    ..Default::default()
                }),
            )
            .map(|_| ())
            .map_err(validation_error)
    })
}
#[wasm_bindgen(js_name = "setReferenceResourceContext")]
pub fn context(
    id: u32,
    source_index: u32,
    ready: bool,
    bindings_json: &str,
) -> Result<(), JsValue> {
    let context = validation_context(ready, bindings_json)?;
    EXECUTIONS.with(|runs| {
        runs.borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| validation_error("Unknown resource execution"))?
            .set_loaded_context(source_index as usize, context)
            .map_err(validation_error)
    })
}
#[wasm_bindgen(js_name = "allowReferenceResourceCrossing")]
pub fn crossing(id: u32, from: u32, to: u32) -> Result<(), JsValue> {
    EXECUTIONS.with(|runs| {
        runs.borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| validation_error("Unknown resource execution"))?
            .allow_crossing(from as usize, to as usize)
            .map_err(validation_error)
    })
}
#[wasm_bindgen(js_name = "cancelReferenceResourceExecution")]
pub fn cancel(id: u32) -> Result<(), JsValue> {
    EXECUTIONS.with(|runs| {
        runs.borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| validation_error("Unknown resource execution"))?
            .cancel();
        Ok(())
    })
}
#[wasm_bindgen(js_name = "disposeReferenceResourceExecution")]
pub fn dispose_execution(id: u32) -> bool {
    EXECUTIONS.with(|runs| runs.borrow_mut().remove(&id).is_some())
}
#[wasm_bindgen(js_name = "prepareReferenceQuerySnapshot")]
pub fn snapshot(session_id: u32) -> Result<u32, JsValue> {
    let snapshot = VALIDATIONS.with(|sessions| {
        sessions
            .borrow()
            .get(&session_id)
            .ok_or_else(|| validation_error("Unknown validation session"))?
            .query_snapshot()
            .map_err(validation_error)
    })?;
    let id = next_id().map_err(validation_error)?;
    SNAPSHOTS.with(|snapshots| snapshots.borrow_mut().insert(id, Arc::new(snapshot)));
    Ok(id)
}
fn retained_snapshot(id: u32) -> Result<Arc<ReferenceQuerySnapshot>, JsValue> {
    SNAPSHOTS
        .with(|snapshots| snapshots.borrow().get(&id).cloned())
        .ok_or_else(|| validation_error("Unknown query snapshot"))
}
#[wasm_bindgen(js_name = "inspectReferenceQuerySnapshot")]
pub fn inspect_snapshot(id: u32) -> Result<String, JsValue> {
    serde_json::to_string(retained_snapshot(id)?.report()).map_err(validation_error)
}
#[wasm_bindgen(js_name = "queryReferenceQuerySnapshot")]
pub fn query_snapshot(id: u32, expression: &str, query_uri: &str) -> Result<u32, JsValue> {
    let snapshot = retained_snapshot(id)?;
    let context = crate::api::StandaloneExpressionContext {
        source_uri: Some(query_uri.into()),
        ..Default::default()
    };
    let evaluated = snapshot
        .evaluate(expression, &context)
        .map_err(|e| validation_error(e.message))?;
    if let Some(error) = &evaluated.result.error {
        return Err(validation_error(format!("{error:?}")));
    }
    let id = next_id().map_err(validation_error)?;
    // Native result nodes retain their completed-name view independently.
    INPUTS.with(|inputs| {
        inputs.borrow_mut().insert(
            id,
            RetainedInput {
                values: evaluated.result,
                source: Some(snapshot.source().clone()),
            },
        )
    });
    Ok(id)
}
#[wasm_bindgen(js_name = "disposeReferenceQuerySnapshot")]
pub fn dispose_snapshot(id: u32) -> bool {
    SNAPSHOTS.with(|snapshots| snapshots.borrow_mut().remove(&id).is_some())
}
fn occurrence(value_id: u32, index: u32) -> Result<crate::eval::Item, JsValue> {
    INPUTS
        .with(|inputs| {
            inputs
                .borrow()
                .get(&value_id)
                .and_then(|v| v.values.items.get(index as usize))
                .cloned()
        })
        .ok_or_else(|| validation_error("Unknown occurrence result handle or index"))
}
#[wasm_bindgen(js_name = "setReferenceValidationOccurrenceContext")]
pub fn occurrence_context(
    session: u32,
    source: u32,
    value_id: u32,
    index: u32,
    ready: bool,
    bindings: &str,
) -> Result<(), JsValue> {
    let value = occurrence(value_id, index)?;
    let context = validation_context(ready, bindings)?;
    VALIDATIONS.with(|sessions| {
        sessions
            .borrow_mut()
            .get_mut(&session)
            .ok_or_else(|| validation_error("Unknown validation session"))?
            .set_occurrence_value_context(source as usize, &value, context)
            .map_err(validation_error)
    })
}
#[wasm_bindgen(js_name = "clearReferenceValidationOccurrenceContext")]
pub fn clear_occurrence_context(
    session: u32,
    source: u32,
    value_id: u32,
    index: u32,
) -> Result<(), JsValue> {
    let value = occurrence(value_id, index)?;
    VALIDATIONS.with(|sessions| {
        sessions
            .borrow_mut()
            .get_mut(&session)
            .ok_or_else(|| validation_error("Unknown validation session"))?
            .clear_occurrence_value_context(source as usize, &value)
            .map_err(validation_error)
    })
}
#[wasm_bindgen(js_name = "setReferenceResourceOccurrenceContext")]
pub fn resource_occurrence_context(
    id: u32,
    source: u32,
    value_id: u32,
    index: u32,
    ready: bool,
    bindings: &str,
) -> Result<(), JsValue> {
    let value = occurrence(value_id, index)?;
    let context = validation_context(ready, bindings)?;
    EXECUTIONS.with(|runs| {
        runs.borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| validation_error("Unknown resource execution"))?
            .set_loaded_occurrence_value_context(source as usize, &value, context)
            .map_err(validation_error)
    })
}
#[wasm_bindgen(js_name = "clearReferenceResourceOccurrenceContext")]
pub fn clear_resource_occurrence_context(
    id: u32,
    source: u32,
    value_id: u32,
    index: u32,
) -> Result<(), JsValue> {
    let value = occurrence(value_id, index)?;
    EXECUTIONS.with(|runs| {
        runs.borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| validation_error("Unknown resource execution"))?
            .clear_loaded_occurrence_value_context(source as usize, &value)
            .map_err(validation_error)
    })
}
#[wasm_bindgen(js_name = "prepareReferenceResourceQuerySnapshot")]
pub fn resource_snapshot(id: u32, source: u32) -> Result<u32, JsValue> {
    let snapshot = EXECUTIONS.with(|runs| {
        runs.borrow()
            .get(&id)
            .ok_or_else(|| validation_error("Unknown resource execution"))?
            .query_snapshot(source as usize)
            .map_err(validation_error)
    })?;
    let id = next_id().map_err(validation_error)?;
    SNAPSHOTS.with(|snapshots| snapshots.borrow_mut().insert(id, Arc::new(snapshot)));
    Ok(id)
}
#[wasm_bindgen(js_name = "prepareReferenceResourceNamespaces")]
pub fn resource_names(id: u32, source: u32) -> Result<String, JsValue> {
    EXECUTIONS.with(|runs| {
        let report = runs
            .borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| validation_error("Unknown resource execution"))?
            .prepare_loaded_names(source as usize)
            .map_err(validation_error)?;
        serde_json::to_string(&report).map_err(validation_error)
    })
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PublicExport {
    part: String,
    select: String,
}
/// Host-owned explicit export selectors, evaluated only over the imported native
/// owner. They interpret no URL and create no crossing or package authority.
#[wasm_bindgen(js_name = "completeReferenceResourceWithExports")]
pub fn complete_with_exports(
    id: u32,
    request: u32,
    bytes: &[u8],
    content_type: &str,
    uri: &str,
    exports_json: &str,
) -> Result<String, JsValue> {
    if exports_json.len() > 128 * 1024
        || bytes.len() > cem_ml::ast::reload::ReloadLimits::default().max_bytes
    {
        return Err(validation_error(
            "Resource or export metadata limit exceeded",
        ));
    }
    let exports: Vec<PublicExport> =
        serde_json::from_str(exports_json).map_err(validation_error)?;
    let mut parts = BTreeMap::new();
    for export in exports {
        if export.part.is_empty()
            || export.select.trim().is_empty()
            || parts.insert(export.part, export.select).is_some()
        {
            return Err(validation_error(
                "Empty or duplicate public export contract",
            ));
        }
    }
    let loaded = EXECUTIONS.with(|runs| {
        runs.borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| validation_error("Unknown resource execution"))?
            .complete_with_exports(
                request.into(),
                Ok(cem_ml::resolver::ResolvedRead {
                    uri: uri.into(),
                    content_type: (!content_type.is_empty()).then(|| content_type.into()),
                    bytes: bytes.into(),
                }),
                |imported, part| {
                    let expression = parts
                        .get(part)
                        .ok_or("Requested part is not publicly exposed")?;
                    let context = crate::api::StandaloneExpressionContext::default().with_input(
                        crate::eval::ItemStream::once(crate::eval::imported_cem_tree(
                            imported.tree.clone(),
                        )),
                        crate::types::Type::Node(crate::types::NodeKind::Node),
                    );
                    let values = crate::api::evaluate_expression(expression, &context)
                        .map_err(|e| e.message)?
                        .result;
                    if let Some(error) = values.error {
                        return Err(format!("Public export selector: {error:?}"));
                    }
                    values
                        .items
                        .iter()
                        .map(|item| {
                            let node = crate::eval::retained_cem_node(item)
                                .ok_or("Export must select original native nodes")?;
                            if !Arc::ptr_eq(node.owner().ast_owner(), imported.tree.ast_owner()) {
                                return Err("Export selected a foreign source owner".into());
                            }
                            cem_ml::schema::declaration_references::SchemaDeclarationNode::new(
                                imported.tree.ast_owner().clone(),
                                node.node_id(),
                            )
                            .ok_or("Unknown original export node".into())
                        })
                        .collect()
                },
            )
            .map_err(validation_error)
    })?;
    match loaded {
        Some(loaded) => Ok(
            json!({"sourceIndex":loaded.index,"sourceId":retain_source(loaded.source)?})
                .to_string(),
        ),
        None => Ok("null".into()),
    }
}
