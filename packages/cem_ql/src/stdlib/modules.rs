use super::{StdlibFunction, Tier};

pub const MODULE: &str = "cem:stdlib/modules";

pub const FUNCTIONS: &[StdlibFunction] = &[StdlibFunction::host_context_range(
    MODULE,
    "module_url",
    1,
    2,
    Tier::A,
)];
