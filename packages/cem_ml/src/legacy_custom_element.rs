//! Legacy `@epa-wg/custom-element` HTML+XSLT compatibility contract.
//!
//! This module deliberately records the CEM-owned compatibility surface before
//! the full legacy compiler is moved out of the browser TypeScript adapter. The
//! current implementation still lives in `cem-elements/src/lib/legacy-xslt`,
//! but these constants define the engine-owned Tier 1/2 subset that browser
//! runtime, CLI, SSR, and fixture gates must converge on.
//!
//! Scope boundary:
//! - Tier 1/2 pull-style constructs lower to canonical CEM-ML + `cem_ql`.
//! - Tier 3 push-template / standalone stylesheet constructs remain an explicit
//!   handoff, not an accidental browser-only feature.

/// Host-template language marker used by the package adapter for untyped
/// legacy declarations.
pub const TEMPLATE_LANG: &str = "custom-element-xslt";

/// Diagnostic code emitted when a legacy XPath function has no CEM-QL mapping.
pub const UNSUPPORTED_FUNCTION_CODE: &str = "legacy_xslt.unsupported_function";

/// Diagnostic code emitted when a Tier 3 XSLT construct is encountered.
pub const UNSUPPORTED_CONSTRUCT_CODE: &str = "legacy_xslt.unsupported_construct";

/// Legacy elements that are lowered as control-flow / expression nodes.
pub const CONTROL_FLOW_ELEMENTS: &[&str] = &[
    "value-of",
    "text",
    "if",
    "choose",
    "when",
    "otherwise",
    "for-each",
    "variable",
    "slot",
];

/// Legacy declaration/resource helper elements preserved as CEM-ML declarations
/// or inert render helpers.
pub const DECLARATION_ELEMENTS: &[&str] = &["attribute", "slice", "data", "option", "module-url"];

/// Tier 3 XSLT constructs that are not part of the material/demo bridge.
pub const TIER3_HANDOFF_ELEMENTS: &[&str] = &[
    "template",
    "apply-templates",
    "call-template",
    "with-param",
    "param",
    "sort",
    "copy",
    "copy-of",
    "element",
    "function",
    "script",
    "stylesheet",
    "output",
];

/// XPath functions the bridge lowers to CEM-QL directly or by special rewrite.
pub const SUPPORTED_XPATH_FUNCTIONS: &[&str] = &[
    "contains",
    "starts-with",
    "ends-with",
    "normalize-space",
    "translate",
    "substring",
    "substring-before",
    "substring-after",
    "string-length",
    "count",
    "not",
    "concat",
    "position",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyElementDisposition {
    /// Lower as legacy control flow or expression output.
    ControlFlow,
    /// Preserve as a CEM declaration/resource helper.
    Declaration,
    /// Treat as ordinary output markup.
    OutputElement,
    /// Explicit Tier 3 handoff/deferred construct.
    Tier3Handoff,
}

/// Classify a local element name after any `xsl:` prefix has been stripped.
pub fn element_disposition(local_name: &str) -> LegacyElementDisposition {
    if CONTROL_FLOW_ELEMENTS.contains(&local_name) {
        LegacyElementDisposition::ControlFlow
    } else if DECLARATION_ELEMENTS.contains(&local_name) {
        LegacyElementDisposition::Declaration
    } else if TIER3_HANDOFF_ELEMENTS.contains(&local_name) {
        LegacyElementDisposition::Tier3Handoff
    } else {
        LegacyElementDisposition::OutputElement
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyFunctionDisposition {
    /// Function lowers to the given CEM-QL function name.
    CemQl(&'static str),
    /// Function is supported by special syntax rewrite rather than direct call.
    Special,
    /// Function is not in the bridge subset.
    Unsupported,
}

/// Classify a legacy XPath function by the CEM-QL lowering contract.
pub fn function_disposition(name: &str) -> LegacyFunctionDisposition {
    match name {
        "contains" => LegacyFunctionDisposition::CemQl("str:contains"),
        "starts-with" => LegacyFunctionDisposition::CemQl("str:starts_with"),
        "ends-with" => LegacyFunctionDisposition::CemQl("str:ends_with"),
        "normalize-space" => LegacyFunctionDisposition::CemQl("str:normalize_space"),
        "translate" => LegacyFunctionDisposition::CemQl("str:translate"),
        "substring" => LegacyFunctionDisposition::CemQl("str:substring"),
        "substring-before" => LegacyFunctionDisposition::CemQl("str:substring_before"),
        "substring-after" => LegacyFunctionDisposition::CemQl("str:substring_after"),
        "string-length" => LegacyFunctionDisposition::CemQl("str:length"),
        "count" => LegacyFunctionDisposition::CemQl("seq:count"),
        "not" | "concat" | "position" => LegacyFunctionDisposition::Special,
        _ => LegacyFunctionDisposition::Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_material_bridge_elements() {
        assert_eq!(
            element_disposition("if"),
            LegacyElementDisposition::ControlFlow
        );
        assert_eq!(
            element_disposition("choose"),
            LegacyElementDisposition::ControlFlow
        );
        assert_eq!(
            element_disposition("attribute"),
            LegacyElementDisposition::Declaration
        );
        assert_eq!(
            element_disposition("module-url"),
            LegacyElementDisposition::Declaration
        );
        assert_eq!(
            element_disposition("span"),
            LegacyElementDisposition::OutputElement
        );
    }

    #[test]
    fn tier3_push_model_constructs_are_handoff_only() {
        for name in [
            "template",
            "apply-templates",
            "call-template",
            "sort",
            "stylesheet",
        ] {
            assert_eq!(
                element_disposition(name),
                LegacyElementDisposition::Tier3Handoff
            );
        }
    }

    #[test]
    fn maps_supported_xpath_functions_to_cem_ql_contract() {
        assert_eq!(
            function_disposition("contains"),
            LegacyFunctionDisposition::CemQl("str:contains")
        );
        assert_eq!(
            function_disposition("string-length"),
            LegacyFunctionDisposition::CemQl("str:length")
        );
        assert_eq!(
            function_disposition("count"),
            LegacyFunctionDisposition::CemQl("seq:count")
        );
        assert_eq!(
            function_disposition("concat"),
            LegacyFunctionDisposition::Special
        );
        assert_eq!(
            function_disposition("position"),
            LegacyFunctionDisposition::Special
        );
    }

    #[test]
    fn keeps_legacy_dce_helpers_out_of_the_supported_subset() {
        assert_eq!(
            function_disposition("hasBoolAttribute"),
            LegacyFunctionDisposition::Unsupported
        );
    }
}
