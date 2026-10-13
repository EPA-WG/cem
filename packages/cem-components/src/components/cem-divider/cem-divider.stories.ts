import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, cemStoryRuntime, createCemStoryRuntime, loadCemWorkflowDependencies, nextRenderFrame } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-divider.xhtml?raw';
import '../../styles.css';
const meta = preview.meta({ component: 'cem-divider', title: 'CEM Components/cem-divider', loaders: [async () => { await loadCemDeclaration('cem-divider', declarationSource); await loadCemWorkflowDependencies(); return {}; }] });

export const Semantics = meta.story({
 name: "exposes exact semantic orientation and an explicit decorative boundary",
 render: () => "\n            <div>\n                <button id=\"before\" type=\"button\">Before</button>\n                <cem-divider id=\"horizontal\"></cem-divider>\n                <cem-divider id=\"vertical\" orientation=\"vertical\"></cem-divider>\n                <cem-divider id=\"fallback\" orientation=\"diagonal\"></cem-divider>\n                <cem-divider id=\"decorative\" decorative></cem-divider>\n                <button id=\"after\" type=\"button\">After</button>\n            </div>\n        ",
 play: async ({ canvasElement }) => {
        const root = canvasElement, runtime = cemStoryRuntime();
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);
        await nextRenderFrame();

        await waitForDivider(root, '#decorative > .cem-divider');

        const horizontal = harness.query<HTMLElement>('#horizontal > .cem-divider');
        const vertical = harness.query<HTMLElement>('#vertical > .cem-divider');
        const fallback = harness.query<HTMLElement>('#fallback > .cem-divider');
        const decorative = harness.query<HTMLElement>('#decorative > .cem-divider');
        const before = harness.query<HTMLButtonElement>('#before');

        expect(horizontal.getAttribute('role')).toBe('separator');
        expect(horizontal.getAttribute('aria-orientation')).toBe('horizontal');
        expect(horizontal.hasAttribute('aria-label')).toBe(false);
        expect(horizontal.hasAttribute('tabindex')).toBe(false);
        expect(vertical.getAttribute('role')).toBe('separator');
        expect(vertical.getAttribute('aria-orientation')).toBe('vertical');
        expect(fallback.getAttribute('aria-orientation')).toBe('horizontal');
        expect(decorative.getAttribute('aria-hidden')).toBe('true');
        expect(decorative.hasAttribute('role')).toBe(false);
        expect(decorative.hasAttribute('aria-orientation')).toBe(false);
        expect(decorative.hasAttribute('tabindex')).toBe(false);

        before.focus();
        decorative.focus();
        expect(document.activeElement).toBe(before);
 },
});

