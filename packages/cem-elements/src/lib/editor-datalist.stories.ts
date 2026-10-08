import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import { getCemEditorProvider } from './form-control-capability.js';
import { CemSuggestionsPlacementCoordinator } from './suggestions-placements.js';

export default { title: 'CEM Elements/Editor Datalist Leases', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
async function fixture(root: HTMLElement, type = 'text') {
    const suffix = crypto.randomUUID(), declarationTag = `datalist-declaration-${suffix}`;
    const runtime = new CemElementRuntime({ declarationTag }); runtime.install(window);
    const declaration = document.createElement(declarationTag);
    declaration.setAttribute('tag', `datalist-field-${suffix}`); declaration.setAttribute('capability', 'form-control');
    const template = document.createElement('template'); template.type = 'text/cem-ml';
    template.textContent = `{input @part=control @form="" @type=${type} @value={datadom.slices.value} @slice=value @slice-event=input @slice-value="$target.value"}`;
    declaration.append(template); root.append(declaration); runtime.registerDeclaration(declaration); await runtime.whenDeclarationSettled(declaration);
    const form = document.createElement('form'), field = document.createElement(`datalist-field-${suffix}`);
    field.setAttribute('name', 'choice'); field.setAttribute('value', type === 'number' ? '1' : 'original'); form.append(field); root.append(form);
    await runtime.whenRenderSettled(field);
    const provider = getCemEditorProvider(field), editor = field.querySelector('input');
    if (!provider || !editor) throw new Error('Missing original editor');
    const coordinator = new CemSuggestionsPlacementCoordinator(document);
    const editorPlacement = coordinator.register({ producer: 'field', revision: '1', current: () => true, element: field, kind: 'editor' });
    const datalist = document.createElement('datalist'), option = document.createElement('option');
    option.value = '1'; option.label = 'One'; datalist.append(option); form.append(datalist);
    let ready = true;
    const target = coordinator.register({ producer: 'source', revision: '1', current: () => ready, element: datalist, kind: 'datalist' });
    const lease = provider.lease({});
    const grants = () => [coordinator.grant('source', editorPlacement, ['editor-for']), coordinator.grant('field', target, ['list'])];
    const prepare = () => coordinator.prepareDatalist(editorPlacement, target);
    const request = (placements = prepare()) => ({ revision: provider.revision, current: () => ready, placements });
    return { runtime, form, field, editor, provider, coordinator, editorPlacement, datalist, option, target, lease, grants, prepare, request,
        readiness(value: boolean) { ready = value; coordinator.refresh(); },
        dispose() { lease.release(); coordinator.dispose(); form.remove(); declaration.remove(); } };
}

export const PreparedOptionsRequireExactDirectedAuthority: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const f = await fixture(canvasElement);
        try {
            expect(() => f.prepare()).toThrow('both directions'); expect(f.datalist.id).toBe('');
            f.coordinator.grant('source', f.editorPlacement, ['editor-for']);
            expect(() => f.prepare()).toThrow('both directions');
            const revoke = f.coordinator.grant('field', f.target, ['list']);
            f.option.value = ''; expect(() => f.prepare()).toThrow('prepared options'); f.option.value = '1';
            const placements = f.prepare(), request = f.request(placements);
            expect(f.lease.datalist.set({ ...request, placements: { ...placements } })).toBe(false);
            expect(f.lease.datalist.set({ ...request, placements: JSON.parse(JSON.stringify(placements)) })).toBe(false);
            expect(f.lease.datalist.set(request)).toBe(true);
            expect(f.editor.list).toBe(f.datalist); expect(f.editor.hasAttribute('role')).toBe(false);
            expect(f.editor.hasAttribute('aria-expanded')).toBe(false); expect(new FormData(f.form).get('choice')).toBe('original');
            expect(f.lease.commit('1', { revision: f.provider.revision })).toBe(false);
            const key = new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true });
            f.editor.addEventListener('keydown', event => expect(f.lease.handlePress(event)).toBe(false), { once: true });
            f.editor.dispatchEvent(key); expect(key.defaultPrevented).toBe(false);
            revoke(); expect(f.editor.hasAttribute('list')).toBe(false); expect(placements.current()).toBe(false);
            f.coordinator.grant('field', f.target, ['list']);
            expect(f.lease.datalist.set(request)).toBe(false); // grant reissue cannot revive an expired admission
            expect(f.lease.datalist.set(f.request())).toBe(true);
            const duplicate = document.createElement('div'); duplicate.id = f.datalist.id; f.form.append(duplicate);
            await waitFor(() => expect(f.editor.hasAttribute('list')).toBe(false)); duplicate.remove();
            expect(f.lease.datalist.set(f.request())).toBe(true);
            f.datalist.removeAttribute('id'); await waitFor(() => expect(f.lease.datalist.valid).toBe(false));
        } finally { f.dispose(); }
    },
};

