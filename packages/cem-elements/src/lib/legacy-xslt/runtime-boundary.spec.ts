import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const RUNTIME_SOURCE = readFileSync(fileURLToPath(new URL('../cem-elements.ts', import.meta.url)), 'utf8');

describe('legacy bridge runtime boundary', () => {
    it('routes custom-element-v0 through the shared legacy-xslt engine path', () => {
        expect(RUNTIME_SOURCE).not.toContain("'legacy-v0'");
        expect(RUNTIME_SOURCE).not.toContain('projectLegacyTemplate');
        expect(RUNTIME_SOURCE).toContain("template.getAttribute('lang') === 'custom-element-v0'");
        expect(RUNTIME_SOURCE).toContain("return 'legacy-xslt'");
        expect(RUNTIME_SOURCE).toContain('convertLegacyTemplate(compiled.legacySource)');
    });
});
