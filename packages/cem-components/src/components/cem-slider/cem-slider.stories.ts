import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-slider.xhtml?raw';
const meta = preview.meta({ component: 'cem-slider', title: 'CEM Components/cem-slider', loaders: [async () => { await loadCemDeclaration('cem-slider', declarationSource); return {}; }] });

export const NativeOwners = meta.story({
    name: "keeps exact native single and range inputs as the accessible form owners",
    render: () => sliderContractFixture,
    parameters: { docs: { description: { story: 'Browser-runner-only: trusted native pointer and keyboard assertions.' } } },
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        expect(sliderContractFixture).not.toMatch(/<script\b/i);
        expect(sliderContractFixture).not.toMatch(/\son[a-z]+\s*=/i);

        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-slider')) await whenCemRendered(host);
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
        expect(volume).toHaveAccessibleName('Volume');
        expect(start).toHaveAccessibleName('Minimum price');
        expect(end).toHaveAccessibleName('Maximum price');
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
    },
});

export const NativeKeyboardAndRange = meta.story({
    name: "uses native keyboard events and clamps each range thumb without redispatch",
    render: () => sliderContractFixture,
    parameters: { docs: { description: { story: 'Browser-runner-only: trusted native pointer and keyboard assertions.' } } },
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-slider')) await whenCemRendered(host);
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
    },
});

export const FormsAndIdentity = meta.story({
    name: "serializes independent native values and preserves input identity through live parent control",
    render: () => sliderContractFixture,
    parameters: { docs: { description: { story: 'Browser-runner-only: trusted native pointer and keyboard assertions.' } } },
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-slider')) await whenCemRendered(host);
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
    },
});

export const PointerStateAndGeometry = meta.story({
    name: "keeps pointer enter/leave, hover, active, and focus-visible on the native thumb with stable state",
    render: () => sliderContractFixture,
    parameters: { docs: { description: { story: 'Browser-runner-only: trusted native pointer and keyboard assertions.' } } },
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-slider')) await whenCemRendered(host);
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
    },
});

export const DisabledOwners = meta.story({
    name: "projects global disabled suppression and skips the disabled native owner",
    render: () => sliderContractFixture,
    parameters: { docs: { description: { story: 'Browser-runner-only: trusted native pointer and keyboard assertions.' } } },
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-slider')) await whenCemRendered(host);
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
    },
});

export const VisualValues = meta.story({
    name: "mirrors normalized positions, ticks, and discrete labels without accessible duplication",
    render: () => sliderContractFixture,
    parameters: { docs: { description: { story: 'Browser-runner-only: trusted native pointer and keyboard assertions.' } } },
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-slider')) await whenCemRendered(host);
        const single = sliderParts(root, '#single-slider');
        const range = sliderParts(root, '#range-slider');
        expect(single.owner.style.getPropertyValue('--_cem-range-start-position')).toBe('0%');
        expect(single.owner.style.getPropertyValue('--_cem-range-end-position')).toBe('50%');
        expect(single.owner.style.getPropertyValue('--_cem-range-tick-spacing')).toBe('5%');
        expect(valueLabel(single, 'single').textContent).toBe('50 percent');
        expect(getComputedStyle(valueLabel(single, 'single')).display).toBe('block');
        expect(getComputedStyle(single.ticks).display).toBe('block');

        expect(range.owner.style.getPropertyValue('--_cem-range-start-position')).toBe('25%');
        expect(range.owner.style.getPropertyValue('--_cem-range-end-position')).toBe('75%');
        expect(valueLabel(range, 'start').textContent).toBe('25');
        expect(valueLabel(range, 'end').textContent).toBe('75');
        expect(getComputedStyle(valueLabel(range, 'single')).display).toBe('none');
        expect(single.visual.getAttribute('aria-hidden')).toBe('true');

        const input = requiredInput(single, 'single');
        input.value = '60';
        input.setAttribute('aria-valuetext', 'Sixty percent');
        input.dispatchEvent(new InputEvent('input', { bubbles: true }));
        expect(single.owner.style.getPropertyValue('--_cem-range-end-position')).toBe('60%');
        expect(valueLabel(single, 'single').textContent).toBe('Sixty percent');
    },
});

