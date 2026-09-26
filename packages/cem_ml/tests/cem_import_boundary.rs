//! CEM-IMPORT-BOUNDARY: source decoding cannot creep back into tree consumers.
#[test]
fn query_consumers_do_not_interpret_external_document_formats() {
    for (name, source) in [
        (
            "XPath evaluator",
            include_str!("../src/validation/xpath.rs"),
        ),
        (
            "XPath nodes",
            include_str!("../src/validation/xpath/node.rs"),
        ),
        (
            "XPath text",
            include_str!("../src/validation/xpath/text.rs"),
        ),
        (
            "XPath containers",
            include_str!("../src/validation/xpath/containers.rs"),
        ),
        (
            "XPath sequences",
            include_str!("../src/validation/xpath/sequence.rs"),
        ),
        (
            "XPath aggregates",
            include_str!("../src/validation/xpath/aggregate.rs"),
        ),
        (
            "XPath inline functions",
            include_str!("../src/validation/xpath/functions.rs"),
        ),
        (
            "XPath sorting",
            include_str!("../src/validation/xpath/sort.rs"),
        ),
        (
            "XPath string parsing",
            include_str!("../src/validation/xpath/parsing.rs"),
        ),
        ("XPath source metadata", include_str!("../src/validation/xpath/provenance.rs")),
        (
            "XPath regular expressions",
            include_str!("../src/validation/xpath/regular_expression.rs"),
        ),
        ("native result construction", include_str!("../../cem_ql/src/render/construction.rs")),
        ("query runtime", include_str!("../src/query/runtime.rs")),
        ("semantic CEM tree", include_str!("../src/parser/tree.rs")),
        ("typed source inspection", include_str!("../src/projection/inspection.rs")),
        (
            "CEM-QL reader",
            include_str!("../../cem_ql/src/eval/data.rs"),
        ),
        ("CEM-QL string import", include_str!("../../cem_ql/src/eval/data/string_import.rs")),
        (
            "CEM-QL reader retention",
            include_str!("../../cem_ql/src/eval/data/xpath_view.rs"),
        ),
        (
            "CEM-QL XPath binding",
            include_str!("../../cem_ql/src/xpath/functions.rs"),
        ),
        (
            "CEM-QL lifecycle ingress",
            include_str!("../../cem_ml_transform_cem_ql/src/lib.rs")
                .split("fn artifact_query_stream")
                .nth(1)
                .unwrap()
                .split("pub struct CemQlQueryAstOwner")
                .next()
                .unwrap(),
        ),
    ] {
        let production = source.split("\n#[cfg(test)]\nmod tests").next().unwrap();
        for forbidden in [
            "LoadedInputAstStream::",
            "XmlDocumentAst",
            "JsonDocumentAst",
            "YamlDocumentAst",
            "CsvDocumentAst",
            "CssDocumentAst",
            "CssEventAst",
            "JsonValueAst",
            "JsonNumberKind",
            "YamlNodeAst",
            "XmlEventKind",
            "xml_decode_entity_reference",
            "to_generic_data_ast",
            "_document_ast_from_source_bytes",
            "validation::xml",
            "validation::json",
            "validation::yaml",
            "validation::csv",
        ] {
            assert!(
                !production.contains(forbidden),
                "{name} contains {forbidden}; resolve external syntax in cem_ml::import"
            );
        }
    }
}

#[test]
fn browser_http_transports_only_bytes_and_control_metadata() {
    let runtime = include_str!("../../cem-elements/src/lib/cem-elements.ts");
    let loader = runtime
        .split("private async runHttpRequestResource")
        .nth(1)
        .unwrap()
        .split("private httpRequestEnvelope")
        .next()
        .unwrap();
    assert!(loader.contains("host.document("));
    for forbidden in [
        "JSON.parse",
        "JSON.stringify",
        "DOMParser",
        "TextDecoder",
        "response.json(",
        "xmlElementToRecord",
    ] {
        assert!(
            !loader.contains(forbidden),
            "HTTP loader contains {forbidden}"
        );
    }
    assert!(!runtime.contains("parseHttpResourceData"));
    for demo in [
        include_str!("../../cem-elements/demo/http-request.html"),
        include_str!("../../cem-elements/demo/for-each.html"),
        include_str!("../../cem-elements/demo/npm-versions-demo.html"),
        include_str!("../../custom-element/demo/http-request.html"),
        include_str!("../../custom-element/demo/npm-versions-demo.html"),
    ] {
        for forbidden in [
            ".data.results",
            ".data.items",
            ".data.versions",
            "JSON.parse",
            "response.json(",
        ] {
            assert!(!demo.contains(forbidden), "HTTP demo contains {forbidden}");
        }
    }
}
