//! Host-neutral first-connection preparation, without island/render publication.
use crate::retained_template::RetainedTemplate;
use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    import::element_tree::{retain_html_elements, RetainedElementImport},
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::{
        tree::{CemDocumentMetadata, CemTreeNodeKind, RetainedCemTree},
        AstNodeId,
    },
    schema::registry::HTML_NAMESPACE_URI,
    source_map::TransformKind,
    ssr_artifact::{CemSsrPageArtifact, SsrLifecycleState, SsrPartialReason},
    transform_artifact::TransformArtifactBody,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub const MAX_INLINE_TEMPLATE_BYTES: usize = 32_768;

pub struct PreparedInlineDce {
    page: CemSsrPageArtifact,
    elements: RetainedElementImport,
    declaration: AstNodeId,
    template: AstNodeId,
    instance: AstNodeId,
    tag: String,
    retained: Arc<RetainedTemplate>,
    pub(crate) registered_participants: Option<Arc<BTreeSet<String>>>,
    pub(crate) lifecycle_control: Option<OperationControl>,
    pub(crate) payload_sources:
        Option<Arc<cem_ml::import::element_tree::RetainedCemPayloadSourcePool>>,
    pub(crate) stylesheet_coordination: bool,
    pub(crate) stylesheet_control: Option<OperationControl>,
    pub(crate) shared_scope: Option<String>,
}
impl std::fmt::Debug for PreparedInlineDce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedInlineDce")
            .field("tag", &self.tag)
            .field("instance", &self.instance)
            .finish_non_exhaustive()
    }
}
impl PreparedInlineDce {
    pub fn page(&self) -> &CemSsrPageArtifact {
        &self.page
    }
    pub fn elements(&self) -> &RetainedElementImport {
        &self.elements
    }
    pub fn declaration_node(&self) -> AstNodeId {
        self.declaration
    }
    pub fn template_node(&self) -> AstNodeId {
        self.template
    }
    pub fn instance_node(&self) -> AstNodeId {
        self.instance
    }
    pub fn tag(&self) -> &str {
        &self.tag
    }
    pub(crate) fn declaration_lease_identity(&self) -> &Arc<RetainedTemplate> {
        &self.retained
    }
    pub fn declaration_owner(&self) -> &RetainedTemplate {
        &self.retained
    }
}

