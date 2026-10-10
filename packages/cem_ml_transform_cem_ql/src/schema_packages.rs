//! Explicit package compilation bridge. Installing ordinary QL adapters does
//! not install this compiler or supply runtime data/replace permissions.
use cem_ml::{
    diagnostics::Diagnostic,
    engine::EngineContext,
    schema::{
        datatype_contracts::{DatatypeCompilation, DatatypeIssueState},
        declaration_references::SchemaDeclarationHost,
        document_model::SchemaDocumentModel,
        package_compilation::{
            compilation_failure, SchemaPackageCompilationRequest, SchemaPackageCompiler,
        },
        reference_traversal::ReferenceTraversalLimits,
    },
};
use cem_ql::schema_references::CemQlSchemaDeclarationHost;
use std::sync::Arc;
mod discovery;
mod scalars;
pub use scalars::ScalarPackagePlan;

type Prepare = dyn Fn(
        &SchemaPackageCompilationRequest,
    ) -> Result<(CemQlSchemaDeclarationHost, ReferenceTraversalLimits), Vec<Diagnostic>>
    + Send
    + Sync;
type CompileDatatypes = dyn Fn(
        &SchemaPackageCompilationRequest,
        &mut CemQlSchemaDeclarationHost,
        ReferenceTraversalLimits,
    ) -> Result<DatatypeCompilation, Vec<Diagnostic>>
    + Send
    + Sync;

/// Prepare a new consumer host for each lifecycle snapshot. The callback
/// registers the request's retained source and supplies contexts, effective
/// scopes and crossing grants. No reference selections are cached or shared.
#[derive(Clone)]
pub struct CemQlSchemaPackageCompiler {
    prepare: Arc<Prepare>,
    datatypes: Option<Arc<CompileDatatypes>>,
    scalars: Option<Arc<scalars::PrepareScalars>>,
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
            datatypes: None,
            scalars: None,
        }
    }
    /// Opt in to executable datatype readiness for this package. The callback
    /// supplies fresh original sources and exact implementation registrations;
    /// compilation never promotes local names into execution authority.
    pub fn with_datatypes<F>(mut self, compile: F) -> Self
    where
        F: Fn(
                &SchemaPackageCompilationRequest,
                &mut CemQlSchemaDeclarationHost,
                ReferenceTraversalLimits,
            ) -> Result<DatatypeCompilation, Vec<Diagnostic>>
            + Send
            + Sync
            + 'static,
    {
        self.datatypes = Some(Arc::new(compile));
        self.scalars = None;
        self
    }
}
impl SchemaPackageCompiler for CemQlSchemaPackageCompiler {
    fn compile(
        &self,
        request: &SchemaPackageCompilationRequest,
    ) -> Result<SchemaDocumentModel, Vec<Diagnostic>> {
        let (mut host, mut limits) = (self.prepare)(request)?;
        if self.datatypes.is_some() || self.scalars.is_some() {
            host.enable_attribute_datatypes();
        }
        let mut model = host
            .compile(&request.schema_uri, request.source.clone(), limits)
            .map_err(|error| {
                vec![compilation_failure(
                    request.source.source_uri(),
                    error.to_string(),
                )]
            })?;
        if self.datatypes.is_some() || self.scalars.is_some() {
            let control = cem_ml::operation_control::OperationControl::default();
            let runtime = cem_ql::datatype_validation::ValidationRuntime {
                control: &control,
                scope: cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
                query: Default::default(),
            }
            .with_query_budget();
            let mut datatypes = if let Some(prepare) = &self.scalars {
                let plan = prepare(request, &mut host)?;
                scalars::compile(plan, request, &mut model, &mut host, &mut limits, &runtime)?
            } else {
                self.datatypes.as_ref().unwrap()(request, &mut host, limits)?
            };
            if !datatypes.matches_owner(request.source.ast_owner()) {
                return Err(vec![compilation_failure(
                    request.source.source_uri(),
                    "Datatype compilation belongs to another source owner",
                )]);
            }
            cem_ql::attribute_activation::activate_attribute_datatypes(
                &mut model,
                &mut datatypes,
                &mut host,
                limits,
                &runtime,
            )
            .map_err(|error| {
                vec![compilation_failure(
                    request.source.source_uri(),
                    error.to_string(),
                )]
            })?;
            model
                .compile_diagnostics
                .extend(datatypes.diagnostics.clone());
            for issue in &datatypes.issues {
                if issue.state != DatatypeIssueState::Invalid {
                    continue;
                }
                let tree = host.input_source_tree(&issue.source);
                let mut diagnostic = compilation_failure(
                    tree.as_ref()
                        .map_or(request.source.source_uri(), |tree| tree.source_uri()),
                    format!("Invalid datatype contract: {}", issue.code),
                );
                diagnostic.node = Some(issue.source.identity());
                if let cem_ml::parser::CemAstNode::Element { source, .. }
                | cem_ml::parser::CemAstNode::Attribute { source, .. }
                | cem_ml::parser::CemAstNode::Reference { source, .. } = issue.source.node()
                {
                    diagnostic.source_map = Some(source.clone());
                }
                model.compile_diagnostics.push(diagnostic);
            }
            model.datatype_compilation = Some(Arc::new(datatypes));
        }
        Ok(model)
    }
}
pub fn register_cem_ql_schema_package_compiler(
    context: &mut EngineContext,
    compiler: CemQlSchemaPackageCompiler,
) {
    context.schema_package_compiler = Some(Arc::new(compiler));
}
