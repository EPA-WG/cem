//! Explicit native linking. Immutable deployment records confer no live grant.
use super::{
    attribute_datatypes::*,
    attribute_references::NativeAttributeTargetAccess,
    declaration_references::SchemaDeclarationNode,
    document_model::{self, SchemaDocumentModel},
    input_references::{validate_structural_input_roots_references, InputReferenceHost},
    reference_policy::{ReferenceOccurrence, ReferenceScopePolicy, ReferenceUnresolvedPolicy},
    reference_traversal::ReferenceTraversalLimits,
    structural_capsule::VerifiedStructuralCapsule,
};
use crate::{
    diagnostics::{Diagnostic, Severity},
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::{tree::RetainedCemTree, CemAstNode},
    value::reference_resolution::*,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
const INVALID: &str = "native structural receiver refused";

pub struct StructuralNodeReceiver {
    capsule: Arc<VerifiedStructuralCapsule>,
    declaration: SchemaDeclarationNode,
    receiver: SchemaDocumentModel,
    context: AttributeDatatypeContextLease,
    control: OperationControl,
    active: AtomicBool,
}
impl std::fmt::Debug for StructuralNodeReceiver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StructuralNodeReceiver(..)")
    }
}
impl StructuralNodeReceiver {
    /// Called by a concrete host for one original literal node declaration.
    /// The caller must own the receiving model/source and current context.
    pub fn link(
        capsule: Arc<VerifiedStructuralCapsule>,
        registry_hash: &str,
        declaration: SchemaDeclarationNode,
        receiver: &SchemaDocumentModel,
        context: &AttributeDatatypeContext,
        control: &OperationControl,
    ) -> Result<Arc<Self>, String> {
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID)?;
        if !context.lease().matches(context)
            || capsule.pin().registry_hash != registry_hash
            || !receiver.is_ready_for_structural_validation()
            || !receiver.compile_diagnostics.is_empty()
            || !receiver.attribute_datatypes.is_empty()
        {
            return Err(INVALID.into());
        }
        let CemAstNode::Element {
            expanded_name,
            attributes,
            ..
        } = declaration.node()
        else {
            return Err(INVALID.into());
        };
        if expanded_name.local_name != "attribute"
            || !matches!(
                expanded_name.namespace_uri.as_str(),
                "" | super::registry::CEM_SCHEMA_URI
            )
        {
            return Err(INVALID.into());
        }
        let mut fields = BTreeSet::new();
        for id in attributes {
            let Some(CemAstNode::Attribute {
                expanded_name,
                value_nodes,
                ..
            }) = declaration.document().get(*id)
            else {
                return Err(INVALID.into());
            };
            if !expanded_name.namespace_uri.is_empty()
                || !value_nodes.is_empty()
                || !matches!(
                    expanded_name.local_name.as_str(),
                    "name" | "type" | "itemCount" | "minItems" | "maxItems"
                )
                || !fields.insert(&expanded_name.local_name)
            {
                return Err(INVALID.into());
            }
        }
        let Some(model) =
            document_model::compile_attribute_model(declaration.document(), declaration.node_id())
        else {
            return Err(INVALID.into());
        };
        if !matches!(model.value_type.as_deref(), Some("node" | "schema:node"))
            || receiver.attributes.get(&model.name) != Some(&model)
            || !receiver
                .declaration_references
                .attribute_declarations
                .get(&model.name)
                .is_some_and(|source| {
                    Arc::ptr_eq(source.document(), declaration.document())
                        && source.node_id() == declaration.node_id()
                })
        {
            return Err(INVALID.into());
        }
        let mut rest = model.clone();
        rest.name.clear();
        rest.value_type = None;
        rest.source_map = Default::default();
        rest.item_count = None;
        rest.min_items = None;
        rest.max_items = None;
        if rest != Default::default() {
            return Err(INVALID.into());
        }
        let parse = |s: &Option<String>| {
            s.as_ref()
                .map(|v| v.parse::<usize>().ok().filter(|n| *n <= 64).ok_or(INVALID))
                .transpose()
        };
        let (exact, min, max) = (
            parse(&model.item_count)?,
            parse(&model.min_items)?,
            parse(&model.max_items)?,
        );
        if min.zip(max).is_some_and(|(a, b)| a > b)
            || exact.is_some_and(|n| min.is_some_and(|m| n < m) || max.is_some_and(|m| n > m))
        {
            return Err(INVALID.into());
        }
        let mut receiver = receiver.clone();
        receiver.attributes = BTreeMap::from([(model.name.clone(), model)]);
        Ok(Arc::new(Self {
            capsule,
            declaration,
            receiver,
            context: context.lease(),
            control: control.clone(),
            active: AtomicBool::new(true),
        }))
    }
    fn live(&self, context: &AttributeDatatypeContext) -> bool {
        self.active.load(Ordering::Acquire)
            && self.context.matches(context)
            && self.control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_ok()
    }
}
impl CompiledAttributeDatatype for StructuralNodeReceiver {
    fn receiver_snapshot_matches(&self, model: &SchemaDocumentModel, name: &str) -> bool {
        model.schema_uri == self.receiver.schema_uri
            && model.attributes.get(name) == self.receiver.attributes.get(name)
            && model
                .declaration_references
                .attribute_declarations
                .get(name)
                .is_some_and(|node| {
                    Arc::ptr_eq(node.document(), self.declaration.document())
                        && node.node_id() == self.declaration.node_id()
                })
    }
    fn declaration(&self) -> &SchemaDeclarationNode {
        &self.declaration
    }
    fn is_node_valued(&self) -> bool {
        true
    }
    fn retire_preparations(&self) {
        self.active.store(false, Ordering::Release);
    }
    fn validate(&self, input: AttributeDatatypeInput<'_>) -> AttributeDatatypeValidation {
        let stop = |accepted| {
            let mut d = diagnostic(
                input.source,
                if accepted == Some(false) {
                    "cem.schema_validation.attribute_datatype_invalid"
                } else {
                    "cem.schema_validation.attribute_datatype_incomplete"
                },
            );
            if let Some(tree) = input.source_tree.as_ref() {
                attribute_diagnostic(tree, input.source, &mut d);
            }
            AttributeDatatypeValidation {
                accepted,
                diagnostics: vec![d],
            }
        };
        let AttributeDatatypeValue::Nodes(access) = &input.value else {
            return stop(Some(false));
        };
        let (Some(context), Some(tree)) = (input.context, input.source_tree.as_ref()) else {
            return stop(None);
        };
        if !self.live(context)
            || !access.is_complete()
            || !access.matches_invocation(context, tree, input.source)
            || input.control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_err()
        {
            return stop(None);
        }
        let CemAstNode::Attribute {
            node_id,
            expanded_name,
            ..
        } = input.source
        else {
            return stop(None);
        };
        let Some(model) = self.receiver.attributes.get(&expanded_name.local_name) else {
            return stop(None);
        };
        if !access.matches_receiver(&self.declaration, &self.receiver.schema_uri, model) {
            return stop(None);
        }
        let parent = tree
            .source_parent(*node_id)
            .and_then(|id| tree.ast().get(id));
        if !expanded_name.namespace_uri.is_empty()
            || !matches!(parent,Some(CemAstNode::Element{expanded_name,..}) if expanded_name.local_name==input.element_name)
        {
            return stop(None);
        }
        let mut host = match LiteralAccessHost::new(access, &self.control) {
            Ok(host) => host,
            Err(_) => return stop(None),
        };
        let mut diagnostics = vec![];
        document_model::validate_native_attribute_count(
            &self.receiver,
            input.element_name,
            &model.name,
            access.roots().len(),
            input.source,
            &mut diagnostics,
        );
        for d in &mut diagnostics {
            attribute_diagnostic(tree, input.source, d);
        }
        for index in access.roots() {
            let node = access.node(*index).expect("verified access root");
            let report = match validate_structural_input_roots_references(
                node.document().clone(),
                &[node.node_id()],
                self.capsule.model(),
                &mut host,
                ReferenceTraversalLimits {
                    max_depth: 64,
                    max_work: 100_000,
                },
            ) {
                Ok(report) => report,
                Err(_) => return stop(None),
            };
            if !report.complete {
                return stop(None);
            }
            diagnostics.extend(report.diagnostics);
        }
        if !self.live(context) || input.control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_err() {
            return stop(None);
        }
        let accepted = !diagnostics.iter().any(|d| d.severity.is_hard_violation());
        AttributeDatatypeValidation {
            accepted: Some(accepted),
            diagnostics,
        }
    }
}
fn diagnostic(source: &CemAstNode, code: &str) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        message: INVALID.into(),
        severity: Severity::Error,
        source_map: Some(document_model::source_stack_for_node(source).clone()),
        ..Default::default()
    }
}
fn attribute_diagnostic(tree: &Arc<RetainedCemTree>, source: &CemAstNode, d: &mut Diagnostic) {
    if d.uri.is_some() {
        return;
    }
    let id = match source {
        CemAstNode::Element { node_id, .. } | CemAstNode::Attribute { node_id, .. } => *node_id,
        _ => return,
    };
    d.uri = tree.node_source_uri(id).map(str::to_owned);
    // Native CEM tokenizer frames may cover the complete input. Preserve the
    // stack but prefer the last authored builder span for this diagnostic.
    let authored =
        d.source_map
            .as_ref()
            .and_then(|s| {
                s.frames.iter().rev().find(|f| {
                    matches!(f.transform, crate::source_map::TransformKind::CemAstBuilder)
                })
            })
            .and_then(|f| match &f.span {
                crate::source_map::FrameSpan::Single(r) => Some(r.start),
                crate::source_map::FrameSpan::Multi(r) => r.first().map(|r| r.start),
            });
    let offset = authored.or(d.byte_offset).or_else(|| {
        d.source_map.as_ref()?.origin().and_then(|f| match &f.span {
            crate::source_map::FrameSpan::Single(r) => Some(r.start),
            crate::source_map::FrameSpan::Multi(r) => r.first().map(|r| r.start),
        })
    });
    d.byte_offset = offset;
    if let Some(p) = offset.and_then(|o| tree.source_byte_coordinate(o)) {
        d.line = Some(p.line);
        d.column = Some(p.column);
    } else if let Some(range) = tree.source_node_range(id) {
        if offset == Some(range.offset) && range.line > 0 && range.column > 0 {
            d.line = Some(range.line);
            d.column = Some(range.column);
        }
    }
}

