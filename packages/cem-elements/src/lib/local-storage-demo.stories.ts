import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { cemDiagnosticCodes, traceCemReadiness, whenCemRendered, whenCemSourceRendered } from '../../.storybook/preview.js';
import { readinessCheckpoint, readinessWait } from '../../.storybook/readiness-timing.js';
import { expect, userEvent } from 'storybook/test';
import storagePage from '../../demo/local-storage.html?raw';
import { processNativeCemValue, renderCemMlTemplate } from './internal/runtime-support/cem-ql-render.js';
import { DEFAULT_CEM_VALUE_ARTIFACT_LIMITS } from './native-values.js';
import { applyRenderPlanToRange, type RenderPlan } from './projection.js';

const SOURCE_TAG = 'story-local-storage-demo-document';
const DEMO_URL = new URL('../../demo/local-storage.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '0. Read a live text value',
    '1. Always override a stored value',
    '2. Stored value with a default',
    '3a. Date validation',
    '3b. Time validation',
    '3c. Local date and time validation',
    '3d. Number validation',
    '3e. JSON validation',
    '4. Simplest initial read',
    '5. Live JSON basket',
    '6. Fruit buttons and a storage watcher',
    '7. Write a slice back to storage',
] as const;

const STORAGE_DEFAULTS = {
    cemDemoLiveText: 'stored initial',
    cemDemoPersistedDefault: 'DEF',
    cemDemoDate: '2024-04-20',
    cemDemoTime: '13:30',
    cemDemoLocalDateTime: '1977-04-01T14:00:30',
    cemDemoNumber: '1.23456e+5',
    cemDemoJson: '{"a":1,"b":"B"}',
    cemDemoCherries: '12',
    cemDemoBasket: '{"cherries":12,"lemons":1}',
    cemDemoFruitLemons: '1',
    cemDemoFruitCherries: '12',
    cemDemoFruitApples: '0',
    cemDemoFruitBananas: '0',
    cemDemoSliceEditor: 'shared initial',
} as const;

const meta: Meta = {
    title: 'CEM Elements/Local Storage Demo',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => {
        seedStorage();

        const root = document.createElement('section');
        root.setAttribute('aria-label', 'source-loaded local-storage demo coverage');

        const declaration = document.createElement('cem-element');
        declaration.hidden = true;
        declaration.setAttribute('tag', SOURCE_TAG);
        declaration.setAttribute('src', DEMO_URL.href);

        root.append(declaration, document.createElement(SOURCE_TAG));
        traceCemReadiness(root, 'local-storage/EveryAuthoredSample');
        return root;
    },
    play: async ({ canvasElement, step }) => {
        const host = requiredElement(canvasElement, SOURCE_TAG);
        await waitForCondition(
            () => host.querySelectorAll('cem-demo-element[legend]').length === EXPECTED_LEGENDS.length,
            'all local-storage samples render from the HTML source',
            300
        );

        assertDeepEqual(
            Array.from(host.querySelectorAll('cem-demo-element[legend]'), (sample) =>
                normalize(sample.getAttribute('legend') ?? '')
            ),
            [...EXPECTED_LEGENDS],
            'local-storage sample inventory'
        );

        await whenCemSourceRendered(host);
        readinessCheckpoint('initial-render-settled');
        // Card presence precedes child declaration/render settlement. Keep the
        // hydration and interaction predicates below with their existing limits.
        const checks = [verifyLiveText, verifyAuthoritativeValue, verifyPersistedDefault,
            ...DATE_CASES.map(data => (sample: HTMLElement) => verifyDateAndTime(sample, data)),
            verifyNumbers, verifyJson, verifyInitialRead, verifyJsonBasket, verifyFruitWriterAndWatcher, verifySliceEditor];
        const keys = [['cemDemoLiveText'], ['cemDemoOverride'], ['cemDemoPersistedDefault'],
            ...DATE_CASES.map(([key]) => [key]), ['cemDemoNumber'], ['cemDemoJson'], ['cemDemoCherries'],
            ['cemDemoBasket'], ['cemDemoFruitLemons', 'cemDemoFruitCherries', 'cemDemoFruitApples', 'cemDemoFruitBananas'],
            ['cemDemoSliceEditor']];
        await waitForCondition(() => localStorage.getItem('cemDemoOverride') === 'ABC', 'authoritative storage has settled');
        for (const [index, legend] of EXPECTED_LEGENDS.entries()) {
            await step(legend, async () => {
                const sample = sampleByLegend(host, legend);
                const unchanged = [...Object.keys(STORAGE_DEFAULTS), 'cemDemoOverride'].filter(key => !keys[index].includes(key));
                const before = unchanged.map(key => localStorage.getItem(key));
                await checks[index](sample);
                const instances = producedInstances(sample);
                await Promise.all(instances.map(instance => whenCemRendered(instance)));
                expect(unchanged.map(key => localStorage.getItem(key))).toEqual(before);
                for (const instance of instances) {
                    expect([...new Set(cemDiagnosticCodes(instance))].sort(), `${legend} diagnostics`).toEqual(index === 7
                        ? ['cem-element.local_storage_json_invalid', 'cem.render_plan_apply.replace_scope'] : []);
                }
            });
        }
        expect(cemDiagnosticCodes(host)).toEqual([]);
    },
};

