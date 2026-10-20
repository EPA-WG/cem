//! Scalar interaction checkpoints and original native source envelopes. No grants.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
const SAFE: u64 = 9_007_199_254_740_991;
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    version: String,
    revision: String,
    pub row_count: u32,
    select: String,
    resolve: bool,
    requesting: usize,
    sources: Vec<Input>,
    bindings: Vec<Binding>,
    crossings: Vec<Crossing>,
    labels: Labels,
    disposition: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Input {
    codec: String,
    base64: String,
    source_uri: Option<String>,
    primary_source_id: u32,
    context: bool,
    #[serde(default)]
    digest: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    source: usize,
    name: String,
    select: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Crossing {
    from: usize,
    to: usize,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Labels {
    option: Option<String>,
    group: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadySuggestions {
    publication: Publication,
    source: Source,
    provider_revision: u64,
    query_revision: u64,
    prepared_revision: u64,
    query: String,
    intent: bool,
    feedback_intent: bool,
    feedback_state: String,
    active_index: i32,
    committed_index: i32,
    committed_value: Option<String>,
    visible: bool,
    scroll_top: f64,
    scroll_left: f64,
    rows: Vec<Row>,
}
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Suggestions {
    Ready(ReadySuggestions),
    Inactive(InactiveSuggestions),
    Datalist(DatalistSuggestions),
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InactiveSuggestions {
    phase: String,
    profile: String,
    source: Source,
    provider_revision: u64,
    query_revision: u64,
    query: String,
    feedback_state: String,
    feedback_intent: bool,
    editable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    proof: Option<RetainedProof>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RetainedProof {
    source: Source,
    index: u32,
    value: String,
    disposition: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DatalistSuggestions {
    phase: String,
    source: Source,
    provider_revision: u64,
    target_node: String,
    claim: String,
    presentation: String,
}
impl Suggestions {
    pub fn validate(&mut self, bytes: &mut usize) -> Result<(), &'static str> {
        match self {
            Self::Ready(state) => state.validate(bytes),
            Self::Inactive(state) => {
                state.source.validate(bytes)?;
                if state.phase != "inactive"
                    || !matches!(state.profile.as_str(), "listbox" | "native-datalist")
                    || state.source.row_count != 0
                    || state.provider_revision > SAFE
                    || state.query_revision > SAFE
                    || state.query.len() > 65536
                    || !matches!(
                        state.feedback_state.as_str(),
                        "idle" | "failed" | "unavailable"
                    )
                    || state.feedback_state == "unavailable"
                        && (state.editable || state.feedback_intent)
                {
                    return Err("cem.checkpoint.runtime.suggestions");
                }
                if let Some(proof) = &mut state.proof {
                    proof.source.validate(bytes)?;
                    if state.profile != "listbox"
                        || proof.index >= proof.source.row_count
                        || proof.value != state.query
                        || proof.disposition != "revalidate"
                    {
                        return Err("cem.checkpoint.runtime.suggestions");
                    }
                }
                Ok(())
            }
            Self::Datalist(state) => {
                state.source.validate(bytes)?;
                if state.phase != "native-datalist"
                    || state.provider_revision > SAFE
                    || !identity(&state.target_node)
                    || !matches!(state.claim.as_str(), "attached" | "unavailable")
                    || state.presentation != "reset-on-resume"
                {
                    return Err("cem.checkpoint.runtime.datalist");
                }
                Ok(())
            }
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Publication {
    filter: String,
    filter_by: Option<String>,
    expanded: Option<bool>,
    active: Option<i32>,
    committed: Option<i32>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Row {
    index: u32,
    value: String,
    eligible: bool,
    available: bool,
    node_id: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Provider {
    pub revision: u64,
    pub consumer: String,
    pub validity: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Placement {
    pub requester_node: String,
    pub property: String,
    pub producer: String,
    pub revision: String,
    pub provider_node: String,
    source_index: usize,
    path: Vec<u32>,
    select: String,
    pub source: Source,
    disposition: String,
}
fn identity(s: &str) -> bool {
    !s.is_empty() && s.len() <= 1024 && !s.chars().any(char::is_control)
}
impl Source {
    pub fn validate(&mut self, bytes: &mut usize) -> Result<(), &'static str> {
        let fail = "cem.checkpoint.runtime.source";
        if self.version != "cem-native-source/1"
            || !identity(&self.revision)
            || self.row_count > 1024
            || self.sources.is_empty()
            || self.sources.len() > 64
            || self.requesting >= self.sources.len()
            || self.bindings.len() > 256
            || self.crossings.len() > 256
            || self.select.is_empty()
            || self.select.len() > 4096
            || self.disposition != "reacquire"
            || [&self.labels.option, &self.labels.group]
                .iter()
                .any(|s| s.as_ref().is_some_and(|s| s.len() > 4096))
        {
            return Err(fail);
        }
        let mut names = BTreeSet::new();
        for binding in &self.bindings {
            if binding.source >= self.sources.len()
                || !identity(&binding.name)
                || !names.insert(&binding.name)
                || binding.select.len() > 4096
            {
                return Err(fail);
            }
        }
        let mut crossings = BTreeSet::new();
        for c in &self.crossings {
            if c.from >= self.sources.len()
                || c.to >= self.sources.len()
                || !crossings.insert((c.from, c.to))
            {
                return Err(fail);
            }
        }
        let source_count = self.sources.len();
        for source in &mut self.sources {
            let decoded = STANDARD.decode(&source.base64).map_err(|_| fail)?;
            *bytes += decoded.len();
            if *bytes > 256 * 1024 {
                return Err("cem.checkpoint.runtime.limit");
            }
            let digest = format!("{:x}", Sha256::digest(&decoded));
            if source.digest.as_ref().is_some_and(|d| d != &digest) {
                return Err("cem.checkpoint.runtime.digest");
            }
            match source.codec.as_str() {
                "xml" => {
                    if source_count != 1 || !self.bindings.is_empty() || !self.crossings.is_empty()
                    {
                        return Err(fail);
                    }
                    let uri = source.source_uri.as_deref().ok_or(fail)?;
                    let url = url::Url::parse(uri).map_err(|_| fail)?;
                    if !url.username().is_empty() || url.password().is_some() {
                        return Err(fail);
                    }
                    let mut documents = crate::import::documents::CemDocuments::default();
                    documents
                        .retain(&decoded, "application/xml", uri)
                        .map_err(|_| fail)?;
                }
                "cemb" => {
                    if source.source_uri.is_some() {
                        return Err(fail);
                    }
                    let bundle = crate::ast::reload::ReferenceReloadBundle::decode(
                        &decoded,
                        crate::ast::reload::ReloadLimits {
                            max_bytes: 256 * 1024,
                            max_nodes: 4096,
                        },
                    )
                    .map_err(|_| fail)?;
                    let reloaded = bundle
                        .reload(crate::ast::reload::ReloadLimits {
                            max_bytes: 256 * 1024,
                            max_nodes: 4096,
                        })
                        .map_err(|_| fail)?;
                    if source.context {
                        reloaded.require_lexical().map_err(|_| fail)?;
                    }
                    if bundle.sources.iter().any(|s| s.bytes.is_none())
                        || !bundle
                            .sources
                            .iter()
                            .any(|s| s.source_id.0 == source.primary_source_id)
                    {
                        return Err(fail);
                    }
                }
                _ => return Err(fail),
            }
            source.digest = Some(digest);
        }
        Ok(())
    }
}
impl ReadySuggestions {
    pub fn validate(&mut self, bytes: &mut usize) -> Result<(), &'static str> {
        self.source.validate(bytes)?;
        let count = self.source.row_count as i32;
        self.publication.validate(count)?;
        if self.provider_revision > SAFE
            || self.query_revision > SAFE
            || self.provider_revision != self.prepared_revision
            || self.feedback_state != "ready"
            || self.query.len() > 65536
            || self.active_index < -1
            || self.active_index >= count
            || self.committed_index < -1
            || self.committed_index >= count
            || (self.committed_index >= 0) != self.committed_value.is_some()
            || self.rows.len() > 1024
            || !self.scroll_top.is_finite()
            || !self.scroll_left.is_finite()
            || self.visible && !self.intent
        {
            return Err("cem.checkpoint.runtime.suggestions");
        }
        validate_rows(&self.rows, self.source.row_count)?;
        if self.active_index >= 0
            && (!self.visible
                || !self
                    .rows
                    .iter()
                    .any(|row| row.index == self.active_index as u32 && row.eligible))
        {
            return Err("cem.checkpoint.runtime.suggestions");
        }
        Ok(())
    }
}
impl Placement {
    pub fn validate(&mut self, bytes: &mut usize) -> Result<(), &'static str> {
        if self.source_index >= self.source.sources.len()
            || ![
                &self.requester_node,
                &self.provider_node,
                &self.producer,
                &self.revision,
            ]
            .iter()
            .all(|s| identity(s))
            || self.path.is_empty()
            || self.path.len() > 64
            || self.select.is_empty()
            || self.select.len() > 4096
            || self.disposition != "reacquire"
            || !crate::committed_checkpoint::placement_property(&self.property)
        {
            return Err("cem.checkpoint.runtime.placement");
        }
        self.source.validate(bytes)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Endpoint {
    instance_id: String,
    data_revision: String,
    node_id: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rect {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Invocation {
    source: Option<Endpoint>,
    return_destination: Option<Endpoint>,
    logical_parent: Option<Endpoint>,
    command: Option<String>,
    context_key: Option<String>,
    input_kind: String,
    pointer: Option<Rect>,
    selection: Option<Rect>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Surface {
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<DeferredBody>,
    return_value: Option<String>,
    modal: bool,
    focus_within: bool,
    close_reason: String,
    invocation: Option<Invocation>,
    disposition: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Popup {
    trigger: String,
    panel: String,
    scroll_top: f64,
    scroll_left: f64,
    disposition: String,
}
impl Surface {
    pub fn validate(&self, phase: &str, bytes: &mut usize) -> Result<(), &'static str> {
        if let Some(body) = &self.body {
            body.validate(bytes)?;
        }
        let fail = "cem.checkpoint.runtime.surface";
        if self.disposition != "reacquire"
            || self.close_reason.len() > 1024
            || phase == "closed" && (self.modal || self.invocation.is_some())
        {
            return Err(fail);
        }
        if let Some(i) = &self.invocation {
            if !matches!(
                i.input_kind.as_str(),
                "pointer" | "keyboard" | "programmatic"
            ) {
                return Err(fail);
            }
            for endpoint in [&i.source, &i.return_destination, &i.logical_parent]
                .into_iter()
                .flatten()
            {
                if ![
                    &endpoint.instance_id,
                    &endpoint.data_revision,
                    &endpoint.node_id,
                ]
                .iter()
                .all(|s| identity(s))
                {
                    return Err(fail);
                }
            }
            for rect in [&i.pointer, &i.selection].into_iter().flatten() {
                if ![rect.left, rect.right, rect.top, rect.bottom]
                    .iter()
                    .all(|n| n.is_finite())
                    || rect.right < rect.left
                    || rect.bottom < rect.top
                {
                    return Err(fail);
                }
            }
        }
        Ok(())
    }
}
impl Popup {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !identity(&self.trigger)
            || !identity(&self.panel)
            || !self.scroll_top.is_finite()
            || !self.scroll_left.is_finite()
            || self.disposition != "reacquire"
        {
            return Err("cem.checkpoint.runtime.surface");
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChangeSuppression {
    pub revision: u64,
    pub value: String,
    pub disposition: String,
}

/// Validated controller relationships, never authored IDs or geometry leases.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Menu {
    parent: Option<Endpoint>,
    launcher: Option<Endpoint>,
    panel: Option<Endpoint>,
    disposition: String,
}
impl Menu {
    pub fn validate(&self, open: bool) -> Result<(), &'static str> {
        if self.disposition != "reacquire"
            || self.parent.is_some() != self.launcher.is_some()
            || open != self.panel.is_some()
            || [&self.parent, &self.launcher, &self.panel]
                .into_iter()
                .flatten()
                .any(|e| {
                    ![&e.instance_id, &e.data_revision, &e.node_id]
                        .iter()
                        .all(|s| identity(s))
                })
        {
            return Err("cem.checkpoint.runtime.menu");
        }
        Ok(())
    }
}

/// A committed native publication consumed declaratively without an editor controller.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativePublication {
    profile: String,
    source: Source,
    query: String,
    query_revision: u64,
    publication: Publication,
    rows: Vec<Row>,
    disposition: String,
}
impl Publication {
    fn validate(&self, count: i32) -> Result<(), &'static str> {
        if !matches!(
            self.filter.as_str(),
            "contains" | "prefix" | "external" | "none"
        ) || self.filter_by.as_ref().is_some_and(|s| s.len() > 4096)
            || [self.active, self.committed]
                .into_iter()
                .flatten()
                .any(|i| i < -1 || i >= count)
        {
            return Err("cem.checkpoint.runtime.suggestions");
        }
        Ok(())
    }
}
fn validate_rows(rows: &[Row], count: u32) -> Result<(), &'static str> {
    if rows.len() > 1024 {
        return Err("cem.checkpoint.runtime.suggestions");
    }
    let mut indexes = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for row in rows {
        if row.index >= count
            || !indexes.insert(row.index)
            || !identity(&row.node_id)
            || !ids.insert(&row.node_id)
            || row.eligible && !row.available
        {
            return Err("cem.checkpoint.runtime.suggestions");
        }
    }
    Ok(())
}
impl NativePublication {
    pub fn validate(&mut self, bytes: &mut usize) -> Result<(), &'static str> {
        self.source.validate(bytes)?;
        if self.profile != "listbox"
            || self.query.len() > 65536
            || self.query_revision > SAFE
            || self.disposition != "reacquire"
        {
            return Err("cem.checkpoint.runtime.publication");
        }
        self.publication.validate(self.source.row_count as i32)?;
        validate_rows(&self.rows, self.source.row_count)
    }
}

/// Recreate a current native template projection before applying control checkpoints.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeferredBody {
    template_node: String,
    mode: String,
    materialized: bool,
    roots: Vec<String>,
    disposition: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    projection: Option<crate::checkpoint_projection::Projection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reuse_on_open: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    owners: Vec<BodyOwner>,
}
impl DeferredBody {
    fn validate(&self, bytes: &mut usize) -> Result<(), &'static str> {
        let mut ids = BTreeSet::new();
        if !identity(&self.template_node)
            || !matches!(self.mode.as_str(), "retain" | "dispose")
            || self.disposition != "rematerialize"
            || self.roots.len() > 256
            || !self.materialized && !self.roots.is_empty()
            || self
                .roots
                .iter()
                .any(|id| !identity(id) || id == &self.template_node || !ids.insert(id))
        {
            return Err("cem.checkpoint.runtime.body");
        }
        if self.projection.is_some() != self.reuse_on_open.is_some()
            || !self.materialized && (self.projection.is_some() || !self.owners.is_empty())
            || self.owners.len() > 256
        {
            return Err("cem.checkpoint.runtime.body");
        }
        let elements = self
            .projection
            .as_ref()
            .map(|p| p.validate(&self.roots, bytes))
            .transpose()?;
        let mut hosts = BTreeSet::new();
        let mut owners = BTreeSet::new();
        for binding in &self.owners {
            if !elements
                .as_ref()
                .is_some_and(|ids| ids.contains(binding.host_node.as_str()))
                || !hosts.insert(&binding.host_node)
                || !owners.insert(&binding.owner.instance_id)
                || ![&binding.owner.instance_id, &binding.owner.data_revision]
                    .iter()
                    .all(|s| identity(s))
                || binding.owner.node_id != "host"
            {
                return Err("cem.checkpoint.runtime.body");
            }
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BodyOwner {
    host_node: String,
    owner: Endpoint,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManualListbox {
    panel: Endpoint,
    editor: Endpoint,
    provider_revision: u64,
    visible: bool,
    presentation: String,
    disposition: String,
}
impl ManualListbox {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.provider_revision > SAFE
            || self.presentation != "reset-on-resume"
            || self.disposition != "reacquire"
            || [&self.panel, &self.editor].iter().any(|e| {
                ![&e.instance_id, &e.data_revision, &e.node_id]
                    .iter()
                    .all(|s| identity(s))
            })
        {
            return Err("cem.checkpoint.runtime.delegate");
        }
        Ok(())
    }
}

/// Standalone ownership of a committed native options frame. Platform UI resets.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StandaloneDatalist {
    editor: Endpoint,
    target: Endpoint,
    provider_revision: u64,
    claim: String,
    source: Source,
    presentation: String,
    disposition: String,
}
impl StandaloneDatalist {
    pub fn validate(&mut self, bytes: &mut usize) -> Result<(), &'static str> {
        self.source.validate(bytes)?;
        if self.provider_revision > SAFE
            || self.editor.node_id != "host"
            || self.target.node_id == "host"
            || self.editor.instance_id == self.target.instance_id
            || !matches!(self.claim.as_str(), "attached" | "unavailable")
            || self.presentation != "reset-on-resume"
            || self.disposition != "reacquire"
            || [&self.editor, &self.target].iter().any(|e| {
                ![&e.instance_id, &e.data_revision, &e.node_id]
                    .iter()
                    .all(|s| identity(s))
            })
        {
            return Err("cem.checkpoint.runtime.datalist");
        }
        Ok(())
    }
}
