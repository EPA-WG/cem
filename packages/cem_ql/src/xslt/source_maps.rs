//! Validate native member provenance without projecting ASTs through JSON.
use super::*;
use crate::render::{
    CompiledTemplateExpression, TemplateAttributePart, TemplateAttributeValue, TemplateNode,
};
use cem_ml::{
    source::ByteRange,
    source_map::{FrameSpan, SourceMapStack, TransformKind},
    validation::xpath::{XPathHostNodeKind, XPathSourceRange},
};

pub(super) fn validate_xpath(expression: &XPathExpressionAst, source: &BundleSource) -> Result<()> {
    let XPathAttachment::Host(host) = &expression.attachment else {
        return Err(BundleError::invalid("bundle requires XSLT-owned XPath"));
    };
    if expression.source.uri != source.uri
        || host.owner.source_uri != source.uri
        || host.owner.source_id == 0
        || host.owner.node_kind != XPathHostNodeKind::XsltAttribute
        || host.owner.schema_uri.as_deref() != Some(XSLT_SCHEMA_URI)
    {
        return Err(BundleError::mismatch("XPath source ownership mismatch"));
    }
    fn range(range: XPathSourceRange, source: &BundleSource) -> Result<()> {
        if range
            .start
            .byte_offset
            .checked_add(range.byte_length)
            .is_none_or(|end| end > source.byte_length)
        {
            return Err(BundleError::invalid(
                "XPath source range exceeds owning stylesheet",
            ));
        }
        Ok(())
    }
    range(host.owner.source_range, source)?;
    range(host.expression_range, source)?;
    let syntax = expression
        .syntax_ast
        .as_ref()
        .ok_or_else(|| BundleError::invalid("missing XPath typed syntax"))?;
    range(syntax.root.source_range, source)?;
    for event in &syntax.events {
        range(event.source_range, source)?;
    }
    Ok(())
}
fn byte_range(range: &ByteRange, source: &BundleSource) -> Result<()> {
    if range
        .start
        .checked_add(u64::from(range.len))
        .is_none_or(|end| end > source.byte_length)
    {
        return Err(BundleError::invalid(
            "CEMT source range exceeds generated source",
        ));
    }
    Ok(())
}
fn map(map: &SourceMapStack, source: &BundleSource) -> Result<()> {
    map_with_query(map, source, None)
}
fn map_with_query(
    map: &SourceMapStack,
    source: &BundleSource,
    query_bytes: Option<u64>,
) -> Result<()> {
    if map.frames.len() > 64 {
        return Err(BundleError::limit());
    }
    for frame in &map.frames {
        let length = match frame.source_id.0 {
            1 => source.byte_length,
            0 if matches!(
                frame.transform,
                TransformKind::Query | TransformKind::QueryStep
            ) =>
            {
                query_bytes
                    .ok_or_else(|| BundleError::mismatch("query-local frame outside expression"))?
            }
            _ => return Err(BundleError::mismatch("unknown generated CEMT source ID")),
        };
        let ranges = match &frame.span {
            FrameSpan::Single(range) => std::slice::from_ref(range),
            FrameSpan::Multi(ranges) => ranges.as_slice(),
        };
        if ranges.is_empty() || ranges.len() > 64 {
            return Err(BundleError::invalid("invalid CEMT source ranges"));
        }
        for range in ranges {
            if range
                .start
                .checked_add(u64::from(range.len))
                .is_none_or(|end| end > length)
            {
                return Err(BundleError::invalid(
                    "CEMT source range exceeds its owning source",
                ));
            }
        }
        if let TransformKind::TemplateEmbedding { host } = &frame.transform {
            byte_range(host, source)?;
        }
    }
    Ok(())
}
fn expression(expression: &CompiledTemplateExpression, source: &BundleSource) -> Result<()> {
    map(&expression.source_map, source)?;
    if expression.byte_offset > source.byte_length {
        return Err(BundleError::invalid(
            "CEMT expression offset exceeds generated source",
        ));
    }
    let query = expression
        .query
        .as_ref()
        .ok_or_else(|| BundleError::invalid("uncompiled CEMT expression"))?;
    for source_map in &query.tree.source_maps {
        map_with_query(source_map, source, Some(expression.source.len() as u64))?;
    }
    Ok(())
}
pub(super) fn validate_template(template: &TemplateArtifact, source: &BundleSource) -> Result<()> {
    let mut pending: Vec<_> = template.nodes.iter().map(|node| (node, 0usize)).collect();
    let mut count = 0;
    while let Some((node, depth)) = pending.pop() {
        count += 1;
        if count > 100_000 || depth > 128 {
            return Err(BundleError::limit());
        }
        match node {
            TemplateNode::Expression(value) => expression(value, source)?,
            TemplateNode::Text { source_map, .. } | TemplateNode::Comment { source_map, .. } => {
                map(source_map, source)?
            }
            TemplateNode::Element {
                attributes,
                children,
                source_map,
                ..
            }
            | TemplateNode::Result {
                attributes,
                children,
                source_map,
                ..
            } => {
                map(source_map, source)?;
                for attribute in attributes {
                    map(&attribute.source_map, source)?;
                    match &attribute.value {
                        Some(TemplateAttributeValue::Expression(value)) => {
                            expression(value, source)?
                        }
                        Some(TemplateAttributeValue::Template(parts)) => {
                            for part in parts {
                                if let TemplateAttributePart::Expression(value) = part {
                                    expression(value, source)?;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                pending.extend(children.iter().map(|node| (node, depth + 1)));
            }
            TemplateNode::If {
                test: select,
                children,
                source_map,
            }
            | TemplateNode::ForEach {
                select,
                children,
                source_map,
                ..
            } => {
                map(source_map, source)?;
                if let Some(value) = select {
                    expression(value, source)?;
                }
                pending.extend(children.iter().map(|node| (node, depth + 1)));
            }
            TemplateNode::ProjectPayload { select, source_map }
            | TemplateNode::Variable {
                select, source_map, ..
            } => {
                map(source_map, source)?;
                if let Some(value) = select {
                    expression(value, source)?;
                }
            }
            TemplateNode::Choose {
                branches,
                source_map,
            } => {
                map(source_map, source)?;
                for branch in branches {
                    if let Some(value) = &branch.test {
                        expression(value, source)?;
                    }
                    pending.extend(branch.children.iter().map(|node| (node, depth + 1)));
                }
            }
        }
    }
    for diagnostic in &template.diagnostics {
        if let Some(value) = &diagnostic.source_map {
            map(value, source)?;
        }
    }
    Ok(())
}
