//! Bounded declaration control metadata. Names classify retained input; they grant no authority.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const HEADERS: [&str; 3] = ["payload-item", "payload-group", "payload-topology"];
const VERSION: &str = "cem-retained-payload/1";
fn invalid() -> String {
    "cem.payload_contract.invalid: expected bounded, unambiguous item/group declaration metadata"
        .into()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PayloadName {
    pub namespace: Option<String>,
    pub local_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PayloadVocabulary {
    pub item: PayloadName,
    pub group: Option<PayloadName>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedPayloadContract {
    pub contract: String,
    pub topology: String,
    pub vocabularies: Vec<PayloadVocabulary>,
}
fn space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{c}')
}
fn start(c: char) -> bool {
    matches!(c, 'A'..='Z' | '_' | 'a'..='z' | '\u{c0}'..='\u{d6}' | '\u{d8}'..='\u{f6}'
        | '\u{f8}'..='\u{2ff}' | '\u{370}'..='\u{37d}' | '\u{37f}'..='\u{1fff}' | '\u{200c}'..='\u{200d}'
        | '\u{2070}'..='\u{218f}' | '\u{2c00}'..='\u{2fef}' | '\u{3001}'..='\u{d7ff}'
        | '\u{f900}'..='\u{fdcf}' | '\u{fdf0}'..='\u{fffd}' | '\u{10000}'..='\u{effff}')
}
fn valid_name(name: &PayloadName) -> bool {
    let mut chars = name.local_name.chars();
    chars.next().is_some_and(start) && name.local_name.chars().count() <= 256
        && chars.all(|c| start(c) || matches!(c, '0'..='9' | '.' | '-' | '\u{b7}' | '\u{300}'..='\u{36f}' | '\u{203f}'..='\u{2040}'))
        && name.namespace.as_ref().is_none_or(|ns| !ns.is_empty() && ns.chars().count() <= 1024
            && !ns.chars().any(|c| c.is_whitespace() || c.is_control() || matches!(c, '\u{feff}' | '{' | '}')))
}
fn parse_name(value: &str) -> Result<PayloadName, String> {
    let (namespace, local_name) = if let Some(rest) = value.strip_prefix("Q{") {
        let (ns, local) = rest.split_once('}').ok_or_else(invalid)?;
        ((!ns.is_empty()).then(|| ns.to_string()), local.to_string())
    } else {
        (None, value.to_string())
    };
    let name = PayloadName {
        namespace,
        local_name,
    };
    if !valid_name(&name) {
        return Err(invalid());
    }
    Ok(name)
}
impl RetainedPayloadContract {
    pub fn validate(&self) -> Result<(), String> {
        if self.contract != VERSION
            || !["flat", "grouped", "recursive"].contains(&self.topology.as_str())
            || self.vocabularies.is_empty()
            || self.vocabularies.len() > 8
        {
            return Err(invalid());
        }
        let mut names = BTreeSet::new();
        for vocabulary in &self.vocabularies {
            if (self.topology == "grouped") != vocabulary.group.is_some() {
                return Err(invalid());
            }
            for name in std::iter::once(&vocabulary.item).chain(vocabulary.group.as_ref()) {
                if !valid_name(name)
                    || !names.insert((name.namespace.as_deref(), name.local_name.as_str()))
                {
                    return Err(invalid());
                }
            }
        }
        Ok(())
    }
    fn block(header: &BTreeMap<String, String>) -> Result<Option<Self>, String> {
        if !HEADERS.iter().any(|key| header.contains_key(*key)) {
            return Ok(None);
        }
        if HEADERS
            .iter()
            .any(|key| header.get(*key).is_some_and(|value| value.len() > 8192))
        {
            return Err(invalid());
        }
        let items = header
            .get("payload-item")
            .filter(|value| !value.trim_matches(space).is_empty())
            .ok_or_else(invalid)?;
        let items: Vec<_> = items
            .split(space)
            .filter(|s| !s.is_empty())
            .take(9)
            .map(parse_name)
            .collect::<Result<_, _>>()?;
        let topology = header
            .get("payload-topology")
            .map(|s| s.trim_matches(space))
            .unwrap_or("flat");
        let groups = header
            .get("payload-group")
            .map(|value| {
                value
                    .split(space)
                    .filter(|s| !s.is_empty())
                    .take(9)
                    .map(parse_name)
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;
        if groups
            .as_ref()
            .is_some_and(|groups| groups.len() != items.len())
        {
            return Err(invalid());
        }
        let contract = Self {
            contract: VERSION.into(),
            topology: topology.into(),
            vocabularies: items
                .into_iter()
                .enumerate()
                .map(|(index, item)| PayloadVocabulary {
                    item,
                    group: groups.as_ref().map(|groups| groups[index].clone()),
                })
                .collect(),
        };
        contract.validate()?;
        Ok(Some(contract))
    }
    /// Header blocks are atomic: never merge a partial template block into its owner.
    pub fn resolve(
        owner: &BTreeMap<String, String>,
        template: &BTreeMap<String, String>,
    ) -> Result<Option<Self>, String> {
        let own = Self::block(owner)?;
        let inner = Self::block(template)?;
        if own.is_some() && inner.is_some() && own != inner {
            return Err(invalid());
        }
        Ok(own.or(inner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn header(values: &[(&str, &str)]) -> BTreeMap<String, String> {
        values
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect()
    }
    #[test]
    fn retained_payload_contract_names_and_atomic_blocks() {
        let root = header(&[
            ("payload-item", "Q{urn:data}entrée Q{}item"),
            ("payload-topology", "recursive"),
        ]);
        let contract = RetainedPayloadContract::resolve(&root, &root)
            .unwrap()
            .unwrap();
        assert_eq!(
            contract.vocabularies[0].item.namespace.as_deref(),
            Some("urn:data")
        );
        assert_eq!(contract.vocabularies[1].item.namespace, None);
        assert!(
            RetainedPayloadContract::resolve(&root, &header(&[("payload-group", "group")]))
                .is_err()
        );
        assert_eq!(
            RetainedPayloadContract::resolve(
                &header(&[("payload-item", "item")]),
                &header(&[("payload-item", "Q{}item")])
            )
            .unwrap()
            .unwrap()
            .topology,
            "flat"
        );
    }
    #[test]
    fn retained_payload_contract_rejects_selector_syntax_and_bounds() {
        let oversized = "a".repeat(257);
        let aliases = (0..9)
            .map(|i| format!("item{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        for name in [
            "*",
            ".entry",
            "[slot=item]",
            "a:item",
            "1item",
            "Q{urn:missing",
            "Q{urn:bad space}item",
            &oversized,
            &aliases,
        ] {
            assert!(
                RetainedPayloadContract::resolve(
                    &header(&[("payload-item", name)]),
                    &BTreeMap::new()
                )
                .is_err(),
                "{name}"
            );
        }
        for topology in ["grouped", "unknown", ""] {
            assert!(RetainedPayloadContract::resolve(
                &header(&[("payload-item", "item"), ("payload-topology", topology)]),
                &BTreeMap::new()
            )
            .is_err());
        }
    }
}
