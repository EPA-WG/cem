//! Opt-in source discovery before exact datatype implementation registration.
use super::*;
use cem_ml::schema::{
    datatype_contracts::DatatypeCompilationIssue, datatype_registry::DatatypeSource,
};
use cem_ql::datatype_names::{
    DatatypeNameCatalog, DatatypeNameError, DatatypeNameLimits, DatatypeSchemaSource,
};
impl CemQlSchemaPackageCompiler {
    /// The source callback supplies original selected schemas and explicit exports;
    /// it does not grant scope access or choose implementations. Discovery finishes
    /// before the compile callback receives any sources. Pending/invalid discovery
    /// becomes an inspectable compilation snapshot under normal package readiness.
    pub fn with_datatype_discovery<P, F>(
        self,
        name_limits: DatatypeNameLimits,
        sources: P,
        compile: F,
    ) -> Self
    where
        P: Fn(
                &SchemaPackageCompilationRequest,
                &mut CemQlSchemaDeclarationHost,
            ) -> Result<Vec<DatatypeSchemaSource>, DatatypeNameError>
            + Send
            + Sync
            + 'static,
        F: Fn(
                &SchemaPackageCompilationRequest,
                &mut CemQlSchemaDeclarationHost,
                &[DatatypeSource],
                ReferenceTraversalLimits,
            ) -> Result<DatatypeCompilation, Vec<Diagnostic>>
            + Send
            + Sync
            + 'static,
    {
        self.with_datatypes(move |request, host, limits| {
            let inputs = match sources(request, host) {
                Ok(inputs) => inputs,
                Err(error) => return Ok(incomplete(request, error)),
            };
            // A package callback may include selected external schemas, but it may
            // not omit the candidate owner and certify some unrelated catalog.
            if !inputs
                .iter()
                .any(|input| Arc::ptr_eq(input.schema.document(), request.source.ast_owner()))
            {
                return Err(vec![compilation_failure(
                    request.source.source_uri(),
                    "Datatype discovery omitted the package source owner",
                )]);
            }
            let catalog = match DatatypeNameCatalog::discover(&inputs, host, name_limits) {
                Ok(catalog) => Arc::new(catalog),
                Err(error) => return Ok(incomplete(request, error)),
            };
            let declarations = catalog.sources().cloned().collect::<Vec<_>>();
            if let Err(reason) = host.install_datatype_names(catalog) {
                let source = inputs[0].schema.clone();
                return Ok(incomplete(
                    request,
                    DatatypeNameError {
                        code: reason,
                        source,
                        related: None,
                        pending: reason == "unregistered-datatype-owner",
                    },
                ));
            }
            let mut compilation = compile(request, host, &declarations, limits)?;
            // A callback cannot claim readiness by silently ignoring a discovered type.
            let mut required = compilation
                .sources
                .iter()
                .map(|source| (source.declaration().identity(), source.scope().identity()))
                .collect::<std::collections::BTreeSet<_>>();
            for source in declarations {
                if required.insert((source.declaration().identity(), source.scope().identity())) {
                    compilation.sources.push(source);
                }
            }
            Ok(compilation)
        })
    }
}
fn incomplete(
    request: &SchemaPackageCompilationRequest,
    error: DatatypeNameError,
) -> DatatypeCompilation {
    let mut compilation = DatatypeCompilation::new(request.source.ast_owner().clone());
    compilation.issues.push(DatatypeCompilationIssue {
        code: error.code,
        state: if error.pending {
            DatatypeIssueState::Pending
        } else {
            DatatypeIssueState::Invalid
        },
        source: error.source,
        related: error.related,
    });
    compilation
}