export const TokenGeometry = meta.story({
 name: "composes D0 color, D1 relationship and inset, D2 guard, and D5 line geometry",
 render: () => "\n            <div>\n                <cem-divider id=\"related\" spacing=\"related\"></cem-divider>\n                <cem-divider id=\"group\"></cem-divider>\n                <cem-divider id=\"block\" spacing=\"block\" inset></cem-divider>\n                <cem-divider id=\"section\" spacing=\"section\"></cem-divider>\n                <div id=\"vertical-frame\">\n                    <span>Before</span>\n                    <cem-divider id=\"vertical\" orientation=\"vertical\" inset></cem-divider>\n                    <span>After</span>\n                </div>\n            </div>\n        ",
 play: async ({ canvasElement }) => {
        const root = canvasElement, runtime = cemStoryRuntime();
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);
        await nextRenderFrame();

        const verticalFrame = harness.query<HTMLElement>('#vertical-frame');
        verticalFrame.style.display = 'flex';
        verticalFrame.style.height = '8rem';
        await waitForDivider(root, '#vertical > .cem-divider');

        for (const [id, spacingToken] of [
            ['related', '--cem-gap-related'],
            ['group', '--cem-gap-group'],
            ['block', '--cem-gap-block'],
            ['section', '--cem-gap-section'],
        ] as const) {
            const host = harness.query<HTMLElement>(`#${id}`);
            const line = harness.query<HTMLElement>(`#${id} > .cem-divider`);
            const hostStyle = getComputedStyle(host);
            const lineStyle = getComputedStyle(line);
            const expectedTrack = Math.max(
                resolveLength(host, spacingToken),
                resolveLength(host, '--cem-coupling-guard-min'),
            );
            const actualTrack =
                pixels(hostStyle.marginBlockStart) +
                pixels(lineStyle.borderBlockStartWidth) +
                pixels(hostStyle.marginBlockEnd);

            expect(lineStyle.borderBlockStartWidth).toBe(resolveCssLength(host, '--cem-stroke-divider'));
            expect(lineStyle.borderBlockStartColor).toBe(resolveCssColor(host, '--cem-separator-color'));
            expect(actualTrack).toBeCloseTo(expectedTrack, 4);
            expect(line.getBoundingClientRect().width).toBeGreaterThan(0);
        }

        const insetLine = harness.query<HTMLElement>('#block > .cem-divider');
        expect(getComputedStyle(insetLine).marginInlineStart).toBe(
            resolveCssLength(insetLine, '--cem-inset-container'),
        );

        const verticalHost = harness.query<HTMLElement>('#vertical');
        const verticalLine = harness.query<HTMLElement>('#vertical > .cem-divider');
        const verticalHostStyle = getComputedStyle(verticalHost);
        const verticalLineStyle = getComputedStyle(verticalLine);
        const expectedVerticalTrack = Math.max(
            resolveLength(verticalHost, '--cem-gap-group'),
            resolveLength(verticalHost, '--cem-coupling-guard-min'),
        );
        const actualVerticalTrack =
            pixels(verticalHostStyle.marginInlineStart) +
            pixels(verticalLineStyle.borderInlineStartWidth) +
            pixels(verticalHostStyle.marginInlineEnd);

        expect(verticalLineStyle.borderInlineStartWidth).toBe(
            resolveCssLength(verticalLine, '--cem-stroke-divider'),
        );
        expect(verticalLineStyle.marginBlockStart).toBe(resolveCssLength(verticalLine, '--cem-inset-container'));
        expect(actualVerticalTrack).toBeCloseTo(expectedVerticalTrack, 4);
        expect(verticalLine.getBoundingClientRect().height).toBeGreaterThan(0);
 },
});

export const PointerNeutral = meta.story({
 name: "keeps trusted pointer and click input event-neutral and geometry-stable",
 render: () => "\n            <div>\n                <cem-divider id=\"owner\" inset></cem-divider>\n            </div>\n        ",
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted pointer input assertions.' } } },
 play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const root = canvasElement, runtime = cemStoryRuntime();
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);
        await nextRenderFrame();

        await waitForDivider(root, '#owner > .cem-divider');

        const host = harness.query<HTMLElement>('#owner');
        const line = harness.query<HTMLElement>('#owner > .cem-divider');
        const pointerEvents: string[] = [];
        const applicationEvents: string[] = [];
        const mutations: MutationRecord[] = [];
        const observer = new MutationObserver((records) => mutations.push(...records));
        observer.observe(host, { attributes: true, childList: true, subtree: true });
        line.addEventListener('pointerenter', (event) => pointerEvents.push(`pointerenter:${event.isTrusted}`));
        line.addEventListener('pointerleave', (event) => pointerEvents.push(`pointerleave:${event.isTrusted}`));
        line.addEventListener('click', (event) => applicationEvents.push(`click:${event.isTrusted}`));
        for (const eventName of ['input', 'change', 'cem-divider-change']) {
            host.addEventListener(eventName, () => applicationEvents.push(eventName));
        }
        const baseline = captureDivider(host, line);

        await userEvent.hover(line);
        expect(captureDivider(host, line)).toEqual(baseline);
        await userEvent.click(line, { force: true });
        expect(captureDivider(host, line)).toEqual(baseline);
        await userEvent.unhover(line);
        await nextRenderFrame();

        expect(pointerEvents).toEqual(['pointerenter:true', 'pointerleave:true']);
        expect(applicationEvents).toEqual(['click:true']);
        expect(mutations).toEqual([]);
        expect(captureDivider(host, line)).toEqual(baseline);
        observer.disconnect();
 },
});

function captureDivider(host: HTMLElement, line: HTMLElement) {
    const hostRect = host.getBoundingClientRect();
    const lineRect = line.getBoundingClientRect();
    const hostStyle = getComputedStyle(host);
    const lineStyle = getComputedStyle(line);

    return {
        html: host.outerHTML,
        host: [hostRect.width, hostRect.height],
        line: [lineRect.width, lineRect.height],
        margin: [
            hostStyle.marginBlockStart,
            hostStyle.marginInlineEnd,
            hostStyle.marginBlockEnd,
            hostStyle.marginInlineStart,
        ],
        stroke: [
            lineStyle.borderBlockStartColor,
            lineStyle.borderBlockStartWidth,
            lineStyle.borderInlineStartColor,
            lineStyle.borderInlineStartWidth,
        ],
    };
}

