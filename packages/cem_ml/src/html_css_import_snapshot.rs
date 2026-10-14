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
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct HtmlExecutionCssImports {
    document: Arc<HtmlExecutionInventory>,
    snapshot: Arc<CssSourceSnapshot>,
    closures: Vec<SnapshotCssImportClosure>,
    external: Vec<crate::html_stylesheet_links::HtmlLinkCssImports>,
    abort: AbortSignal,
    blockers: Vec<String>,
    invalidated: AtomicBool,
}
impl HtmlExecutionCssImports {
    pub fn external(&self) -> &[crate::html_stylesheet_links::HtmlLinkCssImports] {
        &self.external
    }
    pub fn all_closures(&self) -> impl Iterator<Item = &SnapshotCssImportClosure> {
        self.closures
            .iter()
            .chain(self.external.iter().map(|e| &e.closure))
    }
    pub fn sheet_count(&self) -> usize {
        self.all_closures()
            .map(|c| c.closure().sheets().len())
            .sum()
    }
    pub fn received_bytes(&self) -> usize {
        self.all_closures()
            .map(|c| c.closure().received_bytes())
            .sum::<usize>()
            + self
                .external
                .iter()
                .map(|e| e.source.bytes.len())
                .sum::<usize>()
    }
    pub fn root_nodes(&self) -> usize {
        self.all_closures()
            .map(|c| c.closure().sheets()[0].resources.tree.ast().nodes.len())
            .sum()
    }
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
    /// Revalidate only retained source owners/policy, not live DOM or edit authority.
    pub fn validate_sources(
        &self,
        capability: &CemModuleUrlResolutionCapability,
        snapshot: &Arc<CssSourceSnapshot>,
    ) -> Result<(), String> {
        if self.invalidated.load(Ordering::SeqCst) {
            return Err("HTML CSS capture was invalidated".into());
        }
        // Observe the internally retained policy even for an empty forest and
        // before caller substitution checks. A-B-A cannot revive observed drift.
        if let Err(error) = self.document.validate_recorded_policy() {
            self.invalidated.store(true, Ordering::SeqCst);
            return Err(error);
        }
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
        for external in &self.external {
            external.closure.validate_current(
                &external.closure.closure().sheets()[0].resources.tree,
                &external.source.final_url,
                capability,
                snapshot,
            )?;
        }
        Ok(())
    }
}
impl HtmlBaseExecutionHost {
    /// Close imports only for the exact inline roots already retained by this
    /// execution inventory plus its external stylesheet links. This does not
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
        if document
            .stylesheets()
            .len()
            .saturating_add(document.stylesheet_links().len())
            > limits.imports.max_sheets
        {
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
        let mut external = Vec::new();
        for link in document.stylesheet_links() {
            let captured = crate::html_stylesheet_links::capture(
                link,
                document.base().effective_url(),
                capability,
                snapshot.clone(),
                remaining,
                abort.clone(),
            )?;
            remaining.imports.max_sheets -= captured.closure.closure().sheets().len();
            remaining.responses.max_total_bytes -=
                captured.source.bytes.len() + captured.closure.closure().received_bytes();
            remaining.max_root_nodes -= captured.closure.closure().sheets()[0]
                .resources
                .tree
                .ast()
                .nodes
                .len();
            external.push(captured);
        }
        let mut blockers: Vec<String> = document
            .blockers()
            .iter()
            .filter(|b| {
                !matches!(
                    b.as_str(),
                    "css-import-and-runtime-closure-required"
                        | "external-stylesheet-closure-required"
                )
            })
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
        for captured in closures.iter().chain(external.iter().map(|e| &e.closure)) {
            blockers.extend(captured.blockers().iter().cloned());
        }
        blockers.sort();
        blockers.dedup();
        let result = HtmlExecutionCssImports {
            document,
            snapshot,
            closures,
            external,
            abort,
            blockers,
            invalidated: AtomicBool::new(false),
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
        imports.validate_sources(capability, snapshot)?;
        self.validate_dependencies(&imports.document, capability, &mut matches, base_uri)?;
        imports.validate_sources(capability, snapshot)?;
        self.validate_dependencies(&imports.document, capability, matches, base_uri)?;
        imports.validate_sources(capability, snapshot)
    }
}
