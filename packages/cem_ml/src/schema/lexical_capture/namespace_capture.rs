//! Bind declaration events to actual builder handles, without parsing AST text.
use super::super::{CemSchemaMachine, DirectiveKind, EventNormalizer, NormalizedEvent};
use crate::{events::ScalarValue, parser::AstNodeId, schema::namespace::NamespaceBinding};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct NamespaceDeclarationCapture {
    frames: Vec<Option<AstNodeId>>,
    opening: bool,
    attribute: Option<NamespaceBinding>,
    bindings: BTreeMap<AstNodeId, NamespaceBinding>,
}
impl NamespaceDeclarationCapture {
    pub(super) fn consume<E: EventNormalizer>(
        &mut self,
        machine: &mut CemSchemaMachine<E>,
        event: &NormalizedEvent,
    ) {
        let directive = matches!(event, NormalizedEvent::CloseScope { .. })
            && matches!(
                machine.active_directive,
                Some(DirectiveKind::Ns | DirectiveKind::Default)
            );
        let attribute = matches!(event, NormalizedEvent::Value { value, .. }
            if !matches!(value, ScalarValue::Expression(_)))
            && machine
                .pending_attr
                .as_ref()
                .is_some_and(|attr| attr.name == "xmlns" || attr.name.starts_with("xmlns:"));
        let count = machine.current_ns_context().local_bindings().len();
        self.attribute = None;
        machine.consume(event.clone());
        if directive || attribute {
            let declared = machine.current_ns_context().local_bindings();
            if declared.len() > count {
                let binding = declared.last().unwrap().clone();
                if directive {
                    if let Some(Some(node)) = self.frames.last() {
                        self.bindings.insert(*node, binding);
                    }
                } else {
                    self.attribute = Some(binding);
                }
            }
        }
        self.opening = matches!(event, NormalizedEvent::OpenScope { .. });
        if self.opening {
            self.frames.push(None);
        } else if matches!(event, NormalizedEvent::CloseScope { .. }) {
            self.frames.pop();
        }
    }
    pub(super) fn observed(&mut self, node: Option<AstNodeId>, attribute: Option<AstNodeId>) {
        if self.opening {
            if let Some(frame) = self.frames.last_mut() {
                *frame = node;
            }
        }
        if let (Some(node), Some(binding)) = (attribute, self.attribute.take()) {
            self.bindings.insert(node, binding);
        }
    }
    pub(super) fn take_bindings(&mut self) -> BTreeMap<AstNodeId, NamespaceBinding> {
        std::mem::take(&mut self.bindings)
    }
}
