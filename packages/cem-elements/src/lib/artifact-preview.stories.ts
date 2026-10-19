import { expect, waitFor } from 'storybook/test';
import preview from '../../.storybook/preview.js';
import { mountCemArtifactPreview, registerCemArtifactTree } from './artifact-preview.js';
const meta = preview.meta({ title: 'Runtime/Artifact tree preview' });
const artifact = (path: string, contentType: string, text: string) => ({ path, contentType, bytes: new TextEncoder().encode(text) });
const png = { path: 'assets/a b.png', contentType: 'image/png', bytes: Uint8Array.from(atob('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jJXcAAAAASUVORK5CYII='), ch => ch.charCodeAt(0)) };
export const NestedResources = meta.story({
    render: () => '<iframe title="Site output"></iframe><iframe title="Another run"></iframe>',
    play: async ({ canvasElement }) => {
        const [frame, other] = [...canvasElement.querySelectorAll('iframe')];
        const home = artifact('posts/index.html', 'text/html', '<!doctype html><html><head><link rel="stylesheet" href="../assets/site.css"></head><body><h1 id="title">First run</h1><img alt="Pixel" src="../assets/a%20b.png"><a href="./details/index.html?view=wide#section">Details</a><a href="#title">Title</a></body></html>');
        const tree = registerCemArtifactTree([home, png,
            artifact('assets/site.css', 'text/css', '@import "./base.css" layer(base) screen; h1 { color:rgb(12,34,56); background-image:image-set("./a\\20 b.png" 1x); }'),
            artifact('assets/base.css', 'text/css', 'h1 { padding-left:17px; }'),
            artifact('posts/details/index.html', 'text/html', '<h2 id="section">Details</h2><a href="#section">Section</a><a href="../">Home</a>'),
        ]);
        // Registration owns an immutable copy, not the caller's mutable byte buffer.
        home.bytes.fill(0);
        const two = registerCemArtifactTree([artifact('posts/index.html', 'text/html', '<h1>Second run</h1>')]);
        const paths: string[] = [], statuses: string[] = [];
        const one = mountCemArtifactPreview(frame, { resourceSet: tree.resourceSet, path: 'posts/index.html', onNavigate: value => paths.push(value), onStatus: value => statuses.push(value) });
        const independent = mountCemArtifactPreview(other, { resourceSet: two.resourceSet, path: 'posts/index.html' });
        try {
            await Promise.all([one.ready, independent.ready]);
            const doc = frame.contentDocument!, heading = doc.querySelector('h1')!, image = doc.querySelector('img')!;
            await waitFor(() => { expect(frame.contentWindow!.getComputedStyle(heading).color).toBe('rgb(12, 34, 56)'); expect(frame.contentWindow!.getComputedStyle(heading).paddingLeft).toBe('17px'); });
            await waitFor(() => expect(image.naturalWidth).toBe(1));
            expect(frame.contentWindow!.getComputedStyle(heading).backgroundImage).toContain('blob:');
            expect(other.contentDocument!.querySelector('h1')!.textContent).toBe('Second run');
            doc.querySelectorAll('a')[0].click(); doc.querySelectorAll('a')[1].click();
            expect(paths).toEqual(['posts/details/index.html?view=wide#section', 'posts/index.html#title']);
            expect(frame.contentWindow!.location.href).toBe('about:srcdoc');
            expect(statuses.at(-1)).toBe('Artifact view ready');
            const replacement = mountCemArtifactPreview(frame, { resourceSet: tree.resourceSet, path: paths[0], onNavigate: value => paths.push(value) });
            await replacement.ready;
            expect(frame.contentDocument!.querySelector('h2')!.textContent).toBe('Details');
            const article = frame.contentDocument;
            replacement.setLocation('posts/details/index.html?view=compact#section');
            expect(frame.contentDocument).toBe(article);
            article!.querySelector('a')!.click();
            expect(paths.at(-1)).toBe('posts/details/index.html?view=compact#section');
            article!.querySelectorAll('a')[1].click(); expect(paths.at(-1)).toBe('posts/index.html');
            one.dispose(); expect(frame.hasAttribute('srcdoc')).toBe(true);
            tree.dispose(); expect(frame.hasAttribute('srcdoc')).toBe(false);
            await expect(fetch(image.src)).rejects.toThrow();
            expect(other.contentDocument!.querySelector('h1')!.textContent).toBe('Second run');
            expect(() => mountCemArtifactPreview(frame, { resourceSet: tree.resourceSet, path: 'posts/index.html' })).toThrow('released');
        } finally { tree.dispose(); two.dispose(); one.dispose(); independent.dispose(); }
    },
});
export const MissingAndDeniedResources = meta.story({
    render: () => '<iframe title="Restricted site"></iframe>',
    play: async ({ canvasElement }) => {
        const frame = canvasElement.querySelector('iframe')!, statuses: string[] = [], paths: string[] = [];
        const tree = registerCemArtifactTree([
            artifact('index.html', 'text/html', `<base href="https://example.test/"><meta http-equiv="refresh" content="0;url=https://example.test/"><script>parent.__cemArtifactAttack=true</script><iframe srcdoc="unsafe"></iframe><img alt="Missing" src="missing.png" onerror="parent.__cemArtifactAttack=true"><img alt="External" src="https://example.test/pixel"><link rel="stylesheet" href="style.css"><p style="background:url(https://example.test/css-image)">Safe text</p><a href="missing.html">Missing</a><a href="javascript:parent.__cemArtifactAttack=true">Unsafe</a><a href="https://example.test/">External</a><form action="https://example.test/"><button>Send</button></form>`),
            artifact('style.css', 'text/css', '@import "./style.css"; p { color: rgb(25,50,75) }'),
        ]);
        const controller = mountCemArtifactPreview(frame, { resourceSet: tree.resourceSet, path: 'index.html', onStatus: message => statuses.push(message), onNavigate: value => paths.push(value) });
        try {
            await controller.ready;
            const doc = frame.contentDocument!;
            expect(doc.querySelector('script,base,iframe,meta[http-equiv=refresh],[onerror]')).toBeNull();
            expect(frame.sandbox.value).toBe('allow-same-origin'); expect(frame.referrerPolicy).toBe('no-referrer');
            expect(doc.querySelector('meta[http-equiv="Content-Security-Policy"]')!.getAttribute('content')).toContain("script-src 'none'");
            expect(doc.querySelectorAll('img')[0].src).toBe('about:invalid#cem-artifact');
            expect(statuses.at(-1)).toContain('Missing artifact: missing.png');
            expect(statuses.at(-1)).toContain('Cyclic stylesheet import: style.css');
            expect(statuses.at(-1)).toContain('Blocked artifact reference: https://example.test/');
            doc.querySelectorAll('a').forEach(link => link.click());
            expect(paths).toEqual(['missing.html']);
            expect(statuses.at(-1)).toContain('Missing artifact: missing.html');
            expect((window as Window & { __cemArtifactAttack?: boolean }).__cemArtifactAttack).toBeUndefined();
            expect(frame.contentWindow!.location.href).toBe('about:srcdoc');
            expect(performance.getEntriesByType('resource').some(entry => entry.name.startsWith('https://example.test/'))).toBe(false);
            const missing = mountCemArtifactPreview(frame, { resourceSet: tree.resourceSet, path: 'missing.html' });
            await expect(missing.ready).rejects.toThrow('artifact_missing'); missing.dispose();
        } finally { controller.dispose(); tree.dispose(); }
    },
});
export const LimitsAndCancellation = meta.story({
    render: () => '<iframe title="Lifecycle"></iframe>',
    play: async ({ canvasElement }) => {
        const frame = canvasElement.querySelector('iframe')!;
        expect(() => registerCemArtifactTree([artifact('../escape.html', 'text/html', '')])).toThrow('artifact_path');
        expect(() => registerCemArtifactTree([png, png])).toThrow('artifact_set');
        expect(() => registerCemArtifactTree(Array.from({ length: 129 }, (_, i) => artifact(`${i}.html`, 'text/html', '')))).toThrow('artifact_limit');
        expect(() => registerCemArtifactTree([{ ...png, bytes: new Uint8Array(16 * 1024 * 1024 + 1) }])).toThrow('artifact_set');
        const tree = registerCemArtifactTree([artifact('index.html', 'text/html', '<style>p { color:green }</style><p>Cancelled</p>')]);
        const one = mountCemArtifactPreview(frame, { resourceSet: tree.resourceSet, path: 'index.html' });
        const two = mountCemArtifactPreview(frame, { resourceSet: tree.resourceSet, path: 'index.html' });
        await expect(one.ready).rejects.toThrow('disposed');
        await two.ready;
        tree.dispose(); expect(frame.hasAttribute('srcdoc')).toBe(false); two.dispose();
    },
});
