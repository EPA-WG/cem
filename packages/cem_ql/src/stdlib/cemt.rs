//! Explicit template-host operations; no browser or ambient renderer dependency.
use super::{StdlibFunction, Tier};
pub const MODULE_URI: &str = "cem:stdlib/cemt";
pub const FUNCTIONS: &[StdlibFunction] = &[StdlibFunction::host_context(
    MODULE_URI,
    "apply_templates",
    2,
    Tier::A,
)];
