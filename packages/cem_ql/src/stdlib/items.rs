//! `cem:stdlib/items`.

use super::{StdlibFunction, Tier};

pub const MODULE_URI: &str = "cem:stdlib/items";

pub const FUNCTIONS: &[StdlibFunction] = &[StdlibFunction::native(MODULE_URI, "kind", 1, Tier::A)];
