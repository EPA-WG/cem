//! Explicit lifecycle compilation. Registrations select contracts for original
//! declarations; names and successful reference selection alone confer no authority.
use crate::{
    datatype_facets::{FacetProfileBinding, RegisteredFacetProfile},
    datatype_conversion::{BoundDatatypeConverter, ConverterBinding},
    datatype_serialization::{BoundListSerializer, ListSerializerBinding},
    datatype_preparation::{BoundLexicalPreparation, PreparationBinding},
    datatype_enumeration::{
        ConstantBinding, ConstantPreparationLimits, EnumerationRestriction, EqualityBinding,
    },
    datatype_validation::{BoundDatatypeRule, DatatypeValidationRegistry, ValidationRuntime},
    eval::RetainedCemNode,
};
use cem_ml::{
    parser::{document::CemDocument, CemAstNode},
    schema::{
        datatype_contracts::{
            CompiledDatatypeContract, DatatypeCompilation, DatatypeCompilationIssue,
            DatatypeIssueState, DatatypeReferenceIssue, ItemBounds, RegisteredTokenizer,
        },
        datatype_registry::{
            traverse_native_datatype_dependencies, DatatypeDependencyHost, DatatypeDependencyRole,
            DatatypeKind, DatatypeKindSource, DatatypeSource, DatatypeSourcePlan,
            DatatypeTraversalNode,
        },
        datatype_validation::ValueRepresentation,
        declaration_references::SchemaDeclarationNode,
        function_references::ScalarCompilationBudget,
        reference_traversal::ReferenceTraversalLimits,
        registry::CEM_SCHEMA_URI,
    },
};
use std::{
    any::Any,
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
mod enumeration;
mod retained_constants;
mod execution;
pub use execution::{CardinalityRejection, DatatypeValidation, ItemValidationOccurrence};

#[derive(Debug, Clone)]
pub enum TokenizerBinding {
    Absent,
    Unavailable,
    Ready(RegisteredTokenizer),
    /// Preserve every inherited tokenizer's admission and exact decoded spans.
    CheckedReplacement(RegisteredTokenizer),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseCompatibility {
    pub kind: DatatypeKind,
    pub representation: ValueRepresentation,
}
/// Host authority over one original declaration, never a local-name lookup.
#[derive(Debug, Clone)]
pub struct DatatypeImplementation {
    pub source: DatatypeSource,
    pub kind: DatatypeKind,
    pub representation: ValueRepresentation,
    pub accepted_bases: Vec<BaseCompatibility>,
    pub bounds: ItemBounds,
    pub tokenizer: TokenizerBinding,
    pub validator: Option<(SchemaDeclarationNode, SchemaDeclarationNode)>,
}
#[derive(Debug, Clone, Default)]
pub struct DatatypeImplementations {
    entries: BTreeMap<String, DatatypeImplementation>,
    equalities: BTreeMap<String, (DatatypeSource, EqualityBinding)>,
    interpreters: BTreeMap<String, (DatatypeSource, ConstantBinding)>,
    facet_profiles: BTreeMap<String, (DatatypeSource, FacetProfileBinding)>,
    preparations: BTreeMap<String, (DatatypeSource, PreparationBinding)>,
    converters: BTreeMap<String, (DatatypeSource, ConverterBinding)>,
    list_serializers: BTreeMap<String, (DatatypeSource, ListSerializerBinding)>,
}
impl DatatypeImplementations {
    /// A lexical export capability for this exact list, never its item base.
    /// Absence keeps the descriptor usable; an unavailable selection blocks readiness.
    pub fn select_list_serializer(
        &mut self,
        source: DatatypeSource,
        binding: ListSerializerBinding,
    ) -> Result<(), &'static str> {
        let key = source.declaration().identity();
        if self.list_serializers.contains_key(&key) {
            return Err("duplicate-list-serializer-selection");
        }
        if let ListSerializerBinding::Ready(serializer) = &binding {
            let registered = &serializer.identity().source;
            if registered.declaration().identity() != key
                || registered.scope().identity() != source.scope().identity()
            {
                return Err("unrelated-list-serializer-source");
            }
        }
        self.list_serializers.insert(key, (source, binding));
        Ok(())
    }

    pub fn select_facets(
        &mut self,
        source: DatatypeSource,
        binding: FacetProfileBinding,
    ) -> Result<(), &'static str> {
        let key = source.declaration().identity();
        if self.facet_profiles.contains_key(&key) {
            return Err("duplicate-facet-profile-selection");
        }
        if let FacetProfileBinding::Ready(profile) | FacetProfileBinding::CheckedReplacement(profile) = &binding {
            if profile.source().declaration().identity() != key
                || profile.source().scope().identity() != source.scope().identity()
            {
                return Err("unrelated-facet-profile-source");
            }
        }
        self.facet_profiles.insert(key, (source, binding));
        Ok(())
    }

    /// Explicit lexical ingress selection; absence preserves typed-only validation.
    pub fn select_preparation(
        &mut self,
        source: DatatypeSource,
        binding: PreparationBinding,
    ) -> Result<(), &'static str> {
        let key = source.declaration().identity();
        if self.preparations.contains_key(&key) {
            return Err("duplicate-preparation-selection");
        }
        if let PreparationBinding::Ready(registered) | PreparationBinding::CheckedReplacement(registered) = &binding {
            let selected = &registered.identity().source;
            if selected.declaration().identity() != key
                || selected.scope().identity() != source.scope().identity()
            {
                return Err("unrelated-preparation-source");
            }
        }
        self.preparations.insert(key, (source, binding));
        Ok(())
    }

    /// No entry means inherit an available base converter, or remain validation-only.
    /// An explicit unavailable selection blocks readiness; it never falls back.
    pub fn select_converter(
        &mut self,
        source: DatatypeSource,
        binding: ConverterBinding,
    ) -> Result<(), &'static str> {
        let key = source.declaration().identity();
        if self.converters.contains_key(&key) {
            return Err("duplicate-converter-selection");
        }
        if let ConverterBinding::Ready(converter) = &binding {
            let registered = &converter.identity().source;
            if registered.declaration().identity() != key
                || registered.scope().identity() != source.scope().identity()
            {
                return Err("unrelated-converter-source");
            }
        }
        self.converters.insert(key, (source, binding));
        Ok(())
    }

    pub fn select_equality(
        &mut self,
        source: DatatypeSource,
        binding: EqualityBinding,
    ) -> Result<(), &'static str> {
        let key = source.declaration().identity();
        if self.equalities.contains_key(&key) {
            return Err("duplicate-scalar-capability-selection");
        }
        if let EqualityBinding::Ready(registered) = &binding {
            let selected = &registered.identity().source;
            if selected.declaration().identity() != key
                || selected.scope().identity() != source.scope().identity()
            {
                return Err("unrelated-scalar-capability-source");
            }
        }
        self.equalities.insert(key, (source, binding));
        Ok(())
    }
    pub fn select_constant_interpreter(
        &mut self,
        source: DatatypeSource,
        binding: ConstantBinding,
    ) -> Result<(), &'static str> {
        let key = source.declaration().identity();
        if self.interpreters.contains_key(&key) {
            return Err("duplicate-scalar-capability-selection");
        }
        if let ConstantBinding::Ready(registered) = &binding {
            let selected = &registered.identity().source;
            if selected.declaration().identity() != key
                || selected.scope().identity() != source.scope().identity()
            {
                return Err("unrelated-scalar-capability-source");
            }
        }
        self.interpreters.insert(key, (source, binding));
        Ok(())
    }
    pub fn register(&mut self, implementation: DatatypeImplementation) -> Result<(), &'static str> {
        let key = implementation.source.declaration().identity();
        let compatible = match implementation.kind {
            DatatypeKind::Node => implementation.representation == ValueRepresentation::Nodes,
            DatatypeKind::List => {
                matches!(implementation.representation, ValueRepresentation::List(_))
            }
            _ => matches!(
                implementation.representation,
                ValueRepresentation::Scalar(_)
            ),
        };
        if !compatible {
            return Err("kind-representation-mismatch");
        }
        ItemBounds::new(implementation.bounds.min, implementation.bounds.max)?;
        if implementation.kind != DatatypeKind::List
            && !matches!(implementation.tokenizer, TokenizerBinding::Absent)
        {
            return Err("tokenizer-requires-list");
        }
        if !matches!(implementation.kind, DatatypeKind::List | DatatypeKind::Node)
            && implementation.bounds != ItemBounds::default()
        {
            return Err("bounds-require-sequence");
        }
        if self.entries.contains_key(&key) {
            return Err("duplicate-datatype-implementation");
        }
        self.entries.insert(key, implementation);
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct CardinalityRestriction {
    pub source: SchemaDeclarationNode,
    pub bounds: ItemBounds,
}
#[derive(Debug, Clone)]
pub struct ExecutableDatatype {
    source: DatatypeSource,
    kind: DatatypeKind,
    representation: ValueRepresentation,
    bounds: ItemBounds,
    restrictions: Vec<CardinalityRestriction>,
    tokenizer: Option<RegisteredTokenizer>,
    base: Option<Arc<ExecutableDatatype>>,
    item: Option<Arc<ExecutableDatatype>>,
    rules: Vec<BoundDatatypeRule>,
    converter: Option<BoundDatatypeConverter>,
    list_serializer: Option<BoundListSerializer>,
    preparation: Option<BoundLexicalPreparation>,
    facet_profiles: Vec<RegisteredFacetProfile>,
    equality: Option<EqualityBinding>,
    interpreter: Option<ConstantBinding>,
    enumerations: Vec<Arc<EnumerationRestriction>>,
}
impl CompiledDatatypeContract for ExecutableDatatype {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn source(&self) -> &DatatypeSource {
        &self.source
    }
    fn representation(&self) -> ValueRepresentation {
        self.representation
    }
}
impl ExecutableDatatype {
    /// Explicit original profiles and the compiler's entire dependency check
    /// authorize typed ingress. Missing lexical fields never establish it.
    pub fn admits_external_typed(&self) -> bool {
        self.facet_profile().is_some_and(|p| p.family().is_typed_only())
    }
    pub fn facet_profile(&self) -> Option<&RegisteredFacetProfile> {
        self.facet_profiles.last()
    }
    /// Effective original registrations, oldest ancestor first and selected last.
    pub fn facet_profiles(&self) -> &[RegisteredFacetProfile] {
        &self.facet_profiles
    }

