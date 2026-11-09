//! Native URL preflight and original-operation page ingress limits.
use std::{collections::BTreeMap, fs, path::Path, sync::Arc};

use crate::diagnostics::{Diagnostic, Severity};
use crate::operation_control::{
    cpu::NativeCpuAccounting, OperationControl, ROOT_EXECUTION_SCOPE_ID,
};
use crate::resolver::{
    local_path_or_file_uri, ResolveDirection, ResolvePurpose, ResolveRequest, SsrReadBudget,
};
use crate::source_map::SourceMapStack;
use crate::validation::html::{
    HtmlDocumentAst, HtmlDocumentMode, HtmlEventAst, HtmlEventKind, HtmlNamespace,
};
use url::Url;

pub const SSR_PAGE_ENTRYPOINT: &str = "ssr:render-page";

/// Limits selected by the trusted page template before the native HTML loader runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SsrPageIngressPolicy {
    pub max_bytes: u64,
    pub wall_ms: u64,
    pub cpu_ms: u64,
}

impl SsrPageIngressPolicy {
    pub fn read_local_page(
        self,
        control: &OperationControl,
        path: &Path,
    ) -> Result<Vec<u8>, String> {
        self.run(control, &path.to_string_lossy(), |request, budget| {
            let local =
                local_path_or_file_uri(path, "SSR page path").map_err(|error| error.to_string())?;
            let metadata = fs::metadata(&local).map_err(|error| error.to_string())?;
            if !metadata.is_file() {
                return Err("SSR page acquisition requires a regular local file".into());
            }
            if metadata.len() > self.max_bytes {
                return Err(
                    "Original SSR page byte budget exceeded before opening the file".into(),
                );
            }
            budget.check(request).map_err(|error| error.to_string())?;
            let mut options = fs::OpenOptions::new();
            options.read(true);
            #[cfg(target_os = "linux")]
            {
                use std::os::unix::fs::OpenOptionsExt;
                // A path replaced by a FIFO between metadata and open must not block.
                options.custom_flags(libc::O_NONBLOCK);
            }
            let file = options.open(local).map_err(|error| error.to_string())?;
            if !file
                .metadata()
                .map_err(|error| error.to_string())?
                .is_file()
            {
                return Err("SSR page acquisition requires a regular local file".into());
            }
            budget
                .read_from(request, file)
                .map_err(|error| error.to_string())
        })
    }

    pub(crate) fn run<T>(
        self,
        control: &OperationControl,
        uri: &str,
        phase: impl FnOnce(&ResolveRequest, &SsrReadBudget<'_>) -> Result<T, String>,
    ) -> Result<T, String> {
        let max_bytes =
            usize::try_from(self.max_bytes).map_err(|_| "SSR page byte limit exceeds usize")?;
        let cpu = Arc::new(NativeCpuAccounting::new(control, self.cpu_ms)?);
        let guard = cpu.begin(control)?;
        let budget = SsrReadBudget {
            control,
            cpu: &cpu,
            scope: ROOT_EXECUTION_SCOPE_ID,
            max_bytes,
            max_wall_time_ms: self.wall_ms,
        };
        let request = ResolveRequest::new(uri, ResolvePurpose::Input, ResolveDirection::Read);
        let result = (|| {
            budget.check(&request).map_err(|e| e.to_string())?;
            let result = phase(&request, &budget)?;
            budget.check(&request).map_err(|e| e.to_string())?;
            Ok(result)
        })();
        let receipt = guard.finish();
        result.and_then(|value| receipt.map(|_| value))
    }
}

/// A host decision for the selected base URL, not permission to read a resource.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SsrBasePolicy {
    /// Caller asserts no effective header CSP. A CSP meta element requires a
    /// host decision. This module never parses or grants policy capabilities.
    #[default]
    Unspecified,
    /// Host evaluated the selected candidate against the effective policy.
    Allowed,
    /// Host evaluated the selected candidate and requires the page fallback.
    Blocked,
}

#[derive(Debug, Clone, Copy)]
pub struct SsrPageRouteRequest<'a> {
    pub document: &'a HtmlDocumentAst,
    pub page_url: &'a str,
    /// Prepared absolute URI; path-to-URI conversion belongs to the host.
    pub output_uri: Option<&'a str>,
    pub page_url_source_map: Option<&'a SourceMapStack>,
    pub output_uri_source_map: Option<&'a SourceMapStack>,
    pub base_policy: SsrBasePolicy,
}

