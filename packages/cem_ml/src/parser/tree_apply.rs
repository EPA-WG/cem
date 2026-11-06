//! Bounded native CEM child replacement, independent of source syntax/host.
use super::{tree::RetainedCemTree, AstNodeId};
use crate::{
    operation_control::OperationControl,
    value::{artifact::CemValueArtifactLimits, CemValueNode},
};
use std::{any::Any, collections::BTreeMap, sync::Arc};
#[derive(Debug)]
pub struct CemTreeApplyError {
    pub code: String,
}
#[derive(Debug)]
pub struct CemTreeApplication {
    source: Arc<RetainedCemTree>,
    tree: Arc<RetainedCemTree>,
    target: AstNodeId,
    output_target: AstNodeId,
    children: Vec<AstNodeId>,
    source_ids: BTreeMap<AstNodeId, AstNodeId>,
    qualified_names: Arc<BTreeMap<AstNodeId, String>>,
}
impl CemTreeApplication {
    pub fn source(&self) -> &Arc<RetainedCemTree> {
        &self.source
    }
    pub fn tree(&self) -> &Arc<RetainedCemTree> {
        &self.tree
    }
    pub fn target(&self) -> AstNodeId {
        self.target
    }
    pub fn output_target(&self) -> AstNodeId {
        self.output_target
    }
    pub fn inserted_children(&self) -> &[AstNodeId] {
        &self.children
    }
    pub fn output_for_source(&self, id: AstNodeId) -> Option<AstNodeId> {
        self.source_ids.get(&id).copied()
    }
    pub fn qualified_name(&self, id: AstNodeId) -> Option<&str> {
        self.qualified_names.get(&id).map(String::as_str)
    }
}
struct ApplicationOwner {
    _source: Arc<RetainedCemTree>,
    _nodes: Arc<dyn Any + Send + Sync>,
    _attributes: Arc<dyn Any + Send + Sync>,
    _retained: Arc<dyn Any + Send + Sync>,
    _permits: Vec<crate::operation_control::MemoryPermit>,
    qualified_names: Arc<BTreeMap<AstNodeId, String>>,
}
/// Lexical names survive native handoff alongside expanded CEM names.
pub fn applied_qualified_name(tree: &RetainedCemTree, id: AstNodeId) -> Option<&str> {
    tree.native_owner()?
        .downcast_ref::<ApplicationOwner>()?
        .qualified_names
        .get(&id)
        .map(String::as_str)
}

/// One immutable element transaction: ordered attribute writes and owned children.
pub struct CemElementEdit<T, S> {
    pub children: Arc<Vec<CemValueNode<T, S>>>,
    pub attributes: Arc<Vec<crate::value::CemValueAttribute<S>>>,
}

