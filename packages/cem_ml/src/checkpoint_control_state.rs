//! Explicit JSON control-state boundary, not a DOM/AST representation. These
//! browser-owned scalar edits remain inert until semantic resume admits owners.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
pub const MAX_BYTES: usize = 256 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct State {
    version: String,
    controls: Vec<Control>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Control {
    id: String,
    kind: String,
    input_type: String,
    value: String,
    checked: bool,
    indeterminate: bool,
    multiple: bool,
    options: Vec<OptionState>,
    selection: Option<Selection>,
    focused: bool,
    disabled: bool,
    read_only: bool,
    custom_validity: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OptionState {
    value: String,
    selected: bool,
    disabled: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    start: usize,
    end: usize,
    direction: String,
}
/// Complete bounded shape validation; no resource callbacks, execution or replay.
pub fn validate(input: &str) -> Result<String, &'static str> {
    if input.len() > MAX_BYTES {
        return Err("cem.checkpoint.controls.limit");
    }
    let state: State = serde_json::from_str(input).map_err(|_| "cem.checkpoint.controls.format")?;
    if state.version != "cem-checkpoint-controls/1" || state.controls.len() > 256 {
        return Err("cem.checkpoint.controls.format");
    }
    let mut ids = BTreeSet::new();
    let mut focused = 0;
    for control in &state.controls {
        if control.id.is_empty()
            || control.id.len() > 1024
            || control.id.chars().any(char::is_control)
            || !ids.insert(&control.id)
            || control.options.len() > 1024
        {
            return Err("cem.checkpoint.controls.identity");
        }
        focused += usize::from(control.focused);
        if focused > 1 {
            return Err("cem.checkpoint.controls.focus");
        }
        let text = match control.kind.as_str() {
            "text" => matches!(
                control.input_type.as_str(),
                "text"
                    | "search"
                    | "tel"
                    | "url"
                    | "email"
                    | "number"
                    | "date"
                    | "datetime-local"
                    | "month"
                    | "week"
                    | "time"
                    | "color"
                    | "range"
                    | "hidden"
                    | "textarea"
                    | "plaintext"
            ),
            "credential-empty" => {
                control.value.is_empty()
                    && control.custom_validity.is_empty()
                    && control.selection.is_none()
                    && matches!(
                        control.input_type.as_str(),
                        "password"
                            | "text"
                            | "search"
                            | "tel"
                            | "url"
                            | "email"
                            | "number"
                            | "textarea"
                    )
            }
            "check" => matches!(control.input_type.as_str(), "checkbox" | "radio"),
            "select" => matches!(
                control.input_type.as_str(),
                "select-one" | "select-multiple"
            ),
            "button" => matches!(control.input_type.as_str(), "button" | "submit" | "reset"),
            _ => false,
        };
        if !text
            || (control.kind != "check" && (control.checked || control.indeterminate))
            || (control.kind == "check" && control.input_type == "radio" && control.indeterminate)
            || (control.kind != "select" && (control.multiple || !control.options.is_empty()))
        {
            return Err("cem.checkpoint.controls.shape");
        }
        if control.kind == "select" {
            let selected: Vec<_> = control
                .options
                .iter()
                .filter(|option| option.selected)
                .collect();
            if control.multiple != (control.input_type == "select-multiple")
                || (!control.multiple && selected.len() > 1)
                || selected.first().map_or("", |option| option.value.as_str()) != control.value
            {
                return Err("cem.checkpoint.controls.selection");
            }
        }
        if let Some(selection) = &control.selection {
            if control.kind != "text"
                || !matches!(
                    control.input_type.as_str(),
                    "text" | "search" | "tel" | "url" | "textarea" | "plaintext"
                )
                || selection.start > selection.end
                || selection.end > control.value.encode_utf16().count()
                || !matches!(
                    selection.direction.as_str(),
                    "none" | "forward" | "backward"
                )
            {
                return Err("cem.checkpoint.controls.selection");
            }
        }
    }
    let output = serde_json::to_string(&state).map_err(|_| "cem.checkpoint.controls.format")?;
    if output.len() > MAX_BYTES {
        return Err("cem.checkpoint.controls.limit");
    }
    Ok(output)
}
