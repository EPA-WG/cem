//! A reviewed selected-registry recipe. It binds an inventory, never upgrades
//! unresolved contracts into permissions or claims native link closure.
use super::*;
use crate::selected_registry::{NativeExecutionUnit, SelectedBuiltin, SelectedFunctionRegistry};
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedRegistryPlan {
    contract: &'static str,
    requirements_hash: String,
    native_link_closure_verified: bool,
    entries: BTreeSet<SelectedBuiltin>,
    execution_units: BTreeSet<NativeExecutionUnit>,
    source_paths: BTreeSet<&'static str>,
}
impl ApplicationRequirements {
    pub fn selected_registry_plan(&self) -> Result<SelectedRegistryPlan, String> {
        self.require_static_inventory()?;
        self.plan_native_registry()
    }
    /// CEMAPPD4 has separately verified the closed set of host demands. This
    /// computes only native function selection and keeps the complete pin.
    pub(crate) fn plan_native_registry(&self) -> Result<SelectedRegistryPlan, String> {
        let (_, requirements_hash) = self.pinned_json()?;
        let mut entries = BTreeSet::new();
        for declaration in self.declarations.values() {
            for function in &declaration.functions {
                entries.insert(reviewed_function(function)?);
            }
            for operation in &declaration.pipeline_operations {
                entries.extend(reviewed_pipeline(operation)?);
            }
        }
        let execution_units = selected_execution_units(&entries);
        Ok(SelectedRegistryPlan {
            contract: "cem-selected-registry/1",
            requirements_hash,
            native_link_closure_verified: false,
            entries,
            source_paths: execution_units
                .iter()
                .flat_map(|unit| unit.source_paths().iter().copied())
                .collect(),
            execution_units,
        })
    }
}
pub(crate) fn selected_execution_units(
    entries: &BTreeSet<SelectedBuiltin>,
) -> BTreeSet<NativeExecutionUnit> {
    let mut units = BTreeSet::from([NativeExecutionUnit::SelectedDispatch]);
    if entries.is_empty() {
        // An empty registry still needs the evaluator/core value contracts.
        units.extend(SelectedBuiltin::SequenceCount.execution_units());
        units.remove(&NativeExecutionUnit::SequenceValues);
    } else {
        units.extend(entries.iter().flat_map(|entry| entry.execution_units()));
    }
    units
}

pub(crate) fn reviewed_function(function: &FunctionRequirement) -> Result<SelectedBuiltin, String> {
    if !matches!(function.dispatch.as_str(), "call" | "pipeline") {
        return Err("unknown registry dispatch contract".into());
    }
    let selected = SelectedBuiltin::from_key(&function.module, &function.name, function.arity)?;
    if function.implementation != selected.implementation() {
        return Err("selected registry catalogue kind mismatch".into());
    }
    if function.dispatch == "pipeline"
        && !matches!(
            selected,
            SelectedBuiltin::SequenceFirst
                | SelectedBuiltin::SequenceLast
                | SelectedBuiltin::SequenceTake
                | SelectedBuiltin::SequenceDrop
                | SelectedBuiltin::SequenceWhere
        )
    {
        return Err("pipeline needs a reviewed selected adapter".into());
    }
    Ok(selected)
}

pub(crate) fn reviewed_pipeline(
    operation: &PipelineOperation,
) -> Result<Option<SelectedBuiltin>, String> {
    match operation.kind.as_str() {
        "field" if operation.explicit_arity == 0 => Ok(None),
        "builtin" => SelectedBuiltin::from_key(
            "cem:stdlib/sequence",
            &operation.name,
            operation
                .explicit_arity
                .checked_add(1)
                .ok_or("pipeline arity limit")?,
        )
        .map(Some),
        _ => Err("receiver/unknown pipeline operation needs an execution contract".into()),
    }
}

impl SelectedRegistryPlan {
    pub fn execution_units(&self) -> &BTreeSet<NativeExecutionUnit> {
        &self.execution_units
    }
    pub fn registry(&self) -> SelectedFunctionRegistry {
        SelectedFunctionRegistry::new(self.entries.iter().copied())
            .expect("unique reviewed registry")
    }
    pub fn pinned_json(&self) -> Result<(Vec<u8>, String), String> {
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let hash = cem_ml::content_cache::ContentHash::from_blake3(&bytes).header_value();
        Ok((bytes, hash))
    }
    /// Only closed enum identifiers are emitted, never query text or arbitrary
    /// strings. Include this in a native prototype target with cem-ql available.
    pub fn rust_source(&self) -> String {
        registry_rust_source(&self.entries, &self.requirements_hash)
    }
}

pub(crate) fn registry_rust_source(
    entries: &BTreeSet<SelectedBuiltin>,
    inventory_hash: &str,
) -> String {
    let link_mask = entries
        .iter()
        .map(|entry| format!(" | (1 << SelectedBuiltin::{} as u32)", entry.rust_variant()))
        .collect::<String>();
    let entries = entries
        .iter()
        .map(|entry| format!("        SelectedBuiltin::{},\n", entry.rust_variant()))
        .collect::<String>();
    format!("// Generated from {inventory_hash}; no runtime grants or link-closure claim.\nuse cem_ql::selected_registry::{{SelectedBuiltin, SelectedFunctionRegistry}};\npub const SELECTED_BUILTIN_LINK_MASK: u32 = 0{link_mask};\npub fn selected_registry() -> SelectedFunctionRegistry {{\n    SelectedFunctionRegistry::new([\n{entries}    ]).expect(\"reviewed generated registry\")\n}}\n")
}
