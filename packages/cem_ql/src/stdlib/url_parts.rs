//! Typed immutable assembly/update core. Query registration and source mapping
//! belong to the evaluator; this layer never returns a mutable URL handle.
use std::{collections::BTreeMap, fmt};

use super::{
    url::{text, UrlError},
    url_params::{fields, singleton_string, UrlParams},
    url_setters::{self, SetterOutcome},
};
use crate::eval::{AtomValue, Item};
use ::url::{quirks, Url};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartsError {
    Url(UrlError),
    Type(String),
    Invalid(&'static str),
}
impl PartsError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Url(error) => error.code(),
            Self::Type(_) => "cem.ql.type_error",
            Self::Invalid(_) => "cem.ql.url_parts_invalid",
        }
    }
}
impl fmt::Display for PartsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Url(error) => error.fmt(f),
            Self::Type(reason) => f.write_str(reason),
            Self::Invalid(reason) => f.write_str(reason),
        }
    }
}
impl std::error::Error for PartsError {}
impl From<UrlError> for PartsError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

/// Attach the call's source range/map at evaluation time. No input values,
/// credentials or complete URLs are copied into diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartsWarning {
    pub field: &'static str,
    pub subcomponent: Option<&'static str>,
}
impl PartsWarning {
    pub fn code(self) -> &'static str {
        "cem.ql.url_setter_ignored"
    }
}
impl fmt::Display for PartsWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "URL field {} ", self.field)?;
        if let Some(part) = self.subcomponent {
            write!(f, "subcomponent {part} ")?;
        }
        f.write_str("was ignored because it is invalid or inapplicable")
    }
}

#[derive(Debug)]
pub struct PartsResult {
    pub href: AtomValue,
    pub warnings: Vec<PartsWarning>,
}

struct Parts {
    raw: BTreeMap<String, Vec<Item>>,
    values: BTreeMap<String, String>,
}
impl Parts {
    fn read(items: &[Item], assembly: bool) -> Result<Self, PartsError> {
        let [item] = items else {
            return Err(PartsError::Type("parts requires exactly one record".into()));
        };
        let raw = fields(item).map_err(|error| PartsError::Type(error.to_string()))?;
        let mut values = BTreeMap::new();
        // Validate known field shapes before keys/conflicts or URL syntax.
        // BTreeMap makes this independent of native record enumeration order.
        for (key, value) in &raw {
            let parsed = match key.as_str() {
                "href" if assembly => text(value, "parts.href")?,
                "query" => UrlParams::from_entries(value)
                    .map_err(|_| PartsError::Type("parts.query requires parameter entries".into()))?
                    .serialize(),
                "protocol" | "host" | "hostname" | "port" | "username" | "password"
                | "pathname" | "search" | "hash" => singleton_string(value).map_err(|_| {
                    PartsError::Type(format!("parts.{key} requires exactly one string"))
                })?,
                _ => continue,
            };
            values.insert(key.clone(), parsed);
        }
        Ok(Self { raw, values })
    }

    fn validate(&self, assembly: bool, has_base: bool) -> Result<(), PartsError> {
        for key in self.raw.keys() {
            match key.as_str() {
                "href" if assembly => (),
                "host" if self.raw.contains_key("hostname") || self.raw.contains_key("port") => {
                    return Err(PartsError::Invalid("host conflicts with hostname or port"))
                }
                "query" if self.raw.contains_key("search") => {
                    return Err(PartsError::Invalid("query conflicts with search"))
                }
                "protocol" | "host" | "hostname" | "port" | "username" | "password"
                | "pathname" | "search" | "query" | "hash" => (),
                _ => {
                    return Err(PartsError::Invalid(
                        "parts contains an unknown or read-only field",
                    ))
                }
            }
        }
        if assembly && !has_base && !self.values.contains_key("href") {
            return Err(PartsError::Invalid(
                "assembly requires href or an explicit base",
            ));
        }
        Ok(())
    }

    fn apply(self, mut url: Url) -> PartsResult {
        let mut warnings = Vec::new();
        for field in [
            "protocol", "host", "hostname", "port", "username", "password", "pathname", "search",
            "query", "hash",
        ] {
            let Some(value) = self.values.get(field) else {
                continue;
            };
            let checked = |result: Result<(), ()>| match result {
                Ok(()) => SetterOutcome::Applied,
                Err(()) => SetterOutcome::Ignored(field),
            };
            let outcome = match field {
                "protocol" => url_setters::set_protocol(&mut url, value),
                "host" => url_setters::set_host(&mut url, value),
                "hostname" => checked(quirks::set_hostname(&mut url, value)),
                "port" => url_setters::set_port(&mut url, value),
                "username" => checked(quirks::set_username(&mut url, value)),
                "password" => checked(quirks::set_password(&mut url, value)),
                "pathname" => url_setters::set_pathname(&mut url, value),
                "search" | "query" => {
                    quirks::set_search(&mut url, value);
                    SetterOutcome::Applied
                }
                "hash" => {
                    quirks::set_hash(&mut url, value);
                    SetterOutcome::Applied
                }
                _ => unreachable!(),
            };
            if let SetterOutcome::Ignored(component) = outcome {
                warnings.push(PartsWarning {
                    field,
                    subcomponent: (component == "host.port").then_some("port"),
                });
            }
        }
        PartsResult {
            href: AtomValue::AnyUri(url.into()),
            warnings,
        }
    }
}

pub fn assemble(parts: &[Item], base: Option<&[Item]>) -> Result<PartsResult, PartsError> {
    let parts = Parts::read(parts, true)?;
    let base = base.map(|items| text(items, "base")).transpose()?;
    parts.validate(true, base.is_some())?;
    let base = base
        .as_deref()
        .map(Url::parse)
        .transpose()
        .map_err(|_| UrlError::InvalidBase)?;
    let url = match parts.values.get("href") {
        Some(seed) => Url::options()
            .base_url(base.as_ref())
            .parse(seed)
            .map_err(|_| UrlError::InvalidUrl)?,
        None => base.expect("validated assembly seed"),
    };
    Ok(parts.apply(url))
}

pub fn with_parts(input: &[Item], parts: &[Item]) -> Result<PartsResult, PartsError> {
    let input = text(input, "input")?;
    let parts = Parts::read(parts, false)?;
    parts.validate(false, false)?;
    let url = Url::parse(&input).map_err(|_| UrlError::InvalidUrl)?;
    Ok(parts.apply(url))
}
