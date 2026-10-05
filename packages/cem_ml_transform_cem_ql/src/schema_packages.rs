//! Explicit package compilation bridge. Installing ordinary QL adapters does
//! not install this compiler or supply runtime data/replace permissions.
use cem_ml::{
    diagnostics::Diagnostic,
    engine::EngineContext,
    schema::{
        document_model::SchemaDocumentModel,
        package_compilation::{
            compilation_failure, SchemaPackageCompilationRequest, SchemaPackageCompiler,
        },
        reference_traversal::ReferenceTraversalLimits,
    },
};
use cem_ql::schema_references::CemQlSchemaDeclarationHost;
use std::sync::Arc;

type Prepare = dyn Fn(
        &SchemaPackageCompilationRequest,
    ) -> Result<(CemQlSchemaDeclarationHost, ReferenceTraversalLimits), Vec<Diagnostic>>
    + Send
    + Sync;
/// Prepare a new consumer host for each lifecycle snapshot. The callback
/// registers the request's retained source and supplies contexts, effective
/// scopes and crossing grants. No reference selections are cached or shared.
#[derive(Clone)]
pub struct CemQlSchemaPackageCompiler {
    prepare: Arc<Prepare>,
}
impl std::fmt::Debug for CemQlSchemaPackageCompiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CemQlSchemaPackageCompiler")
            .finish_non_exhaustive()
    }
}
impl CemQlSchemaPackageCompiler {
    pub fn new<F>(prepare: F) -> Self
    where
        F: Fn(
                &SchemaPackageCompilationRequest,
            )
                -> Result<(CemQlSchemaDeclarationHost, ReferenceTraversalLimits), Vec<Diagnostic>>
            + Send
            + Sync
            + 'static,
    {
        Self {
            prepare: Arc::new(prepare),
        }
    }
}
impl SchemaPackageCompiler for CemQlSchemaPackageCompiler {
    fn compile(
        &self,
        request: &SchemaPackageCompilationRequest,
    ) -> Result<SchemaDocumentModel, Vec<Diagnostic>> {
        let (mut host, limits) = (self.prepare)(request)?;
        host.compile(&request.schema_uri, request.source.clone(), limits)
            .map_err(|error| {
                vec![compilation_failure(
                    request.source.source_uri(),
                    error.to_string(),
                )]
            })
    }
}
pub fn register_cem_ql_schema_package_compiler(
    context: &mut EngineContext,
    compiler: CemQlSchemaPackageCompiler,
) {
    context.schema_package_compiler = Some(Arc::new(compiler));
}
