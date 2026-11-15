import { expect, within, waitFor } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, createCemStoryRuntime, nextRenderFrame, cemComponentDeclarationSource } from '../../../../cem-elements/.storybook/preview.js';
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
        const { userEvent, page } = await import('vitest/browser');
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
        await page.getByRole('menuitem', { name: 'File', exact: true }).click();
        await whenCemRendered(file.parentElement!);
        expect(getComputedStyle(panel).position).toBe('fixed');
        expect(panel.hidden).toBe(false);
        await page.getByRole('menuitem', { name: 'Edit', exact: true }).click();
        expect(panel.hidden).toBe(true);
        expect(document.activeElement?.textContent).toBe('Copy');
        await userEvent.keyboard('{Enter}');
        expect(edit.getAttribute('aria-expanded')).toBe('false');
        await page.getByRole('menuitem', { name: 'File', exact: true }).click();
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

function required<T extends HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector);
    if (!node) throw new Error(`Missing ${selector}`);
    return node;
}

async function nativeRoleLifecycle(root: HTMLElement, fallback: boolean) {
    const { runtime, scope, declare } = createCemStoryRuntime(root, fallback);
    const suffix = crypto.randomUUID(), menuTag = `cem-role-menu-${suffix}`, itemTag = `cem-role-item-${suffix}`;
    const itemDeclaration = await declare(await cemComponentDeclarationSource('cem-menu-item'), itemTag);
    // The isolated scope registers the canonical dependency under its own tag.
    const source = declarationSource.replace('{cem-element @tag=cem-menu-item @capability=action-control @src="../cem-menu-item/cem-menu-item.xhtml#cem-menu-item"}', '');
    const declaration = await declare(source, menuTag);
    const declarations = [declaration, itemDeclaration];
    const container = document.createElement('section');
    container.innerHTML = `<${menuTag} aria-label="Modes">
        <${itemTag} label="Command"></${itemTag}>
        <${itemTag} label="Destination" href="#destination"></${itemTag}>
        <${itemTag} label="Disabled destination" href="#blocked" disabled></${itemTag}>
        <${itemTag} label="More"><${menuTag} slot="submenu"><${itemTag} label="Nested command"></${itemTag}></${menuTag}></${itemTag}>
        <a href="#peer" role="link" tabindex="0">Authored peer</a>
    </${menuTag}><${itemTag} label="Standalone"></${itemTag}>
    <${itemTag} label="Standalone destination" href="#standalone"></${itemTag}>
    <${menuTag} aria-label="Other modes"></${menuTag}>`;
    root.append(container);
    const menu = required<HTMLElement>(container, menuTag);
    const settle = async () => {
        for (let pass = 0; pass < 4; pass++) {
            await runtime.whenRenderSettled(menu);
            await Promise.all([...container.querySelectorAll<HTMLElement>(`${menuTag},${itemTag}`)].map(node => runtime.whenRenderSettled(node)));
            await nextRenderFrame();
        }
    };
    try {
        await settle();
        const commandHost = required<HTMLElement>(menu, `${itemTag}[label="Command"]`);
        const command = required<HTMLButtonElement>(commandHost, 'button');
        const destination = required<HTMLAnchorElement>(menu, `${itemTag}[href="#destination"] a`);
        const more = required<HTMLButtonElement>(menu, `${itemTag}[label="More"] > button`);
        const nested = required<HTMLElement>(menu, `${menuTag}[slot="submenu"]`);
        const nestedBox = required<HTMLElement>(nested, '[part=composite]');
        const nestedButton = required<HTMLButtonElement>(nested, 'button');
        const standalone = required<HTMLElement>(container, `${itemTag}[label="Standalone"]`);
        const native = () => {
            expect(within(menu).getByRole('group', { name: 'Modes', exact: true })).toBe(menu.querySelector('[part=composite]'));
            expect(within(menu).getByRole('button', { name: commandHost.getAttribute('label') ?? '', exact: true })).toBe(command);
            expect(within(menu).getByRole('link', { name: 'Destination', exact: true })).toBe(destination);
            expect(command.hasAttribute('role')).toBe(false);
            expect(destination.hasAttribute('role')).toBe(false);
            expect(command.tabIndex).toBe(0); expect(destination.tabIndex).toBe(0);
            expect(more.hasAttribute('aria-haspopup')).toBe(false);
            expect(nestedBox.getAttribute('role')).toBe('group');
            expect(nestedButton.hasAttribute('role')).toBe(false);
            expect(within(standalone).getByRole('button', { name: 'Standalone', exact: true })).toBe(standalone.querySelector('button'));
        };
        native();
        expect(within(container).getByRole('link', { name: 'Standalone destination', exact: true }).getAttribute('href')).toBe('#standalone');
        expect(required(menu, `${itemTag}[disabled] a`).hasAttribute('href')).toBe(false);
        expect(required(menu, `${itemTag}[disabled] a`).tabIndex).toBe(-1);
        if (import.meta.env.MODE === 'test') {
            const { userEvent } = await import('vitest/browser');
            let activations = 0; command.addEventListener('click', () => activations++);
            command.focus(); await userEvent.keyboard('{ArrowRight}'); expect(document.activeElement).toBe(command);
            await userEvent.keyboard('{Enter}'); await settle(); expect(activations).toBe(1);
            await userEvent.keyboard('{Tab}'); expect(document.activeElement).toBe(destination);
        }
        for (const mode of ['menu', 'native', 'menu', null]) {
            if (mode === null) menu.removeAttribute('keyboard'); else menu.setAttribute('keyboard', mode);
            await settle();
            if (mode === 'menu') {
                await waitFor(() => expect(command.getAttribute('role')).toBe('menuitem'));
                expect(within(menu).getByRole('menubar', { name: 'Modes', exact: true })).toBe(menu.querySelector('[part=composite]'));
                expect(destination.getAttribute('role')).toBe('menuitem');
                expect(nestedBox.getAttribute('role')).toBe('menu'); expect(nestedButton.getAttribute('role')).toBe('menuitem');
                expect(more.getAttribute('aria-haspopup')).toBe('menu');
                commandHost.setAttribute('label', 'Updated command'); await settle();
                expect(within(menu).getByRole('menuitem', { name: 'Updated command', exact: true })).toBe(command);
                commandHost.classList.toggle('consumer-class'); await settle();
                expect(command.getAttribute('role')).toBe('menuitem');
                more.click();
                await waitFor(() => expect(document.activeElement).toBe(nestedButton));
            } else {
                await waitFor(native);
                expect(nested.hidden).toBe(true);
                expect(document.activeElement).toBe(more);
            }
        }
        const peer = required<HTMLAnchorElement>(menu, '[href="#peer"]');
        expect(peer.getAttribute('role')).toBe('link'); expect(peer.getAttribute('tabindex')).toBe('0');
        menu.setAttribute('keyboard', 'menu'); await settle();
        const other = required<HTMLElement>(container, `${menuTag}[aria-label="Other modes"]`);
        required(other, '[part=composite]').append(commandHost); await settle();
        await waitFor(() => expect(within(other).getByRole('button', { name: 'Updated command', exact: true })).toBe(command));
        other.setAttribute('keyboard', 'menu'); await settle();
        expect(command.getAttribute('role')).toBe('menuitem');
        container.append(commandHost); await runtime.whenRenderSettled(commandHost);
        await waitFor(() => expect(within(commandHost).getByRole('button', { name: 'Updated command', exact: true })).toBe(command));
        expect(command.hasAttribute('role')).toBe(false); expect(command.tabIndex).toBe(0);
        menu.remove();
        expect(destination.hasAttribute('role')).toBe(false);
        container.prepend(menu); await settle();
        expect(destination.getAttribute('role')).toBe('menuitem');
        more.click(); await waitFor(() => expect(document.activeElement).toBe(nestedButton));
        const moreHost = required<HTMLElement>(menu, `${itemTag}[label="More"]`);
        container.append(moreHost); await runtime.whenRenderSettled(moreHost);
        await waitFor(() => expect(more.hasAttribute('role')).toBe(false));
        for (const name of ['aria-haspopup', 'aria-controls', 'aria-expanded']) expect(more.hasAttribute(name)).toBe(false);
        const producerTag = `cem-role-producer-${suffix}`;
        declarations.push(await declare(`<cem-element><template type="text/cem-ml">
            {slice @name=keyboard | native}
            {button @type=button @value=menu @slice=keyboard @slice-event=click @slice-value="$target.value" | Use menu}
            {button @type=button @value=native @slice=keyboard @slice-event=click @slice-value="$target.value" | Use native}
            {${menuTag} @aria-label="Produced modes" @keyboard="{$datadom.slices.keyboard}" |
                {${itemTag} @label="Produced command"}{${itemTag} @label="Produced destination" @href=#produced}}
            </template></cem-element>`, producerTag));
        const producer = document.createElement(producerTag); container.append(producer);
        await runtime.whenRenderSettled(producer); await settle();
        const producedMenu = required<HTMLElement>(producer, menuTag);
        const producedCommand = required<HTMLButtonElement>(producedMenu, 'button');
        for (const mode of ['menu', 'native', 'menu', 'native']) {
            within(producer).getByRole('button', { name: `Use ${mode}`, exact: true }).click();
            await runtime.whenRenderSettled(producer); await settle();
            await waitFor(() => expect(within(producedMenu).getByRole(mode === 'menu' ? 'menuitem' : 'button', { name: 'Produced command', exact: true })).toBe(producedCommand));
        }
        expect(runtime.diagnosticsFor(menu).filter(item => item.severity === 'error' || item.severity === 'fatal')).toEqual([]);
    } finally { container.remove(); declarations.forEach(node => node.remove()); scope.dispose(); }
}

export const NativeRolesWorker = meta.story({
    name: 'Native/composite role lifecycle (worker; trusted keys in browser runner)',
    render: () => '<section></section>', play: async ({ canvasElement }) => nativeRoleLifecycle(canvasElement, false),
});
export const NativeRolesFallback = meta.story({
    name: 'Native/composite role lifecycle (fallback; trusted keys in browser runner)',
    render: () => '<section></section>', play: async ({ canvasElement }) => nativeRoleLifecycle(canvasElement, true),
});
