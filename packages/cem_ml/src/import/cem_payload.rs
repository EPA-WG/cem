//! Source-preserving, non-executing CEM-ML payload admission.
use super::*;
use crate::{
    content_cache::ContentHash,
    events::cem::CemEventNormalizer,
    operation_control::{MemoryPermit, OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::tree::CemDocumentMetadata,
    schema::{machine::CemSchemaMachine, vocab::CompiledSchema},
    source::BytesSource,
    tokenizer::{
        cem::CemTokenizer, SchemaToken, SchemaTokenKind, SchemaTokenizer, TokenizerProfile,
    },
    value::artifact::CemValueArtifactLimits,
};
use std::sync::atomic::{AtomicBool, Ordering};
pub const MAX_CEM_PAYLOAD_SOURCE_BYTES: usize = 32_768;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalCemPayloadInput {
    pub uri: String,
    pub content: ContentHash,
    pub byte_length: usize,
}

/// The retained literal template remains authoritative; its parsed CEM tree is
/// an immutable native view. JSON records cannot supply source authority.
pub struct RetainedCemPayloadSource {
    source_tree: Arc<RetainedCemTree>,
    template: AstNodeId,
    tree: Arc<RetainedCemTree>,
    control: OperationControl,
    canonical_tree: Arc<RetainedCemTree>,
    canonical_template: AstNodeId,
    limits: CemValueArtifactLimits,
    external: Option<ExternalCemPayloadInput>,
}
impl std::fmt::Debug for RetainedCemPayloadSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RetainedCemPayloadSource")
            .field("template", &self.template)
            .field("tree", &self.tree)
            .finish()
    }
}
impl RetainedCemPayloadSource {
    pub fn source_tree(&self) -> &Arc<RetainedCemTree> {
        &self.source_tree
    }
    pub fn template_node(&self) -> AstNodeId {
        self.template
    }
    pub fn canonical_tree(&self) -> &Arc<RetainedCemTree> {
        &self.canonical_tree
    }
    pub fn canonical_template_node(&self) -> AstNodeId {
        self.canonical_template
    }
    pub fn matches_canonical_source(
        &self,
        tree: &Arc<RetainedCemTree>,
        template: AstNodeId,
    ) -> bool {
        Arc::ptr_eq(&self.canonical_tree, tree) && self.canonical_template == template
    }
    pub fn tree(&self) -> &Arc<RetainedCemTree> {
        &self.tree
    }
    pub fn external_input(&self) -> Option<&ExternalCemPayloadInput> {
        self.external.as_ref()
    }
    pub fn matches_operation(&self, control: &OperationControl) -> bool {
        self.control.same_operation(control)
    }
    pub fn matches_source(&self, tree: &Arc<RetainedCemTree>, template: AstNodeId) -> bool {
        Arc::ptr_eq(&self.source_tree, tree) && self.template == template
    }
}
/// A bounded original-operation pool of weak source leases. Shared parsed views
/// carry no authority to select another host's canonical envelope.
pub struct RetainedCemPayloadSourcePool {
    control: OperationControl,
    entries: std::sync::Mutex<Vec<std::sync::Weak<RetainedCemPayloadSource>>>,
    _memory: MemoryPermit,
}
impl RetainedCemPayloadSourcePool {
    pub fn new(control: &OperationControl) -> Result<Self, String> {
        let memory = control
            .charge_memory(ROOT_EXECUTION_SCOPE_ID, 4096, None)
            .map_err(|e| e.code().to_owned())?;
        Ok(Self {
            control: control.clone(),
            entries: Default::default(),
            _memory: memory,
        })
    }
    pub fn retain(
        &self,
        original: Arc<RetainedCemTree>,
        template: AstNodeId,
        control: &OperationControl,
        limits: &CemValueArtifactLimits,
    ) -> Result<Arc<RetainedCemPayloadSource>, String> {
        if !self.control.same_operation(control) {
            return Err("Payload source pool belongs to the original operation".into());
        }
        let result =
            retain_inline_cem_payload_mode(original, template, control, limits, Some(self), false)?;
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| "Payload source pool unavailable")?;
        entries.retain(|entry| entry.strong_count() != 0);
        if entries.len() >= 64 {
            return Err("Payload source lease inventory exceeds bound".into());
        }
        entries.push(Arc::downgrade(&result));
        Ok(result)
    }
    fn shared_view(
        &self,
        source: &InlineCemTemplateSource,
        original: &Arc<RetainedCemTree>,
        limits: &CemValueArtifactLimits,
    ) -> Result<Option<Arc<RetainedCemTree>>, String> {
        let entries = self
            .entries
            .lock()
            .map_err(|_| "Payload source pool unavailable")?;
        for candidate in entries.iter().filter_map(std::sync::Weak::upgrade) {
            let owner = candidate
                .tree
                .native_owner()
                .and_then(|owner| owner.downcast_ref::<Owner>())
                .ok_or("Missing native payload producer")?;
            if candidate.limits.max_bytes <= limits.max_bytes
                && candidate.limits.max_values <= limits.max_values
                && candidate.limits.max_depth <= limits.max_depth
                && candidate.tree.source_uri() == original.source_uri()
                && candidate.tree.base_uri() == original.base_uri()
                && candidate.tree.document_uri() == original.document_uri()
                && owner._source.same_native_source(source)
            {
                return Ok(Some(candidate.tree.clone()));
            }
        }
        Ok(None)
    }
}
/// Match the browser's CEM payload type aliases and String.trim whitespace.
/// U+0085 is meaningful here; Rust's broader trim must not admit it.
pub fn canonical_cem_payload_type(value: &str) -> Option<&'static str> {
    let value=value.trim_matches(|c|matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'));
    (value.eq_ignore_ascii_case("cem-ml") || value.eq_ignore_ascii_case("text/cem-ml"))
        .then_some("text/cem-ml")
}
struct Owner {
    _source: InlineCemTemplateSource,
    _original: Arc<RetainedCemTree>,
    _memory: Vec<MemoryPermit>,
}
struct BoundedTokens {
    tokenizer: CemTokenizer,
    failed: Arc<AtomicBool>,
    work: usize,
    depth: usize,
    limits: CemValueArtifactLimits,
    diagnostics: Arc<std::sync::Mutex<Vec<Diagnostic>>>,
}
impl SchemaTokenizer for BoundedTokens {
    fn profile(&self) -> TokenizerProfile {
        TokenizerProfile::Cem
    }
    fn next_token(&mut self) -> Option<SchemaToken> {
        if self.failed.load(Ordering::Relaxed) {
            return None;
        }
        let Some(token) = self.tokenizer.next_token() else {
            self.diagnostics
                .lock()
                .unwrap()
                .extend(self.tokenizer.take_diagnostics());
            return None;
        };
        self.work = self.work.saturating_add(1);
        match token.kind {
            SchemaTokenKind::NodeStart { .. } | SchemaTokenKind::AnonymousScopeStart => {
                self.depth += 1
            }
            SchemaTokenKind::NodeEnd { .. } => self.depth = self.depth.saturating_sub(1),
            _ => {}
        }
        if self.work > self.limits.max_values.saturating_mul(8)
            || self.depth > self.limits.max_depth
        {
            self.failed.store(true, Ordering::Relaxed);
            None
        } else {
            Some(token)
        }
    }
}

