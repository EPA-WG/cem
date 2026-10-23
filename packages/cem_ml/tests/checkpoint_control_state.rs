use cem_ml::checkpoint_control_state::validate;
fn state() -> serde_json::Value {
    serde_json::json!({"version":"cem-checkpoint-controls/1","controls":[{
        "id":"node:1","kind":"text","inputType":"text","value":"é😀draft","checked":false,
        "indeterminate":false,"multiple":false,"options":[],"focused":true,"disabled":true,"readOnly":true,
        "customValidity":"draft needs review","selection":{"start":1,"end":3,"direction":"backward"}
    }]})
}
#[test]
fn validates_utf16_selection_and_preserves_all_control_edits() {
    let input = state();
    let result = validate(&input.to_string()).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&result).unwrap(),
        input
    );
    let mut multi = state();
    let c = &mut multi["controls"][0];
    c["kind"] = "select".into();
    c["inputType"] = "select-multiple".into();
    c["multiple"] = true.into();
    c["selection"] = serde_json::Value::Null;
    c["value"] = "a".into();
    c["options"] = serde_json::json!([{"value":"a","selected":true,"disabled":true},{"value":"b","selected":true,"disabled":false}]);
    assert!(validate(&multi.to_string()).is_ok());
}
#[test]
fn rejects_secrets_unknown_fields_bad_shapes_and_stale_selection() {
    for typ in ["password", "file", "unexpected"] {
        let mut s = state();
        s["controls"][0]["inputType"] = typ.into();
        assert!(validate(&s.to_string()).is_err());
    }
    let mut s = state();
    s["controls"][0]["selection"]["end"] = 100.into();
    assert!(validate(&s.to_string()).is_err());
    let mut s = state();
    s["controls"][0]["hostHandle"] = 7.into();
    assert!(validate(&s.to_string()).is_err());
    let mut s = state();
    s["controls"][0]["kind"] = "check".into();
    assert!(validate(&s.to_string()).is_err());
    let mut s = state();
    let duplicate = s["controls"][0].clone();
    s["controls"].as_array_mut().unwrap().push(duplicate);
    assert!(validate(&s.to_string()).is_err());
    let mut s = state();
    s["controls"][0]["selection"]["start"] = (-1i32).into();
    assert!(validate(&s.to_string()).is_err());
}
#[test]
fn refuses_oversized_or_excessive_control_state() {
    let mut s = state();
    s["controls"][0]["value"] = "x".repeat(262145).into();
    assert!(validate(&s.to_string()).is_err());
    let mut s = state();
    let mut controls = vec![];
    for i in 0..257 {
        let mut c = s["controls"][0].clone();
        c["id"] = format!("node:{i}").into();
        c["focused"] = false.into();
        controls.push(c);
    }
    s["controls"] = controls.into();
    assert!(validate(&s.to_string()).is_err());
}

#[test]
fn bounds_normalized_output_even_when_optional_selection_is_omitted() {
    let mut s = state();
    s["controls"][0]
        .as_object_mut()
        .unwrap()
        .remove("selection");
    s["controls"][0]["value"] = "".into();
    let remaining = 256 * 1024 - s.to_string().len();
    s["controls"][0]["value"] = "x".repeat(remaining).into();
    assert!(validate(&s.to_string()).is_err());
}

#[test]
fn credential_policy_refuses_empty_and_entered_controls_without_echoing_values() {
    for input_type in ["password", "file", "PASSWORD"] {
        for value in ["", "fixture-private-credential"] {
            let mut s = state();
            let control = &mut s["controls"][0];
            control["inputType"] = input_type.into();
            control["value"] = value.into();
            control["selection"] = serde_json::Value::Null;
            assert_eq!(
                validate(&s.to_string()).unwrap_err(),
                "cem.checkpoint.controls.shape"
            );
        }
    }
}

#[test]
fn empty_credential_controls_have_a_distinct_fail_closed_contract() {
    for input_type in ["password", "text", "tel", "number"] {
        let mut s = state();
        let c = &mut s["controls"][0];
        c["kind"] = "credential-empty".into();
        c["inputType"] = input_type.into();
        c["value"] = "".into();
        c["customValidity"] = "".into();
        c["selection"] = serde_json::Value::Null;
        assert!(validate(&s.to_string()).is_ok());
        for field in ["value", "customValidity"] {
            let mut bad = s.clone();
            bad["controls"][0][field] = "fixture-private-credential".into();
            assert!(validate(&bad.to_string()).is_err());
        }
        let mut bad = s.clone();
        bad["controls"][0]["inputType"] = "file".into();
        assert!(validate(&bad.to_string()).is_err());
    }
}
