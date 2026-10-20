//! Private native binary output codec. No native/DOM identities or callbacks are
//! exported. Unsupported values remain ordinary uncached renders, without lossy
//! projection or reconstruction of live native owners.
use super::*;
use crate::{
    eval::{AtomValue, Item, ItemStream},
    render::{HostAttributeUpdate, RenderPlanAttribute, RenderPlanNode},
};
use cem_ml::{schema::document_model::AttributeValueContract, source_map::SourceMapStack};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_ITEMS: usize = 16384;
const MAX_DEPTH: usize = 64;
#[derive(Serialize, Deserialize)]
struct OwnedPlan {
    nodes: Vec<Node>,
    updates: Vec<(String, String)>,
}
#[derive(Serialize, Deserialize)]
enum Node {
    Element {
        tag: String,
        namespace: Option<String>,
        qualified_name: Option<String>,
        attributes: Vec<Attribute>,
        children: Vec<Node>,
        source_map: SourceMapStack,
    },
    Text(String, SourceMapStack),
    Comment(String, SourceMapStack),
    Cdata(String, SourceMapStack),
    ProcessingInstruction(String, String, SourceMapStack),
}
#[derive(Serialize, Deserialize)]
struct Attribute {
    name: String,
    namespace: Option<String>,
    qualified_name: Option<String>,
    value: String,
    atoms: Vec<AtomValue>,
    contract: Option<AttributeValueContract>,
    source_map: SourceMapStack,
}
struct Budget<'a> {
    bytes: usize,
    items: usize,
    control: &'a OperationControl,
    scope: ExecutionScopeId,
}
impl Budget<'_> {
    fn charge(&mut self, bytes: usize, depth: usize) -> Option<()> {
        if depth > MAX_DEPTH {
            return None;
        }
        self.control.check_scope(self.scope).ok()?;
        self.bytes = self.bytes.checked_sub(bytes)?;
        self.items = self.items.checked_sub(1)?;
        Some(())
    }
    fn metadata(&mut self, value: &impl Serialize, depth: usize) -> Option<()> {
        let encoded = binary(value, self.control, self.scope)?;
        self.charge(encoded.len(), depth)
    }
}
fn binary(
    value: &impl Serialize,
    control: &OperationControl,
    scope: ExecutionScopeId,
) -> Option<Vec<u8>> {
    struct Bounded<'a>(Vec<u8>, &'a OperationControl, ExecutionScopeId);
    impl Write for Bounded<'_> {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.len() > MAX_BYTES.saturating_sub(self.0.len())
                || self.1.check_scope(self.2).is_err()
            {
                return Err(io::Error::other("pure render output bound/control"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut out = Bounded(Vec::new(), control, scope);
    rmp_serde::encode::write_named(&mut out, value).ok()?;
    Some(out.0)
}
pub(super) fn encode(
    plan: &RenderPlan,
    control: &OperationControl,
    scope: ExecutionScopeId,
) -> Option<Vec<u8>> {
    if !plan.diagnostics.is_empty() {
        return None;
    }
    let mut budget = Budget {
        bytes: MAX_BYTES,
        items: MAX_ITEMS,
        control,
        scope,
    };
    let nodes = plan
        .nodes
        .iter()
        .map(|node| own(node, &mut budget, 0))
        .collect::<Option<Vec<_>>>()?;
    let mut updates = Vec::new();
    for update in &plan.host_attribute_updates {
        budget.metadata(&(&update.name, &update.value), 0)?;
        updates.push((update.name.clone(), update.value.clone()));
    }
    binary(&OwnedPlan { nodes, updates }, control, scope)
}
fn own(node: &RenderPlanNode, budget: &mut Budget<'_>, depth: usize) -> Option<Node> {
    budget.charge(0, depth)?;
    Some(match node {
        RenderPlanNode::Reference { .. } => return None,
        RenderPlanNode::Element {
            tag,
            namespace,
            qualified_name,
            attributes,
            children,
            source_map,
        } => {
            budget.metadata(&(tag, namespace, qualified_name, source_map), depth)?;
            let mut attrs = Vec::new();
            for a in attributes {
                budget.metadata(
                    &(
                        &a.name,
                        &a.namespace,
                        &a.qualified_name,
                        &a.value,
                        &a.source_map,
                        a.contract.as_deref(),
                    ),
                    depth,
                )?;
                let mut atoms = Vec::new();
                for item in a.value_stream.cache_materialized_items()? {
                    let Item::Atomic(atom) = item else {
                        return None;
                    };
                    budget.metadata(atom, depth)?;
                    atoms.push(atom.clone());
                }
                attrs.push(Attribute {
                    name: a.name.clone(),
                    namespace: a.namespace.clone(),
                    qualified_name: a.qualified_name.clone(),
                    value: a.value.clone(),
                    atoms,
                    contract: a.contract.as_deref().cloned(),
                    source_map: a.source_map.clone(),
                });
            }
            Node::Element {
                tag: tag.clone(),
                namespace: namespace.clone(),
                qualified_name: qualified_name.clone(),
                attributes: attrs,
                children: children
                    .iter()
                    .map(|child| own(child, budget, depth + 1))
                    .collect::<Option<Vec<_>>>()?,
                source_map: source_map.clone(),
            }
        }
        RenderPlanNode::Text { text, source_map } => {
            budget.metadata(&(text, source_map), depth)?;
            Node::Text(text.clone(), source_map.clone())
        }
        RenderPlanNode::Comment { text, source_map } => {
            budget.metadata(&(text, source_map), depth)?;
            Node::Comment(text.clone(), source_map.clone())
        }
        RenderPlanNode::Cdata { text, source_map } => {
            budget.metadata(&(text, source_map), depth)?;
            Node::Cdata(text.clone(), source_map.clone())
        }
        RenderPlanNode::ProcessingInstruction {
            target,
            data,
            source_map,
        } => {
            budget.metadata(&(target, data, source_map), depth)?;
            Node::ProcessingInstruction(target.clone(), data.clone(), source_map.clone())
        }
    })
}
pub(super) fn decode(
    bytes: &[u8],
    control: &OperationControl,
    scope: ExecutionScopeId,
) -> Option<RenderPlan> {
    if bytes.len() > MAX_BYTES {
        return None;
    }
    control.check_scope(scope).ok()?;
    // Charge temporary input decoding to this operation, not an earlier scope.
    // Returned-plan heap accounting remains the separate runtime-budget task.
    let tree = control.scope_tree();
    for ancestor in tree.ancestors(scope) {
        let ceiling = tree.scope(ancestor)?.effective_policy.memory_bytes;
        if ceiling.saturating_sub(control.memory_charged(ancestor).ok()?) < bytes.len() as u64 {
            return None;
        }
    }
    let _decode_memory = control
        .charge_memory(scope, bytes.len() as u64, None)
        .ok()?;
    let plan: OwnedPlan = rmp_serde::from_slice(bytes).ok()?;
    let mut budget = Budget {
        bytes: MAX_BYTES,
        items: MAX_ITEMS,
        control,
        scope,
    };
    let nodes = plan
        .nodes
        .into_iter()
        .map(|n| restore(n, &mut budget, 0))
        .collect::<Option<Vec<_>>>()?;
    let mut updates = Vec::new();
    for (name, value) in plan.updates {
        budget.charge(name.len() + value.len(), 0)?;
        updates.push(HostAttributeUpdate::new(name, value));
    }
    control.check_scope(scope).ok()?;
    Some(RenderPlan {
        nodes,
        host_attribute_updates: updates,
        diagnostics: vec![],
    })
}
fn restore(node: Node, budget: &mut Budget<'_>, depth: usize) -> Option<RenderPlanNode> {
    budget.charge(0, depth)?;
    Some(match node {
        Node::Element {
            tag,
            namespace,
            qualified_name,
            attributes,
            children,
            source_map,
        } => {
            let mut attrs = Vec::new();
            for a in attributes {
                budget.charge(0, depth)?;
                for _ in &a.atoms {
                    budget.charge(0, depth)?;
                }
                attrs.push(RenderPlanAttribute {
                    name: a.name,
                    namespace: a.namespace,
                    qualified_name: a.qualified_name,
                    value: a.value,
                    value_stream: ItemStream::from_items(
                        a.atoms.into_iter().map(Item::Atomic).collect(),
                    ),
                    contract: a.contract.map(std::sync::Arc::new),
                    source_map: a.source_map,
                });
            }
            RenderPlanNode::Element {
                tag,
                namespace,
                qualified_name,
                attributes: attrs,
                children: children
                    .into_iter()
                    .map(|child| restore(child, budget, depth + 1))
                    .collect::<Option<Vec<_>>>()?,
                source_map,
            }
        }
        Node::Text(text, source_map) => RenderPlanNode::Text { text, source_map },
        Node::Comment(text, source_map) => RenderPlanNode::Comment { text, source_map },
        Node::Cdata(text, source_map) => RenderPlanNode::Cdata { text, source_map },
        Node::ProcessingInstruction(target, data, source_map) => {
            RenderPlanNode::ProcessingInstruction {
                target,
                data,
                source_map,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cem_ml::{operation_control::ROOT_EXECUTION_SCOPE_ID, value::CemReference};
    #[test]
    fn reference_values_and_output_bounds_refuse_without_lossy_projection() {
        let control = OperationControl::default();
        let scope = ROOT_EXECUTION_SCOPE_ID;
        for node in [
            RenderPlanNode::Reference {
                reference: CemReference::new(vec![Item::Atomic(AtomValue::Integer(1))]),
                source_map: Default::default(),
            },
            RenderPlanNode::Text {
                text: "x".repeat(MAX_BYTES + 1),
                source_map: Default::default(),
            },
        ] {
            let plan = RenderPlan {
                nodes: vec![node],
                host_attribute_updates: vec![],
                diagnostics: vec![],
            };
            assert!(encode(&plan, &control, scope).is_none());
        }
        assert!(decode(b"invalid output", &control, scope).is_none());
        assert!(control.check_scope(scope).is_ok());
    }
    #[test]
    fn native_owned_atomic_types_roundtrip_without_json_float_aliases() {
        let atoms = vec![
            AtomValue::Integer(42),
            AtomValue::Double(f64::INFINITY),
            AtomValue::Decimal("2.50".into()),
            AtomValue::Boolean(true),
            AtomValue::AnyUri("relative/path".into()),
            AtomValue::Null,
        ];
        let plan = RenderPlan {
            nodes: vec![RenderPlanNode::Element {
                tag: "p".into(),
                namespace: None,
                qualified_name: None,
                attributes: vec![RenderPlanAttribute {
                    name: "value".into(),
                    namespace: None,
                    qualified_name: None,
                    value: "projection".into(),
                    value_stream: ItemStream::from_items(
                        atoms.into_iter().map(Item::Atomic).collect(),
                    ),
                    contract: None,
                    source_map: Default::default(),
                }],
                children: vec![],
                source_map: Default::default(),
            }],
            host_attribute_updates: vec![],
            diagnostics: vec![],
        };
        let control = OperationControl::default();
        let scope = ROOT_EXECUTION_SCOPE_ID;
        let bytes = encode(&plan, &control, scope).unwrap();
        let decoded = decode(&bytes, &control, scope).unwrap();
        assert_eq!(plan.nodes, decoded.nodes);
        assert_eq!(control.memory_charged(scope).unwrap(), 0);
    }
}
