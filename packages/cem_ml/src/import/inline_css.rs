//! Extract and map an authored HTML style block at the native import boundary.
use super::*;
use crate::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    value::artifact::CemValueArtifactLimits,
};
pub struct RetainedInlineCss {
    tree: Arc<RetainedCemTree>,
    css: String,
}
impl RetainedInlineCss {
    pub fn css(&self) -> &str {
        &self.css
    }
    pub fn into_parts(self) -> (Arc<RetainedCemTree>, String) {
        (self.tree, self.css)
    }
}
struct Owner {
    _source: Arc<RetainedCemTree>,
    _css: crate::validation::css::CssDocumentAst,
    _memory: crate::operation_control::MemoryPermit,
}
pub fn retain_inline_css(
    source: Arc<RetainedCemTree>,
    style: AstNodeId,
    control: &OperationControl,
    limits: &CemValueArtifactLimits,
) -> Result<RetainedInlineCss, String> {
    let check = || {
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|e| e.code().to_owned())
    };
    check()?;
    let original = super::element_tree::original_html_owner(&source)?;
    let node = source.node(style).ok_or("Missing style node")?;
    if !node.name.as_ref().is_some_and(|n| {
        n.namespace_uri == crate::schema::registry::HTML_NAMESPACE_URI && n.local_name == "style"
    }) || node.attributes.iter().any(|id| {
        source
            .node(*id)
            .and_then(|n| n.name.as_ref())
            .is_none_or(|name| name.namespace_uri != "http://www.w3.org/2000/xmlns/")
    }) {
        return Err("Inline instance CSS requires an unqualified style block".into());
    }
    let mut bytes = 0usize;
    for id in &node.children {
        check()?;
        let child = source.node(*id).ok_or("Missing style text")?;
        if child.kind != crate::parser::tree::CemTreeNodeKind::Text {
            return Err("Style block requires literal text".into());
        }
        bytes = bytes.saturating_add(child.value.len());
    }
    if bytes > limits.max_bytes.min(32_768) || limits.max_values == 0 {
        return Err("Inline CSS source bounds exceeded".into());
    }
    let memory = control
        .charge_memory(
            ROOT_EXECUTION_SCOPE_ID,
            bytes.saturating_mul(128).saturating_add(1024) as u64,
            source.node_source_map(style),
        )
        .map_err(|e| e.code().to_owned())?;
    let css = source
        .text_fragments(style)
        .ok_or("Missing style text")?
        .collect::<String>();
    // HTML raw-text has no character references. Defer newline normalization
    // requiring a segmented source map rather than inventing byte coordinates.
    let derived = source
        .source_key(style)
        .is_some_and(|key| key.starts_with("cem-derived:"))
        || node.children.iter().any(|id| {
            source
                .source_key(*id)
                .is_some_and(|key| key.starts_with("cem-derived:"))
        });
    let mapped = if derived {
        Some(super::element_tree::retained_inline_text(&source, style)?)
    } else {
        None
    };
    let start = source.source_node_range(style);
    let offset = if mapped.is_some() {
        0
    } else {
        let start = start.ok_or("Missing style coordinates")?;
        let offset = start.offset + start.length;
        if original
            .source_text
            .get(offset as usize..offset as usize + css.len())
            != Some(css.as_str())
            || css.contains('\r')
        {
            return Err("Inline CSS source mapping requires unchanged raw text".into());
        }
        offset
    };
    let mut doc = super::css::parse(
        css.as_bytes(),
        source.source_uri(),
        "text/css; mode=scoped-style-block",
    )?;
    check()?;
    for event in &mut doc.events {
        if let Some(mapped) = &mapped {
            event.source_map = mapped
                .source_map_for_range(ByteRange::new(
                    event.source_range.start.byte_offset,
                    event
                        .source_range
                        .byte_length
                        .try_into()
                        .map_err(|_| "CSS map range overflow")?,
                ))
                .unwrap_or_default();
            continue;
        }
        let mut range = html_parser::HtmlSourceRange {
            start: html_parser::HtmlSourcePosition {
                line: 0,
                column: 0,
                byte_offset: offset + event.source_range.start.byte_offset,
            },
            byte_length: event.source_range.byte_length,
        };
        // The original owner resolves authoritative byte positions to frames.
        if let Some(position) = source.source_byte_coordinate(range.start.byte_offset) {
            range.start.line = position.line;
            range.start.column = position.column;
        }
        event.source_map = original
            .source_map_for_range(range)
            .ok_or("Missing authored CSS source map")?;
    }
    let (ast, mut semantics) = super::css::project(&doc)?;
    let policy = control
        .scope_tree()
        .scope(ROOT_EXECUTION_SCOPE_ID)
        .ok_or("Missing CSS scope")?
        .effective_policy;
    if ast.nodes.len() > limits.max_values
        || doc
            .events
            .iter()
            .any(|e| e.depth > limits.max_depth.min(policy.stack_depth as usize))
    {
        return Err("Inline CSS tree bounds exceeded".into());
    }
    if mapped.is_some() {
        semantics.ranges.clear();
    }
    for range in semantics.ranges.values_mut() {
        range.offset += offset;
        if let Some(position) = source.source_byte_coordinate(range.offset) {
            range.line = position.line;
            range.column = position.column;
        }
    }
    semantics
        .sources
        .insert(0, source.node_source_map(style).unwrap_or_default());
    semantics.source_fingerprint = Some(source_fingerprint(
        source.source_uri(),
        css.as_bytes(),
        &[
            "html-inline-css/1",
            &source
                .source_key(style)
                .ok_or("Missing style source identity")?,
        ],
    ));
    for id in 0..ast.nodes.len() as u32 {
        let node_map = match &ast.nodes[id as usize] {
            CemAstNode::Element { source, .. } | CemAstNode::Attribute { source, .. }
                if !source.frames.is_empty() =>
            {
                source.clone()
            }
            _ => source.node_source_map(style).unwrap_or_default(),
        };
        semantics.sources.entry(id).or_insert(node_map);
        if mapped.is_none() {
            semantics.ranges.entry(id).or_insert(start.unwrap());
        }
        if mapped.is_some() || start.is_some_and(|start| start.line == 0) {
            semantics.unknown_source_lines.insert(id);
        }
    }
    let uri = source.source_uri().to_owned();
    check()?;
    let tree = RetainedCemTree::new(
        ast,
        &uri,
        "",
        semantics,
        Some(Arc::new(Owner {
            _source: source,
            _css: doc,
            _memory: memory,
        })),
    )?;
    let mut pending = vec![(0, 0usize)];
    while let Some((id, depth)) = pending.pop() {
        check()?;
        if depth > limits.max_depth.min(64).min(policy.stack_depth as usize) {
            return Err("Inline CSS tree depth exceeded".into());
        }
        let node = tree.node(id).ok_or("Missing retained CSS node")?;
        pending.extend(
            node.children
                .iter()
                .chain(&node.attributes)
                .map(|id| (*id, depth + 1)),
        );
    }
    Ok(RetainedInlineCss { tree, css })
}
