//! Pinned declarations and native stylesheet leases for the selected ABI.
//! The browser receives CSS/control metadata, never a retained native pointer.
use super::{render_loaded, MAX_INPUT_BYTES};
use crate::{
    declaration_artifact::{admit_declaration_metadata, DeclarationArtifactMetadata},
    retained_template::{
        RetainedStylesheet, RetainedTemplate, StylesheetLoadOptions, StylesheetLoadProgress,
    },
    template_artifact::CompiledTemplateArtifact,
};
use cem_ml::{
    css_emission::CssManagedScope,
    module_resolution::{
        CemModuleUrlContext, CemModuleUrlResolutionCapability, CemResolutionContextHandle,
        CemScopedModuleUrlResolver,
    },
};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

#[cfg(test)]
#[path = "owners_tests.rs"]
mod tests;
static OWNERS: AtomicUsize = AtomicUsize::new(0);
static LEASES: AtomicUsize = AtomicUsize::new(0);

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub struct CompiledDeclarationOwner {
    metadata: DeclarationArtifactMetadata,
    identity: String,
    retained: Option<RetainedTemplate>,
    leases: BTreeMap<(String, usize), (String, Arc<RetainedStylesheet>)>,
    next_consumer: u64,
}
struct OwnerAdmission {
    _entry: cem_ml::runtime_admission::NativeEntry<'static>,
    #[cfg(target_arch = "wasm32")]
    _memory: cem_ml::runtime_memory::RuntimeMemoryPermit,
}
fn admission(cleanup: bool, bytes: usize) -> Result<OwnerAdmission, String> {
    let entry = cem_ml::runtime_admission::admit(cleanup).map_err(str::to_owned)?;
    #[cfg(target_arch = "wasm32")]
    let memory = cem_ml::runtime_memory::admit_input(bytes).map_err(|error| error.to_string())?;
    #[cfg(not(target_arch = "wasm32"))]
    let _ = bytes;
    Ok(OwnerAdmission {
        _entry: entry,
        #[cfg(target_arch = "wasm32")]
        _memory: memory,
    })
}