// Protect the accepted whole-scope recovery policy separately from native import.
// The decision is recorded in the browser stabilization checklist.
export const NativeJsonRootTransitions: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        // Select authored template text; source documents enter only CEM-ML import.
        const source = storagePage.split(`legend="${EXPECTED_LEGENDS[7]}"`)[1]
            .split('<template type="text/cem-ml">')[1].split('</template>')[0];
        const start = document.createComment('cem-render-start');
        const end = document.createComment('cem-render-end');
        canvasElement.append(start, end);
        const bounds = { start, end };
        let revision = 0;
        const plan = async (text: string | null): Promise<RenderPlan> => {
            const imported = text === null ? null : await processNativeCemValue({
                action: 'import', bytes: new TextEncoder().encode(text).buffer,
                contentType: 'application/json', sourceUri: 'storage:diagnostics',
                scopePolicyStamp: 'test', limits: DEFAULT_CEM_VALUE_ARTIFACT_LIMITS,
            });
            if (imported && !('value' in imported)) throw new Error('Expected a native CEM document');
            const result = await renderCemMlTemplate(source, { datadom: { slices: { raw: text, json: null } } }, {
                nativeSlices: imported ? [{ name: 'json', value: imported.value }] : [],
            });
            expect(result.diagnostics).toEqual([]);
            return { nodes: result.nodes, producedTag: 'storage-projection', instanceId: 'one',
                dataRevision: String(++revision), templateArtifactId: 'storage',
                scopePolicyStamp: 'test', outputTarget: 'light-dom' };
        };
        const pending = await plan(null);
        expect(applyRenderPlanToRange(bounds, pending, document).diagnostics).toEqual([]);
        const originalParagraph = canvasElement.querySelector('p');
        const loaded = await plan('{"a":1,"b":"B"}');
        const rootIds = (value: RenderPlan) => value.nodes.flatMap(node =>
            node.kind === 'element' && node.tag === 'p' ? [node.renderNodeId] : []);
        expect(rootIds(loaded)).toEqual(rootIds(pending));
        const applied = applyRenderPlanToRange(bounds, loaded, document);
        expect(applied.mode).toBe('replaceScope');
        expect(applied.diagnostics).toEqual([{
            code: 'cem.render_plan_apply.replace_scope', severity: 'warning', reason: 'recovery',
            message: 'retained render scope root identities did not match the next render plan; replaced the scope',
        }]);
        expect(canvasElement.querySelector('p')).not.toBe(originalParagraph);
        expect(textList(canvasElement, 'li')).toEqual(['a: 1', 'b: B']);
        const loadedParagraph = canvasElement.querySelector('p');
        const array = await plan('[1,2,3]');
        expect(rootIds(array)).toEqual(rootIds(loaded));
        // Object and array lists come from distinct authored branches. Their
        // source-based IDs differ, so the accepted root-set recovery applies.
        const listId = (value: RenderPlan) => value.nodes.find(node =>
            node.kind === 'element' && (node.tag === 'ul' || node.tag === 'ol'))?.renderNodeId;
        expect(listId(loaded)).toBeTruthy();
        expect(listId(array)).toBeTruthy();
        expect(listId(array)).not.toBe(listId(loaded));
        const arrayApplied = applyRenderPlanToRange(bounds, array, document);
        expect(arrayApplied.mode).toBe('replaceScope');
        expect(arrayApplied.diagnostics).toEqual(applied.diagnostics);
        expect(canvasElement.querySelector('p')).not.toBe(loadedParagraph);
        expect(textList(canvasElement, 'li')).toEqual(['1', '2', '3']);
        const scalar = await plan('false');
        expect(applyRenderPlanToRange(bounds, scalar, document).mode).toBe('replaceScope');
        expect(textList(canvasElement, 'output')).toEqual(['false', 'false']);
    },
};

