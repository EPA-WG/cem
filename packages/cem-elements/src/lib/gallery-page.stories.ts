import { expect, userEvent, waitFor } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, whenCemSourceRendered, createCemStoryRuntime, storybookCemRuntime } from '../../.storybook/preview.js';

const pageUrl = new URL('../../../cem-components/playgrounds/cem-gallery-page.xhtml?no-inline', import.meta.url).href;
const navigationUrl = new URL('../../../cem-components/playgrounds/cem-gallery-nav.xhtml?no-inline', import.meta.url).href;
let source = '';
const meta = preview.meta({
    title: 'CEM Elements/Gallery Page',
    component: 'cem-gallery-page',
    globals: { cemTheme: 'light' },
    loaders: [async () => {
        const response = await fetch(pageUrl);
        if (!response.ok) throw new Error('Gallery page source could not be loaded');
        source = (await response.text()).replace('./cem-gallery-nav.xhtml#cem-gallery-nav', `${navigationUrl}#cem-gallery-nav`);
        await loadCemDeclaration('cem-gallery-page', source); return {};
    }],
});

export const PersistentPageTheme = meta.story({
    render: () => '<cem-gallery-page heading="Theme persistence"><form><label>Retained draft <input name="draft" value="Draft"></label></form>'
        + '<section class="cem-theme-contrast-light" style="background:var(--cem-palette-comfort);color:var(--cem-palette-comfort-text)">Fixed contrast light</section>'
        + '<cem-theme-switch storage-key="story-gallery-example">Independent example</cem-theme-switch></cem-gallery-page>',
    beforeEach: () => {
        localStorage.setItem('cem-playground-theme', 'cem-theme-dark');
        localStorage.setItem('story-gallery-example', 'cem-theme-contrast-light');
    },
    play: async ({ canvasElement }) => {
        const page = required(canvasElement, 'cem-gallery-page');
        await ready(page);
        await expect(page).toHaveAttribute('data-theme', 'cem-theme-dark');
        await expect(pageSwitch(page)).toHaveAttribute('value', 'cem-theme-dark');
        const input = required<HTMLInputElement>(page, 'input[name=draft]');
        input.value = 'Edited';
        const fixed = required(page, 'section'), fixedPaint = [getComputedStyle(fixed).backgroundColor, getComputedStyle(fixed).color];
        const example = required(page, 'cem-theme-switch[storage-key="story-gallery-example"]');
        await whenCemRendered(example);
        for (const theme of ['contrast-dark', 'native', 'light', 'contrast-light', 'dark']) {
            await choose(pageSwitch(page), theme);
            await waitFor(() => expect(page).toHaveAttribute('data-theme', `cem-theme-${theme}`));
            await waitFor(() => expect(pageSwitch(page)).toHaveAttribute('value', `cem-theme-${theme}`));
            expect(localStorage.getItem('cem-playground-theme')).toBe(`cem-theme-${theme}`);
            expect(page.querySelector('input[name=draft]')).toBe(input);
            expect(new FormData(required<HTMLFormElement>(page, 'form')).get('draft')).toBe('Edited');
            expect([getComputedStyle(fixed).backgroundColor, getComputedStyle(fixed).color]).toEqual(fixedPaint);
            expect(localStorage.getItem('story-gallery-example')).toBe('cem-theme-contrast-light');
            await expect(example).toHaveAttribute('data-theme', 'cem-theme-contrast-light');
        }
        page.remove();
        const restored = document.createElement('cem-gallery-page'); canvasElement.append(restored);
        await ready(restored);
        await expect(restored).toHaveAttribute('data-theme', 'cem-theme-dark');
    },
});

