//! Opt-in source-position observation while forwarding one normalized stream.
use super::{CemSchemaMachine, EventNormalizer, NormalizedEvent};
use crate::{
    diagnostics::Diagnostic,
    parser::{builder::CemAstBuilder, document::CemDocument, AstNodeId, CemAstNode, ExpandedName},
    schema::{
        declaration_references::SchemaDeclarationNode, namespace::NsContext,
        scoping::SchemaScopeFrame,
    },
};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};

/// Original source form, distinct even when both elements have no child nodes.
/// Importers retain this event metadata; it cannot be inferred from a generic
/// producer's default `has_explicit_boundary` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaElementForm {
    Wrapping,
    Following,
}

/// Original builder allocation with source-position expression and name metadata.
/// Lookup checks allocation identity, not IDs or source-coordinate equality.
#[derive(Debug)]
pub struct LexicallyScopedDocument {
    document: Arc<CemDocument>,
    occurrences: BTreeMap<AstNodeId, LexicalScopeSnapshot>,
    names: BTreeMap<AstNodeId, ExpandedName>,
    schema_element_forms: BTreeMap<AstNodeId, SchemaElementForm>,
    diagnostics: Vec<Diagnostic>,
}

impl LexicallyScopedDocument {
    pub(crate) fn from_import(
        document: Arc<CemDocument>,
        mut occurrences: BTreeMap<AstNodeId, LexicalScopeSnapshot>,
        diagnostics: Vec<Diagnostic>,
        schema_element_forms: BTreeMap<AstNodeId, SchemaElementForm>,
    ) -> Self {
        occurrences
            .retain(|node, _| matches!(document.get(*node), Some(CemAstNode::Reference { .. })));
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
            names,
            schema_element_forms,
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

    pub fn schema_element_form(
        &self,
        owner: &Arc<CemDocument>,
        node: AstNodeId,
    ) -> Option<SchemaElementForm> {
        Arc::ptr_eq(owner, &self.document)
            .then(|| self.schema_element_forms.get(&node).copied())
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
#[derive(Debug, Clone)]
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
}

impl<E: EventNormalizer> CemSchemaMachine<E> {
    /// Build one retained fragment and attach lexical snapshots through builder
    /// node identities. No source-map matching, AST copy or reference evaluation.
    /// Runtime contexts, policies, readiness and grants remain caller supplied.
    pub fn build_with_lexical_scopes(self) -> LexicallyScopedDocument {
        let pending = Mutex::new(None);
        let diagnostics = Mutex::new(Vec::new());
        let opening_name = Mutex::new(None);
        let attribute_names = Mutex::new(VecDeque::new());
        let builder_node = Arc::new(Mutex::new(None));
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
            *opening_name.lock().unwrap() = match event {
                Some(NormalizedEvent::OpenScope { name, .. }) => {
                    capture_name(machine.current_ns_context(), &name.lexical_name)
                }
                _ => None,
            };
            if let Some(NormalizedEvent::Name { name, .. }) = event {
                attribute_names.lock().unwrap().push_back(capture_name(
                    machine.current_ns_context(),
                    &name.lexical_name,
                ));
            }
            if event.is_none() {
                *diagnostics.lock().unwrap() = machine.diagnostics().to_vec();
            }
        });
        events.builder_node = Some(builder_node.clone());
        let mut occurrences = BTreeMap::new();
        let mut names = BTreeMap::new();
        let document = CemAstBuilder::new(events).build_with_node_observer(|node, attribute| {
            *builder_node.lock().unwrap() = node;
            if let Some(attribute) = attribute {
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
        let schema_element_forms = names
            .iter()
            .filter_map(|(id, name)| {
                if name.local_name != "schema"
                    || !matches!(
                        name.namespace_uri.as_str(),
                        "" | "https://cem.dev/ns/core/1"
                    )
                {
                    return None;
                }
                let Some(CemAstNode::Element {
                    children,
                    has_explicit_boundary,
                    ..
                }) = document.get(*id)
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
                Some((
                    *id,
                    if body {
                        SchemaElementForm::Wrapping
                    } else {
                        SchemaElementForm::Following
                    },
                ))
            })
            .collect();
        LexicallyScopedDocument {
            document: Arc::new(document),
            occurrences,
            names,
            schema_element_forms,
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
                self.machine.consume(event.clone());
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
