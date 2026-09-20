//! Native source import into the shared CEM AST (Tier B).
use super::{StdlibFunction, Tier};
pub const FUNCTIONS: &[StdlibFunction] = &[
    StdlibFunction::native_range("cem:stdlib/data", "read", 2, 3, Tier::B),
    StdlibFunction::native_range("cem:stdlib/data", "parse", 2, 3, Tier::B),
    StdlibFunction::native("cem:stdlib/data", "node_key", 1, Tier::B),
    StdlibFunction::native("cem:stdlib/data", "line_number", 1, Tier::B),
    StdlibFunction::native("cem:stdlib/data", "base_uri", 1, Tier::B),
    StdlibFunction::native("cem:stdlib/data", "document_uri", 1, Tier::B),
];
