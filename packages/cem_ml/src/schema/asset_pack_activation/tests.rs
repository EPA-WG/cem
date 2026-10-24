use super::*;
use crate::{
    engine::{
        CemMlEngine, ConvertRequest, EngineContext, EngineInput, FormatIdentity, InputFormat,
        LayerFormat, SchemaPackageReplacementGrant,
    },
    operation_control::OperationControl,
    real::RealCemMlEngine,
    run_config::ScopeConfig,
    schema::{
        asset_pack::{
            builtin_schema_asset_pack_export, schema_asset_sha256, SchemaAssetPack,
            SchemaAssetPackLimits,
        },
        registry::SchemaPackageOrigin,
    },
};
use std::sync::Arc;

const CSV: &str = "https://cem.dev/ns/data/csv/1";
const MANIFEST: &str = "schema-packages/csv/v1/package.cem";

fn pack(paths: &[&str]) -> Arc<SchemaAssetPack> {
    static EXPORT: std::sync::OnceLock<crate::schema::asset_pack::SchemaAssetPackExport> =
        std::sync::OnceLock::new();
    let export = EXPORT.get_or_init(|| builtin_schema_asset_pack_export().unwrap());
    let bytes = export.index.to_bytes().unwrap();
    let mut pack = SchemaAssetPack::from_index_bytes(
        &bytes,
        &schema_asset_sha256(&bytes),
        SchemaAssetPackLimits::default(),
    )
    .unwrap();
    let control = OperationControl::default();
    for source in export
        .sources
        .iter()
        .filter(|s| paths.contains(&s.descriptor.path.as_str()))
    {
        pack.admit_source(
            &source.descriptor.package_id,
            &source.descriptor.version,
            &source.descriptor.path,
            source.source.as_bytes().to_vec(),
            &source.descriptor.content_type,
            &control,
            control.root_scope(),
        )
        .unwrap();
    }
    Arc::new(pack)
}

fn sources() -> Vec<&'static str> {
    vec![
        MANIFEST,
        "schema-packages/csv/v1/schema/csv.cem",
        "schema-packages/csv/v1/formatters/compact.cemt",
        "schema-packages/csv/v1/colorizers/terminal.cemt",
    ]
}

fn grant(context: &mut EngineContext) {
    context
        .schema_package_replacement_grants
        .push(SchemaPackageReplacementGrant {
            package_id: "csv".into(),
            expected_origin: SchemaPackageOrigin::Builtin,
            replacement_manifest_uri: MANIFEST.into(),
        });
}

fn convert(context: EngineContext, profile: &str) -> crate::engine::ConvertResponse {
    RealCemMlEngine
        .convert(ConvertRequest {
            input: EngineInput {
                uri: "memory://activation/input.csv".into(),
                bytes: b"id,name\n1,Ada\n".to_vec(),
                from_format: None,
                identity: Some(FormatIdentity {
                    content_type: Some("text/csv; header=present".into()),
                    schema: Some(CSV.into()),
                    ..Default::default()
                }),
                root_scope: ScopeConfig::default(),
            },
            to_format: LayerFormat::Csv,
            preserve_source_offsets: true,
            context,
            target: Some(FormatIdentity {
                content_type: Some("text/csv".into()),
                schema: Some(CSV.into()),
                ..Default::default()
            }),
            target_scope: ScopeConfig {
                cemt_formatter_profile: Some(profile.into()),
                cemt_color_profile: Some("terminal".into()),
                ..Default::default()
            },
            scheduler_scope_id: 0,
        })
        .unwrap()
}

const DOM: &str = "https://cem.dev/ns/projection/dom/1";
const DOM_MANIFEST: &str = "schema-packages/cem-dom-projection/v1/package.cem";
const DOM_SCHEMA: &str = "schema-packages/cem-dom-projection/v1/schema/cem-dom-projection.cem";
const DOM_HTML: &str = "schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt";

fn dom_context(pack: Arc<SchemaAssetPack>) -> EngineContext {
    let mut context = EngineContext::default();
    context
        .schema_package_replacement_grants
        .push(SchemaPackageReplacementGrant {
            package_id: "cem-dom-projection".into(),
            expected_origin: SchemaPackageOrigin::Builtin,
            replacement_manifest_uri: DOM_MANIFEST.into(),
        });
    let diagnostics =
        activate_schema_asset_packages(&mut context, pack, &[("cem-dom-projection", "1.0.0")])
            .unwrap();
    assert!(
        !diagnostics.iter().any(|d| d.severity.is_hard_violation()),
        "{diagnostics:?}"
    );
    context
}

