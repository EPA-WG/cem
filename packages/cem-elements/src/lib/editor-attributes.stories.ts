import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import { getCemEditorProvider, isCemEditorLeaseFor } from './form-control-capability.js';
import { connectCemManualListbox } from './manual-listbox.js';

export default { title: 'CEM Elements/Editor Attribute Leases', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
let sequence = 0;
function required<T>(value: T | null | undefined): T {
    if (value == null) throw new Error('Missing editor attribute fixture value');
    return value;
}
async function fixture(root: HTMLElement) {
    const suffix = ++sequence, runtime = new CemElementRuntime({ declarationTag: `editor-attributes-declaration-${suffix}` });
    runtime.install(window);
    const declaration = document.createElement(`editor-attributes-declaration-${suffix}`);
    declaration.setAttribute('tag', `attribute-editor-${suffix}`); declaration.setAttribute('capability', 'form-control');
    const template = document.createElement('template'); template.type = 'text/cem-ml';
    template.textContent = '{input @part=control @form="" @type=text @role={datadom.attributes.role} @value={datadom.slices.value} @aria-label=Choice @slice=value @slice-event=input @slice-value="$target.value"}';
    declaration.append(template); root.append(declaration); runtime.registerDeclaration(declaration); await runtime.whenDeclarationSettled(declaration);
    const form = document.createElement('form'), field = document.createElement(`attribute-editor-${suffix}`);
    field.setAttribute('name', 'choice'); field.setAttribute('value', 'original'); form.append(field); root.append(form);
    const panel = document.createElement('div'); panel.setAttribute('role', 'listbox'); panel.setAttribute('popover', 'manual');
    panel.id = `attribute-listbox-${suffix}`; panel.style.cssText = 'width:120px;height:50px';
    const row = document.createElement('div'); row.setAttribute('role', 'option'); row.id = `attribute-row-${suffix}`; row.textContent = 'Choice';
    panel.append(row); form.append(panel); await runtime.whenRenderSettled(field);
    const provider = required(getCemEditorProvider(field)), editor = required(field.querySelector('input'));
    const lease = provider.lease({}); editor.focus();
    let current = true;
    const request = (expanded = false) => ({ revision: provider.revision, listbox: panel, expanded, autocomplete: 'list' as const,
        current: () => current, ...(expanded ? { row } : {}) });
    return { runtime, field, form, panel, row, editor, provider, lease, request, revoke: () => { current = false; } };
}

export const ProviderClaimsAreAtomicAndPreserveAuthoredRelationships: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const f = await fixture(canvasElement);
        try {
            f.editor.setAttribute('aria-controls', 'other'); f.editor.setAttribute('aria-describedby', 'help');
            f.editor.setAttribute('role', 'textbox');
            await expect(f.lease.attributes.set(f.request())).toBe(false);
            await expect(f.editor.getAttribute('aria-controls')).toBe('other');
            await expect(f.editor.hasAttribute('aria-expanded')).toBe(false);
            f.editor.removeAttribute('role');
            f.editor.setAttribute('aria-autocomplete', 'both');
            await expect(f.lease.attributes.set(f.request())).toBe(false);
            await expect(f.editor.hasAttribute('role')).toBe(false);
            await expect(f.editor.getAttribute('aria-autocomplete')).toBe('both');
            f.editor.removeAttribute('aria-autocomplete');
            await expect(f.lease.attributes.set(f.request())).toBe(true);
            await expect(f.editor.getAttribute('role')).toBe('combobox');
            await expect(f.editor.getAttribute('aria-controls')).toBe(`other ${f.panel.id}`);
            await expect(f.editor.getAttribute('aria-describedby')).toBe('help');
            await expect(new FormData(f.form).get('choice')).toBe('original');
            await expect(f.lease.attributes.set(f.request(true))).toBe(false); // an ID is not evidence of actual visibility
            f.panel.showPopover(); await expect(f.lease.attributes.set(f.request(true))).toBe(true);
            await expect(f.editor.getAttribute('aria-activedescendant')).toBe(f.row.id);
            const foreign = document.createElement('div'); foreign.id = `${f.row.id}-foreign`; foreign.setAttribute('role', 'option');
            f.form.append(foreign);
            await expect(f.lease.attributes.set({ ...f.request(true), row: foreign })).toBe(false); foreign.remove();
            await expect(f.lease.attributes.set(f.request(true))).toBe(true);
            await expect(f.lease.attributes.set(f.request())).toBe(false); // collapsed claims cannot describe an open listbox
            await expect(f.lease.attributes.set(f.request(true))).toBe(true);
            const oldId = f.row.id; f.row.id = `${oldId}-new`; f.lease.attributes.refresh();
            await expect(f.editor.hasAttribute('aria-activedescendant')).toBe(false);
            f.row.id = oldId; await expect(f.lease.attributes.set(f.request(true))).toBe(true);
            f.row.hidden = true; f.lease.attributes.refresh();
            await expect(f.lease.attributes.valid).toBe(false); await expect(f.editor.hasAttribute('aria-activedescendant')).toBe(false);
            await expect(f.editor.getAttribute('aria-controls')).toBe('other');
            f.row.hidden = false; f.panel.hidePopover();
            await expect(f.lease.attributes.set(f.request())).toBe(true);
            f.editor.setAttribute('aria-controls', `other ${f.panel.id} newly-authored`);
            f.editor.setAttribute('role', 'searchbox'); f.lease.release();
            await expect(f.editor.getAttribute('role')).toBe('searchbox');
            await expect(f.editor.getAttribute('aria-controls')).toBe('other newly-authored');
            await expect(f.editor.getAttribute('aria-describedby')).toBe('help');
        } finally { f.lease.release(); if (f.panel.matches(':popover-open')) f.panel.hidePopover(); }
    },
};

