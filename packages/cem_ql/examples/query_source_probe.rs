//! Test-only WASM harness. Not linked into either production WASM entry point.
//! Endpoints below observe real Rust owners; packets never recreate old handles.
#![cfg(target_arch = "wasm32")]
use cem_ml::{
    events::cem::CemEventNormalizer,
    operation_control::OperationControl,
    parser::CemAstNode,
    runtime_admission::{self, generation::GenerationDrain},
    schema::{
        declaration_references::SchemaDeclarationNode,
        machine::CemSchemaMachine,
        reference_policy::ReferenceScopePolicy,
        registry::CEM_SCHEMA_URI,
        value_contracts::{ContractName, ValueContractSource, ValueContracts},
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use cem_ql::{
    datatype_results::DatatypeResultAdapter,
    datatype_validation::ValidationInput,
    eval::{AtomValue, Item},
    query_validator_capsule::handoff::{self, QueryHandoffPin},
    schema_references::DeclarationScope,
    selected_registry::{SelectedBuiltin, SelectedFunctionRegistry},
    structural_receiver::{FreshQueryBinding, FreshQueryInvocation, FreshStructuralHost},
};
use std::{cell::RefCell, collections::BTreeMap, sync::Arc, time::Duration};
use wasm_bindgen::prelude::*;
struct Session {
    host: FreshStructuralHost,
    binding: FreshQueryBinding,
    request: FreshQueryInvocation,
    control: OperationControl,
    scope: DeclarationScope,
}
thread_local! {
    static SESSIONS: RefCell<Vec<Option<Session>>> = const { RefCell::new(Vec::new()) };
    static DRAIN: RefCell<Option<GenerationDrain>> = const { RefCell::new(None) };
}
fn error(e: impl std::fmt::Debug) -> JsValue {
    JsValue::from_str(&format!("{e:?}"))
}
fn adapter() -> Result<DatatypeResultAdapter, JsValue> {
    // Trusted fixture registration, built from the original shipped schema. It
    // is not a packet-supplied implementation or a minimal-runtime size proof.
    let text = cem_ml::schema::package_sources::builtin_schema_package_source("schema")
        .unwrap()
        .schema_source;
    let capture = Arc::new(
        CemSchemaMachine::new(
            CompiledSchema::cem_core(),
            CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
                SourceId(1),
                text.as_bytes().to_vec(),
            ))),
        )
        .build_with_lexical_scopes(),
    );
    let schema = capture
        .document()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "schema" => Some(*node_id),
            _ => None,
        })
        .ok_or_else(|| error("missing fixture schema"))?;
    let source = ValueContractSource::new(
        SchemaDeclarationNode::new(capture.document().clone(), schema).unwrap(),
        CEM_SCHEMA_URI,
        BTreeMap::from([("schema".into(), CEM_SCHEMA_URI.into())]),
    )
    .with_captured_names(capture)
    .map_err(error)?;
    let contracts = ValueContracts::compile(&[source], Default::default()).map_err(error)?;
    DatatypeResultAdapter::new(
        Arc::new(contracts),
        ContractName::new(CEM_SCHEMA_URI, "datatype-validation-result"),
        ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"),
    )
    .map_err(error)
}
#[wasm_bindgen(js_name = probeOpen)]
pub fn open(bytes: &[u8], pins: &str, functions: bool) -> Result<u32, JsValue> {
    open_with_queue(bytes, pins, functions, None)
}
#[wasm_bindgen(js_name = probeOpenBudget)]
pub fn open_budget(bytes: &[u8], pins: &str) -> Result<u32, JsValue> {
    open_with_queue(bytes, pins, true, Some(4))
}
fn open_with_queue(
    bytes: &[u8],
    pins: &str,
    functions: bool,
    queue: Option<u32>,
) -> Result<u32, JsValue> {
    let _entry = runtime_admission::admit(false).map_err(error)?;
    if bytes.len() > handoff::MAX_BYTES
        || pins.len() > 16 * 1024
        || SESSIONS.with(|s| s.borrow().len() >= 64)
    {
        return Err(error("fixture input limit"));
    }
    let control = OperationControl::default();
    let pin: QueryHandoffPin = serde_json::from_str(pins).map_err(error)?;
    let source = handoff::verify(bytes, &pin, &control).map_err(error)?;
    let generation = runtime_admission::linked_generation().map_err(error)?;
    let mut host = FreshStructuralHost::new_in_generation(
        &format!("cem-bin/1+blake3:{}", "a".repeat(64)),
        control.clone(),
        &generation,
    )
    .map_err(error)?;
    let d = source.declaration();
    let mut context = cem_ql::api::StandaloneExpressionContext::default();
    if let Some(queue) = queue {
        context.scope_policy = cem_ml::scheduler::ScopePolicy::host_root().with_queue_size(queue);
    }
    let scope = host
        .register_captured_scope(
            d.source().clone(),
            d.capture().clone(),
            Some(context),
            ReferenceScopePolicy::schema_defaults().map_err(error)?,
        )
        .map_err(error)?;
    let selected = SelectedFunctionRegistry::new(if functions {
        vec![
            SelectedBuiltin::StringLength,
            SelectedBuiltin::SequenceCount,
        ]
    } else {
        vec![]
    })
    .map_err(error)?;
    let binding = host
        .link_query_handoff(&source, scope, &selected, adapter()?)
        .map_err(error)?;
    let request = host.query_invocation(scope).map_err(error)?;
    SESSIONS.with(|s| {
        let mut s = s.borrow_mut();
        s.push(Some(Session {
            host,
            binding,
            request,
            control,
            scope,
        }));
        (s.len() - 1).try_into().map_err(error)
    })
}
#[wasm_bindgen(js_name = probeValidate)]
pub fn validate(id: u32, value: &str) -> Result<String, JsValue> {
    SESSIONS.with(|s| {
        let s = s.borrow();
        let s = s
            .get(id as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| error("missing original session"))?;
        let batch = s
            .host
            .validate_query(
                &s.request,
                &s.binding,
                &ValidationInput {
                    value: vec![Item::Atomic(AtomValue::String(value.into()))],
                    candidate: vec![],
                    fallback: Default::default(),
                },
                Default::default(),
            )
            .map_err(error)?;
        let diagnostics = batch.completed.first().map(|r| &r.result.diagnostics);
        Ok(serde_json::json!({"accepted":batch.accepted,"diagnostics":diagnostics}).to_string())
    })
}
#[wasm_bindgen(js_name = probeCrossRefused)]
pub fn cross_refused(request: u32, binding: u32) -> Result<bool, JsValue> {
    SESSIONS.with(|s| {
        let s = s.borrow();
        let a = s
            .get(request as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| error("request"))?;
        let b = s
            .get(binding as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| error("binding"))?;
        Ok(b.host
            .validate_query(
                &a.request,
                &b.binding,
                &ValidationInput {
                    value: vec![Item::Atomic(AtomValue::String("same".into()))],
                    candidate: vec![],
                    fallback: Default::default(),
                },
                Default::default(),
            )
            .is_err())
    })
}
#[wasm_bindgen(js_name = probeCapture)]
pub fn capture(id: u32) -> Result<Vec<u8>, JsValue> {
    SESSIONS.with(|s| {
        let s = s.borrow();
        let s = s
            .get(id as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| error("session"))?;
        Ok(s.host
            .capture_query_handoff(&s.binding)
            .map_err(error)?
            .bytes)
    })
}
#[wasm_bindgen(js_name = probeFence)]
pub fn fence(id: u32, mode: &str) -> Result<(), JsValue> {
    SESSIONS.with(|s| {
        let mut s = s.borrow_mut();
        let s = s
            .get_mut(id as usize)
            .and_then(Option::as_mut)
            .ok_or_else(|| error("session"))?;
        match mode {
            "cancel" => {
                s.control.cancel_root(None, None).map_err(error)?;
            }
            "context" => {
                if !s.host.set_context(s.scope, Some(Default::default())) {
                    return Err(error("context"));
                }
            }
            "retire" => {
                s.request.retire();
                s.binding.retire();
                s.host.close();
            }
            _ => return Err(error("mode")),
        }
        Ok(())
    })
}
#[wasm_bindgen(js_name = probeDrop)]
pub fn drop_session(id: u32) -> Result<(), JsValue> {
    let _entry = runtime_admission::admit(true).map_err(error)?;
    SESSIONS.with(|s| {
        s.borrow_mut()
            .get_mut(id as usize)
            .ok_or_else(|| error("session"))?
            .take();
        Ok(())
    })
}
#[wasm_bindgen(js_name = probeInventory)]
pub fn inventory() -> Result<String, JsValue> {
    serde_json::to_string(
        &runtime_admission::linked_generation()
            .map_err(error)?
            .inventory()
            .map_err(error)?,
    )
    .map_err(error)
}
#[wasm_bindgen(js_name = probeClose)]
pub fn close() -> Result<(), JsValue> {
    DRAIN.with(|d| {
        *d.borrow_mut() = Some(
            runtime_admission::linked_generation()
                .map_err(error)?
                .begin_drain(Duration::from_secs(5), Default::default())
                .map_err(error)?,
        );
        Ok(())
    })
}
#[wasm_bindgen(js_name = probeSeal)]
pub fn seal() -> Result<bool, JsValue> {
    let g = runtime_admission::linked_generation().map_err(error)?;
    DRAIN.with(|d| {
        Ok(d.borrow()
            .as_ref()
            .ok_or_else(|| error("drain"))?
            .seal()
            .is_ok_and(|w| w.is_current(&g)))
    })
}
#[wasm_bindgen(js_name = probeMemory)]
pub fn memory() -> JsValue {
    wasm_bindgen::memory()
}
