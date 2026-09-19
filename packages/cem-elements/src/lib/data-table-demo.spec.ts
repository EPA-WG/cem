import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { scopeCssText } from './projection.js';

const source = readFileSync(new URL('../../demo/data-table.html', import.meta.url), 'utf8');
const helper = readFileSync(new URL('../../demo/data-table-view.cemt', import.meta.url), 'utf8');
const aspects = readFileSync(new URL('../../demo/data-table-aspects.cemt', import.meta.url), 'utf8');

describe('multi-format data table demo', () => {
    it('gives each native format an editable teaching case using one reusable helper', () => {
        expect(Array.from(source.matchAll(/legend="([^"]+)"/gu), (match) => match[1])).toEqual([
            '1. XML table: attributes and text',
            '2. CSV table: quoted fields',
            '3. YAML table: nested collections',
            '4. JSON table: empty and missing',
            '5. Presentation aspects: tree and IP-filter form',
            './data-table-view.cemt',
        ]);
        for (const format of ['xml', 'csv', 'yaml', 'json']) {
            expect(source).toContain(`<template>\n<cem-data-table format="${format}">`);
        }
        expect(source).toContain('src="./data-table-view.cemt"');
        expect(helper).toContain('{cem-data @name=document @select=source');
        expect(helper).toContain('seq:group_by(');
        expect(helper).toContain('seq:sorted(records,');
        expect(helper).not.toContain('data:table');
        expect(helper).toContain('@slice=source @slice-event=change');
        expect(helper).toContain('@value="{$record.source.id}"');
        expect(helper).toContain('@aria-pressed="{selected == record.source.id}"');
    });

    it('keeps source text inert and states parsing, sorting and selection limits', () => {
        for (const forbidden of ['DOMParser', 'JSON.parse', 'XSLTProcessor', 'onclick=', 'data-testid', '.sort(']) {
            expect(source + helper + aspects).not.toContain(forbidden);
        }
        for (const lesson of ['32 KiB', 'missing', 'empty string', 'source order', 'never execute', 'Selection follows source']) {
            expect(source).toContain(lesson);
        }
        expect(source).toContain('See also');
        expect(helper).toContain('@role=alert');
        expect(helper).toContain('@aria-label="Reset source"');
        expect(source).toMatch(/Namespace\s+declarations/u);
        expect(source).toContain('are not data columns');
    });

    it('extends the unchanged imported viewer through matching presentation rules', () => {
        expect(aspects).toContain('{import @as=base @src="./data-table-view.cemt"}');
        expect(aspects).toContain('@name=notes-as-tree @mode=inspect @priority=10');
        expect(aspects).toContain('@name=ip-filter-form @mode=inspect @priority=10');
        expect(aspects).toContain('{call @from=base @template=tree');
        expect(aspects).toContain('{call @from=base @template=viewer}');
        expect(aspects).toContain('@aria-label="Presentation aspects"');
        expect(aspects).toContain('@aria-label="IP filter"');
        expect(helper).not.toContain('ip-filter');
    });

    it('links the working table view from the index and related demos', () => {
        for (const path of ['../../index.html', '../../demo/external-template.html', '../../demo/for-each.html']) {
            expect(readFileSync(new URL(path, import.meta.url), 'utf8')).toContain('data-table.html');
        }
    });

    it('keeps table scrolling internal and scoped selectors within the specificity policy', () => {
        const css = helper.match(/\{style \|```([\s\S]*?)```\}/u)?.[1] ?? '';
        expect(css).toContain('overflow: auto');
        expect(css).toContain('flex-wrap: wrap');
        const result = scopeCssText(css, 'table-demo-contract');
        expect(result.diagnostics).toEqual([]);
    });
});
