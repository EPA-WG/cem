use super::*;
use crate::dependency_identity::{
    DependencyClosureState, DependencyLayer, DependencyVersion, SourceDependency,
    RENDER_DEPENDENCY_INPUTS,
};
use crate::ssr_route::{preflight_ssr_page_routes, SsrPageRouteRequest};
use crate::transform_artifact::TransformArtifactExporterRegistry;
use crate::validation::html::{html_document_ast_from_source_bytes, HtmlSourceValidationRequest};
use std::collections::BTreeMap;

fn source() -> Arc<HtmlDocumentAst> {
    Arc::new(html_document_ast_from_source_bytes(HtmlSourceValidationRequest {
        bytes: b"<!doctype html><html><head><base href='/assets/'></head><body><p>Original native body</p></body></html>",
        source_uri: "memory:original.html", content_type: Some("text/html"),
    }).0.unwrap())
}

fn policy() -> SsrSettlePolicy {
    SsrSettlePolicy {
        max_declaration_depth: 8,
        max_instances: 32,
        max_render_passes: 4,
        max_resources: 16,
        max_total_bytes: 1024 * 1024,
        max_wall_time_ms: 1000,
        max_cpu_time_ms: 500,
        allow_static_fallback: false,
    }
}

fn identity(source: &HtmlDocumentAst) -> SsrExecutionIdentity {
    let content = |bytes: &[u8]| DependencyVersion::Content(ContentHash::from_blake3(bytes));
    SsrExecutionIdentity {
        dependencies: DependencySnapshot {
            layer: DependencyLayer::RenderProjection,
            engine_version: crate::VERSION.into(),
            format_version: "ssr-input/1".into(),
            closure_state: DependencyClosureState::Ready,
            root_uri: source.source.uri.clone(),
            sources: BTreeMap::from([(
                source.source.uri.clone(),
                SourceDependency {
                    content_type: "text/html".into(),
                    schema: Some(crate::schema::registry::HTML_SCHEMA_URI.into()),
                    version: content(source.source_text.as_bytes()),
                },
            )]),
            imports: vec![],
            options: BTreeMap::new(),
            policy: content(b"policy"),
            resolution: content(b"read-map-policy"),
            scope: content(b"module-scope"),
            inputs: RENDER_DEPENDENCY_INPUTS
                .iter()
                .map(|input| (*input, content(b"native-input-snapshot")))
                .collect(),
        },
        lifecycle_version: "0.2.0".into(),
        island_schema_version: "0.2.0".into(),
        render_plan_version: "1.7.0".into(),
        cem_ql_version: "test-query/1".into(),
        entrypoint: "ssr:render-page".into(),
        parameters_hash: ContentHash::from_blake3(b"canonical-native-params"),
        source_map_mode: SsrSourceMapMode::Retained,
        formatter_profile: "dom-safe/1".into(),
    }
}

fn artifact_with(
    source: Arc<HtmlDocumentAst>,
    output: &str,
    identity: SsrExecutionIdentity,
    policy: SsrSettlePolicy,
) -> Result<CemSsrPageArtifact, SsrArtifactError> {
    let routes = preflight_ssr_page_routes(&[SsrPageRouteRequest::new(
        &source,
        "https://ssr.example.test/guide/",
        Some(output),
    )])
    .unwrap();
    CemSsrPageArtifact::unrendered(
        Arc::clone(&source),
        &routes[0],
        TransformArtifactBody::HtmlDomProjection(Arc::clone(&source)),
        identity,
        policy,
    )
}

fn artifact() -> CemSsrPageArtifact {
    let source = source();
    artifact_with(
        Arc::clone(&source),
        "file:///dist/one/index.html",
        identity(&source),
        policy(),
    )
    .unwrap()
}

