//! Native source import into the shared CEM AST (Tier B).
use super::{StdlibFunction, Tier};
pub const FUNCTIONS: &[StdlibFunction] = &[StdlibFunction::native_range(
    "cem:stdlib/data",
    "read",
    2,
    3,
    Tier::B,
)];
