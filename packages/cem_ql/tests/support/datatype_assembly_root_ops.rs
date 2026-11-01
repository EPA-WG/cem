use super::*;
use cem_ml::schema::datatype_contracts::LexicalInput;
use cem_ql::{
    datatype_conversion::{ConversionInput, ConversionStop, ConversionValue},
    datatype_preparation::{PreparationInput, PreparationStop},
    datatype_serialization::SerializationStop,
};
fn lexical(text: &str) -> PreparationInput {
    PreparationInput {
        lexical: LexicalInput::new(Arc::from(text), Default::default()),
        candidate: vec![],
        fallback: Default::default(),
    }
}
#[test]
fn assembly_root_operations_require_exported_roots_and_explicit_capabilities() {
    let g = configured();
    let (mut h, root, scopes) = prepared(&g, true, true);
    let bindings = owners(
        &mut h,
        &g,
        root,
        &scopes,
        Engine(Arc::new(AtomicUsize::new(0))),
    );
    let c = compile(&mut h, &g, root, &scopes, &bindings).unwrap();
    let id = *g.module("app").unwrap().roots().first().unwrap();
    assert!(matches!(
        c.prepare_lexical(&h, id, &lexical("blue"), Default::default())
            .unwrap()
            .stopped,
        Some(PreparationStop::NoPreparation)
    ));
    let conversion = ConversionInput {
        value: ConversionValue::Lexical(lexical("blue").lexical),
        candidate: vec![],
        fallback: Default::default(),
    };
    assert!(matches!(
        c.convert(&h, id, &conversion, Default::default())
            .unwrap()
            .stopped,
        Some(ConversionStop::NoConverter)
    ));
    assert!(matches!(
        c.serialize_list(&h, id, &input("blue"), Default::default())
            .unwrap()
            .stopped,
        Some(SerializationStop::NoSerializer)
    ));
    assert!(c
        .prepare_lexical(&h, id + 1, &lexical("blue"), Default::default())
        .is_err());
    assert!(c
        .convert(&h, id + 1, &conversion, Default::default())
        .is_err());
    assert!(c
        .serialize_list(&h, id + 1, &input("blue"), Default::default())
        .is_err());
}

