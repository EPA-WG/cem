//! One build-owned compiled entry, registry and GraphQL contract per heap.
use super::{
    render_control, render_data, CompiledTemplateArtifact, PinnedTemplateArtifactContext,
    MAX_INPUT_BYTES,
};
use crate::{
    eval::{imported_cem_tree, ItemStream},
    render::{TemplateArtifact, TemplateData},
    selected_registry::SelectedFunctionRegistry,
};
use cem_ml::{
    content_cache::ContentHash,
    operation_control::ROOT_EXECUTION_SCOPE_ID,
    parser::tree::RetainedCemTree,
    validation::json::graphql::{GraphqlContracts, OperationKind},
};
use std::cell::{Cell, RefCell};
use std::sync::Arc;

mod response;

#[derive(Clone, Copy)]
pub(super) struct SelectedApplicationAssets<'a> {
    pub entry: &'a [u8],
    pub entry_pins: &'a str,
    pub guard: &'a [u8],
    pub guard_hash: &'a str,
    pub operations: &'a str,
}

pub(super) struct SelectedApplication<'a> {
    template: TemplateArtifact,
    binding: String,
    registry: Arc<SelectedFunctionRegistry>,
    graphql: GraphqlContracts,
    operations: &'a str,
    deployment: String,
    response: RefCell<Option<response::ResponseOwner>>,
    retired: Cell<bool>,
}

impl<'a> SelectedApplication<'a> {
    pub(super) fn admit(
        assets: SelectedApplicationAssets<'a>,
        registry: SelectedFunctionRegistry,
    ) -> Result<Self, String> {
        if assets.entry.len() > MAX_INPUT_BYTES
            || assets.entry_pins.len() > 16 * 1024
            || assets.operations.len() > 256 * 1024
            || !(assets.entry.starts_with(b"CEMTPLA2\n") || assets.entry.starts_with(b"CEMTPLA3\n"))
        {
            return Err("cem.application.entry.refused".into());
        }
        let pins: PinnedTemplateArtifactContext =
            serde_json::from_str(assets.entry_pins).map_err(|_| "cem.application.entry.pins")?;
        let [binding] = pins.host_bindings.as_slice() else {
            return Err("cem.application.entry.binding".into());
        };
        if binding.is_empty() || binding.len() > 256 {
            return Err("cem.application.entry.binding".into());
        }
        let graphql = GraphqlContracts::admit(assets.guard, assets.guard_hash)?;
        if ContentHash::from_blake3(assets.operations.as_bytes()).header_value()
            != graphql.document_hash()
        {
            return Err("cem.application.graphql.document".into());
        }
        let template = CompiledTemplateArtifact::from_bytes(assets.entry.to_vec())
            .map_err(|error| error.code.to_owned())?
            .reload_pinned(&pins)
            .map_err(|error| error.code.to_owned())?;
        Ok(Self {
            template,
            binding: binding.clone(),
            registry: Arc::new(registry),
            graphql,
            operations: assets.operations,
            deployment: ContentHash::from_blake3(
                &serde_json::to_vec(&(assets.entry_pins, assets.guard_hash))
                    .map_err(|_| "cem.application.entry.pins")?,
            )
            .header_value(),
            response: RefCell::new(None),
            retired: Cell::new(false),
        })
    }

    pub(super) fn operations(&self) -> &str {
        self.operations
    }

    pub(super) fn validate_variables(
        &self,
        operation: &str,
        bytes: &[u8],
        uri: &str,
        query_only: bool,
    ) -> Result<(), String> {
        self.active()?;
        check_input(operation, bytes, uri)?;
        if query_only {
            let kind = self
                .graphql
                .operations()
                .get(operation)
                .map(|entry| entry.kind);
            if !matches!(kind, Some(OperationKind::Query)) {
                return Err("cem.graphql.operation.kind".into());
            }
        }
        self.graphql
            .validate_variables(operation, bytes, uri, &render_control()?)
    }

