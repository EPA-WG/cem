//! Multi-document run context derived from normalized run configuration.
//!
//! This is the library boundary for build/CI and host runtimes that need one
//! invocation to account for several input roots and their output bindings
//! before command-specific execution starts.

use crate::diagnostics::Diagnostic;
use crate::engine::FormatIdentity;
use crate::run_config::{
    NormalizedBudgets, NormalizedInput, NormalizedPrimaryOutputPolicy, NormalizedRootScope,
    NormalizedRunPlan, NormalizedScopePolicy, SchedulerConfig,
};
use crate::source_map::SourceMapStack;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunContext {
    pub run_id: String,
    pub report_identity: RunReportIdentity,
    pub scheduler_boundary: RunSchedulerBoundary,
    #[serde(default)]
    pub documents: Vec<RunDocument>,
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
}

impl RunContext {
    pub fn from_plan(plan: &NormalizedRunPlan) -> Self {
        build_run_context(plan)
    }
}

pub fn build_run_context(plan: &NormalizedRunPlan) -> RunContext {
    let mut run_diagnostics = plan_unbound_diagnostics(plan);
    let mut documents: Vec<RunDocument> = plan
        .inputs
        .iter()
        .enumerate()
        .map(|(index, input)| {
            let scheduler_scope_id = index as u32;
            let document_id = format!("document:{index}");
            RunDocument {
                document_id: document_id.clone(),
                input_id: input.input_id.clone(),
                input_uri: input_uri(input),
                declared_uri: input.declared_uri.clone(),
                resolved_uri: input.resolved_uri.clone(),
                identity: input.identity.clone(),
                root_scope: input.root_scope.clone(),
                scheduler_scope_id,
                source_map_ref: format!("sourceMap:{document_id}"),
                source_maps: Vec::new(),
                outputs: Vec::new(),
                diagnostics: document_diagnostics(plan, index),
            }
        })
        .collect();

    for (index, output) in plan.outputs.iter().enumerate() {
        let Some(input_id) = output.input_id.as_deref() else {
            continue;
        };
        let Some(document_index) = document_index_for_input_id(input_id) else {
            run_diagnostics.extend(output_diagnostics(plan, index));
            continue;
        };
        let Some(document) = documents.get_mut(document_index) else {
            run_diagnostics.extend(output_diagnostics(plan, index));
            continue;
        };

        document.outputs.push(RunDocumentOutput {
            output_id: output.output_id.clone(),
            output_index: index,
            input_id: input_id.to_owned(),
            declared_destination: output.declared_destination.clone(),
            resolved_destination: output.resolved_destination.clone(),
            identity: output.identity.clone(),
            root_scope: output.root_scope.clone(),
            primary_output_policy: output.primary_output_policy,
            scheduler_scope_id: document.scheduler_scope_id,
        });
    }

    let scheduler_boundary = RunSchedulerBoundary {
        scheduler: plan.scheduler.clone(),
        root_scope_id: 0,
        document_scopes: documents
            .iter()
            .map(|document| RunSchedulerScope {
                scheduler_scope_id: document.scheduler_scope_id,
                document_id: document.document_id.clone(),
                input_id: document.input_id.clone(),
                root_scope_id: document.root_scope.scope_id.clone(),
                policy: document.root_scope.policy.clone(),
                budgets: document.root_scope.budgets.clone(),
            })
            .collect(),
    };
    let report_identity = run_report_identity(plan, &documents, &run_diagnostics);

    RunContext {
        run_id: plan.run_id.clone(),
        report_identity,
        scheduler_boundary,
        documents,
        diagnostics: run_diagnostics,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDocument {
    pub document_id: String,
    pub input_id: String,
    pub input_uri: String,
    pub declared_uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_uri: Option<String>,
    pub identity: FormatIdentity,
    pub root_scope: NormalizedRootScope,
    pub scheduler_scope_id: u32,
    pub source_map_ref: String,
    #[serde(default)]
    pub source_maps: Vec<SourceMapStack>,
    #[serde(default)]
    pub outputs: Vec<RunDocumentOutput>,
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDocumentOutput {
    pub output_id: String,
    pub output_index: usize,
    pub input_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared_destination: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_destination: Option<String>,
    pub identity: FormatIdentity,
    pub root_scope: NormalizedRootScope,
    pub primary_output_policy: NormalizedPrimaryOutputPolicy,
    pub scheduler_scope_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSchedulerBoundary {
    pub scheduler: SchedulerConfig,
    pub root_scope_id: u32,
    #[serde(default)]
    pub document_scopes: Vec<RunSchedulerScope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSchedulerScope {
    pub scheduler_scope_id: u32,
    pub document_id: String,
    pub input_id: String,
    pub root_scope_id: String,
    pub policy: NormalizedScopePolicy,
    pub budgets: NormalizedBudgets,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunReportIdentity {
    pub report_id: String,
    pub input_count: u32,
    pub output_count: u32,
    #[serde(default)]
    pub input_uris: Vec<String>,
    #[serde(default)]
    pub output_ids: Vec<String>,
    pub diagnostic_count: u32,
}

fn input_uri(input: &NormalizedInput) -> String {
    input
        .resolved_uri
        .clone()
        .unwrap_or_else(|| input.declared_uri.clone())
}

fn document_index_for_input_id(input_id: &str) -> Option<usize> {
    input_id.strip_prefix("input:")?.parse::<usize>().ok()
}

fn document_diagnostics(plan: &NormalizedRunPlan, input_index: usize) -> Vec<Diagnostic> {
    let input_prefix = format!("inputs[{input_index}]");
    plan.diagnostics
        .iter()
        .filter(|diagnostic| diagnostic_field_path_starts_with(diagnostic, &input_prefix))
        .cloned()
        .collect()
}

fn output_diagnostics(plan: &NormalizedRunPlan, output_index: usize) -> Vec<Diagnostic> {
    let output_prefix = format!("outputs[{output_index}]");
    plan.diagnostics
        .iter()
        .filter(|diagnostic| diagnostic_field_path_starts_with(diagnostic, &output_prefix))
        .cloned()
        .collect()
}

fn plan_unbound_diagnostics(plan: &NormalizedRunPlan) -> Vec<Diagnostic> {
    plan.outputs
        .iter()
        .enumerate()
        .filter(|(_, output)| output.input_id.is_none())
        .flat_map(|(index, _)| output_diagnostics(plan, index))
        .collect()
}

fn diagnostic_field_path_starts_with(diagnostic: &Diagnostic, prefix: &str) -> bool {
    diagnostic
        .details
        .as_ref()
        .and_then(|details| details.get("fieldPath"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|field_path| {
            field_path == prefix || field_path.starts_with(&format!("{prefix}."))
        })
}

fn run_report_identity(
    plan: &NormalizedRunPlan,
    documents: &[RunDocument],
    run_diagnostics: &[Diagnostic],
) -> RunReportIdentity {
    let input_uris: Vec<_> = documents
        .iter()
        .map(|document| document.input_uri.clone())
        .collect();
    let output_ids: Vec<_> = documents
        .iter()
        .flat_map(|document| {
            document
                .outputs
                .iter()
                .map(|output| output.output_id.clone())
        })
        .collect();
    let diagnostic_count = documents
        .iter()
        .map(|document| document.diagnostics.len() as u32)
        .sum::<u32>()
        + run_diagnostics.len() as u32;
    let report_id = stable_report_id(plan, &input_uris, &output_ids, diagnostic_count);

    RunReportIdentity {
        report_id,
        input_count: documents.len() as u32,
        output_count: output_ids.len() as u32,
        input_uris,
        output_ids,
        diagnostic_count,
    }
}

fn stable_report_id(
    plan: &NormalizedRunPlan,
    input_uris: &[String],
    output_ids: &[String],
    diagnostic_count: u32,
) -> String {
    let payload = serde_json::to_string(&(
        &plan.run_id,
        &plan.command_profile,
        &plan.scheduler,
        input_uris,
        output_ids,
        diagnostic_count,
    ))
    .unwrap_or_default();
    format!("report:{:016x}", stable_hash(payload.as_bytes()))
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::FormatIdentity;
    use crate::run_config::{NormalizedRunPlanRequest, RunConfigDefaults};

    fn json_plan(json: &[u8]) -> NormalizedRunPlan {
        crate::run_config::parse_normalized_run_plan(NormalizedRunPlanRequest {
            config_bytes: Some(json.to_vec()),
            config_identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                schema: Some(crate::run_config::RUN_CONFIG_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            },
            config_base_uri: Some("/workspace/run.json".to_owned()),
            defaults: RunConfigDefaults::default(),
            ..NormalizedRunPlanRequest::default()
        })
        .unwrap()
    }

    #[test]
    fn run_context_binds_inputs_outputs_and_scheduler_scopes() {
        let plan = json_plan(
            br#"{
                "scheduler": {
                    "threadPool": "deterministic",
                    "maxParallelDocuments": 2
                },
                "inputs": [
                    {
                        "uri": "src/one.cem",
                        "rootScope": {
                            "budgets": {
                                "cpuWorkers": "2",
                                "parseMs": "11"
                            }
                        }
                    },
                    {
                        "uri": "src/two.cem",
                        "rootScope": {
                            "budgets": {
                                "queueSize": "4",
                                "validateMs": "13"
                            }
                        }
                    }
                ],
                "outputs": [
                    {
                        "inputRef": "src/one.cem",
                        "destination": "dist/one.cem"
                    },
                    {
                        "inputRef": "src/two.cem",
                        "destination": "dist/two.cem"
                    }
                ]
            }"#,
        );

        let context = RunContext::from_plan(&plan);
        let second_context = RunContext::from_plan(&plan);

        assert_eq!(context.run_id, plan.run_id);
        assert_eq!(
            context.report_identity.report_id,
            second_context.report_identity.report_id
        );
        assert_eq!(context.documents.len(), 2);
        assert_eq!(context.documents[0].document_id, "document:0");
        assert_eq!(context.documents[0].input_id, "input:0");
        assert_eq!(context.documents[0].scheduler_scope_id, 0);
        assert_eq!(context.documents[1].scheduler_scope_id, 1);
        assert_eq!(context.documents[0].outputs[0].output_id, "output:0");
        assert_eq!(
            context.documents[0].outputs[0]
                .resolved_destination
                .as_deref(),
            Some("/workspace/dist/one.cem")
        );
        assert_eq!(context.documents[1].outputs[0].output_id, "output:1");
        assert_eq!(context.scheduler_boundary.document_scopes.len(), 2);
        assert_eq!(
            context.scheduler_boundary.document_scopes[0]
                .budgets
                .parse_ms,
            Some(11)
        );
        assert_eq!(
            context.scheduler_boundary.document_scopes[1]
                .budgets
                .validate_ms,
            Some(13)
        );
        assert_eq!(
            context.report_identity.input_uris,
            vec!["src/one.cem".to_owned(), "src/two.cem".to_owned()]
        );
        assert_eq!(
            context.report_identity.output_ids,
            vec!["output:0".to_owned(), "output:1".to_owned()]
        );
    }

    #[test]
    fn run_context_projects_root_scope_diagnostics_to_document() {
        let plan = json_plan(
            br#"{
                "inputs": [{
                    "uri": "src/invalid.cem",
                    "rootScope": {
                        "namespaces": {
                            "xml": "urn:not-xml"
                        }
                    }
                }]
            }"#,
        );

        let context = RunContext::from_plan(&plan);

        assert!(context.diagnostics.is_empty());
        assert_eq!(context.documents.len(), 1);
        assert!(context.documents[0]
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.run_config.scope_namespace_invalid"));
        assert_eq!(context.report_identity.diagnostic_count, 1);
    }

    #[test]
    fn run_context_keeps_ambiguous_outputs_at_run_level() {
        let plan = json_plan(
            br#"{
                "inputs": [
                    { "uri": "src/one.cem" },
                    { "uri": "src/two.cem" }
                ],
                "outputs": [{
                    "destination": "dist/out.cem"
                }]
            }"#,
        );

        let context = RunContext::from_plan(&plan);

        assert!(context
            .documents
            .iter()
            .all(|document| document.outputs.is_empty()));
        assert!(context
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.run_config.output_input_ref_ambiguous"));
        assert_eq!(context.report_identity.output_count, 0);
    }

    #[test]
    fn run_context_keeps_unknown_output_refs_at_run_level() {
        let plan = json_plan(
            br#"{
                "inputs": [{ "uri": "src/one.cem" }],
                "outputs": [{
                    "inputRef": "src/missing.cem",
                    "destination": "dist/out.cem"
                }]
            }"#,
        );

        let context = RunContext::from_plan(&plan);

        assert!(context.documents[0].outputs.is_empty());
        assert!(context
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.run_config.output_input_ref_unknown"));
        assert_eq!(context.report_identity.diagnostic_count, 1);
    }
}
