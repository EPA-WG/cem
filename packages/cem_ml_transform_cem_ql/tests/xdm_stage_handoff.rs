//! Constructed stage documents cross the native value boundary, never XML text.
use cem_ml::{
    command_service::VirtualResourceV1,
    engine::{CemMlEngine, FormatIdentity, TransformGraphResponse},
    real::RealCemMlEngine,
    transform_config::{
        parse_transform_graph_config, TransformGraphParseRequest, TRANSFORM_CONFIG_SCHEMA_URI,
    },
    transform_graph_request::{
        lower_transform_graph_request, ManifestTransformGraphResourceProvider,
    },
};
use cem_ml_transform_cem_ql::engine_context_with_cem_ql_template_adapter;
use std::collections::BTreeMap;
const CONFIG: &str = "studio://xdm/graphs/build.cem";
const CEM: &str = r#"@doc cem-ml 1
@ns template = "https://cem.dev/ns/template/cem-native/1"
@default template
{param @name=input @required=true}
{template @name=make @visibility=public |
 {element @name=orders @namespace=urn:orders | {element @name=item @namespace=urn:orders | Café 🍒}}
}"#;
const XSLT: &str = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:template name="make"><orders xmlns="urn:orders"><item>Café 🍒</item></orders></xsl:template></xsl:stylesheet>"#;
const PAGE: &str = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0" xmlns:o="urn:orders"><xsl:template name="page" match="/"><p><xsl:value-of select="/o:orders/o:item"/></p></xsl:template></xsl:stylesheet>"#;
fn run(
    producer: &str,
    mime: &str,
    schema: &str,
    target: &str,
    page: &str,
    named: bool,
) -> TransformGraphResponse {
    let entrypoint = if named { "@entrypoint=page" } else { "" };
    let graph = format!(
        r#"{{run | {{import @id=input @src=../data/input.xml @content-type=application/xml |
      {{transform @id=made @src=../templates/make @entrypoint=make @template-content-type="{mime}" @template-schema="{schema}" {target} |
        {{transform @id=page @src=../templates/page.xsl {entrypoint} @template-content-type="application/xslt+xml" |
          {{export @id=html @out=../generated/index.html @content-type=text/html}}
    }} }} }} }}"#
    );
    let parsed = parse_transform_graph_config(TransformGraphParseRequest {
        bytes: graph.into_bytes(),
        identity: FormatIdentity {
            content_type: Some("application/cem".into()),
            schema: Some(TRANSFORM_CONFIG_SCHEMA_URI.into()),
            ..Default::default()
        },
        base_uri: Some(CONFIG.into()),
    })
    .unwrap();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let resource = |text: &str| VirtualResourceV1 {
        bytes: text.as_bytes().to_vec(),
        identity: None,
    };
    let resources = BTreeMap::from([
        ("studio://xdm/data/input.xml".into(), resource("<seed/>")),
        ("studio://xdm/templates/make".into(), resource(producer)),
        ("studio://xdm/templates/page.xsl".into(), resource(page)),
    ]);
    let context = engine_context_with_cem_ql_template_adapter();
    let provider = ManifestTransformGraphResourceProvider::new(CONFIG, &resources);
    let request =
        lower_transform_graph_request(&context, &parsed.graph, &provider, CONFIG, true).unwrap();
    RealCemMlEngine::new().transform_graph(request).unwrap()
}
#[test]
fn constructed_cem_and_xslt_stages_feed_absolute_xpath_without_encoding() {
    for (producer, mime, schema) in [
        (
            CEM,
            "application/vnd.cem.template+cem",
            "https://cem.dev/ns/template/cem-native/1",
        ),
        (
            XSLT,
            "application/xslt+xml",
            "https://cem.dev/ns/transform/xslt/1",
        ),
    ] {
        for named in [true, false] {
            let result = run(
                producer,
                mime,
                schema,
                "@content-type=application/cem @schema=https://cem.dev/ns/cem-ml/1",
                PAGE,
                named,
            );
            assert!(
                result.diagnostics.is_empty(),
                "{mime}/{named}: {:?}",
                result.diagnostics
            );
            assert_eq!(result.artifacts.len(), 1);
            assert_eq!(
                result.artifacts[0].primary.as_str().unwrap(),
                "<p>Café 🍒</p>"
            );
            assert!(!result.artifacts[0].output_spans.is_empty());
        }
    }
}

#[test]
fn encoded_xml_remains_text_and_does_not_silently_import_a_document() {
    let result = run(
        XSLT,
        "application/xslt+xml",
        "https://cem.dev/ns/transform/xslt/1",
        "@content-type=application/xml",
        PAGE,
        true,
    );
    assert!(result.artifacts.is_empty());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        result.diagnostics
    );
}
#[test]
fn empty_and_multi_root_constructed_output_has_one_document_context() {
    let page = PAGE.replace("/o:orders/o:item", "count(/*)");
    let empty = XSLT.replace(
        "<orders xmlns=\"urn:orders\"><item>Café 🍒</item></orders>",
        "",
    );
    let multiple = XSLT.replace("</orders>", "</orders><extra/>");
    for (source, expected) in [(empty, "<p>0</p>"), (multiple, "<p>2</p>")] {
        let result = run(
            &source,
            "application/xslt+xml",
            "https://cem.dev/ns/transform/xslt/1",
            "@content-type=application/cem @schema=https://cem.dev/ns/cem-ml/1",
            &page,
            false,
        );
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.artifacts[0].primary.as_str().unwrap(), expected);
    }
}
