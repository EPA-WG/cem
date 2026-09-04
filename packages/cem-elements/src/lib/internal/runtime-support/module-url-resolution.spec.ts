import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('./cem-ql-render.js', () => ({
    resolveCemModuleUrl: vi.fn(),
}));

import {
    CemModuleUrlResolutionError,
    createBrowserModuleUrlContext,
    createBrowserModuleUrlRoot,
    resolveBrowserModuleUrl,
} from './module-url-resolution.js';
import { resolveCemModuleUrl } from './cem-ql-render.js';

describe('browser module URL resolution contexts', () => {
    beforeEach(() => {
        vi.mocked(resolveCemModuleUrl).mockReset();
    });

    it('captures page import maps and resolves relative URLs against the template source', async () => {
        const document = browserDocument(`
            <script type="importmap">{
                "imports": { "pkg/": "https://cdn.example.test/pkg/" }
            }</script>
        `);
        const root = createBrowserModuleUrlRoot(document, 'policy:v1');
        const template = createBrowserModuleUrlContext(
            root.context,
            'card-template',
            'https://components.example.test/card/card.cem',
            'template:card',
            'policy:v1',
        );

        vi.mocked(resolveCemModuleUrl)
            .mockResolvedValueOnce(wasmResolution('https://cdn.example.test/pkg/icon.svg'))
            .mockResolvedValueOnce(wasmResolution('https://components.example.test/card/icon.svg'));

        await expect(resolveBrowserModuleUrl(template, 'pkg/icon.svg')).resolves.toEqual(
            expect.objectContaining({ resolvedUrl: 'https://cdn.example.test/pkg/icon.svg' }),
        );
        await expect(resolveBrowserModuleUrl(template, './icon.svg')).resolves.toEqual(
            expect.objectContaining({
                resolvedUrl: 'https://components.example.test/card/icon.svg',
            }),
        );
        expect(resolveCemModuleUrl).toHaveBeenNthCalledWith(1, expect.objectContaining({
            purpose: 'template-slice',
            authoredSpecifier: 'pkg/icon.svg',
            currentContext: template.handle,
            contexts: expect.arrayContaining([
                expect.objectContaining({
                    handle: root.context.handle,
                    context: expect.objectContaining({
                        frames: [expect.objectContaining({
                            baseUrl: 'https://app.example.test/page/index.html',
                            specifiers: expect.objectContaining({
                                imports: {
                                    'pkg/': { target: 'https://cdn.example.test/pkg/' },
                                },
                            }),
                        })],
                    }),
                }),
            ]),
        }));
    });

    it('resolves a bare scalar referrer once before import-map scope matching', async () => {
        const document = browserDocument(`
            <script type="importmap">{
                "imports": {
                    "worker": "./workers/worker.js"
                },
                "scopes": {
                    "./workers/": {
                        "asset": "./assets/worker.wasm"
                    }
                }
            }</script>
        `);
        const root = createBrowserModuleUrlRoot(document, 'policy:v1');
        const template = createBrowserModuleUrlContext(
            root.context,
            'worker-template',
            'https://components.example.test/worker.cem',
            'template:worker',
            'policy:v1',
        );

        vi.mocked(resolveCemModuleUrl).mockResolvedValueOnce(wasmResolution(
            'https://app.example.test/page/assets/worker.wasm',
            { resolvedReferrerUrl: 'https://app.example.test/page/workers/worker.js' },
        ));

        await expect(
            resolveBrowserModuleUrl(template, 'asset', { kind: 'url', value: 'worker' }),
        ).resolves.toEqual(expect.objectContaining({
            resolvedReferrerUrl: 'https://app.example.test/page/workers/worker.js',
            resolvedUrl: 'https://app.example.test/page/assets/worker.wasm',
        }));
        expect(resolveCemModuleUrl).toHaveBeenCalledWith(expect.objectContaining({
            referrer: { kind: 'url', value: 'worker' },
            contexts: expect.arrayContaining([
                expect.objectContaining({
                    context: expect.objectContaining({
                        frames: [expect.objectContaining({
                            scopes: [expect.objectContaining({
                                prefix: 'https://app.example.test/page/workers/',
                            })],
                        })],
                    }),
                }),
            ]),
        }));
    });

    it('uses descendant node contexts as bases and rejects sibling scope escalation', async () => {
        const document = browserDocument('');
        const root = createBrowserModuleUrlRoot(document, 'policy:v1');
        const current = createBrowserModuleUrlContext(
            root.context,
            'current',
            'https://app.example.test/current/template.cem',
            'template:current',
            'policy:v1',
        );
        const descendant = createBrowserModuleUrlContext(
            current,
            'descendant',
            'https://app.example.test/child/template.cem',
            'template:descendant',
            'policy:v1',
        );
        const sibling = createBrowserModuleUrlContext(
            root.context,
            'sibling',
            'https://app.example.test/sibling/template.cem',
            'template:sibling',
            'policy:v1',
        );

        vi.mocked(resolveCemModuleUrl)
            .mockResolvedValueOnce(wasmResolution(
                'https://app.example.test/child/asset.css',
                { selectedContextIdentity: descendant.identity },
            ))
            .mockResolvedValueOnce({
                status: 'error',
                error: {
                    ...wasmFailure('./asset.css'),
                    reason: 'referrer-scope-denied',
                },
            });

        await expect(
            resolveBrowserModuleUrl(current, './asset.css', { kind: 'context', context: descendant }),
        ).resolves.toEqual(expect.objectContaining({
            resolvedUrl: 'https://app.example.test/child/asset.css',
            selectedContextIdentity: descendant.identity,
        }));

        await expect(
            resolveBrowserModuleUrl(current, './asset.css', { kind: 'context', context: sibling }),
        ).rejects.toMatchObject({
            name: CemModuleUrlResolutionError.name,
            failure: { reason: 'referrer-scope-denied' },
        });
        expect(resolveCemModuleUrl).toHaveBeenNthCalledWith(1, expect.objectContaining({
            currentContext: current.handle,
            referrer: { kind: 'context', context: descendant.handle },
            contexts: expect.arrayContaining([
                expect.objectContaining({ handle: current.handle, parent: root.context.handle }),
                expect.objectContaining({ handle: descendant.handle, parent: current.handle }),
            ]),
        }));
    });

    it('installs component-local mappings after the outer page frame', async () => {
        const document = browserDocument(`
            <script type="importmap">{
                "imports": { "shared-image": "./outer.svg" }
            }</script>
        `);
        const root = createBrowserModuleUrlRoot(document, 'policy:v1');
        const component = createBrowserModuleUrlContext(
            root.context,
            'local-map-component',
            'https://components.example.test/card/card.cem',
            'template:card',
            'policy:v1',
            {
                scopes: [],
                specifiers: {
                    imports: {},
                    resources: {
                        'shared-image': { target: './inner.svg', contentTypeHint: 'image/svg+xml' },
                        'inner-only-image': { target: './only.svg' },
                    },
                },
            },
        );

        vi.mocked(resolveCemModuleUrl).mockResolvedValueOnce(
            wasmResolution('https://app.example.test/page/outer.svg'),
        );
        await resolveBrowserModuleUrl(component, 'shared-image');

        expect(resolveCemModuleUrl).toHaveBeenCalledWith(expect.objectContaining({
            contexts: expect.arrayContaining([
                expect.objectContaining({
                    handle: component.handle,
                    context: expect.objectContaining({
                        frames: [
                            expect.objectContaining({ frameId: 'browser-document-root' }),
                            expect.objectContaining({
                                baseUrl: 'https://components.example.test/card/card.cem',
                                specifiers: expect.objectContaining({
                                    resources: expect.objectContaining({
                                        'shared-image': {
                                            target: './inner.svg',
                                            contentTypeHint: 'image/svg+xml',
                                        },
                                        'inner-only-image': { target: './only.svg' },
                                    }),
                                }),
                            }),
                        ],
                    }),
                }),
            ]),
        }));
    });
});

