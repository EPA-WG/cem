use super::*;

const SOURCE: &str =
    "(: Café (: 茶 :) :)\r\n/Q{urn:catalog}catalog/Q{urn:catalog}book[@stock >= 4]/@title";
const URI: &str = "studio://queries/original.xpath";

fn source(text: &[u8]) -> EngineInput {
    EngineInput {
        uri: URI.into(),
        bytes: text.to_vec(),
        identity: Some(FormatIdentity {
            content_type: Some(XPATH_CONTENT_TYPE.into()),
            schema: Some(XPATH_SCHEMA_URI.into()),
            ..Default::default()
        }),
        from_format: None,
        root_scope: Default::default(),
    }
}

fn presentation() -> ScopeConfig {
    ScopeConfig {
        cemt_formatter_profile: Some("tabular".into()),
        output_color_type: Some("none".into()),
        ..Default::default()
    }
}

#[test]
fn xpath_source_parse_and_inspect_use_native_syntax_and_original_token_ranges() {
    let engine = RealCemMlEngine;
    for projection in [ParseProjection::Ast, ParseProjection::Events] {
        let response = engine
            .parse(ParseRequest {
                input: source(SOURCE.as_bytes()),
                projection,
                fail_level: FailLevel::Parse,
                preserve_source_offsets: true,
                presentation_scope: Some(presentation()),
                context: EngineContext::default(),
            })
            .unwrap();
        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        let bytes = response.primary_bytes.unwrap();
        assert_eq!(bytes.content_type, CEM_ML_CONTENT_TYPE);
        let text = String::from_utf8(bytes.bytes).unwrap();
        assert!(text.starts_with("@doc cem-ml 1\n"), "{text}");
        assert!(text.contains(XPATH_SCHEMA_URI), "{text}");
        assert!(text.contains("Q{urn:catalog}catalog"), "{text}");
        assert!(text.contains("path-expression"), "{text}");
        assert!(!text.contains("\"kind\":"), "{text}");
    }
    let input = source(SOURCE.as_bytes());
    let loaded = load_document_input(&input, &EngineContext::default());
    let LoadedInputAstStream::XPathExpression(owner) = loaded.ast_stream.unwrap() else {
        panic!("XPath native owner")
    };
    let inspection = load(&input, &EngineContext::default()).expect("native source inspection");
    assert_eq!(inspection.offsets.len(), owner.tokens.len());
    for (offset, token) in inspection.offsets.iter().zip(&owner.tokens) {
        assert_eq!(offset.start, token.source_range.start.byte_offset);
        assert_eq!(u64::from(offset.len), token.source_range.byte_length);
        assert_eq!(
            &SOURCE[offset.start as usize..offset.start as usize + offset.len as usize],
            token.lexeme
        );
    }
    for view in [
        InspectView::Ast,
        InspectView::Events,
        InspectView::Tree,
        InspectView::Summary,
        InspectView::SourceOffsets,
        InspectView::Diagnostics,
    ] {
        let response = engine
            .inspect(InspectRequest {
                input: input.clone(),
                show: view,
                presentation_scope: Some(presentation()),
                context: EngineContext::default(),
            })
            .unwrap();
        assert_eq!(
            response.primary_bytes.as_ref().unwrap().content_type,
            CEM_ML_CONTENT_TYPE
        );
        if view == InspectView::Summary {
            assert_eq!(response.body["nodes"], owner.tokens.len());
        }
        if view == InspectView::SourceOffsets {
            assert_eq!(
                response.body["offsets"].as_array().unwrap().len(),
                owner.tokens.len()
            );
        }
    }
}

