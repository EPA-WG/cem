//! Opt-in source-position observation while forwarding one normalized stream.
use super::{CemSchemaMachine, EventNormalizer, NormalizedEvent};
use crate::{
    diagnostics::Diagnostic,
    parser::{builder::CemAstBuilder, document::CemDocument, AstNodeId, CemAstNode},
    schema::{namespace::NsContext, scoping::SchemaScopeFrame},
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

/// Original builder allocation with source-position expression metadata.
/// Lookup checks allocation identity, not IDs or source-coordinate equality.
pub struct LexicallyScopedDocument {
    document: Arc<CemDocument>,
    occurrences: BTreeMap<AstNodeId, LexicalScopeSnapshot>,
    diagnostics: Vec<Diagnostic>,
}

impl LexicallyScopedDocument {
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
}

impl<E: EventNormalizer> CemSchemaMachine<E> {
    /// Build one retained fragment and attach lexical snapshots through builder
    /// node identities. No source-map matching, AST copy or reference evaluation.
    /// Runtime contexts, policies, readiness and grants remain caller supplied.
    pub fn build_with_lexical_scopes(self) -> LexicallyScopedDocument {
        let pending = Mutex::new(None);
        let diagnostics = Mutex::new(Vec::new());
        let events = self.track_lexical_scope(|event, machine| {
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
            if event.is_none() {
                *diagnostics.lock().unwrap() = machine.diagnostics().to_vec();
            }
        });
        let mut occurrences = BTreeMap::new();
        let document = CemAstBuilder::new(events).build_with_node_observer(|node| {
            if let Some(node) = node {
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
        LexicallyScopedDocument {
            document: Arc::new(document),
            occurrences,
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
        match self.machine.events.next_event() {
            Some(event) => {
                (self.observer)(Some(&event), &self.machine);
                // Only the current normalized event is copied for validation;
                // the original reaches the builder and no AST/history is copied.
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
