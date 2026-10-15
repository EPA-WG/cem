//! Authored SVG replay candidates. These plans preserve provenance but do not
//! establish cascade winners, authorize edits, or prove browser consumption.
pub mod admission;

use crate::{
    import::svg_presentation::RetainedSvgPresentationAttribute, parser::AstNodeId,
    scheduler::AbortSignal, svg_resource_acquisition::single_url,
};
use std::{collections::BTreeSet, sync::Arc};

pub struct SvgReplayRoute {
    pub consumer: RetainedSvgPresentationAttribute,
    /// Reference in consumer.css(), never in computed-observation CSS.
    pub reference: AstNodeId,
    pub source_url: String,
    pub request_url: String,
    /// URL-serialized fragment; target resolution remains a separate step.
    pub fragment: String,
    /// Fragment-free transport route, derived natively from the same alias.
    pub alias_request_url: String,
    /// Unique per authored occurrence, including its fragment.
    pub alias_url: String,
    /// Proposed whole decoded attribute value, not a source-file edit range.
    pub replacement: String,
}
pub struct SvgReplayPlan {
    routes: Vec<SvgReplayRoute>,
    cancel: AbortSignal,
    closed: bool,
    base: String,
    document: String,
}
impl SvgReplayPlan {
    pub fn new(
        sources: &[RetainedSvgPresentationAttribute],
        base: &str,
        document: &str,
        nonce: &str,
        cancel: AbortSignal,
    ) -> Result<Self, String> {
        if cancel.is_aborted() || sources.is_empty() || sources.len() > 256 {
            return Err("live bounded SVG replay sources required".into());
        }
        if nonce.len() != 32
            || !nonce
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err("bounded lowercase SVG replay nonce required".into());
        }
        let base = http_url(base)?;
        let mut document = http_url(document)?;
        document.set_fragment(None);
        let mut seen = BTreeSet::new();
        let mut routes = Vec::new();
        let mut bytes = 0usize;
        for source in sources {
            if cancel.is_aborted() {
                return Err("SVG replay cancelled".into());
            }
            if !Arc::ptr_eq(source.document(), sources[0].document())
                || !seen.insert(source.attribute())
            {
                return Err("unique SVG replay occurrences in one native document required".into());
            }
            let mut ancestor = Some(source.owner());
            let mut depth = 0;
            while let Some(id) = ancestor {
                depth += 1;
                if depth > 256 {
                    return Err("SVG replay ancestry bound exceeded".into());
                }
                let node = source
                    .document()
                    .node(id)
                    .ok_or("SVG replay ancestor missing")?;
                if node.attributes.iter().any(|id| {
                    source
                        .document()
                        .node(*id)
                        .and_then(|n| n.name.as_ref())
                        .is_some_and(|n| {
                            n.namespace_uri == "http://www.w3.org/XML/1998/namespace"
                                && n.local_name == "base"
                        })
                }) {
                    return Err("SVG replay XML base context unsupported".into());
                }
                ancestor = node.parent;
            }
            let property = &source
                .document()
                .node(source.attribute())
                .and_then(|n| n.name.as_ref())
                .ok_or("SVG replay attribute missing")?
                .local_name;
            if !matches!(property.as_str(), "fill" | "stroke") {
                return Err("SVG replay supports only fill/stroke presentation attributes".into());
            }
            let (reference, value) = single_url(source.css(), property)?;
            // A fragment-only value is a local reference even when a base points
            // elsewhere; it is outside this external-resource replay profile.
            if value.trim_matches(|c: char| c <= '\u{20}').starts_with('#') {
                return Err("external SVG replay source required".into());
            }
            let source_url = base
                .join(&value)
                .map_err(|_| "SVG replay source resolution failed")?;
            let source_url = http_url(source_url.as_str())?;
            let fragment = source_url
                .fragment()
                .filter(|f| !f.is_empty())
                .ok_or("nonempty SVG replay fragment required")?
                .to_owned();
            let mut request = source_url.clone();
            request.set_fragment(None);
            if source_url.origin() != document.origin()
                || request == document
                || request.path().starts_with("/__cem_svg_delivery/")
                || request.path().starts_with("/__cem_image_delivery/")
            {
                return Err("same-origin original external SVG replay source required".into());
            }
            let mut alias = document
                .join(&format!("/__cem_svg_delivery/{nonce}/{}", routes.len()))
                .map_err(|_| "SVG replay alias invalid")?;
            let alias_request_url = alias.to_string();
            alias.set_fragment(Some(&fragment));
            let alias_url = alias.to_string();
            // Escape CSS string delimiters after URL serialization. The URL's
            // fragment remains opaque and is never interpreted as CSS syntax.
            let replacement = format!(
                "url(\"{}\")",
                alias_url.replace('\\', "\\\\").replace('"', "\\\"")
            );
            if replacement.len() > 4096 {
                return Err("SVG replay replacement exceeds observation bound".into());
            }
            bytes = bytes
                .saturating_add(source.value().len())
                .saturating_add(source_url.as_str().len())
                .saturating_add(request.as_str().len())
                .saturating_add(alias_request_url.len())
                .saturating_add(alias_url.len())
                .saturating_add(replacement.len());
            if bytes > 4 * 1024 * 1024 {
                return Err("SVG replay byte bound exceeded".into());
            }
            routes.push(SvgReplayRoute {
                consumer: source.clone(),
                reference,
                source_url: source_url.to_string(),
                request_url: request.to_string(),
                fragment,
                alias_request_url,
                alias_url,
                replacement,
            });
        }
        if cancel.is_aborted() {
            return Err("SVG replay cancelled".into());
        }
        Ok(Self {
            routes,
            cancel,
            closed: false,
            base: base.to_string(),
            document: document.to_string(),
        })
    }
    /// Inspection only. No proposed replacement may be consumed without source,
    /// base/policy, winner and browser preparation validation by its owner.
    pub fn routes(&self) -> &[SvgReplayRoute] {
        if self.closed || self.cancel.is_aborted() {
            &[]
        } else {
            &self.routes
        }
    }
    pub fn base(&self) -> &str {
        &self.base
    }
    pub fn document(&self) -> &str {
        &self.document
    }
    pub fn validate(&mut self, sources: &[RetainedSvgPresentationAttribute]) -> Result<(), String> {
        if self.closed
            || self.cancel.is_aborted()
            || sources.len() != self.routes.len()
            || sources.iter().zip(&self.routes).any(|(a, route)| {
                let b = &route.consumer;
                !Arc::ptr_eq(a.document(), b.document())
                    || !Arc::ptr_eq(a.css(), b.css())
                    || a.owner() != b.owner()
                    || a.attribute() != b.attribute()
            })
        {
            self.close();
            return Err("SVG replay source owner or lifetime changed".into());
        }
        Ok(())
    }
    pub fn close(&mut self) {
        self.closed = true;
        self.routes.clear();
    }
    pub fn blockers(&self) -> &'static [&'static str] {
        &[
            "svg-replay-preparation-binding-required",
            "svg-replay-base-policy-validation-required",
            "svg-computed-winning-source-required",
            "svg-resource-browser-response-identity-required",
            "svg-resource-consumption-required",
            "svg-resource-fragment-target-required",
            "svg-resource-transitive-dependencies-required",
            "general-runtime-resource-coverage-required",
        ]
    }
}
fn http_url(value: &str) -> Result<url::Url, String> {
    if value.len() > 4096 {
        return Err("SVG replay URL byte bound exceeded".into());
    }
    let url = url::Url::parse(value).map_err(|_| "absolute SVG replay context required")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("credential-free HTTP(S) SVG replay URL required".into());
    }
    Ok(url)
}

