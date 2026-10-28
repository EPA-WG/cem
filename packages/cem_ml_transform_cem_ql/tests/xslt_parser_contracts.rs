//! Original-owned XSLT parsing must survive preflight and shared compilation.
use cem_ml::{
    engine::{
        CemMlEngine, EngineInput, FormatIdentity, TemplateInput, TransformRequest,
        TransformRuntimePhase, TransformTemplateKind,
    },
    real::RealCemMlEngine,
    schema::registry::XSLT_SCHEMA_URI,
};
use cem_ml_transform_cem_ql::engine_context_with_cem_ql_template_adapter;

const SCHEMA: &str = include_str!("../../cem_ml/schema-packages/xslt/v1/schema/xslt.cem");
const SOURCE: &str = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:template match="/"><p data-literal="{">Café 🍒</p></xsl:template></xsl:stylesheet>"#;

#[test]
fn engine_compilation_keeps_the_original_owned_attribute_grammar() {
    let schema = SCHEMA.replace("literal-result@*", "literal-result@title");
    let context = admitted_context(&schema);
    let result = render(context, SOURCE);
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        result.diagnostics
    );
    assert_eq!(
        result.primary.as_str(),
        Some("<p data-literal=\"{\">Café 🍒</p>")
    );
    assert!(!result.output_spans.is_empty());
    let output = result.primary.as_str().unwrap();
    assert!(result
        .output_spans
        .iter()
        .all(|span| span.output_range.start + span.output_range.len as u64 <= output.len() as u64));
}

fn render(
    context: cem_ml::engine::EngineContext,
    source: &str,
) -> cem_ml::engine::TransformResponse {
    RealCemMlEngine
        .transform(TransformRequest {
            data: EngineInput {
                uri: "memory:owned-input.xml".into(),
                bytes: b"<r/>".to_vec(),
                from_format: None,
                identity: Some(FormatIdentity {
                    content_type: Some("application/xml".into()),
                    ..Default::default()
                }),
                root_scope: Default::default(),
            },
            template: TemplateInput {
                uri: "memory:owned-view.xsl".into(),
                bytes: source.as_bytes().to_vec(),
                identity: Some(FormatIdentity {
                    content_type: Some("application/xslt+xml".into()),
                    ..Default::default()
                }),
                root_scope: Default::default(),
            },
            template_kind: TransformTemplateKind::Xslt,
            template_entrypoint: Default::default(),
            params: Default::default(),
            preserve_source_offsets: true,
            context,
            target: Some(FormatIdentity {
                content_type: Some("text/html".into()),
                ..Default::default()
            }),
            target_scope: Default::default(),
            scheduler_scope_ids: Default::default(),
            execution_policy: cem_ml::engine::TransformExecutionPolicy {
                runtime_phase: TransformRuntimePhase::XsltParity,
                ..Default::default()
            },
        })
        .unwrap()
}