#[test]
fn ssr_artifact_retains_original_native_owner_route_maps_and_unrendered_gate() {
    let owner = source();
    let original = (*owner).clone();
    let artifact = artifact_with(
        Arc::clone(&owner),
        "file:///dist/index.html",
        identity(&owner),
        policy(),
    )
    .unwrap();
    assert!(Arc::ptr_eq(artifact.source_document(), &owner));
    let TransformArtifactBody::HtmlDomProjection(document) = artifact.document() else {
        panic!("native HTML projection")
    };
    assert!(Arc::ptr_eq(document, &owner));
    assert_eq!(*owner, original);
    assert_eq!(artifact.route().source_uri, "memory:original.html");
    assert_eq!(artifact.route().page_url, "https://ssr.example.test/guide/");
    assert_eq!(
        artifact.route().document_base_url,
        "https://ssr.example.test/assets/"
    );
    assert_eq!(
        artifact.route().output_uri.as_deref(),
        Some("file:///dist/index.html")
    );
    assert_eq!(artifact.lifecycle(), SsrLifecycleState::Unrendered);
    assert!(!artifact.adoption_identity().hydration_ready());
    assert!(!artifact.static_fallback_candidate());
    assert!(artifact.source_map().is_some());
}

#[test]
fn ssr_artifact_rejects_foreign_preflight_owner_encoded_or_replacement_html() {
    let owner = source();
    let routes = preflight_ssr_page_routes(&[SsrPageRouteRequest::new(
        &owner,
        "https://ssr.example.test/",
        None,
    )])
    .unwrap();
    let clone = Arc::new((*owner).clone());
    assert_eq!(
        CemSsrPageArtifact::unrendered(
            Arc::clone(&clone),
            &routes[0],
            TransformArtifactBody::HtmlDomProjection(Arc::clone(&clone)),
            identity(&owner),
            policy()
        )
        .unwrap_err(),
        SsrArtifactError::SourceOwnerMismatch
    );
    assert_eq!(
        CemSsrPageArtifact::unrendered(
            Arc::clone(&owner),
            &routes[0],
            TransformArtifactBody::HtmlDomProjection(clone),
            identity(&owner),
            policy()
        )
        .unwrap_err(),
        SsrArtifactError::SourceOwnerMismatch
    );
    assert_eq!(
        CemSsrPageArtifact::unrendered(
            Arc::clone(&owner),
            &routes[0],
            TransformArtifactBody::CemDocument(Arc::new(Default::default())),
            identity(&owner),
            policy()
        )
        .unwrap_err(),
        SsrArtifactError::DocumentRequired
    );
}

#[test]
fn ssr_artifact_accepts_existing_cem_dom_projection_without_copying_or_json_handoff() {
    let owner = source();
    let dom = Arc::new(
        crate::parser::builder::CemAstBuilder::new(crate::events::cem::CemEventNormalizer::new(
            crate::tokenizer::cem::CemTokenizer::from_source(crate::source::BytesSource::new(
                crate::source::SourceId(41),
                br#"{main @id=root |Retained projected content}"#.to_vec(),
            )),
        ))
        .build(),
    );
    let node_count = dom.nodes.len();
    assert!(node_count > 1);
    let routes = preflight_ssr_page_routes(&[SsrPageRouteRequest::new(
        &owner,
        "https://ssr.example.test/",
        None,
    )])
    .unwrap();
    let artifact = CemSsrPageArtifact::unrendered(
        Arc::clone(&owner),
        &routes[0],
        TransformArtifactBody::DomProjection(Arc::clone(&dom)),
        identity(&owner),
        policy(),
    )
    .unwrap();
    let TransformArtifactBody::DomProjection(retained) = artifact.document() else {
        panic!("native CEM DOM")
    };
    assert!(Arc::ptr_eq(&dom, retained));
    let routed = artifact.into_transform_artifact("ssr-page");
    assert_eq!(routed.body.representation_id(), SSR_PAGE_REPRESENTATION);
    assert_eq!(
        routed.identity.as_ref().unwrap().content_type.as_deref(),
        Some(SSR_PAGE_CONTENT_TYPE)
    );
    assert_eq!(
        routed.identity.as_ref().unwrap().schema.as_deref(),
        Some(SSR_PAGE_SCHEMA)
    );
    let TransformArtifactBody::Extension(native) = &routed.body else {
        panic!("native extension")
    };
    let retained = native
        .as_any()
        .downcast_ref::<CemSsrPageArtifact>()
        .unwrap();
    let TransformArtifactBody::DomProjection(retained_dom) = retained.document() else {
        panic!("native CEM DOM")
    };
    assert!(Arc::ptr_eq(&dom, retained_dom));
    assert_eq!(retained_dom.nodes.len(), node_count);
    assert!(routed.encoded_text().is_err());
    assert!(routed.cemt_evaluator_view().is_none());
    assert!(TransformArtifactExporterRegistry::with_builtin_exporters()
        .export(&routed.body, routed.identity.as_ref().unwrap())
        .is_err());
}