fn html(context: EngineContext) -> crate::engine::ConvertResponse {
    RealCemMlEngine
        .convert(ConvertRequest {
            input: EngineInput {
                uri: "memory://activation/input.cem".into(),
                bytes: b"{h1 | Ada}".to_vec(),
                from_format: Some(InputFormat::Cem),
                identity: None,
                root_scope: ScopeConfig::default(),
            },
            to_format: LayerFormat::Html,
            preserve_source_offsets: true,
            context,
            target: Some(FormatIdentity {
                content_type: Some("text/html".into()),
                ..Default::default()
            }),
            target_scope: ScopeConfig::default(),
            scheduler_scope_id: 0,
        })
        .unwrap()
}

#[test]
fn native_asset_activation_owns_sources_and_drives_ordinary_conversion_without_fallback() {
    let pack = pack(&[DOM_MANIFEST, DOM_SCHEMA, DOM_HTML]);
    let weak = Arc::downgrade(&pack);
    let context = dom_context(pack.clone());
    assert_eq!(
        context.schema_registry.package_origin("cem-dom-projection"),
        Some(&SchemaPackageOrigin::Manifest(DOM_MANIFEST.into()))
    );
    assert!(context
        .schema_document_models
        .resolve_for_identity(Some(DOM), None, None)
        .is_some());
    drop(pack);
    assert!(weak.upgrade().is_some());
    let retained = context.clone();
    let output = html(context);
    assert!(
        !output
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        output.diagnostics
    );
    assert!(output_text(&output).contains("Ada"));
    assert_eq!(
        output_text(&output),
        output_text(&html(EngineContext::default()))
    );
    // Unselected packages keep their existing default formatter path.
    assert!(!convert(retained.clone(), "pretty")
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation()));
    drop(retained);
    assert!(weak.upgrade().is_none());
    let missing = dom_context(pack_without_converter());
    let refused = html(missing);
    assert!(refused.primary.is_null());
    assert!(refused.primary_bytes.is_none());
    assert!(
        refused
            .diagnostics
            .iter()
            .any(|d| d.message.contains("no embedded fallback")),
        "{:?}",
        refused.diagnostics
    );
}

fn pack_without_converter() -> Arc<SchemaAssetPack> {
    pack(&[DOM_MANIFEST, DOM_SCHEMA])
}

#[test]
fn native_asset_activation_refuses_unsupported_csv_model_without_publishing() {
    let pack = pack(&sources());
    let mut context = EngineContext::default();
    grant(&mut context);
    let diagnostics =
        activate_schema_asset_packages(&mut context, pack, &[("csv", "1.0.0")]).unwrap();
    assert!(diagnostics
        .iter()
        .any(|d| d.code == "cem.schema_definition.unknown_diagnostic_behavior"));
    assert!(context.schema_asset_activation.is_none());
    assert_eq!(
        context.schema_registry.package_origin("csv"),
        Some(&SchemaPackageOrigin::Builtin)
    );
}

#[test]
fn native_asset_ordinary_csv_output_uses_selected_reader_and_preserves_legacy_fallback() {
    // Isolate output-reader wiring from schema publication: the public API
    // correctly refuses this package's currently unsupported schema behaviors.
    let binding = Arc::new(SchemaAssetActivation {
        pack: pack(&sources()),
        packages: [("csv".into(), "1.0.0".into())].into(),
    });
    let context = EngineContext {
        schema_asset_activation: Some(binding),
        ..Default::default()
    };
    let output = convert(context.clone(), "compact");
    assert!(
        !output
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        output.diagnostics
    );
    assert!(output_text(&output).contains("Ada"));
    assert_eq!(
        output_text(&output),
        output_text(&convert(EngineContext::default(), "compact"))
    );
    let refused = convert(context, "pretty");
    assert!(refused.primary.is_null());
    assert!(refused.primary_bytes.is_none());
    assert!(
        refused
            .diagnostics
            .iter()
            .any(|d| d.message.contains("no embedded fallback")),
        "{:?}",
        refused.diagnostics
    );
    let legacy = convert(EngineContext::default(), "pretty");
    assert!(
        !legacy
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        legacy.diagnostics
    );
}

#[test]
fn native_asset_activation_requires_replacement_authority_and_is_atomic() {
    let pack = pack(&sources());
    let mut context = EngineContext::default();
    let refused =
        activate_schema_asset_packages(&mut context, pack.clone(), &[("csv", "1.0.0")]).unwrap();
    assert!(
        refused
            .iter()
            .any(|d| d.code == "cem.schema_package.replacement_not_authorized"),
        "{refused:?}"
    );
    assert!(context.schema_asset_activation.is_none());
    assert_eq!(
        context.schema_registry.package_origin("csv"),
        Some(&SchemaPackageOrigin::Builtin)
    );
    grant(&mut context);
    let refused =
        activate_schema_asset_packages(&mut context, pack, &[("csv", "1.0.0"), ("json", "1.0.0")]);
    assert!(refused.is_err());
    assert!(context.schema_asset_activation.is_none());
    assert_eq!(
        context.schema_registry.package_origin("csv"),
        Some(&SchemaPackageOrigin::Builtin)
    );
}

