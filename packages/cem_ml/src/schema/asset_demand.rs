//! Advisory package identities needed by an already lowered native command.
//! No source bodies, native owners, host grants or activation authority cross
//! this boundary. Original package admission remains a separate host action.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::package_sources::OPTIONAL_SCHEMA_ASSET_PACKAGES;
use super::registry::{SchemaDescriptor, SchemaRegistry};
use crate::command_operation::{
    command_schema_demand_identities, CommandOperationPreparationError,
};
use crate::command_service::{
    validate_command_service_request_v1, CommandServiceLimitsV1, CommandServiceRequestV1,
};
use crate::engine::FormatIdentity;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaAssetDemandIdentity {
    pub package_id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SchemaAssetDemandPlan {
    pub kind: &'static str,
    pub packages: Vec<SchemaAssetDemandIdentity>,
}

pub fn plan_command_schema_assets(
    request: &CommandServiceRequestV1,
) -> Result<SchemaAssetDemandPlan, CommandOperationPreparationError> {
    validate_command_service_request_v1(request, CommandServiceLimitsV1::default())?;
    // The native registry is the complete identity catalog, including when
    // optional original bodies have been excluded from this binary.
    let registry = SchemaRegistry::with_builtin_schemas();
    let mut packages = BTreeMap::new();
    let mut visited = BTreeSet::new();
    for identity in command_schema_demand_identities(request)? {
        for descriptor in descriptors_for_identity(&registry, &identity) {
            collect_dependencies(&registry, descriptor, &mut visited, &mut packages);
        }
    }
    Ok(SchemaAssetDemandPlan {
        kind: "cem-schema-asset-demand-plan/1",
        packages: packages
            .into_iter()
            .map(|(package_id, version)| SchemaAssetDemandIdentity {
                package_id,
                version,
            })
            .collect(),
    })
}

fn descriptors_for_identity<'a>(
    registry: &'a SchemaRegistry,
    identity: &FormatIdentity,
) -> Vec<&'a SchemaDescriptor> {
    // Match the explicit-schema precedence of native missing-source admission.
    if let Some(schema) = identity.schema.as_deref().map(str::trim) {
        if let Some(descriptor) = registry.schema(schema) {
            return vec![descriptor];
        }
        // Native lifecycle adapters also accept their registered namespace
        // identities as schema aliases (for example the XSLT namespace).
        if let [descriptor] = registry.lookup_namespace(schema).as_slice() {
            return vec![*descriptor];
        }
    }
    if let Some(content_type) = identity.content_type.as_deref() {
        if let Ok(descriptor) = registry.resolve_content_type(content_type) {
            return vec![descriptor];
        }
        // Reuse the native import boundary's aliases and MIME normalization;
        // the optional package catalog itself need not duplicate those aliases.
        if let Ok((_, canonical)) = crate::import::import_content_type(content_type) {
            if let Ok(descriptor) = registry.resolve_content_type(&canonical) {
                return vec![descriptor];
            }
        }
        return Vec::new();
    }
    let mut descriptors = BTreeMap::new();
    for namespace in identity
        .default_namespace
        .iter()
        .chain(identity.namespaces.values())
    {
        let matches = registry.lookup_namespace(namespace.trim());
        if let [descriptor] = matches.as_slice() {
            descriptors.insert(descriptor.package_id.clone(), *descriptor);
        }
    }
    descriptors.into_values().collect()
}