function resolveCssColor(owner: HTMLElement, token: string): string {
    const probe = owner.ownerDocument.createElement('span');
    probe.style.color = `var(${token})`;
    owner.append(probe);
    const value = getComputedStyle(probe).color;
    probe.remove();
    return value;
}

function resolveCssLength(owner: HTMLElement, token: string): string {
    const probe = owner.ownerDocument.createElement('span');
    probe.style.display = 'block';
    probe.style.inlineSize = `var(${token})`;
    owner.append(probe);
    const value = getComputedStyle(probe).inlineSize;
    probe.remove();
    return value;
}

function resolveLength(owner: HTMLElement, token: string): number {
    return pixels(resolveCssLength(owner, token));
}

function pixels(value: string): number {
    const parsed = Number.parseFloat(value);
    if (!Number.isFinite(parsed)) {
        throw new Error(`Expected a resolved CSS length, received ${value}`);
    }
    return parsed;
}

async function waitForDivider(root: ParentNode, selector: string): Promise<void> {
    for (let attempt = 0; attempt < 60; attempt += 1) {
        if (root.querySelector(selector)) {
            return;
        }
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for ${selector}`);
}

export const WorkerRetention = meta.story({
 render: () => '<div></div>',
 play: async ({ canvasElement }) => {
        const { runtime, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-divider-${crypto.randomUUID()}`;
        await declare(declarationSource, tag);
        const host = document.createElement(tag);
        host.setAttribute('label', 'Retained content');
        host.innerHTML = "";
        canvasElement.append(host);
        await runtime.whenRenderSettled(host);
        const owner = host.querySelector<HTMLElement>('[part=root]')!;
        const authored = Array.from(owner.querySelectorAll('[slot], label')), input = host.querySelector('input');
        expect(owner).not.toBeNull();
        expect(owner.tabIndex).toBe(-1);
        expect(owner.hasAttribute('inert')).toBe(false);
        expect(owner.hasAttribute('aria-live')).toBe(false);
        expect(host.shadowRoot).toBeNull();
        if (input) { input.value = 'Edited'; input.focus(); }
        for (const value of ['', 'false']) {
            host.setAttribute('busy', value); host.setAttribute('empty', value); host.setAttribute('decorative', value);
            host.setAttribute('gap', 'lg'); host.setAttribute('columns', '2'); host.setAttribute('orientation', 'vertical');
            host.setAttribute('label', 'Updated label'); host.setAttribute('variant', 'caption');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner);
            expect(owner.getAttribute('data-state')).toBe(null);
            expect(owner.getAttribute('aria-busy')).toBe(null);
            if (input) { expect(host.querySelector('input')).toBe(input); expect(input.value).toBe('Edited'); expect(document.activeElement).toBe(input); }
            for (const child of authored) expect(host.contains(child)).toBe(true);
            expect(owner.getAttribute('aria-hidden')).toBe('true'); expect(owner.hasAttribute('role')).toBe(false); expect(owner.getAttribute('data-orientation')).toBe('vertical'); expect(owner.hasAttribute('aria-orientation')).toBe(false);
            host.removeAttribute('busy'); await runtime.whenRenderSettled(host);
            expect(owner.getAttribute('data-state')).toBe(null);
            expect(owner.hasAttribute('aria-busy')).toBe(false);
            host.removeAttribute('empty'); host.removeAttribute('decorative');
            await runtime.whenRenderSettled(host);
            expect(owner.hasAttribute('data-state')).toBe(false);
        }
        host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
        expect(host.querySelector('[part=root]')).toBe(owner);
        if (input) { expect(host.querySelector('input')).toBe(input); expect(input.value).toBe('Edited'); }
        const snapshot = runtime.snapshotInstance(host);
        for (const state of ['busy', 'empty', 'decorative']) {
            expect(snapshot.slices).not.toHaveProperty(state); expect(snapshot.eventPayloads).not.toHaveProperty(state);
        }
        expect(runtime.diagnosticsFor(host).filter(d => d.severity === 'error' || d.severity === 'fatal')).toEqual([]);
        host.hidden = true; await runtime.whenRenderSettled(host);
        expect(getComputedStyle(host).display).toBe('none');
        host.remove();

 },
});