function browserDocument(body: string): Document {
    const scripts = Array.from(
        body.matchAll(/<script\s+type="importmap">([\s\S]*?)<\/script>/g),
        (match) => ({ textContent: match[1] }),
    );
    return {
        baseURI: 'https://app.example.test/page/index.html',
        querySelectorAll: (selector: string) => selector === 'script[type="importmap"]' ? scripts : [],
    } as unknown as Document;
}

function wasmResolution(resolvedUrl: string, extra: Record<string, unknown> = {}): unknown {
    return {
        status: 'resolved',
        resolution: {
            authoredSpecifier: 'fixture',
            normalizedSpecifier: 'fixture',
            resolvedUrl,
            contextIdentity: 'fixture',
            resolverIdentity: 'fixture',
            resourcePolicyStamp: 'policy:v1',
            currentContextIdentity: 'fixture',
            selectedContextIdentity: 'fixture',
            ...extra,
        },
    };
}

function wasmFailure(authoredSpecifier: string): Record<string, unknown> {
    return {
        authoredSpecifier,
        contextIdentity: 'fixture',
        resolverIdentity: 'fixture',
        resourcePolicyStamp: 'policy:v1',
        currentContextIdentity: 'fixture',
        reason: 'unresolved',
        message: 'fixture failure',
    };
}
