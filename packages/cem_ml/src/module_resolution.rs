//! Scope-aware module and resource URL resolution shared by CEM hosts.
//!
//! Context construction owns module-map loading and normalization. Resolution is
//! deliberately synchronous and side-effect free: it never fetches the resolved
//! target or falls back to a host package search.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::source_map::SourceMapStack;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CemResolutionContextHandle(pub String);

impl CemResolutionContextHandle {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CemModuleUrlResolutionPurpose {
    TemplateSlice,
    CemQl,
    XPath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemModuleUrlResolutionRequest {
    pub purpose: CemModuleUrlResolutionPurpose,
    pub authored_specifier: String,
    pub current_context: CemResolutionContextHandle,
    pub referrer: Option<CemModuleUrlReferrer>,
    pub source_map: SourceMapStack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CemModuleUrlReferrer {
    Url(String),
    Context(CemResolutionContextHandle),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CemModuleUrlReferrerKind {
    Url,
    Context,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CemModuleUrlCollection {
    Imports,
    Resources,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CemModuleUrlResolution {
    pub authored_specifier: String,
    pub normalized_specifier: String,
    pub resolved_url: String,
    pub context_identity: String,
    pub resolver_identity: String,
    pub resource_policy_stamp: String,
    pub referrer_kind: Option<CemModuleUrlReferrerKind>,
    pub authored_referrer: Option<String>,
    pub resolved_referrer_url: Option<String>,
    pub current_context_identity: String,
    pub selected_context_identity: String,
    pub matched_frame_id: Option<String>,
    pub matched_scope_prefix: Option<String>,
    pub matched_collection: Option<CemModuleUrlCollection>,
    pub matched_key: Option<String>,
    pub content_type_hint: Option<String>,
    pub integrity: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CemModuleUrlResolutionErrorReason {
    Invalid,
    Unresolved,
    Blocked,
    PolicyDenied,
    Unavailable,
    ReferrerInvalid,
    ReferrerUnresolved,
    ReferrerUnavailable,
    ReferrerScopeDenied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CemModuleUrlResolutionError {
    pub authored_specifier: String,
    pub normalized_specifier: Option<String>,
    pub context_identity: String,
    pub resolver_identity: String,
    pub resource_policy_stamp: String,
    pub referrer_kind: Option<CemModuleUrlReferrerKind>,
    pub authored_referrer: Option<String>,
    pub resolved_referrer_url: Option<String>,
    pub current_context_identity: String,
    pub selected_context_identity: Option<String>,
    pub reason: CemModuleUrlResolutionErrorReason,
    pub message: String,
    pub matched_frame_id: Option<String>,
    pub matched_key: Option<String>,
}

impl fmt::Display for CemModuleUrlResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CemModuleUrlResolutionError {}

pub trait CemModuleUrlResolver: Send + Sync {
    fn resolve_module_url(
        &self,
        request: &CemModuleUrlResolutionRequest,
    ) -> Result<CemModuleUrlResolution, CemModuleUrlResolutionError>;
}

#[derive(Clone)]
pub struct CemModuleUrlResolutionCapability {
    resolver: Arc<dyn CemModuleUrlResolver>,
    context: CemResolutionContextHandle,
    node_contexts: Arc<BTreeMap<String, CemResolutionContextHandle>>,
}

impl CemModuleUrlResolutionCapability {
    pub fn new(
        resolver: Arc<dyn CemModuleUrlResolver>,
        context: CemResolutionContextHandle,
    ) -> Self {
        Self {
            resolver,
            context,
            node_contexts: Arc::new(BTreeMap::new()),
        }
    }

    pub fn context(&self) -> &CemResolutionContextHandle {
        &self.context
    }

    pub fn with_node_context(
        mut self,
        node_identity: impl Into<String>,
        context: CemResolutionContextHandle,
    ) -> Self {
        Arc::make_mut(&mut self.node_contexts).insert(node_identity.into(), context);
        self
    }

    pub fn node_context(&self, node_identity: &str) -> Option<&CemResolutionContextHandle> {
        self.node_contexts.get(node_identity)
    }

    pub fn resolve(
        &self,
        purpose: CemModuleUrlResolutionPurpose,
        authored_specifier: impl Into<String>,
        source_map: SourceMapStack,
    ) -> Result<CemModuleUrlResolution, CemModuleUrlResolutionError> {
        self.resolve_from_context(purpose, authored_specifier, &self.context, None, source_map)
    }

    pub fn resolve_with_referrer(
        &self,
        purpose: CemModuleUrlResolutionPurpose,
        authored_specifier: impl Into<String>,
        referrer: CemModuleUrlReferrer,
        source_map: SourceMapStack,
    ) -> Result<CemModuleUrlResolution, CemModuleUrlResolutionError> {
        self.resolve_from_context(
            purpose,
            authored_specifier,
            &self.context,
            Some(referrer),
            source_map,
        )
    }

    pub fn resolve_from_context(
        &self,
        purpose: CemModuleUrlResolutionPurpose,
        authored_specifier: impl Into<String>,
        current_context: &CemResolutionContextHandle,
        referrer: Option<CemModuleUrlReferrer>,
        source_map: SourceMapStack,
    ) -> Result<CemModuleUrlResolution, CemModuleUrlResolutionError> {
        self.resolver
            .resolve_module_url(&CemModuleUrlResolutionRequest {
                purpose,
                authored_specifier: authored_specifier.into(),
                current_context: current_context.clone(),
                referrer,
                source_map,
            })
    }
}

impl fmt::Debug for CemModuleUrlResolutionCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CemModuleUrlResolutionCapability")
            .field("context", &self.context)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CemModuleUrlMapping {
    pub target: Option<String>,
    pub content_type_hint: Option<String>,
    pub integrity: Option<String>,
}

impl CemModuleUrlMapping {
    pub fn target(target: impl Into<String>) -> Self {
        Self {
            target: Some(target.into()),
            ..Self::default()
        }
    }

    pub fn blocked() -> Self {
        Self::default()
    }

    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type_hint = Some(content_type.into());
        self
    }

    pub fn with_integrity(mut self, integrity: impl Into<String>) -> Self {
        self.integrity = Some(integrity.into());
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CemModuleUrlSpecifierMap {
    pub imports: BTreeMap<String, CemModuleUrlMapping>,
    pub resources: BTreeMap<String, CemModuleUrlMapping>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CemModuleUrlScopedMap {
    pub prefix: String,
    pub specifiers: CemModuleUrlSpecifierMap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CemModuleUrlFrame {
    pub frame_id: String,
    /// Base URL of the CEM/template context that owns this frame.
    pub base_url: String,
    /// Final source URL of an external module map. Inline maps use `base_url`.
    pub module_map_base_url: Option<String>,
    pub scopes: Vec<CemModuleUrlScopedMap>,
    pub specifiers: CemModuleUrlSpecifierMap,
    pub allowed_schemes: Option<BTreeSet<String>>,
}

impl CemModuleUrlFrame {
    pub fn new(frame_id: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            frame_id: frame_id.into(),
            base_url: base_url.into(),
            module_map_base_url: None,
            scopes: Vec::new(),
            specifiers: CemModuleUrlSpecifierMap::default(),
            allowed_schemes: None,
        }
    }

    pub fn with_module_map_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.module_map_base_url = Some(base_url.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CemModuleUrlContext {
    pub identity: String,
    pub resolver_identity: String,
    pub resource_policy_stamp: String,
    /// Frames are stored owner-first: root/outermost to current/innermost.
    pub frames: Vec<CemModuleUrlFrame>,
}

#[derive(Debug, Clone, Default)]
pub struct CemScopedModuleUrlResolver {
    contexts: BTreeMap<CemResolutionContextHandle, CemModuleUrlContext>,
    parents: BTreeMap<CemResolutionContextHandle, CemResolutionContextHandle>,
}

impl CemScopedModuleUrlResolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_context(
        mut self,
        handle: CemResolutionContextHandle,
        context: CemModuleUrlContext,
    ) -> Self {
        self.contexts.insert(handle, context);
        self
    }

    pub fn with_child_context(
        mut self,
        handle: CemResolutionContextHandle,
        parent: CemResolutionContextHandle,
        context: CemModuleUrlContext,
    ) -> Self {
        self.parents.insert(handle.clone(), parent);
        self.contexts.insert(handle, context);
        self
    }

    pub fn insert_context(
        &mut self,
        handle: CemResolutionContextHandle,
        context: CemModuleUrlContext,
    ) -> Option<CemModuleUrlContext> {
        self.contexts.insert(handle, context)
    }

    pub fn insert_child_context(
        &mut self,
        handle: CemResolutionContextHandle,
        parent: CemResolutionContextHandle,
        context: CemModuleUrlContext,
    ) -> Option<CemModuleUrlContext> {
        self.parents.insert(handle.clone(), parent);
        self.contexts.insert(handle, context)
    }

    fn is_current_or_descendant(
        &self,
        current: &CemResolutionContextHandle,
        selected: &CemResolutionContextHandle,
    ) -> bool {
        let mut cursor = selected;
        let mut visited = BTreeSet::new();
        loop {
            if cursor == current {
                return true;
            }
            if !visited.insert(cursor.clone()) {
                return false;
            }
            let Some(parent) = self.parents.get(cursor) else {
                return false;
            };
            cursor = parent;
        }
    }
}

impl CemModuleUrlResolver for CemScopedModuleUrlResolver {
    fn resolve_module_url(
        &self,
        request: &CemModuleUrlResolutionRequest,
    ) -> Result<CemModuleUrlResolution, CemModuleUrlResolutionError> {
        let Some(current_context) = self.contexts.get(&request.current_context) else {
            return Err(unavailable_error(request));
        };

        let (selected_context, active_base, provenance) = match request.referrer.as_ref() {
            None => {
                let active_base = context_active_base(request, current_context)?;
                (current_context, active_base, ReferrerProvenance::default())
            }
            Some(CemModuleUrlReferrer::Url(authored_referrer)) => {
                let authored_referrer = authored_referrer.trim();
                if authored_referrer.is_empty() {
                    return Err(context_error_with_provenance(
                        request,
                        current_context,
                        current_context,
                        CemModuleUrlResolutionErrorReason::ReferrerInvalid,
                        "module URL referrer is empty",
                        None,
                        None,
                        None,
                        ReferrerProvenance::url(authored_referrer, None),
                    ));
                }

                let referrer_url = match Url::parse(authored_referrer) {
                    Ok(url) => {
                        enforce_scheme(
                            request,
                            current_context,
                            current_context,
                            &current_context.frames,
                            &url,
                            authored_referrer,
                            &ReferrerProvenance::url(
                                authored_referrer,
                                Some(url.as_str().to_owned()),
                            ),
                        )?;
                        url
                    }
                    Err(error) if has_url_scheme_prefix(authored_referrer) => {
                        return Err(context_error_with_provenance(
                            request,
                            current_context,
                            current_context,
                            CemModuleUrlResolutionErrorReason::ReferrerInvalid,
                            format!(
                                "invalid absolute module URL referrer `{authored_referrer}`: {error}"
                            ),
                            None,
                            None,
                            None,
                            ReferrerProvenance::url(authored_referrer, None),
                        ));
                    }
                    Err(_) => {
                        let nested_request = CemModuleUrlResolutionRequest {
                            purpose: request.purpose,
                            authored_specifier: authored_referrer.to_owned(),
                            current_context: request.current_context.clone(),
                            referrer: None,
                            source_map: request.source_map.clone(),
                        };
                        let nested_base = context_active_base(&nested_request, current_context)?;
                        let resolved = self
                            .resolve_in_context(
                                &nested_request,
                                current_context,
                                current_context,
                                nested_base,
                                ReferrerProvenance::default(),
                            )
                            .map_err(|error| {
                                referrer_resolution_error(
                                    request,
                                    current_context,
                                    authored_referrer,
                                    error,
                                )
                            })?;
                        Url::parse(&resolved.resolved_url).map_err(|error| {
                            context_error_with_provenance(
                                request,
                                current_context,
                                current_context,
                                CemModuleUrlResolutionErrorReason::ReferrerInvalid,
                                format!(
                                    "resolved module URL referrer `{authored_referrer}` is invalid: {error}"
                                ),
                                None,
                                resolved.matched_frame_id,
                                resolved.matched_key,
                                ReferrerProvenance::url(authored_referrer, None),
                            )
                        })?
                    }
                };
                let provenance = ReferrerProvenance::url(
                    authored_referrer,
                    Some(referrer_url.as_str().to_owned()),
                );
                (current_context, referrer_url, provenance)
            }
            Some(CemModuleUrlReferrer::Context(selected_handle)) => {
                let Some(selected_context) = self.contexts.get(selected_handle) else {
                    return Err(context_error_with_provenance(
                        request,
                        current_context,
                        current_context,
                        CemModuleUrlResolutionErrorReason::ReferrerUnavailable,
                        format!(
                            "module URL referrer context `{}` is not installed",
                            selected_handle.0
                        ),
                        None,
                        None,
                        None,
                        ReferrerProvenance::context(None),
                    ));
                };
                if !self.is_current_or_descendant(&request.current_context, selected_handle) {
                    return Err(context_error_with_provenance(
                        request,
                        current_context,
                        selected_context,
                        CemModuleUrlResolutionErrorReason::ReferrerScopeDenied,
                        format!(
                            "module URL referrer context `{}` is not the current context or its descendant",
                            selected_handle.0
                        ),
                        None,
                        None,
                        None,
                        ReferrerProvenance::context(None),
                    ));
                }
                let active_base =
                    context_active_base(request, selected_context).map_err(|error| {
                        reclassify_referrer_error(
                            request,
                            current_context,
                            selected_context,
                            ReferrerProvenance::context(None),
                            error,
                        )
                    })?;
                let provenance = ReferrerProvenance::context(Some(active_base.as_str().to_owned()));
                enforce_scheme(
                    request,
                    current_context,
                    selected_context,
                    &selected_context.frames,
                    &active_base,
                    active_base.as_str(),
                    &provenance,
                )?;
                (selected_context, active_base, provenance)
            }
        };

        self.resolve_in_context(
            request,
            current_context,
            selected_context,
            active_base,
            provenance,
        )
    }
}

#[derive(Debug, Clone, Default)]
struct ReferrerProvenance {
    kind: Option<CemModuleUrlReferrerKind>,
    authored: Option<String>,
    resolved_url: Option<String>,
}

impl ReferrerProvenance {
    fn url(authored: &str, resolved_url: Option<String>) -> Self {
        Self {
            kind: Some(CemModuleUrlReferrerKind::Url),
            authored: Some(authored.to_owned()),
            resolved_url,
        }
    }

    fn context(resolved_url: Option<String>) -> Self {
        Self {
            kind: Some(CemModuleUrlReferrerKind::Context),
            authored: None,
            resolved_url,
        }
    }
}

impl CemScopedModuleUrlResolver {
    fn resolve_in_context(
        &self,
        request: &CemModuleUrlResolutionRequest,
        current_context: &CemModuleUrlContext,
        context: &CemModuleUrlContext,
        active_base: Url,
        provenance: ReferrerProvenance,
    ) -> Result<CemModuleUrlResolution, CemModuleUrlResolutionError> {
        let Some(_active_frame) = context.frames.last() else {
            return Err(context_error_with_provenance(
                request,
                current_context,
                context,
                CemModuleUrlResolutionErrorReason::Unavailable,
                "module URL resolution context has no active frame",
                None,
                None,
                None,
                provenance,
            ));
        };
        let authored = request.authored_specifier.trim();
        if authored.is_empty() {
            return Err(context_error_with_provenance(
                request,
                current_context,
                context,
                CemModuleUrlResolutionErrorReason::Invalid,
                "module URL specifier is empty",
                None,
                None,
                None,
                provenance,
            ));
        }
        let normalized_url = normalize_url_like(authored, &active_base).map_err(|message| {
            context_error_with_provenance(
                request,
                current_context,
                context,
                CemModuleUrlResolutionErrorReason::Invalid,
                message,
                None,
                None,
                None,
                provenance.clone(),
            )
        })?;
        let normalized_specifier = normalized_url
            .as_ref()
            .map_or_else(|| authored.to_owned(), |url| url.as_str().to_owned());

        for frame in &context.frames {
            let map_base = frame
                .module_map_base_url
                .as_deref()
                .unwrap_or(&frame.base_url);
            let frame_base = parse_base_url(map_base).map_err(|message| {
                context_error_with_provenance(
                    request,
                    current_context,
                    context,
                    CemModuleUrlResolutionErrorReason::Invalid,
                    message,
                    Some(normalized_specifier.clone()),
                    Some(frame.frame_id.clone()),
                    None,
                    provenance.clone(),
                )
            })?;
            let mut applicable_scopes = frame
                .scopes
                .iter()
                .filter_map(|scope| {
                    normalize_map_url(&scope.prefix, &frame_base)
                        .ok()
                        .filter(|prefix| scope_applies(prefix, active_base.as_str()))
                        .map(|prefix| (prefix, &scope.specifiers))
                })
                .collect::<Vec<_>>();
            applicable_scopes.sort_by(|left, right| {
                right
                    .0
                    .len()
                    .cmp(&left.0.len())
                    .then_with(|| left.0.cmp(&right.0))
            });

            for (scope_prefix, specifiers) in applicable_scopes
                .into_iter()
                .map(|(prefix, map)| (Some(prefix), map))
                .chain(std::iter::once((None, &frame.specifiers)))
            {
                for (collection, entries) in [
                    (CemModuleUrlCollection::Imports, &specifiers.imports),
                    (CemModuleUrlCollection::Resources, &specifiers.resources),
                ] {
                    let Some((matched_key, mapping, normalized_key)) =
                        find_mapping(entries, &normalized_specifier, &frame_base)
                    else {
                        continue;
                    };
                    let Some(target) = mapping.target.as_deref() else {
                        return Err(context_error_with_provenance(
                            request,
                            current_context,
                            context,
                            CemModuleUrlResolutionErrorReason::Blocked,
                            format!(
                                "module URL specifier `{authored}` is blocked by `{matched_key}`"
                            ),
                            Some(normalized_specifier),
                            Some(frame.frame_id.clone()),
                            Some(matched_key),
                            provenance.clone(),
                        ));
                    };
                    let resolved = resolve_mapping_target(
                        target,
                        &normalized_key,
                        &normalized_specifier,
                        &frame_base,
                    )
                    .map_err(|message| {
                        context_error_with_provenance(
                            request,
                            current_context,
                            context,
                            CemModuleUrlResolutionErrorReason::Blocked,
                            message,
                            Some(normalized_specifier.clone()),
                            Some(frame.frame_id.clone()),
                            Some(matched_key.clone()),
                            provenance.clone(),
                        )
                    })?;
                    enforce_scheme(
                        request,
                        current_context,
                        context,
                        &context.frames,
                        &resolved,
                        &normalized_specifier,
                        &provenance,
                    )?;
                    return Ok(CemModuleUrlResolution {
                        authored_specifier: authored.to_owned(),
                        normalized_specifier,
                        resolved_url: resolved.as_str().to_owned(),
                        context_identity: context.identity.clone(),
                        resolver_identity: context.resolver_identity.clone(),
                        resource_policy_stamp: context.resource_policy_stamp.clone(),
                        referrer_kind: provenance.kind,
                        authored_referrer: provenance.authored,
                        resolved_referrer_url: provenance.resolved_url,
                        current_context_identity: current_context.identity.clone(),
                        selected_context_identity: context.identity.clone(),
                        matched_frame_id: Some(frame.frame_id.clone()),
                        matched_scope_prefix: scope_prefix,
                        matched_collection: Some(collection),
                        matched_key: Some(matched_key),
                        content_type_hint: mapping.content_type_hint.clone(),
                        integrity: mapping.integrity.clone(),
                    });
                }
            }
        }

        if let Some(url) = normalized_url {
            enforce_scheme(
                request,
                current_context,
                context,
                &context.frames,
                &url,
                &normalized_specifier,
                &provenance,
            )?;
            return Ok(CemModuleUrlResolution {
                authored_specifier: authored.to_owned(),
                normalized_specifier,
                resolved_url: url.as_str().to_owned(),
                context_identity: context.identity.clone(),
                resolver_identity: context.resolver_identity.clone(),
                resource_policy_stamp: context.resource_policy_stamp.clone(),
                referrer_kind: provenance.kind,
                authored_referrer: provenance.authored,
                resolved_referrer_url: provenance.resolved_url,
                current_context_identity: current_context.identity.clone(),
                selected_context_identity: context.identity.clone(),
                matched_frame_id: None,
                matched_scope_prefix: None,
                matched_collection: None,
                matched_key: None,
                content_type_hint: None,
                integrity: None,
            });
        }

        Err(context_error_with_provenance(
            request,
            current_context,
            context,
            CemModuleUrlResolutionErrorReason::Unresolved,
            format!("bare module URL specifier `{authored}` is not mapped in the active context"),
            Some(normalized_specifier),
            None,
            None,
            provenance,
        ))
    }
}

fn parse_base_url(value: &str) -> Result<Url, String> {
    Url::parse(value).map_err(|error| format!("invalid module URL base `{value}`: {error}"))
}

fn context_active_base(
    request: &CemModuleUrlResolutionRequest,
    context: &CemModuleUrlContext,
) -> Result<Url, CemModuleUrlResolutionError> {
    let Some(active_frame) = context.frames.last() else {
        return Err(context_error_with_provenance(
            request,
            context,
            context,
            CemModuleUrlResolutionErrorReason::Unavailable,
            "module URL resolution context has no active frame",
            None,
            None,
            None,
            ReferrerProvenance::default(),
        ));
    };
    parse_base_url(&active_frame.base_url).map_err(|message| {
        context_error_with_provenance(
            request,
            context,
            context,
            CemModuleUrlResolutionErrorReason::Invalid,
            message,
            None,
            Some(active_frame.frame_id.clone()),
            None,
            ReferrerProvenance::default(),
        )
    })
}

fn has_url_scheme_prefix(value: &str) -> bool {
    let Some((scheme, _)) = value.split_once(':') else {
        return false;
    };
    let mut characters = scheme.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
        })
}

fn is_url_like(specifier: &str) -> bool {
    specifier.starts_with("//")
        || specifier.starts_with('/')
        || specifier.starts_with("./")
        || specifier.starts_with("../")
        || specifier.starts_with('?')
        || specifier.starts_with('#')
        || has_url_scheme_prefix(specifier)
        || Url::parse(specifier).is_ok()
}

fn normalize_url_like(specifier: &str, base: &Url) -> Result<Option<Url>, String> {
    if !is_url_like(specifier) {
        return Ok(None);
    }
    base.join(specifier)
        .or_else(|_| Url::parse(specifier))
        .map(Some)
        .map_err(|error| format!("invalid module URL specifier `{specifier}`: {error}"))
}

fn normalize_map_url(value: &str, base: &Url) -> Result<String, String> {
    base.join(value)
        .or_else(|_| Url::parse(value))
        .map(|url| url.as_str().to_owned())
        .map_err(|error| format!("invalid module-map URL `{value}`: {error}"))
}

fn normalized_map_key(key: &str, base: &Url) -> Result<String, String> {
    if is_url_like(key) {
        normalize_map_url(key, base)
    } else {
        Ok(key.to_owned())
    }
}

fn scope_applies(scope_prefix: &str, referrer: &str) -> bool {
    scope_prefix == referrer || (scope_prefix.ends_with('/') && referrer.starts_with(scope_prefix))
}

fn find_mapping<'a>(
    entries: &'a BTreeMap<String, CemModuleUrlMapping>,
    normalized_specifier: &str,
    base: &Url,
) -> Option<(String, &'a CemModuleUrlMapping, String)> {
    let mut exact = None;
    let mut prefix = None;
    for (key, mapping) in entries {
        let normalized_key = normalized_map_key(key, base).ok()?;
        if normalized_key == normalized_specifier {
            exact = Some((key.clone(), mapping, normalized_key));
            break;
        }
        if normalized_key.ends_with('/') && normalized_specifier.starts_with(&normalized_key) {
            let replace = prefix.as_ref().is_none_or(
                |(_, _, current): &(String, &CemModuleUrlMapping, String)| {
                    normalized_key.len() > current.len()
                },
            );
            if replace {
                prefix = Some((key.clone(), mapping, normalized_key));
            }
        }
    }
    exact.or(prefix)
}

fn resolve_mapping_target(
    target: &str,
    normalized_key: &str,
    normalized_specifier: &str,
    base: &Url,
) -> Result<Url, String> {
    let target_url = base
        .join(target)
        .or_else(|_| Url::parse(target))
        .map_err(|error| format!("invalid module-map target `{target}`: {error}"))?;
    if !normalized_key.ends_with('/') {
        return Ok(target_url);
    }
    if !target_url.as_str().ends_with('/') {
        return Err(format!(
            "module-map prefix target `{target}` must end with `/`"
        ));
    }
    let suffix = normalized_specifier
        .strip_prefix(normalized_key)
        .expect("prefix mapping is selected only for matching specifiers");
    let resolved = target_url
        .join(suffix)
        .map_err(|error| format!("invalid module-map prefix suffix `{suffix}`: {error}"))?;
    if !resolved.as_str().starts_with(target_url.as_str()) {
        return Err(format!(
            "module-map prefix resolution for `{normalized_specifier}` backtracks outside `{target}`"
        ));
    }
    Ok(resolved)
}

fn enforce_scheme(
    request: &CemModuleUrlResolutionRequest,
    current_context: &CemModuleUrlContext,
    context: &CemModuleUrlContext,
    policy_frames: &[CemModuleUrlFrame],
    url: &Url,
    normalized_specifier: &str,
    provenance: &ReferrerProvenance,
) -> Result<(), CemModuleUrlResolutionError> {
    for frame in policy_frames {
        if frame
            .allowed_schemes
            .as_ref()
            .is_some_and(|schemes| !schemes.contains(url.scheme()))
        {
            return Err(context_error_with_provenance(
                request,
                current_context,
                context,
                CemModuleUrlResolutionErrorReason::PolicyDenied,
                format!(
                    "module URL scheme `{}` is denied by context frame `{}`",
                    url.scheme(),
                    frame.frame_id
                ),
                Some(normalized_specifier.to_owned()),
                Some(frame.frame_id.clone()),
                None,
                provenance.clone(),
            ));
        }
    }
    Ok(())
}

fn unavailable_error(request: &CemModuleUrlResolutionRequest) -> CemModuleUrlResolutionError {
    CemModuleUrlResolutionError {
        authored_specifier: request.authored_specifier.clone(),
        normalized_specifier: None,
        context_identity: request.current_context.0.clone(),
        resolver_identity: "unavailable".to_owned(),
        resource_policy_stamp: "unavailable".to_owned(),
        referrer_kind: request.referrer.as_ref().map(|referrer| match referrer {
            CemModuleUrlReferrer::Url(_) => CemModuleUrlReferrerKind::Url,
            CemModuleUrlReferrer::Context(_) => CemModuleUrlReferrerKind::Context,
        }),
        authored_referrer: request
            .referrer
            .as_ref()
            .and_then(|referrer| match referrer {
                CemModuleUrlReferrer::Url(value) => Some(value.clone()),
                CemModuleUrlReferrer::Context(_) => None,
            }),
        resolved_referrer_url: None,
        current_context_identity: request.current_context.0.clone(),
        selected_context_identity: None,
        reason: CemModuleUrlResolutionErrorReason::Unavailable,
        message: format!(
            "module URL resolution context `{}` is not installed",
            request.current_context.0
        ),
        matched_frame_id: None,
        matched_key: None,
    }
}

fn context_error_with_provenance(
    request: &CemModuleUrlResolutionRequest,
    current_context: &CemModuleUrlContext,
    context: &CemModuleUrlContext,
    reason: CemModuleUrlResolutionErrorReason,
    message: impl Into<String>,
    normalized_specifier: Option<String>,
    matched_frame_id: Option<String>,
    matched_key: Option<String>,
    provenance: ReferrerProvenance,
) -> CemModuleUrlResolutionError {
    CemModuleUrlResolutionError {
        authored_specifier: request.authored_specifier.clone(),
        normalized_specifier,
        context_identity: context.identity.clone(),
        resolver_identity: context.resolver_identity.clone(),
        resource_policy_stamp: context.resource_policy_stamp.clone(),
        referrer_kind: provenance.kind,
        authored_referrer: provenance.authored,
        resolved_referrer_url: provenance.resolved_url,
        current_context_identity: current_context.identity.clone(),
        selected_context_identity: Some(context.identity.clone()),
        reason,
        message: message.into(),
        matched_frame_id,
        matched_key,
    }
}

fn referrer_resolution_error(
    request: &CemModuleUrlResolutionRequest,
    current_context: &CemModuleUrlContext,
    authored_referrer: &str,
    error: CemModuleUrlResolutionError,
) -> CemModuleUrlResolutionError {
    let reason = match error.reason {
        CemModuleUrlResolutionErrorReason::Invalid => {
            CemModuleUrlResolutionErrorReason::ReferrerInvalid
        }
        CemModuleUrlResolutionErrorReason::PolicyDenied => {
            CemModuleUrlResolutionErrorReason::PolicyDenied
        }
        CemModuleUrlResolutionErrorReason::Unavailable => {
            CemModuleUrlResolutionErrorReason::ReferrerUnavailable
        }
        _ => CemModuleUrlResolutionErrorReason::ReferrerUnresolved,
    };
    context_error_with_provenance(
        request,
        current_context,
        current_context,
        reason,
        format!(
            "module URL referrer `{authored_referrer}` could not be resolved: {}",
            error.message
        ),
        error.normalized_specifier,
        error.matched_frame_id,
        error.matched_key,
        ReferrerProvenance::url(authored_referrer, error.resolved_referrer_url),
    )
}

fn reclassify_referrer_error(
    request: &CemModuleUrlResolutionRequest,
    current_context: &CemModuleUrlContext,
    selected_context: &CemModuleUrlContext,
    provenance: ReferrerProvenance,
    error: CemModuleUrlResolutionError,
) -> CemModuleUrlResolutionError {
    let reason = match error.reason {
        CemModuleUrlResolutionErrorReason::Unavailable => {
            CemModuleUrlResolutionErrorReason::ReferrerUnavailable
        }
        CemModuleUrlResolutionErrorReason::PolicyDenied => {
            CemModuleUrlResolutionErrorReason::PolicyDenied
        }
        _ => CemModuleUrlResolutionErrorReason::ReferrerInvalid,
    };
    context_error_with_provenance(
        request,
        current_context,
        selected_context,
        reason,
        error.message,
        error.normalized_specifier,
        error.matched_frame_id,
        error.matched_key,
        provenance,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(frames: Vec<CemModuleUrlFrame>) -> CemModuleUrlContext {
        CemModuleUrlContext {
            identity: "context:v1".to_owned(),
            resolver_identity: "resolver:v1".to_owned(),
            resource_policy_stamp: "policy:v1".to_owned(),
            frames,
        }
    }

    fn request(specifier: &str) -> CemModuleUrlResolutionRequest {
        CemModuleUrlResolutionRequest {
            purpose: CemModuleUrlResolutionPurpose::CemQl,
            authored_specifier: specifier.to_owned(),
            current_context: CemResolutionContextHandle::new("test"),
            referrer: None,
            source_map: SourceMapStack::default(),
        }
    }

    fn request_with_referrer(
        specifier: &str,
        referrer: CemModuleUrlReferrer,
    ) -> CemModuleUrlResolutionRequest {
        let mut request = request(specifier);
        request.referrer = Some(referrer);
        request
    }

    #[test]
    fn outer_prefix_wins_before_inner_exact() {
        let mut outer = CemModuleUrlFrame::new("page", "https://example.test/app/index.html");
        outer.specifiers.imports.insert(
            "pkg/".to_owned(),
            CemModuleUrlMapping::target("https://cdn.example.test/safe/"),
        );
        let mut inner =
            CemModuleUrlFrame::new("template", "https://example.test/components/card.cem");
        inner.specifiers.imports.insert(
            "pkg/special".to_owned(),
            CemModuleUrlMapping::target("./local.js"),
        );
        let resolver = CemScopedModuleUrlResolver::new().with_context(
            CemResolutionContextHandle::new("test"),
            context(vec![outer, inner]),
        );
        let resolved = resolver
            .resolve_module_url(&request("pkg/special"))
            .unwrap();
        assert_eq!(
            resolved.resolved_url,
            "https://cdn.example.test/safe/special"
        );
        assert_eq!(resolved.matched_frame_id.as_deref(), Some("page"));
    }

    #[test]
    fn inner_map_fills_outer_miss_and_preserves_resource_metadata() {
        let outer = CemModuleUrlFrame::new("page", "https://example.test/index.html");
        let mut inner =
            CemModuleUrlFrame::new("template", "https://example.test/components/card.cem");
        inner.specifiers.resources.insert(
            "logo".to_owned(),
            CemModuleUrlMapping::target("./logo.svg")
                .with_content_type("image/svg+xml")
                .with_integrity("sha256-example"),
        );
        let resolver = CemScopedModuleUrlResolver::new().with_context(
            CemResolutionContextHandle::new("test"),
            context(vec![outer, inner]),
        );
        let resolved = resolver.resolve_module_url(&request("logo")).unwrap();
        assert_eq!(
            resolved.resolved_url,
            "https://example.test/components/logo.svg"
        );
        assert_eq!(
            resolved.matched_collection,
            Some(CemModuleUrlCollection::Resources)
        );
        assert_eq!(resolved.content_type_hint.as_deref(), Some("image/svg+xml"));
        assert_eq!(resolved.integrity.as_deref(), Some("sha256-example"));
    }

    #[test]
    fn relative_url_uses_active_template_base_and_bare_miss_fails() {
        let resolver = CemScopedModuleUrlResolver::new().with_context(
            CemResolutionContextHandle::new("test"),
            context(vec![CemModuleUrlFrame::new(
                "template",
                "https://example.test/components/card.cem",
            )]),
        );
        assert_eq!(
            resolver
                .resolve_module_url(&request("./logo.svg"))
                .unwrap()
                .resolved_url,
            "https://example.test/components/logo.svg"
        );
        assert_eq!(
            resolver
                .resolve_module_url(&request("missing"))
                .unwrap_err()
                .reason,
            CemModuleUrlResolutionErrorReason::Unresolved
        );
        assert_eq!(
            resolver
                .resolve_module_url(&request("https://[invalid"))
                .unwrap_err()
                .reason,
            CemModuleUrlResolutionErrorReason::Invalid
        );
    }

    #[test]
    fn blocked_outer_mapping_prevents_inner_fallback() {
        let mut outer = CemModuleUrlFrame::new("page", "https://example.test/index.html");
        outer
            .specifiers
            .imports
            .insert("pkg".to_owned(), CemModuleUrlMapping::blocked());
        let mut inner = CemModuleUrlFrame::new("template", "https://example.test/card.cem");
        inner
            .specifiers
            .imports
            .insert("pkg".to_owned(), CemModuleUrlMapping::target("./pkg.js"));
        let resolver = CemScopedModuleUrlResolver::new().with_context(
            CemResolutionContextHandle::new("test"),
            context(vec![outer, inner]),
        );
        let error = resolver.resolve_module_url(&request("pkg")).unwrap_err();
        assert_eq!(error.reason, CemModuleUrlResolutionErrorReason::Blocked);
        assert_eq!(error.matched_frame_id.as_deref(), Some("page"));
    }

    #[test]
    fn outer_policy_constrains_an_inner_mapping() {
        let mut outer = CemModuleUrlFrame::new("page", "https://example.test/index.html");
        outer.allowed_schemes = Some(BTreeSet::from(["https".to_owned()]));
        let mut inner = CemModuleUrlFrame::new("template", "https://example.test/card.cem");
        inner.specifiers.resources.insert(
            "asset".to_owned(),
            CemModuleUrlMapping::target("data:text/plain,blocked"),
        );
        let resolver = CemScopedModuleUrlResolver::new().with_context(
            CemResolutionContextHandle::new("test"),
            context(vec![outer, inner]),
        );
        let error = resolver.resolve_module_url(&request("asset")).unwrap_err();
        assert_eq!(
            error.reason,
            CemModuleUrlResolutionErrorReason::PolicyDenied
        );
        assert_eq!(error.matched_frame_id.as_deref(), Some("page"));
    }

    #[test]
    fn effective_inner_policy_constrains_an_outer_mapping() {
        let mut outer = CemModuleUrlFrame::new("page", "https://example.test/index.html");
        outer.specifiers.resources.insert(
            "asset".to_owned(),
            CemModuleUrlMapping::target("data:text/plain,blocked"),
        );
        let mut inner = CemModuleUrlFrame::new("template", "https://example.test/card.cem");
        inner.allowed_schemes = Some(BTreeSet::from(["https".to_owned()]));
        let resolver = CemScopedModuleUrlResolver::new().with_context(
            CemResolutionContextHandle::new("test"),
            context(vec![outer, inner]),
        );

        let error = resolver.resolve_module_url(&request("asset")).unwrap_err();

        assert_eq!(
            error.reason,
            CemModuleUrlResolutionErrorReason::PolicyDenied
        );
        assert_eq!(error.matched_frame_id.as_deref(), Some("template"));
    }

    #[test]
    fn authored_references_and_external_map_targets_use_their_own_bases() {
        let mut frame =
            CemModuleUrlFrame::new("template", "https://example.test/components/card/card.cem")
                .with_module_map_base_url("https://cdn.example.test/maps/runtime-map.json");
        frame.specifiers.resources.insert(
            "logo".to_owned(),
            CemModuleUrlMapping::target("./assets/logo.svg"),
        );
        let resolver = CemScopedModuleUrlResolver::new().with_context(
            CemResolutionContextHandle::new("test"),
            context(vec![frame]),
        );

        assert_eq!(
            resolver
                .resolve_module_url(&request("logo"))
                .unwrap()
                .resolved_url,
            "https://cdn.example.test/maps/assets/logo.svg"
        );
        assert_eq!(
            resolver
                .resolve_module_url(&request("./assets/logo.svg"))
                .unwrap()
                .resolved_url,
            "https://example.test/components/card/assets/logo.svg"
        );
    }

    #[test]
    fn absolute_referrer_is_not_remapped_and_selects_scoped_target_mapping() {
        let mut frame = CemModuleUrlFrame::new("page", "https://example.test/index.html");
        frame.specifiers.imports.insert(
            "https://cdn.example.test/pkg/module.js".to_owned(),
            CemModuleUrlMapping::target("https://evil.example.test/module.js"),
        );
        let mut scoped = CemModuleUrlSpecifierMap::default();
        scoped.imports.insert(
            "asset".to_owned(),
            CemModuleUrlMapping::target("https://cdn.example.test/assets/scoped.css"),
        );
        frame.scopes.push(CemModuleUrlScopedMap {
            prefix: "https://cdn.example.test/pkg/".to_owned(),
            specifiers: scoped,
        });
        let resolver = CemScopedModuleUrlResolver::new().with_context(
            CemResolutionContextHandle::new("test"),
            context(vec![frame]),
        );

        let resolved = resolver
            .resolve_module_url(&request_with_referrer(
                "asset",
                CemModuleUrlReferrer::Url("https://cdn.example.test/pkg/module.js".to_owned()),
            ))
            .unwrap();

        assert_eq!(
            resolved.resolved_referrer_url.as_deref(),
            Some("https://cdn.example.test/pkg/module.js")
        );
        assert_eq!(
            resolved.resolved_url,
            "https://cdn.example.test/assets/scoped.css"
        );
        assert_eq!(resolved.referrer_kind, Some(CemModuleUrlReferrerKind::Url));
    }

    #[test]
    fn bare_referrer_resolves_once_through_current_maps() {
        let mut frame = CemModuleUrlFrame::new("page", "https://example.test/index.html");
        frame.specifiers.imports.insert(
            "worker".to_owned(),
            CemModuleUrlMapping::target("https://cdn.example.test/workers/worker.js"),
        );
        let mut scoped = CemModuleUrlSpecifierMap::default();
        scoped.resources.insert(
            "asset".to_owned(),
            CemModuleUrlMapping::target("https://cdn.example.test/workers/worker.wasm"),
        );
        frame.scopes.push(CemModuleUrlScopedMap {
            prefix: "https://cdn.example.test/workers/".to_owned(),
            specifiers: scoped,
        });
        let resolver = CemScopedModuleUrlResolver::new().with_context(
            CemResolutionContextHandle::new("test"),
            context(vec![frame]),
        );

        let resolved = resolver
            .resolve_module_url(&request_with_referrer(
                "asset",
                CemModuleUrlReferrer::Url("worker".to_owned()),
            ))
            .unwrap();

        assert_eq!(
            resolved.resolved_referrer_url.as_deref(),
            Some("https://cdn.example.test/workers/worker.js")
        );
        assert_eq!(
            resolved.resolved_url,
            "https://cdn.example.test/workers/worker.wasm"
        );
    }

    #[test]
    fn descendant_context_referrer_uses_child_base_and_local_map() {
        let root_handle = CemResolutionContextHandle::new("root");
        let child_handle = CemResolutionContextHandle::new("child");
        let root_frame = CemModuleUrlFrame::new("page", "https://example.test/index.html");
        let root_context = CemModuleUrlContext {
            identity: "context:root".to_owned(),
            resolver_identity: "resolver:v1".to_owned(),
            resource_policy_stamp: "policy:v1".to_owned(),
            frames: vec![root_frame.clone()],
        };
        let mut child_frame =
            CemModuleUrlFrame::new("template", "https://example.test/components/card.cem");
        child_frame.specifiers.resources.insert(
            "asset".to_owned(),
            CemModuleUrlMapping::target("./card.css"),
        );
        let child_context = CemModuleUrlContext {
            identity: "context:child".to_owned(),
            resolver_identity: "resolver:v1".to_owned(),
            resource_policy_stamp: "policy:v1".to_owned(),
            frames: vec![root_frame, child_frame],
        };
        let resolver = CemScopedModuleUrlResolver::new()
            .with_context(root_handle.clone(), root_context)
            .with_child_context(child_handle.clone(), root_handle, child_context);
        let mut request = CemModuleUrlResolutionRequest {
            purpose: CemModuleUrlResolutionPurpose::CemQl,
            authored_specifier: "asset".to_owned(),
            current_context: CemResolutionContextHandle::new("root"),
            referrer: Some(CemModuleUrlReferrer::Context(child_handle)),
            source_map: SourceMapStack::default(),
        };

        let resolved = resolver.resolve_module_url(&request).unwrap();
        assert_eq!(
            resolved.resolved_url,
            "https://example.test/components/card.css"
        );
        assert_eq!(resolved.current_context_identity, "context:root");
        assert_eq!(resolved.selected_context_identity, "context:child");

        request.current_context = CemResolutionContextHandle::new("child");
        request.referrer = Some(CemModuleUrlReferrer::Context(
            CemResolutionContextHandle::new("root"),
        ));
        assert_eq!(
            resolver.resolve_module_url(&request).unwrap_err().reason,
            CemModuleUrlResolutionErrorReason::ReferrerScopeDenied
        );
    }

    #[test]
    fn sibling_and_uninstalled_context_referrers_are_rejected() {
        let root_handle = CemResolutionContextHandle::new("root");
        let left_handle = CemResolutionContextHandle::new("left");
        let right_handle = CemResolutionContextHandle::new("right");
        let root_frame = CemModuleUrlFrame::new("page", "https://example.test/index.html");
        let make_context = |identity: &str| CemModuleUrlContext {
            identity: identity.to_owned(),
            resolver_identity: "resolver:v1".to_owned(),
            resource_policy_stamp: "policy:v1".to_owned(),
            frames: vec![root_frame.clone()],
        };
        let resolver = CemScopedModuleUrlResolver::new()
            .with_context(root_handle.clone(), make_context("context:root"))
            .with_child_context(
                left_handle.clone(),
                root_handle.clone(),
                make_context("context:left"),
            )
            .with_child_context(
                right_handle.clone(),
                root_handle,
                make_context("context:right"),
            );
        let mut request = CemModuleUrlResolutionRequest {
            purpose: CemModuleUrlResolutionPurpose::XPath,
            authored_specifier: "./asset.css".to_owned(),
            current_context: left_handle,
            referrer: Some(CemModuleUrlReferrer::Context(right_handle)),
            source_map: SourceMapStack::default(),
        };

        assert_eq!(
            resolver.resolve_module_url(&request).unwrap_err().reason,
            CemModuleUrlResolutionErrorReason::ReferrerScopeDenied
        );
        request.referrer = Some(CemModuleUrlReferrer::Context(
            CemResolutionContextHandle::new("missing"),
        ));
        assert_eq!(
            resolver.resolve_module_url(&request).unwrap_err().reason,
            CemModuleUrlResolutionErrorReason::ReferrerUnavailable
        );
    }
}
