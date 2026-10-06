//! Trusted embedding-host setup, separate from command request data.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
pub struct CommandHostConfigurationV1 {
    #[serde(default)]
    pub schema_package_replacement_grants: Vec<CommandHostReplacementGrantV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
pub struct CommandHostReplacementGrantV1 {
    pub package_id: String,
    pub expected_origin: CommandHostPackageOriginV1,
    pub replacement_manifest_uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
pub enum CommandHostPackageOriginV1 {
    Builtin {},
    Manifest { uri: String },
    Untracked {},
}

/// Decode only an explicit trusted-host argument. Requests/configs must never
/// call this helper to manufacture authority. Omission retains legacy behavior.
pub fn decode_command_host_configuration_v1(
    source: Option<&str>,
) -> Result<CommandHostConfigurationV1, String> {
    let config: CommandHostConfigurationV1 = match source {
        Some(source) => {
            // JSON is an explicit host control boundary. Serde structs also
            // admit positional arrays; authority must use named object fields.
            let value: serde_json::Value =
                serde_json::from_str(source).map_err(|error| error.to_string())?;
            if !value.is_object() {
                return Err("host configuration must be a JSON object".into());
            }
            if let Some(grants) = value
                .get("schemaPackageReplacementGrants")
                .and_then(|value| value.as_array())
            {
                if grants.iter().any(|grant| !grant.is_object()) {
                    return Err("replacement grants must be JSON objects".into());
                }
            }
            serde_json::from_str(source).map_err(|error| error.to_string())?
        }
        None => CommandHostConfigurationV1::default(),
    };
    for grant in &config.schema_package_replacement_grants {
        if grant.package_id.trim().is_empty()
            || grant.replacement_manifest_uri.trim().is_empty()
            || matches!(&grant.expected_origin, CommandHostPackageOriginV1::Manifest { uri } if uri.trim().is_empty())
        {
            return Err(
                "replacement grants require nonempty package and manifest identities".into(),
            );
        }
    }
    Ok(config)
}

impl CommandHostConfigurationV1 {
    /// Preserve exact caller identities. The shared engine checks ownership;
    /// authority does not load a manifest or mutate another execution's context.
    pub fn apply_to_context(&self, context: &mut crate::engine::EngineContext) {
        use crate::schema::registry::SchemaPackageOrigin;
        context.schema_package_replacement_grants.extend(
            self.schema_package_replacement_grants.iter().map(|grant| {
                crate::engine::SchemaPackageReplacementGrant {
                    package_id: grant.package_id.clone(),
                    expected_origin: match &grant.expected_origin {
                        CommandHostPackageOriginV1::Builtin {} => SchemaPackageOrigin::Builtin,
                        CommandHostPackageOriginV1::Untracked {} => SchemaPackageOrigin::Untracked,
                        CommandHostPackageOriginV1::Manifest { uri } => {
                            SchemaPackageOrigin::Manifest(uri.clone())
                        }
                    },
                    replacement_manifest_uri: grant.replacement_manifest_uri.clone(),
                }
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::registry::SchemaPackageOrigin;

    #[test]
    fn exact_host_grants_enter_context_without_loading_manifests() {
        for (origin, expected) in [
            (r#"{"kind":"builtin"}"#, SchemaPackageOrigin::Builtin),
            (r#"{"kind":"untracked"}"#, SchemaPackageOrigin::Untracked),
            (
                r#"{"kind":"manifest","uri":"vendor://old/package.cem"}"#,
                SchemaPackageOrigin::Manifest("vendor://old/package.cem".into()),
            ),
        ] {
            let json = format!(
                r#"{{"schemaPackageReplacementGrants":[{{"packageId":"vendor","expectedOrigin":{origin},"replacementManifestUri":"vendor://new/package.cem"}}]}}"#
            );
            let config = decode_command_host_configuration_v1(Some(&json)).unwrap();
            let mut context = crate::engine::EngineContext::default();
            config.apply_to_context(&mut context);
            assert_eq!(
                context.schema_package_replacement_grants,
                vec![crate::engine::SchemaPackageReplacementGrant {
                    package_id: "vendor".into(),
                    expected_origin: expected,
                    replacement_manifest_uri: "vendor://new/package.cem".into(),
                }]
            );
            assert!(context.schema_package_manifests.is_empty());
        }
        for source in [
            None,
            Some("{}"),
            Some(r#"{"schemaPackageReplacementGrants":[]}"#),
        ] {
            let config = decode_command_host_configuration_v1(source).unwrap();
            let mut context = crate::engine::EngineContext::default();
            config.apply_to_context(&mut context);
            assert!(context.schema_package_replacement_grants.is_empty());
        }
    }

    #[test]
    fn host_configuration_rejects_ambiguous_authority() {
        for origin in [
            r#"{"kind":"any"}"#,
            r#"{"kind":"builtin","uri":"extra"}"#,
            r#"{"kind":"manifest","uri":" "}"#,
        ] {
            let json = format!(
                r#"{{"schemaPackageReplacementGrants":[{{"packageId":"vendor","expectedOrigin":{origin},"replacementManifestUri":"vendor://new"}}]}}"#
            );
            assert!(decode_command_host_configuration_v1(Some(&json)).is_err());
        }
        for source in [
            "null",
            "[]",
            r#"{"schemaPackageReplacementGrants":[["vendor",{"kind":"builtin"},"vendor://new"]]}"#,
            r#"{"extra":true}"#,
            r#"{"schemaPackageReplacementGrants":[],"schemaPackageReplacementGrants":[]}"#,
            r#"{"schemaPackageReplacementGrants":[{"packageId":" ","expectedOrigin":{"kind":"builtin"},"replacementManifestUri":"vendor://new"}]}"#,
            r#"{"schemaPackageReplacementGrants":[{"packageId":"vendor","expectedOrigin":{"kind":"builtin"},"replacementManifestUri":""}]}"#,
            r#"{"schemaPackageReplacementGrants":[{"packageId":"vendor","expectedOrigin":{"kind":"builtin"},"replacementManifestUri":"vendor://new","extra":true}]}"#,
        ] {
            assert!(
                decode_command_host_configuration_v1(Some(source)).is_err(),
                "{source}"
            );
        }
    }
}