export const LiveDefaultsAndInvalidValues = meta.story({
    render: () => '<cem-gallery-page storage-key="story-gallery-live" heading="First"></cem-gallery-page><cem-gallery-page storage-key="story-gallery-live" heading="Second"></cem-gallery-page>',
    beforeEach: () => { localStorage.removeItem('story-gallery-live'); },
    play: async ({ canvasElement }) => {
        const pages = [...canvasElement.querySelectorAll<HTMLElement>('cem-gallery-page')];
        for (const page of pages) await ready(page);
        expect(localStorage.getItem('story-gallery-live')).toBeNull();
        for (const [value, expected] of [['unrecognized', 'light'], ['cem-theme-contrast-dark', 'contrast-dark'], [null, 'light']] as const) {
            if (value === null) localStorage.removeItem('story-gallery-live');
            else localStorage.setItem('story-gallery-live', value);
            for (const page of pages) {
                await waitFor(() => expect(page).toHaveAttribute('data-theme', `cem-theme-${expected}`));
                await waitFor(() => expect(pageSwitch(page)).toHaveAttribute('value', `cem-theme-${expected}`));
            }
            expect(localStorage.getItem('story-gallery-live')).toBe(value);
        }
        await choose(pageSwitch(pages[0]), 'dark');
        await waitFor(() => expect(pages[1]).toHaveAttribute('data-theme', 'cem-theme-dark'));
    },
});

export const NestedExampleThemes = meta.story({
    render: () => '<cem-gallery-page storage-key="story-gallery-matrix">'
        + ['light', 'dark', 'contrast-light', 'contrast-dark', 'native'].map(theme =>
            `<section class="cem-theme-${theme}" data-gallery-theme="${theme}"><cem-icon-button selected image="★" label="Independent choice"></cem-icon-button></section>`).join('')
        + '</cem-gallery-page>',
    beforeEach: () => { localStorage.removeItem('story-gallery-matrix'); },
    play: async ({ canvasElement }) => {
        const page = required(canvasElement, 'cem-gallery-page'); await ready(page);
        await expect(page).toHaveAttribute('data-cem-density', 'super-compact');
        await expect(page).toHaveAttribute('data-cem-spacing', 'normal');
        const content = required(page, ':scope > [part=content]');
        await expect(content).toHaveAttribute('data-cem-density', 'normal');
        await expect(content).toHaveAttribute('data-cem-spacing', 'normal');
        const fixed = [...page.querySelectorAll<HTMLElement>('section[data-gallery-theme]')];
        const paint = () => fixed.map(section => {
            const style = getComputedStyle(section), control = getComputedStyle(required(section, 'button'));
            return [style.backgroundColor, style.color, style.colorScheme, control.backgroundColor, control.color, control.boxShadow];
        });
        const baseline = paint();
        for (const theme of ['dark', 'contrast-light', 'contrast-dark', 'native', 'light']) {
            await choose(pageSwitch(page), theme);
            await waitFor(() => expect(page).toHaveAttribute('data-theme', `cem-theme-${theme}`));
            expect(paint()).toEqual(baseline);
            for (const section of fixed) {
                const control = required(section, 'button');
                expect(getComputedStyle(control).getPropertyValue('--cem-coupling-zone-min').trim()).toBe('3rem');
                expect(control.getBoundingClientRect().height).toBeGreaterThanOrEqual(48);
            }
            const trigger = required(pageSwitch(page), '[part=trigger] button');
            const triggerStyle = getComputedStyle(trigger);
            expect(triggerStyle.getPropertyValue('--cem-coupling-zone-min').trim()).toBe('0px');
            expect(triggerStyle.getPropertyValue('--cem-control-height').trim()).toBe('0px');
            expect(triggerStyle.getPropertyValue('--cem-control-padding-y').trim()).toBe('0px');
            expect(triggerStyle.minBlockSize).toBe('0px');
            expect(triggerStyle.paddingBlockStart).toBe('0px');
            expect(triggerStyle.paddingBlockEnd).toBe('0px');
            expect(triggerStyle.borderTopWidth).toBe('0px');
            const icon = required(trigger, 'cem-icon');
            expect(Math.abs(trigger.getBoundingClientRect().height - icon.getBoundingClientRect().height)).toBeLessThan(0.1);
            expect(trigger.getBoundingClientRect().height).toBeLessThan(48);
        }
        const trigger = required(pageSwitch(page), '[part=trigger] button');
        page.setAttribute('data-cem-density', 'normal'); await whenCemRendered(page);
        expect(page.querySelector('cem-theme-switch[part=theme] [part=trigger] button')).toBe(trigger);
        expect(trigger.getBoundingClientRect().height).toBeGreaterThanOrEqual(48);
        page.setAttribute('data-cem-density', 'super-compact'); await whenCemRendered(page);
        expect(getComputedStyle(trigger).minBlockSize).toBe('0px');
        expect(getComputedStyle(trigger).paddingBlockStart).toBe('0px');
        expect(trigger.getBoundingClientRect().height).toBeLessThan(48);
        expect(paint()).toEqual(baseline);
    },
});

