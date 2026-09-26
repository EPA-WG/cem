import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor } from 'storybook/test';
import { createCemDeclarationScope } from './declaration-scope.js';
import { CemElementRuntime, writeDataIslandHydrationData } from './cem-elements.js';
import { cemDiagnosticCodes, whenCemSourceRendered } from '../../.storybook/preview.js';

const SOURCE_TAG = 'story-scoped-css-demo-document';
const DEMO_URL = new URL('../../demo/scoped-css.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '1. Private declaration CSS and ordinary outer cascade',
    '2. Component in a named scope shares default declaration styles with peers in the same scope',
    '3. Style can be scoped explicitly',
    '4. Mixed private and shared styles',
    '5. Invalid and mismatched scopes fail closed',
    '6. Payload style belongs to one instance',
    '7. Declaration styles must be static',
    '8. Fragment template CSS uses the effective produced tag',
    '9. Anonymous declaration CSS uses its generated tag',
    '10. uid-seed stabilizes keyframe names',
    '11. Descendant selectors stay inside the component',
    '12. CSS from an external template fragment',
    '13. Instance IDs in URL-valued custom properties',
] as const;

const meta: Meta = {
    title: 'CEM Elements/Scoped CSS Demo',
    tags: ['test'],
};

export default meta;
type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => sourceLoadedDemo(),
    play: async ({ canvasElement, step }) => {
        const host = requiredElement(canvasElement, SOURCE_TAG);
        await waitForCondition(() => host.querySelectorAll('cem-demo-element[legend]').length === EXPECTED_LEGENDS.length,
            'all scoped-CSS samples render from the HTML source');
        expect(sampleLegends(host)).toEqual([...EXPECTED_LEGENDS]);
        await whenCemSourceRendered(host);
        const samples = EXPECTED_LEGENDS.map(legend => sampleByLegend(host, legend));
        await step(EXPECTED_LEGENDS[0], async () => {
            const sample = samples[0];
            const buttons = Array.from(sample.querySelectorAll('cem-css-private button'));
            expect(buttons).toHaveLength(2);
            expect(buttons.map(button => normalize(button.textContent ?? ''))).toEqual(['First DCE dashed border', 'Second DCE dashed border']);
            for (const button of buttons) {
                expect(getComputedStyle(button).borderTopStyle).toBe('dashed');
                expect(getComputedStyle(button).borderTopColor).toBe('rgb(0, 128, 0)');
                expect(getComputedStyle(button).color).toBe('rgb(148, 0, 211)');
            }
            expect(style(sample, '[slot="demo"] > button', 'borderTopStyle')).not.toBe('dashed');
            expect(sample.querySelectorAll('cem-css-private style')).toHaveLength(0);
            expect(managed(sample, 'cem-css-private', 'private')).toHaveLength(1);
            expect(managed(sample, 'cem-css-private')[0].textContent).toContain('cem-css-private');
            expect(sample.querySelectorAll('cem-css-private[scope], cem-css-private[data-cem-instance-scope]')).toHaveLength(0);
        });
        await step(EXPECTED_LEGENDS[1], async () => {
            const sample = samples[1];
            for (const tag of ['cem-css-shared-bare', 'cem-css-shared-peer']) {
                expect(style(sample, `${tag} b`, 'color')).toBe('rgb(0, 128, 0)');
                expect(requiredElement(sample, tag).getAttribute('scope')).toBe('css-samples');
            }
            expect(managed(sample, 'cem-css-shared-bare', 'shared')).toHaveLength(1);
            expect(managed(sample, 'cem-css-shared-bare')[0].textContent).toContain('[scope="css-samples"]');
        });
        await step(EXPECTED_LEGENDS[2], async () => {
            const sample = samples[2];
            for (const tag of ['cem-css-shared-explicit', 'cem-css-explicit-peer']) {
                expect(style(sample, `${tag} .shared-bg`, 'backgroundColor')).toBe('rgb(219, 234, 254)');
                expect(requiredElement(sample, tag).getAttribute('scope')).toBe('css-samples');
            }
            expect(managed(sample, 'cem-css-shared-explicit', 'shared')).toHaveLength(1);
        });
        await step(EXPECTED_LEGENDS[3], async () => {
            const sample = samples[3];
            expect(style(sample, 'cem-css-mixed .sample-mixed', 'borderTopColor')).toBe('rgb(0, 0, 255)');
            expect(style(sample, 'cem-css-mixed .sample-mixed', 'borderTopStyle')).toBe('solid');
            expect(style(sample, 'cem-css-mixed-peer .sample-mixed-shared', 'borderTopStyle')).toBe('none');
            for (const tag of ['cem-css-mixed', 'cem-css-mixed-peer']) {
                expect(style(sample, `${tag} .sample-mixed-shared`, 'color')).toBe('rgb(255, 0, 0)');
            }
            expect(managed(sample, 'cem-css-mixed')).toHaveLength(2);
            expect(managed(sample, 'cem-css-mixed', 'private')).toHaveLength(1);
            expect(managed(sample, 'cem-css-mixed', 'shared')).toHaveLength(1);
        });
        await step(EXPECTED_LEGENDS[4], async () => {
            const sample = samples[4];
            for (const tag of ['cem-css-unscoped-explicit', 'cem-css-mismatch']) {
                expect(style(sample, `${tag} .must-not-apply`, 'color')).not.toBe('rgb(255, 0, 0)');
                expect(managed(sample, tag)).toHaveLength(0);
            }
            expect(style(sample, 'cem-css-mismatch-bare .sample-valid-bare', 'color')).toBe('rgb(0, 128, 0)');
            expect(managed(sample, 'cem-css-mismatch-bare', 'shared')).toHaveLength(1);
            expect(style(sample, 'cem-css-invalid-declaration .sample-invalid-declaration', 'color')).toBe('rgb(0, 0, 255)');
            expect(requiredElement(sample, 'cem-css-invalid-declaration').hasAttribute('scope')).toBe(false);
            expect(managed(sample, 'cem-css-invalid-declaration', 'private')).toHaveLength(1);
        });
        await step(EXPECTED_LEGENDS[5], async () => {
            const sample = samples[5];
            expect(style(sample, 'cem-css-instance:first-of-type button', 'borderTopColor')).toBe('rgb(0, 0, 255)');
            expect(style(sample, 'cem-css-instance:last-of-type button', 'borderTopColor')).toBe('rgb(255, 0, 0)');
            expect(managed(sample, 'cem-css-instance', 'private')).toHaveLength(1);
            expect(sample.querySelectorAll('cem-css-instance:first-of-type > style')).toHaveLength(0);
            const payloadStyle = requiredElement(sample, 'cem-css-instance:last-of-type > style');
            expect(payloadStyle.textContent).toContain('@scope to (');
            expect(payloadStyle.textContent).not.toContain('data-cem-render-scope');
            expect(requiredElement(sample, 'cem-css-instance:last-of-type').hasAttribute('data-cem-instance-scope')).toBe(false);
        });
        await step(EXPECTED_LEGENDS[6], async () => {
            const sample = samples[6];
            expect(managed(sample, 'cem-css-dynamic')).toHaveLength(0);
            expect(sample.querySelector('cem-css-dynamic style')).toBeNull();
            expect(style(sample, 'cem-css-dynamic .must-not-apply', 'color')).not.toBe('rgb(255, 0, 0)');
            expect(style(sample, 'cem-css-dynamic .must-not-apply', 'backgroundColor')).not.toBe('rgb(255, 0, 0)');
            expect(normalize(requiredElement(sample, 'cem-css-dynamic .must-not-apply').textContent ?? '')).toBe('dynamic styles rejected');
        });
        await step(EXPECTED_LEGENDS[7], async () => {
            const sample = samples[7];
            expect(style(sample, 'cem-css-fragment .sample-fragment', 'backgroundColor')).toBe('rgb(254, 243, 199)');
            expect(style(sample, 'cem-css-fragment .sample-fragment', 'borderTopColor')).toBe('rgb(180, 83, 9)');
            expect(managed(sample, 'cem-css-fragment', 'private')).toHaveLength(1);
            expect(managed(sample, 'cem-css-fragment')[0].textContent).toContain('cem-css-fragment');
            expect(sample.querySelector('cem-css-fragment style')).toBeNull();
        });
        await step(EXPECTED_LEGENDS[8], async () => {
            const sample = samples[8];
            const declaration = requiredElement(sample, 'cem-element[uid-seed="demo/css/anonymous"]');
            const tag = declaration.getAttribute('tag');
            if (!tag) throw new Error('anonymous declaration has no produced tag');
            const produced = requiredElement(sample, tag);
            expect(style(produced, '.sample-anonymous', 'color')).toBe('rgb(238, 130, 238)');
            expect(managed(sample, tag, 'private')).toHaveLength(1);
            expect(managed(sample, tag)[0].textContent).toContain(tag);
        });
        await step(EXPECTED_LEGENDS[9], async () => {
            const sample = samples[9];
            const produced = requiredElement(sample, 'cem-css-keyframes');
            const identity = produced.getAttribute('data-cem-render-scope');
            expect(identity).toContain('udemoz2fcssz2fkeyframes');
            const name = `seeded-pulse-${identity}-s1`;
            expect(managed(sample, 'cem-css-keyframes', 'private')).toHaveLength(1);
            expect(managed(sample, 'cem-css-keyframes')[0].textContent).toContain(`@keyframes ${name}`);
            expect(style(sample, '[part~="indicator"]', 'animationName')).toBe(name);
            expect(style(sample, '[part~="indicator"]', 'animationDuration')).toBe('0.8s');
            expect(style(sample, '[part~="indicator"]', 'animationIterationCount')).toBe('infinite');
            expect(requiredElement(sample, '[part~="indicator"]').getAnimations()).toHaveLength(1);
        });
        await step(EXPECTED_LEGENDS[10], async () => {
            const sample = samples[10];
            const input = requiredElement(sample, 'input') as HTMLInputElement;
            const bold = requiredElement(sample, '[slot="demo"] b');
            expect(input.checked).toBe(true);
            expect(getComputedStyle(bold).color).toBe('rgb(0, 0, 139)');
            expect(getComputedStyle(bold).textShadow).not.toBe('none');
            input.focus();
            await userEvent.keyboard(' ');
            expect(input.checked).toBe(false);
            expect(getComputedStyle(bold).color).toBe('rgb(0, 0, 255)');
            expect(getComputedStyle(bold).textShadow).toBe('none');
            expect(style(sample, 'label', 'color')).toBe('rgb(0, 128, 0)');
            await userEvent.keyboard(' ');
            expect(input.checked).toBe(true);
            expect(getComputedStyle(bold).color).toBe('rgb(0, 0, 139)');
            expect(getComputedStyle(bold).textShadow).not.toBe('none');
            expect(requiredElement(sample, 'input')).toBe(input);
            expect(document.activeElement).toBe(input);
        });
        await step(EXPECTED_LEGENDS[11], async () => {
            const sample = samples[11];
            const produced = requiredElement(sample, 'cem-css-external-fragment');
            expect(normalize(produced.textContent ?? '')).toContain('projected external template');
            expect(style(produced, '.external-scoped-item', 'backgroundColor')).toBe('rgb(254, 243, 199)');
            expect(style(produced, '.external-scoped-item', 'borderTopColor')).toBe('rgb(180, 83, 9)');
            expect(managed(sample, 'cem-css-external-fragment', 'private')).toHaveLength(1);
            expect(produced.querySelector('style')).toBeNull();
            expect((requiredElement(produced, 'a') as HTMLAnchorElement).href).toBe(new URL('./external-template-templates.html', DEMO_URL).href);
        });
        await step(EXPECTED_LEGENDS[12], async () => {
            const sample = samples[12];
            const instances = Array.from(sample.querySelectorAll<HTMLElement>('cem-css-resource'));
            expect(instances).toHaveLength(2);
            const ids = instances.map(instance => {
                const filter = requiredElement(instance, 'filter');
                const preview = requiredElement(instance, '[part~=preview]');
                expect(filter.namespaceURI).toBe('http://www.w3.org/2000/svg');
                expect(filter.id).toMatch(/^cem-instance-[0-9a-f-]+-filter$/);
                expect(document.getElementById(filter.id)).toBe(filter);
                expect(preview.style.getPropertyValue('--sample-filter')).toBe(`url("#${filter.id}")`);
                expect(getComputedStyle(preview).filter).toContain(`#${filter.id}`);
                expect(instance.hasAttribute('style')).toBe(false);
                return filter.id;
            });
            expect(new Set(ids).size).toBe(2);
            expect(instances.map(instance => requiredElement(instance, 'feGaussianBlur').getAttribute('stdDeviation')))
                .toEqual(['0', '2']);
            expect(managed(sample, 'cem-css-resource', 'private')).toHaveLength(1);
            instances[1].setAttribute('blur', '4');
            await waitFor(() => expect(requiredElement(instances[1], 'feGaussianBlur').getAttribute('stdDeviation')).toBe('4'));
            expect(requiredElement(instances[1], 'filter').id).toBe(ids[1]);
            const parent = instances[1].parentElement;
            if (!parent) throw new Error('Missing demo output parent');
            instances[1].remove();
            parent.append(instances[1]);
            await whenCemSourceRendered(host);
            expect(requiredElement(instances[1], 'filter').id).toBe(ids[1]);
            instances[1].setAttribute('blur', '2');
            await waitFor(() => expect(requiredElement(instances[1], 'feGaussianBlur').getAttribute('stdDeviation')).toBe('2'));
        });
        for (const relative of ['../index.html', './external-template.html', './hex-grid.html']) {
            expect(Array.from(host.querySelectorAll<HTMLAnchorElement>('nav a, main > section a'), link => link.href))
                .toContain(new URL(relative, DEMO_URL).href);
        }
        await whenCemSourceRendered(host);
        expect(cemDiagnosticCodes(host)).toEqual([]);
        for (const declaration of canvasElement.querySelectorAll<HTMLElement>('cem-element[tag]')) {
            const tag = declaration.getAttribute('tag');
            if (!tag) throw new Error('declaration has no produced tag');
            const expected = tag === 'cem-css-invalid-declaration' ? ['cem-element.stylesheet_scope_invalid']
                : ['cem-css-unscoped-explicit', 'cem-css-mismatch', 'cem-css-mismatch-bare'].includes(tag)
                    ? ['cem-element.stylesheet_scope_mismatch']
                    : tag === 'cem-css-dynamic' ? Array(2).fill('cem.ql.template.stylesheet_dynamic_unsupported') : [];
            expect(cemDiagnosticCodes(declaration)).toEqual(expected);
            for (const instance of canvasElement.querySelectorAll<HTMLElement>(tag)) {
                expect(cemDiagnosticCodes(instance)).toEqual(tag === 'cem-css-dynamic' ? expected : []);
            }
        }
    },
};

