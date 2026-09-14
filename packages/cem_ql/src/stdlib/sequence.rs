//! `cem:stdlib/sequence`.

use super::{StdlibFunction, Tier};

pub const MODULE_URI: &str = "cem:stdlib/sequence";

pub const FUNCTIONS: &[StdlibFunction] = &[
    StdlibFunction::macro_form(MODULE_URI, "map", 2, Tier::A),
    StdlibFunction::macro_form(MODULE_URI, "where", 2, Tier::A),
    StdlibFunction::macro_form(MODULE_URI, "flat_map", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "take", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "drop", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "first", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "last", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "nth", 2, Tier::A),
    StdlibFunction::macro_form(MODULE_URI, "peek", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "union", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "intersect", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "difference", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "symmetric_difference", 2, Tier::A),
    // XPath `count()` parity for the legacy HTML+XSLT bridge — number of items in a sequence.
    StdlibFunction::native(MODULE_URI, "count", 1, Tier::A),
    StdlibFunction::macro_form(MODULE_URI, "any", 2, Tier::A),
    StdlibFunction::macro_form(MODULE_URI, "all", 2, Tier::A),
];

/// Materializing collection primitives; independent of any presentation or data format.
pub const TIER_B_FUNCTIONS: &[StdlibFunction] = &[
    StdlibFunction::native(MODULE_URI, "group_by", 2, Tier::B),
    StdlibFunction::native_range(MODULE_URI, "sorted", 2, 4, Tier::B),
];
