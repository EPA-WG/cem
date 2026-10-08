import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';
import { resolveCemSuggestionsEditor } from './suggestions-editor.js';
import { observeInteractionReferences } from './interaction-reference.js';

export default { title: 'CEM Elements/Suggestions Editor', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
export const ExactProvidersWorkerAndFallback: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section'); if (!root) throw new Error('Missing fixture');
        for (const fallback of [false, true]) {
            const scope = createCemDeclarationScope({ document }), suffix = crypto.randomUUID();
            const declarationTag = `suggestions-endpoint-declaration-${suffix}`, fieldTag = `suggestions-field-${suffix}`, ownerTag = `suggestions-owner-${suffix}`;
            const runtime = new CemElementRuntime({ declarationTag, declarationScope: scope,
                ...(fallback ? { processingWorkerFactory: () => { throw new Error('fixture fallback'); } } : {}) });
            runtime.install(window);
            const declare = async (tag: string, source: string, capability?: string) => {
                const declaration = document.createElement(declarationTag); declaration.setAttribute('tag', tag);
                if (capability) declaration.setAttribute('capability', capability);
                const template = document.createElement('template'); template.type = 'text/cem-ml'; template.textContent = source;
                declaration.append(template); root.append(declaration); runtime.registerDeclaration(declaration);
                await runtime.whenDeclarationSettled(declaration); return declaration;
            };
            const fieldDeclaration = await declare(fieldTag, '{input @part=control @type=text @value={datadom.slices.value}}', 'form-control');
            const ownerDeclaration = await declare(ownerTag,
                `{cem:variable @name=field @select='data:read("<${fieldTag}/>", "xml").root.children'}{aside @editor-for={#field}}{$field}`);
            const owner = document.createElement(ownerTag); root.append(owner);
            try {
                await runtime.whenRenderSettled(owner);
                const endpoint = owner.querySelector('aside'), field = owner.querySelector<HTMLElement>(fieldTag);
                if (!endpoint || !field) throw new Error('Missing native editor relationship');
                await runtime.whenRenderSettled(field);
                expect(endpoint.getAttribute('editor-for')).toBe(field.id);
                expect(endpoint.hasAttribute('data-cem-node-ref-editor-for')).toBe(true);
                const result = resolveCemSuggestionsEditor(endpoint);
                expect(result.host).toBe(field); expect(result.editor).toBe(field.querySelector('input'));
                // Endpoint lookup never claims a provider or writes an ARIA relationship.
                expect(result.editor?.hasAttribute('role')).toBe(false);
                const duplicate = document.createElement('div'); duplicate.id = field.id; root.append(duplicate);
                expect(resolveCemSuggestionsEditor(endpoint).code).toBe('interaction-name-duplicate'); duplicate.remove();
                endpoint.removeAttribute('data-cem-node-ref-editor-for');
                for (const invalid of [field.id, `#${field.id}`, 'input', 'https://example.test/#field', '']) {
                    endpoint.setAttribute('editor-for', invalid);
                    expect(resolveCemSuggestionsEditor(endpoint).code).toBe('suggestions-editor-reference-invalid');
                }
                owner.setAttribute('interaction-scope', ''); field.setAttribute('interaction-name', 'field');
                endpoint.setAttribute('editor-for', '@field');
                expect(resolveCemSuggestionsEditor(endpoint).host).toBe(field);
                const nested = document.createElement('div'); nested.setAttribute('interaction-scope', '');
                const other = document.createElement('div'); other.setAttribute('interaction-name', 'field'); nested.append(other); owner.append(nested);
                expect(resolveCemSuggestionsEditor(endpoint).host).toBe(field);
                endpoint.removeAttribute('editor-for'); field.setAttribute('slot', 'editor'); endpoint.append(field);
                await runtime.whenRenderSettled(field);
                expect(resolveCemSuggestionsEditor(endpoint).host).toBe(field);
                endpoint.setAttribute('editor-for', '@field');
                expect(resolveCemSuggestionsEditor(endpoint).code).toBe('suggestions-editor-conflict');
                endpoint.removeAttribute('editor-for');
                const input = field.querySelector('input'); if (!input) throw new Error('Missing editor');
                input.type = 'search'; expect(resolveCemSuggestionsEditor(endpoint).code).toBe('suggestions-editor-profile-conflict');
                input.type = 'text'; input.setAttribute('list', '');
                expect(resolveCemSuggestionsEditor(endpoint).code).toBe('suggestions-editor-profile-conflict'); input.removeAttribute('list');
                const bare = document.createElement('input'); bare.slot = 'editor'; endpoint.append(bare);
                expect(resolveCemSuggestionsEditor(endpoint).code).toBe('suggestions-editor-conflict');
                field.remove(); expect(resolveCemSuggestionsEditor(endpoint).code).toBe('suggestions-editor-provider-invalid'); bare.remove();
                let observed = 0; const stop = observeInteractionReferences(endpoint, () => observed++);
                endpoint.setAttribute('editor-for', '@new'); await new Promise(resolve => setTimeout(resolve));
                expect(observed).toBeGreaterThan(0); const before = observed;
                endpoint.setAttribute('data-cem-node-ref-editor-for', ''); await new Promise(resolve => setTimeout(resolve));
                expect(observed).toBeGreaterThan(before); stop();
            } finally { owner.remove(); fieldDeclaration.remove(); ownerDeclaration.remove(); scope.dispose(); }
        }
    },
};