impl<'a> SsrPageRouteRequest<'a> {
    pub fn new(
        document: &'a HtmlDocumentAst,
        page_url: &'a str,
        output_uri: Option<&'a str>,
    ) -> Self {
        Self {
            document,
            page_url,
            output_uri,
            page_url_source_map: None,
            output_uri_source_map: None,
            base_policy: SsrBasePolicy::Unspecified,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SsrBaseSelection {
    PageUrl,
    HtmlBase {
        #[serde(rename = "eventIndex")]
        event_index: usize,
        #[serde(rename = "usedFallback")]
        used_fallback: bool,
    },
}

/// Validated URL context retaining the original native owner and its provenance.
/// This is neither a render artifact nor a hydration/cache key.
#[derive(Debug)]
pub struct SsrPageRoute<'a> {
    request: SsrPageRouteRequest<'a>,
    page_url: Url,
    document_base_url: Url,
    output_uri: Option<Url>,
    base_selection: SsrBaseSelection,
    base_source_map: Option<SourceMapStack>,
}

impl<'a> SsrPageRoute<'a> {
    pub fn source_document(&self) -> &'a HtmlDocumentAst {
        self.request.document
    }
    pub fn source_uri(&self) -> &'a str {
        &self.request.document.source.uri
    }
    pub fn page_url(&self) -> &str {
        self.page_url.as_str()
    }
    pub fn document_base_url(&self) -> &str {
        self.document_base_url.as_str()
    }
    pub fn output_uri(&self) -> Option<&str> {
        self.output_uri.as_ref().map(Url::as_str)
    }
    pub fn base_selection(&self) -> SsrBaseSelection {
        self.base_selection
    }
    pub fn base_policy(&self) -> SsrBasePolicy {
        self.request.base_policy
    }
    pub fn base_source_map(&self) -> Option<&SourceMapStack> {
        self.base_source_map.as_ref()
    }
    pub fn page_url_source_map(&self) -> Option<&'a SourceMapStack> {
        self.request.page_url_source_map
    }
    pub fn output_uri_source_map(&self) -> Option<&'a SourceMapStack> {
        self.request.output_uri_source_map
    }
    /// Only the URL portion of render identity. Source bytes, schema, lifecycle,
    /// resolver/policy stamps and capabilities must be added by the SSR artifact.
    pub fn render_url_identity(&self) -> (&str, &str, &str) {
        (self.source_uri(), self.page_url(), self.document_base_url())
    }

    /// Source-owned acquisition keeps the source identity, never the page/output.
    pub fn source_read_request(&self) -> ResolveRequest {
        ResolveRequest::new(
            self.source_uri(),
            ResolvePurpose::Input,
            ResolveDirection::Read,
        )
        .with_content_type_hint(&self.request.document.source.content_type)
    }

    /// Resolve the public address only. The caller must still apply its existing
    /// resolver policy/read map before dispatch; local targets never change this route.
    pub fn page_resource_read_request(
        &self,
        uri: &str,
        purpose: ResolvePurpose,
    ) -> Result<ResolveRequest, url::ParseError> {
        Ok(ResolveRequest::new(
            self.document_base_url.join(uri)?.as_str(),
            purpose,
            ResolveDirection::Read,
        )
        .with_base_uri(self.document_base_url()))
    }

    /// A destination request, not a write grant. Absent for an in-memory artifact.
    pub fn output_write_request(&self) -> Option<ResolveRequest> {
        self.output_uri()
            .map(|uri| ResolveRequest::new(uri, ResolvePurpose::Output, ResolveDirection::Write))
    }
}

/// Hard ceiling on one atomic preflight, independent of execution/settle budgets.
pub const SSR_ROUTE_BATCH_LIMIT: usize = 1024;

