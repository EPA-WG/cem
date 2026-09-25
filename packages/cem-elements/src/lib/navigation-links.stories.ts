import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';

const meta: Meta = { title: 'CEM Elements/Navigation Link Policy', tags: ['test'] };
export default meta;
type Story = StoryObj;
type Mode = 'dom' | 'worker' | 'fallback' | 'wasm';
const SOURCE_URL = 'https://source.example.test/redirected/components/links.html';

function navigationLinks(mode: Mode): Story {
    return {
        render: () => document.createElement('section'),
        play: async ({ canvasElement, step }) => {
            const originalUrl = location.href;
            const originalState = history.state;
            const declarationTag = `cem-navigation-${mode}`;
            const scope = createCemDeclarationScope({ document });
            let workerStarts = 0;
            const html = `<attribute name="target">./start.html</attribute>
<a href="{$datadom.attributes.target}">Dynamic</a>
<a href="../index.html">Index</a><a href="#navigation-policy" target="_self">Fragment</a>
<a href="">Empty</a><a href="https://other.example.test/absolute">Absolute</a>
<map><area href="./area.html" alt="Area"></map>
<form action="./submit"><button formaction="./alternate">Submit</button></form>
<template><a href="./inert.html">Inert</a></template><slot></slot>`;
            const cem = `{attribute @name=target | ./start.html}
{a @href="{$target}" | Dynamic}
{a @href="../index.html" | Index}{a @href="#navigation-policy" @target=_self | Fragment}
{a @href="" | Empty}{a @href="https://other.example.test/absolute" | Absolute}
{map | {area @href="./area.html" @alt=Area}}
{form @action="./submit" | {button @formaction="./alternate" | Submit}}
{template | {a @href="./inert.html" | Inert}}{slot}`;
            const source = mode === 'dom' ? html
                : `<template id="navigation" type="text/cem-ml">${mode === 'wasm'
                    ? '{location-element @slice=route @href="https://example.test/static"}' : ''}${cem}</template>`;
            const runtime = new CemElementRuntime({
                declarationTag, declarationScope: scope,
                processingWorkerFactory: request => {
                    workerStarts += 1;
                    if (mode === 'fallback') throw new Error('fixture selects fallback');
                    return new Worker(request.scriptUrl, { type: request.type, name: request.name });
                },
                loadSrcDocument: async () => ({
                    resolvedUrl: SOURCE_URL, contentType: 'text/html', resolverIdentity: 'navigation-redirect',
                    body: (async function* () { yield new TextEncoder().encode(source); })(),
                }),
            });
            runtime.install(window);
            try {
                for (const policy of ['default', 'document', 'source'] as const) {
                    await step(`${mode}: ${policy} links and dynamic updates`, async () => {
                        const declaration = document.createElement(declarationTag);
                        const tag = `story-navigation-${mode}-${policy}`;
                        declaration.setAttribute('tag', tag);
                        declaration.setAttribute('src', `./requested-links.html${mode === 'dom' ? '' : '#navigation'}`);
                        if (policy !== 'default') declaration.setAttribute('link-base', policy);
                        canvasElement.append(declaration);
                        runtime.registerDeclaration(declaration);
                        await runtime.whenDeclarationSettled(declaration);
                        const instance = document.createElement(tag);
                        instance.setAttribute('target', './start.html');
                        const payload = document.createElement('a');
                        payload.href = './consumer.html';
                        payload.textContent = 'Consumer';
                        instance.append(payload);
                        canvasElement.append(instance);
                        await runtime.whenRenderSettled(instance);
                        const link = (label: string) => {
                            const found = Array.from(instance.querySelectorAll('a')).find(a => a.textContent?.trim() === label);
                            if (!found) throw new Error(`Missing ${label} link`);
                            return found;
                        };
                        const expected = (value: string) => policy === 'source' ? new URL(value, SOURCE_URL).href : value;
                        const dynamic = link('Dynamic');
                        expect(dynamic.getAttribute('href')).toBe(expected('./start.html'));
                        expect(link('Index').getAttribute('href')).toBe(expected('../index.html'));
                        expect(instance.querySelector('area')?.getAttribute('href')).toBe(expected('./area.html'));
                        expect(link('Fragment').getAttribute('href')).toBe('#navigation-policy');
                        expect(link('Empty').getAttribute('href')).toBe('');
                        expect(link('Absolute').getAttribute('href')).toBe('https://other.example.test/absolute');
                        expect(link('Consumer').getAttribute('href')).toBe('./consumer.html');
                        expect(instance.querySelector('form')?.getAttribute('action')).toBe('./submit');
                        expect(instance.querySelector('button')?.getAttribute('formaction')).toBe('./alternate');
                        const inert = instance.querySelector<HTMLTemplateElement>('template:not([data-cem-island])');
                        expect(inert?.content.querySelector('a')?.getAttribute('href')).toBe('./inert.html');
                        for (const value of ['../next.html?x=two words#part', '#changed', '', './start.html']) {
                            instance.setAttribute('target', value);
                            await runtime.whenRenderSettled(instance);
                            expect(link('Dynamic')).toBe(dynamic);
                            // CEM-ML omits empty expression attributes; the link policy
                            // preserves that result, as well as literal empty hrefs above.
                            expect(dynamic.getAttribute('href')).toBe(value === ''
                                ? mode === 'dom' ? '' : null
                                : value.startsWith('#') ? value : expected(value));
                        }
                        await userEvent.click(link('Fragment'));
                        expect(location.href).toBe(new URL('#navigation-policy', originalUrl).href);
                        history.replaceState(originalState, '', originalUrl);
                        for (const element of [declaration, instance]) {
                            expect(runtime.diagnosticsFor(element).filter(d => mode !== 'fallback'
                                || d.code !== 'cem.processing_host.worker_startup_fallback')).toEqual([]);
                        }
                    });
                }
                expect(workerStarts > 0).toBe(mode === 'worker' || mode === 'fallback');
            } finally {
                history.replaceState(originalState, '', originalUrl);
                canvasElement.replaceChildren();
                scope.dispose();
            }
        },
    };
}

export const DomTemplates = navigationLinks('dom');
export const ProcessingHost = navigationLinks('worker');
export const ProcessingFallback = navigationLinks('fallback');
export const DirectWasm = navigationLinks('wasm');

export const InvalidPolicy = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const runtime = new CemElementRuntime({ declarationTag: 'cem-navigation-invalid' });
        runtime.install(window);
        const declaration = document.createElement('cem-navigation-invalid');
        declaration.setAttribute('tag', 'story-navigation-invalid');
        declaration.setAttribute('link-base', 'guess');
        const template = document.createElement('template');
        template.innerHTML = '<a href="./next.html">Next</a>';
        declaration.append(template);
        canvasElement.append(declaration);
        runtime.registerDeclaration(declaration);
        await runtime.whenDeclarationSettled(declaration);
        expect(runtime.diagnosticsFor(declaration).map(d => d.code)).toContain('cem-element.link_base_invalid');
        expect(customElements.get('story-navigation-invalid')).toBeUndefined();
    },
} satisfies Story;
