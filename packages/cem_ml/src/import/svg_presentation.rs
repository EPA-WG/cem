//! SVG presentation values enter CSS exclusively at the shared import boundary.
//! Decoded attribute offsets are diagnostic coordinates, not source edit ranges.
use crate::{
    css_resources::{attribute, named, CssResourcePlan},
    parser::{
        tree::{CemTreeRange, RetainedCemTree},
        AstNodeId,
    },
    source_map::SourceMapStack,
};
use std::{ops::Range, sync::Arc};
#[derive(Clone)]
pub struct RetainedSvgPresentationAttribute {
    document: Arc<RetainedCemTree>,
    css: Arc<RetainedCemTree>,
    owner: AstNodeId,
    attribute: AstNodeId,
    prefix: usize,
}
impl RetainedSvgPresentationAttribute {
    pub fn document(&self) -> &Arc<RetainedCemTree> {
        &self.document
    }
    pub fn css(&self) -> &Arc<RetainedCemTree> {
        &self.css
    }
    pub fn owner(&self) -> AstNodeId {
        self.owner
    }
    pub fn attribute(&self) -> AstNodeId {
        self.attribute
    }
    pub fn value(&self) -> &str {
        &self.document.node(self.attribute).unwrap().value
    }
    pub fn source(&self) -> &SourceMapStack {
        &self.document.node(self.attribute).unwrap().source
    }
    pub fn decoded_value_range(&self, range: CemTreeRange) -> Option<Range<usize>> {
        let start = usize::try_from(range.offset)
            .ok()?
            .checked_sub(self.prefix)?;
        let end = start.checked_add(usize::try_from(range.length).ok()?)?;
        (end <= self.value().len()
            && self.value().is_char_boundary(start)
            && self.value().is_char_boundary(end))
        .then_some(start..end)
    }
}
pub struct SvgPresentationPlan {
    pub source: RetainedSvgPresentationAttribute,
    pub resources: CssResourcePlan,
}
/// URL-capable SVG 2 presentation attributes. Other property values and SMIL
/// mutations remain within general runtime coverage, not this bounded URL inventory.
pub fn presentation_slot(element: &str, attribute: &str) -> bool {
    !matches!(
        element,
        "animate" | "animateMotion" | "animateTransform" | "set" | "discard"
    ) && matches!(
        attribute,
        "fill"
            | "stroke"
            | "filter"
            | "clip-path"
            | "mask"
            | "marker-start"
            | "marker-mid"
            | "marker-end"
            | "cursor"
    )
}
pub fn import_svg_presentation_attribute(
    document: Arc<RetainedCemTree>,
    owner: AstNodeId,
    attr: AstNodeId,
    max_bytes: usize,
) -> Result<RetainedSvgPresentationAttribute, String> {
    let element = document.node(owner).ok_or("missing SVG owner")?;
    let value = document
        .node(attr)
        .ok_or("missing SVG presentation attribute")?;
    let name = element.name.as_ref().ok_or("SVG element name required")?;
    let field = value
        .name
        .as_ref()
        .ok_or("SVG presentation attribute name required")?;
    if name.namespace_uri != "http://www.w3.org/2000/svg"
        || !field.namespace_uri.is_empty()
        || !element.attributes.contains(&attr)
        || !presentation_slot(&name.local_name, &field.local_name)
    {
        return Err("unsupported SVG presentation slot or owner".into());
    }
    if value.value.len() > max_bytes.min(super::MAX_DOCUMENT_BYTES) {
        return Err("SVG presentation byte budget exceeded".into());
    }
    let prefix = field.local_name.len() + 1;
    let source = format!("{}:{}", field.local_name, value.value);
    let uri = format!(
        "urn:cem:svg-presentation:{attr}:{}",
        crate::content_cache::ContentHash::from_blake3(source.as_bytes()).header_value()
    );
    let content_type = "text/css; mode=style-attribute";
    let parsed = super::css::parse(source.as_bytes(), &uri, content_type)?;
    // A declaration-list parser may recover a trailing separator as valid CSS.
    // An SVG attribute is a property value, so no top-level separator is allowed.
    if parsed
        .events
        .iter()
        .any(|e| e.depth == 0 && e.token_kind == "semicolon")
    {
        return Err("SVG presentation cannot contain a declaration separator".into());
    }
    let css = super::project_native(
        Arc::new(crate::lifecycle::LoadedInputAstStream::CssDocument(parsed)),
        "",
        "cem",
        &uri,
        content_type,
        source.len(),
        Some(super::source_fingerprint(
            &uri,
            source.as_bytes(),
            &["svg-presentation/1"],
        )),
        super::MAX_DOCUMENT_VALUES,
    )?;
    let root = css
        .node(0)
        .and_then(|n| n.children.first())
        .and_then(|id| css.node(*id))
        .ok_or("missing CSS value root")?;
    let declarations: Vec<_> = root
        .children
        .iter()
        .filter(|id| !named(&css, **id, "comment"))
        .copied()
        .collect();
    if declarations.len() != 1
        || !named(&css, declarations[0], "declaration")
        || attribute(&css, declarations[0], "name") != Some(field.local_name.as_str())
        || attribute(&css, declarations[0], "important").is_some()
        || css
            .ast_owner()
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
    {
        return Err("SVG presentation requires one CSS property value without !important".into());
    }
    Ok(RetainedSvgPresentationAttribute {
        document,
        css,
        owner,
        attribute: attr,
        prefix,
    })
}
