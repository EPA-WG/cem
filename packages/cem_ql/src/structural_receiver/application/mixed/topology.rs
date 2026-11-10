//! Admission of the original retained subtree before consuming receiver values.
use super::*;
use crate::eval::RetainedCemNode;
use cem_ml::schema::{
    declaration_references::SchemaDeclarationHost,
    document_model::validate_retained_field_contracts,
};

pub(crate) struct FrameElement {
    pub(super) name: String,
    pub(super) attributes: BTreeMap<String, AstNodeId>,
    pub(super) children: Vec<FrameElement>,
}

impl FrameElement {
    pub(crate) fn contains_attribute(&self, attribute: AstNodeId) -> bool {
        self.attributes.values().any(|&id| id == attribute)
            || self
                .children
                .iter()
                .any(|child| child.contains_attribute(attribute))
    }
}

impl LinkedMixedReceivers {
    pub(crate) fn check_frame_input(
        &self,
        host: &mut FreshStructuralHost,
        root: &RetainedCemNode,
        control: Option<&OperationControl>,
    ) -> Result<FrameElement, String> {
        self.check_topology(host, root, control, true)
    }

    pub(super) fn check_topology(
        &self,
        host: &mut FreshStructuralHost,
        root: &RetainedCemNode,
        control: Option<&OperationControl>,
        instance_root: bool,
    ) -> Result<FrameElement, String> {
        let _entry = self.group.enter(host)?;
        self.group.refresh(host)?;
        let tree = root.owner();
        let limits = crate::instance_island::LIMITS;
        let inside_instance = instance_root
            && std::iter::successors(tree.source_parent(root.node_id()), |&id| {
                tree.source_parent(id)
            })
            .any(|id| {
                matches!(tree.ast().get(id), Some(CemAstNode::Element { expanded_name, .. })
            if self.group.model.elements.contains_key(&expanded_name.local_name))
            });
        if self
            .group
            .receivers
            .keys()
            .any(|name| !crate::application_runtime::inputs::name_allowed(name))
            || host.registered_source_scope(tree).is_none()
            || tree.ast().nodes.len() > limits.max_values
            || inside_instance
        {
            return Err(REFUSED.into());
        }
        let mut walk = Walk {
            receivers: self,
            host,
            tree,
            control: control.unwrap_or(&host.control),
            remaining_work: 100_000,
            remaining_bytes: limits.max_bytes,
        };
        let result = walk.element(root.node_id(), limits.max_depth, !instance_root)?;
        self.group.refresh(host)?;
        Ok(result)
    }
}

struct Walk<'a> {
    receivers: &'a LinkedMixedReceivers,
    host: &'a FreshStructuralHost,
    tree: &'a Arc<RetainedCemTree>,
    control: &'a OperationControl,
    remaining_work: usize,
    remaining_bytes: usize,
}

impl Walk<'_> {
    fn matches_name(
        &self,
        node: &SchemaDeclarationNode,
        name: &cem_ml::parser::ExpandedName,
    ) -> bool {
        name.namespace_uri.is_empty()
            && SchemaDeclarationHost::input_expanded_name(&self.host.inner, node).is_some_and(
                |actual| actual.namespace_uri.is_empty() && actual.local_name == name.local_name,
            )
    }

    fn element(
        &mut self,
        id: AstNodeId,
        depth: usize,
        nested: bool,
    ) -> Result<FrameElement, String> {
        self.control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| REFUSED)?;
        let model = &self.receivers.group.model;
        let declaration =
            SchemaDeclarationNode::new(self.tree.ast_owner().clone(), id).ok_or(REFUSED)?;
        let CemAstNode::Element {
            expanded_name,
            attributes,
            children,
            ..
        } = declaration.node()
        else {
            return Err(REFUSED.into());
        };
        if depth == 0
            || attributes.len() > 64
            || !self.matches_name(&declaration, expanded_name)
            || self
                .host
                .inner
                .scope(&self.host.inner.source_reference(declaration.clone()))
                .is_none()
        {
            return Err(REFUSED.into());
        }
        let name = &expanded_name.local_name;
        let element = model.element(name).ok_or(REFUSED)?;
        let work = 1 + attributes.len() + children.len();
        let contract_work = element
            .field_contracts
            .iter()
            .fold(work, |total, contract| {
                // Sequence matching can compare a full rule at each child position.
                let comparisons = 1
                    + contract.required_child_sequence.len()
                    + contract.forbidden_child_sequence.len();
                total.saturating_add(work.saturating_mul(comparisons))
            });
        self.remaining_work = self
            .remaining_work
            .checked_sub(contract_work)
            .ok_or(REFUSED)?;
        if !nested
            && element
                .required_attributes
                .iter()
                .chain(&element.optional_attributes)
                .any(|name| !self.receivers.group.receivers.contains_key(name))
        {
            return Err(REFUSED.into());
        }
        let mut fields = BTreeMap::new();
        for &attribute in attributes {
            let original = SchemaDeclarationNode::new(self.tree.ast_owner().clone(), attribute)
                .ok_or(REFUSED)?;
            let CemAstNode::Attribute {
                expanded_name,
                value,
                ..
            } = original.node()
            else {
                return Err(REFUSED.into());
            };
            let field = &expanded_name.local_name;
            if !self.matches_name(&original, expanded_name)
                || !(element.required_attributes.contains(field)
                    || element.optional_attributes.contains(field))
                || !(self.receivers.group.receivers.contains_key(field)
                    || nested && self.receivers.group.nested_scalars.contains_key(field))
                || fields.insert(field.clone(), attribute).is_some()
            {
                return Err(REFUSED.into());
            }
            self.remaining_bytes = self
                .remaining_bytes
                .checked_sub(value.as_ref().map_or(0, String::len))
                .ok_or(REFUSED)?;
            values::input(
                self.host,
                field,
                &ReceiverInput {
                    source: self.tree.clone(),
                    attribute,
                },
            )?;
        }
        if !element
            .required_attributes
            .iter()
            .all(|name| fields.contains_key(name))
        {
            return Err(REFUSED.into());
        }
        let mut nested = Vec::new();
        for &child in children {
            match self.tree.ast().get(child) {
                Some(CemAstNode::Element { expanded_name, .. }) => {
                    if !element.allow_any_child
                        && !element.child_elements.contains(&expanded_name.local_name)
                    {
                        return Err(REFUSED.into());
                    }
                    nested.push(self.element(child, depth - 1, true)?);
                }
                Some(CemAstNode::Comment { data, .. }) => {
                    self.remaining_bytes = self
                        .remaining_bytes
                        .checked_sub(data.len())
                        .ok_or(REFUSED)?;
                }
                Some(CemAstNode::Text { data, .. } | CemAstNode::Whitespace { data, .. }) => {
                    self.remaining_bytes = self
                        .remaining_bytes
                        .checked_sub(data.len())
                        .ok_or(REFUSED)?;
                    if !data.trim().is_empty() {
                        return Err(REFUSED.into());
                    }
                }
                _ => return Err(REFUSED.into()),
            }
        }
        if !element.field_contracts.is_empty() {
            let sequence: Vec<_> = nested.iter().map(|child| child.name.clone()).collect();
            let diagnostics =
                validate_retained_field_contracts(&declaration, model, &sequence).ok_or(REFUSED)?;
            if diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity.is_hard_violation())
            {
                return Err(REFUSED.into());
            }
        }
        Ok(FrameElement {
            name: name.clone(),
            attributes: fields,
            children: nested,
        })
    }
}