fn admitted_context(schema: &str) -> cem_ml::engine::EngineContext {
    use cem_ml::{
        engine::SchemaPackageReplacementGrant,
        schema::{
            asset_pack::{
                schema_asset_sha256, SchemaAssetPack, SchemaAssetPackIndex, SchemaAssetPackLimits,
                SchemaAssetSourceDescriptor, SchemaAssetSourceKind, SCHEMA_ASSET_PACK_FORMAT,
            },
            asset_pack_session::SchemaAssetSession,
            registry::{SchemaPackageOrigin, SchemaRegistry},
        },
    };
    let files = [
        "package.cem",
        "schema/xslt.cem",
        "formatters/compact.cemt",
        "formatters/pretty.cemt",
        "formatters/tabular.cemt",
        "formatters/xslt-format-stylesheet.cemt",
        "colorizers/html.cemt",
        "colorizers/md.cemt",
        "colorizers/terminal.cemt",
        "colorizers/xslt-color-stylesheet.cemt",
    ];
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../cem_ml/schema-packages/xslt/v1");
    let originals: Vec<_> = files
        .iter()
        .map(|path| {
            (
                *path,
                if *path == "schema/xslt.cem" {
                    schema.to_owned()
                } else {
                    std::fs::read_to_string(root.join(path)).unwrap()
                },
            )
        })
        .collect();
    let index = SchemaAssetPackIndex {
        format_version: SCHEMA_ASSET_PACK_FORMAT.into(),
        packages: vec![SchemaRegistry::with_builtin_schemas()
            .schema(XSLT_SCHEMA_URI)
            .unwrap()
            .clone()
            .into()],
        sources: originals
            .iter()
            .map(|(path, text)| {
                let (kind, mime) = if *path == "package.cem" {
                    (
                        SchemaAssetSourceKind::Manifest,
                        "application/vnd.cem.schema-package+cem",
                    )
                } else if path.starts_with("schema/") {
                    (
                        SchemaAssetSourceKind::Schema,
                        "application/vnd.cem.schema+cem",
                    )
                } else if path.starts_with("formatters/") {
                    (
                        SchemaAssetSourceKind::Formatter,
                        "application/vnd.cem.transform+cem",
                    )
                } else {
                    (
                        SchemaAssetSourceKind::Colorizer,
                        "application/vnd.cem.transform+cem",
                    )
                };
                SchemaAssetSourceDescriptor {
                    package_id: "xslt".into(),
                    version: "1.0.0".into(),
                    path: format!("schema-packages/xslt/v1/{path}"),
                    kind,
                    content_type: mime.into(),
                    byte_length: text.len(),
                    digest: schema_asset_sha256(text.as_bytes()),
                }
            })
            .collect(),
    };
    let bytes = index.to_bytes().unwrap();
    let pack = SchemaAssetPack::from_index_bytes(
        &bytes,
        &schema_asset_sha256(&bytes),
        SchemaAssetPackLimits::default(),
    )
    .unwrap();
    let mut context = engine_context_with_cem_ql_template_adapter();
    context
        .schema_package_replacement_grants
        .push(SchemaPackageReplacementGrant {
            package_id: "xslt".into(),
            expected_origin: SchemaPackageOrigin::Builtin,
            replacement_manifest_uri: "schema-packages/xslt/v1/package.cem".into(),
        });
    let mut session = SchemaAssetSession::new(pack, context);
    assert!(session.activate(&[("xslt", "1.0.0")]).is_err());
    for (path, text) in originals {
        session
            .admit(
                "xslt",
                "1.0.0",
                &format!("schema-packages/xslt/v1/{path}"),
                text.into_bytes(),
            )
            .unwrap();
    }
    let diagnostics = session.activate(&[("xslt", "1.0.0")]).unwrap();
    assert!(
        !diagnostics.iter().any(|d| d.severity.is_hard_violation()),
        "{diagnostics:?}"
    );
    session.context().clone()
}

#[test]
fn engine_imports_keep_owned_uri_facts_and_modules_on_fresh_contexts() {
    use cem_ml::resolver::{
        ResolveDirection, ResolvePurpose, ResolveRequest, ResolvedRead, ResolvedWrite,
        ResolverDiagnostic, ResourceResolver,
    };
    struct Child;
    impl ResourceResolver for Child {
        fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
            assert_eq!(request.uri, "memory:child.xsl");
            Ok(ResolvedRead {
                uri: request.uri.clone(),
                bytes: SOURCE.as_bytes().to_vec(),
                content_type: Some("application/xslt+xml".into()),
            })
        }
        fn write(
            &self,
            request: &ResolveRequest,
            _: &[u8],
        ) -> Result<ResolvedWrite, ResolverDiagnostic> {
            Err(ResolverDiagnostic::UnsupportedResolver {
                uri: request.uri.clone(),
                purpose: request.purpose,
                direction: ResolveDirection::Write,
            })
        }
    }
    let schema = SCHEMA
        .replace("literal-result@*", "literal-result@title")
        .replace("cem.xslt.external_uri_rejected", "fixture.owned_uri");
    let source = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:import href="memory:child.xsl"/></xsl:stylesheet>"#;
    let mut owners = Vec::new();
    for _ in 0..2 {
        let mut context = admitted_context(&schema);
        owners.push(
            context
                .schema_document_models
                .resolve_for_identity(Some(XSLT_SCHEMA_URI), None, None)
                .unwrap()
                .parser_contract
                .as_ref()
                .unwrap()
                .source_owner()
                .unwrap()
                .clone(),
        );
        context.resolver_registry.register(
            "memory",
            ResolvePurpose::Template,
            ResolveDirection::Read,
            Child,
        );
        let result = render(context, source);
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()),
            "{:?}",
            result.diagnostics
        );
        assert_eq!(
            result.primary.as_str(),
            Some("<p data-literal=\"{\">Café 🍒</p>")
        );
    }
    assert!(!std::sync::Arc::ptr_eq(&owners[0], &owners[1]));
}