#[test]
fn xpath_source_errors_never_reenter_cem_or_publish_a_dom() {
    for (bytes, code) in [
        (b"/catalog/`book".as_slice(), "cem.xpath.lexical_error"),
        (
            b"/catalog/ns:book".as_slice(),
            "cem.xpath.unknown_namespace_prefix",
        ),
        (
            b"doc('secret.xml')".as_slice(),
            "cem.xpath.external_resource_denied",
        ),
        (b"/catalog/\xff".as_slice(), "cem.xpath.invalid_utf8"),
    ] {
        let response = RealCemMlEngine
            .parse(ParseRequest {
                input: source(bytes),
                projection: ParseProjection::Ast,
                fail_level: FailLevel::Parse,
                preserve_source_offsets: true,
                presentation_scope: None,
                context: EngineContext::default(),
            })
            .unwrap();
        assert!(
            response
                .diagnostics
                .iter()
                .any(|d| d.code == code && d.severity.is_hard_violation()),
            "{code}: {:?}",
            response.diagnostics
        );
        assert!(
            response
                .diagnostics
                .iter()
                .all(|d| !d.code.starts_with("cem.token") && !d.code.starts_with("cem.parser")),
            "{:?}",
            response.diagnostics
        );
        assert!(
            response
                .primary
                .as_array()
                .is_some_and(|nodes| nodes.is_empty()),
            "{:?}",
            response.primary
        );
    }
    let response = RealCemMlEngine
        .parse(ParseRequest {
            input: source(SOURCE.as_bytes()),
            projection: ParseProjection::DomJson,
            fail_level: FailLevel::Parse,
            preserve_source_offsets: true,
            presentation_scope: None,
            context: EngineContext::default(),
        })
        .unwrap();
    assert!(response.primary.is_null());
    assert!(response
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.source.dom_projection_unsupported"));
}

#[test]
fn xpath_source_events_have_one_monotonic_sequence_and_original_native_indexes() {
    use crate::projection::CemTreeAstNode;
    let inspection = load(&source(SOURCE.as_bytes()), &EngineContext::default()).unwrap();
    let events = inspection.events.unwrap();
    let CemTreeAstNode::Element { children, .. } = events.as_nodes().last().unwrap() else {
        panic!("event root")
    };
    let mut lexical = 0usize;
    let mut syntax = 0usize;
    for (sequence, node) in children.iter().enumerate() {
        let CemTreeAstNode::Element {
            attributes, source, ..
        } = node
        else {
            panic!("event")
        };
        let value = |name: &str| {
            attributes
                .iter()
                .find(|attr| attr.name == name)
                .and_then(|attr| attr.value.as_deref())
                .unwrap()
        };
        assert_eq!(value("sequence").parse::<usize>().unwrap(), sequence);
        let original = match value("event-domain") {
            "lexical" => &mut lexical,
            "syntax" => &mut syntax,
            _ => panic!("native domain"),
        };
        assert_eq!(
            value("native-sequence").parse::<usize>().unwrap(),
            *original
        );
        *original += 1;
        assert!(source.frames.iter().any(|frame| matches!(&frame.transform, crate::source_map::TransformKind::SourceBoundary { uri, .. } if uri == URI)));
    }
    assert!(lexical > 0 && syntax > 0);
}

#[test]
fn xpath_source_projections_conform_to_the_shared_field_declarations() {
    use crate::projection::{CemTreeAstNode, CemTreeAstStream};
    use crate::schema::document_model::{compile_schema_document_model, SchemaDocumentModel};
    use crate::schema::package_sources::builtin_schema_package_source;
    fn check(nodes: &[CemTreeAstNode], model: &SchemaDocumentModel) {
        for node in nodes {
            let CemTreeAstNode::Element {
                name,
                attributes,
                children,
                ..
            } = node
            else {
                continue;
            };
            let element = model
                .elements
                .get(name)
                .unwrap_or_else(|| panic!("undeclared element {name}"));
            for required in &element.required_attributes {
                assert!(
                    attributes.iter().any(|attr| &attr.name == required),
                    "{name}: missing {required}"
                );
            }
            for attr in attributes {
                assert!(
                    element.allows_attribute("", &attr.name),
                    "{name}: undeclared {}",
                    attr.name
                );
                let field = model
                    .attributes
                    .get(&attr.name)
                    .unwrap_or_else(|| panic!("undeclared field {}", attr.name));
                if !field.allowed_values.is_empty() {
                    assert!(
                        field
                            .allowed_values
                            .contains(attr.value.as_deref().unwrap()),
                        "{}: {:?}",
                        attr.name,
                        attr.value
                    );
                }
            }
            check(children, model);
        }
    }
    let inspection = load(&source(SOURCE.as_bytes()), &EngineContext::default()).unwrap();
    for (package, uri, stream) in [
        (
            "cem-ast-projection",
            CEM_AST_PROJECTION_SCHEMA_URI,
            inspection.ast.unwrap(),
        ),
        (
            "cem-events-projection",
            CEM_EVENTS_PROJECTION_SCHEMA_URI,
            inspection.events.unwrap(),
        ),
    ] {
        let source = builtin_schema_package_source(package).unwrap();
        let model = compile_schema_document_model(uri, source.schema_source);
        assert!(
            model
                .compile_diagnostics
                .iter()
                .all(|d| !d.severity.is_hard_violation()),
            "{:?}",
            model.compile_diagnostics
        );
        let mut stream: CemTreeAstStream = stream;
        stream.retain_non_directive_nodes();
        check(stream.as_nodes(), &model);
    }
}
