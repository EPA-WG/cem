//! Explicit source profiles for the native SSR page projection.
use crate::{
    content_cache::ContentHash,
    dependency_identity::{DependencyVersion, SourceDependency},
    diagnostics::{Diagnostic, Severity},
    engine::{EngineContext, EngineInput, FormatIdentity, InputFormat},
    lifecycle::{LifecycleRegistry, LoadedInput, LoadedInputAstStream},
    projection::{cem_tree_nodes_with_lexical_names, CemTreeAstAttribute, CemTreeAstNode},
    real::prepare_cem_ssr_page,
    schema::registry::{
        content_type_essence, CEM_ML_CONTENT_TYPE, HTML_NAMESPACE_URI, MARKDOWN_SCHEMA_URI,
    },
    source::{ByteRange, SourceId},
    source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind},
    ssr_route::{
        SSR_CEM_PAGE_CONTENT_TYPE, SSR_CEM_PAGE_SCHEMA, SSR_MARKDOWN_PAGE_CONTENT_TYPE,
        SSR_MARKDOWN_PAGE_SCHEMA,
    },
    validation::{
        html::ssr_projection,
        markdown::{content_type_parameter, MarkdownEventAst},
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthoredPageSource {
    pub uri: String,
    pub byte_length: usize,
    pub dependency: SourceDependency,
    pub origin: SourceMapStack,
}

enum Profile {
    Cem,
    Markdown,
}

impl Profile {
    fn select(identity: &FormatIdentity) -> Result<Option<Self>, String> {
        let media = identity.content_type.as_deref().map(content_type_essence);
        let variant = identity
            .content_type
            .as_deref()
            .and_then(|value| content_type_parameter(value, "variant"));
        match (media.as_deref(), identity.schema.as_deref()) {
            (Some(SSR_CEM_PAGE_CONTENT_TYPE), None | Some(SSR_CEM_PAGE_SCHEMA))
            | (None, Some(SSR_CEM_PAGE_SCHEMA)) => Ok(Some(Self::Cem)),
            (Some("text/markdown"), None | Some(SSR_MARKDOWN_PAGE_SCHEMA))
                if variant
                    .as_deref()
                    .is_some_and(|value| value.eq_ignore_ascii_case("cem")) =>
            {
                Ok(Some(Self::Markdown))
            }
            (None, Some(SSR_MARKDOWN_PAGE_SCHEMA)) => Ok(Some(Self::Markdown)),
            (Some(SSR_CEM_PAGE_CONTENT_TYPE), _)
            | (_, Some(SSR_CEM_PAGE_SCHEMA | SSR_MARKDOWN_PAGE_SCHEMA)) => {
                Err("SSR page content type and schema select different source profiles".into())
            }
            (Some("text/markdown"), _) => {
                Err("SSR Markdown requires the opted-in CEM page profile".into())
            }
            _ => Ok(None),
        }
    }

    fn identity(&self) -> (&'static str, &'static str) {
        match self {
            Self::Cem => (SSR_CEM_PAGE_CONTENT_TYPE, SSR_CEM_PAGE_SCHEMA),
            Self::Markdown => (SSR_MARKDOWN_PAGE_CONTENT_TYPE, SSR_MARKDOWN_PAGE_SCHEMA),
        }
    }
}

pub(crate) fn load_page(
    input: &EngineInput,
    context: &EngineContext,
    max_bytes: usize,
    admit_base: bool,
) -> Result<LoadedInput, String> {
    let registry = LifecycleRegistry::with_builtin_adapters();
    let identity = input
        .identity
        .clone()
        .or_else(|| input.root_scope.format_identity_option())
        .unwrap_or_else(|| FormatIdentity::from(context));
    let Some(profile) = Profile::select(&identity)? else {
        return Ok(if admit_base {
            registry.load_ssr_page(input, context)
        } else {
            registry.load(input, context)
        });
    };
    if identity
        .content_type
        .as_deref()
        .and_then(|media| content_type_parameter(media, "charset"))
        .is_some_and(|charset| !matches!(charset.to_ascii_lowercase().as_str(), "utf-8" | "utf8"))
    {
        return Err("SSR page profiles require UTF-8 source".into());
    }
    if !admit_base {
        return Err("An SSR page profile requires an import dedicated to SSR".into());
    }
    let (content_type, schema) = profile.identity();
    let origin = SourceMapStack {
        frames: vec![SourceMapFrame {
            source_id: SourceId(1),
            span: FrameSpan::Single(ByteRange::new(
                0,
                u32::try_from(input.bytes.len()).map_err(|_| "SSR source exceeds u32")?,
            )),
            transform: TransformKind::ContentTypeTransform {
                content_type: content_type.into(),
            },
        }],
    }
    .with_source_uri(&input.uri);
    let authored = AuthoredPageSource {
        uri: input.uri.clone(),
        byte_length: input.bytes.len(),
        dependency: SourceDependency {
            content_type: content_type.into(),
            schema: Some(schema.into()),
            version: DependencyVersion::Content(ContentHash::from_blake3(&input.bytes)),
        },
        origin,
    };
    let mut diagnostics = Vec::new();
    let nodes = match profile {
        Profile::Cem => cem_nodes(&input.bytes, input, context, &input.uri, &mut diagnostics)
            .and_then(|nodes| validate_page_root(&nodes).map(|_| nodes)),
        Profile::Markdown => markdown_page(input, context, &registry, &mut diagnostics),
    };
    let (document, mut projection_diagnostics) = match nodes {
        Ok(nodes) => ssr_projection::project(&nodes, authored, &input.bytes, max_bytes)?,
        Err(error) => {
            if !diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity.is_hard_violation())
            {
                diagnostics.push(Diagnostic {
                    uri: Some(input.uri.clone()),
                    code: "cem.ssr.page_projection".into(),
                    severity: Severity::Error,
                    message: error,
                    ..Diagnostic::default()
                });
            }
            (None, vec![])
        }
    };
    diagnostics.append(&mut projection_diagnostics);
    Ok(LoadedInput {
        bytes: input.bytes.clone(),
        from_format: InputFormat::Html,
        ast_stream: document.map(LoadedInputAstStream::HtmlDocument),
        diagnostics,
        adapter_id: Some("html"),
    })
}