export const InstanceIdentityLifecycle: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const instances: HTMLElement[] = [];
        for (const lane of ['main-thread', 'worker'] as const) {
            const tag = `story-instance-id-${lane}`;
            let workerCalls = 0;
            const runtime = new CemElementRuntime({
                declarationTag: `story-instance-declaration-${lane}`,
                declarationScope: createCemDeclarationScope({ document }),
                processingWorkerFactory: lane === 'main-thread' ? () => {
                    throw new Error('Exercise the main-thread fallback');
                } : ({ scriptUrl, name, type }) => {
                    workerCalls += 1;
                    return new Worker(scriptUrl, { name, type });
                },
                processingPoolPolicy: { workerCount: 1, maxWorkers: 1 },
            });
            const declaration = document.createElement('div');
            declaration.setAttribute('tag', tag);
            declaration.setAttribute('version', '1.0.0');
            const template = document.createElement('template');
            template.type = 'text/cem-ml';
            template.textContent = '{attribute @name=label | Initial}{p @id="{$instanceID}-label" | {$label}}';
            declaration.append(template);
            runtime.registerDeclaration(declaration);
            const instance = document.createElement(tag);
            canvasElement.append(instance);
            await runtime.whenRenderSettled(instance);
            const snapshot = runtime.snapshotInstance(instance);
            expect(requiredElement(instance, 'p').id).toBe(`${snapshot.instanceId}-label`);
            expect(snapshot.instanceId).toMatch(/^cem-instance-[0-9a-f-]{36}$/);
            expect(instance.hasAttribute('style')).toBe(false);
            instances.push(instance);

            // Resume the serialized owner on a replacement host after removing the old one.
            // snapshotInstance advances the counter; this fixture resumes the retained DOM revision.
            const renderedRevision = requiredElement(instance, 'p').getAttribute('data-cem-data-revision');
            if (!renderedRevision) throw new Error('Missing rendered revision');
            snapshot.dataRevision = renderedRevision;
            const markup = instance.innerHTML;
            instance.remove();
            const restored = document.createElement(tag);
            for (const [name, value] of Object.entries(snapshot.hostAttributes)) restored.setAttribute(name, value);
            restored.innerHTML = markup;
            const island = requiredElement(restored, 'template[data-cem-island="instance"]') as HTMLTemplateElement;
            writeDataIslandHydrationData(island, snapshot);
            canvasElement.append(restored);
            await runtime.whenRenderSettled(restored);
            expect(runtime.snapshotInstance(restored).instanceId).toBe(snapshot.instanceId);
            expect(runtime.diagnosticsFor(restored)).toEqual([]);
            restored.setAttribute('label', 'Resumed');
            await waitFor(() => expect(requiredElement(restored, 'p').textContent).toBe('Resumed'));
            expect(requiredElement(restored, 'p').id).toBe(`${snapshot.instanceId}-label`);
            expect(runtime.diagnosticsFor(restored)).toEqual([]);
            if (lane === 'worker') expect(workerCalls).toBeGreaterThan(0);
        }
        expect(requiredElement(instances[0], 'p').id).not.toBe(requiredElement(instances[1], 'p').id);
    },
};