    pub(super) fn render_response(
        &self,
        operation: &str,
        bytes: &[u8],
        uri: &str,
    ) -> Result<String, String> {
        self.active()?;
        check_input(operation, bytes, uri)?;
        let control = render_control()?;
        let response = self
            .graphql
            .admit_response(operation, bytes, uri, &control)?;
        let tree = response
            .into_retained_tree()
            .map_err(|_| "cem.application.graphql.projection")?;
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|error| error.to_string())?;
        self.render_tree(tree, &control)
    }

    fn render_tree(
        &self,
        tree: Arc<RetainedCemTree>,
        control: &cem_ml::operation_control::OperationControl,
    ) -> Result<String, String> {
        let mut data = TemplateData::default()
            .with_binding(&self.binding, ItemStream::once(imported_cem_tree(tree)));
        data.selected_registry = Some(self.registry.clone());
        render_data(&self.template, data, control)
    }
}

fn check_input(operation: &str, bytes: &[u8], uri: &str) -> Result<(), String> {
    if operation.len() > 256 || bytes.len() > MAX_INPUT_BYTES || uri.len() > 4096 {
        return Err("cem.application.input.limit".into());
    }
    if !url::Url::parse(uri).is_ok_and(|uri| !uri.cannot_be_a_base()) {
        return Err("cem.application.input.source_uri".into());
    }
    Ok(())
}

#[cfg(cem_selected_application)]
mod linked {
    use super::*;
    use crate::application_runtime::generated;
    use std::cell::OnceCell;

    thread_local! {
        static APP: OnceCell<Result<SelectedApplication<'static>, String>> = const { OnceCell::new() };
    }

