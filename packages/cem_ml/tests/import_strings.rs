//! XSLT-DATA-IMPORT: standard string inputs resolve only at CEM import.
use cem_ml::import::{
    import_string, ImportFailureKind, ImportStringProfile, ImportStringRequest, JsonXmlDuplicates,
    JsonXmlProjectionOptions, MAX_BYTES,
};
use cem_ml::validation::xpath::XPathNativeNode;

fn import(
    source: &str,
    profile: ImportStringProfile,
) -> Result<XPathNativeNode, cem_ml::import::ImportFailure> {
    import_string(ImportStringRequest {
        source,
        source_uri: "memory:parsed-input",
        base_uri: Some("https://example.test/styles/view.xslt"),
        profile,
    })
    .map(XPathNativeNode::cem_document)
}

#[test]
fn xml_string_declaration_is_not_a_transport_encoding() {
    let root = import(
        "<?xml version='1.0' encoding='UTF-16'?><r>a<![CDATA[b]]>&amp;</r>",
        ImportStringProfile::Xml,
    )
    .unwrap();
    assert_eq!(root.string_value(), "ab&");
    assert!(root.source_owner().is_some());
    assert_eq!(
        root.owner().base_uri(),
        Some("https://example.test/styles/view.xslt")
    );
    assert_eq!(root.owner().document_uri(), None);
    for source in [
        "",
        "<a/><b/>",
        "<missing:r/>",
        "<r>",
        "<r>&#0;</r>",
        "<r>\u{0}</r>",
        "<1/>",
        "<r a='<'/>",
    ] {
        let error = import(source, ImportStringProfile::Xml).unwrap_err();
        assert_eq!(error.kind, ImportFailureKind::Malformed);
        assert!(!error.diagnostics.is_empty());
    }
}

#[test]
fn json_string_profile_handles_bom_surrogates_and_large_number_lexemes() {
    let options = JsonXmlProjectionOptions::default();
    let root = import(
        "\u{feff}[\"\\uDEAD\",\"\\uD83D\\uDE00\",1e999]",
        ImportStringProfile::JsonXml(options),
    )
    .unwrap();
    assert_eq!(root.string_value(), "�😀1e999");
    assert!(root.source_owner().is_some());
    let escaped = import(
        r#"["\uDEAD","\u0000","\\","\n"]"#,
        ImportStringProfile::JsonXml(JsonXmlProjectionOptions {
            escape: true,
            ..options
        }),
    )
    .unwrap();
    assert_eq!(escaped.string_value(), r#"\udead\u0000\\\n"#);
    // The ordinary data reader keeps its existing strict input profile.
    assert!(cem_ml::import::import_data(r#""\uDEAD""#, "json", "cem", "memory:strict").is_err());
}

#[test]
fn typed_import_failures_distinguish_malformed_duplicates_limits_and_capabilities() {
    let json = ImportStringProfile::JsonXml(JsonXmlProjectionOptions::default());
    assert_eq!(
        import("[", json).unwrap_err().kind,
        ImportFailureKind::Malformed
    );
    assert_eq!(
        import(&format!("\"{}\"", "x".repeat(MAX_BYTES)), json)
            .unwrap_err()
            .kind,
        ImportFailureKind::Limit
    );
    assert_eq!(
        import(&format!("{}0{}", "[".repeat(65), "]".repeat(65)), json)
            .unwrap_err()
            .kind,
        ImportFailureKind::Limit
    );
    assert_eq!(
        import("<!DOCTYPE r><r/>", ImportStringProfile::Xml)
            .unwrap_err()
            .kind,
        ImportFailureKind::Unsupported
    );
    let reject = ImportStringProfile::JsonXml(JsonXmlProjectionOptions {
        duplicates: JsonXmlDuplicates::Reject,
        ..Default::default()
    });
    assert_eq!(
        import(r#"{"a":1,"\u0061":2}"#, reject).unwrap_err().kind,
        ImportFailureKind::DuplicateKey
    );
    // Unpaired surrogate identity survives the fallback replacement character.
    assert!(import(r#"{"\uDEAD":1,"�":2}"#, reject).is_ok());
}
