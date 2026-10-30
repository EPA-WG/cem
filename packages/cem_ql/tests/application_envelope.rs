use cem_ml::operation_control::OperationControl;
use cem_ql::{
    application_envelope::{self as envelope, EnvelopeBuilder},
    declaration_artifact::build_declaration_artifact,
    template_artifact::TemplateArtifactSourceMapMode,
};
use std::collections::{BTreeMap, BTreeSet};
fn hash(bytes: &[u8]) -> String {
    cem_ml::content_cache::ContentHash::from_blake3(bytes).header_value()
}
fn declaration(tag: &str, body: &str) -> cem_ql::declaration_artifact::BuiltDeclarationArtifact {
    build_declaration_artifact(
        &format!("<cem-element tag=\"{tag}\"><template id=\"{tag}\" type=\"text/cem-ml\">{body}</template></cem-element>"),
        &format!("file:///apps/example/{tag}.xhtml"), TemplateArtifactSourceMapMode::Dev, None,
    ).unwrap()
}
#[test]
fn bounded_dependency_envelope_roundtrips_without_runtime_admission() {
    let mut builder = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    builder
        .add_component(declaration("cem-app", "{section | Hello}"), BTreeMap::new())
        .unwrap();
    let built = builder
        .build(
            BTreeSet::from(["cem-app".into()]),
            BTreeSet::new(),
            &OperationControl::default(),
        )
        .unwrap();
    let verified =
        envelope::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap();
    assert_eq!(verified.roots(), &BTreeSet::from(["cem-app".into()]));
    assert_eq!(verified.component_count(), 1);
    assert_eq!(verified.contract_count(), 0);
    assert!(!verified.native_link_closure_verified());
}

