//! `cem:stdlib/strings`.

use super::{StdlibFunction, Tier};

pub const MODULE_URI: &str = "cem:stdlib/strings";

pub const FUNCTIONS: &[StdlibFunction] = &[
    StdlibFunction::native(MODULE_URI, "length", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "codepoints", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "lower", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "upper", 1, Tier::A),
    StdlibFunction::native_range(MODULE_URI, "slice", 2, 3, Tier::A),
    StdlibFunction::native_range(MODULE_URI, "concat", 1, 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "contains", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "starts_with", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "ends_with", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "normalize_space", 1, Tier::A),
    StdlibFunction::native(MODULE_URI, "replace", 3, Tier::A),
    // XPath 1.0 string toolkit used by the legacy HTML+XSLT bridge (DOM→CEM-ML conversion).
    StdlibFunction::native(MODULE_URI, "translate", 3, Tier::A),
    StdlibFunction::native_range(MODULE_URI, "substring", 2, 3, Tier::A),
    StdlibFunction::native(MODULE_URI, "substring_before", 2, Tier::A),
    StdlibFunction::native(MODULE_URI, "substring_after", 2, Tier::A),
];