pub fn retain_inline_cem_payload(
    original: Arc<RetainedCemTree>,
    template: AstNodeId,
    control: &OperationControl,
    limits: &CemValueArtifactLimits,
) -> Result<Arc<RetainedCemPayloadSource>, String> {
    retain_inline_cem_payload_mode(original, template, control, limits, None, false)
}

/// Parse the literal text of a canonical source envelope directly from its
/// live DOM capture. The retained view preserves capture maps and unknown
/// coordinates. An encoded copy cannot supply the native capture owner.
pub fn retain_captured_cem_payload(
    original: Arc<RetainedCemTree>,
    template: AstNodeId,
    control: &OperationControl,
    limits: &CemValueArtifactLimits,
) -> Result<Arc<RetainedCemPayloadSource>, String> {
    if !crate::import::dom::has_live_capture_owner(&original) {
        return Err("CEM payload requires a live DOM capture owner".into());
    }
    retain_inline_cem_payload_mode(original, template, control, limits, None, true)
}

fn retain_inline_cem_payload_mode(
    original: Arc<RetainedCemTree>,
    template: AstNodeId,
    control: &OperationControl,
    limits: &CemValueArtifactLimits,
    pool: Option<&RetainedCemPayloadSourcePool>,
    capture: bool,
) -> Result<Arc<RetainedCemPayloadSource>, String> {
    let check = || check_operation(control);
    check()?;
    let limits = payload_limits(control, limits)?;
    let owner = if capture {
        None
    } else {
        Some(super::original_html_owner(&original)?)
    };
    let node = original.node(template).ok_or("Missing source template")?;
    if !node.name.as_ref().is_some_and(|n| {
        n.namespace_uri == crate::schema::registry::HTML_NAMESPACE_URI && n.local_name == "template"
    }) {
        return Err("CEM payload requires a source template".into());
    }
    let kind = node.attributes.iter().find_map(|id| {
        original.node(*id).and_then(|attr| {
            attr.name
                .as_ref()
                .filter(|name| name.namespace_uri.is_empty() && name.local_name == "type")
                .map(|_| attr.value.as_str())
        })
    });
    if kind.and_then(canonical_cem_payload_type).is_none() || capture && kind != Some("text/cem-ml")
    {
        return Err("CEM payload requires its actual canonical source type".into());
    }
    // The retained semantic tree already owns decoded HTML text. Check its
    // size before allocating the second, mapped source buffer.
    let mut source_bytes = 0usize;
    for child in &node.children {
        check()?;
        let child = original.node(*child).ok_or("Missing source child")?;
        if child.kind == crate::parser::tree::CemTreeNodeKind::Text {
            source_bytes = source_bytes.saturating_add(child.value.len());
        }
        if source_bytes > limits.max_bytes {
            return Err("CEM payload source bounds exceeded".into());
        }
    }
    if limits.max_values == 0 {
        return Err("CEM payload source bounds exceeded".into());
    }
    let memory = control
        .charge_memory(
            ROOT_EXECUTION_SCOPE_ID,
            source_bytes.saturating_mul(8).saturating_add(1) as u64,
            original.node_source_map(template),
        )
        .map_err(|e| e.code().to_owned())?;
    let derived = original
        .source_key(template)
        .is_some_and(|key| key.starts_with("cem-derived:"))
        || node.children.iter().chain(&node.attributes).any(|id| {
            original
                .source_key(*id)
                .is_some_and(|key| key.starts_with("cem-derived:"))
        });
    let source = if capture {
        super::literal_inline_text(&original, template, None)?
    } else if derived {
        super::retained_inline_text(&original, template)?
    } else {
        let range = original
            .source_node_range(template)
            .ok_or("Missing source coordinates")?;
        let event = owner
            .as_ref()
            .ok_or("Missing authored HTML owner")?
            .events
            .iter()
            .position(|event| {
                event.kind == html_parser::HtmlEventKind::StartElement
                    && event.source_range.start.byte_offset == range.offset
                    && event.source_range.byte_length == range.length
            })
            .ok_or("Missing original template event")?;
        let imported = RetainedElementImport {
            tree: original.clone(),
            elements: vec![(
                template,
                ElementSourceHandle::Html { event_index: event },
                0,
            )],
            attribute_names_ascii_case_insensitive: true,
        };
        imported.inline_cem_template_source(template)?
    };
    let source_uri = owner
        .as_ref()
        .map_or(original.source_uri(), |owner| owner.source.uri.as_str());
    if source.text().len() > limits.max_bytes {
        return Err("CEM payload source bounds exceeded".into());
    }
    let shared = match pool {
        Some(pool) => pool.shared_view(&source, &original, &limits)?,
        None => None,
    };
    let tree = if let Some(tree) = shared {
        tree
    } else {
        retain_payload_tree(
            &original, template, source, source_uri, memory, limits, control,
        )?
    };
    check()?;
    let type_id = original
        .node(template)
        .unwrap()
        .attributes
        .iter()
        .copied()
        .find(|id| {
            original
                .node(*id)
                .and_then(|n| n.name.as_ref())
                .is_some_and(|n| n.namespace_uri.is_empty() && n.local_name == "type")
        })
        .ok_or("Missing payload type")?;
    let (canonical_tree, canonical_template) =
        if original.node(type_id).unwrap().value == "text/cem-ml" {
            (original.clone(), template)
        } else {
            let attr = crate::value::CemValueAttribute {
                name: "type".into(),
                namespace: None,
                qualified_name: None,
                value: "text/cem-ml".into(),
                value_stream: "text/cem-ml".to_owned(),
                contract: None,
                source_map: original.node_source_map(type_id).unwrap_or_default(),
            };
            // The whole-page edit uses the existing page-application ceilings;
            // source-code parsing above has its independent, tighter import limits.
            let page_limits = CemValueArtifactLimits {
                max_bytes: super::super::MAX_DOCUMENT_BYTES,
                max_values: super::super::MAX_DOCUMENT_VALUES,
                max_depth: 64,
            };
            let applied = crate::parser::tree_apply::apply_attributes(
                original.clone(),
                template,
                Arc::new(vec![attr]),
                Arc::new(()),
                page_limits,
                control,
            )
            .map_err(|e| e.code)?;
            (applied.tree().clone(), applied.output_target())
        };
    check()?;
    Ok(Arc::new(RetainedCemPayloadSource {
        source_tree: original,
        template,
        tree,
        control: control.clone(),
        canonical_tree,
        canonical_template,
        limits,
        external: None,
    }))
}

