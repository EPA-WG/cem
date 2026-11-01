//! Source-publication-bound reduced rendering. Inputs remain native CEM values;
//! runtime state and declarations never gain authority from routing metadata.
use super::*;
use crate::{
    application_runtime as runtime,
    render::{TemplateArtifact, TemplateData},
};
use cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID;

impl ApplicationSourceSession {
    fn frame<T>(
        &mut self,
        tag: &str,
        instance: &str,
        bind: impl FnOnce(&OperationControl) -> Result<TemplateData, String>,
        render: impl FnOnce(&TemplateArtifact, TemplateData, &OperationControl) -> Result<T, String>,
    ) -> Result<T, String> {
        if tag.len() > 256
            || instance.is_empty()
            || instance.len() > 1024
            || instance
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
        {
            return Err("compiled application frame identity refused".into());
        }
        let owner_control = self.endpoint.upgrade().ok_or(INVALID)?.control.clone();
        let control = OperationControl::with_root_policy(
            owner_control.abort_signal().clone(),
            cem_ml::scheduler::ScopePolicy::host_root().with_timeout_ms(Some(5000)),
        )
        .map_err(|e| e.to_string())?;
        self.with_application(|app, host| {
            let declaration = app.envelope().declaration(tag).ok_or(INVALID)?;
            let artifact = app.envelope().artifact(tag).ok_or(INVALID)?;
            let requirements = app.envelope().requirements();
            let demand = requirements.declarations.get(tag).ok_or(INVALID)?;
            if declaration.capability.is_some()
                || !artifact.stylesheets.is_empty()
                || !demand.unresolved.is_empty()
                || !demand.data_imports.is_empty()
                || requirements
                    .component_dependencies
                    .get(tag)
                    .is_none_or(|v| !v.is_empty())
                || !declaration
                    .template
                    .host_bindings
                    .iter()
                    .any(|s| s == "instanceID")
            {
                return Err("compiled application declaration needs additional ownership".into());
            }
            let (fields, types) = app.receivers(host, tag)?.runtime_scalar_contracts(host)?;
            let mut data = bind(&control)?;
            data.input_attribute_contracts = fields;
            data.value_types = types;
            data.bindings.insert(
                "instanceID".into(),
                crate::eval::ItemStream::once(crate::eval::Item::Atomic(
                    crate::eval::AtomValue::String(instance.into()),
                )),
            );
            app.with_registry(host, |registry, _| {
                data.selected_registry = Some(Arc::new(registry.clone()));
                control
                    .check_scope(ROOT_EXECUTION_SCOPE_ID)
                    .map_err(|_| INVALID)?;
                render(artifact, data, &control)
            })?
        })
    }
    pub fn render_island_native(
        &mut self,
        tag: &str,
        bytes: &[u8],
        instance: &str,
    ) -> Result<String, String> {
        self.frame(
            tag,
            instance,
            |c| runtime::island::bind(bytes, instance, c),
            runtime::render_data,
        )
    }
    pub fn project_island_native(
        &mut self,
        tag: &str,
        bytes: &[u8],
        instance: &str,
    ) -> Result<Vec<u8>, String> {
        self.frame(
            tag,
            instance,
            |c| runtime::island::bind(bytes, instance, c),
            runtime::project_data,
        )
    }
    pub fn render_inputs_native(
        &mut self,
        tag: &str,
        bytes: &[u8],
        routing: &str,
        instance: &str,
    ) -> Result<String, String> {
        self.frame(
            tag,
            instance,
            |c| runtime::inputs::bind_original(bytes, routing, c),
            runtime::render_data,
        )
    }
}
