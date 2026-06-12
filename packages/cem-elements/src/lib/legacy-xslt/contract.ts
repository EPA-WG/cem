/**
 * Legacy `@epa-wg/custom-element` HTML+XSLT compatibility surface.
 *
 * This TypeScript mirror is consumed by the browser adapter and fixture gates. The CEM-owned
 * contract is also recorded in `cem_ml::legacy_custom_element`; keep the two surfaces aligned when
 * moving the converter behind the engine boundary.
 */

export const LEGACY_CUSTOM_ELEMENT_TEMPLATE_LANG = 'custom-element-xslt';

export const LEGACY_XSLT_DIAGNOSTIC_CODES = {
    orphanBranch: 'legacy_xslt.orphan_branch',
    valueOfMissingSelect: 'legacy_xslt.value_of_missing_select',
    ifMissingTest: 'legacy_xslt.if_missing_test',
    whenMissingTest: 'legacy_xslt.when_missing_test',
    forEachMissingSelect: 'legacy_xslt.for_each_missing_select',
    unsupportedConstruct: 'legacy_xslt.unsupported_construct',
    unsupportedFunction: 'legacy_xslt.unsupported_function',
} as const;

export type LegacyXsltDiagnosticCode =
    (typeof LEGACY_XSLT_DIAGNOSTIC_CODES)[keyof typeof LEGACY_XSLT_DIAGNOSTIC_CODES];

export const LEGACY_XSLT_CONTROL_FLOW_ELEMENTS = [
    'value-of',
    'text',
    'if',
    'choose',
    'when',
    'otherwise',
    'for-each',
    'variable',
    'slot',
] as const;

export const LEGACY_XSLT_DECLARATION_ELEMENTS = [
    'attribute',
    'slice',
    'data',
    'option',
    'module-url',
] as const;

export const LEGACY_XSLT_STYLESHEET_COMPAT_ELEMENTS = [
    'stylesheet',
    'template',
    'apply-templates',
    'call-template',
    'with-param',
    'param',
    'sort',
    'copy',
    'copy-of',
    'attribute',
    'element',
    'output',
] as const;

export const LEGACY_XSLT_TIER3_HANDOFF_ELEMENTS = [
    'function',
    'script',
] as const;

export const LEGACY_XPATH_FUNCTION_MAP = {
    contains: 'str:contains',
    'starts-with': 'str:starts_with',
    'ends-with': 'str:ends_with',
    'normalize-space': 'str:normalize_space',
    translate: 'str:translate',
    substring: 'str:substring',
    'substring-before': 'str:substring_before',
    'substring-after': 'str:substring_after',
    'string-length': 'str:length',
    count: 'seq:count',
} as const;

export const LEGACY_XPATH_SPECIAL_FUNCTIONS = ['not', 'concat', 'position'] as const;

export const LEGACY_XPATH_SUPPORTED_FUNCTIONS = [
    ...Object.keys(LEGACY_XPATH_FUNCTION_MAP),
    ...LEGACY_XPATH_SPECIAL_FUNCTIONS,
] as const;
