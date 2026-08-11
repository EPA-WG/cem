import '@epa-wg/cem-theme/styles.css';
import '../styles.css';

import { CemElementRuntime } from '@epa-wg/cem-elements';
import { userEvent } from 'vitest/browser';

import { installCemComponentPrimitives } from './primitives.js';
import { createComponentHarness, nextRenderFrame, type ComponentHarness } from './testing/component-harness.js';

describe('CEM divider contract', () => {
    let harness: ComponentHarness;

    beforeAll(() => {
        const runtime = new CemElementRuntime({ declarationTag: 'cem-divider-contract-declaration' });
        const result = installCemComponentPrimitives(runtime);
        expect(result.diagnostics).toEqual([]);
    });

    afterEach(() => {
        harness?.cleanup();
    });

    it('exposes exact semantic orientation and an explicit decorative boundary', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <div>
                <button id="before" type="button">Before</button>
                <cem-divider id="horizontal"></cem-divider>
                <cem-divider id="vertical" orientation="vertical"></cem-divider>
                <cem-divider id="fallback" orientation="diagonal"></cem-divider>
                <cem-divider id="decorative" decorative></cem-divider>
                <button id="after" type="button">After</button>
            </div>
        `);
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
    });

    it('composes D0 color, D1 relationship and inset, D2 guard, and D5 line geometry', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <div>
                <cem-divider id="related" spacing="related"></cem-divider>
                <cem-divider id="group"></cem-divider>
                <cem-divider id="block" spacing="block" inset></cem-divider>
                <cem-divider id="section" spacing="section"></cem-divider>
                <div id="vertical-frame">
                    <span>Before</span>
                    <cem-divider id="vertical" orientation="vertical" inset></cem-divider>
                    <span>After</span>
                </div>
            </div>
        `);
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
    });

    it('keeps trusted pointer and click input event-neutral and geometry-stable', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <div>
                <cem-divider id="owner" inset></cem-divider>
            </div>
        `);
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
    });
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
