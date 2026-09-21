//! Value conversion uses the same lexical rules and facets as schema validation.
use super::*;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AttributeValueContract {
    pub model: AttributeModel,
    pub content_type: Option<String>,
    /// Additional constraints intersect the base model; they never replace it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub restrictions: Vec<AttributeModel>,
}

impl AttributeValueContract {
    pub fn models(&self) -> impl Iterator<Item = &AttributeModel> {
        std::iter::once(&self.model).chain(&self.restrictions)
    }

    /// Retain another contract's constraints without changing this contract's
    /// conversion type or final representation.
    pub fn restrict_with(&mut self, other: &Self) {
        self.restrictions.extend(other.models().cloned());
    }

    pub fn has_constraints(&self) -> bool {
        self.models().any(|model| {
            let mut facets = model.clone();
            facets.name.clear();
            facets.value_type = None;
            facets.default_value = None;
            facets.source_map = Default::default();
            facets != AttributeModel::default()
        })
    }

    pub fn accounted_bytes(&self) -> usize {
        crate::value::artifact::metadata_bytes(self)
            .saturating_add(std::mem::size_of::<Self>())
            .saturating_add(
                self.restrictions
                    .len()
                    .saturating_mul(std::mem::size_of::<AttributeModel>()),
            )
    }
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
    match convert_attribute_value_with_check(value, contract, source, &mut || {
        Ok::<_, std::convert::Infallible>(())
    }) {
        Ok(value) => Ok(value),
        Err(AttributeValueConversionError::Invalid(diagnostics)) => Err(diagnostics),
        Err(AttributeValueConversionError::Interrupted(never)) => match never {},
    }
}

#[derive(Debug)]
pub enum AttributeValueConversionError<E> {
    Invalid(Vec<Diagnostic>),
    Interrupted(E),
}

/// Poll between constraint models, preserving control failures separately from
/// ordinary (potentially recoverable) invalid input diagnostics.
pub fn convert_attribute_value_with_check<E>(
    value: &str,
    contract: &AttributeValueContract,
    source: &SourceMapStack,
    check: &mut impl FnMut() -> Result<(), E>,
) -> Result<TypedAttributeValue, AttributeValueConversionError<E>> {
    use AttributeValueConversionError::{Interrupted, Invalid};
    let mut normalization = None;
    for model in contract.models() {
        check().map_err(Interrupted)?;
        let rank = match model.white_space.as_deref() {
            None => continue,
            Some("preserve") => 0,
            Some("replace") => 1,
            Some("collapse") => 2,
            Some(_) => {
                return Err(Invalid(contract_failure(
                    "Invalid whitespace constraint",
                    source,
                )))
            }
        };
        normalization = Some(normalization.map_or(rank, |previous: usize| previous.max(rank)));
    }
    // Conversion happens once. Validation of every model sees the same final
    // value, without independently normalizing it into a different value.
    let mut base = contract.model.clone();
    base.white_space = normalization.map(|rank| ["preserve", "replace", "collapse"][rank].into());
    let converted = convert_model(value, &base, source).map_err(Invalid)?;
    for restriction in &contract.restrictions {
        check().map_err(Interrupted)?;
        let mut model = restriction.clone();
        model.value_type = model
            .value_type
            .or_else(|| Some(converted.datatype.clone()));
        model.white_space = Some("preserve".into());
        let validated = convert_model(&converted.lexical, &model, source).map_err(Invalid)?;
        if validated.lexical != converted.lexical {
            return Err(Invalid(contract_failure(
                "A restriction cannot change the final typed value",
                source,
            )));
        }
    }
    check().map_err(Interrupted)?;
    Ok(converted)
}

fn contract_failure(message: &str, source: &SourceMapStack) -> Vec<Diagnostic> {
    vec![Diagnostic {
        uri: None,
        line: None,
        column: None,
        byte_offset: None,
        code: "cem.value.contract".into(),
        severity: Severity::Error,
        message: message.into(),
        node: None,
        details: None,
        source_map: Some(source.clone()),
    }]
}

fn convert_model(
    value: &str,
    model: &AttributeModel,
    source: &SourceMapStack,
) -> Result<TypedAttributeValue, Vec<Diagnostic>> {
    let mut model = model.clone();
    let datatype = model
        .value_type
        .as_deref()
        .unwrap_or("string")
        .trim_start_matches("schema:")
        .to_owned();
    let fail = |message: String| contract_failure(&message, source);
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
    // Validate facet definitions too: a malformed bound is not an absent bound.
    let mut definition = model.clone();
    if datatype != "string" {
        // Internal preservation below is not a schema whitespace declaration.
        definition.white_space = None;
    }
    let mut definitions = Vec::new();
    validate_attribute_datatype_param_definition("cem:value", &definition, &mut definitions);
    if definitions.iter().any(|d| d.severity == Severity::Error) {
        return Err(definitions);
    }
    // The lexical value has already been normalized. Shared schema validation
    // must check that exact value, including leading/trailing string whitespace.
    model.white_space = Some("preserve".into());
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
