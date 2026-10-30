//! Fresh-host query binding and invocation receipts. Nothing here is serialized
//! or inferred from source hashes, public IDs or a previous runtime's handles.
use super::*;
use crate::{
    datatype_results::DatatypeResultAdapter,
    datatype_validation::{
        validate_rules, BoundDatatypeRule, DatatypeValidationRegistry, ValidationBatch,
        ValidationInput, ValidationLimits, ValidationRuntime,
    },
    query_validator_capsule::declarations::{self, VerifiedQueryDeclaration},
    selected_registry::SelectedFunctionRegistry,
};
use cem_ml::schema::attribute_datatypes::AttributeDatatypeContextLease;

pub struct FreshQueryBinding {
    declaration: Arc<VerifiedQueryDeclaration>,
    scope: DeclarationScope,
    context: AttributeDatatypeContextLease,
    rule: BoundDatatypeRule,
    active: AtomicBool,
}
impl FreshQueryBinding {
    pub fn retire(&self) {
        self.active.store(false, Ordering::Release);
    }
    fn live(&self, host: &FreshStructuralHost) -> bool {
        self.active.load(Ordering::Acquire)
            && self.context.matches(&host.context)
            && host
                .inner
                .registered_validator_context(self.scope, self.declaration.source())
                .is_some()
    }
}
/// An enclosing request shares one budget across all of its validated rules.
/// Runtime query scope/policy are obtained from the real registered host scope.
pub struct FreshQueryInvocation {
    scope: DeclarationScope,
    context: AttributeDatatypeContextLease,
    query: crate::api::EvaluationContext,
}
impl FreshStructuralHost {
    pub fn replace_query(
        &self,
        previous: &FreshQueryBinding,
        declaration: Arc<VerifiedQueryDeclaration>,
        scope: DeclarationScope,
        datatype: AstNodeId,
        functions: &SelectedFunctionRegistry,
        adapter: DatatypeResultAdapter,
    ) -> Result<FreshQueryBinding, String> {
        self.check().map_err(|_| INVALID)?;
        if !previous.live(self) || previous.scope != scope {
            return Err(INVALID.into());
        }
        let staged = self.link_query(declaration, scope, datatype, functions, adapter)?;
        self.check().map_err(|_| INVALID)?;
        if previous
            .active
            .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(INVALID.into());
        }
        Ok(staged)
    }
    pub fn link_query(
        &self,
        declaration: Arc<VerifiedQueryDeclaration>,
        scope: DeclarationScope,
        datatype: AstNodeId,
        functions: &SelectedFunctionRegistry,
        adapter: DatatypeResultAdapter,
    ) -> Result<FreshQueryBinding, String> {
        self.check().map_err(|_| INVALID)?;
        self.inner
            .registered_validator_context(scope, declaration.source())
            .ok_or(INVALID)?;
        let value = declarations::datatype(&declaration, datatype)?;
        let mut registry = DatatypeValidationRegistry::default();
        registry
            .register_verified_query(
                declaration.contract.clone(),
                &declaration.pin().program.source,
                &self.registry_hash,
                functions,
                declaration.program.clone(),
                adapter,
            )
            .map_err(|_| INVALID)?;
        let rule = registry
            .bind(
                declaration.contract.owner(),
                declaration.contract.behavior(),
                value,
                declaration.contract.signature().kind,
            )
            .map_err(|_| INVALID)?;
        self.check().map_err(|_| INVALID)?;
        Ok(FreshQueryBinding {
            declaration,
            scope,
            context: self.context.lease(),
            rule,
            active: AtomicBool::new(true),
        })
    }
    pub fn query_invocation(
        &self,
        scope: DeclarationScope,
    ) -> Result<FreshQueryInvocation, String> {
        self.check().map_err(|_| INVALID)?;
        // The actual scope handle must belong to this host; obtain its tree
        // through a private concrete-host accessor, never an identity lookup.
        let mut query = self
            .inner
            .registered_scope_validator_context(scope)
            .ok_or(INVALID)?;
        query.execution_budget = Some(Default::default());
        self.check().map_err(|_| INVALID)?;
        Ok(FreshQueryInvocation {
            scope,
            context: self.context.lease(),
            query,
        })
    }
    pub fn validate_query(
        &self,
        invocation: &FreshQueryInvocation,
        binding: &FreshQueryBinding,
        input: &ValidationInput,
        limits: ValidationLimits,
    ) -> Result<ValidationBatch, String> {
        let check = || {
            self.check().map_err(|_| INVALID)?;
            if !invocation.context.matches(&self.context)
                || !binding.live(self)
                || invocation.scope != binding.scope
            {
                return Err(INVALID);
            }
            Ok(())
        };
        check()?;
        let runtime = ValidationRuntime {
            control: &self.control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: invocation.query.clone(),
        };
        let result = validate_rules(&[binding.rule.clone()], input, &runtime, limits);
        check()?;
        Ok(result)
    }
}
