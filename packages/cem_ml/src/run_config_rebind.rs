//! Side-effect-free, bounded rebinding plans for the explicit JSON run-config boundary.

use crate::run_config::{
    parse_run_config, validate_run_config, RunConfigError, RunConfigParseRequest,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunConfigReferenceKind {
    Input,
    SchemaPackage,
    InputLink,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunConfigReference {
    /// JSON pointer into the authored control document, never a document AST path.
    pub path: String,
    pub kind: RunConfigReferenceKind,
    pub uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunConfigRebindBlocker {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunConfigRebindPlan {
    pub references: Vec<RunConfigReference>,
    pub blockers: Vec<RunConfigRebindBlocker>,
    /// Present only when the complete admitted config can be changed. A no-op or
    /// blocked plan supplies no replacement: the host retains exact source bytes.
    pub rewritten_bytes: Option<Vec<u8>>,
}

/// Plan exact absolute-URI rebinding without fetching, executing or mutating a
/// project. This is a single configuration plan, not project-wide authorization.
/// Hosts must inspect blockers, discover transitive dependencies, validate the
/// whole project and compare revisions before committing any replacement bytes.
///
/// Config bases are deliberately NOT used to resolve inputs: execution currently
/// gives relative inputs and module maps different resolution rules. Guessing a
/// config-relative base here could silently rebind a different resource.
pub fn plan_run_config_rebind(
    request: RunConfigParseRequest,
    from_uri: &str,
    to_uri: &str,
) -> Result<RunConfigRebindPlan, RunConfigError> {
    if !exact_resource_uri(from_uri) || !exact_resource_uri(to_uri) {
        return Err(error("rename endpoints require canonical absolute resource URLs without query, fragment or glob"));
    }
    // Typed parsing first rejects duplicate known fields and wrong identities.
    let config = parse_run_config(request.clone())?.config;
    require_valid(&config, request.base_uri.as_deref())?;
    // JSON here is the named run-config control format, never an AST handoff.
    // Retain authored field presence; do not serialize typed defaults into it.
    let mut authored: Value =
        serde_json::from_slice(&request.bytes).map_err(|e| error(e.to_string()))?;
    let mut plan = RunConfigRebindPlan {
        references: Vec::new(),
        blockers: Vec::new(),
        rewritten_bytes: None,
    };
    check_keys(
        &authored,
        "",
        &[
            "inputs",
            "outputs",
            "schemaPackages",
            "resolvers",
            "scheduler",
        ],
        &mut plan,
    );
    if let Some(scheduler) = authored.get("scheduler") {
        check_keys(
            scheduler,
            "/scheduler",
            &["threadPool", "maxParallelDocuments"],
            &mut plan,
        );
    }
    if !config.resolvers.is_empty() {
        block(
            &mut plan,
            "/resolvers",
            "resolver ownership and transitive dependencies require a separate plan",
        );
    }
    for (collection, kind) in [
        ("inputs", RunConfigReferenceKind::Input),
        ("schemaPackages", RunConfigReferenceKind::SchemaPackage),
        ("outputs", RunConfigReferenceKind::InputLink),
    ] {
        if let Some(items) = authored.get(collection).and_then(Value::as_array) {
            for (index, item) in items.iter().enumerate() {
                let path = format!("/{collection}/{index}");
                let output = collection == "outputs";
                let allowed: &[&str] = if output {
                    &["inputRef", "destination", "rootScope"]
                } else {
                    &["uri", "rootScope"]
                };
                check_keys(item, &path, allowed, &mut plan);
                let field = if output { "inputRef" } else { "uri" };
                if let Some(uri) = item.get(field).and_then(Value::as_str) {
                    let pointer = format!("{path}/{field}");
                    plan.references.push(RunConfigReference {
                        path: pointer.clone(),
                        kind,
                        uri: uri.into(),
                    });
                    if !exact_resource_uri(uri) {
                        block(&mut plan, &pointer, "relative, noncanonical or selector-bearing reference requires execution-aligned resolution");
                    }
                }
                if item.get("destination").is_some_and(|v| !v.is_null()) {
                    block(
                        &mut plan,
                        &format!("{path}/destination"),
                        "output ownership and collision checks require a separate plan",
                    );
                }
                if let Some(scope) = item.get("rootScope") {
                    let path = format!("{path}/rootScope");
                    check_keys(
                        scope,
                        &path,
                        &[
                            "defaultContentType",
                            "schema",
                            "outputColorType",
                            "cemtFormatter",
                            "cemtFormatterProfile",
                            "cemtFormatterOptions",
                            "cemtColorizer",
                            "cemtColorProfile",
                            "versionPins",
                            "defaultNamespace",
                            "namespaces",
                            "moduleMap",
                            "baseUri",
                            "policy",
                            "budgets",
                        ],
                        &mut plan,
                    );
                    for field in [
                        "moduleMap",
                        "baseUri",
                        "cemtFormatter",
                        "cemtFormatterProfile",
                        "cemtColorizer",
                        "cemtColorProfile",
                    ] {
                        if scope.get(field).is_some_and(|v| !v.is_null()) {
                            block(
                                &mut plan,
                                &format!("{path}/{field}"),
                                "scope resolution or pipeline dependency requires a separate plan",
                            );
                        }
                    }
                    if scope
                        .get("cemtFormatterOptions")
                        .and_then(Value::as_object)
                        .is_some_and(|m| !m.is_empty())
                    {
                        block(
                            &mut plan,
                            &format!("{path}/cemtFormatterOptions"),
                            "formatter option dependencies require a separate plan",
                        );
                    }
                }
            }
        }
    }
    if !plan.blockers.is_empty() || from_uri == to_uri {
        return Ok(plan);
    }
    let mut changed = false;
    for reference in &plan.references {
        if reference.uri == from_uri {
            *authored
                .pointer_mut(&reference.path)
                .expect("pointer discovered in authored config") = Value::String(to_uri.into());
            changed = true;
        }
    }
    if changed {
        let bytes = serde_json::to_vec_pretty(&authored).map_err(|e| error(e.to_string()))?;
        let mut rebound = request;
        rebound.bytes = bytes.clone();
        let base = rebound.base_uri.clone();
        let config = parse_run_config(rebound)?.config;
        require_valid(&config, base.as_deref())?;
        plan.rewritten_bytes = Some(bytes);
    }
    Ok(plan)
}

/// Plan a configuration move without rebasing its input identities.
/// Admits only absolute keys with no scope-base, resolver, module-map, schema-package
/// or output-destination dependencies. Exact source bytes are retained. Comparison
/// uses empty invocation defaults; hosts must separately prove effective invocation
/// defaults, referenced document closure, incoming references and destination ownership.
/// This candidate does not authorize acquisition, mutation or execution.
pub fn plan_run_config_relocation(
    request: RunConfigParseRequest,
    to_uri: &str,
) -> Result<RunConfigRebindPlan, RunConfigError> {
    let from = request
        .base_uri
        .clone()
        .ok_or_else(|| error("configuration base required"))?;
    if !exact_resource_uri(&from) || !exact_resource_uri(to_uri) {
        return Err(error(
            "relocation requires canonical absolute configuration URLs",
        ));
    }
    if request
        .identity
        .base_uri
        .as_deref()
        .is_some_and(|base| base != from)
    {
        return Err(error(
            "identity base conflicts with acquired configuration base",
        ));
    }
    let mut plan = plan_run_config_rebind(request.clone(), &from, &from)?;
    for reference in plan.references.clone() {
        if reference.uri == from || reference.uri == to_uri {
            block(
                &mut plan,
                &reference.path,
                "configuration self or destination reference requires ownership analysis",
            );
        }
        if reference.kind == RunConfigReferenceKind::SchemaPackage {
            block(
                &mut plan,
                &reference.path,
                "schema package requires transitive dependency analysis",
            );
        }
    }
    if !plan.blockers.is_empty() {
        return Ok(plan);
    }
    let before = parse_run_config(request.clone())?.config;
    let mut moved = request;
    moved.base_uri = Some(to_uri.into());
    if moved.identity.base_uri.is_some() {
        moved.identity.base_uri = Some(to_uri.into());
    }
    let after = parse_run_config(moved)?.config;
    use crate::run_config::{normalize_run_config, RunConfigDefaults};
    let before = normalize_run_config(before, RunConfigDefaults::default(), Some(&from));
    let after = normalize_run_config(after, RunConfigDefaults::default(), Some(to_uri));
    if before.config != after.config || !after.diagnostics.is_empty() {
        return Err(error(
            "configuration relocation changed normalized execution settings",
        ));
    }
    Ok(plan)
}

fn exact_resource_uri(value: &str) -> bool {
    if value.contains(['*', '[', ']', '{', '}', '\\']) {
        return false;
    }
    url::Url::parse(value).is_ok_and(|url| {
        !url.cannot_be_a_base()
            && url.query().is_none()
            && url.fragment().is_none()
            && url.as_str() == value
    })
}

fn check_keys(value: &Value, path: &str, allowed: &[&str], plan: &mut RunConfigRebindPlan) {
    if let Some(object) = value.as_object() {
        for key in object.keys() {
            if !allowed.contains(&key.as_str()) {
                let escaped = key.replace('~', "~0").replace('/', "~1");
                block(
                    plan,
                    &format!("{path}/{escaped}"),
                    "unknown configuration field may own dependencies",
                );
            }
        }
    } else {
        block(plan, path, "configuration record requires a JSON object");
    }
}

fn block(plan: &mut RunConfigRebindPlan, path: &str, reason: &str) {
    plan.blockers.push(RunConfigRebindBlocker {
        path: path.into(),
        reason: reason.into(),
    });
}

fn require_valid(
    config: &crate::run_config::RunConfig,
    base: Option<&str>,
) -> Result<(), RunConfigError> {
    if let Some(diagnostic) = validate_run_config(config, base)
        .into_iter()
        .find(|d| d.severity.is_hard_violation())
    {
        return Err(error(format!(
            "{}: {}",
            diagnostic.code, diagnostic.message
        )));
    }
    Ok(())
}

fn error(message: impl Into<String>) -> RunConfigError {
    RunConfigError {
        code: "cem.run_config.rebind_invalid",
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::FormatIdentity;
    use crate::run_config::{RunConfigParseRequest, RUN_CONFIG_SCHEMA_URI};

    const OLD: &str = "studio://project/data/old.xml";
    const NEW: &str = "studio://project/data/new.xml";

    fn request(source: &str) -> RunConfigParseRequest {
        RunConfigParseRequest {
            bytes: source.as_bytes().to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".into()),
                schema: Some(RUN_CONFIG_SCHEMA_URI.into()),
                ..Default::default()
            },
            base_uri: Some("studio://project/config/run.json".into()),
        }
    }

    #[test]
    fn configuration_relocation_retains_absolute_run_config_bytes_and_blocks_relative_keys() {
        let source = format!("{{\r\n  \"inputs\": [{{\"uri\":\"{OLD}\"}}], \"outputs\": [{{\"inputRef\":\"{OLD}\"}}]\r\n}}\r\n");
        let original = request(&source);
        let plan = plan_run_config_relocation(original.clone(), "studio://project/moved/run.json")
            .unwrap();
        assert!(plan.blockers.is_empty());
        assert!(plan.rewritten_bytes.is_none());
        assert_eq!(original.bytes, source.as_bytes());
        for source in [
            r#"{"inputs":[{"uri":"../data/old.xml"}]}"#,
            r#"{"inputs":[{"uri":"studio://project/config/run.json"}]}"#,
            r#"{"inputs":[{"uri":"studio://project/data/old.xml","rootScope":{"moduleMap":"map.json"}}]}"#,
        ] {
            let plan =
                plan_run_config_relocation(request(source), "studio://project/moved/run.json")
                    .unwrap();
            assert!(!plan.blockers.is_empty(), "{source}");
            assert!(plan.rewritten_bytes.is_none());
        }
        let mut conflicting = original;
        conflicting.identity.base_uri = Some("studio://project/different/run.json".into());
        assert!(
            plan_run_config_relocation(conflicting, "studio://project/moved/run.json").is_err()
        );
    }

    #[test]
    fn configuration_bases_retain_lookup_keys_but_normalize_file_scoped_slots() {
        use crate::run_config::{parse_normalized_run_plan, NormalizedRunPlanRequest};
        let bytes = br#"{"inputs":[{"uri":"data/old.xml","rootScope":{"moduleMap":"maps/source.json"}}],"schemaPackages":[{"uri":"packages/schema.cem"}],"outputs":[{"inputRef":"data/old.xml","destination":"out/result.xml"}]}"#.to_vec();
        for (base, directory) in [
            (
                "file:///workspace/config/run.json",
                Some("/workspace/config"),
            ),
            ("file:///workspace/moved/run.json", Some("/workspace/moved")),
            ("studio://project/config/run.json", None),
            ("studio://project/moved/run.json", None),
        ] {
            let plan = parse_normalized_run_plan(NormalizedRunPlanRequest {
                config_bytes: Some(bytes.clone()),
                config_base_uri: Some(base.into()),
                ..Default::default()
            })
            .unwrap();
            assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
            assert_eq!(plan.inputs[0].declared_uri, "data/old.xml");
            assert!(plan.inputs[0].resolved_uri.is_none());
            assert_eq!(plan.schema_packages[0].declared_uri, "packages/schema.cem");
            assert!(plan.schema_packages[0].resolved_uri.is_none());
            assert_eq!(plan.outputs[0].input_id.as_deref(), Some("input:0"));
            assert_eq!(
                plan.effective_run_config().outputs[0].input_ref.as_deref(),
                Some("data/old.xml")
            );
            assert_eq!(
                plan.inputs[0]
                    .root_scope
                    .module_map
                    .as_ref()
                    .unwrap()
                    .resolved_uri,
                directory.map(|d| format!("{d}/maps/source.json"))
            );
            assert_eq!(
                plan.outputs[0].resolved_destination,
                Some(directory.map_or_else(
                    || "out/result.xml".into(),
                    |d| format!("{d}/out/result.xml")
                ))
            );
        }
    }

    #[test]
    fn rebinds_owned_slots_and_input_links_without_rewriting_identities() {
        let source = format!(
            r#"{{"inputs":[{{"uri":"{OLD}","rootScope":{{"schema":"{OLD}","namespaces":{{"data":"{OLD}"}}}}}}],"schemaPackages":[{{"uri":"{OLD}"}}],"outputs":[{{"inputRef":"{OLD}"}}]}}"#
        );
        let plan = plan_run_config_rebind(request(&source), OLD, NEW).unwrap();
        assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
        assert_eq!(plan.references.len(), 3);
        assert_eq!(plan.references[0].kind, RunConfigReferenceKind::Input);
        assert_eq!(
            plan.references[1].kind,
            RunConfigReferenceKind::SchemaPackage
        );
        assert_eq!(plan.references[2].kind, RunConfigReferenceKind::InputLink);
        let bytes = plan.rewritten_bytes.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        for pointer in [
            "/inputs/0/uri",
            "/schemaPackages/0/uri",
            "/outputs/0/inputRef",
        ] {
            assert_eq!(value.pointer(pointer).unwrap(), NEW);
        }
        assert_eq!(value["inputs"][0]["rootScope"]["schema"], OLD);
        assert_eq!(value["inputs"][0]["rootScope"]["namespaces"]["data"], OLD);
        assert!(
            value.get("scheduler").is_none(),
            "do not insert native defaults"
        );
        let again = plan_run_config_rebind(request(std::str::from_utf8(&bytes).unwrap()), OLD, NEW)
            .unwrap();
        assert!(again.rewritten_bytes.is_none());
    }

    #[test]
    fn no_match_and_same_uri_preserve_exact_authored_bytes() {
        let source = "{\n  \"inputs\": [{\"uri\": \"studio://project/other.xml\"}]\n}\n";
        assert!(plan_run_config_rebind(request(source), OLD, NEW)
            .unwrap()
            .rewritten_bytes
            .is_none());
        let source = format!(r#"{{"inputs":[{{"uri":"{OLD}"}}]}}"#);
        assert!(plan_run_config_rebind(request(&source), OLD, OLD)
            .unwrap()
            .rewritten_bytes
            .is_none());
    }

    #[test]
    fn relative_references_are_unresolved_even_with_a_config_base() {
        for value in [
            "data/old.xml",
            "../data/old.xml",
            "/data/old.xml",
            "studio://project/data/old.xml#part",
        ] {
            let source = format!(r#"{{"inputs":[{{"uri":"{value}"}}]}}"#);
            let plan = plan_run_config_rebind(request(&source), OLD, NEW).unwrap();
            assert!(plan.blockers.iter().any(|b| b.path == "/inputs/0/uri"));
            assert!(plan.rewritten_bytes.is_none());
        }
    }

    #[test]
    fn dependency_overrides_and_unknown_fields_block_the_whole_plan() {
        for extra in [
            r#""resolvers":[{"uriPrefix":"studio://project/","localRoot":"/tmp"}]"#,
            r#""futureDependency":"studio://project/data/old.xml""#,
            r#""outputs":[{"destination":"studio://project/output.xml"}]"#,
        ] {
            let source = format!(r#"{{"inputs":[{{"uri":"{OLD}"}}],{extra}}}"#);
            let plan = plan_run_config_rebind(request(&source), OLD, NEW).unwrap();
            assert!(!plan.blockers.is_empty(), "{source}");
            assert!(plan.rewritten_bytes.is_none());
        }
        for scope in [
            r#""baseUri":"studio://project/""#,
            r#""moduleMap":"studio://project/maps.json""#,
            r#""cemtFormatter":"custom""#,
            r#""futureDependency":"anything""#,
        ] {
            let source = format!(r#"{{"inputs":[{{"uri":"{OLD}","rootScope":{{{scope}}}}}]}}"#);
            let plan = plan_run_config_rebind(request(&source), OLD, NEW).unwrap();
            assert!(!plan.blockers.is_empty(), "{source}");
            assert!(plan.rewritten_bytes.is_none());
        }
    }

    #[test]
    fn unknown_nested_fields_and_positional_records_cannot_hide_dependencies() {
        for source in [
            format!(r#"{{"inputs":[{{"uri":"{OLD}","future/ref~":"other"}}]}}"#),
            format!(r#"{{"inputs":[{{"uri":"{OLD}"}}],"scheduler":{{"future":"other"}}}}"#),
            format!(r#"{{"inputs":[{{"uri":"{OLD}"}}],"outputs":[{{"future":"other"}}]}}"#),
            format!(r#"{{"inputs":[["{OLD}",{{}}]]}}"#),
            r#"[[],[],[],[],{}]"#.into(),
        ] {
            match plan_run_config_rebind(request(&source), OLD, NEW) {
                Ok(plan) => {
                    assert!(!plan.blockers.is_empty(), "{source}");
                    assert!(plan.rewritten_bytes.is_none());
                }
                Err(_) => {} // Typed parsing may reject unsupported record representations.
            }
        }
        let source = format!(r#"{{"inputs":[{{"uri":"{OLD}","future/ref~":"other"}}]}}"#);
        let plan = plan_run_config_rebind(request(&source), OLD, NEW).unwrap();
        assert_eq!(plan.blockers[0].path, "/inputs/0/future~1ref~0");
    }

    #[test]
    fn rejects_invalid_config_identity_duplicate_keys_and_target_collisions() {
        for source in [
            "{",
            r#"{"inputs":[{"uri":"one","uri":"two"}]}"#,
            r#"{"outputs":[{"inputRef":"missing"}]}"#,
        ] {
            assert!(
                plan_run_config_rebind(request(source), OLD, NEW).is_err(),
                "{source}"
            );
        }
        let source = format!(r#"{{"inputs":[{{"uri":"{OLD}"}},{{"uri":"{NEW}"}}]}}"#);
        assert!(plan_run_config_rebind(request(&source), OLD, NEW).is_err());
        let mut wrong = request("{}");
        wrong.identity.schema = Some("https://example.test/other".into());
        assert!(plan_run_config_rebind(wrong, OLD, NEW).is_err());
        assert!(plan_run_config_rebind(request("{}"), OLD, "../new.xml").is_err());
    }
}
