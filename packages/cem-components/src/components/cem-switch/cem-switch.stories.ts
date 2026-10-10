import { expect, userEvent } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, createCemStoryRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-switch.xhtml?raw';

const meta = preview.meta({ component: 'cem-switch', title: 'CEM Components/cem-switch', loaders: [async () => { await loadCemDeclaration('cem-switch', declarationSource); return {}; }] });
function required<T extends HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node;
}

function expectIndicator(input: HTMLInputElement, appearance: string, anchorToken = '--cem-stroke-none'): void {
    const measure = (token: string) => {
        const probe = document.createElement('span'); probe.style.cssText = `position:absolute;inline-size:var(${token})`;
        input.parentElement?.append(probe); const size = parseFloat(getComputedStyle(probe).inlineSize); probe.remove(); return size;
    };
    const anchor = measure(anchorToken), stripe = measure('--cem-zebra-strip-size');
    const selection = input.type !== 'radio' && (input.checked || input.indeterminate) ? stripe : 0;
    const cumulative = [anchor, anchor + stripe, anchor + stripe + selection];
    const shadow = getComputedStyle(input, input.type === 'radio' ? '::before' : null).boxShadow;
    const layers = shadow.split(/,(?![^()]*\))/);
    expect(layers).toHaveLength(3);
    for (const [index, layer] of layers.entries()) {
        const lengths = [...layer.matchAll(/(-?\d*\.?\d+)px/g)].map(match => Number(match[1])).slice(-4);
        expect(lengths).toEqual(appearance === 'underline' ? [0, cumulative[index], 0, 0] : [0, 0, 0, cumulative[index]]);
    }
    const colors = layers.map(layer => layer.match(/(?:rgba?|color)\([^)]*\)/)?.[0]);
    const computed = getComputedStyle(input);
    const tokens = ['--_cem-input-indicator-anchor-color', '--_cem-input-indicator-focus-color', '--_cem-input-indicator-selection-color'];
    for (const [index, token] of tokens.entries()) {
        const probe = document.createElement('span'); probe.style.color = computed.getPropertyValue(token); input.parentElement?.append(probe);
        expect(colors[index]).toBe(getComputedStyle(probe).color); probe.remove();
    }
}

export const NamesAndPresence = meta.story({
    render: () => '<section class="cem-theme-light"><cem-switch></cem-switch><cem-switch label="Fallback" required aria-describedby="cem-switch-help"><strong>Projected choice</strong><em slot="required-marker">Required</em></cem-switch><p id="cem-switch-help">Choose carefully.</p><p id="cem-switch-name">Referenced choice</p></section>',
    play: async ({ canvasElement }) => {
        const hosts = [...canvasElement.querySelectorAll<HTMLElement>('cem-switch')];
        for (const host of hosts) await whenCemRendered(host);
        expect(required(hosts[0], 'input')).toHaveAccessibleName('Switch');
        const host = hosts[1], input = required<HTMLInputElement>(host, 'input');
        expect(input.type).toBe('checkbox'); expect(input.getAttribute('role')).toBe('switch');
        expect(input).toHaveAccessibleName('Projected choice'); expect(input).toHaveAccessibleDescription('Choose carefully.');
        expect(required(host, '[part=label] strong').textContent).toBe('Projected choice');
        expect(required(host, '[part=required-marker]').getAttribute('aria-hidden')).toBe('true');
        expect(required(host, '[part=required-marker] em').textContent).toBe('Required');
        host.setAttribute('aria-label', 'Accessible choice'); await whenCemRendered(host); expect(input).toHaveAccessibleName('Accessible choice');
        host.setAttribute('aria-labelledby', 'cem-switch-name'); await whenCemRendered(host); expect(input).toHaveAccessibleName('Referenced choice');
        host.removeAttribute('aria-labelledby'); host.removeAttribute('aria-label');
        for (const value of ['', 'false', 'true']) {
            for (const name of ['checked', 'disabled', 'required', 'busy']) host.setAttribute(name, value);
            host.setAttribute('invalid', 'true'); await whenCemRendered(host);
            expect(input.checked).toBe(true); expect(input.disabled).toBe(true); expect(input.required).toBe(true);
            expect(input.getAttribute('aria-invalid')).toBe('true'); expect(input.getAttribute('aria-busy')).toBe('true'); expect(input.getAttribute('data-state')).toBe('loading');
            expect(input).toHaveAccessibleName('Projected choice');
            for (const name of ['checked', 'disabled', 'required', 'busy', 'invalid']) host.removeAttribute(name);
            await whenCemRendered(host); expect(input.checked).toBe(false); expect(input.disabled).toBe(false); expect(input.required).toBe(false);
            expect(input.hasAttribute('aria-busy')).toBe(false); expect(input.hasAttribute('data-state')).toBe(false); expect(host.querySelector('[part=required-marker]')).toBeNull();
        }
        expect(input.getAttribute('value')).toBe('on'); expect(host.hasAttribute('tabindex')).toBe(false); expect(host.querySelector('style')).toBeNull();
    },
});

