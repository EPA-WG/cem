import '@epa-wg/cem-theme/styles.css';
import { CemElementRuntime } from '@epa-wg/cem-elements';
import { userEvent } from 'vitest/browser';

import '../styles.css';
import progressSpinnerContractFixture from '../../tests/progress-spinner/contract.html?raw';
import { installCemComponentPrimitives } from './primitives.js';
import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    createComponentHarness,
    nextRenderFrame,
    type ComponentHarness,
} from './testing/component-harness.js';

describe('progress spinner contract fixture', () => {
    let harness: ComponentHarness;
    let runtime: CemElementRuntime;

    beforeAll(async () => {
        runtime = new CemElementRuntime({ declarationTag: 'cem-components-progress-spinner-declaration' });
        const result = await installCemComponentPrimitives(runtime);
        expect(result.diagnostics).toEqual([]);
    });

    afterEach(() => {
        harness?.cleanup();
    });

    it('renders exact circular determinate and indeterminate progress semantics', async () => {
        expect(progressSpinnerContractFixture).not.toMatch(/<script\b/i);
        expect(progressSpinnerContractFixture).not.toMatch(/\son[a-z]+\s*=/i);

        const root = await renderFixture();
        const indeterminate = spinnerParts(root, '#indeterminate-spinner');
        const determinate = spinnerParts(root, '#determinate-spinner');
        const custom = spinnerParts(root, '#custom-range-spinner');

        expect(indeterminate.owner.getAttribute('role')).toBe('progressbar');
        expect(indeterminate.owner.getAttribute('data-mode')).toBe('indeterminate');
        expect(assertAccessibleName(indeterminate.owner, 'Loading account')).toBe('Loading account');
        expect(indeterminate.owner.hasAttribute('aria-valuenow')).toBe(false);
        expect(indeterminate.owner.hasAttribute('aria-valuemin')).toBe(false);
        expect(indeterminate.owner.hasAttribute('aria-valuemax')).toBe(false);
        expect(indeterminate.indicator.getAttribute('stroke-dasharray')).toBe('25 75');

        expectDeterminate(determinate, { max: 100, value: 25, dash: '25 75' });
        expectDeterminate(custom, { max: 12, value: 3, dash: '25 75' });
        expect(custom.owner.getAttribute('aria-describedby')).toBe('records-help');

        for (const parts of [indeterminate, determinate, custom]) {
            expect(parts.host.querySelectorAll(':scope > .cem-progress-spinner')).toHaveLength(1);
            expect(parts.owner.parentElement).toBe(parts.host);
            expect(Object.prototype.toString.call(parts.svg)).toBe('[object SVGSVGElement]');
            expect(parts.svg.namespaceURI).toBe('http://www.w3.org/2000/svg');
            expect(Object.prototype.toString.call(parts.track)).toBe('[object SVGCircleElement]');
            expect(Object.prototype.toString.call(parts.indicator)).toBe('[object SVGCircleElement]');
            expect(parts.track.namespaceURI).toBe('http://www.w3.org/2000/svg');
            expect(parts.svg.getAttribute('viewBox')).toBe('0 0 100 100');
            expect(parts.svg.getAttribute('aria-hidden')).toBe('true');
            expect(parts.svg.getAttribute('focusable')).toBe('false');
            expect(parts.svg.getAttribute('tabindex')).toBeNull();
            expect(parts.track.getAttribute('pathLength')).toBe('100');
            expect(parts.indicator.getAttribute('pathLength')).toBe('100');
        }

        expect(root.querySelector('progress, [role="status"], [aria-live], [aria-busy]')).toBeNull();
        expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
    });

    it('normalizes invalid and out-of-range values without rewriting author attributes', async () => {
        const root = await renderFixture();
        const upper = spinnerParts(root, '#upper-clamp-spinner');
        const lower = spinnerParts(root, '#lower-clamp-spinner');
        const invalid = spinnerParts(root, '#invalid-spinner');
        const fallback = spinnerParts(root, '#fallback-label-spinner');

        expectDeterminate(upper, { max: 80, value: 80, dash: '100 0' });
        expect(upper.host.getAttribute('value')).toBe('120');
        expect(upper.host.getAttribute('max')).toBe('80');
        expectDeterminate(lower, { max: 100, value: 0, dash: '0 100' });
        expect(lower.host.getAttribute('value')).toBe('-20');
        expectDeterminate(invalid, { max: 100, value: 0, dash: '0 100' });
        expect(invalid.host.getAttribute('value')).toBe('not-a-number');
        expect(invalid.host.getAttribute('max')).toBe('0');
        expectDeterminate(fallback, { max: 100, value: 0, dash: '0 100' });
        expect(assertAccessibleName(fallback.owner, 'Progress')).toBe('Progress');
    });

    it('reuses the SVG owner across live value, range, and mode changes with stable geometry', async () => {
        const root = await renderFixture();
        const parts = spinnerParts(root, '#determinate-spinner');
        const originalOwner = parts.owner;
        const originalSvg = parts.svg;
        const originalTrack = parts.track;
        const originalIndicator = parts.indicator;
        const baseline = captureGeometry(parts);

        parts.host.setAttribute('value', '75');
        await waitForSpinner(parts.host, 'determinate', '75');
        expectDeterminate(spinnerParts(root, '#determinate-spinner'), { max: 100, value: 75, dash: '75 25' });

        parts.host.setAttribute('max', '60');
        await waitForSpinner(parts.host, 'determinate', '60');
        expectDeterminate(spinnerParts(root, '#determinate-spinner'), { max: 60, value: 60, dash: '100 0' });

        parts.host.removeAttribute('value');
        await waitForSpinner(parts.host, 'indeterminate', null);
        const indeterminate = spinnerParts(root, '#determinate-spinner');
        expect(indeterminate.owner).toBe(originalOwner);
        expect(indeterminate.svg).toBe(originalSvg);
        expect(indeterminate.track).toBe(originalTrack);
        expect(indeterminate.indicator).toBe(originalIndicator);
        expect(indeterminate.owner.hasAttribute('aria-valuenow')).toBe(false);
        expect(indeterminate.indicator.getAttribute('stroke-dasharray')).toBe('25 75');
        expectGeometry(captureGeometry(indeterminate), baseline);
        expect(parts.host.getAttribute('max')).toBe('60');
        expect(parts.host.hasAttribute('value')).toBe(false);
    });

    it('binds D0/D2c/D7 tokens while pointer and keyboard input stay event- and state-neutral', async () => {
        const root = await renderFixture();
        const parts = spinnerParts(root, '#indeterminate-spinner');
        const determinate = spinnerParts(root, '#determinate-spinner');
        const events: string[] = [];
        for (const eventName of ['pointerenter', 'pointerleave', 'click', 'input', 'change', 'cem-progress']) {
            parts.owner.addEventListener(eventName, (event) => events.push(`${event.type}:${event.isTrusted}`));
        }

        const baselineHtml = parts.host.innerHTML;
        const baseline = captureGeometry(parts);
        const ownerStyle = getComputedStyle(parts.owner);
        const svgStyle = getComputedStyle(parts.svg);
        const trackStyle = getComputedStyle(parts.track);
        const indicatorStyle = getComputedStyle(parts.indicator);
        const determinateIndicatorStyle = getComputedStyle(determinate.indicator);

        expect(baseline.width).toBeCloseTo(resolveTokenLength(parts.owner, '--cem-progress-spinner-size'), 4);
        expect(baseline.height).toBeCloseTo(resolveTokenLength(parts.owner, '--cem-progress-spinner-size'), 4);
        expect(pixels(trackStyle.strokeWidth)).toBeCloseTo(
            resolveTokenLength(parts.owner, '--cem-progress-track-thickness'),
            4,
        );
        expect(pixels(indicatorStyle.strokeWidth)).toBeCloseTo(pixels(trackStyle.strokeWidth), 4);
        expect(trackStyle.stroke).toBe(resolveTokenColor(parts.owner, '--cem-progress-track-color'));
        expect(indicatorStyle.stroke).toBe(resolveTokenColor(parts.owner, '--cem-progress-indicator-color'));
        expect(indicatorStyle.animationName).toBe('cem-progress-spinner-cycle');
        expect(indicatorStyle.animationDuration).toBe(
            resolveAnimationValue(parts.owner, 'animationDuration', '--cem-duration-continuous-cycle'),
        );
        expect(indicatorStyle.animationTimingFunction).toBe(
            resolveAnimationValue(parts.owner, 'animationTimingFunction', '--cem-easing-uniform'),
        );
        expect(determinateIndicatorStyle.animationName).toBe('none');
        expect(ownerStyle.forcedColorAdjust).toBe('auto');
        expect(svgStyle.overflow).toBe('visible');

        await userEvent.hover(parts.owner);
        await nextRenderFrame();
        await userEvent.click(parts.owner);
        await userEvent.unhover(parts.owner);
        await nextRenderFrame();

        expectGeometry(captureGeometry(parts), baseline);
        expect(parts.host.innerHTML).toBe(baselineHtml);
        expect(parts.owner.getAttribute('data-mode')).toBe('indeterminate');
        expect(parts.owner.hasAttribute('aria-valuenow')).toBe(false);
        expect(events).toEqual(['pointerenter:true', 'click:true', 'pointerleave:true']);

        requiredElement<HTMLButtonElement>(root, '[data-progress-spinner-focus-start]').focus();
        await userEvent.tab();
        expect(document.activeElement).toBe(
            requiredElement<HTMLButtonElement>(root, '[data-progress-spinner-focus-end]'),
        );
        expect(parts.owner.matches(':focus')).toBe(false);
        expect(parts.svg.matches(':focus')).toBe(false);
    });

    async function renderFixture(): Promise<HTMLElement> {
        harness = createComponentHarness({ runtime });
        const root = await harness.render(progressSpinnerContractFixture);
        await waitForSelector(root, '#fallback-label-spinner > .cem-progress-spinner > svg');
        return root;
    }
});

