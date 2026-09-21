import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const read = (path: string): string => readFileSync(new URL(path, import.meta.url), 'utf8');
const page = read('../../demo/cell-overrides.html');

describe('imported cell presentation lessons', () => {
    it('inlines the primitive name override and isolates the conditional fallback lesson', () => {
        expect(Array.from(page.matchAll(/legend="([^"]+)"/gu), match => match[1])).toEqual([
            '1. Name cells become Pokémon pictures', '2. Zero-stock cells get a warning', '3. Native values pass into another component',
        ]);
        expect(page.match(/<template>\n<cem-element/gu)).toHaveLength(3);
        const inline = page.split('<template type="text/cem-ml">')[1].split('</template>')[0];
        expect(inline.match(/@match='([^']+)'/u)?.[1]).toBe('node.name == "name"');
        expect(inline).toContain('dom:children(dom:parent(node))');
        expect(inline).toContain('{$node}');
        expect(inline).toContain('@mode=cell');
        expect(page).toContain('href="./stock-cell.cemt"');
        expect(page).toContain('src="./stock-cell.cemt"');
        for (const template of [inline, read('../../demo/stock-cell.cemt')]) {
            expect(template).toContain('{import @as=base @src="./data-table-view.cemt"}');
            expect(template).toContain('{call @from=base @template=viewer}');
            expect(template).toContain('@mode=');
            expect(template).not.toContain('{table');
            expect(template).not.toContain('{cem-data');
        }
    });

    it('documents retained source matching, fallback and the existing cell boundary', () => {
        for (const contract of ['data:node_key', 'Missing values', 'image plus the original name', 'retained CEM tree']) {
            expect(page).toContain(contract);
        }
        for (const forbidden of ['JSON.parse', 'DOMParser', 'XSLTProcessor', 'onclick=', 'data-testid']) {
            expect(page).not.toContain(forbidden);
        }
        for (const id of [2, 3]) expect(read(`../../demo/pokemon/${id}.svg`)).toContain('<svg');
        expect(read('../../demo/pokemon/README.md')).toContain('pokeapi-sprites@2.0.2');
        expect(read('../../demo/pokemon/LICENCE.txt')).toContain('Copyright The Pokémon Company');
    });

    it('links the focused lesson from the index and related examples', () => {
        expect(read('../../index.html')).toContain('cell-overrides.html');
        for (const file of ['data-table.html', 'http-request.html']) {
            expect(page).toContain(`href="./${file}"`);
            expect(read(`../../demo/${file}`)).toContain('cell-overrides.html');
        }
    });
});