function producedInstance(sample: HTMLElement): HTMLElement {
    const tag = requiredElement(sample, 'cem-element').getAttribute('tag');
    if (!tag) throw new Error('Anonymous storage declaration has no produced tag');
    return requiredElement(sample, tag);
}

function producedInstances(sample: HTMLElement): HTMLElement[] {
    const tag = requiredElement(sample, 'cem-element').getAttribute('tag');
    if (!tag) throw new Error('Storage declaration has no produced tag');
    return Array.from(sample.querySelectorAll<HTMLElement>(tag));
}

async function verifyLiveText(sample: HTMLElement): Promise<void> {
    await waitForText(sample, 'output', 'stored initial', 'the live text slice hydrates');
    for (const [button, value, output] of [
        ['text value', 'text value', 'text value'],
        ['another value', 'another value', 'another value'],
        ['Empty string', '', ''],
        ['Clear key', null, 'null'],
        ['text value', 'text value', 'text value'],
    ] as const) {
        buttonByText(sample, button).click();
        await waitForText(sample, 'output', output, `${button} updates the live slice`);
        assertEqual(localStorage.getItem('cemDemoLiveText'), value, `${button} writes storage directly`);
    }
    const unrelated = localStorage.getItem('cem-demo-unrelated-key');
    try {
        localStorage.setItem('cem-demo-unrelated-key', 'unrelated');
        await whenCemRendered(producedInstance(sample));
        assertEqual(textList(sample, 'output')[0], 'text value', 'unrelated keys leave this slice alone');
    } finally {
        if (unrelated === null) localStorage.removeItem('cem-demo-unrelated-key');
        else localStorage.setItem('cem-demo-unrelated-key', unrelated);
    }
}

async function verifySliceEditor(sample: HTMLElement): Promise<void> {
    const inputs = Array.from(sample.querySelectorAll('input'));
    expect(inputs).toHaveLength(2);
    expect(inputs.map(input => input.labels?.[0]?.textContent?.trim())).toEqual(['Editor A', 'Editor B']);
    const state = async (value: string) => {
        await waitForCondition(() => textList(sample, 'output').join('|') === `${value}|${value}`
            && localStorage.getItem('cemDemoSliceEditor') === value, 'both editor slices and storage agree');
        await Promise.all(producedInstances(sample).map(instance => whenCemRendered(instance)));
        expect(Array.from(sample.querySelectorAll('input'))).toEqual(inputs);
        expect(inputs.map(input => input.value)).toEqual([value, value]);
        for (const instance of producedInstances(sample)) expect(cemDiagnosticCodes(instance), `editor value ${value}`).toEqual([]);
    };
    await state('shared initial');
    for (const [index, value, paste] of [[0, 'from A', false], [1, 'from B', false], [0, '', false], [1, '🍒 B', true]] as const) {
        const input = inputs[index];
        await userEvent.clear(input);
        // Paste inserts a complete Unicode string; the keyboard helper splits surrogate pairs.
        if (paste) await userEvent.paste(value);
        else if (value) await userEvent.type(input, value);
        await state(value);
        expect(document.activeElement).toBe(input);
        expect([input.selectionStart, input.selectionEnd]).toEqual([value.length, value.length]);
    }
    inputs[1].setSelectionRange(1, 3, 'backward');
    localStorage.setItem('cemDemoSliceEditor', 'external');
    await state('external');
    expect(document.activeElement).toBe(inputs[1]);
    expect([inputs[1].selectionStart, inputs[1].selectionEnd, inputs[1].selectionDirection]).toEqual([1, 3, 'backward']);
}

