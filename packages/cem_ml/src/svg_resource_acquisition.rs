//! Bounded joins of observed CSS URL references to separately acquired SVG bytes.
//! A snapshot is not a receipt for the browser's original response or paint use.
use crate::{
    content_cache::ContentHash,
    css_resources::{attribute, named, resource_reference, CssResourceKind},
    parser::{
        tree::{CemTreeNodeKind, RetainedCemTree},
        AstNodeId,
    },
    scheduler::AbortSignal,
    svg_computed_execution::{SvgComputedEvidence, SvgComputedHost, SvgComputedObserver},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
pub mod nested;
pub mod paint;

const MAX_BYTES: usize = 4 * 1024 * 1024;
/// Trusted acquisition input. This API performs no fetch and grants no URL access.
pub struct SvgResourceResponse {
    pub requested_url: String,
    pub final_url: String,
    pub revision: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}
/// Raw entity bytes from an adapter withholding delivery until acceptance.
/// No caller-provided digest, revision or transferable receipt is trusted.
pub struct SvgDeliveryResponse {
    pub requested_url: String,
    pub final_url: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}
/// Execution-owned ingress. An adapter must establish request/delivery identity;
/// this native ledger alone does not attest to browser requests or painting.
pub struct SvgDeliveryCapture {
    responses: BTreeMap<String, SvgDeliveryResponse>,
    cancel: AbortSignal,
    bytes: usize,
    closed: bool,
}
impl SvgDeliveryCapture {
    pub fn new(cancel: AbortSignal) -> Self {
        Self {
            responses: BTreeMap::new(),
            cancel,
            bytes: 0,
            closed: false,
        }
    }
    pub fn accept(&mut self, response: SvgDeliveryResponse) -> Result<(), String> {
        let result = (|| {
            active(&self.cancel)?;
            if self.closed {
                return Err("closed SVG delivery ingress".into());
            }
            let requested = absolute(&response.requested_url)?;
            let final_url = absolute(&response.final_url)?;
            let size = response
                .bytes
                .len()
                .saturating_add(response.requested_url.len())
                .saturating_add(response.final_url.len())
                .saturating_add(response.content_type.len());
            if requested.fragment().is_some()
                || requested != final_url
                || response.requested_url != requested.as_str()
                || response.final_url != final_url.as_str()
                || !response.content_type.eq_ignore_ascii_case("image/svg+xml")
                || response.bytes.is_empty()
                || self.responses.len() >= 256
                || self.bytes.saturating_add(size) > MAX_BYTES
            {
                return Err("bounded canonical direct SVG delivery required".into());
            }
            // Even identical repeated receipts are ambiguous until an adapter
            // can bind an individual request to the browser's selected response.
            if self.responses.contains_key(&response.requested_url) {
                return Err("duplicate SVG delivery request".into());
            }
            self.bytes += size;
            self.responses
                .insert(response.requested_url.clone(), response);
            active(&self.cancel)
        })();
        if result.is_err() {
            self.close();
        }
        result
    }
    pub fn close(&mut self) {
        self.closed = true;
        self.responses.clear();
        self.bytes = 0;
    }
}
pub struct SvgAcquiredBody {
    response: SvgResourceResponse,
    tree: Arc<RetainedCemTree>,
    digest: String,
}
impl SvgAcquiredBody {
    pub fn bytes(&self) -> &[u8] {
        &self.response.bytes
    }
    pub fn tree(&self) -> &Arc<RetainedCemTree> {
        &self.tree
    }
    pub fn url(&self) -> &str {
        &self.response.final_url
    }
    pub fn revision(&self) -> &str {
        &self.response.revision
    }
    pub fn content_type(&self) -> &str {
        &self.response.content_type
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
}
pub struct SvgResourceBinding {
    /// Index in the retained computed evidence; never an authored winner claim.
    pub slot: usize,
    pub css: Arc<RetainedCemTree>,
    pub reference: AstNodeId,
    pub url: String,
    /// URL-serialized fragment, not a resolved SVG element or decoded ID.
    pub fragment: Option<String>,
    pub body: Arc<SvgAcquiredBody>,
}
/// A same-body template edge. All coordinates belong to the binding's acquired
/// tree; authored href spelling and source ranges remain on the attribute node.
pub struct SvgPaintTemplateLink {
    pub owner: AstNodeId,
    pub attribute: AstNodeId,
    pub target: AstNodeId,
}
pub struct SvgResourceRecord {
    computed: Arc<SvgComputedEvidence>,
    bindings: Vec<SvgResourceBinding>,
    cancel: AbortSignal,
    revoked: bool,
    fragment_targets: Vec<AstNodeId>,
    template_links: Option<Vec<Vec<SvgPaintTemplateLink>>>,
    content: Option<Vec<paint::AcquiredPaintContent>>,
    nested: Option<nested::NestedSnapshots>,
    content_context: Option<(
        crate::module_resolution::CemModuleUrlResolutionCapability,
        String,
    )>,
    delivery: Option<AbortSignal>,
}
impl SvgResourceRecord {
    /// Inspection only; callers must validate this record and their enclosing
    /// DOM/base/style/policy preparation before consumption. The acquisition
    /// owner must abort on snapshot-lifetime changes, even at unchanged URLs.
    pub fn bindings(&self) -> &[SvgResourceBinding] {
        &self.bindings
    }
    pub fn computed(&self) -> &Arc<SvgComputedEvidence> {
        &self.computed
    }
    pub fn blockers(&self) -> &'static [&'static str] {
        &[
            "svg-resource-preparation-binding-required",
            "svg-resource-browser-response-identity-required",
            "svg-resource-consumption-required",
            "svg-resource-fragment-target-required",
            "svg-resource-transitive-dependencies-required",
            "svg-computed-winning-source-required",
            "general-runtime-resource-coverage-required",
        ]
    }
    /// Inspection of a target in the exact retained acquired body. This does
    /// not authorize replay. Revalidate the record and enclosing preparation
    /// before use; cancellation immediately hides these results.
    pub fn fragment_target(&self, index: usize) -> Option<(&SvgResourceBinding, AstNodeId)> {
        if self.revoked
            || self.cancel.is_aborted()
            || self.delivery.as_ref().is_some_and(AbortSignal::is_aborted)
            || self.nested.as_ref().is_some_and(|n| n.cancel.is_aborted())
            || self
                .content_context
                .as_ref()
                .is_some_and(|(cap, identity)| cap.cache_identity().as_ref() != Some(identity))
        {
            return None;
        }
        Some((
            self.bindings.get(index)?,
            *self.fragment_targets.get(index)?,
        ))
    }

    /// Inspection only, under the same lifetime as fragment_target. Empty links
    /// mean a validated leaf; None means no live traversal result for this slot.
    pub fn paint_template_links(
        &self,
        index: usize,
    ) -> Option<(&SvgResourceBinding, &[SvgPaintTemplateLink])> {
        let (binding, _) = self.fragment_target(index)?;
        Some((
            binding,
            self.template_links.as_ref()?.get(index)?.as_slice(),
        ))
    }

    /// Validate targets and follow at most 64 same-body template edges per slot.
    /// This records candidate dependencies, not inherited content/styles or paint.
    pub fn validate_paint_templates(
        &mut self,
        host: &mut SvgComputedHost,
        observer: &mut impl SvgComputedObserver,
        budget: usize,
    ) -> Result<(), String> {
        self.validate_targets(host, observer, budget, true)
    }

    /// Candidate content/style/dependency provenance under an explicit pinned
    /// compatibility profile. Includes bounded author-color derivation, never
    /// live cascade proof or automatic nested resource acquisition.
    pub fn prepare_paint_content(
        &mut self,
        host: &mut SvgComputedHost,
        observer: &mut impl SvgComputedObserver,
        profile: crate::svg_paint_clone_profile::SvgPaintCloneProfile,
        capability: &crate::module_resolution::CemModuleUrlResolutionCapability,
        budget: usize,
    ) -> Result<(), String> {
        let result = (|| {
            // Bound traversal and content work separately within the total cap.
            let identity = capability
                .cache_identity()
                .ok_or("stable acquired content resolution context required")?;
            let budget = budget.min(1_000_000);
            self.validate_paint_templates(host, observer, budget / 2)?;
            let mut work = paint::Work(budget - budget / 2);
            let mut inventories = BTreeMap::new();
            let mut content = Vec::new();
            for (index, binding) in self.bindings.iter().enumerate() {
                active(&self.cancel)?;
                if !inventories.contains_key(binding.body.url()) {
                    inventories.insert(
                        binding.body.url().to_owned(),
                        paint::inventory(&binding.body, capability, &mut work, &self.cancel)?,
                    );
                }
                content.push(paint::build(
                    binding,
                    self.fragment_targets[index],
                    &self
                        .template_links
                        .as_ref()
                        .ok_or("template links missing")?[index],
                    inventories[binding.body.url()].clone(),
                    profile,
                    capability,
                    &mut work,
                    &self.cancel,
                )?);
            }
            self.validate(host, observer)?;
            if capability.cache_identity().as_ref() != Some(&identity) {
                return Err("acquired content policy changed".into());
            }
            self.content_context = Some((capability.clone(), identity));
            self.content = Some(content);
            Ok(())
        })();
        if result.is_err() {
            self.close();
        }
        result
    }
    /// Borrowed inspection only, governed by the original acquisition lifetime.
    pub fn paint_content(
        &self,
        index: usize,
    ) -> Option<(&SvgResourceBinding, &paint::AcquiredPaintContent)> {
        let (binding, _) = self.fragment_target(index)?;
        Some((binding, self.content.as_ref()?.get(index)?))
    }

    /// Bind explicitly supplied nested bodies to resolved source references.
    /// Recurses through bounded CSS/SVG snapshots, never fetches or authorizes replay.
    pub fn bind_nested_snapshots(
        &mut self,
        host: &mut SvgComputedHost,
        observer: &mut impl SvgComputedObserver,
        capability: &crate::module_resolution::CemModuleUrlResolutionCapability,
        responses: Vec<SvgResourceResponse>,
        cancel: AbortSignal,
    ) -> Result<(), String> {
        let result = (|| {
            self.validate(host, observer)?;
            if self.nested.is_some() {
                return Err("fresh nested snapshot binding required".into());
            }
            let (owner, _) = self
                .content_context
                .as_ref()
                .ok_or("acquired content context required")?;
            if !owner.same_owner(capability) {
                return Err("nested snapshot resolution owner mismatch".into());
            }
            let nested = nested::NestedSnapshots::capture(self, capability, responses, cancel)?;
            self.validate(host, observer)?;
            active(&nested.cancel)?;
            self.nested = Some(nested);
            Ok(())
        })();
        if result.is_err() {
            self.close();
        }
        result
    }
    pub fn nested_snapshots(&self) -> Option<&nested::NestedSnapshots> {
        self.fragment_target(0)?;
        self.nested.as_ref()
    }

    /// Opt-in, all-or-nothing literal-ID paint target validation. The bounded
    /// profile deliberately shares local paint traversal's ASCII fragment rule.
    /// No percent decoding, view fragments, href traversal or style evaluation.
    pub fn validate_fragments(
        &mut self,
        host: &mut SvgComputedHost,
        observer: &mut impl SvgComputedObserver,
        budget: usize,
    ) -> Result<(), String> {
        self.validate_targets(host, observer, budget, false)
    }
    fn validate_targets(
        &mut self,
        host: &mut SvgComputedHost,
        observer: &mut impl SvgComputedObserver,
        budget: usize,
        templates: bool,
    ) -> Result<(), String> {
        let result = (|| {
            self.validate(host, observer)?;
            let mut remaining = budget.min(1_000_000);
            let mut spend = |amount: usize| -> Result<(), String> {
                remaining = remaining
                    .checked_sub(amount)
                    .ok_or("SVG fragment work budget exceeded")?;
                Ok(())
            };
            let mut indexes = BTreeMap::new();
            let mut targets = Vec::new();
            let mut template_links = Vec::new();
            for binding in &self.bindings {
                active(&self.cancel)?;
                spend(1)?;
                let fragment = binding
                    .fragment
                    .as_deref()
                    .filter(|id| crate::svg_paint_dependencies::literal_fragment(id))
                    .ok_or("unsupported SVG paint fragment")?;
                spend(fragment.len())?;
                if !indexes.contains_key(binding.body.url()) {
                    let tree = binding.body.tree();
                    let mut ids: BTreeMap<String, Vec<AstNodeId>> = BTreeMap::new();
                    let mut pending = vec![0];
                    while let Some(id) = pending.pop() {
                        active(&self.cancel)?;
                        let node = tree.node(id).ok_or("invalid SVG fragment node")?;
                        spend(1 + node.value.len() + node.children.len() + node.attributes.len())?;
                        for attr in &node.attributes {
                            let attr = tree.node(*attr).ok_or("invalid SVG fragment attribute")?;
                            spend(1 + attr.value.len())?;
                            if node.kind == CemTreeNodeKind::Element
                                && attr.name.as_ref().is_some_and(|n| {
                                    n.namespace_uri.is_empty() && n.local_name == "id"
                                })
                            {
                                ids.entry(attr.value.clone()).or_default().push(id);
                            }
                        }
                        pending.extend(node.children.iter().copied());
                    }
                    indexes.insert(binding.body.url().to_owned(), ids);
                }
                let ids = &indexes[binding.body.url()];
                let matches = ids.get(fragment).ok_or("missing SVG fragment target")?;
                if matches.len() != 1 {
                    return Err("ambiguous SVG fragment target".into());
                }
                let target = matches[0];
                if !binding
                    .body
                    .tree()
                    .node(target)
                    .and_then(|n| n.name.as_ref())
                    .is_some_and(|n| {
                        n.namespace_uri == "http://www.w3.org/2000/svg"
                            && matches!(
                                n.local_name.as_str(),
                                "linearGradient" | "radialGradient" | "pattern"
                            )
                    })
                {
                    return Err("SVG fragment target is not a paint server".into());
                }
                if templates {
                    template_links.push(template_chain(
                        binding.body.tree(),
                        target,
                        ids,
                        &self.cancel,
                        &mut spend,
                    )?);
                }
                targets.push(target);
            }
            self.validate(host, observer)?;
            self.content = None;
            self.nested = None;
            self.content_context = None;
            self.fragment_targets = targets;
            self.template_links = templates.then_some(template_links);
            Ok(())
        })();
        if result.is_err() {
            self.close();
        }
        result
    }
    pub fn capture(
        computed: Arc<SvgComputedEvidence>,
        host: &mut SvgComputedHost,
        observer: &mut impl SvgComputedObserver,
        responses: Vec<SvgResourceResponse>,
        cancel: AbortSignal,
    ) -> Result<Self, String> {
        active(&cancel)?;
        host.validate(&computed, computed.sources(), observer)?;
        if responses.len() > 256 {
            return Err("SVG response count bound exceeded".into());
        }
        let mut total = 0usize;
        let mut bodies = BTreeMap::new();
        for mut response in responses {
            active(&cancel)?;
            for size in [
                response.bytes.len(),
                response.requested_url.len(),
                response.final_url.len(),
                response.content_type.len(),
                response.revision.len(),
            ] {
                total = total
                    .checked_add(size)
                    .ok_or("SVG response byte overflow")?;
                if total > MAX_BYTES {
                    return Err("SVG response byte bound exceeded".into());
                }
            }
            let requested = absolute(&response.requested_url)?;
            let final_url = absolute(&response.final_url)?;
            if requested.fragment().is_some()
                || final_url.fragment().is_some()
                || requested != final_url
            {
                return Err("fragment-free nonredirected SVG response required".into());
            }
            if response.revision.trim().is_empty() || response.revision.len() > 4096 {
                return Err("bounded SVG response revision required".into());
            }
            if !response.content_type.eq_ignore_ascii_case("image/svg+xml") {
                return Err("image/svg+xml response required".into());
            }
            response.requested_url = requested.to_string();
            response.final_url = final_url.to_string();
            if bodies.contains_key(&response.requested_url) {
                return Err("duplicate SVG response URL".into());
            }
            let tree = crate::import::import_data_bytes(
                &response.bytes,
                "image/svg+xml",
                "cem",
                &response.final_url,
            )?;
            if tree
                .ast_owner()
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation())
            {
                return Err("SVG response import diagnostics".into());
            }
            let roots: Vec<_> = tree
                .node(0)
                .ok_or("SVG document missing")?
                .children
                .iter()
                .filter_map(|id| tree.node(*id))
                .filter(|n| n.kind == CemTreeNodeKind::Element)
                .collect();
            if roots.len() != 1
                || !roots[0].name.as_ref().is_some_and(|n| {
                    n.namespace_uri == "http://www.w3.org/2000/svg" && n.local_name == "svg"
                })
            {
                return Err("SVG namespace document root required".into());
            }
            let digest = ContentHash::from_blake3(&response.bytes).header_value();
            bodies.insert(
                response.requested_url.clone(),
                Arc::new(SvgAcquiredBody {
                    response,
                    tree,
                    digest,
                }),
            );
        }
        let mut bindings = Vec::new();
        let mut used = BTreeSet::new();
        for (slot, (source, value)) in computed.sources().iter().zip(computed.values()).enumerate()
        {
            active(&cancel)?;
            let property = &source
                .document()
                .node(source.attribute())
                .and_then(|n| n.name.as_ref())
                .ok_or("SVG property missing")?
                .local_name;
            if !matches!(property.as_str(), "fill" | "stroke") {
                return Err("SVG resource profile requires fill or stroke".into());
            }
            let (css, reference, url) = computed_url(property, value, slot)?;
            let fragment = url.fragment().map(str::to_owned);
            let mut request = url.clone();
            request.set_fragment(None);
            let key = request.to_string();
            let body = bodies
                .get(&key)
                .ok_or("computed SVG URL has no acquired body")?
                .clone();
            used.insert(key);
            bindings.push(SvgResourceBinding {
                slot,
                css,
                reference,
                url: url.to_string(),
                fragment,
                body,
            });
        }
        if bindings.is_empty() {
            return Err("SVG resource binding requires observed URL slots".into());
        }
        if used.len() != bodies.len() {
            return Err("unused SVG response in bounded acquisition".into());
        }
        host.validate(&computed, computed.sources(), observer)?;
        active(&cancel)?;
        Ok(Self {
            computed,
            bindings,
            cancel,
            revoked: false,
            fragment_targets: Vec::new(),
            template_links: None,
            content: None,
            nested: None,
            content_context: None,
            delivery: None,
        })
    }
    /// Consume one ledger, joining exact entity bytes to every retained body.
    /// Browser adapter/response identity and consumption remain explicit blockers.
    pub fn bind_delivery(
        &mut self,
        mut delivery: SvgDeliveryCapture,
        host: &mut SvgComputedHost,
        observer: &mut impl SvgComputedObserver,
    ) -> Result<(), String> {
        let result = (|| {
            self.validate(host, observer)?;
            active(&delivery.cancel)?;
            if delivery.closed || self.delivery.is_some() {
                return Err("fresh SVG delivery evidence required".into());
            }
            let mut used = BTreeSet::new();
            for binding in &self.bindings {
                let body = &binding.body;
                let response = delivery
                    .responses
                    .get(body.url())
                    .ok_or("missing SVG delivery body")?;
                if response.bytes != body.bytes()
                    || !response
                        .content_type
                        .eq_ignore_ascii_case(body.content_type())
                {
                    return Err("SVG delivered bytes differ from acquired snapshot".into());
                }
                used.insert(body.url());
            }
            if used.len() != delivery.responses.len() {
                return Err("unused SVG delivery body".into());
            }
            self.validate(host, observer)?;
            active(&delivery.cancel)?;
            self.delivery = Some(delivery.cancel.clone());
            Ok(())
        })();
        delivery.close();
        if result.is_err() {
            self.close();
        }
        result
    }
    /// Diagnostic only, never a transferable response or consumption seal.
    pub fn delivered_bindings(&self) -> usize {
        if self.revoked
            || self.cancel.is_aborted()
            || !self
                .delivery
                .as_ref()
                .is_some_and(|signal| !signal.is_aborted())
        {
            return 0;
        }
        self.bindings.len()
    }
    pub fn validate(
        &mut self,
        host: &mut SvgComputedHost,
        observer: &mut impl SvgComputedObserver,
    ) -> Result<(), String> {
        let result = (|| {
            if self.revoked {
                return Err("revoked SVG resource record".into());
            }
            if self
                .content_context
                .as_ref()
                .is_some_and(|(cap, identity)| cap.cache_identity().as_ref() != Some(identity))
            {
                return Err("acquired content resolution context changed".into());
            }
            active(&self.cancel)?;
            if let Some(nested) = &self.nested {
                active(&nested.cancel)?;
            }
            if let Some(delivery) = &self.delivery {
                active(delivery)?;
            }
            host.validate(&self.computed, self.computed.sources(), observer)?;
            if self
                .content_context
                .as_ref()
                .is_some_and(|(cap, identity)| cap.cache_identity().as_ref() != Some(identity))
            {
                return Err("acquired content resolution context changed".into());
            }
            active(&self.cancel)?;
            if let Some(nested) = &self.nested {
                active(&nested.cancel)?;
            }
            if let Some(delivery) = &self.delivery {
                active(delivery)?;
            }
            Ok(())
        })();
        if result.is_err() {
            self.close();
        }
        result
    }
    pub fn close(&mut self) {
        self.revoked = true;
        self.fragment_targets.clear();
        self.template_links = None;
        self.content = None;
        self.nested = None;
        self.content_context = None;
        self.bindings.clear();
        self.delivery = None;
    }
}
fn active(cancel: &AbortSignal) -> Result<(), String> {
    if cancel.is_aborted() {
        Err("SVG acquisition cancelled".into())
    } else {
        Ok(())
    }
}
fn absolute(value: &str) -> Result<url::Url, String> {
    if value.len() > 4096 {
        return Err("SVG resource URL bound exceeded".into());
    }
    let url = url::Url::parse(value).map_err(|_| "absolute SVG resource URL required")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("HTTP(S) SVG resource URL without credentials required".into());
    }
    Ok(url)
}
pub(crate) fn computed_url(
    property: &str,
    value: &str,
    slot: usize,
) -> Result<(Arc<RetainedCemTree>, AstNodeId, url::Url), String> {
    // External CSS syntax enters through the shared importer, never a regex or
    // browser DTO. This tree describes an observation, not the authored source.
    if value.len() > 4096 {
        return Err("computed SVG CSS byte bound exceeded".into());
    }
    let text = format!("{property}:{value}");
    let uri = format!(
        "urn:cem:computed-svg-resource:{slot}:{}",
        ContentHash::from_blake3(text.as_bytes()).header_value()
    );
    let css = crate::import::import_data_bytes(
        text.as_bytes(),
        "text/css; mode=style-attribute",
        "cem",
        &uri,
    )?;
    let (reference, value) = single_url(&css, property)?;
    let url = absolute(&value)?;
    Ok((css, reference, url))
}
/// Inspect the retained shared CSS import; never substitute an observation tree
/// for authored-source provenance. Relative URL resolution belongs to the caller.
pub(crate) fn single_url(
    css: &Arc<RetainedCemTree>,
    property: &str,
) -> Result<(AstNodeId, String), String> {
    if css
        .ast_owner()
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation())
    {
        return Err("computed SVG CSS diagnostics".into());
    }
    let roots = &css.node(0).ok_or("computed CSS root missing")?.children;
    if roots.len() != 1 || !named(&css, roots[0], "style-attribute") {
        return Err("computed CSS declaration list required".into());
    }
    let declarations: Vec<_> = css
        .node(roots[0])
        .unwrap()
        .children
        .iter()
        .copied()
        .filter(|id| !named(&css, *id, "comment"))
        .collect();
    if declarations.len() != 1
        || !named(&css, declarations[0], "declaration")
        || attribute(&css, declarations[0], "name") != Some(property)
        || attribute(&css, declarations[0], "important").is_some()
    {
        return Err("one computed SVG declaration required".into());
    }
    let components: Vec<_> = css
        .node(declarations[0])
        .unwrap()
        .children
        .iter()
        .copied()
        .filter(|id| !matches!(attribute(&css, *id, "kind"), Some("whitespace" | "comment")))
        .collect();
    if components.len() != 1 {
        return Err("single computed SVG URL without fallback required".into());
    }
    let component = components[0];
    let reference = if attribute(&css, component, "kind") == Some("function") {
        let children = &css.node(component).unwrap().children;
        if children.len() != 1 {
            return Err("computed CSS function shape unsupported".into());
        }
        children[0]
    } else {
        component
    };
    let Some((value, CssResourceKind::Url)) = resource_reference(&css, reference)? else {
        return Err("computed SVG URL required".into());
    };
    Ok((reference, value))
}

