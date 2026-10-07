import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';

export default { title: 'CEM Elements/Surface References', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
function required<T extends Element>(root: ParentNode, selector: string): T {
    const target = root.querySelector<T>(selector);
    if (!target) throw new Error(`Missing fixture ${selector}`);
    return target;
}
function declare(root: HTMLElement, runtime: CemElementRuntime, declarationTag: string, tag: string, source: string, capability?: string): HTMLElement {
    const declaration = document.createElement(declarationTag); declaration.setAttribute('tag', tag);
    if (capability) declaration.setAttribute('capability', capability);
    const template = document.createElement('template'); template.setAttribute('type', 'text/cem-ml'); template.textContent = source;
    declaration.append(template); root.append(declaration); runtime.registerDeclaration(declaration); return declaration;
}
export const NativeFocusGeometryAndLiveEligibility: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = required<HTMLElement>(canvasElement, 'section');
        for (const fallback of [false, true]) {
            const scope = createCemDeclarationScope({ document });
            const suffix = crypto.randomUUID(), popup = `cem-focus-popup-${suffix}`, card = `cem-focus-card-${suffix}`, declarationTag = `declaration-${suffix}`;
            const runtime = new CemElementRuntime({ declarationTag, declarationScope: scope,
                ...(fallback ? { processingWorkerFactory: () => { throw new Error('fixture fallback'); } } : {}) }); runtime.install(window);
            const declarations = [
                declare(root, runtime, declarationTag, popup, '{attribute @name=open | false}{div @part=base | {button @type=button | Open}}{div @part=popup @style="width:180px;height:100px" | {button @type=button | Fallback}{slot}}', 'popup'),
                declare(root, runtime, declarationTag, card, `{cem:variable @name=entry @select='data:read("<button type=\\"button\\">Entry</button>", "xml").root.children'}{cem:variable @name=returner @select='data:read("<button type=\\"button\\">Return</button>", "xml").root.children'}{cem:variable @name=anchorNode @select='data:read("<div style=\\"position:fixed;left:120px;top:80px;width:20px;height:20px\\">Anchor</div>", "xml").root.children'}{cem:variable @name=bounds @select='data:read("<div style=\\"position:fixed;left:100px;top:40px;width:240px;height:180px;pointer-events:none\\">Boundary</div>", "xml").root.children'}{${popup} @focus-target={#entry} @return-focus={#returner} @anchor={#anchorNode} @boundary={#bounds} @open=false | {$entry}}{$returner}{$anchorNode}{$bounds}`),
            ];
            const instance = document.createElement(card); root.append(instance);
            try {
                await runtime.whenRenderSettled(instance);
                const surface = required<HTMLElement>(instance, popup); await runtime.whenRenderSettled(surface);
                const panel = required<HTMLElement>(surface, '[part=popup]'), trigger = required<HTMLButtonElement>(surface, '[part=base] button');
                const entry = required<HTMLButtonElement>(panel, '[slot]'), returner = required<HTMLButtonElement>(instance, ':scope > button');
                const anchor = required<HTMLElement>(instance, ':scope > div'), boundary = required<HTMLElement>(instance, ':scope > div:last-child');
                for (const name of ['focus-target', 'return-focus', 'anchor', 'boundary']) await expect(surface.hasAttribute(`data-cem-node-ref-${name}`)).toBe(true);
                await expect(surface.getAttribute('focus-target')).toBe(entry.id);
                const errors: string[] = []; surface.addEventListener('cem-interaction-error', event => errors.push((event as CustomEvent).detail.code));
                await userEvent.click(trigger); await expect(document.activeElement).toBe(entry);
                const box = panel.getBoundingClientRect(), fit = boundary.getBoundingClientRect();
                await expect(box.left).toBeGreaterThanOrEqual(fit.left + 3);
                await expect(box.right).toBeLessThanOrEqual(fit.right - 3);
                await expect(box.top).toBeGreaterThanOrEqual(fit.top + 3);
                await expect(box.bottom).toBeLessThanOrEqual(fit.bottom - 3);
                await expect(Math.abs(box.left - anchor.getBoundingClientRect().left)).toBeLessThan(1);
                anchor.style.left = '130px';
                await waitFor(() => expect(panel.getBoundingClientRect().left).toBe(130));
                boundary.style.width = '120px';
                await waitFor(() => expect(panel.getBoundingClientRect().width).toBeLessThanOrEqual(112));
                await expect(panel.getBoundingClientRect().right).toBeLessThanOrEqual(boundary.getBoundingClientRect().right - 3);
                boundary.style.width = '240px'; anchor.style.left = '120px';
                await waitFor(() => expect(panel.getBoundingClientRect().width).toBe(180));
                await userEvent.keyboard('{Escape}'); await expect(document.activeElement).toBe(returner);
                returner.disabled = true;
                await userEvent.click(trigger); await userEvent.keyboard('{Escape}'); await expect(document.activeElement).toBe(trigger);
                await expect(errors).toContain('interaction-return-focus-invalid'); returner.disabled = false;
                // A focus reference outside the semantic surface uses its normal entry fallback.
                surface.setAttribute('focus-target', returner.id);
                await userEvent.click(trigger); await expect(document.activeElement).toBe(required(panel, ':scope > button'));
                await expect(errors).toContain('interaction-focus-target-invalid');
                await userEvent.keyboard('{Escape}'); surface.setAttribute('focus-target', entry.id);
                await userEvent.click(trigger); await expect(document.activeElement).toBe(entry);
                // Geometry loss closes transient surfaces; freeze keeps established geometry only.
                const previous = panel.style.left;
                surface.setAttribute('anchor-lost', 'freeze'); anchor.hidden = true;
                await waitFor(() => expect(errors).toContain('interaction-anchor-unavailable'));
                await expect(surface.getAttribute('open')).toBe('true'); await expect(panel.style.left).toBe(previous);
                surface.setAttribute('anchor-lost', 'close');
                await waitFor(() => expect(panel.hidden).toBe(true));
                await expect(document.activeElement).not.toBe(returner);
                await userEvent.click(trigger); await expect(panel.hidden).toBe(true);
                anchor.hidden = false; await userEvent.click(trigger); await expect(panel.hidden).toBe(false);
                surface.setAttribute('boundary', 'missing-placement');
                await waitFor(() => expect(panel.hidden).toBe(true));
                surface.setAttribute('boundary', boundary.id); await userEvent.click(trigger);
                await expect(panel.hidden).toBe(false);
                // Tab/outside close does not hijack focus; literal none remains available.
                await userEvent.keyboard('{Tab}'); await expect(panel.hidden).toBe(true); await expect(document.activeElement).toBe(returner);
                surface.removeAttribute('data-cem-node-ref-focus-target'); surface.setAttribute('focus-target', 'none');
                await userEvent.click(trigger); await expect(document.activeElement).toBe(trigger);
                surface.removeAttribute('data-cem-node-ref-return-focus'); surface.setAttribute('return-focus', 'none');
                entry.focus(); await userEvent.keyboard('{Escape}'); await expect(document.activeElement).not.toBe(returner);
                // Local names for the new slots still use the nearest scope.
                instance.setAttribute('interaction-scope', 'geometry'); anchor.setAttribute('interaction-name', 'anchor'); boundary.setAttribute('interaction-name', 'bounds');
                surface.removeAttribute('data-cem-node-ref-anchor'); surface.setAttribute('anchor', '@anchor');
                surface.removeAttribute('data-cem-node-ref-boundary'); surface.setAttribute('boundary', '@bounds');
                await userEvent.click(trigger); await expect(panel.hidden).toBe(false);
                surface.setAttribute('interaction-scope', 'inner');
                await waitFor(() => expect(panel.hidden).toBe(true));
                surface.removeAttribute('interaction-scope'); await userEvent.click(trigger); await expect(panel.hidden).toBe(false);
                // Default absolute panels keep declaration positioning until
                // activation/resize, and follow their container during scrolling.
                surface.setAttribute('open', 'false'); await waitFor(() => expect(panel.hidden).toBe(true));
                surface.removeAttribute('anchor'); surface.removeAttribute('boundary');
                panel.style.position = 'absolute'; panel.style.left = '11px'; panel.style.top = '17px'; panel.style.display = 'block';
                surface.setAttribute('open', 'true'); await waitFor(() => expect(panel.hidden).toBe(false));
                await expect(panel.style.left).toBe('11px');
                window.dispatchEvent(new Event('resize'));
                await waitFor(() => expect(panel.style.left).not.toBe('11px'));
                const fittedLeft = panel.style.left, fittedTop = panel.style.top;
                window.dispatchEvent(new Event('scroll'));
                await expect(panel.style.left).toBe(fittedLeft); await expect(panel.style.top).toBe(fittedTop);
                surface.setAttribute('open', 'false');
                await waitFor(() => expect(getComputedStyle(panel).display).toBe('none'));
                surface.remove(); await waitFor(() => expect(panel.style.left).toBe('11px'));
            } finally { instance.remove(); declarations.forEach(node => node.remove()); root.replaceChildren(); scope.dispose(); }
        }
    },
};