use cem_ml::schema::{
    datatype_contracts::{RegisteredTokenizer, TokenizationError, TokenizationRequest, Tokenizer},
    datatype_validation::{CandidateRequirement, ValueRepresentation},
};
use cem_ql::{
    datatype_conversion::{
        ConversionCall, ConversionExecution, ConversionLimits, ConversionRepresentation,
        ConversionSignature, NativeDatatypeConverter,
    },
    datatype_preparation::{
        NativeLexicalPreparer, PreparationCall, PreparationExecution, PreparationLimits,
        PreparationSignature,
    },
    datatype_serialization::{
        ListSerializationCall, ListSerializationSignature, NativeListSerializer,
        SerializationExecution, SerializationLimits,
    },
    native_registrations::{NativeSignature, PreparationMode},
    structural_receiver::{FreshDatatypeRuleBinding, FreshDependencyDatatypeCompilation},
};
use std::{ops::Range, sync::Mutex};
#[derive(Default, Debug)]
struct State {
    calls: Mutex<Vec<String>>,
    mode: AtomicUsize,
}
#[derive(Clone, Debug)]
struct Ops(Arc<State>);
thread_local! {static ACTION:std::cell::RefCell<Option<Box<dyn Fn()>>>=const{std::cell::RefCell::new(None)};}
impl Ops {
    fn enter(
        &self,
        role: &str,
        datatype: &Item,
        runtime: &cem_ql::datatype_validation::ValidationRuntime<'_>,
    ) -> usize {
        let n = cem_ql::eval::retained_cem_node(datatype).unwrap();
        let uri = n.owner().source_uri();
        let scope = if uri.ends_with("core.cem") {
            2
        } else if uri.ends_with("lists.cem") {
            3
        } else if uri.ends_with("rules.cem") {
            4
        } else {
            1
        };
        assert_eq!(runtime.query.scope, cem_ql::eval::QueryContextScope(scope));
        assert!(runtime.query.policy_bindings.is_empty());
        self.0.calls.lock().unwrap().push(format!("{role}:{scope}"));
        let mode = self.0.mode.load(Ordering::SeqCst);
        if matches!(mode, 7 | 8) {
            ACTION.with(|a| a.borrow().as_ref().unwrap()());
        }
        if mode == 9 {
            runtime.control.cancel_root(None, None).unwrap();
        }
        if mode == 10 {
            let query = cem_ql::api::compile(
                "declare function local:echo(item as string) { item } local:echo(\"x\")",
                &Default::default(),
            )
            .unwrap();
            let mut context = runtime.query.clone();
            context.scope_policy = context.scope_policy.with_queue_size(2);
            for _ in 0..18 {
                let _ = cem_ql::api::evaluate(&query, &context);
            }
        }
        mode
    }
}
impl NativeLexicalPreparer for Ops {
    fn prepare(&self, c: PreparationCall<'_>) -> PreparationExecution {
        let mode = self.enter("prepare", c.datatype, c.runtime);
        match mode {
            1 => PreparationExecution::Pending(vec![]),
            2 => PreparationExecution::Rejected(vec![]),
            3 => PreparationExecution::Unavailable(vec![]),
            4 => PreparationExecution::Failed(vec![]),
            _ => PreparationExecution::Prepared {
                value: vec![if mode == 5 {
                    Item::Atomic(AtomValue::Boolean(true))
                } else if mode == 11 && c.runtime.query.scope == cem_ql::eval::QueryContextScope(1)
                {
                    Item::Atomic(AtomValue::String(format!("changed:{}", c.lexical.text)))
                } else {
                    Item::Atomic(AtomValue::String(c.lexical.text.to_string()))
                }],
                diagnostics: diagnostics(mode),
            },
        }
    }
}
impl NativeDatatypeConverter for Ops {
    fn convert(&self, c: ConversionCall<'_>) -> ConversionExecution {
        let mode = self.enter("convert", c.datatype, c.runtime);
        let ConversionValue::Lexical(value) = c.value else {
            panic!("lexical converter")
        };
        match mode {
            1 => ConversionExecution::Pending(vec![]),
            2 => ConversionExecution::Rejected {
                diagnostics: diagnostics(mode),
            },
            3 => ConversionExecution::Unavailable(vec![]),
            4 => ConversionExecution::Failed(vec![]),
            _ => ConversionExecution::Converted {
                value: vec![if mode == 5 {
                    Item::Atomic(AtomValue::Boolean(true))
                } else {
                    Item::Atomic(AtomValue::String(value.text.to_string()))
                }],
                diagnostics: diagnostics(mode),
            },
        }
    }
}
impl NativeListSerializer for Ops {
    fn serialize(&self, c: ListSerializationCall<'_>) -> SerializationExecution {
        let mode = self.enter("serialize", c.datatype, c.runtime);
        match mode {
            1 => SerializationExecution::Pending(vec![]),
            2 => SerializationExecution::Rejected {
                diagnostics: diagnostics(mode),
            },
            3 => SerializationExecution::Unavailable(vec![]),
            4 => SerializationExecution::Failed(vec![]),
            _ => SerializationExecution::Serialized {
                text: c
                    .value
                    .iter()
                    .map(|i| match i.atom().unwrap() {
                        AtomValue::String(s) => s,
                        _ => panic!("string item"),
                    })
                    .collect::<Vec<_>>()
                    .join("|"),
                diagnostics: diagnostics(mode),
            },
        }
    }
}
#[derive(Clone, Debug)]
struct Tokens(Arc<State>);
impl Tokenizer for Tokens {
    fn check_available(&self) -> Result<(), TokenizationError> {
        if self.0.mode.load(Ordering::SeqCst) == 13 {
            ACTION.with(|a| a.borrow().as_ref().unwrap()());
        }
        Ok(())
    }
    fn tokenize(&self, r: TokenizationRequest<'_>) -> Result<Vec<Range<usize>>, TokenizationError> {
        let ordinal = {
            let mut calls = self.0.calls.lock().unwrap();
            calls.push("tokens".into());
            calls.iter().filter(|c| c.as_str() == "tokens").count()
        };
        if self.0.mode.load(Ordering::SeqCst) == 12 && ordinal % 2 == 0 {
            return Ok(vec![0..r.input.text.len()]);
        }
        RegisteredTokenizer::whitespace()
            .tokenize(Some(r.input.clone()), r.control, r.scope, r.limits)
            .map(|r| r.tokens)
    }
}
fn dtype(
    m: &assembly::ModuleSource,
    name: &str,
) -> cem_ml::schema::datatype_registry::DatatypeSource {
    let c = &m.types.capture;
    let schema = cem_ml::schema::declaration_references::SchemaDeclarationNode::new(
        c.document().clone(),
        node(c, "schema", 0),
    )
    .unwrap();
    let mut r = DatatypeRegistry::default();
    r.insert(
        schema.clone(),
        cem_ml::schema::declaration_references::SchemaDeclarationNode::new(
            c.document().clone(),
            datatype(c, name),
        )
        .unwrap(),
    )
    .unwrap();
    r.source(&schema, name).unwrap()
}
fn operation_profile(
    m: &mut assembly::ModuleSource,
    name: &str,
    specs: Vec<(OperationRole, Option<OperationMode>, ImplementationIdentity)>,
) -> assembly::ProfileArtifact {
    let s = dtype(m, name);
    let control = OperationControl::default();
    let mut dependencies = BTreeMap::new();
    let mut sources = vec![];
    for (role, mode, id) in specs {
        let a = match mode {
            Some(mode) => dependency_sources::export_operation(
                &m.types.capture,
                &s,
                mode,
                m.types.identity.clone(),
                id,
                &hash(b"registry"),
                &control,
            ),
            None => dependency_sources::export_tokenizer(
                &m.types.capture,
                &s,
                if s.attribute("list-base").is_some() {
                    cem_ql::native_registrations::TokenizerMode::CheckedReplacement
                } else {
                    cem_ql::native_registrations::TokenizerMode::Ready
                },
                m.types.identity.clone(),
                id,
                &hash(b"registry"),
                &control,
            ),
        }
        .unwrap();
        dependencies.insert(role, a.pin.clone());
        sources.push(a);
    }
    let kind = if name == "scalar" {
        DatatypeKind::Scalar
    } else {
        DatatypeKind::List
    };
    let p = profiles::OperationProfile {
        source: m.types.identity.clone(),
        scope: s.scope().node_id(),
        declaration: s.declaration().node_id(),
        kind,
        representation: if kind == DatatypeKind::Scalar {
            ValueRepresentation::Scalar(ScalarRepresentation::String)
        } else {
            ValueRepresentation::List(ScalarRepresentation::String)
        },
        dependencies,
    };
    let b = profiles::export(
        &BTreeMap::from([("ops".into(), p)]),
        &sources,
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    m.profiles.insert(
        s.declaration().node_id(),
        assembly::ProfileSelection {
            pin: b.pin.clone(),
            id: "ops".into(),
        },
    );
    assembly::ProfileArtifact {
        bytes: b.bytes,
        pin: b.pin,
        sources,
    }
}
fn typed_profile(
    m: &mut assembly::ModuleSource,
    name: &str,
    family: cem_ml::schema::document_model::attribute_facets::FacetFamily,
) -> assembly::ProfileArtifact {
    operation_profile(
        m,
        name,
        vec![(
            OperationRole::FacetProfile,
            Some(OperationMode::FacetProfile(
                cem_ql::native_registrations::FacetMode::Ready,
            )),
            ImplementationIdentity::facet_profile(
                format!("typed-{name}"),
                hash(b"code"),
                "typed",
                family,
            ),
        )],
    )
}
fn preparer(id: &str) -> ImplementationIdentity {
    ImplementationIdentity::preparation(
        id,
        hash(b"code"),
        id,
        PreparationSignature {
            kind: DatatypeKind::Scalar,
            output: ValueRepresentation::Scalar(ScalarRepresentation::String),
            candidate: CandidateRequirement::Optional,
        },
    )
}
fn specs_core() -> Vec<(OperationRole, Option<OperationMode>, ImplementationIdentity)> {
    vec![
        (
            OperationRole::Preparation,
            Some(OperationMode::Preparation(PreparationMode::Ready)),
            preparer("prepare-core"),
        ),
        (
            OperationRole::Conversion,
            Some(OperationMode::Conversion),
            ImplementationIdentity::conversion(
                "convert-core",
                hash(b"code"),
                "convert",
                ConversionSignature {
                    kind: DatatypeKind::Scalar,
                    input: ConversionRepresentation::Lexical,
                    output: ValueRepresentation::Scalar(ScalarRepresentation::String),
                    candidate: CandidateRequirement::Optional,
                },
            ),
        ),
    ]
}
fn specs_list() -> Vec<(OperationRole, Option<OperationMode>, ImplementationIdentity)> {
    vec![
        (
            OperationRole::ListItems,
            Some(OperationMode::ListItems(PreparationMode::Ready)),
            ImplementationIdentity::list_items(
                "items-lists",
                hash(b"code"),
                "items",
                ScalarRepresentation::String,
            ),
        ),
        (
            OperationRole::Tokenizer,
            None,
            ImplementationIdentity::tokenizer("tokens-lists", hash(b"code"), "tokens"),
        ),
        (
            OperationRole::ListSerializer,
            Some(OperationMode::ListSerializer),
            ImplementationIdentity::list_serializer(
                "serialize-lists",
                hash(b"code"),
                "serialize",
                ListSerializationSignature {
                    item: ScalarRepresentation::String,
                    candidate: CandidateRequirement::Optional,
                },
            ),
        ),
    ]
}
struct Fixture {
    g: Arc<assembly::VerifiedDatatypeModules>,
    h: FreshStructuralHost,
    c: FreshDependencyDatatypeCompilation,
    bindings: BTreeMap<DatatypeAssemblyOperationKey, DatatypeAssemblyOperationBinding>,
    state: Arc<State>,
    root: DeclarationScope,
    rules: BTreeMap<String, Arc<FreshDatatypeRuleBinding>>,
    scalar: AstNodeId,
    list: AstNodeId,
    scopes: BTreeMap<String, DeclarationScope>,
    implementations: Vec<DatatypeImplementation>,
    control: OperationControl,
}
fn fixture_ops() -> Fixture {
    fixture_with_rule(false)
}
fn fixture_with_rule(rule: bool) -> Fixture {
    fixture_custom(
        rule,
        |bytes, artifacts, profiles| {
            Arc::new(
                assembly::verify(
                    &bytes.bytes,
                    &bytes.pin,
                    &artifacts,
                    &profiles,
                    &OperationControl::default(),
                )
                .unwrap(),
            )
        },
        |_| {},
    )
}
fn fixture_custom(
    rule: bool,
    load: impl FnOnce(
        assembly::BuiltDatatypeModules,
        Vec<cem_ql::datatype_dependency_capsule::DependencyArtifact>,
        Vec<assembly::ProfileArtifact>,
    ) -> Arc<assembly::VerifiedDatatypeModules>,
    setup: impl FnOnce(&mut FreshStructuralHost),
) -> Fixture {
    fixture_custom_profile(rule, false, load, setup)
}
fn fixture_custom_profile(
    rule: bool,
    typed: bool,
    load: impl FnOnce(
        assembly::BuiltDatatypeModules,
        Vec<cem_ql::datatype_dependency_capsule::DependencyArtifact>,
        Vec<assembly::ProfileArtifact>,
    ) -> Arc<assembly::VerifiedDatatypeModules>,
    setup: impl FnOnce(&mut FreshStructuralHost),
) -> Fixture {
    fixture_custom_host(rule, typed, load, setup, |registry, control| {
        FreshStructuralHost::new(registry, control).unwrap()
    })
}
fn fixture_custom_host(
    rule: bool,
    typed: bool,
    load: impl FnOnce(
        assembly::BuiltDatatypeModules,
        Vec<cem_ql::datatype_dependency_capsule::DependencyArtifact>,
        Vec<assembly::ProfileArtifact>,
    ) -> Arc<assembly::VerifiedDatatypeModules>,
    setup: impl FnOnce(&mut FreshStructuralHost),
    create: impl FnOnce(&str, OperationControl) -> FreshStructuralHost,
) -> Fixture {
    fixture_custom_host_with_control(rule,typed,load,setup,create,None)
}
fn fixture_custom_host_with_control(
    rule: bool,
    typed: bool,
    load: impl FnOnce(
        assembly::BuiltDatatypeModules,
        Vec<cem_ql::datatype_dependency_capsule::DependencyArtifact>,
        Vec<assembly::ProfileArtifact>,
    ) -> Arc<assembly::VerifiedDatatypeModules>,
    setup: impl FnOnce(&mut FreshStructuralHost),
    create: impl FnOnce(&str, OperationControl) -> FreshStructuralHost,
    supplied_control: Option<OperationControl>,
) -> Fixture {
    let text="@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:core | {types | {type @name=scalar @kind=scalar}}}";
    let text = if rule {
        text.replace("@kind=scalar", "@kind=scalar @rule={#check}")
    } else {
        text.into()
    };
    let mut core = wrapped("file:///app/core.cem", &text, Some("scalar"));
    let mut lists=wrapped("file:///app/lists.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:lists | {types | {type @name=names @kind=list @base={#item}}}}",Some("names"));
    let mut app=wrapped("file:///app/app.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=scalar @base={#scalar}} {type @name=names @list-base={#list}}}}",Some("scalar"));
    app.types
        .roots
        .insert(datatype(&app.types.capture, "names"));
    lists.types.bindings.insert(
        field(&lists.types.capture, "names", "base"),
        ModuleTarget::ModuleDatatype {
            module: "core".into(),
            root: datatype(&core.types.capture, "scalar"),
        },
    );
    app.types.bindings.insert(
        field(&app.types.capture, "scalar", "base"),
        ModuleTarget::ModuleDatatype {
            module: "core".into(),
            root: datatype(&core.types.capture, "scalar"),
        },
    );
    app.types.bindings.insert(
        field(&app.types.capture, "names", "list-base"),
        ModuleTarget::ModuleDatatype {
            module: "lists".into(),
            root: datatype(&lists.types.capture, "names"),
        },
    );
    lists.imports.insert("core".into());
    app.imports = BTreeSet::from(["core".into(), "lists".into()]);
    let a = if typed {
        typed_profile(
            &mut core,
            "scalar",
            cem_ml::schema::document_model::attribute_facets::FacetFamily::TypedScalar(
                ScalarRepresentation::String,
            ),
        )
    } else {
        operation_profile(&mut core, "scalar", specs_core())
    };
    let b_typed = if typed {
        Some(typed_profile(
            &mut lists,
            "names",
            cem_ml::schema::document_model::attribute_facets::FacetFamily::TypedList(
                ScalarRepresentation::String,
            ),
        ))
    } else {
        None
    };
    let profiles = if let Some(b) = b_typed {
        vec![a, b]
    } else {
        let b = operation_profile(&mut lists, "names", specs_list());
        let c = operation_profile(
            &mut app,
            "scalar",
            vec![(
                OperationRole::Preparation,
                Some(OperationMode::Preparation(
                    PreparationMode::CheckedReplacement,
                )),
                preparer("prepare-app"),
            )],
        );
        let d = operation_profile(
            &mut app,
            "names",
            vec![
                (
                    OperationRole::ListItems,
                    Some(OperationMode::ListItems(
                        PreparationMode::CheckedReplacement,
                    )),
                    ImplementationIdentity::list_items(
                        "items-app",
                        hash(b"code"),
                        "items",
                        ScalarRepresentation::String,
                    ),
                ),
                (
                    OperationRole::Tokenizer,
                    None,
                    ImplementationIdentity::tokenizer("tokens-app", hash(b"code"), "tokens"),
                ),
            ],
        );
        vec![a, b, c, d]
    };
    let artifacts = if rule {
        let (target, a) = rule_artifacts::rule_artifact(false);
        let capsule::DependencyTarget::Native(pin) = target else {
            unreachable!()
        };
        core.types.bindings.insert(
            field(&core.types.capture, "scalar", "rule"),
            ModuleTarget::Native(pin),
        );
        vec![a]
    } else {
        vec![]
    };
    let control = supplied_control.unwrap_or_default();
    let modules = BTreeMap::from([
        ("app".into(), app),
        ("core".into(), core),
        ("lists".into(), lists),
    ]);
    let bytes = assembly::export(
        "app",
        &modules,
        &artifacts,
        &profiles,
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    let g = load(bytes, artifacts, profiles.into());
    let mut h = create(&hash(b"registry"), control.clone());
    let mut scopes = BTreeMap::new();
    for id in g.module_ids().chain(g.leaf_ids()) {
        let mut context = StandaloneExpressionContext::default();
        context.scope = cem_ql::eval::QueryContextScope(match id {
            "app" => 1,
            "core" => 2,
            "lists" => 3,
            _ => 4,
        });
        scopes.insert(
            id.to_owned(),
            h.register_datatype_assembly_scope(
                &g,
                id,
                Some(context),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap(),
        );
    }
    for id in ["app", "lists"] {
        let context = StandaloneExpressionContext::default()
            .with_binding(
                "scalar",
                value(
                    &g,
                    "core",
                    &[*g.module("core").unwrap().roots().first().unwrap()],
                ),
            )
            .with_binding(
                "item",
                value(
                    &g,
                    "core",
                    &[*g.module("core").unwrap().roots().first().unwrap()],
                ),
            )
            .with_binding(
                "list",
                value(
                    &g,
                    "lists",
                    &[*g.module("lists").unwrap().roots().first().unwrap()],
                ),
            );
        h.prepare_datatype_assembly_lexical_scopes(&g, id, |_, _, _| {
            (Some(context.clone()), Default::default())
        })
        .unwrap();
    }
    for (from, to) in [("app", "core"), ("app", "lists"), ("lists", "core")] {
        h.allow_scope_crossing(scopes[from], scopes[to]);
    }
    if rule {
        let id = g.leaf_ids().next().unwrap();
        let c = g.leaf_capture(id).unwrap();
        let context = StandaloneExpressionContext::default().with_binding(
            "check",
            StandaloneExpressionBinding::any(ItemStream::once(
                RetainedCemNode::new(g.leaf_source(id).unwrap().clone(), node(c, "behavior", 0))
                    .unwrap()
                    .query_item(),
            )),
        );
        h.prepare_datatype_assembly_lexical_scopes(&g, "core", |_, _, _| {
            (Some(context.clone()), Default::default())
        })
        .unwrap();
        h.allow_scope_crossing(scopes["core"], scopes[id]);
    }
    setup(&mut h);
    let state = Arc::new(State::default());
    if rule {
        h.register_native_validator(rule_identity(), Gate(state.clone()))
            .unwrap();
    }

    for id in g.module_ids() {
        for p in g.module(id).unwrap().profiles().values() {
            for r in p.requirements().values() {
                let id = r.implementation().clone();
                match id.signature {
                    NativeSignature::Preparation(_) => {
                        h.register_native_preparer(id, Ops(state.clone())).unwrap()
                    }
                    NativeSignature::Conversion(_) => {
                        h.register_native_converter(id, Ops(state.clone())).unwrap()
                    }
                    NativeSignature::ListSerializer(_) => h
                        .register_native_list_serializer(id, Ops(state.clone()))
                        .unwrap(),
                    NativeSignature::ListItems(rep) => {
                        h.register_list_item_preparation(id, rep).unwrap()
                    }
                    NativeSignature::FacetProfile(family) => {
                        h.register_native_facet_profile(id, family).unwrap()
                    }
                    NativeSignature::TokenizerSpans => h
                        .register_native_tokenizer(id, Tokens(state.clone()))
                        .unwrap(),
                    _ => unreachable!(),
                }
            }
        }
    }
    let mut bindings = BTreeMap::new();
    for id in g.module_ids() {
        for (dtype, p) in g.module(id).unwrap().profiles() {
            for (role, r) in p.requirements() {
                let owner = if *role == OperationRole::Tokenizer {
                    DatatypeAssemblyOperationBinding::Tokenizer(Arc::new(
                        h.link_dependency_tokenizer(r.clone(), scopes[id]).unwrap(),
                    ))
                } else {
                    DatatypeAssemblyOperationBinding::Operation(Arc::new(
                        h.link_dependency_operation(r.clone(), scopes[id]).unwrap(),
                    ))
                };
                bindings.insert(
                    DatatypeAssemblyOperationKey {
                        module: id.into(),
                        datatype: *dtype,
                        role: *role,
                    },
                    owner,
                );
            }
        }
    }
    let mut rules = BTreeMap::new();
    if rule {
        let id = g.leaf_ids().next().unwrap();
        let owner = h
            .link_datatype_assembly_rule(
                &g,
                id,
                scopes[id],
                cem_ql::structural_receiver::DatatypeDependencyRuleRegistration::NativeValidator {
                    adapter: rule_adapter(),
                },
            )
            .unwrap();
        rules.insert(id.into(), Arc::new(owner));
    }
    let root = scopes["app"];
    let mut primitives = vec![];
    for id in g.module_ids() {
        for s in g.module(id).unwrap().sources().values() {
            let kind = if matches!(
                s.plan().kind,
                DatatypeKindSource::Explicit(DatatypeKind::Scalar)
            ) || s.attribute("name").is_some_and(
                |n| matches!(n.node(),CemAstNode::Attribute{value:Some(v),..}if v=="scalar"),
            ) {
                DatatypeKind::Scalar
            } else {
                DatatypeKind::List
            };
            primitives.push(DatatypeImplementation {
                source: s.clone(),
                kind,
                representation: if kind == DatatypeKind::Scalar {
                    ValueRepresentation::Scalar(ScalarRepresentation::String)
                } else {
                    ValueRepresentation::List(ScalarRepresentation::String)
                },
                accepted_bases: vec![],
                bounds: Default::default(),
                tokenizer: TokenizerBinding::Absent,
                validator: None,
            });
        }
    }
    let mut others = scopes;
    others.remove("app");
    let c = h
        .compile_datatype_assembly(
            g.clone(),
            root,
            &others,
            &[],
            &primitives,
            &rules,
            &bindings,
            limits(),
        )
        .unwrap();
    let scalar = datatype(g.module("app").unwrap().capture(), "scalar");
    let list = datatype(g.module("app").unwrap().capture(), "names");
    Fixture {
        g,
        h,
        c,
        bindings,
        state,
        root,
        rules,
        scalar,
        list,
        scopes: others,
        implementations: primitives,
        control,
    }
}
fn list_input(words: &[&str]) -> ValidationInput {
    ValidationInput {
        value: words
            .iter()
            .map(|s| Item::Atomic(AtomValue::String((*s).into())))
            .collect(),
        candidate: vec![],
        fallback: Default::default(),
    }
}
fn conversion(text: &str) -> ConversionInput {
    ConversionInput {
        value: ConversionValue::Lexical(lexical(text).lexical),
        candidate: vec![],
        fallback: Default::default(),
    }
}
#[test]
fn assembly_root_operations_use_each_inherited_and_checked_source_context_and_preserve_decoded_spans(
) {
    let f = fixture_ops();
    let p =
        f.c.prepare_lexical(&f.h, f.scalar, &lexical("blue"), Default::default())
            .unwrap();
    assert_eq!(p.accepted, Some(true));
    assert_eq!(p.preparations, 2);
    assert_eq!(
        p.value.unwrap()[0].atom(),
        Some(AtomValue::String("blue".into()))
    );
    assert_eq!(
        *f.state.calls.lock().unwrap(),
        vec!["prepare:2", "prepare:1"]
    );
    f.state.calls.lock().unwrap().clear();
    let p =
        f.c.prepare_lexical(&f.h, f.list, &lexical("  α  beta "), Default::default())
            .unwrap();
    assert_eq!(p.accepted, Some(true));
    assert_eq!(p.token_spans, vec![2..4, 6..10]);
    assert_eq!(p.preparations, 5);
    assert_eq!(p.value.unwrap().len(), 2);
    assert_eq!(
        *f.state.calls.lock().unwrap(),
        vec!["tokens", "tokens", "prepare:2", "prepare:2"]
    );
    f.state.calls.lock().unwrap().clear();
    assert_eq!(
        f.c.convert(&f.h, f.scalar, &conversion("red"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    assert_eq!(*f.state.calls.lock().unwrap(), vec!["convert:2"]);
    f.state.calls.lock().unwrap().clear();
    let s =
        f.c.serialize_list(
            &f.h,
            f.list,
            &list_input(&["red", "blue"]),
            Default::default(),
        )
        .unwrap();
    assert_eq!(s.accepted, Some(true));
    assert_eq!(s.text.as_deref(), Some("red|blue"));
    assert_eq!(*f.state.calls.lock().unwrap(), vec!["serialize:3"]);
    let p =
        f.c.prepare_lexical(&f.h, f.list, &lexical(" \t "), Default::default())
            .unwrap();
    assert_eq!(p.accepted, Some(true));
    assert!(p.value.unwrap().is_empty());
    let s =
        f.c.serialize_list(&f.h, f.list, &list_input(&[]), Default::default())
            .unwrap();
    assert_eq!(s.accepted, Some(true));
    assert_eq!(s.text.as_deref(), Some(""));
}

fn diagnostics(mode: usize) -> Vec<cem_ml::diagnostics::Diagnostic> {
    if mode == 6 {
        vec![cem_ml::diagnostics::Diagnostic {
            code: "root.operation".into(),
            message: "native note".into(),
            ..Default::default()
        }]
    } else {
        vec![]
    }
}
fn rule_identity() -> ImplementationIdentity {
    use cem_ml::schema::{
        datatype_validation::{ResultRepresentation, ValidationSignature},
        value_contracts::ContractName,
    };
    ImplementationIdentity::validator(
        "urn:test:validate",
        hash(b"module"),
        "validate",
        ValidationSignature {
            kind: DatatypeKind::Scalar,
            value: ValueRepresentation::Scalar(ScalarRepresentation::String),
            candidate: CandidateRequirement::Optional,
            result: ResultRepresentation::Accepted(ContractName::new(
                cem_ml::schema::registry::CEM_SCHEMA_URI,
                "datatype-validation-result",
            )),
        },
    )
}
#[derive(Debug)]
struct Gate(Arc<State>);
impl cem_ql::datatype_validation::NativeDatatypeValidator for Gate {
    fn validate(
        &self,
        c: cem_ql::datatype_validation::ValidationCall<'_>,
    ) -> cem_ql::datatype_validation::RuleExecution {
        assert_eq!(c.runtime.query.scope, cem_ql::eval::QueryContextScope(4));
        self.0.calls.lock().unwrap().push("rule:4".into());
        cem_ql::datatype_validation::RuleExecution::Complete(ItemStream::once(Item::Record(
            BTreeMap::from([
                (
                    "accepted".into(),
                    vec![Item::Atomic(AtomValue::Boolean(
                        c.value[0].atom() != Some(AtomValue::String("deny".into())),
                    ))],
                ),
                ("diagnostics".into(), vec![]),
            ]),
        )))
    }
}
fn rule_adapter() -> cem_ql::datatype_results::DatatypeResultAdapter {
    use cem_ml::schema::{
        declaration_references::SchemaDeclarationNode,
        value_contracts::{ContractName, ValueContractSource, ValueContracts},
    };
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
    cem_ql::datatype_results::DatatypeResultAdapter::new(
        Arc::new(ValueContracts::compile(&[s], Default::default()).unwrap()),
        ContractName::new(ns, "datatype-validation-result"),
        ContractName::new(ns, "datatype-diagnostic"),
    )
    .unwrap()
}
#[test]
fn assembly_root_operations_validate_in_original_rule_contexts_before_list_export() {
    let f = fixture_with_rule(true);
    let p =
        f.c.prepare_lexical(&f.h, f.scalar, &lexical("ok"), Default::default())
            .unwrap();
    assert_eq!(p.accepted, Some(true));
    assert_eq!(
        *f.state.calls.lock().unwrap(),
        vec!["prepare:2", "prepare:1", "rule:4"]
    );
    f.state.calls.lock().unwrap().clear();
    let c =
        f.c.convert(&f.h, f.scalar, &conversion("deny"), Default::default())
            .unwrap();
    assert_eq!(c.accepted, Some(false));
    assert!(c.value.is_some());
    assert_eq!(*f.state.calls.lock().unwrap(), vec!["convert:2", "rule:4"]);
    f.state.calls.lock().unwrap().clear();
    let s =
        f.c.serialize_list(
            &f.h,
            f.list,
            &list_input(&["ok", "deny"]),
            Default::default(),
        )
        .unwrap();
    assert_eq!(s.accepted, Some(false));
    assert!(s.text.is_none());
    assert_eq!(*f.state.calls.lock().unwrap(), vec!["rule:4", "rule:4"]);
}
#[test]
fn assembly_root_operations_preserve_bounded_completion_states_without_implicit_other_operations() {
    let f = fixture_ops();
    for mode in 1..=5 {
        f.state.mode.store(mode, Ordering::SeqCst);
        f.state.calls.lock().unwrap().clear();
        let p =
            f.c.prepare_lexical(&f.h, f.scalar, &lexical("x"), Default::default())
                .unwrap();
        assert!(f
            .state
            .calls
            .lock()
            .unwrap()
            .iter()
            .all(|c| c.starts_with("prepare:")));
        f.state.calls.lock().unwrap().clear();
        let c =
            f.c.convert(&f.h, f.scalar, &conversion("x"), Default::default())
                .unwrap();
        assert_eq!(*f.state.calls.lock().unwrap(), vec!["convert:2"]);
        f.state.calls.lock().unwrap().clear();
        let s =
            f.c.serialize_list(&f.h, f.list, &list_input(&["x"]), Default::default())
                .unwrap();
        assert_eq!(*f.state.calls.lock().unwrap(), vec!["serialize:3"]);
        assert_eq!(p.accepted, if mode == 2 { Some(false) } else { None });
        assert_eq!(c.accepted, if mode == 2 { Some(false) } else { None });
        if mode <= 4 {
            assert_eq!(s.accepted, if mode == 2 { Some(false) } else { None });
            assert!(s.text.is_none());
        }
        assert!(p.value.is_none());
        assert!(c.value.is_none());
    }
    f.state.mode.store(11, Ordering::SeqCst);
    let p =
        f.c.prepare_lexical(&f.h, f.scalar, &lexical("x"), Default::default())
            .unwrap();
    assert!(matches!(
        p.stopped,
        Some(PreparationStop::IncompatibleReplacement)
    ));
    assert!(p.value.is_none());
    f.state.calls.lock().unwrap().clear();
    f.state.mode.store(12, Ordering::SeqCst);
    let p =
        f.c.prepare_lexical(&f.h, f.list, &lexical("a b"), Default::default())
            .unwrap();
    assert!(matches!(
        p.stopped,
        Some(PreparationStop::IncompatibleReplacement)
    ));
    assert!(p.value.is_none());
    assert_eq!(*f.state.calls.lock().unwrap(), vec!["tokens", "tokens"]);
}
#[test]
fn assembly_root_operations_enforce_cumulative_item_and_query_budgets() {
    let f = fixture_ops();
    let p =
        f.c.prepare_lexical(
            &f.h,
            f.list,
            &lexical("a b"),
            PreparationLimits {
                max_preparations: 4,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(matches!(
        p.stopped,
        Some(PreparationStop::Limit("preparations"))
    ));
    assert!(p.value.is_none());
    assert_eq!(*f.state.calls.lock().unwrap(), vec!["tokens", "tokens"]);
    f.state.calls.lock().unwrap().clear();
    let c =
        f.c.convert(
            &f.h,
            f.scalar,
            &conversion("a"),
            ConversionLimits {
                max_lexical_bytes: 0,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(matches!(
        c.stopped,
        Some(ConversionStop::Limit("lexical-bytes"))
    ));
    assert!(f.state.calls.lock().unwrap().is_empty());
    let s =
        f.c.serialize_list(
            &f.h,
            f.list,
            &list_input(&["abc"]),
            SerializationLimits {
                max_output_bytes: 1,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(matches!(
        s.stopped,
        Some(SerializationStop::Limit("output-bytes"))
    ));
    assert!(s.text.is_none());
    f.state.calls.lock().unwrap().clear();
    f.state.mode.store(10, Ordering::SeqCst);
    let p =
        f.c.prepare_lexical(&f.h, f.scalar, &lexical("x"), Default::default())
            .unwrap();
    assert_eq!(p.accepted, None);
    assert!(matches!(p.stopped, Some(PreparationStop::Failed)));
    assert_eq!(
        *f.state.calls.lock().unwrap(),
        vec!["prepare:2", "prepare:1"]
    );
    f.state.mode.store(0, Ordering::SeqCst);
    assert_eq!(
        f.c.prepare_lexical(&f.h, f.scalar, &lexical("x"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
}
#[test]
fn assembly_root_operations_preserve_candidate_and_diagnostic_provenance() {
    let f = fixture_ops();
    f.state.mode.store(6, Ordering::SeqCst);
    let item = RetainedCemNode::new(f.g.module("app").unwrap().source().clone(), f.scalar)
        .unwrap()
        .query_item();
    let attribution = cem_ql::datatype_results::DiagnosticAttribution::from_node(&item);
    let mut p = lexical("x");
    p.candidate = vec![item.clone()];
    let p =
        f.c.prepare_lexical(&f.h, f.scalar, &p, Default::default())
            .unwrap();
    assert_eq!(p.accepted, Some(true));
    assert_eq!(p.diagnostics.len(), 2);
    assert!(p.diagnostics.iter().all(|d| d.uri == attribution.uri
        && d.node == attribution.node
        && d.source_map == attribution.source_map));
    let mut c = conversion("x");
    c.candidate = vec![item.clone()];
    let c = f.c.convert(&f.h, f.scalar, &c, Default::default()).unwrap();
    assert_eq!(c.diagnostics[0].node, attribution.node);
    let mut s = list_input(&["x"]);
    s.candidate = vec![item];
    let s =
        f.c.serialize_list(&f.h, f.list, &s, Default::default())
            .unwrap();
    assert_eq!(s.diagnostics[0].uri, attribution.uri);
    let p =
        f.c.prepare_lexical(
            &f.h,
            f.scalar,
            &lexical("x"),
            PreparationLimits {
                validation: cem_ql::datatype_validation::ValidationLimits {
                    max_diagnostics: 1,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
    assert!(matches!(
        p.stopped,
        Some(PreparationStop::Limit("diagnostics"))
    ));
}

#[test]
fn assembly_root_operations_refuse_stale_or_disposed_owners_hosts_and_snapshots() {
    for action in [
        "operation",
        "tokenizer",
        "rule",
        "drop",
        "epoch",
        "host",
        "snapshot",
    ] {
        let mut f = fixture_with_rule(true);
        match action {
            "operation" => {
                for b in f.bindings.values() {
                    if let DatatypeAssemblyOperationBinding::Operation(o) = b {
                        o.retire();
                        break;
                    }
                }
            }
            "tokenizer" => {
                for b in f.bindings.values() {
                    if let DatatypeAssemblyOperationBinding::Tokenizer(o) = b {
                        o.retire();
                        break;
                    }
                }
            }
            "rule" => f.rules.values().next().unwrap().retire(),
            "drop" => f.bindings.clear(),
            "epoch" => {
                assert!(f.h.set_context(f.root, Some(Default::default())));
            }
            "host" => f.h.close(),
            "snapshot" => f.c.retire(),
            _ => unreachable!(),
        }
        assert!(f
            .c
            .prepare_lexical(&f.h, f.scalar, &lexical("x"), Default::default())
            .is_err());
        assert!(f
            .c
            .convert(&f.h, f.scalar, &conversion("x"), Default::default())
            .is_err());
        assert!(f
            .c
            .serialize_list(&f.h, f.list, &list_input(&["x"]), Default::default())
            .is_err());
        assert!(f.state.calls.lock().unwrap().is_empty());
    }
    let f = fixture_ops();
    let other = fixture_ops();
    assert!(f
        .c
        .prepare_lexical(&other.h, f.scalar, &lexical("x"), Default::default())
        .is_err());
    assert!(f
        .c
        .convert(&other.h, f.scalar, &conversion("x"), Default::default())
        .is_err());
    assert!(f
        .c
        .serialize_list(&other.h, f.list, &list_input(&["x"]), Default::default())
        .is_err());
}
#[test]
fn assembly_root_operations_guard_reentry_across_validation_and_every_operation() {
    let f = fixture_ops();
    let h = Arc::new(f.h);
    let c = Arc::new(f.c);
    let scalar = f.scalar;
    let list = f.list;
    let hh = h.clone();
    let cc = c.clone();
    ACTION.with(|a| {
        *a.borrow_mut() = Some(Box::new(move || {
            assert!(cc
                .validate(&hh, scalar, &input("x"), Default::default())
                .is_err());
            assert!(cc
                .prepare_lexical(&hh, scalar, &lexical("x"), Default::default())
                .is_err());
            assert!(cc
                .convert(&hh, scalar, &conversion("x"), Default::default())
                .is_err());
            assert!(cc
                .serialize_list(&hh, list, &list_input(&["x"]), Default::default())
                .is_err());
        }))
    });
    f.state.mode.store(7, Ordering::SeqCst);
    let result = c.prepare_lexical(&h, scalar, &lexical("x"), Default::default());
    ACTION.with(|a| *a.borrow_mut() = None);
    assert_eq!(result.unwrap().accepted, Some(true));
    assert_eq!(
        *f.state.calls.lock().unwrap(),
        vec!["prepare:2", "prepare:1"]
    );
    assert_eq!(
        c.convert(
            &h,
            scalar,
            &conversion("x"),
            ConversionLimits {
                max_lexical_bytes: 0,
                ..Default::default()
            }
        )
        .unwrap()
        .accepted,
        None
    );
    // Every failure releases the same guard for a later ordinary call.
    f.state.mode.store(0, Ordering::SeqCst);
    assert_eq!(
        c.serialize_list(&h, list, &list_input(&["x"]), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
}
#[test]
fn assembly_root_operations_withhold_late_results_and_keep_callbacks_weak() {
    for operation in ["prepare", "convert", "serialize"] {
        for cancel in [false, true] {
            let f = fixture_ops();
            let owner = f
                .bindings
                .values()
                .find_map(|b| match b {
                    DatatypeAssemblyOperationBinding::Operation(o) => Some(o.clone()),
                    _ => None,
                })
                .unwrap();
            ACTION.with(|a| *a.borrow_mut() = Some(Box::new(move || owner.retire())));
            f.state
                .mode
                .store(if cancel { 9 } else { 8 }, Ordering::SeqCst);
            let failed = match operation {
                "prepare" => {
                    f.c.prepare_lexical(&f.h, f.scalar, &lexical("x"), Default::default())
                        .is_err()
                }
                "convert" => {
                    f.c.convert(&f.h, f.scalar, &conversion("x"), Default::default())
                        .is_err()
                }
                _ => {
                    f.c.serialize_list(&f.h, f.list, &list_input(&["x"]), Default::default())
                        .is_err()
                }
            };
            ACTION.with(|a| *a.borrow_mut() = None);
            assert!(failed, "late {operation} success escaped");
        }
    }
    let f = fixture_ops();
    let weak = Arc::downgrade(&f.state);
    drop(f.state);
    drop(f.h);
    assert!(
        weak.upgrade().is_none(),
        "retained snapshots and operation owners must retain no callbacks"
    );
    drop(f.c);
    drop(f.bindings);
}

#[path = "datatype_assembly_installation.rs"]
mod installation;