#[test]
fn ssr_artifact_render_input_key_excludes_physical_output_but_tracks_execution_context() {
    let owner = source();
    let original = artifact_with(
        Arc::clone(&owner),
        "file:///dist/one/index.html",
        identity(&owner),
        policy(),
    )
    .unwrap();
    let key = original.render_input_identity().unwrap();
    let moved = artifact_with(
        Arc::clone(&owner),
        "memory:other-output",
        identity(&owner),
        policy(),
    )
    .unwrap();
    assert_eq!(moved.render_input_identity().unwrap(), key);
    for change in 0..10 {
        let mut context = identity(&owner);
        let mut limits = policy();
        match change {
            0 => context.lifecycle_version.push_str("-changed"),
            1 => context.island_schema_version.push_str("-changed"),
            2 => context.render_plan_version.push_str("-changed"),
            3 => context.cem_ql_version.push_str("-changed"),
            4 => context.parameters_hash = ContentHash::from_blake3(b"other params"),
            5 => context.source_map_mode = SsrSourceMapMode::None,
            6 => context.formatter_profile.push_str("-changed"),
            7 => {
                context.dependencies.policy =
                    DependencyVersion::Content(ContentHash::from_blake3(b"other policy"))
            }
            8 => {
                context.dependencies.resolution =
                    DependencyVersion::Content(ContentHash::from_blake3(b"other resolver"))
            }
            9 => limits.max_render_passes += 1,
            _ => unreachable!(),
        }
        let changed = artifact_with(
            Arc::clone(&owner),
            "file:///dist/one/index.html",
            context,
            limits,
        )
        .unwrap();
        assert_ne!(
            changed.render_input_identity().unwrap(),
            key,
            "change {change}"
        );
        assert!(!changed.adoption_identity().hydration_ready());
    }
}

#[test]
fn ssr_artifact_missing_pending_or_wrong_dependency_closure_cannot_yield_cache_identity() {
    let owner = source();
    for change in 0..5 {
        let mut context = identity(&owner);
        match change {
            0 => context.dependencies.closure_state = DependencyClosureState::Pending,
            1 => {
                context.dependencies.inputs.clear();
            }
            2 => context.dependencies.policy = DependencyVersion::Opaque,
            3 => context.dependencies.root_uri = "memory:wrong-source".into(),
            4 => {
                context
                    .dependencies
                    .sources
                    .get_mut(&owner.source.uri)
                    .unwrap()
                    .version = DependencyVersion::Content(ContentHash::from_blake3(b"wrong bytes"));
            }
            _ => unreachable!(),
        }
        let artifact =
            artifact_with(Arc::clone(&owner), "memory:output", context, policy()).unwrap();
        assert!(artifact.render_input_identity().is_err(), "change {change}");
        assert!(!artifact.adoption_identity().hydration_ready());
        // Explicit JSON control report contains the refusal, never the AST.
        let report: serde_json::Value =
            serde_json::from_slice(&artifact.export_report_json().unwrap()).unwrap();
        assert!(report["renderInputIdentity"].is_null());
        assert!(report["identityRefusal"].is_string());
    }
}