export const IndependentExampleSelection = meta.story({
    render: () => '<cem-gallery-page storage-key="story-gallery-ui"><cem-theme-switch storage-key="story-gallery-example-selection">Example</cem-theme-switch></cem-gallery-page>',
    beforeEach: () => {
        localStorage.removeItem('story-gallery-ui'); localStorage.removeItem('story-gallery-example-selection');
        localStorage.setItem('cem-theme', 'cem-theme-native');
    },
    play: async ({ canvasElement }) => {
        const page = required(canvasElement, 'cem-gallery-page'); await ready(page);
        const example = required(page, 'cem-theme-switch[storage-key="story-gallery-example-selection"]');
        await choose(example, 'dark');
        await expect(page).toHaveAttribute('data-theme', 'cem-theme-light');
        expect(localStorage.getItem('story-gallery-ui')).toBeNull();
        await choose(pageSwitch(page), 'contrast-dark');
        await waitFor(() => expect(page).toHaveAttribute('data-theme', 'cem-theme-contrast-dark'));
        await expect(example).toHaveAttribute('data-theme', 'cem-theme-dark');
        expect(localStorage.getItem('story-gallery-example-selection')).toBe('cem-theme-dark');
        expect(localStorage.getItem('cem-theme')).toBe('cem-theme-native');
    },
});

export const NavigationAndStorageKey = meta.story({
    render: () => '<cem-gallery-page heading="Custom heading" index-href="./components.html" storage-key="story-gallery-first"></cem-gallery-page>',
    beforeEach: () => {
        localStorage.setItem('story-gallery-first', 'cem-theme-dark');
        localStorage.setItem('story-gallery-second', 'cem-theme-native');
    },
    play: async ({ canvasElement }) => {
        const page = required(canvasElement, 'cem-gallery-page'); await ready(page);
        await expect(required(page, 'h1')).toHaveTextContent('Custom heading');
        const link = required(page, 'a[part=index], cem-icon-button[part=index] a');
        await expect(link).toHaveAttribute('href', './components.html');
        page.setAttribute('heading', 'Updated heading'); page.setAttribute('index-href', './index.html');
        page.setAttribute('storage-key', 'story-gallery-second');
        await waitFor(() => expect(page).toHaveAttribute('data-theme', 'cem-theme-native'));
        await waitFor(() => expect(pageSwitch(page)).toHaveAttribute('storage-key', 'story-gallery-second'));
        await waitFor(() => expect(required(page, 'h1')).toHaveTextContent('Updated heading'));
        await waitFor(() => expect(link).toHaveAttribute('href', './index.html'));
        localStorage.setItem('story-gallery-first', 'cem-theme-light');
        await whenCemRendered(page);
        await expect(page).toHaveAttribute('data-theme', 'cem-theme-native');
        await choose(pageSwitch(page), 'contrast-light');
        expect(localStorage.getItem('story-gallery-second')).toBe('cem-theme-contrast-light');
        expect(localStorage.getItem('story-gallery-first')).toBe('cem-theme-light');
    },
});

