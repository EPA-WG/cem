//! `cem:stdlib/state`.

use super::{StdlibFunction, Tier};

pub const MODULE_URI: &str = "cem:stdlib/state";

pub const FUNCTIONS: &[StdlibFunction] = &[
    StdlibFunction::host_context(MODULE_URI, "read", 1, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "keys", 0, Tier::A),
];
