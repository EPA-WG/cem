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
    /// Supply one registered owner's completed selected forest for the consumer
    /// invocation. Original capture is attached idempotently and never changed;
    /// completion restores on return, errors and unwind. A nested invocation for
    /// this owner temporarily replaces its completion; other owners retain theirs.
    ///
    /// This establishes no runtime input, scope assignment, relationship grant or
    /// target-binding readiness. It neither evaluates slots nor installs lexical
    /// contexts. Structural attribute lookup uses these names; retained behavior
    /// name projections require their own handoff.
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
