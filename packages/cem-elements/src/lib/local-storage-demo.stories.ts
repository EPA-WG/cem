import type { Meta, StoryObj } from '@storybook/web-components-vite';

const SOURCE_TAG = 'story-local-storage-demo-document';
const DEMO_URL = new URL('../../demo/local-storage.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '0. Read a live text value',
    '1. Always override a stored value',
    '2. Stored value with a default',
    '3. Typed localStorage values',
    '4. Simplest initial read',
    '5. Live JSON basket',
    '6. Fruit buttons and a storage watcher',
] as const;

const STORAGE_DEFAULTS = {
    cemDemoLiveText: 'stored initial',
    cemDemoPersistedDefault: 'DEF',
    cemDemoDate: '2024-04-20',
    cemDemoTime: '13:30',
    cemDemoLocalDateTime: '1977-04-01T14:00:30',
    cemDemoNumber: '1.23456e+5',
    cemDemoInvalidNumber: 'ABC',
    cemDemoJson: JSON.stringify({ a: 1, b: 'B' }),
    cemDemoCherries: '12',
    cemDemoBasket: JSON.stringify({ cherries: 12, lemons: 1 }),
    cemDemoFruitLemons: '1',
    cemDemoFruitCherries: '12',
    cemDemoFruitApples: '0',
    cemDemoFruitBananas: '0',
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
        return root;
    },
    play: async ({ canvasElement }) => {
        const host = requiredElement(canvasElement, SOURCE_TAG);
        await waitForCondition(
            () => host.querySelectorAll('cem-demo-element[legend]').length === EXPECTED_LEGENDS.length,
            'all seven local-storage samples render from the HTML source',
            300
        );

        assertDeepEqual(
            Array.from(host.querySelectorAll('cem-demo-element[legend]'), (sample) =>
                normalize(sample.getAttribute('legend') ?? '')
            ),
            [...EXPECTED_LEGENDS],
            'local-storage sample inventory'
        );

        await verifyLiveText(sampleByLegend(host, EXPECTED_LEGENDS[0]));
        await verifyAuthoritativeValue(sampleByLegend(host, EXPECTED_LEGENDS[1]));
        await verifyPersistedDefault(sampleByLegend(host, EXPECTED_LEGENDS[2]));
        await verifyTypedValues(sampleByLegend(host, EXPECTED_LEGENDS[3]));
        await verifyInitialRead(sampleByLegend(host, EXPECTED_LEGENDS[4]));
        await verifyJsonBasket(sampleByLegend(host, EXPECTED_LEGENDS[5]));
        await verifyFruitWriterAndWatcher(sampleByLegend(host, EXPECTED_LEGENDS[6]));
    },
};

async function verifyLiveText(sample: HTMLElement): Promise<void> {
    await waitForCondition(
        () => textList(sample, 'output').join('|') === 'stored initial|stored initial',
        'both live text instances hydrate from the same key'
    );

    dispatchInput(requiredElement(sample, 'cem-storage-live-text:first-of-type input') as HTMLInputElement, 'shared edit');
    await waitForCondition(
        () => textList(sample, 'output').join('|') === 'shared edit|shared edit',
        'the second instance observes the first instance storage write'
    );
    assertEqual(localStorage.getItem('cemDemoLiveText'), 'shared edit', 'the live text edit is persisted');
}

async function verifyAuthoritativeValue(sample: HTMLElement): Promise<void> {
    await waitForText(sample, 'output', 'ABC', 'the authoritative value hydrates');
    buttonByText(sample, 'Try text value').click();
    await waitForCondition(
        () => normalize(requiredElement(sample, 'output').textContent ?? '') === 'ABC',
        'the value attribute wins after an attempted slice replacement'
    );
    assertEqual(localStorage.getItem('cemDemoOverride'), 'ABC', 'the authoritative value rewrites storage');
}

async function verifyPersistedDefault(sample: HTMLElement): Promise<void> {
    await waitForText(sample, 'output', 'DEF', 'the missing key receives the host default');
    dispatchInput(requiredElement(sample, 'input') as HTMLInputElement, 'remember me');
    await waitForText(sample, 'output', 'remember me', 'the declaration accepts a later edit');
    assertEqual(
        localStorage.getItem('cemDemoPersistedDefault'),
        'remember me',
        'the declaration without value preserves the edit'
    );
}

async function verifyTypedValues(sample: HTMLElement): Promise<void> {
    await waitForCondition(
        () => definitionValue(sample, 'JSON b') === 'B',
        'typed storage values finish their initial resource render'
    );
    assertDefinitionValue(sample, 'date', '2024-04-20');
    assertDefinitionValue(sample, 'time', '13:30');
    assertDefinitionValue(sample, 'datetime', '1977-04-01T14:00:30');
    assertDefinitionValue(sample, 'number', '123456');
    assertDefinitionValue(sample, 'invalid number', 'null');
    assertDefinitionValue(sample, 'JSON a', '1');
    assertDefinitionValue(sample, 'JSON b', 'B');
}

async function verifyInitialRead(sample: HTMLElement): Promise<void> {
    await waitForCondition(
        () => normalize(requiredElement(sample, 'cem-storage-cherries').textContent ?? '') === '12🍒',
        'the initial-only reader combines the stored count and payload unit'
    );
}

async function verifyJsonBasket(sample: HTMLElement): Promise<void> {
    await waitForCondition(
        () => definitionValue(sample, '🛒 total') === '13',
        'the JSON basket hydrates and calculates its initial total'
    );

    dispatchInput(
        requiredElement(sample, 'textarea') as HTMLTextAreaElement,
        JSON.stringify({ cherries: 13, lemons: 2 })
    );
    await waitForCondition(
        () => definitionValue(sample, '🛒 total') === '15',
        'editing JSON updates the parsed live basket'
    );
    assertDefinitionValue(sample, 'cherries', '13');
    assertDefinitionValue(sample, 'lemons', '2');
    assertEqual(
        localStorage.getItem('cemDemoBasket'),
        JSON.stringify({ cherries: 13, lemons: 2 }),
        'the edited JSON text is persisted'
    );
}

async function verifyFruitWriterAndWatcher(sample: HTMLElement): Promise<void> {
    await waitForCondition(
        () => definitionValue(sample, 'Total') === '13',
        'the watcher hydrates all four fruit slices'
    );

    const cases = [
        { button: 'Add lemon', term: '🍋 lemons', key: 'cemDemoFruitLemons', expected: '2' },
        { button: 'Add cherry', term: '🍒 cherries', key: 'cemDemoFruitCherries', expected: '13' },
        { button: 'Add apple', term: '🍏 apples', key: 'cemDemoFruitApples', expected: '1' },
        { button: 'Add banana', term: '🍌 bananas', key: 'cemDemoFruitBananas', expected: '1' },
    ] as const;

    for (const fruit of cases) {
        buttonByLabel(sample, fruit.button).click();
        await waitForCondition(
            () => definitionValue(sample, fruit.term) === fruit.expected,
            `${fruit.button} updates the watching DCE`
        );
        assertEqual(localStorage.getItem(fruit.key), fruit.expected, `${fruit.button} persists its count`);
    }
    assertDefinitionValue(sample, 'Total', '17');
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

function dispatchInput(control: HTMLInputElement | HTMLTextAreaElement, value: string): void {
    control.value = value;
    control.dispatchEvent(new Event('input', { bubbles: true }));
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
        () => normalize(requiredElement(root, selector).textContent ?? '') === expected,
        label
    );
}

async function waitForCondition(condition: () => boolean, message: string, attempts = 160): Promise<void> {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (condition()) return;
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
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
