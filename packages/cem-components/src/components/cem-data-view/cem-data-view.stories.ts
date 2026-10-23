import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, nextRenderFrame } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-data-view.xhtml?raw';

const meta = preview.meta({
    component: 'cem-data-view', title: 'CEM Components/cem-data-view',
    loaders: [async () => { await loadCemDeclaration('cem-data-view', declarationSource); return {}; }],
});

function attribute(value: string) {
    return value.replaceAll('&', '&amp;').replaceAll('"', '&quot;').replaceAll('<', '&lt;');
}

function sample(value: string, format = 'json', label = 'Data') {
    return `<cem-data-view value="${attribute(value)}" format="${format}" label="${label}"></cem-data-view>`;
}

function required<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector);
    if (!node) throw new Error(`Missing ${selector}`);
    return node;
}

export const TypedJson = meta.story({
    render: () => sample('[{"name":"","value":null},{"name":"Ada"},3,false,[],{}]', 'json', 'Typed values'),
    play: async ({ canvasElement }) => {
        const host = required<HTMLElement>(canvasElement, 'cem-data-view'); await whenCemRendered(host);
        const table = required(host, 'table'); expect(table).toHaveAccessibleName('Typed values');
        expect(table.textContent).toContain('∅ missing'); expect(table.textContent).toContain('null');
        expect(table.textContent).toContain('""'); expect(table.textContent).toContain('false');
        expect(table.textContent).toContain('[]'); expect(table.textContent).toContain('{}');
        expect(table.querySelectorAll('tbody tr')).toHaveLength(6);
    },
});

export const QuotedCsvAndSafeText = meta.story({
    render: () => sample('name,note\nAda,"first, second"\nBob,"line one\nline two"', 'csv', 'Quoted CSV')
        + sample('[{"markup":"<img src=x onerror=alert(1)>"}]', 'json', 'Text only'),
    play: async ({ canvasElement }) => {
        for (const host of canvasElement.querySelectorAll<HTMLElement>('cem-data-view')) await whenCemRendered(host);
        expect(canvasElement.textContent).toContain('first, second');
        expect(canvasElement.textContent).toContain('line one\nline two');
        expect(canvasElement.textContent).toContain('<img src=x onerror=alert(1)>');
        expect(canvasElement.querySelector('img')).toBeNull();
    },
});

export const LimitsAndErrors = meta.story({
    render: () => sample(JSON.stringify(Array.from({ length: 103 }, (_, n) => ({ n }))), 'json', 'Rows')
        + sample(JSON.stringify([Object.fromEntries(Array.from({ length: 35 }, (_, n) => [`c${n}`, n]))]), 'json', 'Columns')
        + sample(JSON.stringify([{ x: 'é'.repeat(2050) }]), 'json', 'Cell')
        + sample('{"a":{"b":{"c":{"d":{"hidden":"tail"}}}}}', 'json', 'Depth')
        + sample('"' + '🟢'.repeat(65535) + '"', 'json', 'Source')
        + sample('[', 'json', 'Malformed') + sample('<r/>', 'xml', 'Unsupported'),
    play: async ({ canvasElement }) => {
        const hosts = [...canvasElement.querySelectorAll<HTMLElement>('cem-data-view')];
        for (const host of hosts) await whenCemRendered(host);
        expect(hosts[0].querySelectorAll('tbody tr')).toHaveLength(100);
        expect(hosts[0].textContent).toContain('Row limit: 100');
        expect(hosts[1].querySelectorAll('th[scope=col]')).toHaveLength(32);
        expect(hosts[1].textContent).toContain('Column limit: 32');
        expect(hosts[2].textContent).toContain('cell limit: 2048');
        expect(hosts[3].textContent).toContain('Depth limit: 4');
        expect(hosts[3].textContent).not.toContain('tail');
        expect(hosts[4].textContent).toContain('Source limit: 65536');
        expect(hosts[4].textContent).not.toContain('🟢');
        for (const host of hosts.slice(5)) expect(host.querySelector('[role=alert]')).not.toBeNull();
    },
});

export const LabelUpdateAndInheritedDensity = meta.story({
    render: () => '<section data-cem-density="compact"><cem-data-view data-cem-density="super-compact" label="Fallback" aria-label="Accessible data" value="[{&quot;a&quot;:1},{&quot;a&quot;:2}]"><strong slot="label">Projected caption</strong></cem-data-view></section>',
    play: async ({ canvasElement }) => {
        const container = required<HTMLElement>(canvasElement, 'section');
        const host = required<HTMLElement>(container, 'cem-data-view'); await whenCemRendered(host);
        expect(required(host, 'table')).toHaveAccessibleName('Accessible data');
        expect(host.textContent).toContain('Projected caption');
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            container.className = `cem-theme-${mode}`;
            container.style.cssText = '--cem-typography-data-font-size:19px; --cem-typography-data-line-height:1.6;';
            await nextRenderFrame();
            const row = required<HTMLElement>(host, 'tbody tr'); const cell = required<HTMLElement>(row, 'td');
            expect(getComputedStyle(cell).fontSize).toBe('19px');
            if (matchMedia('(hover: hover) and (pointer: fine) and (not (any-pointer: coarse))').matches) {
                expect(getComputedStyle(cell).paddingBlockStart).toBe('0px');
                expect(getComputedStyle(cell).paddingBlockEnd).toBe('0px');
                expect(row.getBoundingClientRect().height).toBeCloseTo(parseFloat(getComputedStyle(cell).lineHeight), 0);
            }
        }
        host.setAttribute('value', '[{"changed":true}]'); await whenCemRendered(host);
        expect(host.textContent).toContain('changed'); expect(host.textContent).toContain('true');
        expect(host.textContent).not.toContain('∅ missing');
        container.style.cssText = '--cem-typography-data-font-size:24px; --cem-typography-data-line-height:1.5; inline-size:6rem;';
        host.setAttribute('value', '[{"text":"Several words fit across multiple lines"}]'); await whenCemRendered(host);
        const wrapped = required<HTMLElement>(host, 'tbody tr');
        expect(wrapped.getBoundingClientRect().height).toBeGreaterThan(parseFloat(getComputedStyle(wrapped).lineHeight));
        expect(required(host, 'table').getBoundingClientRect().width).toBeLessThanOrEqual(container.getBoundingClientRect().width + 1);
    },
});
