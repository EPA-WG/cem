import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, cemStoryRuntime, createCemStoryRuntime, loadCemWorkflowDependencies, assertAccessibleName, assertAriaReferenceIntegrity, assertFocusVisible, assertLightDomRendered, nextRenderFrame } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-surface.xhtml?raw';
import '../../styles.css';
const meta = preview.meta({ component: 'cem-surface', title: 'CEM Components/cem-surface', loaders: [async () => { await loadCemDeclaration('cem-surface', declarationSource); await loadCemWorkflowDependencies(); return {}; }] });

export const EmptyWorkflow = meta.story({
 name: "marks explicit empty workflow surfaces without inferring layout emptiness",
 render: () => "\n            <cem-stack gap=\"sm\">\n                <cem-surface label=\"Dashboard\">\n                    <p>Ready</p>\n                </cem-surface>\n                <cem-surface label=\"Asset results\" empty>\n                    <h2>No assets yet</h2>\n                    <p>Upload an asset to begin building this collection.</p>\n                    <a href=\"#new-asset\">Upload an asset</a>\n                </cem-surface>\n                <cem-surface label=\"False-token results\" empty=\"false\">\n                    <p>No matching results.</p>\n                    <a href=\"#clear-filters\">Clear filters</a>\n                </cem-surface>\n                <cem-stack id=\"empty-stack\" empty></cem-stack>\n                <cem-grid id=\"empty-grid\" empty></cem-grid>\n            </cem-stack>\n        ",
 play: async ({ canvasElement }) => {
        const root = canvasElement, runtime = cemStoryRuntime();
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);
        await nextRenderFrame();
        const authoredFallback = document.createElement('cem-surface');
        authoredFallback.setAttribute('label', 'Asset results');
        authoredFallback.setAttribute('empty', '');
        authoredFallback.innerHTML = `
            <h2>No assets yet</h2>
            <p>Upload an asset to begin building this collection.</p>
            <a href="#authored-upload">Upload an asset</a>
        `;
        expect(authoredFallback.childElementCount).toBe(3);
        expect(authoredFallback.textContent).toContain('No assets yet');
        expect(authoredFallback.querySelector('a')?.getAttribute('href')).toBe('#authored-upload');


        await waitForStateSelector(root, '#empty-grid > .cem-grid');

        const ordinaryHost = harness.query<HTMLElement>('cem-surface:not([empty])');
        const ordinary = harness.query<HTMLElement>('cem-surface:not([empty]) > section');
        const emptyHost = harness.query<HTMLElement>('cem-surface[empty=""]');
        const emptySurface = harness.query<HTMLElement>('cem-surface[empty=""] > section');
        const falseTokenSurface = harness.query<HTMLElement>('cem-surface[empty="false"] > section');
        const emptyStack = harness.query<HTMLDivElement>('#empty-stack > .cem-stack');
        const emptyGrid = harness.query<HTMLDivElement>('#empty-grid > .cem-grid');
        const guidance = harness.query<HTMLParagraphElement>('cem-surface[empty=""] p');
        const recovery = harness.query<HTMLAnchorElement>('cem-surface[empty=""] a');

        assertStateHostsRendered(harness.root, 'cem-surface, #empty-stack, #empty-grid');
        expect(ordinary.className).toBe('cem-surface cem-surface--default');
        expect(ordinary.getAttribute('aria-label')).toBe('Dashboard');
        expect(ordinary.hasAttribute('data-state')).toBe(false);
        expect(ordinary.children).toHaveLength(1);
        expect(ordinary.textContent?.trim()).toBe('Ready');
        expect(assertAccessibleName(emptySurface, 'Asset results')).toBe('Asset results');
        expect(emptySurface.className).toBe('cem-surface cem-surface--default');
        expect(emptySurface.getAttribute('data-state')).toBe('empty');
        expect(emptySurface.children).toHaveLength(3);
        expect(emptySurface.querySelector('h2')?.textContent?.trim()).toBe('No assets yet');
        expect(guidance.textContent?.trim()).toBe('Upload an asset to begin building this collection.');
        expect(recovery).toBeInstanceOf(HTMLAnchorElement);
        expect(recovery.getAttribute('href')).toBe('#new-asset');
        expect(assertAccessibleName(recovery, 'Upload an asset')).toBe('Upload an asset');
        expect(falseTokenSurface.getAttribute('data-state')).toBe('empty');
        expect(emptySurface.getAttribute('role')).toBeNull();
        expect(emptySurface.getAttribute('aria-live')).toBeNull();
        expect(emptySurface.getAttribute('aria-atomic')).toBeNull();
        expect(emptySurface.getAttribute('tabindex')).toBeNull();
        expect(emptySurface.querySelector('[role="status"], [role="alert"], [aria-live]')).toBeNull();
        expect(emptyStack.hasAttribute('data-state')).toBe(false);
        expect(emptyStack.getAttribute('role')).toBeNull();
        expect(emptyStack.childElementCount).toBe(0);
        expect(emptyStack.textContent?.trim()).toBe('');
        expect(emptyGrid.hasAttribute('data-state')).toBe(false);
        expect(emptyGrid.getAttribute('role')).toBeNull();
        expect(emptyGrid.childElementCount).toBe(0);
        expect(emptyGrid.textContent?.trim()).toBe('');

        await assertFocusVisible(recovery);
        emptyHost.removeAttribute('empty');
        await nextRenderFrame();
        await runtime.whenRenderSettled(emptyHost);
        await nextRenderFrame();

        const ordinaryTransition = harness.query<HTMLElement>('cem-surface[label="Asset results"] > section');
        const recoveryAfterRemoval = harness.query<HTMLAnchorElement>('cem-surface[label="Asset results"] a');
        expect(ordinaryTransition).toBe(emptySurface);
        expect(recoveryAfterRemoval).toBe(recovery);
        expect(ordinaryTransition.hasAttribute('data-state')).toBe(false);
        expect(document.activeElement).toBe(recoveryAfterRemoval);

        emptyHost.setAttribute('empty', '');
        await nextRenderFrame();
        await runtime.whenRenderSettled(emptyHost);
        await nextRenderFrame();

        const emptyTransition = harness.query<HTMLElement>('cem-surface[label="Asset results"] > section');
        const recoveryAfterAddition = harness.query<HTMLAnchorElement>('cem-surface[label="Asset results"] a');
        const snapshot = runtime.snapshotInstance(emptyHost);
        expect(emptyTransition).toBe(emptySurface);
        expect(recoveryAfterAddition).toBe(recovery);
        expect(emptyTransition.getAttribute('data-state')).toBe('empty');
        expect(document.activeElement).toBe(recoveryAfterAddition);
        expect(snapshot.slices).not.toHaveProperty('empty');
        expect(snapshot.eventPayloads).not.toHaveProperty('empty');
        expect(runtime.snapshotInstance(ordinaryHost).slices).not.toHaveProperty('empty');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
 },
});

