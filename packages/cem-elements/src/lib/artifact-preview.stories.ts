import { expect, waitFor } from 'storybook/test';
import preview from '../../.storybook/preview.js';
import { mountCemArtifactPreview, registerCemArtifactTree } from './artifact-preview.js';
const meta = preview.meta({ title: 'Runtime/Artifact tree preview' });
const artifact = (path: string, contentType: string, text: string) => ({ path, contentType, bytes: new TextEncoder().encode(text) });
function required<T>(value: T | null | undefined, label: string): T {
    if (value === null || value === undefined) throw new Error(`Missing ${label}`);
    return value;
}
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
export const StaticForeignMarkup = meta.story({
    render: () => '<iframe title="Diagram and equation"></iframe>',
    play: async ({ canvasElement }) => {
        const frame = required(canvasElement.querySelector('iframe'), 'artifact frame'), statuses: string[] = [];
        const tree = registerCemArtifactTree([artifact('index.html', 'text/html', `
<h1>Diagram and equation</h1>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 100" width="240" height="100" role="img" aria-labelledby="diagram-title" aria-describedby="diagram-description">
 <title id="diagram-title">Resource pipeline</title><desc id="diagram-description">A rectangle connects to an output circle.</desc>
 <g fill="none" stroke="currentColor" stroke-width="2"><rect x="10" y="10" width="80" height="50" rx="4"></rect><line x1="90" y1="35" x2="170" y2="35"></line><circle cx="190" cy="35" r="20"></circle></g>
 <text x="10" y="90" fill="currentColor">Input → output</text>
</svg>
<math xmlns="http://www.w3.org/1998/Math/MathML" display="block" aria-label="x squared plus one over two"><mfrac><mrow><msup><mi>x</mi><mn>2</mn></msup><mo>+</mo><mn>1</mn></mrow><mn>2</mn></mfrac></math>`) ]);
        const controller = mountCemArtifactPreview(frame, { resourceSet: tree.resourceSet, path: 'index.html', onStatus: message => statuses.push(message) });
        try {
            await controller.ready;
            const doc = required(frame.contentDocument, 'artifact document');
            const svg = required(doc.querySelector('svg'), 'SVG root'), math = required(doc.querySelector('math'), 'MathML root');
            expect(svg).not.toBeNull(); expect(math).not.toBeNull();
            expect(svg.namespaceURI).toBe('http://www.w3.org/2000/svg');
            expect([...svg.querySelectorAll('*')].every(node => node.namespaceURI === svg.namespaceURI)).toBe(true);
            expect(svg.getAttribute('viewBox')).toBe('0 0 240 100');
            expect(required(svg.querySelector('title'), 'SVG title').textContent).toBe('Resource pipeline');
            expect(required(svg.querySelector('desc'), 'SVG description').textContent).toBe('A rectangle connects to an output circle.');
            expect(svg.getAttribute('aria-labelledby')).toBe('diagram-title'); expect(svg.getAttribute('aria-describedby')).toBe('diagram-description');
            expect(required(svg.querySelector('rect'), 'SVG rectangle').getBBox().width).toBe(80); expect(required(svg.querySelector('rect'), 'SVG rectangle').getBBox().height).toBe(50);
            expect(required(svg.querySelector('circle'), 'SVG circle').getBBox().width).toBe(40); expect(required(svg.querySelector('text'), 'SVG text').getBBox().width).toBeGreaterThan(0);
            expect(svg.getBoundingClientRect().width).toBe(240);
            expect(math.namespaceURI).toBe('http://www.w3.org/1998/Math/MathML');
            expect([...math.querySelectorAll('*')].every(node => node.namespaceURI === math.namespaceURI)).toBe(true);
            expect(math.textContent).toBe('x2+12'); expect(math.getAttribute('aria-label')).toBe('x squared plus one over two');
            const fraction = required(math.querySelector('mfrac'), 'MathML fraction').getBoundingClientRect(), numerator = required(math.querySelector('mrow'), 'MathML numerator').getBoundingClientRect();
            const denominator = required(math.querySelector('mfrac > mn'), 'MathML denominator').getBoundingClientRect();
            expect(fraction.width).toBeGreaterThan(0); expect(fraction.height).toBeGreaterThan(denominator.height);
            expect(numerator.bottom).toBeLessThanOrEqual(denominator.top + 1);
            expect(required(math.querySelector('msup'), 'MathML superscript').getBoundingClientRect().height).toBeGreaterThan(0);
            expect(frame.sandbox.value).toBe('allow-same-origin'); expect(frame.referrerPolicy).toBe('no-referrer');
            const policy = required(required(doc.querySelector('meta[http-equiv="Content-Security-Policy"]'), 'CSP policy').getAttribute('content'), 'CSP content');
            for (const rule of ["script-src 'none'", "connect-src 'none'", "object-src 'none'"]) expect(policy).toContain(rule);
            expect(statuses.at(-1)).toBe('Artifact view ready');
            const retained = required(controller.checkpoint(), 'retained checkpoint'); expect(retained()).toBe(true);
            controller.setLocation('index.html#diagram-title'); expect(retained()).toBe(false);
            expect(required(controller.checkpoint(), 'retained checkpoint')()).toBe(true); expect(frame.contentDocument).toBe(doc);
        } finally { controller.dispose(); tree.dispose(); }
    },
});

