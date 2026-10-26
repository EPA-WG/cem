import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';

import {
    CEM_STUDIO_LIMITS,
    CemStudioLimitError,
    assertCemStudioResourceUri,
    assertCemStudioResultSize,
    assertCemStudioSourceSet,
    createCemStudioPreview,
    createCemStudioLivePreview,
    redactCemStudioSecrets,
} from './preview.js';
import { installCemStudioShellComponents } from './shell.js';

let runtime, loginView, loginData;
beforeAll(async () => {
    [loginView, loginData] = await Promise.all([
        new URL('../../../examples/projects/login/views/login.cem', import.meta.url),
        new URL('../../../examples/semantic/login.html', import.meta.url),
    ].map(async url => { const response = await fetch(url); if (!response.ok) throw new Error(String(response.status)); return response.text(); }));
    const map = document.createElement('script'); map.type = 'importmap';
    map.textContent = JSON.stringify({ imports: {
        '@epa-wg/cem-theme': new URL('../../cem-theme/dist/index.js', import.meta.url).href,
        '@epa-wg/cem-components/components.xhtml': new URL('../../cem-components/dist/components.xhtml', import.meta.url).href,
    } });
    document.head.append(map);
    ({ runtime } = await installCemStudioShellComponents());
});

async function renderPreview(root, preview) {
    const host = document.createElement('cem-media-preview');
    host.setAttribute('mode', preview.kind === 'text' ? 'text' : preview.kind === 'sandboxed-html' ? 'document' : 'message');
    host.setAttribute('label', preview.label);
    host.setAttribute('value', preview.text ?? preview.source ?? preview.reason);
    host.toggleAttribute('truncated', Boolean(preview.truncated));
    root.append(host); await runtime.whenRenderSettled(host);
    return host.querySelector('pre, iframe, [part=message]');
}

afterEach(() => {
    document.body.replaceChildren();
    delete globalThis.__cemStudioPreviewAttack;
});

