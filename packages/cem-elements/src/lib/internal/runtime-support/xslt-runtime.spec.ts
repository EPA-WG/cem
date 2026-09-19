import { describe, expect, it, vi } from 'vitest';

const wasm = vi.hoisted(() => ({
    retain: vi.fn(), render: vi.fn(), dispose: vi.fn(), imports: vi.fn(),
}));
vi.mock('../../../../../cem_ql/dist/wasm/cem_ql.js', () => ({
    default: vi.fn(async () => undefined),
    retainXsltComponent: wasm.retain, renderXsltComponent: wasm.render,
    disposeXsltComponent: wasm.dispose, xsltStylesheetImports: wasm.imports,
    templateArtifactPayloadKey: (source: string) => JSON.stringify({ contentType: 'cem-template-artifact',
        sourceHash: `hash:${source}`, cemMlVersion: 'test', cemQlVersion: 'test', sourceMapMode: 'dev' }),
}));

import { preflightXsltModules, retainXsltComponentSource, type RetainedXsltComponent } from './cem-ql-render.js';

describe('XSLT native residency and typed dependency preflight', () => {
    it('evicts at the shared native bound, reloads immutable options, and rejects released leases', async () => {
        let next = 1;
        const residents = new Set<number>();
        wasm.retain.mockReset().mockImplementation(() => {
            if (residents.size >= 16) throw 'XSLT component handle limit exceeded';
            const artifactId = next++;
            residents.add(artifactId);
            return JSON.stringify({ artifactId });
        });
        wasm.dispose.mockReset().mockImplementation((id: number) => residents.delete(id));
        wasm.render.mockReset().mockImplementation((id: number) => {
            expect(residents.has(id)).toBe(true);
            return '{"nodes":[],"diagnostics":[]}';
        });
        const leases: RetainedXsltComponent[] = [];
        const options = { entrypoint: 'view', parameters: [{ name: 'label', select: 'label' }] };
        try {
            for (let i = 0; i < 20; i++) leases.push(await retainXsltComponentSource(`source ${i}`, `https://test/${i}.xslt`, options, ['label']));
            expect(residents.size).toBe(16);
            expect(wasm.dispose).toHaveBeenCalledTimes(4);
            options.parameters[0].select = 'changed';
            await leases[0].render({ label: 'updated' }, {});
            expect(wasm.retain).toHaveBeenLastCalledWith('source 0', 'https://test/0.xslt',
                JSON.stringify({ entrypoint: 'view', parameters: [{ name: 'label', select: 'label' }] }), '["label"]');
            expect(residents.size).toBe(16);
            leases[0].dispose();
            await expect(leases[0].render({}, {})).rejects.toThrow('disposed');
        } finally { for (const lease of leases) lease.dispose(); }
        expect(residents.size).toBe(0);
    });

    it('preflights shared import edges once, preserves parent URIs, and rejects cycles', async () => {
        const edges: Record<string, string[]> = { root: ['./a.xslt', './b.xslt'], a: ['./shared.xslt'], b: ['./shared.xslt'], shared: [] };
        wasm.imports.mockImplementation((source: string) => JSON.stringify(edges[source]));
        const load = vi.fn(async (uri: string) => new URL(uri).pathname.slice(1).replace('.xslt', ''));
        const loader = { rootUrl: 'https://test/root.xslt', resolverPolicyStamp: 'policy',
            resolve: (href: string, uri: string) => new URL(href, uri).href, load };
        const modules = await preflightXsltModules('root', loader);
        expect(load).toHaveBeenCalledTimes(3);
        expect(modules.filter(member => member.uri === 'https://test/shared.xslt').map(member => member.parentUri))
            .toEqual(['https://test/a.xslt', 'https://test/b.xslt']);
        edges.shared = ['./root.xslt'];
        await expect(preflightXsltModules('root', loader)).rejects.toThrow('cyclic');
    });
});
