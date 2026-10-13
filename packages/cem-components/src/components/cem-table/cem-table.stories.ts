import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, loadCemWorkflowDependencies, whenCemRendered, nextRenderFrame, assertAriaReferenceIntegrity } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-table.xhtml?raw';
const meta = preview.meta({ component: 'cem-table', title: 'CEM Components/cem-table', loaders: [async () => { await loadCemDeclaration('cem-table', declarationSource); await loadCemWorkflowDependencies(); return {}; }] });

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
