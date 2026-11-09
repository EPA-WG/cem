//! Resource-free form and live slice bindings for canonical browser adoption.
use super::{fail, plan_attributes, AtomValue, BTreeMap, RenderPlanAttribute};
use cem_ml::source_map::SourceMapStack;

pub(super) fn is_form_tag(tag: &str) -> bool {
    matches!(
        tag,
        "form" | "input" | "textarea" | "select" | "option" | "optgroup" | "fieldset" | "button"
    )
}

pub(super) fn admit(
    tag: &str,
    attributes: &[RenderPlanAttribute],
    defaults: &BTreeMap<String, (AtomValue, SourceMapStack)>,
) -> Result<bool, String> {
    let attributes = plan_attributes(attributes)?;
    let get = |name| attributes.get(name).map(String::as_str);
    let is_form = is_form_tag(tag);
    if get("custom-validity").is_some() || get("slice-value").is_some() {
        return Err(fail());
    }
    if let Some(slice) = get("slice") {
        if !matches!(tag, "input" | "textarea" | "select")
            || !defaults.contains_key(slice)
            || get("slice-event").is_some_and(|event| !matches!(event, "input" | "change"))
            || (tag == "select" && get("multiple").is_some())
        {
            return Err(fail());
        }
    } else if get("slice-event").is_some() {
        return Err(fail());
    }
    if !is_form {
        return Ok(false);
    }
    if attributes.keys().any(|name| {
        matches!(
            name.as_str(),
            "form"
                | "action"
                | "method"
                | "target"
                | "enctype"
                | "formaction"
                | "formmethod"
                | "formtarget"
                | "formenctype"
                | "formnovalidate"
        )
    }) || get("autocomplete").is_some_and(|value| {
        value.split_ascii_whitespace().any(|token| {
            ["current-password", "new-password", "one-time-code"]
                .iter()
                .any(|private| token.eq_ignore_ascii_case(private))
        })
    }) || ["name", "id"].iter().any(|key| {
        get(*key)
            .is_some_and(|value| matches!(value.trim(), "__proto__" | "prototype" | "constructor"))
    }) {
        return Err(fail());
    }
    match tag {
        "input"
            if !matches!(
                get("type").unwrap_or("text").to_ascii_lowercase().as_str(),
                "text"
                    | "search"
                    | "email"
                    | "url"
                    | "tel"
                    | "number"
                    | "range"
                    | "checkbox"
                    | "radio"
            ) =>
        {
            Err(fail())
        }
        "button" if get("type").is_none_or(|kind| !kind.eq_ignore_ascii_case("button")) => {
            Err(fail())
        }
        _ => Ok(true),
    }
}
