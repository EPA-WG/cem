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
    resources: Vec<Resource>,
    writers: Vec<Writer>,
    write_disposition: String,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum Capability {
    #[serde(rename_all = "camelCase")]
    FormControl {
        authored_value: Option<String>,
        form_disabled: bool,
        custom_validity: String,
        selection_request: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    PlaintextEditor {
        authored: Option<String>,
        selection_request: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    CheckableControl { authored_indeterminate: bool },
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
    match &state.capability {
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
    let mut bytes = 0;
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
