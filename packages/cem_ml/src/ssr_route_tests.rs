use super::*;
use crate::source_map::{SourceMapIndex, TransformKind};
use crate::validation::html::{html_document_ast_from_source_bytes, HtmlSourceValidationRequest};

fn document(markup: &str) -> HtmlDocumentAst {
    html_document_ast_from_source_bytes(HtmlSourceValidationRequest {
        bytes: markup.as_bytes(),
        source_uri: "file:///source/guide.html",
        content_type: Some("text/html"),
    })
    .0
    .expect("native HTML owner")
}

const PAGE: &str = "https://ssr.example.test/guide/page.html?edition=2";

#[test]
fn ssr_route_matches_shared_browser_url_vectors_without_mutating_the_owner() {
    // Explicit JSON test-control boundary, not an AST handoff.
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../tests/fixtures/ssr-route-urls.json")).unwrap();
    for vector in vectors.as_array().unwrap() {
        let doc = document(vector["html"].as_str().unwrap());
        let original = doc.clone();
        let result = preflight_ssr_page_routes(&[SsrPageRouteRequest::new(&doc, PAGE, None)]);
        if let Some(code) = vector["nativeError"].as_str() {
            let errors = result.unwrap_err();
            assert_eq!(errors[0].code, code);
            assert_eq!(doc, original);
            continue;
        }
        let routes = result.unwrap();
        assert_eq!(
            routes[0].document_base_url(),
            vector["base"].as_str().unwrap(),
            "{}",
            vector["name"]
        );
        assert_eq!(routes[0].source_uri(), "file:///source/guide.html");
        assert_eq!(routes[0].page_url(), PAGE);
        assert!(std::ptr::eq(routes[0].source_document(), &doc));
        assert_eq!(doc, original);
    }
}

#[test]
fn ssr_route_separates_source_page_base_output_and_keeps_original_maps() {
    let doc =
        document("<!doctype html><html><head><base href='../assets/'></head><body></body></html>");
    let attr = &doc
        .events
        .iter()
        .find(|event| event.local_name.as_deref() == Some("base"))
        .unwrap()
        .attributes[0];
    let config_map = attr
        .source_range
        .source_map()
        .with_source_uri("file:///config/routes.cem");
    let mut first = SsrPageRouteRequest::new(
        &doc,
        "HTTPS://SSR.EXAMPLE.TEST:443/a/../guide/?v=2",
        Some("file:///dist/a/../guide/index.html"),
    );
    first.page_url_source_map = Some(&config_map);
    first.output_uri_source_map = Some(&config_map);
    let routes = preflight_ssr_page_routes(&[first]).unwrap();
    let route = &routes[0];
    assert_eq!(route.page_url(), "https://ssr.example.test/guide/?v=2");
    assert_eq!(
        route.document_base_url(),
        "https://ssr.example.test/assets/"
    );
    assert_eq!(route.output_uri(), Some("file:///dist/guide/index.html"));
    assert_eq!(
        route.base_source_map(),
        doc.source_map_for_range(attr.source_range).as_ref()
    );
    assert_eq!(route.page_url_source_map(), Some(&config_map));
    assert_eq!(route.output_uri_source_map(), Some(&config_map));
    assert!(matches!(
        route.base_selection(),
        SsrBaseSelection::HtmlBase {
            used_fallback: false,
            ..
        }
    ));
    first.output_uri = Some("memory:second-output");
    let changed = preflight_ssr_page_routes(&[first]).unwrap();
    assert_eq!(
        route.render_url_identity(),
        changed[0].render_url_identity()
    );
    assert_ne!(route.output_uri(), changed[0].output_uri());
}

#[test]
fn ssr_route_preserves_trailing_slash_and_query_as_distinct_page_identities() {
    let doc = document("<html><head></head><body></body></html>");
    let urls = [
        "https://ssr.example.test/guide",
        "https://ssr.example.test/guide/",
        "https://ssr.example.test/guide/?v=1",
        "https://ssr.example.test/guide/?v=2",
    ];
    let requests: Vec<_> = urls
        .iter()
        .map(|url| SsrPageRouteRequest::new(&doc, url, None))
        .collect();
    let routes = preflight_ssr_page_routes(&requests).unwrap();
    assert_eq!(routes.len(), 4);
    for (route, url) in routes.iter().zip(urls) {
        assert_eq!(route.page_url(), url);
    }
}

