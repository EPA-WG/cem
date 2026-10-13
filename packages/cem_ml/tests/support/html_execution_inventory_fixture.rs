//! Named test-control boundary: source bytes stay in native import; only actual
//! browser URL observations and scalar verification results cross this boundary.
use cem_ml::{html_base::*, html_base_evidence::*, html_base_host::*, module_resolution::*};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryObservation {
    pub document_url: String,
    pub base_uri: String,
    pub engine: String,
    pub version: String,
    pub image_url: String,
    pub navigation_url: String,
    pub stylesheet_url: String,
    pub inline_url: String,
}

pub fn verify(observations: &[InventoryObservation]) -> Vec<serde_json::Value> {
    assert_eq!(observations.len(), 1);
    observations.iter().map(|observed| {
        let source = include_str!("../fixtures/html-execution-inventory.xhtml").trim();
        let tree = cem_ml::import::import_data_bytes(source.as_bytes(), "application/xml", "cem", "studio://fixture/inventory.xhtml").unwrap();
        let mut host = HtmlBaseExecutionHost::new(HtmlBaseCompatibilityProfile::Chromium148,
            HtmlBaseBrowser { engine: observed.engine.clone(), version: observed.version.clone() },
            HtmlBaseLimits { nodes: 100, url_bytes: 4096 }).unwrap();
        // Native slot stand-ins in this process are fixture-only. The browser
        // separately validates the real DOM through the packaged WASM host.
        host.prepare(tree, HtmlBaseContext { fallback: HtmlFallbackBase::DocumentUrl(observed.document_url.clone()), encoding: HtmlBaseEncoding::Utf8, policy_stamp: "fixture:allow".into() }, |slot| Ok(slot.id + 1), |_| Ok(HtmlBasePolicyDecision::Allow)).unwrap();
        let evidence = host.confirm(observed.base_uri.clone()).unwrap();
        let context = CemResolutionContextHandle::new("fixture:browser-css");
        let cap = CemModuleUrlResolutionCapability::new(Arc::new(CemScopedModuleUrlResolver::new().with_context(context.clone(), CemModuleUrlContext {
            identity: "fixture:browser-css".into(), resolver_identity: "fixture:literal-urls".into(), resource_policy_stamp: "fixture:candidates-only".into(),
            frames: vec![CemModuleUrlFrame::new("fixture", &observed.document_url)],
        })), context);
        let inventory = host.inspect_dependencies(&evidence, &cap, 100, |slot| Ok(slot.id + 1), &observed.base_uri).unwrap();
        let references: Vec<_> = inventory.references().iter().map(|r| r.resolved_uri.as_ref().unwrap().as_str()).collect();
        let stylesheets: Vec<_> = inventory.stylesheets().iter().map(|s| s.resources.as_ref().unwrap().references[0].resolution.as_ref().unwrap().resolved_url.as_str()).collect();
        assert_eq!(references, [observed.image_url.as_str(), observed.navigation_url.as_str()]);
        assert_eq!(stylesheets, [observed.stylesheet_url.as_str(), observed.inline_url.as_str()]);
        host.validate_dependencies(&inventory, &cap, |slot| Ok(slot.id + 1), &observed.base_uri).unwrap();
        serde_json::json!({ "base_uri": observed.base_uri, "references": references, "stylesheets": stylesheets,
            "body_analysis_required": inventory.document().body_analysis_required, "blockers": inventory.blockers() })
    }).collect()
}