fn check_operation(control: &OperationControl) -> Result<(), String> {
    control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|e| e.code().to_owned())
}

fn payload_limits(
    control: &OperationControl,
    limits: &CemValueArtifactLimits,
) -> Result<CemValueArtifactLimits, String> {
    let scopes = control.scope_tree();
    let policy = scopes
        .scope(ROOT_EXECUTION_SCOPE_ID)
        .ok_or("Missing payload operation scope")?
        .effective_policy;
    Ok(CemValueArtifactLimits {
        max_bytes: limits.max_bytes.min(MAX_CEM_PAYLOAD_SOURCE_BYTES),
        max_values: limits.max_values.min(16_384),
        max_depth: limits.max_depth.min(64).min(policy.stack_depth as usize),
    })
}

fn retain_payload_tree(
    original: &Arc<RetainedCemTree>,
    template: AstNodeId,
    source: InlineCemTemplateSource,
    source_uri: &str,
    memory: MemoryPermit,
    limits: CemValueArtifactLimits,
    control: &OperationControl,
) -> Result<Arc<RetainedCemTree>, String> {
    let check = || check_operation(control);
    let external = source.external_uri.is_some();
    let full_range = ByteRange::new(0, source.text().len() as u32);
    let root_map = if external {
        source.source_map_for_range(full_range)
    } else {
        original.node_source_map(template)
    };
    let failed = Arc::new(AtomicBool::new(false));
    let diagnostics = Arc::new(std::sync::Mutex::new(Vec::new()));
    let tokenizer = CemTokenizer::from_source_with_control(
        BytesSource::new(SourceId(1), source.text().as_bytes().to_vec()),
        control.clone(),
        ROOT_EXECUTION_SCOPE_ID,
    );
    let captured = CemSchemaMachine::new(
        CompiledSchema::cem_core(),
        CemEventNormalizer::new(BoundedTokens {
            tokenizer,
            failed: failed.clone(),
            work: 0,
            depth: 0,
            limits,
            diagnostics: diagnostics.clone(),
        }),
    )
    .build_with_lexical_scopes();
    check()?;
    let ast = captured.document();
    if failed.load(Ordering::Relaxed) || ast.nodes.len() > limits.max_values {
        return Err("CEM payload AST bounds exceeded".into());
    }
    let lexical_diagnostics = diagnostics.lock().unwrap();
    if let Some(error) = captured
        .diagnostics()
        .iter()
        .chain(ast.diagnostics.iter())
        .chain(lexical_diagnostics.iter())
        .find(|d| d.severity.is_hard_violation())
    {
        return Err(format!("CEM payload source refused: {}", error.code));
    }
    let source_key = original
        .source_key(template)
        .ok_or("Missing native source selection key")?;
    let mut semantics = CemTreeSemantics {
        source_fingerprint: Some(source_fingerprint(
            source_uri,
            source.text().as_bytes(),
            &[
                if external {
                    "external-cem-payload/1"
                } else {
                    "inline-cem-payload/1"
                },
                &source_key,
            ],
        )),
        document_metadata: Some(CemDocumentMetadata {
            document_uri: if external {
                Some(source_uri.into())
            } else {
                original.document_uri().map(str::to_owned)
            },
            base_uri: if external {
                Some(source_uri.into())
            } else {
                original.base_uri().map(str::to_owned)
            },
        }),
        ..Default::default()
    };
    for (id, node) in ast.nodes.iter().enumerate() {
        check()?;
        let id = id as u32;
        if captured.pending_namespace_declaration(ast, id).is_some()
            || captured.pending_namespace_name(ast, id).is_some()
            || captured
                .pending_namespace_bindings(ast, id)
                .is_some_and(|n| !n.is_empty())
        {
            return Err("CEM payload requires ready namespace bindings".into());
        }
        if let Some(name) = captured.expanded_name(ast, id) {
            semantics.names.insert(id, name.clone());
        } else if matches!(node,CemAstNode::Element{expanded_name,..}|CemAstNode::Attribute{expanded_name,..} if !expanded_name.namespace_uri.is_empty() || expanded_name.local_name.contains(':'))
        {
            return Err("CEM payload contains an unbound namespace name".into());
        }
        let parsed = match node {
            CemAstNode::Document { source, .. }
            | CemAstNode::Element { source, .. }
            | CemAstNode::Attribute { source, .. }
            | CemAstNode::Reference { source, .. }
            | CemAstNode::Text { source, .. }
            | CemAstNode::Whitespace { source, .. }
            | CemAstNode::Comment { source, .. }
            | CemAstNode::Cdata { source, .. }
            | CemAstNode::RawText { source, .. }
            | CemAstNode::ProcessingInstruction { source, .. }
            | CemAstNode::Error { source, .. } => source,
        };
        let mut map = if id == 0 {
            root_map.clone().unwrap_or_default()
        } else if external {
            parsed.clone().with_source_uri(source_uri)
        } else {
            parsed
                .origin()
                .and_then(|f| match f.span {
                    FrameSpan::Single(r) => source.source_map_for_range(r),
                    _ => None,
                })
                .unwrap_or_default()
        };
        if let Some(frame) = map.current().cloned() {
            map.push(SourceMapFrame {
                transform: TransformKind::HandoffBoundary {
                    child_content_type: "text/cem-ml".into(),
                },
                ..frame
            });
        }
        semantics.sources.insert(id, map);
        // The origin may be another authored resource; decoded-buffer lines and
        // the enclosing HTML line index are not interchangeable with it.
        if !external {
            semantics.unknown_source_lines.insert(id);
        }
    }
    let retained = control
        .charge_memory(
            ROOT_EXECUTION_SCOPE_ID,
            ast.nodes
                .len()
                .saturating_mul(1024)
                .saturating_add(source.text().len()) as u64,
            root_map,
        )
        .map_err(|e| e.code().to_owned())?;
    let ast = ast.clone();
    let uri = source_uri.to_owned();
    let native = Arc::new(Owner {
        _source: source,
        _original: original.clone(),
        _memory: vec![memory, retained],
    });
    let source_text = if external { native._source.text() } else { "" };
    RetainedCemTree::from_shared(ast, uri, source_text, semantics, Some(native.clone()))
}

