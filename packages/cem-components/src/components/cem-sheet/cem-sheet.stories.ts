import { expect, userEvent } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, createCemStoryRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-sheet.xhtml?raw';

const meta = preview.meta({ component: 'cem-sheet', title: 'CEM Components/cem-sheet', loaders: [async () => { await loadCemDeclaration('cem-sheet', declarationSource); return {}; }] });
function required<T extends HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector);
    if (!node) throw new Error(`Missing ${selector}`);
    return node;
}

export const PersistentRegionAndNames = meta.story({
    render: () => '<section class="cem-theme-light"><cem-sheet><p>Body content</p></cem-sheet><cem-sheet label="Filters" expanded><label>Search <input value="Saved"></label></cem-sheet><cem-sheet label="Fallback" aria-labelledby="sheet-heading" aria-describedby="sheet-help"><strong slot="label" id="sheet-heading">Projected filters</strong><p id="sheet-help">Choose a filter.</p></cem-sheet></section>',
    play: async ({ canvasElement }) => {
        const hosts = [...canvasElement.querySelectorAll<HTMLElement>('cem-sheet')];
        for (const host of hosts) await whenCemRendered(host);
        expect(required(hosts[0], 'aside')).toHaveAccessibleName('Sheet');
        const host = hosts[1], owner = required(host, 'aside'), input = required<HTMLInputElement>(owner, 'input');
        expect(owner.getAttribute('role')).toBe('region');
        expect(owner.hidden).toBe(false); expect(owner).toHaveAccessibleName('Filters');
        expect(required(owner, '[part=heading]').textContent).toBe('Filters');
        input.value = 'Draft'; host.removeAttribute('expanded'); await whenCemRendered(host);
        expect(owner.hidden).toBe(false); expect(required(host, 'aside')).toBe(owner); expect(required(owner, 'input')).toBe(input); expect(input.value).toBe('Draft');
        host.setAttribute('expanded', 'false'); host.setAttribute('label', 'Renamed filters'); await whenCemRendered(host);
        expect(owner.hidden).toBe(false); expect(owner).toHaveAccessibleName('Renamed filters');
        host.setAttribute('aria-label', 'Accessible filters'); await whenCemRendered(host);
        expect(owner).toHaveAccessibleName('Accessible filters'); expect(required(owner, '[part=heading]').textContent).toBe('Renamed filters');
        expect(required(hosts[2], 'aside')).toHaveAccessibleName('Projected filters');
        expect(required(hosts[2], 'aside')).toHaveAccessibleDescription('Choose a filter.');
        expect(required(hosts[2], '[part=heading] > strong').textContent).toBe('Projected filters');
        expect(host.querySelector('style')).toBeNull(); expect(host.shadowRoot).toBeNull();
    },
});

export const TransientFocusAndKeyboard = meta.story({
    render: () => '<section class="cem-theme-light"><button type="button" aria-controls="sheet-task" aria-expanded="false">Application opener</button><cem-sheet id="sheet-task" label="Task" transient><label>Draft <input value="Saved"></label></cem-sheet><button type="button">Outside</button></section>',
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'cem-sheet'); await whenCemRendered(host);
        const owner = required(host, 'aside'), input = required<HTMLInputElement>(owner, 'input'), opener = required<HTMLButtonElement>(canvasElement, 'button');
        const events: string[] = []; const record = (event: Event) => events.push(event.type);
        for (const type of ['cem-dismiss', 'cem-open', 'cem-close']) host.addEventListener(type, record);
        expect(owner.hidden).toBe(true); opener.focus(); host.setAttribute('expanded', ''); opener.setAttribute('aria-expanded', 'true'); await whenCemRendered(host);
        expect(owner.hidden).toBe(false); expect(document.activeElement).toBe(opener);
        expect(owner.hasAttribute('aria-modal')).toBe(false); expect(owner.hasAttribute('tabindex')).toBe(false); expect(host.hasAttribute('tabindex')).toBe(false);
        expect(host.querySelector('dialog, [popover], [inert]')).toBeNull();
        await userEvent.type(input, ' draft'); const draft = input.value; const width = owner.getBoundingClientRect().width;
        host.setAttribute('label', 'Renamed task'); await whenCemRendered(host);
        expect(required(host, 'aside')).toBe(owner); expect(required(owner, 'input')).toBe(input); expect(input.value).toBe(draft); expect(document.activeElement).toBe(input);
        expect(owner.getBoundingClientRect().width).toBe(width);
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent: native } = await import('vitest/browser');
        await native.keyboard('{ArrowRight}'); expect(input.matches(':focus-visible')).toBe(true);
        await native.keyboard('{Escape}'); expect(owner.hidden).toBe(false); expect(document.activeElement).toBe(input);
        await native.keyboard('{Tab}'); expect(document.activeElement).toBe(required(canvasElement, 'section > button:last-child')); expect(owner.hidden).toBe(false);
        opener.focus(); host.removeAttribute('expanded'); opener.setAttribute('aria-expanded', 'false'); await whenCemRendered(host);
        expect(owner.hidden).toBe(true); expect(document.activeElement).toBe(opener); expect(events).toEqual([]);
        for (const type of ['cem-dismiss', 'cem-open', 'cem-close']) host.removeEventListener(type, record);
    },
    parameters: { docs: { description: { story: 'Browser-runner-only trusted keyboard checks cover native focus-visible, Escape and document tab order. The application controls visibility and focus.' } } },
});

export const PresenceFormsAndReconnectWorkerAndFallback = meta.story({
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = required(canvasElement, 'section');
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(root, fallback);
            const tag = `cem-sheet-contract-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
            const host = document.createElement(tag);
            host.innerHTML = '<form><label>Required draft <input name="draft" value="Saved" required></label><button type="reset">Reset</button></form>';
            root.append(host);
            try {
                await runtime.whenRenderSettled(host);
                const owner = required(host, 'aside'), input = required<HTMLInputElement>(owner, 'input'), form = required<HTMLFormElement>(owner, 'form');
                input.value = 'Draft';
                for (const transient of [null, '', 'false', 'true']) {
                    if (transient === null) host.removeAttribute('transient'); else host.setAttribute('transient', transient);
                    for (const expanded of [null, '', 'false', 'true']) {
                        if (expanded === null) host.removeAttribute('expanded'); else host.setAttribute('expanded', expanded);
                        await runtime.whenRenderSettled(host);
                        expect(owner.hidden).toBe(transient !== null && expanded === null);
                        expect(required(host, 'aside')).toBe(owner); expect(required(owner, 'input')).toBe(input); expect(input.value).toBe('Draft');
                        expect(new FormData(form).get('draft')).toBe('Draft');
                    }
                }
                input.value = ''; expect(form.checkValidity()).toBe(false); input.value = 'Draft'; expect(form.checkValidity()).toBe(true);
                await userEvent.click(required(form, 'button')); expect(input.value).toBe('Saved');
                input.value = 'Reconnected'; host.remove(); root.append(host); await runtime.whenRenderSettled(host);
                expect(required(host, 'aside')).toBe(owner); expect(required(owner, 'input')).toBe(input); expect(input.value).toBe('Reconnected');
                expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { host.remove(); declaration.remove(); scope.dispose(); }
        }
    },
});
