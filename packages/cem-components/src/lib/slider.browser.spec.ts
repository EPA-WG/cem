import '@epa-wg/cem-theme/styles.css';
import { CemElementRuntime } from '@epa-wg/cem-elements';
import { userEvent } from 'vitest/browser';

import '../styles.css';
import sliderContractFixture from '../../tests/slider/contract.html?raw';
import { installCemComponentPrimitives } from './primitives.js';
import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    createComponentHarness,
    nextRenderFrame,
    type ComponentHarness,
} from './testing/component-harness.js';

describe('slider contract fixture', () => {
    let harness: ComponentHarness;
    let runtime: CemElementRuntime;

    beforeAll(() => {
        runtime = new CemElementRuntime({ declarationTag: 'cem-components-slider-declaration' });
        const result = installCemComponentPrimitives(runtime);
        expect(result.diagnostics).toEqual([]);
    });

    afterEach(() => {
        harness?.cleanup();
    });

    it('keeps exact native single and range inputs as the accessible form owners', async () => {
        expect(sliderContractFixture).not.toMatch(/<script\b/i);
        expect(sliderContractFixture).not.toMatch(/\son[a-z]+\s*=/i);

        const root = await renderFixture();
        const single = sliderParts(root, '#single-slider');
        const range = sliderParts(root, '#range-slider');
        const normalized = sliderParts(root, '#normalized-slider');

        expect(single.owner.dataset.mode).toBe('single');
        expect(single.inputs).toHaveLength(1);
        expect(range.owner.dataset.mode).toBe('range');
        expect(range.inputs.map((input) => input.dataset.cemSliderThumb)).toEqual(['start', 'end']);
        expect(single.inputs[0]?.parentElement).toBe(single.inputsOwner);
        expect(range.inputs.every((input) => input.parentElement === range.inputsOwner)).toBe(true);
        expect(root.querySelector('cem-slider-thumb')).toBeNull();
        expect(single.visual.getAttribute('aria-hidden')).toBe('true');
        expect(single.visual.querySelector('[role], input, button, [tabindex]')).toBeNull();

        const volume = requiredInput(single, 'single');
        const start = requiredInput(range, 'start');
        const end = requiredInput(range, 'end');
        expect(assertAccessibleName(volume, 'Volume')).toBe('Volume');
        expect(assertAccessibleName(start, 'Minimum price')).toBe('Minimum price');
        expect(assertAccessibleName(end, 'Maximum price')).toBe('Maximum price');
        expect([volume.min, volume.max, volume.step, volume.value]).toEqual(['0', '100', '5', '50']);
        expect([start.min, start.max, start.step, start.value]).toEqual(['0', '100', '5', '25']);
        expect([end.min, end.max, end.step, end.value]).toEqual(['0', '100', '5', '75']);
        expect([normalized.inputs[0]?.min, normalized.inputs[0]?.max, normalized.inputs[0]?.step]).toEqual([
            '10',
            '110',
            '1',
        ]);

        const form = requiredElement<HTMLFormElement>(root, '#slider-form');
        expect([...new FormData(form).entries()]).toEqual([
            ['volume', '50'],
            ['minimum', '25'],
            ['maximum', '75'],
            ['normalized', '35'],
        ]);
        expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
    });

    it('uses native keyboard events and clamps each range thumb without redispatch', async () => {
        const root = await renderFixture();
        const range = sliderParts(root, '#range-slider');
        const start = requiredInput(range, 'start');
        const end = requiredInput(range, 'end');
        const events: Array<{ target: string; trusted: boolean; type: string; value: string }> = [];
        for (const eventName of ['input', 'change', 'cem-slider-change']) {
            range.host.addEventListener(eventName, (event) => {
                const target = event.target as HTMLInputElement;
                events.push({ target: target.id, trusted: event.isTrusted, type: event.type, value: target.value });
            });
        }

        start.focus();
        await userEvent.keyboard('{ArrowRight}');
        expect(start.value).toBe('30');
        expect(events.some((event) => event.type === 'input' && event.target === start.id && event.trusted)).toBe(true);
        expect(events.some((event) => event.type === 'change' && event.target === start.id && event.trusted)).toBe(true);
        expect(events.some((event) => event.type === 'cem-slider-change')).toBe(false);

        const beforePageUp = start.valueAsNumber;
        await userEvent.keyboard('{PageUp}');
        expect(start.valueAsNumber).toBeGreaterThan(beforePageUp);
        await userEvent.keyboard('{PageDown}');
        expect(start.valueAsNumber).toBe(beforePageUp);

        await userEvent.keyboard('{End}');
        expect(start.value).toBe('75');
        expect(end.value).toBe('75');
        end.focus();
        await userEvent.keyboard('{Home}');
        expect(start.value).toBe('75');
        expect(end.value).toBe('75');

        end.value = '80';
        end.dispatchEvent(new InputEvent('input', { bubbles: true }));
        expect(end.value).toBe('80');
        start.value = '90';
        start.dispatchEvent(new InputEvent('input', { bubbles: true }));
        expect(start.value).toBe('80');
        expect(events.at(-1)).toEqual({ target: start.id, trusted: false, type: 'input', value: '80' });
    });

    it('serializes independent native values and preserves input identity through live parent control', async () => {
        const root = await renderFixture();
        const range = sliderParts(root, '#range-slider');
        const start = requiredInput(range, 'start');
        const end = requiredInput(range, 'end');
        const owner = range.owner;
        const events: string[] = [];
        range.host.addEventListener('input', (event) => events.push(`${event.type}:${event.isTrusted}`));
        range.host.addEventListener('change', (event) => events.push(`${event.type}:${event.isTrusted}`));

        range.host.setAttribute('min', '20');
        range.host.setAttribute('max', '80');
        range.host.setAttribute('step', '10');
        await waitForInputBounds(start, '20', '80', '10');
        expect(requiredInput(sliderParts(root, '#range-slider'), 'start')).toBe(start);
        expect(requiredInput(sliderParts(root, '#range-slider'), 'end')).toBe(end);
        expect(sliderParts(root, '#range-slider').owner).toBe(owner);
        expect(events).toEqual([]);

        range.host.setAttribute('disabled', '');
        await waitForDisabled(start, true);
        expect(end.disabled).toBe(true);
        range.host.removeAttribute('disabled');
        await waitForDisabled(start, false);
        expect(end.disabled).toBe(false);
        expect(events).toEqual([]);

        start.value = '30';
        end.value = '70';
        const form = requiredElement<HTMLFormElement>(root, '#slider-form');
        expect([...new FormData(form).entries()].slice(1, 3)).toEqual([
            ['minimum', '30'],
            ['maximum', '70'],
        ]);
    });

    it('keeps pointer enter/leave, hover, active, and focus-visible on the native thumb with stable state', async () => {
        const root = await renderFixture();
        const single = sliderParts(root, '#single-slider');
        const input = requiredInput(single, 'single');
        const originalOwnerHtml = single.owner.innerHTML;
        const originalHostAttributes = attributes(single.host);
        const originalInputAttributes = attributes(input);
        const originalGeometry = geometry(single.owner);
        const events: string[] = [];
        const mutations: MutationRecord[] = [];
        for (const eventName of ['pointerenter', 'pointerleave', 'input', 'change']) {
            input.addEventListener(eventName, (event) => events.push(`${event.type}:${event.isTrusted}`));
        }
        const observer = new MutationObserver((records) => mutations.push(...records));
        observer.observe(single.host, { attributes: true, childList: true, subtree: true });

        await userEvent.hover(input);
        expect(input.matches(':hover')).toBe(true);
        expect(events).toContain('pointerenter:true');
        expect(geometry(single.owner)).toEqual(originalGeometry);

        const pointerDown = nextTrustedPointerDown(input);
        const click = userEvent.click(input, { delay: 200 });
        const downEvent = await eventBeforeInteractionCompletes(pointerDown, click, 'slider pointerdown');
        expect(downEvent.isTrusted).toBe(true);
        await waitForPseudoClass(input, ':active');
        expect(input.matches(':active')).toBe(true);
        expect(geometry(single.owner)).toEqual(originalGeometry);
        await click;
        await userEvent.unhover(input);
        expect(events).toContain('pointerleave:true');

        requiredElement<HTMLButtonElement>(root, '[data-slider-focus-start]').focus();
        await userEvent.keyboard('{Tab}');
        expect(document.activeElement).toBe(input);
        expect(input.matches(':focus-visible')).toBe(true);
        expect(geometry(single.owner)).toEqual(originalGeometry);
        await userEvent.keyboard('{Tab}');
        expect(document.activeElement).toBe(requiredInput(sliderParts(root, '#range-slider'), 'start'));

        await nextRenderFrame();
        observer.disconnect();
        expect(events.filter((event) => /^(input|change):/.test(event))).toEqual([]);
        expect(mutations).toEqual([]);
        expect(single.owner.innerHTML).toBe(originalOwnerHtml);
        expect(attributes(single.host)).toEqual(originalHostAttributes);
        expect(attributes(input)).toEqual(originalInputAttributes);
        expect(input.value).toBe('50');
    });

    it('projects global disabled suppression and skips the disabled native owner', async () => {
        const root = await renderFixture();
        const disabled = sliderParts(root, '#disabled-slider');
        const input = requiredInput(disabled, 'single');
        const events: string[] = [];
        input.addEventListener('input', (event) => events.push(event.type));
        input.addEventListener('change', (event) => events.push(event.type));
        expect(input.disabled).toBe(true);

        input.click();
        await nextRenderFrame();
        expect(input.value).toBe('5');
        expect(events).toEqual([]);

        requiredElement<HTMLButtonElement>(root, '[data-slider-focus-start]').focus();
        await userEvent.keyboard('{Tab}{Tab}{Tab}{Tab}{Tab}');
        expect(document.activeElement).toBe(requiredElement<HTMLButtonElement>(root, '[data-slider-focus-end]'));
        expect(document.activeElement).not.toBe(input);
    });

    it('mirrors normalized positions, ticks, and discrete labels without accessible duplication', async () => {
        const root = await renderFixture();
        const single = sliderParts(root, '#single-slider');
        const range = sliderParts(root, '#range-slider');
        expect(single.owner.style.getPropertyValue('--_cem-slider-start-position')).toBe('0%');
        expect(single.owner.style.getPropertyValue('--_cem-slider-end-position')).toBe('50%');
        expect(single.owner.style.getPropertyValue('--_cem-slider-tick-spacing')).toBe('5%');
        expect(valueLabel(single, 'single').textContent).toBe('50 percent');
        expect(getComputedStyle(valueLabel(single, 'single')).display).toBe('block');
        expect(getComputedStyle(single.ticks).display).toBe('block');

        expect(range.owner.style.getPropertyValue('--_cem-slider-start-position')).toBe('25%');
        expect(range.owner.style.getPropertyValue('--_cem-slider-end-position')).toBe('75%');
        expect(valueLabel(range, 'start').textContent).toBe('25');
        expect(valueLabel(range, 'end').textContent).toBe('75');
        expect(getComputedStyle(valueLabel(range, 'single')).display).toBe('none');
        expect(single.visual.getAttribute('aria-hidden')).toBe('true');

        const input = requiredInput(single, 'single');
        input.value = '60';
        input.setAttribute('aria-valuetext', 'Sixty percent');
        input.dispatchEvent(new InputEvent('input', { bubbles: true }));
        expect(single.owner.style.getPropertyValue('--_cem-slider-end-position')).toBe('60%');
        expect(valueLabel(single, 'single').textContent).toBe('Sixty percent');
    });

    async function renderFixture(): Promise<HTMLElement> {
        harness = createComponentHarness(runtime);
        const root = await harness.render(sliderContractFixture);
        await waitForSelector(root, '#single-slider > .cem-slider > .cem-slider__inputs > input[type="range"]');
        return root;
    }
});