export const AuthorRelationshipsAndCompetingProfilesArePreserved: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const f = await fixture(canvasElement); f.grants();
        try {
            for (const value of ['', 'unresolved', f.prepare().datalist.id]) {
                f.editor.setAttribute('list', value); expect(f.lease.datalist.set(f.request())).toBe(false);
                expect(f.editor.getAttribute('list')).toBe(value); f.editor.removeAttribute('list');
            }
            expect(f.lease.datalist.set(f.request())).toBe(true);
            f.editor.setAttribute('list', f.datalist.id); // same string, newer author ownership
            f.lease.datalist.clear(); expect(f.editor.getAttribute('list')).toBe(f.datalist.id);
            f.editor.removeAttribute('list'); expect(f.lease.datalist.set(f.request())).toBe(true);
            f.editor.setAttribute('list', 'new-author'); f.lease.datalist.refresh();
            expect(f.editor.getAttribute('list')).toBe('new-author'); expect(f.lease.datalist.valid).toBe(false);
            f.editor.removeAttribute('list');
            const panel = document.createElement('div'); panel.id = `panel-${crypto.randomUUID()}`;
            panel.setAttribute('role', 'listbox'); panel.setAttribute('popover', 'manual'); f.form.append(panel);
            const custom = () => ({ revision: f.provider.revision, current: () => true, listbox: panel, expanded: false, autocomplete: 'list' as const });
            expect(f.lease.attributes.set(custom())).toBe(true);
            expect(f.lease.datalist.set(f.request())).toBe(false); expect(f.lease.attributes.valid).toBe(true);
            f.lease.attributes.clear(); expect(f.lease.datalist.set(f.request())).toBe(true);
            expect(f.lease.attributes.set(custom())).toBe(false); expect(f.lease.datalist.valid).toBe(true);
            const competing = f.provider.lease({});
            expect(f.lease.valid).toBe(false); expect(competing.valid).toBe(false); expect(f.editor.hasAttribute('list')).toBe(false);
            competing.release(); expect(f.editor.hasAttribute('list')).toBe(false);
            expect(f.lease.datalist.set(f.request())).toBe(true);
            f.runtime.setInstanceSlices(f.field, { unrelated: 'render' }); await f.runtime.whenRenderSettled(f.field);
            expect(f.provider.control).toBe(f.editor); expect(f.editor.list).toBe(f.datalist);
            f.field.setAttribute('list', 'host-author'); await waitFor(() => expect(f.lease.datalist.valid).toBe(false));
            expect(f.field.getAttribute('list')).toBe('host-author');
        } finally { f.dispose(); }
    },
};

export const SourceReplacementAndRevisionLossWithdrawClaims: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const f = await fixture(canvasElement); f.grants();
        try {
            const first = f.request(); expect(f.lease.datalist.set(first)).toBe(true);
            f.readiness(false); expect(f.editor.hasAttribute('list')).toBe(false);
            expect(() => f.prepare()).toThrow('prepared options'); f.readiness(true);
            expect(f.lease.datalist.set(first)).toBe(false);
            expect(f.lease.datalist.set(f.request())).toBe(true);
            f.option.value = '2'; await waitFor(() => expect(f.editor.hasAttribute('list')).toBe(false));
            expect(f.lease.datalist.set(f.request())).toBe(true);
            const replacement = document.createElement('datalist'), option = document.createElement('option');
            option.value = '3'; option.label = 'Three'; replacement.append(option); f.form.append(replacement);
            const target = f.coordinator.register({ producer: 'source', revision: '2', current: () => true, element: replacement, kind: 'datalist' });
            f.coordinator.grant('field', target, ['list']);
            const placements = f.coordinator.prepareDatalist(f.editorPlacement, target);
            expect(f.lease.datalist.set(f.request(placements))).toBe(true); expect(f.editor.list).toBe(replacement);
            f.target.dispose(); expect(f.editor.list).toBe(replacement); // old producer disposal cannot clear its replacement
            const stale = f.request(placements); f.form.reset(); expect(f.editor.hasAttribute('list')).toBe(false);
            expect(f.lease.datalist.set(stale)).toBe(false); expect(placements.current()).toBe(false);
            await f.runtime.whenRenderSettled(f.field);
            const fresh = f.coordinator.prepareDatalist(f.editorPlacement, target);
            expect(f.lease.datalist.set(f.request(fresh))).toBe(true);
            f.field.remove(); expect(f.editor.hasAttribute('list')).toBe(false);
            f.form.prepend(f.field); await f.runtime.whenRenderSettled(f.field);
            expect(f.lease.datalist.set(f.request(fresh))).toBe(false);
            const resumed = f.provider.lease({});
            try {
                const admission = f.coordinator.prepareDatalist(f.editorPlacement, target);
                expect(resumed.datalist.set(f.request(admission))).toBe(true);
                f.coordinator.dispose(); expect(f.editor.hasAttribute('list')).toBe(false);
                expect(resumed.datalist.set(f.request(admission))).toBe(false);
            } finally { resumed.release(); }
        } finally { f.dispose(); }
    },
};

export const SupportedInputsKeepTheirNativeSemantics: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        for (const type of ['text', 'search', 'tel', 'url', 'email', 'number']) {
            const f = await fixture(canvasElement, type); f.grants();
            try {
                expect(f.lease.datalist.set(f.request())).toBe(true);
                expect(f.provider.control).toBe(f.editor); expect(f.editor.type).toBe(type);
                expect(f.editor.hasAttribute('role')).toBe(false); expect(f.editor.hasAttribute('aria-controls')).toBe(false);
                if (type === 'email') f.editor.multiple = true; else f.editor.type = type === 'number' ? 'text' : 'number';
                await waitFor(() => expect(f.editor.hasAttribute('list')).toBe(false));
                if (type === 'email') expect(() => f.prepare()).toThrow('endpoints');
            } finally { f.dispose(); }
        }
    },
};