pub fn prepare_inline_dce(
    page: CemSsrPageArtifact,
    control: &OperationControl,
) -> Result<PreparedInlineDce, Vec<Diagnostic>> {
    prepare_inline_dce_mode(page, control, false)
}
/// Compile scoped CSS sources without granting resource loading or publication.
pub fn prepare_inline_dce_with_stylesheets(
    page: CemSsrPageArtifact,
    control: &OperationControl,
) -> Result<PreparedInlineDce, Vec<Diagnostic>> {
    prepare_inline_dce_mode(page, control, true)
}
fn prepare_inline_dce_mode(
    page: CemSsrPageArtifact,
    control: &OperationControl,
    stylesheet_coordination: bool,
) -> Result<PreparedInlineDce, Vec<Diagnostic>> {
    check(&page, control)?;
    if page.lifecycle() != SsrLifecycleState::Unrendered {
        return Err(problem(
            &page,
            None,
            None,
            "cem.ssr.preparation_state",
            "Inline preparation requires an unrendered first-connection page",
        ));
    }
    if !matches!(page.document(), TransformArtifactBody::HtmlDomProjection(_)) {
        return Err(problem(
            &page,
            None,
            None,
            "cem.ssr.preparation_source_unsupported",
            "The first preparation profile requires its original native HTML projection",
        ));
    }
    let elements = retain_html_elements(
        page.source_document().clone(),
        CemDocumentMetadata {
            base_uri: Some(page.route().document_base_url.clone()),
            document_uri: Some(page.route().page_url.clone()),
        },
    )
    .map_err(|_| {
        problem(
            &page,
            None,
            None,
            "cem.ssr.page_import_refused",
            "Native HTML import policy refused the page",
        )
    })?;
    let tree = &elements.tree;
    let mut pending = vec![0];
    let mut declarations = Vec::new();
    let mut custom = Vec::new();
    while let Some(id) = pending.pop() {
        check(&page, control)?;
        let node = tree.node(id).expect("validated native CEM tree");
        if let Some(name) = &node.name {
            if name.namespace_uri == HTML_NAMESPACE_URI {
                if name.local_name == "template" {
                    continue;
                }
                if name.local_name == "cem-element" {
                    declarations.push(id);
                    continue;
                }
                if attributes(tree, id).contains_key("is") {
                    return Err(problem(
                        &page,
                        Some(tree),
                        Some(id),
                        "cem.ssr.participant_unavailable",
                        "Customized built-in participants are outside the autonomous inline profile",
                    ));
                }
                if name.local_name.contains('-') {
                    custom.push(id);
                }
            }
        }
        pending.extend(node.children.iter().rev().copied());
    }
    let [declaration] = declarations.as_slice() else {
        return Err(problem(
            &page,
            Some(tree),
            declarations.first().copied(),
            "cem.ssr.inline_declaration_count",
            "Exactly one active inline declaration is required",
        ));
    };
    let declaration = *declaration;
    let header = declaration_header(&page, &elements, declaration, stylesheet_coordination)?;
    let tag = header.tag;
    let template = header.template;
    let root = header.root;
    let instances: Vec<_> = custom
        .iter()
        .copied()
        .filter(|id| is_html(tree, *id, &tag))
        .collect();
    if let Some(id) = custom.iter().find(|id| !is_html(tree, **id, &tag)) {
        return Err(problem(
            &page,
            Some(tree),
            Some(*id),
            "cem.ssr.participant_unavailable",
            "An active custom element has no admitted inline participant",
        ));
    }
    let [instance] = instances.as_slice() else {
        return Err(problem(
            &page,
            Some(tree),
            instances.first().copied(),
            "cem.ssr.inline_instance_count",
            "Exactly one active instance is required",
        ));
    };
    let instance = *instance;
    if tree.node(instance).unwrap().children.iter().any(|id| {
        is_html(tree, *id, "template")
            && attributes(tree, *id)
                .get("data-cem-island")
                .map(String::as_str)
                == Some("instance")
    }) {
        return Err(problem(
            &page,
            Some(tree),
            Some(instance),
            "cem.ssr.resume_unavailable",
            "A marked island selects resume; first-connection preparation cannot recapture it",
        ));
    }
    let retained = compile_source(&page, &elements, template, stylesheet_coordination, control)?;
    check(&page, control)?;
    let mut usage = page.usage();
    usage.instances = 1;
    usage.declaration_depth = 1;
    let mut failure = problem(
        &page,
        Some(tree),
        Some(instance),
        "cem.ssr.settle_budget",
        "Preparation progress was refused by the artifact policy",
    );
    let page = page
        .into_partial(
            SsrPartialReason::Incomplete,
            usage,
            retained.artifact().diagnostics.clone(),
        )
        .map_err(|error| {
            failure[0].code = error.code().into();
            failure
        })?;
    Ok(PreparedInlineDce {
        page,
        elements,
        declaration,
        template,
        instance,
        tag,
        retained,
        registered_participants: None,
        lifecycle_control: None,
        payload_sources: None,
        stylesheet_coordination,
        stylesheet_control: stylesheet_coordination.then(|| control.clone()),
        shared_scope: root.get("scope").cloned(),
    })
}