#[test]
fn ssr_artifact_policy_and_initial_source_bytes_are_bounded() {
    let owner = source();
    for change in 0..8 {
        let mut limits = policy();
        match change {
            0 => limits.max_declaration_depth = 0,
            1 => limits.max_instances = u32::MAX,
            2 => limits.max_render_passes = 0,
            3 => limits.max_resources = u32::MAX,
            4 => limits.max_total_bytes = 0,
            5 => limits.max_wall_time_ms = 0,
            6 => limits.max_cpu_time_ms = u64::MAX,
            7 => limits.max_declaration_depth = u32::MAX,
            _ => unreachable!(),
        }
        assert_eq!(
            artifact_with(
                Arc::clone(&owner),
                "memory:output",
                identity(&owner),
                limits
            )
            .unwrap_err(),
            SsrArtifactError::InvalidPolicy
        );
    }
    let mut limits = policy();
    limits.max_total_bytes = 1;
    assert_eq!(
        artifact_with(
            Arc::clone(&owner),
            "memory:output",
            identity(&owner),
            limits
        )
        .unwrap_err(),
        SsrArtifactError::BudgetExceeded
    );
}

#[test]
fn ssr_artifact_partial_and_exhausted_states_never_claim_adoption() {
    for reason in [
        SsrPartialReason::ParticipantUnavailable,
        SsrPartialReason::CapabilityUnavailable,
        SsrPartialReason::Cancelled,
        SsrPartialReason::BudgetExhausted,
        SsrPartialReason::Incomplete,
    ] {
        for fallback in [false, true] {
            let owner = source();
            let mut limits = policy();
            limits.allow_static_fallback = fallback;
            let artifact = artifact_with(
                Arc::clone(&owner),
                "memory:output",
                identity(&owner),
                limits,
            )
            .unwrap();
            assert!(
                !artifact.static_fallback_candidate(),
                "unrendered fallback is never a publication candidate"
            );
            let mut usage = artifact.usage();
            usage.render_passes = 1;
            if reason == SsrPartialReason::BudgetExhausted {
                usage.wall_time_ms = 1001;
            }
            let partial = artifact.into_partial(reason, usage, vec![]).unwrap();
            assert_eq!(partial.lifecycle(), SsrLifecycleState::Partial { reason });
            assert!(!partial.adoption_identity().hydration_ready());
            assert_eq!(partial.static_fallback_candidate(), fallback);
        }
    }
}

#[test]
fn ssr_artifact_usage_cannot_reset_or_hide_budget_exhaustion() {
    let initial = artifact();
    let usage = initial.usage();
    assert_eq!(
        initial
            .into_partial(
                SsrPartialReason::Incomplete,
                SsrExecutionUsage::default(),
                vec![]
            )
            .unwrap_err(),
        SsrArtifactError::InvalidProgress
    );
    let mut over = usage;
    over.resources = 17;
    assert_eq!(
        artifact()
            .into_partial(SsrPartialReason::Incomplete, over, vec![])
            .unwrap_err(),
        SsrArtifactError::BudgetExceeded
    );
    let mut progress = usage;
    progress.instances = 2;
    let partial = artifact()
        .into_partial(SsrPartialReason::Incomplete, progress, vec![])
        .unwrap();
    assert_eq!(
        partial
            .into_partial(SsrPartialReason::Incomplete, usage, vec![])
            .unwrap_err(),
        SsrArtifactError::InvalidProgress
    );
}

#[test]
fn ssr_artifact_report_is_explicit_bounded_control_metadata_without_source_or_dom() {
    let artifact = artifact();
    let report: serde_json::Value =
        serde_json::from_slice(&artifact.export_report_json().unwrap()).unwrap();
    assert_eq!(report["contentType"], SSR_PAGE_CONTENT_TYPE);
    assert_eq!(report["schema"], SSR_PAGE_SCHEMA);
    assert_eq!(report["documentRepresentation"], "cem.html-dom-projection");
    assert_eq!(report["adoption"]["hydrationReady"], false);
    assert_eq!(report["adoption"]["lifecycleVersion"], "0.2.0");
    assert!(report["renderInputIdentity"].is_object());
    assert!(report.get("document").is_none());
    assert!(!String::from_utf8(artifact.export_report_json().unwrap())
        .unwrap()
        .contains("Original native body"));
    let mut diagnostic = Diagnostic::default();
    diagnostic.message = "x".repeat(SSR_METADATA_LIMIT + 1);
    let usage = artifact.usage();
    assert_eq!(
        artifact
            .into_partial(SsrPartialReason::Incomplete, usage, vec![diagnostic])
            .unwrap_err(),
        SsrArtifactError::MetadataBounds
    );
}