pub fn apply_children<T: Send + Sync + 'static, S: Send + Sync + 'static>(
    source: Arc<RetainedCemTree>,
    target: AstNodeId,
    nodes: Arc<Vec<CemValueNode<T, S>>>,
    retained: Arc<dyn Any + Send + Sync>,
    limits: CemValueArtifactLimits,
    control: &OperationControl,
) -> Result<CemTreeApplication, CemTreeApplyError> {
    apply_element(
        source,
        target,
        CemElementEdit {
            children: nodes,
            attributes: Arc::new(vec![]),
        },
        retained,
        limits,
        control,
    )
}
pub fn apply_element<T: Send + Sync + 'static, S: Send + Sync + 'static>(
    source: Arc<RetainedCemTree>,
    target: AstNodeId,
    edit: CemElementEdit<T, S>,
    retained: Arc<dyn Any + Send + Sync>,
    mut limits: CemValueArtifactLimits,
    control: &OperationControl,
) -> Result<CemTreeApplication, CemTreeApplyError> {
    let nodes = edit.children;
    let attributes = edit.attributes;
    use crate::{operation_control::ROOT_EXECUTION_SCOPE_ID, parser::tree::CemTreeNodeKind};
    control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(control_error)?;
    if !source
        .node(target)
        .is_some_and(|n| n.kind == CemTreeNodeKind::Element)
    {
        return Err(error("cem.ast.apply_target"));
    }
    let scopes = control.scope_tree();
    let policy = scopes
        .scope(ROOT_EXECUTION_SCOPE_ID)
        .ok_or_else(|| error("cem.ast.apply_scope"))?
        .effective_policy;
    limits.max_depth = limits.max_depth.min(policy.stack_depth as usize);
    limits.max_bytes = limits
        .max_bytes
        .min(policy.memory_bytes.try_into().unwrap_or(usize::MAX));
    let mut names = std::collections::BTreeSet::new();
    if attributes.len() > limits.max_values {
        return Err(error("cem.ast.apply_bounds"));
    }
    for a in attributes.iter() {
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(control_error)?;
        if a.name.is_empty()
            || a.contract.is_some()
            || !names.insert((a.namespace.as_deref().unwrap_or_default(), a.name.as_str()))
        {
            return Err(error("cem.ast.apply_attributes"));
        }
    }
    let mut builder = Builder {
        source: source.clone(),
        target,
        nodes: &nodes,
        attributes: &attributes,
        limits,
        control,
        ast: super::document::CemDocument::default(),
        semantics: super::tree::CemTreeSemantics::default(),
        bytes: 0,
        permits: vec![],
        source_ids: BTreeMap::new(),
        qualified_names: BTreeMap::new(),
        inserted_children: vec![],
    };
    builder.semantics.document_metadata = Some(super::tree::CemDocumentMetadata {
        base_uri: source.base_uri().map(str::to_owned),
        document_uri: source.document_uri().map(str::to_owned),
    });
    builder.ast.format_identity = source.ast().format_identity.clone();
    builder.copy_source(0, 0)?;
    let Builder {
        ast,
        semantics,
        permits,
        source_ids,
        qualified_names,
        inserted_children,
        ..
    } = builder;
    control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(control_error)?;
    // Retain both source and native producer inputs; no source/AST wire round trip.
    let qualified_names = Arc::new(qualified_names);
    let owner: Arc<dyn Any + Send + Sync> = Arc::new(ApplicationOwner {
        _source: source.clone(),
        _nodes: nodes,
        _attributes: attributes,
        _retained: retained,
        _permits: permits,
        qualified_names: qualified_names.clone(),
    });
    let tree = RetainedCemTree::new(ast, source.source_uri(), "", semantics, Some(owner))
        .map_err(|_| error("cem.ast.apply_tree"))?;
    let output_target = *source_ids
        .get(&target)
        .ok_or_else(|| error("cem.ast.apply_target"))?;
    Ok(CemTreeApplication {
        source,
        tree,
        target,
        output_target,
        children: inserted_children,
        source_ids,
        qualified_names,
    })
}
fn error(code: &str) -> CemTreeApplyError {
    CemTreeApplyError { code: code.into() }
}
fn control_error(e: crate::operation_control::ControlError) -> CemTreeApplyError {
    error(e.code())
}
struct Builder<'a, T, S> {
    source: Arc<RetainedCemTree>,
    target: AstNodeId,
    nodes: &'a [CemValueNode<T, S>],
    attributes: &'a [crate::value::CemValueAttribute<S>],
    limits: CemValueArtifactLimits,
    control: &'a OperationControl,
    ast: super::document::CemDocument,
    semantics: super::tree::CemTreeSemantics,
    bytes: usize,
    permits: Vec<crate::operation_control::MemoryPermit>,
    source_ids: BTreeMap<AstNodeId, AstNodeId>,
    qualified_names: BTreeMap<AstNodeId, String>,
    inserted_children: Vec<AstNodeId>,
}
impl<T, S> Builder<'_, T, S> {
    fn allocate(&mut self, bytes: usize, depth: usize) -> Result<AstNodeId, CemTreeApplyError> {
        use crate::operation_control::ROOT_EXECUTION_SCOPE_ID;
        self.control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(control_error)?;
        self.bytes = self
            .bytes
            .saturating_add(bytes)
            .saturating_add(std::mem::size_of::<super::CemAstNode>() + 64);
        if depth > self.limits.max_depth
            || self.ast.nodes.len() >= self.limits.max_values
            || self.bytes > self.limits.max_bytes
        {
            return Err(error("cem.ast.apply_bounds"));
        }
        self.permits.push(
            self.control
                .charge_memory(
                    ROOT_EXECUTION_SCOPE_ID,
                    (bytes + std::mem::size_of::<super::CemAstNode>() + 64) as u64,
                    None,
                )
                .map_err(control_error)?,
        );
        let id = self.ast.nodes.len() as u32;
        self.ast.nodes.push(super::CemAstNode::Text {
            node_id: id,
            data: String::new(),
            source: Default::default(),
        });
        Ok(id)
    }
    fn copy_source(
        &mut self,
        old: AstNodeId,
        depth: usize,
    ) -> Result<AstNodeId, CemTreeApplyError> {
        use super::{tree::CemTreeNodeKind as K, CemAstNode as N};
        let n = self
            .source
            .node(old)
            .ok_or_else(|| error("cem.ast.apply_tree"))?;
        let (kind, name, value, children, attrs, map, base) = (
            n.kind,
            n.name.clone(),
            n.value.clone(),
            n.children.clone(),
            n.attributes.clone(),
            n.source.clone(),
            n.base_uri.clone(),
        );
        if kind == K::Reference
            || matches!(self.source.ast().get(old),Some(N::Attribute{value_nodes,..}) if !value_nodes.is_empty())
        {
            return Err(error("cem.ast.apply_reference"));
        }
        let id = self.allocate(
            value.len()
                + name
                    .as_ref()
                    .map_or(0, |n| n.local_name.len() + n.namespace_uri.len())
                + map.frames.len() * std::mem::size_of::<crate::source_map::SourceMapFrame>(),
            depth,
        )?;
        self.source_ids.insert(old, id);
        self.semantics.provenance.insert(
            id,
            crate::value::artifact::CemValueProvenance {
                source_uri: self.source.node_source_uri(old).map(str::to_owned),
                source_key: self.source.source_key(old),
                line_number: self.source.source_line_number(old),
            },
        );
        if let Some(range) = self.source.source_node_range(old) {
            self.semantics.ranges.insert(id, range);
        }
        if let Some(base) = base {
            self.semantics.base_uris.insert(id, base);
        }
        let mut output_attrs = vec![];
        for attr in &attrs {
            let current = self
                .source
                .node(*attr)
                .and_then(|n| n.name.as_ref())
                .ok_or_else(|| error("cem.ast.apply_tree"))?;
            let replacement = (old == self.target)
                .then(|| {
                    self.attributes.iter().find(|a| {
                        a.name == current.local_name
                            && a.namespace.as_deref().unwrap_or_default() == current.namespace_uri
                    })
                })
                .flatten();
            if let Some(a) = replacement {
                let id = self.insert_attribute(a, depth + 1)?;
                self.source_ids.insert(*attr, id);
                output_attrs.push(id);
            } else {
                output_attrs.push(self.copy_source(*attr, depth + 1)?);
            }
        }
        if old == self.target {
            for a in self.attributes {
                if !attrs.iter().any(|id| {
                    self.source
                        .node(*id)
                        .and_then(|n| n.name.as_ref())
                        .is_some_and(|n| {
                            n.local_name == a.name
                                && n.namespace_uri == a.namespace.as_deref().unwrap_or_default()
                        })
                }) {
                    output_attrs.push(self.insert_attribute(a, depth + 1)?);
                }
            }
        }
        let mut output_children = vec![];
        if old == self.target {
            for index in 0..self.nodes.len() {
                output_children.push(self.insert(&self.nodes[index], depth + 1)?);
            }
            self.inserted_children = output_children.clone();
        } else {
            for child in children {
                output_children.push(self.copy_source(child, depth + 1)?);
            }
        }
        self.ast.nodes[id as usize] = match kind {
            K::Document => N::Document {
                node_id: id,
                root_children: output_children,
                source: map,
            },
            K::Element => N::Element {
                node_id: id,
                expanded_name: name.unwrap(),
                attributes: output_attrs,
                children: output_children,
                has_explicit_boundary: true,
                source: map,
            },
            K::Attribute => N::Attribute {
                node_id: id,
                expanded_name: name.unwrap(),
                value: Some(value),
                value_nodes: vec![],
                source: map,
            },
            K::Comment => N::Comment {
                node_id: id,
                data: value,
                source: map,
            },
            K::ProcessingInstruction => N::ProcessingInstruction {
                node_id: id,
                target: name.map(|n| n.local_name).unwrap_or_default(),
                data: value,
                source: map,
            },
            _ => match self.source.ast().get(old) {
                Some(N::RawText { .. }) => N::RawText {
                    node_id: id,
                    data: value,
                    source: map,
                },
                Some(N::Cdata { .. }) => N::Cdata {
                    node_id: id,
                    data: value,
                    source: map,
                },
                Some(N::Whitespace { .. }) => N::Whitespace {
                    node_id: id,
                    data: value,
                    source: map,
                },
                _ => N::Text {
                    node_id: id,
                    data: value,
                    source: map,
                },
            },
        };
        Ok(id)
    }
    fn insert_attribute(
        &mut self,
        attr: &crate::value::CemValueAttribute<S>,
        depth: usize,
    ) -> Result<AstNodeId, CemTreeApplyError> {
        let aid = self.allocate(
            attr.name.len()
                + attr.value.len()
                + attr.namespace.as_ref().map_or(0, String::len)
                + attr.qualified_name.as_ref().map_or(0, String::len)
                + attr.source_map.frames.len()
                    * std::mem::size_of::<crate::source_map::SourceMapFrame>(),
            depth,
        )?;
        self.semantics.unknown_source_lines.insert(aid);
        if let Some(name) = &attr.qualified_name {
            self.qualified_names.insert(aid, name.clone());
        }
        self.ast.nodes[aid as usize] = super::CemAstNode::Attribute {
            node_id: aid,
            expanded_name: super::ExpandedName {
                namespace_uri: attr.namespace.clone().unwrap_or_default(),
                local_name: attr.name.clone(),
                schema_id: None,
            },
            value: Some(attr.value.clone()),
            value_nodes: vec![],
            source: attr.source_map.clone(),
        };
        Ok(aid)
    }
    fn insert(
        &mut self,
        node: &CemValueNode<T, S>,
        depth: usize,
    ) -> Result<AstNodeId, CemTreeApplyError> {
        use super::{CemAstNode as N, ExpandedName};
        use CemValueNode::*;
        let (map, bytes) = match node {
            Reference { .. } => return Err(error("cem.ast.apply_reference")),
            Element {
                tag,
                namespace,
                attributes,
                ..
            } => (
                match node {
                    Element { source_map, .. } => source_map,
                    _ => unreachable!(),
                },
                tag.len() + namespace.as_ref().map_or(0, String::len) + attributes.len() * 8,
            ),
            Text { text, source_map }
            | Comment { text, source_map }
            | Cdata { text, source_map } => (source_map, text.len()),
            ProcessingInstruction {
                target,
                data,
                source_map,
            } => (source_map, target.len() + data.len()),
        };
        let id = self.allocate(
            bytes + map.frames.len() * std::mem::size_of::<crate::source_map::SourceMapFrame>(),
            depth,
        )?;
        self.semantics.unknown_source_lines.insert(id);
        let map = map.clone();
        self.ast.nodes[id as usize] = match node {
            Element {
                tag,
                namespace,
                qualified_name,
                attributes,
                children,
                ..
            } => {
                if let Some(name) = qualified_name {
                    self.qualified_names.insert(id, name.clone());
                }
                let mut attrs = vec![];
                for attr in attributes {
                    let aid = self.insert_attribute(attr, depth + 1)?;
                    attrs.push(aid);
                }
                let mut kids = vec![];
                for child in children {
                    kids.push(self.insert(child, depth + 1)?);
                }
                N::Element {
                    node_id: id,
                    expanded_name: ExpandedName {
                        namespace_uri: namespace.clone().unwrap_or_default(),
                        local_name: tag.clone(),
                        schema_id: None,
                    },
                    attributes: attrs,
                    children: kids,
                    has_explicit_boundary: true,
                    source: map,
                }
            }
            Text { text, .. } => N::Text {
                node_id: id,
                data: text.clone(),
                source: map,
            },
            Comment { text, .. } => N::Comment {
                node_id: id,
                data: text.clone(),
                source: map,
            },
            Cdata { text, .. } => N::Cdata {
                node_id: id,
                data: text.clone(),
                source: map,
            },
            ProcessingInstruction { target, data, .. } => N::ProcessingInstruction {
                node_id: id,
                target: target.clone(),
                data: data.clone(),
                source: map,
            },
            Reference { .. } => unreachable!(),
        };
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        import::element_tree::retain_html_elements,
        parser::tree::CemDocumentMetadata,
        scheduler::AbortSignal,
        validation::html::{html_document_ast_from_source_bytes, HtmlSourceValidationRequest},
        value::CemReference,
    };
    fn source() -> Arc<RetainedCemTree> {
        let owner=Arc::new(html_document_ast_from_source_bytes(HtmlSourceValidationRequest{bytes:b"<!doctype html><html><body><div><b>Old</b></div><p>A &amp; B</p></body></html>",source_uri:"file:///source/page.html",content_type:Some("text/html")}).0.unwrap());
        retain_html_elements(
            owner,
            CemDocumentMetadata {
                document_uri: Some("https://public.test/page/".into()),
                base_uri: Some("https://public.test/page/".into()),
            },
        )
        .unwrap()
        .tree
    }
    fn target(tree: &RetainedCemTree) -> u32 {
        (0..tree.ast().nodes.len())
            .find(|id| {
                tree.node(*id as u32)
                    .and_then(|n| n.name.as_ref())
                    .is_some_and(|n| n.local_name == "div")
            })
            .unwrap() as u32
    }
    fn control() -> OperationControl {
        OperationControl::new(AbortSignal::new())
    }
    fn text() -> Arc<Vec<CemValueNode<(), ()>>> {
        Arc::new(vec![CemValueNode::Text {
            text: "New & literal".into(),
            source_map: Default::default(),
        }])
    }
    #[test]
    fn transaction_retains_source_and_decoded_untouched_nodes() {
        let source = source();
        let old = source.ast_owner().clone();
        let t = target(&source);
        let r = apply_children(
            source.clone(),
            t,
            text(),
            Arc::new(()),
            Default::default(),
            &control(),
        )
        .unwrap();
        assert!(Arc::ptr_eq(r.source(), &source));
        assert!(Arc::ptr_eq(&old, source.ast_owner()));
        assert_eq!(
            r.tree()
                .text_fragments(r.output_target())
                .unwrap()
                .collect::<String>(),
            "New & literal"
        );
        assert_eq!(source.text_fragments(t).unwrap().collect::<String>(), "Old");
        assert_eq!(r.tree().base_uri(), source.base_uri());
        let p = (0..source.ast().nodes.len())
            .find(|id| {
                source
                    .node(*id as u32)
                    .and_then(|n| n.name.as_ref())
                    .is_some_and(|n| n.local_name == "p")
            })
            .unwrap() as u32;
        let derived = r.output_for_source(p).unwrap();
        assert_eq!(
            r.tree()
                .text_fragments(derived)
                .unwrap()
                .collect::<String>(),
            "A & B"
        );
        assert_eq!(r.tree().source_key(derived), source.source_key(p));
        assert_eq!(r.tree().node_source_map(derived), source.node_source_map(p));
    }
    #[test]
    fn unprojected_references_and_invalid_targets_refuse_without_mutation() {
        let source = source();
        let t = target(&source);
        let nodes: Arc<Vec<CemValueNode<(), ()>>> = Arc::new(vec![CemValueNode::Reference {
            reference: CemReference::new(vec![()]),
            source_map: Default::default(),
        }]);
        assert_eq!(
            apply_children(
                source.clone(),
                t,
                nodes,
                Arc::new(()),
                Default::default(),
                &control()
            )
            .unwrap_err()
            .code,
            "cem.ast.apply_reference"
        );
        assert_eq!(
            apply_children(
                source.clone(),
                u32::MAX,
                text(),
                Arc::new(()),
                Default::default(),
                &control()
            )
            .unwrap_err()
            .code,
            "cem.ast.apply_target"
        );
        assert_eq!(source.text_fragments(t).unwrap().collect::<String>(), "Old");
    }
    #[test]
    fn bounds_and_original_scope_state_are_enforced() {
        let source = source();
        let t = target(&source);
        for limits in [
            CemValueArtifactLimits {
                max_values: 1,
                ..Default::default()
            },
            CemValueArtifactLimits {
                max_bytes: 1,
                ..Default::default()
            },
            CemValueArtifactLimits {
                max_depth: 1,
                ..Default::default()
            },
        ] {
            assert_eq!(
                apply_children(source.clone(), t, text(), Arc::new(()), limits, &control())
                    .unwrap_err()
                    .code,
                "cem.ast.apply_bounds"
            );
        }
        let c = control();
        c.abort_signal().abort();
        assert_eq!(
            apply_children(
                source.clone(),
                t,
                text(),
                Arc::new(()),
                Default::default(),
                &c
            )
            .unwrap_err()
            .code,
            "host-cancellation"
        );
        let c = control();
        c.complete_scope(crate::operation_control::ROOT_EXECUTION_SCOPE_ID)
            .unwrap();
        assert_eq!(
            apply_children(
                source.clone(),
                t,
                text(),
                Arc::new(()),
                Default::default(),
                &c
            )
            .unwrap_err()
            .code,
            "cem.control.scope_completed"
        );
    }
    #[test]
    fn explicit_namespaces_and_qualified_names_stay_native() {
        let source = source();
        let t = target(&source);
        let nodes: Arc<Vec<CemValueNode<(), ()>>> = Arc::new(vec![CemValueNode::Element {
            tag: "context-root".into(),
            namespace: Some("https://cem.dev/ns/runtime/data-island".into()),
            qualified_name: Some("cem-island:context-root".into()),
            attributes: vec![],
            children: vec![],
            source_map: Default::default(),
        }]);
        let r = apply_children(
            source,
            t,
            nodes,
            Arc::new(()),
            Default::default(),
            &control(),
        )
        .unwrap();
        let id = r.inserted_children()[0];
        assert_eq!(
            r.tree()
                .node(id)
                .unwrap()
                .name
                .as_ref()
                .unwrap()
                .namespace_uri,
            "https://cem.dev/ns/runtime/data-island"
        );
        assert_eq!(r.qualified_name(id), Some("cem-island:context-root"));
        assert_eq!(r.tree().source_line_number(id), None);
    }
}

