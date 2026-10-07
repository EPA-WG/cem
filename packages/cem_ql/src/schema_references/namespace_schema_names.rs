//! Invocation-scoped completed names for schema admission and structural consumers.
use super::{CemQlSchemaDeclarationHost, LexicalScopeHandoffError};
use cem_ml::{
    parser::ExpandedName,
    schema::{
        declaration_references::SchemaDeclarationNode,
        namespace_references::NamespaceNameCompletion,
    },
};
use std::sync::Arc;

struct NameInvocation<'a> {
    host: &'a mut CemQlSchemaDeclarationHost,
    owner: usize,
    previous: Option<Arc<NamespaceNameCompletion>>,
}
impl Drop for NameInvocation<'_> {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            self.host
                .namespace_name_completions
                .insert(self.owner, previous);
        } else {
            self.host.namespace_name_completions.remove(&self.owner);
        }
    }
}

impl CemQlSchemaDeclarationHost {
    /// Install independently prepared owners together for one consumer invocation.
    /// Iterative guards avoid nesting the stack once per loaded resource.
    pub(super) fn with_completed_namespace_name_views<R, F>(
        &mut self,
        completions: &[Arc<NamespaceNameCompletion>],
        consume: F,
    ) -> Result<R, LexicalScopeHandoffError>
    where
        F: FnOnce(&mut Self) -> R,
    {
        for completion in completions {
            self.attach_captured_names(completion.captured())?;
        }
        struct Views<'a> {
            host: &'a mut CemQlSchemaDeclarationHost,
            previous: Vec<(usize, Option<Arc<NamespaceNameCompletion>>)>,
        }
        impl Drop for Views<'_> {
            fn drop(&mut self) {
                for (owner, names) in self.previous.drain(..).rev() {
                    if let Some(names) = names {
                        self.host.namespace_name_completions.insert(owner, names);
                    } else {
                        self.host.namespace_name_completions.remove(&owner);
                    }
                }
            }
        }
        let mut views = Views {
            host: self,
            previous: Vec::with_capacity(completions.len()),
        };
        for completion in completions {
            let owner = Arc::as_ptr(completion.captured().document()) as usize;
            let previous = views
                .host
                .namespace_name_completions
                .insert(owner, completion.clone());
            views.previous.push((owner, previous));
        }
        Ok(consume(views.host))
    }
    /// Supply one registered owner's completed selected forest for the consumer
    /// invocation. Original capture is attached idempotently and never changed;
    /// completion restores on return, errors and unwind. A nested invocation for
    /// this owner temporarily replaces its completion; other owners retain theirs.
    ///
    /// This establishes no runtime input, scope assignment, relationship grant or
    /// target-binding readiness. It neither evaluates slots nor installs lexical
    /// contexts. Structural and retained behavior consumers snapshot these names
    /// while the invocation is active; snapshots retain original source owners.
    pub fn with_completed_namespace_names<R, F>(
        &mut self,
        completion: Arc<NamespaceNameCompletion>,
        consume: F,
    ) -> Result<R, LexicalScopeHandoffError>
    where
        F: FnOnce(&mut Self) -> R,
    {
        self.attach_captured_names(completion.captured())?;
        let owner = Arc::as_ptr(completion.captured().document()) as usize;
        let previous = self.namespace_name_completions.insert(owner, completion);
        let invocation = NameInvocation {
            host: self,
            owner,
            previous,
        };
        Ok(consume(&mut *invocation.host))
    }

    /// Effective name for this schema consumer invocation. Only the completion's
    /// original owner and selected owning forest can supply pending names. Other
    /// nodes keep their original captured names (including absent pending names).
    /// `captured_expanded_name` always remains the unchanged source-position view.
    pub fn consuming_expanded_name(&self, source: &SchemaDeclarationNode) -> Option<&ExpandedName> {
        let owner = Arc::as_ptr(source.document()) as usize;
        if let Some(completion) = self.namespace_name_completions.get(&owner) {
            if completion.contains(source.node_id()) {
                return completion.expanded_name(source.node_id());
            }
        }
        self.captured_expanded_name(source)
    }
}
