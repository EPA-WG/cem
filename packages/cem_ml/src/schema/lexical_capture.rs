//! Opt-in source-position observation while forwarding one normalized stream.
use super::{CemSchemaMachine, EventNormalizer, NormalizedEvent};
use crate::{
    diagnostics::Diagnostic,
    parser::{builder::CemAstBuilder, document::CemDocument, AstNodeId, CemAstNode, ExpandedName},
    schema::{
        declaration_references::SchemaDeclarationNode,
        namespace::{NamespaceBinding, NsContext},
        namespace_references::{PendingNamespaceDeclaration, PendingNamespaceName},
        scoping::SchemaScopeFrame,
    },
};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};
use serde::{Deserialize, Serialize};
#[path = "lexical_capture/reload.rs"]
pub(crate) mod reload;
pub use reload::LexicalReloadMetadata;

#[path = "lexical_capture/namespace_capture.rs"]
mod namespace_capture;
use namespace_capture::NamespaceDeclarationCapture;

/// Original source form, distinct even when both elements have no child nodes.
/// Importers retain this event metadata; it cannot be inferred from a generic
/// producer's default `has_explicit_boundary` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaElementForm {
    Wrapping,
    Following,
    /// Original document/block directive, retained as an opaque text payload.
    Prelude,
}

/// Passive namespace context at an original attribute name. Literal QName consumers
/// use this without manufacturing an expression occurrence or runtime context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeNamespaceSnapshot {
    pub namespaces: NsContext,
    pub pending: BTreeMap<String, AstNodeId>,
}

/// Original builder allocation with source-position expression and name metadata.
/// Lookup checks allocation identity, not IDs or source-coordinate equality.
#[derive(Debug)]
pub struct LexicallyScopedDocument {
    document: Arc<CemDocument>,
    occurrences: BTreeMap<AstNodeId, LexicalScopeSnapshot>,
    attribute_namespaces: BTreeMap<AstNodeId, AttributeNamespaceSnapshot>,
    names: BTreeMap<AstNodeId, ExpandedName>,
    schema_element_forms: BTreeMap<AstNodeId, SchemaElementForm>,
    namespace_bindings: BTreeMap<AstNodeId, NamespaceBinding>,
    pending_namespace_declarations: BTreeMap<AstNodeId, PendingNamespaceDeclaration>,
    pending_namespace_names: BTreeMap<AstNodeId, PendingNamespaceName>,
    pending_namespace_bindings: BTreeMap<AstNodeId, BTreeMap<String, AstNodeId>>,
    diagnostics: Vec<Diagnostic>,
}

impl LexicallyScopedDocument {
    pub(crate) fn from_import(
        document: Arc<CemDocument>,
        mut occurrences: BTreeMap<AstNodeId, LexicalScopeSnapshot>,
        diagnostics: Vec<Diagnostic>,
        schema_element_forms: BTreeMap<AstNodeId, SchemaElementForm>,
        namespace_bindings: BTreeMap<AstNodeId, NamespaceBinding>,
    ) -> Self {
        occurrences.retain(|node, _| {
            matches!(document.get(*node), Some(CemAstNode::Reference { .. }))
                || matches!(document.get(*node),
                    Some(CemAstNode::Element { expanded_name, .. }) if expanded_name.local_name == "$"
                )
        });
        let names = document
            .nodes
            .iter()
            .filter_map(|node| match node {
                CemAstNode::Element {
                    node_id,
                    expanded_name,
                    ..
                }
                | CemAstNode::Attribute {
                    node_id,
                    expanded_name,
                    ..
                } => Some((*node_id, expanded_name.clone())),
                _ => None,
            })
            .collect();
        Self {
            document,
            occurrences,
            attribute_namespaces: BTreeMap::new(),
            names,
            schema_element_forms,
            namespace_bindings,
            pending_namespace_declarations: BTreeMap::new(),
            pending_namespace_names: BTreeMap::new(),
            pending_namespace_bindings: BTreeMap::new(),
            diagnostics,
        }
    }
    /// Engine finalization before this owner is shared with consumers.
    pub(crate) fn document_mut(&mut self) -> &mut CemDocument {
        Arc::get_mut(&mut self.document).expect("captured document is not yet shared")
    }

