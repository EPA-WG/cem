//! `cem:stdlib/dom`.

use super::{StdlibFunction, Tier};

pub const MODULE_URI: &str = "cem:stdlib/dom";

pub const FUNCTIONS: &[StdlibFunction] = &[
    StdlibFunction::host_context(MODULE_URI, "chain", 1, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "text", 0, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "text", 1, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "reference", 1, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "clone", 1, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "element", 1, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "children", 1, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "descendants", 1, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "parent", 1, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "attribute", 2, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "resolve_ref", 1, Tier::A),
    StdlibFunction::host_context(MODULE_URI, "tainted", 1, Tier::A),
];

/// The receiver-method contract shared by lowering/type checking and execution.
pub(crate) fn chain_method_arity(name: &str) -> Option<(usize, usize)> {
    Some(match name {
        "parent" | "children" | "child_nodes" | "ancestors" | "name" | "text" | "first"
        | "last" | "reversed" | "count" | "is_empty" => (0, 0),
        "closest" | "find" | "find_last" | "filter" | "map" | "flat_map" | "any" | "all"
        | "take" | "skip" | "attribute" => (1, 1),
        "sorted" => (0, 2),
        "sorted_by_key" => (1, 3),
        _ => return None,
    })
}
