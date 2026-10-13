import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, nextRenderFrame, createCemStoryRuntime, assertAccessibleName, assertAriaReferenceIntegrity } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-progress-spinner.xhtml?raw';
const progressSpinnerContractFixture = "<section class=\"cem-theme-light\" data-progress-spinner-contract-fixture aria-label=\"Progress spinner contract fixture\">\n  <button data-progress-spinner-focus-start type=\"button\">Focus start</button>\n\n  <cem-progress-spinner id=\"indeterminate-spinner\" label=\"Loading account\"></cem-progress-spinner>\n  <cem-progress-spinner id=\"determinate-spinner\" label=\"Uploading files\" value=\"25\"></cem-progress-spinner>\n  <cem-progress-spinner id=\"custom-range-spinner\" label=\"Processing records\" value=\"3\" max=\"12\" describedby=\"records-help\"></cem-progress-spinner>\n  <span id=\"records-help\">Three of twelve records processed.</span>\n  <cem-progress-spinner id=\"upper-clamp-spinner\" label=\"Complete\" value=\"120\" max=\"80\"></cem-progress-spinner>\n  <cem-progress-spinner id=\"lower-clamp-spinner\" label=\"Not started\" value=\"-20\"></cem-progress-spinner>\n  <cem-progress-spinner id=\"invalid-spinner\" label=\"Invalid input fallback\" value=\"not-a-number\" max=\"0\"></cem-progress-spinner>\n  <cem-progress-spinner id=\"fallback-label-spinner\" value=\"\"></cem-progress-spinner>\n\n  <button data-progress-spinner-focus-end type=\"button\">Focus end</button>\n</section>\n";
const meta = preview.meta({ component: 'cem-progress-spinner', title: 'CEM Components/cem-progress-spinner', loaders: [async () => { await loadCemDeclaration('cem-progress-spinner', declarationSource); return {}; }] });

export const Semantics = meta.story({
    name: 'renders exact circular determinate and indeterminate progress semantics',
    render: () => progressSpinnerContractFixture,

    play: async ({ canvasElement }) => {
        expect(progressSpinnerContractFixture).not.toMatch(/<script\b/i);
        expect(progressSpinnerContractFixture).not.toMatch(/\son[a-z]+\s*=/i);

        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-progress-spinner')) await whenCemRendered(host);
        await nextRenderFrame();
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
    },
});

export const Normalization = meta.story({
    name: 'normalizes invalid and out-of-range values without rewriting author attributes',
    render: () => progressSpinnerContractFixture,

    play: async ({ canvasElement }) => {
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-progress-spinner')) await whenCemRendered(host);
        await nextRenderFrame();
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
    },
});

export const RetainedGeometry = meta.story({
    name: 'reuses the SVG owner across live value, range, and mode changes with stable geometry',
    render: () => progressSpinnerContractFixture,

    play: async ({ canvasElement }) => {
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-progress-spinner')) await whenCemRendered(host);
        await nextRenderFrame();
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
    },
});

export const NativeInputNeutrality = meta.story({
    name: 'binds D0/D2c/D7 tokens while pointer and keyboard input stay event- and state-neutral',
    render: () => progressSpinnerContractFixture,
    parameters: { docs: { description: { story: 'Browser-runner-only: trusted pointer and keyboard checks.' } } },
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-progress-spinner')) await whenCemRendered(host);
        await nextRenderFrame();
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
        expect(indicatorStyle.animationName).toMatch(/^cem-progress-spinner-cycle-cem-[a-f0-9]+$/);
        expect(parts.indicator.getAnimations()).toHaveLength(1);
        expect((parts.indicator.getAnimations()[0] as CSSAnimation).animationName).toBe(indicatorStyle.animationName);
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

        required<HTMLButtonElement>(root, '[data-progress-spinner-focus-start]').focus();
        await userEvent.tab();
        expect(document.activeElement).toBe(
            required<HTMLButtonElement>(root, '[data-progress-spinner-focus-end]'),
        );
        expect(parts.owner.matches(':focus')).toBe(false);
        expect(parts.svg.matches(':focus')).toBe(false);
    },
});

