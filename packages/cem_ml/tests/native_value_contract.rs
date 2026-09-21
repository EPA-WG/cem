use cem_ml::schema::document_model::{
    convert_attribute_value, AttributeModel, AttributeValueContract,
};

fn convert(value: &str, model: AttributeModel, restrictions: Vec<AttributeModel>) -> bool {
    convert_attribute_value(
        value,
        &AttributeValueContract {
            model,
            restrictions,
            ..Default::default()
        },
        &Default::default(),
    )
    .is_ok()
}

#[test]
fn inherited_and_local_bounds_both_apply() {
    let base = AttributeModel {
        value_type: Some("integer".into()),
        min_inclusive: Some("3".into()),
        ..Default::default()
    };
    let local = AttributeModel {
        min_inclusive: Some("1".into()),
        max_inclusive: Some("5".into()),
        ..Default::default()
    };
    assert!(!convert("2", base.clone(), vec![local.clone()]));
    assert!(!convert("6", base.clone(), vec![local.clone()]));
    assert!(convert("003", base, vec![local]));
}

#[test]
fn independent_patterns_intersect_without_rewriting_regexes() {
    let base = AttributeModel {
        value_type: Some("string".into()),
        pattern: Some("[A-Z]+".into()),
        ..Default::default()
    };
    let local = AttributeModel {
        pattern: Some("[A-Za-z]{3}".into()),
        ..Default::default()
    };
    assert!(!convert("abc", base.clone(), vec![local.clone()]));
    assert!(!convert("ABCD", base.clone(), vec![local.clone()]));
    assert!(convert("ABC", base.clone(), vec![local]));
    assert!(!convert(
        "ABC",
        base,
        vec![AttributeModel {
            pattern: Some("[0-9]+".into()),
            ..Default::default()
        }]
    ));
}

#[test]
fn final_normalized_value_satisfies_every_constraint() {
    let mut contract = AttributeValueContract {
        model: AttributeModel {
            value_type: Some("string".into()),
            white_space: Some("collapse".into()),
            pattern: Some("A B".into()),
            ..Default::default()
        },
        restrictions: vec![AttributeModel {
            white_space: Some("preserve".into()),
            length: Some("3".into()),
            ..Default::default()
        }],
        ..Default::default()
    };
    assert_eq!(
        convert_attribute_value(" A\t B ", &contract, &Default::default())
            .unwrap()
            .lexical,
        "A B"
    );
    contract.model.white_space = Some("preserve".into());
    contract.restrictions[0].white_space = Some("collapse".into());
    assert_eq!(
        convert_attribute_value(" A\t B ", &contract, &Default::default())
            .unwrap()
            .lexical,
        "A B"
    );
    contract.restrictions[0].pattern = Some(" A B ".into());
    assert!(convert_attribute_value(" A\t B ", &contract, &Default::default()).is_err());
    contract.restrictions[0].white_space = Some("unknown".into());
    assert!(convert_attribute_value("A B", &contract, &Default::default()).is_err());
}

#[test]
fn validation_can_be_cancelled_between_flat_restrictions() {
    use cem_ml::schema::document_model::{
        convert_attribute_value_with_check, AttributeValueConversionError,
    };
    let contract = AttributeValueContract {
        model: AttributeModel {
            value_type: Some("integer".into()),
            ..Default::default()
        },
        restrictions: vec![
            AttributeModel {
                min_inclusive: Some("1".into()),
                ..Default::default()
            };
            100
        ],
        ..Default::default()
    };
    let mut steps = 0;
    let result =
        convert_attribute_value_with_check("3", &contract, &Default::default(), &mut || {
            steps += 1;
            if steps == 120 {
                Err("cancelled")
            } else {
                Ok(())
            }
        });
    assert!(matches!(
        result,
        Err(AttributeValueConversionError::Interrupted("cancelled"))
    ));
    assert_eq!(steps, 120);
}

#[test]
fn invalid_restrictions_are_errors_not_ignored_facets() {
    let integer = AttributeModel {
        value_type: Some("integer".into()),
        ..Default::default()
    };
    assert!(!convert(
        "3",
        integer.clone(),
        vec![AttributeModel {
            min_inclusive: Some("invalid".into()),
            ..Default::default()
        }]
    ));
    assert!(!convert(
        "3",
        integer,
        vec![AttributeModel {
            total_digits: Some("invalid".into()),
            ..Default::default()
        }]
    ));
    let string = AttributeModel {
        value_type: Some("string".into()),
        ..Default::default()
    };
    assert!(!convert(
        "ABC",
        string,
        vec![AttributeModel {
            pattern: Some("[".into()),
            ..Default::default()
        }]
    ));
}