/// A closed view of an already-authorized literal forest. It cannot evaluate
/// references, access ancestors or fetch/parse another resource.
struct LiteralAccessHost<'a> {
    access: &'a NativeAttributeTargetAccess,
    original: BTreeMap<(usize, u32), usize>,
    control: &'a OperationControl,
    work: std::cell::Cell<usize>,
    policy: ReferenceScopePolicy,
}
impl<'a> LiteralAccessHost<'a> {
    fn new(
        access: &'a NativeAttributeTargetAccess,
        control: &'a OperationControl,
    ) -> Result<Self, String> {
        if access.roots().len() > 64 {
            return Err(INVALID.into());
        }
        let mut result = Self {
            access,
            original: BTreeMap::new(),
            control,
            work: std::cell::Cell::new(100_000),
            policy: ReferenceScopePolicy::schema_defaults().map_err(|_| INVALID)?,
        };
        let mut pending = access
            .roots()
            .iter()
            .map(|i| (*i, 0usize))
            .collect::<Vec<_>>();
        let mut seen = BTreeSet::new();
        let mut bytes = 0usize;
        while let Some((index, depth)) = pending.pop() {
            result.poll()?;
            if !seen.insert(index) || seen.len() > 16_384 || depth > 64 {
                return Err(INVALID.into());
            }
            let source = access.node(index).ok_or(INVALID)?;
            let view = access.input_view(index).ok_or(INVALID)?;
            if view
                .source_tree
                .as_ref()
                .is_none_or(|t| !Arc::ptr_eq(t.ast_owner(), source.document()))
            {
                return Err(INVALID.into());
            }
            for (id, name) in &view.names {
                if !matches!(source.document().get(*id),Some(CemAstNode::Element{expanded_name,..}|CemAstNode::Attribute{expanded_name,..}) if name.local_name==expanded_name.local_name)
                {
                    return Err(INVALID.into());
                }
                bytes = bytes
                    .saturating_add(name.namespace_uri.len())
                    .saturating_add(name.local_name.len());
            }
            bytes = bytes.saturating_add(crate::value::artifact::metadata_bytes(
                document_model::source_stack_for_node(source.node()),
            ));
            let key = key(source);
            if let Some(previous) = result.original.insert(key, index) {
                let old = access.input_view(previous).ok_or(INVALID)?;
                if old.names.len() != view.names.len()
                    || old.names.iter().any(|(id, n)| {
                        view.names.get(id).is_none_or(|v| {
                            v.namespace_uri != n.namespace_uri
                                || v.local_name != n.local_name
                                || v.schema_id != n.schema_id
                        })
                    })
                {
                    return Err(INVALID.into());
                }
            }
            let owning: &[u32] = match source.node() {
                CemAstNode::Element {
                    attributes,
                    children: _,
                    ..
                } => {
                    if !view.names.contains_key(&source.node_id())
                        || attributes.iter().any(|id| !view.names.contains_key(id))
                    {
                        return Err(INVALID.into());
                    }
                    let expected = super::input_references::consumed_walk::receiver_owning_edges(
                        source.node(),
                    );
                    check_edges(access, index, &expected)?;
                    &[]
                }
                CemAstNode::Attribute {
                    value, value_nodes, ..
                } if value_nodes.is_empty() => {
                    bytes += value.as_ref().map_or(0, String::len);
                    &[]
                }
                CemAstNode::Text { data, .. }
                | CemAstNode::Whitespace { data, .. }
                | CemAstNode::Cdata { data, .. }
                | CemAstNode::RawText { data, .. }
                | CemAstNode::Comment { data, .. }
                | CemAstNode::ProcessingInstruction { data, .. } => {
                    bytes += data.len();
                    &[]
                }
                _ => return Err(INVALID.into()),
            };
            if !matches!(source.node(), CemAstNode::Element { .. }) {
                check_edges(access, index, owning)?;
            }
            if bytes > 1024 * 1024 {
                return Err(INVALID.into());
            }
            pending.extend(
                access
                    .children(index)
                    .ok_or(INVALID)?
                    .iter()
                    .map(|i| (*i, depth + 1)),
            );
        }
        if access.roots().iter().any(|i| {
            !matches!(
                access.node(*i).map(|n| n.node()),
                Some(CemAstNode::Element { .. })
            )
        }) {
            return Err(INVALID.into());
        }
        Ok(result)
    }
    fn poll(&self) -> Result<(), String> {
        self.work
            .set(self.work.get().checked_sub(1).ok_or(INVALID)?);
        self.control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID.into())
    }
}
fn key(node: &SchemaDeclarationNode) -> (usize, u32) {
    (Arc::as_ptr(node.document()) as usize, node.node_id())
}
fn check_edges(
    access: &NativeAttributeTargetAccess,
    index: usize,
    expected: &[u32],
) -> Result<(), String> {
    let children = access.children(index).ok_or(INVALID)?;
    let source = access.node(index).ok_or(INVALID)?;
    if children.len() != expected.len()
        || children.iter().zip(expected).any(|(i, id)| {
            access.parent(*i) != Some(index)
                || access.node(*i).is_none_or(|n| {
                    !Arc::ptr_eq(n.document(), source.document()) || n.node_id() != *id
                })
        })
    {
        return Err(INVALID.into());
    }
    Ok(())
}
impl ReferenceResolutionHost for LiteralAccessHost<'_> {
    type Node = SchemaDeclarationNode;
    type Scope = ();
    fn prepare_node(&mut self, _: &mut Self::Node) -> Result<(), ReferenceResolutionError> {
        self.poll()
            .map_err(|_| ReferenceResolutionError::OperationStopped)
    }
    fn scope(&self, _: &Self::Node) {}
    fn scope_limits(&self, _: &()) -> ReferenceTraversalLimits {
        self.policy.limits
    }
    fn reference_occurrence(&self, _: &Self::Node) -> Option<ReferenceOccurrence> {
        None
    }
    fn unresolved_policy(&self, _: &Self::Node) -> &ReferenceUnresolvedPolicy {
        &self.policy.unresolved
    }
    fn permits_edge(&self, from: &Self::Node, to: &Self::Node) -> bool {
        self.original.contains_key(&key(from)) && self.original.contains_key(&key(to))
    }
    fn evaluate(&mut self, _: &Self::Node) -> ReferenceLinkEvaluation<Self::Node> {
        ReferenceLinkEvaluation::Pending("literal structural receiver".into())
    }
}
impl InputReferenceHost for LiteralAccessHost<'_> {
    fn source_node(&self, node: SchemaDeclarationNode) -> Self::Node {
        node
    }
    fn retained_node(&self, node: &Self::Node) -> Option<SchemaDeclarationNode> {
        self.original.contains_key(&key(node)).then(|| node.clone())
    }
    fn input_source_tree(&self, node: &SchemaDeclarationNode) -> Option<Arc<RetainedCemTree>> {
        self.access
            .input_view(*self.original.get(&key(node))?)?
            .source_tree
            .clone()
    }
    fn input_expanded_name<'b>(
        &'b self,
        node: &'b SchemaDeclarationNode,
    ) -> Option<&'b crate::parser::ExpandedName> {
        self.poll().ok()?;
        self.access
            .input_view(*self.original.get(&key(node))?)?
            .names
            .get(&node.node_id())
    }
    fn structural_diagnostic(
        &self,
        source: &SchemaDeclarationNode,
        mut d: Diagnostic,
    ) -> Diagnostic {
        if let Some(tree) = self.input_source_tree(source) {
            attribute_diagnostic(&tree, source.node(), &mut d);
        }
        d
    }
}