struct DeclarationHeader {
    tag: String,
    template: AstNodeId,
    root: BTreeMap<String, String>,
}
fn declaration_header(
    page: &CemSsrPageArtifact,
    elements: &RetainedElementImport,
    declaration: AstNodeId,
    stylesheet_coordination: bool,
) -> Result<DeclarationHeader, Vec<Diagnostic>> {
    let tree = &elements.tree;
    let root = attributes(tree, declaration);
    let tag = root
        .get("tag")
        .filter(|tag| crate::declaration_artifact::valid_tag(tag))
        .cloned();
    if tag.is_none()
        || root.keys().any(|key| {
            !(["tag", "version", "xmlns"].contains(&key.as_str())
                || stylesheet_coordination && key == "scope")
        })
        || root
            .get("xmlns")
            .is_some_and(|value| value != HTML_NAMESPACE_URI)
        || root
            .get("version")
            .is_some_and(|value| value.trim().is_empty() || value.len() > 1024)
    {
        return Err(problem(
            page,
            Some(tree),
            Some(declaration),
            "cem.ssr.inline_declaration_unsupported",
            "Unsupported inline declaration header or capability",
        ));
    }
    let tag = tag.unwrap();
    let children: Vec<_> = tree
        .node(declaration)
        .unwrap()
        .children
        .iter()
        .copied()
        .filter(|id| {
            let child = tree.node(*id).unwrap();
            child.kind != CemTreeNodeKind::Comment
                && !(child.kind == CemTreeNodeKind::Text && child.value.trim().is_empty())
        })
        .collect();
    let template = children.first().copied();
    if children.len() != 1 || !template.is_some_and(|id| is_html(tree, id, "template")) {
        return Err(problem(
            page,
            Some(tree),
            Some(declaration),
            "cem.ssr.inline_template_unsupported",
            "Expected one direct canonical CEM source template",
        ));
    }
    let template = template.unwrap();
    let attrs = attributes(tree, template);
    if attrs.get("id") != Some(&tag)
        || attrs.get("type").map(String::as_str) != Some("text/cem-ml")
        || attrs
            .keys()
            .any(|key| !["id", "type"].contains(&key.as_str()))
    {
        return Err(problem(
            page,
            Some(tree),
            Some(template),
            "cem.ssr.inline_template_unsupported",
            "Unsupported template identity, type or source adapter",
        ));
    }
    Ok(DeclarationHeader {
        tag,
        template,
        root,
    })
}
fn compile_source(
    page: &CemSsrPageArtifact,
    elements: &RetainedElementImport,
    template: AstNodeId,
    stylesheet_coordination: bool,
    control: &OperationControl,
) -> Result<Arc<RetainedTemplate>, Vec<Diagnostic>> {
    let tree = &elements.tree;
    let source = elements.inline_cem_template_source(template).map_err(|_| {
        problem(
            page,
            Some(tree),
            Some(template),
            "cem.ssr.inline_template_unsupported",
            "The template does not expose a supported native CEM source handoff",
        )
    })?;
    if source.text().len() > MAX_INLINE_TEMPLATE_BYTES {
        return Err(problem(
            page,
            Some(tree),
            Some(template),
            "cem.ssr.inline_template_bounds",
            "Inline CEM source exceeds the preparation limit",
        ));
    }
    check(page, control)?;
    let options = crate::render::CompileTemplateOptions {
        host_bindings: crate::declaration_artifact::DECLARATION_HOST_BINDINGS
            .iter()
            .map(|name| (*name).into())
            .collect(),
        source_uri: Some(page.route().source_uri.clone()),
        ..Default::default()
    };
    let compiled =
        crate::render::compile_template_with_source_map(source.text(), &options, &|range| {
            source.source_map_for_range(range)
        });
    check(page, control)?;
    if compiled
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        let mut errors = problem(
            page,
            Some(tree),
            Some(template),
            "cem.ssr.inline_template_invalid",
            "Native CEM template compilation failed",
        );
        errors.extend(
            compiled
                .diagnostics
                .into_iter()
                .filter(|diagnostic| diagnostic.severity.is_hard_violation())
                .take(255),
        );
        return Err(errors);
    }
    if (!stylesheet_coordination && !compiled.stylesheets.is_empty())
        || compiled.module_map.is_some()
        || crate::declaration_artifact::has_external_template_imports(
            source.text(),
            &page.route().source_uri,
        )
    {
        return Err(problem(
            page,
            Some(tree),
            Some(template),
            "cem.ssr.inline_template_unsupported",
            "Stylesheets, module maps and external modules need a later admitted closure",
        ));
    }
    Ok(Arc::new(RetainedTemplate::new(compiled)))
}
#[derive(Clone)]
pub(crate) struct InlineDefinition {
    pub tag: String,
    pub declaration: AstNodeId,
    pub template: AstNodeId,
    pub retained: Arc<RetainedTemplate>,
    pub shared_scope: Option<String>,
    pub stylesheet_coordination: bool,
}
pub(crate) fn compile_inline_definition(
    page: &CemSsrPageArtifact,
    elements: &RetainedElementImport,
    declaration: AstNodeId,
    control: &OperationControl,
    stylesheet_coordination: bool,
) -> Result<InlineDefinition, Vec<Diagnostic>> {
    let header = declaration_header(page, elements, declaration, stylesheet_coordination)?;
    let retained = compile_source(
        page,
        elements,
        header.template,
        stylesheet_coordination,
        control,
    )?;
    Ok(InlineDefinition {
        tag: header.tag,
        declaration,
        template: header.template,
        retained,
        shared_scope: header.root.get("scope").cloned(),
        stylesheet_coordination,
    })
}
pub(crate) struct InlineParticipantSelection {
    pub elements: RetainedElementImport,
    pub definition: InlineDefinition,
    pub instance: AstNodeId,
    pub depth: u32,
    pub registry: Arc<BTreeSet<String>>,
    pub payload_sources: Option<Arc<cem_ml::import::element_tree::RetainedCemPayloadSourcePool>>,
}
pub(crate) fn prepare_inline_participant(
    page: CemSsrPageArtifact,
    selection: InlineParticipantSelection,
    control: &OperationControl,
) -> Result<PreparedInlineDce, Vec<Diagnostic>> {
    let InlineParticipantSelection {
        elements,
        definition,
        instance,
        depth,
        registry,
        payload_sources,
    } = selection;
    check(&page, control)?;
    let tree = &elements.tree;
    let fail = |code, message| problem(&page, Some(tree), Some(instance), code, message);
    if !page.accepts_source_tree(tree) || !is_html(tree, instance, &definition.tag) {
        return Err(fail(
            "cem.ssr.participant_source_stale",
            "Participant source is not the exact current native page",
        ));
    }
    if tree.node(instance).unwrap().children.iter().any(|id| {
        is_html(tree, *id, "template")
            && attributes(tree, *id)
                .get("data-cem-island")
                .is_some_and(|v| v == "instance")
    }) {
        return Err(fail(
            "cem.ssr.resume_unavailable",
            "A first-connection participant cannot recapture a marked island",
        ));
    }
    let mut usage = page.usage();
    usage.instances = usage
        .instances
        .checked_add(1)
        .ok_or_else(|| fail("cem.ssr.settle_budget", "Participant count overflow"))?;
    usage.declaration_depth = usage.declaration_depth.max(depth);
    let refused = fail(
        "cem.ssr.settle_budget",
        "Participant preparation exceeds the original page budget",
    );
    let page = page
        .into_partial(
            SsrPartialReason::Incomplete,
            usage,
            definition.retained.artifact().diagnostics.clone(),
        )
        .map_err(|_| refused)?;
    Ok(PreparedInlineDce {
        page,
        elements,
        declaration: definition.declaration,
        template: definition.template,
        instance,
        tag: definition.tag,
        retained: definition.retained,
        registered_participants: Some(registry),
        lifecycle_control: Some(control.clone()),
        payload_sources,
        stylesheet_coordination: definition.stylesheet_coordination,
        stylesheet_control: definition.stylesheet_coordination.then(|| control.clone()),
        shared_scope: definition.shared_scope,
    })
}