fn cem_nodes(
    bytes: &[u8],
    input: &EngineInput,
    context: &EngineContext,
    uri: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Vec<CemTreeAstNode>, String> {
    std::str::from_utf8(bytes).map_err(|_| "CEM page source must be UTF-8")?;
    let mut scope = input.root_scope.clone();
    if scope.module_map.is_some() {
        return Err(
            "SSR page conversion requires inline declarations and a local source profile".into(),
        );
    }
    scope.schema = None;
    scope.default_content_type = Some(CEM_ML_CONTENT_TYPE.into());
    scope
        .default_namespace
        .get_or_insert_with(|| HTML_NAMESPACE_URI.into());
    let run = prepare_cem_ssr_page(bytes, &scope, context, uri);
    let hard = run
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation());
    diagnostics.extend(run.diagnostics);
    if hard {
        return Err("CEM page source contains invalid syntax or schema diagnostics".into());
    }
    check_cem_page_tree(&run.document)?;
    let lexical = run
        .lexical_scopes
        .as_ref()
        .ok_or("Missing native CEM source bindings")?;
    let mut stream = cem_tree_nodes_with_lexical_names(lexical, CEM_ML_CONTENT_TYPE)
        .with_original_source_uri(uri);
    stream.retain_non_directive_nodes();
    Ok(stream.as_nodes().to_vec())
}

