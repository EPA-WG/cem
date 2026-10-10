//! Explicit host-authorized datatype replacement. Source ASTs and original
//! lexical selections stay immutable; only a complete prepared model is published.
use crate::{
    attribute_activation::activate_attribute_datatypes,
    datatype_compilation::{compile_datatypes_with_rebindings, DatatypeImplementations},
    datatype_enumeration::ConstantPreparationLimits,
    datatype_names::{DatatypeNameCatalog, DatatypeNameLookup},
    datatype_validation::{DatatypeValidationRegistry, ValidationRuntime},
    schema_references::{CemQlSchemaDeclarationHost, DatatypeOverrideHostStamp},
};
use cem_ml::{
    operation_control::{ExecutionScopeId, OperationControl},
    schema::{
        datatype_contracts::{DatatypeCompilation, DatatypeIssueState},
        datatype_registry::{DatatypeDependencyHost, DatatypeDependencyRole, DatatypeSource},
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        document_model::SchemaDocumentModel,
        function_references::ScalarCompilationBudget,
        reference_traversal::ReferenceTraversalLimits,
    },
    value::reference_resolution::ReferenceResolutionHost,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Debug, Clone)]
pub struct DatatypeOverrideRequest {
    pub scope: SchemaDeclarationNode,
    pub namespace: String,
    pub name: String,
    pub expected: DatatypeSource,
    pub replacement: DatatypeSource,
    /// Exact original datatype inheritance/list-item slots. Omission pins them.
    pub rebind: Vec<SchemaDeclarationNode>,
}
/// Issued only through the embedding host's explicit authorization call. It has
/// no source syntax or serialization representation and expires with its generation.
#[derive(Debug, Clone)]
pub struct DatatypeOverrideGrant {
    registry: Arc<()>,
    generation: u64,
    request: DatatypeOverrideRequest,
}
#[derive(Debug, Clone)]
pub struct DatatypeOverrideFailure {
    pub code: &'static str,
    pub pending: bool,
    pub source: Option<SchemaDeclarationNode>,
    pub related: Option<SchemaDeclarationNode>,
    /// Complete diagnostic evidence for failed dependency/contract preparation.
    pub compilation: Option<Box<DatatypeCompilation>>,
}
fn failure(
    code: &'static str,
    pending: bool,
    source: Option<&SchemaDeclarationNode>,
) -> DatatypeOverrideFailure {
    DatatypeOverrideFailure {
        code,
        pending,
        source: source.cloned(),
        related: None,
        compilation: None,
    }
}
fn same_source(a: &DatatypeSource, b: &DatatypeSource) -> bool {
    a.declaration().identity() == b.declaration().identity()
        && a.scope().identity() == b.scope().identity()
}
fn same_request(a: &DatatypeOverrideRequest, b: &DatatypeOverrideRequest) -> bool {
    a.scope.identity() == b.scope.identity()
        && a.namespace == b.namespace
        && a.name == b.name
        && same_source(&a.expected, &b.expected)
        && same_source(&a.replacement, &b.replacement)
        && a.rebind.len() == b.rebind.len()
        && a.rebind
            .iter()
            .zip(&b.rebind)
            .all(|(a, b)| a.identity() == b.identity())
}
#[derive(Debug, Clone)]
pub(crate) struct DependencyRebinding {
    pub slot: SchemaDeclarationNode,
    /// Source selection still resolves this immutable authored target.
    pub original: DatatypeSource,
    pub replacement: DatatypeSource,
}
/// One finite allowance covers admission, compilation, and final model binding.
pub struct DatatypeOverrideOptions<'a, 'r> {
    pub implementations: &'a DatatypeImplementations,
    pub validations: &'a DatatypeValidationRegistry,
    pub runtime: &'a ValidationRuntime<'r>,
    pub limits: ReferenceTraversalLimits,
    pub preparation: ConstantPreparationLimits,
}
#[derive(Debug)]
pub struct PreparedDatatypeOverride {
    registry: Arc<()>,
    generation: u64,
    inputs: DatatypeOverrideHostStamp,
    host: CemQlSchemaDeclarationHost,
    names: Arc<DatatypeNameCatalog>,
    model: Arc<SchemaDocumentModel>,
    rebindings: BTreeMap<String, DependencyRebinding>,
    operation: OperationControl,
    scope: ExecutionScopeId,
}
impl PreparedDatatypeOverride {
    pub fn model(&self) -> &Arc<SchemaDocumentModel> {
        &self.model
    }
    pub fn names(&self) -> &Arc<DatatypeNameCatalog> {
        &self.names
    }
}
/// Immutable active model plus host-issued authorization generation. This is an
/// explicit native lifecycle owner, not an automatic package/name registry policy.
#[derive(Debug)]
pub struct DatatypeOverrideRegistry {
    identity: Arc<()>,
    generation: u64,
    names: Arc<DatatypeNameCatalog>,
    model: Arc<SchemaDocumentModel>,
    rebindings: BTreeMap<String, DependencyRebinding>,
}
fn model_ready(model: &SchemaDocumentModel) -> bool {
    model.is_ready_for_validation()
        && !model
            .compile_diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
        && model.datatype_compilation.as_ref().is_some_and(|c| {
            c.is_ready() && !c.diagnostics.iter().any(|d| d.severity.is_hard_violation())
        })
}
impl DatatypeOverrideRegistry {
    pub fn new(
        names: Arc<DatatypeNameCatalog>,
        model: SchemaDocumentModel,
    ) -> Result<Self, DatatypeOverrideFailure> {
        // Effective dependency policy belongs to this lifecycle registry. A pair
        // of public views cannot silently reset that policy into a fresh registry.
        if names.has_public_overrides() {
            return Err(failure("datatype-override-registry-required", false, None));
        }
        if !model_ready(&model) {
            return Err(failure(
                "datatype-override-active-model-unready",
                true,
                None,
            ));
        }
        let compilation = model.datatype_compilation.as_ref().unwrap();
        for source in &compilation.sources {
            if !names
                .source(source.declaration())
                .is_some_and(|s| same_source(s, source))
            {
                return Err(failure(
                    "datatype-override-source-unadmitted",
                    false,
                    Some(source.declaration()),
                ));
            }
        }
        Ok(Self {
            identity: Arc::new(()),
            generation: 0,
            names,
            model: Arc::new(model),
            rebindings: BTreeMap::new(),
        })
    }
    pub fn model(&self) -> &Arc<SchemaDocumentModel> {
        &self.model
    }
    pub fn names(&self) -> &Arc<DatatypeNameCatalog> {
        &self.names
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }

    fn check_binding(
        &self,
        request: &DatatypeOverrideRequest,
    ) -> Result<(), DatatypeOverrideFailure> {
        let source = Some(request.expected.declaration());
        let DatatypeNameLookup::Target(current) =
            self.names
                .lookup_in_scope(&request.scope, Some(&request.namespace), &request.name)
        else {
            return Err(failure(
                "datatype-override-binding-unavailable",
                true,
                source,
            ));
        };
        if !same_source(current, &request.expected) || same_source(current, &request.replacement) {
            return Err(failure(
                "datatype-override-original-mismatch",
                false,
                source,
            ));
        }
        let active = self.model.datatype_compilation.as_ref().unwrap();
        if !active
            .contracts
            .iter()
            .any(|c| same_source(c.source(), current))
        {
            return Err(failure(
                "datatype-override-original-uncompiled",
                true,
                source,
            ));
        }
        if !self
            .names
            .source(request.replacement.declaration())
            .is_some_and(|s| same_source(s, &request.replacement))
        {
            return Err(failure(
                "datatype-override-replacement-unadmitted",
                false,
                Some(request.replacement.declaration()),
            ));
        }
        Ok(())
    }
    /// This call is the host's explicit authority decision for the entire request.
    /// Selection, import, package replacement, or name matching never issues a grant.
    pub fn authorize(
        &self,
        request: &DatatypeOverrideRequest,
    ) -> Result<DatatypeOverrideGrant, DatatypeOverrideFailure> {
        self.check_binding(request)?;
        Ok(DatatypeOverrideGrant {
            registry: self.identity.clone(),
            generation: self.generation,
            request: request.clone(),
        })
    }
    pub fn revoke_authorizations(&mut self) -> Result<(), DatatypeOverrideFailure> {
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or_else(|| failure("datatype-override-generation-exhausted", false, None))?;
        Ok(())
    }
    pub fn prepare(
        &self,
        requests: &[DatatypeOverrideRequest],
        grants: &[DatatypeOverrideGrant],
        host: &CemQlSchemaDeclarationHost,
        options: DatatypeOverrideOptions<'_, '_>,
    ) -> Result<PreparedDatatypeOverride, DatatypeOverrideFailure> {
        let shared_runtime = options.runtime.with_query_budget();
        let options = DatatypeOverrideOptions {
            runtime: &shared_runtime,
            ..options
        };
        let Some(first) = requests.first() else {
            return Err(failure("datatype-override-empty-request", false, None));
        };
        let at = first.expected.declaration();
        let mut budget = ScalarCompilationBudget::new(options.limits)
            .map_err(|e| failure(e.code, true, Some(at)))?;
        let mut spend = |amount, source: &SchemaDeclarationNode| {
            budget
                .spend(amount, source)
                .map_err(|e| failure(e.code, true, Some(source)))
        };
        // Charge every request and its authority/slot comparisons before cloning
        // execution configuration or evaluating any source expression.
        for request in requests {
            spend(1, request.expected.declaration())?;
            let mut authorized = false;
            for grant in grants {
                spend(1 + request.rebind.len(), request.expected.declaration())?;
                if Arc::ptr_eq(&grant.registry, &self.identity)
                    && grant.generation == self.generation
                    && same_request(&grant.request, request)
                {
                    authorized = true;
                    break;
                }
            }
            if !authorized {
                return Err(failure(
                    "datatype-override-unauthorized",
                    false,
                    Some(request.expected.declaration()),
                ));
            }
        }
        if options
            .runtime
            .control
            .check_scope(options.runtime.scope)
            .is_err()
            || !host.datatype_override_operation_ready()
        {
            return Err(failure(
                "datatype-override-operation-stopped",
                true,
                Some(at),
            ));
        }
        if !host.datatype_override_catalog_matches(&self.names) {
            return Err(failure("datatype-override-stale-catalog", true, Some(at)));
        }
        let active = self.model.datatype_compilation.as_ref().unwrap();
        spend(
            self.names.sources().count() + active.sources.len() + self.rebindings.len(),
            at,
        )?;
        let inputs = host.datatype_override_stamp();
        let mut candidate_host = host.clone();
        candidate_host
            .set_operation_control(options.runtime.control.clone(), options.runtime.scope);
        let mut names = self.names.as_ref().clone();
        let mut roots = active.sources.clone();
        let mut rebindings = self.rebindings.clone();
        let mut public_sites = BTreeSet::new();
        let mut dependency_sites = BTreeSet::new();
        for request in requests {
            self.check_binding(request)?;
            if !public_sites.insert((
                request.scope.identity(),
                request.namespace.clone(),
                request.name.clone(),
            )) {
                return Err(failure(
                    "datatype-override-duplicate-binding",
                    false,
                    Some(request.expected.declaration()),
                ));
            }
            if host.source_tree(&request.scope).is_none()
                || !host
                    .datatype_source(request.replacement.declaration())
                    .is_some_and(|s| same_source(&s, &request.replacement))
            {
                return Err(failure(
                    "datatype-override-replacement-unavailable",
                    true,
                    Some(request.replacement.declaration()),
                ));
            }
            let from = host.source_reference(request.scope.clone());
            let to = host.source_reference(request.replacement.declaration().clone());
            if !host.permits_edge(&from, &to) {
                return Err(failure(
                    "datatype-override-scope-denied",
                    false,
                    Some(request.replacement.declaration()),
                ));
            }
            names
                .replace_public_binding(
                    &request.scope,
                    &request.namespace,
                    &request.name,
                    &request.expected,
                    &request.replacement,
                )
                .map_err(|code| failure(code, false, Some(request.expected.declaration())))?;
            roots.push(request.replacement.clone());
            for slot in &request.rebind {
                spend(1 + active.dependency_sites.len(), slot)?;
                if !dependency_sites.insert(slot.identity()) {
                    return Err(failure(
                        "datatype-override-duplicate-dependency",
                        false,
                        Some(slot),
                    ));
                }
                let matches: Vec<_> = active
                    .dependency_sites
                    .iter()
                    .filter(|site| site.attribute.identity() == slot.identity())
                    .collect();
                if matches.is_empty()
                    || matches.iter().any(|site| {
                        !site.complete
                            || site.role == DatatypeDependencyRole::ValidationRule
                            || site.targets.len() != 1
                            || site.targets[0].identity()
                                != request.expected.declaration().identity()
                    })
                {
                    return Err(failure(
                        "datatype-override-dependency-mismatch",
                        false,
                        Some(slot),
                    ));
                }
                let original = rebindings
                    .get(&slot.identity())
                    .map(|old| old.original.clone())
                    .unwrap_or_else(|| request.expected.clone());
                rebindings.insert(
                    slot.identity(),
                    DependencyRebinding {
                        slot: slot.clone(),
                        original,
                        replacement: request.replacement.clone(),
                    },
                );
            }
        }
        let names = Arc::new(names);
        candidate_host
            .install_datatype_names(names.clone())
            .map_err(|code| failure(code, true, Some(at)))?;
        // Keep roots finite and deterministic; one native identity has one scope.
        let mut seen = BTreeSet::new();
        roots.retain(|source| {
            seen.insert((source.declaration().identity(), source.scope().identity()))
        });
        let mut compilation = compile_datatypes_with_rebindings(
            active.owner().clone(),
            &roots,
            &mut candidate_host,
            options.implementations,
            options.validations,
            &mut budget,
            options.runtime,
            options.preparation,
            &rebindings,
        );
        if !compilation.is_ready() {
            return Err(compilation_failure(compilation));
        }
        let mut model = self.model.as_ref().clone();
        activate_attribute_datatypes(
            &mut model,
            &mut compilation,
            &mut candidate_host,
            budget.remaining_limits(),
            options.runtime,
        )
        .map_err(|_| failure("datatype-override-attribute-selection", true, Some(at)))?;
        model.datatype_compilation = Some(Arc::new(compilation));
        if !model_ready(&model) {
            return Err(compilation_failure(
                model
                    .datatype_compilation
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .clone(),
            ));
        }
        if options
            .runtime
            .control
            .check_scope(options.runtime.scope)
            .is_err()
            || !candidate_host.datatype_override_operation_ready()
        {
            return Err(failure(
                "datatype-override-operation-stopped",
                true,
                Some(at),
            ));
        }
        Ok(PreparedDatatypeOverride {
            registry: self.identity.clone(),
            generation: self.generation,
            inputs,
            host: candidate_host,
            names,
            model: Arc::new(model),
            rebindings,
            operation: options.runtime.control.clone(),
            scope: options.runtime.scope,
        })
    }
    /// Publish one ready transaction. Every fallible check precedes replacement;
    /// failed/stale attempts preserve the original host and active model.
    pub fn activate(
        &mut self,
        host: &mut CemQlSchemaDeclarationHost,
        candidate: PreparedDatatypeOverride,
    ) -> Result<(), DatatypeOverrideFailure> {
        if !Arc::ptr_eq(&self.identity, &candidate.registry)
            || self.generation != candidate.generation
        {
            return Err(failure("datatype-override-stale-generation", true, None));
        }
        if host.datatype_override_stamp() != candidate.inputs {
            return Err(failure("datatype-override-stale-host", true, None));
        }
        if candidate.operation.check_scope(candidate.scope).is_err()
            || !host.datatype_override_operation_ready()
        {
            return Err(failure("datatype-override-operation-stopped", true, None));
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or_else(|| failure("datatype-override-generation-exhausted", false, None))?;
        self.model.retire_preparations_replaced_by(Some(&candidate.model));
        *host = candidate.host;
        self.names = candidate.names;
        self.model = candidate.model;
        self.rebindings = candidate.rebindings;
        self.generation = generation;
        Ok(())
    }
}
fn compilation_failure(compilation: DatatypeCompilation) -> DatatypeOverrideFailure {
    let pending = compilation
        .issues
        .iter()
        .all(|i| i.state == DatatypeIssueState::Pending);
    DatatypeOverrideFailure {
        compilation: Some(Box::new(compilation)),
        ..failure("datatype-override-compilation-unready", pending, None)
    }
}