fn output_text(response: &crate::engine::ConvertResponse) -> &str {
    if let Some(bytes) = &response.primary_bytes {
        std::str::from_utf8(&bytes.bytes).unwrap()
    } else {
        response
            .primary
            .get("content")
            .and_then(serde_json::Value::as_str)
            .expect("ordinary text output")
    }
}

#[test]
fn native_asset_activation_requires_dependencies_and_exact_versions() {
    let pack = pack_without_converter();
    let mut context = EngineContext::default();
    context.schema_registry.remove_package("schema");
    assert!(activate_schema_asset_packages(
        &mut context,
        pack.clone(),
        &[("cem-dom-projection", "1.0.0")]
    )
    .unwrap_err()
    .to_string()
    .contains("requires active schema"));
    assert!(context.schema_asset_activation.is_none());
    for selection in [
        vec![],
        vec![("cem-dom-projection", "2.0.0")],
        vec![
            ("cem-dom-projection", "1.0.0"),
            ("cem-dom-projection", "1.0.0"),
        ],
    ] {
        assert!(activate_schema_asset_packages(
            &mut EngineContext::default(),
            pack.clone(),
            &selection
        )
        .is_err());
    }
}

#[derive(Debug)]
struct PendingCompiler;
impl crate::schema::package_compilation::SchemaPackageCompiler for PendingCompiler {
    fn compile(
        &self,
        request: &crate::schema::package_compilation::SchemaPackageCompilationRequest,
    ) -> Result<
        crate::schema::document_model::SchemaDocumentModel,
        Vec<crate::diagnostics::Diagnostic>,
    > {
        // Same source arena; an unresolved typed slot must block even without
        // an error-severity diagnostic or a different schema identity.
        assert!(Arc::ptr_eq(
            request.source.ast_owner(),
            request.lexical_scopes.document()
        ));
        let mut model = crate::schema::package_compilation::source_only_model(
            &request.schema_uri,
            &request.source,
        );
        model.attributes.insert(
            "pending".into(),
            crate::schema::document_model::AttributeModel {
                native_type_pending: true,
                ..Default::default()
            },
        );
        Ok(model)
    }
}

#[test]
fn native_asset_activation_does_not_publish_a_pending_native_model() {
    let pack = pack_without_converter();
    let mut context = dom_context(pack.clone());
    let incumbent = context.schema_asset_activation.clone().unwrap();
    context.schema_package_compiler = Some(Arc::new(PendingCompiler));
    let diagnostics =
        activate_schema_asset_packages(&mut context, pack, &[("cem-dom-projection", "1.0.0")])
            .unwrap();
    assert!(diagnostics
        .iter()
        .any(|d| d.code == "cem.schema_package.activation_not_ready"));
    assert!(Arc::ptr_eq(
        &incumbent,
        context.schema_asset_activation.as_ref().unwrap()
    ));
    assert!(!context
        .schema_document_models
        .resolve_for_identity(Some(DOM), None, None)
        .unwrap()
        .attributes
        .contains_key("pending"));
}