    pub fn document(&self) -> &Arc<CemDocument> {
        &self.document
    }

    pub fn attribute_namespaces(
        &self,
        owner: &Arc<CemDocument>,
        node: AstNodeId,
    ) -> Option<&AttributeNamespaceSnapshot> {
        Arc::ptr_eq(owner, &self.document)
            .then(|| self.attribute_namespaces.get(&node))
            .flatten()
    }

    pub fn snapshot(
        &self,
        owner: &Arc<CemDocument>,
        node: AstNodeId,
    ) -> Option<&LexicalScopeSnapshot> {
        if !Arc::ptr_eq(owner, &self.document) {
            return None;
        }
        self.occurrences.get(&node)
    }

    /// Immutable source-position namespace metadata, available only to this
    /// original owner. Missing/unbound prefix metadata is not a guessed URI.
    /// CEM source keeps its lexical AST names; XML retains importer-resolved names.
    pub fn expanded_name(
        &self,
        owner: &Arc<CemDocument>,
        node: AstNodeId,
    ) -> Option<&ExpandedName> {
        if !Arc::ptr_eq(owner, &self.document) {
            return None;
        }
        self.names.get(&node)
    }

    /// Original producer form; a pending schema QName can retain this metadata
    /// before its namespace is known. Form alone does not establish core kind.
    pub fn schema_element_form(
        &self,
        owner: &Arc<CemDocument>,
        node: AstNodeId,
    ) -> Option<SchemaElementForm> {
        Arc::ptr_eq(owner, &self.document)
            .then(|| self.schema_element_forms.get(&node).copied())
            .flatten()
    }

    /// Completed parser binding declared by this original node. These records
    /// are not AST IDs or runtime contexts; lookup requires the original owner.
    /// No declaration text is reparsed and no later binding is substituted.
    pub fn namespace_binding(
        &self,
        owner: &Arc<CemDocument>,
        node: AstNodeId,
    ) -> Option<&NamespaceBinding> {
        Arc::ptr_eq(owner, &self.document)
            .then(|| self.namespace_bindings.get(&node))
            .flatten()
    }
    pub fn pending_namespace_declaration(
        &self,
        owner: &Arc<CemDocument>,
        node: AstNodeId,
    ) -> Option<&PendingNamespaceDeclaration> {
        Arc::ptr_eq(owner, &self.document)
            .then(|| self.pending_namespace_declarations.get(&node))
            .flatten()
    }
    pub fn pending_namespace_name(
        &self,
        owner: &Arc<CemDocument>,
        node: AstNodeId,
    ) -> Option<&PendingNamespaceName> {
        Arc::ptr_eq(owner, &self.document)
            .then(|| self.pending_namespace_names.get(&node))
            .flatten()
    }
    /// Pending bindings effective at an original expression occurrence. Complete
    /// namespace snapshots never substitute an inherited URI for these prefixes.
    pub fn pending_namespace_bindings(
        &self,
        owner: &Arc<CemDocument>,
        occurrence: AstNodeId,
    ) -> Option<&BTreeMap<String, AstNodeId>> {
        Arc::ptr_eq(owner, &self.document)
            .then(|| self.pending_namespace_bindings.get(&occurrence))
            .flatten()
    }

    /// Original named declaration visible at this occurrence. Lookup requires
    /// this captured owner; no source-coordinate matching or evaluation occurs.
    pub fn inline_schema(
        &self,
        owner: &Arc<CemDocument>,
        occurrence: AstNodeId,
        name: &str,
    ) -> Option<SchemaDeclarationNode> {
        let declaration = self
            .snapshot(owner, occurrence)?
            .schema
            .resolve_name(name)?;
        SchemaDeclarationNode::new(self.document.clone(), declaration.source_node?)
    }

