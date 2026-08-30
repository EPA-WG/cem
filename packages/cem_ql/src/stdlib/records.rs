//! `cem:stdlib/records`.

use super::{StdlibFunction, Tier};

pub const MODULE_URI: &str = "cem:stdlib/records";

pub const FUNCTIONS: &[StdlibFunction] =
    &[StdlibFunction::native(MODULE_URI, "entries", 1, Tier::A)];
