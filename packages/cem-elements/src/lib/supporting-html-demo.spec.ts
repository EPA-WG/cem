import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

function source(file: string): string {
    return readFileSync(new URL(`../../demo/${file}`, import.meta.url), 'utf8');
}

describe('supporting HTML source contracts', () => {
    it('keeps the nested anonymous declaration in the embedded document', () => {
        const html = source('embed-1.html');
        expect(html).toContain('<h4>embed-1.html</h4>');
        expect(html).toMatch(/<cem-element>\s*<template type="text\/cem-ml">\s*🖖\s*<\/template>\s*<\/cem-element>/u);
        expect(html).not.toContain('<script');
    });

    for (const directory of ['', 'lib-dir/']) {
        it(`keeps all three ${directory}embed-lib fragments and their own relative bases`, () => {
            const html = source(`${directory}embed-lib.html`);
            expect(Array.from(html.matchAll(/<template id="([^"]+)" type="text\/cem-ml">/gu), match => match[1]))
                .toEqual(['embed-lib-component', 'embed-relative-hash', 'embed-relative-file']);
            expect(html).toContain('👋 from embed-lib-component');
            const library = directory ? './embed-lib.html' : './lib-dir/embed-lib.html';
            const smiley = directory ? './Smiley.svg' : './lib-dir/Smiley.svg';
            const document = directory ? '../embed-1.html' : './embed-1.html';
            expect(html).toContain(`@src="${library}#embed-lib-component"`);
            expect(html).toContain(`@slice=full-url @src="${library}"`);
            expect(html).toContain(`@slice=smiley-url @src="${smiley}"`);
            expect(html).toContain(`@slice=up-url @src="${document}"`);
            expect(html).toContain(`@tag=dce-embed-lib-file @src="${document}"`);
        });
    }

    it('keeps the external whole-document slot fallback', () => {
        const html = source('external-template-document.html');
        expect(html).toContain('<article class="demo-card external-document-template">');
        expect(html).toContain('<h2>External document</h2>');
        expect(html).toContain('<p><slot>External document fallback</slot></p>');
        expect(html).not.toContain('<template');
    });

    it('keeps card, ordinary subtree and scoped-style library selections distinct', () => {
        const html = source('external-template-templates.html');
        expect(Array.from(html.matchAll(/<(template|article) id="([^"]+)"/gu), match => [match[1], match[2]]))
            .toEqual([['template', 'external-card-template'], ['article', 'external-subtree-template'],
                ['template', 'scoped-css-external-template']]);
        expect(html).toContain('datadom.attributes.title ?? "External template"');
        expect(html).toContain('{slot | External fallback}');
        expect(html).toContain('<slot>External subtree fallback</slot>');
        expect(html).toContain('{slot | External scoped fallback}');
        expect(html).toContain('@href="./external-template-templates.html#external-card-template"');
        expect(html).toContain('--external-scoped-bg: rgb(254, 243, 199)');
    });

    it('retains the HTML/SVG/MathML fragment boundaries and intentional script exclusion probe', () => {
        const html = source('html-template.html');
        expect(Array.from(html.matchAll(/\bid="([^"]+)"/gu), match => match[1]))
            .toEqual(['wave', 'ok', 'dwc-logo', 'sophomores-dream']);
        expect(html).toContain('<script>console.error(\'Stranger danger!\')</script>');
        expect(html).toContain('xmlns="http://www.w3.org/2000/svg" viewBox="0 0 216 209.18"');
        expect(html).toContain('<math id="sophomores-dream"');
        expect(html).toContain('<msubsup>');
        expect(html).toContain('<munderover>');
    });
});