function managed(root: ParentNode, tag: string, kind?: string): HTMLStyleElement[] {
    return Array.from(root.querySelectorAll<HTMLStyleElement>(`cem-element[tag="${tag}"] > style[data-cem-declaration-style${kind ? `="${kind}"` : ''}]`));
}

function sourceLoadedDemo(): HTMLElement {
    const root = document.createElement('section');
    root.setAttribute('aria-label', 'source-loaded scoped CSS demo coverage');
    const declaration = document.createElement('cem-element');
    declaration.hidden = true;
    declaration.setAttribute('tag', SOURCE_TAG);
    declaration.setAttribute('src', DEMO_URL.href);
    declaration.setAttribute('link-base', 'source');
    root.append(declaration, document.createElement(SOURCE_TAG));
    return root;
}


function sampleLegends(host: ParentNode): string[] {
    return Array.from(host.querySelectorAll('cem-demo-element[legend]'), (sample) =>
        normalize(sample.getAttribute('legend') ?? '')
    );
}

function sampleByLegend(host: ParentNode, legend: string): HTMLElement {
    const sample = Array.from(host.querySelectorAll<HTMLElement>('cem-demo-element[legend]')).find(
        (candidate) => normalize(candidate.getAttribute('legend') ?? '') === legend
    );
    if (!sample) throw new Error(`expected sample ${legend}`);
    return sample;
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`expected ${selector}`);
    return element;
}

function style(root: ParentNode, selector: string, property: keyof CSSStyleDeclaration): string {
    return String(getComputedStyle(requiredElement(root, selector))[property]);
}

async function waitForCondition(condition: () => boolean, message: string): Promise<void> {
    await waitFor(() => {
        if (!condition()) throw new Error(message);
    }, { timeout: 30000 });
}

function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}