/// Follow only explicit literal fragment hrefs. No fetch, aliases or inherited
/// content semantics. Each edge retains the exact selected attribute coordinate.
fn template_chain(
    tree: &RetainedCemTree,
    root: AstNodeId,
    ids: &BTreeMap<String, Vec<AstNodeId>>,
    cancel: &AbortSignal,
    spend: &mut impl FnMut(usize) -> Result<(), String>,
) -> Result<Vec<SvgPaintTemplateLink>, String> {
    use crate::svg_paint_dependencies::{href, kind, literal_fragment};
    let mut links = Vec::new();
    let mut seen = BTreeSet::new();
    let mut owner = root;
    loop {
        active(cancel)?;
        spend(
            1 + tree
                .node(owner)
                .ok_or("invalid template owner")?
                .attributes
                .len(),
        )?;
        seen.insert(owner);
        let Some(attribute) = href(tree, owner) else {
            break;
        };
        if links.len() >= 64 {
            return Err("SVG paint template depth exceeded".into());
        }
        // Fragment href resolves against the element's base. This initial
        // profile excludes any XML base attribute on the referencing ancestry,
        // even if empty or apparently equivalent to the acquisition URL.
        let mut ancestor = Some(owner);
        while let Some(id) = ancestor {
            let node = tree.node(id).ok_or("invalid template ancestry")?;
            spend(1 + node.attributes.len())?;
            if node.attributes.iter().any(|a| {
                tree.node(*a)
                    .and_then(|n| n.name.as_ref())
                    .is_some_and(|n| {
                        n.namespace_uri == "http://www.w3.org/XML/1998/namespace"
                            && n.local_name == "base"
                    })
            }) {
                return Err("SVG paint template XML base context unsupported".into());
            }
            ancestor = node.parent;
        }
        let value = &tree.node(attribute).ok_or("invalid template href")?.value;
        spend(value.len())?;
        let fragment = value
            .strip_prefix('#')
            .filter(|id| literal_fragment(id))
            .ok_or("literal same-body SVG template fragment required")?;
        let matches = ids.get(fragment).ok_or("missing SVG template target")?;
        if matches.len() != 1 {
            return Err("ambiguous SVG template target".into());
        }
        let target = matches[0];
        let target_kind = kind(tree, target).ok_or("unsupported SVG template target kind")?;
        let owner_kind = kind(tree, owner).ok_or("unsupported SVG template owner kind")?;
        if (owner_kind == "pattern") != (target_kind == "pattern") {
            return Err("incompatible SVG template target kind".into());
        }
        if seen.contains(&target) {
            return Err("SVG paint template cycle".into());
        }
        links.push(SvgPaintTemplateLink {
            owner,
            attribute,
            target,
        });
        owner = target;
    }
    Ok(links)
}
