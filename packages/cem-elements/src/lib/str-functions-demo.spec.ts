import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const DEMO_SOURCE = readFileSync(
    fileURLToPath(new URL('../../demo/functions/str.html', import.meta.url)),
    'utf8'
);

describe('CEM-QL string-functions demo source contract', () => {
    it('owns the moved str:shorten query/result matrix', () => {
        const normalizedSource = DEMO_SOURCE.replace(/\s+/gu, ' ');
        expect(normalizedSource).toContain('legend="str:shorten query/result matrix"');
        expect(normalizedSource).toContain('<cem-element tag="cem-str-shorten-matrix">');
        expect(normalizedSource).toContain('{code | str:shorten("short", 8)}');
        expect(normalizedSource).toContain('{$str:shorten("short", 8)}');
        expect(normalizedSource).toContain('{$str:shorten("abcdefghij", 7)}');
        expect(normalizedSource).toContain('{$str:shorten("abcdefghij", 8)}');
        expect(normalizedSource).toContain('{$str:shorten("abcdefghij", 8, "...")}');
        expect(normalizedSource).toContain('{$str:shorten("abcdefghij", 6, "")}');
        expect(normalizedSource).toContain('{$str:shorten("αβ😀δεζη", 5, "💠")}');
        expect(normalizedSource).toContain(
            '{$str:shorten( "https://example.test/lib/semantic-card.cem" , 32)}'
        );
        expect(normalizedSource).toContain('<cem-str-shorten-matrix></cem-str-shorten-matrix>');
    });

    it('loads nested demo assets from the correct locations', () => {
        expect(DEMO_SOURCE).toContain('href="../demo.css"');
        expect(DEMO_SOURCE).toContain("from '../../dist/index.js'");
        expect(DEMO_SOURCE).toContain('href="../../index.html"');
    });
});
