//! Replace typed URL spans within retained component tokens. The importer owns
//! decoding and byte ranges; emission does not scan or reparse CSS strings.
use std::collections::BTreeMap;

use super::{diagnostic, CssEmissionDiagnostic};
use crate::{
    css_resources::{resource_reference, CssResourceKind, CssResourcePlan, CssResourceReference},
    parser::{tree::RetainedCemTree, AstNodeId},
};

pub(super) struct ResourceRewriter<'a> {
    tree: &'a RetainedCemTree,
    references: BTreeMap<AstNodeId, &'a CssResourceReference>,
}

impl<'a> ResourceRewriter<'a> {
    pub fn new(plan: &'a CssResourcePlan) -> Result<Self, CssEmissionDiagnostic> {
        let mut references = BTreeMap::new();
        for reference in &plan.references {
            if references.insert(reference.node_id, reference).is_some() {
                return Err(invalid(
                    &plan.tree,
                    reference.node_id,
                    "duplicate CSS resource plan entry",
                ));
            }
        }
        Ok(Self {
            tree: &plan.tree,
            references,
        })
    }

    pub fn token(&self, id: AstNodeId, token: &str) -> Result<String, CssEmissionDiagnostic> {
        let tree = self.tree;
        let outer = tree
            .node(id)
            .ok_or_else(|| invalid(tree, id, "missing component"))?;
        let mut pending = vec![id];
        let mut edits = Vec::new();
        while let Some(child) = pending.pop() {
            let node = tree
                .node(child)
                .ok_or_else(|| invalid(tree, child, "missing component child"))?;
            let reference = resource_reference(tree, child)
                .map_err(|message| invalid(tree, child, &message))?;
            let Some((authored, CssResourceKind::Url)) = reference else {
                pending.extend(node.children.iter().rev().copied());
                continue;
            };
            // Fragment binding is explicitly deferred. Preserve the whole token,
            // including its authored case, escapes and trivia, without a map rewrite.
            if authored.starts_with('#') {
                continue;
            }
            let reference = self
                .references
                .get(&child)
                .ok_or_else(|| invalid(tree, child, "missing external URL resolution"))?;
            if reference.kind != CssResourceKind::Url
                || reference.authored_specifier != authored
                || reference.range.offset != node.range.offset
                || reference.range.length != node.range.length
            {
                return Err(invalid(
                    tree,
                    child,
                    "CSS resource plan does not match its retained node",
                ));
            }
            let resolution = reference.resolution.as_ref().map_err(|error| {
                diagnostic(
                    tree,
                    child,
                    "cem.scoped_css.resource_resolution_failed",
                    &format!("CSS resource `{authored}`: {}", error.message),
                )
            })?;
            let start = node
                .range
                .offset
                .checked_sub(outer.range.offset)
                .and_then(|offset| usize::try_from(offset).ok())
                .ok_or_else(|| invalid(tree, child, "URL is outside its component"))?;
            let end = usize::try_from(node.range.length)
                .ok()
                .and_then(|length| start.checked_add(length))
                .ok_or_else(|| invalid(tree, child, "URL range overflow"))?;
            if token.get(start..end).is_none() {
                return Err(invalid(
                    tree,
                    child,
                    "URL range is outside the retained token",
                ));
            }
            edits.push((
                start,
                end,
                format!("url({})", css_string(&resolution.resolved_url)),
            ));
        }
        edits.sort_by_key(|(start, _, _)| *start);
        let mut output = String::new();
        let mut cursor = 0;
        for (start, end, replacement) in edits {
            if start < cursor {
                return Err(invalid(tree, id, "overlapping CSS resource ranges"));
            }
            output.push_str(&token[cursor..start]);
            output.push_str(&replacement);
            cursor = end;
        }
        output.push_str(&token[cursor..]);
        Ok(output)
    }
}

fn css_string(value: &str) -> String {
    let mut output = String::from("\"");
    for ch in value.chars() {
        if ch == '\0' {
            output.push('\u{fffd}');
        } else if ch == '"' || ch == '\\' || ch.is_control() {
            output.push_str(&format!("\\{:X} ", ch as u32));
        } else {
            output.push(ch);
        }
    }
    output.push('"');
    output
}

fn invalid(tree: &RetainedCemTree, id: AstNodeId, message: &str) -> CssEmissionDiagnostic {
    diagnostic(tree, id, "cem.scoped_css.resource_plan_invalid", message)
}