export const NativeFormsWorkerAndFallback = meta.story({
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = required(canvasElement, 'section');
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(root, fallback);
            const tag = `cem-switch-contract-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
            const form = document.createElement('form'), external = document.createElement('form'); external.id = `external-${crypto.randomUUID()}`;
            form.innerHTML = `<fieldset><${tag} name="choice" value="yes" checked required label="Choice"></${tag}></fieldset><button type="reset">Reset</button>`;
            root.append(form, external); const host = required(form, tag);
            try {
                await runtime.whenRenderSettled(host); const input = required<HTMLInputElement>(host, 'input'), fieldset = required<HTMLFieldSetElement>(form, 'fieldset');
                expect(input.form).toBe(form); expect([...form.elements]).not.toContain(host); expect(new FormData(form).getAll('choice')).toEqual(['yes']);
                expect(form.checkValidity()).toBe(true); input.checked = false;
                host.setAttribute('label', 'Renamed'); await runtime.whenRenderSettled(host);
                expect(input.checked).toBe(false); expect(form.checkValidity()).toBe(false); expect(new FormData(form).getAll('choice')).toEqual([]);
                expect(input.defaultChecked).toBe(true); expect(required(host, 'input')).toBe(input);
                const cancel = (event: Event) => event.preventDefault(); form.addEventListener('reset', cancel); form.reset(); expect(input.checked).toBe(false);
                form.removeEventListener('reset', cancel); await userEvent.click(required(form, 'button')); expect(input.checked).toBe(true);
                fieldset.disabled = true; expect(input.matches(':disabled')).toBe(true); expect(new FormData(form).getAll('choice')).toEqual([]); fieldset.disabled = false;
                host.setAttribute('form', ''); await runtime.whenRenderSettled(host); expect(input.form).toBeNull(); expect(new FormData(form).getAll('choice')).toEqual([]);
                host.setAttribute('form', external.id); await runtime.whenRenderSettled(host);
                expect(input.form).toBe(external); expect(new FormData(form).getAll('choice')).toEqual([]); expect(new FormData(external).getAll('choice')).toEqual(['yes']);
                input.checked = false; external.reset(); expect(input.checked).toBe(true);
                host.setAttribute('value', 'updated'); await runtime.whenRenderSettled(host); expect(new FormData(external).getAll('choice')).toEqual(['updated']);
                host.setAttribute('value', ''); await runtime.whenRenderSettled(host); expect(new FormData(external).getAll('choice')).toEqual(['']);
                host.removeAttribute('checked'); await runtime.whenRenderSettled(host); expect(input.checked).toBe(false); expect(input.defaultChecked).toBe(false);
                host.setAttribute('checked', ''); await runtime.whenRenderSettled(host); expect(input.checked).toBe(true);
                host.removeAttribute('name'); await runtime.whenRenderSettled(host); expect([...new FormData(external)]).toEqual([]);
                host.remove(); fieldset.append(host); await runtime.whenRenderSettled(host); expect(required(host, 'input')).toBe(input); expect(input.checked).toBe(true);
                expect(runtime.diagnosticsFor(host).map(({ code, message }) => `${code}: ${message}`)).toEqual([]);
            } finally { form.remove(); external.remove(); declaration.remove(); scope.dispose(); }
        }
    },
});

export const NativeKeyboardAndIndicators = meta.story({
    render: () => '<section class="cem-theme-light"><button type="button">Before</button><cem-switch label="Choice"></cem-switch><cem-switch label="Unavailable" disabled></cem-switch><button type="button">After</button></section>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent: native } = await import('vitest/browser');
        const host = required<HTMLElement>(canvasElement, 'cem-switch'), section = required(canvasElement, 'section'); await whenCemRendered(host);
        const input = required<HTMLInputElement>(host, 'input');
        required(canvasElement, 'button').focus(); await native.keyboard('{Tab}'); expect(document.activeElement).toBe(input);
        expect(input.matches(':focus-visible')).toBe(true);
        await native.keyboard('{Enter}'); expect(input.checked).toBe(false);
        await native.keyboard(' '); expect(input.checked).toBe(true);
        await native.keyboard('{Tab}'); expect(document.activeElement).toBe(required(canvasElement, 'button:last-child'));
        const events: string[] = []; const record = (event: Event) => events.push(event.type);
        host.addEventListener('input', record); host.addEventListener('change', record);
        for (const theme of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            section.className = `cem-theme-${theme}`;
            for (const appearance of ['outline', 'underline', 'unsupported']) {
                host.setAttribute('indicator', appearance); await whenCemRendered(host);
                input.focus(); await native.keyboard('{Shift}');
                const rect = input.getBoundingClientRect(), style = () => getComputedStyle(input, null);
                const expected = getComputedStyle(input).getPropertyValue(appearance === 'underline' ? '--cem-indicator-appearance-underline' : '--cem-indicator-appearance-outline').trim();
                expect(getComputedStyle(input).getPropertyValue('--_cem-input-indicator-appearance').trim()).toBe(expected);
                expect(style().boxShadow).not.toBe('none'); expectIndicator(input, appearance);
                await native.hover(input); expectIndicator(input, appearance); expect(input.getBoundingClientRect().width).toBe(rect.width);
                host.setAttribute('busy', ''); await whenCemRendered(host); expect(input.getAttribute('aria-busy')).toBe('true'); expectIndicator(input, appearance, '--cem-stroke-pending');
                host.setAttribute('invalid', 'true'); await whenCemRendered(host);
                expect(input.getAttribute('aria-invalid')).toBe('true'); expectIndicator(input, appearance, '--cem-stroke-boundary');
                expect(input.getBoundingClientRect().width).toBe(rect.width); expect(document.activeElement).toBe(input); expect(required(host, 'input')).toBe(input);
                host.removeAttribute('busy'); host.removeAttribute('invalid'); await whenCemRendered(host);
            }
        }
        expect(events).toEqual([]); host.removeEventListener('input', record); host.removeEventListener('change', record);
    },
    parameters: { docs: { description: { story: 'Browser-runner-only: trusted Space, Enter and Tab, disabled skipping, five themes and native hover/focus indicators without state-changing component events.' } } },
});
