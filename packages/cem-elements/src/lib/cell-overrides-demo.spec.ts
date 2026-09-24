import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const read = (path: string): string => readFileSync(new URL(path, import.meta.url), 'utf8');
const page = read('../../demo/cell-overrides.html');

describe('imported cell presentation lessons', () => {
    it('inlines the primitive name override and isolates the conditional fallback lesson', () => {
        expect(Array.from(page.matchAll(/legend="([^"]+)"/gu), match => match[1])).toEqual([
            '1. Name cells become Pokémon pictures', 'pokemon-cells.json',
            '2. Zero-stock cells get a warning', 'stock-cells.xml', 'stock-cell.cemt',
            '3. Native values pass into another component',
        ]);
        expect(page.match(/<template>\n<cem-element/gu)).toHaveLength(3);
        const inline = page.split('<template type="text/cem-ml">')[1].split('</template>')[0];
        expect(inline).not.toMatch(/\{(?:module|body)\b/u);
        expect(inline).toContain('{template @mode=cell');
        expect(inline.match(/@match='([^']+)'/u)?.[1]).toBe('dom::chain(node).parent().attribute("name").text() == "name"');
        expect(inline).toContain('dom::chain(node).parent().parent().children()');
        expect(inline).toContain('.find(|field| field.attribute("name").text() == "url")');
        expect(inline).toContain('{$node}');
        expect(inline).toContain('.text().split("/").nth(6)');
        expect(inline).toContain('@mode=cell');
        expect(page).toContain('href="./stock-cell.cemt"');
        expect(page).toContain('src="./stock-cell.cemt"');
        expect(page).toContain('cemt:apply_templates(label, "label")');
        for (const template of [inline, read('../../demo/stock-cell.cemt')]) {
            expect(template).toContain('{import @as=base @src="./data-table-view.cemt"}');
            expect(template).toMatch(/\{call @from=base @template=(?:viewer|inspect)\b/u);
            expect(template).toContain('@mode=');
            expect(template).not.toContain('{table');
            expect(template).not.toContain('{cem-data');
        }
    });

    it('loads the Pokémon file and previews its source in a separate card', () => {
        const sample = page.split('legend="1. Name cells become Pokémon pictures"')[1].split('</cem-demo-element>')[0];
        expect(sample).toContain('@url="./pokemon-cells.json"');
        expect(sample).toContain("@with:root='{datadom.slices.catalog.data}'");
        expect(sample).toContain('<cem-pokemon-cells></cem-pokemon-cells>');
        expect(sample).not.toContain('&lt;catalog&gt;');
        for (const file of ['pokemon-cells.json', 'stock-cells.xml', 'stock-cell.cemt']) {
            expect(page).toContain(`<cem-demo-element id="${file}"`);
            expect(page).toContain(`src="./${file}"`);
        }
        expect(page).not.toContain('cem-module-url');
        const data = JSON.parse(read('../../demo/pokemon-cells.json'));
        expect(data.count).toBe(1351);
        expect(data.previous).toBeNull();
        expect(data.results).toHaveLength(10);
        expect(data.results[0]).toEqual({ name: 'bulbasaur', url: 'https://pokeapi.co/api/v2/pokemon/1/' });
        expect(page).toContain('PokéAPI (pokeapi.co)');
        expect(read('../../demo/stock-cells.xml').match(/<product>/gu)).toHaveLength(5);
        expect(read('../../demo/stock-cell.cemt')).toContain('@url="./stock-cells.xml"');
        expect(page).toContain('<cem-stock-cells></cem-stock-cells>');
    });

    it('documents retained source matching, fallback and the existing cell boundary', () => {
        for (const contract of ['parent()', 'Missing values', 'image plus the original name', 'retained CEM tree']) {
            expect(page).toContain(contract);
        }
        for (const forbidden of ['JSON.parse', 'DOMParser', 'XSLTProcessor', 'onclick=', 'data-testid']) {
            expect(page).not.toContain(forbidden);
        }
        expect(page).toContain('@name=imgUrlRoot');
        expect(page).toContain('https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/');
        expect(page).toContain('@src="{$imgUrlRoot}{$id}.svg"');
        expect(page).toContain('Pokémon Company');
    });

    it('links the focused lesson from the index and related examples', () => {
        expect(read('../../index.html')).toContain('cell-overrides.html');
        for (const file of ['data-table.html', 'http-request.html']) {
            expect(page).toContain(`href="./${file}"`);
            expect(read(`../../demo/${file}`)).toContain('cell-overrides.html');
        }
    });
});