export const SuperCompactGeometry = meta.story({
    name: 'preserves intrinsic thumbs and native range input when the page coupling minimum is zero',
    parameters: { docs: { description: { story: 'Geometry and hit-target checks run in Storybook; trusted ArrowRight event checks require the browser runner.' } } },
    render: () => '<section data-theme="cem-theme-light" data-cem-density="super-compact" data-cem-spacing="normal">'
        + '<cem-slider min="0" max="100" step="5" discrete show-tick-marks><input type="range" name="volume" data-cem-slider-thumb="single" value="50" aria-label="Volume"></cem-slider>'
        + '<cem-slider min="0" max="100" step="5" discrete show-tick-marks><input type="range" name="minimum" data-cem-slider-thumb="start" value="25" aria-label="Minimum price"><input type="range" name="maximum" data-cem-slider-thumb="end" value="75" aria-label="Maximum price"></cem-slider>'
        + '</section>',
    play: async ({ canvasElement }) => {
        const boundary = requiredElement<HTMLElement>(canvasElement, '[data-cem-density]');
        for (const host of boundary.querySelectorAll<HTMLElement>('cem-slider')) await whenCemRendered(host);
        const sliders = [sliderParts(boundary, 'cem-slider:has(input[name=volume])'), sliderParts(boundary, 'cem-slider:has(input[name=minimum])')];
        const originals = sliders.flatMap(parts => parts.inputs);
        const rem = Number.parseFloat(getComputedStyle(document.documentElement).fontSize);
        const expectGeometry = (extent: number) => {
            for (const parts of sliders) {
                const visual = parts.visual.getBoundingClientRect();
                expect(parts.owner.getBoundingClientRect().height).toBeGreaterThanOrEqual(extent);
                expect(Math.abs(visual.height - extent)).toBeLessThan(0.1);
                for (const layer of [parts.inputsOwner, ...parts.inputs]) {
                    const rect = layer.getBoundingClientRect();
                    expect(Math.abs(rect.height - extent)).toBeLessThan(0.1);
                    expect(Math.abs(rect.top - visual.top)).toBeLessThan(0.1);
                }
                for (const track of parts.visual.querySelectorAll<HTMLElement>('[part~=track], [part~=active-track], [part~=ticks]')) {
                    const rect = track.getBoundingClientRect();
                    expect(Math.abs(rect.height - 0.25 * rem)).toBeLessThan(0.1);
                    expect(Math.abs(rect.top + rect.height / 2 - visual.top - visual.height / 2)).toBeLessThan(0.1);
                }
                for (const input of parts.inputs) {
                    const rect = input.getBoundingClientRect();
                    const fraction = (input.valueAsNumber - Number(input.min)) / (Number(input.max) - Number(input.min));
                    const centerX = rect.left + extent / 2 + (rect.width - extent) * fraction;
                    const centerY = rect.top + rect.height / 2;
                    // The native input is pointer-events:none; only its thumb can own these points.
                    expect(document.elementFromPoint(centerX, centerY)).toBe(input);
                    expect(document.elementFromPoint(centerX + extent / 4, centerY)).toBe(input);
                    expect(valueLabel(parts, input.dataset.cemSliderThumb as 'single' | 'start' | 'end').getBoundingClientRect().bottom).toBeLessThan(visual.top);
                }
            }
        };
        for (const theme of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            boundary.setAttribute('data-theme', `cem-theme-${theme}`); await nextRenderFrame();
            expect(getComputedStyle(sliders[0].host).getPropertyValue('--cem-coupling-zone-min').trim()).toBe('0px');
            expect(getComputedStyle(sliders[0].host).getPropertyValue('--cem-slider-thumb-size').trim()).toBe('1.25rem');
            expectGeometry(1.25 * rem);
        }
        boundary.setAttribute('data-cem-density', 'normal'); await nextRenderFrame();
        expectGeometry(3 * rem);
        boundary.setAttribute('data-cem-density', 'super-compact');
        boundary.style.setProperty('--cem-coupling-zone-min', '60px'); await nextRenderFrame();
        expectGeometry(60);
        boundary.style.removeProperty('--cem-coupling-zone-min');
        boundary.style.setProperty('--cem-slider-thumb-size', '32px'); await nextRenderFrame();
        expectGeometry(32);
        boundary.style.removeProperty('--cem-slider-thumb-size'); await nextRenderFrame();
        expectGeometry(1.25 * rem);
        expect(sliders.flatMap(parts => parts.inputs)).toEqual(originals);

        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const events: Array<{ type: string; target: EventTarget | null; trusted: boolean }> = [];
        for (const input of originals) {
            for (const type of ['input', 'change']) input.addEventListener(type, event => events.push({ type: event.type, target: event.target, trusted: event.isTrusted }));
        }
        for (const input of originals) {
            const before = input.valueAsNumber;
            events.length = 0; input.focus(); await userEvent.keyboard('{ArrowRight}');
            expect(input.valueAsNumber).toBe(before + 5);
            expect(events).toEqual([{ type: 'input', target: input, trusted: true }, { type: 'change', target: input, trusted: true }]);
        }
        expectGeometry(1.25 * rem);
    },
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

async function nextRenderFrame(): Promise<void> {
    await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
}
function assertAriaReferenceIntegrity(root: ParentNode): void {
    for (const node of root.querySelectorAll('[aria-controls], [aria-labelledby], [aria-describedby]')) {
        for (const name of ['aria-controls', 'aria-labelledby', 'aria-describedby']) {
            for (const id of (node.getAttribute(name) ?? '').split(/\s+/).filter(Boolean)) {
                if (!node.ownerDocument.getElementById(id)) throw new Error(`Missing ARIA reference ${id}`);
            }
        }
    }
}

const sliderContractFixture = "<section class=\"cem-theme-light\" data-slider-contract-fixture aria-label=\"Slider contract fixture\">\n  <button data-slider-focus-start type=\"button\">Focus start</button>\n\n  <form id=\"slider-form\">\n    <label id=\"volume-label\">Volume</label>\n    <cem-slider id=\"single-slider\" min=\"0\" max=\"100\" step=\"5\" discrete show-tick-marks>\n      <input\n        id=\"volume-thumb\"\n        type=\"range\"\n        data-cem-slider-thumb=\"single\"\n        name=\"volume\"\n        value=\"50\"\n        aria-labelledby=\"volume-label\"\n        aria-valuetext=\"50 percent\"\n      >\n    </cem-slider>\n\n    <span id=\"price-label\">Price range</span>\n    <cem-slider id=\"range-slider\" min=\"0\" max=\"100\" step=\"5\" discrete show-tick-marks>\n      <input\n        id=\"price-start-thumb\"\n        type=\"range\"\n        data-cem-slider-thumb=\"start\"\n        name=\"minimum\"\n        value=\"25\"\n        aria-label=\"Minimum price\"\n        aria-describedby=\"price-label\"\n      >\n      <input\n        id=\"price-end-thumb\"\n        type=\"range\"\n        data-cem-slider-thumb=\"end\"\n        name=\"maximum\"\n        value=\"75\"\n        aria-label=\"Maximum price\"\n        aria-describedby=\"price-label\"\n      >\n    </cem-slider>\n\n    <cem-slider id=\"normalized-slider\" min=\"10\" max=\"bad\" step=\"0\">\n      <input type=\"range\" data-cem-slider-thumb=\"single\" name=\"normalized\" value=\"35\" aria-label=\"Normalized value\">\n    </cem-slider>\n\n    <cem-slider id=\"disabled-slider\" min=\"0\" max=\"10\" step=\"1\" disabled>\n      <input type=\"range\" data-cem-slider-thumb=\"single\" name=\"disabled-value\" value=\"5\" aria-label=\"Disabled value\">\n    </cem-slider>\n  </form>\n\n  <button data-slider-focus-end type=\"button\">Focus end</button>\n</section>\n";