async function verifyAuthoritativeValue(sample: HTMLElement): Promise<void> {
    await waitForText(sample, 'output', 'ABC', 'the authoritative value hydrates');
    for (const button of ['Try text value', 'Clear key']) {
        buttonByText(sample, button).click();
        await waitForCondition(
            () => localStorage.getItem('cemDemoOverride') === 'ABC'
                && textList(sample, 'output')[0] === 'ABC',
            `${button}: the value attribute restores both storage and the slice`
        );
    }
}

async function verifyPersistedDefault(sample: HTMLElement): Promise<void> {
    await waitForText(sample, 'output', 'DEF', 'the missing key receives the host default');
    buttonByText(sample, 'remember me').click();
    await waitForText(sample, 'output', 'remember me', 'the declaration accepts a later edit');
    assertEqual(
        localStorage.getItem('cemDemoPersistedDefault'),
        'remember me',
        'the declaration without value preserves the edit'
    );
    buttonByText(sample, 'Empty string').click();
    await waitForText(sample, 'output', '', 'an empty string does not invoke the default');
    assertEqual(localStorage.getItem('cemDemoPersistedDefault'), '', 'empty is a stored value');
    buttonByText(sample, 'Clear key').click();
    await waitForText(sample, 'output', 'null', 'the default is seeded on page load, not on every render');
    assertEqual(localStorage.getItem('cemDemoPersistedDefault'), null, 'clear removes the key');
    buttonByText(sample, 'remember me').click();
    await waitForText(sample, 'output', 'remember me', 'a later external write recovers');
}

const DATE_CASES = [
    ['cemDemoDate', '2024-04-20', 'ABC — invalid', '2024-02-29'],
    ['cemDemoTime', '13:30', '25:00 — invalid', '09:15'],
    ['cemDemoLocalDateTime', '1977-04-01T14:00:30', 'ABC — invalid', '2024-04-20T09:15'],
] as const;

async function verifyDateAndTime(sample: HTMLElement, [key, initial, invalid, recovery]: typeof DATE_CASES[number]): Promise<void> {
    await waitForText(sample, 'output', initial, `${key} hydrates`);
    buttonByText(sample, invalid).click();
    await waitForText(sample, 'output', 'null', `${key} rejects invalid storage text`);
    assertEqual(localStorage.getItem(key), invalid.split(' ')[0], 'invalid raw text remains in storage');
    const input = requiredElement(sample, 'input') as HTMLInputElement;
    for (const value of [recovery, '', initial]) {
        await submitRaw(sample, value);
        await waitForText(sample, 'output', value || 'null', `${key} recovers or clears through its text form`);
        expect(localStorage.getItem(key)).toBe(value);
        expect(sample.querySelector('input')).toBe(input);
        expect(input.value).toBe(value);
    }
    if (key === 'cemDemoDate') {
        buttonByText(sample, 'ISO timestamp').click();
        await waitForText(sample, 'output', '2024-04-21', 'ISO timestamps normalize to a date');
        expect(localStorage.getItem(key)).toBe('2024-04-21T03:58:42.131Z');
    }
}

