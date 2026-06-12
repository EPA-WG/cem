//! `cem:stdlib/numbers`.

use super::{StdlibFunction, Tier};

pub const MODULE_URI: &str = "cem:stdlib/numbers";

pub const FUNCTIONS: &[StdlibFunction] = &[
    StdlibFunction::native(MODULE_URI, "double", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "decimal", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "integer", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "string", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "abs", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "floor", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "ceil", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "round", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "format", 2, Tier::A),
];
