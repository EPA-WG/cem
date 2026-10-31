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
    capsule: Arc<VerifiedDatatypeDependencies>,
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
            || self.capsule.pin().registry_hash != host.registry_hash
            || self
                .rules
                .values()
                .any(|b| b.upgrade().is_none_or(|b| !b.is_live(host)))
            || host
                .inner
                .registered_validator_context(self.root, self.capsule.source())
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
        if !self.capsule.roots().contains(&root) {
            return Err(REFUSED.into());
        }
        let source = self.capsule.sources().get(&root).ok_or(REFUSED)?;
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
                .registered_validator_context(self.root, self.capsule.source())
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
        self.check().map_err(|_| REFUSED)?;
        if bindings.is_empty() && capsule.sites().iter().any(|s| s.role == "validation-rule") {
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
        } = self.dependency_environment(&capsule, root, scopes, imports, limits)?;
        let (validations, owners) = rules::registrations(self, &capsule, scopes, bindings)?;
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
        let runtime = ValidationRuntime {
            control: &self.control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: self
                .inner
                .registered_validator_context(root, capsule.source())
                .ok_or(REFUSED)?,
        };
        let roots = capsule
            .roots()
            .iter()
            .map(|id| capsule.sources()[id].clone())
            .collect::<Vec<_>>();
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
            capsule.capture().document().clone(),
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
        if capsule.sites().iter().any(|s| {
            !compilation.dependency_sites.iter().any(|site| {
                site.attribute.node_id() == s.field
                    && Arc::ptr_eq(site.attribute.document(), capsule.capture().document())
            })
        }) {
            return Err(DatatypeDependencyCompileError::refused(
                "datatype-dependency-selection-incomplete",
            ));
        }
        let result = FreshDependencyDatatypeCompilation {
            capsule,
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
