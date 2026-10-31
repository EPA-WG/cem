use super::*;
use cem_ml::schema::datatype_registry::{DatatypeKind, DatatypeKindSource};
use cem_ql::{
    datatype_compilation::{DatatypeImplementation, TokenizerBinding},
    datatype_validation::ValidationInput,
    eval::{AtomValue, Item},
    native_registrations::profiles::OperationRole,
    structural_receiver::{DatatypeAssemblyOperationBinding, DatatypeAssemblyOperationKey},
};
use std::sync::atomic::{AtomicUsize, Ordering};
fn profile_for(
    m: &mut assembly::ModuleSource,
    name: &str,
    role: OperationRole,
) -> assembly::ProfileArtifact {
    let c = &m.types.capture;
    let schema = cem_ml::schema::declaration_references::SchemaDeclarationNode::new(
        c.document().clone(),
        node(c, "schema", 0),
    )
    .unwrap();
    let mut registry = DatatypeRegistry::default();
    registry
        .insert(
            schema.clone(),
            cem_ml::schema::declaration_references::SchemaDeclarationNode::new(
                c.document().clone(),
                datatype(c, name),
            )
            .unwrap(),
        )
        .unwrap();
    let source = registry.source(&schema, name).unwrap();
    let (mode, id) = match role {
        OperationRole::Constant => (
            OperationMode::Constant,
            ImplementationIdentity::constant_interpreter(
                "constants",
                hash(b"native"),
                "constants",
                ScalarRepresentation::String,
            ),
        ),
        OperationRole::Equality => (
            OperationMode::Equality,
            ImplementationIdentity::equality(
                format!("equality-{name}"),
                hash(b"native"),
                "equality",
                ScalarRepresentation::String,
            ),
        ),
        _ => unreachable!(),
    };
    let control = OperationControl::default();
    let a = dependency_sources::export_operation(
        c,
        &source,
        mode,
        m.types.identity.clone(),
        id,
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    let p = profiles::OperationProfile {
        source: m.types.identity.clone(),
        scope: schema.node_id(),
        declaration: source.declaration().node_id(),
        kind: DatatypeKind::Scalar,
        representation: cem_ml::schema::datatype_validation::ValueRepresentation::Scalar(
            ScalarRepresentation::String,
        ),
        dependencies: BTreeMap::from([(role, a.pin.clone())]),
    };
    let sources = vec![a];
    let built = profiles::export(
        &BTreeMap::from([("operation".into(), p)]),
        &sources,
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    m.profiles.insert(
        datatype(c, name),
        assembly::ProfileSelection {
            pin: built.pin.clone(),
            id: "operation".into(),
        },
    );
    assembly::ProfileArtifact {
        bytes: built.bytes,
        pin: built.pin,
        sources,
    }
}
fn configured() -> Arc<assembly::VerifiedDatatypeModules> {
    let (m, _) = fixture();
    configured_modules(m)
}
fn configured_modules(
    mut m: BTreeMap<String, assembly::ModuleSource>,
) -> Arc<assembly::VerifiedDatatypeModules> {
    let a = profile_for(m.get_mut("app").unwrap(), "choice", OperationRole::Constant);
    let b = profile_for(
        m.get_mut("core").unwrap(),
        "scalar",
        OperationRole::Equality,
    );
    graph(&m, &[a, b])
}
fn primitives(g: &assembly::VerifiedDatatypeModules) -> Vec<DatatypeImplementation> {
    g.module_ids()
        .flat_map(|id| g.module(id).unwrap().sources().values())
        .filter_map(|s| {
            let DatatypeKindSource::Explicit(kind) = s.plan().kind else {
                return None;
            };
            Some(DatatypeImplementation {
                source: s.clone(),
                kind,
                representation: cem_ml::schema::datatype_validation::ValueRepresentation::Scalar(
                    ScalarRepresentation::String,
                ),
                accepted_bases: vec![],
                bounds: Default::default(),
                tokenizer: TokenizerBinding::Absent,
                validator: None,
            })
        })
        .collect()
}
#[derive(Debug, Clone)]
struct Engine(Arc<AtomicUsize>);
impl cem_ql::datatype_enumeration::NativeConstantInterpreter for Engine {
    fn interpret(
        &self,
        call: cem_ql::datatype_enumeration::ConstantCall<'_>,
    ) -> cem_ql::datatype_enumeration::ConstantExecution {
        check_context(call.datatype, call.runtime);
        self.0.fetch_add(1, Ordering::SeqCst);
        cem_ql::datatype_enumeration::ConstantExecution::Prepared {
            value: vec![Item::Atomic(AtomValue::String(call.token.text().into()))],
            diagnostics: vec![],
        }
    }
}
impl cem_ql::datatype_enumeration::NativeScalarEquality for Engine {
    fn compare(
        &self,
        call: cem_ql::datatype_enumeration::EqualityCall<'_>,
    ) -> cem_ql::datatype_enumeration::EqualityExecution {
        check_context(call.datatype, call.runtime);
        cem_ql::datatype_enumeration::EqualityExecution::Complete {
            equal: call.left.atom() == call.right.atom(),
            diagnostics: vec![],
        }
    }
}
fn owners(
    h: &mut FreshStructuralHost,
    g: &assembly::VerifiedDatatypeModules,
    root: DeclarationScope,
    scopes: &BTreeMap<String, DeclarationScope>,
    e: Engine,
) -> BTreeMap<DatatypeAssemblyOperationKey, DatatypeAssemblyOperationBinding> {
    for id in g.module_ids() {
        let mut context = StandaloneExpressionContext::default();
        context.scope = cem_ql::eval::QueryContextScope(match id {
            "app" => 1,
            "core" => 2,
            _ => 3,
        });
        assert!(h.set_context(if id == "app" { root } else { scopes[id] }, Some(context)));
    }
    for id in g.module_ids() {
        for p in g.module(id).unwrap().profiles().values() {
            if !used(p) {
                continue;
            }
            for (role, r) in p.requirements() {
                match role {
                    OperationRole::Constant => h
                        .register_native_constant_interpreter(r.implementation().clone(), e.clone())
                        .unwrap(),
                    OperationRole::Equality => h
                        .register_native_equality(r.implementation().clone(), e.clone())
                        .unwrap(),
                    _ => unreachable!(),
                }
            }
        }
    }
    bind_owners(h, g, root, scopes)
}
fn bind_owners(
    h: &FreshStructuralHost,
    g: &assembly::VerifiedDatatypeModules,
    root: DeclarationScope,
    scopes: &BTreeMap<String, DeclarationScope>,
) -> BTreeMap<DatatypeAssemblyOperationKey, DatatypeAssemblyOperationBinding> {
    let mut bindings = BTreeMap::new();
    for id in g.module_ids() {
        for (dtype, p) in g.module(id).unwrap().profiles() {
            if !used(p) {
                continue;
            }
            for (role, r) in p.requirements() {
                let binding = h
                    .link_dependency_operation(
                        r.clone(),
                        if id == "app" { root } else { scopes[id] },
                    )
                    .unwrap();
                bindings.insert(
                    DatatypeAssemblyOperationKey {
                        module: id.into(),
                        datatype: *dtype,
                        role: *role,
                    },
                    DatatypeAssemblyOperationBinding::Operation(Arc::new(binding)),
                );
            }
        }
    }
    bindings
}
fn input(s: &str) -> ValidationInput {
    ValidationInput {
        value: vec![Item::Atomic(AtomValue::String(s.into()))],
        candidate: vec![],
        fallback: Default::default(),
    }
}
#[test]
fn assembly_compiler_renews_selections_and_prepares_original_ordered_constants_with_owned_operations(
) {
    let g = configured();
    let (mut h, root, scopes) = prepared(&g, true, true);
    let calls = Arc::new(AtomicUsize::new(0));
    let bindings = owners(&mut h, &g, root, &scopes, Engine(calls.clone()));
    let c = h
        .compile_datatype_assembly(
            g.clone(),
            root,
            &scopes,
            &[],
            &primitives(&g),
            &BTreeMap::new(),
            &bindings,
            limits(),
        )
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    let root_id = *g.module("app").unwrap().roots().first().unwrap();
    assert_eq!(
        c.validate(&h, root_id, &input("blue"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    assert_eq!(
        c.validate(&h, root_id, &input("absent"), Default::default())
            .unwrap()
            .accepted,
        Some(false)
    );
}

fn check_context(datatype: &Item, runtime: &cem_ql::datatype_validation::ValidationRuntime<'_>) {
    let original = cem_ql::eval::retained_cem_node(datatype).unwrap();
    assert_eq!(
        runtime.query.scope,
        cem_ql::eval::QueryContextScope(if original.owner().source_uri().ends_with("/app.cem") {
            1
        } else {
            2
        })
    );
    assert!(
        runtime.query.policy_bindings.is_empty(),
        "closed native contexts must not inherit ambient inputs"
    );
}
fn compile(
    h: &mut FreshStructuralHost,
    g: &Arc<assembly::VerifiedDatatypeModules>,
    root: DeclarationScope,
    scopes: &BTreeMap<String, DeclarationScope>,
    bindings: &BTreeMap<DatatypeAssemblyOperationKey, DatatypeAssemblyOperationBinding>,
) -> Result<
    cem_ql::structural_receiver::FreshDependencyDatatypeCompilation,
    cem_ql::structural_receiver::DatatypeDependencyCompileError,
> {
    h.compile_datatype_assembly(
        g.clone(),
        root,
        scopes,
        &[],
        &primitives(g),
        &BTreeMap::new(),
        bindings,
        limits(),
    )
}
#[test]
fn assembly_compiler_requires_exact_reachable_owner_sets_and_preserves_prior_snapshots() {
    let g = configured();
    let (mut h, root, scopes) = prepared(&g, true, true);
    let bindings = owners(
        &mut h,
        &g,
        root,
        &scopes,
        Engine(Arc::new(AtomicUsize::new(0))),
    );
    let prior = compile(&mut h, &g, root, &scopes, &bindings).unwrap();
    assert!(compile(&mut h, &g, root, &scopes, &BTreeMap::new()).is_err());
    let mut extra = BTreeMap::new();
    let key = bindings.keys().next().unwrap().clone();
    let DatatypeAssemblyOperationBinding::Operation(owner) = &bindings[&key] else {
        unreachable!()
    };
    extra.insert(
        DatatypeAssemblyOperationKey {
            module: "extra".into(),
            ..key.clone()
        },
        DatatypeAssemblyOperationBinding::Operation(owner.clone()),
    );
    assert!(compile(&mut h, &g, root, &scopes, &extra).is_err());
    let mut wrong = BTreeMap::new();
    let values = bindings.values().collect::<Vec<_>>();
    for (key, value) in bindings.keys().zip(values.into_iter().rev()) {
        let DatatypeAssemblyOperationBinding::Operation(owner) = value else {
            unreachable!()
        };
        wrong.insert(
            key.clone(),
            DatatypeAssemblyOperationBinding::Operation(owner.clone()),
        );
    }
    assert!(compile(&mut h, &g, root, &scopes, &wrong).is_err());
    let root_id = *g.module("app").unwrap().roots().first().unwrap();
    assert!(prior.is_live(&h));
    assert_eq!(
        prior
            .validate(&h, root_id, &input("red"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    let foreign = root_id + 1;
    assert!(prior
        .validate(&h, foreign, &input("red"), Default::default())
        .is_err());
    let limited = cem_ql::datatype_validation::ValidationLimits {
        max_comparisons: 0,
        ..Default::default()
    };
    assert_eq!(
        prior
            .validate(&h, root_id, &input("red"), limited)
            .unwrap()
            .accepted,
        None
    );
}
#[test]
fn assembly_compiler_weak_owners_retirement_and_host_epochs_disable_use() {
    for action in ["drop", "retire", "context", "close"] {
        let g = configured();
        let (mut h, root, scopes) = prepared(&g, true, true);
        let bindings = owners(
            &mut h,
            &g,
            root,
            &scopes,
            Engine(Arc::new(AtomicUsize::new(0))),
        );
        let prior = compile(&mut h, &g, root, &scopes, &bindings).unwrap();
        match action {
            "drop" => drop(bindings),
            "retire" => {
                for b in bindings.values() {
                    let DatatypeAssemblyOperationBinding::Operation(o) = b else {
                        unreachable!()
                    };
                    o.retire();
                }
            }
            "context" => {
                assert!(h.set_context(root, Some(Default::default())));
            }
            "close" => h.close(),
            _ => unreachable!(),
        }
        assert!(!prior.is_live(&h));
        assert!(prior
            .validate(
                &h,
                *g.module("app").unwrap().roots().first().unwrap(),
                &input("blue"),
                Default::default()
            )
            .is_err());
    }
}
#[test]
fn assembly_compiler_rejects_wrong_constant_sequences_and_missing_occurrence_contexts_before_callbacks(
) {
    let g = configured();
    for (grant, ready, wrong) in [
        (false, true, false),
        (true, false, false),
        (true, true, true),
    ] {
        let ctx = if wrong {
            let p = g.module("palette").unwrap();
            context(&g).with_binding(
                "colors",
                value(
                    &g,
                    "palette",
                    &[
                        node(p.capture(), "constant", 1),
                        node(p.capture(), "constant", 0),
                        node(p.capture(), "constant", 0),
                    ],
                ),
            )
        } else {
            context(&g)
        };
        let (mut h, root, scopes) = prepared_with_context(&g, grant, ready, ctx);
        let calls = Arc::new(AtomicUsize::new(0));
        let bindings = owners(&mut h, &g, root, &scopes, Engine(calls.clone()));
        assert!(compile(&mut h, &g, root, &scopes, &bindings).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
#[test]
fn assembly_compiler_renews_mutable_constant_selections_without_replaying_on_validation() {
    let (mut m, _) = fixture();
    replace_app(&mut m,"@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=choice @base={#base} | {#native:call(\"fixture.pick\", good, wrong)} {constant @value=local}}}}");
    let g = configured_modules(m);
    let choice = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let p = g.module("palette").unwrap();
    let red = node(p.capture(), "constant", 1);
    let mut ctx = context(&g)
        .with_binding("good", context(&g).bindings["colors"].clone())
        .with_binding("wrong", value(&g, "palette", &[red, red, red]));
    ctx.native_functions
        .register(
            "fixture.pick",
            2,
            Pick {
                choice: choice.clone(),
                stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                calls: calls.clone(),
                _marker: Arc::new(()),
            },
        )
        .unwrap();
    let (mut h, root, scopes) = prepared_with_context(&g, true, true, ctx);
    let bindings = owners(
        &mut h,
        &g,
        root,
        &scopes,
        Engine(Arc::new(AtomicUsize::new(0))),
    );
    let inspection = h
        .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let prior = compile(&mut h, &g, root, &scopes, &bindings).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    choice.store(1, Ordering::SeqCst);
    assert!(inspection.is_live(&h));
    assert!(compile(&mut h, &g, root, &scopes, &bindings).is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(
        prior
            .validate(
                &h,
                *g.module("app").unwrap().roots().first().unwrap(),
                &input("blue"),
                Default::default()
            )
            .unwrap()
            .accepted,
        Some(true)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}
#[test]
fn assembly_compiler_applies_shared_literal_admission_and_fixed_work_depth_bounds() {
    let (mut m, _) = fixture();
    let app = m.get_mut("app").unwrap();
    let old = app.types.clone();
    let t="@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=choice @base={#base} | {#colors} {constant @value=local @bad=bad}}}}";
    let mut replacement = wrapped(&old.identity.uri, t, Some("choice"));
    replacement.imports = app.imports.clone();
    replacement.types.bindings = old.bindings;
    replacement.constants = app.constants.clone();
    // The source's IDs change with the extra attribute; all prior fields precede it.
    *app = replacement;
    let g = configured_modules(m);
    let (mut h, root, scopes) = prepared(&g, true, true);
    let bindings = owners(
        &mut h,
        &g,
        root,
        &scopes,
        Engine(Arc::new(AtomicUsize::new(0))),
    );
    let error = match compile(&mut h, &g, root, &scopes, &bindings) {
        Err(e) => e,
        Ok(_) => panic!("malformed literal admitted"),
    };
    assert!(error
        .issues
        .iter()
        .any(|e| e.code == "unsupported-datatype-constant-field"));
    for bounds in [(0, 100), (65, 100), (64, 0), (64, 1), (64, 100001)] {
        assert!(h
            .compile_datatype_assembly(
                g.clone(),
                root,
                &scopes,
                &[],
                &primitives(&g),
                &BTreeMap::new(),
                &bindings,
                ReferenceTraversalLimits {
                    max_depth: bounds.0,
                    max_work: bounds.1
                }
            )
            .is_err());
    }
}

#[test]
fn assembly_compiler_rejects_copied_source_owners_on_the_same_actual_host() {
    let g = configured();
    let copy = configured();
    let (mut h, root, scopes) = prepared(&g, true, true);
    let (copy_root, copy_scopes) = register(&mut h, &copy);
    let mut bindings = owners(
        &mut h,
        &g,
        root,
        &scopes,
        Engine(Arc::new(AtomicUsize::new(0))),
    );
    let key = bindings.keys().next().unwrap().clone();
    let r = copy.module(&key.module).unwrap().profiles()[&key.datatype].requirements()[&key.role]
        .clone();
    let owner = h
        .link_dependency_operation(
            r,
            if key.module == "app" {
                copy_root
            } else {
                copy_scopes[&key.module]
            },
        )
        .unwrap();
    assert!(owner.is_live_on(&h));
    bindings.insert(
        key,
        DatatypeAssemblyOperationBinding::Operation(Arc::new(owner)),
    );
    assert!(compile(&mut h, &g, root, &scopes, &bindings).is_err());
}
#[test]
fn assembly_compiler_does_not_retain_embedding_callbacks_after_host_disposal() {
    let g = configured();
    let (mut h, root, scopes) = prepared(&g, true, true);
    let calls = Arc::new(AtomicUsize::new(0));
    let weak = Arc::downgrade(&calls);
    let bindings = owners(&mut h, &g, root, &scopes, Engine(calls));
    let c = compile(&mut h, &g, root, &scopes, &bindings).unwrap();
    drop(h);
    assert!(weak.upgrade().is_none());
    drop(bindings);
    drop(c);
}

fn used(p: &profiles::VerifiedOperationProfile) -> bool {
    !matches!(p.source().attribute("name").map(|n|n.node()),Some(CemAstNode::Attribute{value:Some(v),..}) if v=="unused")
}
#[test]
fn assembly_compiler_unused_export_profiles_need_no_operation_owner() {
    let (mut m, _) = fixture();
    let mut core=wrapped("file:///app/types/core.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:core | {types | {type @name=scalar @kind=scalar} {type @name=unused @kind=scalar}}}",Some("scalar"));
    core.types
        .roots
        .insert(datatype(&core.types.capture, "unused"));
    m.insert("core".into(), core);
    let a = profile_for(m.get_mut("app").unwrap(), "choice", OperationRole::Constant);
    let b = profile_for(
        m.get_mut("core").unwrap(),
        "scalar",
        OperationRole::Equality,
    );
    let unused = profile_for(
        m.get_mut("core").unwrap(),
        "unused",
        OperationRole::Equality,
    );
    let g = graph(&m, &[a, b, unused]);
    let (mut h, root, scopes) = prepared(&g, true, true);
    let bindings = owners(
        &mut h,
        &g,
        root,
        &scopes,
        Engine(Arc::new(AtomicUsize::new(0))),
    );
    assert_eq!(bindings.len(), 2);
    assert!(compile(&mut h, &g, root, &scopes, &bindings).is_ok());
}
#[derive(Debug)]
struct Stopping;
impl cem_ql::datatype_enumeration::NativeConstantInterpreter for Stopping {
    fn interpret(
        &self,
        call: cem_ql::datatype_enumeration::ConstantCall<'_>,
    ) -> cem_ql::datatype_enumeration::ConstantExecution {
        call.runtime.control.cancel_root(None, None).unwrap();
        cem_ql::datatype_enumeration::ConstantExecution::Prepared {
            value: vec![Item::Atomic(AtomValue::String(call.token.text().into()))],
            diagnostics: vec![],
        }
    }
}
#[test]
fn assembly_compiler_never_returns_success_after_callback_cancellation() {
    let g = configured();
    let (mut h, root, scopes) = prepared(&g, true, true);
    let prior = owners(
        &mut h,
        &g,
        root,
        &scopes,
        Engine(Arc::new(AtomicUsize::new(0))),
    );
    drop(prior);
    let r = g
        .module("app")
        .unwrap()
        .profiles()
        .values()
        .next()
        .unwrap()
        .requirements()[&OperationRole::Constant]
        .clone();
    h.remove_native_implementation(&r.implementation().id)
        .unwrap();
    h.register_native_constant_interpreter(r.implementation().clone(), Stopping)
        .unwrap();
    let bindings = bind_owners(&h, &g, root, &scopes);
    assert!(compile(&mut h, &g, root, &scopes, &bindings).is_err());
}

use cem_ql::datatype_dependency_capsule as capsule;
fn identity(text: &str, uri: &str) -> SourceIdentity {
    SourceIdentity::new(uri, SourceId(1), text.as_bytes())
}
#[path = "datatype_rule_artifacts.rs"]
mod rule_artifacts;
#[test]
fn assembly_compiler_checks_imported_rule_owners_while_preparing_and_validating_constants() {
    use cem_ml::schema::declaration_references::SchemaDeclarationNode;
    use cem_ml::schema::value_contracts::{ContractName, ValueContractSource, ValueContracts};
    use cem_ql::{
        datatype_results::DatatypeResultAdapter, selected_registry::SelectedFunctionRegistry,
        structural_receiver::DatatypeDependencyRuleRegistration,
    };
    let (target, artifact) = rule_artifacts::rule_artifact(true);
    let id = artifact.id().to_owned();
    let (mut m, _) = fixture();
    let mut core=wrapped("file:///app/types/core.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:core | {types | {type @name=scalar @kind=scalar @rule={#check}}}}",Some("scalar"));
    let capsule::DependencyTarget::Query(pin) = target else {
        unreachable!()
    };
    core.types.bindings.insert(
        field(&core.types.capture, "scalar", "rule"),
        ModuleTarget::Query(pin),
    );
    m.insert("core".into(), core);
    let a = profile_for(m.get_mut("app").unwrap(), "choice", OperationRole::Constant);
    let b = profile_for(
        m.get_mut("core").unwrap(),
        "scalar",
        OperationRole::Equality,
    );
    let profiles = [a, b];
    let artifacts = [artifact];
    let control = OperationControl::default();
    let bytes = assembly::export(
        "app",
        &m,
        &artifacts,
        &profiles,
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    let g = Arc::new(
        assembly::verify(&bytes.bytes, &bytes.pin, &artifacts, &profiles, &control).unwrap(),
    );
    let (mut h, root, scopes) = prepared(&g, true, true);
    let leaf = g.leaf_capture(&id).unwrap();
    let behavior = node(leaf, "behavior", 0);
    let ctx = StandaloneExpressionContext::default().with_binding(
        "check",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(g.leaf_source(&id).unwrap().clone(), behavior)
                .unwrap()
                .query_item(),
        )),
    );
    h.prepare_datatype_assembly_lexical_scopes(&g, "core", |_, _, _| {
        (Some(ctx.clone()), Default::default())
    })
    .unwrap();
    h.allow_scope_crossing(scopes["core"], scopes[&id]);
    let bindings = owners(
        &mut h,
        &g,
        root,
        &scopes,
        Engine(Arc::new(AtomicUsize::new(0))),
    );
    assert!(compile(&mut h, &g, root, &scopes, &bindings).is_err());
    let ns = cem_ml::schema::registry::CEM_SCHEMA_URI;
    let c = capture(
        cem_ml::schema::package_sources::builtin_schema_package_source("schema")
            .unwrap()
            .schema_source,
    );
    let s = ValueContractSource::new(
        SchemaDeclarationNode::new(c.document().clone(), node(&c, "schema", 0)).unwrap(),
        ns,
        BTreeMap::from([("schema".into(), ns.into())]),
    )
    .with_captured_names(c)
    .unwrap();
    let adapter = DatatypeResultAdapter::new(
        Arc::new(ValueContracts::compile(&[s], Default::default()).unwrap()),
        ContractName::new(ns, "datatype-validation-result"),
        ContractName::new(ns, "datatype-diagnostic"),
    )
    .unwrap();
    let rule = Arc::new(
        h.link_datatype_assembly_rule(
            &g,
            &id,
            scopes[&id],
            DatatypeDependencyRuleRegistration::Query {
                functions: SelectedFunctionRegistry::new([]).unwrap(),
                adapter,
            },
        )
        .unwrap(),
    );
    let rules = BTreeMap::from([(id, rule.clone())]);
    let c = h
        .compile_datatype_assembly(
            g.clone(),
            root,
            &scopes,
            &[],
            &primitives(&g),
            &rules,
            &bindings,
            limits(),
        )
        .unwrap();
    assert_eq!(
        c.validate(
            &h,
            *g.module("app").unwrap().roots().first().unwrap(),
            &input("blue"),
            Default::default()
        )
        .unwrap()
        .accepted,
        Some(true)
    );
    rule.retire();
    assert!(!c.is_live(&h));
}
