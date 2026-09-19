import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';

const meta: Meta = { title: 'CEM Elements/Native XSLT Lifecycle', tags: ['test'] };
export default meta;
type Story = StoryObj;

const principal = `<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
    <xsl:import href="./controls.xslt"/>
    <xsl:template name="view">
        <xsl:param name="text"/><xsl:param name="payload"/>
        <xsl:call-template name="controls"><xsl:with-param name="text" select="$text"/></xsl:call-template>
        <p><xsl:value-of select="$payload"/></p>
    </xsl:template>
</xsl:stylesheet>`;
const dependency = `<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
    <xsl:template name="controls">
        <xsl:param name="text"/>
        <label>Text<input value="{$text}" slice="text" slice-event="input" slice-value="$target.value"/></label>
        <output><xsl:value-of select="$text"/></output>
    </xsl:template>
</xsl:stylesheet>`;

function lifecycle(fallback: boolean): Story {
    return {
        render: () => '<section aria-label="Native XSLT lifecycle"></section>',
        play: async ({ canvasElement }) => {
            const root = canvasElement.querySelector('section');
            if (!root) throw new Error('fixture root missing');
            const suffix = fallback ? 'fallback' : 'worker';
            const declarationTag = `cem-element-xslt-${suffix}`;
            const scope = createCemDeclarationScope({ document });
            const loads: string[] = [];
            const worker: { current: Worker | null } = { current: null };
            let failDependency = fallback;
            const runtime = new CemElementRuntime({
                declarationTag, declarationScope: scope,
                processingWorkerFactory: request => {
                    if (fallback) throw new Error('fixture selects fallback');
                    worker.current = new Worker(request.scriptUrl, { type: request.type, name: request.name });
                    return worker.current;
                },
                loadSrcDocument: async uri => {
                    const url = new URL(uri, document.baseURI);
                    loads.push(url.pathname);
                    if (url.pathname.endsWith('/view.xslt')) return principal;
                    if (url.pathname.endsWith('/controls.xslt')) {
                        if (failDependency) {
                            failDependency = false;
                            throw new Error('stylesheet temporarily unavailable');
                        }
                        return dependency;
                    }
                    throw new Error(`unexpected stylesheet ${uri}`);
                },
            });
            runtime.install(window);
            const mount = async (tag?: string) => {
                const declaration = document.createElement(declarationTag);
                if (tag) declaration.setAttribute('tag', tag);
                declaration.setAttribute('src', './native-xslt/view.xslt');
                declaration.setAttribute('xslt-template', 'view');
                declaration.setAttribute('label', 'First');
                declaration.innerHTML = `<xslt-param name="text" select='datadom.slices.text ?? datadom.attributes.label'></xslt-param>
                    <xslt-param name="payload" select='datadom.payload.nodes.text'></xslt-param>
                    ${tag ? '' : '<template>Anonymous payload</template>'}`;
                root.append(declaration);
                runtime.registerDeclaration(declaration);
                await runtime.whenDeclarationSettled(declaration);
                if (!tag) return declaration;
                const instance = document.createElement(tag);
                instance.setAttribute('label', 'First');
                instance.innerHTML = '<template>Named payload</template>';
                root.append(instance);
                await runtime.whenRenderSettled(instance);
                return instance;
            };
            try {
                const first = await mount(`story-xslt-${suffix}`);
                if (fallback) {
                    expect(runtime.diagnosticsFor(first).some(d => d.message.includes('temporarily unavailable'))).toBe(true);
                    first.setAttribute('label', 'Retried');
                }
                await waitFor(() => expect(first.querySelector('output')?.textContent).toBe(fallback ? 'Retried' : 'First'), { timeout: 20000 });
                expect(first.querySelector('p')?.textContent).toBe('Named payload');
                const second = document.createElement(`story-xslt-${suffix}`);
                second.setAttribute('label', 'Second');
                root.append(second);
                await waitFor(() => expect(second.querySelector('output')?.textContent).toBe('Second'));
                const input = first.querySelector('input');
                if (!input) throw new Error('input missing');
                input.focus();
                input.value = 'Changed';
                input.setSelectionRange(3, 3);
                input.dispatchEvent(new Event('input', { bubbles: true }));
                await waitFor(() => expect(first.querySelector('output')?.textContent).toBe('Changed'));
                expect(first.querySelector('input')).toBe(input);
                expect(document.activeElement).toBe(input);
                expect(input.selectionStart).toBe(3);
                expect(second.querySelector('output')?.textContent).toBe('Second');
                expect(loads.filter(path => path.endsWith('/controls.xslt'))).toHaveLength(fallback ? 2 : 1);
                const anonymous = await mount();
                await waitFor(() => expect(anonymous.querySelector('output')?.textContent).toBe('First'));
                expect(anonymous.querySelector('p')?.textContent).toBe('Anonymous payload');
                expect(anonymous.querySelector('[data-cem-anonymous-instance]')?.querySelector('xslt-param')).toBeNull();
                expect(loads.filter(path => path.endsWith('/controls.xslt'))).toHaveLength(fallback ? 3 : 2);
                if (!fallback) {
                    expect(worker.current).not.toBeNull();
                    worker.current?.dispatchEvent(new ErrorEvent('error', { message: 'fixture worker failure' }));
                    second.setAttribute('label', 'Recovered');
                    await waitFor(() => expect(second.querySelector('output')?.textContent).toBe('Recovered'));
                    expect(first.querySelector('output')?.textContent).toBe('Changed');
                }
            } finally {
                root.replaceChildren();
                scope.dispose();
            }
        },
    };
}

