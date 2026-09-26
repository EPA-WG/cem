use cem_ml::{
    css_imports::{CssImportClosure, CssImportLimits, CssImportResponsePolicy, CssImportState},
    import::import_data,
    module_resolution::{
        CemModuleUrlContext, CemModuleUrlFrame, CemModuleUrlResolutionCapability,
        CemResolutionContextHandle, CemScopedModuleUrlResolver,
    },
    parser::tree::RetainedCemTree,
    scheduler::AbortSignal,
};
use std::sync::Arc;
fn tree(css: &str) -> Arc<RetainedCemTree> {
    import_data(css, "text/css", "cem", "urn:fixture:css").unwrap()
}
fn closure(css: &str, limits: CssImportLimits, abort: AbortSignal) -> CssImportClosure {
    configured_closure(css, limits, abort, None)
}
fn configured_closure(
    css: &str,
    limits: CssImportLimits,
    abort: AbortSignal,
    integrity: Option<&str>,
) -> CssImportClosure {
    closure_with_mapping(
        css,
        limits,
        abort,
        integrity.map(|value| {
            cem_ml::module_resolution::CemModuleUrlMapping::target("./a.css").with_integrity(value)
        }),
    )
}
fn closure_with_mapping(
    css: &str,
    limits: CssImportLimits,
    abort: AbortSignal,
    mapping: Option<cem_ml::module_resolution::CemModuleUrlMapping>,
) -> CssImportClosure {
    try_closure(css, limits, abort, mapping).unwrap()
}
fn try_closure(
    css: &str,
    limits: CssImportLimits,
    abort: AbortSignal,
    mapping: Option<cem_ml::module_resolution::CemModuleUrlMapping>,
) -> Result<CssImportClosure, cem_ml::css_imports::CssImportFailure> {
    let mut frame = CemModuleUrlFrame::new("root", "https://example.test/main.css");
    if let Some(mapping) = mapping {
        frame.specifiers.resources.insert("a.css".into(), mapping);
    }
    frame.allowed_schemes = Some(["https".into()].into());
    let context = CemResolutionContextHandle::new("test");
    let capability = CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(
            context.clone(),
            CemModuleUrlContext {
                identity: "test".into(),
                resolver_identity: "fixture".into(),
                resource_policy_stamp: "https".into(),
                frames: vec![frame],
            },
        )),
        context,
    );
    CssImportClosure::new(
        tree(css),
        "https://example.test/main.css",
        capability,
        limits,
        abort,
    )
}
#[test]
fn css_import_closure_retains_order_conditions_and_final_sheet_bases() {
    let mut c = closure(
        "@import 'a.css' layer(base) screen; @import 'a.css' layer;",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    assert_eq!(c.state(), CssImportState::Pending);
    let a = c.next_import().unwrap().unwrap();
    assert_eq!(a.resolution.resolved_url, "https://example.test/a.css");
    assert!(c.next_import().is_err()); // At most one delivery can be outstanding.
    let shared = tree("@import 'b.css'; a {background:url(icon.svg)}");
    c.complete_import(a.id, shared.clone(), "https://example.test/cdn/a.css")
        .unwrap();
    let b = c.next_import().unwrap().unwrap();
    assert_eq!(b.resolution.resolved_url, "https://example.test/cdn/b.css");
    c.complete_import(b.id, tree("b { color:red }"), &b.resolution.resolved_url)
        .unwrap();
    let again = c.next_import().unwrap().unwrap();
    assert_eq!(again.resolution.resolved_url, a.resolution.resolved_url);
    c.complete_import(again.id, shared.clone(), "https://example.test/cdn/a.css")
        .unwrap();
    let b = c.next_import().unwrap().unwrap();
    c.complete_import(b.id, tree("b {}"), &b.resolution.resolved_url)
        .unwrap();
    assert!(c.next_import().unwrap().is_none());
    assert_eq!(c.state(), CssImportState::Ready);
    let sheets = c.sheets();
    assert_eq!(sheets.len(), 5);
    assert!(Arc::ptr_eq(&sheets[1].resources.tree, &shared));
    assert!(Arc::ptr_eq(&sheets[3].resources.tree, &shared));
    assert_eq!(
        sheets[1].resources.references[1]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://example.test/cdn/icon.svg"
    );
    assert_eq!(
        c.edges().iter().map(|e| e.parent_sheet).collect::<Vec<_>>(),
        [0, 1, 0, 3]
    );
    assert!(
        matches!(&c.edges()[0].conditions, cem_ml::css_resources::CssResourceKind::Import {layer:Some(layer),media:Some(media),..} if layer == "base" && media == "screen")
    );
}
#[test]
fn css_import_closure_rejects_cycles_and_denied_redirects() {
    for url in ["https://example.test/main.css", "http://example.test/a.css"] {
        let mut c = closure(
            "@import 'a.css';",
            CssImportLimits::default(),
            AbortSignal::new(),
        );
        let request = c.next_import().unwrap().unwrap();
        assert!(c.complete_import(request.id, tree("a {}"), url).is_err());
        assert_eq!(c.state(), CssImportState::Failed);
        assert!(c.next_import().is_err());
    }
    let mut c = closure(
        "@import 'main.css#again';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    assert!(c.next_import().is_err());
    assert_eq!(c.state(), CssImportState::Failed);
}
#[test]
fn css_import_closure_bounds_cancellation_and_loader_failures() {
    for limits in [
        CssImportLimits {
            max_sheets: 1,
            ..Default::default()
        },
        CssImportLimits {
            max_depth: 0,
            ..Default::default()
        },
    ] {
        let mut c = closure("@import 'a.css';", limits, AbortSignal::new());
        assert!(c.next_import().is_err());
        assert_eq!(c.state(), CssImportState::Failed);
    }
    let abort = AbortSignal::new();
    let mut c = closure(
        "@import 'a.css';",
        CssImportLimits::default(),
        abort.clone(),
    );
    let r = c.next_import().unwrap().unwrap();
    abort.abort();
    assert_eq!(c.state(), CssImportState::Failed);
    assert_eq!(c.failure().unwrap().code, "cem.css.import_cancelled");
    assert!(c
        .complete_import(r.id, tree("a {}"), &r.resolution.resolved_url)
        .is_err());
    assert_eq!(c.state(), CssImportState::Failed);
    let mut c = closure(
        "@import 'a.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    let r = c.next_import().unwrap().unwrap();
    c.fail_import(r.id, "loader.mime_mismatch", "expected CSS")
        .unwrap();
    assert_eq!(c.state(), CssImportState::Failed);
    assert_eq!(c.failure().unwrap().code, "loader.mime_mismatch");
}

#[test]
fn css_import_closure_rejects_wrong_deliveries_without_losing_the_pending_request() {
    let mut c = closure(
        "@import 'a.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    let r = c.next_import().unwrap().unwrap();
    assert!(c
        .complete_import(r.id + 1, tree("a {}"), &r.resolution.resolved_url)
        .is_err());
    assert!(c.fail_import(r.id + 1, "bad", "wrong delivery").is_err());
    assert_eq!(c.state(), CssImportState::Pending);
    assert_eq!(c.sheets().len(), 1);
    c.complete_import(r.id, tree("a {}"), &r.resolution.resolved_url)
        .unwrap();
    assert_eq!(c.state(), CssImportState::Ready);
    assert!(c
        .complete_import(r.id, tree("a {}"), &r.resolution.resolved_url)
        .is_err());
    assert_eq!(c.sheets().len(), 2);
}

#[test]
fn css_import_closure_rejects_non_css_and_bounds_recursive_imports() {
    let mut c = closure(
        "@import 'a.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    let r = c.next_import().unwrap().unwrap();
    let xml = import_data("<a/>", "application/xml", "cem", "urn:xml").unwrap();
    assert!(c
        .complete_import(r.id, xml, &r.resolution.resolved_url)
        .is_err());
    assert_eq!(c.failure().unwrap().code, "cem.css.import_tree_invalid");
    assert!(c.failure().unwrap().source.origin().is_some());
    assert_eq!(c.sheets().len(), 1);

    let mut wrong_mode = closure(
        "@import 'a.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    let r = wrong_mode.next_import().unwrap().unwrap();
    let declarations = cem_ml::import::import_data_bytes(
        b"color:red",
        "text/css; mode=declaration-list",
        "cem",
        "urn:css:declarations",
    )
    .unwrap();
    assert_eq!(
        wrong_mode
            .complete_import(r.id, declarations, &r.resolution.resolved_url)
            .unwrap_err()
            .code,
        "cem.css.import_tree_invalid"
    );

    for limits in [
        CssImportLimits {
            max_depth: 1,
            ..Default::default()
        },
        CssImportLimits {
            max_sheets: 2,
            ..Default::default()
        },
    ] {
        let mut c = closure("@import 'a.css';", limits, AbortSignal::new());
        let r = c.next_import().unwrap().unwrap();
        c.complete_import(r.id, tree("@import 'b.css';"), &r.resolution.resolved_url)
            .unwrap();
        assert_eq!(c.next_import().unwrap_err().code, "cem.css.import_limit");
        assert_eq!(c.sheets().len(), 2);
    }
    let mut c = closure(
        "@import 'a.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    let r = c.next_import().unwrap().unwrap();
    c.complete_import(
        r.id,
        tree("@import 'a.css';"),
        "https://example.test/redirect.css",
    )
    .unwrap();
    assert_eq!(c.next_import().unwrap_err().code, "cem.css.import_cycle");
}

fn response(bytes: &[u8], mime: Option<&str>) -> cem_ml::resolver::ResolvedRead {
    cem_ml::resolver::ResolvedRead {
        uri: "https://example.test/cdn/a.css".into(),
        bytes: bytes.to_vec(),
        content_type: mime.map(str::to_owned),
    }
}
#[test]
fn css_import_byte_delivery_validates_mime_limits_and_integrity() {
    for (body, mime, policy, expected) in [
        (
            b"a {}".as_slice(),
            Some("text/html"),
            CssImportResponsePolicy::default(),
            "cem.css.import_content_type",
        ),
        (
            b"a {}".as_slice(),
            Some("text/css"),
            CssImportResponsePolicy {
                max_response_bytes: 3,
                ..Default::default()
            },
            "cem.css.import_byte_limit",
        ),
        (
            b"a {}".as_slice(),
            Some("text/css"),
            CssImportResponsePolicy {
                max_total_bytes: 3,
                ..Default::default()
            },
            "cem.css.import_byte_limit",
        ),
        (
            b"a {".as_slice(),
            Some("text/css"),
            CssImportResponsePolicy::default(),
            "cem.css.import_parse_failed",
        ),
    ] {
        let mut c = closure(
            "@import 'a.css';",
            CssImportLimits::default(),
            AbortSignal::new(),
        );
        let r = c.next_import().unwrap().unwrap();
        assert_eq!(
            c.complete_response(r.id, response(body, mime), &policy)
                .unwrap_err()
                .code,
            expected
        );
        assert_eq!(c.state(), CssImportState::Failed);
        assert_eq!(c.sheets().len(), 1);
    }
    let mut c = configured_closure(
        "@import 'a.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
        Some("sha256-invalid"),
    );
    let r = c.next_import().unwrap().unwrap();
    assert!(c
        .complete_response(
            r.id,
            response(b"a {}", Some("text/css")),
            &CssImportResponsePolicy::default()
        )
        .is_err());
    assert_eq!(c.sheets().len(), 1);
}
#[test]
fn css_import_byte_delivery_tracks_aggregate_bytes_and_final_bases() {
    let mut c = closure(
        "@import 'a.css'; @import 'b.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    let r = c.next_import().unwrap().unwrap();
    let body = b"a {background:url(icon.svg)}";
    let policy = CssImportResponsePolicy {
        max_total_bytes: body.len(),
        ..Default::default()
    };
    c.complete_response(r.id, response(body, None), &policy)
        .unwrap();
    assert_eq!(c.received_bytes(), body.len());
    assert_eq!(
        c.sheets()[1].resources.references[0]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://example.test/cdn/icon.svg"
    );
    let r = c.next_import().unwrap().unwrap();
    assert_eq!(
        c.complete_response(r.id, response(b"b {}", None), &policy)
            .unwrap_err()
            .code,
        "cem.css.import_byte_limit"
    );
    assert_eq!(c.sheets().len(), 2);
}

#[test]
fn css_import_byte_integrity_accepts_expected_bytes_and_rejects_changes() {
    let integrity = "sha256-mkSHzL7faOU7/U/v8Umg+058R69+vN2A2xmE3Fz1q98=";
    for (bytes, success) in [(b"a {}".as_slice(), true), (b"b {}".as_slice(), false)] {
        let mut c = configured_closure(
            "@import 'a.css';",
            CssImportLimits::default(),
            AbortSignal::new(),
            Some(integrity),
        );
        let r = c.next_import().unwrap().unwrap();
        let result = c.complete_response(
            r.id,
            response(bytes, Some("Text/CSS; charset=utf-8")),
            &CssImportResponsePolicy::default(),
        );
        assert_eq!(result.is_ok(), success);
        assert_eq!(c.sheets().len(), if success { 2 } else { 1 });
    }
}

#[test]
fn resource_integrity_uses_strongest_digest_and_rejects_unsupported_metadata() {
    use cem_ml::resource_integrity::verify_resource_integrity as verify;
    let sha256 = "sha256-ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=";
    let sha384 = "sha384-ywB1P0WjXou1oD1pmsZQBycsMqsO3tFjGotgWkP/W+2AhgcroefMI1i67KE0yCWn";
    let sha512 = "sha512-3a81oZNherrMQXNJriBBMRLm+k6JqX6iCp7u5ktV05ohkpkqJ0/BqDa6PCOj/uu9RU1EI2Q86A4qmslPpUyknw==";
    let wrong512 = "sha512-sqhF9JAEi5h3ziP48SBnzQnaeei8cf/pfYJBdKL4F7xdu3v5yr71eQ0kCL11/jWRFjLG4TKOudUnS/u6WLMqYw==";
    for digest in [sha256, sha384, sha512, sha256.trim_end_matches('=')] {
        assert!(verify(b"abc", digest).is_ok());
        assert!(verify(b"abcd", digest).is_err());
    }
    assert!(verify(b"abc", &format!("{sha256} {wrong512}")).is_err());
    assert!(verify(b"abc", &format!("{wrong512} {sha512}")).is_ok());
    for digest in ["", "garbage", "sha1-aGVsbG8=", "sha256-%%%", "sha256-YQ=="] {
        assert!(verify(b"abc", digest).is_err());
    }
}

#[derive(Default)]
struct CssTransport {
    requests: std::sync::Mutex<Vec<String>>,
    abort: Option<AbortSignal>,
}
impl cem_ml::resolver::ResourceResolver for CssTransport {
    fn read(
        &self,
        request: &cem_ml::resolver::ResolveRequest,
    ) -> Result<cem_ml::resolver::ResolvedRead, cem_ml::resolver::ResolverDiagnostic> {
        self.requests.lock().unwrap().push(request.uri.clone());
        assert_eq!(request.content_type_hint.as_deref(), Some("text/css"));
        if let Some(abort) = &self.abort {
            abort.abort();
        }
        match request.uri.as_str() {
            "https://example.test/a.css" => {
                Ok(response(b"@import 'b.css'; a {}", Some("text/css")))
            }
            "https://example.test/cdn/b.css" => Ok(cem_ml::resolver::ResolvedRead {
                uri: request.uri.clone(),
                bytes: b"b {}".to_vec(),
                content_type: Some("text/css".into()),
            }),
            _ => Err(cem_ml::resolver::ResolverDiagnostic::Io {
                uri: request.uri.clone(),
                message: "fixture missing".into(),
            }),
        }
    }
    fn write(
        &self,
        _: &cem_ml::resolver::ResolveRequest,
        _: &[u8],
    ) -> Result<cem_ml::resolver::ResolvedWrite, cem_ml::resolver::ResolverDiagnostic> {
        panic!("CSS imports must never write resources")
    }
}

#[test]
fn css_import_shared_resolver_driver_loads_in_order_and_propagates_failure() {
    use cem_ml::resolver::{ResolveDirection, ResolvePurpose, ResolverRegistry};
    let mut registry = ResolverRegistry::new();
    let transport = Arc::new(CssTransport::default());
    registry.register_arc(
        "https",
        ResolvePurpose::Input,
        ResolveDirection::Read,
        transport.clone(),
    );
    let mut c = closure(
        "@import 'a.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    c.load_imports(&registry, &CssImportResponsePolicy::default())
        .unwrap();
    assert_eq!(c.state(), CssImportState::Ready);
    assert_eq!(c.sheets().len(), 3);
    assert_eq!(
        *transport.requests.lock().unwrap(),
        [
            "https://example.test/a.css",
            "https://example.test/cdn/b.css"
        ]
    );
    let mut c = closure(
        "@import 'missing.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    assert_eq!(
        c.load_imports(&registry, &CssImportResponsePolicy::default())
            .unwrap_err()
            .code,
        "cem.resolver.io"
    );
    assert_eq!(c.state(), CssImportState::Failed);
    let abort = AbortSignal::new();
    registry.register(
        "https",
        ResolvePurpose::Input,
        ResolveDirection::Read,
        CssTransport {
            abort: Some(abort.clone()),
            ..Default::default()
        },
    );
    let mut c = closure("@import 'a.css';", CssImportLimits::default(), abort);
    assert!(c
        .load_imports(&registry, &CssImportResponsePolicy::default())
        .is_err());
    assert_eq!(c.sheets().len(), 1);
}

#[test]
fn css_import_response_metadata_fails_before_read_or_parse() {
    use cem_ml::resolver::{ResolveDirection, ResolvePurpose, ResolverRegistry};
    let mut registry = ResolverRegistry::new();
    let transport = Arc::new(CssTransport::default());
    registry.register_arc(
        "https",
        ResolvePurpose::Input,
        ResolveDirection::Read,
        transport.clone(),
    );
    let mut c = closure_with_mapping(
        "@import 'a.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
        Some(
            cem_ml::module_resolution::CemModuleUrlMapping::target("./a.css")
                .with_content_type("text/html"),
        ),
    );
    assert_eq!(
        c.load_imports(&registry, &CssImportResponsePolicy::default())
            .unwrap_err()
            .code,
        "cem.css.import_content_type"
    );
    assert!(transport.requests.lock().unwrap().is_empty());
    let mut c = closure(
        "@import 'a.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    let r = c.next_import().unwrap().unwrap();
    let mut denied = response(b"a {", Some("text/css"));
    denied.uri = "http://example.test/a.css".into();
    assert_eq!(
        c.complete_response(r.id, denied, &CssImportResponsePolicy::default())
            .unwrap_err()
            .code,
        "cem.css.import_redirect_denied"
    );
    assert_eq!(c.received_bytes(), 0);
    assert_eq!(c.sheets().len(), 1);
}

#[test]
fn css_import_placement_preserves_the_top_level_import_prefix() {
    let mut c = closure(
        "@charset 'UTF-8'; /* before */ @layer base, theme; @import 'a.css'; \
         /* between */ @LAYER extra; @import 'b.css'; a {}",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    for path in ["a.css", "b.css"] {
        let request = c.next_import().unwrap().unwrap();
        assert_eq!(
            request.resolution.resolved_url,
            format!("https://example.test/{path}")
        );
        c.complete_import(request.id, tree("a {}"), &request.resolution.resolved_url)
            .unwrap();
    }
    assert_eq!(c.state(), CssImportState::Ready);
}

#[test]
fn css_import_placement_rejects_late_and_nested_imports_before_loading() {
    for css in [
        "a {} @import 'late.css';",
        "@layer base {} @import 'late.css';",
        "@media screen {} @import 'late.css';",
        "@namespace svg 'http://www.w3.org/2000/svg'; @import 'late.css';",
        "a {} @layer base; @import 'late.css';",
        "@import 'first.css'; a {} @import 'late.css';",
        "@media screen { @import 'nested.css'; }",
        "@layer base { @import 'nested.css'; }",
        "a { @import 'nested.css'; }",
    ] {
        let error = try_closure(css, CssImportLimits::default(), AbortSignal::new(), None)
            .err()
            .unwrap_or_else(|| panic!("accepted misplaced import: {css}"));
        assert_eq!(error.code, "cem.css.import_placement_invalid", "{css}");
    }
}

#[test]
fn css_import_placement_failure_does_not_attach_or_queue_a_delivered_sheet() {
    let mut c = closure(
        "@import 'a.css'; @import 'b.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    let request = c.next_import().unwrap().unwrap();
    let error = c
        .complete_import(
            request.id,
            tree("a {} @import 'late.css';"),
            &request.resolution.resolved_url,
        )
        .unwrap_err();
    assert_eq!(error.code, "cem.css.import_placement_invalid");
    assert_eq!(c.state(), CssImportState::Failed);
    assert_eq!(c.sheets().len(), 1);
    assert!(c.edges().is_empty());
    assert!(c.next_import().is_err());
}

#[test]
fn css_import_invalid_layer_response_fails_before_attaching_dependencies() {
    let mut c = closure(
        "@import 'a.css'; @import 'b.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    let request = c.next_import().unwrap().unwrap();
    let error = c
        .complete_response(
            request.id,
            response(b"@import 'child.css' layer();", Some("text/css")),
            &CssImportResponsePolicy::default(),
        )
        .unwrap_err();
    assert_eq!(error.code, "cem.css.import_parse_failed");
    assert!(error.message.contains("layer"));
    assert_eq!(c.state(), CssImportState::Failed);
    assert_eq!(c.sheets().len(), 1);
    assert!(c.edges().is_empty());
    assert_eq!(c.received_bytes(), 0);
    assert!(c.next_import().is_err());
}

#[test]
fn css_import_invalid_supports_response_fails_before_attaching_dependencies() {
    let mut c = closure(
        "@import 'a.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    let request = c.next_import().unwrap().unwrap();
    let error = c
        .complete_response(
            request.id,
            response(
                b"@import 'child.css' supports((display:grid) and);",
                Some("text/css"),
            ),
            &CssImportResponsePolicy::default(),
        )
        .unwrap_err();
    assert_eq!(error.code, "cem.css.import_parse_failed");
    assert!(error.message.contains("supports"));
    assert_eq!(c.state(), CssImportState::Failed);
    assert_eq!(c.sheets().len(), 1);
    assert!(c.edges().is_empty());
    assert_eq!(c.received_bytes(), 0);
    assert!(c.next_import().is_err());
}

#[test]
fn css_import_media_recovery_retains_downloaded_sheet_and_valid_siblings() {
    let mut c = closure(
        "@import 'a.css';",
        CssImportLimits::default(),
        AbortSignal::new(),
    );
    let request = c.next_import().unwrap().unwrap();
    c.complete_response(
        request.id,
        response(b"@import 'child.css' &bad, screen;", Some("text/css")),
        &CssImportResponsePolicy::default(),
    )
    .unwrap();
    let child = c.next_import().unwrap().unwrap();
    assert_eq!(
        child.resolution.resolved_url,
        "https://example.test/cdn/child.css"
    );
    let retained = &c.sheets()[1].resources.tree;
    let queries: Vec<_> = (0..retained.ast().nodes.len() as u32)
        .filter_map(|id| retained.node(id))
        .filter(|node| {
            node.name
                .as_ref()
                .is_some_and(|name| name.local_name == "media-query")
        })
        .collect();
    assert_eq!(queries.len(), 2);
    for (query, expected) in queries.iter().zip(["false", "true"]) {
        assert!(query
            .attributes
            .iter()
            .filter_map(|id| retained.node(*id))
            .any(|attr| attr
                .name
                .as_ref()
                .is_some_and(|name| name.local_name == "syntax-valid")
                && attr.value == expected));
    }
    c.complete_import(child.id, tree("a {}"), &child.resolution.resolved_url)
        .unwrap();
    assert_eq!(c.state(), CssImportState::Ready);
}
