use cem_ml::{
    content_cache::ContentHash,
    events::cem::CemEventNormalizer,
    operation_control::OperationControl,
    parser::{AstNodeId, CemAstNode},
    schema::{
        datatype_registry::DatatypeRegistry,
        datatype_validation::ScalarRepresentation,
        declaration_references::SchemaDeclarationNode,
        machine::{CemSchemaMachine, LexicallyScopedDocument},
        reference_policy::{ReferenceScopePolicy, ReferenceScopePolicyOverrides},
        reference_traversal::ReferenceTraversalLimits,
        structural_capsule::SourceIdentity,
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    datatype_module_capsule::{v2 as assembly, ModuleSource, ModuleTarget},
    eval::{ItemStream, RetainedCemNode},
    native_registrations::{
        dependency_sources, profiles::v2 as profiles, profiles::OperationRole,
        ImplementationIdentity, OperationMode,
    },
    schema_references::DeclarationScope,
    structural_receiver::{DatatypeDependencyImport, FreshStructuralHost},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[path = "support/datatype_module_sources.rs"]
mod module_sources;
use module_sources::*;
#[path = "support/datatype_assembly_sources.rs"]
mod assembly_sources;
use assembly_sources::*;
fn limits() -> ReferenceTraversalLimits {
    ReferenceTraversalLimits {
        max_depth: 64,
        max_work: 100_000,
    }
}
fn graph(
    m: &BTreeMap<String, assembly::ModuleSource>,
    p: &[assembly::ProfileArtifact],
) -> Arc<assembly::VerifiedDatatypeModules> {
    let c = OperationControl::default();
    let b = assembly::export("app", m, &[], p, &hash(b"registry"), &c).unwrap();
    Arc::new(assembly::verify(&b.bytes, &b.pin, &[], p, &c).unwrap())
}
fn verified() -> Arc<assembly::VerifiedDatatypeModules> {
    let (m, p) = fixture();
    graph(&m, &p)
}
fn register(
    h: &mut FreshStructuralHost,
    g: &assembly::VerifiedDatatypeModules,
) -> (DeclarationScope, BTreeMap<String, DeclarationScope>) {
    let mut scopes = BTreeMap::new();
    for id in g.module_ids().chain(g.leaf_ids()) {
        let s = h
            .register_datatype_assembly_scope(
                g,
                id,
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        scopes.insert(id.into(), s);
    }
    let root = scopes.remove(&g.pin().root).unwrap();
    (root, scopes)
}
fn value(
    g: &assembly::VerifiedDatatypeModules,
    id: &str,
    nodes: &[u32],
) -> StandaloneExpressionBinding {
    let m = g.module(id).unwrap();
    StandaloneExpressionBinding::any(ItemStream::from_items(
        nodes
            .iter()
            .map(|n| {
                RetainedCemNode::new(m.source().clone(), *n)
                    .unwrap()
                    .query_item()
            })
            .collect(),
    ))
}
fn context(g: &assembly::VerifiedDatatypeModules) -> StandaloneExpressionContext {
    let palette = g.module("palette").unwrap();
    let blue = node(palette.capture(), "constant", 0);
    let red = node(palette.capture(), "constant", 1);
    StandaloneExpressionContext::default()
        .with_binding(
            "base",
            value(
                g,
                "core",
                &[*g.module("core").unwrap().roots().first().unwrap()],
            ),
        )
        .with_binding("colors", value(g, "palette", &[blue, red, blue]))
}
fn prepared(
    g: &assembly::VerifiedDatatypeModules,
    grant: bool,
    ready: bool,
) -> (
    FreshStructuralHost,
    DeclarationScope,
    BTreeMap<String, DeclarationScope>,
) {
    prepared_with_context(g, grant, ready, context(g))
}
fn prepared_with_context(
    g: &assembly::VerifiedDatatypeModules,
    grant: bool,
    ready: bool,
    ctx: StandaloneExpressionContext,
) -> (
    FreshStructuralHost,
    DeclarationScope,
    BTreeMap<String, DeclarationScope>,
) {
    let mut h = FreshStructuralHost::new(&hash(b"registry"), Default::default()).unwrap();
    let (root, scopes) = register(&mut h, g);
    h.prepare_datatype_assembly_lexical_scopes(g, "app", |source, _, _| {
        let pending = matches!(source.node(), CemAstNode::Reference { expression, .. }
            if expression == "#colors")
            && !ready;
        (
            if pending { None } else { Some(ctx.clone()) },
            ReferenceScopePolicyOverrides::default(),
        )
    })
    .unwrap();
    h.allow_scope_crossing(root, scopes["core"]);
    if grant {
        h.allow_scope_crossing(root, scopes["palette"]);
    }
    (h, root, scopes)
}
#[test]
fn actual_host_assembly_selects_original_datatypes_constants_and_duplicates_without_compilation() {
    let g = verified();
    let (mut h, root, scopes) = prepared(&g, true, true);
    let selected = h
        .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
        .unwrap();
    assert_eq!(selected.datatype_count(&h).unwrap(), 2);
    assert_eq!(selected.site_count(&h).unwrap(), 1);
    assert_eq!(selected.constant_site_count(&h).unwrap(), 2);
    assert_eq!(selected.constant_target_count(&h).unwrap(), 4);
    assert!(!g.selections_verified());
    assert!(!g.executable_closure_verified());
    let renewed = h
        .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
        .unwrap();
    assert_eq!(renewed.constant_target_count(&h).unwrap(), 4);
    assert!(selected.is_live(&h));
    selected.retire();
    assert!(!selected.is_live(&h));
    assert!(selected.constant_site_count(&h).is_err());
    assert!(renewed.is_live(&h));
}
#[test]
fn actual_host_assembly_needs_explicit_contexts_occurrences_and_constant_crossing_grants() {
    let g = verified();
    for (grant, ready) in [(false, true), (true, false)] {
        let (mut h, root, scopes) = prepared(&g, grant, ready);
        assert!(h
            .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
            .is_err());
    }
    let mut h = FreshStructuralHost::new(&hash(b"registry"), Default::default()).unwrap();
    let (root, scopes) = register(&mut h, &g);
    h.allow_scope_crossing(root, scopes["core"]);
    h.allow_scope_crossing(root, scopes["palette"]);
    assert!(h
        .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
        .is_err());
}
#[test]
fn actual_host_assembly_rejects_wrong_constant_order_and_original_owner_copies() {
    let g = verified();
    let pal = g.module("palette").unwrap();
    let blue = node(pal.capture(), "constant", 0);
    let red = node(pal.capture(), "constant", 1);
    let wrong = StandaloneExpressionContext::default()
        .with_binding("base", context(&g).bindings["base"].clone())
        .with_binding("colors", value(&g, "palette", &[red, blue, blue]));
    let (mut h, root, scopes) = prepared_with_context(&g, true, true, wrong);
    assert!(h
        .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
        .is_err());
    let copy = verified();
    let wrong = context(&copy).with_binding("base", context(&g).bindings["base"].clone());
    let (mut h, root, scopes) = prepared_with_context(&g, true, true, wrong);
    assert!(h
        .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
        .is_err());
}

fn replace_app(
    m: &mut BTreeMap<String, assembly::ModuleSource>,
    text: &str,
) -> Vec<assembly::ProfileArtifact> {
    let old = m.remove("app").unwrap();
    let mut app = wrapped(&old.types.identity.uri, text, Some("choice"));
    app.imports = old.imports;
    app.types.bindings.insert(
        field(&app.types.capture, "choice", "base"),
        old.types.bindings.into_values().next().unwrap(),
    );
    let reference = app
        .types
        .capture
        .occurrences()
        .find(|id| {
            matches!(app.types.capture.document().get(*id),
            Some(CemAstNode::Reference { expression, .. }) if expression != "#base")
        })
        .unwrap();
    app.constants
        .insert(reference, old.constants.into_values().next().unwrap());
    let p = pack(&app, "choice");
    app.profiles.insert(
        datatype(&app.types.capture, "choice"),
        assembly::ProfileSelection {
            pin: p.pin.clone(),
            id: "constant".into(),
        },
    );
    m.insert("app".into(), app);
    vec![p]
}
#[test]
fn actual_host_assembly_literal_imports_are_explicit_and_never_supply_crossing_permission() {
    let (mut m, _) = fixture();
    let p=replace_app(&mut m,"@ns s = https://cem.dev/ns/schema/1\n@ns k = urn:core\n@default s\n{schema @namespace=urn:app | {types | {type @name=choice @base=k:scalar | {#colors} {constant @value=local}}}}");
    let g = graph(&m, &p);
    let (mut h, root, scopes) = prepared(&g, true, true);
    assert!(h
        .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
        .is_err());
    let imports = [DatatypeDependencyImport {
        consumer: root,
        module: "core".into(),
        root: *g.module("core").unwrap().roots().first().unwrap(),
    }];
    assert!(h
        .select_datatype_assembly(g.clone(), root, &scopes, &imports, limits())
        .is_ok());
    let mut h = FreshStructuralHost::new(&hash(b"registry"), Default::default()).unwrap();
    let (root, scopes) = register(&mut h, &g);
    let ctx = context(&g);
    h.prepare_datatype_assembly_lexical_scopes(&g, "app", |_, _, _| {
        (Some(ctx.clone()), Default::default())
    })
    .unwrap();
    h.allow_scope_crossing(root, scopes["palette"]);
    let imports = [DatatypeDependencyImport {
        consumer: root,
        module: "core".into(),
        root: *g.module("core").unwrap().roots().first().unwrap(),
    }];
    assert!(h
        .select_datatype_assembly(g.clone(), root, &scopes, &imports, limits())
        .is_err());
}
#[test]
fn actual_host_assembly_checks_raw_foreign_extra_scopes_and_ready_constant_library_contexts() {
    let g = verified();
    let (mut h, root, scopes) = prepared(&g, true, true);
    let mut extra = scopes.clone();
    extra.insert("extra".into(), root);
    assert!(h
        .select_datatype_assembly(g.clone(), root, &extra, &[], limits())
        .is_err());
    let copy = verified();
    let (mut foreign, r, s) = prepared(&copy, true, true);
    assert!(foreign
        .select_datatype_assembly(g.clone(), r, &s, &[], limits())
        .is_err());
    let mut h = FreshStructuralHost::new(&hash(b"registry"), Default::default()).unwrap();
    let mut raw = BTreeMap::new();
    for id in g.module_ids() {
        let m = g.module(id).unwrap();
        raw.insert(
            id.to_owned(),
            h.register_scope(
                m.source().clone(),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap(),
        );
    }
    let r = raw.remove("app").unwrap();
    assert!(h
        .select_datatype_assembly(g.clone(), r, &raw, &[], limits())
        .is_err());
    let (mut h, r, s) = prepared(&g, true, true);
    assert!(h.set_context(s["palette"], None));
    assert!(h
        .select_datatype_assembly(g.clone(), r, &s, &[], limits())
        .is_err());
}
#[test]
fn actual_host_assembly_inspections_expire_with_context_changes_closure_and_fixed_limits() {
    let g = verified();
    let (mut h, root, scopes) = prepared(&g, true, true);
    for bounds in [(0, 100), (65, 100), (64, 0), (64, 1), (64, 100001)] {
        let limits = ReferenceTraversalLimits {
            max_depth: bounds.0,
            max_work: bounds.1,
        };
        assert!(h
            .select_datatype_assembly(g.clone(), root, &scopes, &[], limits)
            .is_err());
    }
    let selected = h
        .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
        .unwrap();
    assert!(h.set_context(scopes["palette"], Some(Default::default())));
    assert!(!selected.is_live(&h));
    let renewed = h
        .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
        .unwrap();
    h.close();
    assert!(!renewed.is_live(&h));
    assert!(h
        .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
        .is_err());
}
#[test]
fn actual_host_assembly_transitive_constants_need_per_hop_contexts_and_exact_intermediates() {
    let (mut m, p) = fixture();
    let mut bank=wrapped("file:///app/bank.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:bank | {constants | {constant @value=blue} {constant @value=red}}}",None);
    let blue = node(&bank.types.capture, "constant", 0);
    let red = node(&bank.types.capture, "constant", 1);
    bank.constant_exports = BTreeSet::from([blue, red]);
    let mut palette=wrapped("file:///app/palette.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:palette | {constants | {#more} {constant @value=unused0} {constant @value=unused1}}}",None);
    let r = reference(&palette.types.capture, "#more");
    palette.constant_exports.insert(r);
    palette.imports.insert("bank".into());
    palette.constants.insert(
        r,
        [blue, red, blue]
            .into_iter()
            .map(|node| assembly::ModuleNodeId {
                module: "bank".into(),
                node,
            })
            .collect(),
    );
    *m.get_mut("app")
        .unwrap()
        .constants
        .values_mut()
        .next()
        .unwrap() = vec![assembly::ModuleNodeId {
        module: "palette".into(),
        node: r,
    }];
    m.insert("palette".into(), palette);
    m.insert("bank".into(), bank);
    let g = graph(&m, &p);
    let more = StandaloneExpressionContext::default()
        .with_binding("more", value(&g, "bank", &[blue, red, blue]));
    let direct = context(&g).with_binding("colors", value(&g, "bank", &[blue, red, blue]));
    let (mut h, root, scopes) = prepared_with_context(&g, true, true, direct);
    h.allow_scope_crossing(root, scopes["bank"]);
    h.allow_scope_crossing(scopes["palette"], scopes["bank"]);
    h.prepare_datatype_assembly_lexical_scopes(&g, "palette", |_, _, _| {
        (Some(more.clone()), Default::default())
    })
    .unwrap();
    assert!(h
        .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
        .is_err());
    let app = context(&g).with_binding("colors", value(&g, "palette", &[r]));
    let (mut h, root, scopes) = prepared_with_context(&g, true, true, app);
    h.prepare_datatype_assembly_lexical_scopes(&g, "palette", |_, _, _| {
        (Some(more.clone()), Default::default())
    })
    .unwrap();
    h.allow_scope_crossing(root, scopes["bank"]);
    // Root-to-bank permission does not authorize the palette-to-bank edge.
    assert!(h
        .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
        .is_err());
    h.allow_scope_crossing(scopes["palette"], scopes["bank"]);
    assert_eq!(
        h.select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
            .unwrap()
            .constant_target_count(&h)
            .unwrap(),
        4
    );
}

#[derive(Debug)]
struct Pick {
    choice: Arc<std::sync::atomic::AtomicUsize>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    calls: Arc<std::sync::atomic::AtomicUsize>,
    _marker: Arc<()>,
}
impl cem_ql::native::NativeQueryFunction for Pick {
    fn call(&self, r: cem_ql::native::NativeQueryRequest<'_>) -> ItemStream {
        use std::sync::atomic::Ordering;
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.stop.load(Ordering::SeqCst) {
            r.control.cancel_root(None, None).unwrap();
        }
        r.arguments[self.choice.load(Ordering::SeqCst)].clone()
    }
}
#[test]
fn actual_host_assembly_renews_mutable_selectors_checks_late_stops_and_releases_callbacks() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    let (mut m, _) = fixture();
    let p=replace_app(&mut m,"@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=choice @base={#base} | {#native:call(\"fixture.pick\", good, wrong)} {constant @value=local}}}}");
    let g = graph(&m, &p);
    for stop_after in [false, true] {
        let choice = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let calls = Arc::new(AtomicUsize::new(0));
        let marker = Arc::new(());
        let weak = Arc::downgrade(&marker);
        let pal = g.module("palette").unwrap();
        let red = node(pal.capture(), "constant", 1);
        let mut ctx = context(&g)
            .with_binding("good", context(&g).bindings["colors"].clone())
            .with_binding("wrong", value(&g, "palette", &[red, red, red]));
        ctx.native_functions
            .register(
                "fixture.pick",
                2,
                Pick {
                    choice: choice.clone(),
                    stop: stop.clone(),
                    calls: calls.clone(),
                    _marker: marker,
                },
            )
            .unwrap();
        let (mut h, root, scopes) = prepared_with_context(&g, true, true, ctx);
        let selected = h
            .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        if stop_after {
            stop.store(true, Ordering::SeqCst);
        } else {
            choice.store(1, Ordering::SeqCst);
        }
        assert!(h
            .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
            .is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(selected.is_live(&h), !stop_after);
        if !stop_after {
            choice.store(0, Ordering::SeqCst);
            assert!(h
                .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
                .is_ok());
        }
        drop(h);
        assert!(
            weak.upgrade().is_none(),
            "inspection must retain no selector callback/context"
        );
    }
}
#[test]
fn actual_host_assembly_unused_constant_exports_need_no_prepared_occurrences() {
    let (mut m, p) = fixture();
    let mut palette=wrapped("file:///app/palette.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:palette | {constants | {#unused} {constant @value=blue} {constant @value=red}}}",None);
    let r = reference(&palette.types.capture, "#unused");
    let blue = node(&palette.types.capture, "constant", 0);
    let red = node(&palette.types.capture, "constant", 1);
    palette.constant_exports = BTreeSet::from([blue, red, r]);
    palette.constants.insert(
        r,
        vec![assembly::ModuleNodeId {
            module: "palette".into(),
            node: blue,
        }],
    );
    *m.get_mut("app")
        .unwrap()
        .constants
        .values_mut()
        .next()
        .unwrap() = [blue, red, blue]
        .into_iter()
        .map(|node| assembly::ModuleNodeId {
            module: "palette".into(),
            node,
        })
        .collect();
    m.insert("palette".into(), palette);
    let g = graph(&m, &p);
    let (mut h, root, scopes) = prepared(&g, true, true);
    assert_eq!(
        h.select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
            .unwrap()
            .constant_target_count(&h)
            .unwrap(),
        4
    );
}
#[test]
fn actual_host_assembly_reuses_transitive_datatype_resolver_without_constant_work() {
    let mut m = BTreeMap::new();
    for (id, types) in chain() {
        let imports = types
            .bindings
            .values()
            .filter_map(|target| match target {
                ModuleTarget::ModuleDatatype { module, .. } => Some(module.clone()),
                _ => None,
            })
            .collect();
        let mut module = assembly::ModuleSource::new(types);
        module.imports = imports;
        m.insert(id, module);
    }
    let g = graph(&m, &[]);
    let mut h = FreshStructuralHost::new(&hash(b"registry"), Default::default()).unwrap();
    let (root, scopes) = register(&mut h, &g);
    for (from, to) in [("app", "vendor"), ("vendor", "core")] {
        let ctx = StandaloneExpressionContext::default().with_binding(
            "selected",
            value(&g, to, &[*g.module(to).unwrap().roots().first().unwrap()]),
        );
        h.prepare_datatype_assembly_lexical_scopes(&g, from, |_, _, _| {
            (Some(ctx.clone()), Default::default())
        })
        .unwrap();
        h.allow_scope_crossing(if from == "app" { root } else { scopes[from] }, scopes[to]);
    }
    let selected = h
        .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
        .unwrap();
    assert_eq!(selected.datatype_count(&h).unwrap(), 3);
    assert_eq!(selected.constant_target_count(&h).unwrap(), 0);
}

#[test]
fn actual_host_assembly_destination_policies_retain_their_own_work_bounds() {
    let g = verified();
    let mut h = FreshStructuralHost::new(&hash(b"registry"), Default::default()).unwrap();
    let mut scopes = BTreeMap::new();
    for id in g.module_ids() {
        let mut policy = ReferenceScopePolicy::schema_defaults().unwrap();
        if id == "palette" {
            policy.limits.max_work = 1;
        }
        scopes.insert(
            id.to_owned(),
            h.register_datatype_assembly_scope(&g, id, Some(Default::default()), policy)
                .unwrap(),
        );
    }
    let root = scopes.remove("app").unwrap();
    let ctx = context(&g);
    h.prepare_datatype_assembly_lexical_scopes(&g, "app", |_, _, _| {
        (Some(ctx.clone()), Default::default())
    })
    .unwrap();
    h.allow_scope_crossing(root, scopes["core"]);
    h.allow_scope_crossing(root, scopes["palette"]);
    assert!(h
        .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
        .is_err());
}

#[derive(Debug)]
struct NoConstants;
impl cem_ql::datatype_enumeration::NativeConstantInterpreter for NoConstants {
    fn interpret(
        &self,
        _: cem_ql::datatype_enumeration::ConstantCall<'_>,
    ) -> cem_ql::datatype_enumeration::ConstantExecution {
        panic!("linking and source selection must not interpret constants")
    }
}
#[test]
fn actual_host_assembly_links_mapped_cemos1_owners_without_authorizing_compilation() {
    use cem_ql::{
        datatype_compilation::DatatypeImplementations,
        native_registrations::{profiles::OperationRole, NativeLinkError},
    };
    let g = verified();
    let module = g.module("app").unwrap();
    let profile = module.profiles().values().next().unwrap();
    let r = profile.requirements()[&OperationRole::Constant].clone();
    assert!(Arc::ptr_eq(r.source(), module.source()));
    let (mut h, root, scopes) = prepared(&g, true, true);
    h.register_native_constant_interpreter(r.implementation().clone(), NoConstants)
        .unwrap();
    let owner = h.link_dependency_operation(r.clone(), root).unwrap();
    assert!(owner.is_live_on(&h));
    let foreign = verified();
    let foreign_profile = foreign
        .module("app")
        .unwrap()
        .profiles()
        .values()
        .next()
        .unwrap();
    let foreign_r = foreign_profile.requirements()[&OperationRole::Constant].clone();
    assert!(matches!(
        h.link_dependency_operation(foreign_r, root),
        Err(NativeLinkError::InvalidSource)
    ));
    owner
        .install(profile.source(), &mut DatatypeImplementations::default())
        .unwrap();
    assert!(h
        .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
        .is_ok());
    owner.retire();
    assert!(h
        .select_datatype_assembly(g.clone(), root, &scopes, &[], limits())
        .is_ok());
    assert!(!g.selections_verified());
    assert!(!g.executable_closure_verified());
}