async function verifyNumbers(sample: HTMLElement): Promise<void> {
    await waitForText(sample, 'p:last-of-type output', '123456', 'the exponent hydrates as a number');
    for (const [button, raw, parsed] of [
        ['0001', '0001', '1'],
        ['0', '0', '0'],
        ['ABC — invalid', 'ABC', 'null'],
        ['1.23456e+5', '1.23456e+5', '123456'],
    ]) {
        buttonByText(sample, button).click();
        await waitForCondition(
            () => textList(sample, 'output').join('|') === `${raw}|${parsed}`,
            `${button} projects raw text and a coerced number`
        );
        assertEqual(localStorage.getItem('cemDemoNumber'), raw, 'coercion does not rewrite storage');
    }
    await submitRaw(sample, '-2.5');
    await waitForText(sample, 'p:last-of-type output', '-2.5', 'a custom decimal is accepted');
    await submitRaw(sample, '');
    await waitForText(sample, 'p:last-of-type output', 'null', 'empty numeric text becomes null');
    assertEqual(localStorage.getItem('cemDemoNumber'), '', 'invalid empty text is retained');
}

async function verifyJson(sample: HTMLElement): Promise<void> {
    await waitForCondition(() => textList(sample, 'ul li').join('|') === 'a: 1|b: B', 'JSON object fields hydrate');
    assertDeepEqual(
        cemDiagnosticCodes(producedInstance(sample)).filter(code => code !== 'cem.render_plan_apply.replace_scope'),
        [],
        'valid native JSON hydration has no import or evaluation diagnostics'
    );
    for (const [button, raw, parsed] of [
        ['JSON string', '"ABC"', 'ABC'],
        ['JSON number', '12.345', '12.345'],
        ['JSON false', 'false', 'false'],
        ['ABC — invalid', 'ABC', 'null'],
    ]) {
        buttonByText(sample, button).click();
        await waitForCondition(
            () => textList(sample, 'output').join('|') === `${raw}|${parsed}`,
            `${button} projects the raw and parsed JSON values`
        );
        expect(sample.querySelectorAll('li')).toHaveLength(0);
        assertEqual(localStorage.getItem('cemDemoJson'), raw, 'JSON parsing does not rewrite storage');
        if (raw === 'ABC') {
            assertEqual(cemDiagnosticCodes(producedInstance(sample)).includes('cem-element.local_storage_json_invalid'),
                true, 'invalid JSON retains the import diagnostic');
        }
    }
    for (const [raw, parsed] of [['0', '0'], ['true', 'true'], ['null', 'null'], ['""', ''], ['{}', 'object'], ['[]', 'array']]) {
        await submitRaw(sample, raw);
        await waitForCondition(() => textList(sample, 'output').join('|') === `${raw}|${parsed}`, `JSON ${raw} retains its type`);
        expect(localStorage.getItem('cemDemoJson')).toBe(raw);
        expect(sample.querySelectorAll('li')).toHaveLength(0);
    }
    await submitRaw(sample, '[1,2,3]');
    await waitForCondition(
        () => textList(sample, 'ol li').join('|') === '1|2|3'
            && textList(sample, 'output')[1] === 'array',
        'a JSON array projects each parsed member'
    );
    expect(sample.querySelectorAll('ul')).toHaveLength(0);
    expect(localStorage.getItem('cemDemoJson')).toBe('[1,2,3]');
    await submitRaw(sample, '{"fruit":"cherry","count":0}');
    await waitForCondition(
        () => textList(sample, 'ul li').join('|') === 'fruit: cherry|count: 0',
        'object projection follows newly written field names and retains zero'
    );
    expect(sample.querySelectorAll('ol')).toHaveLength(0);
    expect(localStorage.getItem('cemDemoJson')).toBe('{"fruit":"cherry","count":0}');
    buttonByText(sample, 'Object').click();
    await waitForCondition(() => textList(sample, 'ul li').join('|') === 'a: 1|b: B', 'valid JSON restores object fields');
}