#[test]
fn ssr_route_refuses_missing_relative_fragmented_credential_or_disallowed_page_urls() {
    let doc = document("<html></html>");
    for url in [
        "",
        "guide/",
        "/guide/",
        "//ssr.example.test/guide",
        "https://ssr.example.test/#",
        "https://ssr.example.test/#part",
        "file:///guide.html",
        "javascript:alert(1)",
        "https://user:secret@ssr.example.test/",
        " https://ssr.example.test/",
        "http://[",
    ] {
        let errors =
            preflight_ssr_page_routes(&[SsrPageRouteRequest::new(&doc, url, None)]).unwrap_err();
        assert_eq!(errors[0].code, "cem.ssr.page_url_invalid", "{url}");
        assert!(!serde_json::to_string(&errors).unwrap().contains("secret"));
    }
}

#[test]
fn ssr_route_rejects_duplicate_normalized_pages_and_outputs_atomically() {
    let doc = document("<html></html>");
    for (pages, outputs, code) in [
        (
            [
                "HTTPS://SSR.EXAMPLE.TEST:443/x/../guide/",
                "https://ssr.example.test/guide/",
            ],
            [Some("memory:one"), Some("memory:two")],
            "cem.ssr.page_url_duplicate",
        ),
        (
            [
                "https://ssr.example.test/one",
                "https://ssr.example.test/two",
            ],
            [
                Some("file:///dist/a/../page.html"),
                Some("file:///dist/page.html"),
            ],
            "cem.transform_runtime.duplicate_destination",
        ),
    ] {
        let requests: Vec<_> = (0..2)
            .map(|i| SsrPageRouteRequest::new(&doc, pages[i], outputs[i]))
            .collect();
        let errors = preflight_ssr_page_routes(&requests).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, code);
        assert_eq!(errors[0].details.as_ref().unwrap()["routeIndex"], 1);
        assert_eq!(errors[0].details.as_ref().unwrap()["firstRouteIndex"], 0);
    }
}

#[test]
fn ssr_route_refuses_bad_source_and_output_uris_without_cwd_or_page_fallback() {
    let mut doc = document("<html></html>");
    for uri in [
        "",
        "src/page.html",
        "file:///page.html#part",
        "https://user:secret@source.test/page",
    ] {
        doc.source.uri = uri.into();
        let errors =
            preflight_ssr_page_routes(&[SsrPageRouteRequest::new(&doc, PAGE, None)]).unwrap_err();
        assert_eq!(errors[0].code, "cem.ssr.source_uri_invalid");
        assert!(!serde_json::to_string(&errors).unwrap().contains("secret"));
    }
    doc.source.uri = "memory:authored-page".into();
    for uri in [
        "",
        "dist/page.html",
        "file:///dist/page.html#part",
        "https://user:secret@output.test/page",
    ] {
        let errors = preflight_ssr_page_routes(&[SsrPageRouteRequest::new(&doc, PAGE, Some(uri))])
            .unwrap_err();
        assert_eq!(errors[0].code, "cem.ssr.output_uri_invalid");
        assert!(!serde_json::to_string(&errors).unwrap().contains("secret"));
    }
    let routes =
        preflight_ssr_page_routes(&[SsrPageRouteRequest::new(&doc, PAGE, Some("memory:output"))])
            .unwrap();
    assert_eq!(routes[0].source_uri(), "memory:authored-page");
}

#[test]
fn ssr_route_does_not_fabricate_source_maps_for_unmapped_generated_html() {
    let mut doc = document("<html><head><base href='/assets/'></head></html>");
    doc.origin_spans = Some(SourceMapIndex::new(vec![]));
    let routes = preflight_ssr_page_routes(&[SsrPageRouteRequest::new(&doc, PAGE, None)]).unwrap();
    assert_eq!(routes[0].base_source_map(), None);
}

