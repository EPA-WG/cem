import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, loadCemWorkflowDependencies, whenCemRendered, nextRenderFrame, assertAriaReferenceIntegrity } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-table.xhtml?raw';
const meta = preview.meta({ component: 'cem-table', title: 'CEM Components/cem-table', loaders: [async () => { await loadCemDeclaration('cem-table', declarationSource); await loadCemWorkflowDependencies(); return {}; }] });

export const SuperCompactRows = meta.story({
    render: () => '<cem-table label="Dense records" data-cem-density="super-compact"><div role="row"><strong role="columnheader">Name</strong><strong role="columnheader">Count</strong></div><div role="row"><span role="cell">Ada</span><span role="cell">12</span></div><div role="row"><span role="cell">Grace</span><span role="cell">24</span></div></cem-table>',
    play: async ({ canvasElement }) => {
        const host = requiredElement<HTMLElement>(canvasElement, 'cem-table'); await whenCemRendered(host);
        const rows = [...host.querySelectorAll<HTMLElement>('[role=row]')];
        expect(host.querySelectorAll('[role=columnheader]')).toHaveLength(2);
        expect(host.querySelectorAll('[role=cell]')).toHaveLength(4);
        const style = getComputedStyle(rows[0]);
        expect(style.getPropertyValue('--cem-coupling-zone-min').trim()).toBe('0px');
        expect(style.getPropertyValue('--cem-control-padding-y').trim()).toBe('0px');
        expect(style.paddingBlockStart).toBe('0px');
        expect(style.paddingBlockEnd).toBe('0px');
        expect(style.paddingInlineStart).toBe('0px');
        const line = Number.parseFloat(style.lineHeight);
        expect(Math.abs(rows[0].getBoundingClientRect().height - line)).toBeLessThan(0.1);
        expect(Math.abs(rows[1].getBoundingClientRect().top - rows[0].getBoundingClientRect().bottom)).toBeLessThan(0.1);
        const cells = [...host.querySelectorAll<HTMLElement>('[role=cell]')];
        expect(cells[0].getBoundingClientRect().left).toBe(cells[2].getBoundingClientRect().left);
        expect(cells[1].getBoundingClientRect().left).toBe(cells[3].getBoundingClientRect().left);
        host.setAttribute('label', 'Updated dense records'); await whenCemRendered(host);
        expect(host.querySelector('[role=row]')).toBe(rows[0]);
    },
});

export const PassiveTable = meta.story({
 render: () => '<cem-table label="Records"><div role="row"><span role="cell">First</span><span role="cell"><input aria-label="Note" value="default"></span></div></cem-table><cem-table></cem-table>',
 play: async ({ canvasElement }) => {
    const [host, empty] = [...canvasElement.querySelectorAll<HTMLElement>('cem-table')];
    await whenCemRendered(host); await whenCemRendered(empty);
    const owner = requiredElement<HTMLElement>(host, '[part=root]');
    expect(owner).toHaveAccessibleName('Records'); expect(owner.getAttribute('role')).toBe('table');
    expect(owner.hasAttribute('tabindex')).toBe(false); expect(host.hasAttribute('tabindex')).toBe(false);
    expect(empty.querySelector('[role=cell]')?.textContent).toBe('No rows');
    const input = requiredElement<HTMLInputElement>(host, 'input'); input.value = 'edited'; input.focus();
    const events: string[] = []; for (const type of ['input','change','cem-sort','cem-page']) host.addEventListener(type, () => events.push(type));
    host.setAttribute('label','Renamed'); host.setAttribute('busy','false'); host.setAttribute('empty','false'); await whenCemRendered(host);
    expect(owner).toHaveAccessibleName('Renamed'); expect(host.querySelector('[part=root]')).toBe(owner);
    expect(host.querySelector('input')).toBe(input); expect(input.value).toBe('edited'); expect(document.activeElement).toBe(input);
    expect(owner.hasAttribute('aria-busy')).toBe(false); expect(owner.hasAttribute('data-state')).toBe(false); expect(events).toEqual([]);
    expect(() => assertAriaReferenceIntegrity(canvasElement)).not.toThrow();
 },
});

export const InheritedThemeContract = meta.story({
 render: () => '<section><cem-table label="Audit" length="120" page-size="25"><p>Content</p></cem-table></section>',
 play: async ({ canvasElement }) => {
    const container = requiredElement<HTMLElement>(canvasElement, 'section');
    const host = requiredElement<HTMLElement>(container, 'cem-table'); await whenCemRendered(host);
    const owner = requiredElement<HTMLElement>(host, '[part=root]');
    for (const mode of ['light','dark','contrast-light','contrast-dark','native']) {
        container.className = `cem-theme-${mode}`;
        container.style.cssText = '--cem-typography-data-font-size: 19px; --cem-typography-data-line-height: 1.7; --cem-typography-data-font-variant-numeric: lining-nums proportional-nums; --cem-coupling-zone-min: 52px; --cem-table-row-height: 56px; --cem-control-height: 54px; --cem-icon-button-size: 58px; --cem-gap-related: 13px; --cem-action-contextual-default-background: rgb(11, 31, 51); --cem-action-contextual-default-text: rgb(241, 242, 243); --cem-palette-comfort: rgb(23, 33, 43); --cem-palette-comfort-text: rgb(231, 232, 233);';
        await nextRenderFrame();
        expect(getComputedStyle(owner).fontSize).toBe('19px');
        expect(getComputedStyle(owner).lineHeight).toBe('32.3px');
        expect(getComputedStyle(owner).fontVariantNumeric).toBe('lining-nums proportional-nums');
        expect(host.querySelector('[part=root]')).toBe(owner);
    }
 },
});

function requiredElement<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector);
    if (!node) throw new Error(`Missing ${selector}`);
    return node;
}
