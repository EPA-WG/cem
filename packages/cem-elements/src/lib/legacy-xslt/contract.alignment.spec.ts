import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import {
    LEGACY_CUSTOM_ELEMENT_TEMPLATE_LANG,
    LEGACY_XSLT_CONTROL_FLOW_ELEMENTS,
    LEGACY_XSLT_DECLARATION_ELEMENTS,
    LEGACY_XSLT_DIAGNOSTIC_CODES,
    LEGACY_XSLT_TIER3_HANDOFF_ELEMENTS,
    LEGACY_XPATH_SUPPORTED_FUNCTIONS,
} from './contract.js';

// The CEM-owned legacy-compatibility contract lives in `cem_ml::legacy_custom_element` (Rust); this
// TS mirror (`contract.ts`) is consumed by the browser adapter, runtime, and fixture gates. The two
// surfaces must not drift as the engine path catches up — this guard reads the Rust source (the
// authoritative side) and asserts the TS constants match. If you change one, change both.
const RUST_SOURCE = readFileSync(
    fileURLToPath(new URL('../../../../cem_ml/src/legacy_custom_element.rs', import.meta.url)),
    'utf8'
);

function rustStringConst(name: string): string {
    const match = RUST_SOURCE.match(new RegExp(`pub const ${name}: &str = "([^"]*)"`));
    if (!match) {
        throw new Error(`Rust const ${name}: &str not found`);
    }
    return match[1];
}

function rustStringArray(name: string): string[] {
    const match = RUST_SOURCE.match(new RegExp(`pub const ${name}: &\\[&str\\] = &\\[([^\\]]*)\\]`, 's'));
    if (!match) {
        throw new Error(`Rust const ${name}: &[&str] not found`);
    }
    return Array.from(match[1].matchAll(/"([^"]*)"/g), (m) => m[1]);
}

const sorted = (values: readonly string[]): string[] => [...values].sort();

describe('legacy-xslt contract alignment (TS mirror ⇄ cem_ml authoritative)', () => {
    it('template language marker matches', () => {
        expect(LEGACY_CUSTOM_ELEMENT_TEMPLATE_LANG).toBe(rustStringConst('TEMPLATE_LANG'));
    });

    it('control-flow element set matches', () => {
        expect(sorted(LEGACY_XSLT_CONTROL_FLOW_ELEMENTS)).toEqual(sorted(rustStringArray('CONTROL_FLOW_ELEMENTS')));
    });

    it('declaration element set matches', () => {
        expect(sorted(LEGACY_XSLT_DECLARATION_ELEMENTS)).toEqual(sorted(rustStringArray('DECLARATION_ELEMENTS')));
    });

    it('Tier 3 handoff element set matches', () => {
        expect(sorted(LEGACY_XSLT_TIER3_HANDOFF_ELEMENTS)).toEqual(sorted(rustStringArray('TIER3_HANDOFF_ELEMENTS')));
    });

    it('supported XPath function set matches', () => {
        expect(sorted(LEGACY_XPATH_SUPPORTED_FUNCTIONS)).toEqual(sorted(rustStringArray('SUPPORTED_XPATH_FUNCTIONS')));
    });

    it('diagnostic codes match', () => {
        expect(LEGACY_XSLT_DIAGNOSTIC_CODES.unsupportedFunction).toBe(rustStringConst('UNSUPPORTED_FUNCTION_CODE'));
        expect(LEGACY_XSLT_DIAGNOSTIC_CODES.unsupportedConstruct).toBe(rustStringConst('UNSUPPORTED_CONSTRUCT_CODE'));
    });
});