#[cfg(test)]
mod attribute_tests {
    use super::*;
    use crate::{
        import::element_tree::retain_html_elements,
        parser::tree::CemDocumentMetadata,
        scheduler::AbortSignal,
        validation::html::{html_document_ast_from_source_bytes, HtmlSourceValidationRequest},
        value::CemValueAttribute,
    };
    fn source() -> Arc<RetainedCemTree> {
        let doc = html_document_ast_from_source_bytes(HtmlSourceValidationRequest {
            bytes: b"<div id='kept' label='old'><b>Old</b></div>",
            source_uri: "file:///source.html",
            content_type: Some("text/html"),
        })
        .0
        .unwrap();
        retain_html_elements(Arc::new(doc), CemDocumentMetadata::default())
            .unwrap()
            .tree
    }
    fn attr(name: &str, value: &str) -> CemValueAttribute<()> {
        CemValueAttribute {
            name: name.into(),
            namespace: None,
            qualified_name: None,
            value: value.into(),
            value_stream: (),
            contract: None,
            source_map: Default::default(),
        }
    }
    fn target(tree: &RetainedCemTree) -> u32 {
        (0..tree.ast().nodes.len())
            .find(|id| {
                tree.node(*id as u32)
                    .and_then(|n| n.name.as_ref())
                    .is_some_and(|n| n.local_name == "div")
            })
            .unwrap() as u32
    }
    fn edit() -> CemElementEdit<(), ()> {
        CemElementEdit {
            children: Arc::new(vec![CemValueNode::Text {
                text: "New".into(),
                source_map: Default::default(),
            }]),
            attributes: Arc::new(vec![attr("label", "new"), attr("data-theme", "native")]),
        }
    }
    #[test]
    fn attributes_and_children_are_one_derived_transaction() {
        let source = source();
        let t = target(&source);
        let old = source.node(t).unwrap().attributes.clone();
        let c = OperationControl::new(AbortSignal::new());
        let r = apply_element(
            source.clone(),
            t,
            edit(),
            Arc::new(()),
            Default::default(),
            &c,
        )
        .unwrap();
        let attrs = r
            .tree()
            .node(r.output_target())
            .unwrap()
            .attributes
            .iter()
            .map(|id| {
                let n = r.tree().node(*id).unwrap();
                (
                    n.name.as_ref().unwrap().local_name.as_str(),
                    n.value.as_str(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            attrs,
            vec![("id", "kept"), ("label", "new"), ("data-theme", "native")]
        );
        assert_eq!(
            r.tree()
                .text_fragments(r.output_target())
                .unwrap()
                .collect::<String>(),
            "New"
        );
        assert_eq!(source.text_fragments(t).unwrap().collect::<String>(), "Old");
        assert_eq!(source.node(old[1]).unwrap().value, "old");
        assert!(r.output_for_source(old[0]).is_some());
        assert!(r.output_for_source(old[1]).is_some());
        assert_eq!(
            r.tree()
                .source_line_number(r.output_for_source(old[1]).unwrap()),
            None
        );
    }
    #[test]
    fn duplicate_writes_refuse_atomically_and_release_accounting() {
        let source = source();
        let t = target(&source);
        let mut edit = edit();
        edit.attributes = Arc::new(vec![attr("label", "first"), attr("label", "second")]);
        let c = OperationControl::new(AbortSignal::new());
        assert_eq!(
            apply_element(
                source.clone(),
                t,
                edit,
                Arc::new(()),
                Default::default(),
                &c
            )
            .unwrap_err()
            .code,
            "cem.ast.apply_attributes"
        );
        assert_eq!(
            c.memory_charged(crate::operation_control::ROOT_EXECUTION_SCOPE_ID)
                .unwrap(),
            0
        );
        assert_eq!(source.text_fragments(t).unwrap().collect::<String>(), "Old");
    }
    #[test]
    fn late_child_failure_never_publishes_prepared_attributes() {
        let source = source();
        let t = target(&source);
        let c = OperationControl::new(AbortSignal::new());
        let mut edit = edit();
        edit.children = Arc::new(vec![CemValueNode::Reference {
            reference: crate::value::CemReference::new(vec![()]),
            source_map: Default::default(),
        }]);
        assert_eq!(
            apply_element(
                source.clone(),
                t,
                edit,
                Arc::new(()),
                Default::default(),
                &c
            )
            .unwrap_err()
            .code,
            "cem.ast.apply_reference"
        );
        assert_eq!(
            c.memory_charged(crate::operation_control::ROOT_EXECUTION_SCOPE_ID)
                .unwrap(),
            0
        );
        assert_eq!(source.text_fragments(t).unwrap().collect::<String>(), "Old");
    }
}