/// Admit bounded public CEM-ML bytes through the same inert parser as inline
/// payloads. The original stub and the materialized canonical envelope have
/// separate native owners and source ranges.
pub fn retain_external_cem_payload(
    original: Arc<RetainedCemTree>,
    template: AstNodeId,
    uri: &str,
    bytes: Vec<u8>,
    control: &OperationControl,
    limits: &CemValueArtifactLimits,
) -> Result<Arc<RetainedCemPayloadSource>, String> {
    check_operation(control)?;
    let limits = payload_limits(control, limits)?;
    if bytes.len() > limits.max_bytes || limits.max_values == 0 {
        return Err("CEM payload source bounds exceeded".into());
    }
    let node = original.node(template).ok_or("Missing source template")?;
    if !node.name.as_ref().is_some_and(|name| {
        name.namespace_uri == crate::schema::registry::HTML_NAMESPACE_URI
            && name.local_name == "template"
    }) || !node.children.is_empty()
    {
        return Err("External CEM payload requires an empty source template".into());
    }
    let type_node =
        node.attributes
            .iter()
            .copied()
            .find(|id| {
                original.node(*id).is_some_and(|attr| {
                    attr.name.as_ref().is_some_and(|name| {
                        name.namespace_uri.is_empty() && name.local_name == "type"
                    }) && canonical_cem_payload_type(&attr.value).is_some()
                })
            })
            .ok_or("External CEM payload requires its canonical source type")?;
    let type_map = original.node_source_map(type_node).unwrap_or_default();
    let external = ExternalCemPayloadInput {
        uri: uri.into(),
        content: ContentHash::from_blake3(&bytes),
        byte_length: bytes.len(),
    };
    let text = String::from_utf8(bytes).map_err(|_| "External CEM payload must use UTF-8")?;
    if text.contains('\0') {
        return Err("External CEM payload contains an unsupported NUL".into());
    }
    let source = InlineCemTemplateSource {
        owner: None,
        text,
        segments: vec![],
        mapped: None,
        external_uri: Some(uri.into()),
    };
    let origin = source
        .source_map_for_range(ByteRange::new(0, external.byte_length as u32))
        .ok_or("Missing external payload source range")?;
    let memory = control
        .charge_memory(
            ROOT_EXECUTION_SCOPE_ID,
            external.byte_length.saturating_mul(8).saturating_add(4096) as u64,
            Some(origin.clone()),
        )
        .map_err(|e| e.code().to_owned())?;
    let text = source.text().to_owned();
    let tree = retain_payload_tree(&original, template, source, uri, memory, limits, control)?;
    let envelope_key = original
        .source_key(template)
        .ok_or("Missing native source selection key")?;
    let fingerprint = source_fingerprint(
        uri,
        text.as_bytes(),
        &["external-cem-payload-envelope/1", &envelope_key],
    );
    let canonical_template = 1;
    let ast = crate::parser::document::CemDocument {
        nodes: vec![
            CemAstNode::Document {
                node_id: 0,
                root_children: vec![canonical_template],
                source: origin.clone(),
            },
            CemAstNode::Element {
                node_id: canonical_template,
                expanded_name: ExpandedName {
                    namespace_uri: crate::schema::registry::HTML_NAMESPACE_URI.into(),
                    local_name: "template".into(),
                    schema_id: None,
                },
                attributes: vec![2],
                children: vec![3],
                has_explicit_boundary: true,
                source: original.node_source_map(template).unwrap_or_default(),
            },
            CemAstNode::Attribute {
                node_id: 2,
                expanded_name: ExpandedName {
                    namespace_uri: String::new(),
                    local_name: "type".into(),
                    schema_id: None,
                },
                value: Some("text/cem-ml".into()),
                value_nodes: vec![],
                source: type_map,
            },
            CemAstNode::Text {
                node_id: 3,
                data: text,
                source: origin,
            },
        ],
        ..Default::default()
    };
    let canonical_tree = RetainedCemTree::new(
        ast,
        uri,
        "",
        CemTreeSemantics {
            source_fingerprint: Some(fingerprint),
            ..Default::default()
        },
        Some(tree.clone()),
    )?;
    check_operation(control)?;
    Ok(Arc::new(RetainedCemPayloadSource {
        source_tree: original,
        template,
        tree,
        control: control.clone(),
        canonical_tree,
        canonical_template,
        limits,
        external: Some(external),
    }))
}
