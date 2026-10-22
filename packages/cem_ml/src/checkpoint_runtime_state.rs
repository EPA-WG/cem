//! Explicit scalar lifecycle metadata and original document byte envelopes.
//! Native documents are reimported through the normal CEM boundary on resume;
//! this transport never contains handles, callbacks, credentials or a JS AST.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
pub const MAX_BYTES: usize = 512 * 1024;
const SAFE: u64 = 9_007_199_254_740_991;
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct State {
    version: String,
    capability: Option<Capability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus: Option<Focus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    placements: Option<Vec<crate::checkpoint_interactions::Placement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    references: Option<crate::checkpoint_interactions::Source>,
    #[serde(skip_serializing_if = "Option::is_none")]
    native_publication: Option<crate::checkpoint_interactions::NativePublication>,
    #[serde(skip_serializing_if = "Option::is_none")]
    manual_listbox: Option<crate::checkpoint_interactions::ManualListbox>,
    #[serde(skip_serializing_if = "Option::is_none")]
    standalone_datalist: Option<crate::checkpoint_interactions::StandaloneDatalist>,
    resources: Vec<Resource>,
    writers: Vec<Writer>,
    write_disposition: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Focus {
    target: String,
    node_id: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum Capability {
    Suggestions {
        state: Box<crate::checkpoint_interactions::Suggestions>,
    },
    NavigationLandmark {},
    #[serde(rename_all = "camelCase")]
    ActionCommand {
        control_node: String,
        command: Option<String>,
        context_key: Option<String>,
        target: Option<ActionEndpoint>,
        descriptor: Option<ActionEndpoint>,
        disposition: String,
    },
    #[serde(rename_all = "camelCase")]
    ActionControl {
        control_node: String,
        submenu: Option<ActionEndpoint>,
    },
    #[serde(rename_all = "camelCase")]
    DisclosurePanel {
        heading_id: String,
        panel_id: String,
        summary_id: String,
        heading_level: String,
        expanded: bool,
    },
    #[serde(rename_all = "camelCase")]
    NativeList {
        selectable: bool,
        scroll_top: f64,
        scroll_left: f64,
    },
    #[serde(rename_all = "camelCase")]
    TreeView {
        focus_value: String,
        typeahead: String,
        typeahead_at: u64,
    },
    #[serde(rename_all = "camelCase")]
    TabPanels {
        focus_index: i32,
        tab_count: u32,
    },
    #[serde(rename_all = "camelCase")]
    CompositeMenu {
        active_node: Option<String>,
        typeahead: String,
        typeahead_at: u64,
        open: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        state: Option<crate::checkpoint_interactions::Menu>,
    },
    Popup {
        open: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        state: Option<crate::checkpoint_interactions::Popup>,
    },
    #[serde(rename_all = "camelCase")]
    NativeSurface {
        phase: String,
        defaults_initialized: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        state: Option<Box<crate::checkpoint_interactions::Surface>>,
    },
    #[serde(rename_all = "camelCase")]
    FormControl {
        authored_value: Option<String>,
        form_disabled: bool,
        custom_validity: String,
        selection_request: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider: Option<crate::checkpoint_interactions::Provider>,
        #[serde(skip_serializing_if = "Option::is_none")]
        change_suppression: Option<crate::checkpoint_interactions::ChangeSuppression>,
    },
    #[serde(rename_all = "camelCase")]
    PlaintextEditor {
        authored: Option<String>,
        selection_request: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    CheckableControl {
        authored_indeterminate: bool,
    },
    #[serde(rename_all = "camelCase")]
    ChoiceSelect {
        selection: Vec<String>,
        default_values: Vec<String>,
        active_index: i32,
        range_anchor: i32,
        expanded: bool,
        preview_selection: Vec<String>,
        mode: String,
        authored_value: Option<String>,
        value_request: Option<String>,
        form_disabled: bool,
        typeahead: String,
        typeahead_at: u64,
        focused_part: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    EditableChoice {
        active_index: i32,
        authored_value: Option<String>,
        committed_option_value: Option<String>,
        default_committed_option_value: Option<String>,
        default_display_value: String,
        default_value: String,
        display_value: String,
        edited: bool,
        expanded: bool,
        form_disabled: bool,
        value: String,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ActionEndpoint {
    instance_id: String,
    data_revision: String,
    node_id: String,
}
impl ActionEndpoint {
    fn valid(&self) -> bool {
        [&self.instance_id, &self.data_revision, &self.node_id].into_iter().all(|value| identity(value))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Resource {
    name: String,
    kind: String,
    disposition: String,
    revision: u64,
    cursor: u64,
    live: bool,
    state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    document: Option<Document>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Document {
    base64: String,
    content_type: String,
    source_uri: String,
    digest: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Writer {
    index: u32,
    trigger: Option<String>,
    implicit: bool,
}
fn identity(value: &str) -> bool {
    !value.is_empty() && value.len() <= 1024 && !value.chars().any(char::is_control)
}
fn url(value: &str) -> bool {
    url::Url::parse(value).is_ok_and(|url| {
        matches!(url.scheme(), "http" | "https")
            && url.username().is_empty()
            && url.password().is_none()
    })
}
pub fn validate(input: &str) -> Result<String, &'static str> {
    if input.len() > MAX_BYTES {
        return Err("cem.checkpoint.runtime.limit");
    }
    let mut state: State =
        serde_json::from_str(input).map_err(|_| "cem.checkpoint.runtime.format")?;
    if state.version != "cem-checkpoint-runtime/1"
        || state.write_disposition != "suppress-until-new-input"
        || state.resources.len() > 256
        || state.writers.len() > 256
    {
        return Err("cem.checkpoint.runtime.format");
    }
    if state
        .focus
        .as_ref()
        .is_some_and(|focus| match focus.target.as_str() {
            "host" => focus.node_id.is_some(),
            "node" => !focus.node_id.as_deref().is_some_and(identity),
            _ => true,
        })
    {
        return Err("cem.checkpoint.runtime.focus");
    }
    if let Some(delegate) = &state.manual_listbox {
        delegate.validate()?;
    }
    let mut source_bytes = 0;
    if let Some(datalist) = &mut state.standalone_datalist {
        datalist.validate(&mut source_bytes)?;
    }
    if let Some(source) = &mut state.references {
        source.validate(&mut source_bytes)?;
    }
    if let Some(placements) = &mut state.placements {
        if placements.len() > 256 {
            return Err("cem.checkpoint.runtime.limit");
        }
        let mut routes = BTreeSet::new();
        for placement in placements {
            placement.validate(&mut source_bytes)?;
            if !routes.insert((placement.requester_node.clone(), placement.property.clone())) {
                return Err("cem.checkpoint.runtime.placement");
            }
        }
    }
    if let Some(publication) = &mut state.native_publication {
        publication.validate(&mut source_bytes)?;
    }
    if let Some(Capability::Suggestions { state }) = &mut state.capability {
        state.validate(&mut source_bytes)?;
    }
    if let Some(Capability::FormControl {
        change_suppression: Some(change),
        provider,
        ..
    }) = &state.capability
    {
        if change.revision > SAFE
            || change.value.len() > 256 * 1024
            || change.disposition != "suppress-once"
            || provider
                .as_ref()
                .is_some_and(|provider| provider.revision != change.revision)
        {
            return Err("cem.checkpoint.runtime.provider");
        }
    }
    match &state.capability {
        Some(Capability::ActionCommand { control_node, command, context_key, target, descriptor, disposition }) => {
            let command_valid = command.as_deref().is_none_or(|command| {
                identity(command) && (command.starts_with("--") && command.len() > 2
                    || matches!(command, "toggle-popover" | "show-popover" | "hide-popover" | "show-modal" | "close" | "request-close"))
            });
            if !identity(control_node) || disposition != "retain-without-invocation" || !command_valid
                || context_key.as_ref().is_some_and(|key| key.len() > 4096)
                || target.as_ref().is_some_and(|endpoint| !endpoint.valid())
                || descriptor.as_ref().is_some_and(|endpoint| !endpoint.valid())
                || target.is_none() && (command.is_some() || descriptor.is_some())
            { return Err("cem.checkpoint.runtime.capability"); }
        }
        Some(Capability::ActionControl { control_node, submenu }) => {
            if !identity(control_node) || submenu.as_ref().is_some_and(|endpoint| !endpoint.valid()) {
                return Err("cem.checkpoint.runtime.capability");
            }
        }
        Some(Capability::DisclosurePanel { heading_id, panel_id, summary_id, heading_level, .. }) => {
            if ![heading_id, panel_id, summary_id].into_iter().all(|value| identity(value))
                || heading_id == panel_id || heading_id == summary_id || panel_id == summary_id
                || !matches!(heading_level.as_str(), "1" | "2" | "3" | "4" | "5" | "6") {
                return Err("cem.checkpoint.runtime.capability");
            }
        }
        Some(Capability::NativeList { scroll_top, scroll_left, .. }) => {
            if !scroll_top.is_finite() || !scroll_left.is_finite() || scroll_top.abs() > 1e9 || scroll_left.abs() > 1e9 {
                return Err("cem.checkpoint.runtime.capability");
            }
        }
        Some(Capability::FormControl {
            provider: Some(provider),
            ..
        }) if provider.revision > SAFE || !identity(&provider.consumer) => {
            return Err("cem.checkpoint.runtime.provider")
        }
        Some(Capability::TreeView {
            focus_value,
            typeahead,
            typeahead_at,
        }) => {
            if focus_value.len() > 1024 || typeahead.len() > 4096 || *typeahead_at > SAFE {
                return Err("cem.checkpoint.runtime.capability");
            }
        }
        Some(Capability::TabPanels {
            focus_index,
            tab_count,
        }) => {
            if *tab_count > 1024 || *focus_index < -1 || *focus_index >= *tab_count as i32 {
                return Err("cem.checkpoint.runtime.capability");
            }
        }
        Some(Capability::CompositeMenu {
            active_node,
            typeahead,
            typeahead_at,
            open,
            state,
        }) => {
            if let Some(state) = state {
                state.validate(*open)?;
            } else if *open {
                return Err("cem.checkpoint.runtime.menu");
            }
            if active_node.as_deref().is_some_and(|node| !identity(node))
                || typeahead.len() > 4096
                || *typeahead_at > SAFE
            {
                return Err("cem.checkpoint.runtime.capability");
            }
        }
        Some(Capability::Popup { open, state }) => {
            if let Some(state) = state {
                state.validate()?;
            } else if *open {
                return Err("cem.checkpoint.runtime.surface");
            }
        }
        Some(Capability::NativeSurface {
            phase,
            defaults_initialized,
            state,
        }) => {
            if !matches!(phase.as_str(), "open" | "closed")
                || !defaults_initialized
                || phase == "open" && state.is_none()
            {
                return Err("cem.checkpoint.runtime.surface");
            }
            if let Some(state) = state {
                state.validate(phase, &mut source_bytes)?;
            }
        }
        Some(Capability::ChoiceSelect {
            selection,
            default_values,
            active_index,
            range_anchor,
            preview_selection,
            mode,
            typeahead,
            typeahead_at,
            focused_part,
            ..
        }) => {
            if !matches!(
                mode.as_str(),
                "dropdown" | "single-listbox" | "multiple-listbox"
            ) || !(-1..1024).contains(active_index)
                || !(-1..1024).contains(range_anchor)
                || *typeahead_at > SAFE
                || typeahead.len() > 4096
                || selection.len() > 1024
                || default_values.len() > 1024
                || preview_selection.len() > 1024
                || mode != "multiple-listbox" && selection.len() > 1
                || selection.iter().collect::<BTreeSet<_>>().len() != selection.len()
                || focused_part
                    .as_deref()
                    .is_some_and(|part| !matches!(part, "host" | "control"))
            {
                return Err("cem.checkpoint.runtime.capability");
            }
        }
        Some(Capability::EditableChoice { active_index, .. })
            if !(-1..1024).contains(active_index) =>
        {
            return Err("cem.checkpoint.runtime.capability")
        }
        _ => {}
    }
    let mut names = BTreeSet::new();
    let mut bytes = source_bytes;
    for resource in &mut state.resources {
        if !identity(&resource.name)
            || !names.insert(&resource.name)
            || resource.revision > SAFE
            || resource.cursor > SAFE
            || !matches!(resource.state.as_str(), "loaded" | "failed")
        {
            return Err("cem.checkpoint.runtime.resource");
        }
        if resource.kind == "http" {
            if resource.live
                || resource.disposition != "reimport-and-read-again"
                || !matches!(resource.method.as_deref(), Some("GET" | "HEAD"))
                || !resource.url.as_deref().is_some_and(url)
                || resource.state == "loaded"
                    && resource.method.as_deref() == Some("GET")
                    && resource.document.is_none()
            {
                return Err("cem.checkpoint.runtime.http");
            }
            if let Some(document) = &mut resource.document {
                if document.content_type.is_empty()
                    || document.content_type.len() > 256
                    || !url(&document.source_uri)
                {
                    return Err("cem.checkpoint.runtime.document");
                }
                let decoded = STANDARD
                    .decode(&document.base64)
                    .map_err(|_| "cem.checkpoint.runtime.document")?;
                bytes += decoded.len();
                if bytes > 256 * 1024 {
                    return Err("cem.checkpoint.runtime.limit");
                }
                let digest = format!("{:x}", Sha256::digest(&decoded));
                if document
                    .digest
                    .as_ref()
                    .is_some_and(|expected| expected != &digest)
                {
                    return Err("cem.checkpoint.runtime.digest");
                }
                document.digest = Some(digest);
            }
        } else {
            if !matches!(
                resource.kind.as_str(),
                "location" | "local-storage" | "repository-query" | "storage-status"
            ) || resource.document.is_some()
                || resource.method.is_some()
                || resource.disposition
                    != if resource.live {
                        "read-and-subscribe"
                    } else {
                        "read-again"
                    }
                || (resource.kind == "location" && !resource.url.as_deref().is_some_and(url))
                || (resource.kind != "location" && resource.url.is_some())
            {
                return Err("cem.checkpoint.runtime.resource");
            }
        }
    }
    let mut writers = BTreeSet::new();
    for writer in &state.writers {
        if writer.index >= 256
            || !writers.insert(writer.index)
            || (!writer.implicit && writer.trigger.is_none())
            || writer
                .trigger
                .as_ref()
                .is_some_and(|token| !identity(token))
        {
            return Err("cem.checkpoint.runtime.writer");
        }
    }
    let output = serde_json::to_string(&state).map_err(|_| "cem.checkpoint.runtime.format")?;
    if output.len() > MAX_BYTES {
        return Err("cem.checkpoint.runtime.limit");
    }
    Ok(output)
}
