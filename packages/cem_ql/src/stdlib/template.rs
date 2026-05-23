//! `cem:stdlib/template`.

use super::{StdlibFunction, Tier};

pub const MODULE_URI: &str = "cem:stdlib/template";

pub const FUNCTIONS: &[StdlibFunction] = &[
    StdlibFunction::host_context(MODULE_URI, "lookup", 1, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "names", 0, Tier::A),
];
