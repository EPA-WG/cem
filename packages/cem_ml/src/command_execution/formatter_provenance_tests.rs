// Included in the native command tests to reuse admitted requests and atomic writer fixtures.
#[test]
fn native_formatter_and_markup_outputs_publish_owned_ranges() {
    let cases = [
        ("application/xml", "https://cem.dev/ns/data/xml/1", "<p title=\"Ready\">Café &amp; tea</p>\r\n", LayerFormat::Xml, "application/xml"),
        ("text/html", "https://cem.dev/ns/data/html/1", "<p title=\"Ready\">Café &amp; tea</p>\r\n", LayerFormat::Html, "text/html"),
        ("application/json", "https://cem.dev/ns/data/json/1", "{\"message\":\"Café & tea\",\"source-id\":\"never://forged\",\"byte-offset\":9999}\r\n", LayerFormat::Json, "application/json"),
        ("text/yaml", "https://cem.dev/ns/data/yaml/1", "message: 'Café & tea'\r\n", LayerFormat::Yaml, "text/yaml"),
        ("text/csv", "https://cem.dev/ns/data/csv/1", "name,note\r\nCafé,\"quoted, tea\"\r\n", LayerFormat::Csv, "text/csv"),
        ("text/css", "https://cem.dev/ns/data/css/1", "/* Café */\r\np { color: red; }\r\n", LayerFormat::Css, "text/css"),
        ("application/vnd.cem.xpath", "https://cem.dev/ns/query/xpath/1", "//item[@name = 'Café']\r\n", LayerFormat::Cem, "application/vnd.cem.xpath"),
        ("text/markdown", "https://cem.dev/ns/data/markdown/1", "# Café\r\n\r\nTea & biscuits.\r\n", LayerFormat::Markdown, "text/markdown"),
        ("application/cem", "https://cem.dev/ns/cem-ml/1", "{p @title=Ready | Café & tea}\r\n", LayerFormat::Html, "text/html"),
        ("application/cem", "https://cem.dev/ns/cem-ml/1", "{p @title=Ready | Café & tea}\r\n", LayerFormat::Xml, "application/xml"),
        ("application/json", "https://cem.dev/ns/data/json/1", "{\"message\":\"Café & tea\"}\r\n", LayerFormat::Yaml, "text/yaml"),
        ("text/yaml", "https://cem.dev/ns/data/yaml/1", "message: 'Café & tea'\r\n", LayerFormat::Json, "application/json"),
        ("text/csv", "https://cem.dev/ns/data/csv/1", "name,note\r\nCafé,\"quoted, tea\"\r\n", LayerFormat::Xml, "application/xml"),
        ("text/vnd.cem.scss", "https://cem.dev/ns/data/scss/1", "$tone: red;\r\np { color: $tone; }\r\n", LayerFormat::Css, "text/css"),
    ];
    for (mime, schema, text, target, output_mime) in cases {
        let mut request = fixture_request(false);
        request.resources.get_mut(DATA_URI).unwrap().bytes = text.as_bytes().to_vec();
        request.resource_versions.get_mut(DATA_URI).unwrap().sha256 = sha256_hex(text.as_bytes());
        request.operation = PortableOperationRequestV1::Convert {
            input_id: "input:0".into(),
            to_format: target,
            preserve_source_offsets: true,
        };
        request.run_plan = parse_normalized_run_plan(NormalizedRunPlanRequest {
            input_records: vec![format!("uri={DATA_URI},contentType={mime},schema={schema}")],
            output_records: vec![format!(
                "input={DATA_URI},dest={OUTPUT_URI},contentType={output_mime}"
            )],
            ..Default::default()
        })
        .unwrap()
        .into();
        let prepared = crate::command_operation::prepare_command_operation_v1(
            &request,
            CommandServiceLimitsV1::default(),
            &EngineContext::default(),
            &capability(),
        )
        .unwrap();
        let product = execute_operation(
            &crate::real::RealCemMlEngine,
            &request,
            prepared,
            &QueryResultExporterRegistry::new(),
            CommandServiceLimitsV1::default(),
        )
        .unwrap_or_else(|_| panic!("native formatter {mime}"));
        assert!(
            !product
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()),
            "{mime}: {:?}",
            product.diagnostics
        );
        assert!(
            !product.source_maps.is_empty(),
            "{mime} -> {output_mime}: native named formatter provenance is missing"
        );
        let artifact = &product.publication_items[0];
        let mut precise = false;
        for map in &product.source_maps {
            let output = map.output_range.expect("typed writer range");
            assert!(output.len > 0 && output.end() <= artifact.bytes.len() as u64);
            let CommandPayloadV1::Inline { value } = &map.source_map else {
                panic!("bounded inline source map")
            };
            for frame in &value.frames {
                assert!(
                    matches!(&frame.transform, TransformKind::SourceBoundary { uri } if uri == DATA_URI)
                );
                let FrameSpan::Single(range) = frame.span else {
                    panic!("original range")
                };
                assert!(range.end() <= text.len() as u64);
                assert!(
                    text.is_char_boundary(range.start as usize)
                        && text.is_char_boundary(range.end() as usize)
                );
                precise |= range.start > 0 && range.len > 0;
            }
        }
        assert!(
            precise,
            "{mime}: do not substitute whole-input metadata for native ranges"
        );
        assert!(artifact.source_map_id.as_ref().is_some_and(|id| product
            .source_maps
            .iter()
            .any(|map| &map.source_map_id == id)));
    }
}

