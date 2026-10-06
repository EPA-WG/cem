//! Opt-in source-position observation while forwarding one normalized stream.
use super::{CemSchemaMachine, EventNormalizer, NormalizedEvent};
use crate::{
    diagnostics::Diagnostic,
    schema::{namespace::NsContext, scoping::SchemaScopeFrame},
};

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