#[test]
fn ssr_route_error_maps_keep_their_owner_instead_of_using_public_or_output_urls() {
    let doc = document("<html><head><base href='https://user:secret@assets.test/'></head></html>");
    let errors =
        preflight_ssr_page_routes(&[SsrPageRouteRequest::new(&doc, PAGE, Some("memory:out"))])
            .unwrap_err();
    assert_eq!(errors[0].code, "cem.ssr.base_url_invalid");
    assert_eq!(errors[0].uri.as_deref(), Some("file:///source/guide.html"));
    assert!(errors[0]
        .source_map
        .as_ref()
        .unwrap()
        .frames
        .iter()
        .any(|frame| matches!(&frame.transform,
        TransformKind::SourceBoundary { uri } if uri == "file:///source/guide.html")));
    assert!(!serde_json::to_string(&errors).unwrap().contains("secret"));
    let map = doc.events[0]
        .source_range
        .source_map()
        .with_source_uri("memory:config");
    let mut request = SsrPageRouteRequest::new(&doc, "/bad-page", None);
    request.page_url_source_map = Some(&map);
    let errors = preflight_ssr_page_routes(&[request]).unwrap_err();
    assert_eq!(errors[0].source_map.as_ref(), Some(&map));
}

#[test]
fn ssr_route_requires_host_decision_for_policy_dependent_base_and_respects_blocking() {
    let doc = document("<html><head><meta http-equiv='Content-Security-Policy' content=\"base-uri 'none'\"><base href='/assets/'></head></html>");
    let mut request = SsrPageRouteRequest::new(&doc, PAGE, None);
    let errors = preflight_ssr_page_routes(&[request]).unwrap_err();
    assert_eq!(errors[0].code, "cem.ssr.base_policy_required");
    request.base_policy = SsrBasePolicy::Blocked;
    let routes = preflight_ssr_page_routes(&[request]).unwrap();
    assert_eq!(routes[0].document_base_url(), PAGE);
    assert!(matches!(
        routes[0].base_selection(),
        SsrBaseSelection::HtmlBase {
            used_fallback: true,
            ..
        }
    ));
    request.base_policy = SsrBasePolicy::Allowed;
    let routes = preflight_ssr_page_routes(&[request]).unwrap();
    assert_eq!(
        routes[0].document_base_url(),
        "https://ssr.example.test/assets/"
    );
}

#[test]
fn ssr_route_empty_batch_and_hard_batch_limit() {
    assert!(preflight_ssr_page_routes(&[]).unwrap().is_empty());
    let doc = document("<html></html>");
    let request = SsrPageRouteRequest::new(&doc, PAGE, None);
    let errors = preflight_ssr_page_routes(&vec![request; SSR_ROUTE_BATCH_LIMIT + 1]).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "cem.ssr.route_limit");
    let urls: Vec<_> = (0..SSR_ROUTE_BATCH_LIMIT)
        .map(|index| format!("https://ssr.example.test/page/{index}/"))
        .collect();
    let requests: Vec<_> = urls
        .iter()
        .map(|url| SsrPageRouteRequest::new(&doc, url, None))
        .collect();
    assert_eq!(
        preflight_ssr_page_routes(&requests).unwrap().len(),
        SSR_ROUTE_BATCH_LIMIT
    );
}

#[test]
fn ssr_route_keeps_resolver_policy_and_local_read_maps_in_charge() {
    use crate::resolver::{
        ResolveDirection, ResolvePolicyDenial, ResolvePolicyRequestKey, ResolvePolicySubstitution,
        ResolvePurpose, ResolverPolicy,
    };
    let doc = document("<html><head><base href='../assets/'></head></html>");
    let routes = preflight_ssr_page_routes(&[SsrPageRouteRequest::new(
        &doc,
        PAGE,
        Some("file:///dist/index.html"),
    )])
    .unwrap();
    let route = &routes[0];
    let identity = route.render_url_identity();
    let source = route.source_read_request();
    assert_eq!(source.uri, doc.source.uri);
    assert_eq!(source.purpose, ResolvePurpose::Input);
    assert_eq!(source.direction, ResolveDirection::Read);
    assert_eq!(source.base_uri, None);
    let resource = route
        .page_resource_read_request("./card.cem", ResolvePurpose::Template)
        .unwrap();
    assert_eq!(resource.uri, "https://ssr.example.test/assets/card.cem");
    assert_eq!(
        resource.base_uri.as_deref(),
        Some("https://ssr.example.test/assets/")
    );
    let key = ResolvePolicyRequestKey::new(
        &resource.uri,
        ResolvePurpose::Template,
        ResolveDirection::Read,
    );
    let policy = ResolverPolicy::new().with_substitution(
        key.clone(),
        ResolvePolicySubstitution::new("file:///local-only/card.cem", "explicit-read-map"),
    );
    assert_eq!(
        policy.decide(&resource).unwrap().effective_uri,
        "file:///local-only/card.cem"
    );
    let denied = policy.with_denial(key, ResolvePolicyDenial::new("not-admitted"));
    assert_eq!(denied.decide(&resource).unwrap_err().reason, "not-admitted");
    assert_eq!(route.render_url_identity(), identity);
    assert_eq!(route.source_uri(), "file:///source/guide.html");
    let output = route.output_write_request().unwrap();
    assert_eq!(output.uri, "file:///dist/index.html");
    assert_eq!(output.purpose, ResolvePurpose::Output);
    assert_eq!(output.direction, ResolveDirection::Write);
    assert_eq!(output.base_uri, None);
}

