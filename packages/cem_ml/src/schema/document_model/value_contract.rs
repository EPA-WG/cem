//! Value conversion uses the same lexical rules and facets as schema validation.
use super::*;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AttributeValueContract {
    pub model: AttributeModel,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TypedAttributeValue {
    pub datatype: String,
    pub lexical: String,
}

pub fn convert_attribute_value(
    value: &str,
    contract: &AttributeValueContract,
    source: &SourceMapStack,
) -> Result<TypedAttributeValue, Vec<Diagnostic>> {
    let mut model = contract.model.clone();
    let datatype = model
        .value_type
        .as_deref()
        .unwrap_or("string")
        .trim_start_matches("schema:")
        .to_owned();
    let fail = |message: String| {
        vec![Diagnostic {
            uri: None,
            line: None,
            column: None,
            byte_offset: None,
            code: "cem.value.contract".into(),
            severity: Severity::Error,
            message,
            node: None,
            details: None,
            source_map: Some(source.clone()),
        }]
    };
    if let Some(pattern) = &model.pattern {
        if compile_full_value_pattern(pattern).is_err() {
            return Err(fail("Invalid full-value regex constraint".into()));
        }
    }
    let mut lexical = if model.white_space.is_some() {
        validation_attribute_value(value, &model).into_owned()
    } else if datatype == "string" {
        value.to_owned()
    } else {
        value.trim().to_owned()
    };
    match datatype.as_str() {
        "integer" => {
            let Some((negative, digits)) = normalize_decimal_integer(&lexical) else {
                return Err(fail("Invalid integer value".into()));
            };
            lexical = format!(
                "{}{digits}",
                if negative && digits != "0" { "-" } else { "" }
            );
            model.value_type = Some("schema:integer".into());
        }
        "decimal" | "number" => {
            if !is_finite_decimal_number(&lexical) {
                return Err(fail("Invalid finite decimal value".into()));
            }
            model.value_type = Some("schema:number".into());
        }
        "boolean" => {
            lexical = match lexical.as_str() {
                "true" | "1" => "true",
                "false" | "0" => "false",
                _ => return Err(fail("Invalid boolean value".into())),
            }
            .into();
            model.value_type = Some("schema:boolean".into());
        }
        "date" | "time" | "dateTime" | "datetime" => {
            if !valid_temporal(&lexical, &datatype) {
                return Err(fail(format!("Invalid {datatype} value")));
            }
            model.value_type = Some("schema:string".into());
        }
        "string" => model.value_type = Some("schema:string".into()),
        _ => return Err(fail(format!("Unresolved attribute datatype `{datatype}`"))),
    }
    let node = CemAstNode::Attribute {
        node_id: 0,
        expanded_name: ExpandedName {
            namespace_uri: String::new(),
            local_name: model.name.clone(),
            schema_id: None,
        },
        value: Some(lexical.clone()),
        source: source.clone(),
    };
    let mut diagnostics = Vec::new();
    validate_attribute_contracts(
        "cem:value",
        &BTreeMap::new(),
        "",
        &model.name,
        &lexical,
        &model,
        &BTreeMap::new(),
        &node,
        &mut diagnostics,
    );
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        Err(diagnostics)
    } else {
        Ok(TypedAttributeValue { datatype, lexical })
    }
}

// Explicit, locale-independent lexical profile. An absent zone stays absent;
// conversion never consults a clock, machine locale or environment timezone.
fn valid_temporal(value: &str, datatype: &str) -> bool {
    if !value.is_ascii() {
        return false;
    }
    fn zone(value: &str) -> Option<&str> {
        if let Some(value) = value.strip_suffix('Z') {
            return Some(value);
        }
        if value.len() >= 6 {
            let (value, zone) = value.split_at(value.len() - 6);
            if zone.starts_with(['+', '-']) && zone.as_bytes()[3] == b':' {
                let (hours, minutes) = zone[1..].split_once(':')?;
                if hours.len() != 2
                    || minutes.len() != 2
                    || !hours
                        .bytes()
                        .chain(minutes.bytes())
                        .all(|b| b.is_ascii_digit())
                {
                    return None;
                }
                let h = hours.parse::<u32>().ok()?;
                let m = minutes.parse::<u32>().ok()?;
                return (h <= 14 && m < 60 && (h != 14 || m == 0)).then_some(value);
            }
        }
        Some(value)
    }
    fn date(value: &str) -> bool {
        let parts: Vec<_> = value.split('-').collect();
        if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
            return false;
        }
        if !parts
            .iter()
            .all(|part| part.bytes().all(|b| b.is_ascii_digit()))
        {
            return false;
        }
        let values: Option<Vec<u32>> = parts.iter().map(|p| p.parse().ok()).collect();
        let Some(values) = values else {
            return false;
        };
        let (y, m, d) = (values[0], values[1], values[2]);
        let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
        let days = match m {
            2 => {
                if leap {
                    29
                } else {
                    28
                }
            }
            4 | 6 | 9 | 11 => 30,
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            _ => 0,
        };
        y != 0 && d > 0 && d <= days
    }
    fn time(value: &str) -> bool {
        let parts: Vec<_> = value.split(':').collect();
        if parts.len() != 3 || parts[0].len() != 2 || parts[1].len() != 2 {
            return false;
        }
        let seconds = parts[2]
            .split_once('.')
            .map_or(Some(parts[2]), |(whole, fraction)| {
                (!fraction.is_empty() && fraction.bytes().all(|b| b.is_ascii_digit()))
                    .then_some(whole)
            });
        let Some(seconds) = seconds.filter(|s| s.len() == 2) else {
            return false;
        };
        if !parts[0]
            .bytes()
            .chain(parts[1].bytes())
            .chain(seconds.bytes())
            .all(|b| b.is_ascii_digit())
        {
            return false;
        }
        matches!((parts[0].parse::<u32>(), parts[1].parse::<u32>(), seconds.parse::<u32>()), (Ok(h), Ok(m), Ok(s)) if h < 24 && m < 60 && s < 60)
    }
    let Some(value) = zone(value) else {
        return false;
    };
    match datatype {
        "date" => date(value),
        "time" => time(value),
        _ => value
            .split_once('T')
            .is_some_and(|(d, t)| date(d) && time(t)),
    }
}
