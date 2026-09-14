//! Generic opt-in host query-function invocation (Tier B).
use super::{StdlibFunction, Tier};
pub const MODULE_URI: &str = "cem:stdlib/native";
pub const FUNCTIONS: &[StdlibFunction] = &[StdlibFunction::host_context_range(
    MODULE_URI,
    "call",
    1,
    u8::MAX,
    Tier::B,
)];