#[test]
fn native_formatter_coloring_binds_final_bytes_and_original_identity() {
    for (mime, schema, text, target) in [
        (
            "application/json",
            "https://cem.dev/ns/data/json/1",
            "{\"message\":\"Café < & tea\"}\r\n",
            LayerFormat::Json,
        ),
        (
            "text/markdown",
            "https://cem.dev/ns/data/markdown/1",
            "# Café\r\n\r\nTea & biscuits.\r\n",
            LayerFormat::Markdown,
        ),
        (
            "application/xml",
            "https://cem.dev/ns/data/xml/1",
            "<p>Café &amp; tea</p>\r\n",
            LayerFormat::Xml,
        ),
    ] {
        for uri in ["studio://first/original", "studio://second/original"] {
            for color in ["ansi-256", "html-css-vars"] {
                let response = crate::real::RealCemMlEngine
                    .convert(ConvertRequest {
                        input: EngineInput {
                            uri: uri.into(),
                            bytes: text.as_bytes().to_vec(),
                            from_format: None,
                            identity: Some(FormatIdentity {
                                content_type: Some(mime.into()),
                                schema: Some(schema.into()),
                                ..Default::default()
                            }),
                            root_scope: Default::default(),
                        },
                        to_format: target,
                        preserve_source_offsets: true,
                        context: EngineContext::default(),
                        target: Some(FormatIdentity {
                            content_type: Some(mime.into()),
                            schema: Some(schema.into()),
                            ..Default::default()
                        }),
                        target_scope: crate::run_config::ScopeConfig {
                            output_color_type: Some(color.into()),
                            ..Default::default()
                        },
                        scheduler_scope_id: 0,
                    })
                    .unwrap();
                assert!(
                    !response
                        .diagnostics
                        .iter()
                        .any(|d| d.severity.is_hard_violation()),
                    "{mime}/{color}: {:?}",
                    response.diagnostics
                );
                let primary = response.primary_bytes.expect("final native bytes");
                let output = std::str::from_utf8(&primary.bytes).unwrap();
                assert!(
                    if color.starts_with("html") {
                        output.contains("<span")
                    } else {
                        output.contains("\u{1b}[")
                    },
                    "{mime}/{color}: {output}"
                );
                let proof = primary.provenance.expect("typed writer witness");
                assert_eq!(proof.source_uri(), uri);
                let spans = proof.admitted_spans(text.as_bytes(), &primary.bytes);
                assert!(!spans.is_empty(), "{mime}/{color}: original writer spans");
                for span in spans {
                    assert!(output.is_char_boundary(span.output_range.start as usize));
                    assert!(output.is_char_boundary(span.output_range.end() as usize));
                    assert!(span.origin.frames.iter().all(|frame| matches!(&frame.transform, TransformKind::SourceBoundary { uri: origin } if origin == uri)));
                }
                assert!(proof.admitted_spans(b"changed", &primary.bytes).is_empty());
                let mut changed = primary.bytes.clone();
                changed.push(b' ');
                assert!(
                    proof.admitted_spans(text.as_bytes(), &changed).is_empty(),
                    "color wrappers and trailing bytes are part of the witness"
                );
            }
        }
    }
}
