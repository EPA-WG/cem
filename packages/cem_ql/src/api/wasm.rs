//! WASM-callable CEM-QL template render boundary.
//!
//! The Rust render module owns the semantic boundary. This module keeps the
//! browser transport deliberately small: JSON strings cross the JS/WASM edge,
//! and compiled templates live behind opaque handles in WASM memory.

use std::cell::RefCell;

use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

use super::json_boundary::{diagnostics_json, evaluate_query_source_json, json_value_to_stream};
use crate::render::{
    compile_template, render_compiled_template, CompileTemplateOptions, RenderPlan, RenderPlanNode,
    TemplateArtifact, TemplateData,
};

thread_local! {
    static ARTIFACTS: RefCell<Vec<Option<TemplateArtifact>>> = const { RefCell::new(Vec::new()) };
}

#[wasm_bindgen(js_name = "version")]
pub fn wasm_version() -> String {
    crate::VERSION.to_owned()
}

// `convertLegacyCustomElementTemplate` is provided by the `cem_ml` crate's own `#[wasm_bindgen]`
// export (`cem_ml::api::wasm`), which is linked into this `cem_ql` WASM module (cem_ql depends on
// cem_ml). The browser loads the cem_ql module and gets the one CEM-owned legacy compiler — no
// re-export needed here (re-exporting would collide on the `js_name`).

#[wasm_bindgen(js_name = "compileTemplate")]
pub fn wasm_compile_template(source: &str, host_bindings_json: &str) -> String {
    let host_bindings = match parse_host_bindings(host_bindings_json) {
        Ok(bindings) => bindings,
        Err(message) => return error_json("cem.ql.wasm.invalid_host_bindings", message),
    };
    let artifact = compile_template(
        source,
        &CompileTemplateOptions {
            host_bindings,
            ..CompileTemplateOptions::default()
        },
    );
    let diagnostics = diagnostics_json(&artifact.diagnostics);
    let artifact_id = ARTIFACTS.with(|cell| {
        let mut artifacts = cell.borrow_mut();
        artifacts.push(Some(artifact));
        artifacts.len() as u32
    });
    json!({
        "artifactId": artifact_id,
        "diagnostics": diagnostics
    })
    .to_string()
}

#[wasm_bindgen(js_name = "renderTemplate")]
pub fn wasm_render_template(artifact_id: u32, data_json: &str) -> String {
    let data = match parse_template_data(data_json) {
        Ok(data) => data,
        Err(message) => return error_json("cem.ql.wasm.invalid_data", message),
    };
    ARTIFACTS.with(|cell| {
        let artifacts = cell.borrow();
        let Some(Some(artifact)) = artifact_id
            .checked_sub(1)
            .and_then(|index| artifacts.get(index as usize))
        else {
            return error_json(
                "cem.ql.wasm.unknown_artifact",
                format!("template artifact `{artifact_id}` is not registered"),
            );
        };
        plan_json(&render_compiled_template(artifact, &data)).to_string()
    })
}

#[wasm_bindgen(js_name = "renderTemplateSource")]
pub fn wasm_render_template_source(source: &str, data_json: &str) -> String {
    let data = match parse_template_data(data_json) {
        Ok(data) => data,
        Err(message) => return error_json("cem.ql.wasm.invalid_data", message),
    };
    let artifact = compile_template(
        source,
        &CompileTemplateOptions {
            host_bindings: data.bindings.keys().cloned().collect(),
            ..CompileTemplateOptions::default()
        },
    );
    plan_json(&render_compiled_template(&artifact, &data)).to_string()
}

#[wasm_bindgen(js_name = "evaluateQuerySource")]
pub fn wasm_evaluate_query_source(source: &str, bindings_json: &str) -> String {
    evaluate_query_source_json(source, bindings_json)
}

#[wasm_bindgen(js_name = "disposeTemplate")]
pub fn wasm_dispose_template(artifact_id: u32) -> bool {
    ARTIFACTS.with(|cell| {
        let mut artifacts = cell.borrow_mut();
        let Some(slot) = artifact_id
            .checked_sub(1)
            .and_then(|index| artifacts.get_mut(index as usize))
        else {
            return false;
        };
        let existed = slot.is_some();
        *slot = None;
        existed
    })
}

