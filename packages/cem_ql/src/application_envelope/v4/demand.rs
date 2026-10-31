//! Recompute static UI and portable native-query demand from original checked IR.
use super::*;
use crate::{
    application_requirements::declaration_requirements,
    selected_registry::{NativeExecutionUnit, SelectedBuiltin, SelectedFunctionRegistry},
};
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryDemand {
    contract: &'static str,
    requirements_hash: String,
    template_plan_hash: String,
    query_functions: BTreeSet<SelectedBuiltin>,
    entries: BTreeSet<SelectedBuiltin>,
    execution_units: BTreeSet<NativeExecutionUnit>,
}
impl RegistryDemand {
    pub fn registry(&self) -> SelectedFunctionRegistry {
        SelectedFunctionRegistry::new(self.entries.iter().copied()).expect("checked demand")
    }
    pub fn execution_units(&self) -> &BTreeSet<NativeExecutionUnit> {
        &self.execution_units
    }
    pub fn query_functions(&self) -> &BTreeSet<SelectedBuiltin> {
        &self.query_functions
    }
    pub fn pinned_json(&self) -> Result<(Vec<u8>, String), String> {
        let bytes = serde_json::to_vec_pretty(self).map_err(|_| INVALID)?;
        if bytes.len() > 1024 * 1024 {
            return Err(INVALID.into());
        }
        let pin = hash(&bytes);
        Ok((bytes, pin))
    }
}
pub(super) fn compose(
    parent: &Arc<v3::VerifiedApplicationEnvelope>,
    mixed: &BTreeMap<String, Arc<mixed::VerifiedMixedReceiverDeclaration>>,
    roots: BTreeSet<String>,
    work: &Work<'_>,
) -> Result<(ApplicationRequirements, RegistryDemand, Vec<String>), String> {
    if roots.len() > 64 || parent.application().component_count() + mixed.len() > 64 {
        return Err(INVALID.into());
    }
    let mut requirements = ApplicationRequirements::default();
    requirements.roots = roots;
    let base = parent.application();
    for (tag, component) in &base.components {
        work.check()?;
        requirements.declarations.insert(
            tag.clone(),
            declaration_requirements(
                &component.artifact,
                &component.metadata,
                &base.manifest.components[tag].metadata,
            )?,
        );
        requirements.component_dependencies.insert(
            tag.clone(),
            base.manifest.components[tag].dependencies.clone(),
        );
    }
    let mut sources = super::claims::Claims::new(work);
    sources.parent(parent)?;
    let mut occurrences = 0usize;
    for (tag, value) in mixed {
        work.check()?;
        if !Arc::ptr_eq(value.application(), parent) || requirements.declarations.contains_key(tag)
        {
            return Err(INVALID.into());
        }
        occurrences = occurrences
            .checked_add(value.occurrence_count())
            .ok_or(INVALID)?;
        if occurrences > 256 {
            return Err(INVALID.into());
        }
        for occurrence in value.occurrences().values() {
            if let Some(mixed::ReceiverContract::Scalar(s)) =
                value.receiver(&occurrence.fields["type"])
            {
                work.check()?;
                sources.remember(&s.source_uri, &s.source_hash)?;
            }
        }
        requirements.declarations.insert(
            tag.clone(),
            declaration_requirements(
                value.artifact(),
                value.declaration(),
                &value.requirements().metadata_hash,
            )?,
        );
        requirements
            .component_dependencies
            .insert(tag.clone(), value.component_dependencies().clone());
    }
    for declaration in requirements.declarations.values() {
        sources.remember(
            &declaration.source_uri,
            &declaration.declaration_source_hash,
        )?;
    }
    let order = super::super::graph::order(
        requirements.component_dependencies.clone(),
        requirements.roots.clone(),
        work,
    )?;
    let plan = requirements.selected_registry_plan()?;
    let mut entries = plan.registry().entries().collect::<BTreeSet<_>>();
    let mut query_functions = BTreeSet::new();
    for value in base.contracts.values() {
        work.check()?;
        if let super::super::VerifiedContract::Query(q) = value {
            query_functions.extend(q.program.required_functions()?);
        }
    }
    for id in parent.assembly_ids() {
        let g = parent.assembly(id).ok_or(INVALID)?;
        for id in g.leaf_ids() {
            work.check()?;
            if let Some(crate::datatype_dependency_capsule::VerifiedArtifact::Query(q)) = g.leaf(id)
            {
                query_functions.extend(q.program.required_functions()?);
            }
        }
    }
    entries.extend(query_functions.iter().copied());
    let mut execution_units = plan.execution_units().clone();
    for e in &entries {
        execution_units.extend(e.execution_units());
    }
    let demand = RegistryDemand {
        contract: "cem-application-registry-demand/1",
        requirements_hash: requirements.pinned_json()?.1,
        template_plan_hash: plan.pinned_json()?.1,
        query_functions,
        entries,
        execution_units,
    };
    Ok((requirements, demand, order))
}
