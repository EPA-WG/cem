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

// CEMT-ATTRIBUTE-MATRIX: the same conversion is used by output, receivers and artifacts.
#[test]
fn scalar_lexical_profiles_convert_without_losing_precision_or_inventing_zones() {
    for (datatype, input, expected) in [
        ("integer", " +00042 ", "42"),
        ("integer", "-000", "0"),
        (
            "integer",
            "922337203685477580812345",
            "922337203685477580812345",
        ),
        (
            "decimal",
            "12345678901234567890.123456789",
            "12345678901234567890.123456789",
        ),
        ("number", "1.25e2", "1.25e2"),
        ("boolean", " 1 ", "true"),
        ("boolean", "0", "false"),
        ("string", "  🍒 & <b>  ", "  🍒 & <b>  "),
        ("date", "2000-02-29", "2000-02-29"),
        ("date", "2024-02-29-14:00", "2024-02-29-14:00"),
        ("time", "23:59:59.123456789", "23:59:59.123456789"),
        ("time", "00:00:00Z", "00:00:00Z"),
        ("time", "12:00:00+14:00", "12:00:00+14:00"),
        (
            "dateTime",
            "2024-02-29T23:59:59.5-07:30",
            "2024-02-29T23:59:59.5-07:30",
        ),
        ("datetime", "2024-02-29T00:00:00", "2024-02-29T00:00:00"),
    ] {
        let contract = AttributeValueContract {
            model: AttributeModel {
                value_type: Some(datatype.into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let converted = convert_attribute_value(input, &contract, &Default::default())
            .unwrap_or_else(|errors| panic!("{datatype} {input}: {errors:?}"));
        assert_eq!(converted.datatype, datatype);
        assert_eq!(converted.lexical, expected, "{datatype} {input}");
    }
}

#[test]
fn invalid_scalar_and_temporal_lexicals_are_rejected() {
    for (datatype, input) in [
        ("integer", ""),
        ("integer", "1.0"),
        ("integer", "1e2"),
        ("decimal", "NaN"),
        ("number", "INF"),
        ("number", "1,25"),
        ("boolean", "TRUE"),
        ("boolean", "yes"),
        ("boolean", "2"),
        ("date", "0000-01-01"),
        ("date", "1900-02-29"),
        ("date", "2024-04-31"),
        ("date", "2024-2-01"),
        ("date", "2024-02-29+14:01"),
        ("time", "24:00:00"),
        ("time", "23:59:60"),
        ("time", "12:00:00."),
        ("time", "12:00:00+15:00"),
        ("time", "12:00:00+01:60"),
        ("time", "12:00:00z"),
        ("time", "12:00:00+1:00"),
        ("dateTime", "2024-02-29 12:00:00"),
        ("datetime", "2023-02-29T12:00:00Z"),
        ("dateTime", "2024-02-29T１２:00:00Z"),
    ] {
        assert!(
            !convert(
                input,
                AttributeModel {
                    value_type: Some(datatype.into()),
                    ..Default::default()
                },
                vec![]
            ),
            "accepted {datatype} {input}"
        );
    }
}

#[test]
fn numeric_facets_and_unicode_string_constraints_check_the_final_value() {
    let amount = AttributeModel {
        value_type: Some("decimal".into()),
        min_exclusive: Some("-1.25".into()),
        max_inclusive: Some("12.50".into()),
        total_digits: Some("4".into()),
        fraction_digits: Some("2".into()),
        ..Default::default()
    };
    for value in ["0", "-1.24", "12.50"] {
        assert!(convert(value, amount.clone(), vec![]), "rejected {value}");
    }
    for value in ["-1.25", "12.51", "1.234"] {
        assert!(!convert(value, amount.clone(), vec![]), "accepted {value}");
    }
    let label = AttributeModel {
        value_type: Some("string".into()),
        length: Some("2".into()),
        pattern: Some("🍒[A-Z]".into()),
        ..Default::default()
    };
    assert!(convert("🍒A", label.clone(), vec![]));
    assert!(!convert("🍒AB", label.clone(), vec![]));
    assert!(!convert("🍒a", label, vec![]));
}
