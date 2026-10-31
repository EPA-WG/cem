use super::*;
pub(super) fn rule_artifact(
    query: bool,
) -> (capsule::DependencyTarget, capsule::DependencyArtifact) {
    rule_artifact_body(query, "{accepted: true, diagnostics: ()}")
}
pub(super) fn rule_artifact_body(
    query: bool,
    expression: &str,
) -> (capsule::DependencyTarget, capsule::DependencyArtifact) {
    rule_artifact_at(query, expression, "file:///app/rules.cem")
}
pub(super) fn rule_artifact_at(
    query: bool,
    expression: &str,
    uri: &str,
) -> (capsule::DependencyTarget, capsule::DependencyArtifact) {
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
    let text = text.replace("{accepted: true, diagnostics: ()}", expression);
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
            identity(&text, uri),
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
            identity(&text, uri),
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
