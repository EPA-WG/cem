//! Migration evidence for the general datatype compiler design.
//! This inventory does not supply an executable registry or enable native types.
use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::{builder::CemAstBuilder, document::CemDocument, CemAstNode},
    schema::{
        declaration_references::SchemaDeclarationNode,
        document_model::{
            compile_schema_document_model, convert_attribute_value, validate_document_model,
            AttributeModel, AttributeValueContract,
        },
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use std::{collections::BTreeMap, sync::Arc};

fn parse(text: &str) -> Arc<CemDocument> {
    let document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), text.as_bytes().to_vec()),
    )))
    .build();
    assert!(
        document.diagnostics.is_empty(),
        "{:?}",
        document.diagnostics
    );
    Arc::new(document)
}

fn attribute<'a>(declaration: &'a SchemaDeclarationNode, name: &str) -> Option<&'a str> {
    let CemAstNode::Element { attributes, .. } = declaration.node() else {
        return None;
    };
    let id = attributes.iter().rev().find(|id| matches!(declaration.document().get(**id), Some(CemAstNode::Attribute {expanded_name, ..}) if expanded_name.local_name == name))?;
    let CemAstNode::Attribute { value, .. } = declaration.document().get(*id)? else {
        return None;
    };
    value.as_deref()
}

#[test]
fn shipped_datatype_inventory_retains_original_declarations_and_kind_boundaries() {
    let owner = parse(include_str!(
        "../schema-packages/cem-ml/v1/schema/cem-ml-generic.cem"
    ));
    let CemAstNode::Element {children, ..} = owner.nodes.iter().find(|node| matches!(node, CemAstNode::Element {expanded_name, ..} if expanded_name.local_name == "types")).unwrap() else {unreachable!()};
    let declarations: Vec<_> = children
        .iter()
        .filter(|id| matches!(owner.get(**id), Some(CemAstNode::Element {expanded_name, ..}) if expanded_name.local_name == "type"))
        .map(|id| SchemaDeclarationNode::new(owner.clone(), *id).unwrap())
        .collect();
    assert_eq!(declarations.len(), 18);
    let shipped = cem_ml::schema::document_model::shipped_datatypes::ShippedDatatype::ALL;
    let names: std::collections::BTreeSet<_> = declarations
        .iter()
        .map(|d| attribute(d, "name").unwrap())
        .collect();
    assert_eq!(names, shipped.iter().map(|t| t.name()).collect());
    for datatype in shipped {
        let declaration = declarations
            .iter()
            .find(|d| attribute(d, "name") == Some(datatype.name()))
            .unwrap();
        assert_eq!(
            attribute(declaration, "kind").unwrap(),
            format!("{:?}", datatype.kind()).to_ascii_lowercase()
        );
    }
    assert!(declarations
        .iter()
        .all(|declaration| Arc::ptr_eq(declaration.document(), &owner)));
    drop(owner);
    let mut kinds: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for declaration in &declarations {
        kinds
            .entry(attribute(declaration, "kind").unwrap())
            .or_default()
            .push(attribute(declaration, "name").unwrap());
    }
    assert_eq!(kinds.len(), 5);
    assert_eq!(
        kinds["lexical"],
        [
            "identifier",
            "qualified-name",
            "symbol-reference",
            "wildcard-name"
        ]
    );
    assert_eq!(kinds["scalar"].len(), 9);
    assert_eq!(kinds["list"], ["name-list", "wildcard-name-list"]);
    assert_eq!(kinds["grammar"], ["content-model"]);
    assert_eq!(
        kinds["reference"],
        ["type-reference", "wildcard-type-reference"]
    );
    let named = |name: &str| {
        declarations
            .iter()
            .find(|node| attribute(node, "name") == Some(name))
            .unwrap()
    };
    assert_eq!(attribute(named("name-list"), "base"), Some("identifier"));
    assert_eq!(
        attribute(named("wildcard-name-list"), "base"),
        Some("wildcard-name")
    );
    assert_eq!(
        attribute(named("type-reference"), "base"),
        Some("qualified-name")
    );
    assert_eq!(attribute(named("identifier"), "rule"), Some("local-name"));
    assert_eq!(
        attribute(named("integer"), "rule"),
        Some("signed decimal integer")
    );
    assert_eq!(
        attribute(named("string"), "rule"),
        Some("quoted-value | raw-text")
    );
}

#[test]
fn scalar_primitive_conversion_outputs_pass_the_existing_schema_validator() {
    for (datatype, input, expected) in [
        ("string", " keep spaces ", " keep spaces "),
        ("boolean", "1", "true"),
        ("integer", "003", "3"),
        ("number", "1.25", "1.25"),
    ] {
        let schema = format!("{{schema | {{elements | {{element @name=box @required-attributes=target}}}} {{attributes | {{attribute @name=target @type=schema:{datatype}}}}}}}");
        let model = compile_schema_document_model("consumer", &schema);
        assert!(model.is_ready_for_validation());
        let converted = convert_attribute_value(
            input,
            &AttributeValueContract {
                model: model.attributes["target"].clone(),
                ..Default::default()
            },
            &Default::default(),
        )
        .unwrap();
        assert_eq!(converted.datatype, datatype);
        assert_eq!(converted.lexical, expected);
        let text = format!("{{box @target={:?}}}", converted.lexical);
        let diagnostics = validate_document_model(&parse(&text), &model);
        assert!(diagnostics.is_empty(), "{datatype}: {diagnostics:?}");
    }
}

#[test]
fn current_conversion_gaps_remain_explicit_before_general_datatype_compilation() {
    for datatype in [
        "identifier",
        "qualified-name",
        "uri",
        "semver",
        "media-type",
        "path",
        "name-list",
        "content-model",
        "type-reference",
    ] {
        let errors = convert_attribute_value(
            "sample",
            &AttributeValueContract {
                model: AttributeModel {
                    value_type: Some(datatype.into()),
                    ..Default::default()
                },
                ..Default::default()
            },
            &Default::default(),
        )
        .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|diagnostic| diagnostic.message.contains("Unresolved attribute datatype")),
            "{datatype}: {errors:?}"
        );
    }
}