    pub fn preparation(&self) -> Option<&BoundLexicalPreparation> {
        self.preparation.as_ref()
    }

    pub fn enumerations(&self) -> &[Arc<EnumerationRestriction>] {
        &self.enumerations
    }
    pub fn converter(&self) -> Option<&BoundDatatypeConverter> {
        self.converter.as_ref()
    }
    pub fn list_serializer(&self) -> Option<&BoundListSerializer> {
        self.list_serializer.as_ref()
    }

    pub fn kind(&self) -> DatatypeKind {
        self.kind
    }
    pub fn bounds(&self) -> ItemBounds {
        self.bounds
    }
    pub fn restrictions(&self) -> &[CardinalityRestriction] {
        &self.restrictions
    }
    /// Tokenization is an explicit preparation capability, never implicit validation/conversion.
    pub fn tokenizer(&self) -> Option<&RegisteredTokenizer> {
        self.tokenizer.as_ref()
    }
    pub fn base(&self) -> Option<&Arc<ExecutableDatatype>> {
        self.base.as_ref()
    }
    pub fn item(&self) -> Option<&Arc<ExecutableDatatype>> {
        self.item.as_ref()
    }
    pub fn rules(&self) -> &[BoundDatatypeRule] {
        &self.rules
    }
}
fn issue(
    code: &'static str,
    state: DatatypeIssueState,
    source: &SchemaDeclarationNode,
) -> DatatypeCompilationIssue {
    DatatypeCompilationIssue {
        code,
        state,
        source: source.clone(),
        related: None,
    }
}
fn invalid(code: &'static str, source: &SchemaDeclarationNode) -> DatatypeCompilationIssue {
    issue(code, DatatypeIssueState::Invalid, source)
}
fn pending(code: &'static str, source: &SchemaDeclarationNode) -> DatatypeCompilationIssue {
    issue(code, DatatypeIssueState::Pending, source)
}
/// All roots share a work budget. Each dependency is selected by the existing
/// scope-aware consumer traversal; only complete selected graphs are compiled.
pub fn compile_datatypes<H: DatatypeDependencyHost>(
    owner: Arc<CemDocument>,
    roots: &[DatatypeSource],
    host: &mut H,
    implementations: &DatatypeImplementations,
    validations: &DatatypeValidationRegistry,
    limits: ReferenceTraversalLimits,
) -> DatatypeCompilation {
    compile_datatypes_inner(
        owner,
        roots,
        host,
        implementations,
        validations,
        limits,
        None,
        Default::default(),
        None,
        None,
    )
}
/// Prepare token or retained constants only with an explicit lifecycle context.
pub fn compile_datatypes_with_runtime<H: DatatypeDependencyHost>(
    owner: Arc<CemDocument>,
    roots: &[DatatypeSource],
    host: &mut H,
    implementations: &DatatypeImplementations,
    validations: &DatatypeValidationRegistry,
    limits: ReferenceTraversalLimits,
    runtime: &ValidationRuntime<'_>,
    preparation: ConstantPreparationLimits,
) -> DatatypeCompilation {
    compile_datatypes_inner(
        owner,
        roots,
        host,
        implementations,
        validations,
        limits,
        Some(runtime),
        preparation,
        None,
        None,
    )
}
/// Continue a scalar compilation attempt after function selection/binding (or
/// before it) without resetting its finite allowance. The explicit runtime also
/// supplies the operation and shared query budgets for constant preparation.
pub fn compile_datatypes_with_budget<H: DatatypeDependencyHost>(
    owner: Arc<CemDocument>,
    roots: &[DatatypeSource],
    host: &mut H,
    implementations: &DatatypeImplementations,
    validations: &DatatypeValidationRegistry,
    budget: &mut ScalarCompilationBudget,
    runtime: Option<&ValidationRuntime<'_>>,
    preparation: ConstantPreparationLimits,
) -> DatatypeCompilation {
    compile_datatypes_inner(
        owner, roots, host, implementations, validations,
        budget.remaining_limits(), runtime, preparation, Some(budget), None,
    )
}
/// Only the override transaction can supply effective edges. It has already
/// checked exact grants; the compiler rechecks current original selections.
pub(crate) fn compile_datatypes_with_rebindings<H: DatatypeDependencyHost>(
    owner: Arc<CemDocument>, roots: &[DatatypeSource], host: &mut H,
    implementations: &DatatypeImplementations, validations: &DatatypeValidationRegistry,
    budget: &mut ScalarCompilationBudget, runtime: &ValidationRuntime<'_>,
    preparation: ConstantPreparationLimits,
    rebindings: &BTreeMap<String, crate::datatype_overrides::DependencyRebinding>,
) -> DatatypeCompilation {
    compile_datatypes_inner(owner, roots, host, implementations, validations,
        budget.remaining_limits(), Some(runtime), preparation, Some(budget), Some(rebindings))
}
fn compile_datatypes_inner<H: DatatypeDependencyHost>(
    owner: Arc<CemDocument>,
    roots: &[DatatypeSource],
    host: &mut H,
    implementations: &DatatypeImplementations,
    validations: &DatatypeValidationRegistry,
    limits: ReferenceTraversalLimits,
    runtime: Option<&ValidationRuntime<'_>>,
    preparation: ConstantPreparationLimits,
    budget: Option<&mut ScalarCompilationBudget>,
    rebindings: Option<&BTreeMap<String, crate::datatype_overrides::DependencyRebinding>>,
) -> DatatypeCompilation {
    let mut output = DatatypeCompilation::new(owner);
    let shared_runtime = runtime.map(ValidationRuntime::with_query_budget);
    let runtime = shared_runtime.as_ref();
    let mut compiler = Compiler {
        host,
        implementations,
        validations,
        plans: BTreeMap::new(),
        targets: BTreeMap::new(),
        compiled: BTreeMap::new(),
        active: BTreeSet::new(),
        remaining: limits.max_work,
        max_depth: limits.max_depth,
        runtime,
        preparation,
        diagnostics: vec![],
        reference_issues: vec![],
    };
    let mut complete_roots = Vec::new();
    for root in roots {
        let id = root.declaration().identity();
        if output.sources.iter().any(|s| {
            s.declaration().identity() == id && s.scope().identity() != root.scope().identity()
        }) {
            output
                .issues
                .push(invalid("conflicting-datatype-scope", root.declaration()));
            continue;
        }
        if !output
            .sources
            .iter()
            .any(|s| s.declaration().identity() == id)
        {
            output.sources.push(root.clone());
        }
        if compiler.runtime.is_some_and(|runtime| runtime.control.check_scope(runtime.scope).is_err()) {
            output.issues.push(pending("datatype-compilation-control", root.declaration()));
            break;
        }
        if compiler.remaining == 0 {
            output
                .issues
                .push(pending("datatype-work-limit", root.declaration()));
            break;
        }
        let walk = match traverse_native_datatype_dependencies(
            root.clone(),
            compiler.host,
            ReferenceTraversalLimits {
                max_depth: limits.max_depth,
                max_work: compiler.remaining,
            },
        ) {
            Ok(walk) => walk,
            Err(_) => {
                output
                    .issues
                    .push(invalid("datatype-dependency-traversal", root.declaration()));
                continue;
            }
        };
        compiler.remaining = compiler
            .remaining
            .saturating_sub(walk.walk.resolution.work_used);
        output.dependency_sites.extend(walk.sites.clone());
        output
            .diagnostics
            .extend(walk.walk.resolution.diagnostics.clone());
        for error in &walk.walk.resolution.issues {
            let source = match &error.reference {
                DatatypeTraversalNode::Value(value, _) => compiler.host.declaration_node(value),
                DatatypeTraversalNode::Field(edge, _) | DatatypeTraversalNode::Literal(edge, _) => {
                    Some(edge.attribute.clone())
                }
            };
            output.reference_issues.push(DatatypeReferenceIssue {
                source,
                kind: error.kind,
                occurrence: error.occurrence.clone(),
                reason: error.reason.clone(),
            });
        }
        for error in &walk.issues {
            output.issues.push(invalid(
                "invalid-datatype-dependency-target",
                &error.attribute,
            ));
        }
        for plan in &walk.plans {
            for error in &plan.issues {
                output
                    .issues
                    .push(invalid("invalid-datatype-source-plan", &error.source));
            }
            if output.sources.iter().any(|s| {
                s.declaration().identity() == plan.source.declaration().identity()
                    && s.scope().identity() != plan.source.scope().identity()
            }) {
                output.issues.push(invalid(
                    "conflicting-datatype-scope",
                    plan.source.declaration(),
                ));
            }
            if !output
                .sources
                .iter()
                .any(|s| s.declaration().identity() == plan.source.declaration().identity())
            {
                output.sources.push(plan.source.clone());
            }
        }
        if !walk.is_complete() {
            output.issues.push(issue(
                "datatype-dependencies-incomplete",
                if walk.failed() {
                    DatatypeIssueState::Invalid
                } else {
                    DatatypeIssueState::Pending
                },
                root.declaration(),
            ));
            continue;
        }
        for plan in walk.plans {
            compiler
                .plans
                .insert(plan.source.declaration().identity(), plan);
        }
        for site in walk.sites {
            if rebindings.is_some() && compiler.targets.get(&site.attribute.identity())
                .is_some_and(|previous| previous.identity() != site.targets[0].identity()) {
                output.issues.push(invalid("conflicting-datatype-dependency-selection", &site.attribute));
            }
            compiler
                .targets
                .insert(site.attribute.identity(), site.targets[0].clone());
        }
        if rebindings.is_some() {
            complete_roots.push(id);
        } else if let Err(e) = compiler.compile(&id, 0) {
            output.issues.push(e);
        }
    }
    if let Some(rebindings) = rebindings {
        for binding in rebindings.values() {
            if let Err(error) = compiler.spend(1, &binding.slot) {
                output.issues.push(error);
                break;
            }
            let valid = compiler.targets.get(&binding.slot.identity()).is_some_and(|target|
                target.identity() == binding.original.declaration().identity())
                && compiler.plans.contains_key(&binding.replacement.declaration().identity())
                && output.dependency_sites.iter().any(|site| site.attribute.identity() == binding.slot.identity())
                && output.dependency_sites.iter().filter(|site| site.attribute.identity() == binding.slot.identity())
                    .all(|site| site.complete && site.role != DatatypeDependencyRole::ValidationRule
                        && site.targets.len() == 1 && site.targets[0].identity() == binding.original.declaration().identity());
            if !valid {
                output.issues.push(invalid("datatype-override-dependency-mismatch", &binding.slot));
                continue;
            }
            let from = compiler.host.source_reference(binding.slot.clone());
            let to = compiler.host.source_reference(binding.replacement.declaration().clone());
            if !compiler.host.permits_edge(&from, &to) {
                output.issues.push(invalid("datatype-override-scope-denied", &binding.slot));
                continue;
            }
            compiler.targets.insert(binding.slot.identity(), binding.replacement.declaration().clone());
            for site in &mut output.dependency_sites {
                if site.attribute.identity() == binding.slot.identity() {
                    site.targets = vec![binding.replacement.declaration().clone()];
                }
            }
        }
        // No root is compiled until all original selections and effective edges
        // are known. Transitive dependents therefore cannot reuse a pinned cache.
        if output.issues.is_empty() {
            for id in complete_roots {
                if let Err(error) = compiler.compile(&id, 0) { output.issues.push(error); }
            }
        }
    }
    if let (Some(budget), Some(root)) = (budget, roots.first()) {
        // The inner walker never exceeds the supplied remaining allowance.
        budget.spend(limits.max_work - compiler.remaining, root.declaration())
            .expect("datatype compiler accounts only its remaining budget");
    }
    output.diagnostics.extend(compiler.diagnostics);
    output.reference_issues.extend(compiler.reference_issues);
    output.contracts = compiler
        .compiled
        .into_values()
        .map(|c| c as Arc<dyn CompiledDatatypeContract>)
        .collect();
    output
}
struct Compiler<'a, 'r, H> {
    runtime: Option<&'a ValidationRuntime<'r>>,
    preparation: ConstantPreparationLimits,
    diagnostics: Vec<cem_ml::diagnostics::Diagnostic>,
    reference_issues: Vec<DatatypeReferenceIssue>,
    host: &'a mut H,
    implementations: &'a DatatypeImplementations,
    validations: &'a DatatypeValidationRegistry,
    plans: BTreeMap<String, DatatypeSourcePlan>,
    targets: BTreeMap<String, SchemaDeclarationNode>,
    compiled: BTreeMap<String, Arc<ExecutableDatatype>>,
    active: BTreeSet<String>,
    remaining: usize,
    max_depth: usize,
}
impl<H: DatatypeDependencyHost> Compiler<'_, '_, H> {
    fn spend(
        &mut self,
        amount: usize,
        source: &SchemaDeclarationNode,
    ) -> Result<(), DatatypeCompilationIssue> {
        if self.runtime.is_some_and(|runtime| runtime.control.check_scope(runtime.scope).is_err()) {
            return Err(pending("datatype-compilation-control", source));
        }
        if let Some(failure) = self.runtime.and_then(ValidationRuntime::query_failure) {
            self.diagnostics.extend(failure.diagnostics);
            return Err(pending("datatype-compilation-query-budget", source));
        }
        if amount > self.remaining {
            self.remaining = 0;
            return Err(pending("datatype-work-limit", source));
        }
        self.remaining -= amount;
        Ok(())
    }
    fn compile(
        &mut self,
        id: &str,
        depth: usize,
    ) -> Result<Arc<ExecutableDatatype>, DatatypeCompilationIssue> {
        let plan = self
            .plans
            .get(id)
            .expect("selected complete dependency has a plan")
            .clone();
        if depth > self.max_depth {
            return Err(pending("datatype-depth-limit", plan.source.declaration()));
        }
        self.spend(1, plan.source.declaration())?;
        if let Some(compiled) = self.compiled.get(id) {
            return Ok(compiled.clone());
        }
        if !self.active.insert(id.into()) {
            return Err(invalid("datatype-cycle", plan.source.declaration()));
        }
        let result = self.build(&plan, depth);
        self.active.remove(id);
        if let Ok(contract) = &result {
            self.compiled.insert(id.into(), contract.clone());
        }
        result
    }
    fn build(
        &mut self,
        plan: &DatatypeSourcePlan,
        depth: usize,
    ) -> Result<Arc<ExecutableDatatype>, DatatypeCompilationIssue> {
        let source = &plan.source;
        if source
            .declaration()
            .document()
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
        {
            return Err(invalid("invalid-datatype-source", source.declaration()));
        }
        if let CemAstNode::Element { children, .. } = source.declaration().node() {
            self.spend(children.len(), source.declaration())?;
            for id in children {
                match source.declaration().document().get(*id) {
                    Some(CemAstNode::Whitespace { .. } | CemAstNode::Comment { .. }) => {}
                    Some(CemAstNode::Text { data, .. }) if data.trim().is_empty() => {}
                    Some(CemAstNode::Reference { .. }) => {}
                    Some(CemAstNode::Element { expanded_name, .. })
                        if expanded_name.local_name == "constant" => {}
                    _ => {
                        return Err(invalid(
                            "unsupported-datatype-child",
                            &SchemaDeclarationNode::new(
                                source.declaration().document().clone(),
                                *id,
                            )
                            .unwrap(),
                        ))
                    }
                }
            }
        }
        if !plan.constant_slots.is_empty() && source.attribute("values").is_some() {
            return Err(invalid("datatype-mixed-vocabulary", source.declaration()));
        }
        self.spend(
            source.attributes().len() + plan.dependencies.len(),
            source.declaration(),
        )?;
        let name = self
            .host
            .input_expanded_name(source.declaration())
            .ok_or_else(|| pending("datatype-name-pending", source.declaration()))?;
        if (!name.namespace_uri.is_empty() && name.namespace_uri != CEM_SCHEMA_URI)
            || name.local_name != "type"
        {
            return Err(invalid("datatype-metamodel-name", source.declaration()));
        }
        let mut fields = BTreeSet::new();
        for attribute in source.attributes() {
            if self.host.input_consumed_namespace_attribute(attribute) {
                continue;
            }
            let name = self
                .host
                .input_expanded_name(attribute)
                .ok_or_else(|| pending("datatype-name-pending", attribute))?;
            if (!name.namespace_uri.is_empty() && name.namespace_uri != CEM_SCHEMA_URI)
                || !matches!(
                    name.local_name.as_str(),
                    "name" | "kind" | "base" | "list-base" | "rule" | "values" | "min-items" | "max-items"
                )
            {
                return Err(invalid("unsupported-datatype-field", attribute));
            }
            if !fields.insert(name.local_name.clone()) {
                return Err(invalid("duplicate-datatype-field", attribute));
            }
        }
        let mut base = None;
        let mut item = None;
        let mut selected_rule = None;
        for dependency in &plan.dependencies {
            let target = self
                .targets
                .get(&dependency.attribute.identity())
                .expect("complete singleton dependency")
                .clone();
            match dependency.role {
                DatatypeDependencyRole::InheritedBase | DatatypeDependencyRole::InheritedList => {
                    base = Some(self.compile(&target.identity(), depth + 1)?)
                }
                DatatypeDependencyRole::ListItem => {
                    item = Some(self.compile(&target.identity(), depth + 1)?)
                }
                DatatypeDependencyRole::ValidationRule => selected_rule = Some(target),
            }
        }
        if let Some(slot) = source.attribute("list-base") {
            let inherited = base.as_ref().expect("complete whole-list dependency");
            if inherited.kind != DatatypeKind::List {
                let mut error = invalid("incompatible-list-base", slot);
                error.related = Some(inherited.source.declaration().clone());
                return Err(error);
            }
            // Share the effective scalar item contract, including all of its
            // restrictions. Validation schedules it once per item, independently
            // of how many list ancestors contributed sequence restrictions.
            item = inherited.item.clone();
        } else if base.as_ref().is_some_and(|b| b.kind == DatatypeKind::List) {
            return Err(invalid(
                "list-base-required",
                source.attribute("base").unwrap(),
            ));
        }
        let kind = match plan.kind {
            DatatypeKindSource::Explicit(k) => k,
            DatatypeKindSource::Inherited => {
                base.as_ref()
                    .ok_or_else(|| pending("datatype-base-unavailable", source.declaration()))?
                    .kind
            }
            _ => return Err(invalid("datatype-kind-required", source.declaration())),
        };
        let own = self
            .implementations
            .entries
            .get(&source.declaration().identity())
            .cloned();
        if own
            .as_ref()
            .is_some_and(|o| o.source.scope().identity() != source.scope().identity())
        {
            return Err(invalid(
                "datatype-implementation-scope",
                source.declaration(),
            ));
        }
        if own.as_ref().is_some_and(|o| o.kind != kind) {
            return Err(invalid(
                "datatype-implementation-kind",
                source.declaration(),
            ));
        }
        let representation = own
            .as_ref()
            .map(|o| o.representation)
            .or_else(|| base.as_ref().map(|b| b.representation))
            .ok_or_else(|| pending("datatype-implementation-unavailable", source.declaration()))?;
        if let Some(base) = &base {
            if representation != base.representation
                || (kind != base.kind
                    && !own.as_ref().is_some_and(|o| {
                        o.accepted_bases.contains(&BaseCompatibility {
                            kind: base.kind,
                            representation: base.representation,
                        })
                    }))
            {
                let mut e = invalid(
                    "incompatible-datatype-base",
                    source
                        .attribute("list-base")
                        .or_else(|| source.attribute("base"))
                        .unwrap(),
                );
                e.related = Some(base.source.declaration().clone());
                return Err(e);
            }
        }
        if kind == DatatypeKind::List {
            let item = item
                .as_ref()
                .ok_or_else(|| invalid("list-item-required", source.declaration()))?;
            if !matches!((representation,item.representation),(ValueRepresentation::List(a),ValueRepresentation::Scalar(b)) if a==b)
            {
                return Err(invalid(
                    "incompatible-list-item",
                    source
                        .attribute("list-base")
                        .or_else(|| source.attribute("base"))
                        .unwrap(),
                ));
            }
        }
        if let Some(values) = source.attribute("values") {
            if matches!(kind, DatatypeKind::Node | DatatypeKind::List) {
                return Err(invalid("unsupported-datatype-facet", values));
            }
        }
        if !plan.constant_slots.is_empty()
            && matches!(kind, DatatypeKind::Node | DatatypeKind::List)
        {
            return Err(invalid(
                "unsupported-datatype-child",
                &plan.constant_slots[0],
            ));
        }
        let authored = read_bounds(source)?;
        if !matches!(kind, DatatypeKind::List | DatatypeKind::Node)
            && (source.attribute("min-items").is_some() || source.attribute("max-items").is_some())
        {
            return Err(invalid("bounds-require-sequence", source.declaration()));
        }
        let mut bounds = base.as_ref().map(|b| b.bounds).unwrap_or_default();
        let mut restrictions = base
            .as_ref()
            .map(|b| b.restrictions.clone())
            .unwrap_or_default();
        for restriction in [own.as_ref().map(|o| o.bounds), Some(authored)]
            .into_iter()
            .flatten()
        {
            bounds = bounds
                .intersect(restriction)
                .map_err(|code| invalid(code, source.declaration()))?;
            if restriction != ItemBounds::default() {
                restrictions.push(CardinalityRestriction {
                    source: source.declaration().clone(),
                    bounds: restriction,
                });
            }
        }
        let tokenizer = match own.as_ref().map(|o| &o.tokenizer) {
            Some(TokenizerBinding::Unavailable) => {
                return Err(pending(
                    "datatype-tokenizer-unavailable",
                    source.declaration(),
                ))
            }
            Some(TokenizerBinding::Ready(_)) if source.attribute("list-base").is_some() => {
                return Err(invalid(
                    "tokenizer-base-replacement-unsupported",
                    source.declaration(),
                ))
            }
            Some(TokenizerBinding::Ready(t)) => Some(t.clone()),
            Some(TokenizerBinding::CheckedReplacement(t)) => {
                let inherited = base
                    .as_ref()
                    .filter(|b| b.kind == DatatypeKind::List)
                    .ok_or_else(|| {
                        invalid("tokenizer-replacement-requires-base", source.declaration())
                    })?;
                let original = inherited.tokenizer.as_ref().ok_or_else(|| {
                    pending(
                        "tokenizer-replacement-base-unavailable",
                        source.declaration(),
                    )
                })?;
                self.spend(
                    original.invocations().saturating_add(t.invocations()),
                    source.declaration(),
                )?;
                Some(t.clone().checked_replacement(original))
            }
            _ => base.as_ref().and_then(|b| b.tokenizer.clone()),
        };
        self.spend(
            base.as_ref().map_or(0, |b| {
                b.rules.len() + b.restrictions.len() + b.enumerations.len()
            }),
            source.declaration(),
        )?;
        let mut rules = base.as_ref().map(|b| b.rules.clone()).unwrap_or_default();
        let mut local_rules = vec![];
        if let Some(validator) = own.as_ref().and_then(|o| o.validator.clone()) {
            local_rules.push(validator);
        }
        if let Some(rule) = selected_rule {
            let owner = self
                .host
                .declaration_schema(&rule)
                .ok_or_else(|| pending("validation-owner-unavailable", &rule))?;
            if !local_rules
                .iter()
                .any(|(o, b)| o.identity() == owner.identity() && b.identity() == rule.identity())
            {
                local_rules.push((owner, rule));
            }
        }
        self.spend(local_rules.len(), source.declaration())?;
        for (owner, behavior) in local_rules {
            let signature = self
                .validations
                .signature(&owner, &behavior)
                .ok_or_else(|| pending("validation-capability-unavailable", &behavior))?;
            if signature.kind != kind || signature.value != representation {
                return Err(invalid("validation-signature-incompatible", &behavior));
            }
            let tree = self
                .host
                .input_source_tree(source.declaration())
                .filter(|tree| Arc::ptr_eq(tree.ast_owner(), source.declaration().document()))
                .ok_or_else(|| {
                    pending("datatype-source-owner-unavailable", source.declaration())
                })?;
            let datatype = RetainedCemNode::new(tree, source.declaration().node_id())
                .unwrap()
                .query_item();
            rules.push(
                self.validations
                    .bind(&owner, &behavior, datatype, kind)
                    .map_err(|_| invalid("validation-binding-invalid", &behavior))?,
            );
        }
        if rules.is_empty()
            && matches!(
                kind,
                DatatypeKind::Lexical | DatatypeKind::Grammar | DatatypeKind::Reference
            )
        {
            return Err(pending(
                "validation-capability-unavailable",
                source.declaration(),
            ));
        }
        let converter = match self
            .implementations
            .converters
            .get(&source.declaration().identity())
        {
            Some((registered, _)) if registered.scope().identity() != source.scope().identity() => {
                return Err(invalid("converter-source-scope", source.declaration()))
            }
            Some((_, ConverterBinding::Unavailable)) => {
                return Err(pending(
                    "datatype-converter-unavailable",
                    source.declaration(),
                ))
            }
            Some((_, ConverterBinding::Ready(converter))) => {
                if converter.signature().kind != kind
                    || converter.signature().output != representation
                {
                    return Err(invalid(
                        "converter-output-incompatible",
                        source.declaration(),
                    ));
                }
                let tree = self
                    .host
                    .input_source_tree(source.declaration())
                    .ok_or_else(|| {
                        pending("datatype-source-owner-unavailable", source.declaration())
                    })?;
                Some(
                    converter
                        .bind(tree)
                        .ok_or_else(|| invalid("converter-source-owner", source.declaration()))?,
                )
            }
            None => base.as_ref().and_then(|base| base.converter.clone()),
        };
        let list_serializer = match self
            .implementations
            .list_serializers
            .get(&source.declaration().identity())
        {
            Some((registered, _)) if registered.scope().identity() != source.scope().identity() => {
                return Err(invalid("list-serializer-source-scope", source.declaration()))
            }
            Some(_) if kind != DatatypeKind::List => {
                return Err(invalid("list-serializer-requires-list", source.declaration()))
            }
            Some((_, ListSerializerBinding::Unavailable)) => {
                return Err(pending("datatype-list-serializer-unavailable", source.declaration()))
            }
            Some((_, ListSerializerBinding::Ready(serializer))) => {
                if ValueRepresentation::List(serializer.signature().item) != representation {
                    return Err(invalid("list-serializer-input-incompatible", source.declaration()));
                }
                let tree = self.host.input_source_tree(source.declaration()).ok_or_else(|| {
                    pending("datatype-source-owner-unavailable", source.declaration())
                })?;
                Some(serializer.bind(tree).ok_or_else(|| {
                    invalid("list-serializer-source-owner", source.declaration())
                })?)
            }
            None => base.as_ref().and_then(|b| b.list_serializer.clone()),
        };
        let preparation = match self
            .implementations
            .preparations
            .get(&source.declaration().identity())
        {
            Some((registered, _)) if registered.scope().identity() != source.scope().identity() => {
                return Err(invalid("preparation-source-scope", source.declaration()))
            }
            Some((_, PreparationBinding::Unavailable)) => {
                return Err(pending(
                    "datatype-preparation-unavailable",
                    source.declaration(),
                ))
            }
            Some((
                _,
                binding @ (PreparationBinding::Ready(selected)
                | PreparationBinding::CheckedReplacement(selected)),
            )) => {
                // Ordinary selection cannot replace lexical admission. The
                // explicit checked mode retains the original base guard chain.
                if base.is_some() && matches!(binding, PreparationBinding::Ready(_)) {
                    return Err(invalid(
                        "preparation-base-replacement-unsupported",
                        source.declaration(),
                    ));
                }
                if selected.signature().kind != kind || selected.signature().output != representation {
                    return Err(invalid(
                        "preparation-output-incompatible",
                        source.declaration(),
                    ));
                }
                if kind == DatatypeKind::List
                    && (tokenizer.is_none() || item.as_ref().and_then(|i| i.preparation()).is_none())
                {
                    return Err(pending(
                        "list-item-preparation-unavailable",
                        source.declaration(),
                    ));
                }
                let tree = self
                    .host
                    .input_source_tree(source.declaration())
                    .ok_or_else(|| {
                        pending("datatype-source-owner-unavailable", source.declaration())
                    })?;
                let selected = selected
                    .bind(tree)
                    .ok_or_else(|| invalid("preparation-source-owner", source.declaration()))?;
                Some(
                    if matches!(binding, PreparationBinding::CheckedReplacement(_)) {
                        let inherited = base.as_ref().ok_or_else(|| {
                            invalid(
                                "preparation-replacement-requires-base",
                                source.declaration(),
                            )
                        })?;
                        let original = inherited.preparation.as_ref().ok_or_else(|| {
                            pending(
                                "preparation-replacement-base-unavailable",
                                source.declaration(),
                            )
                        })?;
                        self.spend(original.invocations(), source.declaration())?;
                        selected.checked_replacement(original)
                    } else {
                        selected
                    },
                )
            }
            None => base.as_ref().and_then(|base| base.preparation.clone()),
        };
        if preparation
            .as_ref()
            .is_some_and(|p| p.signature().output != representation)
        {
            return Err(invalid(
                "inherited-preparation-output-incompatible",
                source.declaration(),
            ));
        }
        let facet_profiles = match self
            .implementations
            .facet_profiles
            .get(&source.declaration().identity())
        {
            Some((registered, _)) if registered.scope().identity() != source.scope().identity() => {
                return Err(invalid("facet-profile-source-scope", source.declaration()))
            }
            Some((_, FacetProfileBinding::Unavailable)) => {
                return Err(pending(
                    "datatype-facet-profile-unavailable",
                    source.declaration(),
                ))
            }
            Some((
                _,
                binding @ (FacetProfileBinding::Ready(profile)
                | FacetProfileBinding::CheckedReplacement(profile)),
            )) => {
                if base.is_some() && matches!(binding, FacetProfileBinding::Ready(_)) {
                    return Err(invalid(
                        "facet-profile-base-replacement-unsupported",
                        source.declaration(),
                    ));
                }
                if profile.family().representation() != representation {
                    return Err(invalid(
                        "facet-profile-representation-incompatible",
                        source.declaration(),
                    ));
                }
                let mut profiles = if matches!(binding, FacetProfileBinding::CheckedReplacement(_)) {
                    let inherited = base.as_ref().ok_or_else(|| {
                        invalid(
                            "facet-profile-replacement-requires-base",
                            source.declaration(),
                        )
                    })?;
                    if inherited.facet_profiles.is_empty() {
                        return Err(pending(
                            "facet-profile-replacement-base-unavailable",
                            source.declaration(),
                        ));
                    }
                    self.spend(
                        inherited.facet_profiles.len().saturating_add(1),
                        source.declaration(),
                    )?;
                    inherited.facet_profiles.clone()
                } else {
                    self.spend(1, source.declaration())?;
                    vec![]
                };
                profiles.push(profile.clone());
                profiles
            }
            None => {
                let inherited = base
                    .as_ref()
                    .map_or(&[][..], |b| b.facet_profiles.as_slice());
                self.spend(inherited.len(), source.declaration())?;
                inherited.to_vec()
            }
        };
        if facet_profiles
            .iter()
            .any(|p| p.family().representation() != representation)
        {
            return Err(invalid(
                "facet-profile-representation-incompatible",
                source.declaration(),
            ));
        }
        let typed = facet_profiles.iter().any(|p| p.family().is_typed_only());
        if typed && facet_profiles.iter().any(|p| !p.family().is_typed_only()) {
            return Err(invalid(
                "facet-profile-input-incompatible",
                source.declaration(),
            ));
        }
        if typed
            && (!matches!(kind, DatatypeKind::Scalar | DatatypeKind::List)
                || preparation.is_some()
                || tokenizer.is_some()
                || base.as_ref().is_some_and(|b| !b.admits_external_typed())
                || item.as_ref().is_some_and(|i| !i.admits_external_typed()))
        {
            return Err(invalid(
                "typed-profile-lexical-dependency",
                source.declaration(),
            ));
        }
        // A root implementation registration explicitly admits its native value
        // representation. Optional additional rules restrict that admission.
        let equality = self.equality(source, base.as_deref(), representation)?;
        let interpreter = self.interpreter(source, base.as_deref(), representation)?;
        let enumerations = base
            .as_ref()
            .map(|b| b.enumerations.clone())
            .unwrap_or_default();
        let mut descriptor = ExecutableDatatype {
            source: source.clone(),
            kind,
            representation,
            bounds,
            restrictions,
            tokenizer,
            base,
            item,
            rules,
            converter,
            list_serializer,
            preparation,
            facet_profiles,
            equality,
            interpreter,
            enumerations,
        };
        if source.attribute("values").is_some() || !plan.constant_slots.is_empty() {
            let restriction = self.prepare_enumeration(&descriptor, plan)?;
            descriptor.enumerations.push(Arc::new(restriction));
        }
        Ok(Arc::new(descriptor))
    }
}
fn read_bounds(source: &DatatypeSource) -> Result<ItemBounds, DatatypeCompilationIssue> {
    let read = |name: &str| -> Result<Option<usize>, DatatypeCompilationIssue> {
        let Some(field) = source.attribute(name) else {
            return Ok(None);
        };
        match field.node() {
            CemAstNode::Attribute {
                value: Some(value),
                value_nodes,
                ..
            } if value_nodes.is_empty()
                && !value.is_empty()
                && value.bytes().all(|b| b.is_ascii_digit()) =>
            {
                value
                    .parse()
                    .map(Some)
                    .map_err(|_| invalid("invalid-item-bound", field))
            }
            _ => Err(invalid("invalid-item-bound", field)),
        }
    };
    ItemBounds::new(read("min-items")?.unwrap_or(0), read("max-items")?)
        .map_err(|code| invalid(code, source.declaration()))
}
