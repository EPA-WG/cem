//! Bounded native selection commands, not load or execution evidence. The browser
//! owns activation/load observation; recapture must retain this exact source set.
use crate::{
    css_import_snapshot::CssSourceSnapshot, html_css_import_snapshot::HtmlExecutionCssImports,
    module_resolution::CemModuleUrlResolutionCapability, parser::AstNodeId,
    stylesheet_execution::StylesheetOwner,
};
use std::sync::Arc;

pub struct TitledStylesheetAction {
    pub owner: AstNodeId,
    pub link: bool,
    pub selected: bool,
}
pub struct HtmlStylesheetSelection {
    css: Arc<HtmlExecutionCssImports>,
    actions: Vec<TitledStylesheetAction>,
}
impl HtmlStylesheetSelection {
    /// Empty title explicitly disables all titled roots. Persistent sheets are
    /// outside the command set; authored-disabled links are never activated.
    pub fn new(
        css: Arc<HtmlExecutionCssImports>,
        title: &str,
        budget: usize,
    ) -> Result<Self, String> {
        let roots = css.execution_roots();
        if title.len() > 4096 || roots.len() > budget || budget == 0 {
            return Err("bounded stylesheet selection required".into());
        }
        let mut actions = Vec::new();
        for root in roots.iter().filter(|r| !r.title.is_empty()) {
            let selected = !title.is_empty() && root.title == title && !root.disabled;
            let (owner, link) = match root.owner {
                StylesheetOwner::Link { owner } => (owner, true),
                StylesheetOwner::Document { owner, .. } => (owner, false),
                _ => return Err("HTML stylesheet owner required".into()),
            };
            if selected && !link {
                return Err(
                    "active inline titled stylesheet is outside the browser profile".into(),
                );
            }
            actions.push(TitledStylesheetAction {
                owner,
                link,
                selected,
            });
        }
        if !title.is_empty() && !actions.iter().any(|a| a.selected) {
            return Err("selected title has no enabled external stylesheet roots".into());
        }
        Ok(Self { css, actions })
    }
    pub fn actions(&self) -> &[TitledStylesheetAction] {
        &self.actions
    }

    /// Consumed once after the browser's controlled writes and load completion.
    /// A new same-owner HTML epoch is mandatory even for same-title transitions.
    pub fn validate_recapture(
        self,
        fresh: &HtmlExecutionCssImports,
        cap: &CemModuleUrlResolutionCapability,
        snapshot: &Arc<CssSourceSnapshot>,
    ) -> Result<(), String> {
        self.css.validate_sources(cap, snapshot)?;
        fresh.validate_sources(cap, snapshot)?;
        let old = self.css.document().base();
        let new = fresh.document().base();
        let a = old.observation();
        let b = new.observation();
        if !Arc::ptr_eq(&old.native_resolution().tree, &new.native_resolution().tree)
            || a.binding.document_id != b.binding.document_id
            || b.binding.epoch <= a.binding.epoch
            || a.policy_stamp != b.policy_stamp
            || old.effective_url() != new.effective_url()
            || old.native_resolution().fallback_url != new.native_resolution().fallback_url
            || old.policy_decision() != new.policy_decision()
            || old.profile() != new.profile()
        {
            return Err("stylesheet transition needs a fresh same-owner HTML capture with unchanged source/base/policy".into());
        }
        Ok(())
    }
}
