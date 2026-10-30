use cem_ml::{
    content_cache::ContentHash,
    events::cem::CemEventNormalizer,
    operation_control::OperationControl,
    parser::CemAstNode,
    schema::{
        machine::{CemSchemaMachine, LexicallyScopedDocument},
        structural_capsule::SourceIdentity,
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use cem_ql::{datatype_source_capsule as capsule, structural_receiver::FreshStructuralHost};
use std::{collections::BTreeSet, sync::Arc};
const TEXT: &str = r#"@ns schema = "https://cem.dev/ns/schema/1"
@default schema
{schema @name=example @namespace=urn:example |
 {types |
  {type @name=word @kind=scalar}
  {type @name=short @base=word}
  {type @name=words @kind=list @base=short @min-items=1 @max-items=3}
  {type @name=more @list-base=words @max-items=2}
 }}"#;
fn hash(b: &[u8]) -> String {
    ContentHash::from_blake3(b).header_value()
}
fn capture(text: &str) -> Arc<LexicallyScopedDocument> {
    Arc::new(
        CemSchemaMachine::new(
            CompiledSchema::cem_core(),
            CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
                SourceId(1),
                text.as_bytes().to_vec(),
            ))),
        )
        .build_with_lexical_scopes(),
    )
}
fn node(c: &LexicallyScopedDocument, name: &str) -> u32 {
    c.document().nodes.iter().find_map(|n|match n {CemAstNode::Element{node_id,expanded_name,attributes,..} if expanded_name.local_name=="type"&&attributes.iter().any(|id|matches!(c.document().get(*id),Some(CemAstNode::Attribute{expanded_name,value:Some(v),..}) if expanded_name.local_name=="name"&&v==name))=>Some(*node_id),_=>None}).unwrap()
}
fn built(text: &str) -> capsule::BuiltDatatypeSources {
    let c = capture(text);
    capsule::export(
        &c,
        SourceIdentity::new("file:///app/types.cem", SourceId(1), text.as_bytes()),
        &hash(b"registry"),
        BTreeSet::from([node(&c, "more")]),
        &OperationControl::default(),
    )
    .unwrap()
}
#[test]
fn literal_type_closure_reloads_original_owners_and_exact_dependency_occurrences() {
    let b = built(TEXT);
    let a = capsule::verify(&b.bytes, &b.pin, &OperationControl::default()).unwrap();
    let other = capsule::verify(&b.bytes, &b.pin, &OperationControl::default()).unwrap();
    assert_eq!(a.sources().len(), 4);
    assert_eq!(a.dependency_order().len(), 4);
    assert_eq!(
        a.source_named("words").unwrap().plan().dependencies.len(),
        1
    );
    assert!(!Arc::ptr_eq(
        a.source().ast_owner(),
        other.source().ast_owner()
    ));
    assert_eq!(a.source().source_uri(), "file:///app/types.cem");
    assert!(!a.executable_closure_verified());
    assert_eq!(b.bytes, built(TEXT).bytes);
}
#[test]
fn shared_compiler_validates_inherited_scalar_and_list_contracts_with_explicit_registration() {
    use cem_ml::schema::{
        datatype_contracts::ItemBounds,
        datatype_registry::DatatypeKind,
        datatype_validation::{ScalarRepresentation, ValueRepresentation},
        reference_policy::ReferenceScopePolicy,
    };
    use cem_ql::{
        datatype_compilation::{DatatypeImplementation, DatatypeImplementations, TokenizerBinding},
        datatype_validation::{ValidationInput, ValidationLimits},
        eval::{AtomValue, Item},
    };
    let b = built(TEXT);
    let sources =
        Arc::new(capsule::verify(&b.bytes, &b.pin, &OperationControl::default()).unwrap());
    let mut host =
        FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    let scope = host
        .register_scope(
            sources.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let mut implementations = DatatypeImplementations::default();
    for (name, kind) in [
        ("word", DatatypeKind::Scalar),
        ("words", DatatypeKind::List),
    ] {
        implementations
            .register(DatatypeImplementation {
                source: sources.source_named(name).unwrap().clone(),
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
            .unwrap();
    }
    let compiled = host
        .compile_datatype_sources(sources, scope, &implementations, &Default::default())
        .unwrap();
    let input = |count| ValidationInput {
        value: vec![Item::Atomic(AtomValue::String("Tea".into())); count],
        candidate: vec![],
        fallback: Default::default(),
    };
    assert_eq!(
        compiled
            .validate(&host, "more", &input(2), ValidationLimits::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    assert_eq!(
        compiled
            .validate(&host, "more", &input(3), ValidationLimits::default())
            .unwrap()
            .accepted,
        Some(false)
    );
    host.close();
    assert!(compiled
        .validate(&host, "more", &input(2), ValidationLimits::default())
        .is_err());
}

use cem_ml::schema::{
    datatype_contracts::ItemBounds,
    datatype_registry::DatatypeKind,
    datatype_validation::{ScalarRepresentation, ValueRepresentation},
    reference_policy::ReferenceScopePolicy,
};
use cem_ql::{
    datatype_compilation::{DatatypeImplementation, DatatypeImplementations, TokenizerBinding},
    datatype_validation::{ValidationInput, ValidationLimits},
    eval::{AtomValue, Item},
};
fn verified() -> Arc<capsule::VerifiedDatatypeSources> {
    let b = built(TEXT);
    Arc::new(capsule::verify(&b.bytes, &b.pin, &OperationControl::default()).unwrap())
}
fn implementations(sources: &capsule::VerifiedDatatypeSources) -> DatatypeImplementations {
    let mut implementations = DatatypeImplementations::default();
    for (name, kind) in [
        ("word", DatatypeKind::Scalar),
        ("words", DatatypeKind::List),
    ] {
        implementations
            .register(DatatypeImplementation {
                source: sources.source_named(name).unwrap().clone(),
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
            .unwrap();
    }
    implementations
}
fn input(count: usize) -> ValidationInput {
    ValidationInput {
        value: vec![Item::Atomic(AtomValue::String("Tea".into())); count],
        candidate: vec![],
        fallback: Default::default(),
    }
}
#[test]
fn missing_cyclic_qualified_duplicate_and_unsupported_source_dependencies_refuse() {
    for text in [
        TEXT.replace("@base=word", "@base=missing"),
        TEXT.replace("@name=word @kind=scalar", "@name=word @base=more"),
        TEXT.replace("@name=short", "@name=word"),
        TEXT.replace("@base=word", "@base=p:word"),
        TEXT.replace("@name=word @kind=scalar", "@name=word @kind=node"),
        TEXT.replace("@name=word @kind=scalar", "@name=word @kind=grammar"),
        TEXT.replace("@base=word", "@base={#other}"),
        TEXT.replace(
            "{type @name=word @kind=scalar}",
            "{type @name=word @kind=scalar | {constant @value=Tea}}",
        ),
    ] {
        let c = capture(&text);
        assert!(
            capsule::export(
                &c,
                SourceIdentity::new("file:///app/types.cem", SourceId(1), text.as_bytes()),
                &hash(b"registry"),
                BTreeSet::from([node(&c, "more")]),
                &OperationControl::default()
            )
            .is_err(),
            "{text}"
        );
    }
    let c = capture(TEXT);
    for roots in [BTreeSet::new(), BTreeSet::from([u32::MAX])] {
        assert!(capsule::export(
            &c,
            SourceIdentity::new("file:///app/types.cem", SourceId(1), TEXT.as_bytes()),
            &hash(b"registry"),
            roots,
            &OperationControl::default()
        )
        .is_err());
    }
}
#[test]
fn cancellation_and_pin_mismatch_refuse_without_changing_an_existing_capture() {
    let b = built(TEXT);
    let live = capsule::verify(&b.bytes, &b.pin, &OperationControl::default()).unwrap();
    let control = OperationControl::default();
    control.cancel_root(None, None).unwrap();
    assert!(capsule::verify(&b.bytes, &b.pin, &control).is_err());
    let mut pin = b.pin.clone();
    pin.registry_hash = hash(b"foreign registry");
    assert!(capsule::verify(&b.bytes, &pin, &OperationControl::default()).is_err());
    pin = b.pin.clone();
    pin.source.uri = "file:///other/types.cem".into();
    assert!(capsule::verify(&b.bytes, &pin, &OperationControl::default()).is_err());
    assert_eq!(live.sources().len(), 4);
    assert!(cem_ql::application_declaration::admit(&b.bytes, "{}", &hash(b"{}")).is_err());
}
#[test]
fn compilation_requires_exact_live_scope_implementations_and_original_policy() {
    for case in 0..6 {
        let sources = verified();
        let registry_hash = hash(if case == 5 {
            b"foreign registry"
        } else {
            b"registry"
        });
        let mut host =
            FreshStructuralHost::new(&registry_hash, OperationControl::default()).unwrap();
        let mut policy = ReferenceScopePolicy::schema_defaults().unwrap();
        if case == 4 {
            policy.limits.max_work = 0;
        }
        let scope = host
            .register_scope(
                if case == 1 {
                    verified().source().clone()
                } else {
                    sources.source().clone()
                },
                if case == 2 {
                    None
                } else {
                    Some(Default::default())
                },
                policy,
            )
            .unwrap();
        let mut foreign =
            FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
        let other = foreign
            .register_scope(
                sources.source().clone(),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        let implementations = if case == 3 {
            DatatypeImplementations::default()
        } else {
            implementations(&sources)
        };
        assert!(
            host.compile_datatype_sources(
                sources,
                if case == 0 { other } else { scope },
                &implementations,
                &Default::default()
            )
            .is_err(),
            "case {case}"
        );
    }
}
#[test]
fn failed_compilation_preserves_previous_results_and_roots_are_not_implicit_exports() {
    let sources = verified();
    let mut host =
        FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    let scope = host
        .register_scope(
            sources.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let compiled = host
        .compile_datatype_sources(
            sources.clone(),
            scope,
            &implementations(&sources),
            &Default::default(),
        )
        .unwrap();
    assert!(host
        .compile_datatype_sources(
            sources,
            scope,
            &DatatypeImplementations::default(),
            &Default::default()
        )
        .is_err());
    let report = compiled
        .validate(&host, "more", &input(3), ValidationLimits::default())
        .unwrap();
    assert_eq!(report.accepted, Some(false));
    assert!(!report.cardinality.is_empty());
    assert!(compiled
        .validate(&host, "word", &input(1), ValidationLimits::default())
        .is_err());
    assert!(
        compiled
            .validate(&host, "more", &input(2), ValidationLimits::default())
            .unwrap()
            .accepted
            == Some(true)
    );
    assert!(host.set_context(scope, Some(Default::default())));
    assert!(compiled
        .validate(&host, "more", &input(2), ValidationLimits::default())
        .is_err());
}
#[derive(Debug)]
struct NoLexicalExecution;
impl cem_ql::datatype_preparation::NativeLexicalPreparer for NoLexicalExecution {
    fn prepare(
        &self,
        _: cem_ql::datatype_preparation::PreparationCall<'_>,
    ) -> cem_ql::datatype_preparation::PreparationExecution {
        panic!("source compile and typed validation must not prepare lexical input")
    }
}
fn native_anchor(
    original: &capsule::VerifiedDatatypeSources,
) -> Arc<cem_ql::native_registrations::VerifiedNativeRequirement> {
    use cem_ml::schema::datatype_validation::CandidateRequirement;
    use cem_ql::{
        datatype_preparation::PreparationSignature,
        native_registrations::{
            self as native, ImplementationIdentity, OperationMode, PreparationMode,
        },
    };
    let built = native::export_operation(
        original.capture(),
        original.source_named("word").unwrap(),
        OperationMode::Preparation(PreparationMode::Ready),
        original.pin().source.clone(),
        ImplementationIdentity::preparation(
            "prepare",
            hash(b"module"),
            "prepare",
            PreparationSignature {
                kind: DatatypeKind::Scalar,
                output: ValueRepresentation::Scalar(ScalarRepresentation::String),
                candidate: CandidateRequirement::Optional,
            },
        ),
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    Arc::new(native::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap())
}
#[test]
fn exact_native_source_sharing_allows_explicit_original_operation_installation() {
    let original = verified();
    let native = native_anchor(&original);
    let sources = Arc::new(
        original
            .share_native_source(&native, &OperationControl::default())
            .unwrap(),
    );
    assert!(Arc::ptr_eq(sources.source(), native.source()));
    assert!(!Arc::ptr_eq(sources.source(), original.source()));
    let mut host =
        FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    host.register_native_preparer(native.implementation().clone(), NoLexicalExecution)
        .unwrap();
    let scope = host
        .register_scope(
            sources.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let operation = host.link_native_operation(native, scope).unwrap();
    let mut implementations = implementations(&sources);
    assert!(operation
        .install(original.source_named("word").unwrap(), &mut implementations)
        .is_err());
    operation
        .install(sources.source_named("word").unwrap(), &mut implementations)
        .unwrap();
    let compiled = host
        .compile_datatype_sources(sources, scope, &implementations, &Default::default())
        .unwrap();
    assert_eq!(
        compiled
            .validate(&host, "more", &input(2), ValidationLimits::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    compiled.retire();
    assert!(compiled
        .validate(&host, "more", &input(2), ValidationLimits::default())
        .is_err());
}

#[test]
fn whole_snapshot_agreement_is_required_even_with_the_same_claimed_source_identity() {
    let original = verified();
    let native = native_anchor(&original);
    let text = TEXT.replace("@max-items=2", "@max-items=1");
    let changed = capture(&text);
    let b = capsule::export(
        &changed,
        original.pin().source.clone(),
        &hash(b"registry"),
        BTreeSet::from([node(&changed, "more")]),
        &OperationControl::default(),
    )
    .unwrap();
    let changed = capsule::verify(&b.bytes, &b.pin, &OperationControl::default()).unwrap();
    assert!(changed
        .share_native_source(&native, &OperationControl::default())
        .is_err());
    assert!(original
        .share_native_source(&native, &OperationControl::default())
        .is_ok());
}
#[test]
fn validation_limits_and_cancellation_preserve_stop_semantics() {
    let sources = verified();
    let control = OperationControl::default();
    let mut host = FreshStructuralHost::new(&hash(b"registry"), control.clone()).unwrap();
    let scope = host
        .register_scope(
            sources.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let compiled = host
        .compile_datatype_sources(
            sources.clone(),
            scope,
            &implementations(&sources),
            &Default::default(),
        )
        .unwrap();
    let report = compiled
        .validate(
            &host,
            "more",
            &input(2),
            ValidationLimits {
                max_input_values: 1,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(report.accepted, None);
    assert!(matches!(
        report.stopped.unwrap().reason,
        cem_ql::datatype_validation::ValidationStopReason::Limit(_)
    ));
    control.cancel_root(None, None).unwrap();
    assert!(compiled
        .validate(&host, "more", &input(2), ValidationLimits::default())
        .is_err());
    assert!(host
        .compile_datatype_sources(
            sources.clone(),
            scope,
            &implementations(&sources),
            &Default::default()
        )
        .is_err());
}
