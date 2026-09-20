import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

interface CaseMapping {
    legacy: string;
    status: 'covered' | 'migrated' | 'missing';
    current: { file: string; legend: string }[];
    note: string;
    todo?: string;
}

interface LegacySource {
    file: string;
    sha256: string;
    kind: string;
    cases: CaseMapping[];
    unwrapped: CaseMapping[];
    commentedLegends: string[];
    currentFiles?: string[];
    note?: string;
}

const read = (path: string) => readFileSync(new URL(path, import.meta.url), 'utf8');
const manifest = JSON.parse(read('../../docs/legacy-demo-cases.json')) as {
    formatVersion: number;
    sources: LegacySource[];
    evidence: { gallery: string; native: string[] };
    standaloneViewerReview?: {
        document: string;
        status: string;
        sources: string[];
        sorting: string;
        selection: string;
        completedFixtures: string[];
        openFixtures: string[];
        tableDemo: string;
        tableLegends: string[];
    };
};
const gallery = read(`../../../../${manifest.evidence.gallery}`);
const executableLegends = new Set(Array.from(
    gallery.matchAll(/sampleContract\('([^']+)'/gu), (match) => match[1],
));
const allCases = manifest.sources.flatMap((source) => [...source.cases, ...source.unwrapped]);

describe('local legacy demo case map', () => {
    it('pins the complete audited local file and case inventory without a home-directory dependency', () => {
        expect(manifest.formatVersion).toBe(1);
        expect(manifest.sources).toHaveLength(32);
        expect(new Set(manifest.sources.map((source) => source.file)).size).toBe(32);
        expect(manifest.sources.flatMap((source) => source.cases)).toHaveLength(80);
        expect(manifest.sources.flatMap((source) => source.unwrapped)).toHaveLength(5);
        for (const source of manifest.sources) {
            expect(source.sha256, source.file).toMatch(/^[a-f0-9]{64}$/u);
            expect(source.kind, source.file).toBeTruthy();
            if (source.cases.length + source.unwrapped.length === 0) {
                expect(source.note, source.file).toBeTruthy();
            }
            for (const file of source.currentFiles ?? []) {
                expect(read(`../../demo/${file}`), file).not.toBe('');
            }
        }
    });

    it('resolves every covered mapping to an authored and executable current case', () => {
        const documents = new Map<string, string[]>();
        for (const item of allCases) {
            expect(['covered', 'migrated', 'missing']).toContain(item.status);
            expect(item.note, item.legacy).toBeTruthy();
            if (item.status === 'missing') continue;
            expect(item.current.length, item.legacy).toBeGreaterThan(0);
            for (const target of item.current) {
                if (!documents.has(target.file)) {
                    documents.set(target.file, Array.from(
                        read(`../../demo/${target.file}`).matchAll(/<cem-demo-element\b[\s\S]*?legend="([^"]+)"/gu),
                        (match) => match[1].replace(/\s+/gu, ' ').trim(),
                    ));
                }
                expect(documents.get(target.file), item.legacy).toContain(target.legend);
                expect(executableLegends.has(target.legend), `${target.file}: ${target.legend}`).toBe(true);
            }
        }
    });

    it('keeps missing active variants attached to open execution items', () => {
        const todo = read('../../../../docs/todo.md');
        const openItems = todo.split(/(?=^- \[[ x]\] |^## )/mu)
            .filter((item) => item.startsWith('- [ ] '))
            .map((item) => item.replace(/\s+/gu, ' '));
        for (const item of allCases.filter((item) => item.status === 'missing')) {
            expect(item.current, item.legacy).toEqual([]);
            expect(item.todo, item.legacy).toBeTruthy();
            expect(openItems.some((entry) => entry.includes(item.todo ?? '')), item.legacy).toBe(true);
        }
    });

    it('does not count commented prototypes or unported XML viewers as covered cases', () => {
        expect(manifest.sources.flatMap((source) => source.commentedLegends)).toHaveLength(4);
        const viewers = manifest.sources.filter((source) => source.kind === 'unported-xml-viewer');
        expect(viewers.map((source) => source.file)).toEqual(['tree.xml']);
        for (const source of viewers) {
            expect(source.currentFiles).toEqual([]);
            expect(source.cases).toEqual([]);
            expect(source.unwrapped).toEqual([]);
        }
    });

    it('retains the native loader fixture evidence', () => {
        for (const file of manifest.evidence.native) {
            expect(read(`../../../../${file}`), file).toContain('#[test]');
        }
    });

    it('records the standalone review without claiming executable legacy sorting', () => {
        const review = manifest.standaloneViewerReview;
        expect(review).toBeDefined();
        expect(review?.status).toBe('table-view-implemented-tree-open');
        expect(review?.sources).toEqual(['tree.xml', 'tree.xsl', 'table.xml', 'table.xsl']);
        expect(review?.sorting).toBe('scaffold-only');
        expect(review?.selection).toBe('independent-branch-checkboxes');
        for (const file of review?.sources ?? []) {
            expect(manifest.sources.some((source) => source.file === file), file).toBe(true);
        }
    });

    it('maps the partial native table migration without declaring full XML viewer parity', () => {
        expect(manifest.sources.filter((source) => source.kind === 'partially-migrated-xml-viewer')
            .map((source) => source.file)).toEqual(['table.xml', 'table.xsl']);
        const review = manifest.standaloneViewerReview;
        expect(review?.tableDemo).toBe('data-table.html');
        const document = read(`../../demo/${review?.tableDemo}`);
        expect(review?.tableLegends).toHaveLength(5);
        for (const legend of review?.tableLegends ?? []) {
            expect(document).toContain(`legend="${legend}"`);
            expect(executableLegends.has(legend)).toBe(true);
        }
    });

    it('links every pending viewer fixture to the review and the execution checklist', () => {
        const review = manifest.standaloneViewerReview;
        expect(review).toBeDefined();
        if (!review) return;
        const document = read(`../../../../${review.document}`);
        const todo = read('../../../../docs/todo.md');
        expect(review.completedFixtures).toEqual(['XML-VIEW-1']);
        for (const id of review.completedFixtures) {
            expect(document, id).toContain(id);
            expect(todo, id).toContain(`- [x] Fixture ${id}:`);
        }
        expect(review.openFixtures).toEqual(['XML-VIEW-2', 'XML-VIEW-3', 'XML-VIEW-4']);
        for (const id of review.openFixtures) {
            expect(document, id).toContain(id);
            expect(todo, id).toContain(`- [ ] Fixture ${id}:`);
        }
    });
});
