//! Shared content-addressed cache verification (AC-CC-1..AC-CC-7).

use cem_ml::content_cache::{
    ArtifactContentType, CacheArtifact, CacheMode, CemHashRequest, CemHashResponse, ContentCache,
    ContentHash, InMemoryCemHashTransport, PolicyStamps,
};
use cem_ml::source::{ByteRange, SourceId};
use cem_ml::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};

fn policy(name: &str) -> PolicyStamps {
    let mut stamps = PolicyStamps::new(format!("policy:{name}"));
    stamps
        .declared_schema_uris
        .insert("urn:cem:schema:core".into());
    stamps.plugin_imports.insert("plugin:scss".into());
    stamps.external_reads.insert("file:///schema.rng".into());
    stamps
}

fn source_map() -> SourceMapStack {
    let mut stack = SourceMapStack::default();
    stack.push(SourceMapFrame {
        source_id: SourceId(1),
        span: FrameSpan::Single(ByteRange::new(4, 8)),
        transform: TransformKind::CemTokenizer,
    });
    stack
}

#[test]
fn ac_cc_1_hash_scheme_is_cem_bin_blake3_and_deterministic() {
    let a = ContentHash::from_blake3(b"artifact bytes");
    let b = ContentHash::from_blake3(b"artifact bytes");
    let c = ContentHash::from_blake3(b"other bytes");
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(a.scheme, "cem-bin/1+blake3");
    assert!(a.header_value().starts_with("cem-bin/1+blake3:"));
}

#[test]
fn ac_cc_2_serialization_round_trips_and_cache_skips_parser() {
    let (artifact, sidecar) = CacheArtifact::new(
        ArtifactContentType::CemMlDocument,
        CacheMode::Dev,
        b"canonical ast".to_vec(),
        policy("root"),
        Some(source_map()),
    );
    let encoded = artifact.serialize();
    let decoded = CacheArtifact::deserialize(&encoded).unwrap();
    assert_eq!(decoded, artifact);

    let mut cache = ContentCache::new();
    cache.insert(decoded.clone(), sidecar);
    let mut parser_enters = 0;
    let hit = cache
        .get(&decoded.key, &policy("root"))
        .unwrap_or_else(|_| {
            parser_enters += 1;
            panic!("cache hit expected")
        });
    assert_eq!(hit.bytes, b"canonical ast");
    assert_eq!(parser_enters, 0, "loader must not parse on cache hit");
}

#[test]
fn ac_cc_3_policy_stamp_mismatch_rejects_cached_binary() {
    let (artifact, sidecar) = CacheArtifact::new(
        ArtifactContentType::TransformPlan,
        CacheMode::Prod,
        b"plan".to_vec(),
        policy("permissive"),
        None,
    );
    let mut cache = ContentCache::new();
    cache.insert(artifact.clone(), sidecar);
    let err = cache.get(&artifact.key, &policy("strict")).unwrap_err();
    assert_eq!(err.code, "cem.cc.policy_mismatch");
}

#[test]
fn ac_cc_4_and_5_dev_prod_mode_axis_controls_source_map_sidecars() {
    let bytes = b"same canonical ast".to_vec();
    let (dev, dev_sidecar) = CacheArtifact::new(
        ArtifactContentType::CemMlSchema,
        CacheMode::Dev,
        bytes.clone(),
        policy("root"),
        Some(source_map()),
    );
    let (prod, prod_sidecar) = CacheArtifact::new(
        ArtifactContentType::CemMlSchema,
        CacheMode::Prod,
        bytes,
        policy("root"),
        Some(source_map()),
    );
    assert_ne!(dev.key, prod.key, "mode must be part of the cache key");
    assert!(dev.source_map_sidecar_hash.is_some());
    assert!(dev_sidecar.is_some());
    assert!(prod.source_map_sidecar_hash.is_none());
    assert!(prod_sidecar.is_none());
}

#[test]
fn ac_cc_6_cem_hash_transport_confirms_cached_artifact_with_304() {
    let mut transport = InMemoryCemHashTransport::new();
    let published = transport.publish(
        "https://example.test/doc.cem",
        ArtifactContentType::CemMlDocument,
        b"{document}".to_vec(),
    );
    let first = transport
        .fetch(&CemHashRequest {
            uri: "https://example.test/doc.cem".into(),
            if_cem_hash: None,
        })
        .unwrap();
    assert!(matches!(first, CemHashResponse::Body { .. }));

    let second = transport
        .fetch(&CemHashRequest {
            uri: "https://example.test/doc.cem".into(),
            if_cem_hash: Some(published.clone()),
        })
        .unwrap();
    assert_eq!(
        second,
        CemHashResponse::NotModified {
            cem_hash: published,
            content_type: ArtifactContentType::CemMlDocument
        }
    );
}

#[test]
fn ac_cc_7_transport_applies_to_secondary_content() {
    let mut transport = InMemoryCemHashTransport::new();
    let hash = transport.publish(
        "https://example.test/query.cemql",
        ArtifactContentType::CemQlModule,
        b"from //button".to_vec(),
    );
    let response = transport
        .fetch(&CemHashRequest {
            uri: "https://example.test/query.cemql".into(),
            if_cem_hash: Some(hash.clone()),
        })
        .unwrap();
    assert_eq!(
        response,
        CemHashResponse::NotModified {
            cem_hash: hash,
            content_type: ArtifactContentType::CemQlModule,
        }
    );
}
