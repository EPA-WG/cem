//! Conservative dependencies derived from existing IR, including every function
//! body (not just nodes reachable from the root). No new artifact metadata.
use super::{CompiledQuery, IrNode, IrStep};
use crate::resolve::{BindingId, ModuleUri};
use std::collections::HashSet;

impl CompiledQuery {
    /// `None` requires a complete context and preserves mutable template-host
    /// access. `Some` also proves evaluation can omit that host, allowing CEMT
    /// to borrow its context and the evaluator to borrow selected input streams.
    /// Keep declarations intact for artifact identity/reload.
    pub(crate) fn binding_dependencies(&self) -> Option<HashSet<BindingId>> {
        let functions: HashSet<_> = self
            .tree
            .nodes
            .iter()
            .zip(&self.tree.resolutions)
            .filter_map(|(node, binding)| {
                matches!(node, IrNode::Lambda { .. })
                    .then_some(*binding)
                    .flatten()
            })
            .collect();
        let mut used = HashSet::new();
        for node in &self.tree.nodes {
            match node {
                IrNode::LocalVar(binding) => {
                    used.insert(*binding);
                }
                IrNode::Lambda { captures, .. } => used.extend(captures),
                IrNode::FunctionRef(binding) => {
                    if !functions.contains(binding) {
                        return None;
                    }
                }
                IrNode::Call { callee, .. } => match self.tree.node(*callee) {
                    Some(IrNode::Lambda { .. }) => (),
                    Some(IrNode::FunctionRef(binding)) if functions.contains(binding) => (),
                    _ => return None,
                },
                IrNode::StdlibCall { module, .. } => {
                    if !closed_module(module) {
                        return None;
                    }
                }
                IrNode::Pipeline { steps, .. } => {
                    for step in steps {
                        match step {
                            IrStep::NamedStdlib { module, .. } if !closed_module(module) => {
                                return None
                            }
                            IrStep::Named {
                                binding: Some(binding),
                                ..
                            } if !functions.contains(binding) => return None,
                            IrStep::Method { name, .. }
                                if crate::stdlib::dom::chain_method_arity(&name.local)
                                    .is_none()
                                    && !matches!(
                                        name.local.as_str(),
                                        "drop" | "nth" | "where" | "target"
                                    ) =>
                            {
                                return None
                            }
                            IrStep::NamedStdlib { .. }
                            | IrStep::Named { .. }
                            | IrStep::Method { .. }
                            | IrStep::Lambda(_) => (),
                        }
                    }
                }
                // Explicitly enumerate closed nodes so new IR operations need
                // a dependency review. Child expressions live in the same tree.
                IrNode::LitString(_)
                | IrNode::LitInt(_)
                | IrNode::LitDecimal(_)
                | IrNode::LitDouble(_)
                | IrNode::LitBool(_)
                | IrNode::LitNull
                | IrNode::SchemaType(_)
                | IrNode::TemplateRef(_)
                | IrNode::StateSlot(_)
                | IrNode::Record(_)
                | IrNode::Array(_)
                | IrNode::Sequence(_)
                | IrNode::AxisStep { .. }
                | IrNode::Parent
                | IrNode::Self_
                | IrNode::Reference
                | IrNode::LeadingDot
                | IrNode::BinaryOp { .. }
                | IrNode::UnaryOp { .. }
                | IrNode::SetOp { .. }
                | IrNode::If { .. }
                | IrNode::Let { .. }
                | IrNode::For { .. }
                | IrNode::Quantified { .. }
                | IrNode::InstanceOf { .. }
                | IrNode::CastAs { .. }
                | IrNode::TreatAs { .. }
                | IrNode::Is { .. }
                | IrNode::TryCatch { .. } => (),
            }
        }
        Some(used)
    }
}

fn closed_module(module: &ModuleUri) -> bool {
    // These built-ins invoke only callbacks from this IR and explicit focus /
    // scope / reader / resolver capabilities, all retained in EvaluationContext.
    // Native extensions, CEMT dispatch and unknown modules keep the full context.
    matches!(
        module.0.as_str(),
        "cem:stdlib/sequence"
            | "cem:stdlib/strings"
            | "cem:stdlib/numbers"
            | "cem:stdlib/records"
            | "cem:stdlib/url"
            | "cem:stdlib/dom"
            | "cem:stdlib/datetime"
            | "cem:stdlib/cemml"
            | "cem:stdlib/data"
            | "cem:stdlib/report"
            | "cem:stdlib/content-types"
            | "cem:stdlib/modules"
            | "cem:stdlib/items"
    )
}
