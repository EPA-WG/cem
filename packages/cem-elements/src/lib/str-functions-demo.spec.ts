import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const DEMO_SOURCE = readFileSync(
    fileURLToPath(new URL('../../demo/functions/str.html', import.meta.url)),
    'utf8'
);
const METHODS = ['split', 'trim', 'trim_start', 'trim_end', 'char_at', 'at', 'index_of', 'last_index_of'] as const;

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
        expect(DEMO_SOURCE).toContain('href="../dom-merge.html"');
        expect(DEMO_SOURCE).toContain('literal separator');
        expect(DEMO_SOURCE).toContain('codepoints');
    });

    it('lists every independently authored string-function case', () => {
        expect(Array.from(DEMO_SOURCE.matchAll(/legend="([^"]+)"/gu), (match) => match[1])).toEqual([
            'str:shorten query/result matrix', 'URL ID with a string chain', ...METHODS.map((method) => `str:${method}`),
            'XPath normalize-space', 'XPath tokenize and string-join',
        ]);
    });

    it.each(['XPath normalize-space', 'XPath tokenize and string-join'])('%s loads its text library relative to the nested demo', (legend) => {
        const sample = DEMO_SOURCE.split(`<cem-demo-element legend="${legend}"`)[1]?.split('</cem-demo-element>')[0] ?? '';
        expect(sample).toMatch(/<template>\n<cem-element>/u);
        expect(sample).toContain('xpath-functions="../xpath-text.cemt"');
        expect(sample).toContain('@slice-event=input');
        expect(sample).not.toContain('tag=');
    });

    it('teaches Rust-style string splitting and zero-based selection', () => {
        expect(DEMO_SOURCE).toContain('url.split("/").nth(6)');
        expect(DEMO_SOURCE).toContain('{$id ?? "No ID"}');
    });

    it.each(METHODS)('str:%s has an anonymous, flush-left, interactive example', (method) => {
        const sample = DEMO_SOURCE.split(`<cem-demo-element legend="str:${method}"`)[1]?.split('</cem-demo-element>')[0] ?? '';
        expect(sample).toMatch(/<template>\n<cem-element>/u);
        expect(sample).toContain('description=');
        expect(sample).toContain('@slice-event=input');
        expect(sample).toContain(`str:${method}(`);
        expect(sample).not.toContain('tag=');
        expect(sample).not.toContain('onclick=');
    });
});