#[test]
fn ssr_artifact_preserves_route_config_origins_and_unmapped_generated_provenance() {
    let mut native = (*source()).clone();
    native.origin_spans = Some(crate::source_map::SourceMapIndex::new(vec![]));
    let owner = Arc::new(native);
    let config_map = owner.events[0]
        .source_range
        .source_map()
        .with_source_uri("memory:route-config");
    let mut request = SsrPageRouteRequest::new(
        &owner,
        "https://ssr.example.test/guide/",
        Some("memory:output"),
    );
    request.page_url_source_map = Some(&config_map);
    request.output_uri_source_map = Some(&config_map);
    let routes = preflight_ssr_page_routes(&[request]).unwrap();
    let artifact = CemSsrPageArtifact::unrendered(
        Arc::clone(&owner),
        &routes[0],
        TransformArtifactBody::HtmlDomProjection(Arc::clone(&owner)),
        identity(&owner),
        policy(),
    )
    .unwrap();
    assert_eq!(artifact.source_map(), None);
    assert_eq!(artifact.route().base_source_map, None);
    assert_eq!(artifact.route().page_source_map.as_ref(), Some(&config_map));
    assert_eq!(
        artifact.route().output_source_map.as_ref(),
        Some(&config_map)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&artifact.export_report_json().unwrap()).unwrap();
    assert!(report["sourceMap"].is_null());
    assert_eq!(report["route"]["baseSelection"]["kind"], "htmlBase");
    assert_eq!(report["route"]["baseSelection"]["usedFallback"], false);
}

#[test]
fn ssr_artifact_rejects_invalid_protocol_hashes_and_oversized_control_metadata() {
    let owner = source();
    for change in 0..4 {
        let mut context = identity(&owner);
        match change {
            0 => context.lifecycle_version.clear(),
            1 => context.parameters_hash.hex = "not-a-native-hash".into(),
            2 => context.parameters_hash.scheme = "unknown-hash".into(),
            3 => context.entrypoint = "e".repeat(4097),
            _ => unreachable!(),
        }
        assert_eq!(
            artifact_with(Arc::clone(&owner), "memory:output", context, policy()).unwrap_err(),
            SsrArtifactError::InvalidIdentity
        );
    }
    let mut context = identity(&owner);
    context
        .dependencies
        .options
        .insert("too-large".into(), "x".repeat(SSR_METADATA_LIMIT));
    assert_eq!(
        artifact_with(Arc::clone(&owner), "memory:output", context, policy()).unwrap_err(),
        SsrArtifactError::MetadataBounds
    );
    for diagnostics in [
        vec![Diagnostic::default(); 257],
        vec![Diagnostic {
            message: "\"".repeat(SSR_METADATA_LIMIT / 2 + 1),
            ..Diagnostic::default()
        }],
    ] {
        let initial = artifact();
        let usage = initial.usage();
        assert_eq!(
            initial
                .into_partial(SsrPartialReason::Incomplete, usage, diagnostics)
                .unwrap_err(),
            SsrArtifactError::MetadataBounds
        );
    }
}

