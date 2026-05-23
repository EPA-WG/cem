//! `cem:stdlib/sequence`.

use super::{StdlibFunction, Tier};

pub const MODULE_URI: &str = "cem:stdlib/sequence";

pub const FUNCTIONS: &[StdlibFunction] = &[
    StdlibFunction::macro_form(MODULE_URI, "map", 2, Tier::A),
    StdlibFunction::macro_form(MODULE_URI, "where", 2, Tier::A),
    StdlibFunction::macro_form(MODULE_URI, "flat_map", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "take", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "drop", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "first", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "last", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "nth", 2, Tier::A),
    StdlibFunction::macro_form(MODULE_URI, "peek", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "union", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "intersect", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "difference", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "symmetric_difference", 2, Tier::A),
];