    pub fn occurrences(&self) -> impl Iterator<Item = AstNodeId> + '_ {
        self.occurrences.keys().copied()
    }

    /// Schema-machine diagnostics, separate from builder diagnostics on document.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Owned lexical metadata only. No AST allocation, evaluator environment,
/// resolved target list or reference relationship boundary is captured.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexicalScopeSnapshot {
    pub namespaces: NsContext,
    pub schema: SchemaScopeFrame,
}

/// Forwards the original events to the AST builder. Observation precedes each
/// event's effects, so later declarations cannot alter earlier snapshots.
/// The observer receives None once, after EOF validation, for final diagnostics.
/// Completed snapshots and diagnostics are retained only by the caller.
pub struct LexicalScopeEvents<E: EventNormalizer, F> {
    machine: CemSchemaMachine<E>,
    observer: F,
    complete: bool,
    builder_node: Option<Arc<Mutex<Option<AstNodeId>>>>,
    awaiting_open_node: bool,
    namespace_capture: Option<Arc<Mutex<NamespaceDeclarationCapture>>>,
}

impl<E: EventNormalizer> CemSchemaMachine<E> {
    /// Build one retained fragment and attach lexical snapshots through builder
    /// node identities. No source-map matching, AST copy or reference evaluation.
    /// Runtime contexts, policies, readiness and grants remain caller supplied.
    pub fn build_with_lexical_scopes(self) -> LexicallyScopedDocument {
        let pending = Mutex::new(None);
        let pending_bindings = Mutex::new(None);
        let diagnostics = Mutex::new(Vec::new());
        let opening_name = Mutex::new(None);
        let attribute_names = Mutex::new(VecDeque::new());
        let attribute_contexts = Mutex::new(VecDeque::new());
        let builder_node = Arc::new(Mutex::new(None));
        let namespace_capture = Arc::new(Mutex::new(NamespaceDeclarationCapture::default()));
        let mut events = self.track_lexical_scope(|event, machine| {
            let expression = match event {
                Some(NormalizedEvent::OpenScope { name, .. }) => {
                    name.lexical_name == "$" || name.lexical_name == "cem:expr"
                }
                Some(NormalizedEvent::Value {
                    value: crate::events::ScalarValue::Expression(_),
                    ..
                }) => true,
                _ => false,
            };
            *pending.lock().unwrap() = expression.then(|| machine.lexical_snapshot());
            *pending_bindings.lock().unwrap() =
                expression.then(|| namespace_capture.lock().unwrap().pending_bindings());
            *opening_name.lock().unwrap() = match event {
                Some(NormalizedEvent::OpenScope { name, .. }) => {
                    capture_name(machine.current_ns_context(), &name.lexical_name)
                }
                _ => None,
            };
            if let Some(NormalizedEvent::Name { name, .. }) = event {
                let pending = namespace_capture.lock().unwrap().pending_bindings();
                let mut namespaces = machine.current_ns_context().clone();
                for prefix in pending.keys() {
                    namespaces.defer_binding(prefix);
                }
                attribute_contexts.lock().unwrap().push_back(AttributeNamespaceSnapshot {
                    namespaces,
                    pending,
                });
                // Namespace headers are intrinsic attributes, as in the
                // completed-name view; ordinary prefixed names still require
                // their original lexical binding. Never apply this to elements.
                let captured = if let Some(local) = name.lexical_name.strip_prefix("xmlns:") {
                    Some(ExpandedName {
                        namespace_uri: "http://www.w3.org/2000/xmlns/".into(),
                        local_name: local.into(),
                        schema_id: None,
                    })
                } else {
                    capture_name(machine.current_ns_context(), &name.lexical_name)
                };
                attribute_names.lock().unwrap().push_back(captured);
            }
            if event.is_none() {
                *diagnostics.lock().unwrap() = machine.diagnostics().to_vec();
            }
        });
        events.builder_node = Some(builder_node.clone());
        events.namespace_capture = Some(namespace_capture.clone());
        let mut attribute_namespaces = BTreeMap::new();
        let mut occurrences = BTreeMap::new();
        let mut occurrence_pending_bindings = BTreeMap::new();
        let mut names = BTreeMap::new();
        let document = CemAstBuilder::new(events).build_with_node_observer(|node, attribute| {
            *builder_node.lock().unwrap() = node;
            namespace_capture.lock().unwrap().observed(node, attribute);
            if let Some(attribute) = attribute {
                if let Some(snapshot) = attribute_contexts.lock().unwrap().pop_front() {
                    attribute_namespaces.insert(attribute, snapshot);
                }
                if let Some(Some(name)) = attribute_names.lock().unwrap().pop_front() {
                    names.insert(attribute, name);
                }
            }
            if let Some(node) = node {
                if let Some(name) = opening_name.lock().unwrap().take() {
                    names.insert(node, name);
                }
                if let Some(snapshot) = pending.lock().unwrap().take() {
                    occurrences.insert(node, snapshot);
                    occurrence_pending_bindings.insert(
                        node,
                        pending_bindings.lock().unwrap().take().unwrap_or_default(),
                    );
                }
            }
        });
        occurrences.retain(|node, _| {
            matches!(document.get(*node), Some(CemAstNode::Reference { .. }))
                || matches!(document.get(*node),
            Some(CemAstNode::Element { expanded_name, .. }) if expanded_name.local_name == "$")
        });
        // Standalone expressions fold to native references and can discard
        // transient nodes. Keep only surviving named nodes in this original arena.
        names.retain(|node, _| {
            matches!(
                document.get(*node),
                Some(CemAstNode::Element { .. } | CemAstNode::Attribute { .. })
            )
        });
        let (pending_namespace_declarations, mut pending_namespace_names) =
            namespace_capture.lock().unwrap().take_pending();
        pending_namespace_names.retain(|node, _| {
            matches!(
                document.get(*node),
                Some(CemAstNode::Element { .. } | CemAstNode::Attribute { .. })
            )
        });
        names.retain(|node, _| !pending_namespace_names.contains_key(node));
        occurrence_pending_bindings.retain(|node, _| occurrences.contains_key(node));
        attribute_namespaces.retain(|node, _| {
            matches!(document.get(*node), Some(CemAstNode::Attribute { .. }))
        });
        let mut schema_element_forms: BTreeMap<_, _> = names
            .iter()
            .filter_map(|(id, name)| {
                if name.local_name == "@schema" {
                    return Some((*id, SchemaElementForm::Prelude));
                }
                if name.local_name != "schema"
                    || !matches!(
                        name.namespace_uri.as_str(),
                        "" | "https://cem.dev/ns/core/1"
                    )
                {
                    return None;
                }
                captured_cem_schema_form(&document, *id).map(|form| (*id, form))
            })
            .collect();
        // Pending QName identity does not discard the original producer form.
        // Later consumers still check the completed namespace for core kind.
        for (id, name) in &pending_namespace_names {
            if name.local_name == "schema" {
                if let Some(form) = captured_cem_schema_form(&document, *id) {
                    schema_element_forms.insert(*id, form);
                }
            }
        }
        let mut namespace_bindings = namespace_capture.lock().unwrap().take_bindings();
        namespace_bindings.retain(|node, _| {
            matches!(
                document.get(*node),
                Some(CemAstNode::Element { .. } | CemAstNode::Attribute { .. })
            )
        });
        LexicallyScopedDocument {
            document: Arc::new(document),
            attribute_namespaces,
            occurrences,
            names,
            schema_element_forms,
            namespace_bindings,
            pending_namespace_declarations,
            pending_namespace_names,
            pending_namespace_bindings: occurrence_pending_bindings,
            diagnostics: diagnostics.into_inner().unwrap(),
        }
    }

