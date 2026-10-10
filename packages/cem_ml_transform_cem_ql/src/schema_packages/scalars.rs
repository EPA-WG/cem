//! Coordinated, opt-in assembly and registration of admitted scalar functions.
use super::*;
use cem_ml::schema::{
    datatype_contracts::DatatypeCompilationIssue, datatype_registry::DatatypeSource,
    declaration_references::SchemaDeclarationNode, function_references::ScalarCompilationBudget,
    value_contracts::ValueContractSource,
};
use cem_ql::{
    datatype_compilation::{compile_datatypes_with_budget, DatatypeImplementations},
    datatype_enumeration::ConstantPreparationLimits,
    datatype_validation::{DatatypeValidationRegistry, ValidationRuntime},
    function_activation::{compile_function_bindings, FunctionAdmission},
};

/// Exact host admissions for a fresh package attempt. Existing native primitive
/// registrations may be supplied here; authored source cannot add admissions.
#[derive(Debug, Clone, Default)]
pub struct ScalarPackagePlan {
    pub sources: Vec<ValueContractSource>,
    pub exports: Vec<SchemaDeclarationNode>,
    pub admissions: Vec<FunctionAdmission>,
    pub roots: Vec<DatatypeSource>,
    pub implementations: DatatypeImplementations,
    pub validators: DatatypeValidationRegistry,
    pub preparation_limits: ConstantPreparationLimits,
}
pub(super) type PrepareScalars = dyn Fn(
        &SchemaPackageCompilationRequest,
        &mut CemQlSchemaDeclarationHost,
    ) -> Result<ScalarPackagePlan, Vec<Diagnostic>>
    + Send
    + Sync;
impl CemQlSchemaPackageCompiler {
    /// Replaces a datatype-only compiler with automatic checked function
    /// registration followed by datatype and attribute activation. The callback
    /// supplies exact profile authority, sources, contexts and datatype choices.
    pub fn with_scalar_datatypes<F>(mut self, prepare: F) -> Self
    where
        F: Fn(
                &SchemaPackageCompilationRequest,
                &mut CemQlSchemaDeclarationHost,
            ) -> Result<ScalarPackagePlan, Vec<Diagnostic>>
            + Send
            + Sync
            + 'static,
    {
        self.scalars = Some(Arc::new(prepare));
        self.datatypes = None;
        self
    }
}
pub(super) fn compile(
    plan: ScalarPackagePlan,
    request: &SchemaPackageCompilationRequest,
    model: &mut SchemaDocumentModel,
    host: &mut CemQlSchemaDeclarationHost,
    limits: &mut ReferenceTraversalLimits,
    runtime: &ValidationRuntime<'_>,
) -> Result<DatatypeCompilation, Vec<Diagnostic>> {
    let owner = request.source.ast_owner().clone();
    let Some(source) = plan
        .sources
        .iter()
        .find(|source| Arc::ptr_eq(source.schema.document(), &owner))
    else {
        return Err(vec![compilation_failure(
            request.source.source_uri(),
            "Function sources must include the exact candidate owner",
        )]);
    };
    let fallback = source.schema.clone();
    let mut budget = ScalarCompilationBudget::new(*limits)
        .map_err(|e| vec![compilation_failure(request.source.source_uri(), e.code)])?;
    // Declaration assembly has already performed bounded selections. Carry their
    // work into the shared scalar allowance instead of granting a fresh phase.
    for site in &model.declaration_references.sites {
        if let Some(resolution) = &site.resolution {
            if let Err(error) = budget.spend(resolution.work_used, &fallback) {
                let mut output = DatatypeCompilation::new(owner);
                output.sources = plan.roots;
                output.issues.push(DatatypeCompilationIssue {
                    code: error.code,
                    state: DatatypeIssueState::Pending,
                    source: fallback,
                    related: None,
                });
                *limits = budget.remaining_limits();
                return Ok(output);
            }
        }
    }
    let bindings = compile_function_bindings(
        &plan.sources,
        &plan.exports,
        &plan.admissions,
        host,
        &mut budget,
        plan.validators,
        plan.implementations,
    );
    // A failed function attempt cannot invoke constant preparation through an
    // older or partial registration supplied in the same plan.
    let mut output = if bindings.issues.is_empty() {
        compile_datatypes_with_budget(
            owner,
            &plan.roots,
            host,
            bindings.implementations(),
            bindings.validators(),
            &mut budget,
            Some(runtime),
            plan.preparation_limits,
        )
    } else {
        let mut output = DatatypeCompilation::new(owner);
        output.sources = plan.roots;
        output
    };
    output.issues.extend(bindings.issues.clone());
    output.diagnostics.extend(bindings.diagnostics.clone());
    bindings.activate(model);
    model.function_bindings = Some(Arc::new(bindings));
    *limits = budget.remaining_limits();
    Ok(output)
}
