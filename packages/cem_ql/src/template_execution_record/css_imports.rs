//! Snapshot CSS closures bound to the roots and resolver contexts of a native execution.
use super::{ExecutedHtmlTemplatePreparation, SnapshotTemplateExecution};
use cem_ml::{
    css_import_snapshot::{
        capture_css_import_snapshot, CssSnapshotCaptureLimits, CssSourceSnapshot,
        SnapshotCssImportClosure,
    },
    engine::EngineContext,
    html_base_host::{HtmlBaseDomSlot, HtmlBaseExecutionHost},
    html_css_import_snapshot::HtmlExecutionCssImports,
    module_resolution::CemModuleUrlResolutionCapability,
    template_snapshot::TemplateSourceSnapshot,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct TemplateBodyCssImports {
    pub body_uri: String,
    /// Same occurrence order as the recorded body's compiled stylesheets.
    pub closures: Vec<SnapshotCssImportClosure>,
}

/// Completes snapshot imports for retained document and compiled body CSS.
/// It establishes neither stylesheet installation nor authority to rewrite sources.
pub struct ExecutedTemplateCssImports {
    preparation: Arc<ExecutedHtmlTemplatePreparation>,
    document: HtmlExecutionCssImports,
    snapshot: Arc<CssSourceSnapshot>,
    bodies: Vec<TemplateBodyCssImports>,
    blockers: Vec<String>,
    invalidated: AtomicBool,
}
impl ExecutedTemplateCssImports {
    pub fn preparation(&self) -> &ExecutedHtmlTemplatePreparation {
        &self.preparation
    }
    pub fn document(&self) -> &HtmlExecutionCssImports {
        &self.document
    }
    /// Same body order as the retained execution, including bodies with no CSS.
    pub fn bodies(&self) -> &[TemplateBodyCssImports] {
        &self.bodies
    }
    pub fn blockers(&self) -> &[String] {
        &self.blockers
    }

    fn validate_recorded_sources(&self) -> Result<(), String> {
        if self.invalidated.load(Ordering::SeqCst) {
            return Err("recorded template CSS preparation was invalidated".into());
        }
        let execution = &self.preparation.execution;
        let result = (|| {
            // Check every body context, even with no stylesheets, using the
            // internally retained owners. Caller substitution must not poison
            // otherwise valid evidence, but observed policy drift is permanent.
            execution.validate_current(
                execution.preflight.snapshot(),
                execution.preflight.context(),
                &execution.capability,
            )?;
            for (captured, body) in self.bodies.iter().zip(&execution.bodies.bodies) {
                let capability = &execution.body_capabilities[&body.uri];
                for (closure, sheet) in captured.closures.iter().zip(&body.stylesheets) {
                    closure.validate_current(
                        sheet.css_tree(),
                        &body.uri,
                        capability,
                        &self.snapshot,
                    )?;
                }
            }
            Ok(())
        })();
        if result.is_err() {
            self.invalidated.store(true, Ordering::SeqCst);
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    pub fn validate(
        &self,
        host: &mut HtmlBaseExecutionHost,
        execution: &Arc<SnapshotTemplateExecution>,
        template_snapshot: &Arc<TemplateSourceSnapshot>,
        context: &Arc<EngineContext>,
        capability: &CemModuleUrlResolutionCapability,
        css_snapshot: &Arc<CssSourceSnapshot>,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base_uri: &str,
    ) -> Result<(), String> {
        self.validate_recorded_sources()?;
        if !Arc::ptr_eq(css_snapshot, &self.snapshot) {
            return Err("recorded template CSS snapshot owner changed".into());
        }
        let preparation = self.preparation.validate(
            host,
            execution,
            template_snapshot,
            context,
            capability,
            &mut matches,
            base_uri,
        );
        // Always observe internal drift after callbacks, including on failure.
        self.validate_recorded_sources()?;
        preparation?;
        let document =
            host.validate_css_imports(&self.document, capability, css_snapshot, matches, base_uri);
        self.validate_recorded_sources()?;
        document
    }
}

impl ExecutedHtmlTemplatePreparation {
    /// Reuse this execution's native roots and body capabilities without resolving
    /// template sources again or reconstructing authority from public descriptors.
    /// Root nodes, sheet occurrences and response bytes share one aggregate budget.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_css_imports(
        self: &Arc<Self>,
        host: &mut HtmlBaseExecutionHost,
        execution: &Arc<SnapshotTemplateExecution>,
        template_snapshot: &Arc<TemplateSourceSnapshot>,
        context: &Arc<EngineContext>,
        capability: &CemModuleUrlResolutionCapability,
        css_snapshot: Arc<CssSourceSnapshot>,
        limits: CssSnapshotCaptureLimits,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base_uri: &str,
    ) -> Result<ExecutedTemplateCssImports, String> {
        self.validate(
            host,
            execution,
            template_snapshot,
            context,
            capability,
            &mut matches,
            base_uri,
        )?;
        let document = self.preparation.document_owner();
        let mut root_count = 0usize;
        let mut root_nodes = 0usize;
        let mut count_root = |nodes: usize| -> Result<(), String> {
            root_count = root_count.checked_add(1).ok_or("CSS root count overflow")?;
            root_nodes = root_nodes
                .checked_add(nodes)
                .ok_or("CSS root node count overflow")?;
            if root_count > limits.imports.max_sheets {
                return Err("recorded template CSS root/sheet limit exceeded".into());
            }
            if root_nodes > limits.max_root_nodes {
                return Err("recorded template CSS root node limit exceeded".into());
            }
            Ok(())
        };
        // Precheck all roots before invoking any new resolution callback.
        for sheet in document.stylesheets() {
            count_root(
                sheet
                    .resources
                    .as_ref()
                    .map_err(Clone::clone)?
                    .tree
                    .ast()
                    .nodes
                    .len(),
            )?;
        }
        for body in &self.execution.bodies.bodies {
            for sheet in &body.stylesheets {
                count_root(sheet.css_tree().ast().nodes.len())?;
            }
        }
        let abort = context.operation_control.abort_signal().clone();
        let document = host.inspect_css_imports(
            document.clone(),
            capability,
            css_snapshot.clone(),
            limits,
            abort.clone(),
            &mut matches,
            base_uri,
        )?;
        let mut remaining = limits;
        let mut consume = |closure: &SnapshotCssImportClosure| {
            remaining.imports.max_sheets -= closure.closure().sheets().len();
            remaining.responses.max_total_bytes -= closure.closure().received_bytes();
            remaining.max_root_nodes -= closure.closure().sheets()[0]
                .resources
                .tree
                .ast()
                .nodes
                .len();
        };
        for closure in document.closures() {
            consume(closure);
        }
        let mut bodies = Vec::new();
        for body in &self.execution.bodies.bodies {
            let mut closures = Vec::new();
            for sheet in &body.stylesheets {
                let captured = capture_css_import_snapshot(
                    sheet.css_tree().clone(),
                    &body.uri,
                    &self.execution.body_capabilities[&body.uri],
                    css_snapshot.clone(),
                    remaining,
                    abort.clone(),
                )
                .map_err(|error| error.to_string())?;
                remaining.imports.max_sheets -= captured.closure().sheets().len();
                remaining.responses.max_total_bytes -= captured.closure().received_bytes();
                remaining.max_root_nodes -= sheet.css_tree().ast().nodes.len();
                closures.push(captured);
            }
            bodies.push(TemplateBodyCssImports {
                body_uri: body.uri.clone(),
                closures,
            });
        }
        let mut blockers = self.blockers.clone();
        blockers.extend(document.blockers().iter().cloned());
        for body in &bodies {
            for closure in &body.closures {
                blockers.extend(closure.blockers().iter().cloned());
            }
        }
        blockers.retain(|b| {
            !matches!(
                b.as_str(),
                "stylesheet-owner-context-required"
                    | "template-stylesheet-import-closure-required"
                    | "css-import-and-runtime-closure-required"
            )
        });
        blockers.sort();
        blockers.dedup();
        let result = ExecutedTemplateCssImports {
            preparation: self.clone(),
            document,
            snapshot: css_snapshot,
            bodies,
            blockers,
            invalidated: AtomicBool::new(false),
        };
        result.validate(
            host,
            execution,
            template_snapshot,
            context,
            capability,
            &result.snapshot,
            matches,
            base_uri,
        )?;
        Ok(result)
    }
}
