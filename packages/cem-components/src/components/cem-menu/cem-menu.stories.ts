import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-menu.xhtml?raw';

const meta = preview.meta({
    component: 'cem-menu',
    title: 'CEM Components/cem-menu',
    loaders: [async () => {
        await loadCemDeclaration('cem-menu-item', await (await fetch(new URL('../cem-menu-item/cem-menu-item.xhtml', import.meta.url))).text());
        // Retain the declaration's dependency base when registering raw markup.
        await loadCemDeclaration('cem-menu', declarationSource.replace('tag="cem-menu"', 'tag="cem-menu" xml:base="/packages/cem-components/src/components/cem-menu/cem-menu.xhtml"'));
        return {};
    }],
});
async function settled(root: HTMLElement): Promise<void> {
    for (let pass = 0; pass < 4; pass++) {
        await Promise.all(Array.from(root.querySelectorAll<HTMLElement>('cem-menu,cem-menu-item'), whenCemRendered));
        await new Promise(resolve => setTimeout(resolve, 20));
    }
}
const nested = `<cem-menu keyboard="menu" aria-label="Commands">
    <cem-menu-item label="File" href="#ignored-owner-link" expanded="true"><cem-menu slot="submenu">
        <cem-menu-item label="New"></cem-menu-item>
        <cem-menu-item label="More"><cem-menu slot="submenu"><cem-menu-item label="Deep command"></cem-menu-item></cem-menu></cem-menu-item>
        <cem-menu-item disabled label="Unavailable"></cem-menu-item>
    </cem-menu></cem-menu-item>
    <cem-menu-item label="Edit"><cem-menu slot="submenu"><cem-menu-item label="Copy"></cem-menu-item></cem-menu></cem-menu-item>
    <cem-menu-item label="Help" href="#help"></cem-menu-item>
</cem-menu>`;

export const SourceLayout = meta.story({
    render: () => '<section class="cem-theme-light"><cem-menu aria-label="Quick links"><a href="#one">One</a><button disabled>Disabled</button><button>Three</button><span>Note</span></cem-menu></section>',
    play: async ({ canvasElement }) => {
        await settled(canvasElement);
        const host = canvasElement.querySelector('cem-menu') as HTMLElement;
        const box = host.querySelector('[part="composite"]') as HTMLElement;
        expect(box.getAttribute('aria-label')).toBe('Quick links');
        expect(box.getAttribute('role')).toBe('group');
        expect(getComputedStyle(box).flexDirection).toBe('row');
        expect(getComputedStyle(box).flexWrap).toBe('wrap');
        expect(host.querySelector('a')?.hasAttribute('tabindex')).toBe(false);
        host.setAttribute('direction', 'column'); host.setAttribute('justify', 'end');
        await whenCemRendered(host);
        expect(getComputedStyle(host.querySelector('[part="composite"]')!).flexDirection).toBe('column');
        expect(getComputedStyle(host.querySelector('[part="composite"]')!).justifyContent).toBe('flex-end');
        host.hidden = true;
        expect(getComputedStyle(host).display).toBe('none');
    },
});