export const BusyWorkflow = meta.story({
 name: "marks explicit busy workflow surfaces without making formatting containers loading owners",
 render: () => "\n            <cem-stack gap=\"sm\">\n                <cem-surface label=\"Ordinary workspace\">\n                    <p>Ready</p>\n                </cem-surface>\n                <cem-surface label=\"Initial workspace\" busy>\n                    <h2>Asset workspace</h2>\n                    <p class=\"loading-message\">Loading filters and results\u2026</p>\n                    <cem-stack gap=\"md\">\n                        <cem-skeleton label=\"Asset filters\"></cem-skeleton>\n                        <cem-skeleton label=\"Asset results\"></cem-skeleton>\n                    </cem-stack>\n                    <cem-surface label=\"Nested ordinary surface\">\n                        <p>Nested content</p>\n                    </cem-surface>\n                </cem-surface>\n                <cem-surface label=\"Profile workspace\" busy=\"false\">\n                    <h2>Profile workspace</h2>\n                    <cem-grid columns=\"2\" gap=\"lg\">\n                        <cem-card label=\"Profile details\">\n                            <p>Grace Hopper</p>\n                            <button type=\"button\">Edit profile</button>\n                        </cem-card>\n                        <cem-card label=\"Preferences\">\n                            <p>Daily summaries</p>\n                        </cem-card>\n                    </cem-grid>\n                </cem-surface>\n                <cem-surface label=\"Empty transition\" busy empty>\n                    <h2>No assets yet</h2>\n                    <p>Upload an asset to begin building this collection.</p>\n                    <a href=\"#new-asset\">Upload an asset</a>\n                </cem-surface>\n                <cem-stack id=\"busy-stack\" gap=\"sm\" busy>\n                    <p>Formatting stack</p>\n                </cem-stack>\n                <cem-grid id=\"busy-grid\" columns=\"2\" gap=\"md\" busy>\n                    <p>First cell</p>\n                    <p>Second cell</p>\n                </cem-grid>\n            </cem-stack>\n        ",
 play: async ({ canvasElement }) => {
        const root = canvasElement, runtime = cemStoryRuntime();
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);
        await nextRenderFrame();
        const authoredFallback = document.createElement('cem-surface');
        authoredFallback.setAttribute('label', 'Asset workspace');
        authoredFallback.setAttribute('busy', '');
        authoredFallback.innerHTML = `
            <h2>Asset workspace</h2>
            <p>Loading filters and results…</p>
            <cem-stack gap="md">
                <cem-skeleton label="Asset filters"></cem-skeleton>
                <cem-skeleton label="Asset results"></cem-skeleton>
            </cem-stack>
        `;
        expect(authoredFallback.childElementCount).toBe(3);
        expect(authoredFallback.textContent).toContain('Loading filters and results…');
        expect(authoredFallback.querySelectorAll('cem-skeleton')).toHaveLength(2);


        await waitForStateSelector(root, '#busy-grid > .cem-grid');

        const ordinary = harness.query<HTMLElement>('cem-surface[label="Ordinary workspace"] > section');
        const initialHost = harness.query<HTMLElement>('cem-surface[label="Initial workspace"]');
        const initial = harness.query<HTMLElement>('cem-surface[label="Initial workspace"] > section');
        const initialMessage = harness.query<HTMLParagraphElement>(
            'cem-surface[label="Initial workspace"] > section > .loading-message',
        );
        const initialStack = harness.query<HTMLDivElement>(
            'cem-surface[label="Initial workspace"] > section > cem-stack > .cem-stack',
        );
        const skeletons = Array.from(
            harness.root.querySelectorAll<HTMLElement>(
                'cem-surface[label="Initial workspace"] > section > cem-stack cem-skeleton > .cem-skeleton',
            ),
        );
        const nestedSurface = harness.query<HTMLElement>(
            'cem-surface[label="Initial workspace"] cem-surface[label="Nested ordinary surface"] > section',
        );
        const refreshHost = harness.query<HTMLElement>('cem-surface[label="Profile workspace"]');
        const refreshSurface = harness.query<HTMLElement>('cem-surface[label="Profile workspace"] > section');
        const refreshGrid = harness.query<HTMLDivElement>(
            'cem-surface[label="Profile workspace"] > section > cem-grid > .cem-grid',
        );
        const refreshCards = Array.from(
            harness.root.querySelectorAll<HTMLElement>('cem-surface[label="Profile workspace"] cem-card > section'),
        );
        const refreshButton = harness.query<HTMLButtonElement>('cem-surface[label="Profile workspace"] button');
        const transitionHost = harness.query<HTMLElement>('cem-surface[label="Empty transition"]');
        const transitionSurface = harness.query<HTMLElement>('cem-surface[label="Empty transition"] > section');
        const transitionRecovery = harness.query<HTMLAnchorElement>('cem-surface[label="Empty transition"] a');
        const busyStack = harness.query<HTMLDivElement>('#busy-stack > .cem-stack');
        const busyGrid = harness.query<HTMLDivElement>('#busy-grid > .cem-grid');

        assertStateHostsRendered(
            harness.root,
            'cem-surface, #busy-stack, #busy-grid, cem-surface cem-stack, cem-surface cem-grid, cem-surface cem-card, cem-surface cem-skeleton',
        );
        expect(ordinary.className).toBe('cem-surface cem-surface--default');
        expect(ordinary.getAttribute('aria-label')).toBe('Ordinary workspace');
        expect(ordinary.hasAttribute('data-state')).toBe(false);
        expect(ordinary.hasAttribute('aria-busy')).toBe(false);

        expect(assertAccessibleName(initial, 'Initial workspace')).toBe('Initial workspace');
        expect(initial.getAttribute('data-state')).toBe('loading');
        expect(initial.getAttribute('aria-busy')).toBe('true');
        expect(initialMessage.textContent?.trim()).toBe('Loading filters and results…');
        expect(initialMessage.getAttribute('role')).toBeNull();
        expect(initialMessage.getAttribute('aria-live')).toBeNull();
        expect(initialStack.getAttribute('data-gap')).toBe('md');
        expect(initialStack.hasAttribute('data-state')).toBe(false);
        expect(initialStack.hasAttribute('aria-busy')).toBe(false);
        expect(skeletons).toHaveLength(2);
        expect(skeletons.every((skeleton) => skeleton.getAttribute('aria-hidden') === 'true')).toBe(true);
        expect(nestedSurface.hasAttribute('data-state')).toBe(false);
        expect(nestedSurface.hasAttribute('aria-busy')).toBe(false);
        expect(initial.querySelector('[role="status"], [role="alert"], [aria-live]')).toBeNull();
        expect(initial.hasAttribute('inert')).toBe(false);

        expect(refreshSurface.getAttribute('data-state')).toBe('loading');
        expect(refreshSurface.getAttribute('aria-busy')).toBe('true');
        expect(refreshGrid.getAttribute('data-columns')).toBe('2');
        expect(refreshGrid.getAttribute('data-gap')).toBe('lg');
        expect(refreshCards).toHaveLength(2);
        expect(refreshCards.every((card) => !card.hasAttribute('data-state'))).toBe(true);
        expect(refreshCards.every((card) => !card.hasAttribute('aria-busy'))).toBe(true);
        expect(refreshButton.disabled).toBe(false);
        expect(refreshButton.tabIndex).toBe(0);

        expect(transitionSurface.getAttribute('data-state')).toBe('loading');
        expect(transitionSurface.getAttribute('aria-busy')).toBe('true');
        expect(transitionSurface.textContent).toContain('No assets yet');
        for (const candidate of [busyStack, busyGrid]) {
            expect(candidate.hasAttribute('data-state')).toBe(false);
            expect(candidate.hasAttribute('aria-busy')).toBe(false);
        }

        const refreshRect = refreshSurface.getBoundingClientRect();
        const refreshGridRect = refreshGrid.getBoundingClientRect();
        const refreshChildren = Array.from(refreshGrid.children);
        const refreshChildPositions = refreshChildren.map((child) => {
            const rect = child.getBoundingClientRect();
            return [rect.x, rect.y, rect.width, rect.height];
        });
        expect(refreshRect.width).toBeGreaterThan(0);
        expect(refreshRect.height).toBeGreaterThan(0);
        expect(refreshGridRect.width).toBeGreaterThan(0);
        expect(refreshGridRect.height).toBeGreaterThan(0);
        const lifecycleEvents: string[] = [];
        for (const name of ['cem-loaded', 'cem-error', 'cem-cancel']) {
            refreshHost.addEventListener(name, () => lifecycleEvents.push(name));
        }

        await assertFocusVisible(refreshButton);
        refreshHost.removeAttribute('busy');
        await nextRenderFrame();
        await runtime.whenRenderSettled(refreshHost);
        await nextRenderFrame();

        const settledSurface = harness.query<HTMLElement>('cem-surface[label="Profile workspace"] > section');
        const settledGrid = harness.query<HTMLDivElement>(
            'cem-surface[label="Profile workspace"] > section > cem-grid > .cem-grid',
        );
        const settledButton = harness.query<HTMLButtonElement>('cem-surface[label="Profile workspace"] button');
        const settledRect = settledSurface.getBoundingClientRect();
        const settledGridRect = settledGrid.getBoundingClientRect();
        const settledChildren = Array.from(settledGrid.children);
        const settledChildPositions = settledChildren.map((child) => {
            const rect = child.getBoundingClientRect();
            return [rect.x, rect.y, rect.width, rect.height];
        });
        expect(settledSurface).toBe(refreshSurface);
        expect(settledGrid).toBe(refreshGrid);
        expect(settledButton).toBe(refreshButton);
        expect(settledChildren[0]).toBe(refreshChildren[0]);
        expect(settledChildren[1]).toBe(refreshChildren[1]);
        expect(settledSurface.hasAttribute('data-state')).toBe(false);
        expect(settledSurface.hasAttribute('aria-busy')).toBe(false);
        expect([settledRect.width, settledRect.height]).toEqual([refreshRect.width, refreshRect.height]);
        expect([settledGridRect.x, settledGridRect.y, settledGridRect.width, settledGridRect.height]).toEqual([
            refreshGridRect.x,
            refreshGridRect.y,
            refreshGridRect.width,
            refreshGridRect.height,
        ]);
        expect(settledChildPositions).toEqual(refreshChildPositions);
        expect(document.activeElement).toBe(settledButton);

        refreshHost.setAttribute('busy', '');
        await nextRenderFrame();
        await runtime.whenRenderSettled(refreshHost);
        await nextRenderFrame();

        const pendingAgain = harness.query<HTMLElement>('cem-surface[label="Profile workspace"] > section');
        const pendingGrid = harness.query<HTMLDivElement>(
            'cem-surface[label="Profile workspace"] > section > cem-grid > .cem-grid',
        );
        const pendingButton = harness.query<HTMLButtonElement>('cem-surface[label="Profile workspace"] button');
        expect(pendingAgain).toBe(refreshSurface);
        expect(pendingGrid).toBe(refreshGrid);
        expect(pendingButton).toBe(refreshButton);
        expect(pendingAgain.getAttribute('data-state')).toBe('loading');
        expect(pendingAgain.getAttribute('aria-busy')).toBe('true');
        expect(document.activeElement).toBe(pendingButton);

        await assertFocusVisible(transitionRecovery);
        transitionHost.removeAttribute('busy');
        await nextRenderFrame();
        await runtime.whenRenderSettled(transitionHost);
        await nextRenderFrame();

        const settledEmpty = harness.query<HTMLElement>('cem-surface[label="Empty transition"] > section');
        const settledRecovery = harness.query<HTMLAnchorElement>('cem-surface[label="Empty transition"] a');
        expect(settledEmpty).toBe(transitionSurface);
        expect(settledRecovery).toBe(transitionRecovery);
        expect(settledEmpty.getAttribute('data-state')).toBe('empty');
        expect(settledEmpty.hasAttribute('aria-busy')).toBe(false);
        expect(document.activeElement).toBe(settledRecovery);

        for (const host of [initialHost, refreshHost, transitionHost]) {
            const snapshot = runtime.snapshotInstance(host);
            expect(snapshot.slices).not.toHaveProperty('busy');
            expect(snapshot.slices).not.toHaveProperty('loading');
            expect(snapshot.slices).not.toHaveProperty('empty');
            expect(snapshot.eventPayloads).not.toHaveProperty('busy');
            expect(snapshot.eventPayloads).not.toHaveProperty('loading');
            expect(snapshot.eventPayloads).not.toHaveProperty('empty');
        }
        expect(lifecycleEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
 },
});