use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::CemAstNode,
    schema::{
        datatype_registry::{DatatypeKind, DatatypeRegistry},
        datatype_validation::{
            CandidateRequirement, DatatypeBehaviorContract, ResultRepresentation,
            ScalarRepresentation, ValidationSignature, ValueRepresentation,
        },
        declaration_references::SchemaDeclarationNode,
        machine::{CemSchemaMachine, LexicallyScopedDocument},
        structural_capsule::{self, SourceIdentity},
        value_contracts::{ContractName, ValueContractSource},
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use cem_ql::{
    application_envelope::{ContractArtifact, DependencyKey, NodeReceiverRequirement},
    datatype_preparation::PreparationSignature,
    native_registrations::{self, ImplementationIdentity, OperationMode, PreparationMode},
    query_validator_capsule::declarations as query,
};
use std::sync::Arc;
const NS: &str = "https://cem.dev/ns/schema/1";
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
fn node(c: &LexicallyScopedDocument, name: &str) -> SchemaDeclarationNode {
    c.document()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                expanded_name,
                node_id,
                ..
            } if expanded_name.local_name == name => {
                SchemaDeclarationNode::new(c.document().clone(), *node_id)
            }
            _ => None,
        })
        .unwrap()
}
const TYPES:&str="@ns schema = \"https://cem.dev/ns/schema/1\"\n@default schema\n{schema @name=example @namespace=urn:example | {types | {type @name=sample @kind=scalar}}}";
fn native(id: &str) -> ContractArtifact {
    native_custom(id, TYPES, "sample", b"native-module")
}
fn native_custom(id: &str, text: &str, name: &str, module: &[u8]) -> ContractArtifact {
    let c = capture(text);
    let schema = node(&c, "schema");
    let mut registry = DatatypeRegistry::default();
    registry.insert(schema.clone(), node(&c, "type")).unwrap();
    let source = registry.source(&schema, name).unwrap();
    let signature = PreparationSignature {
        kind: DatatypeKind::Scalar,
        output: ValueRepresentation::Scalar(ScalarRepresentation::String),
        candidate: CandidateRequirement::Optional,
    };
    ContractArtifact::native(
        native_registrations::export_operation(
            &c,
            &source,
            OperationMode::Preparation(PreparationMode::Ready),
            SourceIdentity::new(
                "file:///apps/example/types.cem",
                SourceId(1),
                TYPES.as_bytes(),
            ),
            ImplementationIdentity::preparation(id, hash(module), "prepare", signature),
            &hash(b"registry"),
            &OperationControl::default(),
        )
        .unwrap(),
    )
}
const SCHEMA: &str = "https://app.test/schema/cart/1";
fn structural() -> ContractArtifact {
    let text = include_str!("../../cem_ml/fixtures/structural-cart.cem");
    let capture = cem_ml::import::import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "file:///apps/example/cart.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap();
    ContractArtifact::structural(
        structural_capsule::export(
            &capture.captured,
            SourceIdentity::new(
                "file:///apps/example/cart.cem",
                SourceId(1),
                text.as_bytes(),
            ),
            SCHEMA,
            &hash(b"registry"),
            &OperationControl::default(),
        )
        .unwrap(),
    )
}
fn query() -> ContractArtifact {
    let text = r#"@ns schema = "https://cem.dev/ns/schema/1"
@default schema
{schema @name=example @namespace=urn:example |
 {types | {type @name=sample @kind=scalar}}
 {behaviors | {behavior @name=check @implementation=function @function=run @execution=datatype-validation |
 {inputs | {input-binding @name=value @type=schema:string @source=value @required=true @cardinality=one} {input-binding @name=datatype @type=schema:node @source=datatype @required=true @cardinality=one} {input-binding @name=candidate @type=schema:node @source=candidate @required=false @cardinality=zero-or-one}}
 {result @type=schema:datatype-validation-result}
 {function @name=run @returns=datatype-validation-result | {param @name=value @type=string @required=true @cardinality=one} {param @name=datatype @type=node @required=true @cardinality=one} {param @name=candidate @type=node @required=false @cardinality=zero-or-one} {body | {$ {accepted: true, diagnostics: ()} }}}}}
}"#;
    let c = capture(text);
    let source = ValueContractSource::new(
        node(&c, "schema"),
        NS,
        BTreeMap::from([("schema".into(), NS.into())]),
    )
    .with_captured_names(c.clone())
    .unwrap();
    let signature = ValidationSignature {
        kind: DatatypeKind::Scalar,
        value: ValueRepresentation::Scalar(ScalarRepresentation::String),
        candidate: CandidateRequirement::Optional,
        result: ResultRepresentation::Accepted(ContractName::new(NS, "datatype-validation-result")),
    };
    let contract =
        DatatypeBehaviorContract::compile(&source, &node(&c, "behavior"), signature).unwrap();
    ContractArtifact::query(
        query::export(
            &c,
            &contract,
            SourceIdentity::new(
                "file:///apps/example/query.cem",
                SourceId(1),
                text.as_bytes(),
            ),
            &hash(b"registry"),
            &OperationControl::default(),
        )
        .unwrap(),
    )
}
#[test]
fn graph_pins_structural_query_and_native_dependencies_and_shares_complete_sources() {
    let mut builder = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    let native_a = builder
        .add_contract(native("prepare-a"), BTreeSet::new())
        .unwrap();
    let native_b = builder
        .add_contract(native("prepare-b"), BTreeSet::from([native_a.clone()]))
        .unwrap();
    let query = builder
        .add_contract(query(), BTreeSet::from([native_b.clone()]))
        .unwrap();
    let structural = builder.add_contract(structural(), BTreeSet::new()).unwrap();
    let receivers = BTreeMap::from([(
        "cart-lines".into(),
        NodeReceiverRequirement {
            contract: structural.clone(),
            schema_uri: SCHEMA.into(),
        },
    )]);
    builder
        .add_component(
            declaration(
                "cem-app",
                "{attribute @name=items @type=cart-lines}{section | {cem-child}}",
            ),
            receivers.clone(),
        )
        .unwrap();
    builder
        .add_component(
            declaration(
                "cem-child",
                "{attribute @name=items @type=cart-lines}{p | Child}",
            ),
            receivers,
        )
        .unwrap();
    let built = builder
        .build(
            BTreeSet::from(["cem-app".into()]),
            BTreeSet::from([query.clone()]),
            &OperationControl::default(),
        )
        .unwrap();
    let verified =
        envelope::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap();
    assert_eq!(verified.component_count(), 2);
    assert_eq!(verified.contract_count(), 4);
    assert_eq!(
        verified
            .receiver("cem-child", "cart-lines")
            .unwrap()
            .schema_uri,
        SCHEMA
    );
    assert_eq!(
        verified.structural(&structural).unwrap().pin().schema_uri,
        SCHEMA
    );
    assert!(verified.query(&query).is_some());
    assert!(Arc::ptr_eq(
        verified.native(&native_a).unwrap().source(),
        verified.native(&native_b).unwrap().source()
    ));
    let second = envelope::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap();
    assert!(!Arc::ptr_eq(
        verified.native(&native_a).unwrap().source(),
        second.native(&native_a).unwrap().source()
    ));
    let order = verified.dependency_order();
    let position = |key| order.iter().position(|n| n == &key).unwrap();
    assert!(
        position(DependencyKey::Contract(native_a)) < position(DependencyKey::Contract(native_b))
    );
    assert!(
        position(DependencyKey::Component("cem-child".into()))
            < position(DependencyKey::Component("cem-app".into()))
    );
    assert_eq!(verified.artifact("cem-app").unwrap().nodes.len(), 2);
}
#[test]
fn missing_orphan_cyclic_and_wrong_receiver_dependencies_never_verify() {
    for case in 0..7 {
        let mut builder = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
        let asset = structural();
        let id = asset.id().to_owned();
        let deps = if case == 3 {
            BTreeSet::from([id.clone()])
        } else if case == 4 {
            BTreeSet::from([hash(b"missing")])
        } else {
            BTreeSet::new()
        };
        builder.add_contract(asset, deps).unwrap();
        let receiver = NodeReceiverRequirement {
            contract: if case == 1 {
                hash(b"missing")
            } else {
                id.clone()
            },
            schema_uri: if case == 2 {
                "urn:wrong".into()
            } else {
                SCHEMA.into()
            },
        };
        let receivers = if case == 0 {
            BTreeMap::new()
        } else {
            BTreeMap::from([("cart-lines".into(), receiver)])
        };
        let body = if case == 5 {
            "{section | Hello}"
        } else if case == 6 {
            "{attribute @name=items @type=cart-lines}{cem-missing}"
        } else {
            "{attribute @name=items @type=cart-lines}{section | Hello}"
        };
        builder
            .add_component(declaration("cem-app", body), receivers)
            .unwrap();
        assert!(
            builder
                .build(
                    BTreeSet::from(["cem-app".into()]),
                    BTreeSet::new(),
                    &OperationControl::default()
                )
                .is_err(),
            "case {case}"
        );
    }
    let mut builder = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    builder
        .add_component(declaration("cem-app", "{cem-child}"), BTreeMap::new())
        .unwrap();
    builder
        .add_component(declaration("cem-child", "{cem-app}"), BTreeMap::new())
        .unwrap();
    assert!(builder
        .build(
            BTreeSet::from(["cem-app".into()]),
            BTreeSet::new(),
            &OperationControl::default()
        )
        .is_err());
}
#[test]
fn repeated_content_is_deduplicated_and_packing_is_deterministic() {
    let build = |reverse| {
        let mut b = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
        let a = native("prepare");
        let id = a.id().to_owned();
        b.add_contract(a, BTreeSet::new()).unwrap();
        assert_eq!(
            b.add_contract(native("prepare"), BTreeSet::new()).unwrap(),
            id
        );
        let tags = if reverse {
            ["cem-child", "cem-app"]
        } else {
            ["cem-app", "cem-child"]
        };
        for tag in tags {
            b.add_component(
                declaration(
                    tag,
                    if tag == "cem-app" {
                        "{cem-child}"
                    } else {
                        "{p | Child}"
                    },
                ),
                BTreeMap::new(),
            )
            .unwrap();
        }
        b.build(
            BTreeSet::from(["cem-app".into()]),
            BTreeSet::from([id]),
            &OperationControl::default(),
        )
        .unwrap()
    };
    let a = build(false);
    let b = build(true);
    assert_eq!(a.bytes, b.bytes);
    assert_eq!(a.pin.manifest_hash, b.pin.manifest_hash);
}
#[test]
fn cancellation_registry_mismatch_and_legacy_refusal_preserve_a_verified_owner() {
    let mut builder = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    builder
        .add_component(declaration("cem-app", "{section | Hello}"), BTreeMap::new())
        .unwrap();
    let built = builder
        .build(
            BTreeSet::from(["cem-app".into()]),
            BTreeSet::new(),
            &OperationControl::default(),
        )
        .unwrap();
    let live = envelope::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap();
    let control = OperationControl::default();
    control.cancel_root(None, None).unwrap();
    assert!(envelope::verify(&built.bytes, &built.pin, &control).is_err());
    let mut pin = built.pin.clone();
    pin.registry_hash = hash(b"other registry");
    assert!(envelope::verify(&built.bytes, &pin, &OperationControl::default()).is_err());
    assert!(cem_ql::application_declaration::admit(&built.bytes, "{}", &hash(b"{}")).is_err());
    assert_eq!(live.declaration("cem-app").unwrap().tag, "cem-app");
    let mut other = EnvelopeBuilder::new(&hash(b"wrong registry")).unwrap();
    assert!(other
        .add_contract(native("prepare"), BTreeSet::new())
        .is_err());
}
#[test]
fn dynamic_resource_and_behavior_gaps_refuse_instead_of_inventing_dependencies() {
    for body in [
        "{cem-element @src=\"./unknown.xhtml\"}",
        "{slot}",
        "{img @src=\"logo.png\"}",
    ] {
        let mut b = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
        assert!(b
            .add_component(declaration("cem-app", body), BTreeMap::new())
            .is_err());
    }
    let mut b = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    b.add_component(declaration("cem-app", "{p | First}"), BTreeMap::new())
        .unwrap();
    assert!(b
        .add_component(declaration("cem-app", "{p | Replacement}"), BTreeMap::new())
        .is_err());
    assert!(b
        .build(
            BTreeSet::from(["cem-app".into()]),
            BTreeSet::new(),
            &OperationControl::default()
        )
        .is_ok());
}