export const EndpointAndEditorReplacementRequireFreshClaims: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const f = await fixture(canvasElement);
        let fresh: ReturnType<typeof f.provider.lease> | undefined;
        try {
            const duplicate = document.createElement('div'); duplicate.id = f.panel.id; f.form.append(duplicate);
            await expect(f.lease.attributes.set(f.request())).toBe(false); duplicate.remove();
            await expect(f.lease.attributes.set(f.request())).toBe(true);
            const originalControls = f.editor.getAttribute('aria-controls');
            (f.field as HTMLElement & { formStateRestoreCallback(value: string, mode: 'restore'): void }).formStateRestoreCallback('restored', 'restore');
            await expect(f.editor.hasAttribute('aria-controls')).toBe(false);
            await f.runtime.whenRenderSettled(f.field);
            await expect(f.lease.attributes.set(f.request())).toBe(true);
            const replacement = f.editor.cloneNode(true) as HTMLInputElement;
            // A fresh renderer output does not inherit transient attributes from the old control.
            for (const name of ['role', 'aria-controls', 'aria-expanded', 'aria-haspopup', 'aria-autocomplete', 'aria-activedescendant']) replacement.removeAttribute(name);
            f.editor.replaceWith(replacement);
            f.runtime.setInstanceSlices(f.field, { other: 'rebind' }); await f.runtime.whenRenderSettled(f.field);
            await expect(f.lease.valid).toBe(false); await expect(f.lease.attributes.valid).toBe(false);
            await expect(f.editor.hasAttribute('aria-controls')).toBe(false);
            await expect(replacement.hasAttribute('aria-controls')).toBe(false);
            await expect(f.lease.attributes.set(f.request())).toBe(false);
            fresh = f.provider.lease({}); f.lease.release();
            await expect(fresh.attributes.set(f.request())).toBe(true);
            await expect(replacement.getAttribute('aria-controls')).toBe(originalControls);
            f.field.remove(); f.form.append(f.field); await f.runtime.whenRenderSettled(f.field);
            await expect(fresh.valid).toBe(false); await expect(fresh.attributes.set(f.request())).toBe(false);
            await expect(required(f.provider.control).hasAttribute('aria-controls')).toBe(false);
        } finally { fresh?.release(); f.lease.release(); }
    },
};