fn collect_dependencies(
    registry: &SchemaRegistry,
    descriptor: &SchemaDescriptor,
    visited: &mut BTreeSet<String>,
    packages: &mut BTreeMap<String, String>,
) {
    if !visited.insert(descriptor.package_id.clone()) {
        return;
    }
    if OPTIONAL_SCHEMA_ASSET_PACKAGES.contains(&descriptor.package_id.as_str()) {
        packages.insert(descriptor.package_id.clone(), descriptor.version.clone());
    }
    for dependency in &descriptor.uses {
        if let Some(descriptor) = registry.schema(dependency) {
            collect_dependencies(registry, descriptor, visited, packages);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_invocation::{
        build_command_invocation_v1, CommandInvocationBuildResponseV1,
        CommandInvocationEnvironmentV1, ParsedCommandInvocationV1, ParsedCommandValueV1,
        COMMAND_SCHEMA_VERSION,
    };
    use crate::command_service::{CommandUriMapV1, VirtualResourceV1};

    fn request(
        command: &str,
        input: &str,
        content_type: &str,
        options: &[(&str, &str)],
        resources: &[(&str, &[u8])],
    ) -> CommandServiceRequestV1 {
        let mut parsed = ParsedCommandInvocationV1 {
            schema_version: COMMAND_SCHEMA_VERSION,
            common_version: crate::VERSION.into(),
            command_path: vec![command.into()],
            global_options: BTreeMap::new(),
            options: BTreeMap::new(),
            positionals: BTreeMap::new(),
            meta_action: None,
        };
        if command == "transform" {
            parsed
                .options
                .insert("config".into(), ParsedCommandValueV1::String(input.into()));
        } else {
            parsed
                .positionals
                .insert("input".into(), ParsedCommandValueV1::String(input.into()));
        }
        if !content_type.is_empty() {
            parsed.options.insert(
                "content_type".into(),
                ParsedCommandValueV1::String(content_type.into()),
            );
        }
        for (key, value) in options {
            parsed
                .options
                .insert((*key).into(), ParsedCommandValueV1::String((*value).into()));
        }
        let environment = CommandInvocationEnvironmentV1 {
            request_id: "demand-fixture".into(),
            project_id: "project".into(),
            project_revision: 1,
            resource_revision: 1,
            cwd: "studio://project/".into(),
            resolver_policy_stamp: "fixture".into(),
            safety_policy_stamp: "fixture".into(),
            budget_policy_stamp: "fixture".into(),
            stdout_is_terminal: false,
        };
        let resources = resources
            .iter()
            .map(|(path, bytes)| {
                (
                    format!("studio://project/{path}"),
                    VirtualResourceV1 {
                        bytes: bytes.to_vec(),
                        identity: None,
                    },
                )
            })
            .collect::<CommandUriMapV1<_>>();
        match build_command_invocation_v1(parsed, environment, resources) {
            CommandInvocationBuildResponseV1::Ready { invocation } => invocation.request,
            result => panic!("fixture command did not lower: {result:?}"),
        }
    }

    fn identities(request: &CommandServiceRequestV1) -> Vec<String> {
        plan_command_schema_assets(request)
            .unwrap()
            .packages
            .into_iter()
            .map(|package| format!("{}@{}", package.package_id, package.version))
            .collect()
    }

    #[test]
    fn schema_asset_demand_html_and_unrelated_csv_resource_do_not_load_optional_packages() {
        let request = request(
            "parse",
            "view.html",
            "text/html",
            &[("format", "ast")],
            &[
                ("view.html", b"<p>Ready</p>"),
                ("unused.csv", b"secret,password\nAda,private\n"),
            ],
        );
        assert!(identities(&request).is_empty());
    }

    #[test]
    fn schema_asset_demand_parse_does_not_activate_unused_output_target_identity() {
        let mut request = request(
            "parse",
            "view.html",
            "text/html",
            &[("format", "ast")],
            &[("view.html", b"<p>Ready</p>")],
        );
        let super::super::super::command_service::CommandRunPlanV1::Plan(plan) =
            &mut request.run_plan
        else {
            panic!("plan");
        };
        assert!(!plan.outputs.is_empty());
        for output in &mut plan.outputs {
            output.identity.content_type = Some("text/x-scss".to_owned());
            output.identity.schema = Some("https://cem.dev/ns/data/scss/1".to_owned());
        }
        assert!(identities(&request).is_empty());
    }

    #[test]
    fn schema_asset_demand_metadata_provider_keeps_only_module_map_control_bodies() {
        use crate::resolver::ResolvePurpose;
        use crate::transform_graph_request::{
            ManifestTransformGraphResourceProvider, TransformGraphResourceProvider,
        };
        let body = vec![b'x'; 2 * 1024 * 1024];
        let map = br#"{"imports":{}}"#.to_vec();
        let resources = BTreeMap::from([
            (
                "studio://project/data.json".into(),
                VirtualResourceV1 {
                    bytes: body.clone(),
                    identity: None,
                },
            ),
            (
                "studio://project/template.cemt".into(),
                VirtualResourceV1 {
                    bytes: body,
                    identity: None,
                },
            ),
            (
                "studio://project/map.json".into(),
                VirtualResourceV1 {
                    bytes: map.clone(),
                    identity: None,
                },
            ),
        ]);
        let provider = ManifestTransformGraphResourceProvider::for_demand(
            "studio://project/graph.cem",
            &resources,
        );
        assert!(provider.metadata_only());
        assert!(provider.import_resources("data.json", None).unwrap()[0]
            .bytes
            .is_empty());
        assert!(provider
            .read_resource("template.cemt", ResolvePurpose::Template, None)
            .unwrap()
            .bytes
            .is_empty());
        assert_eq!(
            provider
                .read_resource("map.json", ResolvePurpose::ModuleMap, None)
                .unwrap()
                .bytes,
            map
        );
    }

    #[test]
    fn schema_asset_demand_native_csv_aliases_parameters_and_schema_resolve_once() {
        for content_type in ["text/csv", "csv", "TEXT/CSV;header=present"] {
            let request = request(
                "parse",
                "input.csv",
                content_type,
                &[("format", "ast")],
                &[("input.csv", b"id,name\n1,Ada\n")],
            );
            assert_eq!(identities(&request), ["csv@1.0.0"]);
        }
        let request = request(
            "parse",
            "input.cem",
            "application/cem",
            &[
                ("format", "ast"),
                ("schema", super::super::registry::CSV_SCHEMA_URI),
            ],
            &[("input.cem", b"{csv-document}")],
        );
        assert_eq!(identities(&request), ["csv@1.0.0"]);
    }

    #[test]
    fn schema_asset_demand_json_to_csv_and_duplicate_inputs_keep_only_identity() {
        let request = request(
            "convert",
            "input.json",
            "application/json",
            &[
                ("to_content_type", "text/csv"),
                ("to_schema", super::super::registry::CSV_SCHEMA_URI),
            ],
            &[("input.json", b"[{\"password\":\"do-not-export\"}]")],
        );
        let plan = plan_command_schema_assets(&request).unwrap();
        assert_eq!(
            plan.packages,
            [SchemaAssetDemandIdentity {
                package_id: "csv".into(),
                version: "1.0.0".into()
            }]
        );
        let json = serde_json::to_string(&plan).unwrap();
        assert_eq!(
            json,
            r#"{"kind":"cem-schema-asset-demand-plan/1","packages":[{"packageId":"csv","version":"1.0.0"}]}"#
        );
        assert!(
            !json.contains("password")
                && !json.contains("do-not-export")
                && !json.contains("grant")
        );
    }

    #[test]
    fn schema_asset_demand_convert_enum_fallback_respects_explicit_target_override() {
        let fallback = request(
            "convert",
            "input.json",
            "application/json",
            &[("to_format", "csv")],
            &[("input.json", b"[]")],
        );
        assert_eq!(identities(&fallback), ["csv@1.0.0"]);
        let explicit = request(
            "convert",
            "input.json",
            "application/json",
            &[
                ("to_format", "csv"),
                ("to_content_type", "application/json"),
            ],
            &[("input.json", b"[]")],
        );
        assert!(identities(&explicit).is_empty());
    }

    #[test]
    fn schema_asset_demand_unknown_convert_target_preserves_native_csv_fallback() {
        let fallback = request(
            "convert",
            "input.json",
            "application/json",
            &[
                ("to_format", "csv"),
                ("to_content_type", "application/vnd.unknown"),
            ],
            &[("input.json", b"[]")],
        );
        assert_eq!(identities(&fallback), ["csv@1.0.0"]);
    }

    #[test]
    fn schema_asset_demand_ignored_scss_output_does_not_override_native_fallback() {
        let fallback = request(
            "convert",
            "input.json",
            "application/json",
            &[
                ("to_format", "dom-json"),
                ("to_content_type", "text/x-scss"),
            ],
            &[("input.json", b"[]")],
        );
        assert!(identities(&fallback).is_empty());
    }

    #[test]
    fn schema_asset_demand_trims_native_schema_selectors_and_resolves_xslt_schema_alias() {
        let padded = request(
            "parse",
            "input.txt",
            "",
            &[
                ("format", "ast"),
                ("schema", " https://cem.dev/ns/data/csv/1 "),
            ],
            &[("input.txt", b"id\n1\n")],
        );
        assert_eq!(identities(&padded), ["csv@1.0.0"]);
        let alias = request(
            "parse",
            "input.txt",
            "",
            &[
                ("format", "ast"),
                ("schema", "http://www.w3.org/1999/XSL/Transform"),
            ],
            &[("input.txt", b"<xsl:stylesheet/>")],
        );
        assert_eq!(identities(&alias), ["xslt@1.0.0"]);
        let registry = SchemaRegistry::with_builtin_schemas();
        let identity = FormatIdentity {
            default_namespace: Some(" http://www.w3.org/1999/XSL/Transform ".into()),
            ..Default::default()
        };
        assert_eq!(
            descriptors_for_identity(&registry, &identity)[0].package_id,
            "xslt"
        );
    }

    #[test]
    fn schema_asset_demand_graph_expands_native_csv_import_and_output_identity() {
        let graph = br#"{run | {import @id=source @src="input.json" @content-type="application/json" | {convert @id=csv @content-type="text/csv" @schema="https://cem.dev/ns/data/csv/1" | {export @out="output.csv" @content-type="text/csv"}}}}"#;
        let request = request(
            "transform",
            "graph.cem",
            "",
            &[],
            &[("graph.cem", graph), ("input.json", b"[{\"id\":1}]")],
        );
        assert_eq!(identities(&request), ["csv@1.0.0"]);
    }

    #[test]
    fn schema_asset_demand_graph_glob_inference_deduplicates_and_opaque_copy_needs_no_parser() {
        let graph = br#"{run | {import @src="tables/*.csv" | {export @out="out/{file}"}}}"#;
        let request = request(
            "transform",
            "graph.cem",
            "",
            &[],
            &[
                ("graph.cem", graph),
                ("tables/a.csv", b"id\n1\n"),
                ("tables/b.csv", b"id\n2\n"),
            ],
        );
        assert_eq!(identities(&request), ["csv@1.0.0"]);
        let graph = br#"{run | {import @src="input.csv" @content-type="text/csv" @opaque=true | {export @out="out.csv"}}}"#;
        let request = self::request(
            "transform",
            "graph.cem",
            "",
            &[],
            &[
                ("graph.cem", graph),
                ("input.csv", b"unparsed private body"),
            ],
        );
        assert!(identities(&request).is_empty());
    }

    #[test]
    fn schema_asset_demand_unknown_identity_is_no_grant_and_requests_remain_bounded() {
        let mut request = request(
            "parse",
            "input.unknown",
            "application/vnd.unknown",
            &[("format", "ast")],
            &[("input.unknown", b"private")],
        );
        assert!(identities(&request).is_empty());
        request.protocol_version = 42;
        assert!(plan_command_schema_assets(&request).is_err());
    }

    #[test]
    fn schema_asset_demand_plans_ungranted_native_identities_without_authority_or_bytes() {
        for (content_type, package_id) in [
            ("text/markdown; charset=utf-8", "markdown"),
            ("text/x-scss", "scss"),
            ("application/relax-ng+xml", "relax-ng"),
            ("application/xslt+xml", "xslt"),
        ] {
            let request = request(
                "parse",
                "input.txt",
                content_type,
                &[("format", "ast")],
                &[("input.txt", b"sensitive-body")],
            );
            let plan = plan_command_schema_assets(&request).unwrap();
            assert_eq!(plan.packages[0].package_id, package_id);
            let json = serde_json::to_string(&plan).unwrap();
            assert!(
                !json.contains("sensitive-body")
                    && !json.contains("owner")
                    && !json.contains("grant")
            );
        }
        let mut request = request(
            "parse",
            "input.csv",
            "text/csv",
            &[("format", "ast")],
            &[("input.csv", b"id\n1\n")],
        );
        request
            .resources
            .get_mut("studio://project/input.csv")
            .unwrap()
            .bytes
            .push(b'2');
        assert!(
            plan_command_schema_assets(&request).is_err(),
            "a changed inline resource must fail the ordinary digest checks"
        );
    }
}
