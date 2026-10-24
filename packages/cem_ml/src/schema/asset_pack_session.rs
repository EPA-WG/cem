//! One embedding-owned package store and its atomically published consumer context.
//! Source bytes may accumulate; failed activation never changes the incumbent.
use super::{asset_pack::SchemaAssetPack, asset_pack_activation::activate_schema_asset_packages};
use crate::{
    diagnostics::Diagnostic,
    engine::{EngineContext, EngineResult},
    operation_control::OperationControl,
};
use std::{collections::BTreeMap, sync::Arc};

pub struct SchemaAssetSession {
    pack: Arc<SchemaAssetPack>,
    context: EngineContext,
    active: BTreeMap<String, String>,
}

impl SchemaAssetSession {
    pub fn new(pack: SchemaAssetPack, context: EngineContext) -> Self {
        Self {
            pack: Arc::new(pack),
            context,
            active: BTreeMap::new(),
        }
    }
    pub fn pack(&self) -> &SchemaAssetPack {
        &self.pack
    }
    pub fn context(&self) -> &EngineContext {
        &self.context
    }
    pub fn admit(
        &mut self,
        id: &str,
        version: &str,
        path: &str,
        bytes: Vec<u8>,
    ) -> Result<(), super::asset_pack::SchemaAssetPackError> {
        let mime = self
            .pack
            .source_descriptor(id, version, path)?
            .content_type
            .clone();
        if self
            .pack
            .source_bytes(id, version, path)
            .is_ok_and(|original| original == bytes.as_slice())
        {
            return Ok(());
        }
        let control = OperationControl::default();
        Arc::make_mut(&mut self.pack).admit_source(
            id,
            version,
            path,
            bytes,
            &mime,
            &control,
            control.root_scope(),
        )
    }
    pub fn activate(&mut self, packages: &[(&str, &str)]) -> EngineResult<Vec<Diagnostic>> {
        let mut seen = std::collections::BTreeSet::new();
        if packages.is_empty() || packages.iter().any(|(id, _)| !seen.insert(*id)) {
            return Err(crate::engine::EngineError::SchemaResolution(
                "Package selection must be nonempty and unique".into(),
            ));
        }
        let pending: Vec<_> = packages
            .iter()
            .copied()
            .filter(|(id, version)| self.active.get(*id).map(String::as_str) != Some(*version))
            .collect();
        if pending.is_empty()
            && self
                .context
                .schema_asset_activation
                .as_ref()
                .is_some_and(|binding| binding.references_pack(&self.pack))
        {
            return Ok(Vec::new());
        }
        let selected = if pending.is_empty() {
            packages
        } else {
            &pending
        };
        let diagnostics =
            activate_schema_asset_packages(&mut self.context, self.pack.clone(), selected)?;
        if !diagnostics.iter().any(|d| d.severity.is_hard_violation()) {
            self.active.extend(
                pending
                    .into_iter()
                    .map(|(id, version)| (id.into(), version.into())),
            );
        }
        Ok(diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        engine::SchemaPackageReplacementGrant,
        schema::{
            asset_pack::{
                builtin_schema_asset_pack_export, schema_asset_sha256, SchemaAssetPackLimits,
            },
            registry::SchemaPackageOrigin,
        },
    };

    #[test]
    fn persistent_session_retains_sources_and_publishes_only_complete_authorized_packages() {
        let export = builtin_schema_asset_pack_export().unwrap();
        let bytes = export.index.to_bytes().unwrap();
        let pack = SchemaAssetPack::from_index_bytes(
            &bytes,
            &schema_asset_sha256(&bytes),
            SchemaAssetPackLimits::default(),
        )
        .unwrap();
        let mut context = EngineContext::default();
        let mut unbound = EngineContext::default();
        let schema = crate::schema::package_sources::builtin_schema_package_source("csv").unwrap();
        unbound.schema_document_models.register(
            crate::schema::document_model::compile_schema_document_model(
                "https://cem.dev/ns/data/csv/1",
                schema.schema_source,
            ),
        );
        assert!(
            crate::import::import_data_bytes_with_context(
                b"id,name\n1,Ada\n",
                "text/csv",
                "cem",
                "memory://unbound.csv",
                Some(&unbound),
            )
            .is_err(),
            "An unbound parser cannot silently select the embedded contract"
        );
        context
            .schema_package_replacement_grants
            .push(SchemaPackageReplacementGrant {
                package_id: "csv".into(),
                expected_origin: SchemaPackageOrigin::Builtin,
                replacement_manifest_uri: "schema-packages/csv/v1/package.cem".into(),
            });
        let mut session = SchemaAssetSession::new(pack, context);
        assert!(session.activate(&[("csv", "1.0.0")]).is_err());
        assert!(session.context().schema_asset_activation.is_none());
        for source in export.sources.iter().filter(|s| {
            s.descriptor.package_id == "csv"
                && matches!(
                    s.descriptor.kind,
                    super::super::asset_pack::SchemaAssetSourceKind::Manifest
                        | super::super::asset_pack::SchemaAssetSourceKind::Schema
                )
        }) {
            session
                .admit(
                    "csv",
                    "1.0.0",
                    &source.descriptor.path,
                    source.source.as_bytes().to_vec(),
                )
                .unwrap();
        }
        assert!(!session
            .activate(&[("csv", "1.0.0")])
            .unwrap()
            .iter()
            .any(|d| d.severity.is_hard_violation()));
        assert!(session
            .context()
            .schema_asset_activation
            .as_ref()
            .unwrap()
            .read_import(
                "schema-packages/csv/v1/package.cem",
                "formatters/compact.cemt"
            )
            .unwrap()
            .is_err());
        for source in export
            .sources
            .iter()
            .filter(|s| s.descriptor.package_id == "csv")
        {
            session
                .admit(
                    "csv",
                    "1.0.0",
                    &source.descriptor.path,
                    source.source.as_bytes().to_vec(),
                )
                .unwrap();
        }
        let diagnostics = session.activate(&[("csv", "1.0.0")]).unwrap();
        assert!(
            !diagnostics.iter().any(|d| d.severity.is_hard_violation()),
            "{diagnostics:?}"
        );
        assert!(session
            .context()
            .schema_asset_activation
            .as_ref()
            .unwrap()
            .read_import(
                "schema-packages/csv/v1/package.cem",
                "formatters/compact.cemt"
            )
            .unwrap()
            .is_ok());
        let incumbent = session
            .context()
            .schema_asset_activation
            .as_ref()
            .unwrap()
            .clone();
        let retained = session.context().clone();
        assert!(session.activate(&[("csv", "1.0.0")]).unwrap().is_empty());
        assert!(crate::import::import_data_bytes_with_context(
            b"id,name\n1,Ada\n",
            "text/csv;header=present",
            "cem",
            "memory://owned.csv",
            Some(session.context()),
        )
        .is_ok());
        let error = crate::import::import_data_bytes_with_context(
            b"id,name\n1,\"Ada\n",
            "text/csv;header=present",
            "cem",
            "memory://owned.csv",
            Some(session.context()),
        )
        .unwrap_err();
        assert!(error.contains("cem.csv.unclosed_quote"), "{error}");
        // Runtime admission did not grant a packaging-fixture validation pass.
        // A full validation still needs the declared artifact/example resources.
        use crate::engine::{
            CemMlEngine, EngineInput, FailLevel, FormatIdentity, ValidateProjection,
            ValidateRequest,
        };
        let manifest = export
            .sources
            .iter()
            .find(|source| source.descriptor.path == "schema-packages/csv/v1/package.cem")
            .unwrap();
        let validation = crate::real::RealCemMlEngine
            .validate(ValidateRequest {
                inputs: vec![EngineInput {
                    uri: "memory://unresolved/package.cem".into(),
                    bytes: manifest.source.as_bytes().to_vec(),
                    from_format: Some(crate::engine::InputFormat::Cem),
                    identity: Some(FormatIdentity {
                        content_type: Some("application/vnd.cem.schema-package+cem".into()),
                        schema: Some("https://cem.dev/ns/schema-package/1".into()),
                        ..Default::default()
                    }),
                    root_scope: crate::run_config::ScopeConfig {
                        schema: Some("https://cem.dev/ns/schema-package/1".into()),
                        default_content_type: Some("application/vnd.cem.schema-package+cem".into()),
                        ..Default::default()
                    },
                }],
                projection: ValidateProjection::Text,
                fail_level: FailLevel::Validate,
                context: session.context().clone(),
            })
            .unwrap();
        assert!(
            validation
                .report
                .diagnostics
                .iter()
                .any(|d| d.code == "cem.schema_package.example_check"),
            "{:?}",
            validation.report.diagnostics
        );
        assert!(session.activate(&[("csv", "unknown")]).is_err());
        assert!(Arc::ptr_eq(
            &incumbent,
            session.context().schema_asset_activation.as_ref().unwrap()
        ));
        drop(session);
        assert!(retained
            .schema_asset_activation
            .unwrap()
            .owns_package("csv"));
    }
}