export const RevisionsAndCompetingClaimsCannotRestoreOldAttributes: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const f = await fixture(canvasElement);
        try {
            await expect(f.lease.attributes.set(f.request())).toBe(true);
            const stale = f.request(); (f.field as HTMLElement & { value: string }).value = 'newer';
            await expect(f.editor.hasAttribute('aria-controls')).toBe(false);
            await expect(f.lease.attributes.set(stale)).toBe(false);
            await f.runtime.whenRenderSettled(f.field);
            await expect(f.lease.attributes.set(f.request())).toBe(true);
            // Rendering must preserve provider-owned relationships that the declaration does not author.
            f.runtime.setInstanceSlices(f.field, { other: 'render' }); await f.runtime.whenRenderSettled(f.field);
            await expect(f.editor.getAttribute('aria-controls')).toBe(f.panel.id);
            f.field.setAttribute('role', 'textbox'); await f.runtime.whenRenderSettled(f.field); f.lease.attributes.refresh();
            await expect(f.editor.getAttribute('role')).toBe('textbox'); await expect(f.editor.hasAttribute('aria-controls')).toBe(false);
            f.field.removeAttribute('role'); await f.runtime.whenRenderSettled(f.field);
            await expect(f.lease.attributes.set(f.request())).toBe(true);
            const competing = f.provider.lease({});
            await expect(f.lease.valid).toBe(false); await expect(competing.valid).toBe(false);
            await expect(f.editor.hasAttribute('role')).toBe(false); competing.release();
            await expect(f.editor.hasAttribute('role')).toBe(false);
            await expect(f.lease.attributes.set(f.request())).toBe(true); f.revoke(); f.lease.attributes.refresh();
            await expect(f.editor.hasAttribute('role')).toBe(false);
        } finally { f.lease.release(); }
    },
};

export const SharedVerifiedLeaseClosesThroughTheNativeDelegate: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const f = await fixture(canvasElement), listeners = new Set<() => void>();
        const lifecycle = { current: () => true, ready: () => true, subscribe(listener: () => void) { listeners.add(listener); return () => { listeners.delete(listener); }; } };
        await expect(isCemEditorLeaseFor(f.lease, f.provider)).toBe(true);
        await expect(() => connectCemManualListbox(f.panel, { host: f.form, editorHost: f.field, lifecycle,
            editorLease: { ...f.lease } })).toThrow('verified editor lease');
        const controller = connectCemManualListbox(f.panel, { host: f.form, editorHost: f.field, lifecycle, editorLease: f.lease,
            onVisibility(open) { f.lease.attributes.set(f.request(open)); } });
        try {
            await expect(controller.open()).toBe(true); await expect(f.editor.getAttribute('aria-expanded')).toBe('true');
            controller.dismiss('escape'); await expect(f.editor.getAttribute('aria-expanded')).toBe('false');
            await expect(f.editor.hasAttribute('aria-activedescendant')).toBe(false);
            await expect(controller.open()).toBe(true); f.editor.setAttribute('role', 'textbox');
            await waitFor(() => expect(f.panel.matches(':popover-open')).toBe(false));
            await expect(f.editor.getAttribute('role')).toBe('textbox');
            controller.disconnect(); await expect(f.lease.valid).toBe(true); // caller retains ownership of the shared lease
            f.editor.removeAttribute('role'); await expect(f.lease.attributes.set(f.request())).toBe(true);
            f.form.reset(); await expect(f.editor.hasAttribute('aria-controls')).toBe(false);
            f.field.remove(); await expect(f.lease.valid).toBe(false); await expect(f.lease.attributes.valid).toBe(false);
        } finally { controller.disconnect(); f.lease.release(); }
    },
};
