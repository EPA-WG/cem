//! Source-publication-bound reduced rendering. Inputs remain native CEM values;
//! runtime state and declarations never gain authority from routing metadata.
use super::*;
use crate::{
    application_runtime as runtime,
    render::{TemplateArtifact, TemplateData},
};
use cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID;

/// Original registered input on one exact installed publication. Rust issues
/// this owner; serialized nodes, scopes and type labels cannot construct it.
pub struct ApplicationReceiverInput {
    pub(super) app: Weak<LinkedApplication>,
    pub(super) tag: String,
    pub(super) root: crate::eval::RetainedCemNode,
    _lease: NativeResourceLease,
    owners: Arc<AtomicUsize>,
}
impl Drop for ApplicationReceiverInput {
    fn drop(&mut self) {
        self.owners.fetch_sub(1, Ordering::AcqRel);
    }
}

pub(super) fn receiver_artifact<'a>(
    app: &'a LinkedApplication,
    tag: &str,
) -> Result<&'a TemplateArtifact, String> {
    let artifact = app.envelope().artifact(tag).ok_or(INVALID)?;
    let demand = app.envelope().requirements();
    let requirements = demand.declarations.get(tag).ok_or(INVALID)?;
    if !requirements.host_bindings.contains("instanceID")
        || requirements.capability.is_some()
        || !requirements.stylesheets.is_empty()
        || !requirements.unresolved.is_empty()
        || !requirements.data_imports.is_empty()
        || !demand.component_dependencies[tag].is_empty()
    {
        return Err(INVALID.into());
    }
    Ok(artifact)
}

impl ApplicationSourceSession {
    pub fn register_receiver_input(
        &mut self,
        tag: &str,
        root: crate::eval::RetainedCemNode,
    ) -> Result<ApplicationReceiverInput, String> {
        if tag.len() > 256 {
            return Err(INVALID.into());
        }
        let _entry = self.enter()?;
        self.with_current_application(|app, host| {
            app.receivers(host, tag)?
                .check_frame_input(host, &root)
                .map(|_| ())
        })?;
        let app = Arc::downgrade(self.prepared.current()?);
        let generation = self
            .endpoint
            .upgrade()
            .and_then(|e| e.generation.upgrade())
            .ok_or(INVALID)?;
        let lease = generation
            .resource(NativeRootKind::Registry)
            .map_err(|_| INVALID)?;
        self.receiver_inputs
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < 64).then_some(n + 1)
            })
            .map_err(|_| INVALID)?;
        #[cfg(feature = "application-source-runtime")]
        self.receiver_checkpoint_epoch.fetch_add(1, Ordering::AcqRel);
        Ok(ApplicationReceiverInput {
            app,
            tag: tag.into(),
            root,
            _lease: lease,
            owners: self.receiver_inputs.clone(),
        })
    }
    pub fn render_receiver_native(
        &mut self,
        input: &ApplicationReceiverInput,
        instance: &str,
    ) -> Result<String, String> {
        let _entry = self.enter()?;
        self.receiver_plan(input, instance)
            .and_then(|(plan, control)| {
                let html = crate::render::render_plan_to_html_with_control(
                    &plan,
                    &control,
                    ROOT_EXECUTION_SCOPE_ID,
                )
                .map_err(|_| INVALID)?
                .rendered;
                if html.len() > runtime::MAX_INPUT_BYTES {
                    return Err(INVALID.into());
                }
                self.check()?;
                Ok(html)
            })
    }
    pub fn project_receiver_native(
        &mut self,
        input: &ApplicationReceiverInput,
        instance: &str,
    ) -> Result<Vec<u8>, String> {
        let _entry = self.enter()?;
        let (plan, control) = self.receiver_plan(input, instance)?;
        let bytes = crate::eval::portable::encode_values_with_control(
            &crate::eval::output::output_nodes(plan.nodes),
            &runtime::inputs::LIMITS,
            crate::eval::QueryContextScope(0),
            &control,
            ROOT_EXECUTION_SCOPE_ID,
        )
        .map_err(|_| INVALID.to_string())?;
        self.check()?;
        Ok(bytes)
    }
    fn receiver_plan(
        &mut self,
        input: &ApplicationReceiverInput,
        instance: &str,
    ) -> Result<(crate::render::RenderPlan, OperationControl), String> {
        if instance.is_empty()
            || instance.len() > 1024
            || instance
                .chars()
                .any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(INVALID.into());
        }
        let original = self.prepared.current()?;
        if input
            .app
            .upgrade()
            .is_none_or(|a| !Arc::ptr_eq(&a, original))
        {
            return Err(INVALID.into());
        }
        let owner = self.endpoint.upgrade().ok_or(INVALID)?.control.clone();
        let control = OperationControl::with_root_policy(
            owner.abort_signal().clone(),
            cem_ml::scheduler::ScopePolicy::host_root().with_timeout_ms(Some(5000)),
        )
        .map_err(|_| INVALID)?;
        let plan = self.with_current_application(|app, host| {
            let artifact = receiver_artifact(app, &input.tag)?;
            control
                .check_scope(ROOT_EXECUTION_SCOPE_ID)
                .map_err(|_| INVALID)?;
            let frame =
                app.receivers(host, &input.tag)?
                    .consume_frame(host, &input.root, &control)?;
            control
                .check_scope(ROOT_EXECUTION_SCOPE_ID)
                .map_err(|_| INVALID)?;
            let mut data = TemplateData::default().with_binding(
                "instanceID",
                crate::eval::ItemStream::once(crate::eval::Item::Atomic(
                    crate::eval::AtomValue::String(instance.into()),
                )),
            );
            app.with_registry(host, |registry, _| {
                data.selected_registry = Some(Arc::new(registry.clone()));
                runtime::render_plan_original_data(artifact, data, &control, Some(&frame))
            })?
        })?;
        Ok((plan, control))
    }
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
        self.with_current_application(|app, host| {
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
