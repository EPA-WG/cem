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
use cem_ql::{datatype_dependency_capsule as capsule, datatype_source_capsule as dt};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
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

fn identity(text: &str, uri: &str) -> SourceIdentity {
    SourceIdentity::new(uri, SourceId(1), text.as_bytes())
}
fn declaration(c: &LexicallyScopedDocument, name: &str) -> u32 {
    c.document().nodes.iter().find_map(|n|match n {CemAstNode::Element{node_id,expanded_name,attributes,..} if expanded_name.local_name=="type" && attributes.iter().any(|id|matches!(c.document().get(*id),Some(CemAstNode::Attribute{expanded_name,value:Some(v),..}) if expanded_name.local_name=="name" && v==name))=>Some(*node_id),_=>None}).unwrap()
}
fn field(c: &LexicallyScopedDocument, type_name: &str, name: &str) -> u32 {
    let CemAstNode::Element { attributes, .. } =
        c.document().get(declaration(c, type_name)).unwrap()
    else {
        unreachable!()
    };
    *attributes.iter().find(|id|matches!(c.document().get(**id),Some(CemAstNode::Attribute{expanded_name,..}) if expanded_name.local_name==name)).unwrap()
}
const SOURCE:&str = "@ns s = https://cem.dev/ns/schema/1\n@ns v = urn:vendor\n@default s\n{schema @namespace=urn:app | {types | {type @name=local @kind=scalar} {type @name=imported @base=v:base} {type @name=words @kind=list @base={#selected}} {type @name=root @list-base=words}}}";
const VENDOR:&str = "@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:vendor | {types | {type @name=base @kind=scalar}}}";
fn fixture() -> (
    Arc<LexicallyScopedDocument>,
    BTreeMap<u32, capsule::DependencyTarget>,
    Vec<capsule::DependencyArtifact>,
) {
    let c = capture(SOURCE);
    let vendor = capture(VENDOR);
    let root = declaration(&vendor, "base");
    let b = dt::export(
        &vendor,
        identity(VENDOR, "file:///app/vendor.cem"),
        &hash(b"registry"),
        BTreeSet::from([root]),
        &OperationControl::default(),
    )
    .unwrap();
    let pin = b.pin.clone();
    let bindings = BTreeMap::from([
        (
            field(&c, "imported", "base"),
            capsule::DependencyTarget::Datatype {
                pin: pin.clone(),
                root,
            },
        ),
        (
            field(&c, "words", "base"),
            capsule::DependencyTarget::LocalDatatype(declaration(&c, "imported")),
        ),
        (
            field(&c, "root", "list-base"),
            capsule::DependencyTarget::LocalDatatype(declaration(&c, "words")),
        ),
    ]);
    (c, bindings, vec![capsule::DependencyArtifact::datatype(b)])
}
fn build() -> capsule::BuiltDatatypeDependencies {
    let (c, bindings, artifacts) = fixture();
    capsule::export(
        &c,
        identity(SOURCE, "file:///app/main.cem"),
        &hash(b"registry"),
        BTreeSet::from([declaration(&c, "root")]),
        bindings,
        &artifacts,
        &OperationControl::default(),
    )
    .unwrap()
}
#[test]
fn original_dependency_fields_and_imported_pins_reload_inertly_and_deterministically() {
    let (c, bindings, artifacts) = fixture();
    assert!(dt::export(
        &c,
        identity(SOURCE, "file:///app/main.cem"),
        &hash(b"registry"),
        BTreeSet::from([declaration(&c, "root")]),
        &OperationControl::default()
    )
    .is_err());
    let b = build();
    assert_eq!(b.bytes, build().bytes);
    let a = capsule::verify(&b.bytes, &b.pin, &artifacts, &OperationControl::default()).unwrap();
    let other =
        capsule::verify(&b.bytes, &b.pin, &artifacts, &OperationControl::default()).unwrap();
    assert!(!Arc::ptr_eq(
        a.capture().document(),
        other.capture().document()
    ));
    assert_eq!(a.sites().len(), 3);
    assert_eq!(a.sources().len(), 3);
    assert_eq!(a.dependency_order().len(), 3);
    assert_eq!(a.source().source_uri(), "file:///app/main.cem");
    assert!(!a.executable_closure_verified());
    assert!(!a.selections_verified());
    assert_eq!(
        a.sites().iter().filter(|s| s.reference.is_some()).count(),
        1
    );
    assert_eq!(bindings.len(), 3);
    let namespaces = a
        .capture()
        .attribute_namespaces(a.capture().document(), field(&c, "imported", "base"))
        .unwrap();
    assert_eq!(
        namespaces.namespaces.binding("v").unwrap().namespace_uri,
        "urn:vendor"
    );
}
#[test]
fn omitted_or_orphan_fields_modules_and_foreign_pins_refuse() {
    for case in 0..5 {
        let (c, mut bindings, mut artifacts) = fixture();
        match case {
            0 => {
                bindings.pop_first();
            }
            1 => {
                bindings.insert(
                    u32::MAX,
                    capsule::DependencyTarget::LocalDatatype(declaration(&c, "local")),
                );
            }
            2 => {
                artifacts.clear();
            }
            3 => {
                if let capsule::DependencyTarget::Datatype { pin, .. } =
                    bindings.get_mut(&field(&c, "imported", "base")).unwrap()
                {
                    pin.registry_hash = hash(b"foreign");
                }
            }
            _ => {
                if let capsule::DependencyTarget::Datatype { root, .. } =
                    bindings.get_mut(&field(&c, "imported", "base")).unwrap()
                {
                    *root = u32::MAX;
                }
            }
        }
        assert!(
            capsule::export(
                &c,
                identity(SOURCE, "file:///app/main.cem"),
                &hash(b"registry"),
                BTreeSet::from([declaration(&c, "root")]),
                bindings,
                &artifacts,
                &OperationControl::default()
            )
            .is_err(),
            "case {case}"
        );
    }
}
fn rule_artifact(query: bool) -> (capsule::DependencyTarget, capsule::DependencyArtifact) {
    use cem_ml::schema::{
        datatype_registry::DatatypeKind,
        datatype_validation::{
            CandidateRequirement, DatatypeBehaviorContract, ResultRepresentation,
            ScalarRepresentation, ValidationSignature, ValueRepresentation,
        },
        declaration_references::SchemaDeclarationNode,
        registry::CEM_SCHEMA_URI,
        value_contracts::{ContractName, ValueContractSource},
    };
    use cem_ql::{native_registrations as nr, query_validator_capsule::declarations as qd};
    let header = if query {
        "@implementation=function @function=check-body"
    } else {
        "@implementation=engine @primitive=urn:test:validate"
    };
    let body = if query {
        "{function @name=check-body @returns=datatype-validation-result | {param @name=value @type=string @required=true @cardinality=one} {param @name=datatype @type=node @required=true @cardinality=one} {param @name=candidate @type=node @required=false @cardinality=zero-or-one} {body | {$ {accepted: true, diagnostics: ()} }}}"
    } else {
        ""
    };
    let text=format!("@ns schema = https://cem.dev/ns/schema/1\n@default schema\n{{schema @namespace=urn:rules | {{behaviors | {{behavior @name=check {header} @execution=datatype-validation | {{inputs | {{input-binding @name=value @type=schema:string @source=value @required=true @cardinality=one}} {{input-binding @name=datatype @type=schema:node @source=datatype @required=true @cardinality=one}} {{input-binding @name=candidate @type=schema:node @source=candidate @required=false @cardinality=zero-or-one}}}} {{result @type=schema:datatype-validation-result}} {body}}}}}}}");
    let c = capture(&text);
    let node = |name: &str| {
        c.document()
            .nodes
            .iter()
            .find_map(|n| match n {
                CemAstNode::Element {
                    node_id,
                    expanded_name,
                    ..
                } if expanded_name.local_name == name => {
                    SchemaDeclarationNode::new(c.document().clone(), *node_id)
                }
                _ => None,
            })
            .unwrap()
    };
    let source = ValueContractSource::new(
        node("schema"),
        CEM_SCHEMA_URI,
        BTreeMap::from([("schema".into(), CEM_SCHEMA_URI.into())]),
    )
    .with_captured_names(c.clone())
    .unwrap();
    let signature = ValidationSignature {
        kind: DatatypeKind::Scalar,
        value: ValueRepresentation::Scalar(ScalarRepresentation::String),
        candidate: CandidateRequirement::Optional,
        result: ResultRepresentation::Accepted(ContractName::new(
            CEM_SCHEMA_URI,
            "datatype-validation-result",
        )),
    };
    let contract =
        DatatypeBehaviorContract::compile(&source, &node("behavior"), signature.clone()).unwrap();
    let (target, artifact) = if query {
        let b = qd::export(
            &c,
            &contract,
            identity(&text, "file:///app/rules.cem"),
            &hash(b"registry"),
            &OperationControl::default(),
        )
        .unwrap();
        (
            capsule::DependencyTarget::Query(b.pin.clone()),
            capsule::DependencyArtifact::query(b),
        )
    } else {
        let b = nr::export_validator(
            &c,
            &contract,
            identity(&text, "file:///app/rules.cem"),
            nr::ImplementationIdentity::validator(
                "urn:test:validate",
                hash(b"module"),
                "validate",
                signature,
            ),
            &hash(b"registry"),
            &OperationControl::default(),
        )
        .unwrap();
        (
            capsule::DependencyTarget::Native(b.pin.clone()),
            capsule::DependencyArtifact::native(b),
        )
    };
    (target, artifact)
}
#[test]
fn exact_native_and_query_rule_requirements_are_inert_not_registered() {
    let consumer="@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:consumer | {types | {type @name=root @kind=scalar @rule={#library#check}}}}";
    for query in [false, true] {
        let (target, artifact) = rule_artifact(query);
        let c = capture(consumer);
        let original_target = target.clone();
        let artifacts = [artifact];
        let b = capsule::export(
            &c,
            identity(consumer, "file:///app/consumer.cem"),
            &hash(b"registry"),
            BTreeSet::from([declaration(&c, "root")]),
            BTreeMap::from([(field(&c, "root", "rule"), target)]),
            &artifacts,
            &OperationControl::default(),
        )
        .unwrap();
        assert!(b.bytes.starts_with(capsule::MAGIC));
        let mut changed = original_target.clone();
        match &mut changed {
            capsule::DependencyTarget::Query(pin) => {
                pin.program.declaration_hash = hash(b"changed")
            }
            capsule::DependencyTarget::Native(pin) => pin.source.hash = hash(b"changed"),
            _ => unreachable!(),
        }
        assert!(capsule::export(
            &c,
            identity(consumer, "file:///app/consumer.cem"),
            &hash(b"registry"),
            BTreeSet::from([declaration(&c, "root")]),
            BTreeMap::from([(field(&c, "root", "rule"), changed)]),
            &artifacts,
            &OperationControl::default()
        )
        .is_err());
        let wrong_role = consumer.replace("@rule=", "@base=");
        let c = capture(&wrong_role);
        assert!(capsule::export(
            &c,
            identity(&wrong_role, "file:///app/consumer.cem"),
            &hash(b"registry"),
            BTreeSet::from([declaration(&c, "root")]),
            BTreeMap::from([(field(&c, "root", "base"), original_target)]),
            &artifacts,
            &OperationControl::default()
        )
        .is_err());
    }
}
#[test]
fn native_reference_assertions_do_not_bypass_local_cycles_or_literal_names() {
    for text in [
        SOURCE.replace("@list-base=words", "@list-base=root"),
        SOURCE.replace("@list-base=words", "@list-base=local"),
    ] {
        let (c, bindings, artifacts) = fixture();
        let edited = capture(&text);
        // IDs/field occurrences are stable for these literal-only source edits.
        assert_eq!(
            field(&c, "root", "list-base"),
            field(&edited, "root", "list-base")
        );
        assert!(capsule::export(
            &edited,
            identity(&text, "file:///app/main.cem"),
            &hash(b"registry"),
            BTreeSet::from([declaration(&edited, "root")]),
            bindings,
            &artifacts,
            &OperationControl::default()
        )
        .is_err());
    }
}
#[test]
fn same_uri_conflicting_snapshots_and_unused_or_duplicate_artifacts_refuse() {
    let consumer = SOURCE.replace(
        "{type @name=local @kind=scalar}",
        "{type @name=local @base=v:base}",
    );
    for case in 0..4 {
        let c = capture(&consumer);
        let make = |text: &str| {
            let vendor = capture(text);
            dt::export(
                &vendor,
                identity(text, "file:///app/vendor.cem"),
                &hash(b"registry"),
                BTreeSet::from([declaration(&vendor, "base")]),
                &OperationControl::default(),
            )
            .unwrap()
        };
        let first = make(VENDOR);
        let second = if case == 3 {
            let changed = VENDOR.replace("urn:vendor", "urn:vendOr");
            let c = capture(&changed);
            dt::export(
                &c,
                identity(VENDOR, "file:///app/vendor.cem"),
                &hash(b"registry"),
                BTreeSet::from([declaration(&c, "base")]),
                &OperationControl::default(),
            )
            .unwrap()
        } else {
            make(&VENDOR.replace("@kind=scalar", "@kind=scalar @max-length=3"))
        };
        let mut bindings = BTreeMap::from([
            (
                field(&c, "local", "base"),
                capsule::DependencyTarget::Datatype {
                    pin: second.pin.clone(),
                    root: declaration(&capture(VENDOR), "base"),
                },
            ),
            (
                field(&c, "imported", "base"),
                capsule::DependencyTarget::Datatype {
                    pin: first.pin.clone(),
                    root: declaration(&capture(VENDOR), "base"),
                },
            ),
        ]);
        let roots = BTreeSet::from([declaration(&c, "local"), declaration(&c, "imported")]);
        let artifacts = match case {
            0 | 3 => vec![
                capsule::DependencyArtifact::datatype(first),
                capsule::DependencyArtifact::datatype(second),
            ],
            1 => {
                bindings.remove(&field(&c, "local", "base"));
                vec![
                    capsule::DependencyArtifact::datatype(first),
                    capsule::DependencyArtifact::datatype(second),
                ]
            }
            _ => {
                *bindings.get_mut(&field(&c, "local", "base")).unwrap() =
                    capsule::DependencyTarget::Datatype {
                        pin: first.pin.clone(),
                        root: declaration(&capture(VENDOR), "base"),
                    };
                vec![
                    capsule::DependencyArtifact::datatype(first),
                    capsule::DependencyArtifact::datatype(make(VENDOR)),
                ]
            }
        };
        assert!(
            capsule::export(
                &c,
                identity(&consumer, "file:///app/main.cem"),
                &hash(b"registry"),
                roots,
                bindings,
                &artifacts,
                &OperationControl::default()
            )
            .is_err(),
            "case {case}"
        );
    }
}
#[test]
fn cancellation_and_foreign_outer_pins_preserve_existing_verified_owners() {
    let b = build();
    let (_, _, artifacts) = fixture();
    let good = capsule::verify(&b.bytes, &b.pin, &artifacts, &OperationControl::default()).unwrap();
    let cancelled = OperationControl::default();
    cancelled.cancel_root(None, None).unwrap();
    assert!(capsule::verify(&b.bytes, &b.pin, &artifacts, &cancelled).is_err());
    let mut pin = b.pin.clone();
    pin.registry_hash = hash(b"foreign");
    assert!(capsule::verify(&b.bytes, &pin, &artifacts, &OperationControl::default()).is_err());
    assert_eq!(good.sites().len(), 3);
}
#[test]
fn complete_agreeing_source_snapshots_allow_distinct_inert_module_roots_without_owner_sharing() {
    let vendor = VENDOR.replace(
        "{type @name=base @kind=scalar}",
        "{type @name=base @kind=scalar} {type @name=other @kind=scalar}",
    );
    let vc = capture(&vendor);
    let make = |name: &str| {
        dt::export(
            &vc,
            identity(&vendor, "file:///app/vendor.cem"),
            &hash(b"registry"),
            BTreeSet::from([declaration(&vc, name)]),
            &OperationControl::default(),
        )
        .unwrap()
    };
    let a = make("base");
    let b = make("other");
    let text="@ns s = https://cem.dev/ns/schema/1\n@ns v = urn:vendor\n@default s\n{schema @namespace=urn:main | {types | {type @name=a @base=v:base} {type @name=b @base=v:other}}}";
    let c = capture(text);
    let built = capsule::export(
        &c,
        identity(text, "file:///app/main.cem"),
        &hash(b"registry"),
        BTreeSet::from([declaration(&c, "a"), declaration(&c, "b")]),
        BTreeMap::from([
            (
                field(&c, "a", "base"),
                capsule::DependencyTarget::Datatype {
                    pin: a.pin.clone(),
                    root: declaration(&vc, "base"),
                },
            ),
            (
                field(&c, "b", "base"),
                capsule::DependencyTarget::Datatype {
                    pin: b.pin.clone(),
                    root: declaration(&vc, "other"),
                },
            ),
        ]),
        &[
            capsule::DependencyArtifact::datatype(a),
            capsule::DependencyArtifact::datatype(b),
        ],
        &OperationControl::default(),
    )
    .unwrap();
    assert!(built.bytes.starts_with(capsule::MAGIC));
}

#[path = "datatype_dependency_capsule/selection.rs"]
mod selection;
