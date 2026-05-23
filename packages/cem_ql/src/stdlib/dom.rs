//! `cem:stdlib/dom`.

use super::{StdlibFunction, Tier};

pub const MODULE_URI: &str = "cem:stdlib/dom";

pub const FUNCTIONS: &[StdlibFunction] = &[
    StdlibFunction::host_context(MODULE_URI, "children", 1, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "descendants", 1, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "parent", 1, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "attribute", 2, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "resolve_ref", 1, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "tainted", 1, Tier::A),
];
