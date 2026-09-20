//! `cem:stdlib/cemml`.

use super::{StdlibFunction, Tier};

pub const MODULE_URI: &str = "cem:stdlib/cemml";

pub const FUNCTIONS: &[StdlibFunction] = &[
    StdlibFunction::native(MODULE_URI, "parse", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "format", 1, Tier::A),
];

pub const TIER_B_FUNCTIONS: &[StdlibFunction] =
    &[StdlibFunction::native(MODULE_URI, "inspect", 1, Tier::B)];
