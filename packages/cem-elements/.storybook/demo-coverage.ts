import { storyNameFromExport, toId } from 'storybook/internal/csf';
import inventory from '../docs/demo-storybook-coverage.json';

export interface DemoCoverage {
    page: string;
    source: string;
    storyFile: string;
    title: string;
    exportName: string;
    legends: string[];
}

export const demoCoverage: readonly DemoCoverage[] = inventory;
// Keep the base separate so Vite does not turn a dynamic URL into an asset glob.
const packageUrl = new URL('../index.html', import.meta.url);
export const normalizeLegend = (value: string): string => value.replace(/\s+/gu, ' ').trim();
export const coverageStoryId = (entry: DemoCoverage): string =>
    toId(entry.title, storyNameFromExport(entry.exportName));

export function assertInventory(actual: readonly string[], expected: readonly string[], label: string): void {
    for (const [kind, values] of [['actual', actual], ['contract', expected]] as const) {
        if (values.some(value => value === '') || new Set(values).size !== values.length) {
            throw new Error(`${label}: blank or duplicate ${kind} entries`);
        }
    }
    const missing = actual.filter(value => !expected.includes(value));
    const stale = expected.filter(value => !actual.includes(value));
    if (missing.length || stale.length) {
        throw new Error(`${label}: missing contracts ${JSON.stringify(missing)}; stale contracts ${JSON.stringify(stale)}`);
    }
    if (actual.some((value, index) => value !== expected[index])) {
        throw new Error(`${label}: order differs from the authored source`);
    }
}

/** A structural coverage gate; the owning play function remains responsible for behavior. */
export function assertAuthoredInventory(
    pages: ReadonlyMap<string, readonly string[]>,
    contracts: readonly DemoCoverage[] = demoCoverage,
): void {
    assertInventory([...pages.keys()].sort(), contracts.map(entry => entry.page).sort(), 'demo pages');
    assertInventory(contracts.map(coverageStoryId), contracts.map(coverageStoryId), 'inventory story owners');
    for (const entry of contracts) {
        if (entry.source.split('#')[0] !== entry.page) {
            throw new Error(`${entry.page}: source must load the authored page`);
        }
        assertInventory(pages.get(entry.page) ?? [], entry.legends, entry.page);
    }
}

/** Run after the designated story's assertions, without introducing another readiness wait. */
export function assertSourceLoadedCoverage(id: string, canvas: HTMLElement): void {
    const entry = demoCoverage.find(candidate => coverageStoryId(candidate) === id);
    if (!entry) return;
    const expected = new URL(entry.source, packageUrl).href;
    const declarations = Array.from(canvas.querySelectorAll<HTMLElement>('cem-element[src]'))
        .filter(element => new URL(element.getAttribute('src') ?? '', element.baseURI).href === expected);
    if (declarations.length !== 1) {
        throw new Error(`${entry.page}: expected one source declaration for ${expected}, found ${declarations.length}`);
    }
    const tag = declarations[0].getAttribute('tag');
    const host = tag ? canvas.querySelector(tag) : null;
    if (!host) throw new Error(`${entry.page}: missing produced source-document host`);
    assertInventory(Array.from(host.querySelectorAll('cem-demo-element'), sample =>
        normalizeLegend(sample.getAttribute('legend') ?? '')), entry.legends, `${entry.page}: rendered legends`);
}
