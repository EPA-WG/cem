//! Typed scalar/list compilation in the actual selected source invocation.
use super::*;
use crate::{
    datatype_compilation::{
        compile_datatypes_with_runtime, DatatypeImplementation, DatatypeImplementations,
        DatatypeValidation, ExecutableDatatype, TokenizerBinding,
    },
    datatype_validation::{ValidationInput, ValidationLimits, ValidationRuntime},
};
use cem_ml::{
    diagnostics::Diagnostic,
    schema::{
        datatype_contracts::{
            CompiledDatatypeContract, DatatypeCompilation, DatatypeCompilationIssue,
            DatatypeReferenceIssue,
        },
        datatype_registry::DatatypeKind,
        datatype_validation::ValueRepresentation,
    },
};
#[derive(Debug)]
pub struct DatatypeDependencyCompileError {
    pub code: &'static str,
    pub diagnostics: Vec<Diagnostic>,
    pub issues: Vec<DatatypeCompilationIssue>,
    pub reference_issues: Vec<DatatypeReferenceIssue>,
}
impl DatatypeDependencyCompileError {
    fn refused(code: &'static str) -> Self {
        Self {
            code,
            diagnostics: vec![],
            issues: vec![],
            reference_issues: vec![],
        }
    }
    fn compilation(compilation: &DatatypeCompilation) -> Self {
        Self {
            code: "datatype-dependency-compilation-incomplete",
            diagnostics: compilation.diagnostics.clone(),
            issues: compilation.issues.clone(),
            reference_issues: compilation.reference_issues.clone(),
        }
    }
}
impl From<String> for DatatypeDependencyCompileError {
    fn from(_: String) -> Self {
        Self::refused("datatype-dependency-compilation-refused")
    }
}
impl From<&str> for DatatypeDependencyCompileError {
    fn from(_: &str) -> Self {
        Self::refused("datatype-dependency-compilation-refused")
    }
}
impl std::fmt::Display for DatatypeDependencyCompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code)
    }
}
impl std::error::Error for DatatypeDependencyCompileError {}
/// Private executable snapshots; never a receiver/publication or portable proof.
pub struct FreshDependencyDatatypeCompilation {
    inputs: Inputs,
    compilation: DatatypeCompilation,
    context: AttributeDatatypeContextLease,
    root: DeclarationScope,
    active: AtomicBool,
    validating: AtomicBool,
    rules: rules::RuleOwners,
}
struct ValidationEntry<'a>(&'a AtomicBool);
impl Drop for ValidationEntry<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
impl FreshDependencyDatatypeCompilation {
    fn check(&self, host: &FreshStructuralHost) -> Result<(), String> {
        host.check().map_err(|_| REFUSED)?;
        if !self.active.load(Ordering::Acquire)
            || !self.context.matches(&host.context)
            || self.inputs.borrow().registry() != host.registry_hash
            || self
                .rules
                .values()
                .any(|b| b.upgrade().is_none_or(|b| !b.is_live(host)))
            || host
                .inner
                .registered_validator_context(self.root, self.inputs.borrow().view(None)?.tree)
                .is_none()
        {
            return Err(REFUSED.into());
        }
        Ok(())
    }
    pub fn is_live(&self, host: &FreshStructuralHost) -> bool {
        self.check(host).is_ok()
    }
    pub fn retire(&self) {
        self.active.store(false, Ordering::Release)
    }
    pub fn validate(
        &self,
        host: &FreshStructuralHost,
        root: AstNodeId,
        input: &ValidationInput,
        limits: ValidationLimits,
    ) -> Result<DatatypeValidation, String> {
        self.check(host)?;
        self.validating
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| REFUSED)?;
        let _entry = ValidationEntry(&self.validating);
        let root_view = self.inputs.borrow().view(None)?;
        if !root_view.exports.ok_or(REFUSED)?.contains(&root) {
            return Err(REFUSED.into());
        }
        let source = root_view
            .datatypes
            .ok_or(REFUSED)?
            .get(&root)
            .ok_or(REFUSED)?;
        let descriptor = self
            .compilation
            .contracts
            .iter()
            .filter_map(|c| c.as_any().downcast_ref::<ExecutableDatatype>())
            .find(|c| {
                c.source().declaration().identity() == source.declaration().identity()
                    && c.source().scope().identity() == source.scope().identity()
            })
            .ok_or(REFUSED)?;
        let runtime = ValidationRuntime {
            control: &host.control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: host
                .inner
                .registered_validator_context(self.root, self.inputs.borrow().view(None)?.tree)
                .ok_or(REFUSED)?,
        };
        let result = descriptor.validate_with_rule_contexts(
            input,
            &runtime,
            limits,
            &rules::Contexts {
                host,
                owners: &self.rules,
            },
        );
        self.check(host)?;
        Ok(result)
    }
}
impl FreshStructuralHost {
    /// Primitive-only convenience path; imported rules require explicit owners.
    pub fn compile_datatype_dependencies(
        &mut self,
        capsule: Arc<VerifiedDatatypeDependencies>,
        root: DeclarationScope,
        scopes: &BTreeMap<String, DeclarationScope>,
        imports: &[DatatypeDependencyImport],
        implementations: &[DatatypeImplementation],
        limits: ReferenceTraversalLimits,
    ) -> Result<FreshDependencyDatatypeCompilation, DatatypeDependencyCompileError> {
        self.compile_datatype_dependencies_with_rules(
            capsule,
            root,
            scopes,
            imports,
            implementations,
            &BTreeMap::new(),
            limits,
        )
    }
    /// Renew source selection and bind explicitly owned imported rules. No
    /// registration may outlive its actual host/context or caller-held owner.
    pub fn compile_datatype_dependencies_with_rules(
        &mut self,
        capsule: Arc<VerifiedDatatypeDependencies>,
        root: DeclarationScope,
        scopes: &BTreeMap<String, DeclarationScope>,
        imports: &[DatatypeDependencyImport],
        implementations: &[DatatypeImplementation],
        bindings: &BTreeMap<String, Arc<FreshDatatypeRuleBinding>>,
        limits: ReferenceTraversalLimits,
    ) -> Result<FreshDependencyDatatypeCompilation, DatatypeDependencyCompileError> {
        self.compile_inputs(
            Inputs::Dependencies(capsule),
            root,
            scopes,
            imports,
            implementations,
            bindings,
            limits,
        )
    }
    pub(super) fn compile_inputs(
        &mut self,
        inputs: Inputs,
        root: DeclarationScope,
        scopes: &BTreeMap<String, DeclarationScope>,
        imports: &[DatatypeDependencyImport],
        implementations: &[DatatypeImplementation],
        bindings: &BTreeMap<String, Arc<FreshDatatypeRuleBinding>>,
        limits: ReferenceTraversalLimits,
    ) -> Result<FreshDependencyDatatypeCompilation, DatatypeDependencyCompileError> {
        let input = inputs.borrow();
        self.check().map_err(|_| REFUSED)?;
        if bindings.is_empty() && input.root_has_rules() {
            return Err(DatatypeDependencyCompileError::refused(
                "datatype-dependency-rule-bindings-unavailable",
            ));
        }
        let Environment {
            context,
            catalog,
            allowed,
            expected,
            captures,
            required,
            rules: required_rules,
        } = self.dependency_environment(input, root, scopes, imports, limits)?;
        if bindings.is_empty() && !required_rules.is_empty() {
            return Err(DatatypeDependencyCompileError::refused(
                "datatype-dependency-rule-bindings-unavailable",
            ));
        }
        let (validations, owners) =
            rules::registrations(self, input, scopes, bindings, &required_rules)?;
        let mut primitive = DatatypeImplementations::default();
        if implementations.len() > allowed.len() {
            return Err(DatatypeDependencyCompileError::refused(
                "datatype-dependency-primitive-metadata-required",
            ));
        }
        for implementation in implementations {
            if !allowed
                .get(&implementation.source.declaration().identity())
                .is_some_and(|source| {
                    source.scope().identity() == implementation.source.scope().identity()
                })
                || !matches!(
                    implementation.kind,
                    DatatypeKind::Scalar | DatatypeKind::Lexical | DatatypeKind::List
                )
                || matches!(implementation.representation, ValueRepresentation::Nodes)
                || implementation.validator.is_some()
                || !matches!(implementation.tokenizer, TokenizerBinding::Absent)
            {
                return Err(DatatypeDependencyCompileError::refused(
                    "datatype-dependency-primitive-metadata-required",
                ));
            }
            primitive
                .register(implementation.clone())
                .map_err(|_| REFUSED)?;
        }
        let root_view = input.view(None)?;
        let runtime = ValidationRuntime {
            control: &self.control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: self
                .inner
                .registered_validator_context(root, root_view.tree)
                .ok_or(REFUSED)?,
        };
        let roots = root_view
            .exports
            .ok_or(REFUSED)?
            .iter()
            .map(|id| root_view.datatypes.ok_or(REFUSED).map(|d| d[id].clone()))
            .collect::<Result<Vec<_>, _>>()?;
        let mut adapter = host::Host {
            inner: &mut self.inner,
            catalog: &catalog,
            allowed: &allowed,
            expected: &expected,
            captures: &captures,
            lease: &context,
            closed: &self.closed,
            control: &self.control,
        };
        let compilation = compile_datatypes_with_runtime(
            root_view.capture.document().clone(),
            &roots,
            &mut adapter,
            &primitive,
            &validations,
            limits,
            &runtime,
            Default::default(),
        );
        adapter.check().map_err(|_| REFUSED)?;
        if !compilation.is_ready()
            || !compilation.reference_issues.is_empty()
            || compilation
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation())
        {
            return Err(DatatypeDependencyCompileError::compilation(&compilation));
        }
        for site in &compilation.dependency_sites {
            let [target] = site.targets.as_slice() else {
                return Err(DatatypeDependencyCompileError::refused(
                    "datatype-dependency-selection-mismatch",
                ));
            };
            if expected.get(&site.attribute.identity()) != Some(&target.identity()) {
                return Err(DatatypeDependencyCompileError::refused(
                    "datatype-dependency-selection-mismatch",
                ));
            }
        }
        if required.iter().any(|id| {
            !compilation
                .dependency_sites
                .iter()
                .any(|s| s.attribute.identity() == *id)
        }) {
            return Err(DatatypeDependencyCompileError::refused(
                "datatype-dependency-selection-incomplete",
            ));
        }
        let result = FreshDependencyDatatypeCompilation {
            inputs,
            compilation,
            context,
            root,
            active: AtomicBool::new(true),
            validating: AtomicBool::new(false),
            rules: owners,
        };
        result.check(self)?;
        Ok(result)
    }
}
