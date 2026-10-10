//! Per-placement consuming models over immutable original source handles.
use super::*;
use std::collections::{BTreeMap, HashMap};

/// An explicit child override selected by the consumer. None is an unavailable
/// override, never permission to inherit the enclosing model for its body.
#[derive(Debug, Clone)]
pub struct InputSchemaRegion<'a> {
    pub host: SchemaDeclarationNode,
    pub model: Option<&'a SchemaDocumentModel>,
}

/// Prepared metadata supplied on first entry to an original element host.
/// The compiled model is shared consumer state, never a copied source arena.
#[derive(Debug, Clone)]
pub struct DiscoveredInputSchemaRegion {
    pub contract: crate::schema::scope_controls::SchemaHostControlContract,
    pub model: Option<Arc<SchemaDocumentModel>>,
    pub diagnostics: Vec<Diagnostic>,
}

enum RegionModel<'a> {
    Borrowed(&'a SchemaDocumentModel),
    Prepared(Arc<SchemaDocumentModel>),
}
impl RegionModel<'_> {
    fn model(&self) -> &SchemaDocumentModel {
        match self {
            Self::Borrowed(model) => model,
            Self::Prepared(model) => model,
        }
    }
}

pub(super) struct RegionModels<'a> {
    models: Vec<RegionModel<'a>>,
    discovering: bool,
    inspected: HashMap<(usize, crate::parser::AstNodeId), SchemaDeclarationNode>,
    available: Vec<bool>,
    boundaries: HashMap<(usize, crate::parser::AstNodeId), Option<usize>>,
    controls: HashMap<(usize, crate::parser::AstNodeId), RegionControl>,
    positions: HashMap<usize, HashMap<crate::parser::AstNodeId, (crate::parser::AstNodeId, usize)>>,
    following: HashMap<(usize, crate::parser::AstNodeId), BTreeMap<usize, Option<usize>>>,
}
struct RegionControl {
    attributes: HashSet<crate::parser::AstNodeId>,
    blocked: bool,
    diagnostics: Vec<Diagnostic>,
}
impl<'a> RegionModels<'a> {
    pub fn new(
        model: &'a SchemaDocumentModel,
        regions: &[InputSchemaRegion<'a>],
    ) -> Result<Self, Vec<Diagnostic>> {
        let mut result = Self {
            models: vec![RegionModel::Borrowed(model)],
            discovering: false,
            inspected: HashMap::new(),
            available: vec![ready(model)],
            boundaries: HashMap::new(),
            controls: HashMap::new(),
            positions: HashMap::new(),
            following: HashMap::new(),
        };
        let mut diagnostics = vec![];
        for region in regions {
            let key = key(&region.host);
            if !matches!(region.host.node(), CemAstNode::Element { .. })
                || result.boundaries.contains_key(&key)
            {
                diagnostics.push(invalid_target(
                    "Expected distinct original element hosts for child schema regions",
                    document_model::source_stack_for_node(region.host.node()).clone(),
                ));
                continue;
            }
            let index = region.model.map(|model| {
                let index = result.models.len();
                result.models.push(RegionModel::Borrowed(model));
                result.available.push(ready(model));
                index
            });
            result.boundaries.insert(key, index);
        }
        if diagnostics.is_empty() {
            Ok(result)
        } else {
            Err(diagnostics)
        }
    }
    pub fn len(&self) -> usize {
        self.models.len()
    }
    pub fn model(&self, index: usize) -> &SchemaDocumentModel {
        self.models[index].model()
    }
    pub fn enable_discovery(&mut self) {
        self.discovering = true;
    }
    pub fn discover<H, D>(
        &mut self,
        source: &SchemaDeclarationNode,
        host: &mut H,
        limits: ReferenceTraversalLimits,
        scheduler: &mut D,
    ) -> Result<(), ReferenceResolutionError>
    where
        H: InputReferenceHost,
        D: FnMut(
            &mut H,
            SchemaDeclarationNode,
            ReferenceTraversalLimits,
        ) -> Result<Option<DiscoveredInputSchemaRegion>, ReferenceResolutionError>,
    {
        let source_key = key(source);
        if !self.discovering
            || !matches!(source.node(), CemAstNode::Element { .. })
            || self.inspected.contains_key(&source_key)
        {
            return Ok(());
        }
        self.inspected.insert(source_key, source.clone());
        let Some(region) = scheduler(host, source.clone(), limits)? else {
            return Ok(());
        };
        if key(region.contract.host()) != source_key {
            self.boundaries.insert(source_key, None);
            self.controls.insert(
                source_key,
                RegionControl {
                    attributes: HashSet::new(),
                    blocked: true,
                    diagnostics: vec![invalid_target(
                        "Discovered controls require the same original element host",
                        document_model::source_stack_for_node(source.node()).clone(),
                    )],
                },
            );
            return Ok(());
        }
        if !region.contract.has_override() {
            return Ok(());
        }
        let index = region.model.map(|model| {
            let index = self.models.len();
            self.available.push(ready(&model));
            self.models.push(RegionModel::Prepared(model));
            index
        });
        let following = region.contract.extent()
            == crate::schema::scope_controls::SchemaScopeControlExtent::Following;
        if following {
            self.index_positions(source);
            let (parent, position) = self.positions[&source_key.0]
                .get(&source.node_id())
                .copied()
                .ok_or(ReferenceResolutionError::InvalidScopeHandoff)?;
            self.following
                .entry((source_key.0, parent))
                .or_default()
                .insert(position + 1, index);
        } else {
            self.boundaries.insert(source_key, index);
        }
        let mut attributes: HashSet<_> = region.contract.attributes().iter().copied().collect();
        attributes.extend(region.contract.pending_attributes().iter().copied());
        let mut diagnostics = region.contract.diagnostics();
        diagnostics.extend(region.diagnostics);
        self.controls.insert(
            source_key,
            RegionControl {
                attributes,
                blocked: region.contract.issue().is_some()
                    || (following && !index.is_some_and(|index| self.available[index])),
                diagnostics,
            },
        );
        Ok(())
    }
    // Scalar original owning positions, never a second AST or target projection.
    // Index each arena once when its first following control is entered.
    fn index_positions(&mut self, source: &SchemaDeclarationNode) {
        self.positions.entry(key(source).0).or_insert_with(|| {
            source
                .document()
                .nodes
                .iter()
                .enumerate()
                .flat_map(|(parent, node)| {
                    let children: &[crate::parser::AstNodeId] = match node {
                        CemAstNode::Document { root_children, .. } => root_children,
                        CemAstNode::Element { children, .. } => children,
                        _ => &[],
                    };
                    children
                        .iter()
                        .enumerate()
                        .map(move |(index, id)| (*id, (parent as crate::parser::AstNodeId, index)))
                })
                .collect()
        });
    }
    /// A following override starts after the original control and ends with its
    /// containing scope. Nearest body overrides still take precedence. None is
    /// unavailable governance, not permission to use the preceding model.
    pub fn effective_model(&self, model: usize, source: &SchemaDeclarationNode) -> Option<usize> {
        let owner = key(source).0;
        let Some(positions) = self.positions.get(&owner) else {
            return Some(model);
        };
        let mut child = source.node_id();
        let mut remaining = positions.len();
        while let Some((parent, index)) = positions.get(&child) {
            // Malformed owning cycles cannot make metadata lookup unbounded.
            if remaining == 0 {
                return None;
            }
            remaining -= 1;
            if let Some((_, selected)) = self
                .following
                .get(&(owner, *parent))
                .and_then(|transitions| transitions.range(..=*index).next_back())
            {
                return selected.filter(|index| self.available[*index]);
            }
            if let Some(selected) = self.boundaries.get(&(owner, *parent)) {
                return selected.filter(|index| self.available[*index]);
            }
            child = *parent;
        }
        Some(model)
    }
    // A reference placement starts under its consuming model. Only original
    // owning child edges inside that placement admit local following controls;
    // the target's external source ancestry cannot replace the consumer model.
    pub fn selected_child_model(
        &self,
        model: usize,
        source: &SchemaDeclarationNode,
        parent: Option<&SchemaDeclarationNode>,
    ) -> Option<usize> {
        let owner = key(source).0;
        let Some((parent_id, position)) = self
            .positions
            .get(&owner)
            .and_then(|positions| positions.get(&source.node_id()))
        else {
            return Some(model);
        };
        if !parent.is_some_and(|parent| key(parent) == (owner, *parent_id)) {
            return Some(model);
        }
        match self
            .following
            .get(&(owner, *parent_id))
            .and_then(|transitions| transitions.range(..=*position).next_back())
        {
            Some((_, selected)) => selected.filter(|index| self.available[*index]),
            None => Some(model),
        }
    }
    pub fn with_controls(
        mut self,
        contracts: &[crate::schema::scope_controls::SchemaHostControlContract],
    ) -> Result<Self, Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        for contract in contracts {
            if !contract.has_override() {
                continue;
            }
            let key = key(contract.host());
            if contract.extent() != crate::schema::scope_controls::SchemaScopeControlExtent::Body
                || !self.boundaries.contains_key(&key)
                || self.controls.contains_key(&key)
            {
                diagnostics.push(invalid_target(
                    "Shared body control contracts require distinct matching original region hosts",
                    document_model::source_stack_for_node(contract.host().node()).clone(),
                ));
                continue;
            }
            let mut attributes: HashSet<_> = contract.attributes().iter().copied().collect();
            // Unclassified attributes are deferred while metadata is pending;
            // the blocked body prevents this from becoming a validity exemption.
            attributes.extend(contract.pending_attributes().iter().copied());
            self.controls.insert(
                key,
                RegionControl {
                    attributes,
                    blocked: contract.issue().is_some(),
                    diagnostics: contract.diagnostics(),
                },
            );
        }
        if diagnostics.is_empty() {
            Ok(self)
        } else {
            Err(diagnostics)
        }
    }
    pub fn control_attributes(
        &self,
        source: &SchemaDeclarationNode,
    ) -> Option<&HashSet<crate::parser::AstNodeId>> {
        self.controls
            .get(&key(source))
            .map(|control| &control.attributes)
    }
    pub fn is_boundary(&self, source: &SchemaDeclarationNode) -> bool {
        self.boundaries.contains_key(&key(source))
    }
    pub fn children<'b>(
        &self,
        model: usize,
        source: &'b SchemaDeclarationNode,
    ) -> (&'b [crate::parser::AstNodeId], usize, bool) {
        if self
            .controls
            .get(&key(source))
            .is_some_and(|control| control.blocked)
        {
            return (&[], model, false);
        }
        // Directive children are control payloads even when the consuming model
        // is empty. Only their explicit lifecycle may evaluate native slots.
        if matches!(source.node(), CemAstNode::Element { expanded_name, .. }
            if expanded_name.local_name.starts_with('@'))
        {
            return (&[], model, true);
        }
        match self.boundaries.get(&key(source)) {
            Some(Some(index)) if self.available[*index] => {
                let CemAstNode::Element { children, .. } = source.node() else {
                    unreachable!()
                };
                (children, *index, true)
            }
            Some(_) => (&[], model, false),
            None if self.model(model).is_empty() => {
                let children = match source.node() {
                    CemAstNode::Element { children, .. } => children.as_slice(),
                    _ => &[],
                };
                (children, model, true)
            }
            None => (
                consumable_children(source.node(), self.model(model)).unwrap_or(&[]),
                model,
                true,
            ),
        }
    }
    pub fn blockers(&self, source: &SchemaDeclarationNode) -> Vec<Diagnostic> {
        let mut diagnostics = self
            .controls
            .get(&key(source))
            .map_or_else(Vec::new, |control| control.diagnostics.clone());
        if let Some(Some(index)) = self.boundaries.get(&key(source)) {
            if !self.available[*index]
                && !self
                    .controls
                    .get(&key(source))
                    .is_some_and(|control| control.blocked)
            {
                diagnostics.extend(self.model(*index).validation_blocker_diagnostics());
            }
        }
        diagnostics
    }
}
fn key(source: &SchemaDeclarationNode) -> (usize, crate::parser::AstNodeId) {
    (Arc::as_ptr(source.document()) as usize, source.node_id())
}
fn ready(model: &SchemaDocumentModel) -> bool {
    model.is_ready_for_validation()
        && !model
            .compile_diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
}