/// Validate the entire batch before a caller starts execution. No reads, writes,
/// resolver admission or JavaScript execution occur here. No partial success.
pub fn preflight_ssr_page_routes<'a>(
    requests: &[SsrPageRouteRequest<'a>],
) -> Result<Vec<SsrPageRoute<'a>>, Vec<Diagnostic>> {
    if requests.len() > SSR_ROUTE_BATCH_LIMIT {
        return Err(vec![Diagnostic {
            code: "cem.ssr.route_limit".into(),
            severity: Severity::Error,
            message: format!("SSR route batch exceeds the {SSR_ROUTE_BATCH_LIMIT} route limit"),
            ..Diagnostic::default()
        }]);
    }
    let mut routes = Vec::with_capacity(requests.len());
    let mut errors = Vec::new();
    let mut pages = BTreeMap::new();
    let mut outputs = BTreeMap::new();
    for (index, request) in requests.iter().copied().enumerate() {
        match preflight_route(request, index) {
            Ok(route) => {
                for (key, seen, code, message, source_map) in [
                    (
                        Some(route.page_url()),
                        &mut pages,
                        "cem.ssr.page_url_duplicate",
                        "Duplicate normalized SSR page URL",
                        request.page_url_source_map,
                    ),
                    (
                        route.output_uri(),
                        &mut outputs,
                        "cem.transform_runtime.duplicate_destination",
                        "Duplicate normalized SSR output URI",
                        request.output_uri_source_map,
                    ),
                ] {
                    if let Some(key) = key {
                        if let Some(first) = seen.get(key) {
                            let mut error =
                                route_error(request, index, code, message, source_map.cloned());
                            error.details.as_mut().unwrap()["firstRouteIndex"] =
                                serde_json::json!(first);
                            errors.push(error);
                        } else {
                            seen.insert(key.to_owned(), index);
                        }
                    }
                }
                routes.push(route);
            }
            Err(error) => errors.push(*error),
        }
    }
    if errors.is_empty() {
        Ok(routes)
    } else {
        Err(errors)
    }
}

// Prepared addresses are explicit absolute URIs, without fragments, credentials,
// or implicit whitespace cleanup. Unknown source/output schemes remain passive
// addresses for a host resolver, never newly admitted capabilities.
fn prepared_uri(value: &str) -> Option<Url> {
    if value.trim() != value || value.chars().any(char::is_control) {
        return None;
    }
    let url = Url::parse(value).ok()?;
    if url.fragment().is_some() || has_credentials(&url) {
        return None;
    }
    Some(url)
}

fn page_url(value: &str) -> Option<Url> {
    prepared_uri(value).filter(|url| matches!(url.scheme(), "https" | "http") && url.has_host())
}

fn has_credentials(url: &Url) -> bool {
    !url.username().is_empty() || url.password().is_some()
}

fn route_error(
    request: SsrPageRouteRequest<'_>,
    index: usize,
    code: &str,
    message: &str,
    source_map: Option<SourceMapStack>,
) -> Diagnostic {
    // Invalid authored values are deliberately omitted from diagnostic control
    // metadata. Keep the supplied origin stack unchanged, including foreign owners.
    let source_uri =
        prepared_uri(&request.document.source.uri).map(|_| request.document.source.uri.as_str());
    let map_uri = source_map.as_ref().and_then(|map| {
        map.frames.iter().find_map(|frame| {
            if let crate::source_map::TransformKind::SourceBoundary { uri } = &frame.transform {
                Some(uri.clone())
            } else {
                None
            }
        })
    });
    Diagnostic {
        uri: map_uri.or_else(|| source_uri.map(str::to_owned)),
        code: code.into(),
        severity: Severity::Error,
        message: message.into(),
        source_map,
        // Explicit diagnostic control boundary; no document/AST serialization.
        details: Some(
            serde_json::json!({ "routeIndex": index, "sourceUri": source_uri,
            "pageUrl": page_url(request.page_url).map(|url| url.to_string()),
            "outputUri": request.output_uri.and_then(prepared_uri).map(|url| url.to_string()) }),
        ),
        ..Diagnostic::default()
    }
}

/// Native HTML events retain namespaces and decoded attributes. Template children
/// are in an inert fragment, not descendants in the document tree. No source-text
/// search, reparse, serialization or script execution is used to discover bases.
fn document_elements(document: &HtmlDocumentAst) -> impl Iterator<Item = &HtmlEventAst> {
    let mut template_depth = None;
    document.events.iter().filter(move |event| {
        if let Some(depth) = template_depth {
            if event.depth > depth {
                return false;
            }
            template_depth = None;
        }
        if event.kind != HtmlEventKind::StartElement || event.namespace != HtmlNamespace::Html {
            return false;
        }
        if event.local_name.as_deref() == Some("template") {
            template_depth = Some(event.depth);
            return false;
        }
        true
    })
}

fn attribute<'a>(
    event: &'a HtmlEventAst,
    name: &str,
) -> Option<&'a crate::validation::html::HtmlAttributeAst> {
    event
        .attributes
        .iter()
        .find(|attr| !attr.duplicate && attr.local_name == name)
}