async function verifyInitialRead(sample: HTMLElement): Promise<void> {
    await waitForCondition(
        () => normalize(requiredElement(sample, 'cem-storage-cherries').textContent ?? '') === '12🍒',
        'the initial-only reader combines the stored count and payload unit'
    );
    const writer = buttonByLabel(sample, 'Store 24 cherries');
    const output = requiredElement(sample, 'output');
    assertEqual(normalize(writer.textContent ?? ''), '24🍒', 'the initial writer displays its fruit count');
    writer.click();
    assertEqual(localStorage.getItem('cemDemoCherries'), '24', 'the external button writes a new count');
    await nextFrame();
    await nextFrame();
    assertEqual(textList(sample, 'output')[0], '12', 'without live the existing instance retains its initial read');
    const fresh = document.createElement('cem-storage-cherries');
    fresh.textContent = '🍒';
    sample.append(fresh);
    try {
        await waitForText(fresh, 'output', '24', 'a fresh instance reads the changed storage value');
    } finally {
        fresh.remove();
    }
    buttonByLabel(sample, 'Store 12 cherries').click();
    expect(localStorage.getItem('cemDemoCherries')).toBe('12');
    await whenCemRendered(requiredElement(sample, 'cem-storage-cherries'));
    expect(sample.querySelector('output')).toBe(output);
    expect(output.textContent).toBe('12');
}

async function verifyJsonBasket(sample: HTMLElement): Promise<void> {
    await waitForCondition(
        () => definitionValue(sample, '🛒') === '13',
        'the JSON basket hydrates and calculates its initial total'
    );

    const buttons = Array.from(sample.querySelectorAll('button'));
    const state = async (cherries: number, lemons: number) => {
        const stored = `{"cherries":${cherries},"lemons":${lemons}}`;
        await waitForCondition(() => textList(sample, 'dd').join('|') === `${cherries}|${lemons}|${cherries + lemons}`
            && localStorage.getItem('cemDemoBasket') === stored, 'basket fields, total and exact JSON agree');
        expect(Array.from(sample.querySelectorAll('button'))).toEqual(buttons);
    };
    await state(12, 1);
    assertEqual(normalize(buttonByLabel(sample, 'Add cherry').textContent ?? ''), '+🍒', 'add cherry is symbolic');
    assertEqual(normalize(buttonByLabel(sample, 'Add lemon').textContent ?? ''), '+🍋', 'add lemon is symbolic');
    assertEqual(normalize(buttonByLabel(sample, 'Reset basket').textContent ?? ''), '↺🛒', 'reset is symbolic');
    buttonByLabel(sample, 'Add cherry').click();
    await state(13, 1);
    buttonByLabel(sample, 'Add lemon').click();
    await waitForCondition(
        () => definitionValue(sample, '🛒') === '15'
            && localStorage.getItem('cemDemoBasket') === '{"cherries":13,"lemons":2}',
        'the native edit renders and completes JSON export'
    );
    assertDefinitionValue(sample, '🍒', '13');
    assertDefinitionValue(sample, '🍋', '2');
    assertEqual(
        localStorage.getItem('cemDemoBasket'),
        '{"cherries":13,"lemons":2}',
        'the edited JSON text is persisted'
    );
    buttonByLabel(sample, 'Reset basket').click();
    await state(12, 1);
    buttons[0].focus();
    await userEvent.keyboard('{Enter}');
    await state(13, 1);
    await userEvent.keyboard(' ');
    await state(14, 1);
    expect(document.activeElement).toBe(buttons[0]);
    localStorage.setItem('cemDemoBasket', '{"cherries":0,"lemons":2}');
    await state(0, 2);
    buttonByLabel(sample, 'Reset basket').click();
    await state(12, 1);
}

