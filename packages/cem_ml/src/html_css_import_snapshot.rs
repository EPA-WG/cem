//! Import closure for the exact retained inline CSS roots of a live HTML inventory.
use crate::{
    css_import_snapshot::{
        capture_css_import_snapshot, CssSnapshotCaptureLimits, CssSourceSnapshot,
        SnapshotCssImportClosure,
    },
    html_base_host::{HtmlBaseDomSlot, HtmlBaseExecutionHost},
    html_execution_inventory::HtmlExecutionInventory,
    module_resolution::CemModuleUrlResolutionCapability,
    scheduler::AbortSignal,
};
use std::sync::Arc;

pub struct HtmlExecutionCssImports {
    document: Arc<HtmlExecutionInventory>,
    snapshot: Arc<CssSourceSnapshot>,
    closures: Vec<SnapshotCssImportClosure>,
    abort: AbortSignal,
    blockers: Vec<String>,
}
impl HtmlExecutionCssImports {
    pub fn document(&self) -> &HtmlExecutionInventory {
        &self.document
    }
    /// Same occurrence order as document.stylesheets().
    pub fn closures(&self) -> &[SnapshotCssImportClosure] {
        &self.closures
    }
    pub fn blockers(&self) -> &[String] {
        &self.blockers
    }
    fn validate_sources(
        &self,
        capability: &CemModuleUrlResolutionCapability,
        snapshot: &Arc<CssSourceSnapshot>,
    ) -> Result<(), String> {
        if !Arc::ptr_eq(snapshot, &self.snapshot) || self.abort.is_aborted() {
            return Err("HTML CSS snapshot replaced or inspection cancelled".into());
        }
        for (captured, source) in self.closures.iter().zip(self.document.stylesheets()) {
            let root = source.resources.as_ref().map_err(Clone::clone)?;
            captured.validate_current(
                &root.tree,
                self.document.base().effective_url(),
                capability,
                snapshot,
            )?;
        }
        Ok(())
    }
}
impl HtmlBaseExecutionHost {
    /// Close imports only for the exact inline roots already retained by this
    /// execution inventory. This does not discover external <link> stylesheets,
    /// evaluate conditions, install CSS, or authorize any edit.
    #[allow(clippy::too_many_arguments)]
    pub fn inspect_css_imports(
        &mut self,
        document: Arc<HtmlExecutionInventory>,
        capability: &CemModuleUrlResolutionCapability,
        snapshot: Arc<CssSourceSnapshot>,
        limits: CssSnapshotCaptureLimits,
        abort: AbortSignal,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base_uri: &str,
    ) -> Result<HtmlExecutionCssImports, String> {
        self.validate_dependencies(&document, capability, &mut matches, base_uri)?;
        if abort.is_aborted() {
            return Err("HTML CSS snapshot inspection cancelled".into());
        }
        // Check every root before invoking any new resolution callback. Aggregate
        // limits count repeated sheets and bytes across all retained roots.
        if document.stylesheets().len() > limits.imports.max_sheets {
            return Err("HTML CSS snapshot root/sheet limit exceeded".into());
        }
        let mut nodes = 0usize;
        for source in document.stylesheets() {
            let root = source.resources.as_ref().map_err(Clone::clone)?;
            nodes = nodes
                .checked_add(root.tree.ast().nodes.len())
                .ok_or("HTML CSS root node count overflow")?;
            if nodes > limits.max_root_nodes {
                return Err("HTML CSS snapshot root node limit exceeded".into());
            }
        }
        let mut remaining = limits;
        let mut closures = Vec::new();
        for source in document.stylesheets() {
            let root = source.resources.as_ref().map_err(Clone::clone)?;
            let captured = capture_css_import_snapshot(
                root.tree.clone(),
                document.base().effective_url(),
                capability,
                snapshot.clone(),
                remaining,
                abort.clone(),
            )
            .map_err(|error| error.to_string())?;
            remaining.imports.max_sheets -= captured.closure().sheets().len();
            remaining.responses.max_total_bytes -= captured.closure().received_bytes();
            remaining.max_root_nodes -= root.tree.ast().nodes.len();
            closures.push(captured);
        }
        let mut blockers: Vec<String> = document
            .blockers()
            .iter()
            .filter(|b| b.as_str() != "css-import-and-runtime-closure-required")
            .cloned()
            .collect();
        blockers.extend(
            [
                "css-runtime-dependency-closure-required",
                "stylesheet-execution-history-required",
                "atomic-host-transaction-required",
            ]
            .map(str::to_owned),
        );
        for captured in &closures {
            blockers.extend(captured.blockers().iter().cloned());
        }
        blockers.sort();
        blockers.dedup();
        let result = HtmlExecutionCssImports {
            document,
            snapshot,
            closures,
            abort,
            blockers,
        };
        self.validate_css_imports(&result, capability, &result.snapshot, matches, base_uri)?;
        Ok(result)
    }

    pub fn validate_css_imports(
        &mut self,
        imports: &HtmlExecutionCssImports,
        capability: &CemModuleUrlResolutionCapability,
        snapshot: &Arc<CssSourceSnapshot>,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base_uri: &str,
    ) -> Result<(), String> {
        self.validate_dependencies(&imports.document, capability, &mut matches, base_uri)?;
        imports.validate_sources(capability, snapshot)?;
        self.validate_dependencies(&imports.document, capability, matches, base_uri)?;
        imports.validate_sources(capability, snapshot)
    }
}
