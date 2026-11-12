use cem_ml::{
    diagnostics::Severity,
    schema::package_sources::builtin_schema_package_source,
    validation::json::{
        extract_json_parse_report, validate_json_parse_report, JsonDiagnosticBinding,
        JsonSchemaContractCatalog, JsonSourceValidationRequest,
    },
};

#[path = "../src/validation/json/builtin_contracts.rs"]
#[rustfmt::skip]
mod compiled;

#[test]
fn compiled_json_contracts_match_the_canonical_native_schema() {
    assert_eq!(
        compiled::catalog(),
        JsonSchemaContractCatalog::from_builtin()
    );
    let source = builtin_schema_package_source("json").unwrap().schema_source;
    assert!(compiled::source_matches(source.as_bytes()));
    assert!(!compiled::source_matches(b""));
    let mut changed = source.as_bytes().to_vec();
    changed[0] ^= 1;
    assert!(!compiled::source_matches(&changed));
}

#[test]
fn generated_json_contracts_refuse_edited_bindings_without_rewriting_them() {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/validation/json/builtin_contracts.rs");
    let emitter = env!("CARGO_BIN_EXE_cem-ml-json-contracts-emit");
    assert!(std::process::Command::new(emitter)
        .arg("--check")
        .arg(&source)
        .output()
        .unwrap()
        .status
        .success());
    let original = std::fs::read_to_string(&source).unwrap();
    let changed = original.replacen(
        "diagnostic_code: \"cem.json.parse_error\"",
        "diagnostic_code: \"forged\"",
        1,
    );
    assert_ne!(changed, original);
    let temporary = std::env::temp_dir().join(format!(
        "cem-json-contracts-{}-{}.rs",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&temporary, &changed).unwrap();
    let refused = std::process::Command::new(emitter)
        .arg("--check")
        .arg(&temporary)
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert_eq!(std::fs::read_to_string(&temporary).unwrap(), changed);
    assert_eq!(std::fs::read_to_string(source).unwrap(), original);
    std::fs::remove_file(temporary).unwrap();
}

#[test]
fn compiled_json_contracts_preserve_diagnostics_and_original_ranges() {
    let canonical = JsonSchemaContractCatalog::from_builtin();
    let selected = compiled::catalog();
    for (source, mime, expected) in [
        (
            "{\r\n\"literal\":\"Café 🧬 {$never()}\",\"number\":1.2300e+04\r\n}".as_bytes(),
            "application/json",
            None,
        ),
        (
            b"{\"x\":1,\"x\":2}".as_slice(),
            "application/json",
            Some("cem.json.duplicate_member_name"),
        ),
        (
            b"{\r\n\"x\":[1,}",
            "application/json",
            Some("cem.json.parse_error"),
        ),
        (
            b"{\"x\":NaN}",
            "application/json",
            Some("cem.json.parse_error"),
        ),
        (
            b"{\"x\":\"\xff\"}",
            "application/json",
            Some("cem.json.unsupported_encoding"),
        ),
        // This grammar boundary consumes UTF-8 bytes; MIME parameters do not transcode them.
        (b"{}", "application/json; charset=utf-16", None),
    ] {
        let report = extract_json_parse_report(JsonSourceValidationRequest {
            bytes: source,
            source_uri: "app://original/data.json",
            content_type: Some(mime),
        });
        let actual = validate_json_parse_report(&report, &selected);
        let original = validate_json_parse_report(&report, &canonical);
        assert_eq!(
            serde_json::to_value(&actual).unwrap(),
            serde_json::to_value(&original).unwrap()
        );
        if let Some(code) = expected {
            assert!(
                actual.iter().any(|item| item.code == code),
                "{mime} {source:?}: {actual:?}"
            );
            assert!(actual
                .iter()
                .all(|item| item.uri.as_deref() == Some("app://original/data.json")));
            assert!(actual
                .iter()
                .any(|item| item.byte_offset.is_some() && item.source_map.is_some()));
        } else {
            assert!(actual.is_empty(), "{actual:?}");
            assert!(report.root.is_some());
        }
    }
}