interface SliderParts {
    host: HTMLElement;
    inputs: HTMLInputElement[];
    inputsOwner: HTMLElement;
    owner: HTMLElement;
    ticks: HTMLElement;
    visual: HTMLElement;
}

function sliderParts(root: ParentNode, selector: string): SliderParts {
    const host = requiredElement<HTMLElement>(root, selector);
    const owner = requiredElement<HTMLElement>(host, ':scope > .cem-slider');
    const inputsOwner = requiredElement<HTMLElement>(owner, ':scope > .cem-slider__inputs');
    return {
        host,
        inputs: [...inputsOwner.querySelectorAll<HTMLInputElement>(':scope > input[type="range"]')],
        inputsOwner,
        owner,
        ticks: requiredElement<HTMLElement>(owner, ':scope > .cem-slider__visual > .cem-slider__ticks'),
        visual: requiredElement<HTMLElement>(owner, ':scope > .cem-slider__visual'),
    };
}

function requiredInput(parts: SliderParts, thumb: 'end' | 'single' | 'start'): HTMLInputElement {
    const input = parts.inputs.find((candidate) => candidate.dataset.cemSliderThumb === thumb);
    if (!input) throw new Error(`Missing ${thumb} slider input`);
    return input;
}

function valueLabel(parts: SliderParts, thumb: 'end' | 'single' | 'start'): HTMLElement {
    return requiredElement(parts.visual, `[data-cem-slider-value="${thumb}"]`);
}

