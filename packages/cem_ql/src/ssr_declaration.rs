//! Bounded admission of a canonical external declaration and its source owner.
use super::{attr, html, ssr_resource::public_read_request, ResourceInput, SsrResourceReader};
use crate::ssr_preparation::{check, compile_inline_definition, problem, InlineDefinition};
use cem_ml::{
    content_cache::ContentHash,
    diagnostics::Diagnostic,
    import::element_tree::retain_html_elements,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::{
        tree::{CemDocumentMetadata, CemTreeNodeKind, RetainedCemTree},
        AstNodeId,
    },
    schema::{
        document_model::is_canonical_semver,
        registry::{content_type_essence, HTML_NAMESPACE_URI},
    },
    ssr_artifact::CemSsrPageArtifact,
    validation::html::{
        html_document_ast_from_source_bytes, HtmlDocumentMode, HtmlSourceValidationRequest,
    },
};
use std::sync::Arc;

const MAX_DECLARATION_BYTES: usize = 64 * 1024;

pub(crate) struct ExternalDeclarationLease {
    pub(super) input: ResourceInput,
    pub(crate) tree: Arc<RetainedCemTree>,
    pub(super) declaration: AstNodeId,
    pub(super) control: OperationControl,
}

impl ExternalDeclarationLease {
    pub(super) fn uri(&self) -> &str {
        &self.input.resolved_uri
    }

    pub(super) fn bytes(&self) -> usize {
        self.input.declaration_bytes.unwrap()
    }
}

pub(super) fn read_definition(
    page: &CemSsrPageArtifact,
    stub_tree: &Arc<RetainedCemTree>,
    stub: AstNodeId,
    src: &str,
    read: &mut SsrResourceReader<'_>,
    remaining: u64,
    control: &OperationControl,
) -> Result<InlineDefinition, Vec<Diagnostic>> {
    let fail = |code, message: &str| problem(page, Some(stub_tree), Some(stub), code, message);
    let invalid = || {
        fail(
            "cem.ssr.external_declaration_unsupported",
            "Expected an empty canonical declaration with a tag, version and matching URL fragment",
        )
    };
    let tag = attr(stub_tree, stub, "tag")
        .filter(|tag| crate::declaration_artifact::valid_tag(tag))
        .ok_or_else(invalid)?;
    let version = attr(stub_tree, stub, "version")
        .filter(|version| version.len() <= 1024 && is_canonical_semver(version))
        .ok_or_else(invalid)?;
    if meaningful_children(stub_tree, stub).next().is_some()
        || attr(stub_tree, stub, "xmlns").is_some_and(|namespace| namespace != HTML_NAMESPACE_URI)
    {
        return Err(invalid());
    }
    let request = public_read_request(page, src, Some(&tag), "text/html").ok_or_else(invalid)?;
    let uri = &request.uri;
    let limit = remaining.min(MAX_DECLARATION_BYTES as u64) as usize;
    if limit == 0 {
        return Err(fail(
            "cem.ssr.settle_budget",
            "External declaration byte budget exhausted",
        ));
    }
    let source_map = stub_tree.node_source_map(stub);
    let (response, _buffer) = read(&request, limit, source_map.as_ref())
        .map_err(|error| fail("cem.ssr.resource_read", &error))?;
    check(page, control)?;
    if response.bytes.len() > limit {
        return Err(fail(
            "cem.ssr.settle_budget",
            "External declaration exceeds its read budget",
        ));
    }
    if &response.uri != uri
        || response
            .content_type
            .as_deref()
            .map(content_type_essence)
            .as_deref()
            != Some("text/html")
    {
        return Err(fail(
            "cem.ssr.external_declaration_response",
            "External declaration requires its exact public URL and a text/html response",
        ));
    }

    let workspace = control
        .charge_memory(
            ROOT_EXECUTION_SCOPE_ID,
            (response
                .bytes
                .len()
                .saturating_mul(128)
                .saturating_add(1024)) as u64,
            source_map,
        )
        .map_err(|_| {
            fail(
                "cem.ssr.page_bounds",
                "External declaration workspace refused",
            )
        })?;
    let (document, diagnostics) =
        html_document_ast_from_source_bytes(HtmlSourceValidationRequest {
            bytes: &response.bytes,
            source_uri: uri,
            content_type: response.content_type.as_deref(),
        });
    let mut errors: Vec<_> = diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.severity.is_hard_violation())
        .collect();
    if !errors.is_empty() {
        errors.splice(
            0..0,
            fail(
                "cem.ssr.external_declaration_response",
                "External declaration HTML was refused",
            ),
        );
        return Err(errors);
    }
    let document = document
        .filter(|document| {
            document.mode == HtmlDocumentMode::Fragment && document.recovery_count == 0
        })
        .ok_or_else(|| {
            fail(
                "cem.ssr.external_declaration_response",
                "External declaration requires a standalone HTML fragment without parser recovery",
            )
        })?;
    let elements = retain_html_elements(
        Arc::new(document),
        CemDocumentMetadata {
            document_uri: Some(uri.clone()),
            base_uri: Some(uri.clone()),
        },
    )
    .map_err(|error| fail("cem.ssr.external_declaration_response", &error))?;
    elements
        .validate_html_text_projection()
        .map_err(|error| fail("cem.ssr.external_declaration_response", &error))?;
    elements
        .tree
        .retain_native_resource(Arc::new(workspace))
        .map_err(|_| {
            fail(
                "cem.ssr.page_bounds",
                "External declaration owner retention refused",
            )
        })?;

    let mut roots = meaningful_children(&elements.tree, 0);
    let declaration = roots
        .next()
        .filter(|id| html(&elements.tree, *id, "cem-element"))
        .filter(|_| roots.next().is_none())
        .ok_or_else(|| {
            fail(
                "cem.ssr.external_declaration_response",
                "Expected a single external cem-element declaration",
            )
        })?;
    let mut definition = compile_inline_definition(page, &elements, declaration, control, false)?;
    if definition.tag != tag || definition.version.as_deref() != Some(&version) {
        return Err(problem(
            page,
            Some(&elements.tree),
            Some(declaration),
            "cem.ssr.external_declaration_identity",
            "External declaration tag and version must match the page registry",
        ));
    }

    definition.external = Some(Arc::new(ExternalDeclarationLease {
        input: ResourceInput {
            resolved_uri: uri.clone(),
            content: ContentHash::from_blake3(&response.bytes),
            declaration_bytes: Some(response.bytes.len()),
            payload_bytes: None,
        },
        tree: elements.tree.clone(),
        declaration,
        control: control.clone(),
    }));
    definition.source = stub_tree.clone();
    definition.source_declaration = stub;
    definition.declaration = stub;
    definition.stylesheet_coordination = true;
    Ok(definition)
}

fn meaningful_children(
    tree: &RetainedCemTree,
    id: AstNodeId,
) -> impl Iterator<Item = AstNodeId> + '_ {
    tree.node(id)
        .unwrap()
        .children
        .iter()
        .copied()
        .filter(|id| {
            let child = tree.node(*id).unwrap();
            child.kind != CemTreeNodeKind::Comment
                && !(child.kind == CemTreeNodeKind::Text && child.value.trim().is_empty())
        })
}