interface SpinnerParts {
    host: HTMLElement;
    indicator: SVGCircleElement;
    owner: HTMLElement;
    svg: SVGSVGElement;
    track: SVGCircleElement;
}

function spinnerParts(root: ParentNode, selector: string): SpinnerParts {
    const host = required<HTMLElement>(root, selector);
    const owner = required<HTMLElement>(host, ':scope > .cem-progress-spinner');
    return {
        host,
        indicator: required<SVGCircleElement>(owner, ':scope > svg > .cem-progress-spinner__indicator'),
        owner,
        svg: required<SVGSVGElement>(owner, ':scope > svg'),
        track: required<SVGCircleElement>(owner, ':scope > svg > .cem-progress-spinner__track'),
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

function required<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
    const element = root.querySelector<T>(selector);
    if (!element) throw new Error(`Missing required fixture element: ${selector}`);
    return element;
}

export const WorkerRetention = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-progress-spinner-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
        const host = document.createElement(tag); host.innerHTML = ''; canvasElement.append(host);
        try {
            await runtime.whenRenderSettled(host);
            const owner = required<HTMLElement>(host, '[part=root]'), child = host.querySelector('strong'), svg = host.querySelector('svg');
            const events: string[] = []; for (const type of ['input', 'change', 'cem-progress']) host.addEventListener(type, () => events.push(type));
            host.setAttribute('label', 'Updated'); host.setAttribute('value', '30'); host.setAttribute('max', '12');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(child); expect(host.querySelector('svg')).toBe(svg);
            expect(owner.getAttribute('aria-valuenow')).toBe('12'); expect(owner.getAttribute('aria-valuemax')).toBe('12');
            owner.click(); expect(events).toEqual([]);
            host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(child);
            host.hidden = true; await runtime.whenRenderSettled(host); expect(getComputedStyle(host).display).toBe('none');
            expect(runtime.diagnosticsFor(host).map(({code, message}) => ({code, message}))).toEqual([]);
        } finally { host.remove(); declaration.remove(); scope.dispose(); }
    },
});

export const FallbackRetention = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, true);
        const tag = `cem-progress-spinner-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
        const host = document.createElement(tag); host.innerHTML = ''; canvasElement.append(host);
        try {
            await runtime.whenRenderSettled(host);
            const owner = required<HTMLElement>(host, '[part=root]'), child = host.querySelector('strong'), svg = host.querySelector('svg');
            const events: string[] = []; for (const type of ['input', 'change', 'cem-progress']) host.addEventListener(type, () => events.push(type));
            host.setAttribute('label', 'Updated'); host.setAttribute('value', '30'); host.setAttribute('max', '12');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(child); expect(host.querySelector('svg')).toBe(svg);
            expect(owner.getAttribute('aria-valuenow')).toBe('12'); expect(owner.getAttribute('aria-valuemax')).toBe('12');
            owner.click(); expect(events).toEqual([]);
            host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(child);
            host.hidden = true; await runtime.whenRenderSettled(host); expect(getComputedStyle(host).display).toBe('none');
            expect(runtime.diagnosticsFor(host).map(({code, message}) => ({code, message}))).toEqual([]);
        } finally { host.remove(); declaration.remove(); scope.dispose(); }
    },
});

export const ThemeInheritance = meta.story({
    render: () => '<div class="theme-audit"><cem-progress-spinner></cem-progress-spinner></div>',
    play: async ({ canvasElement }) => {
        const container = required<HTMLElement>(canvasElement, '.theme-audit'), host = required<HTMLElement>(container, 'cem-progress-spinner'); await whenCemRendered(host);
        const owner = required<HTMLElement>(host, '[part=root]'), style = () => getComputedStyle(owner);
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            container.className = `theme-audit cem-theme-${mode}`;
            container.style.setProperty('--cem-progress-spinner-size', '48px'); container.style.setProperty('--cem-progress-track-thickness', '6px'); container.style.setProperty('--cem-progress-indicator-color', 'rgb(24, 36, 48)'); expect(style().width).toBe('48px'); const indicator = required(host, '[part=indicator]'); expect(getComputedStyle(indicator).strokeWidth).toBe('6px'); expect(getComputedStyle(indicator).stroke).toBe('rgb(24, 36, 48)');
        }
    },
});
