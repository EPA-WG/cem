use super::*;
use crate::svg_resource_acquisition::paint::AcquiredPaintContent;
/// Explicit numeric assumptions in the graphics consumer's local user space.
/// This is not a browser layout receipt or transferable execution evidence.
#[derive(Clone, Copy, Debug)]
pub struct GeometryContext {
    pub viewport: [f64; 2],
    pub bounding_box: [f64; 4],
}
#[derive(Clone, Copy, Debug)]
pub enum GeometryOrigin {
    Attribute {
        owner: AstNodeId,
        attribute: AstNodeId,
    },
    Initial,
    DerivedFrom(&'static str),
}
#[derive(Clone, Copy, Debug)]
pub enum ResolvedGradient {
    Linear {
        start: [f64; 2],
        end: [f64; 2],
    },
    Radial {
        center: [f64; 2],
        radius: f64,
        focus: [f64; 2],
        inner_radius: f64,
    },
}
pub struct GradientEvaluation<'a> {
    pub source: &'a AcquiredPaintContent,
    pub context: GeometryContext,
    pub shape: ResolvedGradient,
    pub paint_to_user: Matrix,
    pub origins: BTreeMap<&'static str, GeometryOrigin>,
    pub disabled: bool,
}
impl AcquiredPaintContent {
    pub fn resolve_gradient(
        &self,
        context: GeometryContext,
    ) -> Result<GradientEvaluation<'_>, String> {
        if !context
            .viewport
            .iter()
            .chain(context.bounding_box.iter())
            .all(|n| n.is_finite())
            || context.viewport.iter().any(|n| *n < 0.)
            || context.bounding_box[2] < 0.
            || context.bounding_box[3] < 0.
        {
            return Err("invalid SVG geometry context".into());
        }
        // CSS transform overrides, units and layout are a separate profile. This
        // method resolves retained attribute candidates without guessing winners.
        if !self.inventory.stylesheets.is_empty() {
            return Err("gradient CSS geometry winner evidence required".into());
        }
        let kind = crate::svg_paint_dependencies::kind(&self.inventory.document.tree, self.host)
            .ok_or("gradient host missing")?;
        if !matches!(kind, "linearGradient" | "radialGradient") {
            return Err("gradient geometry required".into());
        }
        let mut origins = BTreeMap::new();
        let units = field(self, "gradientUnits", &mut origins)?;
        let bbox = match units {
            None => true,
            Some(GeometryValue::Keyword(s)) if s == "objectBoundingBox" => true,
            Some(GeometryValue::Keyword(s)) if s == "userSpaceOnUse" => false,
            _ => return Err("unsupported gradient units".into()),
        };
        let transform = match field(self, "gradientTransform", &mut origins)? {
            None => Matrix::IDENTITY,
            Some(GeometryValue::Transform(m)) => *m,
            _ => return Err("unsupported gradient transform".into()),
        };
        let paint_to_user = if bbox {
            Matrix::checked([
                context.bounding_box[2],
                0.,
                0.,
                context.bounding_box[3],
                context.bounding_box[0],
                context.bounding_box[1],
            ])?
            .then_local(transform)?
        } else {
            transform
        };
        let axes = if bbox {
            [1., 1., 1.]
        } else {
            [
                context.viewport[0],
                context.viewport[1],
                context.viewport[0].hypot(context.viewport[1]) / 2f64.sqrt(),
            ]
        };
        let mut length = |name, default: GeometryValue, axis: usize| -> Result<f64, String> {
            let value = field(self, name, &mut origins)?.unwrap_or(&default);
            let n = match value {
                GeometryValue::Number(n) => f64::from(*n),
                GeometryValue::Pixels(n) if !bbox => f64::from(*n),
                GeometryValue::Percentage(n) => f64::from(*n) * axes[axis] / 100.,
                _ => return Err("unsupported gradient length units".into()),
            };
            if n.is_finite() {
                Ok(n)
            } else {
                Err("nonfinite resolved gradient length".into())
            }
        };
        let shape = if kind == "linearGradient" {
            ResolvedGradient::Linear {
                start: [
                    length("x1", GeometryValue::Percentage(0.), 0)?,
                    length("y1", GeometryValue::Percentage(0.), 1)?,
                ],
                end: [
                    length("x2", GeometryValue::Percentage(100.), 0)?,
                    length("y2", GeometryValue::Percentage(0.), 1)?,
                ],
            }
        } else {
            let center = [
                length("cx", GeometryValue::Percentage(50.), 0)?,
                length("cy", GeometryValue::Percentage(50.), 1)?,
            ];
            let radius = length("r", GeometryValue::Percentage(50.), 2)?;
            let inner_radius = length("fr", GeometryValue::Number(0.), 2)?;
            // Defaults deriving from resolved centers must not round-trip f64
            // through an authored f32 field or fabricate source coordinates.
            let mut focus = center;
            for (i, name) in ["fx", "fy"].into_iter().enumerate() {
                if self
                    .geometry
                    .iter()
                    .any(|g| g.name == name && g.source.is_some())
                {
                    focus[i] = length(name, GeometryValue::Number(0.), i)?;
                }
            }
            if inner_radius > radius
                || (focus[0] - center[0]).hypot(focus[1] - center[1]) + inner_radius > radius
            {
                return Err("radial focal limiting outside bounded profile".into());
            }
            for (i, name) in ["fx", "fy"].into_iter().enumerate() {
                origins
                    .entry(name)
                    .or_insert(GeometryOrigin::DerivedFrom(if i == 0 {
                        "cx"
                    } else {
                        "cy"
                    }));
            }
            ResolvedGradient::Radial {
                center,
                radius,
                focus,
                inner_radius,
            }
        };
        let disabled = context.viewport.contains(&0.)
            || (bbox && (context.bounding_box[2] == 0. || context.bounding_box[3] == 0.))
            || paint_to_user.singular()
            || matches!(shape, ResolvedGradient::Radial { radius: 0., .. });
        Ok(GradientEvaluation {
            source: self,
            context,
            shape,
            paint_to_user,
            origins,
            disabled,
        })
    }
}
fn field<'a>(
    plan: &'a AcquiredPaintContent,
    name: &'static str,
    origins: &mut BTreeMap<&'static str, GeometryOrigin>,
) -> Result<Option<&'a GeometryValue>, String> {
    let f = plan
        .geometry
        .iter()
        .find(|f| f.name == name)
        .ok_or("geometry field missing")?;
    origins.insert(
        name,
        match f.source {
            Some((owner, attribute)) => GeometryOrigin::Attribute { owner, attribute },
            None => GeometryOrigin::Initial,
        },
    );
    f.value.as_ref().map(Option::as_ref).map_err(Clone::clone)
}
