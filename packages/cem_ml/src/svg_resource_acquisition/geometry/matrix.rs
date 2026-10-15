//! Bounded SVG attribute transform semantics over retained attribute text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix([f64; 6]);
impl Matrix {
    pub const IDENTITY: Self = Self([1., 0., 0., 1., 0., 0.]);
    pub fn coefficients(self) -> [f64; 6] {
        self.0
    }
    pub(crate) fn checked(v: [f64; 6]) -> Result<Self, String> {
        if v.iter().all(|v| v.is_finite()) {
            Ok(Self(v))
        } else {
            Err("nonfinite SVG matrix".into())
        }
    }
    pub fn then_local(self, rhs: Self) -> Result<Self, String> {
        let [a, b, c, d, e, f] = self.0;
        let [g, h, i, j, k, l] = rhs.0;
        Self::checked([
            a * g + c * h,
            b * g + d * h,
            a * i + c * j,
            b * i + d * j,
            a * k + c * l + e,
            b * k + d * l + f,
        ])
    }
    pub fn map_point(self, p: [f64; 2]) -> Result<[f64; 2], String> {
        let [a, b, c, d, e, f] = self.0;
        let q = [a * p[0] + c * p[1] + e, b * p[0] + d * p[1] + f];
        if q.iter().all(|v| v.is_finite()) {
            Ok(q)
        } else {
            Err("nonfinite SVG point".into())
        }
    }
    pub fn singular(self) -> bool {
        self.0[0] * self.0[3] == self.0[1] * self.0[2]
    }
    pub fn inverse(self) -> Result<Self, String> {
        let [a, b, c, d, e, f] = self.0;
        let det = a * d - b * c;
        if !det.is_finite() || det == 0. {
            return Err("noninvertible SVG matrix".into());
        }
        Self::checked([
            d / det,
            -b / det,
            -c / det,
            a / det,
            (c * f - d * e) / det,
            (b * e - a * f) / det,
        ])
    }
    pub fn parse_svg(value: &str) -> Result<Self, String> {
        if value.len() > 4096 || !value.is_ascii() {
            return Err("SVG transform input bound exceeded".into());
        }
        let mut rest = value.trim_matches(space);
        let mut out = Self::IDENTITY;
        let mut count = 0;
        while !rest.is_empty() {
            count += 1;
            if count > 64 {
                return Err("SVG transform count bound exceeded".into());
            }
            let end = rest.find('(').ok_or("SVG transform function required")?;
            let name = rest[..end].trim_end_matches(space);
            let tail = &rest[end + 1..];
            let end = tail.find(')').ok_or("unterminated SVG transform")?;
            let n = numbers(&tail[..end])?;
            let m = match (name, n.as_slice()) {
                ("matrix", [a, b, c, d, e, f]) => Self::checked([*a, *b, *c, *d, *e, *f])?,
                ("translate", [x]) => Self::checked([1., 0., 0., 1., *x, 0.])?,
                ("translate", [x, y]) => Self::checked([1., 0., 0., 1., *x, *y])?,
                ("scale", [x]) => Self::checked([*x, 0., 0., *x, 0., 0.])?,
                ("scale", [x, y]) => Self::checked([*x, 0., 0., *y, 0., 0.])?,
                ("rotate", [angle]) | ("rotate", [angle, _, _]) => {
                    let (sin, cos) = angle.rem_euclid(360.).to_radians().sin_cos();
                    let (x, y) = if n.len() == 3 { (n[1], n[2]) } else { (0., 0.) };
                    Self::checked([
                        cos,
                        sin,
                        -sin,
                        cos,
                        x - cos * x + sin * y,
                        y - sin * x - cos * y,
                    ])?
                }
                ("skewX" | "skewY", [angle]) => {
                    let angle = angle.rem_euclid(180.);
                    if angle == 90. {
                        return Err("singular SVG skew".into());
                    }
                    let t = angle.to_radians().tan();
                    if name == "skewX" {
                        Self::checked([1., 0., t, 1., 0., 0.])?
                    } else {
                        Self::checked([1., t, 0., 1., 0., 0.])?
                    }
                }
                _ => return Err("unsupported SVG transform or arity".into()),
            };
            out = out.then_local(m)?;
            rest = tail[end + 1..].trim_start_matches(space);
            if let Some(next) = rest.strip_prefix(',') {
                rest = next.trim_start_matches(space);
                if rest.is_empty() {
                    return Err("trailing SVG transform comma".into());
                }
            }
        }
        Ok(out)
    }
}
fn space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}
fn numbers(value: &str) -> Result<Vec<f64>, String> {
    let mut rest = value.trim_matches(space);
    let mut out = Vec::new();
    while !rest.is_empty() {
        if out.len() == 6 {
            return Err("SVG transform arity bound exceeded".into());
        }
        let end = rest
            .find(|c: char| space(c) || c == ',')
            .unwrap_or(rest.len());
        let token = &rest[..end];
        if token.is_empty()
            || !token
                .bytes()
                .all(|c| c.is_ascii_digit() || matches!(c, b'+' | b'-' | b'.' | b'e' | b'E'))
        {
            return Err("invalid SVG transform number".into());
        }
        let v: f64 = token.parse().map_err(|_| "invalid SVG transform number")?;
        if !v.is_finite() {
            return Err("nonfinite SVG transform number".into());
        }
        out.push(v);
        rest = rest[end..].trim_start_matches(space);
        if let Some(next) = rest.strip_prefix(',') {
            rest = next.trim_start_matches(space);
            if rest.is_empty() {
                return Err("trailing SVG transform number comma".into());
            }
        }
    }
    Ok(out)
}

