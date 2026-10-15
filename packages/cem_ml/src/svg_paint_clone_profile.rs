//! Pinned expectations for six isolated SVG-image raster probes.
//! A matching report is compatibility data, never native execution evidence.
//! It cannot replace source/host provenance or authorize arbitrary CSS evaluation.
pub const CHROMIUM_CLONE_PROFILE_VERSION: &str = "148.0.7778.96";

/// Fixed red-source / blue-host, opaque currentColor fixture cases. See
/// docs/cem-studio.md#explicit-clone-style-browser-compatibility-profile for the
/// exact SVG shape, raster conditions and intentionally unsupported dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CloneProbe {
    OwnGradient,
    InheritedGradient,
    DescriptiveGradient,
    OwnPattern,
    InheritedPattern,
    DescriptivePattern,
}
impl CloneProbe {
    pub const ALL: [Self; 6] = [
        Self::OwnGradient,
        Self::InheritedGradient,
        Self::DescriptiveGradient,
        Self::OwnPattern,
        Self::InheritedPattern,
        Self::DescriptivePattern,
    ];
}
#[derive(Debug, Clone)]
pub struct CloneProbeObservation {
    pub case: CloneProbe,
    /// sRGB RGBA values, each in 0..=255.
    pub source_color: [u8; 4],
    pub host_color: [u8; 4],
    pub pixel: [u8; 4],
}
/// No Default or caller-defined rules. An exact supported identity is mandatory.
/// This profile is separate from SvgPaintStylePlan's SVG 2 structural model.
#[derive(Debug, Clone, Copy)]
pub struct SvgPaintCloneProfile {
    _private: (),
}
impl SvgPaintCloneProfile {
    pub fn for_browser(engine: &str, version: &str) -> Result<Self, String> {
        if engine != "chromium" || version != CHROMIUM_CLONE_PROFILE_VERSION {
            return Err("unsupported browser identity for the pinned SVG clone profile".into());
        }
        Ok(Self { _private: () })
    }
    pub fn browser(&self) -> (&'static str, &'static str) {
        ("chromium", CHROMIUM_CLONE_PROFILE_VERSION)
    }
    /// Fixture expectations only. This is not a rule for evaluating arbitrary
    /// currentColor, custom properties, selectors or inherited declarations.
    pub fn expected_pixel(&self, case: CloneProbe) -> [u8; 4] {
        match case {
            CloneProbe::OwnGradient | CloneProbe::OwnPattern => [0, 0, 255, 255],
            CloneProbe::InheritedGradient
            | CloneProbe::DescriptiveGradient
            | CloneProbe::InheritedPattern => [255, 0, 0, 255],
            CloneProbe::DescriptivePattern => [0, 0, 0, 0],
        }
    }
    /// Checks a complete, bounded report without retaining it or creating a seal.
    /// Callers remain responsible for running the specified fixtures; accepting
    /// these public observations cannot authenticate a browser or a document.
    pub fn verify_probes(&self, observations: &[CloneProbeObservation]) -> Result<(), String> {
        if observations.len() != CloneProbe::ALL.len() {
            return Err("SVG clone profile requires exactly six probe observations".into());
        }
        let mut seen = [false; 6];
        for observation in observations {
            let index = CloneProbe::ALL
                .iter()
                .position(|case| *case == observation.case)
                .ok_or("unsupported SVG clone probe")?;
            if seen[index] {
                return Err("duplicate SVG clone probe observation".into());
            }
            seen[index] = true;
            if observation.source_color != [255, 0, 0, 255]
                || observation.host_color != [0, 0, 255, 255]
                || observation.pixel != self.expected_pixel(observation.case)
            {
                return Err(format!(
                    "SVG clone profile mismatch for {:?}",
                    observation.case
                ));
            }
        }
        Ok(())
    }
    pub fn blockers(&self) -> &'static [&'static str] {
        &[
            "svg-paint-clone-style-evaluation-required",
            "svg-paint-clone-execution-evidence-required",
            "svg-paint-rendering-evidence-required",
            "svg-computed-winning-source-required",
            "svg-computed-resource-acquisition-required",
        ]
    }
}