fn parse_host_bindings(input: &str) -> Result<Vec<String>, String> {
    if input.trim().is_empty() {
        return Ok(Vec::new());
    }
    let value: Value = serde_json::from_str(input).map_err(|err| err.to_string())?;
    match value {
        Value::Array(items) => items
            .into_iter()
            .map(|item| match item {
                Value::String(name) => Ok(name),
                _ => Err("host bindings must be an array of strings".to_owned()),
            })
            .collect(),
        Value::Object(map) => Ok(map.keys().cloned().collect()),
        _ => Err("host bindings must be an array of strings or an object".to_owned()),
    }
}

fn parse_template_data(input: &str) -> Result<TemplateData, String> {
    if input.trim().is_empty() {
        return Ok(TemplateData::default());
    }
    let value: Value = serde_json::from_str(input).map_err(|err| err.to_string())?;
    let Value::Object(map) = value else {
        return Err("template data must be a JSON object".to_owned());
    };
    let mut data = TemplateData::default();
    for (name, value) in map {
        data.bindings.insert(name, json_value_to_stream(value)?);
    }
    Ok(data)
}

fn plan_json(plan: &RenderPlan) -> Value {
    json!({
        "nodes": plan.nodes.iter().map(node_json).collect::<Vec<_>>(),
        "diagnostics": diagnostics_json(&plan.diagnostics)
    })
}

fn node_json(node: &RenderPlanNode) -> Value {
    match node {
        RenderPlanNode::Element {
            tag,
            namespace,
            attributes,
            children,
            source_map,
        } => json!({
            "kind": "element",
            "tag": tag,
            "namespace": namespace,
            "attributes": attributes.iter().map(|attribute| json!({
                "name": attribute.name,
                "namespace": attribute.namespace,
                "value": attribute.value,
                "byteOffset": source_map_offset(&attribute.source_map),
                "sourceMap": source_map_json(&attribute.source_map)
            })).collect::<Vec<_>>(),
            "children": children.iter().map(node_json).collect::<Vec<_>>(),
            "byteOffset": source_map_offset(source_map),
            "sourceMap": source_map_json(source_map)
        }),
        RenderPlanNode::Text { text, source_map } => json!({
            "kind": "text",
            "text": text,
            "byteOffset": source_map_offset(source_map),
            "sourceMap": source_map_json(source_map)
        }),
        RenderPlanNode::Comment { text, source_map } => json!({
            "kind": "comment",
            "text": text,
            "byteOffset": source_map_offset(source_map),
            "sourceMap": source_map_json(source_map)
        }),
        RenderPlanNode::Cdata { text, source_map } => json!({
            "kind": "cdata",
            "text": text,
            "byteOffset": source_map_offset(source_map),
            "sourceMap": source_map_json(source_map)
        }),
        RenderPlanNode::ProcessingInstruction {
            target,
            data,
            source_map,
        } => json!({
            "kind": "processing-instruction",
            "target": target,
            "data": data,
            "byteOffset": source_map_offset(source_map),
            "sourceMap": source_map_json(source_map)
        }),
    }
}

/// The current (most specific) frame's absolute byte offset, for host
/// `cem:<offset>` source-frame attributes. `None` when no frame is present.
fn source_map_offset(source_map: &cem_ml::source_map::SourceMapStack) -> Option<u64> {
    use cem_ml::source_map::FrameSpan;
    source_map.current().map(|frame| match &frame.span {
        FrameSpan::Single(range) => range.start,
        FrameSpan::Multi(ranges) => ranges.first().map(|range| range.start).unwrap_or(0),
    })
}

fn source_map_json(source_map: &cem_ml::source_map::SourceMapStack) -> Value {
    serde_json::to_value(source_map).unwrap_or(Value::Null)
}

fn error_json(code: &str, message: impl Into<String>) -> String {
    json!({
        "nodes": [],
        "diagnostics": [{
            "code": code,
            "severity": "error",
            "message": message.into()
        }]
    })
    .to_string()
}