#[derive(Debug)]
struct RetireCounter {
    declaration: crate::schema::declaration_references::SchemaDeclarationNode,
    retired: Arc<std::sync::atomic::AtomicUsize>,
}
impl crate::schema::attribute_datatypes::CompiledAttributeDatatype for RetireCounter {
    fn declaration(&self) -> &crate::schema::declaration_references::SchemaDeclarationNode {
        &self.declaration
    }
    fn is_node_valued(&self) -> bool {
        false
    }
    fn validate(
        &self,
        _: crate::schema::attribute_datatypes::AttributeDatatypeInput<'_>,
    ) -> crate::schema::attribute_datatypes::AttributeDatatypeValidation {
        crate::schema::attribute_datatypes::AttributeDatatypeValidation {
            accepted: None,
            diagnostics: vec![],
        }
    }
    fn retire_preparations(&self) {
        self.retired
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[test]
fn native_asset_activation_defers_preparation_retirement_until_the_whole_set_is_ready() {
    let mut paths = sources();
    paths.extend([DOM_MANIFEST, DOM_SCHEMA]);
    let pack = pack(&paths);
    let mut context = dom_context(pack.clone());
    let incumbent = context.schema_asset_activation.clone().unwrap();
    let source = context.schema_package_sources.get(DOM_SCHEMA).unwrap();
    let retired = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut model = context
        .schema_document_models
        .resolve_for_identity(Some(DOM), None, None)
        .unwrap()
        .clone();
    model.attribute_datatypes.insert(
        "prepared".into(),
        Arc::new(RetireCounter {
            declaration: crate::schema::declaration_references::SchemaDeclarationNode::new(
                source.ast_owner().clone(),
                0,
            )
            .unwrap(),
            retired: retired.clone(),
        }),
    );
    context.schema_document_models.register(model);
    grant(&mut context);
    let diagnostics = activate_schema_asset_packages(
        &mut context,
        pack.clone(),
        &[("cem-dom-projection", "1.0.0"), ("csv", "1.0.0")],
    )
    .unwrap();
    assert!(diagnostics.iter().any(|d| d.severity.is_hard_violation()));
    assert!(Arc::ptr_eq(
        &incumbent,
        context.schema_asset_activation.as_ref().unwrap()
    ));
    assert_eq!(retired.load(std::sync::atomic::Ordering::SeqCst), 0);
    let diagnostics =
        activate_schema_asset_packages(&mut context, pack, &[("cem-dom-projection", "1.0.0")])
            .unwrap();
    assert!(
        !diagnostics.iter().any(|d| d.severity.is_hard_violation()),
        "{diagnostics:?}"
    );
    assert_eq!(retired.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn native_asset_activation_republication_cannot_drop_owned_bodies_or_escape_to_local_files() {
    let mut context = dom_context(pack(&[DOM_MANIFEST, DOM_SCHEMA, DOM_HTML]));
    let incumbent = context.schema_asset_activation.clone().unwrap();
    let mut bytes = incumbent.pack.index().to_bytes().unwrap();
    bytes.push(b'\n');
    let independently_pinned = SchemaAssetPack::from_index_bytes(
        &bytes,
        &schema_asset_sha256(&bytes),
        SchemaAssetPackLimits::default(),
    )
    .unwrap();
    assert!(activate_schema_asset_packages(
        &mut context,
        Arc::new(independently_pinned),
        &[("cem-dom-projection", "1.0.0")]
    )
    .unwrap_err()
    .to_string()
    .contains("mix independently pinned"));
    assert!(activate_schema_asset_packages(
        &mut context,
        pack_without_converter(),
        &[("cem-dom-projection", "1.0.0")]
    )
    .is_err());
    assert!(Arc::ptr_eq(
        &incumbent,
        context.schema_asset_activation.as_ref().unwrap()
    ));
    let read = incumbent
        .read_import(DOM_MANIFEST, "./schema/../schema/cem-dom-projection.cem")
        .unwrap()
        .unwrap();
    assert_eq!(read.uri, DOM_SCHEMA);
    for path in [
        "../../../../Cargo.toml",
        "/etc/passwd",
        "file:///etc/passwd",
        "missing.cemt",
    ] {
        assert!(
            incumbent.read_import(DOM_HTML, path).unwrap().is_err(),
            "{path}"
        );
    }
    assert!(incumbent
        .read_import("memory://user/template.cemt", "Cargo.toml")
        .is_none());
    assert_eq!(incumbent.owned_source_count(), 3);
    assert!(incumbent.owned_source_bytes() > 0);
    let source = context.schema_package_sources.get(DOM_SCHEMA).unwrap();
    assert_eq!(source.source_uri(), DOM_SCHEMA);
    assert!(Arc::ptr_eq(
        source.ast_owner(),
        context
            .schema_package_sources
            .get_lexical_scopes(DOM_SCHEMA)
            .unwrap()
            .document()
    ));
}

#[test]
fn native_asset_activation_cannot_be_mutated_through_the_ordinary_manifest_loader() {
    let pack = pack_without_converter();
    let original = pack
        .source_bytes("cem-dom-projection", "1.0.0", DOM_MANIFEST)
        .unwrap()
        .to_vec();
    let mut context = dom_context(pack);
    let incumbent = context.schema_asset_activation.clone().unwrap();
    for (uri, bytes) in [
        (DOM_MANIFEST, [original.as_slice(), b"\n"].concat()),
        ("memory://different/package.cem", original),
    ] {
        // An exact owner replacement grant alone does not unpin source bytes.
        context
            .schema_package_replacement_grants
            .push(SchemaPackageReplacementGrant {
                package_id: "cem-dom-projection".into(),
                expected_origin: SchemaPackageOrigin::Manifest(DOM_MANIFEST.into()),
                replacement_manifest_uri: uri.into(),
            });
        let diagnostics = crate::real::load_schema_package_manifest_into_context(
            &mut context,
            &EngineInput {
                uri: uri.into(),
                bytes,
                from_format: None,
                identity: None,
                root_scope: Default::default(),
            },
        )
        .unwrap();
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == "cem.schema_package.activated_source_mismatch"),
            "{diagnostics:?}"
        );
        assert!(Arc::ptr_eq(
            &incumbent,
            context.schema_asset_activation.as_ref().unwrap()
        ));
        assert_eq!(
            context.schema_registry.package_origin("cem-dom-projection"),
            Some(&SchemaPackageOrigin::Manifest(DOM_MANIFEST.into()))
        );
    }
}