export const WorkerAndFallbackProjection = meta.story({
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const key = `story-gallery-${crypto.randomUUID()}`, tag = `gallery-${crypto.randomUUID()}`;
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const declaration = await declare(source, tag);
            const page = document.createElement(tag); page.setAttribute('storage-key', key);
            page.innerHTML = '<form><label>Draft <input name="draft" value="Draft"></label></form>';
            canvasElement.append(page);
            try {
                await runtime.whenRenderSettled(page); await ready(page);
                const input = required<HTMLInputElement>(page, 'input'); input.value = 'Edited';
                await choose(pageSwitch(page), 'dark');
                await waitFor(() => expect(page).toHaveAttribute('data-theme', 'cem-theme-dark'));
                expect(page.querySelector('input')).toBe(input);
                expect(new FormData(required<HTMLFormElement>(page, 'form')).get('draft')).toBe('Edited');
                expect(runtime.diagnosticsFor(page)).toEqual([]);
            } finally { page.remove(); declaration.remove(); scope.dispose(); localStorage.removeItem(key); }
        }
    },
});

export const InertNestedDeclarations = meta.story({
    render: () => '<cem-gallery-page storage-key="story-gallery-nested"><template>'
        + '<cem-demo-element legend="Nested property form"><template slot="source"><cem-element><template type="text/cem-ml">'
        + '{slice @name=draft | Draft}{label | Retained editor {input @name=draft @value={datadom.slices.draft} @slice=draft @slice-event=input @slice-value="$target.value"}}'
        + '</template></cem-element></template></cem-demo-element></template></cem-gallery-page>',
    beforeEach: () => { localStorage.removeItem('story-gallery-nested'); },
    play: async ({ canvasElement }) => {
        const page = required(canvasElement, 'cem-gallery-page'); await ready(page);
        await waitFor(() => expect(page.querySelector('cem-demo-element [slot=demo] input')).not.toBeNull(), { timeout: 5000 });
        const input = required<HTMLInputElement>(page, 'cem-demo-element [slot=demo] input');
        const declaration = required(page, 'cem-demo-element [slot=demo] cem-element');
        await userEvent.clear(input); await userEvent.type(input, 'Edited');
        for (const theme of ['dark', 'native', 'light']) {
            await choose(pageSwitch(page), theme);
            await waitFor(() => expect(page).toHaveAttribute('data-theme', `cem-theme-${theme}`));
            expect(page.querySelector('cem-demo-element [slot=demo] input')).toBe(input);
            expect(input.value).toBe('Edited');
            expect(page.querySelector('cem-demo-element [slot=demo] cem-element')).toBe(declaration);
            expect(storybookCemRuntime().diagnosticsFor(declaration)).toEqual([]);
        }
    },
});

function pageSwitch(page: HTMLElement): HTMLElement {
    return required(page, ':scope > cem-gallery-nav[part=navigation] cem-theme-switch[part=theme]');
}
async function ready(page: HTMLElement): Promise<void> {
    await whenCemRendered(page);
    await whenCemSourceRendered(page);
    await waitFor(() => {
        if (page.querySelectorAll(':scope > cem-gallery-nav[part=navigation] cem-menu button').length !== 5) {
            throw new Error(`Gallery navigation is not ready: ${page.outerHTML.slice(0, 1800)} ${JSON.stringify(storybookCemRuntime().diagnosticsFor(page))}`);
        }
    }, { timeout: 5000 });
    await Promise.all([...page.querySelectorAll<HTMLElement>('cem-gallery-nav, cem-theme-switch, cem-icon-button, cem-icon, cem-dropdown, cem-menu')].map(whenCemRendered));
}
async function choose(control: HTMLElement, theme: string): Promise<void> {
    await whenCemRendered(control);
    await waitFor(() => expect(control.querySelectorAll('cem-menu button')).toHaveLength(5));
    await userEvent.click(required(control, '[part=trigger] button'));
    await userEvent.click(required(control, `cem-menu cem-icon-button[value="cem-theme-${theme}"] [part=glyph]`));
    await whenCemRendered(control);
}
function required<T extends HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector);
    if (!node) throw new Error(`Missing ${selector}`);
    return node;
}