export const MenuFocusBoundaryInheritanceAndCleanup: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = required<HTMLElement>(canvasElement, 'section'), scope = createCemDeclarationScope({ document });
        const suffix = crypto.randomUUID(), menu = `cem-geometry-menu-${suffix}`, declarationTag = `declaration-${suffix}`;
        const runtime = new CemElementRuntime({ declarationTag, declarationScope: scope }); runtime.install(window);
        const declaration = declare(root, runtime, declarationTag, menu, '{attribute @name=keyboard | menu}{div @part=composite | {slot}}', 'composite-menu');
        const parent = document.createElement(menu), child = document.createElement(menu);
        const boundary = document.createElement('div'); boundary.id = `boundary-${suffix}`;
        boundary.style.cssText = 'position:fixed;left:40px;top:40px;width:240px;height:180px;pointer-events:none';
        parent.innerHTML = '<button>Parent</button>'; child.innerHTML = '<button>First</button><button>Second</button>';
        parent.setAttribute('boundary', boundary.id); parent.setAttribute('data-cem-node-ref-boundary', '');
        root.append(parent, child, boundary);
        try {
            await Promise.all([runtime.whenRenderSettled(parent), runtime.whenRenderSettled(child)]);
            const trigger = required<HTMLButtonElement>(parent, 'button'), first = required<HTMLButtonElement>(child, 'button'), second = required<HTMLButtonElement>(child, 'button:last-child');
            trigger.id ||= `parent-${suffix}`; second.id = `entry-${suffix}`;
            child.setAttribute('parent-item', trigger.id); child.setAttribute('data-cem-node-ref-parent-item', '');
            child.setAttribute('focus-target', second.id); child.setAttribute('data-cem-node-ref-focus-target', '');
            child.setAttribute('return-focus', first.id = `return-${suffix}`); child.setAttribute('data-cem-node-ref-return-focus', '');
            await waitFor(() => expect(trigger.getAttribute('aria-controls')).toBe(child.id));
            await userEvent.click(trigger); await expect(document.activeElement).toBe(second);
            await expect(child.getBoundingClientRect().left).toBeGreaterThanOrEqual(boundary.getBoundingClientRect().left + 3);
            await userEvent.keyboard('{Escape}'); await expect(document.activeElement).toBe(trigger); // hidden return target is ineligible
            child.removeAttribute('data-cem-node-ref-focus-target'); child.setAttribute('focus-target', 'none');
            await userEvent.click(trigger); await expect(document.activeElement).toBe(trigger);
            await userEvent.keyboard('{Escape}'); await expect(child.hidden).toBe(true);
            child.remove(); await waitFor(() => expect(trigger.hasAttribute('aria-controls')).toBe(false));
            await expect(child.style.left).toBe('');
        } finally { parent.remove(); child.remove(); boundary.remove(); declaration.remove(); scope.dispose(); }
    },
};
