//! Per-placement consuming models over immutable original source handles.
use super::*;
use std::collections::HashMap;

/// An explicit child override selected by the consumer. None is an unavailable
/// override, never permission to inherit the enclosing model for its body.
#[derive(Debug, Clone)]
pub struct InputSchemaRegion<'a> {
    pub host: SchemaDeclarationNode,
    pub model: Option<&'a SchemaDocumentModel>,
}

pub(super) struct RegionModels<'a> {
    pub models: Vec<&'a SchemaDocumentModel>,
    available: Vec<bool>,
    boundaries: HashMap<(usize, crate::parser::AstNodeId), Option<usize>>,
}
impl<'a> RegionModels<'a> {
    pub fn new(
        model: &'a SchemaDocumentModel,
        regions: &[InputSchemaRegion<'a>],
    ) -> Result<Self, Vec<Diagnostic>> {
        let mut result = Self {
            models: vec![model],
            available: vec![ready(model)],
            boundaries: HashMap::new(),
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
                result.models.push(model);
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
    pub fn is_boundary(&self, source: &SchemaDeclarationNode) -> bool {
        self.boundaries.contains_key(&key(source))
    }
    pub fn children<'b>(
        &self,
        model: usize,
        source: &'b SchemaDeclarationNode,
    ) -> (&'b [crate::parser::AstNodeId], usize, bool) {
        match self.boundaries.get(&key(source)) {
            Some(Some(index)) if self.available[*index] => {
                let CemAstNode::Element { children, .. } = source.node() else {
                    unreachable!()
                };
                (children, *index, true)
            }
            Some(_) => (&[], model, false),
            None if self.models[model].is_empty() => {
                let children = match source.node() {
                    CemAstNode::Element { children, .. } => children.as_slice(),
                    _ => &[],
                };
                (children, model, true)
            }
            None => (
                consumable_children(source.node(), self.models[model]).unwrap_or(&[]),
                model,
                true,
            ),
        }
    }
    pub fn blockers(&self, source: &SchemaDeclarationNode) -> Vec<Diagnostic> {
        match self.boundaries.get(&key(source)) {
            Some(Some(index)) if !self.available[*index] => {
                self.models[*index].validation_blocker_diagnostics()
            }
            _ => vec![],
        }
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