fn preflight_route<'a>(
    request: SsrPageRouteRequest<'a>,
    index: usize,
) -> Result<SsrPageRoute<'a>, Box<Diagnostic>> {
    let error = |code, message, source_map| {
        Box::new(route_error(request, index, code, message, source_map))
    };
    if prepared_uri(&request.document.source.uri).is_none() {
        return Err(error(
            "cem.ssr.source_uri_invalid",
            "SSR source requires a prepared absolute URI without credentials or fragment",
            None,
        ));
    }
    let page_url = page_url(request.page_url).ok_or_else(|| {
        error(
            "cem.ssr.page_url_invalid",
            "SSR page requires an absolute HTTP(S) URL without credentials or fragment",
            request.page_url_source_map.cloned(),
        )
    })?;
    let output_uri = request
        .output_uri
        .map(|uri| {
            prepared_uri(uri).ok_or_else(|| {
                error(
                    "cem.ssr.output_uri_invalid",
                    "SSR output requires a prepared absolute URI without credentials or fragment",
                    request.output_uri_source_map.cloned(),
                )
            })
        })
        .transpose()?;
    // Do not infer browser tree-builder, scripting-mode or legacy URL encoding
    // state from a lexical event stream. These profiles need an import contract
    // extension before the SSR coordinator can use them faithfully.
    let document = request.document;
    let unsupported_event = document.events.iter().find(|event| {
        event.kind == HtmlEventKind::StartElement
            && event.namespace == HtmlNamespace::Html
            && ((event.self_closing && !event.void_element)
                || event.local_name.as_deref() == Some("noscript"))
    });
    if document.mode != HtmlDocumentMode::Document
        || document.recovery_count != 0
        || document.encoding_report.normalized_encoding != "utf-8"
        || document
            .encoding_report
            .meta_charset
            .as_deref()
            .is_some_and(|encoding| {
                !encoding.trim().eq_ignore_ascii_case("utf-8")
                    && !encoding.trim().eq_ignore_ascii_case("utf8")
            })
        || unsupported_event.is_some()
    {
        return Err(error(
            "cem.ssr.page_profile_unsupported",
            "SSR URL preflight requires a UTF-8 HTML document without parser recovery, self-closing non-void HTML or noscript content",
            unsupported_event.or(document.events.first())
                .and_then(|event| document.source_map_for_range(event.source_range)),
        ));
    }
    let mut route = SsrPageRoute {
        request,
        document_base_url: page_url.clone(),
        page_url,
        output_uri,
        base_selection: SsrBaseSelection::PageUrl,
        base_source_map: request.page_url_source_map.cloned(),
    };
    let selected = document_elements(request.document).find_map(|event| {
        (event.local_name.as_deref() == Some("base"))
            .then(|| attribute(event, "href").map(|attr| (event, attr)))
            .flatten()
    });
    if let Some((event, attr)) = selected {
        let source_map = request.document.source_map_for_range(attr.source_range);
        let base = route
            .page_url
            .join(attr.value.as_deref().unwrap_or(""))
            .map_err(|_| {
                error(
                    "cem.ssr.base_url_invalid",
                    "The first document base href is invalid; later bases cannot replace it",
                    source_map.clone(),
                )
            })?;
        if has_credentials(&base) {
            return Err(error(
                "cem.ssr.base_url_invalid",
                "SSR document base must not contain credentials",
                source_map,
            ));
        }
        let always_blocked = matches!(base.scheme(), "data" | "javascript");
        if !always_blocked
            && request.base_policy == SsrBasePolicy::Unspecified
            && document_elements(request.document).any(|event| {
                event.local_name.as_deref() == Some("meta")
                    && attribute(event, "http-equiv")
                        .and_then(|attr| attr.value.as_deref())
                        .is_some_and(|value| value.eq_ignore_ascii_case("content-security-policy"))
            })
        {
            return Err(error(
                "cem.ssr.base_policy_required",
                "A CSP-dependent document base requires an explicit host base-policy decision",
                source_map,
            ));
        }
        let used_fallback = always_blocked || request.base_policy == SsrBasePolicy::Blocked;
        if !used_fallback {
            route.document_base_url = base;
        }
        route.base_selection = SsrBaseSelection::HtmlBase {
            event_index: event.index,
            used_fallback,
        };
        route.base_source_map = source_map;
    }
    Ok(route)
}

#[cfg(test)]
#[path = "ssr_route_tests.rs"]
mod tests;
