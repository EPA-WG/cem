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
        let (_, requirements_hash) = self.pinned_json()?;
        let mut entries = BTreeSet::new();
        let mut execution_units = BTreeSet::from([NativeExecutionUnit::SelectedDispatch]);
        for declaration in self.declarations.values() {
            for function in &declaration.functions {
                if !matches!(function.dispatch.as_str(), "call" | "pipeline") {
                    return Err("unknown registry dispatch contract".into());
                }
                let selected =
                    SelectedBuiltin::from_key(&function.module, &function.name, function.arity)?;
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
                entries.insert(selected);
            }
            for operation in &declaration.pipeline_operations {
                match operation.kind.as_str() {
                    "field" if operation.explicit_arity == 0 => (),
                    "builtin" => {
                        entries.insert(SelectedBuiltin::from_key(
                            "cem:stdlib/sequence",
                            &operation.name,
                            operation.explicit_arity + 1,
                        )?);
                    }
                    _ => {
                        return Err(
                            "receiver/unknown pipeline operation needs an execution contract"
                                .into(),
                        )
                    }
                }
            }
        }
        for entry in &entries {
            execution_units.extend(entry.execution_units());
        }
        // An empty query registry still needs the evaluator/core value contracts.
        if entries.is_empty() {
            execution_units.extend(SelectedBuiltin::SequenceCount.execution_units());
            execution_units.remove(&NativeExecutionUnit::SequenceValues);
        }
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
impl SelectedRegistryPlan {
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
        let entries = self
            .entries
            .iter()
            .map(|entry| format!("        SelectedBuiltin::{},\n", entry.rust_variant()))
            .collect::<String>();
        format!("// Generated from {}; no runtime grants or link-closure claim.\nuse cem_ql::selected_registry::{{SelectedBuiltin, SelectedFunctionRegistry}};\npub fn selected_registry() -> SelectedFunctionRegistry {{\n    SelectedFunctionRegistry::new([\n{entries}    ]).expect(\"reviewed generated registry\")\n}}\n",self.requirements_hash)
    }
}