#[test]
fn ssr_route_retains_transformed_base_origin_without_inventing_generated_offsets() {
    use crate::interpreter::OutputSpan;
    use crate::source::{ByteRange, SourceId};
    use crate::source_map::{FrameSpan, SourceMapFrame};
    for href in ["/assets/", "http://["] {
        let mut doc = document(&format!("<html><head><base href='{href}'></head></html>"));
        let attr = &doc
            .events
            .iter()
            .find(|event| event.local_name.as_deref() == Some("base"))
            .unwrap()
            .attributes[0];
        let origin = SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(7),
                span: FrameSpan::Single(ByteRange::new(102, 9)),
                transform: TransformKind::CemAstBuilder,
            }],
        }
        .with_source_uri("memory:original.cem");
        doc.origin_spans = Some(SourceMapIndex::new(vec![OutputSpan {
            output_range: ByteRange::new(
                attr.source_range.start.byte_offset,
                attr.source_range.byte_length as u32,
            ),
            origin: origin.clone(),
        }]));
        let result = preflight_ssr_page_routes(&[SsrPageRouteRequest::new(&doc, PAGE, None)]);
        if href == "/assets/" {
            let routes = result.unwrap();
            assert_eq!(routes[0].base_source_map(), Some(&origin));
        } else {
            let errors = result.unwrap_err();
            assert_eq!(errors[0].source_map.as_ref(), Some(&origin));
            assert_eq!(errors[0].uri.as_deref(), Some("memory:original.cem"));
            assert_eq!(errors[0].byte_offset, None);
        }
    }
}

#[test]
fn ssr_route_url_identity_changes_with_source_page_or_effective_base() {
    let doc = document("<html><head><base href='/assets/'></head></html>");
    let original =
        preflight_ssr_page_routes(&[SsrPageRouteRequest::new(&doc, PAGE, None)]).unwrap();
    let mut changed_source = doc.clone();
    changed_source.source.uri = "FILE:///source/./other.html".into();
    let other_source =
        preflight_ssr_page_routes(&[SsrPageRouteRequest::new(&changed_source, PAGE, None)])
            .unwrap();
    assert_eq!(other_source[0].source_uri(), "FILE:///source/./other.html");
    assert_eq!(
        other_source[0].source_read_request().uri,
        changed_source.source.uri
    );
    assert_eq!(other_source[0].output_write_request(), None);
    let other_page = preflight_ssr_page_routes(&[SsrPageRouteRequest::new(
        &doc,
        "https://ssr.example.test/another/",
        None,
    )])
    .unwrap();
    let mut blocked = SsrPageRouteRequest::new(&doc, PAGE, None);
    blocked.base_policy = SsrBasePolicy::Blocked;
    let other_base = preflight_ssr_page_routes(&[blocked]).unwrap();
    for other in [&other_source[0], &other_page[0], &other_base[0]] {
        assert_ne!(
            original[0].render_url_identity(),
            other.render_url_identity()
        );
    }
}

#[test]
fn ssr_route_refuses_page_profiles_requiring_unimplemented_browser_tree_or_encoding_state() {
    for markup in [
        "<base href='/fragment/'>",
        "<html><head><template/><base href='/inert/'></head></html>",
        "<html><head><noscript><base href='/depends-on-scripting/'></noscript></head></html>",
        "<html><head><meta charset='windows-1252'><base href='/?q=é'></head></html>",
        "<html><head><div></head></html>",
    ] {
        let doc = document(markup);
        let errors =
            preflight_ssr_page_routes(&[SsrPageRouteRequest::new(&doc, PAGE, None)]).unwrap_err();
        assert_eq!(
            errors[0].code, "cem.ssr.page_profile_unsupported",
            "{markup}"
        );
    }
}