impl CompiledDeclarationOwner {
    pub fn admit(bytes: &[u8], metadata_json: &str, pin: &str) -> Result<Self, String> {
        if bytes.len() > MAX_INPUT_BYTES || metadata_json.len() > MAX_INPUT_BYTES || pin.len() > 256
        {
            return Err("compiled declaration input limit exceeded".into());
        }
        let _permit = admission(false, bytes.len() + metadata_json.len())?;
        let metadata = admit_declaration_metadata(metadata_json, pin)
            .map_err(|_| "compiled declaration metadata refused")?;
        // This milestone owns passive declarations only. Widget/provider inputs
        // and their checkpoint contracts must be bound before those are admitted.
        if metadata.capability.is_some() {
            return Err("compiled declaration capability needs an owned runtime contract".into());
        }
        if !(bytes.starts_with(b"CEMTPLA2\n") || bytes.starts_with(b"CEMTPLA3\n")) {
            return Err("compiled declaration requires compiled CSS v2/v3 artifacts".into());
        }
        let envelope =
            CompiledTemplateArtifact::from_bytes(bytes.to_vec()).map_err(|e| e.code.to_owned())?;
        if envelope.identity.artifact_version != metadata.template_format {
            return Err("compiled declaration template format mismatch".into());
        }
        let artifact = envelope
            .reload_pinned(&metadata.template)
            .map_err(|e| e.code.to_owned())?;
        if artifact.stylesheets.len() > 64
            || artifact.module_map != metadata.module_map
            || artifact
                .stylesheets
                .iter()
                .any(|sheet| sheet.scope.is_some() && sheet.scope != metadata.scope)
        {
            return Err("compiled declaration stylesheet/module policy mismatch".into());
        }
        OWNERS
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |live| {
                (live < 64).then_some(live + 1)
            })
            .map_err(|_| "compiled declaration owner limit exceeded")?;
        cem_ml::runtime_inventory::note_mutation();
        Ok(Self {
            metadata,
            identity: pin.into(),
            retained: Some(RetainedTemplate::new(artifact)),
            leases: BTreeMap::new(),
            next_consumer: 0,
        })
    }
    pub fn describe_native(&self) -> Result<String, String> {
        let retained = self.active()?;
        Ok(serde_json::json!({"metadata":self.metadata, "stylesheets":retained.artifact().stylesheets.iter().enumerate().map(|(index,sheet)|serde_json::json!({"index":index,"scope":sheet.scope})).collect::<Vec<_>>(), "declarationIdentity":self.identity}).to_string())
    }
    pub fn render_native(&self, json: &[u8], uri: &str, binding: &str) -> Result<String, String> {
        let artifact = self.active()?.artifact();
        if !self
            .metadata
            .template
            .host_bindings
            .iter()
            .any(|name| name == binding)
        {
            return Err("compiled declaration needs a pinned native JSON binding".into());
        }
        let _permit = admission(false, json.len())?;
        render_loaded(artifact, json, uri, binding)
    }
    pub fn stylesheet_native(
        &mut self,
        consumer: &str,
        index: usize,
        context_json: &str,
    ) -> Result<Arc<RetainedStylesheet>, String> {
        self.active()?;
        if consumer.is_empty() || consumer.len() > 256 || context_json.len() > 64 * 1024 {
            return Err("compiled stylesheet control limit exceeded".into());
        }
        let key = (consumer.to_owned(), index);
        if !self.leases.contains_key(&key) && self.leases.len() >= 128 {
            return Err("compiled stylesheet lease limit exceeded".into());
        }
        let _permit = admission(false, context_json.len())?;
        let context: CemModuleUrlContext = serde_json::from_str(context_json)
            .map_err(|_| "invalid compiled stylesheet context")?;
        self.next_consumer = self
            .next_consumer
            .checked_add(1)
            .ok_or("compiled stylesheet identity exhausted")?;
        let attempt = format!("compiled-style:{}", self.next_consumer);
        let retained = self.retained.as_mut().unwrap();
        let scope = match &retained
            .artifact()
            .stylesheets
            .get(index)
            .ok_or("unknown compiled stylesheet occurrence")?
            .scope
        {
            Some(name) => CssManagedScope::Shared {
                name: name.clone(),
                context: None,
            },
            None => CssManagedScope::Private {
                tag: self.metadata.tag.clone(),
                context: None,
            },
        };
        let handle = CemResolutionContextHandle::new("compiled-declaration-style");
        let capability = CemModuleUrlResolutionCapability::new(
            Arc::new(CemScopedModuleUrlResolver::new().with_context(handle.clone(), context)),
            handle,
        );
        cem_ml::runtime_inventory::note_mutation();
        let id = retained
            .begin_stylesheet_load(StylesheetLoadOptions {
                consumer: attempt.clone(),
                index,
                scope,
                declaration_identity: self.identity.clone(),
                base_url: self.metadata.resource_base_url.clone(),
                capability,
                response_policy: Default::default(),
            })
            .map_err(|_| "compiled stylesheet policy refused")?;
        let result = match retained.advance_stylesheet_load(id, &attempt) {
            Ok(result) => result,
            Err(_) => {
                retained.release_stylesheet_generation(&attempt, id);
                return Err("compiled stylesheet policy refused".into());
            }
        };
        let record = match result {
            StylesheetLoadProgress::Ready(record) => record,
            StylesheetLoadProgress::Pending(_) => {
                retained.release_stylesheet_generation(&attempt, id);
                return Err(
                    "compiled stylesheet needs a closed import artifact; source fallback refused"
                        .into(),
                );
            }
        };
        if record.emission().map_err(str::to_owned)?.css().len() > MAX_INPUT_BYTES {
            retained.release_stylesheet_generation(&attempt, id);
            return Err("compiled stylesheet output limit exceeded".into());
        }
        if let Some((previous, _)) = self.leases.insert(key, (attempt, record.clone())) {
            retained.release_stylesheet_consumer(&previous);
        } else {
            LEASES.fetch_add(1, Ordering::SeqCst);
        }
        Ok(record)
    }
    pub fn stylesheet_json(
        &mut self,
        consumer: &str,
        index: usize,
        context: &str,
    ) -> Result<String, String> {
        let record = self.stylesheet_native(consumer, index, context)?;
        let emission = record.emission().map_err(str::to_owned)?;
        // Same native CSS identity/scoping output as the full processing host.
        let diagnostics=emission.diagnostics.iter().map(|diagnostic| {
            let source=record.source(diagnostic.sheet)?;
            Ok(serde_json::json!({"code":diagnostic.diagnostic.code,"message":diagnostic.diagnostic.message,"severity":"warning","sheet":diagnostic.sheet,"sourceUri":source.source_uri,"stylesheetUrl":source.stylesheet_url,"line":diagnostic.diagnostic.range.line,"column":diagnostic.diagnostic.range.column,"offset":diagnostic.diagnostic.range.offset,"length":diagnostic.diagnostic.range.length}))
        }).collect::<Result<Vec<_>,&str>>().map_err(str::to_owned)?;
        Ok(serde_json::json!({"status":"ready","loadId":"compiled","css":emission.css(),"identity":{"ownerKey":record.identity().owner_key,"cacheKey":record.identity().cache_key,"contextMarker":record.identity().context_marker},"diagnostics":diagnostics}).to_string())
    }
    pub fn release_native(&mut self, consumer: &str) -> Result<usize, String> {
        self.active()?;
        let _permit = admission(true, 0)?;
        cem_ml::runtime_inventory::note_mutation();
        let retained = self.retained.as_mut().unwrap();
        let before = self.leases.len();
        self.leases.retain(|(name, _), (native, _)| {
            if name == consumer {
                retained.release_stylesheet_consumer(native);
                false
            } else {
                true
            }
        });
        let released = before - self.leases.len();
        LEASES.fetch_sub(released, Ordering::SeqCst);
        Ok(released)
    }
    pub fn dispose_native(&mut self) {
        if self.retained.take().is_some() {
            cem_ml::runtime_inventory::note_mutation();
            LEASES.fetch_sub(self.leases.len(), Ordering::SeqCst);
            self.leases.clear();
        }
    }
    fn active(&self) -> Result<&RetainedTemplate, String> {
        self.retained
            .as_ref()
            .ok_or_else(|| "compiled declaration owner is disposed".into())
    }
}
impl Drop for CompiledDeclarationOwner {
    fn drop(&mut self) {
        self.dispose_native();
        OWNERS.fetch_sub(1, Ordering::SeqCst);
        cem_ml::runtime_inventory::note_mutation();
    }
}
pub fn inventory_native() -> String {
    serde_json::json!({"contract":"cem-component-owner-inventory/1","revision":cem_ml::runtime_inventory::inventory_revision(),"liveDeclarationOwners":OWNERS.load(Ordering::SeqCst),"stylesheetConsumerLeases":LEASES.load(Ordering::SeqCst)}).to_string()
}
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = "compiledDeclarationInventory")]
pub fn inventory_wasm() -> String {
    inventory_native()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
impl CompiledDeclarationOwner {
    #[wasm_bindgen(constructor)]
    pub fn new(
        bytes: &[u8],
        metadata: &str,
        pin: &str,
    ) -> Result<CompiledDeclarationOwner, wasm_bindgen::JsValue> {
        Self::admit(bytes, metadata, pin).map_err(|e| wasm_bindgen::JsValue::from_str(&e))
    }
    #[wasm_bindgen(js_name = "describe")]
    pub fn describe_wasm(&self) -> Result<String, wasm_bindgen::JsValue> {
        self.describe_native()
            .map_err(|e| wasm_bindgen::JsValue::from_str(&e))
    }
    #[wasm_bindgen(js_name = "renderJson")]
    pub fn render_wasm(
        &self,
        json: &[u8],
        uri: &str,
        binding: &str,
    ) -> Result<String, wasm_bindgen::JsValue> {
        self.render_native(json, uri, binding)
            .map_err(|e| wasm_bindgen::JsValue::from_str(&e))
    }
    #[wasm_bindgen(js_name = "stylesheet")]
    pub fn stylesheet_wasm(
        &mut self,
        consumer: &str,
        index: usize,
        context: &str,
    ) -> Result<String, wasm_bindgen::JsValue> {
        self.stylesheet_json(consumer, index, context)
            .map_err(|e| wasm_bindgen::JsValue::from_str(&e))
    }
    #[wasm_bindgen(js_name = "releaseConsumer")]
    pub fn release_wasm(&mut self, consumer: &str) -> Result<usize, wasm_bindgen::JsValue> {
        self.release_native(consumer)
            .map_err(|e| wasm_bindgen::JsValue::from_str(&e))
    }
    #[wasm_bindgen(js_name = "dispose")]
    pub fn dispose_wasm(&mut self) {
        self.dispose_native();
    }
}
