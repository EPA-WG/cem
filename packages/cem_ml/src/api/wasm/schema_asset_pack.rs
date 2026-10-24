//! Optional probes and one embedding-owned package context per physical runtime.
//! Original sources and native contracts never leave the owning heap.
use crate::runtime_inventory::NativeRegistry;
use crate::schema::asset_pack_session::SchemaAssetSession;
use crate::{
    operation_control::OperationControl,
    runtime_memory::{runtime_budget, runtime_sample, RuntimeMemoryPermit},
    scheduler::AbortSignal,
    schema::asset_pack::{SchemaAssetPack, SchemaAssetPackLimits},
};
use js_sys::Uint8Array;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

thread_local! {
    static OWNER: NativeRegistry<Option<(u32, SchemaAssetSession)>> = const { NativeRegistry::new(None) };
    static NEXT_OWNER: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Trusted embedding selection is per physical heap. Never serialized in a
/// checkpoint. In-flight commands clone the context and retain NativeActivity.
pub fn active_context() -> Option<crate::engine::EngineContext> {
    OWNER.with(|owner| {
        owner.borrow().as_ref().and_then(|(_, session)| {
            session
                .context()
                .schema_asset_activation
                .as_ref()
                .map(|_| session.context().clone())
        })
    })
}
pub(super) fn inventory_count() -> u64 {
    OWNER.with(|owner| u64::from(owner.borrow().is_some()))
}

#[wasm_bindgen(js_name = "schemaAssetOwnerOpenV1")]
pub fn owner_open(index: Uint8Array, pin: &str, host_configuration: &str) -> Result<u32, JsValue> {
    let _native = match crate::runtime_admission::admit(false) {
        Ok(permit) => permit,
        Err(error) => return Err(failure(error)),
    };
    let _memory = reserve(index.length() as usize, host_configuration.len())?;
    if host_configuration.len() > 64 * 1024 {
        return Err(failure("cem.schema_asset.limit"));
    }
    let config =
        crate::command_host_config::decode_command_host_configuration_v1(Some(host_configuration))
            .map_err(|_| failure("cem.schema_asset.policy"))?;
    let pack = pack(&index.to_vec(), pin)?;
    OWNER.with(|owner| {
        let mut owner = owner.borrow_mut();
        if owner.is_some() {
            return Err(failure("cem.schema_asset.owner_busy"));
        }
        let id = NEXT_OWNER.with(|next| {
            let id = next
                .get()
                .checked_add(1)
                .ok_or_else(|| failure("cem.schema_asset.limit"))?;
            next.set(id);
            Ok::<_, JsValue>(id)
        })?;
        let (mut context, _) = super::command_service::fresh_runtime_context();
        config.apply_to_context(&mut context);
        *owner = Some((id, SchemaAssetSession::new(pack, context)));
        Ok(id)
    })
}

#[wasm_bindgen(js_name = "schemaAssetOwnerAdmitV1")]
pub fn owner_admit(
    owner_id: u32,
    package_id: &str,
    version: &str,
    path: &str,
    bytes: Uint8Array,
) -> Result<(), JsValue> {
    let _native = match crate::runtime_admission::admit(false) {
        Ok(permit) => permit,
        Err(error) => return Err(failure(error)),
    };
    let _memory = reserve(0, bytes.length() as usize)?;
    if package_id.len() > 256 || version.len() > 64 || path.len() > 4096 {
        return Err(failure("cem.schema_asset.identity"));
    }
    OWNER.with(|owner| {
        let mut owner = owner.borrow_mut();
        let session = owner
            .as_mut()
            .filter(|(id, _)| *id == owner_id)
            .map(|(_, session)| session)
            .ok_or_else(|| failure("cem.schema_asset.owner"))?;
        let descriptor = session
            .pack()
            .source_descriptor(package_id, version, path)
            .map_err(|e| failure(e.code))?;
        if bytes.length() as usize != descriptor.byte_length {
            return Err(failure("cem.schema_asset.limit"));
        }
        session
            .admit(package_id, version, path, bytes.to_vec())
            .map_err(|e| failure(e.code))
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Selection {
    package_id: String,
    version: String,
}

#[wasm_bindgen(js_name = "schemaAssetOwnerActivateV1")]
pub fn owner_activate(owner_id: u32, selection: &str) -> Result<String, JsValue> {
    let _native = match crate::runtime_admission::admit(false) {
        Ok(permit) => permit,
        Err(error) => return Err(failure(error)),
    };
    let _memory = reserve(0, selection.len())?;
    if selection.len() > 64 * 1024 {
        return Err(failure("cem.schema_asset.limit"));
    }
    let selected: Vec<Selection> =
        serde_json::from_str(selection).map_err(|_| failure("cem.schema_asset.identity"))?;
    OWNER.with(|owner| {
        let mut owner = owner.borrow_mut();
        let session = owner
            .as_mut()
            .filter(|(id, _)| *id == owner_id)
            .map(|(_, session)| session)
            .ok_or_else(|| failure("cem.schema_asset.owner"))?;
        let _compilation_memory = reserve(0, session.pack().admitted_bytes())?;
        let identities: Vec<_> = selected
            .iter()
            .map(|p| (p.package_id.as_str(), p.version.as_str()))
            .collect();
        let diagnostics = session
            .activate(&identities)
            .map_err(|_| failure("cem.schema_asset.activation"))?;
        let ready = !diagnostics.iter().any(|d| d.severity.is_hard_violation());
        encode(
            &serde_json::json!({"ready": ready, "diagnosticCount": diagnostics.len(),
            "diagnosticCodes": diagnostics.iter().take(8).map(|d| d.code.chars().take(128).collect::<String>()).collect::<Vec<_>>(),
            "indexDigest": session.pack().index_digest()}),
        )
    })
}

#[wasm_bindgen(js_name = "schemaAssetOwnerDisposeV1")]
pub fn owner_dispose(owner_id: u32) -> Result<bool, JsValue> {
    let _native = match crate::runtime_admission::admit(true) {
        Ok(permit) => permit,
        Err(error) => return Err(failure(error)),
    };
    OWNER.with(|owner| {
        let mut owner = owner.borrow_mut();
        if owner.as_ref().is_some_and(|(id, _)| *id == owner_id) {
            owner.take();
            return Ok(true);
        }
        Ok(false)
    })
}

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

/// Embedding evidence only; not an activation or retirement capability.
#[wasm_bindgen(js_name = "schemaAssetEmbeddedSourcesV1")]
pub fn embedded_sources() -> Result<String, JsValue> {
    let _native = crate::runtime_admission::admit(false).map_err(failure)?;
    let _memory = reserve(4096, 0)?;
    encode(&crate::schema::package_sources::embedded_source_profile())
}

/// Advisory requirements for an already native-lowered request. Command
/// identity, inline-byte and memory budgets remain the normal service bounds.
#[wasm_bindgen(js_name = "schemaAssetDemandPlanV1")]
pub fn demand_plan(request_json: &str) -> Result<String, JsValue> {
    let _native = crate::runtime_admission::admit(false).map_err(failure)?;
    let _memory = crate::runtime_memory::admit_input(request_json.len()).map_err(|error| failure(error.0))?;
    let request = crate::command_service::decode_command_service_request_v1(request_json.as_bytes())
        .map_err(|error| failure(error.code()))?;
    let plan = crate::schema::asset_demand::plan_command_schema_assets(&request)
        .map_err(|error| failure(error.code()))?;
    encode(&plan)
}

#[wasm_bindgen(js_name = "schemaAssetPackBootstrapV1")]
pub fn bootstrap() -> Result<String, JsValue> {
    let _native = match crate::runtime_admission::admit(false) {
        Ok(permit) => permit,
        Err(error) => return Err(failure(error)),
    };
    let _memory = reserve(BOOTSTRAP.len(), 0)?;
    encode(&pinned_bootstrap()?)
}

#[wasm_bindgen(js_name = "schemaAssetPackSummaryV1")]
pub fn summary(index: Uint8Array, pin: &str) -> Result<String, JsValue> {
    let _native = match crate::runtime_admission::admit(false) {
        Ok(permit) => permit,
        Err(error) => return Err(failure(error)),
    };
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
    let _native = match crate::runtime_admission::admit(false) {
        Ok(permit) => permit,
        Err(error) => return Err(failure(error)),
    };
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