export const ForeignContentAdmission = meta.story({
    render: () => '<iframe title="Bounded foreign markup"></iframe>',
    play: async ({ canvasElement }) => {
        const frame = required(canvasElement.querySelector('iframe'), 'artifact frame'), statuses: string[] = [];
        const tree = registerCemArtifactTree([artifact('index.html', 'text/html', `
<svg xmlns="http://www.w3.org/2000/svg" role="img" aria-label="Safe geometry" width="100" height="100" viewBox="0 0 100 100" onload="parent.__cemForeignArtifactAttack=true" style="background:url(https://example.test/svg-style)">
 <title>Safe geometry</title><rect id="shape" width="20" height="30" fill="red" stroke="url(https://example.test/paint)" filter="url(#filter)" onclick="parent.__cemForeignArtifactAttack=true"></rect>
 <circle cx="50" cy="50" r="10" fill="url(#gradient)"></circle>
 <foreignObject><div xmlns="http://www.w3.org/1999/xhtml"><img src="https://example.test/integration"></div></foreignObject>
 <script>parent.__cemForeignArtifactAttack=true</script><animate attributeName="href" values="https://example.test/animation"></animate><set attributeName="fill" to="url(https://example.test/set)"></set>
 <image href="https://example.test/image"></image><use href="#shape"></use><a href="https://example.test/link"><text>Denied link</text></a><unknown><circle r="5"></circle></unknown>
 <g xmlns="urn:wrong"><rect width="10" height="10"></rect></g>
 <desc>Plain description<img src="https://example.test/description"></desc>
</svg>
<math xmlns="http://www.w3.org/1998/Math/MathML" display="block" aria-label="Safe expression" href="https://example.test/math" definitionURL="https://example.test/definition" style="color:red" onclick="parent.__cemForeignArtifactAttack=true">
 <mrow><mi>x<img src="https://example.test/token"></mi><mo>+</mo><mn>1</mn></mrow>
 <annotation-xml encoding="text/html"><img src="https://example.test/annotation"></annotation-xml><semantics><annotation src="https://example.test/semantics">Denied annotation</annotation></semantics>
 <mtext><svg><circle r="7"></circle></svg></mtext><apply><plus></plus><ci>x</ci><cn>1</cn></apply><unknown>Denied MathML</unknown>
</math>
<svg xmlns="urn:wrong" role="img" aria-label="Wrong SVG namespace"><rect width="7" height="7"></rect></svg>
<math xmlns="urn:wrong" aria-label="Wrong MathML namespace"><mi>y</mi></math>`) ]);
        const controller = mountCemArtifactPreview(frame, { resourceSet: tree.resourceSet, path: 'index.html', onStatus: message => statuses.push(message) });
        try {
            await controller.ready;
            const doc = required(frame.contentDocument, 'artifact document');
            const svg = required(doc.querySelector('svg'), 'SVG root'), math = required(doc.querySelector('math'), 'MathML root');
            expect(svg).not.toBeNull(); expect(math).not.toBeNull();
            expect(doc.querySelectorAll('svg')).toHaveLength(1); expect(doc.querySelectorAll('math')).toHaveLength(1);
            expect(required(svg.querySelector('rect'), 'SVG rectangle').getAttribute('fill')).toBe('red');
            expect(required(svg.querySelector('rect'), 'SVG rectangle').hasAttribute('stroke')).toBe(false); expect(required(svg.querySelector('circle'), 'SVG circle').hasAttribute('fill')).toBe(false);
            expect(doc.querySelector('script,foreignObject,animate,set,use,image,svg a,annotation-xml,semantics,mtext,apply,plus,ci,cn,unknown,img,[onload],[onclick],[href],[definitionURL],[style],[filter]')).toBeNull();
            expect(required(svg.querySelector('desc'), 'SVG description').textContent).toBe('Plain description'); expect(required(math.textContent, 'MathML text').trim()).toBe('x+1');
            expect([...svg.attributes, ...math.attributes].every(attribute => !attribute.namespaceURI || attribute.name === 'xmlns')).toBe(true);
            expect(statuses.at(-1)).toContain('Blocked artifact');
            expect(required(frame.contentWindow, 'artifact window').location.href).toBe('about:srcdoc');
            expect((window as Window & { __cemForeignArtifactAttack?: boolean }).__cemForeignArtifactAttack).toBeUndefined();
            expect(performance.getEntriesByType('resource').some(entry => entry.name.startsWith('https://example.test/'))).toBe(false);
            expect(required(controller.checkpoint(), 'retained checkpoint')()).toBe(true);
        } finally { controller.dispose(); tree.dispose(); }
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
