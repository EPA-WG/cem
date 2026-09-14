//! `cem:stdlib/report`.

use super::{StdlibFunction, Tier};

pub const MODULE_URI: &str = "cem:stdlib/report";

pub const FUNCTIONS: &[StdlibFunction] = &[
    StdlibFunction::host_context_range(MODULE_URI, "emit", 2, 3, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "severity_floor", 1, Tier::A),
];

pub const TIER_B_FUNCTIONS: &[StdlibFunction] = &[StdlibFunction::host_context(
    MODULE_URI,
    "raise",
    2,
    Tier::B,
)];