// Bound the flat arena before the shared recursive writer projection allocates.
fn check_cem_page_tree(document: &crate::parser::document::CemDocument) -> Result<(), String> {
    use crate::parser::CemAstNode;

    if document.nodes.len() > crate::import::MAX_DOCUMENT_VALUES {
        return Err("CEM page exceeds the native document value limit".into());
    }
    let mut pending = vec![(0, 0)];
    while let Some((id, depth)) = pending.pop() {
        if depth > 64 {
            return Err("CEM page exceeds the native document nesting limit".into());
        }
        match document.get(id) {
            Some(CemAstNode::Document { root_children, .. }) => {
                pending.extend(root_children.iter().map(|child| (*child, depth)));
            }
            Some(CemAstNode::Element {
                attributes,
                children,
                ..
            }) => {
                pending.extend(
                    attributes
                        .iter()
                        .chain(children)
                        .map(|child| (*child, depth + 1)),
                );
            }
            Some(CemAstNode::Attribute { value_nodes, .. }) => {
                pending.extend(value_nodes.iter().map(|child| (*child, depth + 1)));
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_page_root(nodes: &[CemTreeAstNode]) -> Result<(), String> {
    fn structural(nodes: &[CemTreeAstNode]) -> impl Iterator<Item = &CemTreeAstNode> {
        nodes.iter().filter(|node| {
            !matches!(
                node,
                CemTreeAstNode::Comment { .. } | CemTreeAstNode::Whitespace { .. }
            ) && !matches!(node, CemTreeAstNode::Text { value, .. } if value.trim().is_empty())
        })
    }
    fn html_element<'a>(node: &'a CemTreeAstNode, local: &str) -> Option<&'a [CemTreeAstNode]> {
        let CemTreeAstNode::Element { name, children, .. } = node else {
            return None;
        };
        (name == local
            || name
                .strip_prefix(HTML_NAMESPACE_URI)
                .and_then(|s| s.strip_prefix(':'))
                == Some(local))
        .then_some(children)
    }
    let mut roots = structural(nodes);
    if let Some(children) = roots.next().and_then(|node| html_element(node, "html")) {
        let mut parts = structural(children);
        if roots.next().is_none()
            && parts
                .next()
                .and_then(|node| html_element(node, "head"))
                .is_some()
            && parts
                .next()
                .and_then(|node| html_element(node, "body"))
                .is_some()
            && parts.next().is_none()
        {
            return Ok(());
        }
    }
    Err("The CEM page profile requires one HTML document with head followed by body".into())
}

fn markdown_page(
    input: &EngineInput,
    context: &EngineContext,
    registry: &LifecycleRegistry,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Vec<CemTreeAstNode>, String> {
    let mut markdown = input.clone();
    let content_type = "text/markdown; charset=utf-8; variant=GFM".to_owned();
    markdown.root_scope.default_content_type = Some(content_type.clone());
    markdown.root_scope.schema = Some(MARKDOWN_SCHEMA_URI.into());
    markdown.identity = Some(FormatIdentity {
        content_type: Some(content_type),
        schema: Some(MARKDOWN_SCHEMA_URI.into()),
        ..FormatIdentity::default()
    });
    let loaded = registry.load(&markdown, context);
    let hard = loaded
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation());
    diagnostics.extend(loaded.diagnostics);
    let Some(LoadedInputAstStream::MarkdownDocument(document)) = loaded.ast_stream else {
        return Err("SSR Markdown requires a native Markdown event stream".into());
    };
    if hard {
        return Err("SSR Markdown source did not pass ordinary Markdown validation".into());
    }
    let mut lowerer = MarkdownPage {
        input,
        context,
        events: document.events.iter(),
        diagnostics,
    };
    let body = lowerer.children(None, 0)?;
    let synthetic = SourceMapStack::default();
    Ok(vec![element(
        "html",
        vec![],
        vec![
            element(
                "head",
                vec![],
                vec![element(
                    "meta",
                    vec![attribute("charset", "utf-8", &synthetic)],
                    vec![],
                    synthetic.clone(),
                )],
                synthetic.clone(),
            ),
            element("body", vec![], body, synthetic.clone()),
        ],
        synthetic,
    )])
}

struct MarkdownPage<'a> {
    input: &'a EngineInput,
    context: &'a EngineContext,
    events: std::slice::Iter<'a, MarkdownEventAst>,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl MarkdownPage<'_> {
    fn children(
        &mut self,
        parent: Option<&str>,
        depth: usize,
    ) -> Result<Vec<CemTreeAstNode>, String> {
        if depth > 60 {
            return Err("SSR Markdown exceeds the page nesting limit".into());
        }
        let mut nodes = Vec::new();
        while let Some(event) = self.events.next() {
            let origin = event
                .source_range
                .source_map()
                .with_source_uri(&self.input.uri);
            let tag = event.tag.as_deref().unwrap_or_default();
            let value = event.text.as_deref().unwrap_or_default();
            let node = match event.kind.as_str() {
                "end" if Some(tag) == parent => return Ok(nodes),
                "start" if tag == "code-block" => {
                    nodes.extend(self.code_block(event, origin)?);
                    continue;
                }
                "start" => self.container(event, parent, depth, origin)?,
                "text" | "soft-break" => text(
                    if event.kind == "soft-break" {
                        "\n"
                    } else {
                        value
                    },
                    origin,
                ),
                "code" => element("code", vec![], vec![text(value, origin.clone())], origin),
                "hard-break" => element("br", vec![], vec![], origin),
                "thematic-break" => element("hr", vec![], vec![], origin),
                "task-list-marker" => {
                    let mut attributes = vec![
                        attribute("type", "checkbox", &origin),
                        attribute("disabled", "", &origin),
                    ];
                    if event.checked == Some(true) {
                        attributes.push(attribute("checked", "", &origin));
                    }
                    element("input", attributes, vec![], origin)
                }
                kind => return Err(format!("Unsupported SSR Markdown event `{kind}`")),
            };
            nodes.push(node);
        }
        if parent.is_some() {
            return Err("Unbalanced Markdown page events".into());
        }
        Ok(nodes)
    }

    fn container(
        &mut self,
        event: &MarkdownEventAst,
        parent: Option<&str>,
        depth: usize,
        origin: SourceMapStack,
    ) -> Result<CemTreeAstNode, String> {
        let tag = event.tag.as_deref().unwrap_or_default();
        let mut children = self.children(Some(tag), depth + 1)?;
        let mut attrs = Vec::new();
        let name = match tag {
            "paragraph" => "p",
            "heading" => ["h1", "h2", "h3", "h4", "h5", "h6"]
                [(event.level.unwrap_or(1).clamp(1, 6) - 1) as usize],
            "blockquote" => "blockquote",
            "list" => "ul",
            "ordered-list" => {
                if let Some(start) = event.ordered_start {
                    attrs.push(attribute("start", &start.to_string(), &origin));
                }
                "ol"
            }
            "list-item" => "li",
            "emphasis" => "em",
            "strong" => "strong",
            "strikethrough" => "del",
            "link" | "image" => {
                let destination = event.destination.as_deref().unwrap_or_default();
                if let Some(title) = &event.title {
                    attrs.push(attribute("title", title, &origin));
                }
                if tag == "link" {
                    attrs.push(attribute("href", destination, &origin));
                    "a"
                } else {
                    attrs.push(attribute("src", destination, &origin));
                    let mut alt = String::new();
                    collect_text(&children, &mut alt);
                    attrs.push(attribute("alt", &alt, &origin));
                    children.clear();
                    "img"
                }
            }
            "table" => {
                let start =
                    usize::from(children.first().and_then(CemTreeAstNode::name) == Some("thead"));
                if children.len() > start {
                    let rows = children.split_off(start);
                    children.push(element("tbody", vec![], rows, SourceMapStack::default()));
                }
                "table"
            }
            "table-head" => {
                children = vec![element("tr", vec![], children, origin.clone())];
                "thead"
            }
            "table-row" => "tr",
            "table-cell" if parent == Some("table-head") => "th",
            "table-cell" => "td",
            _ => return Err(format!("Unsupported SSR Markdown container `{tag}`")),
        };
        Ok(element(name, attrs, children, origin))
    }

    fn code_block(
        &mut self,
        event: &MarkdownEventAst,
        origin: SourceMapStack,
    ) -> Result<Vec<CemTreeAstNode>, String> {
        let info = event.info.as_deref().unwrap_or_default();
        let mut code = String::new();
        let mut segments = Vec::new();
        for part in self.events.by_ref() {
            if part.kind == "end" && part.tag.as_deref() == Some("code-block") {
                let language = info.split_whitespace().next().unwrap_or_default();
                if info.trim() == "cem-ml" {
                    return self.fence(&code, &segments, origin);
                }
                let attrs = if language.is_empty() || language == "indented" {
                    vec![]
                } else {
                    vec![attribute("class", &format!("language-{language}"), &origin)]
                };
                return Ok(vec![element(
                    "pre",
                    vec![],
                    vec![element(
                        "code",
                        attrs,
                        vec![text(&code, origin.clone())],
                        origin.clone(),
                    )],
                    origin,
                )]);
            }
            if let Some(value) = &part.text {
                let range = ByteRange::new(code.len() as u64, value.len() as u32);
                segments.push((
                    range,
                    part.source_range.start.byte_offset,
                    part.source_range.byte_length,
                ));
                code.push_str(value);
            }
        }
        Err("Unclosed Markdown code block".into())
    }

    fn fence(
        &mut self,
        code: &str,
        segments: &[(ByteRange, u64, u64)],
        origin: SourceMapStack,
    ) -> Result<Vec<CemTreeAstNode>, String> {
        let offset = origin
            .origin()
            .and_then(|frame| single_range(&frame.span))
            .map_or(0, |range| range.start);
        let uri = format!("{}#cem-ml-fence-{offset}", self.input.uri);
        let mut local = Vec::new();
        let mut nodes = cem_nodes(code.as_bytes(), self.input, self.context, &uri, &mut local);
        for diagnostic in &mut local {
            let child = diagnostic
                .source_map
                .take()
                .unwrap_or_default()
                .with_source_uri(&uri);
            diagnostic.source_map = Some(fence_origin(child, segments, &origin, &self.input.uri));
            diagnostic.uri = Some(self.input.uri.clone());
            diagnostic.byte_offset = None;
            diagnostic.line = None;
            diagnostic.column = None;
            if let Some(details) = diagnostic
                .details
                .as_mut()
                .and_then(serde_json::Value::as_object_mut)
            {
                details.remove("coordinates");
                details.remove("sourceMapCoordinates");
            }
        }
        self.diagnostics.extend(local);
        if let Ok(nodes) = &mut nodes {
            for node in nodes {
                map_node(node, &|source| {
                    fence_origin(source, segments, &origin, &self.input.uri)
                });
            }
        }
        nodes
    }
}

fn collect_text(nodes: &[CemTreeAstNode], value: &mut String) {
    for node in nodes {
        if let CemTreeAstNode::Text { value: text, .. } = node {
            value.push_str(text);
        }
        collect_text(node.children(), value);
    }
}

fn map_node(node: &mut CemTreeAstNode, map: &impl Fn(SourceMapStack) -> SourceMapStack) {
    let source = match node {
        CemTreeAstNode::Element {
            attributes,
            children,
            source,
            ..
        } => {
            for attr in attributes {
                attr.source = map(std::mem::take(&mut attr.source));
            }
            for child in children {
                map_node(child, map);
            }
            source
        }
        CemTreeAstNode::Text { source, .. }
        | CemTreeAstNode::Whitespace { source, .. }
        | CemTreeAstNode::RawText { source, .. }
        | CemTreeAstNode::Comment { source, .. } => source,
        _ => return,
    };
    *source = map(std::mem::take(source));
}

fn fence_origin(
    mut child: SourceMapStack,
    segments: &[(ByteRange, u64, u64)],
    fence: &SourceMapStack,
    uri: &str,
) -> SourceMapStack {
    let mut mapped = fence.clone();
    let child_range = child.current().and_then(|frame| single_range(&frame.span));
    if let Some(range) = child_range {
        let ranges = segments
            .iter()
            .filter_map(|(decoded, start, length)| {
                let from = decoded.start.max(range.start);
                let to = decoded.end().min(range.end());
                if from >= to {
                    return None;
                }
                if u64::from(decoded.len) == *length {
                    Some(ByteRange::new(
                        start + from - decoded.start,
                        (to - from) as u32,
                    ))
                } else {
                    Some(ByteRange::new(*start, *length as u32))
                }
            })
            .collect::<Vec<_>>();
        if !ranges.is_empty() {
            mapped.push(SourceMapFrame {
                source_id: SourceId(1),
                span: FrameSpan::Multi(ranges),
                transform: TransformKind::SourceBoundary { uri: uri.into() },
            });
        }
    }
    mapped.frames.append(&mut child.frames);
    mapped
}

fn single_range(span: &FrameSpan) -> Option<ByteRange> {
    match span {
        FrameSpan::Single(range) => Some(*range),
        FrameSpan::Multi(_) => None,
    }
}

fn element(
    name: &str,
    attributes: Vec<CemTreeAstAttribute>,
    children: Vec<CemTreeAstNode>,
    source: SourceMapStack,
) -> CemTreeAstNode {
    CemTreeAstNode::Element {
        name: name.into(),
        attributes,
        children,
        source,
    }
}
fn text(value: &str, source: SourceMapStack) -> CemTreeAstNode {
    CemTreeAstNode::Text {
        value: value.into(),
        source,
    }
}
fn attribute(name: &str, value: &str, source: &SourceMapStack) -> CemTreeAstAttribute {
    CemTreeAstAttribute {
        name: name.into(),
        value: Some(value.into()),
        value_nodes: vec![],
        source: source.clone(),
    }
}
