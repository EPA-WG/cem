//! Bounded reference/CSS candidates bound to a current native execution host.
//! These plans describe current inputs, never historical resource loads or edits.
use crate::{
    document_dependencies::{
        DocumentDependencyInspection, DocumentReference, DocumentReferenceRole,
    },
    document_stylesheet_dependencies::{
        inspect_html_execution_stylesheets, DocumentStylesheetInventory,
        DocumentStylesheetOccurrence,
    },
    html_base_evidence::CapturedHtmlBaseEvidence,
    html_base_host::{HtmlBaseDomSlot, HtmlBaseExecutionHost},
    module_resolution::CemModuleUrlResolutionCapability,
};
use std::sync::Arc;

pub struct HtmlReferenceProvenance {
    pub reference: DocumentReference,
    pub resolved_uri: Result<String, String>,
    pub source_uri: Option<String>,
    pub source_key: Option<String>,
}

/// Private construction ties every node ID, CSS tree and resolver result to the
/// original confirmed base and native capability owner. Retained data remains
/// inspectable after invalidation; consuming it requires validate_dependencies.
pub struct HtmlExecutionInventory {
    base: Arc<CapturedHtmlBaseEvidence>,
    capability: CemModuleUrlResolutionCapability,
    resolver_identity: String,
    stylesheets: DocumentStylesheetInventory,
    references: Vec<HtmlReferenceProvenance>,
    blockers: Vec<String>,
}

impl HtmlExecutionInventory {
    pub fn base(&self) -> &CapturedHtmlBaseEvidence {
        &self.base
    }
    pub fn document(&self) -> &DocumentDependencyInspection {
        &self.stylesheets.document
    }
    pub fn references(&self) -> &[HtmlReferenceProvenance] {
        &self.references
    }
    pub fn stylesheets(&self) -> &[DocumentStylesheetOccurrence] {
        &self.stylesheets.stylesheets
    }
    pub fn blockers(&self) -> &[String] {
        &self.blockers
    }
}

impl HtmlBaseExecutionHost {
    /// Revalidate before analysis and after resolver callbacks. HTML/SVG URI
    /// candidates use the confirmed browser base; only CSS uses the execution
    /// module capability. Neither source URIs nor imported XML bases substitute.
    pub fn inspect_dependencies(
        &mut self,
        evidence: &Arc<CapturedHtmlBaseEvidence>,
        capability: &CemModuleUrlResolutionCapability,
        budget: usize,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base_uri: &str,
    ) -> Result<HtmlExecutionInventory, String> {
        self.validate(evidence, &mut matches, base_uri)?;
        let resolver_identity = resolver_identity(capability)?;
        let stylesheets = inspect_html_execution_stylesheets(evidence, capability, budget);
        if stylesheets
            .blockers
            .iter()
            .any(|b| b == "complete-document-base-inventory-required")
        {
            return Err("complete HTML execution inventory required within budget".into());
        }
        let base = url::Url::parse(evidence.effective_url())
            .map_err(|_| "invalid confirmed HTML base URL")?;
        let tree = &stylesheets.document.tree;
        let references: Vec<_> = stylesheets
            .document
            .references
            .iter()
            .filter(|reference| reference.role != DocumentReferenceRole::Base)
            .map(|reference| HtmlReferenceProvenance {
                reference: reference.clone(),
                // The URL parser owns whitespace handling. Unicode trim would
                // change decoded authored URIs and their source correspondence.
                resolved_uri: base
                    .join(&reference.authored_uri)
                    .map(|u| u.to_string())
                    .map_err(|e| format!("unresolved HTML reference: {e}")),
                source_uri: tree.node_source_uri(reference.attribute).map(str::to_owned),
                source_key: tree.source_key(reference.attribute),
            })
            .collect();
        let mut blockers: Vec<String> = stylesheets
            .document
            .blockers
            .iter()
            .filter(|b| {
                b.as_str() != "document-base-context-required"
                    && b.as_str() != "reference-base-and-policy-required"
            })
            .chain(stylesheets.blockers.iter())
            .cloned()
            .collect();
        blockers.extend(evidence.blockers().iter().map(|b| (*b).into()));
        blockers.extend(
            [
                "document-reference-policy-required",
                "general-document-reference-coverage-required",
                "stylesheet-execution-history-required",
            ]
            .map(str::to_owned),
        );
        if references.iter().any(|r| r.resolved_uri.is_err()) {
            blockers.push("document-reference-resolution-incomplete".into());
        }
        if stylesheets.stylesheets.iter().any(|s| {
            s.resources
                .as_ref()
                .map_or(true, |p| p.references.iter().any(|r| r.resolution.is_err()))
        }) {
            blockers.push("inline-css-resolution-incomplete".into());
        }
        blockers.sort();
        blockers.dedup();
        let result = HtmlExecutionInventory {
            base: evidence.clone(),
            capability: capability.clone(),
            resolver_identity,
            stylesheets,
            references,
            blockers,
        };
        self.validate_dependencies(&result, capability, matches, base_uri)?;
        Ok(result)
    }

    /// A URL string or equal resolver labels cannot restore stale authority.
    /// Host context writers must invalidate before replacing resolver policy too.
    pub fn validate_dependencies(
        &mut self,
        inventory: &HtmlExecutionInventory,
        capability: &CemModuleUrlResolutionCapability,
        mut matches: impl FnMut(HtmlBaseDomSlot<'_>) -> Result<u32, String>,
        base_uri: &str,
    ) -> Result<(), String> {
        self.validate(&inventory.base, &mut matches, base_uri)?;
        if !inventory.capability.same_owner(capability)
            || resolver_identity(capability)? != inventory.resolver_identity
        {
            return Err("HTML execution inventory resolver owner or policy changed".into());
        }
        self.validate(&inventory.base, matches, base_uri)
    }
}

fn resolver_identity(capability: &CemModuleUrlResolutionCapability) -> Result<String, String> {
    capability
        .cache_identity()
        .filter(|identity| !identity.trim().is_empty())
        .ok_or_else(|| "HTML execution inventory requires a complete resolver fingerprint".into())
}