#[test]
fn conflicting_implementation_and_claimed_source_identities_refuse() {
    for source_conflict in [false, true] {
        let mut b = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
        let a = b.add_contract(native("prepare"), BTreeSet::new()).unwrap();
        let changed = if source_conflict {
            native_custom(
                "other",
                &TYPES.replace("@name=sample", "@name=samplX"),
                "samplX",
                b"native-module",
            )
        } else {
            native_custom("prepare", TYPES, "sample", b"other-module")
        };
        let id = b.add_contract(changed, BTreeSet::new()).unwrap();
        b.add_component(declaration("cem-app", "{p | Hello}"), BTreeMap::new())
            .unwrap();
        assert!(b
            .build(
                BTreeSet::from(["cem-app".into()]),
                BTreeSet::from([a, id]),
                &OperationControl::default()
            )
            .is_err());
    }
}
#[test]
fn contract_count_and_graph_depth_have_independent_bounds() {
    let mut b = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    let mut roots = BTreeSet::new();
    for i in 0..64 {
        roots.insert(
            b.add_contract(native(&format!("prepare-{i}")), BTreeSet::new())
                .unwrap(),
        );
    }
    assert!(b.add_contract(native("overflow"), BTreeSet::new()).is_err());
    b.add_component(declaration("cem-app", "{p | Hello}"), BTreeMap::new())
        .unwrap();
    let built = b
        .build(
            BTreeSet::from(["cem-app".into()]),
            roots,
            &OperationControl::default(),
        )
        .unwrap();
    assert_eq!(
        envelope::verify(&built.bytes, &built.pin, &OperationControl::default())
            .unwrap()
            .contract_count(),
        64
    );
    let mut b = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    let mut previous = None;
    for i in 0..34 {
        let deps = previous.into_iter().collect();
        previous = Some(b.add_contract(native(&format!("chain-{i}")), deps).unwrap());
    }
    b.add_component(declaration("cem-app", "{p | Hello}"), BTreeMap::new())
        .unwrap();
    assert!(b
        .build(
            BTreeSet::from(["cem-app".into()]),
            previous.into_iter().collect(),
            &OperationControl::default()
        )
        .is_err());
}
#[test]
fn component_and_contract_orphans_cannot_hide_outside_explicit_roots() {
    for contract in [false, true] {
        let mut b = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
        b.add_component(declaration("cem-app", "{p | Hello}"), BTreeMap::new())
            .unwrap();
        if contract {
            b.add_contract(native("unused"), BTreeSet::new()).unwrap();
        } else {
            b.add_component(declaration("cem-orphan", "{p | Hidden}"), BTreeMap::new())
                .unwrap();
        }
        assert!(b
            .build(
                BTreeSet::from(["cem-app".into()]),
                BTreeSet::new(),
                &OperationControl::default()
            )
            .is_err());
    }
}

#[test]
fn declared_graph_edge_budget_accepts_boundary_and_refuses_one_more_edge() {
    for overflow in [false, true] {
        let mut builder = EnvelopeBuilder::new(&hash(b"registry")).unwrap();
        let leaves = (0..32)
            .map(|i| {
                builder
                    .add_contract(native(&format!("leaf-{i}")), BTreeSet::new())
                    .unwrap()
            })
            .collect::<BTreeSet<_>>();
        let mut roots = BTreeSet::new();
        let mut first_parent = None;
        for i in 0..32 {
            let mut dependencies = leaves.clone();
            if overflow && i == 31 {
                dependencies.insert(first_parent.clone().unwrap());
            }
            let id = builder
                .add_contract(native(&format!("parent-{i}")), dependencies)
                .unwrap();
            first_parent.get_or_insert_with(|| id.clone());
            roots.insert(id);
        }
        builder
            .add_component(declaration("cem-app", "{p | Hello}"), BTreeMap::new())
            .unwrap();
        let result = builder.build(
            BTreeSet::from(["cem-app".into()]),
            roots,
            &OperationControl::default(),
        );
        assert_eq!(result.is_err(), overflow);
    }
}

#[path = "application_envelope/assembly.rs"]
mod assembly;
