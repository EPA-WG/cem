use cem_ml::{
    content_cache::ContentHash,
    events::cem::CemEventNormalizer,
    operation_control::OperationControl,
    parser::{AstNodeId, CemAstNode},
    schema::{
        datatype_contracts::ItemBounds,
        datatype_registry::{DatatypeKind, DatatypeKindSource},
        datatype_validation::{ScalarRepresentation, ValueRepresentation},
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
    datatype_compilation::{DatatypeImplementation, TokenizerBinding},
    datatype_module_capsule::{
        self as modules, ModuleSource, ModuleTarget, VerifiedDatatypeModules,
    },
    datatype_validation::ValidationInput,
    eval::{AtomValue, Item, ItemStream, RetainedCemNode},
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
fn limits() -> ReferenceTraversalLimits {
    ReferenceTraversalLimits {
        max_depth: 64,
        max_work: 100_000,
    }
}
fn verified(input: &BTreeMap<String, ModuleSource>) -> Arc<VerifiedDatatypeModules> {
    let b = modules::export(
        "app",
        input,
        &[],
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    Arc::new(modules::verify(&b.bytes, &b.pin, &[], &OperationControl::default()).unwrap())
}
fn implementations(g: &VerifiedDatatypeModules) -> Vec<DatatypeImplementation> {
    let mut sources = g
        .module_ids()
        .flat_map(|id| g.module(id).unwrap().sources().values())
        .collect::<Vec<_>>();
    for id in g.leaf_ids() {
        if let Some(s) = g.leaf_datatypes(id) {
            sources.extend(s.values());
        }
    }
    sources
        .into_iter()
        .filter_map(|source| {
            let DatatypeKindSource::Explicit(kind) = source.plan().kind else {
                return None;
            };
            Some(DatatypeImplementation {
                source: source.clone(),
                kind,
                representation: if kind == DatatypeKind::List {
                    ValueRepresentation::List(ScalarRepresentation::String)
                } else {
                    ValueRepresentation::Scalar(ScalarRepresentation::String)
                },
                accepted_bases: vec![],
                bounds: ItemBounds::default(),
                tokenizer: TokenizerBinding::Absent,
                validator: None,
            })
        })
        .collect()
}
fn register(
    h: &mut FreshStructuralHost,
    g: &VerifiedDatatypeModules,
) -> (DeclarationScope, BTreeMap<String, DeclarationScope>) {
    let mut scopes = BTreeMap::new();
    for id in g.module_ids().chain(g.leaf_ids()) {
        let s = h
            .register_datatype_module_scope(
                g,
                id,
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        scopes.insert(id.to_owned(), s);
    }
    let root = scopes.remove(&g.pin().root).unwrap();
    (root, scopes)
}
fn context(g: &VerifiedDatatypeModules, target: &str) -> StandaloneExpressionContext {
    let m = g.module(target).unwrap();
    StandaloneExpressionContext::default().with_binding(
        "selected",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(m.source().clone(), *m.roots().first().unwrap())
                .unwrap()
                .query_item(),
        )),
    )
}
fn prepared(
    g: &Arc<VerifiedDatatypeModules>,
    grant: bool,
) -> (
    FreshStructuralHost,
    DeclarationScope,
    BTreeMap<String, DeclarationScope>,
) {
    let mut h = FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    let (root, scopes) = register(&mut h, g);
    for (consumer, target) in [("app", "vendor"), ("vendor", "core")] {
        let c = context(g, target);
        h.prepare_datatype_module_lexical_scopes(g, consumer, |_, _, _| {
            (Some(c.clone()), ReferenceScopePolicyOverrides::default())
        })
        .unwrap();
        if grant {
            h.allow_scope_crossing(
                if consumer == "app" {
                    root
                } else {
                    scopes[consumer]
                },
                scopes[target],
            );
        }
    }
    (h, root, scopes)
}
fn input(n: usize) -> ValidationInput {
    ValidationInput {
        value: vec![Item::Atomic(AtomValue::String("same".into())); n],
        candidate: vec![],
        fallback: Default::default(),
    }
}
#[test]
fn actual_host_module_chain_selects_and_compiles_original_sources() {
    let g = verified(&chain());
    let (mut h, root, scopes) = prepared(&g, true);
    let selected = h
        .select_datatype_modules(g.clone(), root, &scopes, &[], limits())
        .unwrap();
    assert_eq!(selected.datatype_count(&h).unwrap(), 3);
    assert_eq!(selected.site_count(&h).unwrap(), 2);
    let compiled = h
        .compile_datatype_modules(
            g.clone(),
            root,
            &scopes,
            &[],
            &implementations(&g),
            &BTreeMap::new(),
            limits(),
        )
        .unwrap();
    let root_id = *g.module("app").unwrap().roots().first().unwrap();
    assert_eq!(
        compiled
            .validate(&h, root_id, &input(1), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    assert!(compiled
        .validate(&h, u32::MAX, &input(1), Default::default())
        .is_err());
    assert!(!g.selections_verified());
    assert!(!g.executable_closure_verified());
}
#[path = "datatype_module_runtime/admission.rs"]
mod admission;
#[path = "datatype_module_runtime/outcomes.rs"]
mod outcomes;
#[path = "datatype_module_runtime/owners.rs"]
mod owners;
