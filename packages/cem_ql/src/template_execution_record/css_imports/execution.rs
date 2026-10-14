//! Stylesheet observation binding for a complete recorded template CSS preparation.
use super::*;
use cem_ml::stylesheet_execution::*;
pub struct ExecutedTemplateStylesheets {
    css: Arc<ExecutedTemplateCssImports>,
    execution: StylesheetExecutionEvidence,
    blockers: Vec<String>,
}
impl ExecutedTemplateCssImports {
    fn execution_roots(&self) -> Vec<StylesheetExecutionRoot<'_>> {
        let mut roots = self.document.execution_roots();
        for (captured, body) in self
            .bodies
            .iter()
            .zip(&self.preparation.execution.bodies.bodies)
        {
            for (index, closure) in captured.closures.iter().enumerate() {
                roots.push(StylesheetExecutionRoot {
                    owner: StylesheetOwner::Body {
                        uri: body.uri.clone(),
                        index,
                        scope: body.stylesheets[index].scope.clone(),
                    },
                    closure,
                    media: String::new(),
                    title: String::new(),
                    disabled: false,
                    alternate: false,
                });
            }
        }
        roots
    }
    #[allow(clippy::too_many_arguments)]
    pub fn record_execution(
        self: &Arc<Self>,
        html: &mut HtmlBaseExecutionHost,
        execution: &Arc<SnapshotTemplateExecution>,
        templates: &Arc<TemplateSourceSnapshot>,
        context: &Arc<EngineContext>,
        cap: &CemModuleUrlResolutionCapability,
        snapshot: &Arc<CssSourceSnapshot>,
        host: &mut StylesheetExecutionHost,
        observer: &mut impl StylesheetExecutionObserver,
        budget: usize,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base: &str,
    ) -> Result<ExecutedTemplateStylesheets, String> {
        self.validate(
            html,
            execution,
            templates,
            context,
            cap,
            snapshot,
            &mut matches,
            base,
        )?;
        let observed = host.capture(&self.execution_roots(), observer, budget)?;
        if let Err(error) = self.validate(
            html, execution, templates, context, cap, snapshot, matches, base,
        ) {
            host.invalidate()?;
            return Err(error);
        }
        // The source/DOM checks themselves may trigger host callbacks.
        host.validate(&observed, &self.execution_roots(), observer)?;
        self.validate_recorded_sources()?;
        Ok(ExecutedTemplateStylesheets {
            css: self.clone(),
            execution: observed,
            blockers: remaining_blockers(self.blockers()),
        })
    }
}
impl ExecutedTemplateStylesheets {
    pub fn execution(&self) -> &StylesheetExecutionEvidence {
        &self.execution
    }
    pub fn blockers(&self) -> &[String] {
        &self.blockers
    }
    #[allow(clippy::too_many_arguments)]
    pub fn validate(
        &self,
        html: &mut HtmlBaseExecutionHost,
        execution: &Arc<SnapshotTemplateExecution>,
        templates: &Arc<TemplateSourceSnapshot>,
        context: &Arc<EngineContext>,
        cap: &CemModuleUrlResolutionCapability,
        snapshot: &Arc<CssSourceSnapshot>,
        host: &mut StylesheetExecutionHost,
        observer: &mut impl StylesheetExecutionObserver,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base: &str,
    ) -> Result<(), String> {
        self.css.validate(
            html,
            execution,
            templates,
            context,
            cap,
            snapshot,
            &mut matches,
            base,
        )?;
        host.validate(&self.execution, &self.css.execution_roots(), observer)?;
        if let Err(error) = self.css.validate(
            html, execution, templates, context, cap, snapshot, matches, base,
        ) {
            host.invalidate()?;
            return Err(error);
        }
        host.validate(&self.execution, &self.css.execution_roots(), observer)?;
        self.css.validate_recorded_sources()
    }
}
