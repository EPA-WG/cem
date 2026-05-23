//! `cem:stdlib/content-types` Tier B shell.

use super::{StdlibFunction, Tier};

pub const MODULE_URI: &str = "cem:stdlib/content-types";

pub const FUNCTIONS: &[StdlibFunction] = &[
    StdlibFunction::deferred(MODULE_URI, "html", 0, Tier::B),
    StdlibFunction::deferred(MODULE_URI, "xml", 0, Tier::B),
    StdlibFunction::deferred(MODULE_URI, "svg", 0, Tier::B),
    StdlibFunction::deferred(MODULE_URI, "mathml", 0, Tier::B),
    StdlibFunction::deferred(MODULE_URI, "css", 0, Tier::B),
    StdlibFunction::deferred(MODULE_URI, "scss", 0, Tier::B),
    StdlibFunction::deferred(MODULE_URI, "json", 0, Tier::B),
    StdlibFunction::deferred(MODULE_URI, "yaml", 0, Tier::B),
    StdlibFunction::deferred(MODULE_URI, "csv", 0, Tier::B),
    StdlibFunction::deferred(MODULE_URI, "js", 0, Tier::B),
    StdlibFunction::deferred(MODULE_URI, "ts", 0, Tier::B),
    StdlibFunction::deferred(MODULE_URI, "cemml", 0, Tier::B),
    StdlibFunction::deferred(MODULE_URI, "floor", 0, Tier::B),
    StdlibFunction::deferred(MODULE_URI, "default_accepts", 0, Tier::B),
];