function geometry(element: Element): { height: number; width: number; x: number; y: number } {
    const rect = element.getBoundingClientRect();
    return { height: rect.height, width: rect.width, x: rect.x, y: rect.y };
}

function attributes(element: Element): Record<string, string> {
    return Object.fromEntries([...element.attributes].map((attribute) => [attribute.name, attribute.value]));
}

function requiredElement<T extends Element>(root: ParentNode, selector: string): T {
    const element = root.querySelector<T>(selector);
    if (!element) throw new Error(`Missing required fixture element: ${selector}`);
    return element;
}

async function waitForSelector(root: ParentNode, selector: string): Promise<void> {
    for (let attempt = 0; attempt < 80; attempt += 1) {
        if (root.querySelector(selector)) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for selector: ${selector}`);
}

async function waitForInputBounds(input: HTMLInputElement, min: string, max: string, step: string): Promise<void> {
    for (let attempt = 0; attempt < 80; attempt += 1) {
        if (input.min === min && input.max === max && input.step === step) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for slider bounds ${min}/${max}/${step}`);
}

async function waitForDisabled(input: HTMLInputElement, disabled: boolean): Promise<void> {
    for (let attempt = 0; attempt < 80; attempt += 1) {
        if (input.disabled === disabled) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for slider disabled=${String(disabled)}`);
}

function nextTrustedPointerDown(input: HTMLInputElement): Promise<PointerEvent> {
    return new Promise((resolve, reject) => {
        const timeout = window.setTimeout(() => {
            input.removeEventListener('pointerdown', onPointerDown);
            reject(new Error('Expected a trusted pointerdown before the slider interaction completed'));
        }, 1000);
        const onPointerDown = (event: PointerEvent): void => {
            window.clearTimeout(timeout);
            resolve(event);
        };
        input.addEventListener('pointerdown', onPointerDown, { once: true });
    });
}

async function eventBeforeInteractionCompletes<T extends Event>(
    event: Promise<T>,
    interaction: Promise<void>,
    label: string,
): Promise<T> {
    return Promise.race([
        event,
        interaction.then(() => {
            throw new Error(`Expected ${label} while the provider interaction was still held`);
        }),
    ]);
}

async function waitForPseudoClass(element: Element, pseudoClass: string): Promise<void> {
    const deadline = Date.now() + 1000;
    while (Date.now() < deadline) {
        if (element.matches(pseudoClass)) return;
        await nextRenderFrame();
    }
    throw new Error(`Expected ${element.tagName.toLowerCase()} to match ${pseudoClass}`);
}