async function verifyFruitWriterAndWatcher(sample: HTMLElement): Promise<void> {
    await waitForCondition(
        () => definitionValue(sample, '🛒') === '13',
        'the watcher hydrates all four fruit slices'
    );

    const cases = [
        { button: 'Add lemon', term: '🍋', key: 'cemDemoFruitLemons', expected: '2' },
        { button: 'Add cherry', term: '🍒', key: 'cemDemoFruitCherries', expected: '13' },
        { button: 'Add apple', term: '🍏', key: 'cemDemoFruitApples', expected: '1' },
        { button: 'Add banana', term: '🍌', key: 'cemDemoFruitBananas', expected: '1' },
    ] as const;

    const values = ['1', '12', '0', '0'];
    for (const [index, fruit] of cases.entries()) {
        const button = buttonByLabel(sample, fruit.button);
        assertEqual(normalize(button.textContent ?? ''), `+${fruit.term}`, `${fruit.button} displays its symbol`);
        assertEqual(button.title, fruit.button, `${fruit.button} retains its tooltip`);
        button.click();
        await waitForCondition(
            () => definitionValue(sample, fruit.term) === fruit.expected,
            `${fruit.button} updates the watching DCE`
        );
        assertEqual(localStorage.getItem(fruit.key), fruit.expected, `${fruit.button} persists its count`);
        values[index] = fruit.expected;
        expect(textList(sample, 'dd')).toEqual([...values, String(14 + index)]);
    }
    assertDefinitionValue(sample, '🛒', '17');
}

function seedStorage(): void {
    for (const [key, value] of Object.entries(STORAGE_DEFAULTS)) localStorage.setItem(key, value);
    localStorage.removeItem('cemDemoOverride');
}


function sampleByLegend(host: ParentNode, legend: string): HTMLElement {
    const sample = Array.from(host.querySelectorAll<HTMLElement>('cem-demo-element[legend]')).find(
        (candidate) => normalize(candidate.getAttribute('legend') ?? '') === legend
    );
    if (!sample) throw new Error(`expected sample ${legend}`);
    return sample;
}

function buttonByText(root: ParentNode, expected: string): HTMLButtonElement {
    const button = Array.from(root.querySelectorAll('button')).find(
        (candidate) => normalize(candidate.textContent ?? '') === expected
    );
    if (!button) throw new Error(`expected ${expected} button`);
    return button;
}

function buttonByLabel(root: ParentNode, expected: string): HTMLButtonElement {
    const button = root.querySelector<HTMLButtonElement>(`button[aria-label="${expected}"]`);
    if (!button) throw new Error(`expected ${expected} button`);
    return button;
}

async function submitRaw(sample: HTMLElement, value: string): Promise<void> {
    const form = requiredElement(sample, 'form') as HTMLFormElement;
    const input = requiredElement(form, 'input[name="raw"]') as HTMLInputElement;
    await userEvent.clear(input);
    if (value) await userEvent.type(input, value.replaceAll('{', '{{').replaceAll('[', '[['));
    await userEvent.click(requiredElement(form, 'button'));
}

function nextFrame(): Promise<void> {
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

function textList(root: ParentNode, selector: string): string[] {
    return Array.from(root.querySelectorAll(selector), (element) => normalize(element.textContent ?? ''));
}

function definitionValue(root: ParentNode, term: string): string {
    const definitionTerm = Array.from(root.querySelectorAll('dt')).find(
        (candidate) => normalize(candidate.textContent ?? '') === term
    );
    const value = definitionTerm?.nextElementSibling;
    if (!(value instanceof HTMLElement) || value.localName !== 'dd') {
        throw new Error(`expected definition value for ${term}`);
    }
    return normalize(value.textContent ?? '');
}

function assertDefinitionValue(root: ParentNode, term: string, expected: string): void {
    assertEqual(definitionValue(root, term), expected, term);
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`expected ${selector}`);
    return element;
}

async function waitForText(root: ParentNode, selector: string, expected: string, label: string): Promise<void> {
    await waitForCondition(
        () => {
            const element = root.querySelector(selector);
            return element !== null && normalize(element.textContent ?? '') === expected;
        },
        label
    );
}

async function waitForCondition(condition: () => boolean, message: string, attempts = 160): Promise<void> {
    const mark = readinessWait(message, attempts);
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (condition()) {
            mark('ready', attempt);
            return;
        }
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
    mark('timeout', attempts);
    throw new Error(message);
}

function assertDeepEqual(actual: readonly string[], expected: readonly string[], label: string): void {
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
        throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
    }
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
    if (actual !== expected) {
        throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
    }
}

function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}