pub struct ViewBoxMapping {
    pub matrix: Matrix,
    pub disabled: bool,
}
impl ViewBoxMapping {
    pub fn resolve(view_box: [f64; 4], viewport: [f64; 4], aspect: &str) -> Result<Self, String> {
        for r in [view_box, viewport] {
            if !r.iter().all(|n| n.is_finite()) || r[2] < 0. || r[3] < 0. {
                return Err("invalid SVG viewport rectangle".into());
            }
        }
        if aspect.len() > 64 {
            return Err("aspect ratio bound exceeded".into());
        }
        let mut words = aspect.split_ascii_whitespace();
        let align = words.next().unwrap_or("xMidYMid");
        let mode = words.next().unwrap_or("meet");
        if words.next().is_some() || !matches!(mode, "meet" | "slice") {
            return Err("unsupported aspect ratio".into());
        }
        let (ax, ay) = match align {
            "none" => (0., 0.),
            "xMinYMin" => (0., 0.),
            "xMidYMin" => (0.5, 0.),
            "xMaxYMin" => (1., 0.),
            "xMinYMid" => (0., 0.5),
            "xMidYMid" => (0.5, 0.5),
            "xMaxYMid" => (1., 0.5),
            "xMinYMax" => (0., 1.),
            "xMidYMax" => (0.5, 1.),
            "xMaxYMax" => (1., 1.),
            _ => return Err("unsupported aspect ratio alignment".into()),
        };
        if view_box[2] == 0. || view_box[3] == 0. || viewport[2] == 0. || viewport[3] == 0. {
            return Ok(Self {
                matrix: Matrix::IDENTITY,
                disabled: true,
            });
        }
        let mut sx = viewport[2] / view_box[2];
        let mut sy = viewport[3] / view_box[3];
        if align != "none" {
            let scale = if mode == "meet" {
                sx.min(sy)
            } else {
                sx.max(sy)
            };
            sx = scale;
            sy = scale;
        }
        let tx = viewport[0] - view_box[0] * sx + (viewport[2] - view_box[2] * sx) * ax;
        let ty = viewport[1] - view_box[1] * sy + (viewport[3] - view_box[3] * sy) * ay;
        Ok(Self {
            matrix: Matrix::checked([sx, 0., 0., sy, tx, ty])?,
            disabled: false,
        })
    }
}
