import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-dropdown.xhtml?raw';

const meta = preview.meta({
    component: 'cem-dropdown', title: 'CEM Components/cem-dropdown',
    loaders: [async () => {
        await loadCemDeclaration('cem-menu-item', await (await fetch(new URL('../cem-menu-item/cem-menu-item.xhtml', import.meta.url))).text());
        await loadCemDeclaration('cem-menu', await (await fetch(new URL('../cem-menu/cem-menu.xhtml', import.meta.url))).text());
        await loadCemDeclaration('cem-dropdown', declarationSource);
        return {};
    }],
});
async function settled(root: HTMLElement): Promise<void> {
    for (let pass = 0; pass < 4; pass++) {
        await Promise.all(Array.from(root.querySelectorAll<HTMLElement>('cem-dropdown,cem-menu,cem-menu-item'), whenCemRendered));
        await new Promise(resolve => setTimeout(resolve, 20));
    }
}
export const SourceAndProjectedBase = meta.story({
    render: () => '<section class="cem-theme-light"><cem-dropdown label="Content"><p>Any HTML</p></cem-dropdown><cem-dropdown open="false"><button slot="base">Custom trigger</button><a href="#target">Projected link</a></cem-dropdown></section>',
    play: async ({ canvasElement }) => {
        await settled(canvasElement);
        const hosts = canvasElement.querySelectorAll<HTMLElement>('cem-dropdown');
        const first = hosts[0]; const second = hosts[1];
        expect(first.getAttribute('open')).toBe('true');
        expect(first.querySelector('aside')?.hidden).toBe(false);
        expect(first.querySelector('button')?.textContent).toBe('Content');
        expect(first.querySelector('button')?.hasAttribute('aria-haspopup')).toBe(false);
        expect(second.querySelector('aside')?.hidden).toBe(true);
        expect(second.querySelector('button')?.textContent).toBe('Custom trigger');
        first.setAttribute('open', 'false'); await whenCemRendered(first);
        expect(first.querySelector('aside')?.hidden).toBe(true);
        first.setAttribute('open', 'true'); await whenCemRendered(first);
        expect(first.querySelector('aside')?.hidden).toBe(false);
        first.hidden = true; await whenCemRendered(first);
        expect(getComputedStyle(first).display).toBe('none');
    },
});
export const NestedMenu = meta.story({
    render: () => `<section class="cem-theme-light"><p>Trusted keyboard and pointer checks run in the browser test runner.</p>
        <cem-dropdown label="Commands" open="false"><cem-menu keyboard="menu" direction="column" aria-label="Commands">
          <cem-menu-item label="File"><cem-menu slot="submenu"><cem-menu-item label="New"></cem-menu-item><cem-menu-item label="More"><cem-menu slot="submenu"><cem-menu-item label="Deep"></cem-menu-item></cem-menu></cem-menu-item></cem-menu></cem-menu-item>
          <cem-menu-item href="#help" label="Help"></cem-menu-item>
        </cem-menu></cem-dropdown><button style="margin-inline-start: 25rem">Outside</button></section>`,
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        await settled(canvasElement);
        const dropdown = canvasElement.querySelector('cem-dropdown') as HTMLElement;
        const trigger = dropdown.querySelector(':scope > [part="base"] > button') as HTMLButtonElement;
        const popup = dropdown.querySelector('aside')!;
        trigger.focus(); await userEvent.keyboard('{Enter}');
        expect(popup.hidden).toBe(false);
        expect(document.activeElement?.textContent).toBe('File');
        await userEvent.keyboard('{ArrowRight}');
        expect(document.activeElement?.textContent).toBe('New');
        await userEvent.keyboard('{ArrowDown}{ArrowRight}');
        expect(document.activeElement?.textContent).toBe('Deep');
        await userEvent.keyboard('{Escape}');
        expect(document.activeElement?.textContent).toBe('More');
        await userEvent.keyboard('{Escape}');
        expect(document.activeElement?.textContent).toBe('File');
        expect(popup.hidden).toBe(false);
        await userEvent.keyboard('{Escape}');
        expect(popup.hidden).toBe(true);
        expect(document.activeElement).toBe(trigger);
        const leaf = dropdown.querySelector('a[href]')!;
        let nativeLeafActivation = false;
        leaf.addEventListener('click', event => { nativeLeafActivation = !event.defaultPrevented; event.preventDefault(); });
        await userEvent.click(trigger); await userEvent.keyboard('{End}{Enter}');
        expect(nativeLeafActivation).toBe(true);
        expect(popup.hidden).toBe(true);
        await userEvent.click(trigger); await userEvent.click(canvasElement.querySelector('section > button')!);
        expect(popup.hidden).toBe(true);
        expect(document.activeElement?.textContent).toBe('Outside');
        await userEvent.click(trigger); await userEvent.keyboard('{Tab}');
        expect(popup.hidden).toBe(true);
        expect(document.activeElement).not.toBe(trigger);
        dropdown.setAttribute('disabled', ''); await whenCemRendered(dropdown);
        expect(trigger.disabled).toBe(true);
        trigger.click(); expect(popup.hidden).toBe(true);
    },
});
