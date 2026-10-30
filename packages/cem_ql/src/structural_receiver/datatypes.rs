//! Explicit native compilation of an inert local source closure. No component
//! admission, source parsing, module activation or callback deserialization.
use super::*;
use crate::{
    datatype_compilation::{
        compile_datatypes, DatatypeImplementations, DatatypeValidation, ExecutableDatatype,
    },
    datatype_source_capsule::VerifiedDatatypeSources,
    datatype_validation::{
        DatatypeValidationRegistry, ValidationInput, ValidationLimits, ValidationRuntime,
    },
};
use cem_ml::schema::{
    attribute_datatypes::AttributeDatatypeContextLease,
    datatype_contracts::{CompiledDatatypeContract, DatatypeCompilation},
};
const INVALID: &str = "fresh datatype source compilation refused";
pub struct FreshDatatypeCompilation {
    sources: Arc<VerifiedDatatypeSources>,
    compilation: DatatypeCompilation,
    context: AttributeDatatypeContextLease,
    scope: DeclarationScope,
    active: AtomicBool,
}
impl FreshDatatypeCompilation {
    fn check(&self, host: &FreshStructuralHost) -> Result<(), String> {
        host.check().map_err(|_| INVALID)?;
        if !self.active.load(Ordering::Acquire)
            || !self.context.matches(&host.context)
            || self.sources.pin().registry_hash != host.registry_hash
            || host
                .inner
                .registered_validator_context(self.scope, self.sources.source())
                .is_none()
        {
            return Err(INVALID.into());
        }
        Ok(())
    }
    pub fn retire(&self) {
        self.active.store(false, Ordering::Release)
    }
    pub fn sources(&self) -> &Arc<VerifiedDatatypeSources> {
        &self.sources
    }
    pub fn validate(
        &self,
        host: &FreshStructuralHost,
        name: &str,
        input: &ValidationInput,
        limits: ValidationLimits,
    ) -> Result<DatatypeValidation, String> {
        self.check(host)?;
        let source = self.sources.source_named(name).ok_or(INVALID)?;
        if !self
            .sources
            .roots()
            .contains(&source.declaration().node_id())
        {
            return Err(INVALID.into());
        }
        let descriptor = self
            .compilation
            .contracts
            .iter()
            .filter_map(|c| c.as_any().downcast_ref::<ExecutableDatatype>())
            .find(|c| {
                c.source().declaration().identity() == source.declaration().identity()
                    && c.source().scope().identity() == source.scope().identity()
            })
            .ok_or(INVALID)?;
        let query = host
            .inner
            .registered_validator_context(self.scope, self.sources.source())
            .ok_or(INVALID)?;
        let runtime = ValidationRuntime {
            control: &host.control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query,
        };
        let result = descriptor.validate(input, &runtime, limits);
        self.check(host)?;
        Ok(result)
    }
}
impl FreshStructuralHost {
    pub fn compile_datatype_sources(
        &self,
        sources: Arc<VerifiedDatatypeSources>,
        scope: DeclarationScope,
        implementations: &DatatypeImplementations,
        validations: &DatatypeValidationRegistry,
    ) -> Result<FreshDatatypeCompilation, String> {
        self.check().map_err(|_| INVALID)?;
        if sources.pin().registry_hash != self.registry_hash {
            return Err(INVALID.into());
        }
        let query = self
            .inner
            .registered_validator_context(scope, sources.source())
            .ok_or(INVALID)?;
        let context = self.context.lease();
        // The verified profile contains only same-source literal edges. Preserve
        // the actual scope's traversal policy, never add crossing permissions.
        let policy = self
            .inner
            .reference_scope_policy(scope)
            .ok_or(INVALID)?
            .clone();
        let mut compiler = CemQlSchemaDeclarationHost::new();
        compiler.set_operation_control(self.control.clone(), ROOT_EXECUTION_SCOPE_ID);
        compiler.register_scope(
            sources.source().clone(),
            Some(StandaloneExpressionContext {
                scope: query.scope,
                scope_policy: query.scope_policy,
                ..Default::default()
            }),
            policy.clone(),
        );
        for source in sources.sources().values() {
            compiler
                .register_datatype_source(source.clone())
                .map_err(|_| INVALID)?;
        }
        for (source, name, target) in sources.bindings() {
            compiler
                .bind_literal_datatype(source.scope(), name, target.declaration().clone())
                .map_err(|_| INVALID)?;
        }
        let roots = sources
            .roots()
            .iter()
            .map(|id| sources.sources()[id].clone())
            .collect::<Vec<_>>();
        let compilation = compile_datatypes(
            sources.source().ast_owner().clone(),
            &roots,
            &mut compiler,
            implementations,
            validations,
            policy.limits,
        );
        if !compilation.is_ready()
            || !compilation.reference_issues.is_empty()
            || compilation
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation())
        {
            return Err(INVALID.into());
        }
        let result = FreshDatatypeCompilation {
            sources,
            compilation,
            context,
            scope,
            active: AtomicBool::new(true),
        };
        result.check(self)?;
        Ok(result)
    }
}
