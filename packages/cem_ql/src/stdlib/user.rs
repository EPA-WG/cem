//! `cem:stdlib/user` Tier B policy-accessor shell.

use super::{StdlibFunction, Tier};

pub const MODULE_URI: &str = "cem:stdlib/user";

pub const FUNCTIONS: &[StdlibFunction] = &[StdlibFunction::host_context(
    MODULE_URI,
    "has_role",
    2,
    Tier::B,
)];
