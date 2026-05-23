//! `cem:stdlib/datetime`.

use super::{StdlibFunction, Tier};

pub const MODULE_URI: &str = "cem:stdlib/datetime";

pub const FUNCTIONS: &[StdlibFunction] = &[
    StdlibFunction::native(MODULE_URI, "to_utc", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "components", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "format", 2, Tier::A),
];
