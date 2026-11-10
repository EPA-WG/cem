//! Resource-free form and live slice bindings for canonical browser adoption.
use super::{fail, island, plan_attributes, AtomValue, BTreeMap, Item, RenderPlanAttribute};
use cem_ml::source_map::SourceMapStack;
use std::{collections::VecDeque, rc::Rc};

pub(super) struct Author {
    pub id: String,
    pub defaults: BTreeMap<String, (AtomValue, SourceMapStack)>,
}

pub(super) type PayloadAuthors = BTreeMap<String, Rc<Author>>;

pub(super) struct Projection {
    pub author: Rc<Author>,
    pub payload_authors: PayloadAuthors,
    pub origins: VecDeque<Option<String>>,
    pub proofs: Vec<ControlProof>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlProof {
    pub source_key: String,
    pub form_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slice_owner: Option<String>,
}

pub(super) fn capture_key(item: &Item) -> Result<String, String> {
    crate::render::dom_origin_key(item).ok_or_else(fail)
}

impl Projection {
    pub fn take_author(&mut self) -> Result<(Option<String>, Rc<Author>), String> {
        let origin = self.origins.pop_front().ok_or_else(fail)?;
        // A copied control inherits only the author already proven at its
        // position in the inert payload. Its receiving form is bound separately.
        let author = match &origin {
            Some(key) => self.payload_authors.get(key).ok_or_else(fail)?,
            None => &self.author,
        };
        Ok((origin, Rc::clone(author)))
    }
}

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
) -> Result<(), String> {
    let attributes = plan_attributes(attributes)?;
    admit_attributes(tag, &attributes, defaults)
}

pub(super) fn admit_attributes(
    tag: &str,
    attributes: &BTreeMap<String, String>,
    defaults: &BTreeMap<String, (AtomValue, SourceMapStack)>,
) -> Result<(), String> {
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
        return Ok(());
    }
    if attributes.keys().any(|name| {
        matches!(
            name.as_str(),
            "form"
                | "dirname"
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
        _ => Ok(()),
    }
}

pub(super) fn admit_payload(
    payload: Vec<Item>,
    author: &Rc<Author>,
) -> Result<PayloadAuthors, String> {
    let mut owners = BTreeMap::new();
    for item in payload {
        let tag = island::field(&item, "name").ok_or_else(fail)?;
        admit_attributes(&tag, &island::attrs(&item)?, &author.defaults)?;
        if owners
            .insert(capture_key(&item)?, Rc::clone(author))
            .is_some()
        {
            return Err(fail());
        }
    }
    Ok(owners)
}