export const NestedInteraction = meta.story({
    render: () => `<section class="cem-theme-light"><p>Trusted keyboard and pointer checks run in the browser test runner.</p>${nested}<button style="margin-block-start: 20rem">Outside</button></section>`,
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        await settled(canvasElement);
        const root = canvasElement.querySelector('cem-menu') as HTMLElement;
        const triggers = root.querySelectorAll<HTMLButtonElement>(':scope > [part="composite"] > cem-menu-item > button');
        const file = triggers[0]; const edit = triggers[1];
        const panel = file.parentElement!.querySelector('cem-menu') as HTMLElement;
        expect(root.querySelector('[part="composite"]')?.getAttribute('role')).toBe('menubar');
        expect(panel.hidden).toBe(true);
        expect(file.getAttribute('aria-expanded')).toBe('false');
        expect(file.getAttribute('tabindex')).toBe('0');
        expect(edit.getAttribute('tabindex')).toBe('-1');
        file.focus();
        await userEvent.keyboard('{ArrowDown}');
        expect(panel.hidden).toBe(false);
        expect(file.getAttribute('aria-expanded')).toBe('true');
        expect(document.activeElement?.textContent).toBe('New');
        expect(getComputedStyle(panel.querySelector('[part="composite"]')!).flexDirection).toBe('column');
        await userEvent.keyboard('{ArrowDown}{ArrowRight}');
        expect(document.activeElement?.textContent).toBe('Deep command');
        await userEvent.keyboard('{Escape}');
        expect(document.activeElement?.textContent).toBe('More');
        await userEvent.keyboard('{Escape}');
        expect(document.activeElement).toBe(file);
        expect(panel.hidden).toBe(true);
        await userEvent.click(file);
        await whenCemRendered(file.parentElement!);
        expect(getComputedStyle(panel).position).toBe('fixed');
        expect(panel.hidden).toBe(false);
        await userEvent.click(edit);
        expect(panel.hidden).toBe(true);
        expect(document.activeElement?.textContent).toBe('Copy');
        await userEvent.keyboard('{Enter}');
        expect(edit.getAttribute('aria-expanded')).toBe('false');
        await userEvent.click(file);
        await userEvent.click(canvasElement.querySelector('section > button')!);
        expect(panel.hidden).toBe(true);
        expect(document.activeElement?.textContent).toBe('Outside');
        file.focus(); await userEvent.keyboard('{ArrowDown}{Tab}');
        expect(panel.hidden).toBe(true);
        expect(document.activeElement).not.toBe(file);
        file.focus(); await userEvent.keyboard('{End}');
        expect(document.activeElement?.textContent).toBe('Help');
        await userEvent.keyboard('{Home}e');
        expect(document.activeElement).toBe(edit);
        root.setAttribute('dir', 'rtl'); file.focus();
        await userEvent.keyboard('{ArrowLeft}');
        expect(document.activeElement).toBe(edit);
        edit.parentElement!.remove();
        await new Promise(resolve => setTimeout(resolve, 0));
        expect(root.querySelectorAll(':scope > [part="composite"] [tabindex="0"]')).toHaveLength(1);
        const help = root.querySelector('a')!;
        help.setAttribute('disabled', '');
        await new Promise(resolve => setTimeout(resolve, 20));
        expect(help.getAttribute('tabindex')).toBe('-1');
        help.removeAttribute('disabled');
        await new Promise(resolve => setTimeout(resolve, 20));
        file.focus(); await userEvent.keyboard('{End}');
        expect(document.activeElement).toBe(help);
    },
});

export const NativeActivationAndBoundaries = meta.story({
    render: () => '<section class="cem-theme-light"><p>Trusted input checks run in the browser test runner.</p><cem-menu><a href="#native">Link</a><a href="#blocked" disabled>Blocked</a><cem-menu-item label="Command"></cem-menu-item><cem-menu-item href="#leaf" label="Leaf"></cem-menu-item><cem-menu-item href="#disabled" disabled label="Disabled leaf"></cem-menu-item></cem-menu><cem-menu keyboard="menu"><button disabled>Disabled</button></cem-menu><cem-menu keyboard="menu"></cem-menu><cem-menu keyboard="menu"><button>Independent</button></cem-menu></section>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        await settled(canvasElement);
        const menus = canvasElement.querySelectorAll('cem-menu');
        const command = menus[0].querySelector('button')!;
        let activations = 0;
        command.addEventListener('click', () => activations++);
        await userEvent.click(command); await userEvent.keyboard('{Enter}'); await userEvent.keyboard(' ');
        expect(activations).toBe(3);
        expect(menus[0].querySelector('cem-menu-item[href] a')?.getAttribute('href')).toBe('#leaf');
        expect(menus[0].querySelector('cem-menu-item[disabled] a')?.hasAttribute('href')).toBe(false);
        const blocked = menus[0].querySelector('a[disabled]')!;
        const event = new MouseEvent('click', { bubbles: true, cancelable: true });
        blocked.dispatchEvent(event);
        expect(event.defaultPrevented).toBe(true);
        expect(menus[1].querySelector('[tabindex="0"]')).toBeNull();
        expect(menus[2].querySelector('[tabindex="0"]')).toBeNull();
        expect(menus[3].querySelector('button')?.getAttribute('tabindex')).toBe('0');
    },
});

export const EmptySubmenus = meta.story({
    render: () => '<section class="cem-theme-light"><p>Trusted keyboard checks run in the browser test runner.</p><cem-menu keyboard="menu"><cem-menu-item label="Empty"><cem-menu slot="submenu"></cem-menu></cem-menu-item><cem-menu-item label="All disabled"><cem-menu slot="submenu"><cem-menu-item disabled label="Unavailable"></cem-menu-item><cem-menu-item hidden label="Hidden"></cem-menu-item></cem-menu></cem-menu-item></cem-menu></section>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        await settled(canvasElement);
        const root = canvasElement.querySelector('cem-menu')!;
        const triggers = root.querySelectorAll<HTMLButtonElement>(':scope > [part="composite"] > cem-menu-item > button');
        for (const trigger of triggers) {
            trigger.focus(); await userEvent.keyboard('{ArrowDown}');
            expect(document.activeElement?.getAttribute('role')).toBe('menu');
            expect(trigger.getAttribute('aria-expanded')).toBe('true');
            await userEvent.keyboard('{Escape}');
            expect(document.activeElement).toBe(trigger);
            expect(trigger.getAttribute('aria-expanded')).toBe('false');
        }
    },
});