    fn with_application<T>(
        operation: &str,
        bytes: &[u8],
        uri: &str,
        run: impl FnOnce(&SelectedApplication<'static>) -> Result<T, String>,
    ) -> Result<T, String> {
        if operation.len() > 256 || bytes.len() > MAX_INPUT_BYTES || uri.len() > 4096 {
            return Err("cem.application.input.limit".into());
        }
        with_state(false, bytes.len() + operation.len() + uri.len(), run)
    }

    fn with_state<T>(
        cleanup: bool,
        input_bytes: usize,
        run: impl FnOnce(&SelectedApplication<'static>) -> Result<T, String>,
    ) -> Result<T, String> {
        let _native = cem_ml::runtime_admission::admit(cleanup).map_err(str::to_owned)?;
        if input_bytes > 5 * cem_ml::committed_checkpoint::MAX_CHECKPOINT_BYTES + 32 * 1024 {
            return Err("cem.application.input.limit".into());
        }
        APP.with(|cell| {
            #[cfg(target_arch = "wasm32")]
            let _memory = {
                let cold = if cell.get().is_none() {
                    generated::ENTRY.len() + generated::GUARD.len() + generated::OPERATIONS.len()
                } else {
                    0
                };
                cem_ml::runtime_memory::admit_input(cold + input_bytes)
                    .map_err(|error| error.to_string())?
            };
            let app = cell
                .get_or_init(|| {
                    SelectedApplication::admit(
                        SelectedApplicationAssets {
                            entry: generated::ENTRY,
                            entry_pins: generated::ENTRY_PINS,
                            guard: generated::GUARD,
                            guard_hash: generated::GUARD_HASH,
                            operations: generated::OPERATIONS,
                        },
                        generated::selected_registry(),
                    )
                })
                .as_ref()
                .map_err(Clone::clone)?;
            run(app)
        })
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(js_name = selectedApplicationDocument))]
    pub fn selected_application_document() -> Result<String, String> {
        with_application("", &[], "", |app| {
            app.active()?;
            Ok(app.operations().to_owned())
        })
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(js_name = validateSelectedApplicationVariables))]
    pub fn validate_selected_application_variables(
        operation: &str,
        bytes: &[u8],
        uri: &str,
    ) -> Result<(), String> {
        with_application(operation, bytes, uri, |app| {
            app.validate_variables(operation, bytes, uri, false)
        })
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(js_name = validateSelectedApplicationQueryVariables))]
    pub fn validate_selected_application_query_variables(
        operation: &str,
        bytes: &[u8],
        uri: &str,
    ) -> Result<(), String> {
        with_application(operation, bytes, uri, |app| {
            app.validate_variables(operation, bytes, uri, true)
        })
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(js_name = renderSelectedApplicationResponse))]
    pub fn render_selected_application_response(
        operation: &str,
        bytes: &[u8],
        uri: &str,
    ) -> Result<String, String> {
        with_application(operation, bytes, uri, |app| {
            app.render_response(operation, bytes, uri)
        })
    }

    pub(crate) fn response_inventory_native() -> (usize, usize) {
        APP.with(|cell| {
            cell.get()
                .and_then(|app| app.as_ref().ok())
                .map_or((0, 0), SelectedApplication::response_inventory)
        })
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(js_name = selectedApplicationIdentity))]
    pub fn selected_application_identity() -> Result<String, String> {
        with_state(false, 0, |app| {
            app.active()?;
            Ok(app.deployment().into())
        })
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(js_name = retainSelectedApplicationResponse))]
    pub fn retain_selected_application_response(
        operation: &str,
        bytes: &[u8],
        uri: &str,
    ) -> Result<String, String> {
        with_application(operation, bytes, uri, |app| {
            app.retain_response(operation, bytes, uri)
        })
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(js_name = claimSelectedApplicationMutation))]
    pub fn claim_selected_application_mutation(operation_id: &str) -> Result<f64, String> {
        if operation_id.len() > 1024 {
            return Err("cem.application.input.limit".into());
        }
        with_state(false, operation_id.len(), |app| {
            app.claim_mutation(operation_id)
                .map(|sequence| sequence as f64)
        })
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(js_name = captureSelectedApplicationResponse))]
    pub fn capture_selected_application_response() -> Result<Vec<u8>, String> {
        with_state(false, 0, SelectedApplication::capture_response)
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(js_name = selectedApplicationResponseRevision))]
    pub fn selected_application_response_revision() -> Result<String, String> {
        with_state(false, 0, SelectedApplication::response_revision)
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(js_name = sealSelectedApplicationCheckpoint))]
    pub fn seal_selected_application_checkpoint(
        candidate: &str,
        receipt: &str,
    ) -> Result<String, String> {
        if candidate.len() > cem_ml::committed_checkpoint::MAX_WORKSPACE_CHECKPOINT_BYTES
            || receipt.len() > MAX_INPUT_BYTES
        {
            return Err("cem.application.input.limit".into());
        }
        with_state(false, candidate.len() + receipt.len(), |app| {
            app.active()?;
            cem_ml::committed_checkpoint::seal(candidate, receipt).map_err(str::to_owned)
        })
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(js_name = confirmSelectedApplicationResponseRetirement))]
    pub fn confirm_selected_application_response_retirement(
        capsule: &[u8],
        journal: &[u8],
        runtime_id: &str,
        instance_id: &str,
    ) -> Result<String, String> {
        if capsule.len() > MAX_INPUT_BYTES
            || journal.len() > cem_ml::committed_checkpoint::MAX_WORKSPACE_CHECKPOINT_BYTES
            || runtime_id.len() > 1024
            || instance_id.len() > 1024
        {
            return Err("cem.application.input.limit".into());
        }
        with_state(
            false,
            capsule.len() + journal.len() + runtime_id.len() + instance_id.len(),
            |app| app.confirm_response_retirement(capsule, journal, runtime_id, instance_id),
        )
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(js_name = restoreSelectedApplicationResponse))]
    pub fn restore_selected_application_response(
        capsule: &[u8],
        pin: &str,
        journal: &[u8],
        previous_runtime_id: &str,
    ) -> Result<String, String> {
        if capsule.len() > MAX_INPUT_BYTES
            || pin.len() > 16 * 1024
            || journal.len() > cem_ml::committed_checkpoint::MAX_WORKSPACE_CHECKPOINT_BYTES
            || previous_runtime_id.len() > 1024
        {
            return Err("cem.application.input.limit".into());
        }
        with_state(
            false,
            capsule.len() + pin.len() + journal.len() + previous_runtime_id.len(),
            |app| app.restore_response(capsule, pin, journal, previous_runtime_id),
        )
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(js_name = disposeSelectedApplicationResponse))]
    pub fn dispose_selected_application_response() -> Result<(), String> {
        with_state(true, 0, |app| {
            app.dispose_response();
            Ok(())
        })
    }
}

#[cfg(cem_selected_application)]
pub use linked::*;
