//! Bind exclusive stylesheet observations to the exact live HTML CSS preparation.
use crate::{
    css_import_snapshot::CssSourceSnapshot,
    html_base_host::{HtmlBaseDomSlot, HtmlBaseExecutionHost},
    html_css_import_snapshot::HtmlExecutionCssImports,
    html_stylesheet_links::attribute,
    module_resolution::CemModuleUrlResolutionCapability,
    stylesheet_execution::*,
};
use std::sync::Arc;
pub struct HtmlStylesheetExecution {
    css: Arc<HtmlExecutionCssImports>,
    execution: StylesheetExecutionEvidence,
    blockers: Vec<String>,
}
impl HtmlExecutionCssImports {
    pub fn execution_roots(&self) -> Vec<StylesheetExecutionRoot<'_>> {
        let tree = &self.document().document().tree;
        let mut roots: Vec<_> = self
            .closures()
            .iter()
            .zip(self.document().stylesheets())
            .map(|(closure, sheet)| {
                let get = |name| {
                    attribute(tree, sheet.owner, name)
                        .map(|(_, v)| v.to_owned())
                        .unwrap_or_default()
                };
                StylesheetExecutionRoot {
                    owner: StylesheetOwner::Document {
                        owner: sheet.owner,
                        source_node: sheet.source_node,
                    },
                    closure,
                    media: if sheet.owner == sheet.source_node {
                        get("media")
                    } else {
                        String::new()
                    },
                    title: get("title"),
                    disabled: false,
                    alternate: false,
                }
            })
            .collect();
        roots.extend(self.external().iter().map(|e| StylesheetExecutionRoot {
            owner: StylesheetOwner::Link {
                owner: e.link.owner,
            },
            closure: &e.closure,
            media: e.link.media.clone(),
            title: e.link.title.clone(),
            disabled: e.link.disabled,
            alternate: e.link.alternate,
        }));
        roots
    }
    #[allow(clippy::too_many_arguments)]
    pub fn record_execution(
        self: &Arc<Self>,
        html: &mut HtmlBaseExecutionHost,
        cap: &CemModuleUrlResolutionCapability,
        snapshot: &Arc<CssSourceSnapshot>,
        host: &mut StylesheetExecutionHost,
        observer: &mut impl StylesheetExecutionObserver,
        budget: usize,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base: &str,
    ) -> Result<HtmlStylesheetExecution, String> {
        html.validate_css_imports(self, cap, snapshot, &mut matches, base)?;
        let execution = host.capture(&self.execution_roots(), observer, budget)?;
        if let Err(error) = html.validate_css_imports(self, cap, snapshot, matches, base) {
            host.invalidate()?;
            return Err(error);
        }
        host.validate(&execution, &self.execution_roots(), observer)?;
        self.validate_sources(cap, snapshot)?;
        Ok(HtmlStylesheetExecution {
            css: self.clone(),
            execution,
            blockers: remaining_blockers(self.blockers()),
        })
    }
}
impl HtmlStylesheetExecution {
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
        cap: &CemModuleUrlResolutionCapability,
        snapshot: &Arc<CssSourceSnapshot>,
        host: &mut StylesheetExecutionHost,
        observer: &mut impl StylesheetExecutionObserver,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base: &str,
    ) -> Result<(), String> {
        html.validate_css_imports(&self.css, cap, snapshot, &mut matches, base)?;
        host.validate(&self.execution, &self.css.execution_roots(), observer)?;
        if let Err(error) = html.validate_css_imports(&self.css, cap, snapshot, matches, base) {
            host.invalidate()?;
            return Err(error);
        }
        // DOM callbacks may change stylesheet state without changing the DOM.
        host.validate(&self.execution, &self.css.execution_roots(), observer)?;
        self.css.validate_sources(cap, snapshot)
    }
}