export const WorkerScalarParameters = lifecycle(false);
export const FallbackDependencyRecovery = lifecycle(true);

// XSLT-VIEW-CONTROL-BUDGET-BOUNDARY: characterize the current transport cap
// before changing a public resource limit. Document input remains source text.
export const ViewerControlEnvelopeBoundary: Story = {
    render: () => '<section aria-label="XSLT control envelope boundary"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('fixture root missing');
        const { RetainedXsltComponent } = await import('./internal/runtime-support/cem-ql-render.js');
        const originalRender = RetainedXsltComponent.prototype.render;
        const sizes: number[] = [];
        RetainedXsltComponent.prototype.render = function (data, options) {
            sizes.push(new TextEncoder().encode(JSON.stringify(data)).byteLength);
            return originalRender.call(this, data, options);
        };
        const scope = createCemDeclarationScope({ document });
        const viewerUrl = new URL('../../demo/data-table-view.xslt', import.meta.url).href;
        const viewer = await (await fetch(viewerUrl)).text();
        const runtime = new CemElementRuntime({
            declarationTag: 'cem-element-xslt-budget', declarationScope: scope,
            processingWorkerFactory: () => { throw new Error('fixture selects fallback'); },
            loadSrcDocument: async uri => {
                if (uri.endsWith('/document.html')) return `<cem-element-xslt-budget src="./viewer.xslt" xslt-template="viewer">
                    <xslt-param name="initial" select='str:trim(str:concat(datadom.payload.nodes.text, ""))'></xslt-param>
                    <xslt-param name="source" select='datadom.slices.source ?? str:trim(str:concat(datadom.payload.nodes.text, ""))'></xslt-param>
                    <xslt-param name="format" select='"json"'></xslt-param>
                    <xslt-param name="selected" select='datadom.slices.selected ?? ""'></xslt-param>
                    <template>[{"qty":10,"fruit":"🍒"},{"qty":2,"fruit":"🍋"},{"qty":3,"fruit":"🍌"}]</template>
                </cem-element-xslt-budget>`;
                if (uri.endsWith('/viewer.xslt')) return viewer;
                throw new Error(`unexpected fixture URI ${uri}`);
            },
        });
        runtime.install(window);
        try {
            const declaration = document.createElement('cem-element-xslt-budget');
            declaration.setAttribute('tag', 'story-xslt-budget-document');
            declaration.setAttribute('src', './native-xslt-budget/document.html');
            root.append(declaration);
            runtime.registerDeclaration(declaration);
            await runtime.whenDeclarationSettled(declaration);
            root.append(document.createElement('story-xslt-budget-document'));
            await waitFor(() => expect(root.querySelector('tbody button')).toBeTruthy(), { timeout: 20000 });
            const instance = root.querySelector('article')?.parentElement;
            if (!instance) throw new Error('viewer instance missing');
            expect(runtime.diagnosticsFor(instance).filter(d => d.severity === 'error')).toEqual([]);
            const initialBytes = sizes.at(-1) ?? 0;
            expect(initialBytes).toBeLessThanOrEqual(128 * 1024);
            root.querySelectorAll<HTMLButtonElement>('tbody button')[1].click();
            await waitFor(() => expect(runtime.diagnosticsFor(instance).some(d => d.code === 'cem.xslt.binding_limit')).toBe(true), { timeout: 10000 });
            const selectionBytes = sizes.at(-1) ?? 0;
            expect(selectionBytes).toBeGreaterThan(128 * 1024);
            expect(root.querySelector('[aria-selected="true"]')).toBeNull();
            const source = '[{"note":"' + 'a'.repeat(32700) + '"}]';
            expect(new TextEncoder().encode(source).byteLength).toBeLessThan(32 * 1024);
            const before = sizes.length;
            runtime.setInstanceSlices(instance, { source });
            await waitFor(() => expect(sizes.length).toBeGreaterThan(before));
            await runtime.whenRenderSettled(instance);
            expect(sizes.at(-1)).toBeGreaterThan(selectionBytes);
            expect(sizes.at(-1)).toBeLessThan(8 * 1024 * 1024);
        } finally {
            root.replaceChildren();
            scope.dispose();
            RetainedXsltComponent.prototype.render = originalRender;
        }
    },
};