/// Closed presentation-only author cascade. Browser/user origins and actual
/// alias consumption remain separate obligations, even when computed URLs agree.
pub struct SvgReplayAuthorPlan {
    identity: Arc<()>,
    plan: SvgReplayPlan,
    winners: Vec<crate::svg_paint_colors::PaintDeclarationWinner>,
}
impl SvgReplayAuthorPlan {
    pub fn new(plan: SvgReplayPlan) -> Result<Self, String> {
        use crate::parser::tree::CemTreeNodeKind;
        use crate::svg_paint_colors::{PaintDeclarationOrigin, PaintDeclarationWinner};
        let document = plan
            .routes()
            .first()
            .ok_or("live replay plan required")?
            .consumer
            .document();
        if document.ast().nodes.len() > 10_000 {
            return Err("replay author universe bound exceeded".into());
        }
        for id in 0..document.ast().nodes.len() {
            if plan.cancel.is_aborted() {
                return Err("SVG replay cancelled".into());
            }
            let Some(node) = document.node(id as AstNodeId) else {
                continue;
            };
            match node.kind {
                CemTreeNodeKind::Element => {
                    let name = node.name.as_ref().ok_or("unnamed replay element")?;
                    let allowed = match name.namespace_uri.as_str() {
                        "http://www.w3.org/1999/xhtml" => matches!(
                            name.local_name.as_str(),
                            "html"
                                | "head"
                                | "body"
                                | "base"
                                | "title"
                                | "div"
                                | "span"
                                | "p"
                                | "main"
                                | "section"
                        ),
                        "http://www.w3.org/2000/svg" => matches!(
                            name.local_name.as_str(),
                            "svg"
                                | "g"
                                | "path"
                                | "rect"
                                | "circle"
                                | "ellipse"
                                | "line"
                                | "polyline"
                                | "polygon"
                                | "defs"
                                | "title"
                                | "desc"
                                | "linearGradient"
                                | "radialGradient"
                                | "stop"
                                | "pattern"
                                | "image"
                        ),
                        _ => false,
                    };
                    if !allowed {
                        return Err("replay requires closed static author universe".into());
                    }
                }
                CemTreeNodeKind::Attribute => {
                    let name = node.name.as_ref().ok_or("unnamed replay attribute")?;
                    let local = name.local_name.to_ascii_lowercase();
                    if local == "style" || local == "is" || local.starts_with("on") {
                        return Err(
                            "replay competing or active author attribute unsupported".into()
                        );
                    }
                }
                CemTreeNodeKind::Reference | CemTreeNodeKind::ProcessingInstruction => {
                    return Err("replay active source unsupported".into())
                }
                _ => {}
            }
        }
        let winners = plan
            .routes()
            .iter()
            .map(|r| PaintDeclarationWinner {
                origin: PaintDeclarationOrigin::Presentation {
                    attribute: r.consumer.attribute(),
                },
                important: false,
                order: 0,
            })
            .collect();
        Ok(Self {
            identity: Arc::new(()),
            plan,
            winners,
        })
    }
    pub fn winners(&self) -> &[crate::svg_paint_colors::PaintDeclarationWinner] {
        if self.plan.routes().is_empty() {
            &[]
        } else {
            &self.winners
        }
    }
    pub fn validate(
        &mut self,
        evidence: &crate::svg_computed_execution::SvgComputedEvidence,
    ) -> Result<(), String> {
        let result = (|| {
            self.plan.validate(evidence.sources())?;
            if evidence.values().len() != self.plan.routes().len() {
                return Err("replay observation count changed".into());
            }
            for (slot, (route, value)) in
                self.plan.routes().iter().zip(evidence.values()).enumerate()
            {
                let property = &route
                    .consumer
                    .document()
                    .node(route.consumer.attribute())
                    .unwrap()
                    .name
                    .as_ref()
                    .unwrap()
                    .local_name;
                let (_, _, url) =
                    crate::svg_resource_acquisition::computed_url(property, value, slot)?;
                if url.as_str() != route.source_url {
                    return Err("replay computed URL disagrees with author winner".into());
                }
            }
            Ok(())
        })();
        if result.is_err() {
            self.plan.close();
            self.winners.clear();
        }
        result
    }
}