#[test]
fn ssr_artifact_input_keys_use_structured_versions_and_all_dependency_inputs() {
    let owner = source();
    let mut first = identity(&owner);
    first.lifecycle_version = "a|b".into();
    first.island_schema_version = "c".into();
    let mut second = first.clone();
    second.lifecycle_version = "a".into();
    second.island_schema_version = "b|c".into();
    let a = artifact_with(Arc::clone(&owner), "memory:one", first, policy()).unwrap();
    let b = artifact_with(Arc::clone(&owner), "memory:two", second, policy()).unwrap();
    assert_ne!(
        a.render_input_identity().unwrap(),
        b.render_input_identity().unwrap()
    );
    let baseline = artifact().render_input_identity().unwrap();
    for input in RENDER_DEPENDENCY_INPUTS {
        let mut context = identity(&owner);
        context.dependencies.inputs.insert(
            *input,
            DependencyVersion::Content(ContentHash::from_blake3(b"new native state")),
        );
        let changed =
            artifact_with(Arc::clone(&owner), "memory:output", context, policy()).unwrap();
        assert_ne!(
            changed.render_input_identity().unwrap(),
            baseline,
            "{input:?}"
        );
    }
    let mut context = identity(&owner);
    context.dependencies.sources.insert(
        "https://ssr.example.test/card.cem".into(),
        SourceDependency {
            content_type: "text/cem-ml".into(),
            schema: None,
            version: DependencyVersion::Content(ContentHash::from_blake3(b"declaration bytes")),
        },
    );
    context
        .dependencies
        .imports
        .push(crate::dependency_identity::ImportDependency {
            parent_uri: owner.source.uri.clone(),
            specifier: "./card.cem".into(),
            resolved_uri: "https://ssr.example.test/card.cem".into(),
        });
    let imported = artifact_with(Arc::clone(&owner), "memory:output", context, policy()).unwrap();
    assert_ne!(imported.render_input_identity().unwrap(), baseline);
}

#[test]
fn ssr_artifact_page_source_and_base_policy_are_part_of_input_identity() {
    let owner = source();
    let baseline = artifact().render_input_identity().unwrap();
    let mut blocked = SsrPageRouteRequest::new(&owner, "https://ssr.example.test/guide/", None);
    blocked.base_policy = crate::ssr_route::SsrBasePolicy::Blocked;
    let other_page = SsrPageRouteRequest::new(&owner, "https://other.example.test/guide/", None);
    for request in [blocked, other_page] {
        let routes = preflight_ssr_page_routes(&[request]).unwrap();
        let artifact = CemSsrPageArtifact::unrendered(
            Arc::clone(&owner),
            &routes[0],
            TransformArtifactBody::HtmlDomProjection(Arc::clone(&owner)),
            identity(&owner),
            policy(),
        )
        .unwrap();
        assert_ne!(artifact.render_input_identity().unwrap(), baseline);
    }
    let mut changed_source = (*owner).clone();
    changed_source.source.uri = "memory:other-source.html".into();
    let changed_source = Arc::new(changed_source);
    let changed = artifact_with(
        Arc::clone(&changed_source),
        "memory:output",
        identity(&changed_source),
        policy(),
    )
    .unwrap();
    assert_ne!(changed.render_input_identity().unwrap(), baseline);
}

#[test]
fn ssr_artifact_exhausted_usage_reports_every_counter_without_authorizing_hydration() {
    for index in 0..7 {
        let initial = artifact();
        let key = initial.render_input_identity().unwrap();
        let mut usage = initial.usage();
        match index {
            0 => usage.declaration_depth = 9,
            1 => usage.instances = 33,
            2 => usage.render_passes = 5,
            3 => usage.resources = 17,
            4 => usage.total_bytes = 1024 * 1024 + 1,
            5 => usage.wall_time_ms = 1001,
            6 => usage.cpu_time_ms = 501,
            _ => unreachable!(),
        }
        let partial = initial
            .into_partial(SsrPartialReason::BudgetExhausted, usage, vec![])
            .unwrap();
        assert_eq!(
            partial.render_input_identity().unwrap(),
            key,
            "measured usage is not a deterministic input"
        );
        let report: serde_json::Value =
            serde_json::from_slice(&partial.export_report_json().unwrap()).unwrap();
        assert_eq!(report["budgetExceeded"], true);
        assert_eq!(report["adoption"]["hydrationReady"], false);
        assert!(!partial.static_fallback_candidate());
    }
}
