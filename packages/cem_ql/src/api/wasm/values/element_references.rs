//! Control metadata names local retained source handles, never AST records.
use super::*;
use crate::api::element_references::{
    ElementPlacementInputs, ElementPlacementSelection, ElementReferenceBinding,
    ElementReferenceExecution, ElementReferenceSource,
};
use crate::render::{ElementPlacementGrant, ElementPlacementTransaction};
use cem_ml::schema::reference_policy::{
    ReferenceScopePolicy, ReferenceUnresolvedPolicy, UnresolvedDisposition,
};
use std::collections::{BTreeMap, BTreeSet};
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Source {
    source_id: u32,
    context: bool,
    max_depth: Option<usize>,
    max_work: Option<usize>,
    unresolved: Option<String>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    source: usize,
    name: String,
    select: String,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Inputs {
    requesting: usize,
    sources: Vec<Source>,
    bindings: Vec<Binding>,
    grants: Vec<(usize, usize)>,
    #[serde(default)]
    placements: Option<Placements>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Placements {
    admissions: Vec<Admission>,
    grants: Vec<PlacementGrant>,
    committed_revisions: BTreeMap<String, String>,
    prepared_transaction: Option<Transaction>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Transaction {
    token: String,
    participants: BTreeSet<String>,
    producer_revisions: BTreeMap<String, String>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Admission {
    source: usize,
    select: String,
    token: String,
    producer: String,
    path: Vec<usize>,
    revision: String,
    id: String,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PlacementGrant {
    requester: String,
    token: String,
    properties: Vec<String>,
}
pub(crate) fn prepare(
    data: &mut TemplateData,
    json: Option<&str>,
) -> Result<Option<ElementReferenceExecution>, String> {
    let Some(json) = json else {
        return Ok(None);
    };
    if json.len() > 128 * 1024 {
        return Err("Reference lifecycle metadata byte limit exceeded".into());
    }
    let inputs: Inputs = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let sources = inputs
        .sources
        .into_iter()
        .map(|s| {
            let mut policy = ReferenceScopePolicy::schema_defaults().map_err(|e| e.to_string())?;
            if let Some(depth) = s.max_depth {
                policy.limits.max_depth = depth;
            }
            if let Some(work) = s.max_work {
                policy.limits.max_work = work;
            }
            if let Some(disposition) = s.unresolved {
                policy.unresolved =
                    ReferenceUnresolvedPolicy::standard(match disposition.as_str() {
                        "mandatory" => UnresolvedDisposition::Mandatory,
                        "warning" => UnresolvedDisposition::Warning,
                        "ignore" => UnresolvedDisposition::Ignore,
                        "neutral" => UnresolvedDisposition::Neutral,
                        _ => return Err("Unknown reference disposition".into()),
                    })
                    .map_err(|e| e.to_string())?;
            }
            Ok(ElementReferenceSource {
                source: retained_source(s.source_id)
                    .map_err(|_| "Unknown reference source handle")?,
                context: s.context,
                policy,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let bindings = inputs
        .bindings
        .into_iter()
        .map(|b| ElementReferenceBinding {
            source: b.source,
            name: b.name,
            select: b.select,
        })
        .collect::<Vec<_>>();
    let placements = inputs
        .placements
        .map(|p| ElementPlacementInputs {
            admissions: p
                .admissions
                .into_iter()
                .map(|a| ElementPlacementSelection {
                    source: a.source,
                    select: a.select,
                    token: a.token,
                    producer: a.producer,
                    path: a.path,
                    revision: a.revision,
                    id: a.id,
                })
                .collect(),
            grants: p
                .grants
                .into_iter()
                .map(|g| ElementPlacementGrant {
                    requester: g.requester,
                    token: g.token,
                    properties: g.properties,
                })
                .collect(),
            committed_revisions: p.committed_revisions,
            prepared_transaction: p.prepared_transaction.map(|t| ElementPlacementTransaction {
                token: t.token,
                participants: t.participants,
                producer_revisions: t.producer_revisions,
            }),
        })
        .unwrap_or_default();
    ElementReferenceExecution::prepare_with_placements(
        sources,
        inputs.requesting,
        &bindings,
        &inputs.grants,
        placements,
        data,
    )
    .map(Some)
}