fn is_html(tree: &RetainedCemTree, node: AstNodeId, local: &str) -> bool {
    tree.node(node)
        .and_then(|node| node.name.as_ref())
        .is_some_and(|name| name.namespace_uri == HTML_NAMESPACE_URI && name.local_name == local)
}
fn attributes(tree: &RetainedCemTree, id: AstNodeId) -> BTreeMap<String, String> {
    tree.node(id)
        .unwrap()
        .attributes
        .iter()
        .filter_map(|id| {
            let attribute = tree.node(*id)?;
            let name = attribute.name.as_ref()?;
            Some((name.local_name.clone(), attribute.value.clone()))
        })
        .collect()
}
pub(crate) fn check(
    page: &CemSsrPageArtifact,
    control: &OperationControl,
) -> Result<(), Vec<Diagnostic>> {
    control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|error| {
            problem(
                page,
                None,
                None,
                error.code(),
                "The original operation control refused preparation",
            )
        })
}
pub(crate) fn problem(
    page: &CemSsrPageArtifact,
    tree: Option<&RetainedCemTree>,
    node: Option<AstNodeId>,
    code: &str,
    message: &str,
) -> Vec<Diagnostic> {
    let source_map = match (tree, node) {
        (Some(tree), Some(node)) => tree.node_source_map(node),
        _ => page.source_map().cloned(),
    }
    .filter(|map| !map.frames.is_empty());
    let uri = source_map
        .as_ref()
        .and_then(|map| {
            map.frames.iter().find_map(|frame| match &frame.transform {
                TransformKind::SourceBoundary { uri } => Some(uri.clone()),
                _ => None,
            })
        })
        .unwrap_or_else(|| page.route().source_uri.clone());
    vec![Diagnostic {
        uri: Some(uri),
        code: code.into(),
        severity: Severity::Error,
        message: message.into(),
        source_map,
        ..Diagnostic::default()
    }]
}
