import { afterEach, describe, expect, it } from 'vitest';

import {
    CEM_STUDIO_LIMITS,
    CEM_STUDIO_PREVIEW_CSP,
    CemStudioLimitError,
    assertCemStudioResourceUri,
    assertCemStudioResultSize,
    assertCemStudioSourceSet,
    createCemStudioPreview,
    mountCemStudioPreview,
    redactCemStudioSecrets,
} from './preview.js';

afterEach(() => {
    document.body.replaceChildren();
    delete globalThis.__cemStudioPreviewAttack;
});

describe('CEM Studio bounded preview boundary', () => {
    it('rejects oversized sources, resource sets, dependency fan-out, and results with stable diagnostics', () => {
        expect(assertCemStudioSourceSet(new Uint8Array(CEM_STUDIO_LIMITS.sourceBytes), [])).toEqual({
            sourceBytes: CEM_STUDIO_LIMITS.sourceBytes,
            resourceCount: 1,
            totalBytes: CEM_STUDIO_LIMITS.sourceBytes,
        });
        expect(() => assertCemStudioSourceSet(new Uint8Array(CEM_STUDIO_LIMITS.sourceBytes + 1))).toThrowError(
            expect.objectContaining({
                name: 'CemStudioLimitError',
                code: 'cem.studio.limit.source_bytes',
            }),
        );
        expect(() =>
            assertCemStudioSourceSet(new Uint8Array(CEM_STUDIO_LIMITS.sourceBytes), [
                { bytes: new Uint8Array(CEM_STUDIO_LIMITS.dependencyBytes) },
                { bytes: new Uint8Array(1) },
            ]),
        ).toThrowError(expect.objectContaining({ code: 'cem.studio.limit.resource_set_bytes' }));
        expect(() =>
            assertCemStudioSourceSet(
                new Uint8Array(1),
                Array.from({ length: CEM_STUDIO_LIMITS.resourceCount + 1 }, () => ({ bytes: new Uint8Array(0) })),
            ),
        ).toThrowError(expect.objectContaining({ code: 'cem.studio.limit.resource_count' }));
        expect(() => assertCemStudioResultSize(CEM_STUDIO_LIMITS.resultBytes + 1)).toThrowError(CemStudioLimitError);
    });

    it('renders declared text only as text and bounds its focusable DOM projection', () => {
        const attack = '<img src=x onerror="globalThis.__cemStudioPreviewAttack=true"><script>attack()</script>';
        const preview = createCemStudioPreview({
            bytes: new TextEncoder().encode(attack),
            contentType: 'text/plain; charset=utf-8',
        });
        const root = document.createElement('section');
        document.body.append(root);
        const output = mountCemStudioPreview(root, preview);

        expect(preview).toMatchObject({ kind: 'text', truncated: false });
        expect(output.tagName).toBe('PRE');
        expect(output.textContent).toBe(attack);
        expect(root.querySelector('img, script')).toBeNull();
        expect(output.tabIndex).toBe(0);
        expect(globalThis.__cemStudioPreviewAttack).toBeUndefined();
    });

    it('isolates active markup in an opaque scriptless sandbox with a deny-all policy', async () => {
        const attack =
            '<form action="https://example.test/leak"><button>Send</button></form>' +
            '<img src="https://example.test/pixel" onerror="parent.__cemStudioPreviewAttack=true">' +
            '<script>parent.__cemStudioPreviewAttack=true</script>';
        const preview = createCemStudioPreview({
            bytes: new TextEncoder().encode(attack),
            contentType: 'text/html',
            label: 'Untrusted result preview',
        });
        const root = document.createElement('section');
        document.body.append(root);
        const frame = mountCemStudioPreview(root, preview);
        await new Promise((resolve) => frame.addEventListener('load', resolve, { once: true }));

        expect(preview).toMatchObject({ kind: 'sandboxed-html', truncated: false });
        expect(frame.srcdoc.indexOf(CEM_STUDIO_PREVIEW_CSP)).toBeLessThan(frame.srcdoc.indexOf(attack));
        expect(frame.getAttribute('sandbox')).toBe('');
        expect(frame.getAttribute('allow')).toBe('');
        expect(frame.getAttribute('referrerpolicy')).toBe('no-referrer');
        expect(frame.title).toBe('Untrusted result preview');
        expect(globalThis.__cemStudioPreviewAttack).toBeUndefined();
    });

    it('does not guess binary, invalid UTF-8, or oversized active markup', () => {
        expect(
            createCemStudioPreview({ bytes: new Uint8Array([0, 1, 2]), contentType: 'application/octet-stream' }),
        ).toMatchObject({ kind: 'download', byteLength: 3 });
        expect(createCemStudioPreview({ bytes: new Uint8Array([0xff]), contentType: 'text/plain' })).toMatchObject({
            kind: 'download',
            byteLength: 1,
        });
        expect(
            createCemStudioPreview({
                bytes: new Uint8Array(CEM_STUDIO_LIMITS.inlinePreviewBytes + 1),
                contentType: 'text/html',
            }),
        ).toMatchObject({ kind: 'download', byteLength: CEM_STUDIO_LIMITS.inlinePreviewBytes + 1 });
    });

    it('rejects unsafe resolver URLs and redacts credentials before diagnostics cross the UI boundary', () => {
        expect(assertCemStudioResourceUri('studio://project/source.cem', 'studio')).toBe('studio://project/source.cem');
        expect(() => assertCemStudioResourceUri('javascript:alert(1)', 'studio')).toThrowError(
            expect.objectContaining({ code: 'cem.studio.security.resource_url' }),
        );
        expect(() => assertCemStudioResourceUri('https://example.test/redirected.cem', 'studio')).toThrowError(
            expect.objectContaining({ code: 'cem.studio.security.resource_url' }),
        );
        expect(() => assertCemStudioResourceUri('studio://user:password@project/source.cem', 'studio')).toThrowError(
            expect.objectContaining({ code: 'cem.studio.security.resource_url' }),
        );
        expect(
            redactCemStudioSecrets(
                'GET https://user:password@example.test/file?token=abc&safe=yes Authorization: Bearer secret-value',
            ),
        ).toBe('GET https://[redacted]@example.test/file?token=[redacted]&safe=yes Authorization: Bearer [redacted]');
    });
});