interface SpinnerParts {
    host: HTMLElement;
    indicator: SVGCircleElement;
    owner: HTMLElement;
    svg: SVGSVGElement;
    track: SVGCircleElement;
}

function spinnerParts(root: ParentNode, selector: string): SpinnerParts {
    const host = requiredElement<HTMLElement>(root, selector);
    const owner = requiredElement<HTMLElement>(host, ':scope > .cem-progress-spinner');
    return {
        host,
        indicator: requiredElement<SVGCircleElement>(owner, ':scope > svg > .cem-progress-spinner__indicator'),
        owner,
        svg: requiredElement<SVGSVGElement>(owner, ':scope > svg'),
        track: requiredElement<SVGCircleElement>(owner, ':scope > svg > .cem-progress-spinner__track'),
    };
}

function expectDeterminate(parts: SpinnerParts, expected: { dash: string; max: number; value: number }): void {
    expect(parts.owner.getAttribute('data-mode')).toBe('determinate');
    expect(parts.owner.getAttribute('aria-valuemin')).toBe('0');
    expect(parts.owner.getAttribute('aria-valuemax')).toBe(String(expected.max));
    expect(parts.owner.getAttribute('aria-valuenow')).toBe(String(expected.value));
    expect(parts.indicator.getAttribute('stroke-dasharray')).toBe(expected.dash);
}