async function waitForStateSelector(root: ParentNode, selector: string): Promise<Element> {
    const deadline = Date.now() + 1000;

    while (Date.now() < deadline) {
        const element = root.querySelector(selector);

        if (element) {
            return element;
        }

        await nextRenderFrame();
    }

    throw new Error(`Expected state render output matching ${selector}`);
}

function assertStateHostsRendered(root: ParentNode, selector: string): void {
    for (const host of Array.from(root.querySelectorAll<HTMLElement>(selector))) {
        assertLightDomRendered(host);
        expect(host.shadowRoot).toBeNull();
    }
}

export const WorkerRetention = meta.story({
 render: () => '<div></div>',
 play: async ({ canvasElement }) => {
        const { runtime, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-surface-${crypto.randomUUID()}`;
        await declare(declarationSource, tag);
        const host = document.createElement(tag);
        host.setAttribute('label', 'Retained content');
        host.innerHTML = "<label>Retained field <input value=\"Initial\"></label>";
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
            expect(owner.getAttribute('data-state')).toBe('loading');
            expect(owner.getAttribute('aria-busy')).toBe('true');
            if (input) { expect(host.querySelector('input')).toBe(input); expect(input.value).toBe('Edited'); expect(document.activeElement).toBe(input); }
            for (const child of authored) expect(host.contains(child)).toBe(true);

            host.removeAttribute('busy'); await runtime.whenRenderSettled(host);
            expect(owner.getAttribute('data-state')).toBe('empty');
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
        const tag = `cem-surface-${crypto.randomUUID()}`;
        await declare(declarationSource, tag);
        const host = document.createElement(tag);
        host.setAttribute('label', 'Retained content');
        host.innerHTML = "<label>Retained field <input value=\"Initial\"></label>";
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
            expect(owner.getAttribute('data-state')).toBe('loading');
            expect(owner.getAttribute('aria-busy')).toBe('true');
            if (input) { expect(host.querySelector('input')).toBe(input); expect(input.value).toBe('Edited'); expect(document.activeElement).toBe(input); }
            for (const child of authored) expect(host.contains(child)).toBe(true);

            host.removeAttribute('busy'); await runtime.whenRenderSettled(host);
            expect(owner.getAttribute('data-state')).toBe('empty');
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
 render: () => '<div class="theme-audit"><cem-surface>123.45</cem-surface></div>',
 play: async ({ canvasElement }) => {
        const container = canvasElement.querySelector<HTMLElement>('.theme-audit');
        const host = container?.querySelector<HTMLElement>('cem-surface');
        if (!container || !host) throw new Error('Missing theme audit fixture');
        await whenCemRendered(host);
        const owner = host.querySelector<HTMLElement>('[part=root]');
        if (!owner) throw new Error('Missing component root');
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            container.className = `theme-audit cem-theme-${mode}`;
            container.style.setProperty('--cem-palette-comfort', 'rgb(12, 24, 36)');
            container.style.setProperty('--cem-palette-comfort-text', 'rgb(240, 244, 248)');
            container.style.setProperty('--cem-inset-container', '23px');
            container.style.setProperty('--cem-bend-container', '7px');
            container.style.setProperty('--cem-layer-work', 'rgb(1, 2, 3) 0px 0px 0px 2px');
            container.style.setProperty('--cem-stroke-boundary', '3px');
            container.style.setProperty('--cem-typography-feature-optical-sizing', 'none');
            const style = getComputedStyle(owner);
            expect(style.backgroundColor).toBe('rgb(12, 24, 36)'); expect(style.color).toBe('rgb(240, 244, 248)');
            expect(style.boxShadow).toBe('rgb(1, 2, 3) 0px 0px 0px 2px');
            expect(style.paddingInlineStart).toBe('23px'); expect(style.borderStartStartRadius).toBe('7px'); expect(style.borderBlockStartWidth).toBe('3px');
            expect(style.fontOpticalSizing).toBe('none');
            container.style.setProperty('--cem-palette-trust', 'rgb(30, 45, 60)'); container.style.setProperty('--cem-palette-trust-text', 'rgb(245, 248, 251)');
            host.setAttribute('tone', 'trust'); await whenCemRendered(host);
            expect(getComputedStyle(owner).backgroundColor).toBe('rgb(30, 45, 60)'); expect(getComputedStyle(owner).color).toBe('rgb(245, 248, 251)');
            host.removeAttribute('tone'); await whenCemRendered(host);
        }

 },
});
