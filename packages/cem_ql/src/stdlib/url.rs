//! Typed, pure URL parsing and projection into CEM-QL control records.
//!
//! No ambient base, resource loading or document conversion.
//! The pinned Rust parser's remaining file/IDNA limitations are in wishlist.md.
use std::{collections::BTreeMap, fmt};

use super::url_origin::serialize_origin;
use super::url_params::UrlParams;
use crate::eval::{AtomValue, Item, QueryItemViewKind};
use ::url::{quirks, Url};

/// The evaluator attaches the call's source range and preserves prior diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlError {
    Type(&'static str),
    InvalidBase,
    InvalidUrl,
}
impl UrlError {
    pub fn code(self) -> &'static str {
        match self {
            Self::Type(_) => "cem.ql.type_error",
            Self::InvalidBase => "cem.ql.url_base_invalid",
            Self::InvalidUrl => "cem.ql.url_invalid",
        }
    }
}
impl fmt::Display for UrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Type(argument) => write!(
                f,
                "{argument} must be exactly one string or anyURI atomic item"
            ),
            Self::InvalidBase => f.write_str("base is not a valid absolute URL"),
            Self::InvalidUrl => f.write_str("input is not a valid URL for the supplied base"),
        }
    }
}
impl std::error::Error for UrlError {}

/// Suppress syntax failures only; argument shape/cardinality errors remain errors.
pub fn can_parse(input: &[Item], base: Option<&[Item]>) -> Result<bool, UrlError> {
    Ok(optional_url(input, base)?.is_some())
}

/// Return authoritative serialization, retaining even empty query/hash delimiters.
pub fn href(input: &[Item], base: Option<&[Item]>) -> Result<AtomValue, UrlError> {
    checked_url(input, base).map(|url| AtomValue::AnyUri(url.into()))
}

/// Return zero or one parsed record; syntax failure is absence, never a null item.
pub fn parse(input: &[Item], base: Option<&[Item]>) -> Result<Option<Item>, UrlError> {
    optional_url(input, base).map(|url| url.map(|url| to_record(&url)))
}

fn optional_url(input: &[Item], base: Option<&[Item]>) -> Result<Option<Url>, UrlError> {
    match checked_url(input, base) {
        Ok(url) => Ok(Some(url)),
        Err(UrlError::InvalidBase | UrlError::InvalidUrl) => Ok(None),
        Err(error) => Err(error),
    }
}

fn checked_url(input: &[Item], base: Option<&[Item]>) -> Result<Url, UrlError> {
    // All argument shapes precede syntax checks, in argument order.
    let input = text(input, "input")?;
    let base = base.map(|items| text(items, "base")).transpose()?;
    let base = base
        .as_deref()
        .map(Url::parse)
        .transpose()
        .map_err(|_| UrlError::InvalidBase)?;
    Url::options()
        .base_url(base.as_ref())
        .parse(&input)
        .map_err(|_| UrlError::InvalidUrl)
}

pub(super) fn text(items: &[Item], argument: &'static str) -> Result<String, UrlError> {
    let atom = match items {
        [Item::Atomic(atom)] => Some(atom.clone()),
        [Item::Native(view)] if view.kind() == QueryItemViewKind::Atomic => view.atom(),
        _ => None,
    };
    match atom {
        Some(AtomValue::String(value) | AtomValue::AnyUri(value)) => Ok(value),
        _ => Err(UrlError::Type(argument)),
    }
}

fn to_record(url: &Url) -> Item {
    let mut fields = BTreeMap::new();
    fields.insert(
        "href".into(),
        vec![Item::Atomic(AtomValue::AnyUri(url.as_str().into()))],
    );
    fields.insert(
        "origin".into(),
        vec![Item::Atomic(AtomValue::String(serialize_origin(url)))],
    );
    for (name, value) in [
        ("protocol", quirks::protocol(url)),
        ("username", quirks::username(url)),
        ("password", quirks::password(url)),
        ("host", quirks::host(url)),
        ("hostname", quirks::hostname(url)),
        ("port", quirks::port(url)),
        ("pathname", quirks::pathname(url)),
        ("search", quirks::search(url)),
        ("hash", quirks::hash(url)),
    ] {
        fields.insert(
            name.into(),
            vec![Item::Atomic(AtomValue::String(value.into()))],
        );
    }
    // This known singleton string cannot fail the Params initializer's shape check.
    let query = UrlParams::from_initializer(Some(&[Item::Atomic(AtomValue::String(
        quirks::search(url).into(),
    ))]))
    .expect("query initializer is one string");
    fields.insert("query".into(), query.to_entries());
    Item::Record(fields)
}

/// Pure URL and immutable parameter query functions.
pub const MODULE_URI: &str = "cem:stdlib/url";
pub const FUNCTIONS: &[super::StdlibFunction] = &[
    super::StdlibFunction::native_range(MODULE_URI, "can_parse", 1, 2, super::Tier::A),
    super::StdlibFunction::native_range(MODULE_URI, "parse", 1, 2, super::Tier::A),
    super::StdlibFunction::native_range(MODULE_URI, "href", 1, 2, super::Tier::A),
    super::StdlibFunction::native_range(MODULE_URI, "assemble", 1, 2, super::Tier::A),
    super::StdlibFunction::native(MODULE_URI, "with_parts", 2, super::Tier::A),
    super::StdlibFunction::native_range(MODULE_URI, "params", 0, 1, super::Tier::A),
    super::StdlibFunction::native_range(MODULE_URI, "params_size", 1, 1, super::Tier::A),
    super::StdlibFunction::native_range(MODULE_URI, "params_entries", 1, 1, super::Tier::A),
    super::StdlibFunction::native_range(MODULE_URI, "params_keys", 1, 1, super::Tier::A),
    super::StdlibFunction::native_range(MODULE_URI, "params_values", 1, 1, super::Tier::A),
    super::StdlibFunction::native_range(MODULE_URI, "params_get", 2, 2, super::Tier::A),
    super::StdlibFunction::native_range(MODULE_URI, "params_get_all", 2, 2, super::Tier::A),
    super::StdlibFunction::native_range(MODULE_URI, "params_has", 2, 3, super::Tier::A),
    super::StdlibFunction::native_range(MODULE_URI, "params_append", 3, 3, super::Tier::A),
    super::StdlibFunction::native_range(MODULE_URI, "params_delete", 2, 3, super::Tier::A),
    super::StdlibFunction::native_range(MODULE_URI, "params_set", 3, 3, super::Tier::A),
    super::StdlibFunction::native_range(MODULE_URI, "params_sort", 1, 1, super::Tier::A),
    super::StdlibFunction::native_range(MODULE_URI, "params_string", 1, 1, super::Tier::A),
];