function captureGeometry(parts: SpinnerParts) {
    const ownerRect = parts.owner.getBoundingClientRect();
    const svgRect = parts.svg.getBoundingClientRect();
    return {
        height: ownerRect.height,
        svgHeight: svgRect.height,
        svgWidth: svgRect.width,
        width: ownerRect.width,
    };
}

function expectGeometry(actual: ReturnType<typeof captureGeometry>, expected: ReturnType<typeof captureGeometry>): void {
    expect(actual.height).toBeCloseTo(expected.height, 4);
    expect(actual.width).toBeCloseTo(expected.width, 4);
    expect(actual.svgHeight).toBeCloseTo(expected.svgHeight, 4);
    expect(actual.svgWidth).toBeCloseTo(expected.svgWidth, 4);
}

function resolveTokenColor(owner: HTMLElement, token: string): string {
    const probe = owner.ownerDocument.createElement('span');
    probe.style.color = `var(${token})`;
    owner.append(probe);
    const value = getComputedStyle(probe).color;
    probe.remove();
    return value;
}

function resolveTokenLength(owner: HTMLElement, token: string): number {
    const probe = owner.ownerDocument.createElement('span');
    probe.style.display = 'block';
    probe.style.inlineSize = `var(${token})`;
    owner.append(probe);
    const value = pixels(getComputedStyle(probe).inlineSize);
    probe.remove();
    return value;
}

function resolveAnimationValue(
    owner: HTMLElement,
    property: 'animationDuration' | 'animationTimingFunction',
    token: string,
): string {
    const probe = owner.ownerDocument.createElement('span');
    probe.style[property] = `var(${token})`;
    owner.append(probe);
    const value = getComputedStyle(probe)[property];
    probe.remove();
    return value;
}

function pixels(value: string): number {
    const parsed = Number.parseFloat(value);
    if (!Number.isFinite(parsed)) throw new Error(`Expected a resolved CSS length, received ${value}`);
    return parsed;
}

async function waitForSpinner(host: HTMLElement, mode: string, value: string | null): Promise<void> {
    for (let attempt = 0; attempt < 60; attempt += 1) {
        const owner = host.querySelector<HTMLElement>(':scope > .cem-progress-spinner');
        if (owner?.getAttribute('data-mode') === mode && owner.getAttribute('aria-valuenow') === value) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for progress spinner mode=${mode} value=${String(value)}`);
}

async function waitForSelector(root: ParentNode, selector: string): Promise<void> {
    for (let attempt = 0; attempt < 60; attempt += 1) {
        if (root.querySelector(selector)) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for ${selector}`);
}

function requiredElement<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
    const element = root.querySelector<T>(selector);
    if (!element) throw new Error(`Missing required fixture element: ${selector}`);
    return element;
}
