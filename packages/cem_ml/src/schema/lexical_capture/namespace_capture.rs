//! Bind declaration effects and pending uses to actual builder handles.
use super::super::{CemSchemaMachine, DirectiveKind, EventNormalizer, NormalizedEvent};
use crate::{
    events::ScalarValue,
    parser::AstNodeId,
    schema::{
        namespace::NamespaceBinding,
        namespace_references::{
            PendingNamespaceDeclaration, PendingNamespaceName, PendingNamespaceValue,
        },
    },
};
use std::collections::{BTreeMap, VecDeque};
struct Frame {
    node: Option<AstNodeId>,
    directive: bool,
}
enum Declaration {
    Complete(NamespaceBinding),
    Pending(PendingNamespaceDeclaration),
}
pub(super) struct NamespaceDeclarationCapture {
    frames: Vec<Frame>,
    contexts: Vec<BTreeMap<String, Option<AstNodeId>>>,
    opening: bool,
    opening_name: Option<PendingNamespaceName>,
    attribute_names: VecDeque<Option<PendingNamespaceName>>,
    attribute: Option<Declaration>,
    bindings: BTreeMap<AstNodeId, NamespaceBinding>,
    declarations: BTreeMap<AstNodeId, PendingNamespaceDeclaration>,
    names: BTreeMap<AstNodeId, PendingNamespaceName>,
}
impl Default for NamespaceDeclarationCapture {
    fn default() -> Self {
        Self {
            frames: vec![],
            contexts: vec![BTreeMap::new()],
            opening: false,
            opening_name: None,
            attribute_names: VecDeque::new(),
            attribute: None,
            bindings: BTreeMap::new(),
            declarations: BTreeMap::new(),
            names: BTreeMap::new(),
        }
    }
}
impl NamespaceDeclarationCapture {
    fn pending(&self, prefix: &str) -> Option<AstNodeId> {
        for context in self.contexts.iter().rev() {
            if let Some(declaration) = context.get(prefix) {
                return *declaration;
            }
        }
        None
    }
    fn pending_name(&self, lexical: &str) -> Option<PendingNamespaceName> {
        if lexical.starts_with('@') || lexical == "xmlns" || lexical.starts_with("xmlns:") {
            return None;
        }
        let (prefix, local_name) = lexical.split_once(':').unwrap_or(("", lexical));
        Some(PendingNamespaceName {
            declaration: self.pending(prefix)?,
            local_name: local_name.into(),
        })
    }
    pub(super) fn pending_bindings(&self) -> BTreeMap<String, AstNodeId> {
        let mut bindings = BTreeMap::new();
        for context in &self.contexts {
            for (prefix, node) in context {
                match node {
                    Some(node) => {
                        bindings.insert(prefix.clone(), *node);
                    }
                    None => {
                        bindings.remove(prefix);
                    }
                }
            }
        }
        bindings
    }
    fn declared(&mut self, node: AstNodeId, declaration: Declaration) {
        let (prefix, pending) = match declaration {
            Declaration::Complete(binding) => {
                let prefix = binding.name.clone();
                self.bindings.insert(node, binding);
                (prefix, None)
            }
            Declaration::Pending(declaration) => {
                let prefix = declaration.prefix.clone();
                self.declarations.insert(node, declaration);
                (prefix, Some(node))
            }
        };
        self.contexts.last_mut().unwrap().insert(prefix, pending);
    }
    pub(super) fn consume<E: EventNormalizer>(
        &mut self,
        machine: &mut CemSchemaMachine<E>,
        event: &NormalizedEvent,
    ) {
        self.opening_name = match event {
            NormalizedEvent::OpenScope { name, .. } => self.pending_name(&name.lexical_name),
            _ => None,
        };
        if let NormalizedEvent::Name { name, .. } = event {
            self.attribute_names
                .push_back(self.pending_name(&name.lexical_name));
        }
        let directive = matches!(event, NormalizedEvent::CloseScope { .. })
            && matches!(
                machine.active_directive,
                Some(DirectiveKind::Ns | DirectiveKind::Default)
            );
        let alias = if directive && matches!(machine.active_directive, Some(DirectiveKind::Default))
        {
            let token = machine
                .pending_directive_body
                .trim()
                .trim_matches('"')
                .trim();
            if token.is_empty() {
                None
            } else {
                self.pending(token)
            }
        } else {
            None
        };
        let prefix = if matches!(event, NormalizedEvent::Value { .. }) {
            machine.pending_attr.as_ref().and_then(|attr| {
                if attr.name == "xmlns" {
                    Some(String::new())
                } else {
                    attr.name.strip_prefix("xmlns:").map(str::to_owned)
                }
            })
        } else {
            None
        };
        let native = matches!(
            event,
            NormalizedEvent::Value {
                value: ScalarValue::Expression(_),
                ..
            }
        );
        let count = machine.current_ns_context().local_bindings().len();
        self.attribute = None;
        machine.consume(event.clone());
        if let Some(prefix) = prefix.clone().filter(|_| native) {
            machine
                .ns_contexts
                .last_mut()
                .unwrap()
                .defer_binding(&prefix);
            self.attribute = Some(Declaration::Pending(PendingNamespaceDeclaration {
                prefix,
                value: PendingNamespaceValue::Native,
            }));
        } else if directive || prefix.is_some() {
            let declared = machine.current_ns_context().local_bindings();
            if declared.len() > count {
                let binding = declared.last().unwrap().clone();
                let declaration = if let Some(alias) = alias {
                    machine.ns_contexts.last_mut().unwrap().defer_binding("");
                    Declaration::Pending(PendingNamespaceDeclaration {
                        prefix: String::new(),
                        value: PendingNamespaceValue::Alias(alias),
                    })
                } else {
                    Declaration::Complete(binding)
                };
                if directive {
                    if let Some(node) = self.frames.last().and_then(|frame| frame.node) {
                        self.declared(node, declaration);
                    }
                } else {
                    self.attribute = Some(declaration);
                }
            }
        }
        self.opening = matches!(event, NormalizedEvent::OpenScope { .. });
        if let NormalizedEvent::OpenScope { name, .. } = event {
            let directive = name.lexical_name.starts_with('@');
            self.frames.push(Frame {
                node: None,
                directive,
            });
            if !directive {
                self.contexts.push(BTreeMap::new());
            }
        } else if matches!(event, NormalizedEvent::CloseScope { .. }) {
            if self.frames.pop().is_some_and(|frame| !frame.directive) {
                self.contexts.pop();
            }
        }
    }
    pub(super) fn observed(&mut self, node: Option<AstNodeId>, attribute: Option<AstNodeId>) {
        if self.opening {
            if let Some(frame) = self.frames.last_mut() {
                frame.node = node;
            }
            if let (Some(node), Some(name)) = (node, self.opening_name.take()) {
                self.names.insert(node, name);
            }
        }
        if let Some(node) = attribute {
            if let Some(Some(name)) = self.attribute_names.pop_front() {
                self.names.insert(node, name);
            }
            if let Some(declaration) = self.attribute.take() {
                self.declared(node, declaration);
            }
        }
    }
    pub(super) fn take_bindings(&mut self) -> BTreeMap<AstNodeId, NamespaceBinding> {
        std::mem::take(&mut self.bindings)
    }
    pub(super) fn take_pending(
        &mut self,
    ) -> (
        BTreeMap<AstNodeId, PendingNamespaceDeclaration>,
        BTreeMap<AstNodeId, PendingNamespaceName>,
    ) {
        (
            std::mem::take(&mut self.declarations),
            std::mem::take(&mut self.names),
        )
    }
}