describe('CEM Studio bounded preview boundary', () => {
    it('previews declared YAML as literal text and preserves its UTF-8 bytes', async () => {
        const text = 'name: Café cups\nmarkup: <img src=x onerror=attack()>\n';
        const bytes = new TextEncoder().encode(text);
        const preview = createCemStudioPreview({ bytes, contentType: 'application/yaml; charset=utf-8' });
        expect(preview).toMatchObject({ kind: 'text', text, byteLength: bytes.byteLength, truncated: false });
        const root = document.createElement('section'); document.body.append(root);
        const output = await renderPreview(root, preview);
        expect(output.textContent).toBe(text);
        expect(root.querySelector('img')).toBeNull();
    });

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

    it('renders declared text only as text and bounds its focusable DOM projection', async () => {
        const attack = '<img src=x onerror="globalThis.__cemStudioPreviewAttack=true"><script>attack()</script>';
        const preview = createCemStudioPreview({
            bytes: new TextEncoder().encode(attack),
            contentType: 'text/plain; charset=utf-8',
        });
        const root = document.createElement('section');
        document.body.append(root);
        const output = await renderPreview(root, preview);

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
        const frame = await renderPreview(root, preview);
        await expect.poll(() => frame.contentDocument).toBeNull();

        expect(preview).toMatchObject({ kind: 'sandboxed-html', truncated: false });
        expect(frame.srcdoc.indexOf("default-src 'none'")).toBeLessThan(frame.srcdoc.indexOf(attack));
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

    it('keeps declared compiled binaries as exact downloads even when every byte is valid UTF-8', () => {
        const bytes = new TextEncoder().encode('CEMPROJ\u0000\u0000\u0001\u0002');
        for (const family of ['ast', 'dom', 'events']) {
            expect(createCemStudioPreview({ bytes, contentType: `application/vnd.cem.${family}+cem-bin; version=1` }))
                .toMatchObject({ kind: 'download', byteLength: bytes.length });
        }
    });

    it('presents declared RELAX NG compact source as text', () => {
        const text = 'element note { text }\n';
        expect(createCemStudioPreview({ bytes: new TextEncoder().encode(text), contentType: 'application/relax-ng-compact-syntax' }))
            .toMatchObject({ kind: 'text', text });
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


it('loads only requested preview declarations, bounds requests and retries failures', async () => {
    const fetcher = vi.spyOn(globalThis, 'fetch');
    const input = { source: '{p | Preview}', sourceUri: 'studio://one/view.cem', components: ['cem-alert', 'cem-alert'] };
    try {
        const first = await createCemStudioLivePreview(input);
        const resources = JSON.parse(first.resourceSet);
        expect(resources.declarations).toEqual([{ tag: 'cem-alert', uri: 'cem-preview://shared/components/cem-alert/cem-alert.xhtml#cem-alert' }]);
        expect(resources.resources.map(resource => resource.uri)).toEqual(['cem-preview://shared/theme.css', 'cem-preview://shared/components/cem-alert/cem-alert.xhtml']);
        expect(resources.resources[1].text).toContain('id="cem-alert"');
        expect(fetcher.mock.calls.some(([url]) => String(url).endsWith('/dist/components.xhtml'))).toBe(false);
        fetcher.mockClear();
        for (const tag of ['../cem-alert', 'cem-alert/path', 'https://example.test/a', 'cem-alert#other', 'CEM-alert']) {
            await expect(createCemStudioLivePreview({ ...input, components: [tag] })).rejects.toThrow('Preview component');
        }
        await expect(createCemStudioLivePreview({ ...input, components: Array.from({ length: 128 }, (_, n) => `cem-test-${n}`) })).rejects.toMatchObject({ code: 'cem.studio.limit.resource_count' });
        expect(fetcher).not.toHaveBeenCalled();
        fetcher.mockResolvedValueOnce(new Response('', { status: 503 }));
        await expect(createCemStudioLivePreview(input)).rejects.toThrow('Preview library unavailable: 503');
        const retried = await createCemStudioLivePreview(input);
        expect(retried.resourceSet).toBe(first.resourceSet);
        expect(fetcher).toHaveBeenCalledTimes(2);
    } finally { fetcher.mockRestore(); }
});

it('preserves relative asset addresses without granting nested declaration authority', async () => {
    const preview = await createCemStudioLivePreview({ source: '{cem-menu | {cem-menu-item @label=Home}}',
        sourceUri: 'studio://menu/view.cem', components: ['cem-menu', 'cem-menu-item'] });
    const envelope = JSON.parse(preview.resourceSet);
    const menu = envelope.declarations.find(item => item.tag === 'cem-menu');
    expect(new URL('../cem-menu-item/cem-menu-item.xhtml', menu.uri).href).toBe(envelope.resources[2].uri);
    const host = document.createElement('cem-media-preview'); host.setAttribute('mode', 'live');
    host.setAttribute('value', preview.source); host.setAttribute('source-uri', preview.sourceUri); host.setAttribute('resource-set', preview.resourceSet);
    document.body.append(host); await runtime.whenRenderSettled(host);
    await expect.poll(() => host.querySelector('[role=status]')?.textContent, { timeout: 15000 }).toContain('cem.preview.element_denied: cem-element');
}, 30000);

it('renders the Login project through shared light DOM with saved data and template replacements', async () => {
    const host = document.createElement('cem-media-preview'); host.setAttribute('mode', 'live');
    host.setAttribute('label', 'Login live view'); document.body.append(host);
    async function show(source, data, project = 'login-one') {
        const preview = await createCemStudioLivePreview({ source, sourceUri: `studio://${project}/views/login.cem`, components: ['cem-text-field', 'cem-action', 'cem-alert'], dependencies: [{
            path: `studio://${project}/data/login.html`, contentType: 'text/html', bytes: new TextEncoder().encode(data),
        }] });
        host.setAttribute('value', preview.source); host.setAttribute('source-uri', preview.sourceUri); host.setAttribute('resource-set', preview.resourceSet);
        await runtime.whenRenderSettled(host);
        await expect.poll(() => host.querySelector('[role=status]')?.textContent, { timeout: 15000 }).toBe('Live view ready');
        return host.querySelector('iframe').contentDocument;
    }
    const first = await show(loginView, loginData);
    expect(first.querySelector('h1').textContent).toBe('Sign in');
    expect(first.querySelector('cem-text-field').shadowRoot).toBeNull();
    expect(first.querySelector('input[type=password]')).not.toBeNull();
    const surface = first.querySelector('section');
    const style = () => first.defaultView.getComputedStyle(surface);
    expect(parseFloat(style().paddingTop)).toBeGreaterThan(0);
    expect(parseFloat(style().rowGap)).toBeGreaterThan(0);
    expect(style().backgroundColor).not.toBe('rgba(0, 0, 0, 0)');
    // Consumer overrides remain inherited through the shared light-DOM root.
    first.querySelector('cem-live-view').style.setProperty('--cem-inset-container', '23px');
    first.querySelector('cem-live-view').style.setProperty('--cem-gap-group', '17px');
    expect(style().paddingTop).toBe('23px'); expect(style().rowGap).toBe('17px');
    first.querySelector('cem-action button').click();
    await expect.poll(() => first.querySelector('cem-alert').getAttribute('label')).toContain('Local interaction received');
    const edited = await show(loginView.replace('Sign in locally', 'Continue locally'), loginData.replace('Sign in', 'Welcome café'));
    expect(edited.querySelector('h1').textContent).toBe('Welcome café');
    expect(edited.querySelector('cem-action button').textContent).toContain('Continue locally');
    const independent = await show(loginView, loginData, 'login-two');
    expect(independent.querySelector('h1').textContent).toBe('Sign in');
    expect(independent.querySelector('cem-alert').getAttribute('label')).toBe('This local example does not send credentials.');
}, 30000);