export const FallbackRetention = meta.story({
 render: () => '<div></div>',
 play: async ({ canvasElement }) => {
        const { runtime, declare } = createCemStoryRuntime(canvasElement, true);
        const tag = `cem-divider-${crypto.randomUUID()}`;
        await declare(declarationSource, tag);
        const host = document.createElement(tag);
        host.setAttribute('label', 'Retained content');
        host.innerHTML = "";
        canvasElement.append(host);
        await runtime.whenRenderSettled(host);
        const owner = host.querySelector<HTMLElement>('[part=root]')!;
        const authored = Array.from(owner.querySelectorAll('[slot], label')), input = host.querySelector('input');
        expect(owner).not.toBeNull();
        expect(owner.tabIndex).toBe(-1);
        expect(owner.hasAttribute('inert')).toBe(false);
        expect(owner.hasAttribute('aria-live')).toBe(false);
        expect(host.shadowRoot).toBeNull();
        if (input) { input.value = 'Edited'; input.focus(); }
        for (const value of ['', 'false']) {
            host.setAttribute('busy', value); host.setAttribute('empty', value); host.setAttribute('decorative', value);
            host.setAttribute('gap', 'lg'); host.setAttribute('columns', '2'); host.setAttribute('orientation', 'vertical');
            host.setAttribute('label', 'Updated label'); host.setAttribute('variant', 'caption');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner);
            expect(owner.getAttribute('data-state')).toBe(null);
            expect(owner.getAttribute('aria-busy')).toBe(null);
            if (input) { expect(host.querySelector('input')).toBe(input); expect(input.value).toBe('Edited'); expect(document.activeElement).toBe(input); }
            for (const child of authored) expect(host.contains(child)).toBe(true);
            expect(owner.getAttribute('aria-hidden')).toBe('true'); expect(owner.hasAttribute('role')).toBe(false); expect(owner.getAttribute('data-orientation')).toBe('vertical'); expect(owner.hasAttribute('aria-orientation')).toBe(false);
            host.removeAttribute('busy'); await runtime.whenRenderSettled(host);
            expect(owner.getAttribute('data-state')).toBe(null);
            expect(owner.hasAttribute('aria-busy')).toBe(false);
            host.removeAttribute('empty'); host.removeAttribute('decorative');
            await runtime.whenRenderSettled(host);
            expect(owner.hasAttribute('data-state')).toBe(false);
        }
        host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
        expect(host.querySelector('[part=root]')).toBe(owner);
        if (input) { expect(host.querySelector('input')).toBe(input); expect(input.value).toBe('Edited'); }
        const snapshot = runtime.snapshotInstance(host);
        for (const state of ['busy', 'empty', 'decorative']) {
            expect(snapshot.slices).not.toHaveProperty(state); expect(snapshot.eventPayloads).not.toHaveProperty(state);
        }
        expect(runtime.diagnosticsFor(host).filter(d => d.severity === 'error' || d.severity === 'fatal')).toEqual([]);
        host.hidden = true; await runtime.whenRenderSettled(host);
        expect(getComputedStyle(host).display).toBe('none');
        host.remove();

 },
});

export const ThemeInheritance = meta.story({
 render: () => '<div class="theme-audit"><cem-divider></cem-divider></div>',
 play: async ({ canvasElement }) => {
        const container = canvasElement.querySelector<HTMLElement>('.theme-audit');
        const host = container?.querySelector<HTMLElement>('cem-divider');
        if (!container || !host) throw new Error('Missing theme audit fixture');
        await whenCemRendered(host);
        const owner = host.querySelector<HTMLElement>('[part=root]');
        if (!owner) throw new Error('Missing component root');
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            container.className = `theme-audit cem-theme-${mode}`;
            container.style.setProperty('--cem-gap-group', '30px'); container.style.setProperty('--cem-coupling-guard-min', '40px');
            container.style.setProperty('--cem-stroke-divider', '4px'); container.style.setProperty('--cem-separator-color', 'rgb(24, 36, 48)');
            const style = getComputedStyle(owner), hostStyle = getComputedStyle(host);
            expect(style.borderBlockStartWidth).toBe('4px'); expect(style.borderBlockStartColor).toBe('rgb(24, 36, 48)');
            expect(parseFloat(hostStyle.marginBlockStart) + parseFloat(style.borderBlockStartWidth) + parseFloat(hostStyle.marginBlockEnd)).toBe(40);
        }

 },
});
