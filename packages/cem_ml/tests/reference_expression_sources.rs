//! Expression byte provenance stays retained across parsing, imports and codecs.
use cem_ml::{
    ast::{decode::DebugBinaryDecoder, encode::DebugBinaryEncoder},
    events::cem::CemEventNormalizer,
    import::import_data_bytes,
    parser::{builder::CemAstBuilder, document::CemDocument, CemAstNode},
    source::{ByteRange, BytesSource, SourceId},
    source_map::{map_expression_range, SourceMapStack},
    tokenizer::cem::CemTokenizer,
};
fn parse(text: &str) -> CemDocument {
    let doc = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), text.as_bytes().to_vec()),
    )))
    .build();
    assert!(doc.diagnostics.is_empty(), "{:?}", doc.diagnostics);
    doc
}
fn reference(doc: &CemDocument) -> (&str, &SourceMapStack) {
    doc.nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Reference {
                expression, source, ..
            } => Some((expression.as_str(), source)),
            _ => None,
        })
        .unwrap()
}
#[test]
fn trimmed_cem_reference_and_native_attribute_payloads_keep_exact_byte_maps() {
    for text in [
        "{section | {#library}}",
        "{section | {$   #library   }}",
        "{section @target={  #library  }}",
        "{section | {$  #native:call(\"é\", library) }}",
    ] {
        let doc = parse(text);
        let (expression, source) = reference(&doc);
        let offset = expression.find("library").unwrap() as u64;
        assert_eq!(
            map_expression_range(source, ByteRange::new(offset, 7)),
            vec![(
                SourceId(1),
                ByteRange::new(text.find("library").unwrap() as u64, 7)
            )]
        );
        let decoded = DebugBinaryDecoder::new()
            .decode(&DebugBinaryEncoder::new().encode(&doc).bytes)
            .unwrap();
        assert_eq!(reference(&decoded), (expression, source));
        assert_eq!(
            map_expression_range(
                reference(&decoded).1,
                ByteRange::new(expression.len() as u64, 0)
            ),
            vec![(
                SourceId(1),
                ByteRange::new(
                    text.find(expression).unwrap() as u64 + expression.len() as u64,
                    0
                )
            )]
        );
    }
}
#[test]
fn xml_entity_and_split_cdata_segments_survive_codec_without_xml_interpretation() {
    let text =
        "<r:expr xmlns:r='https://cem.dev/ns/cem-ml/1'> \r\n#library[&#64;<![CDATA[x]]>]</r:expr>";
    let tree = import_data_bytes(text.as_bytes(), "application/xml", "cem", "source.xml").unwrap();
    let (expression, source) = reference(tree.ast());
    assert_eq!(expression, "#library[@x]");
    let entity = expression.find('@').unwrap() as u64;
    assert_eq!(
        map_expression_range(source, ByteRange::new(entity, 1)),
        vec![(
            SourceId(1),
            ByteRange::new(text.find("&#64;").unwrap() as u64, 5)
        )]
    );
    assert_eq!(
        map_expression_range(source, ByteRange::new(entity, 2)),
        vec![
            (
                SourceId(1),
                ByteRange::new(text.find("&#64;").unwrap() as u64, 5)
            ),
            (
                SourceId(1),
                ByteRange::new(text.find("<![CDATA[x").unwrap() as u64 + 9, 1)
            ),
        ]
    );
    let decoded = DebugBinaryDecoder::new()
        .decode(&DebugBinaryEncoder::new().encode(tree.ast()).bytes)
        .unwrap();
    assert_eq!(reference(&decoded), (expression, source));
}
#[test]
fn normalized_xml_newlines_map_to_authored_crlf_without_shifting_later_bytes() {
    let text = "<r:expr xmlns:r='https://cem.dev/ns/cem-ml/1'>#library[<![CDATA[\r\n@]]]></r:expr>";
    let tree = import_data_bytes(text.as_bytes(), "application/xml", "cem", "source.xml").unwrap();
    let (expression, source) = reference(tree.ast());
    assert_eq!(expression, "#library[\n@]");
    assert_eq!(
        map_expression_range(
            source,
            ByteRange::new(expression.find('\n').unwrap() as u64, 1)
        ),
        vec![(
            SourceId(1),
            ByteRange::new(text.find("\r\n").unwrap() as u64, 2)
        )]
    );
    assert_eq!(
        map_expression_range(
            source,
            ByteRange::new(expression.find('@').unwrap() as u64, 1)
        ),
        vec![(
            SourceId(1),
            ByteRange::new(text.find('@').unwrap() as u64, 1)
        )]
    );
    assert!(
        map_expression_range(source, ByteRange::new(expression.len() as u64 + 1, 1)).is_empty()
    );
    assert!(map_expression_range(&SourceMapStack::default(), ByteRange::new(0, 1)).is_empty());
}
