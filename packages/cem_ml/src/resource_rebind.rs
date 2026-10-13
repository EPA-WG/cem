//! Worker control boundary for bounded, side-effect-free native candidate plans.
use crate::engine::FormatIdentity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceRebindRequest {
    pub kind: ResourceRebindKind,
    #[serde(default)]
    pub mode: ResourceEditMode,
    pub bytes: Vec<u8>,
    pub identity: FormatIdentity,
    pub base_uri: String,
    pub from_uri: String,
    pub to_uri: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceRebindKind {
    RunConfig,
    TransformGraph,
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceEditMode {
    #[default]
    Rebind,
    Relocate,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRebindPlan {
    pub references: Vec<ResourceReference>,
    pub blockers: Vec<String>,
    pub rewritten_bytes: Option<Vec<u8>>,
}

#[derive(Debug, Serialize)]
pub struct ResourceReference {
    pub uri: String,
    pub kind: String,
}

pub fn plan_resource_rebind(request: ResourceRebindRequest) -> Result<ResourceRebindPlan, String> {
    let ResourceRebindRequest {
        kind,
        mode,
        bytes,
        identity,
        base_uri,
        from_uri,
        to_uri,
    } = request;
    if mode == ResourceEditMode::Relocate && from_uri != base_uri {
        return Err("relocation source must equal the acquired configuration base".into());
    }
    match kind {
        ResourceRebindKind::RunConfig => {
            let parse_request = crate::run_config::RunConfigParseRequest {
                bytes,
                identity,
                base_uri: Some(base_uri),
            };
            let plan = match mode {
                ResourceEditMode::Rebind => crate::run_config_rebind::plan_run_config_rebind(
                    parse_request,
                    &from_uri,
                    &to_uri,
                ),
                ResourceEditMode::Relocate => {
                    crate::run_config_rebind::plan_run_config_relocation(parse_request, &to_uri)
                }
            }
            .map_err(|e| format!("{}: {}", e.code, e.message))?;
            Ok(ResourceRebindPlan {
                references: plan
                    .references
                    .into_iter()
                    .map(|r| ResourceReference {
                        uri: r.uri,
                        kind: match r.kind {
                            crate::run_config_rebind::RunConfigReferenceKind::Input => "input",
                            crate::run_config_rebind::RunConfigReferenceKind::SchemaPackage => {
                                "schemaPackage"
                            }
                            crate::run_config_rebind::RunConfigReferenceKind::InputLink => {
                                "inputLink"
                            }
                        }
                        .into(),
                    })
                    .collect(),
                blockers: plan
                    .blockers
                    .into_iter()
                    .map(|b| format!("{}: {}", b.path, b.reason))
                    .collect(),
                rewritten_bytes: plan.rewritten_bytes,
            })
        }
        ResourceRebindKind::TransformGraph => {
            let parse_request = crate::transform_config::TransformGraphParseRequest {
                bytes,
                identity,
                base_uri: Some(base_uri),
            };
            let plan = match mode {
                ResourceEditMode::Rebind => {
                    crate::transform_graph_rebind::plan_transform_graph_rebind(
                        parse_request,
                        &from_uri,
                        &to_uri,
                    )
                }
                ResourceEditMode::Relocate => {
                    crate::transform_graph_rebind::plan_transform_graph_relocation(
                        parse_request,
                        &to_uri,
                    )
                }
            }
            .map_err(|e| format!("{}: {}", e.code, e.message))?;
            Ok(ResourceRebindPlan {
                references: plan
                    .dependencies
                    .references
                    .into_iter()
                    .filter_map(|r| {
                        r.resolved_uri.map(|uri| ResourceReference {
                            uri,
                            kind: if r.role
                                == crate::transform_graph_dependencies::GraphReferenceRole::Output
                            {
                                "output"
                            } else {
                                "input"
                            }
                            .into(),
                        })
                    })
                    .collect(),
                blockers: plan.blockers,
                rewritten_bytes: plan.rewritten_bytes,
            })
        }
    }
}

pub fn plan_resource_rebind_json(json: &str) -> String {
    let result = serde_json::from_str(json)
        .map_err(|e| e.to_string())
        .and_then(plan_resource_rebind);
    match result {
        Ok(plan) => serde_json::to_string(&plan).expect("plain rebinding control records serialize"),
        Err(message) => serde_json::json!({"error": {"code": "cem.resource_rebind.invalid", "message": message}}).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configuration_relocation_wire_is_explicit_and_preserves_bindings() {
        let source = "{run | {import @src='../old.svg' @opaque=true @content-type=image/svg+xml | {export @out='../out.svg'}}}";
        let mut request = serde_json::json!({"mode":"relocate", "kind":"transform-graph",
            "bytes":source.as_bytes(), "identity":{"contentType":"text/cem-ml"},
            "baseUri":"studio://project/graphs/run.cem", "fromUri":"studio://project/graphs/run.cem",
            "toUri":"studio://project/deep/moved/run.cem"});
        let read = |request: &serde_json::Value| -> serde_json::Value {
            serde_json::from_str(&plan_resource_rebind_json(&request.to_string())).unwrap()
        };
        let plan = read(&request);
        assert_eq!(plan["blockers"], serde_json::json!([]));
        let bytes: Vec<u8> = serde_json::from_value(plan["rewrittenBytes"].clone()).unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            source
                .replace("../old.svg", "studio://project/old.svg")
                .replace("../out.svg", "studio://project/out.svg")
        );
        assert_eq!(plan["references"][0]["uri"], "studio://project/old.svg");
        request["fromUri"] = serde_json::json!("studio://project/other.cem");
        assert!(read(&request)["error"].is_object());
        request["fromUri"] = request["baseUri"].clone();
        request["mode"] = serde_json::json!("rebind");
        assert!(read(&request)["error"].is_object());
        request["mode"] = serde_json::json!("unknown");
        assert!(read(&request)["error"].is_object());
        request["mode"] = serde_json::json!("relocate");
        request["kind"] = serde_json::json!("run-config");
        request["identity"] = serde_json::json!({"contentType":"application/json", "schema":"https://cem.dev/ns/cli/run-config/1"});
        request["bytes"] =
            serde_json::json!(br#"{"inputs":[{"uri":"studio://project/old.svg"}]}"#.as_slice());
        assert_eq!(read(&request)["blockers"], serde_json::json!([]));
        assert!(read(&request)["rewrittenBytes"].is_null());
        request["bytes"] = serde_json::json!(br#"{"inputs":[{"uri":"old.svg"}]}"#.as_slice());
        assert!(!read(&request)["blockers"].as_array().unwrap().is_empty());
    }

    #[test]
    fn resource_rebind_wire_preserves_blockers_and_exact_candidate_bytes() {
        let source = "{run | {import @src='../old.svg' @opaque=true @content-type=image/svg+xml | {export @out='../out.svg'}}}";
        let request = serde_json::json!({"kind":"transform-graph", "bytes":source.as_bytes(), "identity":{"contentType":"text/cem-ml"}, "baseUri":"studio://project/graphs/run.cem", "fromUri":"studio://project/old.svg", "toUri":"studio://project/new.svg"});
        let plan: serde_json::Value =
            serde_json::from_str(&plan_resource_rebind_json(&request.to_string())).unwrap();
        assert_eq!(plan["blockers"], serde_json::json!([]));
        let bytes: Vec<u8> = serde_json::from_value(plan["rewrittenBytes"].clone()).unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            source.replace("../old.svg", "studio://project/new.svg")
        );
        let mut blocked = request;
        blocked["bytes"] =
            serde_json::json!(source.replace("@opaque=true", "@opaque=false").as_bytes());
        let plan: serde_json::Value =
            serde_json::from_str(&plan_resource_rebind_json(&blocked.to_string())).unwrap();
        assert!(!plan["blockers"].as_array().unwrap().is_empty());
        assert!(plan["rewrittenBytes"].is_null());
    }
    #[test]
    fn resource_rebind_wire_rejects_malformed_control_and_returns_run_config_inventory() {
        assert!(plan_resource_rebind_json("{}").contains("cem.resource_rebind.invalid"));
        let request = serde_json::json!({"kind":"run-config", "bytes":br#"{"inputs":[{"uri":"studio://project/old.cem"}]}"#.as_slice(), "identity":{"contentType":"application/json","schema":"https://cem.dev/ns/cli/run-config/1"}, "baseUri":"studio://project/run.json", "fromUri":"studio://project/old.cem", "toUri":"studio://project/new.cem"});
        let plan: serde_json::Value =
            serde_json::from_str(&plan_resource_rebind_json(&request.to_string())).unwrap();
        assert_eq!(
            plan["references"][0],
            serde_json::json!({"kind":"input","uri":"studio://project/old.cem"})
        );
        assert_eq!(plan["blockers"], serde_json::json!([]));
        assert!(plan["rewrittenBytes"].is_array());
    }
}