    pub fn lexical_snapshot(&self) -> LexicalScopeSnapshot {
        LexicalScopeSnapshot {
            namespaces: self.current_ns_context().clone(),
            schema: self.schema_scopes().current().clone(),
        }
    }

    /// Current validation diagnostics. Completion does not imply error-free input
    /// or ready schema dependencies; an observer must retain/inspect these facts.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Observe the lexical state effective immediately before each incoming event
    /// and final validation at EOF, without reparsing or buffering event history.
    /// Runtime input preparation and reference evaluation remain consumer stages.
    pub fn track_lexical_scope<F>(self, observer: F) -> LexicalScopeEvents<E, F>
    where
        F: FnMut(Option<&NormalizedEvent>, &Self) + Send,
    {
        LexicalScopeEvents {
            machine: self,
            observer,
            complete: false,
            builder_node: None,
            awaiting_open_node: false,
            namespace_capture: None,
        }
    }
}

impl<E, F> EventNormalizer for LexicalScopeEvents<E, F>
where
    E: EventNormalizer,
    F: FnMut(Option<&NormalizedEvent>, &CemSchemaMachine<E>) + Send,
{
    fn next_event(&mut self) -> Option<NormalizedEvent> {
        if self.complete {
            return None;
        }
        // The builder has consumed the previous opening event and supplied its
        // original node ID. Attach it before processing attributes or closure.
        // Ordinary stream observation has no builder and retains no source ID.
        if self.awaiting_open_node {
            if let Some(Some(pending)) = self.machine.pending_schema_elements.last_mut() {
                pending.source_node = self
                    .builder_node
                    .as_ref()
                    .and_then(|node| node.lock().unwrap().take());
            }
            self.awaiting_open_node = false;
        }
        match self.machine.events.next_event() {
            Some(event) => {
                (self.observer)(Some(&event), &self.machine);
                // Only the current normalized event is copied for validation;
                // the original reaches the builder and no AST/history is copied.
                self.awaiting_open_node = self.builder_node.is_some()
                    && matches!(event, NormalizedEvent::OpenScope { .. });
                if let Some(capture) = &self.namespace_capture {
                    capture.lock().unwrap().consume(&mut self.machine, &event);
                } else {
                    self.machine.consume(event.clone());
                }
                Some(event)
            }
            None => {
                self.machine.finalize();
                self.complete = true;
                (self.observer)(None, &self.machine);
                None
            }
        }
    }
}

fn capture_name(namespaces: &NsContext, lexical: &str) -> Option<ExpandedName> {
    let resolved = namespaces.resolve(lexical);
    let namespace_uri = match resolved.namespace_uri {
        Some(uri) => uri,
        None if resolved.prefix.is_none() => String::new(),
        None => return None,
    };
    Some(ExpandedName {
        namespace_uri,
        local_name: resolved.local_name,
        schema_id: None,
    })
}

// Called only for the original CEM builder observation above. Generic/imported
// producer defaults cannot manufacture a wrapping/following source distinction.
fn captured_cem_schema_form(document: &CemDocument, node: AstNodeId) -> Option<SchemaElementForm> {
    let CemAstNode::Element {
        children,
        has_explicit_boundary,
        ..
    } = document.get(node)?
    else {
        return None;
    };
    let body = *has_explicit_boundary
        || children.iter().any(|child| {
            !matches!(
                document.get(*child),
                Some(
                    CemAstNode::Whitespace { .. }
                        | CemAstNode::Comment { .. }
                        | CemAstNode::ProcessingInstruction { .. }
                )
            )
        });
    Some(if body {
        SchemaElementForm::Wrapping
    } else {
        SchemaElementForm::Following
    })
}
