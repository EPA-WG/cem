import { expect, userEvent } from 'storybook/test';
import preview, { loadCemDeclaration, loadCemComponent, whenCemRendered, nextRenderFrame } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-accordion.xhtml?raw';

const meta = preview.meta({ component: 'cem-accordion', title: 'CEM Components/cem-accordion', loaders: [async () => { await loadCemDeclaration('cem-accordion', declarationSource); await loadCemComponent('cem-expansion'); return {}; }] });
const fixture = '<div style="block-size:420px"><cem-accordion fit="fill" label="Workspace navigation"><cem-expansion layout="panel" indicator="start" label="Projects" expanded><span slot="header-actions"><button type="button" aria-label="Create project">+</button></span><p>Examples</p><input aria-label="Retained draft" value="Saved draft"></cem-expansion><cem-expansion layout="panel" indicator="start" label="Structure" expanded><p>Project files</p></cem-expansion><cem-expansion layout="panel" indicator="start" label="Output files"><p>Build outputs</p></cem-expansion></cem-accordion></div>';

export const DefaultGroup = meta.story({
    render: () => '<cem-accordion><cem-expansion layout="panel" indicator="start" label="Details"><p>Retained content</p></cem-expansion></cem-accordion>',
    play: async ({ canvasElement }) => {
        const { host } = await parts(canvasElement);
        expect(required(host, '[role=group]').getAttribute('aria-label')).toBe('Sections');
        expect(required(host, '[part=summary]').textContent).toBe('Details');
    },
});

async function parts(root: HTMLElement) {
    const host = required<HTMLElement>(root, 'cem-accordion');
    await whenCemRendered(host);
    const sections = [...host.querySelectorAll<HTMLElement>('cem-expansion')];
    await Promise.all(sections.map(whenCemRendered));
    await nextRenderFrame();
    const header = (section: HTMLElement) => required<HTMLButtonElement>(section, ':scope > [part=root] > [part=heading] > button');
    const panel = (section: HTMLElement) => required<HTMLElement>(section, ':scope > [part=root] > [part=panel]');
    return { host, sections, header, panel };
}

export const IndependentSections = meta.story({
    render: () => fixture,
    play: async ({ canvasElement }) => {
        const { host, sections, header, panel } = await parts(canvasElement);
        expect(host.querySelector('[role=group]')?.getAttribute('aria-label')).toBe('Workspace navigation');
        expect(sections.map(section => section.hasAttribute('expanded'))).toEqual([true, true, false]);
        const input = required<HTMLInputElement>(host, 'input');
        input.value = 'Unsaved edit'; input.setSelectionRange(2, 5);
        await userEvent.click(header(sections[0])); await whenCemRendered(sections[0]);
        expect(sections[1].hasAttribute('expanded')).toBe(true);
        expect(panel(sections[0]).hidden).toBe(true);
        expect(host.querySelector('input')).toBe(input);
        expect(input.value).toBe('Unsaved edit');
        await userEvent.click(header(sections[0])); await whenCemRendered(sections[0]);
        expect(host.querySelector('input')).toBe(input);
        expect([input.selectionStart, input.selectionEnd]).toEqual([2, 5]);
        const action = required<HTMLButtonElement>(host, '[aria-label="Create project"]');
        expect(action.closest('[part=header]')).toBeNull();
        await userEvent.click(action); await nextRenderFrame();
        expect(sections[0].hasAttribute('expanded')).toBe(true);
        expect(header(sections[0]).getAttribute('aria-expanded')).toBe('true');
        const marker = required<HTMLElement>(sections[0], '[part=indicator]');
        expect(marker.getBoundingClientRect().left).toBeLessThan(required<HTMLElement>(sections[0], '[part=summary]').getBoundingClientRect().left);
    },
});

export const BoundedAndAllClosed = meta.story({
    render: () => fixture,
    play: async ({ canvasElement }) => {
        const { host, sections, panel } = await parts(canvasElement);
        required<HTMLElement>(panel(sections[0]), 'p').textContent = 'Long row '.repeat(5000);
        await nextRenderFrame();
        expect(host.getBoundingClientRect().height).toBe(420);
        expect(getComputedStyle(panel(sections[0])).overflowY).toBe('auto');
        expect(panel(sections[0]).scrollHeight).toBeGreaterThan(panel(sections[0]).clientHeight);
        expect(panel(sections[1]).clientHeight).toBeGreaterThan(0);
        for (const section of sections) section.removeAttribute('expanded');
        await Promise.all(sections.map(whenCemRendered)); await nextRenderFrame();
        expect(host.getBoundingClientRect().height).toBe(420);
        expect(sections.every(section => panel(section).hidden)).toBe(true);
        const root = required<HTMLElement>(host, ':scope > [part=root]');
        expect(sections[2].getBoundingClientRect().bottom).toBeLessThan(root.getBoundingClientRect().bottom);
        required<HTMLElement>(canvasElement, ':scope > div').setAttribute('style', 'block-size:60px');
        await nextRenderFrame();
        expect(root.scrollHeight).toBeGreaterThan(root.clientHeight);
        expect(getComputedStyle(root).overflowY).toBe('auto');
    },
});

export const CollapseFocusAndTheme = meta.story({
    render: () => fixture,
    play: async ({ canvasElement }) => {
        const { host, sections, header } = await parts(canvasElement);
        const input = required<HTMLInputElement>(host, 'input');
        input.focus(); sections[0].removeAttribute('expanded');
        await whenCemRendered(sections[0]);
        expect(document.activeElement).toBe(header(sections[0]));
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            host.className = `cem-theme-${mode}`;
            const style = getComputedStyle(required(host, '[part=root]'));
            expect(style.color).not.toBe(style.backgroundColor);
            expect(host.querySelector('input')).toBe(input);
        }
        host.setAttribute('aria-label', 'Project panels'); await whenCemRendered(host);
        expect(host.querySelector('[role=group]')?.getAttribute('aria-label')).toBe('Project panels');
        host.hidden = true; await whenCemRendered(host);
        expect(getComputedStyle(host).display).toBe('none');
    },
});

function required<T extends HTMLElement = HTMLElement>(root: ParentNode, selector: string): T {
    const element = root.querySelector<T>(selector);
    if (!element) throw Error(`Missing ${selector}`);
    return element;
}
