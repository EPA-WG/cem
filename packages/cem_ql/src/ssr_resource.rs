//! Public URL preparation for bounded host-owned SSR reads.
use cem_ml::{
    operation_control::MemoryPermit,
    resolver::{ssr_resource_url, ResolveDirection, ResolvePurpose, ResolveRequest, ResolvedRead},
    source_map::SourceMapStack,
    ssr_artifact::CemSsrPageArtifact,
};

/// The host applies resolver policy and bounds acquisition before allocation.
/// The permit charges the response buffer until its native import takes over.
pub type SsrResourceReader<'a> = dyn FnMut(
        &ResolveRequest,
        usize,
        Option<&SourceMapStack>,
    ) -> Result<(ResolvedRead, MemoryPermit), String>
    + 'a;

pub(crate) fn public_read_request(
    page: &CemSsrPageArtifact,
    src: &str,
    fragment: Option<&str>,
    content_type: &str,
) -> Option<ResolveRequest> {
    if src.len() > 4096
        || !(src.starts_with("./")
            || src.starts_with("../")
            || src.starts_with('/')
            || src.starts_with("https://")
            || src.starts_with("http://"))
        || src.trim() != src
        || src.contains('\\')
        || src.chars().any(char::is_control)
    {
        return None;
    }
    let base = url::Url::parse(&page.route().document_base_url).ok()?;
    let url = base.join(src).ok()?;
    if url.fragment() != fragment || url.query().is_some() {
        return None;
    }
    let uri = ssr_resource_url(url.as_str()).ok()?;
    if uri == page.route().page_url || uri.len() > 4096 {
        return None;
    }
    Some(
        ResolveRequest::new(&uri, ResolvePurpose::SsrResource, ResolveDirection::Read)
            .with_base_uri(base.as_str())
            .with_content_type_hint(content_type),
    )
}
