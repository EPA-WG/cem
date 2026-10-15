//! Typed authored geometry candidates. Percentages remain unresolved and no
//! consumer bounding box, viewport, defaults or CSS transform winner is invented.
use super::{paint::PaintAttributeSource, *};
use cssparser::{Parser, ParserInput, Token};
#[derive(Debug, PartialEq)]
pub enum GeometryValue {
    Number(f32),
    Pixels(f32),
    Percentage(f32),
    ViewBox([f32; 4]),
    Keyword(String),
}
pub struct GeometryCandidate {
    pub name: &'static str,
    pub source: Option<(AstNodeId, AstNodeId)>,
    pub value: Result<Option<GeometryValue>, String>,
}
pub(super) fn derive(
    tree: &RetainedCemTree,
    fields: &[PaintAttributeSource],
    work: &mut paint::Work,
) -> Result<Vec<GeometryCandidate>, String> {
    fields
        .iter()
        .map(|field| {
            work.spend(1)?;
            let value = match field.source {
                None => Ok(None),
                Some((_, id)) => {
                    let node = tree.node(id).ok_or("geometry attribute missing")?;
                    work.spend(node.value.len())?;
                    parse(field.name, &node.value).map(Some)
                }
            };
            Ok(GeometryCandidate {
                name: field.name,
                source: field.source,
                value,
            })
        })
        .collect()
}
fn parse(name: &str, value: &str) -> Result<GeometryValue, String> {
    if matches!(
        name,
        "gradientUnits" | "patternUnits" | "patternContentUnits" | "spreadMethod"
    ) {
        let value = value.trim_matches(|c: char| c.is_ascii_whitespace());
        let valid = if name == "spreadMethod" {
            matches!(value, "pad" | "reflect" | "repeat")
        } else {
            matches!(value, "userSpaceOnUse" | "objectBoundingBox")
        };
        return if valid {
            Ok(GeometryValue::Keyword(value.into()))
        } else {
            Err("unsupported geometry keyword".into())
        };
    }
    if matches!(
        name,
        "gradientTransform" | "patternTransform" | "preserveAspectRatio"
    ) {
        return Err("geometry transform/aspect-ratio evaluation required".into());
    }
    // SVG geometry attributes do not accept CSS comments, escapes or functions.
    // Keep the accepted lexical profile narrower than cssparser's CSS grammar.
    if !value.bytes().all(|c| {
        c.is_ascii_digit()
            || matches!(
                c,
                b'+' | b'-'
                    | b'.'
                    | b'e'
                    | b'E'
                    | b'p'
                    | b'x'
                    | b'%'
                    | b','
                    | b' '
                    | b'\t'
                    | b'\r'
                    | b'\n'
            )
    }) {
        return Err("unsupported SVG geometry syntax".into());
    }
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    if name == "viewBox" {
        let mut values = [0.; 4];
        for (i, slot) in values.iter_mut().enumerate() {
            if i > 0 {
                let _ = parser.try_parse(|p| p.expect_comma());
            }
            *slot = parser
                .expect_number()
                .map_err(|_| "viewBox requires four numbers")?;
            if !slot.is_finite() {
                return Err("nonfinite viewBox".into());
            }
        }
        if !parser.is_exhausted() || values[2] < 0. || values[3] < 0. {
            return Err("invalid viewBox".into());
        }
        return Ok(GeometryValue::ViewBox(values));
    }
    let token = parser.next().map_err(|_| "empty geometry value")?.clone();
    if !parser.is_exhausted() {
        return Err("geometry requires a single length".into());
    }
    let (number, result) = match token {
        Token::Number { value, .. } => (value, GeometryValue::Number(value)),
        Token::Percentage { unit_value, .. } => (
            unit_value * 100.,
            GeometryValue::Percentage(unit_value * 100.),
        ),
        Token::Dimension { value, unit, .. } if unit.as_ref() == "px" => {
            (value, GeometryValue::Pixels(value))
        }
        _ => return Err("unsupported geometry length".into()),
    };
    if !number.is_finite() || (matches!(name, "r" | "fr" | "width" | "height") && number < 0.) {
        return Err("invalid geometry length".into());
    }
    Ok(result)
}
