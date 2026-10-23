//! Optional, stateless asset-pack probes. Transport copies are bounded before
//! allocation; original source parsing and provenance remain native. No store,
//! owner handle, recovery grant or credential state survives a call.
use crate::{
    operation_control::OperationControl,
    runtime_memory::{runtime_budget, runtime_sample, RuntimeMemoryPermit},
    scheduler::AbortSignal,
    schema::asset_pack::{SchemaAssetPack, SchemaAssetPackLimits},
};
use js_sys::Uint8Array;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

const BOOTSTRAP: &[u8] = include_bytes!("../../../schema/schema-assets-bootstrap-v1.json");

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Summary {
    index_digest: String,
    index_bytes: usize,
    package_count: usize,
    source_count: usize,
    source_bytes: usize,
}

fn failure(code: &str) -> JsValue {
    JsValue::from_str(code)
}

fn reserve(index_bytes: usize, source_bytes: usize) -> Result<RuntimeMemoryPermit, JsValue> {
    let limits = SchemaAssetPackLimits::default();
    if index_bytes > limits.max_index_bytes || source_bytes > limits.max_entry_bytes {
        return Err(failure("cem.schema_asset.limit"));
    }
    let work = index_bytes
        .checked_add(source_bytes)
        .and_then(|bytes| (bytes as u64).checked_mul(16))
        .and_then(|bytes| bytes.checked_add(64 * 1024))
        .ok_or_else(|| failure("cem.schema_asset.limit"))?;
    runtime_budget()
        .admit(runtime_sample(), work)
        .map_err(|error| failure(error.0))
}

fn pinned_bootstrap() -> Result<Summary, JsValue> {
    serde_json::from_slice(BOOTSTRAP).map_err(|_| failure("cem.schema_asset.bootstrap"))
}

fn pack(index: &[u8], pin: &str) -> Result<SchemaAssetPack, JsValue> {
    // The native-compiled pin is independent of the downloaded index/sidecars.
    if pin != pinned_bootstrap()?.index_digest {
        return Err(failure("cem.schema_asset.digest"));
    }
    SchemaAssetPack::from_index_bytes(index, pin, SchemaAssetPackLimits::default())
        .map_err(|error| failure(error.code))
}

fn encode(value: &impl Serialize) -> Result<String, JsValue> {
    serde_json::to_string(value).map_err(|_| failure("cem.schema_asset.response"))
}

#[wasm_bindgen(js_name = "schemaAssetPackBootstrapV1")]
pub fn bootstrap() -> Result<String, JsValue> {
    let _native = crate::runtime_admission::admit(false).map_err(failure)?;
    let _memory = reserve(BOOTSTRAP.len(), 0)?;
    encode(&pinned_bootstrap()?)
}

#[wasm_bindgen(js_name = "schemaAssetPackSummaryV1")]
pub fn summary(index: Uint8Array, pin: &str) -> Result<String, JsValue> {
    let _native = crate::runtime_admission::admit(false).map_err(failure)?;
    let _memory = reserve(index.length() as usize, 0)?;
    let bytes = index.to_vec();
    let pack = pack(&bytes, pin)?;
    encode(&Summary {
        index_digest: pack.index_digest().into(),
        index_bytes: bytes.len(),
        package_count: pack.index().packages.len(),
        source_count: pack.index().sources.len(),
        source_bytes: pack
            .index()
            .sources
            .iter()
            .map(|source| source.byte_length)
            .sum(),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportedSource<'a> {
    index_digest: &'a str,
    package_id: &'a str,
    version: &'a str,
    path: &'a str,
    digest: &'a str,
    source_uri: &'a str,
    source_bytes: usize,
    node_count: usize,
}

#[wasm_bindgen(js_name = "schemaAssetPackImportV1")]
pub fn import(
    index: Uint8Array,
    pin: &str,
    package_id: &str,
    version: &str,
    path: &str,
    source: Uint8Array,
) -> Result<String, JsValue> {
    let _native = crate::runtime_admission::admit(false).map_err(failure)?;
    let _memory = reserve(index.length() as usize, source.length() as usize)?;
    if package_id.len() > 256 || version.len() > 64 || path.len() > 4096 {
        return Err(failure("cem.schema_asset.identity"));
    }
    let mut pack = pack(&index.to_vec(), pin)?;
    let descriptor = pack
        .source_descriptor(package_id, version, path)
        .map_err(|error| failure(error.code))?
        .clone();
    if source.length() as usize != descriptor.byte_length {
        return Err(failure("cem.schema_asset.limit"));
    }
    let control = OperationControl::new(AbortSignal::new());
    pack.admit_source(
        package_id,
        version,
        path,
        source.to_vec(),
        &descriptor.content_type,
        &control,
        control.root_scope(),
    )
    .map_err(|error| failure(error.code))?;
    let imported = pack
        .import_source_with_control(package_id, version, path, &control, control.root_scope())
        .map_err(|error| failure(error.code))?;
    encode(&ImportedSource {
        index_digest: pack.index_digest(),
        package_id,
        version,
        path,
        digest: &descriptor.digest,
        source_uri: imported.tree.source_uri(),
        source_bytes: descriptor.byte_length,
        node_count: imported.tree.ast().nodes.len(),
    })
}
